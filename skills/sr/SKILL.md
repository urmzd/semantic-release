---
name: sr
description: AI-powered release engineering — from commit to release. AI-powered commits, code review, PR generation, version bumping, changelog generation, and GitHub releases from conventional commits.
argument-hint: [command]
---

# sr — AI-powered Release Engineering

Use `sr` to manage the full release lifecycle.

## Steps

1. Ensure a `sr.yaml` config exists. If not, run `sr init`.
2. If `$ARGUMENTS` is provided, run `sr $ARGUMENTS` instead of the default flow.
3. Default flow: preview with `sr plan`, then execute with `sr release`.

## AI Commands

| Command | Description |
|---------|-------------|
| `sr commit` | Generate atomic conventional commits from changes |
| `sr commit --staged` | Only analyze staged changes |
| `sr commit --dry-run` | Preview commit plan without executing |
| `sr review` | AI code review of staged/branch changes |
| `sr review --base main` | Review against a specific base ref |
| `sr explain` | Explain recent commits |
| `sr branch` | Suggest conventional branch name |
| `sr branch --create` | Create the suggested branch |
| `sr pr` | Generate PR title + body from branch commits |
| `sr pr --create` | Create the PR via gh CLI |
| `sr ask <question>` | Freeform Q&A about the repo |
| `sr cache status` | Show cached commit plans |
| `sr cache clear` | Clear cached entries |

## Release Commands

| Command | Description |
|---------|-------------|
| `sr plan` | Preview next release (version, commits, changelog) |
| `sr release` | Execute a full release |
| `sr release --dry-run` | Simulate without side effects |
| `sr release --force` | Re-release current tag (partial failure recovery) |
| `sr release --sign-tags` | Sign tags with GPG/SSH |
| `sr release --draft` | Create GitHub release as draft |
| `sr changelog --write` | Write changelog to disk |
| `sr version --short` | Print next version number |
| `sr config --resolved` | Show resolved config with defaults |

## Global Flags

| Flag | Env var | Description |
|------|---------|-------------|
| `--backend` | `SR_BACKEND` | AI backend: `claude`, `copilot`, or `gemini` |
| `--model` | `SR_MODEL` | AI model to use |
| `--budget` | `SR_BUDGET` | Max budget in USD (claude only) |
| `--debug` | `SR_DEBUG` | Enable debug output |

## Release Execution Order

1. Pre-release command → 2. Bump version files → 3. Write changelog → 4. Build command → 5. Git commit → 6. Create/push tag (signed if configured) → 7. Floating tag → 8. Create/update GitHub release (draft if configured) → 9. Upload artifacts + SHA256 checksums → 10. Verify release → 11. Post-release command

## Environment

- `GH_TOKEN` / `GITHUB_TOKEN` — Required for GitHub releases
- `SR_VERSION` / `SR_TAG` — Set during hook commands
- `SR_BACKEND` / `SR_MODEL` / `SR_BUDGET` / `SR_DEBUG` — AI configuration

## Exit Codes

- `0` — Success
- `1` — Error (config, git, VCS, AI)
- `2` — No releasable changes
