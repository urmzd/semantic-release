use super::{AiBackend, AiEvent, AiRequest, AiResponse, AiUsage};
use anyhow::{Context, Result};
use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc::UnboundedSender;

pub struct ClaudeBackend {
    model: Option<String>,
    budget: f64,
    debug: bool,
}

impl ClaudeBackend {
    pub fn new(model: Option<String>, budget: f64, debug: bool) -> Self {
        Self {
            model,
            budget,
            debug,
        }
    }

    fn base_command(&self, working_dir: &str) -> Command {
        let model = self.model.as_deref().unwrap_or("haiku");
        let mut cmd = Command::new("claude");
        cmd.current_dir(working_dir)
            .arg("--model")
            .arg(model)
            .arg("--allowed-tools")
            .arg("Bash(git:*)")
            .arg("--max-budget-usd")
            .arg(format!("{:.2}", self.budget))
            .arg("-p");
        cmd
    }

    /// Streaming mode: use --output-format stream-json --verbose for real-time
    /// tool call events. Schema is embedded in the system prompt since
    /// --json-schema is incompatible with stream-json.
    async fn request_streaming(
        &self,
        req: &AiRequest,
        events: UnboundedSender<AiEvent>,
    ) -> Result<AiResponse> {
        let system = embed_schema(&req.system_prompt, req.json_schema.as_deref());

        let mut cmd = self.base_command(&req.working_dir);
        cmd.arg(&req.user_prompt)
            .arg("--system-prompt")
            .arg(&system)
            .arg("--output-format")
            .arg("stream-json")
            .arg("--verbose");

        if self.debug {
            eprintln!(
                "[DEBUG] claude stream-json (model={}, budget={:.2})",
                self.model.as_deref().unwrap_or("haiku"),
                self.budget
            );
        }

        let mut child = cmd
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("failed to run claude CLI")?;

        let stdout = child.stdout.take().unwrap();
        let stderr_handle = child.stderr.take().unwrap();

        // Collect stderr for error reporting
        let stderr_task = tokio::spawn(async move {
            let mut buf = String::new();
            let _ = tokio::io::AsyncReadExt::read_to_string(
                &mut BufReader::new(stderr_handle),
                &mut buf,
            )
            .await;
            buf
        });

        // Stream stdout line-by-line, parse NDJSON events
        let mut reader = BufReader::new(stdout).lines();
        let mut result_text = String::new();
        let mut usage = None;

        while let Ok(Some(line)) = reader.next_line().await {
            let event: serde_json::Value = match serde_json::from_str(line.trim()) {
                Ok(v) => v,
                Err(_) => continue,
            };

            if self.debug {
                let t = event.get("type").and_then(|t| t.as_str()).unwrap_or("?");
                eprintln!("[DEBUG] event: {t}");
            }

            // Tool calls appear in assistant messages as content blocks
            parse_tool_calls(&event, &events);

            // Final result event
            if event.get("type").and_then(|t| t.as_str()) == Some("result") {
                if let Some(r) = event.get("result") {
                    let raw = match r {
                        serde_json::Value::String(s) => s.clone(),
                        _ => r.to_string(),
                    };
                    result_text = crate::ai::copilot::extract_json(&raw).unwrap_or(raw);
                }
                usage = extract_usage(&event);
            }
        }

        let stderr_text = stderr_task.await.unwrap_or_default();
        let status = child.wait().await?;

        if !status.success() {
            anyhow::bail!(crate::error::SrAiError::AiBackend(format!(
                "claude CLI failed (exit {}): {}",
                status,
                stderr_text.trim()
            )));
        }

        if result_text.is_empty() {
            anyhow::bail!(crate::error::SrAiError::ParseResponse(
                "no result in claude stream".into()
            ));
        }

        Ok(AiResponse {
            text: result_text,
            usage,
        })
    }

    /// Batch mode: --output-format json + --json-schema for reliable structured output.
    async fn request_batch(&self, req: &AiRequest) -> Result<AiResponse> {
        let mut cmd = self.base_command(&req.working_dir);
        cmd.arg(&req.user_prompt)
            .arg("--system-prompt")
            .arg(&req.system_prompt)
            .arg("--output-format")
            .arg("json");

        if let Some(schema) = &req.json_schema {
            cmd.arg("--json-schema").arg(schema);
        }

        if self.debug {
            eprintln!(
                "[DEBUG] claude json (model={}, budget={:.2})",
                self.model.as_deref().unwrap_or("haiku"),
                self.budget
            );
        }

        let output = cmd.output().await.context("failed to run claude CLI")?;
        let raw = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr);

        if self.debug {
            eprintln!("[DEBUG] exit: {}", output.status);
            eprintln!("[DEBUG] stdout (first 500): {}", &raw[..raw.len().min(500)]);
            if !stderr.is_empty() {
                eprintln!("[DEBUG] stderr: {stderr}");
            }
        }

        if !output.status.success() {
            anyhow::bail!(crate::error::SrAiError::AiBackend(format!(
                "claude CLI failed (exit {}): {}",
                output.status,
                stderr.trim()
            )));
        }

        let parsed: serde_json::Value =
            serde_json::from_str(&raw).context("failed to parse claude JSON response")?;
        let usage = extract_usage(&parsed);

        if req.json_schema.is_some() {
            let structured = &parsed["structured_output"];
            if structured.is_null() {
                anyhow::bail!(crate::error::SrAiError::ParseResponse(
                    "empty structured_output from claude".into()
                ));
            }
            Ok(AiResponse {
                text: structured.to_string(),
                usage,
            })
        } else {
            let text = parsed
                .get("result")
                .map(|r| match r {
                    serde_json::Value::String(s) => s.clone(),
                    _ => r.to_string(),
                })
                .unwrap_or(raw);
            Ok(AiResponse { text, usage })
        }
    }
}

#[async_trait]
impl AiBackend for ClaudeBackend {
    fn name(&self) -> &str {
        "claude"
    }

    async fn is_available(&self) -> bool {
        Command::new("claude")
            .arg("--version")
            .output()
            .await
            .is_ok_and(|o| o.status.success())
    }

    async fn request(
        &self,
        req: &AiRequest,
        events: Option<tokio::sync::mpsc::UnboundedSender<AiEvent>>,
    ) -> Result<AiResponse> {
        match events {
            Some(tx) => self.request_streaming(req, tx).await,
            None => self.request_batch(req).await,
        }
    }
}

/// Embed a JSON schema into the system prompt (used when --json-schema is unavailable).
fn embed_schema(system_prompt: &str, json_schema: Option<&str>) -> String {
    match json_schema {
        Some(schema) => format!(
            "{system_prompt}\n\n\
             You MUST respond with valid JSON matching this schema:\n\
             ```json\n{schema}\n```\n\n\
             Respond ONLY with the JSON object, no markdown fences, no explanation."
        ),
        None => system_prompt.to_string(),
    }
}

/// Extract tool calls from a stream-json event and send them through the channel.
/// Works with both assistant message events and stream_event content_block_start.
pub(crate) fn parse_tool_calls(event: &serde_json::Value, events: &UnboundedSender<AiEvent>) {
    // Assistant message format: {"message": {"content": [{"type": "tool_use", ...}]}}
    if let Some(content) = event.pointer("/message/content")
        && let Some(arr) = content.as_array()
    {
        for item in arr {
            if item["type"] == "tool_use"
                && let Some(input) = extract_tool_input(item)
            {
                let tool = item["name"].as_str().unwrap_or("unknown").to_string();
                let _ = events.send(AiEvent::ToolCall { tool, input });
            }
        }
    }

    // StreamEvent format: {"type": "stream_event", "event": {"type": "content_block_start",
    //   "content_block": {"type": "tool_use", "name": "Bash", ...}}}
    if event.get("type").and_then(|t| t.as_str()) == Some("stream_event")
        && let Some(inner) = event.get("event")
        && inner.get("type").and_then(|t| t.as_str()) == Some("content_block_start")
        && let Some(block) = inner.get("content_block")
        && block.get("type").and_then(|t| t.as_str()) == Some("tool_use")
    {
        let tool = block["name"].as_str().unwrap_or("unknown").to_string();
        // input may not be available yet in content_block_start
        let input = extract_tool_input(block).unwrap_or_default();
        if !input.is_empty() {
            let _ = events.send(AiEvent::ToolCall { tool, input });
        }
    }
}

/// Extract the tool input command or serialized input from a tool_use content block.
fn extract_tool_input(item: &serde_json::Value) -> Option<String> {
    // Bash tool: {"input": {"command": "git diff"}}
    if let Some(cmd) = item.pointer("/input/command").and_then(|c| c.as_str()) {
        return Some(cmd.to_string());
    }
    // Read tool: {"input": {"file_path": "/path/to/file"}}
    if let Some(path) = item.pointer("/input/file_path").and_then(|p| p.as_str()) {
        return Some(path.to_string());
    }
    // Fallback: serialize the input object
    item.get("input")
        .filter(|i| !i.is_null())
        .map(|i| serde_json::to_string(i).unwrap_or_default())
        .filter(|s| !s.is_empty() && s != "{}")
}

/// Extract usage and cost from a Claude response JSON object.
fn extract_usage(parsed: &serde_json::Value) -> Option<AiUsage> {
    let u = parsed.get("usage")?;
    Some(AiUsage {
        input_tokens: u.get("input_tokens")?.as_u64()?,
        output_tokens: u.get("output_tokens")?.as_u64()?,
        cost_usd: parsed.get("cost_usd").and_then(|c| c.as_f64()),
    })
}
