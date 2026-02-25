use std::fs;
use std::path::Path;

use semver::Version;
use serde::Serialize;

use crate::changelog::{ChangelogEntry, ChangelogFormatter};
use crate::commit::{CommitParser, ConventionalCommit, DefaultCommitClassifier};
use crate::config::ReleaseConfig;
use crate::error::ReleaseError;
use crate::git::GitRepository;
use crate::version::{BumpLevel, apply_bump, determine_bump};
use crate::version_files::bump_version_file;

/// The computed plan for a release, before execution.
#[derive(Debug, Serialize)]
pub struct ReleasePlan {
    pub current_version: Option<Version>,
    pub next_version: Version,
    pub bump: BumpLevel,
    pub commits: Vec<ConventionalCommit>,
    pub tag_name: String,
    pub floating_tag_name: Option<String>,
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

    /// Check if a release already exists for the given tag.
    fn release_exists(&self, tag: &str) -> Result<bool, ReleaseError>;

    /// Delete a release by tag.
    fn delete_release(&self, tag: &str) -> Result<(), ReleaseError>;

    /// Return the base URL of the repository (e.g. `https://github.com/owner/repo`).
    fn repo_url(&self) -> Option<String> {
        None
    }

    /// Upload asset files to an existing release identified by tag.
    fn upload_assets(&self, _tag: &str, _files: &[&str]) -> Result<(), ReleaseError> {
        Ok(())
    }
}

/// Concrete release strategy implementing the trunk-based release flow.
pub struct TrunkReleaseStrategy<G, V, C, F> {
    pub git: G,
    pub vcs: Option<V>,
    pub parser: C,
    pub formatter: F,
    pub config: ReleaseConfig,
    /// When true, re-release the current tag if HEAD is at the latest tag.
    pub force: bool,
}

impl<G, V, C, F> TrunkReleaseStrategy<G, V, C, F>
where
    G: GitRepository,
    V: VcsProvider,
    C: CommitParser,
    F: ChangelogFormatter,
{
    fn format_changelog(&self, plan: &ReleasePlan) -> Result<String, ReleaseError> {
        let today = today_string();
        let entry = ChangelogEntry {
            version: plan.next_version.to_string(),
            date: today,
            commits: plan.commits.clone(),
            compare_url: None,
            repo_url: self.vcs.as_ref().and_then(|v| v.repo_url()),
        };
        self.formatter.format(&[entry])
    }
}

impl<G, V, C, F> ReleaseStrategy for TrunkReleaseStrategy<G, V, C, F>
where
    G: GitRepository,
    V: VcsProvider,
    C: CommitParser,
    F: ChangelogFormatter,
{
    fn plan(&self) -> Result<ReleasePlan, ReleaseError> {
        let tag_info = self.git.latest_tag(&self.config.tag_prefix)?;

        let (current_version, from_sha) = match &tag_info {
            Some(info) => (Some(info.version.clone()), Some(info.sha.as_str())),
            None => (None, None),
        };

        let raw_commits = self.git.commits_since(from_sha)?;
        if raw_commits.is_empty() {
            // Force mode: re-release if HEAD is exactly at the latest tag
            if self.force
                && let Some(ref info) = tag_info
            {
                let head = self.git.head_sha()?;
                if head == info.sha {
                    let floating_tag_name = if self.config.floating_tags {
                        Some(format!("{}{}", self.config.tag_prefix, info.version.major))
                    } else {
                        None
                    };
                    return Ok(ReleasePlan {
                        current_version: Some(info.version.clone()),
                        next_version: info.version.clone(),
                        bump: BumpLevel::Patch, // nominal; no actual bump
                        commits: vec![],
                        tag_name: info.name.clone(),
                        floating_tag_name,
                    });
                }
            }
            let (tag, sha) = match &tag_info {
                Some(info) => (info.name.clone(), info.sha.clone()),
                None => ("(none)".into(), "(none)".into()),
            };
            return Err(ReleaseError::NoCommits { tag, sha });
        }

        let conventional_commits: Vec<ConventionalCommit> = raw_commits
            .iter()
            .filter_map(|c| self.parser.parse(c).ok())
            .collect();

        let classifier = DefaultCommitClassifier::new(
            self.config.types.clone(),
            self.config.commit_pattern.clone(),
        );
        let tag_for_err = tag_info
            .as_ref()
            .map(|i| i.name.clone())
            .unwrap_or_else(|| "(none)".into());
        let commit_count = conventional_commits.len();
        let bump =
            determine_bump(&conventional_commits, &classifier).ok_or(ReleaseError::NoBump {
                tag: tag_for_err,
                commit_count,
            })?;

        let base_version = current_version.clone().unwrap_or(Version::new(0, 0, 0));
        let next_version = apply_bump(&base_version, bump);
        let tag_name = format!("{}{next_version}", self.config.tag_prefix);

        let floating_tag_name = if self.config.floating_tags {
            Some(format!("{}{}", self.config.tag_prefix, next_version.major))
        } else {
            None
        };

        Ok(ReleasePlan {
            current_version,
            next_version,
            bump,
            commits: conventional_commits,
            tag_name,
            floating_tag_name,
        })
    }

    fn execute(&self, plan: &ReleasePlan, dry_run: bool) -> Result<(), ReleaseError> {
        if dry_run {
            let changelog_body = self.format_changelog(plan)?;
            eprintln!("[dry-run] Would create tag: {}", plan.tag_name);
            eprintln!("[dry-run] Would push tag: {}", plan.tag_name);
            if let Some(ref floating) = plan.floating_tag_name {
                eprintln!("[dry-run] Would create/update floating tag: {floating}");
                eprintln!("[dry-run] Would force-push floating tag: {floating}");
            }
            if self.vcs.is_some() {
                eprintln!(
                    "[dry-run] Would create GitHub release for {}",
                    plan.tag_name
                );
            }
            for file in &self.config.version_files {
                let filename = Path::new(file)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default();
                let supported = matches!(
                    filename,
                    "Cargo.toml"
                        | "package.json"
                        | "pyproject.toml"
                        | "pom.xml"
                        | "build.gradle"
                        | "build.gradle.kts"
                ) || filename.ends_with(".go");
                if supported {
                    eprintln!("[dry-run] Would bump version in: {file}");
                } else if self.config.version_files_strict {
                    return Err(ReleaseError::VersionBump(format!(
                        "unsupported version file: {filename}"
                    )));
                } else {
                    eprintln!("[dry-run] warning: unsupported version file, would skip: {file}");
                }
            }
            if !self.config.artifacts.is_empty() {
                let resolved = resolve_artifact_globs(&self.config.artifacts)?;
                if resolved.is_empty() {
                    eprintln!("[dry-run] Artifact patterns matched no files");
                } else {
                    eprintln!("[dry-run] Would upload {} artifact(s):", resolved.len());
                    for f in &resolved {
                        eprintln!("[dry-run]   {f}");
                    }
                }
            }
            if let Some(ref cmd) = self.config.build_command {
                eprintln!("[dry-run] Would run build command: {cmd}");
            }
            eprintln!("[dry-run] Changelog:\n{changelog_body}");
            return Ok(());
        }

        // 1. Format changelog
        let changelog_body = self.format_changelog(plan)?;

        // 2. Bump version files
        let version_str = plan.next_version.to_string();
        let mut bumped_files: Vec<&str> = Vec::new();
        for file in &self.config.version_files {
            match bump_version_file(Path::new(file), &version_str) {
                Ok(()) => bumped_files.push(file.as_str()),
                Err(e) if !self.config.version_files_strict => {
                    eprintln!("warning: {e} — skipping {file}");
                }
                Err(e) => return Err(e),
            }
        }

        // 3. Write changelog file if configured
        if let Some(ref changelog_file) = self.config.changelog.file {
            let path = Path::new(changelog_file);
            let existing = if path.exists() {
                fs::read_to_string(path).map_err(|e| ReleaseError::Changelog(e.to_string()))?
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
            fs::write(path, new_content).map_err(|e| ReleaseError::Changelog(e.to_string()))?;
        }

        // 3.5. Run build command if configured
        if let Some(ref cmd) = self.config.build_command {
            eprintln!("Running build command: {cmd}");
            let status = std::process::Command::new("sh")
                .args(["-c", cmd])
                .env("SR_VERSION", &version_str)
                .env("SR_TAG", &plan.tag_name)
                .status()
                .map_err(|e| ReleaseError::BuildCommand(e.to_string()))?;
            if !status.success() {
                return Err(ReleaseError::BuildCommand(format!(
                    "command exited with {}",
                    status.code().unwrap_or(-1)
                )));
            }
        }

        // 4. Stage and commit changelog + version files (skip if nothing to stage)
        {
            let mut paths_to_stage: Vec<&str> = Vec::new();
            if let Some(ref changelog_file) = self.config.changelog.file {
                paths_to_stage.push(changelog_file.as_str());
            }
            for file in &bumped_files {
                paths_to_stage.push(*file);
            }
            if !paths_to_stage.is_empty() {
                let commit_msg = format!("chore(release): {} [skip ci]", plan.tag_name);
                self.git.stage_and_commit(&paths_to_stage, &commit_msg)?;
            }
        }

        // 5. Create tag (skip if it already exists locally)
        if !self.git.tag_exists(&plan.tag_name)? {
            self.git.create_tag(&plan.tag_name, &changelog_body)?;
        }

        // 6. Push commit (safe to re-run — no-op if up to date)
        self.git.push()?;

        // 7. Push tag (skip if tag already exists on remote)
        if !self.git.remote_tag_exists(&plan.tag_name)? {
            self.git.push_tag(&plan.tag_name)?;
        }

        // 8. Force-create and force-push floating tag (e.g. v3)
        if let Some(ref floating) = plan.floating_tag_name {
            let floating_msg = format!("Floating tag for {}", plan.tag_name);
            self.git.force_create_tag(floating, &floating_msg)?;
            self.git.force_push_tag(floating)?;
        }

        // 9. Create GitHub release (skip if exists, or update it)
        if let Some(ref vcs) = self.vcs {
            let release_name = format!("{} {}", self.config.tag_prefix, plan.next_version);
            if vcs.release_exists(&plan.tag_name)? {
                // Delete and recreate to update the release notes
                vcs.delete_release(&plan.tag_name)?;
            }
            vcs.create_release(&plan.tag_name, &release_name, &changelog_body, false)?;
        }

        // 10. Upload artifacts
        if let Some(ref vcs) = self.vcs
            && !self.config.artifacts.is_empty()
        {
            let resolved = resolve_artifact_globs(&self.config.artifacts)?;
            if !resolved.is_empty() {
                let file_refs: Vec<&str> = resolved.iter().map(|s| s.as_str()).collect();
                vcs.upload_assets(&plan.tag_name, &file_refs)?;
                eprintln!(
                    "Uploaded {} artifact(s) to {}",
                    resolved.len(),
                    plan.tag_name
                );
            }
        }

        eprintln!("Released {}", plan.tag_name);
        Ok(())
    }
}

fn resolve_artifact_globs(patterns: &[String]) -> Result<Vec<String>, ReleaseError> {
    let mut files = std::collections::BTreeSet::new();
    for pattern in patterns {
        let paths = glob::glob(pattern)
            .map_err(|e| ReleaseError::Vcs(format!("invalid glob pattern '{pattern}': {e}")))?;
        for entry in paths {
            match entry {
                Ok(path) if path.is_file() => {
                    files.insert(path.to_string_lossy().into_owned());
                }
                Ok(_) => {} // skip directories
                Err(e) => {
                    eprintln!("warning: glob error: {e}");
                }
            }
        }
    }
    Ok(files.into_iter().collect())
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

    // --- Fakes ---

    struct FakeGit {
        tags: Vec<TagInfo>,
        commits: Vec<Commit>,
        head: String,
        created_tags: Mutex<Vec<String>>,
        pushed_tags: Mutex<Vec<String>>,
        committed: Mutex<Vec<(Vec<String>, String)>>,
        push_count: Mutex<u32>,
        force_created_tags: Mutex<Vec<String>>,
        force_pushed_tags: Mutex<Vec<String>>,
    }

    impl FakeGit {
        fn new(tags: Vec<TagInfo>, commits: Vec<Commit>) -> Self {
            let head = tags
                .last()
                .map(|t| t.sha.clone())
                .unwrap_or_else(|| "0".repeat(40));
            Self {
                tags,
                commits,
                head,
                created_tags: Mutex::new(Vec::new()),
                pushed_tags: Mutex::new(Vec::new()),
                committed: Mutex::new(Vec::new()),
                push_count: Mutex::new(0),
                force_created_tags: Mutex::new(Vec::new()),
                force_pushed_tags: Mutex::new(Vec::new()),
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

        fn stage_and_commit(&self, paths: &[&str], message: &str) -> Result<bool, ReleaseError> {
            self.committed.lock().unwrap().push((
                paths.iter().map(|s| s.to_string()).collect(),
                message.to_string(),
            ));
            Ok(true)
        }

        fn push(&self) -> Result<(), ReleaseError> {
            *self.push_count.lock().unwrap() += 1;
            Ok(())
        }

        fn tag_exists(&self, name: &str) -> Result<bool, ReleaseError> {
            Ok(self
                .created_tags
                .lock()
                .unwrap()
                .contains(&name.to_string()))
        }

        fn remote_tag_exists(&self, name: &str) -> Result<bool, ReleaseError> {
            Ok(self.pushed_tags.lock().unwrap().contains(&name.to_string()))
        }

        fn all_tags(&self, _prefix: &str) -> Result<Vec<TagInfo>, ReleaseError> {
            Ok(self.tags.clone())
        }

        fn commits_between(
            &self,
            _from: Option<&str>,
            _to: &str,
        ) -> Result<Vec<Commit>, ReleaseError> {
            Ok(self.commits.clone())
        }

        fn tag_date(&self, _tag_name: &str) -> Result<String, ReleaseError> {
            Ok("2026-01-01".into())
        }

        fn force_create_tag(&self, name: &str, _message: &str) -> Result<(), ReleaseError> {
            self.force_created_tags
                .lock()
                .unwrap()
                .push(name.to_string());
            Ok(())
        }

        fn force_push_tag(&self, name: &str) -> Result<(), ReleaseError> {
            self.force_pushed_tags
                .lock()
                .unwrap()
                .push(name.to_string());
            Ok(())
        }

        fn head_sha(&self) -> Result<String, ReleaseError> {
            Ok(self.head.clone())
        }
    }

    struct FakeVcs {
        releases: Mutex<Vec<(String, String)>>,
        deleted_releases: Mutex<Vec<String>>,
        uploaded_assets: Mutex<Vec<(String, Vec<String>)>>,
    }

    impl FakeVcs {
        fn new() -> Self {
            Self {
                releases: Mutex::new(Vec::new()),
                deleted_releases: Mutex::new(Vec::new()),
                uploaded_assets: Mutex::new(Vec::new()),
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

        fn release_exists(&self, tag: &str) -> Result<bool, ReleaseError> {
            Ok(self.releases.lock().unwrap().iter().any(|(t, _)| t == tag))
        }

        fn delete_release(&self, tag: &str) -> Result<(), ReleaseError> {
            self.deleted_releases.lock().unwrap().push(tag.to_string());
            self.releases.lock().unwrap().retain(|(t, _)| t != tag);
            Ok(())
        }

        fn upload_assets(&self, tag: &str, files: &[&str]) -> Result<(), ReleaseError> {
            self.uploaded_assets.lock().unwrap().push((
                tag.to_string(),
                files.iter().map(|s| s.to_string()).collect(),
            ));
            Ok(())
        }

        fn repo_url(&self) -> Option<String> {
            Some("https://github.com/test/repo".into())
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
    ) -> TrunkReleaseStrategy<FakeGit, FakeVcs, DefaultCommitParser, DefaultChangelogFormatter>
    {
        let types = config.types.clone();
        let breaking_section = config.breaking_section.clone();
        let misc_section = config.misc_section.clone();
        TrunkReleaseStrategy {
            git: FakeGit::new(tags, commits),
            vcs: Some(FakeVcs::new()),
            parser: DefaultCommitParser,
            formatter: DefaultChangelogFormatter::new(None, types, breaking_section, misc_section),
            config,
            force: false,
        }
    }

    // --- plan() tests ---

    #[test]
    fn plan_no_commits_returns_error() {
        let s = make_strategy(vec![], vec![], ReleaseConfig::default());
        let err = s.plan().unwrap_err();
        assert!(matches!(err, ReleaseError::NoCommits { .. }));
    }

    #[test]
    fn plan_no_releasable_returns_error() {
        let s = make_strategy(
            vec![],
            vec![raw_commit("chore: tidy up")],
            ReleaseConfig::default(),
        );
        let err = s.plan().unwrap_err();
        assert!(matches!(err, ReleaseError::NoBump { .. }));
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

    #[test]
    fn execute_commits_changelog_before_tag() {
        let dir = tempfile::tempdir().unwrap();
        let changelog_path = dir.path().join("CHANGELOG.md");

        let mut config = ReleaseConfig::default();
        config.changelog.file = Some(changelog_path.to_str().unwrap().to_string());

        let s = make_strategy(vec![], vec![raw_commit("feat: something")], config);
        let plan = s.plan().unwrap();
        s.execute(&plan, false).unwrap();

        // Verify changelog was committed
        let committed = s.git.committed.lock().unwrap();
        assert_eq!(committed.len(), 1);
        assert_eq!(
            committed[0].0,
            vec![changelog_path.to_str().unwrap().to_string()]
        );
        assert!(committed[0].1.contains("chore(release): v0.1.0"));

        // Verify tag was created after commit
        assert_eq!(*s.git.created_tags.lock().unwrap(), vec!["v0.1.0"]);
    }

    #[test]
    fn execute_skips_existing_tag() {
        let s = make_strategy(
            vec![],
            vec![raw_commit("feat: something")],
            ReleaseConfig::default(),
        );
        let plan = s.plan().unwrap();

        // Pre-populate the tag to simulate it already existing
        s.git
            .created_tags
            .lock()
            .unwrap()
            .push("v0.1.0".to_string());

        s.execute(&plan, false).unwrap();

        // Tag should not be created again (still only the one we pre-populated)
        assert_eq!(s.git.created_tags.lock().unwrap().len(), 1);
    }

    #[test]
    fn execute_skips_existing_release() {
        let s = make_strategy(
            vec![],
            vec![raw_commit("feat: something")],
            ReleaseConfig::default(),
        );
        let plan = s.plan().unwrap();

        // Pre-populate a release to simulate it already existing
        s.vcs
            .as_ref()
            .unwrap()
            .releases
            .lock()
            .unwrap()
            .push(("v0.1.0".to_string(), "old notes".to_string()));

        s.execute(&plan, false).unwrap();

        // Should have deleted the old release and created a new one
        let deleted = s.vcs.as_ref().unwrap().deleted_releases.lock().unwrap();
        assert_eq!(*deleted, vec!["v0.1.0"]);

        let releases = s.vcs.as_ref().unwrap().releases.lock().unwrap();
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].0, "v0.1.0");
        assert_ne!(releases[0].1, "old notes");
    }

    #[test]
    fn execute_idempotent_rerun() {
        let s = make_strategy(
            vec![],
            vec![raw_commit("feat: something")],
            ReleaseConfig::default(),
        );
        let plan = s.plan().unwrap();

        // First run
        s.execute(&plan, false).unwrap();

        // Second run should also succeed (idempotent)
        s.execute(&plan, false).unwrap();

        // Tag should only have been created once (second run skips because tag_exists)
        assert_eq!(s.git.created_tags.lock().unwrap().len(), 1);

        // Tag push should only happen once (second run skips because remote_tag_exists)
        assert_eq!(s.git.pushed_tags.lock().unwrap().len(), 1);

        // Push (commit) should happen twice (always safe)
        assert_eq!(*s.git.push_count.lock().unwrap(), 2);

        // Release should be deleted and recreated on second run
        let deleted = s.vcs.as_ref().unwrap().deleted_releases.lock().unwrap();
        assert_eq!(*deleted, vec!["v0.1.0"]);

        let releases = s.vcs.as_ref().unwrap().releases.lock().unwrap();
        // One entry: delete removed the first, create added a replacement
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].0, "v0.1.0");
    }

    #[test]
    fn execute_bumps_version_files() {
        let dir = tempfile::tempdir().unwrap();
        let cargo_path = dir.path().join("Cargo.toml");
        std::fs::write(
            &cargo_path,
            "[package]\nname = \"test\"\nversion = \"0.0.0\"\n",
        )
        .unwrap();

        let mut config = ReleaseConfig::default();
        config.version_files = vec![cargo_path.to_str().unwrap().to_string()];

        let s = make_strategy(vec![], vec![raw_commit("feat: something")], config);
        let plan = s.plan().unwrap();
        s.execute(&plan, false).unwrap();

        // Verify the file was bumped
        let contents = std::fs::read_to_string(&cargo_path).unwrap();
        assert!(contents.contains("version = \"0.1.0\""));

        // Verify it was staged alongside the commit
        let committed = s.git.committed.lock().unwrap();
        assert_eq!(committed.len(), 1);
        assert!(
            committed[0]
                .0
                .contains(&cargo_path.to_str().unwrap().to_string())
        );
    }

    #[test]
    fn execute_stages_changelog_and_version_files_together() {
        let dir = tempfile::tempdir().unwrap();
        let cargo_path = dir.path().join("Cargo.toml");
        std::fs::write(
            &cargo_path,
            "[package]\nname = \"test\"\nversion = \"0.0.0\"\n",
        )
        .unwrap();

        let changelog_path = dir.path().join("CHANGELOG.md");

        let mut config = ReleaseConfig::default();
        config.changelog.file = Some(changelog_path.to_str().unwrap().to_string());
        config.version_files = vec![cargo_path.to_str().unwrap().to_string()];

        let s = make_strategy(vec![], vec![raw_commit("feat: something")], config);
        let plan = s.plan().unwrap();
        s.execute(&plan, false).unwrap();

        // Both changelog and version file should be staged in a single commit
        let committed = s.git.committed.lock().unwrap();
        assert_eq!(committed.len(), 1);
        assert!(
            committed[0]
                .0
                .contains(&changelog_path.to_str().unwrap().to_string())
        );
        assert!(
            committed[0]
                .0
                .contains(&cargo_path.to_str().unwrap().to_string())
        );
    }

    // --- artifact upload tests ---

    #[test]
    fn execute_uploads_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("app.tar.gz"), "fake tarball").unwrap();
        std::fs::write(dir.path().join("app.zip"), "fake zip").unwrap();

        let mut config = ReleaseConfig::default();
        config.artifacts = vec![
            dir.path().join("*.tar.gz").to_str().unwrap().to_string(),
            dir.path().join("*.zip").to_str().unwrap().to_string(),
        ];

        let s = make_strategy(vec![], vec![raw_commit("feat: something")], config);
        let plan = s.plan().unwrap();
        s.execute(&plan, false).unwrap();

        let uploaded = s.vcs.as_ref().unwrap().uploaded_assets.lock().unwrap();
        assert_eq!(uploaded.len(), 1);
        assert_eq!(uploaded[0].0, "v0.1.0");
        assert_eq!(uploaded[0].1.len(), 2);
        assert!(uploaded[0].1.iter().any(|f| f.ends_with("app.tar.gz")));
        assert!(uploaded[0].1.iter().any(|f| f.ends_with("app.zip")));
    }

    #[test]
    fn execute_dry_run_shows_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("app.tar.gz"), "fake tarball").unwrap();

        let mut config = ReleaseConfig::default();
        config.artifacts = vec![dir.path().join("*.tar.gz").to_str().unwrap().to_string()];

        let s = make_strategy(vec![], vec![raw_commit("feat: something")], config);
        let plan = s.plan().unwrap();
        s.execute(&plan, true).unwrap();

        // No uploads should happen during dry-run
        let uploaded = s.vcs.as_ref().unwrap().uploaded_assets.lock().unwrap();
        assert!(uploaded.is_empty());
    }

    #[test]
    fn execute_no_artifacts_skips_upload() {
        let s = make_strategy(
            vec![],
            vec![raw_commit("feat: something")],
            ReleaseConfig::default(),
        );
        let plan = s.plan().unwrap();
        s.execute(&plan, false).unwrap();

        let uploaded = s.vcs.as_ref().unwrap().uploaded_assets.lock().unwrap();
        assert!(uploaded.is_empty());
    }

    #[test]
    fn resolve_artifact_globs_basic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "a").unwrap();
        std::fs::write(dir.path().join("b.txt"), "b").unwrap();
        std::fs::create_dir(dir.path().join("subdir")).unwrap();

        let pattern = dir.path().join("*.txt").to_str().unwrap().to_string();
        let result = resolve_artifact_globs(&[pattern]).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|f| f.ends_with("a.txt")));
        assert!(result.iter().any(|f| f.ends_with("b.txt")));
    }

    #[test]
    fn resolve_artifact_globs_deduplicates() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("file.txt"), "data").unwrap();

        let pattern = dir.path().join("*.txt").to_str().unwrap().to_string();
        // Same pattern twice should not produce duplicates
        let result = resolve_artifact_globs(&[pattern.clone(), pattern]).unwrap();
        assert_eq!(result.len(), 1);
    }

    // --- floating tags tests ---

    #[test]
    fn plan_floating_tag_when_enabled() {
        let tag = TagInfo {
            name: "v3.2.0".into(),
            version: Version::new(3, 2, 0),
            sha: "d".repeat(40),
        };
        let mut config = ReleaseConfig::default();
        config.floating_tags = true;

        let s = make_strategy(vec![tag], vec![raw_commit("fix: patch")], config);
        let plan = s.plan().unwrap();
        assert_eq!(plan.next_version, Version::new(3, 2, 1));
        assert_eq!(plan.floating_tag_name.as_deref(), Some("v3"));
    }

    #[test]
    fn plan_no_floating_tag_when_disabled() {
        let s = make_strategy(
            vec![],
            vec![raw_commit("feat: something")],
            ReleaseConfig::default(),
        );
        let plan = s.plan().unwrap();
        assert!(plan.floating_tag_name.is_none());
    }

    #[test]
    fn plan_floating_tag_custom_prefix() {
        let tag = TagInfo {
            name: "release-2.5.0".into(),
            version: Version::new(2, 5, 0),
            sha: "e".repeat(40),
        };
        let mut config = ReleaseConfig::default();
        config.floating_tags = true;
        config.tag_prefix = "release-".into();

        let s = make_strategy(vec![tag], vec![raw_commit("fix: patch")], config);
        let plan = s.plan().unwrap();
        assert_eq!(plan.floating_tag_name.as_deref(), Some("release-2"));
    }

    #[test]
    fn execute_floating_tags_force_create_and_push() {
        let mut config = ReleaseConfig::default();
        config.floating_tags = true;

        let tag = TagInfo {
            name: "v1.2.3".into(),
            version: Version::new(1, 2, 3),
            sha: "f".repeat(40),
        };
        let s = make_strategy(vec![tag], vec![raw_commit("fix: a bug")], config);
        let plan = s.plan().unwrap();
        assert_eq!(plan.floating_tag_name.as_deref(), Some("v1"));

        s.execute(&plan, false).unwrap();

        assert_eq!(*s.git.force_created_tags.lock().unwrap(), vec!["v1"]);
        assert_eq!(*s.git.force_pushed_tags.lock().unwrap(), vec!["v1"]);
    }

    #[test]
    fn execute_no_floating_tags_when_disabled() {
        let s = make_strategy(
            vec![],
            vec![raw_commit("feat: something")],
            ReleaseConfig::default(),
        );
        let plan = s.plan().unwrap();
        assert!(plan.floating_tag_name.is_none());

        s.execute(&plan, false).unwrap();

        assert!(s.git.force_created_tags.lock().unwrap().is_empty());
        assert!(s.git.force_pushed_tags.lock().unwrap().is_empty());
    }

    #[test]
    fn execute_floating_tags_dry_run_no_side_effects() {
        let mut config = ReleaseConfig::default();
        config.floating_tags = true;

        let tag = TagInfo {
            name: "v2.0.0".into(),
            version: Version::new(2, 0, 0),
            sha: "a".repeat(40),
        };
        let s = make_strategy(vec![tag], vec![raw_commit("fix: something")], config);
        let plan = s.plan().unwrap();
        assert_eq!(plan.floating_tag_name.as_deref(), Some("v2"));

        s.execute(&plan, true).unwrap();

        assert!(s.git.force_created_tags.lock().unwrap().is_empty());
        assert!(s.git.force_pushed_tags.lock().unwrap().is_empty());
    }

    #[test]
    fn execute_floating_tags_idempotent() {
        let mut config = ReleaseConfig::default();
        config.floating_tags = true;

        let s = make_strategy(vec![], vec![raw_commit("feat: something")], config);
        let plan = s.plan().unwrap();
        assert_eq!(plan.floating_tag_name.as_deref(), Some("v0"));

        // Run twice
        s.execute(&plan, false).unwrap();
        s.execute(&plan, false).unwrap();

        // Force ops run every time (correct for floating tags)
        assert_eq!(s.git.force_created_tags.lock().unwrap().len(), 2);
        assert_eq!(s.git.force_pushed_tags.lock().unwrap().len(), 2);
    }

    // --- force mode tests ---

    #[test]
    fn force_rerelease_when_tag_at_head() {
        let tag = TagInfo {
            name: "v1.2.3".into(),
            version: Version::new(1, 2, 3),
            sha: "a".repeat(40),
        };
        let mut s = make_strategy(vec![tag], vec![], ReleaseConfig::default());
        // HEAD == tag SHA, and no new commits
        s.git.head = "a".repeat(40);
        s.force = true;

        let plan = s.plan().unwrap();
        assert_eq!(plan.next_version, Version::new(1, 2, 3));
        assert_eq!(plan.tag_name, "v1.2.3");
        assert!(plan.commits.is_empty());
        assert_eq!(plan.current_version, Some(Version::new(1, 2, 3)));
    }

    #[test]
    fn force_fails_when_tag_not_at_head() {
        let tag = TagInfo {
            name: "v1.2.3".into(),
            version: Version::new(1, 2, 3),
            sha: "a".repeat(40),
        };
        let mut s = make_strategy(vec![tag], vec![], ReleaseConfig::default());
        // HEAD != tag SHA
        s.git.head = "b".repeat(40);
        s.force = true;

        let err = s.plan().unwrap_err();
        assert!(matches!(err, ReleaseError::NoCommits { .. }));
    }

    // --- build_command tests ---

    #[test]
    fn execute_runs_build_command_after_version_bump() {
        let dir = tempfile::tempdir().unwrap();
        let output_file = dir.path().join("sr_test_version");

        let mut config = ReleaseConfig::default();
        config.build_command = Some(format!(
            "echo $SR_VERSION > {}",
            output_file.to_str().unwrap()
        ));

        let s = make_strategy(vec![], vec![raw_commit("feat: something")], config);
        let plan = s.plan().unwrap();
        s.execute(&plan, false).unwrap();

        let contents = std::fs::read_to_string(&output_file).unwrap();
        assert_eq!(contents.trim(), "0.1.0");
    }

    #[test]
    fn execute_build_command_failure_aborts_release() {
        let mut config = ReleaseConfig::default();
        config.build_command = Some("exit 1".into());

        let s = make_strategy(vec![], vec![raw_commit("feat: something")], config);
        let plan = s.plan().unwrap();
        let result = s.execute(&plan, false);

        assert!(result.is_err());
        assert!(s.git.created_tags.lock().unwrap().is_empty());
    }

    #[test]
    fn execute_dry_run_skips_build_command() {
        let dir = tempfile::tempdir().unwrap();
        let output_file = dir.path().join("sr_test_should_not_exist");

        let mut config = ReleaseConfig::default();
        config.build_command = Some(format!("echo test > {}", output_file.to_str().unwrap()));

        let s = make_strategy(vec![], vec![raw_commit("feat: something")], config);
        let plan = s.plan().unwrap();
        s.execute(&plan, true).unwrap();

        assert!(!output_file.exists());
    }

    #[test]
    fn force_fails_with_no_tags() {
        let mut s = make_strategy(vec![], vec![], ReleaseConfig::default());
        s.force = true;

        let err = s.plan().unwrap_err();
        assert!(matches!(err, ReleaseError::NoCommits { .. }));
    }
}
