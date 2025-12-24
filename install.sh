#!/bin/bash
set -e

# Hive installation script
# Detects OS/architecture and installs the latest release

REPO="mbourmaud/hive"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "🐝 Installing Hive..."

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
    darwin)
        OS="darwin"
        ;;
    linux)
        OS="linux"
        ;;
    *)
        echo -e "${RED}Unsupported OS: $OS${NC}"
        exit 1
        ;;
esac

case "$ARCH" in
    x86_64)
        ARCH="amd64"
        ;;
    arm64|aarch64)
        ARCH="arm64"
        ;;
    *)
        echo -e "${RED}Unsupported architecture: $ARCH${NC}"
        exit 1
        ;;
esac

# Get latest release version
echo "📦 Fetching latest release..."
LATEST_VERSION=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

if [ -z "$LATEST_VERSION" ]; then
    echo -e "${RED}Failed to fetch latest version${NC}"
    exit 1
fi

echo "   Latest version: $LATEST_VERSION"

# Construct download URL
TARBALL="hive-${OS}-${ARCH}.tar.gz"
DOWNLOAD_URL="https://github.com/$REPO/releases/download/$LATEST_VERSION/$TARBALL"

echo "📥 Downloading Hive..."
echo "   URL: $DOWNLOAD_URL"

# Create temporary directory
TMP_DIR=$(mktemp -d)
trap "rm -rf $TMP_DIR" EXIT

# Download tarball
if ! curl -sL "$DOWNLOAD_URL" -o "$TMP_DIR/$TARBALL"; then
    echo -e "${RED}Failed to download Hive${NC}"
    echo -e "${YELLOW}Please check if the release exists: https://github.com/$REPO/releases/tag/$LATEST_VERSION${NC}"
    exit 1
fi

# Extract tarball
echo "📂 Extracting..."
tar -xzf "$TMP_DIR/$TARBALL" -C "$TMP_DIR"

# Create install directory if it doesn't exist
mkdir -p "$INSTALL_DIR"

# The binary name in the tarball is hive-{os}-{arch}
BINARY_NAME="hive-${OS}-${ARCH}"

# Move binary to install location
echo "📋 Installing to $INSTALL_DIR/hive..."
mv "$TMP_DIR/$BINARY_NAME" "$INSTALL_DIR/hive"
chmod +x "$INSTALL_DIR/hive"

# Check if install dir is in PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo -e "${YELLOW}⚠️  $INSTALL_DIR is not in your PATH${NC}"
    echo ""
    echo "Add it to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
    echo ""
    echo "  export PATH=\"\$PATH:$INSTALL_DIR\""
    echo ""
fi

# Verify installation
if command -v hive &> /dev/null; then
    VERSION=$(hive --version 2>/dev/null || echo "unknown")
    echo -e "${GREEN}✅ Hive installed successfully!${NC}"
    echo "   Version: $VERSION"
    echo ""
    echo "Get started:"
    echo "  cd your-project"
    echo "  hive init"
    echo ""
    echo "Documentation: https://github.com/$REPO"
else
    echo -e "${YELLOW}⚠️  Installation complete, but 'hive' not found in PATH${NC}"
    echo "   Binary location: $INSTALL_DIR/hive"
    echo "   Add $INSTALL_DIR to your PATH"
fi
