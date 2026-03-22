use std::path::Path;
use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand};
use sr_ai::ai::{Backend, BackendConfig};
use sr_core::changelog::DefaultChangelogFormatter;
use sr_core::commit::DefaultCommitParser;
use sr_core::config::{DEFAULT_CONFIG_FILE, HookEntry, LEGACY_CONFIG_FILE, ReleaseConfig};
use sr_core::error::ReleaseError;
use sr_core::release::{ReleaseStrategy, TrunkReleaseStrategy, VcsProvider};
use sr_git::NativeGitRepository;
use sr_github::GitHubProvider;

#[derive(Parser)]
#[command(name = "sr", about = "AI-powered release engineering CLI", version)]
struct Cli {
    /// AI backend to use
    #[arg(long, global = true, env = "SR_BACKEND")]
    backend: Option<Backend>,

    /// AI model to use
    #[arg(long, global = true, env = "SR_MODEL")]
    model: Option<String>,

    /// Max budget in USD (claude only)
    #[arg(long, global = true, env = "SR_BUDGET", default_value = "0.50")]
    budget: f64,

    /// Enable debug output
    #[arg(long, global = true, env = "SR_DEBUG")]
    debug: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute a release (trunk flow: tag + GitHub release)
    Release {
        /// Target a specific package in a monorepo
        #[arg(long, short)]
        package: Option<String>,

        /// Preview what would happen without making changes
        #[arg(long)]
        dry_run: bool,

        /// Glob patterns for artifact files to upload to the release (repeatable)
        #[arg(long = "artifacts")]
        artifacts: Vec<String>,

        /// Re-release the current tag (use when a previous release partially failed)
        #[arg(long)]
        force: bool,

        /// Shell command to run after version bump, before commit (SR_VERSION and SR_TAG env vars available)
        #[arg(long)]
        build_command: Option<String>,

        /// Additional file globs to stage after build command (repeatable, e.g. Cargo.lock)
        #[arg(long = "stage-files")]
        stage_files: Vec<String>,

        /// Shell command to run before the release starts
        #[arg(long)]
        pre_release_command: Option<String>,

        /// Shell command to run after the release completes
        #[arg(long)]
        post_release_command: Option<String>,

        /// Pre-release identifier (e.g. alpha, beta, rc). Produces versions like 1.2.0-alpha.1
        #[arg(long)]
        prerelease: Option<String>,

        /// Sign tags with GPG/SSH (git tag -s)
        #[arg(long)]
        sign_tags: bool,

        /// Create GitHub release as a draft (requires manual publishing)
        #[arg(long)]
        draft: bool,
    },

    /// Show what the next release would look like
    Plan {
        /// Target a specific package in a monorepo
        #[arg(long, short)]
        package: Option<String>,

        /// Output format
        #[arg(long, default_value = "human")]
        format: PlanFormat,
    },

    /// Generate or preview the changelog
    Changelog {
        /// Target a specific package in a monorepo
        #[arg(long, short)]
        package: Option<String>,

        /// Write the changelog to disk
        #[arg(long)]
        write: bool,

        /// Regenerate the entire changelog from all tags
        #[arg(long)]
        regenerate: bool,
    },

    /// Show the next version
    Version {
        /// Target a specific package in a monorepo
        #[arg(long, short)]
        package: Option<String>,

        /// Print only the version number
        #[arg(long)]
        short: bool,
    },

    /// Validate and display resolved configuration
    Config {
        /// Show the fully resolved config with defaults applied
        #[arg(long)]
        resolved: bool,
    },

    /// Create a default configuration file and sync git hooks
    Init {
        /// Overwrite the config file if it already exists
        #[arg(long)]
        force: bool,

        /// Merge new default fields into existing config without overwriting customizations
        #[arg(long, conflicts_with = "force")]
        merge: bool,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: clap_complete::Shell,
    },

    // --- AI-powered commands ---
    /// Generate atomic commits from changes
    Commit(sr_ai::commands::commit::CommitArgs),

    /// AI-powered interactive rebase (reword, squash, reorder commits)
    Rebase(sr_ai::commands::rebase::RebaseArgs),

    /// AI code review of staged/branch changes
    Review(sr_ai::commands::review::ReviewArgs),

    /// Explain recent commits
    Explain(sr_ai::commands::explain::ExplainArgs),

    /// Suggest conventional branch name
    Branch(sr_ai::commands::branch::BranchArgs),

    /// Generate PR title + body from branch commits
    Pr(sr_ai::commands::pr::PrArgs),

    /// Freeform Q&A about the repo
    Ask(sr_ai::commands::ask::AskArgs),

    /// Manage the AI commit plan cache
    Cache(sr_ai::commands::cache::CacheArgs),

    /// Run a configured git hook (also used by generated shim scripts)
    Hook {
        #[command(subcommand)]
        command: HookCommands,
    },

    /// Update sr to the latest version
    Update,
}

#[derive(Subcommand)]
enum HookCommands {
    /// Validate a commit message (reads hook JSON from stdin)
    CommitMsg,

    /// Execute a configured hook — builds a JSON context from git's args and pipes it to each command
    Run {
        /// Git hook name (e.g. commit-msg, pre-commit, pre-push)
        hook_name: String,

        /// Arguments passed by git to the hook
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Clone, clap::ValueEnum)]
enum PlanFormat {
    Human,
    Json,
}

/// A no-op VcsProvider used when GITHUB_TOKEN is not available.
struct NoopVcsProvider;

impl VcsProvider for NoopVcsProvider {
    fn create_release(
        &self,
        _tag: &str,
        _name: &str,
        _body: &str,
        _prerelease: bool,
        _draft: bool,
    ) -> Result<String, sr_core::error::ReleaseError> {
        Ok(String::new())
    }

    fn compare_url(
        &self,
        _base: &str,
        _head: &str,
    ) -> Result<String, sr_core::error::ReleaseError> {
        Ok(String::new())
    }

    fn release_exists(&self, _tag: &str) -> Result<bool, sr_core::error::ReleaseError> {
        Ok(false)
    }

    fn delete_release(&self, _tag: &str) -> Result<(), sr_core::error::ReleaseError> {
        Ok(())
    }
}

fn build_local_strategy(
    config: ReleaseConfig,
    force: bool,
) -> anyhow::Result<
    TrunkReleaseStrategy<
        NativeGitRepository,
        NoopVcsProvider,
        DefaultCommitParser,
        DefaultChangelogFormatter,
    >,
> {
    let git = NativeGitRepository::open(Path::new("."))?;
    let types = config.types.clone();
    let breaking_section = config.breaking_section.clone();
    let misc_section = config.misc_section.clone();
    let formatter = DefaultChangelogFormatter::new(
        config.changelog.template.clone(),
        types,
        breaking_section,
        misc_section,
    );
    Ok(TrunkReleaseStrategy {
        git,
        vcs: None,
        parser: DefaultCommitParser,
        formatter,
        config,
        force,
    })
}

fn build_full_strategy(
    config: ReleaseConfig,
    force: bool,
) -> anyhow::Result<
    TrunkReleaseStrategy<
        NativeGitRepository,
        GitHubProvider,
        DefaultCommitParser,
        DefaultChangelogFormatter,
    >,
> {
    let git = NativeGitRepository::open(Path::new("."))?;
    let (hostname, owner, repo) = git.parse_remote_full()?;

    let token = std::env::var("GH_TOKEN")
        .or_else(|_| std::env::var("GITHUB_TOKEN"))
        .map_err(|_| anyhow::anyhow!("neither GH_TOKEN nor GITHUB_TOKEN is set"))?;

    let git = git.with_http_auth(hostname.clone(), token.clone());
    let vcs = GitHubProvider::new(owner, repo, hostname, token);
    let types = config.types.clone();
    let breaking_section = config.breaking_section.clone();
    let misc_section = config.misc_section.clone();
    let formatter = DefaultChangelogFormatter::new(
        config.changelog.template.clone(),
        types,
        breaking_section,
        misc_section,
    );

    Ok(TrunkReleaseStrategy {
        git,
        vcs: Some(vcs),
        parser: DefaultCommitParser,
        formatter,
        config,
        force,
    })
}

/// Returns true if the error represents "nothing to release" (as opposed to a real failure).
fn is_no_release_error(err: &anyhow::Error) -> bool {
    if let Some(re) = err.downcast_ref::<ReleaseError>() {
        matches!(
            re,
            ReleaseError::NoCommits { .. } | ReleaseError::NoBump { .. }
        )
    } else {
        false
    }
}

/// Load config and optionally resolve a package, returning the effective config.
fn load_config_for_package(package: Option<&str>) -> anyhow::Result<ReleaseConfig> {
    let config_path = resolve_config_path();
    let mut config = ReleaseConfig::load(&config_path)?;
    match package {
        Some(name) => {
            let pkg = config.find_package(name)?;
            Ok(config.resolve_package(pkg))
        }
        None => {
            // Auto-detect version files if none configured
            if config.version_files.is_empty() {
                config.version_files = sr_core::version_files::detect_version_files(Path::new("."));
            }
            Ok(config)
        }
    }
}

/// Find the config file, printing a deprecation warning if the legacy name is used.
fn resolve_config_path() -> std::path::PathBuf {
    match ReleaseConfig::find_config(Path::new(".")) {
        Some((path, is_legacy)) => {
            if is_legacy {
                eprintln!(
                    "warning: {} is deprecated, rename to {} (legacy support will be removed in a future release)",
                    LEGACY_CONFIG_FILE, DEFAULT_CONFIG_FILE,
                );
            }
            path
        }
        None => std::path::PathBuf::from(DEFAULT_CONFIG_FILE),
    }
}

/// Best-effort hook sync — keeps `.githooks/` in sync with `sr.yaml`.
/// Failures print a warning but never abort the calling command.
fn ensure_hooks_synced() {
    let config_path = resolve_config_path();
    if !config_path.exists() {
        return;
    }
    if let Ok(config) = ReleaseConfig::load(&config_path)
        && sr_core::hooks::needs_sync(Path::new("."), &config.hooks)
    {
        match sr_core::hooks::sync_hooks(Path::new("."), &config.hooks) {
            Ok(true) => eprintln!("hooks synced with {}", config_path.display()),
            Ok(false) => {}
            Err(e) => eprintln!("warning: failed to sync hooks: {e}"),
        }
    }
}

/// Build a JSON context object for a git hook based on its name and positional args.
fn build_hook_json(hook_name: &str, args: &[String]) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    obj.insert("hook".into(), serde_json::Value::String(hook_name.into()));
    obj.insert(
        "args".into(),
        serde_json::Value::Array(
            args.iter()
                .map(|a| serde_json::Value::String(a.clone()))
                .collect(),
        ),
    );

    // Add named fields for well-known hooks
    match hook_name {
        "commit-msg" => {
            if let Some(f) = args.first() {
                obj.insert("message_file".into(), serde_json::Value::String(f.clone()));
            }
        }
        "prepare-commit-msg" => {
            if let Some(f) = args.first() {
                obj.insert("message_file".into(), serde_json::Value::String(f.clone()));
            }
            if let Some(s) = args.get(1) {
                obj.insert("source".into(), serde_json::Value::String(s.clone()));
            }
            if let Some(s) = args.get(2) {
                obj.insert("sha".into(), serde_json::Value::String(s.clone()));
            }
        }
        "pre-push" => {
            if let Some(r) = args.first() {
                obj.insert("remote_name".into(), serde_json::Value::String(r.clone()));
            }
            if let Some(u) = args.get(1) {
                obj.insert("remote_url".into(), serde_json::Value::String(u.clone()));
            }
        }
        "pre-rebase" => {
            if let Some(u) = args.first() {
                obj.insert("upstream".into(), serde_json::Value::String(u.clone()));
            }
            if let Some(b) = args.get(1) {
                obj.insert("branch".into(), serde_json::Value::String(b.clone()));
            }
        }
        "post-checkout" => {
            if let Some(r) = args.first() {
                obj.insert("prev_ref".into(), serde_json::Value::String(r.clone()));
            }
            if let Some(r) = args.get(1) {
                obj.insert("new_ref".into(), serde_json::Value::String(r.clone()));
            }
            if let Some(f) = args.get(2) {
                obj.insert(
                    "branch_checkout".into(),
                    serde_json::Value::String(f.clone()),
                );
            }
        }
        "post-merge" => {
            if let Some(s) = args.first() {
                obj.insert("squash".into(), serde_json::Value::String(s.clone()));
            }
        }
        _ => {}
    }

    serde_json::Value::Object(obj)
}

/// Get staged files from git (excluding deletes).
fn staged_files() -> anyhow::Result<Vec<String>> {
    let output = std::process::Command::new("git")
        .args(["diff", "--cached", "--name-only", "--diff-filter=ACMR"])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect())
}

/// Match files against glob patterns. Matches against both the full path and the basename.
fn match_files(files: &[String], patterns: &[String]) -> Vec<String> {
    let compiled: Vec<glob::Pattern> = patterns
        .iter()
        .filter_map(|p| glob::Pattern::new(p).ok())
        .collect();

    files
        .iter()
        .filter(|f| {
            let basename = Path::new(f)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(f);
            compiled
                .iter()
                .any(|pat| pat.matches(f) || pat.matches(basename))
        })
        .cloned()
        .collect()
}

/// Run a single shell command, optionally piping JSON context to stdin.
fn run_shell_cmd(cmd: &str, json_str: Option<&str>) -> anyhow::Result<()> {
    let status = std::process::Command::new("sh")
        .args(["-c", cmd])
        .stdin(if json_str.is_some() {
            std::process::Stdio::piped()
        } else {
            std::process::Stdio::inherit()
        })
        .spawn()
        .and_then(|mut child| {
            if let Some(json) = json_str
                && let Some(ref mut stdin) = child.stdin
            {
                use std::io::Write;
                let _ = stdin.write_all(json.as_bytes());
            }
            child.wait()
        })?;

    if !status.success() {
        let code = status.code().unwrap_or(1);
        std::process::exit(code);
    }

    Ok(())
}

/// Run all entries for a configured hook.
///
/// Simple entries run as shell commands with JSON context piped to stdin.
/// Step entries match staged files against patterns and run rules only when matches exist.
/// Rules containing `{files}` receive the matched file list.
fn run_hook(config: &ReleaseConfig, hook_name: &str, args: &[String]) -> anyhow::Result<()> {
    let entries = config
        .hooks
        .hooks
        .get(hook_name)
        .ok_or_else(|| anyhow::anyhow!("no hook configured for '{hook_name}'"))?;

    if entries.is_empty() {
        return Ok(());
    }

    let json = build_hook_json(hook_name, args);
    let json_str = serde_json::to_string(&json)?;

    // Lazily fetch staged files only when a Step entry needs them.
    let mut cached_staged: Option<Vec<String>> = None;

    for entry in entries {
        match entry {
            HookEntry::Simple(cmd) => {
                run_shell_cmd(cmd, Some(&json_str))?;
            }
            HookEntry::Step {
                step,
                patterns,
                rules,
            } => {
                let all_staged =
                    cached_staged.get_or_insert_with(|| staged_files().unwrap_or_default());

                if all_staged.is_empty() {
                    eprintln!("{hook_name}: no staged files, skipping.");
                    return Ok(());
                }

                let matched = match_files(all_staged, patterns);
                if matched.is_empty() {
                    eprintln!("{hook_name} [{step}]: no files match {patterns:?}, skipping.");
                    continue;
                }

                let files_str = matched.join(" ");

                for rule in rules {
                    let cmd = if rule.contains("{files}") {
                        rule.replace("{files}", &files_str)
                    } else {
                        rule.clone()
                    };

                    eprintln!("{hook_name} [{step}]: {cmd}");
                    run_shell_cmd(&cmd, None)?;
                }
            }
        }
    }

    Ok(())
}

/// Validate a commit message file against the configured conventional commit pattern and types.
/// Reads hook JSON from stdin to get the message_file path.
fn validate_commit_msg(config: &ReleaseConfig) -> anyhow::Result<()> {
    use std::io::Read;
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;

    let json: serde_json::Value =
        serde_json::from_str(&input).map_err(|e| anyhow::anyhow!("invalid JSON on stdin: {e}"))?;

    let file = json["message_file"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing 'message_file' in hook JSON"))?;

    let content = std::fs::read_to_string(file)
        .map_err(|e| anyhow::anyhow!("cannot read commit message file: {e}"))?;

    let first_line = content.lines().next().unwrap_or("").trim();

    // Allow merge commits
    if first_line.starts_with("Merge ") {
        return Ok(());
    }

    // Allow fixup/squash/amend commits (from rebase -i)
    if first_line.starts_with("fixup! ")
        || first_line.starts_with("squash! ")
        || first_line.starts_with("amend! ")
    {
        return Ok(());
    }

    let re = regex::Regex::new(&config.commit_pattern)
        .map_err(|e| anyhow::anyhow!("invalid commit_pattern: {e}"))?;

    if !re.is_match(first_line) {
        let type_names: Vec<&str> = config.types.iter().map(|t| t.name.as_str()).collect();
        anyhow::bail!(
            "commit message does not follow Conventional Commits.\n\n\
             \x20 Expected: <type>(<scope>): <description>\n\
             \x20 Got:      {first_line}\n\n\
             \x20 Valid types: {}\n\
             \x20 Breaking:    append '!' before the colon, e.g. feat!: ...\n\n\
             \x20 Examples:\n\
             \x20   feat: add release dry-run flag\n\
             \x20   fix(core): handle empty tag list\n\
             \x20   feat!: redesign config format",
            type_names.join(", "),
        );
    }

    // Extract and validate the type
    if let Some(caps) = re.captures(first_line) {
        let msg_type = caps.name("type").map(|m| m.as_str()).unwrap_or_default();

        if !config.types.iter().any(|t| t.name == msg_type) {
            let type_names: Vec<&str> = config.types.iter().map(|t| t.name.as_str()).collect();
            anyhow::bail!(
                "commit type '{msg_type}' is not allowed.\n\n\
                 \x20 Valid types: {}",
                type_names.join(", "),
            );
        }
    }

    Ok(())
}

const INSTALL_SCRIPT_URL: &str = "https://raw.githubusercontent.com/urmzd/sr/main/install.sh";

/// Self-update sr by running the install script.
fn self_update() -> anyhow::Result<()> {
    eprintln!("current version: {}", env!("CARGO_PKG_VERSION"));

    // Resolve install dir to wherever the current binary lives
    let current_exe = std::env::current_exe()?;
    let install_dir = current_exe
        .parent()
        .ok_or_else(|| anyhow::anyhow!("cannot determine install directory"))?;

    let status = std::process::Command::new("sh")
        .args(["-c", &format!("curl -fsSL {INSTALL_SCRIPT_URL} | sh")])
        .env("SR_INSTALL_DIR", install_dir)
        .status()?;

    if !status.success() {
        anyhow::bail!("install script failed");
    }

    Ok(())
}

async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let backend_config = BackendConfig {
        backend: cli.backend,
        model: cli.model,
        budget: cli.budget,
        debug: cli.debug,
    };

    match cli.command {
        Commands::Init { force, merge } => {
            let path = Path::new(DEFAULT_CONFIG_FILE);

            if path.exists() && !force && !merge {
                anyhow::bail!(
                    "{DEFAULT_CONFIG_FILE} already exists (use --force to overwrite, or --merge to add new fields)"
                );
            }

            // Auto-detect version files in the current directory
            let detected = sr_core::version_files::detect_version_files(Path::new("."));
            if !detected.is_empty() {
                for f in &detected {
                    eprintln!("detected version file: {f}");
                }
            }

            if merge && path.exists() {
                let existing = std::fs::read_to_string(path)?;
                let merged = sr_core::config::merge_config_yaml(&existing)?;
                std::fs::write(path, merged)?;
                eprintln!("merged new defaults into {DEFAULT_CONFIG_FILE}");
            } else {
                let template = sr_core::config::default_config_template(&detected);
                std::fs::write(path, template)?;
                eprintln!("wrote {DEFAULT_CONFIG_FILE}");
            }

            let config = ReleaseConfig::load(path)?;
            sr_core::hooks::sync_hooks(Path::new("."), &config.hooks)?;

            Ok(())
        }

        Commands::Config { resolved } => {
            let config_path = resolve_config_path();
            let config = ReleaseConfig::load(&config_path)?;
            if resolved {
                let yaml = serde_yaml_ng::to_string(&config)?;
                print!("{yaml}");
            } else if config_path.exists() {
                let raw = std::fs::read_to_string(&config_path)?;
                print!("{raw}");
            } else {
                eprintln!("no config file found; showing defaults");
                let yaml = serde_yaml_ng::to_string(&config)?;
                print!("{yaml}");
            }
            Ok(())
        }

        Commands::Version { short, package } => {
            let config = load_config_for_package(package.as_deref())?;
            let strategy = build_local_strategy(config, false)?;
            let plan = strategy.plan()?;
            if short {
                println!("{}", plan.next_version);
            } else {
                println!(
                    "{} -> {} ({})",
                    plan.current_version
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "none".to_string()),
                    plan.next_version,
                    plan.bump
                );
            }
            Ok(())
        }

        Commands::Plan { format, package } => {
            let config = load_config_for_package(package.as_deref())?;
            let formatter = DefaultChangelogFormatter::new(
                config.changelog.template.clone(),
                config.types.clone(),
                config.breaking_section.clone(),
                config.misc_section.clone(),
            );
            let strategy = build_local_strategy(config, false)?;
            let plan = strategy.plan()?;

            let repo_url = NativeGitRepository::open(Path::new("."))
                .ok()
                .and_then(|git| git.parse_remote_full().ok())
                .map(|(hostname, owner, repo)| format!("https://{hostname}/{owner}/{repo}"));

            let today = sr_core::release::today_string();
            let entry = sr_core::changelog::ChangelogEntry {
                version: plan.next_version.to_string(),
                date: today,
                commits: plan.commits.clone(),
                compare_url: None,
                repo_url,
            };
            let changelog = sr_core::changelog::ChangelogFormatter::format(&formatter, &[entry])?;

            match format {
                PlanFormat::Json => {
                    #[derive(serde::Serialize)]
                    struct PlanOutput<'a> {
                        #[serde(flatten)]
                        plan: &'a sr_core::release::ReleasePlan,
                        changelog: String,
                    }
                    let output = PlanOutput {
                        plan: &plan,
                        changelog,
                    };
                    println!("{}", serde_json::to_string_pretty(&output)?);
                }
                PlanFormat::Human => {
                    println!("Next release: {}", plan.tag_name);
                    println!(
                        "Current version: {}",
                        plan.current_version
                            .as_ref()
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| "none".to_string())
                    );
                    println!("Next version: {}", plan.next_version);
                    println!("Bump: {}", plan.bump);
                    println!("Commits ({})", plan.commits.len());
                    for commit in &plan.commits {
                        let scope = commit
                            .scope
                            .as_deref()
                            .map(|s| format!("({s})"))
                            .unwrap_or_default();
                        let breaking = if commit.breaking { " BREAKING" } else { "" };
                        println!(
                            "  - {}{scope}: {}{breaking} ({})",
                            commit.r#type,
                            commit.description,
                            &commit.sha[..7.min(commit.sha.len())]
                        );
                    }
                    println!("\nChangelog preview:\n{changelog}");
                }
            }
            Ok(())
        }

        Commands::Changelog {
            write,
            regenerate,
            package,
        } => {
            let config = load_config_for_package(package.as_deref())?;

            let formatter = DefaultChangelogFormatter::new(
                config.changelog.template.clone(),
                config.types.clone(),
                config.breaking_section.clone(),
                config.misc_section.clone(),
            );

            let changelog = if regenerate {
                use sr_core::commit::CommitParser;
                use sr_core::git::GitRepository;

                let git = NativeGitRepository::open(Path::new("."))?;
                let repo_url = git
                    .parse_remote_full()
                    .ok()
                    .map(|(hostname, owner, repo)| format!("https://{hostname}/{owner}/{repo}"));

                let tags = git.all_tags(&config.tag_prefix)?;
                if tags.is_empty() {
                    anyhow::bail!("no tags found with prefix '{}'", config.tag_prefix);
                }

                let parser = DefaultCommitParser;
                let mut entries = Vec::new();

                for (i, tag) in tags.iter().enumerate() {
                    let from = if i == 0 {
                        None
                    } else {
                        Some(tags[i - 1].sha.as_str())
                    };
                    let raw_commits = if let Some(ref path) = config.path_filter {
                        git.commits_between_in_path(from, &tag.name, path)?
                    } else {
                        git.commits_between(from, &tag.name)?
                    };
                    let conventional: Vec<_> = raw_commits
                        .iter()
                        .filter(|c| !c.message.starts_with("chore(release):"))
                        .filter_map(|c| parser.parse(c).ok())
                        .collect();

                    let date = git.tag_date(&tag.name)?;
                    let compare_url = if i > 0 {
                        repo_url
                            .as_ref()
                            .map(|url| format!("{url}/compare/{}...{}", tags[i - 1].name, tag.name))
                    } else {
                        None
                    };

                    entries.push(sr_core::changelog::ChangelogEntry {
                        version: tag.version.to_string(),
                        date,
                        commits: conventional,
                        compare_url,
                        repo_url: repo_url.clone(),
                    });
                }

                // Newest first
                entries.reverse();

                sr_core::changelog::ChangelogFormatter::format(&formatter, &entries)?
            } else {
                let strategy = build_local_strategy(config.clone(), false)?;
                let plan = strategy.plan()?;

                let repo_url = NativeGitRepository::open(Path::new("."))
                    .ok()
                    .and_then(|git| git.parse_remote_full().ok())
                    .map(|(hostname, owner, repo)| format!("https://{hostname}/{owner}/{repo}"));

                let today = sr_core::release::today_string();
                let entry = sr_core::changelog::ChangelogEntry {
                    version: plan.next_version.to_string(),
                    date: today,
                    commits: plan.commits,
                    compare_url: None,
                    repo_url,
                };

                sr_core::changelog::ChangelogFormatter::format(&formatter, &[entry])?
            };

            if write {
                let file = config.changelog.file.as_deref().unwrap_or("CHANGELOG.md");
                let path = Path::new(file);
                if regenerate {
                    let content = format!("# Changelog\n\n{changelog}\n");
                    std::fs::write(path, content)?;
                } else {
                    let existing = if path.exists() {
                        std::fs::read_to_string(path)?
                    } else {
                        String::new()
                    };
                    let content = if existing.is_empty() {
                        format!("# Changelog\n\n{changelog}\n")
                    } else {
                        match existing.find("\n\n") {
                            Some(pos) => {
                                let (header, rest) = existing.split_at(pos);
                                format!("{header}\n\n{changelog}\n{rest}")
                            }
                            None => format!("{existing}\n\n{changelog}\n"),
                        }
                    };
                    std::fs::write(path, content)?;
                }
                eprintln!("wrote {file}");
            } else {
                println!("{changelog}");
            }
            Ok(())
        }

        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "sr", &mut std::io::stdout());
            Ok(())
        }

        Commands::Release {
            package,
            dry_run,
            artifacts,
            force,
            build_command,
            stage_files,
            pre_release_command,
            post_release_command,
            prerelease,
            sign_tags,
            draft,
        } => {
            ensure_hooks_synced();

            let mut config = load_config_for_package(package.as_deref())?;
            config.artifacts.extend(artifacts);
            config.stage_files.extend(stage_files);
            if build_command.is_some() {
                config.build_command = build_command;
            }
            if pre_release_command.is_some() {
                config.pre_release_command = pre_release_command;
            }
            if post_release_command.is_some() {
                config.post_release_command = post_release_command;
            }
            if prerelease.is_some() {
                config.prerelease = prerelease;
            }
            if sign_tags {
                config.sign_tags = true;
            }
            if draft {
                config.draft = true;
            }

            // Try to build with GitHub; fall back to local-only if no token
            let plan = match build_full_strategy(config.clone(), force) {
                Ok(strategy) => {
                    let plan = strategy.plan()?;
                    strategy.execute(&plan, dry_run)?;
                    plan
                }
                Err(e) => {
                    if dry_run {
                        eprintln!("warning: {e} (continuing dry-run without GitHub)");
                        let strategy = build_local_strategy(config, force)?;
                        let plan = strategy.plan()?;
                        strategy.execute(&plan, dry_run)?;
                        plan
                    } else {
                        return Err(e);
                    }
                }
            };
            // Print structured JSON to stdout (machine-readable; all logs go to stderr)
            #[derive(serde::Serialize)]
            struct ReleaseOutput {
                version: String,
                previous_version: String,
                tag: String,
                bump: String,
                floating_tag: String,
                commit_count: usize,
            }
            let output = ReleaseOutput {
                version: plan.next_version.to_string(),
                previous_version: plan
                    .current_version
                    .as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                tag: plan.tag_name.clone(),
                bump: plan.bump.to_string(),
                floating_tag: plan.floating_tag_name.as_deref().unwrap_or("").to_string(),
                commit_count: plan.commits.len(),
            };
            println!("{}", serde_json::to_string(&output)?);
            Ok(())
        }

        // --- AI-powered commands ---
        Commands::Commit(args) => {
            ensure_hooks_synced();
            sr_ai::commands::commit::run(&args, &backend_config).await
        }
        Commands::Rebase(args) => sr_ai::commands::rebase::run(&args, &backend_config).await,
        Commands::Review(args) => sr_ai::commands::review::run(&args, &backend_config).await,
        Commands::Explain(args) => sr_ai::commands::explain::run(&args, &backend_config).await,
        Commands::Branch(args) => sr_ai::commands::branch::run(&args, &backend_config).await,
        Commands::Pr(args) => sr_ai::commands::pr::run(&args, &backend_config).await,
        Commands::Ask(args) => sr_ai::commands::ask::run(&args, &backend_config).await,
        Commands::Cache(args) => sr_ai::commands::cache::run(&args),

        Commands::Hook { command } => match command {
            HookCommands::CommitMsg => {
                let config_path = resolve_config_path();
                let config = ReleaseConfig::load(&config_path)?;
                validate_commit_msg(&config)
            }
            HookCommands::Run { hook_name, args } => {
                let config_path = resolve_config_path();
                let config = ReleaseConfig::load(&config_path)?;
                run_hook(&config, &hook_name, &args)
            }
        },

        Commands::Update => self_update(),
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::from(0),
        Err(e) => {
            if is_no_release_error(&e) {
                eprintln!("{e:#}");
                ExitCode::from(2)
            } else {
                eprintln!("error: {e:#}");
                ExitCode::from(1)
            }
        }
    }
}
