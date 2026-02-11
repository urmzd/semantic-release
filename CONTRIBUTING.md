# Contributing

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable)
- [just](https://github.com/casey/just) (task runner)
- [GitHub CLI (`gh`)](https://cli.github.com/) (for release testing)
- Git

## Getting Started

```bash
git clone https://github.com/urmzd/semantic-release.git
cd semantic-release
just init    # Install clippy + rustfmt
just check   # Run all checks (format, lint, test)
```

## Development Workflow

| Task | Command |
|------|---------|
| Build workspace | `just build` |
| Run CLI | `just run plan` |
| Run tests | `just test` |
| Run clippy | `just lint` |
| Format code | `just fmt` |
| All checks | `just check` |
| Release build | `just install` |

## Git Hooks

This project uses a `commit-msg` hook to enforce Conventional Commits at commit time.

### Native git hooks

```bash
just install-hooks   # sets core.hooksPath to .githooks/
```

This is included in `just init`, so new contributors get it automatically.

### pre-commit framework

If you use the [pre-commit](https://pre-commit.com/) framework, add to your `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: https://github.com/urmzd/semantic-release
    rev: v0.5.0
    hooks:
      - id: conventional-commit-msg
```

### How it works

The hook reads `types` and `commit_pattern` from `.urmzd.sr.yml`. If the config file is missing it falls back to the built-in defaults.

- **Allowed types** are derived from the `types` list — only types defined there are accepted.
- **Pattern** is derived from `commit_pattern` (a regex with named groups `type`, `scope`, `breaking`, `description`).

Merge commits (`Merge ...`) and rebase-generated commits (`fixup!`, `squash!`, `amend!`) are always allowed through.

## Commit Messages

This project uses [Conventional Commits](https://www.conventionalcommits.org/).

```
<type>(<scope>): <description>
```

**Scope syntax:** `type(scope): description`

**Breaking changes:** `type!: description` or `type(scope)!: description`

| Type | Bump |
|------|------|
| `feat` | minor |
| `fix` | patch |
| `perf` | patch |
| Breaking (`!`) | major |

Other types (`chore`, `docs`, `ci`, `refactor`, `test`, `build`, `style`, `revert`) do not trigger a release unless configured in `types`.

## Pull Request Process

1. Fork the repo and create a branch from `main`.
2. Make your changes using conventional commits.
3. Run `just check` — all checks must pass.
4. Open a PR against `main`.

## Architecture

```
crates/
  sr-core/     Pure domain logic — traits, config, versioning, changelog
  sr-git/      Git implementation (native git CLI)
  sr-github/   GitHub VCS provider (gh CLI)
  sr-cli/      CLI binary (clap) — wires everything together
action.yml     GitHub Action composite wrapper (repo root)
```

### Core Traits

| Trait | Purpose |
|-------|---------|
| `GitRepository` | Tag discovery, commit listing, tag creation, push |
| `VcsProvider` | Remote release creation (GitHub, GitLab, etc.) |
| `CommitParser` | Raw commit to conventional commit |
| `CommitClassifier` | Single source of truth for type → bump level / changelog section |
| `ChangelogFormatter` | Render changelog entries to text |
| `ReleaseStrategy` | Orchestrate plan + execute |

## Code Style

- `rustfmt` for formatting (`just fmt`)
- `clippy` with `-D warnings` (`just lint`)
- No `unsafe` code
- Prefer `Result` over panics
