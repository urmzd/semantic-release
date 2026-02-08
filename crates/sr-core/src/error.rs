use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReleaseError {
    #[error("no commits found since last release")]
    NoCommits,

    #[error("no releasable commits found (no feat/fix/breaking changes)")]
    NoBump,

    #[error("configuration error: {0}")]
    Config(String),

    #[error("git error: {0}")]
    Git(String),

    #[error("vcs provider error: {0}")]
    Vcs(String),

    #[error("hook failed: {command}")]
    Hook { command: String },

    #[error("changelog error: {0}")]
    Changelog(String),

    #[error("version file error: {0}")]
    VersionBump(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

// Keep anyhow available for conversions even though it's pulled transitively through thiserror.
// sr-core re-exports it so downstream crates don't need a direct dep.
pub use anyhow;
