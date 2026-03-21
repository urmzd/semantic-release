# sr-ai

AI backends, caching, and AI-powered git commands for [sr](https://github.com/urmzd/sr).

[![crates.io](https://img.shields.io/crates/v/sr-ai.svg)](https://crates.io/crates/sr-ai)

## Overview

`sr-ai` provides the AI layer for sr. It includes:

- **AI backends** — Claude, GitHub Copilot, and Gemini with automatic detection and fallback
- **Commands** — commit, review, explain, branch, pr, ask, cache
- **Caching** — fingerprint-based commit plan caching with incremental re-analysis

## Safety & Sandboxing

All AI backends are sandboxed to prevent the agent from modifying the repository:

- **Read-only git** — agents can only run `diff`, `log`, `show`, `status`, `ls-files`, `rev-parse`, `branch`, `cat-file`, `rev-list`, `shortlog`, `blame`. Mutating commands are blocked.
- **No shell access** — agents cannot run arbitrary shell commands or delete files.
- **Working tree snapshots** — `sr commit` snapshots the full working tree before invoking the agent. On failure, the snapshot is automatically restored. On success, it is cleared.
- **Programmatic mutations** — all git writes (staging, committing) happen in sr's Rust code after the agent returns, never inside the agent.

Snapshots are stored in the platform data directory (`~/.local/share/sr/snapshots/<repo-id>/` on Linux, `~/Library/Application Support/sr/snapshots/<repo-id>/` on macOS), keyed by a SHA-256 hash of the repository root path.

## AI Backends

| Backend | CLI required | Env var | Default model |
|---------|-------------|---------|---------------|
| Claude | `claude` | — | `haiku` |
| Copilot | `gh copilot` | — | `gpt-4.1` |
| Gemini | `gemini` | — | (default) |

Backends are auto-detected in order: Claude, Copilot, Gemini. Use `--backend` or `SR_BACKEND` to override.

## Commands

| Command | Description |
|---------|-------------|
| `commit` | Analyze changes and generate atomic conventional commits |
| `review` | AI code review with severity-based feedback |
| `explain` | Explain what a commit does and why |
| `branch` | Suggest a conventional branch name |
| `pr` | Generate PR title and body from branch commits |
| `ask` | Freeform Q&A about the repository |
| `cache` | Manage commit plan cache (status, clear) |

## Caching

Commit plans are cached at `~/.cache/sr/ai/<repo-id>/entries/`. Cache features:

- **Exact hit** — identical fingerprint match, reuse plan directly
- **Incremental hit** — partial match, AI re-analyzes only changed files
- **TTL** — entries expire after 24 hours
- **LRU** — max 20 entries per repo

## Usage

This crate is used as a library by `sr-cli`. It is not intended to be used directly, but can be embedded in other tools:

```toml
[dependencies]
sr-ai = "1"
```

## Prerequisites

At least one AI backend CLI must be installed:

- [Claude Code](https://docs.anthropic.com/en/docs/claude-code) (`claude`)
- [GitHub Copilot](https://docs.github.com/en/copilot) (`gh copilot`)
- [Gemini CLI](https://github.com/google-gemini/gemini-cli) (`gemini`)

## License

[Apache-2.0](../../LICENSE)
