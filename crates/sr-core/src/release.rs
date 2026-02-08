use std::fs;
use std::path::Path;

use semver::Version;
use serde::Serialize;

use crate::changelog::{ChangelogEntry, ChangelogFormatter};
use crate::commit::{CommitParser, ConventionalCommit, DefaultCommitClassifier};
use crate::config::ReleaseConfig;
use crate::error::ReleaseError;
use crate::git::GitRepository;
use crate::hooks::{HookCommand, HookRunner};
use crate::version::{BumpLevel, apply_bump, determine_bump};

/// The computed plan for a release, before execution.
#[derive(Debug, Serialize)]
pub struct ReleasePlan {
    pub current_version: Option<Version>,
    pub next_version: Version,
    pub bump: BumpLevel,
    pub commits: Vec<ConventionalCommit>,
    pub tag_name: String,
}

/// Orchestrates the release flow.
pub trait ReleaseStrategy: Send + Sync {
    /// Plan the release without executing it.
    fn plan(&self) -> Result<ReleasePlan, ReleaseError>;

    /// Execute the release.
    fn execute(&self, plan: &ReleasePlan, dry_run: bool) -> Result<(), ReleaseError>;
}

/// Abstraction over a remote VCS provider (e.g. GitHub, GitLab).
pub trait VcsProvider: Send + Sync {
    /// Create a release on the remote VCS.
    fn create_release(
        &self,
        tag: &str,
        name: &str,
        body: &str,
        prerelease: bool,
    ) -> Result<String, ReleaseError>;

    /// Generate a compare URL between two refs.
    fn compare_url(&self, base: &str, head: &str) -> Result<String, ReleaseError>;
}

/// Concrete release strategy implementing the trunk-based release flow.
pub struct TrunkReleaseStrategy<G, V, C, F, H> {
    pub git: G,
    pub vcs: Option<V>,
    pub parser: C,
    pub formatter: F,
    pub hooks: H,
    pub config: ReleaseConfig,
}

fn to_hook_commands(commands: &[String]) -> Vec<HookCommand> {
    commands
        .iter()
        .map(|c| HookCommand { command: c.clone() })
        .collect()
}

impl<G, V, C, F, H> TrunkReleaseStrategy<G, V, C, F, H>
where
    G: GitRepository,
    V: VcsProvider,
    C: CommitParser,
    F: ChangelogFormatter,
    H: HookRunner,
{
    fn format_changelog(&self, plan: &ReleasePlan) -> Result<String, ReleaseError> {
        let today = today_string();
        let entry = ChangelogEntry {
            version: plan.next_version.to_string(),
            date: today,
            commits: plan.commits.clone(),
            compare_url: None,
        };
        self.formatter.format(&[entry])
    }
}

impl<G, V, C, F, H> ReleaseStrategy for TrunkReleaseStrategy<G, V, C, F, H>
where
    G: GitRepository,
    V: VcsProvider,
    C: CommitParser,
    F: ChangelogFormatter,
    H: HookRunner,
{
    fn plan(&self) -> Result<ReleasePlan, ReleaseError> {
        let tag_info = self.git.latest_tag(&self.config.tag_prefix)?;

        let (current_version, from_sha) = match &tag_info {
            Some(info) => (Some(info.version.clone()), Some(info.sha.as_str())),
            None => (None, None),
        };

        let raw_commits = self.git.commits_since(from_sha)?;
        if raw_commits.is_empty() {
            return Err(ReleaseError::NoCommits);
        }

        let conventional_commits: Vec<ConventionalCommit> = raw_commits
            .iter()
            .filter_map(|c| self.parser.parse(c).ok())
            .collect();

        let classifier = DefaultCommitClassifier::new(
            self.config.types.clone(),
            self.config.commit_pattern.clone(),
        );
        let bump =
            determine_bump(&conventional_commits, &classifier).ok_or(ReleaseError::NoBump)?;

        let base_version = current_version.clone().unwrap_or(Version::new(0, 0, 0));
        let next_version = apply_bump(&base_version, bump);
        let tag_name = format!("{}{next_version}", self.config.tag_prefix);

        Ok(ReleasePlan {
            current_version,
            next_version,
            bump,
            commits: conventional_commits,
            tag_name,
        })
    }

    fn execute(&self, plan: &ReleasePlan, dry_run: bool) -> Result<(), ReleaseError> {
        let run_failure_hooks = |err: ReleaseError| -> ReleaseError {
            let _ = self
                .hooks
                .run(&to_hook_commands(&self.config.hooks.on_failure));
            err
        };

        // Pre-release hooks
        self.hooks
            .run(&to_hook_commands(&self.config.hooks.pre_release))
            .map_err(&run_failure_hooks)?;

        // Format changelog
        let changelog_body = self.format_changelog(plan).map_err(&run_failure_hooks)?;

        if dry_run {
            eprintln!("[dry-run] Would create tag: {}", plan.tag_name);
            eprintln!("[dry-run] Would push tag: {}", plan.tag_name);
            if self.vcs.is_some() {
                eprintln!(
                    "[dry-run] Would create GitHub release for {}",
                    plan.tag_name
                );
            }
            eprintln!("[dry-run] Changelog:\n{changelog_body}");
            return Ok(());
        }

        // Write changelog file if configured
        if let Some(ref changelog_file) = self.config.changelog.file {
            let path = Path::new(changelog_file);
            let existing = if path.exists() {
                fs::read_to_string(path)
                    .map_err(|e| ReleaseError::Changelog(e.to_string()))
                    .map_err(&run_failure_hooks)?
            } else {
                String::new()
            };
            let new_content = if existing.is_empty() {
                format!("# Changelog\n\n{changelog_body}\n")
            } else {
                // Insert after the first heading line
                match existing.find("\n\n") {
                    Some(pos) => {
                        let (header, rest) = existing.split_at(pos);
                        format!("{header}\n\n{changelog_body}\n{rest}")
                    }
                    None => format!("{existing}\n\n{changelog_body}\n"),
                }
            };
            fs::write(path, new_content)
                .map_err(|e| ReleaseError::Changelog(e.to_string()))
                .map_err(&run_failure_hooks)?;
        }

        // Create and push tag
        self.git
            .create_tag(&plan.tag_name, &changelog_body)
            .map_err(&run_failure_hooks)?;
        self.git
            .push_tag(&plan.tag_name)
            .map_err(&run_failure_hooks)?;

        // Post-tag hooks
        self.hooks
            .run(&to_hook_commands(&self.config.hooks.post_tag))
            .map_err(&run_failure_hooks)?;

        // Create GitHub release
        if let Some(ref vcs) = self.vcs {
            let release_name = format!("{} {}", self.config.tag_prefix, plan.next_version);
            vcs.create_release(&plan.tag_name, &release_name, &changelog_body, false)
                .map_err(&run_failure_hooks)?;
        }

        // Post-release hooks
        self.hooks
            .run(&to_hook_commands(&self.config.hooks.post_release))
            .map_err(&run_failure_hooks)?;

        eprintln!("Released {}", plan.tag_name);
        Ok(())
    }
}

pub fn today_string() -> String {
    // Use a simple approach: read from the `date` command or fallback
    std::process::Command::new("date")
        .arg("+%Y-%m-%d")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::changelog::DefaultChangelogFormatter;
    use crate::commit::{Commit, DefaultCommitParser};
    use crate::config::ReleaseConfig;
    use crate::git::{GitRepository, TagInfo};
    use crate::hooks::{HookCommand, HookRunner};

    // --- Fakes ---

    struct FakeGit {
        tags: Vec<TagInfo>,
        commits: Vec<Commit>,
        created_tags: Mutex<Vec<String>>,
        pushed_tags: Mutex<Vec<String>>,
    }

    impl FakeGit {
        fn new(tags: Vec<TagInfo>, commits: Vec<Commit>) -> Self {
            Self {
                tags,
                commits,
                created_tags: Mutex::new(Vec::new()),
                pushed_tags: Mutex::new(Vec::new()),
            }
        }
    }

    impl GitRepository for FakeGit {
        fn latest_tag(&self, _prefix: &str) -> Result<Option<TagInfo>, ReleaseError> {
            Ok(self.tags.last().cloned())
        }

        fn commits_since(&self, _from: Option<&str>) -> Result<Vec<Commit>, ReleaseError> {
            Ok(self.commits.clone())
        }

        fn create_tag(&self, name: &str, _message: &str) -> Result<(), ReleaseError> {
            self.created_tags.lock().unwrap().push(name.to_string());
            Ok(())
        }

        fn push_tag(&self, name: &str) -> Result<(), ReleaseError> {
            self.pushed_tags.lock().unwrap().push(name.to_string());
            Ok(())
        }
    }

    struct FakeVcs {
        releases: Mutex<Vec<(String, String)>>,
    }

    impl FakeVcs {
        fn new() -> Self {
            Self {
                releases: Mutex::new(Vec::new()),
            }
        }
    }

    impl VcsProvider for FakeVcs {
        fn create_release(
            &self,
            tag: &str,
            _name: &str,
            body: &str,
            _prerelease: bool,
        ) -> Result<String, ReleaseError> {
            self.releases
                .lock()
                .unwrap()
                .push((tag.to_string(), body.to_string()));
            Ok(format!("https://github.com/test/release/{tag}"))
        }

        fn compare_url(&self, base: &str, head: &str) -> Result<String, ReleaseError> {
            Ok(format!("https://github.com/test/compare/{base}...{head}"))
        }
    }

    struct FakeHooks {
        run_log: Mutex<Vec<String>>,
    }

    impl FakeHooks {
        fn new() -> Self {
            Self {
                run_log: Mutex::new(Vec::new()),
            }
        }
    }

    impl HookRunner for FakeHooks {
        fn run(&self, hooks: &[HookCommand]) -> Result<(), ReleaseError> {
            for h in hooks {
                self.run_log.lock().unwrap().push(h.command.clone());
            }
            Ok(())
        }
    }

    // --- Helpers ---

    fn raw_commit(msg: &str) -> Commit {
        Commit {
            sha: "a".repeat(40),
            message: msg.into(),
        }
    }

    fn make_strategy(
        tags: Vec<TagInfo>,
        commits: Vec<Commit>,
        config: ReleaseConfig,
    ) -> TrunkReleaseStrategy<
        FakeGit,
        FakeVcs,
        DefaultCommitParser,
        DefaultChangelogFormatter,
        FakeHooks,
    > {
        let types = config.types.clone();
        let breaking_section = config.breaking_section.clone();
        TrunkReleaseStrategy {
            git: FakeGit::new(tags, commits),
            vcs: Some(FakeVcs::new()),
            parser: DefaultCommitParser,
            formatter: DefaultChangelogFormatter::new(None, types, breaking_section),
            hooks: FakeHooks::new(),
            config,
        }
    }

    // --- plan() tests ---

    #[test]
    fn plan_no_commits_returns_error() {
        let s = make_strategy(vec![], vec![], ReleaseConfig::default());
        let err = s.plan().unwrap_err();
        assert!(matches!(err, ReleaseError::NoCommits));
    }

    #[test]
    fn plan_no_releasable_returns_error() {
        let s = make_strategy(
            vec![],
            vec![raw_commit("chore: tidy up")],
            ReleaseConfig::default(),
        );
        let err = s.plan().unwrap_err();
        assert!(matches!(err, ReleaseError::NoBump));
    }

    #[test]
    fn plan_first_release() {
        let s = make_strategy(
            vec![],
            vec![raw_commit("feat: initial feature")],
            ReleaseConfig::default(),
        );
        let plan = s.plan().unwrap();
        assert_eq!(plan.next_version, Version::new(0, 1, 0));
        assert_eq!(plan.tag_name, "v0.1.0");
        assert!(plan.current_version.is_none());
    }

    #[test]
    fn plan_increments_existing() {
        let tag = TagInfo {
            name: "v1.2.3".into(),
            version: Version::new(1, 2, 3),
            sha: "b".repeat(40),
        };
        let s = make_strategy(
            vec![tag],
            vec![raw_commit("fix: patch bug")],
            ReleaseConfig::default(),
        );
        let plan = s.plan().unwrap();
        assert_eq!(plan.next_version, Version::new(1, 2, 4));
    }

    #[test]
    fn plan_breaking_bump() {
        let tag = TagInfo {
            name: "v1.2.3".into(),
            version: Version::new(1, 2, 3),
            sha: "c".repeat(40),
        };
        let s = make_strategy(
            vec![tag],
            vec![raw_commit("feat!: breaking change")],
            ReleaseConfig::default(),
        );
        let plan = s.plan().unwrap();
        assert_eq!(plan.next_version, Version::new(2, 0, 0));
    }

    // --- execute() tests ---

    #[test]
    fn execute_dry_run_no_side_effects() {
        let s = make_strategy(
            vec![],
            vec![raw_commit("feat: something")],
            ReleaseConfig::default(),
        );
        let plan = s.plan().unwrap();
        s.execute(&plan, true).unwrap();

        assert!(s.git.created_tags.lock().unwrap().is_empty());
        assert!(s.git.pushed_tags.lock().unwrap().is_empty());
    }

    #[test]
    fn execute_creates_and_pushes_tag() {
        let s = make_strategy(
            vec![],
            vec![raw_commit("feat: something")],
            ReleaseConfig::default(),
        );
        let plan = s.plan().unwrap();
        s.execute(&plan, false).unwrap();

        assert_eq!(*s.git.created_tags.lock().unwrap(), vec!["v0.1.0"]);
        assert_eq!(*s.git.pushed_tags.lock().unwrap(), vec!["v0.1.0"]);
    }

    #[test]
    fn execute_runs_hooks_in_order() {
        let mut config = ReleaseConfig::default();
        config.hooks.pre_release = vec!["echo pre".into()];
        config.hooks.post_tag = vec!["echo post-tag".into()];
        config.hooks.post_release = vec!["echo post-release".into()];

        let s = make_strategy(vec![], vec![raw_commit("feat: something")], config);
        let plan = s.plan().unwrap();
        s.execute(&plan, false).unwrap();

        let log = s.hooks.run_log.lock().unwrap();
        assert_eq!(*log, vec!["echo pre", "echo post-tag", "echo post-release"]);
    }

    #[test]
    fn execute_calls_vcs_create_release() {
        let s = make_strategy(
            vec![],
            vec![raw_commit("feat: something")],
            ReleaseConfig::default(),
        );
        let plan = s.plan().unwrap();
        s.execute(&plan, false).unwrap();

        let releases = s.vcs.as_ref().unwrap().releases.lock().unwrap();
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].0, "v0.1.0");
        assert!(!releases[0].1.is_empty());
    }
}
