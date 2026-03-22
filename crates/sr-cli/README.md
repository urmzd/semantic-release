# sr-cli

The CLI binary for [sr](https://github.com/urmzd/sr) — an AI-powered release engineering CLI.

[![crates.io](https://img.shields.io/crates/v/sr-cli.svg)](https://crates.io/crates/sr-cli)

## Installation

### From crates.io

```bash
cargo install sr-cli
```

### From source

```bash
cargo install --path crates/sr-cli
```

### Binary download

Pre-built binaries are available on the [releases page](https://github.com/urmzd/sr/releases).

## Quick Start

```bash
# AI-powered commits from your changes
sr commit

# AI code review
sr review

# Generate a PR
sr pr --create

# Preview the next release
sr plan

# Execute the release
sr release
```

## Commands

### AI commands

| Command | Description |
|---------|-------------|
| `sr commit` | Generate atomic commits from changes (AI-powered) |
| `sr rebase` | AI-powered interactive rebase (reword, squash, reorder) |
| `sr review` | AI code review of staged/branch changes |
| `sr explain` | Explain recent commits |
| `sr branch` | Suggest conventional branch name |
| `sr pr` | Generate PR title + body from branch commits |
| `sr ask` | Freeform Q&A about the repo |
| `sr cache` | Manage the AI commit plan cache |

### Release commands

| Command | Description |
|---------|-------------|
| `sr release` | Execute a release (tag + GitHub release) |
| `sr plan` | Show what the next release would look like |
| `sr changelog` | Generate or preview the changelog |
| `sr version` | Show the next version |
| `sr config` | Validate and display resolved configuration |
| `sr init` | Create a default `sr.yaml` config file |
| `sr completions` | Generate shell completions (bash, zsh, fish, powershell, elvish) |

### Global flags

| Flag | Env var | Description |
|------|---------|-------------|
| `--backend` | `SR_BACKEND` | AI backend: `claude`, `copilot`, or `gemini` |
| `--model` | `SR_MODEL` | AI model to use |
| `--budget` | `SR_BUDGET` | Max budget in USD (claude only, default: 0.50) |
| `--debug` | `SR_DEBUG` | Enable debug output |

## Configuration

`sr` reads `sr.yaml` from the repository root. See the [root README](https://github.com/urmzd/sr#configuration) for full configuration documentation.

## Prerequisites

- `git` — for all repository operations
- `GH_TOKEN` or `GITHUB_TOKEN` — for creating GitHub releases (set automatically on GitHub Actions runners)
- At least one AI backend CLI installed: `claude`, `gh copilot`, or `gemini` (for AI commands)

## License

[Apache-2.0](../../LICENSE)
