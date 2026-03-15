---
name: semantic-release
description: Perform semantic releases with sr — version bumping, changelog generation, and GitHub releases from conventional commits. Use when releasing software, managing versions, or generating changelogs.
argument-hint: [command]
---

# Semantic Release

Perform a semantic release using `sr`.

## Steps

1. Ensure a `sr.yaml` config exists. If not, run `sr init`.
2. Preview what the release would look like: `sr plan`
3. To dry-run: `sr release --dry-run`
4. To execute: `sr release`
5. If `$ARGUMENTS` is provided, run `sr $ARGUMENTS` instead.

## Common Commands

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

## Release Execution Order

1. Pre-release command → 2. Bump version files → 3. Write changelog → 4. Build command → 5. Git commit → 6. Create/push tag (signed if configured) → 7. Floating tag → 8. Create/update GitHub release (draft if configured) → 9. Upload artifacts + SHA256 checksums → 10. Verify release → 11. Post-release command

## Environment

- `GH_TOKEN` / `GITHUB_TOKEN` — Required for GitHub releases
- `SR_VERSION` / `SR_TAG` — Set during hook commands

## Exit Codes

- `0` — Success
- `1` — Error (config, git, VCS)
- `2` — No releasable changes
