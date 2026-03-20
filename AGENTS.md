# AGENTS.md

## Identity

You are an agent working on **sr** — an AI-powered release engineering CLI. It handles the full lifecycle from commit to release: AI-powered commits, code review, PR generation, conventional commits, semantic versioning, changelog generation, and GitHub releases.

## Architecture

Rust workspace with five crates:

| Crate | Role |
|-------|------|
| `sr-core` | Pure domain logic — traits, config, versioning, changelog |
| `sr-git` | Git implementation (native `git` CLI via `NativeGitRepository`) |
| `sr-github` | GitHub VCS provider (REST API via `ureq`) |
| `sr-ai` | AI backends (Claude, Copilot, Gemini), caching, and AI-powered git commands |
| `sr-cli` | CLI binary (`clap`) — wires everything together |

### Core Traits

| Trait | Purpose |
|-------|---------|
| `GitRepository` | Tag discovery, commit listing, tag creation, push |
| `VcsProvider` | Remote release creation/update, asset uploads, verification |
| `CommitParser` | Raw commit → conventional commit |
| `ChangelogFormatter` | Render changelog entries to text |
| `ReleaseStrategy` | Orchestrate plan + execute |

## Key Files

- `crates/sr-cli/src/main.rs` — CLI entry point (async, wires all crates)
- `crates/sr-core/src/` — Domain logic, config, versioning, changelog
- `crates/sr-git/src/` — `NativeGitRepository` implementation
- `crates/sr-github/src/` — `VcsProvider` GitHub implementation
- `crates/sr-ai/src/ai/` — AI backends (Claude, Copilot, Gemini)
- `crates/sr-ai/src/commands/` — AI-powered commands (commit, review, explain, branch, pr, ask, cache)
- `crates/sr-ai/src/cache/` — Commit plan caching with fingerprinting
- `action.yml` — GitHub Action composite wrapper
- `sr.yaml` — Config file format

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

## Monorepo Support

`sr` supports monorepos via the `packages` config. Each package defines a `name`, `path`, and optional overrides. Key implementation details:

- **Config**: `PackageConfig` in `sr-core/src/config.rs`. `ReleaseConfig::resolve_package()` merges package overrides with root config.
- **Path filtering**: `GitRepository::commits_since_in_path()` and `commits_between_in_path()` filter commits by directory using `git log -- <path>`.
- **Tag scoping**: Each package uses its own tag prefix (default: `{name}/v`), e.g. `core/v1.2.0`.
- **CLI**: `-p/--package` flag on `release`, `plan`, `version`, and `changelog` commands.
- When `packages` is empty, behavior is unchanged (single-package mode).
