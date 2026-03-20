use crate::ai::AiUsage;
use crate::commands::commit::CommitPlan;
use anyhow::Result;
use crossterm::style::Stylize;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::collections::HashMap;
use std::io::{self, IsTerminal, Write};
use std::time::Duration;

/// Create a styled spinner for long-running operations.
pub fn spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_draw_target(ProgressDrawTarget::stdout());
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("  {spinner:.cyan} {msg}")
            .unwrap(),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

/// Finish a spinner, replacing it with a green checkmark line.
pub fn spinner_done(pb: &ProgressBar, detail: Option<&str>) {
    let msg = pb.message();
    pb.finish_and_clear();
    phase_ok(&msg, detail);
}

/// Print the command header.
pub fn header(cmd: &str) {
    println!();
    println!("  {}", cmd.cyan().bold());
    println!("  {}", "─".repeat(40).dim());
    println!();
}

/// Print a completed phase with green checkmark.
pub fn phase_ok(msg: &str, detail: Option<&str>) {
    let suffix = detail
        .map(|d| format!(" · {}", d.dim()))
        .unwrap_or_default();
    println!("  {} {msg}{suffix}", "✓".green().bold());
}

/// Print a warning message.
pub fn warn(msg: &str) {
    println!("  {} {}", "⚠".yellow().bold(), msg.yellow());
}

/// Print an info message.
pub fn info(msg: &str) {
    println!("  {} {}", "ℹ".cyan(), msg.dim());
}

/// Display the commit plan with file statuses and optional cache label.
pub fn display_plan(
    plan: &CommitPlan,
    statuses: &HashMap<String, char>,
    cache_label: Option<&str>,
) {
    let count = plan.commits.len();
    let count_str = format!("{count} commit{}", if count == 1 { "" } else { "s" });
    let label = match cache_label {
        Some(l) => format!("{count_str} · {l}"),
        None => count_str,
    };

    println!();
    println!("  {} {}", "COMMIT PLAN".bold(), format!("· {label}").dim());
    let rule = "─".repeat(50);
    println!("  {}", rule.as_str().dim());

    for (i, commit) in plan.commits.iter().enumerate() {
        let order = commit.order.unwrap_or(i as u32 + 1);
        let idx = format!("[{order}]");

        println!();
        println!(
            "  {} {}",
            idx.as_str().cyan().bold(),
            commit.message.as_str().bold()
        );

        if let Some(body) = &commit.body
            && !body.is_empty()
        {
            for line in body.lines() {
                println!("   {}  {}", "│".dim(), line.dim());
            }
        }

        if let Some(footer) = &commit.footer
            && !footer.is_empty()
        {
            println!("   {}", "│".dim());
            for line in footer.lines() {
                println!("   {}  {}", "│".dim(), line.yellow());
            }
        }

        println!("   {}", "│".dim());

        let fc = commit.files.len();
        if fc == 0 {
            println!("   {} {}", "└─".dim(), "(no files)".dim());
        } else {
            for (j, file) in commit.files.iter().enumerate() {
                let is_last = j == fc - 1;
                let connector = if is_last { "└─" } else { "├─" };
                let status_char = statuses.get(file).copied().unwrap_or('~');
                let status_styled = match status_char {
                    'A' => format!("{}", "A".green()),
                    'D' => format!("{}", "D".red()),
                    'M' => format!("{}", "M".yellow()),
                    'R' => format!("{}", "R".blue()),
                    _ => format!("{}", "·".dim()),
                };
                println!("   {} {} {}", connector.dim(), status_styled, file);
            }
        }
    }

    println!();
    println!("  {}", rule.as_str().dim());
}

/// Print commit execution header.
pub fn commit_start(index: usize, total: usize, message: &str) {
    println!();
    println!(
        "  {} {}",
        format!("[{index}/{total}]").as_str().cyan().bold(),
        message.bold()
    );
}

/// Print a file staging result.
pub fn file_staged(file: &str, success: bool) {
    if success {
        println!("    {} {}", "✓".green(), file.dim());
    } else {
        println!("    {} {} {}", "⚠".yellow(), file, "(not found)".dim());
    }
}

/// Print commit created with short SHA.
pub fn commit_created(sha: &str) {
    println!("    {} {}", "→".green().bold(), sha.green());
}

/// Print commit skipped notice.
pub fn commit_skipped() {
    println!("    {} {}", "−".yellow(), "skipped (no staged files)".dim());
}

/// Print final summary with commit list.
pub fn summary(commits: &[(String, String)]) {
    let count = commits.len();
    println!();
    println!(
        "  {} {} commit{} created",
        "✓".green().bold(),
        count.to_string().as_str().bold(),
        if count == 1 { "" } else { "s" }
    );
    println!();
    for (sha, msg) in commits {
        println!("    {}  {}", sha.as_str().dim(), msg);
    }
    println!();
}

/// Display a tool call above an active spinner.
pub fn tool_call(pb: &ProgressBar, cmd: &str) {
    pb.println(format!("    {} {}", "▸".cyan(), cmd.dim()));
}

/// Display token usage and cost.
pub fn usage(usage: &AiUsage) {
    let cost = usage
        .cost_usd
        .map(|c| format!(" · ${c:.4}"))
        .unwrap_or_default();
    println!(
        "  {} {} in / {} out{}",
        "⊘".dim(),
        format_tokens(usage.input_tokens).dim(),
        format_tokens(usage.output_tokens).dim(),
        cost.dim()
    );
}

/// Format a token count for display (e.g. 1234 -> "1.2k").
pub fn format_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

/// Ask for yes/no confirmation. Returns false in non-TTY environments.
pub fn confirm(prompt: &str) -> Result<bool> {
    if !io::stdin().is_terminal() {
        return Ok(false);
    }

    print!("  {} ", prompt.bold());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let trimmed = input.trim().to_lowercase();

    Ok(trimmed == "y" || trimmed == "yes")
}
