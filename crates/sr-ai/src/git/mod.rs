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
    /// The snapshot directly copies every changed/added/deleted file into
    /// `files/` alongside a `manifest.json` that records each file's status
    /// and whether it was staged. This avoids git-stash entirely — restore
    /// is a plain file copy that cannot conflict.
    ///
    /// Lives completely outside the repo so the agent cannot touch it.
    pub fn snapshot_working_tree(&self) -> Result<PathBuf> {
        let snapshot_dir = snapshot_dir_for(&self.root)
            .context("failed to resolve snapshot directory (no data directory available)")?;
        // Start fresh — remove any prior snapshot for this repo
        if snapshot_dir.exists() {
            std::fs::remove_dir_all(&snapshot_dir).ok();
        }
        std::fs::create_dir_all(&snapshot_dir).context("failed to create snapshot directory")?;

        let files_dir = snapshot_dir.join("files");
        std::fs::create_dir_all(&files_dir)?;

        // Record which repo this snapshot belongs to
        std::fs::write(
            snapshot_dir.join("repo_root"),
            self.root.to_string_lossy().as_bytes(),
        )
        .context("failed to write repo_root")?;

        // Record current HEAD so we can reset if partial commits were made
        let (has_head, head_ref) = self.git_allow_failure(&["rev-parse", "HEAD"])?;
        if has_head {
            std::fs::write(snapshot_dir.join("head_ref"), head_ref.trim())
                .context("failed to write head_ref")?;
        }

        // Build manifest: every file that shows up in `git status --porcelain`
        // gets its content copied and its status recorded.
        let porcelain = self.git(&["status", "--porcelain"])?;
        let staged_names = self.git(&["diff", "--cached", "--name-only"])?;
        let staged_set: std::collections::HashSet<&str> = staged_names
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();

        #[derive(serde::Serialize, serde::Deserialize)]
        struct ManifestEntry {
            path: String,
            /// X (index) status character from porcelain
            index_status: char,
            /// Y (worktree) status character from porcelain
            worktree_status: char,
            /// Whether the file was staged at snapshot time
            staged: bool,
            /// Whether a file copy exists in the snapshot (false for deletions)
            has_content: bool,
        }

        let mut manifest: Vec<ManifestEntry> = Vec::new();

        for line in porcelain.lines() {
            if line.len() < 3 {
                continue;
            }
            let bytes = line.as_bytes();
            let x = bytes[0] as char;
            let y = bytes[1] as char;
            let mut path = line[3..].to_string();
            // Handle renames: "R  old -> new"
            if let Some(pos) = path.find(" -> ") {
                path = path[pos + 4..].to_string();
            }

            let src = self.root.join(&path);
            let has_content = src.exists() && src.is_file();

            if has_content {
                let dest = files_dir.join(&path);
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent).ok();
                }
                if let Err(e) = std::fs::copy(&src, &dest) {
                    eprintln!("warning: failed to snapshot {path}: {e}");
                }
            }

            manifest.push(ManifestEntry {
                staged: staged_set.contains(path.as_str()),
                path,
                index_status: x,
                worktree_status: y,
                has_content,
            });
        }

        let manifest_json =
            serde_json::to_string_pretty(&manifest).context("failed to serialize manifest")?;
        std::fs::write(snapshot_dir.join("manifest.json"), manifest_json)
            .context("failed to write manifest.json")?;

        // Mark snapshot as valid
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        std::fs::write(snapshot_dir.join("timestamp"), now.to_string())
            .context("failed to write timestamp")?;

        Ok(snapshot_dir)
    }

    /// Restore working tree from the latest snapshot.
    ///
    /// 1. Reset HEAD to the original commit (undoes any partial commits)
    /// 2. Clean the index
    /// 3. Copy every snapshotted file back from `files/`
    /// 4. Delete files that were deleted at snapshot time
    /// 5. Re-stage files that were staged at snapshot time
    ///
    /// This is a plain file copy — no git-stash, no merge conflicts.
    pub fn restore_snapshot(&self) -> Result<()> {
        let snapshot_dir = self.snapshot_dir()?;
        if !snapshot_dir.join("timestamp").exists() {
            bail!("no valid snapshot found");
        }

        let files_dir = snapshot_dir.join("files");

        // Step 1: Reset HEAD to pre-operation state
        let head_ref_path = snapshot_dir.join("head_ref");
        if head_ref_path.exists() {
            let original_head = std::fs::read_to_string(&head_ref_path)?;
            let original_head = original_head.trim();
            if !original_head.is_empty() {
                let _ = self.git_allow_failure(&["reset", "--soft", original_head]);
            }
        }

        // Step 2: Clean the index
        self.reset_head()?;

        // Step 3-5: Restore files from manifest
        let manifest_path = snapshot_dir.join("manifest.json");
        if !manifest_path.exists() {
            bail!("snapshot manifest.json missing — cannot restore");
        }

        #[derive(serde::Deserialize)]
        struct ManifestEntry {
            path: String,
            index_status: char,
            worktree_status: char,
            staged: bool,
            has_content: bool,
        }

        let manifest_data = std::fs::read_to_string(&manifest_path)?;
        let manifest: Vec<ManifestEntry> =
            serde_json::from_str(&manifest_data).context("failed to parse snapshot manifest")?;

        let mut restored = 0usize;
        let mut failed = 0usize;

        for entry in &manifest {
            let dest = self.root.join(&entry.path);

            if entry.has_content {
                // Restore file content from snapshot copy
                let src = files_dir.join(&entry.path);
                if src.exists() {
                    if let Some(parent) = dest.parent() {
                        std::fs::create_dir_all(parent).ok();
                    }
                    match std::fs::copy(&src, &dest) {
                        Ok(_) => restored += 1,
                        Err(e) => {
                            eprintln!("warning: failed to restore {}: {e}", entry.path);
                            failed += 1;
                        }
                    }
                } else {
                    eprintln!("warning: snapshot missing content for {}", entry.path);
                    failed += 1;
                }
            } else if entry.index_status == 'D' || entry.worktree_status == 'D' {
                // File was deleted at snapshot time — ensure it stays deleted
                if dest.exists() {
                    std::fs::remove_file(&dest).ok();
                }
            }

            // Re-stage if it was staged at snapshot time
            if entry.staged {
                let _ = self.git_allow_failure(&["add", "--", &entry.path]);
            }
        }

        if failed > 0 {
            eprintln!("sr: restored {restored} files, {failed} failed");
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Create a temporary git repo with an initial commit and return a GitRepo.
    fn temp_repo() -> (tempfile::TempDir, GitRepo) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();

        let git = |args: &[&str]| {
            Command::new("git")
                .args(["-C", root.to_str().unwrap()])
                .args(args)
                .output()
                .unwrap()
        };

        git(&["init"]);
        git(&["config", "user.email", "test@test.com"]);
        git(&["config", "user.name", "Test"]);
        // Initial commit so HEAD exists
        fs::write(root.join("init.txt"), "init").unwrap();
        git(&["add", "init.txt"]);
        git(&["commit", "-m", "initial"]);

        let repo = GitRepo { root };
        (dir, repo)
    }

    #[test]
    fn snapshot_creates_manifest_with_staged_files() {
        let (_dir, repo) = temp_repo();

        // Create and stage a new file
        fs::write(repo.root.join("new.go"), "package main").unwrap();
        repo.git(&["add", "new.go"]).unwrap();

        let snap_dir = repo.snapshot_working_tree().unwrap();

        // Manifest should exist
        let manifest_path = snap_dir.join("manifest.json");
        assert!(manifest_path.exists(), "manifest.json should exist");

        let data = fs::read_to_string(&manifest_path).unwrap();
        assert!(data.contains("new.go"), "manifest should list new.go");
        assert!(
            data.contains("\"staged\": true"),
            "new.go should be marked staged"
        );

        // File copy should exist
        assert!(
            snap_dir.join("files/new.go").exists(),
            "file content should be copied"
        );
        assert_eq!(
            fs::read_to_string(snap_dir.join("files/new.go")).unwrap(),
            "package main"
        );

        // HEAD ref should be recorded
        assert!(snap_dir.join("head_ref").exists());

        repo.clear_snapshot();
    }

    #[test]
    fn snapshot_restore_recovers_staged_new_files() {
        let (_dir, repo) = temp_repo();

        // Stage two new files
        fs::write(repo.root.join("a.go"), "package a").unwrap();
        fs::write(repo.root.join("b.go"), "package b").unwrap();
        repo.git(&["add", "a.go", "b.go"]).unwrap();

        repo.snapshot_working_tree().unwrap();

        // Simulate what execute_plan does: reset head, stage partially, commit
        repo.reset_head().unwrap();
        repo.git(&["add", "a.go"]).unwrap();
        repo.git(&["commit", "-m", "partial"]).unwrap();

        // Now restore — should undo the partial commit and recover both files staged
        repo.restore_snapshot().unwrap();

        // Both files should exist
        assert!(repo.root.join("a.go").exists());
        assert!(repo.root.join("b.go").exists());
        assert_eq!(
            fs::read_to_string(repo.root.join("a.go")).unwrap(),
            "package a"
        );
        assert_eq!(
            fs::read_to_string(repo.root.join("b.go")).unwrap(),
            "package b"
        );

        // Both should be staged
        let staged = repo.git(&["diff", "--cached", "--name-only"]).unwrap();
        assert!(staged.contains("a.go"), "a.go should be re-staged");
        assert!(staged.contains("b.go"), "b.go should be re-staged");

        // The partial commit should be gone
        let log = repo.git(&["log", "--oneline"]).unwrap();
        assert!(
            !log.contains("partial"),
            "partial commit should be undone by HEAD reset"
        );

        repo.clear_snapshot();
    }

    #[test]
    fn snapshot_restore_with_dirty_index_does_not_conflict() {
        let (_dir, repo) = temp_repo();

        // Stage a new file
        fs::write(repo.root.join("file.rs"), "fn main() {}").unwrap();
        repo.git(&["add", "file.rs"]).unwrap();

        repo.snapshot_working_tree().unwrap();

        // Simulate partial staging left by a failed execute_plan
        repo.reset_head().unwrap();
        repo.git(&["add", "file.rs"]).unwrap();
        // Don't commit — index is dirty with the same file

        // Restore should NOT fail (this was the original bug)
        let result = repo.restore_snapshot();
        assert!(
            result.is_ok(),
            "restore should succeed with dirty index: {result:?}"
        );

        assert_eq!(
            fs::read_to_string(repo.root.join("file.rs")).unwrap(),
            "fn main() {}"
        );

        repo.clear_snapshot();
    }

    #[test]
    fn snapshot_handles_modified_files() {
        let (_dir, repo) = temp_repo();

        // Modify an existing tracked file
        fs::write(repo.root.join("init.txt"), "modified content").unwrap();
        repo.git(&["add", "init.txt"]).unwrap();

        repo.snapshot_working_tree().unwrap();

        // Simulate: reset and make a different change
        repo.reset_head().unwrap();
        fs::write(repo.root.join("init.txt"), "wrong content").unwrap();

        // Restore should bring back the original modified content
        repo.restore_snapshot().unwrap();

        assert_eq!(
            fs::read_to_string(repo.root.join("init.txt")).unwrap(),
            "modified content"
        );

        repo.clear_snapshot();
    }

    #[test]
    fn snapshot_guard_restores_on_drop() {
        let (_dir, repo) = temp_repo();

        fs::write(repo.root.join("guarded.txt"), "important").unwrap();
        repo.git(&["add", "guarded.txt"]).unwrap();

        {
            let _guard = SnapshotGuard::new(&repo).unwrap();
            // Simulate failure: reset and delete the file
            repo.reset_head().unwrap();
            fs::remove_file(repo.root.join("guarded.txt")).ok();
            // Guard drops here without calling success()
        }

        // File should be restored
        assert!(repo.root.join("guarded.txt").exists());
        assert_eq!(
            fs::read_to_string(repo.root.join("guarded.txt")).unwrap(),
            "important"
        );
    }

    #[test]
    fn snapshot_guard_clears_on_success() {
        let (_dir, repo) = temp_repo();

        fs::write(repo.root.join("ok.txt"), "data").unwrap();
        repo.git(&["add", "ok.txt"]).unwrap();

        let guard = SnapshotGuard::new(&repo).unwrap();
        assert!(repo.has_snapshot());
        guard.success();

        // Snapshot should be cleared
        assert!(!repo.has_snapshot());
    }
}
