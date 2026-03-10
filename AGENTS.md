# AGENTS.md

## Identity

You are an agent working on **sr** (Semantic Release) — a single-binary, zero-dependency semantic release tool for any language. It handles conventional commits, semantic versioning, changelog generation, and GitHub releases.

## Architecture

Rust workspace with four crates:

| Crate | Role |
|-------|------|
| `sr-core` | Pure domain logic — traits, config, versioning, changelog |
| `sr-git` | Git implementation (native `git` CLI via `NativeGitRepository`) |
| `sr-github` | GitHub VCS provider (REST API via `ureq`) |
| `sr-cli` | CLI binary (`clap`) — wires everything together |

### Core Traits

| Trait | Purpose |
|-------|---------|
| `GitRepository` | Tag discovery, commit listing, tag creation, push |
| `VcsProvider` | Remote release creation (GitHub releases) |
| `CommitParser` | Raw commit → conventional commit |
| `ChangelogFormatter` | Render changelog entries to text |
| `ReleaseStrategy` | Orchestrate plan + execute |

## Key Files

- `crates/sr-cli/src/main.rs` — CLI entry point
- `crates/sr-core/src/` — Domain logic, config, versioning, changelog
- `crates/sr-git/src/` — `NativeGitRepository` implementation
- `crates/sr-github/src/` — `VcsProvider` GitHub implementation
- `action.yml` — GitHub Action composite wrapper
- `.urmzd.sr.yml` — Config file format

## Commands

| Task | Command |
|------|---------|
| Build | `just build` or `cargo build --workspace` |
| Test | `just test` or `cargo test --workspace` |
| Lint | `just lint` or `cargo clippy --workspace -- -D warnings` |
| Format | `just fmt` or `cargo fmt --all` |
| Check format | `just check-fmt` |
| Install binary | `just install` or `cargo build --release -p sr-cli` |
| Run CLI | `just run <ARGS>` or `cargo run -p sr-cli -- <ARGS>` |
| Full check | `just check` (format + lint + test) |

## Code Style

- Rust 2024 edition, Apache-2.0 license
- `cargo fmt` and `cargo clippy -- -D warnings` enforced via `.githooks/`
- TOML editing with `toml_edit` (preserves formatting/comments)
- Templating with `minijinja` for changelogs
- Workspace version: all crates share `workspace.package.version`

## Supported Version Files

Cargo.toml, package.json, pyproject.toml, pom.xml, build.gradle(.kts), `*.go` (Version var/const).

## Adding a New VCS Provider

1. Create a new crate under `crates/sr-<provider>/`
2. Implement the `VcsProvider` trait from `sr-core`
3. Wire it into `sr-cli` as a new backend option
4. Add integration tests

## Adding a New Version File Format

1. Edit `sr-core/src/` where version file bumping is implemented
2. Add a new match arm for the file extension/name
3. Add tests with sample files
