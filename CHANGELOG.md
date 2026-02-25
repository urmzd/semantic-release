# Changelog

## 1.4.3 (2026-02-25)

### Bug Fixes

- write build-command to temp file to prevent outer shell expansion ([731f774](https://github.com/urmzd/semantic-release/commit/731f774e67fa916805095db13f7969e210a6aa06))

### Miscellaneous

- move crates.io publish to separate job so build is never blocked ([7cf95c1](https://github.com/urmzd/semantic-release/commit/7cf95c1a7a3c328e74d45cd7a574fc370570c023))
- lock file update ([d3f3f73](https://github.com/urmzd/semantic-release/commit/d3f3f7370c22e73139522b639510513100fe5c4b))


## 1.4.2 (2026-02-25)

### Bug Fixes

- don't let crates.io publish failure block binary uploads ([5c0275f](https://github.com/urmzd/semantic-release/commit/5c0275ff5748a0692a919cc56afd5f2c16029ed1))


## 1.4.1 (2026-02-25)

### Bug Fixes

- add force re-release support to workflow dispatch ([bf0b8e3](https://github.com/urmzd/semantic-release/commit/bf0b8e33283993120ec99a1cd5324ec61ba24357))


## 1.4.0 (2026-02-25)

### Features

- add build_command option and improve shell installer PATH setup ([df32798](https://github.com/urmzd/semantic-release/commit/df3279820d7065496eaeac6862240d8d515a9c78))

### Bug Fixes

- trigger binary builds from release workflow ([787cf1e](https://github.com/urmzd/semantic-release/commit/787cf1e89909f2b696ccf768cfbb290eb9408d0b))

### Documentation

- remove License section from README ([e5d24c1](https://github.com/urmzd/semantic-release/commit/e5d24c124acbb0d1f6ecd1d7531bc35c26024a54))

### Miscellaneous

- inline build matrix into release.yml, remove build.yml ([afc423f](https://github.com/urmzd/semantic-release/commit/afc423f4fb50e7ee97a6f7db8f9f63726265227d))
- add sensitive paths to .gitignore ([742bc42](https://github.com/urmzd/semantic-release/commit/742bc4257510b0b5d68eeb485939f2139dd5d5b4))


## 1.3.0 (2026-02-21)

### Features

- add shell installer, Windows release target, and license housekeeping ([039c9fe](https://github.com/urmzd/semantic-release/commit/039c9fe3a1d45ec348cf53b27feda6af3bae8acf))

### Miscellaneous

- cleanup cargo.lock ([f4097bf](https://github.com/urmzd/semantic-release/commit/f4097bf5afe1c798a07773ab4e68fbb1f46e8eb7))


## 1.2.0 (2026-02-19)

### Features

- replace gh CLI with direct GitHub REST API calls ([c04c82c](https://github.com/urmzd/semantic-release/commit/c04c82c48474ee458dae376ce5d70fbf96385977))
- prevent interactive git auth prompts and support CI runner credentials ([5f19d00](https://github.com/urmzd/semantic-release/commit/5f19d00e8ed0845100439b01666c4484e9066c1b))

### Bug Fixes

- clear existing git extraheader before injecting auth ([028e940](https://github.com/urmzd/semantic-release/commit/028e940dd9fe84f3d5157a7d45e8bcc1f490a377))


## 1.1.0 (2026-02-18)

### Features

- add GHES support, MUSL builds, ~/.local/bin install path, and post-release hooks docs ([32cdafa](https://github.com/urmzd/semantic-release/commit/32cdafadcc8b617c02c360b51e564a2437cbba96))

### Miscellaneous

- update README examples to reference v1 ([bf6e998](https://github.com/urmzd/semantic-release/commit/bf6e9986b88c04e31605613d61f1fde8a5763795))
- update Cargo.toml license to Apache-2.0 ([b41d16a](https://github.com/urmzd/semantic-release/commit/b41d16ac4b2afcfacbb69dc0b848eb945a08607f))
- license under Apache 2.0 ([039c7fb](https://github.com/urmzd/semantic-release/commit/039c7fb4edeedbaced90999a85230700410df7f0))


## 1.0.0 (2026-02-11)

### Breaking Changes

- remove lifecycle hooks, output structured JSON from sr release ([56e5ed0](https://github.com/urmzd/semantic-release/commit/56e5ed0125c64455129f40a15ab70950aeb2e349))

### Bug Fixes

- add floating_tags, parse JSON output in self-release workflow ([70e0f04](https://github.com/urmzd/semantic-release/commit/70e0f04f4b5abbd621821637c78c851ede770ac5))


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
