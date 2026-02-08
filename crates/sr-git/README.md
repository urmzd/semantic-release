# sr-git

Git operations for [sr](https://github.com/urmzd/semantic-release) — backed by the native `git` CLI.

[![crates.io](https://img.shields.io/crates/v/sr-git.svg)](https://crates.io/crates/sr-git)

## Overview

`sr-git` provides `NativeGitRepository`, a concrete implementation of the `GitRepository` trait from [`sr-core`](https://crates.io/crates/sr-core). It shells out to the `git` binary for all operations — tag discovery, commit listing, tagging, pushing, and staging.

## Usage

```toml
[dependencies]
sr-git = "0.1"
```

### Opening a repository

```rust
use sr_git::NativeGitRepository;
use sr_core::git::GitRepository;
use std::path::Path;

let repo = NativeGitRepository::open(Path::new("."))?;

// Use any GitRepository trait method
let tag = repo.latest_tag("v")?;
let commits = repo.commits_since(tag.as_ref().map(|t| t.name.as_str()))?;
```

### Parsing a remote URL

```rust
use sr_git::parse_owner_repo;

// Supports both SSH and HTTPS formats
let (owner, repo) = parse_owner_repo("git@github.com:urmzd/semantic-release.git")?;
assert_eq!(owner, "urmzd");
assert_eq!(repo, "semantic-release");

let (owner, repo) = parse_owner_repo("https://github.com/urmzd/semantic-release.git")?;
assert_eq!(owner, "urmzd");
assert_eq!(repo, "semantic-release");
```

## API

| Item | Description |
|------|-------------|
| `NativeGitRepository::open(path)` | Open a git repository at the given path |
| `NativeGitRepository::parse_remote()` | Extract `(owner, repo)` from the git remote URL |
| `parse_owner_repo(url)` | Standalone helper to parse owner/repo from a GitHub remote URL |

`NativeGitRepository` implements all methods of the `GitRepository` trait — see [`sr-core`](https://crates.io/crates/sr-core) for the full trait definition.

## Prerequisites

Requires `git` to be installed and available on `PATH`.

## License

[MIT](../../LICENSE)
