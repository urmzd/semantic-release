use std::collections::BTreeMap;

use crate::commit::{CommitType, ConventionalCommit};
use crate::error::ReleaseError;

/// A single changelog entry representing a release.
#[derive(Debug, Clone)]
pub struct ChangelogEntry {
    pub version: String,
    pub date: String,
    pub commits: Vec<ConventionalCommit>,
    pub compare_url: Option<String>,
}

/// Formats changelog entries into a string representation.
pub trait ChangelogFormatter: Send + Sync {
    fn format(&self, entries: &[ChangelogEntry]) -> Result<String, ReleaseError>;
}

/// Default formatter that produces simple markdown output.
pub struct DefaultChangelogFormatter {
    _template: Option<String>,
    types: Vec<CommitType>,
    breaking_section: String,
}

impl DefaultChangelogFormatter {
    pub fn new(template: Option<String>, types: Vec<CommitType>, breaking_section: String) -> Self {
        Self {
            _template: template,
            types,
            breaking_section,
        }
    }
}

impl ChangelogFormatter for DefaultChangelogFormatter {
    fn format(&self, entries: &[ChangelogEntry]) -> Result<String, ReleaseError> {
        let mut output = String::new();

        // Build ordered list of unique sections, preserving definition order.
        let mut seen_sections = Vec::new();
        let mut section_map: BTreeMap<&str, &str> = BTreeMap::new();
        for ct in &self.types {
            if let Some(ref section) = ct.section {
                if !seen_sections.contains(&section.as_str()) {
                    seen_sections.push(section.as_str());
                }
                section_map.insert(&ct.name, section.as_str());
            }
        }

        for entry in entries {
            output.push_str(&format!("## {} ({})\n", entry.version, entry.date));

            // Group commits by section.
            for section_name in &seen_sections {
                let commits_in_section: Vec<_> = entry
                    .commits
                    .iter()
                    .filter(|c| {
                        section_map
                            .get(c.r#type.as_str())
                            .is_some_and(|s| s == section_name)
                    })
                    .collect();

                if !commits_in_section.is_empty() {
                    output.push_str(&format!("\n### {section_name}\n\n"));
                    for commit in &commits_in_section {
                        format_commit_line(&mut output, commit);
                    }
                }
            }

            // Breaking changes section.
            let breaking: Vec<_> = entry.commits.iter().filter(|c| c.breaking).collect();
            if !breaking.is_empty() {
                output.push_str(&format!("\n### {}\n\n", self.breaking_section));
                for commit in &breaking {
                    format_commit_line(&mut output, commit);
                }
            }

            if let Some(url) = &entry.compare_url {
                output.push_str(&format!("\n[Full Changelog]({url})\n"));
            }

            output.push('\n');
        }

        Ok(output.trim_end().to_string())
    }
}

fn format_commit_line(output: &mut String, commit: &ConventionalCommit) {
    let short_sha = &commit.sha[..7.min(commit.sha.len())];
    if let Some(scope) = &commit.scope {
        output.push_str(&format!(
            "- **{scope}**: {} ({short_sha})\n",
            commit.description
        ));
    } else {
        output.push_str(&format!("- {} ({short_sha})\n", commit.description));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commit::default_commit_types;

    fn make_commit(
        type_: &str,
        desc: &str,
        scope: Option<&str>,
        breaking: bool,
    ) -> ConventionalCommit {
        ConventionalCommit {
            sha: "abc1234def5678".into(),
            r#type: type_.into(),
            scope: scope.map(Into::into),
            description: desc.into(),
            body: None,
            breaking,
        }
    }

    fn entry(commits: Vec<ConventionalCommit>, compare_url: Option<&str>) -> ChangelogEntry {
        ChangelogEntry {
            version: "1.0.0".into(),
            date: "2025-01-01".into(),
            commits,
            compare_url: compare_url.map(Into::into),
        }
    }

    fn format(entries: &[ChangelogEntry]) -> String {
        DefaultChangelogFormatter::new(None, default_commit_types(), "Breaking Changes".into())
            .format(entries)
            .unwrap()
    }

    #[test]
    fn format_features_only() {
        let out = format(&[entry(
            vec![make_commit("feat", "add button", None, false)],
            None,
        )]);
        assert!(out.contains("## 1.0.0"));
        assert!(out.contains("### Features"));
        assert!(out.contains("add button"));
    }

    #[test]
    fn format_fixes_only() {
        let out = format(&[entry(
            vec![make_commit("fix", "null check", None, false)],
            None,
        )]);
        assert!(out.contains("### Bug Fixes"));
        assert!(out.contains("null check"));
    }

    #[test]
    fn format_breaking_changes() {
        let out = format(&[entry(
            vec![make_commit("feat", "new API", None, true)],
            None,
        )]);
        assert!(out.contains("### Breaking Changes"));
    }

    #[test]
    fn format_mixed_commits() {
        let commits = vec![
            make_commit("feat", "add button", None, false),
            make_commit("fix", "null check", None, false),
            make_commit("feat", "breaking thing", None, true),
        ];
        let out = format(&[entry(commits, None)]);
        assert!(out.contains("### Features"));
        assert!(out.contains("### Bug Fixes"));
        assert!(out.contains("### Breaking Changes"));
    }

    #[test]
    fn format_with_scope() {
        let out = format(&[entry(
            vec![make_commit("feat", "add flag", Some("cli"), false)],
            None,
        )]);
        assert!(out.contains("**cli**:"));
    }

    #[test]
    fn format_with_compare_url() {
        let out = format(&[entry(
            vec![make_commit("feat", "add button", None, false)],
            Some("https://github.com/o/r/compare/v0.1.0...v1.0.0"),
        )]);
        assert!(out.contains("[Full Changelog]"));
    }

    #[test]
    fn format_empty_entries() {
        let out = format(&[entry(vec![], None)]);
        assert!(!out.contains("### Features"));
        assert!(!out.contains("### Bug Fixes"));
        assert!(!out.contains("### Breaking Changes"));
    }
}
