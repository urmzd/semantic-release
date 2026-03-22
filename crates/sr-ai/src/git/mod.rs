use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

pub struct GitRepo {
    root: PathBuf,
}

#[allow(dead_code)]
impl GitRepo {
    pub fn discover() -> Result<Self> {
        let output = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .context("failed to run git")?;

        if !output.status.success() {
            bail!(crate::error::SrAiError::NotAGitRepo);
        }

        let root = String::from_utf8(output.stdout)
            .context("invalid utf-8 from git")?
            .trim()
            .into();

        Ok(Self { root })
    }

    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    fn git(&self, args: &[&str]) -> Result<String> {
        let output = Command::new("git")
            .args(["-C", self.root.to_str().unwrap()])
            .args(args)
            .output()
            .with_context(|| format!("failed to run git {}", args.join(" ")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(crate::error::SrAiError::GitCommand(format!(
                "git {} failed: {}",
                args.join(" "),
                stderr.trim()
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn git_allow_failure(&self, args: &[&str]) -> Result<(bool, String)> {
        let output = Command::new("git")
            .args(["-C", self.root.to_str().unwrap()])
            .args(args)
            .output()
            .with_context(|| format!("failed to run git {}", args.join(" ")))?;

        Ok((
            output.status.success(),
            String::from_utf8_lossy(&output.stdout).to_string(),
        ))
    }

    pub fn has_staged_changes(&self) -> Result<bool> {
        let out = self.git(&["diff", "--cached", "--name-only"])?;
        Ok(!out.trim().is_empty())
    }

    pub fn has_any_changes(&self) -> Result<bool> {
        let out = self.git(&["status", "--porcelain"])?;
        Ok(!out.trim().is_empty())
    }

    pub fn has_head(&self) -> Result<bool> {
        let (ok, _) = self.git_allow_failure(&["rev-parse", "HEAD"])?;
        Ok(ok)
    }

    pub fn reset_head(&self) -> Result<()> {
        if self.has_head()? {
            self.git(&["reset", "HEAD", "--quiet"])?;
        } else {
            // Fresh repo with no commits — unstage via rm --cached
            let _ = self.git_allow_failure(&["rm", "--cached", "-r", ".", "--quiet"]);
        }
        Ok(())
    }

    pub fn stage_file(&self, file: &str) -> Result<bool> {
        let full_path = self.root.join(file);
        let exists = full_path.exists();

        if !exists {
            // Check if it's a deleted file
            let out = self.git(&["ls-files", "--deleted"])?;
            let is_deleted = out.lines().any(|l| l.trim() == file);
            if !is_deleted {
                return Ok(false);
            }
        }

        let (ok, _) = self.git_allow_failure(&["add", "--", file])?;
        Ok(ok)
    }

    pub fn has_staged_after_add(&self) -> Result<bool> {
        self.has_staged_changes()
    }

    pub fn commit(&self, message: &str) -> Result<()> {
        let output = Command::new("git")
            .args(["-C", self.root.to_str().unwrap()])
            .args(["commit", "-F", "-"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("failed to spawn git commit")?;

        use std::io::Write;
        let mut child = output;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(message.as_bytes())?;
        }

        let out = child.wait_with_output()?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            bail!(crate::error::SrAiError::GitCommand(format!(
                "git commit failed: {}",
                stderr.trim()
            )));
        }

        Ok(())
    }

    pub fn recent_commits(&self, count: usize) -> Result<String> {
        self.git(&["--no-pager", "log", "--oneline", &format!("-{count}")])
    }

    pub fn diff_cached(&self) -> Result<String> {
        self.git(&["diff", "--cached"])
    }

    pub fn diff_cached_stat(&self) -> Result<String> {
        self.git(&["diff", "--cached", "--stat"])
    }

    pub fn diff_head(&self) -> Result<String> {
        let (ok, out) = self.git_allow_failure(&["diff", "HEAD"])?;
        if ok { Ok(out) } else { self.git(&["diff"]) }
    }

    pub fn status_porcelain(&self) -> Result<String> {
        self.git(&["status", "--porcelain"])
    }

    pub fn untracked_files(&self) -> Result<String> {
        self.git(&["ls-files", "--others", "--exclude-standard"])
    }

    pub fn show(&self, rev: &str) -> Result<String> {
        self.git(&["show", rev])
    }

    pub fn log_range(&self, base: &str, count: Option<usize>) -> Result<String> {
        let mut args = vec!["--no-pager", "log", "--oneline"];
        let count_str;
        if let Some(n) = count {
            count_str = format!("-{n}");
            args.push(&count_str);
        }
        args.push(base);
        self.git(&args)
    }

    pub fn diff_range(&self, base: &str) -> Result<String> {
        self.git(&["diff", base])
    }

    pub fn current_branch(&self) -> Result<String> {
        let out = self.git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
        Ok(out.trim().to_string())
    }

    pub fn head_short(&self) -> Result<String> {
        let out = self.git(&["rev-parse", "--short", "HEAD"])?;
        Ok(out.trim().to_string())
    }

    /// Count commits since the last tag. If no tags exist, counts all commits.
    pub fn commits_since_last_tag(&self) -> Result<usize> {
        // Try to find the most recent tag
        let (ok, tag) = self.git_allow_failure(&["describe", "--tags", "--abbrev=0"])?;
        let tag = tag.trim();

        let out = if ok && !tag.is_empty() {
            self.git(&["rev-list", &format!("{tag}..HEAD"), "--count"])?
        } else {
            self.git(&["rev-list", "HEAD", "--count"])?
        };

        out.trim()
            .parse::<usize>()
            .context("failed to parse commit count")
    }

    /// Get detailed log of recent commits (SHA, subject, body) oldest first.
    pub fn log_detailed(&self, count: usize) -> Result<String> {
        let out = self.git(&[
            "--no-pager",
            "log",
            "--reverse",
            &format!("-{count}"),
            "--format=%h %s%n%b%n---",
        ])?;
        Ok(out)
    }

    pub fn file_statuses(&self) -> Result<HashMap<String, char>> {
        let out = self.git(&["status", "--porcelain"])?;
        let mut map = HashMap::new();
        for line in out.lines() {
            if line.len() < 3 {
                continue;
            }
            let xy = &line.as_bytes()[..2];
            let mut path = line[3..].to_string();
            if let Some(pos) = path.find(" -> ") {
                path = path[pos + 4..].to_string();
            }
            let (x, y) = (xy[0], xy[1]);
            let status = match (x, y) {
                (b'?', b'?') => 'A',
                (b'A', _) | (_, b'A') => 'A',
                (b'D', _) | (_, b'D') => 'D',
                (b'R', _) | (_, b'R') => 'R',
                (b'M', _) | (_, b'M') | (b'T', _) | (_, b'T') => 'M',
                _ => '~',
            };
            map.insert(path, status);
        }
        Ok(map)
    }

    /// Create a snapshot of the working tree state into the platform data directory.
    /// Location: `<data_local_dir>/sr/snapshots/<repo-hash>/`
    ///   - macOS:   ~/Library/Application Support/sr/snapshots/<hash>/
    ///   - Linux:   ~/.local/share/sr/snapshots/<hash>/
    ///   - Windows: %LOCALAPPDATA%/sr/snapshots/<hash>/
    ///
    /// The snapshot includes:
    /// - A `stash` ref created via `git stash create` (staged + unstaged changes)
    /// - An `untracked.tar` of untracked files
    /// - The list of staged files in `staged.txt`
    /// - The repo root path in `repo_root` (so restore targets the right directory)
    ///
    /// Lives completely outside the repo so the agent cannot touch it.
    pub fn snapshot_working_tree(&self) -> Result<PathBuf> {
        let snapshot_dir = snapshot_dir_for(&self.root)
            .context("failed to resolve snapshot directory (no data directory available)")?;
        std::fs::create_dir_all(&snapshot_dir).context("failed to create snapshot directory")?;

        // Record which repo this snapshot belongs to
        std::fs::write(
            snapshot_dir.join("repo_root"),
            self.root.to_string_lossy().as_bytes(),
        )
        .context("failed to write repo_root")?;

        // Capture staged file list
        let staged = self.git(&["diff", "--cached", "--name-only"])?;
        std::fs::write(snapshot_dir.join("staged.txt"), staged.trim())
            .context("failed to write staged.txt")?;

        // Create a stash object without modifying working tree or index.
        // `git stash create` writes a stash commit but doesn't reset anything.
        let (ok, stash_ref) = self.git_allow_failure(&["stash", "create"])?;
        let stash_ref = stash_ref.trim().to_string();
        if ok && !stash_ref.is_empty() {
            std::fs::write(snapshot_dir.join("stash_ref"), &stash_ref)
                .context("failed to write stash_ref")?;
        } else {
            let _ = std::fs::remove_file(snapshot_dir.join("stash_ref"));
        }

        // Archive untracked files
        let untracked = self.git(&["ls-files", "--others", "--exclude-standard"])?;
        let untracked_files: Vec<&str> = untracked
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();
        let tar_path = snapshot_dir.join("untracked.tar");
        if !untracked_files.is_empty() {
            let file_list = untracked_files.join("\n");
            let file_list_path = snapshot_dir.join("untracked_list.txt");
            std::fs::write(&file_list_path, &file_list)?;
            let status = Command::new("tar")
                .args([
                    "cf",
                    tar_path.to_str().unwrap(),
                    "-T",
                    file_list_path.to_str().unwrap(),
                ])
                .current_dir(&self.root)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .context("failed to run tar")?;
            if !status.success() {
                eprintln!("warning: failed to archive untracked files");
            }
        } else {
            let _ = std::fs::remove_file(&tar_path);
        }

        // Mark snapshot as valid
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        std::fs::write(snapshot_dir.join("timestamp"), now.to_string())
            .context("failed to write timestamp")?;

        Ok(snapshot_dir)
    }

    /// Restore working tree from the latest snapshot (best-effort).
    pub fn restore_snapshot(&self) -> Result<()> {
        let snapshot_dir = self.snapshot_dir()?;
        if !snapshot_dir.join("timestamp").exists() {
            bail!("no valid snapshot found");
        }

        // Restore tracked changes from stash ref
        let stash_ref_path = snapshot_dir.join("stash_ref");
        if stash_ref_path.exists() {
            let stash_ref = std::fs::read_to_string(&stash_ref_path)?;
            let stash_ref = stash_ref.trim();
            if !stash_ref.is_empty() {
                let (ok, _) = self.git_allow_failure(&["stash", "apply", stash_ref])?;
                if !ok {
                    eprintln!("warning: failed to apply stash {stash_ref}, trying checkout");
                    let _ = self.git_allow_failure(&["checkout", "."]);
                    let _ = self.git_allow_failure(&["stash", "apply", stash_ref]);
                }
            }
        }

        // Restore staged files
        let staged_path = snapshot_dir.join("staged.txt");
        if staged_path.exists() {
            let staged = std::fs::read_to_string(&staged_path)?;
            for file in staged.lines().filter(|l| !l.trim().is_empty()) {
                let full = self.root.join(file);
                if full.exists() {
                    let _ = self.git_allow_failure(&["add", "--", file]);
                }
            }
        }

        // Restore untracked files
        let tar_path = snapshot_dir.join("untracked.tar");
        if tar_path.exists() {
            let _ = Command::new("tar")
                .args(["xf", tar_path.to_str().unwrap()])
                .current_dir(&self.root)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
        }

        Ok(())
    }

    /// Remove the snapshot after a successful operation.
    pub fn clear_snapshot(&self) {
        if let Ok(dir) = self.snapshot_dir() {
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    /// Returns the snapshot directory path for this repo.
    pub fn snapshot_dir(&self) -> Result<PathBuf> {
        snapshot_dir_for(&self.root)
            .context("failed to resolve snapshot directory (no data directory available)")
    }

    /// Check if a valid snapshot exists.
    pub fn has_snapshot(&self) -> bool {
        self.snapshot_dir()
            .map(|d| d.join("timestamp").exists())
            .unwrap_or(false)
    }
}

/// Resolve the snapshot directory for a repo root.
/// `<data_local_dir>/sr/snapshots/<repo-hash>/`
fn snapshot_dir_for(repo_root: &std::path::Path) -> Option<PathBuf> {
    let base = dirs::data_local_dir()?;
    let repo_id =
        &crate::cache::fingerprint::sha256_hex(repo_root.to_string_lossy().as_bytes())[..16];
    Some(base.join("sr").join("snapshots").join(repo_id))
}

/// Guard that ensures the snapshot is cleaned up on success
/// and restored on failure (drop without explicit success).
pub struct SnapshotGuard<'a> {
    repo: &'a GitRepo,
    succeeded: bool,
}

impl<'a> SnapshotGuard<'a> {
    /// Create a snapshot and return the guard.
    pub fn new(repo: &'a GitRepo) -> Result<Self> {
        repo.snapshot_working_tree()?;
        Ok(Self {
            repo,
            succeeded: false,
        })
    }

    /// Mark the operation as successful — snapshot will be cleared on drop.
    pub fn success(mut self) {
        self.succeeded = true;
        self.repo.clear_snapshot();
    }
}

impl Drop for SnapshotGuard<'_> {
    fn drop(&mut self) {
        if !self.succeeded && self.repo.has_snapshot() {
            eprintln!("sr: operation failed, restoring working tree from snapshot...");
            if let Err(e) = self.repo.restore_snapshot() {
                eprintln!("sr: warning: snapshot restore failed: {e}");
                if let Ok(dir) = self.repo.snapshot_dir() {
                    eprintln!(
                        "sr: snapshot preserved at {} for manual recovery",
                        dir.display()
                    );
                }
            } else {
                self.repo.clear_snapshot();
            }
        }
    }
}
