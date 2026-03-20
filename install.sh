#!/bin/sh
set -e

# rx installer — downloads and installs the rx binary

REPO="iPeluwa/rx"
INSTALL_DIR="${RX_INSTALL_DIR:-$HOME/.rx/bin}"

get_arch() {
    arch=$(uname -m)
    case "$arch" in
        x86_64|amd64) echo "x86_64" ;;
        aarch64|arm64) echo "aarch64" ;;
        *) echo "unsupported architecture: $arch" >&2; exit 1 ;;
    esac
}

get_os() {
    os=$(uname -s)
    case "$os" in
        Linux) echo "unknown-linux-gnu" ;;
        Darwin) echo "apple-darwin" ;;
        MINGW*|MSYS*|CYGWIN*) echo "pc-windows-msvc" ;;
        *) echo "unsupported OS: $os" >&2; exit 1 ;;
    esac
}

main() {
    echo "Installing rx..."

    ARCH=$(get_arch)
    OS=$(get_os)
    TARGET="${ARCH}-${OS}"

    # Get latest release tag
    LATEST=$(curl -sL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)
    if [ -z "$LATEST" ]; then
        echo "No releases found. Building from source..."
        if ! command -v cargo >/dev/null 2>&1; then
            echo "Error: cargo is required to build from source."
            echo "Install Rust: https://rustup.rs"
            exit 1
        fi
        cargo install --git "https://github.com/${REPO}.git"
        echo "rx installed via cargo install."
        exit 0
    fi

    URL="https://github.com/${REPO}/releases/download/${LATEST}/rx-${TARGET}.tar.gz"

    echo "Downloading rx ${LATEST} for ${TARGET}..."

    TMPDIR=$(mktemp -d)
    trap 'rm -rf "$TMPDIR"' EXIT

    if ! curl -sL "$URL" -o "$TMPDIR/rx.tar.gz"; then
        echo "Download failed. Building from source instead..."
        cargo install --git "https://github.com/${REPO}.git" --tag "$LATEST"
        echo "rx ${LATEST} installed via cargo install."
        exit 0
    fi

    tar -xzf "$TMPDIR/rx.tar.gz" -C "$TMPDIR"

    mkdir -p "$INSTALL_DIR"
    mv "$TMPDIR/rx" "$INSTALL_DIR/rx"
    chmod +x "$INSTALL_DIR/rx"

    echo "rx ${LATEST} installed to ${INSTALL_DIR}/rx"

    # Check if install dir is in PATH
    case ":$PATH:" in
        *":$INSTALL_DIR:"*) ;;
        *)
            echo ""
            echo "Add rx to your PATH by adding this to your shell config:"
            echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
            ;;
    esac
}

main
