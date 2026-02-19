use std::path::{Path, PathBuf};
use std::process::Command;

use base64::Engine;
use semver::Version;
use sr_core::commit::Commit;
use sr_core::error::ReleaseError;
use sr_core::git::{GitRepository, TagInfo};

/// Git repository implementation backed by native `git` CLI commands.
pub struct NativeGitRepository {
    path: PathBuf,
    http_auth: Option<(String, String)>, // (hostname, token)
}

impl NativeGitRepository {
    pub fn open(path: &Path) -> Result<Self, ReleaseError> {
        let repo = Self {
            path: path.to_path_buf(),
            http_auth: None,
        };
        // Validate this is a git repo
        repo.git(&["rev-parse", "--git-dir"])?;
        Ok(repo)
    }

    /// Enable HTTP Basic auth for git commands targeting the given hostname.
    ///
    /// Uses the same `http.extraheader` mechanism as `actions/checkout`,
    /// scoped to `https://{hostname}/` to prevent token leakage.
    pub fn with_http_auth(mut self, hostname: String, token: String) -> Self {
        self.http_auth = Some((hostname, token));
        self
    }

    fn git(&self, args: &[&str]) -> Result<String, ReleaseError> {
        let mut cmd = Command::new("git");
        // Prevent git from ever blocking on interactive credential prompts.
        // This makes unauthenticated operations fail fast instead of hanging.
        cmd.env("GIT_TERMINAL_PROMPT", "0");
        cmd.arg("-C").arg(&self.path);

        // Inject HTTP Basic auth header scoped to the target hostname.
        // First clear any existing extraheader (e.g. from actions/checkout) to
        // avoid sending duplicate Authorization headers, then set ours.
        if let Some((hostname, token)) = &self.http_auth {
            let credentials = format!("x-access-token:{token}");
            let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
            let config_key = format!("http.https://{hostname}/.extraheader");
            let config_val = format!("AUTHORIZATION: basic {encoded}");
            cmd.args(["-c", &format!("{config_key}=")]);
            cmd.args(["-c", &format!("{config_key}={config_val}")]);
        }

        let output = cmd
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

    /// Parse (hostname, owner, repo) from a git remote URL.
    pub fn parse_remote_full(&self) -> Result<(String, String, String), ReleaseError> {
        let url = self.git(&["remote", "get-url", "origin"])?;
        parse_remote_url(&url)
    }
}

/// Extract (hostname, owner, repo) from a git remote URL.
/// Supports SSH (git@hostname:owner/repo.git) and HTTPS (https://hostname/owner/repo.git).
pub fn parse_remote_url(url: &str) -> Result<(String, String, String), ReleaseError> {
    let trimmed = url.trim_end_matches(".git");

    // Try HTTPS/HTTP first: https://hostname/owner/repo
    if let Some(rest) = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
    {
        let (hostname, path) = rest
            .split_once('/')
            .ok_or_else(|| ReleaseError::Git(format!("cannot parse remote URL: {url}")))?;
        let (owner, repo) = path
            .split_once('/')
            .ok_or_else(|| ReleaseError::Git(format!("cannot parse owner/repo from: {url}")))?;
        return Ok((hostname.to_string(), owner.to_string(), repo.to_string()));
    }

    // SSH style: git@hostname:owner/repo
    if let Some((host_part, path)) = trimmed.split_once(':') {
        let hostname = host_part.rsplit('@').next().unwrap_or(host_part);
        let (owner, repo) = path
            .split_once('/')
            .ok_or_else(|| ReleaseError::Git(format!("cannot parse owner/repo from: {url}")))?;
        return Ok((hostname.to_string(), owner.to_string(), repo.to_string()));
    }

    Err(ReleaseError::Git(format!("cannot parse remote URL: {url}")))
}

/// Extract owner/repo from a git remote URL (convenience wrapper).
pub fn parse_owner_repo(url: &str) -> Result<(String, String), ReleaseError> {
    let (_, owner, repo) = parse_remote_url(url)?;
    Ok((owner, repo))
}

/// Parse the output of `git log --format=%H%n%B%n--END--` into commits.
fn parse_commit_log(output: &str) -> Vec<Commit> {
    if output.is_empty() {
        return Vec::new();
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

    commits
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
        Ok(parse_commit_log(&output))
    }

    fn create_tag(&self, name: &str, message: &str) -> Result<(), ReleaseError> {
        self.git(&["tag", "-a", name, "-m", message])?;
        Ok(())
    }

    fn push_tag(&self, name: &str) -> Result<(), ReleaseError> {
        self.git(&["push", "origin", name])?;
        Ok(())
    }

    fn stage_and_commit(&self, paths: &[&str], message: &str) -> Result<bool, ReleaseError> {
        let mut args = vec!["add", "--"];
        args.extend(paths);
        self.git(&args)?;

        let status = self.git(&["status", "--porcelain"]);
        match status {
            Ok(s) if s.is_empty() => Ok(false),
            _ => {
                self.git(&["commit", "-m", message])?;
                Ok(true)
            }
        }
    }

    fn push(&self) -> Result<(), ReleaseError> {
        self.git(&["push", "origin", "HEAD"])?;
        Ok(())
    }

    fn tag_exists(&self, name: &str) -> Result<bool, ReleaseError> {
        match self.git(&["rev-parse", "--verify", &format!("refs/tags/{name}")]) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn remote_tag_exists(&self, name: &str) -> Result<bool, ReleaseError> {
        let output = self.git(&["ls-remote", "--tags", "origin", name])?;
        Ok(!output.is_empty())
    }

    fn all_tags(&self, prefix: &str) -> Result<Vec<TagInfo>, ReleaseError> {
        let pattern = format!("{prefix}*");
        let result = self.git(&["tag", "--list", &pattern, "--sort=v:refname"]);

        let tags_output = match result {
            Ok(output) if output.is_empty() => return Ok(Vec::new()),
            Ok(output) => output,
            Err(_) => return Ok(Vec::new()),
        };

        let mut tags = Vec::new();
        for line in tags_output.lines() {
            let tag_name = line.trim();
            if tag_name.is_empty() {
                continue;
            }
            let version_str = tag_name.strip_prefix(prefix).unwrap_or(tag_name);
            let version = match Version::parse(version_str) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let sha = self.git(&["rev-list", "-1", tag_name])?;
            tags.push(TagInfo {
                name: tag_name.to_string(),
                version,
                sha,
            });
        }

        Ok(tags)
    }

    fn commits_between(&self, from: Option<&str>, to: &str) -> Result<Vec<Commit>, ReleaseError> {
        let range = match from {
            Some(sha) => format!("{sha}..{to}"),
            None => to.to_string(),
        };

        let output = self.git(&["log", "--format=%H%n%B%n--END--", &range])?;
        Ok(parse_commit_log(&output))
    }

    fn tag_date(&self, tag_name: &str) -> Result<String, ReleaseError> {
        let date = self.git(&["log", "-1", "--format=%cd", "--date=short", tag_name])?;
        Ok(date)
    }

    fn force_create_tag(&self, name: &str, message: &str) -> Result<(), ReleaseError> {
        self.git(&["tag", "-fa", name, "-m", message])?;
        Ok(())
    }

    fn force_push_tag(&self, name: &str) -> Result<(), ReleaseError> {
        self.git(&["push", "origin", name, "--force"])?;
        Ok(())
    }

    fn head_sha(&self) -> Result<String, ReleaseError> {
        self.git(&["rev-parse", "HEAD"])
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

    #[test]
    fn parse_remote_url_github_https() {
        let (host, owner, repo) =
            parse_remote_url("https://github.com/urmzd/semantic-release.git").unwrap();
        assert_eq!(host, "github.com");
        assert_eq!(owner, "urmzd");
        assert_eq!(repo, "semantic-release");
    }

    #[test]
    fn parse_remote_url_github_ssh() {
        let (host, owner, repo) =
            parse_remote_url("git@github.com:urmzd/semantic-release.git").unwrap();
        assert_eq!(host, "github.com");
        assert_eq!(owner, "urmzd");
        assert_eq!(repo, "semantic-release");
    }

    #[test]
    fn parse_remote_url_ghes_https() {
        let (host, owner, repo) =
            parse_remote_url("https://ghes.example.com/org/my-repo.git").unwrap();
        assert_eq!(host, "ghes.example.com");
        assert_eq!(owner, "org");
        assert_eq!(repo, "my-repo");
    }

    #[test]
    fn parse_remote_url_ghes_ssh() {
        let (host, owner, repo) = parse_remote_url("git@ghes.example.com:org/my-repo.git").unwrap();
        assert_eq!(host, "ghes.example.com");
        assert_eq!(owner, "org");
        assert_eq!(repo, "my-repo");
    }

    #[test]
    fn parse_remote_url_no_git_suffix() {
        let (host, owner, repo) =
            parse_remote_url("https://github.com/urmzd/semantic-release").unwrap();
        assert_eq!(host, "github.com");
        assert_eq!(owner, "urmzd");
        assert_eq!(repo, "semantic-release");
    }

    #[test]
    fn http_auth_header_encodes_correctly() {
        use base64::Engine;

        let token = "ghp_testtoken123";
        let credentials = format!("x-access-token:{token}");
        let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());

        // Round-trip: decode and verify
        let decoded_bytes = base64::engine::general_purpose::STANDARD
            .decode(&encoded)
            .expect("base64 should decode");
        let decoded = String::from_utf8(decoded_bytes).expect("should be valid utf-8");
        assert_eq!(decoded, "x-access-token:ghp_testtoken123");
    }

    #[test]
    fn http_auth_header_scoped_to_hostname() {
        let hostname = "ghes.example.com";
        let config_key = format!("http.https://{hostname}/.extraheader");
        assert_eq!(config_key, "http.https://ghes.example.com/.extraheader");

        // Verify github.com scoping
        let hostname = "github.com";
        let config_key = format!("http.https://{hostname}/.extraheader");
        assert_eq!(config_key, "http.https://github.com/.extraheader");
    }
}
