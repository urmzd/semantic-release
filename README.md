# sr — Semantic Release

A single-binary, zero-dependency semantic release tool for any language.

[![CI](https://github.com/urmzd/semantic-release/actions/workflows/ci.yml/badge.svg)](https://github.com/urmzd/semantic-release/actions/workflows/ci.yml)

## Why?

The npm `semantic-release` ecosystem is battle-tested but comes with friction:

- **Requires Node.js** — even for Go, Rust, Python, and Java projects.
- **Complex plugin config** — wiring together `@semantic-release/*` packages is error-prone.
- **Coupled to CI runtime** — plugins shell out to language-specific toolchains at release time.

**sr** solves this:

- **Single static binary** — no runtime, no package manager, minimal dependencies.
- **Language-agnostic** — works with any project that uses git tags for versioning.
- **Zero-config defaults** — conventional commits + semver + GitHub releases out of the box.
- **Lifecycle hooks** — run arbitrary shell commands at each phase instead of installing plugins.

## Features

- Conventional Commits parsing (via `git-conventional`)
- Semantic versioning bumps (major / minor / patch)
- Changelog generation (Jinja2 templates via `minijinja`)
- GitHub Releases (via `gh` CLI)
- Lifecycle hooks (`pre_release`, `post_tag`, `post_release`, `on_failure`)
- Trunk-based workflow (tag + release from `main`)

## Installation

### GitHub Action (recommended)

```yaml
- uses: urmzd/semantic-release@v1
  with:
    github-token: ${{ secrets.GITHUB_TOKEN }}
```

### Usage

Minimal — release on every push to `main`:

```yaml
name: Release
on:
  push:
    branches: [main]

jobs:
  release:
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - uses: urmzd/semantic-release@v1
```

Dry-run on pull requests:

```yaml
      - uses: urmzd/semantic-release@v1
        with:
          command: release
          dry-run: "true"
```

Use outputs in subsequent steps:

```yaml
      - uses: urmzd/semantic-release@v1
        id: sr
      - if: steps.sr.outputs.released == 'true'
        run: echo "Released ${{ steps.sr.outputs.version }}"
```

#### Inputs

| Input | Description | Default |
|-------|-------------|---------|
| `command` | The `sr` subcommand to run (`release`, `plan`, `changelog`, `version`, `config`) | `release` |
| `dry-run` | Preview changes without executing them | `false` |
| `config` | Path to the config file | `.urmzd.sr.yml` |
| `github-token` | GitHub token for creating releases | `${{ github.token }}` |
| `git-user-name` | Git user name for tag creation | `semantic-release[bot]` |
| `git-user-email` | Git user email for tag creation | `semantic-release[bot]@urmzd.com` |

#### Outputs

| Output | Description |
|--------|-------------|
| `version` | The released version (empty if no release) |
| `released` | Whether a release was created (`true`/`false`) |

### Binary download

Download the latest release for your platform from
[Releases](https://github.com/urmzd/semantic-release/releases):

| Target | File |
|--------|------|
| Linux x86_64 | `sr-x86_64-unknown-linux-gnu` |
| Linux aarch64 | `sr-aarch64-unknown-linux-gnu` |
| macOS x86_64 | `sr-x86_64-apple-darwin` |
| macOS aarch64 | `sr-aarch64-apple-darwin` |

```bash
chmod +x sr-* && mv sr-* /usr/local/bin/sr
```

### Build from source

```bash
cargo install --path crates/sr-cli
```

## Prerequisites

`sr release` uses the [GitHub CLI (`gh`)](https://cli.github.com/) to create GitHub releases. It is pre-installed on all GitHub Actions runners. For local usage, install `gh` and authenticate:

```bash
gh auth login
```

The `gh` CLI reads the `GH_TOKEN` environment variable for authentication. The GitHub Action sets this automatically.

## Quick Start

```bash
# Generate a default config file
sr init

# Preview what the next release would look like
sr plan

# Execute the release
sr release
```

## CLI Reference

| Command | Description |
|---------|-------------|
| `sr release` | Execute a release (tag + GitHub release) |
| `sr plan` | Show what the next release would look like |
| `sr changelog` | Generate or preview the changelog |
| `sr version` | Show the next version |
| `sr config` | Validate and display resolved configuration |
| `sr init` | Create a default `.urmzd.sr.yml` config file |

### Common flags

- `sr release --dry-run` — preview without making changes
- `sr plan --format json` — machine-readable output
- `sr changelog --write` — write changelog to disk
- `sr version --short` — print only the version number
- `sr config --resolved` — show config with defaults applied
- `sr init --force` — overwrite existing config file

## Configuration

`sr` looks for `.urmzd.sr.yml` in the repository root. All fields are optional and have sensible defaults.

```yaml
# Branches that trigger releases
branches:
  - main

# Prefix for git tags
tag_prefix: "v"

# Changelog settings
changelog:
  file: CHANGELOG.md       # Path to the changelog file (optional)
  template: null            # Custom Jinja2 template (optional)

# Lifecycle hooks — shell commands run at each phase
hooks:
  pre_release:              # Before tagging
    - cargo test --workspace
  post_tag: []              # After the tag is created
  post_release: []          # After the GitHub release is published
  on_failure: []            # If any step fails

# Override commit-type to bump-level mapping (merged with defaults)
commit_types: {}
# Example:
#   docs: patch
#   refactor: patch
```

### Default commit-type mapping

| Type | Bump |
|------|------|
| `feat` | minor |
| `fix` | patch |
| `perf` | patch |
| Breaking change (`!`) | major |

All other types (e.g. `chore`, `docs`, `ci`) do not trigger a release unless overridden in `commit_types`.

## Architecture

```
crates/
  sr-core/     Pure domain logic — traits, config, versioning, changelog, hooks
  sr-git/      Git implementation (native git CLI)
  sr-github/   GitHub VCS provider (gh CLI)
  sr-cli/      CLI binary (clap) — wires everything together
action.yml     GitHub Action composite wrapper (repo root)
```

### Core traits

| Trait | Purpose |
|-------|---------|
| `GitRepository` | Tag discovery, commit listing, tag creation, push |
| `VcsProvider` | Remote release creation (GitHub, GitLab, etc.) |
| `CommitParser` | Raw commit to conventional commit |
| `ChangelogFormatter` | Render changelog entries to text |
| `HookRunner` | Execute lifecycle shell commands |
| `ReleaseStrategy` | Orchestrate plan + execute |

## Design Philosophy

1. **Trunk-based flow** — releases happen from a single branch; no release branches.
2. **Conventional commits as source of truth** — commit messages drive versioning.
3. **Zero-config** — works out of the box with reasonable defaults.
4. **Explicit over magic** — lifecycle hooks replace opaque plugins.
5. **Language-agnostic** — sr knows about git and semver, not about cargo or npm.

## Development

Requires [just](https://github.com/casey/just) for task running.

```bash
just init          # Install clippy + rustfmt
just check         # Run all checks (format, lint, test)
just build         # Build workspace
just test          # Run tests
just lint          # Run clippy
just fmt           # Format code
just run plan      # Run the CLI
```

See the [Justfile](Justfile) for all available recipes.

## License

[MIT](LICENSE)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, code style, and PR guidelines.
