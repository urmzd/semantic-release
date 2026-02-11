# Changelog

## 0.10.0 (2026-02-11)

### Features

- add --force flag, structured errors, distinct exit codes, and action outputs ([ea0ba9c](https://github.com/urmzd/semantic-release/commit/ea0ba9cd416465ad5d6d09642b517f232756f557))

### Refactoring

- remove contributors section from release notes ([2831757](https://github.com/urmzd/semantic-release/commit/2831757c0dd85cf63958bf212b4288e229bfffa8))


## 0.9.0 (2026-02-09)

### Features

- add floating major version tags support ([ece1331](https://github.com/urmzd/semantic-release/commit/ece133156582e5fd13a36027e630a524fe2cfa91))

### Contributors

- @urmzd


## 0.8.0 (2026-02-08)

### Features

- add release artifact upload support ([3fa9ea7](https://github.com/urmzd/semantic-release/commit/3fa9ea720e48af6728c799c251c3075bf944a6f7))

### Bug Fixes

- update workspace dependency versions during Cargo.toml version bump ([c7a6634](https://github.com/urmzd/semantic-release/commit/c7a66349a828905f5ab473c9490bd01f035cb088))

### Miscellaneous

- fix clippy warnings, apply cargo fmt, and fix commit-msg hook PCRE parsing ([2611a1c](https://github.com/urmzd/semantic-release/commit/2611a1caf59717a91cf9d005646582b59c95ea57))
- update Cargo.lock for v0.7.0 ([e56772c](https://github.com/urmzd/semantic-release/commit/e56772cde97f758146d0b8dc50f4ad42786fd8fa))

### Contributors

- @urmzd


## 0.7.0 (2026-02-08)

### Features

- expand version file support, add lenient mode, fix root Cargo.toml ([487e5f4](https://github.com/urmzd/semantic-release/commit/487e5f4aa21a1d9e22298efebe310be0f193297c))

### Bug Fixes

- use tempdir in release tests to prevent changelog file pollution ([52e854d](https://github.com/urmzd/semantic-release/commit/52e854d2ccc13dea844d6cfe74e8e233063c24f8))
- add skip-ci flag to release commit messages to avoid redundant CI runs ([87160a8](https://github.com/urmzd/semantic-release/commit/87160a89b2650d52cacfd23db6c46dcaa3c7ffa8))

### Miscellaneous

- add floating v0 tag, update docs, and prepare crates.io publishing ([bda72c2](https://github.com/urmzd/semantic-release/commit/bda72c2242f9119c99ff8850736c64255fa2ff0b))

### Contributors

- @urmzd


## 0.6.0 (2026-02-08)

### Features

- tag contributors with GitHub @username in changelog ([bc13a08](https://github.com/urmzd/semantic-release/commit/bc13a08f8e43eafa9526dbb0607cee9efb3f81b9))

### Documentation

- regenerate changelog with GitHub @username contributors ([1d2906a](https://github.com/urmzd/semantic-release/commit/1d2906a7f71f53c1eb72865bd5cf0c14afbf49aa))

### Miscellaneous

- fix formatting in changelog contributor rendering ([2276d07](https://github.com/urmzd/semantic-release/commit/2276d07facda43f6ab56c32228e47db16f7f7634))

### Contributors

- @urmzd


## 0.5.0 (2026-02-08)

### Features

- add changelog --regenerate flag and improve changelog sections ([799c0f7](https://github.com/urmzd/semantic-release/commit/799c0f7854956f79554e712bfb07beccf6a83c47))

### Miscellaneous

- remove redundant push trigger from CI workflow ([4ece192](https://github.com/urmzd/semantic-release/commit/4ece1927a60234a4335eaf5aef97a691d14e159d))

### Contributors

- @urmzd

[Full Changelog](https://github.com/urmzd/semantic-release/compare/v0.4.0...v0.5.0)

## 0.4.0 (2026-02-08)

### Features

- add hook context, version file bumping, and changelog SHA links ([d95ff03](https://github.com/urmzd/semantic-release/commit/d95ff0373361fc0b04116f9a157f5dd121e76f36))

### Documentation

- update README with version files, completions, and developer workflow ([87fe902](https://github.com/urmzd/semantic-release/commit/87fe90200d3dd965e00aa2a3236aa8fe7fc1e62b))

### Refactoring

- replace git-conventional with built-in regex commit parser ([a687a1c](https://github.com/urmzd/semantic-release/commit/a687a1c4974d0215866f9f304b92ee975324455f))

### Miscellaneous

- fix formatting and clippy warnings, add pre-commit hook ([5bb14a9](https://github.com/urmzd/semantic-release/commit/5bb14a98cf3ff2f9e25d26681deed614aac1eca0))

### Contributors

- @urmzd

[Full Changelog](https://github.com/urmzd/semantic-release/compare/v0.3.0...v0.4.0)

## 0.3.0 (2026-02-08)

### Features

- add shell completions, enhance plan preview, fix dry-run hooks ([9fd9483](https://github.com/urmzd/semantic-release/commit/9fd9483b4e5af9600c2b22f18f558bc588c63e7b))

### Contributors

- @urmzd

[Full Changelog](https://github.com/urmzd/semantic-release/compare/v0.2.0...v0.3.0)

## 0.2.0 (2026-02-08)

### Features

- make release execution idempotent with CHANGELOG commit ([c78a302](https://github.com/urmzd/semantic-release/commit/c78a302584b3ad64ca57d7d1252639324a9e50ac))

### Contributors

- @urmzd

[Full Changelog](https://github.com/urmzd/semantic-release/compare/v0.1.0...v0.2.0)

## 0.1.0 (2026-02-07)

### Features

- initial implementation of semantic-release toolchain ([45eaa61](https://github.com/urmzd/semantic-release/commit/45eaa6164e797045855f056ecd88e15b5dd08437))

### Bug Fixes

- configure git identity for tag creation in CI ([3d76f38](https://github.com/urmzd/semantic-release/commit/3d76f38211cd0c5b78de9b937cc317fae57cd302))

### Documentation

- add action usage examples, branding, and marketplace metadata ([38a3ed3](https://github.com/urmzd/semantic-release/commit/38a3ed306dc2e7865937d74375a1fb3e15317365))

### Refactoring

- replace octocrab with gh CLI, fix macOS runners, move action.yml to root ([b66bb29](https://github.com/urmzd/semantic-release/commit/b66bb29663bda335d3a18607cf22457f2faf4132))

### Miscellaneous

- fix cargo fmt formatting in sr-github ([ed3b56c](https://github.com/urmzd/semantic-release/commit/ed3b56ca781cdf1ce8e14fce26f2bce2955d11b8))

### Contributors

- @urmzd
