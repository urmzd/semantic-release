use crate::ai::{AiEvent, AiRequest, BackendConfig, resolve_backend};
use crate::git::GitRepo;
use crate::ui;
use anyhow::{Context, Result, bail};
use indicatif::ProgressBar;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;

#[derive(Debug, clap::Args)]
pub struct RebaseArgs {
    /// Additional context or instructions for reorganization
    #[arg(short = 'M', long)]
    pub message: Option<String>,

    /// Display plan without executing
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Skip confirmation prompt
    #[arg(short, long)]
    pub yes: bool,

    /// Number of recent commits to reorganize (default: auto-detect since last tag)
    #[arg(long)]
    pub last: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorganizePlan {
    pub commits: Vec<ReorganizedCommit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorganizedCommit {
    /// Original SHA (short) — use "squash" to fold into the previous commit
    pub original_sha: String,
    /// Action: "pick", "reword", "squash", "drop"
    pub action: String,
    /// New commit message (required for pick/reword/squash)
    pub message: String,
    pub body: Option<String>,
    pub footer: Option<String>,
}

const REORGANIZE_SCHEMA: &str = r#"{
    "type": "object",
    "properties": {
        "commits": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "original_sha": { "type": "string", "description": "Short SHA of the original commit" },
                    "action": { "type": "string", "enum": ["pick", "reword", "squash", "drop"], "description": "Rebase action" },
                    "message": { "type": "string", "description": "New commit message header (type(scope): subject)" },
                    "body": { "type": "string", "description": "New commit body (optional)" },
                    "footer": { "type": "string", "description": "New commit footer (optional)" }
                },
                "required": ["original_sha", "action", "message"]
            }
        }
    },
    "required": ["commits"]
}"#;

/// Guard that removes a temp directory on drop.
struct TmpDirGuard(std::path::PathBuf);

impl Drop for TmpDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn format_done_detail(count: usize, label: &str, usage: &Option<crate::ai::AiUsage>) -> String {
    let commits = format!("{count} commit{}", if count == 1 { "" } else { "s" });
    let extra_part = if label.is_empty() {
        String::new()
    } else {
        format!(" · {label}")
    };
    let usage_part = match usage {
        Some(u) => {
            let cost = u
                .cost_usd
                .map(|c| format!(" · ${c:.4}"))
                .unwrap_or_default();
            format!(
                " · {} in / {} out{}",
                ui::format_tokens(u.input_tokens),
                ui::format_tokens(u.output_tokens),
                cost
            )
        }
        None => String::new(),
    };
    format!("{commits}{extra_part}{usage_part}")
}

fn spawn_event_handler(
    spinner: &ProgressBar,
) -> (mpsc::UnboundedSender<AiEvent>, tokio::task::JoinHandle<()>) {
    let (tx, mut rx) = mpsc::unbounded_channel::<AiEvent>();
    let spinner_clone = spinner.clone();
    let handle = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                AiEvent::ToolCall { input, .. } => ui::tool_call(&spinner_clone, &input),
            }
        }
    });
    (tx, handle)
}

pub async fn run(args: &RebaseArgs, backend_config: &BackendConfig) -> Result<()> {
    ui::header("sr rebase");

    let repo = GitRepo::discover()?;
    ui::phase_ok("Repository found", None);

    if repo.has_any_changes()? {
        bail!("cannot rebase: you have uncommitted changes. Please commit or stash them first.");
    }

    // Load config for commit pattern/types
    let config = sr_core::config::ReleaseConfig::find_config(repo.root().as_path())
        .map(|(path, _)| sr_core::config::ReleaseConfig::load(&path))
        .transpose()?
        .unwrap_or_default();
    let type_names: Vec<&str> = config.types.iter().map(|t| t.name.as_str()).collect();

    // Determine how many commits to reorganize
    let commit_count = match args.last {
        Some(n) => n,
        None => {
            // Auto-detect: count commits since last tag
            let count = repo.commits_since_last_tag()?;
            if count == 0 {
                bail!("no commits found to rebase");
            }
            count
        }
    };

    if commit_count < 2 {
        bail!("need at least 2 commits to rebase (found {commit_count})");
    }

    // Get commit details
    let log = repo.log_detailed(commit_count)?;
    ui::phase_ok("Commits loaded", Some(&format!("{commit_count} commits")));

    // Resolve AI backend
    let backend = resolve_backend(backend_config).await?;
    let backend_name = backend.name().to_string();
    let model_name = backend_config
        .model
        .as_deref()
        .unwrap_or("default")
        .to_string();
    ui::phase_ok(
        "Backend resolved",
        Some(&format!("{backend_name} ({model_name})")),
    );

    // Build prompt
    let system_prompt = build_system_prompt(&config.commit_pattern, &type_names);
    let user_prompt = build_user_prompt(&log, args.message.as_deref())?;

    let spinner = ui::spinner(&format!("Analyzing commits with {backend_name}..."));
    let (tx, event_handler) = spawn_event_handler(&spinner);

    let request = AiRequest {
        system_prompt,
        user_prompt,
        json_schema: Some(REORGANIZE_SCHEMA.to_string()),
        working_dir: repo.root().to_string_lossy().to_string(),
    };

    let response = backend.request(&request, Some(tx)).await?;
    let _ = event_handler.await;

    let plan: ReorganizePlan = serde_json::from_str(&response.text)
        .or_else(|_| {
            let value: serde_json::Value = serde_json::from_str(&response.text)?;
            serde_json::from_value(value)
        })
        .context("failed to parse rebase plan from AI response")?;

    let detail = format_done_detail(plan.commits.len(), "", &response.usage);
    ui::spinner_done(&spinner, Some(&detail));

    if plan.commits.is_empty() {
        bail!("AI returned an empty rebase plan");
    }

    // Display the plan
    display_plan(&plan);

    if args.dry_run {
        ui::info("Dry run — no changes made");
        println!();
        return Ok(());
    }

    if !args.yes && !ui::confirm("Execute rebase? [y/N]")? {
        bail!(crate::error::SrAiError::Cancelled);
    }

    // Execute via git rebase
    execute_rebase(&repo, &plan, commit_count)?;

    Ok(())
}

fn build_system_prompt(commit_pattern: &str, type_names: &[&str]) -> String {
    let types_list = type_names.join(", ");
    format!(
        r#"You are an expert at organizing git history. You will be given a list of recent commits and asked to reorganize them.

You can:
- **pick**: keep the commit as-is (but you may reword the message)
- **reword**: keep the commit but change the message
- **squash**: fold the commit into the previous one (combine their changes)
- **drop**: remove the commit entirely (use sparingly — only for truly empty or duplicate commits)

COMMIT MESSAGE FORMAT:
- Must match this regex: {commit_pattern}
- Format: type(scope): subject
- Valid types ONLY: {types_list}
- subject: imperative mood, lowercase first letter, no period at end, max 72 chars

RULES:
- Maintain the chronological order of commits (oldest first) unless reordering improves logical grouping
- The first commit in the list CANNOT be "squash" — squash folds into the previous commit
- Prefer "reword" over "squash" when commits are logically distinct
- Only squash commits that are genuinely part of the same logical change
- Every original commit SHA must appear exactly once in your output
- If the commits are already well-organized, return them all as "pick" with improved messages if needed"#
    )
}

fn build_user_prompt(log: &str, extra: Option<&str>) -> Result<String> {
    let mut prompt = format!(
        "Analyze these recent commits and suggest how to reorganize them for a cleaner history.\n\n\
         Commits (oldest first):\n```\n{log}\n```"
    );

    if let Some(msg) = extra {
        prompt.push_str(&format!(
            "\n\nAdditional instructions from the user:\n{msg}"
        ));
    }

    Ok(prompt)
}

fn display_plan(plan: &ReorganizePlan) {
    use crossterm::style::Stylize;

    println!();
    println!(
        "  {} {}",
        "REBASE PLAN".bold(),
        format!("· {} commits", plan.commits.len()).dim()
    );
    let rule = "─".repeat(50);
    println!("  {}", rule.as_str().dim());
    println!();

    for commit in &plan.commits {
        let action_styled = match commit.action.as_str() {
            "pick" => format!("{}", "pick".green()),
            "reword" => format!("{}", "reword".yellow()),
            "squash" => format!("{}", "squash".magenta()),
            "drop" => format!("{}", "drop".red()),
            other => other.to_string(),
        };

        println!(
            "  {} {} {}",
            action_styled,
            commit.original_sha.as_str().dim(),
            commit.message.as_str().bold()
        );

        if let Some(body) = &commit.body
            && !body.is_empty()
        {
            for line in body.lines() {
                println!("   {}  {}", "│".dim(), line.dim());
            }
        }
    }

    println!();
    println!("  {}", rule.as_str().dim());
    println!();
}

fn execute_rebase(repo: &GitRepo, plan: &ReorganizePlan, commit_count: usize) -> Result<()> {
    // Build the rebase todo script
    let mut todo_lines = Vec::new();
    for commit in &plan.commits {
        let action = match commit.action.as_str() {
            "pick" | "reword" => "pick", // we'll force-reword via GIT_SEQUENCE_EDITOR
            "squash" => "squash",
            "drop" => "drop",
            other => bail!("unknown rebase action: {other}"),
        };
        todo_lines.push(format!("{action} {}", commit.original_sha));
    }
    let todo_content = todo_lines.join("\n") + "\n";

    // Build commit message rewrites: map SHA -> new full message
    let mut rewrites: HashMap<String, String> = HashMap::new();
    // Also track squash messages to combine
    let mut squash_messages: Vec<String> = Vec::new();
    let mut last_pick_sha: Option<String> = None;

    for commit in &plan.commits {
        let mut full_msg = commit.message.clone();
        if let Some(body) = &commit.body
            && !body.is_empty()
        {
            full_msg.push_str("\n\n");
            full_msg.push_str(body);
        }
        if let Some(footer) = &commit.footer
            && !footer.is_empty()
        {
            full_msg.push_str("\n\n");
            full_msg.push_str(footer);
        }

        match commit.action.as_str() {
            "pick" | "reword" => {
                // Flush any pending squash messages into the last pick
                if !squash_messages.is_empty() {
                    if let Some(ref sha) = last_pick_sha
                        && let Some(existing) = rewrites.get_mut(sha)
                    {
                        for sq_msg in &squash_messages {
                            existing.push_str("\n\n");
                            existing.push_str(sq_msg);
                        }
                    }
                    squash_messages.clear();
                }
                last_pick_sha = Some(commit.original_sha.clone());
                rewrites.insert(commit.original_sha.clone(), full_msg);
            }
            "squash" => {
                squash_messages.push(full_msg);
            }
            _ => {}
        }
    }
    // Flush remaining squash messages
    if !squash_messages.is_empty()
        && let Some(ref sha) = last_pick_sha
        && let Some(existing) = rewrites.get_mut(sha)
    {
        for sq_msg in &squash_messages {
            existing.push_str("\n\n");
            existing.push_str(sq_msg);
        }
    }

    // Create a temporary directory for our editor scripts
    let tmp_dir = std::env::temp_dir().join(format!("sr-rebase-{}", std::process::id()));
    std::fs::create_dir_all(&tmp_dir).context("failed to create temp dir")?;
    // Ensure cleanup on exit
    let _cleanup = TmpDirGuard(tmp_dir.clone());

    // Write the todo script (used as GIT_SEQUENCE_EDITOR)
    let todo_script_path = tmp_dir.join("sequence-editor.sh");
    {
        let todo_file_path = tmp_dir.join("todo.txt");
        std::fs::write(&todo_file_path, &todo_content)?;

        let script = format!("#!/bin/sh\ncp '{}' \"$1\"\n", todo_file_path.display());
        std::fs::write(&todo_script_path, &script)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&todo_script_path, std::fs::Permissions::from_mode(0o755))?;
        }
    }

    // Write the commit message editor script (used as GIT_EDITOR / EDITOR)
    let editor_script_path = tmp_dir.join("commit-editor.sh");
    {
        // Write each rewrite message to a file named by SHA
        let msgs_dir = tmp_dir.join("msgs");
        std::fs::create_dir_all(&msgs_dir)?;
        for (sha, msg) in &rewrites {
            std::fs::write(msgs_dir.join(sha), msg)?;
        }

        // The editor script: given a commit message file, find the matching SHA
        // and replace with our rewritten message. For squash commits, git presents
        // a combined message — we replace it entirely with the pick commit's message.
        let script = format!(
            r#"#!/bin/sh
MSGS_DIR='{msgs_dir}'
MSG_FILE="$1"

# Try to find a matching SHA in the message file
for sha_file in "$MSGS_DIR"/*; do
    sha=$(basename "$sha_file")
    if grep -q "$sha" "$MSG_FILE" 2>/dev/null; then
        cp "$sha_file" "$MSG_FILE"
        exit 0
    fi
done

# For squash: the combined message won't contain a single SHA.
# Find the first pick/reword SHA that's referenced in the todo.
# Just use the message as-is if we can't match.
exit 0
"#,
            msgs_dir = msgs_dir.display()
        );
        std::fs::write(&editor_script_path, &script)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&editor_script_path, std::fs::Permissions::from_mode(0o755))?;
        }
    }

    // Run git rebase -i with our custom editors
    let base = format!("HEAD~{commit_count}");

    ui::info(&format!("Rebasing {commit_count} commits..."));

    let output = std::process::Command::new("git")
        .args(["-C", repo.root().to_str().unwrap()])
        .args(["rebase", "-i", &base])
        .env("GIT_SEQUENCE_EDITOR", todo_script_path.to_str().unwrap())
        .env("GIT_EDITOR", editor_script_path.to_str().unwrap())
        .env("EDITOR", editor_script_path.to_str().unwrap())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .context("failed to run git rebase")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Abort the rebase if it failed
        let _ = std::process::Command::new("git")
            .args(["-C", repo.root().to_str().unwrap()])
            .args(["rebase", "--abort"])
            .output();
        bail!("git rebase failed: {}", stderr.trim());
    }

    // Show the new history
    let new_log = repo.recent_commits(commit_count)?;
    println!();
    ui::phase_ok("Rebase complete", None);
    println!();
    for line in new_log.lines() {
        println!("    {line}");
    }
    println!();

    Ok(())
}
