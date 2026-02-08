use std::collections::HashMap;
use std::path::Path;

use clap::{CommandFactory, Parser, Subcommand};
use sr_core::changelog::DefaultChangelogFormatter;
use sr_core::commit::DefaultCommitParser;
use sr_core::config::ReleaseConfig;
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
    })
}

fn build_full_strategy(
    config: ReleaseConfig,
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
    })
}

/// Best-effort construction of a GitHubProvider for contributor resolution.
/// Returns None if the git remote can't be parsed (no remote, not GitHub, etc.).
fn try_github_provider() -> Option<GitHubProvider> {
    let git = NativeGitRepository::open(Path::new(".")).ok()?;
    let (owner, repo) = git.parse_remote().ok()?;
    Some(GitHubProvider::new(owner, repo))
}

fn main() -> anyhow::Result<()> {
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
            let strategy = build_local_strategy(config)?;
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
            let strategy = build_local_strategy(config)?;
            let plan = strategy.plan()?;

            let repo_url = NativeGitRepository::open(Path::new("."))
                .ok()
                .and_then(|git| git.parse_remote().ok())
                .map(|(owner, repo)| format!("https://github.com/{owner}/{repo}"));

            let today = sr_core::release::today_string();
            let mut entry = sr_core::changelog::ChangelogEntry {
                version: plan.next_version.to_string(),
                date: today,
                commits: plan.commits.clone(),
                compare_url: None,
                repo_url,
                contributor_map: HashMap::new(),
            };
            if let Some(provider) = try_github_provider() {
                let author_shas = entry.unique_author_shas();
                entry.contributor_map = provider.resolve_contributors(&author_shas);
            }
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
                        contributor_map: HashMap::new(),
                    });
                }

                // Resolve all unique authors across all entries in one batch
                if let Some(provider) = try_github_provider() {
                    let mut all_author_shas = Vec::new();
                    let mut seen = std::collections::BTreeSet::new();
                    for entry in &entries {
                        for (author, sha) in entry.unique_author_shas() {
                            if seen.insert(author.to_string()) {
                                all_author_shas.push((author.to_string(), sha.to_string()));
                            }
                        }
                    }
                    let refs: Vec<(&str, &str)> = all_author_shas
                        .iter()
                        .map(|(a, s)| (a.as_str(), s.as_str()))
                        .collect();
                    let shared_map = provider.resolve_contributors(&refs);
                    for entry in &mut entries {
                        entry.contributor_map = shared_map.clone();
                    }
                }

                // Newest first
                entries.reverse();

                sr_core::changelog::ChangelogFormatter::format(&formatter, &entries)?
            } else {
                let strategy = build_local_strategy(config.clone())?;
                let plan = strategy.plan()?;

                let repo_url = NativeGitRepository::open(Path::new("."))
                    .ok()
                    .and_then(|git| git.parse_remote().ok())
                    .map(|(owner, repo)| format!("https://github.com/{owner}/{repo}"));

                let today = sr_core::release::today_string();
                let mut entry = sr_core::changelog::ChangelogEntry {
                    version: plan.next_version.to_string(),
                    date: today,
                    commits: plan.commits,
                    compare_url: None,
                    repo_url,
                    contributor_map: HashMap::new(),
                };
                if let Some(provider) = try_github_provider() {
                    let author_shas = entry.unique_author_shas();
                    entry.contributor_map = provider.resolve_contributors(&author_shas);
                }

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

        Commands::Release { dry_run } => {
            let config = ReleaseConfig::load(Path::new(DEFAULT_CONFIG_FILE))?;

            // Try to build with GitHub; fall back to local-only if no token
            match build_full_strategy(config.clone()) {
                Ok(strategy) => {
                    let plan = strategy.plan()?;
                    strategy.execute(&plan, dry_run)?;
                }
                Err(e) => {
                    if dry_run {
                        eprintln!("warning: {e} (continuing dry-run without GitHub)");
                        let strategy = build_local_strategy(config)?;
                        let plan = strategy.plan()?;
                        strategy.execute(&plan, dry_run)?;
                    } else {
                        return Err(e);
                    }
                }
            }
            Ok(())
        }
    }
}
