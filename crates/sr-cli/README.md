# sr-cli

The CLI binary for [sr](https://github.com/urmzd/semantic-release) — a single-binary, zero-dependency semantic release tool.

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

Pre-built binaries are available on the [releases page](https://github.com/urmzd/semantic-release/releases).

## Quick Start

```bash
# Create a default config file
sr init

# Preview the next release
sr plan

# Dry-run (no side effects)
sr release --dry-run

# Execute the release
sr release
```

## Commands

| Command | Description |
|---------|-------------|
| `sr release` | Execute a release (tag + GitHub release) |
| `sr plan` | Show what the next release would look like |
| `sr changelog` | Generate or preview the changelog |
| `sr version` | Show the next version |
| `sr config` | Validate and display resolved configuration |
| `sr init` | Create a default `.urmzd.sr.yml` config file |
| `sr completions` | Generate shell completions (bash, zsh, fish, powershell, elvish) |

### Common flags

| Flag | Description |
|------|-------------|
| `sr release --dry-run` | Preview without making changes |
| `sr plan --format json` | Machine-readable output |
| `sr changelog --write` | Write changelog to disk |
| `sr changelog --regenerate` | Rebuild entire changelog from all tags |
| `sr version --short` | Print only the version number |
| `sr config --resolved` | Show config with defaults applied |
| `sr init --force` | Overwrite existing config file |

## Configuration

`sr` reads `.urmzd.sr.yml` from the repository root. See the [root README](https://github.com/urmzd/semantic-release#configuration) for full configuration documentation.

## Prerequisites

- `git` — for all repository operations
- [`gh`](https://cli.github.com/) — for creating GitHub releases (pre-installed on GitHub Actions runners)

## License

[Apache-2.0](../../LICENSE)
