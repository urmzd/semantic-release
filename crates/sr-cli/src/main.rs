use std::path::Path;
use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand};
use sr_core::changelog::DefaultChangelogFormatter;
use sr_core::commit::DefaultCommitParser;
use sr_core::config::ReleaseConfig;
use sr_core::error::ReleaseError;
use sr_core::hooks::ShellHookRunner;
use sr_core::release::{ReleaseStrategy, TrunkReleaseStrategy, VcsProvider};
use sr_git::NativeGitRepository;
use sr_github::GitHubProvider;

const DEFAULT_CONFIG_FILE: &str = ".urmzd.sr.yml";

#[derive(Parser)]
#[command(name = "sr", about = "Semantic Release CLI", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute a release (trunk flow: tag + GitHub release)
    Release {
        /// Preview what would happen without making changes
        #[arg(long)]
        dry_run: bool,

        /// Glob patterns for artifact files to upload to the release (repeatable)
        #[arg(long = "artifacts")]
        artifacts: Vec<String>,

        /// Re-release the current tag (use when a previous release partially failed)
        #[arg(long)]
        force: bool,
    },

    /// Show what the next release would look like
    Plan {
        /// Output format
        #[arg(long, default_value = "human")]
        format: PlanFormat,
    },

    /// Generate or preview the changelog
    Changelog {
        /// Write the changelog to disk
        #[arg(long)]
        write: bool,

        /// Regenerate the entire changelog from all tags
        #[arg(long)]
        regenerate: bool,
    },

    /// Show the next version
    Version {
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

    /// Create a default configuration file
    Init {
        /// Overwrite the config file if it already exists
        #[arg(long)]
        force: bool,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: clap_complete::Shell,
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
        ShellHookRunner,
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
        hooks: ShellHookRunner,
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
        ShellHookRunner,
    >,
> {
    let git = NativeGitRepository::open(Path::new("."))?;
    let (owner, repo) = git.parse_remote()?;

    let vcs = GitHubProvider::new(owner, repo);
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
        hooks: ShellHookRunner,
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

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { force } => {
            let path = Path::new(DEFAULT_CONFIG_FILE);

            if path.exists() && !force {
                anyhow::bail!("{DEFAULT_CONFIG_FILE} already exists (use --force to overwrite)");
            }

            let config = ReleaseConfig::default();
            let yaml = serde_yaml_ng::to_string(&config)?;
            std::fs::write(path, yaml)?;

            eprintln!("wrote {DEFAULT_CONFIG_FILE}");
            Ok(())
        }

        Commands::Config { resolved } => {
            let config = ReleaseConfig::load(Path::new(DEFAULT_CONFIG_FILE))?;
            if resolved {
                let yaml = serde_yaml_ng::to_string(&config)?;
                print!("{yaml}");
            } else {
                let path = Path::new(DEFAULT_CONFIG_FILE);
                if path.exists() {
                    let raw = std::fs::read_to_string(path)?;
                    print!("{raw}");
                } else {
                    eprintln!("no config file found; showing defaults");
                    let yaml = serde_yaml_ng::to_string(&config)?;
                    print!("{yaml}");
                }
            }
            Ok(())
        }

        Commands::Version { short } => {
            let config = ReleaseConfig::load(Path::new(DEFAULT_CONFIG_FILE))?;
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

        Commands::Plan { format } => {
            let config = ReleaseConfig::load(Path::new(DEFAULT_CONFIG_FILE))?;
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
                .and_then(|git| git.parse_remote().ok())
                .map(|(owner, repo)| format!("https://github.com/{owner}/{repo}"));

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

        Commands::Changelog { write, regenerate } => {
            let config = ReleaseConfig::load(Path::new(DEFAULT_CONFIG_FILE))?;

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
                    .parse_remote()
                    .ok()
                    .map(|(owner, repo)| format!("https://github.com/{owner}/{repo}"));

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
                    let raw_commits = git.commits_between(from, &tag.name)?;
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
                    .and_then(|git| git.parse_remote().ok())
                    .map(|(owner, repo)| format!("https://github.com/{owner}/{repo}"));

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
            dry_run,
            artifacts,
            force,
        } => {
            let mut config = ReleaseConfig::load(Path::new(DEFAULT_CONFIG_FILE))?;
            config.artifacts.extend(artifacts);

            // Try to build with GitHub; fall back to local-only if no token
            let version = match build_full_strategy(config.clone(), force) {
                Ok(strategy) => {
                    let plan = strategy.plan()?;
                    let version = plan.next_version.to_string();
                    strategy.execute(&plan, dry_run)?;
                    version
                }
                Err(e) => {
                    if dry_run {
                        eprintln!("warning: {e} (continuing dry-run without GitHub)");
                        let strategy = build_local_strategy(config, force)?;
                        let plan = strategy.plan()?;
                        let version = plan.next_version.to_string();
                        strategy.execute(&plan, dry_run)?;
                        version
                    } else {
                        return Err(e);
                    }
                }
            };
            // Print version to stdout (machine-readable output; all other logs go to stderr)
            println!("{version}");
            Ok(())
        }
    }
}

fn main() -> ExitCode {
    match run() {
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
