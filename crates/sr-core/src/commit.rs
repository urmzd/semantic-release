use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::ReleaseError;
use crate::version::BumpLevel;

/// A raw commit as read from git history.
#[derive(Debug, Clone)]
pub struct Commit {
    pub sha: String,
    pub message: String,
}

/// A commit parsed according to the Conventional Commits specification.
#[derive(Debug, Clone, Serialize)]
pub struct ConventionalCommit {
    pub sha: String,
    pub r#type: String,
    pub scope: Option<String>,
    pub description: String,
    pub body: Option<String>,
    pub breaking: bool,
}

/// Describes a recognised commit type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommitType {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bump: Option<BumpLevel>,
    /// Changelog section heading (e.g. "Features"). None = exclude from changelog.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
}

/// Single source of truth for commit type classification.
pub trait CommitClassifier: Send + Sync {
    fn types(&self) -> &[CommitType];

    /// Commit message regex with named groups: type, scope, breaking, description.
    fn pattern(&self) -> &str;

    fn bump_level(&self, type_name: &str, breaking: bool) -> Option<BumpLevel> {
        if breaking {
            return Some(BumpLevel::Major);
        }
        self.types().iter().find(|t| t.name == type_name)?.bump
    }

    fn changelog_section(&self, type_name: &str) -> Option<&str> {
        self.types()
            .iter()
            .find(|t| t.name == type_name)?
            .section
            .as_deref()
    }

    fn is_allowed(&self, type_name: &str) -> bool {
        self.types().iter().any(|t| t.name == type_name)
    }
}

/// Default conventional commits pattern.
/// Named groups: type, scope (optional), breaking (optional `!`), description.
pub const DEFAULT_COMMIT_PATTERN: &str =
    r"^(?P<type>\w+)(?:\((?P<scope>[^)]+)\))?(?P<breaking>!)?:\s+(?P<description>.+)";

pub struct DefaultCommitClassifier {
    types: Vec<CommitType>,
    pattern: String,
}

impl DefaultCommitClassifier {
    pub fn new(types: Vec<CommitType>, pattern: String) -> Self {
        Self { types, pattern }
    }
}

impl Default for DefaultCommitClassifier {
    fn default() -> Self {
        Self::new(default_commit_types(), DEFAULT_COMMIT_PATTERN.into())
    }
}

impl CommitClassifier for DefaultCommitClassifier {
    fn types(&self) -> &[CommitType] {
        &self.types
    }
    fn pattern(&self) -> &str {
        &self.pattern
    }
}

pub fn default_commit_types() -> Vec<CommitType> {
    vec![
        CommitType {
            name: "feat".into(),
            bump: Some(BumpLevel::Minor),
            section: Some("Features".into()),
        },
        CommitType {
            name: "fix".into(),
            bump: Some(BumpLevel::Patch),
            section: Some("Bug Fixes".into()),
        },
        CommitType {
            name: "perf".into(),
            bump: Some(BumpLevel::Patch),
            section: Some("Performance".into()),
        },
        CommitType {
            name: "docs".into(),
            bump: None,
            section: Some("Documentation".into()),
        },
        CommitType {
            name: "refactor".into(),
            bump: None,
            section: Some("Refactoring".into()),
        },
        CommitType {
            name: "revert".into(),
            bump: None,
            section: Some("Reverts".into()),
        },
        CommitType {
            name: "chore".into(),
            bump: None,
            section: None,
        },
        CommitType {
            name: "ci".into(),
            bump: None,
            section: None,
        },
        CommitType {
            name: "test".into(),
            bump: None,
            section: None,
        },
        CommitType {
            name: "build".into(),
            bump: None,
            section: None,
        },
        CommitType {
            name: "style".into(),
            bump: None,
            section: None,
        },
    ]
}

/// Parses raw commits into conventional commits.
pub trait CommitParser: Send + Sync {
    fn parse(&self, commit: &Commit) -> Result<ConventionalCommit, ReleaseError>;
}

/// Default parser using the built-in `DEFAULT_COMMIT_PATTERN` regex.
pub struct DefaultCommitParser;

impl CommitParser for DefaultCommitParser {
    fn parse(&self, commit: &Commit) -> Result<ConventionalCommit, ReleaseError> {
        let re =
            Regex::new(DEFAULT_COMMIT_PATTERN).map_err(|e| ReleaseError::Config(e.to_string()))?;

        let caps = re.captures(&commit.message).ok_or_else(|| {
            ReleaseError::Config(format!("not a conventional commit: {}", commit.message))
        })?;

        let r#type = caps.name("type").unwrap().as_str().to_string();
        let scope = caps.name("scope").map(|m| m.as_str().to_string());
        let breaking = caps.name("breaking").is_some();
        let description = caps.name("description").unwrap().as_str().to_string();

        let body = commit
            .message
            .split_once("\n\n")
            .map(|x| x.1)
            .map(|b| b.to_string());

        Ok(ConventionalCommit {
            sha: commit.sha.clone(),
            r#type,
            scope,
            description,
            body,
            breaking,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(message: &str) -> Commit {
        Commit {
            sha: "abc1234".into(),
            message: message.into(),
        }
    }

    #[test]
    fn parse_simple_feat() {
        let result = DefaultCommitParser.parse(&raw("feat: add button")).unwrap();
        assert_eq!(result.r#type, "feat");
        assert_eq!(result.description, "add button");
        assert_eq!(result.scope, None);
        assert!(!result.breaking);
    }

    #[test]
    fn parse_scoped_fix() {
        let result = DefaultCommitParser
            .parse(&raw("fix(core): null check"))
            .unwrap();
        assert_eq!(result.r#type, "fix");
        assert_eq!(result.scope.as_deref(), Some("core"));
    }

    #[test]
    fn parse_breaking_bang() {
        let result = DefaultCommitParser.parse(&raw("feat!: new API")).unwrap();
        assert!(result.breaking);
    }

    #[test]
    fn parse_with_body() {
        let result = DefaultCommitParser
            .parse(&raw("fix: x\n\ndetails"))
            .unwrap();
        assert_eq!(result.body.as_deref(), Some("details"));
    }

    #[test]
    fn parse_invalid_message() {
        let result = DefaultCommitParser.parse(&raw("not conventional"));
        assert!(result.is_err());
    }

    // --- CommitClassifier tests ---

    #[test]
    fn classifier_bump_level_feat() {
        let c = DefaultCommitClassifier::default();
        assert_eq!(c.bump_level("feat", false), Some(BumpLevel::Minor));
    }

    #[test]
    fn classifier_bump_level_fix() {
        let c = DefaultCommitClassifier::default();
        assert_eq!(c.bump_level("fix", false), Some(BumpLevel::Patch));
    }

    #[test]
    fn classifier_bump_level_breaking_overrides() {
        let c = DefaultCommitClassifier::default();
        assert_eq!(c.bump_level("fix", true), Some(BumpLevel::Major));
        assert_eq!(c.bump_level("chore", true), Some(BumpLevel::Major));
    }

    #[test]
    fn classifier_bump_level_no_bump_type() {
        let c = DefaultCommitClassifier::default();
        assert_eq!(c.bump_level("chore", false), None);
        assert_eq!(c.bump_level("docs", false), None);
    }

    #[test]
    fn classifier_bump_level_unknown_type() {
        let c = DefaultCommitClassifier::default();
        assert_eq!(c.bump_level("unknown", false), None);
    }

    #[test]
    fn classifier_changelog_section() {
        let c = DefaultCommitClassifier::default();
        assert_eq!(c.changelog_section("feat"), Some("Features"));
        assert_eq!(c.changelog_section("fix"), Some("Bug Fixes"));
        assert_eq!(c.changelog_section("perf"), Some("Performance"));
        assert_eq!(c.changelog_section("docs"), Some("Documentation"));
        assert_eq!(c.changelog_section("refactor"), Some("Refactoring"));
        assert_eq!(c.changelog_section("revert"), Some("Reverts"));
        assert_eq!(c.changelog_section("chore"), None);
        assert_eq!(c.changelog_section("unknown"), None);
    }

    #[test]
    fn classifier_is_allowed() {
        let c = DefaultCommitClassifier::default();
        assert!(c.is_allowed("feat"));
        assert!(c.is_allowed("chore"));
        assert!(!c.is_allowed("unknown"));
    }

    #[test]
    fn classifier_pattern() {
        let c = DefaultCommitClassifier::default();
        assert_eq!(c.pattern(), DEFAULT_COMMIT_PATTERN);
    }

    #[test]
    fn default_commit_types_count() {
        let types = default_commit_types();
        assert_eq!(types.len(), 11);
    }

    #[test]
    fn commit_type_serialization_roundtrip() {
        let ct = CommitType {
            name: "feat".into(),
            bump: Some(BumpLevel::Minor),
            section: Some("Features".into()),
        };
        let yaml = serde_yaml_ng::to_string(&ct).unwrap();
        let parsed: CommitType = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(parsed, ct);
    }

    #[test]
    fn commit_type_no_bump_no_section_roundtrip() {
        let ct = CommitType {
            name: "chore".into(),
            bump: None,
            section: None,
        };
        let yaml = serde_yaml_ng::to_string(&ct).unwrap();
        assert!(!yaml.contains("bump"));
        assert!(!yaml.contains("section"));
        let parsed: CommitType = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(parsed, ct);
    }
}
