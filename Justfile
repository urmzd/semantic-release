default: check

install-hooks:
    git config core.hooksPath .githooks

init: install-hooks
    rustup component add clippy rustfmt

install:
    cargo build --release -p sr-cli

build:
    cargo build --workspace

run *ARGS:
    cargo run -p sr-cli -- {{ARGS}}

test:
    cargo test --workspace

lint:
    cargo clippy --workspace -- -D warnings

fmt:
    cargo fmt --all

check-fmt:
    cargo fmt --all -- --check

publish:
    cargo publish -p sr-core --dry-run
    cargo publish -p sr-git --dry-run
    cargo publish -p sr-github --dry-run
    cargo publish -p sr-cli --dry-run

check: check-fmt lint test
