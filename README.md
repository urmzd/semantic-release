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
- **Structured JSON output** — pipe `sr release` to `jq` for custom CI pipelines.

## Features

- Conventional Commits parsing (built-in, configurable via `commit_pattern`)
- Semantic versioning bumps (major / minor / patch)
- Automatic version file bumping (`Cargo.toml`, `package.json`, `pyproject.toml`)
- Changelog generation (Jinja2 templates via `minijinja`)
- GitHub Releases (via `gh` CLI)
- Structured JSON output for CI piping (`sr release | jq .version`)
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

Upload artifacts to the release:

```yaml
      # Build artifacts are downloaded into release-assets/
      - uses: actions/download-artifact@v4
        with:
          path: release-assets
          merge-multiple: true

      - uses: urmzd/semantic-release@v1
        with:
          artifacts: "release-assets/*"
```

The `artifacts` input accepts glob patterns (newline or comma separated). All matching files are uploaded to the GitHub release. This keeps artifact handling self-contained in the action — no separate upload steps needed.

Manual re-trigger with `workflow_dispatch` (useful when a previous release partially failed):

```yaml
name: Release
on:
  push:
    branches: [main]
  workflow_dispatch:
    inputs:
      force:
        description: "Re-release the current tag"
        type: boolean
        default: false

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
        with:
          force: ${{ github.event.inputs.force || 'false' }}
```

#### Inputs

| Input | Description | Default |
|-------|-------------|---------|
| `command` | The `sr` subcommand to run (`release`, `plan`, `changelog`, `version`, `config`, `completions`) | `release` |
| `dry-run` | Preview changes without executing them | `false` |
| `force` | Re-release the current tag (use when a previous release partially failed) | `false` |
| `config` | Path to the config file | `.urmzd.sr.yml` |
| `github-token` | GitHub token for creating releases | `${{ github.token }}` |
| `git-user-name` | Git user name for tag creation | `semantic-release[bot]` |
| `git-user-email` | Git user email for tag creation | `semantic-release[bot]@urmzd.com` |
| `artifacts` | Glob patterns for artifact files to upload (newline or comma separated) | `""` |

#### Outputs

| Output | Description |
|--------|-------------|
| `version` | The released version (empty if no release) |
| `previous-version` | The previous version before this release (empty if first release) |
| `tag` | The git tag created for this release (empty if no release) |
| `bump` | The bump level applied (`major`/`minor`/`patch`, empty if no release) |
| `floating-tag` | The floating major tag (e.g. `v3`, empty if disabled or no release) |
| `commit-count` | Number of commits included in this release |
| `released` | Whether a release was created (`true`/`false`) |
| `json` | Full release metadata as JSON (empty if no release) |

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

# Preview what the next release would look like (includes changelog)
sr plan

# Dry-run a release (no side effects)
sr release --dry-run

# Execute the release
sr release

# Set up shell completions (bash)
sr completions bash >> ~/.bashrc
```

## Developer Workflow

### Commit message validation

`sr` ships a `commit-msg` git hook that enforces [Conventional Commits](https://www.conventionalcommits.org/) at commit time. It reads allowed types and patterns from `.urmzd.sr.yml`, falling back to built-in defaults.

**Option 1 — Native git hooks:**

```bash
# Copy the hook into your project
curl -o .githooks/commit-msg https://raw.githubusercontent.com/urmzd/semantic-release/main/.githooks/commit-msg
chmod +x .githooks/commit-msg
git config core.hooksPath .githooks/
```

**Option 2 — pre-commit framework:**

```yaml
# .pre-commit-config.yaml
repos:
  - repo: https://github.com/urmzd/semantic-release
    rev: v0.5.0
    hooks:
      - id: conventional-commit-msg
```

The hook validates the first line of each commit message against the pattern `<type>(<scope>): <description>`. Merge commits and rebase-generated commits (`fixup!`, `squash!`, `amend!`) are always allowed through.

### End-to-end release flow

```
commit (hook validates) → push → sr plan (preview) → sr release (execute)
```

1. **Commit** — the commit-msg hook ensures every commit follows the conventional format (`feat:`, `fix:`, `feat!:`, etc.).
2. **Preview** — run `sr plan` to see the next version, included commits, and a changelog preview.
3. **Dry-run** — run `sr release --dry-run` to simulate the full release without side effects (no tags created).
4. **Release** — run `sr release` to execute the full pipeline:
   - Bumps version in configured manifest files
   - Generates and commits the changelog (with version files)
   - Creates and pushes the git tag
   - Creates a GitHub release
   - Outputs structured JSON to stdout (pipe to `jq` for custom workflows)

## CLI Reference

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

- `sr release --dry-run` — preview without making changes
- `sr release --force` — re-release the current tag (for partial failure recovery)
- `sr plan --format json` — machine-readable output
- `sr changelog --write` — write changelog to disk
- `sr version --short` — print only the version number
- `sr config --resolved` — show config with defaults applied
- `sr init --force` — overwrite existing config file
- `sr completions bash` — generate Bash completions

### Exit codes

| Code | Meaning |
|------|---------|
| `0` | Success — a release was created (or dry-run completed). The released version is printed to stdout. |
| `1` | Real error — configuration issue, git failure, VCS provider error, etc. |
| `2` | No releasable changes — no new commits or no releasable commit types since the last tag. |

### `--force` flag

Use `--force` to re-run a release that partially failed (e.g. the tag was created but artifact upload failed). Force mode only works when HEAD is exactly at the latest tag — it re-executes the release pipeline for that tag without bumping the version.

```bash
# Re-release the current tag after a partial failure
sr release --force
```

Force mode will error if:
- There are no tags yet (nothing to re-release)
- HEAD is not at the latest tag (there are new commits — use a normal release instead)

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

# Version files to bump automatically
version_files:
  - Cargo.toml
  # - package.json
  # - pyproject.toml

# Override commit-type to bump-level mapping (merged with defaults)
commit_types: {}
# Example:
#   docs: patch
#   refactor: patch
```

### Supported version files

| Filename | Key updated | Notes |
|---|---|---|
| `Cargo.toml` | `package.version` (or `workspace.package.version`) | Preserves formatting and comments |
| `package.json` | `version` | Pretty-printed JSON output |
| `pyproject.toml` | `project.version` (or `tool.poetry.version`) | Preserves formatting and comments |

### Default commit-type mapping

| Type | Bump |
|------|------|
| `feat` | minor |
| `fix` | patch |
| `perf` | patch |
| Breaking change (`!`) | major |

All other types (e.g. `chore`, `docs`, `ci`) do not trigger a release unless overridden in `commit_types`.

## Architecture

| Crate | Description |
|-------|-------------|
| [`sr-core`](crates/sr-core/) | Pure domain logic — traits, config, versioning, changelog |
| [`sr-git`](crates/sr-git/) | Git implementation (native `git` CLI) |
| [`sr-github`](crates/sr-github/) | GitHub VCS provider (`gh` CLI) |
| [`sr-cli`](crates/sr-cli/) | CLI binary (`clap`) — wires everything together |

`action.yml` in the repo root is the GitHub Action composite wrapper.

### Core traits

| Trait | Purpose |
|-------|---------|
| `GitRepository` | Tag discovery, commit listing, tag creation, push |
| `VcsProvider` | Remote release creation (GitHub, GitLab, etc.) |
| `CommitParser` | Raw commit to conventional commit |
| `ChangelogFormatter` | Render changelog entries to text |
| `ReleaseStrategy` | Orchestrate plan + execute |

## Design Philosophy

1. **Trunk-based flow** — releases happen from a single branch; no release branches.
2. **Conventional commits as source of truth** — commit messages drive versioning.
3. **Zero-config** — works out of the box with reasonable defaults.
4. **Focused scope** — sr handles versioning, tagging, changelog, and publishing. Pre-release validation and downstream actions belong in CI pipeline steps.
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
