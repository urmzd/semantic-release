# sr-core

Pure domain logic for [sr](https://github.com/urmzd/semantic-release) — a single-binary semantic release tool.

[![crates.io](https://img.shields.io/crates/v/sr-core.svg)](https://crates.io/crates/sr-core)

## Overview

`sr-core` provides the traits, types, and functions that power semantic versioning releases. It contains **no I/O** — all git operations, VCS provider calls, and shell execution are abstracted behind traits so that consumers can supply their own implementations.

## Traits

| Trait | Purpose |
|-------|---------|
| `GitRepository` | Tag discovery, commit listing, tag/push operations |
| `VcsProvider` | Remote release creation (GitHub, GitLab, etc.) |
| `CommitParser` | Parse raw commits into conventional commits |
| `CommitClassifier` | Map commit types to bump levels and changelog sections |
| `ChangelogFormatter` | Render changelog entries to text |
| `ReleaseStrategy` | Orchestrate plan + execute |

## Key Types

| Type | Description |
|------|-------------|
| `ReleaseConfig` | Full configuration (branches, tag prefix, version files, etc.) |
| `ReleasePlan` | The computed next release (current version, next version, bump level, commits) |
| `ConventionalCommit` | A parsed conventional commit (type, scope, description, breaking flag) |
| `BumpLevel` | `Patch`, `Minor`, or `Major` |
| `ChangelogEntry` | A single version's changelog data (version, date, commits, compare URL) |
| `ReleaseError` | Unified error type for all release operations |
| `TrunkReleaseStrategy` | Default `ReleaseStrategy` implementation wiring all traits together |

## Key Functions

| Function | Description |
|----------|-------------|
| `determine_bump(commits, classifier)` | Compute the highest bump level from a set of conventional commits |
| `apply_bump(version, bump)` | Apply a bump level to a semver `Version`, returning the new version |
| `bump_version_file(path, new_version)` | Update the version field in `Cargo.toml`, `package.json`, or `pyproject.toml` |

## Usage

Add `sr-core` to your `Cargo.toml`:

```toml
[dependencies]
sr-core = "0.1"
```

### Building a custom release strategy

```rust
use sr_core::config::ReleaseConfig;
use sr_core::release::TrunkReleaseStrategy;
use sr_core::commit::{DefaultCommitParser, DefaultCommitClassifier};
use sr_core::changelog::DefaultChangelogFormatter;
use sr_core::release::ReleaseStrategy;

// Load configuration
let config = ReleaseConfig::load(Path::new(".urmzd.sr.yml")).unwrap();

// Build the strategy with your own GitRepository and VcsProvider implementations
let strategy = TrunkReleaseStrategy {
    git: my_git_impl,
    vcs: Some(my_vcs_provider),
    parser: DefaultCommitParser,
    formatter: DefaultChangelogFormatter::new(
        config.changelog.template.clone(),
        config.types.clone(),
        config.breaking_section.clone(),
        config.misc_section.clone(),
    ),
    config: config.clone(),
    force: false,
};

// Plan the release
let plan = strategy.plan()?;

// Execute (or dry-run)
strategy.execute(&plan, /* dry_run */ false)?;
```

### Computing a version bump

```rust
use sr_core::version::{determine_bump, apply_bump};
use sr_core::commit::DefaultCommitClassifier;
use semver::Version;

let classifier = DefaultCommitClassifier::default();
if let Some(bump) = determine_bump(&commits, &classifier) {
    let current = Version::new(1, 2, 3);
    let next = apply_bump(&current, bump);
    println!("{current} -> {next}"); // e.g. 1.2.3 -> 1.3.0
}
```

## License

[Apache-2.0](../../LICENSE)
