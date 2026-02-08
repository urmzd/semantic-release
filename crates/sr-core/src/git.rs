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
}
