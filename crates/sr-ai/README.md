# sr-ai

AI backends, caching, and AI-powered git commands for [sr](https://github.com/urmzd/sr).

[![crates.io](https://img.shields.io/crates/v/sr-ai.svg)](https://crates.io/crates/sr-ai)

## Overview

`sr-ai` provides the AI layer for sr. It includes:

- **AI backends** — Claude, GitHub Copilot, and Gemini with automatic detection and fallback
- **Commands** — commit, review, explain, branch, pr, ask, cache
- **Caching** — fingerprint-based commit plan caching with incremental re-analysis

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
