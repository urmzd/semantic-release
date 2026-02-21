#!/bin/sh
# install.sh — Installs the sr binary from GitHub releases.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/urmzd/semantic-release/main/install.sh | sh
#
# Environment variables:
#   SR_VERSION     — version to install (e.g. "v1.2.0"); defaults to latest
#   SR_INSTALL_DIR — installation directory; defaults to $HOME/.local/bin

set -eu

REPO="urmzd/semantic-release"

main() {
    os=$(uname -s)
    arch=$(uname -m)

    case "$os" in
        Linux)
            case "$arch" in
                x86_64)  target="x86_64-unknown-linux-musl" ;;
                aarch64) target="aarch64-unknown-linux-musl" ;;
                *)       err "Unsupported Linux architecture: $arch" ;;
            esac
            ;;
        Darwin)
            case "$arch" in
                x86_64)  target="x86_64-apple-darwin" ;;
                arm64)   target="aarch64-apple-darwin" ;;
                *)       err "Unsupported macOS architecture: $arch" ;;
            esac
            ;;
        MINGW*|MSYS*|CYGWIN*|Windows_NT)
            err "Windows is not supported by this installer. Download a binary from https://github.com/$REPO/releases/latest"
            ;;
        *)
            err "Unsupported operating system: $os"
            ;;
    esac

    if [ -n "${SR_VERSION:-}" ]; then
        tag="$SR_VERSION"
    else
        tag=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
            | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p')
        if [ -z "$tag" ]; then
            err "Failed to fetch latest release tag"
        fi
    fi

    artifact="sr-${target}"
    url="https://github.com/$REPO/releases/download/${tag}/${artifact}"

    install_dir="${SR_INSTALL_DIR:-$HOME/.local/bin}"
    mkdir -p "$install_dir"

    echo "Downloading sr $tag for $target..."
    curl -fsSL "$url" -o "$install_dir/sr"
    chmod +x "$install_dir/sr"

    echo "Installed sr to $install_dir/sr"

    case ":$PATH:" in
        *":$install_dir:"*) ;;
        *)
            echo ""
            echo "Add $install_dir to your PATH:"
            echo "  export PATH=\"$install_dir:\$PATH\""
            ;;
    esac
}

err() {
    echo "Error: $1" >&2
    exit 1
}

main
