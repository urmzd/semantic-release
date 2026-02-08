use std::path::{Path, PathBuf};
use std::process::Command;

use semver::Version;
use sr_core::commit::Commit;
use sr_core::error::ReleaseError;
use sr_core::git::{GitRepository, TagInfo};

/// Git repository implementation backed by native `git` CLI commands.
pub struct NativeGitRepository {
    path: PathBuf,
}

impl NativeGitRepository {
    pub fn open(path: &Path) -> Result<Self, ReleaseError> {
        let repo = Self {
            path: path.to_path_buf(),
        };
        // Validate this is a git repo
        repo.git(&["rev-parse", "--git-dir"])?;
        Ok(repo)
    }

    fn git(&self, args: &[&str]) -> Result<String, ReleaseError> {
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.path)
            .args(args)
            .output()
            .map_err(|e| ReleaseError::Git(format!("failed to run git: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ReleaseError::Git(format!(
                "git {} failed: {}",
                args.join(" "),
                stderr.trim()
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Parse owner/repo from a git remote URL.
    pub fn parse_remote(&self) -> Result<(String, String), ReleaseError> {
        let url = self.git(&["remote", "get-url", "origin"])?;
        parse_owner_repo(&url)
    }
}

/// Extract owner/repo from a GitHub remote URL.
/// Supports SSH (git@github.com:owner/repo.git) and HTTPS (https://github.com/owner/repo.git).
pub fn parse_owner_repo(url: &str) -> Result<(String, String), ReleaseError> {
    let trimmed = url.trim_end_matches(".git");

    // Try HTTPS/HTTP first: https://github.com/owner/repo
    let path = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .and_then(|s| {
            // Skip the hostname: "github.com/owner/repo" -> "owner/repo"
            s.split_once('/').map(|(_, rest)| rest)
        })
        // Fall back to SSH style: git@github.com:owner/repo
        .or_else(|| trimmed.rsplit_once(':').map(|(_, p)| p))
        .ok_or_else(|| ReleaseError::Git(format!("cannot parse remote URL: {url}")))?;

    let (owner, repo) = path
        .split_once('/')
        .ok_or_else(|| ReleaseError::Git(format!("cannot parse owner/repo from: {url}")))?;

    Ok((owner.to_string(), repo.to_string()))
}

impl GitRepository for NativeGitRepository {
    fn latest_tag(&self, prefix: &str) -> Result<Option<TagInfo>, ReleaseError> {
        let pattern = format!("{prefix}*");
        let result = self.git(&["tag", "--list", &pattern, "--sort=-v:refname"]);

        let tags_output = match result {
            Ok(output) if output.is_empty() => return Ok(None),
            Ok(output) => output,
            Err(_) => return Ok(None),
        };

        let tag_name = match tags_output.lines().next() {
            Some(name) => name.trim(),
            None => return Ok(None),
        };

        let version_str = tag_name.strip_prefix(prefix).unwrap_or(tag_name);
        let version = match Version::parse(version_str) {
            Ok(v) => v,
            Err(_) => return Ok(None),
        };

        let sha = self.git(&["rev-list", "-1", tag_name])?;

        Ok(Some(TagInfo {
            name: tag_name.to_string(),
            version,
            sha,
        }))
    }

    fn commits_since(&self, from: Option<&str>) -> Result<Vec<Commit>, ReleaseError> {
        let range = match from {
            Some(sha) => format!("{sha}..HEAD"),
            None => "HEAD".to_string(),
        };

        let output = self.git(&["log", "--format=%H%n%B%n--END--", &range])?;

        if output.is_empty() {
            return Ok(Vec::new());
        }

        let mut commits = Vec::new();
        let mut current_sha: Option<String> = None;
        let mut current_message = String::new();

        for line in output.lines() {
            if line == "--END--" {
                if let Some(sha) = current_sha.take() {
                    commits.push(Commit {
                        sha,
                        message: current_message.trim().to_string(),
                    });
                    current_message.clear();
                }
            } else if current_sha.is_none()
                && line.len() == 40
                && line.chars().all(|c| c.is_ascii_hexdigit())
            {
                current_sha = Some(line.to_string());
            } else {
                if !current_message.is_empty() {
                    current_message.push('\n');
                }
                current_message.push_str(line);
            }
        }

        // Handle last commit if no trailing --END--
        if let Some(sha) = current_sha {
            commits.push(Commit {
                sha,
                message: current_message.trim().to_string(),
            });
        }

        Ok(commits)
    }

    fn create_tag(&self, name: &str, message: &str) -> Result<(), ReleaseError> {
        self.git(&["tag", "-a", name, "-m", message])?;
        Ok(())
    }

    fn push_tag(&self, name: &str) -> Result<(), ReleaseError> {
        self.git(&["push", "origin", name])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ssh_remote() {
        let (owner, repo) = parse_owner_repo("git@github.com:urmzd/semantic-release.git").unwrap();
        assert_eq!(owner, "urmzd");
        assert_eq!(repo, "semantic-release");
    }

    #[test]
    fn parse_https_remote() {
        let (owner, repo) =
            parse_owner_repo("https://github.com/urmzd/semantic-release.git").unwrap();
        assert_eq!(owner, "urmzd");
        assert_eq!(repo, "semantic-release");
    }

    #[test]
    fn parse_https_no_git_suffix() {
        let (owner, repo) = parse_owner_repo("https://github.com/urmzd/semantic-release").unwrap();
        assert_eq!(owner, "urmzd");
        assert_eq!(repo, "semantic-release");
    }
}
