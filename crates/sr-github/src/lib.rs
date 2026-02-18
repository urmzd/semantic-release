use std::process::Command;

use sr_core::error::ReleaseError;
use sr_core::release::VcsProvider;

/// GitHub implementation of the VcsProvider trait using the `gh` CLI.
pub struct GitHubProvider {
    owner: String,
    repo: String,
    hostname: String,
}

impl GitHubProvider {
    pub fn new(owner: String, repo: String, hostname: String) -> Self {
        Self {
            owner,
            repo,
            hostname,
        }
    }

    fn base_url(&self) -> String {
        format!("https://{}/{}/{}", self.hostname, self.owner, self.repo)
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
        let repo_slug = format!("{}/{}", self.owner, self.repo);

        let mut args = vec![
            "release", "create", tag, "--repo", &repo_slug, "--title", name, "--notes", body,
        ];

        if prerelease {
            args.push("--prerelease");
        }

        let output = Command::new("gh")
            .args(&args)
            .output()
            .map_err(|e| ReleaseError::Vcs(format!("failed to run gh: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ReleaseError::Vcs(format!(
                "gh release create failed: {stderr}"
            )));
        }

        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(url)
    }

    fn compare_url(&self, base: &str, head: &str) -> Result<String, ReleaseError> {
        Ok(format!("{}/compare/{base}...{head}", self.base_url()))
    }

    fn release_exists(&self, tag: &str) -> Result<bool, ReleaseError> {
        let repo_slug = format!("{}/{}", self.owner, self.repo);
        let output = Command::new("gh")
            .args(["release", "view", tag, "--repo", &repo_slug])
            .output()
            .map_err(|e| ReleaseError::Vcs(format!("failed to run gh: {e}")))?;
        Ok(output.status.success())
    }

    fn repo_url(&self) -> Option<String> {
        Some(self.base_url())
    }

    fn delete_release(&self, tag: &str) -> Result<(), ReleaseError> {
        let repo_slug = format!("{}/{}", self.owner, self.repo);
        let output = Command::new("gh")
            .args(["release", "delete", tag, "--repo", &repo_slug, "--yes"])
            .output()
            .map_err(|e| ReleaseError::Vcs(format!("failed to run gh: {e}")))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ReleaseError::Vcs(format!(
                "gh release delete failed: {stderr}"
            )));
        }
        Ok(())
    }

    fn upload_assets(&self, tag: &str, files: &[&str]) -> Result<(), ReleaseError> {
        let repo_slug = format!("{}/{}", self.owner, self.repo);
        let mut args = vec!["release", "upload", tag, "--repo", &repo_slug, "--clobber"];
        args.extend(files);

        let output = Command::new("gh")
            .args(&args)
            .output()
            .map_err(|e| ReleaseError::Vcs(format!("failed to run gh: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ReleaseError::Vcs(format!(
                "gh release upload failed: {stderr}"
            )));
        }

        Ok(())
    }
}
