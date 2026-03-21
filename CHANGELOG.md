# Changelog

## 2.1.0 (2026-03-21)

### Features

- auto-detect version files in config and cli ([05d6f82](https://github.com/urmzd/sr/commit/05d6f82f2af810662946dd8f3dfc8bdb22e4c422))

### Documentation

- add demonstration screenshots ([9d78bd5](https://github.com/urmzd/sr/commit/9d78bd5e2cac5a93119770a455f157e207a29f57))

### Refactoring

- **release**: simplify supported file check using new abstraction ([452900b](https://github.com/urmzd/sr/commit/452900bd29e2b0fca3c9ef175390985c7e6f0535))
- **core**: introduce VersionFileHandler trait for version file handling ([c1c6620](https://github.com/urmzd/sr/commit/c1c66208590d51955b20f2e964ffc12ce9c15cc4))

### Miscellaneous

- update teasr configuration ([ff22537](https://github.com/urmzd/sr/commit/ff22537b1419996864d48c090deb196ef6eaccea))

[Full Changelog](https://github.com/urmzd/sr/compare/v2.0.0...v2.1.0)


## 2.0.0 (2026-03-21)

### Breaking Changes

- **ai**: sandbox agent backends to read-only git access with working tree snapshots ([a8937ed](https://github.com/urmzd/sr/commit/a8937ed34df8f5947b6eac6e951ec207e0e6bb2b))

[Full Changelog](https://github.com/urmzd/sr/compare/v1.13.0...v2.0.0)


## 1.13.0 (2026-03-20)

### Features

- **release**: auto-stage lock files after version bump ([8a71452](https://github.com/urmzd/sr/commit/8a71452db9979f2981b5b75ec24c29fc279d9596))

[Full Changelog](https://github.com/urmzd/sr/compare/v1.12.0...v1.13.0)


## 1.12.0 (2026-03-20)

### Features

- **version-files**: auto-discover and bump workspace members ([b28c18e](https://github.com/urmzd/sr/commit/b28c18e6426a4fc077d984b69ff4e7f9179f237b))

[Full Changelog](https://github.com/urmzd/sr/compare/v1.11.0...v1.12.0)


## 1.11.0 (2026-03-20)

### Features

- **core**: add monorepo support with per-package releases ([6b56089](https://github.com/urmzd/sr/commit/6b56089d3d0bd2576e19ff62f29156aef7a795e7))

### Documentation

- add monorepo support documentation ([ef78791](https://github.com/urmzd/sr/commit/ef78791d1a509b0bf5eefb306a3f88c9661d1820))

[Full Changelog](https://github.com/urmzd/sr/compare/v1.10.0...v1.11.0)


## 1.10.0 (2026-03-20)

### Features

- **git**: add head_short and file_statuses methods ([e512a4c](https://github.com/urmzd/sr/commit/e512a4c2b7211e1f001191b85c691209ab4a8c78))
- **commands/commit**: add config-driven commit types and real-time event display ([deb0ea8](https://github.com/urmzd/sr/commit/deb0ea849a8892b3c9ba86209f48bc851f88c2a1))
- **ai**: refactor backends with streaming and event support ([4bed74f](https://github.com/urmzd/sr/commit/4bed74fd185942c6b10d0baeb095b5145294fc98))
- **ai**: add sr-core dependency to sr-ai ([c9e2e38](https://github.com/urmzd/sr/commit/c9e2e38c5185a665bf5014d9f5c0b4b5d3bae882))

### Refactoring

- **ui**: redesign output with improved styling and interactive progress ([a679810](https://github.com/urmzd/sr/commit/a679810ba67bd3e562396e1fa1ca3dfd787b07e7))
- **commands**: update backend request calls for new event parameter ([6877e1e](https://github.com/urmzd/sr/commit/6877e1e39798a2df3b2c143372e6aa81f951eb54))

[Full Changelog](https://github.com/urmzd/sr/compare/v1.9.0...v1.10.0)


## 1.9.0 (2026-03-19)

### Features

- **cli**: add git hooks management, self-update, and remove pre-commit framework ([35460d5](https://github.com/urmzd/sr/commit/35460d50513d448252bd2beaa85553f65f5c1317))

[Full Changelog](https://github.com/urmzd/sr/compare/v1.8.2...v1.9.0)


## 1.8.2 (2026-03-19)

### Bug Fixes

- **ci**: handle already-published crates in publish step ([4b2c8d1](https://github.com/urmzd/sr/commit/4b2c8d1da5ae5fcacbec5dbe73ddec1d0f9b2eee))

[Full Changelog](https://github.com/urmzd/sr/compare/v1.8.1...v1.8.2)


## 1.8.1 (2026-03-19)

### Bug Fixes

- **ci**: checkout release tag for publish and handle re-publishes ([e5e40dd](https://github.com/urmzd/sr/commit/e5e40ddfd7c9cf6456cdb32ee6a9b39d2adde435))

### Documentation

- update all documentation to reflect AI-powered CLI ([6271e9e](https://github.com/urmzd/sr/commit/6271e9ec6b648444eb2d2fa86f4080d211a71222))

[Full Changelog](https://github.com/urmzd/sr/compare/v1.8.0...v1.8.1)


## 1.8.0 (2026-03-19)

### Features

- integrate AI-powered git commands into sr ([0c333ab](https://github.com/urmzd/sr/commit/0c333ab568818dc6d59195541bdf608ce1ac0bd0))

[Full Changelog](https://github.com/urmzd/sr/compare/v1.7.2...v1.8.0)


## 1.7.2 (2026-03-16)

### Refactoring

- rename repo references from urmzd/semantic-release to urmzd/sr ([68659ca](https://github.com/urmzd/sr/commit/68659caa5a61b9b14c0efbb0627171b531d54085))

[Full Changelog](https://github.com/urmzd/sr/compare/v1.7.1...v1.7.2)


## 1.7.1 (2026-03-16)

### Bug Fixes

- make refactor commits trigger a patch release ([290e96e](https://github.com/urmzd/sr/commit/290e96e4d449b2b1ffead4351b2f9a06d6f0ed65))

### Refactoring

- rename config to sr.yaml, remove toml/json support ([ad1c6f2](https://github.com/urmzd/sr/commit/ad1c6f2e0b861aa1c670f96b717784e26c5a91d1))

### Miscellaneous

- standardize project files and README header ([415e32c](https://github.com/urmzd/sr/commit/415e32c9f43fc4b22eddfc8b0e03502f0b89fe78))

[Full Changelog](https://github.com/urmzd/sr/compare/v1.7.0...v1.7.1)


## 1.7.0 (2026-03-14)

### Features

- allow --force to create patch release when no releasable commits exist ([4968d8b](https://github.com/urmzd/sr/commit/4968d8b8a26f3b282124f1a30b462dce50e29dde))

[Full Changelog](https://github.com/urmzd/sr/compare/v1.6.1...v1.7.0)


## 1.6.1 (2026-03-14)

### Bug Fixes

- remove target_commitish from release payload to avoid GitHub 422 error ([973ea38](https://github.com/urmzd/sr/commit/973ea38b115d5174d13c3a5caf4c18d4d3aa2706))

[Full Changelog](https://github.com/urmzd/sr/compare/v1.6.0...v1.6.1)


## 1.6.0 (2026-03-14)

### Features

- improve release cycle with signing, drafts, checksums, PATCH updates, and more ([a23a210](https://github.com/urmzd/sr/commit/a23a210fea1111e5d80b9bc2d869741f1a1611dd))

### Documentation

- add AGENTS.md and agent skill for Claude Code ([3ad54b0](https://github.com/urmzd/sr/commit/3ad54b08888edfa6120af085f2d6b8c0703ed9c8))

[Full Changelog](https://github.com/urmzd/sr/compare/v1.5.1...v1.6.0)


## 1.5.1 (2026-03-10)

### Bug Fixes

- add Cargo.lock to stage_files and sync with v1.5.0 ([5463b67](https://github.com/urmzd/sr/commit/5463b67d9178c311099cc76db7b2e71e8c61138d))

### Documentation

- remove incorrect single-branch limitation ([36d18a8](https://github.com/urmzd/sr/commit/36d18a82b9bbf4729262657a8f5d5820af854e0c))


## 1.5.0 (2026-03-10)

### Features

- add pre-release support (alpha, beta, rc) ([f33b949](https://github.com/urmzd/sr/commit/f33b9492fdd912e91f23b3eaead21cc14a85e9ee))
- add changelog templates, stage_files, pre/post hooks, and rollback ([93c524a](https://github.com/urmzd/sr/commit/93c524a686090d11dd1810dee1bc105cde063743))

### Documentation

- comprehensive configuration and version file documentation ([1ebcf81](https://github.com/urmzd/sr/commit/1ebcf81832575c57e57d81f1e8667d2c12c116c1))

### Miscellaneous

- switch to trusted publishing for crates.io ([ddb0be8](https://github.com/urmzd/sr/commit/ddb0be8fb1e2c237fb6de6194d3cc91d0818e96c))
- standardize GitHub Actions workflows ([eae04e5](https://github.com/urmzd/sr/commit/eae04e58d2afc7e6e07b3441218844a2038969ce))


## 1.4.4 (2026-02-25)

### Bug Fixes

- use env var for build-command to avoid YAML/shell expansion issues ([dd1f8fd](https://github.com/urmzd/sr/commit/dd1f8fd92433dd95d8ec7d300bac4d7d477af74b))


## 1.4.3 (2026-02-25)

### Bug Fixes

- write build-command to temp file to prevent outer shell expansion ([731f774](https://github.com/urmzd/sr/commit/731f774e67fa916805095db13f7969e210a6aa06))

### Miscellaneous

- move crates.io publish to separate job so build is never blocked ([7cf95c1](https://github.com/urmzd/sr/commit/7cf95c1a7a3c328e74d45cd7a574fc370570c023))
- lock file update ([d3f3f73](https://github.com/urmzd/sr/commit/d3f3f7370c22e73139522b639510513100fe5c4b))


## 1.4.2 (2026-02-25)

### Bug Fixes

- don't let crates.io publish failure block binary uploads ([5c0275f](https://github.com/urmzd/sr/commit/5c0275ff5748a0692a919cc56afd5f2c16029ed1))


## 1.4.1 (2026-02-25)

### Bug Fixes

- add force re-release support to workflow dispatch ([bf0b8e3](https://github.com/urmzd/sr/commit/bf0b8e33283993120ec99a1cd5324ec61ba24357))


## 1.4.0 (2026-02-25)

### Features

- add build_command option and improve shell installer PATH setup ([df32798](https://github.com/urmzd/sr/commit/df3279820d7065496eaeac6862240d8d515a9c78))

### Bug Fixes

- trigger binary builds from release workflow ([787cf1e](https://github.com/urmzd/sr/commit/787cf1e89909f2b696ccf768cfbb290eb9408d0b))

### Documentation

- remove License section from README ([e5d24c1](https://github.com/urmzd/sr/commit/e5d24c124acbb0d1f6ecd1d7531bc35c26024a54))

### Miscellaneous

- inline build matrix into release.yml, remove build.yml ([afc423f](https://github.com/urmzd/sr/commit/afc423f4fb50e7ee97a6f7db8f9f63726265227d))
- add sensitive paths to .gitignore ([742bc42](https://github.com/urmzd/sr/commit/742bc4257510b0b5d68eeb485939f2139dd5d5b4))


## 1.3.0 (2026-02-21)

### Features

- add shell installer, Windows release target, and license housekeeping ([039c9fe](https://github.com/urmzd/sr/commit/039c9fe3a1d45ec348cf53b27feda6af3bae8acf))

### Miscellaneous

- cleanup cargo.lock ([f4097bf](https://github.com/urmzd/sr/commit/f4097bf5afe1c798a07773ab4e68fbb1f46e8eb7))


## 1.2.0 (2026-02-19)

### Features

- replace gh CLI with direct GitHub REST API calls ([c04c82c](https://github.com/urmzd/sr/commit/c04c82c48474ee458dae376ce5d70fbf96385977))
- prevent interactive git auth prompts and support CI runner credentials ([5f19d00](https://github.com/urmzd/sr/commit/5f19d00e8ed0845100439b01666c4484e9066c1b))

### Bug Fixes

- clear existing git extraheader before injecting auth ([028e940](https://github.com/urmzd/sr/commit/028e940dd9fe84f3d5157a7d45e8bcc1f490a377))


## 1.1.0 (2026-02-18)

### Features

- add GHES support, MUSL builds, ~/.local/bin install path, and post-release hooks docs ([32cdafa](https://github.com/urmzd/sr/commit/32cdafadcc8b617c02c360b51e564a2437cbba96))

### Miscellaneous

- update README examples to reference v1 ([bf6e998](https://github.com/urmzd/sr/commit/bf6e9986b88c04e31605613d61f1fde8a5763795))
- update Cargo.toml license to Apache-2.0 ([b41d16a](https://github.com/urmzd/sr/commit/b41d16ac4b2afcfacbb69dc0b848eb945a08607f))
- license under Apache 2.0 ([039c7fb](https://github.com/urmzd/sr/commit/039c7fb4edeedbaced90999a85230700410df7f0))


## 1.0.0 (2026-02-11)

### Breaking Changes

- remove lifecycle hooks, output structured JSON from sr release ([56e5ed0](https://github.com/urmzd/sr/commit/56e5ed0125c64455129f40a15ab70950aeb2e349))

### Bug Fixes

- add floating_tags, parse JSON output in self-release workflow ([70e0f04](https://github.com/urmzd/sr/commit/70e0f04f4b5abbd621821637c78c851ede770ac5))


## 0.10.0 (2026-02-11)

### Features

- add --force flag, structured errors, distinct exit codes, and action outputs ([ea0ba9c](https://github.com/urmzd/sr/commit/ea0ba9cd416465ad5d6d09642b517f232756f557))

### Refactoring

- remove contributors section from release notes ([2831757](https://github.com/urmzd/sr/commit/2831757c0dd85cf63958bf212b4288e229bfffa8))


## 0.9.0 (2026-02-09)

### Features

- add floating major version tags support ([ece1331](https://github.com/urmzd/sr/commit/ece133156582e5fd13a36027e630a524fe2cfa91))

### Contributors

- @urmzd


## 0.8.0 (2026-02-08)

### Features

- add release artifact upload support ([3fa9ea7](https://github.com/urmzd/sr/commit/3fa9ea720e48af6728c799c251c3075bf944a6f7))

### Bug Fixes

- update workspace dependency versions during Cargo.toml version bump ([c7a6634](https://github.com/urmzd/sr/commit/c7a66349a828905f5ab473c9490bd01f035cb088))

### Miscellaneous

- fix clippy warnings, apply cargo fmt, and fix commit-msg hook PCRE parsing ([2611a1c](https://github.com/urmzd/sr/commit/2611a1caf59717a91cf9d005646582b59c95ea57))
- update Cargo.lock for v0.7.0 ([e56772c](https://github.com/urmzd/sr/commit/e56772cde97f758146d0b8dc50f4ad42786fd8fa))

### Contributors

- @urmzd


## 0.7.0 (2026-02-08)

### Features

- expand version file support, add lenient mode, fix root Cargo.toml ([487e5f4](https://github.com/urmzd/sr/commit/487e5f4aa21a1d9e22298efebe310be0f193297c))

### Bug Fixes

- use tempdir in release tests to prevent changelog file pollution ([52e854d](https://github.com/urmzd/sr/commit/52e854d2ccc13dea844d6cfe74e8e233063c24f8))
- add skip-ci flag to release commit messages to avoid redundant CI runs ([87160a8](https://github.com/urmzd/sr/commit/87160a89b2650d52cacfd23db6c46dcaa3c7ffa8))

### Miscellaneous

- add floating v0 tag, update docs, and prepare crates.io publishing ([bda72c2](https://github.com/urmzd/sr/commit/bda72c2242f9119c99ff8850736c64255fa2ff0b))

### Contributors

- @urmzd


## 0.6.0 (2026-02-08)

### Features

- tag contributors with GitHub @username in changelog ([bc13a08](https://github.com/urmzd/sr/commit/bc13a08f8e43eafa9526dbb0607cee9efb3f81b9))

### Documentation

- regenerate changelog with GitHub @username contributors ([1d2906a](https://github.com/urmzd/sr/commit/1d2906a7f71f53c1eb72865bd5cf0c14afbf49aa))

### Miscellaneous

- fix formatting in changelog contributor rendering ([2276d07](https://github.com/urmzd/sr/commit/2276d07facda43f6ab56c32228e47db16f7f7634))

### Contributors

- @urmzd


## 0.5.0 (2026-02-08)

### Features

- add changelog --regenerate flag and improve changelog sections ([799c0f7](https://github.com/urmzd/sr/commit/799c0f7854956f79554e712bfb07beccf6a83c47))

### Miscellaneous

- remove redundant push trigger from CI workflow ([4ece192](https://github.com/urmzd/sr/commit/4ece1927a60234a4335eaf5aef97a691d14e159d))

### Contributors

- @urmzd

[Full Changelog](https://github.com/urmzd/sr/compare/v0.4.0...v0.5.0)

## 0.4.0 (2026-02-08)

### Features

- add hook context, version file bumping, and changelog SHA links ([d95ff03](https://github.com/urmzd/sr/commit/d95ff0373361fc0b04116f9a157f5dd121e76f36))

### Documentation

- update README with version files, completions, and developer workflow ([87fe902](https://github.com/urmzd/sr/commit/87fe90200d3dd965e00aa2a3236aa8fe7fc1e62b))

### Refactoring

- replace git-conventional with built-in regex commit parser ([a687a1c](https://github.com/urmzd/sr/commit/a687a1c4974d0215866f9f304b92ee975324455f))

### Miscellaneous

- fix formatting and clippy warnings, add pre-commit hook ([5bb14a9](https://github.com/urmzd/sr/commit/5bb14a98cf3ff2f9e25d26681deed614aac1eca0))

### Contributors

- @urmzd

[Full Changelog](https://github.com/urmzd/sr/compare/v0.3.0...v0.4.0)

## 0.3.0 (2026-02-08)

### Features

- add shell completions, enhance plan preview, fix dry-run hooks ([9fd9483](https://github.com/urmzd/sr/commit/9fd9483b4e5af9600c2b22f18f558bc588c63e7b))

### Contributors

- @urmzd

[Full Changelog](https://github.com/urmzd/sr/compare/v0.2.0...v0.3.0)

## 0.2.0 (2026-02-08)

### Features

- make release execution idempotent with CHANGELOG commit ([c78a302](https://github.com/urmzd/sr/commit/c78a302584b3ad64ca57d7d1252639324a9e50ac))

### Contributors

- @urmzd

[Full Changelog](https://github.com/urmzd/sr/compare/v0.1.0...v0.2.0)

## 0.1.0 (2026-02-07)

### Features

- initial implementation of semantic-release toolchain ([45eaa61](https://github.com/urmzd/sr/commit/45eaa6164e797045855f056ecd88e15b5dd08437))

### Bug Fixes

- configure git identity for tag creation in CI ([3d76f38](https://github.com/urmzd/sr/commit/3d76f38211cd0c5b78de9b937cc317fae57cd302))

### Documentation

- add action usage examples, branding, and marketplace metadata ([38a3ed3](https://github.com/urmzd/sr/commit/38a3ed306dc2e7865937d74375a1fb3e15317365))

### Refactoring

- replace octocrab with gh CLI, fix macOS runners, move action.yml to root ([b66bb29](https://github.com/urmzd/sr/commit/b66bb29663bda335d3a18607cf22457f2faf4132))

### Miscellaneous

- fix cargo fmt formatting in sr-github ([ed3b56c](https://github.com/urmzd/sr/commit/ed3b56ca781cdf1ce8e14fce26f2bce2955d11b8))

### Contributors

- @urmzd
