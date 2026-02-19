# sr-github

GitHub VCS provider for [sr](https://github.com/urmzd/semantic-release) — backed by the GitHub REST API.

[![crates.io](https://img.shields.io/crates/v/sr-github.svg)](https://crates.io/crates/sr-github)

## Overview

`sr-github` provides `GitHubProvider`, a concrete implementation of the `VcsProvider` trait from [`sr-core`](https://crates.io/crates/sr-core). It calls the GitHub REST API directly (via `ureq`) to create releases, upload assets, and check for existing releases — no external tools needed.

## Usage

```toml
[dependencies]
sr-github = "1"
```

### Creating a provider

```rust
use sr_github::GitHubProvider;
use sr_core::release::VcsProvider;

let provider = GitHubProvider::new(
    "urmzd".into(),
    "semantic-release".into(),
    "github.com".into(),
    std::env::var("GH_TOKEN").unwrap(),
);

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
| `GitHubProvider::new(owner, repo, hostname, token)` | Create a new provider for the given GitHub repository |
| `create_release(tag, name, body, prerelease)` | Create a GitHub release, returns the release URL |
| `release_exists(tag)` | Check whether a release already exists for a tag |
| `delete_release(tag)` | Delete a release by tag |
| `upload_assets(tag, files)` | Upload asset files to an existing release |
| `compare_url(base, head)` | Generate a GitHub compare URL between two refs |
| `repo_url()` | Return the repository URL (`https://github.com/owner/repo`) |

## Prerequisites

Requires a `GH_TOKEN` or `GITHUB_TOKEN` environment variable with a GitHub personal access token (or the `GITHUB_TOKEN` provided by GitHub Actions). The token needs `contents: write` permission to create releases and upload assets.

## License

[Apache-2.0](../../LICENSE)
