# sr-github

GitHub VCS provider for [sr](https://github.com/urmzd/semantic-release) — backed by the [`gh` CLI](https://cli.github.com/).

[![crates.io](https://img.shields.io/crates/v/sr-github.svg)](https://crates.io/crates/sr-github)

## Overview

`sr-github` provides `GitHubProvider`, a concrete implementation of the `VcsProvider` trait from [`sr-core`](https://crates.io/crates/sr-core). It uses the GitHub CLI (`gh`) to create releases and check for existing releases.

## Usage

```toml
[dependencies]
sr-github = "0.1"
```

### Creating a provider

```rust
use sr_github::GitHubProvider;
use sr_core::release::VcsProvider;

let provider = GitHubProvider::new("urmzd".into(), "semantic-release".into());

// Create a GitHub release
let url = provider.create_release(
    "v1.0.0",           // tag
    "v1.0.0",           // release name
    "## What's Changed", // body (markdown)
    false,               // prerelease
)?;

// Check if a release exists
let exists = provider.release_exists("v1.0.0")?;

// Generate a compare URL
let url = provider.compare_url("v0.9.0", "v1.0.0")?;
// -> "https://github.com/urmzd/semantic-release/compare/v0.9.0...v1.0.0"
```

## API

| Method | Description |
|--------|-------------|
| `GitHubProvider::new(owner, repo)` | Create a new provider for the given GitHub repository |
| `create_release(tag, name, body, prerelease)` | Create a GitHub release, returns the release URL |
| `release_exists(tag)` | Check whether a release already exists for a tag |
| `delete_release(tag)` | Delete a release by tag |
| `compare_url(base, head)` | Generate a GitHub compare URL between two refs |
| `repo_url()` | Return the repository URL (`https://github.com/owner/repo`) |

## Prerequisites

Requires the [GitHub CLI (`gh`)](https://cli.github.com/) to be installed, authenticated, and available on `PATH`. The `GH_TOKEN` or `GITHUB_TOKEN` environment variable can also be used for authentication.

## License

[MIT](../../LICENSE)
