use std::collections::{BTreeMap, BTreeSet};

use crate::commit::{CommitType, ConventionalCommit};
use crate::error::ReleaseError;

/// A single changelog entry representing a release.
#[derive(Debug, Clone)]
pub struct ChangelogEntry {
    pub version: String,
    pub date: String,
    pub commits: Vec<ConventionalCommit>,
    pub compare_url: Option<String>,
    pub repo_url: Option<String>,
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
    misc_section: String,
}

impl DefaultChangelogFormatter {
    pub fn new(
        template: Option<String>,
        types: Vec<CommitType>,
        breaking_section: String,
        misc_section: String,
    ) -> Self {
        Self {
            _template: template,
            types,
            breaking_section,
            misc_section,
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

        // Set of type names that have an explicit mapping (section or no-section).
        let known_types: BTreeSet<&str> = self.types.iter().map(|t| t.name.as_str()).collect();

        for entry in entries {
            output.push_str(&format!("## {} ({})\n", entry.version, entry.date));

            // 1. Breaking changes section (at the top).
            let breaking: Vec<_> = entry.commits.iter().filter(|c| c.breaking).collect();
            if !breaking.is_empty() {
                output.push_str(&format!("\n### {}\n\n", self.breaking_section));
                for commit in &breaking {
                    format_commit_line(&mut output, commit, entry.repo_url.as_deref());
                }
            }

            // 2. Type sections (Features, Bug Fixes, Performance, Documentation, etc.)
            for section_name in &seen_sections {
                let commits_in_section: Vec<_> = entry
                    .commits
                    .iter()
                    .filter(|c| {
                        !c.breaking
                            && section_map
                                .get(c.r#type.as_str())
                                .is_some_and(|s| s == section_name)
                    })
                    .collect();

                if !commits_in_section.is_empty() {
                    output.push_str(&format!("\n### {section_name}\n\n"));
                    for commit in &commits_in_section {
                        format_commit_line(&mut output, commit, entry.repo_url.as_deref());
                    }
                }
            }

            // 3. Miscellaneous catch-all (commits with no section mapping, excluding breaking).
            let misc: Vec<_> = entry
                .commits
                .iter()
                .filter(|c| {
                    !c.breaking
                        && !section_map.contains_key(c.r#type.as_str())
                        && known_types.contains(c.r#type.as_str())
                })
                .collect();
            if !misc.is_empty() {
                output.push_str(&format!("\n### {}\n\n", self.misc_section));
                for commit in &misc {
                    format_commit_line(&mut output, commit, entry.repo_url.as_deref());
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

fn format_commit_line(output: &mut String, commit: &ConventionalCommit, repo_url: Option<&str>) {
    let short_sha = &commit.sha[..7.min(commit.sha.len())];
    let sha_display = match repo_url {
        Some(url) => format!("[{short_sha}]({url}/commit/{})", commit.sha),
        None => short_sha.to_string(),
    };
    if let Some(scope) = &commit.scope {
        output.push_str(&format!(
            "- **{scope}**: {} ({sha_display})\n",
            commit.description
        ));
    } else {
        output.push_str(&format!("- {} ({sha_display})\n", commit.description));
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
            repo_url: None,
        }
    }

    fn format(entries: &[ChangelogEntry]) -> String {
        DefaultChangelogFormatter::new(
            None,
            default_commit_types(),
            "Breaking Changes".into(),
            "Miscellaneous".into(),
        )
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

    #[test]
    fn format_with_commit_links() {
        let mut e = entry(vec![make_commit("feat", "add button", None, false)], None);
        e.repo_url = Some("https://github.com/o/r".into());
        let out = format(&[e]);
        assert!(out.contains("[abc1234](https://github.com/o/r/commit/abc1234def5678)"));
    }

    #[test]
    fn format_breaking_at_top() {
        let commits = vec![
            make_commit("feat", "add button", None, false),
            make_commit("feat", "breaking thing", None, true),
        ];
        let out = format(&[entry(commits, None)]);
        let breaking_pos = out.find("### Breaking Changes").unwrap();
        let features_pos = out.find("### Features").unwrap();
        assert!(
            breaking_pos < features_pos,
            "Breaking Changes should appear before Features"
        );
    }

    #[test]
    fn format_misc_catch_all() {
        let commits = vec![
            make_commit("feat", "add button", None, false),
            make_commit("chore", "tidy up", None, false),
            make_commit("ci", "fix pipeline", None, false),
        ];
        let out = format(&[entry(commits, None)]);
        assert!(out.contains("### Miscellaneous"));
        assert!(out.contains("tidy up"));
        assert!(out.contains("fix pipeline"));
    }

    #[test]
    fn format_breaking_excluded_from_type_sections() {
        let commits = vec![
            make_commit("feat", "normal feature", None, false),
            make_commit("feat", "breaking feature", None, true),
        ];
        let out = format(&[entry(commits, None)]);
        // The breaking commit should be in Breaking Changes, not in Features
        let features_section_start = out.find("### Features").unwrap();
        let features_section_end = out[features_section_start..]
            .find("\n### ")
            .map(|p| features_section_start + p)
            .unwrap_or(out.len());
        let features_section = &out[features_section_start..features_section_end];
        assert!(features_section.contains("normal feature"));
        assert!(!features_section.contains("breaking feature"));
    }

    #[test]
    fn format_new_type_sections() {
        let commits = vec![
            make_commit("perf", "speed up query", None, false),
            make_commit("docs", "update readme", None, false),
            make_commit("refactor", "clean up code", None, false),
            make_commit("revert", "undo change", None, false),
        ];
        let out = format(&[entry(commits, None)]);
        assert!(out.contains("### Performance"));
        assert!(out.contains("### Documentation"));
        assert!(out.contains("### Refactoring"));
        assert!(out.contains("### Reverts"));
    }
}
