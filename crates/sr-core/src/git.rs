use semver::Version;

use crate::commit::Commit;
use crate::error::ReleaseError;

/// Information about a git tag.
#[derive(Debug, Clone)]
pub struct TagInfo {
    pub name: String,
    pub version: Version,
    pub sha: String,
}

/// Abstraction over git operations.
pub trait GitRepository: Send + Sync {
    /// Find the latest semver tag matching the configured prefix.
    fn latest_tag(&self, prefix: &str) -> Result<Option<TagInfo>, ReleaseError>;

    /// List commits between a starting point (exclusive) and HEAD (inclusive).
    /// If `from` is `None`, returns all commits reachable from HEAD.
    fn commits_since(&self, from: Option<&str>) -> Result<Vec<Commit>, ReleaseError>;

    /// Create an annotated tag at HEAD.
    fn create_tag(&self, name: &str, message: &str) -> Result<(), ReleaseError>;

    /// Push a tag to the remote.
    fn push_tag(&self, name: &str) -> Result<(), ReleaseError>;

    /// Stage files and commit. Returns Ok(false) if nothing to commit.
    fn stage_and_commit(&self, paths: &[&str], message: &str) -> Result<bool, ReleaseError>;

    /// Push current branch to origin.
    fn push(&self) -> Result<(), ReleaseError>;

    /// Check if a tag exists locally.
    fn tag_exists(&self, name: &str) -> Result<bool, ReleaseError>;

    /// Check if a tag exists on the remote.
    fn remote_tag_exists(&self, name: &str) -> Result<bool, ReleaseError>;

    /// List all semver tags matching prefix, sorted by version ascending.
    fn all_tags(&self, prefix: &str) -> Result<Vec<TagInfo>, ReleaseError>;

    /// List commits between two refs (exclusive `from`, inclusive `to`).
    /// If `from` is None, returns all commits reachable from `to`.
    fn commits_between(&self, from: Option<&str>, to: &str) -> Result<Vec<Commit>, ReleaseError>;

    /// Get the date (YYYY-MM-DD) of the commit a tag points to.
    fn tag_date(&self, tag_name: &str) -> Result<String, ReleaseError>;

    /// Force-create an annotated tag at HEAD, overwriting if it already exists.
    fn force_create_tag(&self, name: &str, message: &str) -> Result<(), ReleaseError>;

    /// Force-push a tag to the remote, overwriting the remote tag if it exists.
    fn force_push_tag(&self, name: &str) -> Result<(), ReleaseError>;
}
