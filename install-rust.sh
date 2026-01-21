#!/usr/bin/env bash
set -euo pipefail

# Hive Rust Installation Script
# Usage: curl -fsSL https://raw.githubusercontent.com/anthropics/hive/main/install-rust.sh | bash

REPO="anthropics/hive"
BINARY_NAME="hive-rust"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

log_success() {
    echo -e "${GREEN}✓${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

log_error() {
    echo -e "${RED}✗${NC} $1"
}

# Detect platform
detect_platform() {
    local os=$(uname -s)
    local arch=$(uname -m)

    case "$os" in
        Darwin)
            case "$arch" in
                x86_64)
                    echo "x86_64-apple-darwin"
                    ;;
                arm64|aarch64)
                    echo "aarch64-apple-darwin"
                    ;;
                *)
                    log_error "Unsupported architecture: $arch"
                    exit 1
                    ;;
            esac
            ;;
        Linux)
            case "$arch" in
                x86_64)
                    echo "x86_64-unknown-linux-gnu"
                    ;;
                *)
                    log_error "Unsupported architecture: $arch"
                    exit 1
                    ;;
            esac
            ;;
        *)
            log_error "Unsupported operating system: $os"
            exit 1
            ;;
    esac
}

# Get latest release version
get_latest_version() {
    log_info "Fetching latest release..."

    local version=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
        | grep '"tag_name":' \
        | sed -E 's/.*"rust-v([^"]+)".*/\1/')

    if [ -z "$version" ]; then
        log_error "Failed to fetch latest version"
        exit 1
    fi

    echo "$version"
}

# Download and install binary
install_binary() {
    local version=$1
    local platform=$2
    local artifact="${BINARY_NAME}-${platform}"
    local download_url="https://github.com/$REPO/releases/download/rust-v${version}/${artifact}"

    log_info "Downloading ${artifact}..."

    # Create temp directory
    local tmp_dir=$(mktemp -d)
    trap "rm -rf $tmp_dir" EXIT

    # Download binary
    if ! curl -fsSL -o "$tmp_dir/$BINARY_NAME" "$download_url"; then
        log_error "Failed to download binary"
        exit 1
    fi

    # Make executable
    chmod +x "$tmp_dir/$BINARY_NAME"

    # Create install directory if it doesn't exist
    mkdir -p "$INSTALL_DIR"

    # Install binary
    log_info "Installing to $INSTALL_DIR..."
    mv "$tmp_dir/$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"

    log_success "Installed $BINARY_NAME to $INSTALL_DIR"
}

# Check if binary is in PATH
check_path() {
    if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
        log_warning "$INSTALL_DIR is not in your PATH"
        log_info "Add the following to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
        echo ""
        echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
        echo ""
    fi
}

# Verify installation
verify_installation() {
    if [ -x "$INSTALL_DIR/$BINARY_NAME" ]; then
        local version=$("$INSTALL_DIR/$BINARY_NAME" --version 2>&1 || echo "unknown")
        log_success "Installation verified: $version"
        return 0
    else
        log_error "Installation verification failed"
        return 1
    fi
}

# Main installation flow
main() {
    echo ""
    echo "╔═══════════════════════════════════════╗"
    echo "║  Hive Rust Installer                  ║"
    echo "╚═══════════════════════════════════════╝"
    echo ""

    local platform=$(detect_platform)
    log_info "Detected platform: $platform"

    local version=$(get_latest_version)
    log_info "Latest version: $version"

    install_binary "$version" "$platform"

    echo ""
    verify_installation

    echo ""
    check_path

    echo ""
    log_success "Installation complete!"
    echo ""
    log_info "Run 'hive-rust --help' to get started"
    echo ""
}

main "$@"
