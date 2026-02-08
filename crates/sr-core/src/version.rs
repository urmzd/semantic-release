use std::fmt;

use semver::Version;

use crate::commit::{CommitClassifier, ConventionalCommit};

/// The kind of version bump to apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BumpLevel {
    Patch,
    Minor,
    Major,
}

impl fmt::Display for BumpLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BumpLevel::Patch => write!(f, "patch"),
            BumpLevel::Minor => write!(f, "minor"),
            BumpLevel::Major => write!(f, "major"),
        }
    }
}

/// Determine the highest bump level from a set of conventional commits.
///
/// Returns `None` if no commits warrant a release.
pub fn determine_bump(
    commits: &[ConventionalCommit],
    classifier: &dyn CommitClassifier,
) -> Option<BumpLevel> {
    commits
        .iter()
        .filter_map(|c| classifier.bump_level(&c.r#type, c.breaking))
        .max()
}

/// Apply a bump level to a version, returning the new version.
pub fn apply_bump(version: &Version, bump: BumpLevel) -> Version {
    match bump {
        BumpLevel::Major => Version::new(version.major + 1, 0, 0),
        BumpLevel::Minor => Version::new(version.major, version.minor + 1, 0),
        BumpLevel::Patch => Version::new(version.major, version.minor, version.patch + 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commit::{ConventionalCommit, DefaultCommitClassifier};

    fn commit(type_: &str, breaking: bool) -> ConventionalCommit {
        ConventionalCommit {
            sha: "abc1234".into(),
            r#type: type_.into(),
            scope: None,
            description: "test".into(),
            body: None,
            breaking,
            author: None,
        }
    }

    fn classifier() -> DefaultCommitClassifier {
        DefaultCommitClassifier::default()
    }

    #[test]
    fn patch_bump() {
        let v = Version::new(1, 2, 3);
        assert_eq!(apply_bump(&v, BumpLevel::Patch), Version::new(1, 2, 4));
    }

    #[test]
    fn minor_bump_resets_patch() {
        let v = Version::new(1, 2, 3);
        assert_eq!(apply_bump(&v, BumpLevel::Minor), Version::new(1, 3, 0));
    }

    #[test]
    fn major_bump_resets_minor_and_patch() {
        let v = Version::new(1, 2, 3);
        assert_eq!(apply_bump(&v, BumpLevel::Major), Version::new(2, 0, 0));
    }

    #[test]
    fn no_commits_returns_none() {
        assert_eq!(determine_bump(&[], &classifier()), None);
    }

    #[test]
    fn non_releasable_types_return_none() {
        let commits = vec![
            commit("chore", false),
            commit("docs", false),
            commit("ci", false),
        ];
        assert_eq!(determine_bump(&commits, &classifier()), None);
    }

    #[test]
    fn single_fix_returns_patch() {
        assert_eq!(
            determine_bump(&[commit("fix", false)], &classifier()),
            Some(BumpLevel::Patch)
        );
    }

    #[test]
    fn single_feat_returns_minor() {
        assert_eq!(
            determine_bump(&[commit("feat", false)], &classifier()),
            Some(BumpLevel::Minor)
        );
    }

    #[test]
    fn perf_returns_patch() {
        assert_eq!(
            determine_bump(&[commit("perf", false)], &classifier()),
            Some(BumpLevel::Patch)
        );
    }

    #[test]
    fn breaking_returns_major() {
        assert_eq!(
            determine_bump(&[commit("feat", true)], &classifier()),
            Some(BumpLevel::Major)
        );
    }

    #[test]
    fn highest_bump_wins() {
        let commits = vec![
            commit("fix", false),
            commit("feat", false),
            commit("feat", true),
        ];
        assert_eq!(
            determine_bump(&commits, &classifier()),
            Some(BumpLevel::Major)
        );
    }

    #[test]
    fn feat_beats_fix() {
        let commits = vec![commit("fix", false), commit("feat", false)];
        assert_eq!(
            determine_bump(&commits, &classifier()),
            Some(BumpLevel::Minor)
        );
    }
}
