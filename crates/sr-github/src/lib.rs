use sr_core::error::ReleaseError;
use sr_core::release::VcsProvider;

/// GitHub implementation of the VcsProvider trait using the GitHub REST API.
pub struct GitHubProvider {
    owner: String,
    repo: String,
    hostname: String,
    token: String,
}

#[derive(serde::Deserialize)]
struct ReleaseResponse {
    id: u64,
    html_url: String,
    upload_url: String,
}

impl GitHubProvider {
    pub fn new(owner: String, repo: String, hostname: String, token: String) -> Self {
        Self {
            owner,
            repo,
            hostname,
            token,
        }
    }

    fn base_url(&self) -> String {
        format!("https://{}/{}/{}", self.hostname, self.owner, self.repo)
    }

    fn api_url(&self) -> String {
        if self.hostname == "github.com" {
            "https://api.github.com".to_string()
        } else {
            format!("https://{}/api/v3", self.hostname)
        }
    }

    fn agent(&self) -> ureq::Agent {
        ureq::Agent::new_with_config(
            ureq::config::Config::builder()
                .https_only(true)
                .build(),
        )
    }

    fn get_release_by_tag(&self, tag: &str) -> Result<ReleaseResponse, ReleaseError> {
        let url = format!(
            "{}/repos/{}/{}/releases/tags/{tag}",
            self.api_url(),
            self.owner,
            self.repo
        );
        let resp = self
            .agent()
            .get(&url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "sr-github")
            .call()
            .map_err(|e| ReleaseError::Vcs(format!("GitHub API GET {url}: {e}")))?;
        let release: ReleaseResponse = resp
            .into_body()
            .read_json()
            .map_err(|e| ReleaseError::Vcs(format!("failed to parse release response: {e}")))?;
        Ok(release)
    }
}

impl VcsProvider for GitHubProvider {
    fn create_release(
        &self,
        tag: &str,
        name: &str,
        body: &str,
        prerelease: bool,
    ) -> Result<String, ReleaseError> {
        let url = format!(
            "{}/repos/{}/{}/releases",
            self.api_url(),
            self.owner,
            self.repo
        );
        let payload = serde_json::json!({
            "tag_name": tag,
            "name": name,
            "body": body,
            "prerelease": prerelease,
        });

        let resp = self
            .agent()
            .post(&url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "sr-github")
            .send_json(&payload)
            .map_err(|e| ReleaseError::Vcs(format!("GitHub API POST {url}: {e}")))?;

        let release: ReleaseResponse = resp
            .into_body()
            .read_json()
            .map_err(|e| ReleaseError::Vcs(format!("failed to parse release response: {e}")))?;

        Ok(release.html_url)
    }

    fn compare_url(&self, base: &str, head: &str) -> Result<String, ReleaseError> {
        Ok(format!("{}/compare/{base}...{head}", self.base_url()))
    }

    fn release_exists(&self, tag: &str) -> Result<bool, ReleaseError> {
        let url = format!(
            "{}/repos/{}/{}/releases/tags/{tag}",
            self.api_url(),
            self.owner,
            self.repo
        );
        match self
            .agent()
            .get(&url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "sr-github")
            .call()
        {
            Ok(_) => Ok(true),
            Err(ureq::Error::StatusCode(404)) => Ok(false),
            Err(e) => Err(ReleaseError::Vcs(format!("GitHub API GET {url}: {e}"))),
        }
    }

    fn repo_url(&self) -> Option<String> {
        Some(self.base_url())
    }

    fn delete_release(&self, tag: &str) -> Result<(), ReleaseError> {
        let release = self.get_release_by_tag(tag)?;
        let url = format!(
            "{}/repos/{}/{}/releases/{}",
            self.api_url(),
            self.owner,
            self.repo,
            release.id
        );
        self.agent()
            .delete(&url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "sr-github")
            .call()
            .map_err(|e| ReleaseError::Vcs(format!("GitHub API DELETE {url}: {e}")))?;
        Ok(())
    }

    fn upload_assets(&self, tag: &str, files: &[&str]) -> Result<(), ReleaseError> {
        let release = self.get_release_by_tag(tag)?;
        // The upload_url from the API looks like:
        //   https://uploads.github.com/repos/owner/repo/releases/123/assets{?name,label}
        // Strip the {?name,label} template suffix.
        let upload_base = release
            .upload_url
            .split('{')
            .next()
            .unwrap_or(&release.upload_url);

        for file_path in files {
            let path = std::path::Path::new(file_path);
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| ReleaseError::Vcs(format!("invalid file path: {file_path}")))?;

            let data = std::fs::read(path)
                .map_err(|e| ReleaseError::Vcs(format!("failed to read asset {file_path}: {e}")))?;

            let url = format!("{upload_base}?name={file_name}");
            self.agent()
                .post(&url)
                .header("Authorization", &format!("Bearer {}", self.token))
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .header("User-Agent", "sr-github")
                .header("Content-Type", "application/octet-stream")
                .send(&data[..])
                .map_err(|e| {
                    ReleaseError::Vcs(format!("GitHub API upload asset {file_name}: {e}"))
                })?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn github_com_provider() -> GitHubProvider {
        GitHubProvider::new(
            "urmzd".into(),
            "semantic-release".into(),
            "github.com".into(),
            "test-token".into(),
        )
    }

    fn ghes_provider() -> GitHubProvider {
        GitHubProvider::new(
            "org".into(),
            "repo".into(),
            "ghes.example.com".into(),
            "test-token".into(),
        )
    }

    #[test]
    fn test_api_url_github_com() {
        assert_eq!(github_com_provider().api_url(), "https://api.github.com");
    }

    #[test]
    fn test_api_url_ghes() {
        assert_eq!(
            ghes_provider().api_url(),
            "https://ghes.example.com/api/v3"
        );
    }

    #[test]
    fn test_base_url() {
        assert_eq!(
            github_com_provider().base_url(),
            "https://github.com/urmzd/semantic-release"
        );
        assert_eq!(
            ghes_provider().base_url(),
            "https://ghes.example.com/org/repo"
        );
    }

    #[test]
    fn test_compare_url() {
        let p = github_com_provider();
        assert_eq!(
            p.compare_url("v0.9.0", "v1.0.0").unwrap(),
            "https://github.com/urmzd/semantic-release/compare/v0.9.0...v1.0.0"
        );
    }

    #[test]
    fn test_repo_url() {
        assert_eq!(
            github_com_provider().repo_url().unwrap(),
            "https://github.com/urmzd/semantic-release"
        );
    }
}
