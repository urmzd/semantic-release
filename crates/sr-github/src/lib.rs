use sr_core::error::ReleaseError;
use sr_core::release::VcsProvider;

/// GitHub implementation of the VcsProvider trait.
pub struct GitHubProvider {
    client: octocrab::Octocrab,
    owner: String,
    repo: String,
}

impl GitHubProvider {
    pub fn new(token: &str, owner: String, repo: String) -> Result<Self, ReleaseError> {
        let client = octocrab::Octocrab::builder()
            .personal_token(token.to_string())
            .build()
            .map_err(|e| ReleaseError::Vcs(e.to_string()))?;

        Ok(Self {
            client,
            owner,
            repo,
        })
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
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| ReleaseError::Vcs(format!("failed to create tokio runtime: {e}")))?;

        let release = rt
            .block_on(async {
                self.client
                    .repos(&self.owner, &self.repo)
                    .releases()
                    .create(tag)
                    .name(name)
                    .body(body)
                    .prerelease(prerelease)
                    .send()
                    .await
            })
            .map_err(|e| ReleaseError::Vcs(e.to_string()))?;

        Ok(release.html_url.to_string())
    }

    fn compare_url(&self, base: &str, head: &str) -> Result<String, ReleaseError> {
        Ok(format!(
            "https://github.com/{}/{}/compare/{base}...{head}",
            self.owner, self.repo,
        ))
    }
}
