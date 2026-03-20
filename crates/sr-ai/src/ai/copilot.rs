use super::{AiBackend, AiEvent, AiRequest, AiResponse};
use anyhow::{Context, Result};
use async_trait::async_trait;
use tokio::process::Command;
use tokio::sync::mpsc;

const DEFAULT_MODEL: &str = "gpt-4.1";

pub struct CopilotBackend {
    model: Option<String>,
    debug: bool,
}

impl CopilotBackend {
    pub fn new(model: Option<String>, debug: bool) -> Self {
        Self { model, debug }
    }
}

/// Build the system prompt, embedding the JSON schema when present.
fn build_system_prompt(base: &str, json_schema: Option<&str>) -> String {
    match json_schema {
        Some(schema) => format!(
            "{base}\n\n\
             You MUST respond with valid JSON matching this schema:\n\
             ```json\n{schema}\n```\n\n\
             Respond ONLY with the JSON object, no markdown fences, no explanation."
        ),
        None => base.to_string(),
    }
}

#[async_trait]
impl AiBackend for CopilotBackend {
    fn name(&self) -> &str {
        "copilot"
    }

    async fn is_available(&self) -> bool {
        Command::new("gh")
            .args(["copilot", "--version"])
            .output()
            .await
            .is_ok_and(|o| o.status.success())
    }

    async fn request(
        &self,
        req: &AiRequest,
        _events: Option<mpsc::UnboundedSender<AiEvent>>,
    ) -> Result<AiResponse> {
        let model = self.model.as_deref().unwrap_or(DEFAULT_MODEL);
        let system = build_system_prompt(&req.system_prompt, req.json_schema.as_deref());

        let mut cmd = Command::new("gh");
        cmd.current_dir(&req.working_dir)
            .arg("copilot")
            .arg("-p")
            .arg(&req.user_prompt)
            .arg("-s")
            .arg("--model")
            .arg(model)
            .arg("--allow-tool")
            .arg("shell(git:*)")
            .arg("--no-custom-instructions")
            .arg("--system-prompt")
            .arg(&system);

        if self.debug {
            eprintln!("[DEBUG] Calling gh copilot (model={model})");
        }

        let output = cmd.output().await.context("failed to run gh copilot")?;

        let raw = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr);

        if self.debug {
            eprintln!("[DEBUG] gh copilot exit code: {}", output.status);
            eprintln!(
                "[DEBUG] Raw response (first 500 chars): {}",
                &raw[..raw.len().min(500)]
            );
            if !stderr.is_empty() {
                eprintln!("[DEBUG] Stderr: {stderr}");
            }
        }

        if !output.status.success() {
            anyhow::bail!(crate::error::SrAiError::AiBackend(format!(
                "gh copilot failed (exit {}): {}",
                output.status,
                stderr.trim()
            )));
        }

        // Extract JSON from response (may be wrapped in markdown fences)
        let text = extract_json(&raw).unwrap_or(raw);

        Ok(AiResponse { text, usage: None })
    }
}

/// Extract JSON from a response that may contain markdown code fences.
pub(crate) fn extract_json(raw: &str) -> Option<String> {
    let trimmed = raw.trim();

    // Try direct parse first
    if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
        return Some(trimmed.to_string());
    }

    // Try extracting from ```json ... ``` fences
    if let Some(start) = trimmed.find("```json") {
        let after = &trimmed[start + 7..];
        if let Some(end) = after.find("```") {
            let json_str = after[..end].trim();
            if serde_json::from_str::<serde_json::Value>(json_str).is_ok() {
                return Some(json_str.to_string());
            }
        }
    }

    // Try extracting from ``` ... ``` fences
    if let Some(start) = trimmed.find("```") {
        let after = &trimmed[start + 3..];
        let after = if let Some(nl) = after.find('\n') {
            &after[nl + 1..]
        } else {
            after
        };
        if let Some(end) = after.find("```") {
            let json_str = after[..end].trim();
            if serde_json::from_str::<serde_json::Value>(json_str).is_ok() {
                return Some(json_str.to_string());
            }
        }
    }

    // Try finding first { ... } or [ ... ]
    for (open, close) in [("{", "}"), ("[", "]")] {
        if let Some(start) = trimmed.find(open)
            && let Some(end) = trimmed.rfind(close)
            && end > start
        {
            let candidate = &trimmed[start..=end];
            if serde_json::from_str::<serde_json::Value>(candidate).is_ok() {
                return Some(candidate.to_string());
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- extract_json tests ---

    #[test]
    fn extract_direct_json() {
        let input = r#"{"commits": []}"#;
        assert_eq!(extract_json(input), Some(input.to_string()));
    }

    #[test]
    fn extract_from_json_fences() {
        let input = "Here is the plan:\n```json\n{\"commits\": []}\n```\nDone.";
        assert_eq!(extract_json(input), Some(r#"{"commits": []}"#.to_string()));
    }

    #[test]
    fn extract_from_plain_fences() {
        let input = "Result:\n```\n{\"commits\": [{\"order\": 1}]}\n```";
        assert_eq!(
            extract_json(input),
            Some(r#"{"commits": [{"order": 1}]}"#.to_string())
        );
    }

    #[test]
    fn extract_from_surrounding_text() {
        let input = "The result is {\"commits\": []} and that's it.";
        assert_eq!(extract_json(input), Some(r#"{"commits": []}"#.to_string()));
    }

    #[test]
    fn extract_array_json() {
        let input = "Here: [1, 2, 3] done";
        assert_eq!(extract_json(input), Some("[1, 2, 3]".to_string()));
    }

    #[test]
    fn extract_returns_none_for_invalid() {
        assert_eq!(extract_json("no json here"), None);
        assert_eq!(extract_json(""), None);
        assert_eq!(extract_json("{not valid json}"), None);
    }

    #[test]
    fn extract_with_whitespace() {
        let input = "  \n  {\"key\": \"value\"}  \n  ";
        assert_eq!(extract_json(input), Some(r#"{"key": "value"}"#.to_string()));
    }

    // --- build_system_prompt tests ---

    #[test]
    fn system_prompt_without_schema() {
        let result = build_system_prompt("You are a commit assistant.", None);
        assert_eq!(result, "You are a commit assistant.");
    }

    #[test]
    fn system_prompt_with_schema() {
        let schema = r#"{"type": "object"}"#;
        let result = build_system_prompt("Base prompt.", Some(schema));
        assert!(result.starts_with("Base prompt."));
        assert!(result.contains("You MUST respond with valid JSON"));
        assert!(result.contains(schema));
        assert!(result.contains("no markdown fences"));
    }

    // --- backend metadata tests ---

    #[test]
    fn backend_name() {
        let backend = CopilotBackend::new(None, false);
        assert_eq!(backend.name(), "copilot");
    }

    #[test]
    fn default_model_constant() {
        assert_eq!(DEFAULT_MODEL, "gpt-4.1");
    }

    // --- build_system_prompt preserves base content ---

    #[test]
    fn system_prompt_preserves_multiline_base() {
        let base = "Line one.\nLine two.\nLine three.";
        let result = build_system_prompt(base, None);
        assert_eq!(result, base);

        let with_schema = build_system_prompt(base, Some("{}"));
        assert!(with_schema.starts_with(base));
    }
}
