#!/bin/bash

# Hive installer - Drone Orchestration for Claude Code
# https://github.com/mbourmaud/hive
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/mbourmaud/hive/main/install.sh | bash

set -e

# Colors (only if stdout is a TTY)
if [ -t 1 ]; then
  GREEN='\033[32m'
  YELLOW='\033[33m'
  CYAN='\033[36m'
  RED='\033[31m'
  DIM='\033[2m'
  BOLD='\033[1m'
  RESET='\033[0m'
else
  GREEN=''
  YELLOW=''
  CYAN=''
  RED=''
  DIM=''
  BOLD=''
  RESET=''
fi

REPO="mbourmaud/hive"
GITHUB_API="https://api.github.com/repos/$REPO/releases/latest"

echo ""
echo -e "${YELLOW}${BOLD}👑 Hive${RESET} - Drone Orchestration for Claude Code"
echo ""

# ============================================================================
# Detect Platform
# ============================================================================

detect_platform() {
  local os arch

  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
    Darwin)
      case "$arch" in
        arm64|aarch64) echo "darwin-arm64" ;;
        x86_64) echo "darwin-amd64" ;;
        *) echo "unsupported" ;;
      esac
      ;;
    Linux)
      case "$arch" in
        aarch64|arm64) echo "linux-arm64" ;;
        x86_64) echo "linux-amd64" ;;
        *) echo "unsupported" ;;
      esac
      ;;
    *)
      echo "unsupported"
      ;;
  esac
}

PLATFORM=$(detect_platform)

if [ "$PLATFORM" = "unsupported" ]; then
  echo -e "${RED}Error:${RESET} Unsupported platform: $(uname -s) $(uname -m)"
  echo "Supported: macOS (arm64, x86_64), Linux (x86_64, aarch64)"
  exit 1
fi

echo -e "${CYAN}Detected platform:${RESET} $PLATFORM"

# ============================================================================
# Fetch Latest Release
# ============================================================================

echo -e "${CYAN}Fetching latest release...${RESET}"

# Get latest release info
RELEASE_JSON=$(curl -sL "$GITHUB_API")

VERSION=$(echo "$RELEASE_JSON" | grep -o '"tag_name": *"[^"]*"' | head -1 | sed 's/"tag_name": *"\(.*\)"/\1/' | tr -d 'v')

if [ -z "$VERSION" ]; then
  echo -e "${RED}Error:${RESET} Could not determine latest version"
  echo "Check https://github.com/$REPO/releases for available releases"
  exit 1
fi

echo -e "${GREEN}Latest version:${RESET} v$VERSION"

# ============================================================================
# Download Binary
# ============================================================================

echo -e "${CYAN}Downloading hive for $PLATFORM...${RESET}"

# Determine install directory
if [ -d "$HOME/.local/bin" ]; then
  INSTALL_DIR="$HOME/.local/bin"
elif [ -d "/usr/local/bin" ] && [ -w "/usr/local/bin" ]; then
  INSTALL_DIR="/usr/local/bin"
else
  mkdir -p "$HOME/.local/bin"
  INSTALL_DIR="$HOME/.local/bin"
fi

# Download binary
DOWNLOAD_URL="https://github.com/$REPO/releases/download/v$VERSION/hive-$PLATFORM.tar.gz"
TEMP_DIR=$(mktemp -d)

if ! curl -sL "$DOWNLOAD_URL" | tar -xz -C "$TEMP_DIR" 2>/dev/null; then
  echo -e "${RED}Error:${RESET} Failed to download binary"
  echo "URL: $DOWNLOAD_URL"
  echo ""
  echo "You can try installing manually:"
  echo "  1. Go to https://github.com/$REPO/releases/latest"
  echo "  2. Download the binary for your platform"
  echo "  3. Extract and move to $INSTALL_DIR/hive"
  rm -rf "$TEMP_DIR"
  exit 1
fi

# Find and install the binary
BINARY=$(find "$TEMP_DIR" -name "hive*" -type f -perm +111 2>/dev/null | head -1)
if [ -z "$BINARY" ]; then
  BINARY=$(find "$TEMP_DIR" -name "hive*" -type f | head -1)
fi

if [ -z "$BINARY" ]; then
  echo -e "${RED}Error:${RESET} Could not find binary in downloaded archive"
  rm -rf "$TEMP_DIR"
  exit 1
fi

mv "$BINARY" "$INSTALL_DIR/hive"
chmod +x "$INSTALL_DIR/hive"
rm -rf "$TEMP_DIR"

echo -e "${GREEN}✓${RESET} Binary installed to $INSTALL_DIR/hive"

# Check if install dir is in PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
  echo ""
  echo -e "${YELLOW}⚠${RESET} Add $INSTALL_DIR to your PATH:"
  echo "  export PATH=\"\$PATH:$INSTALL_DIR\""
  echo ""
fi

# ============================================================================
# Install Skills via hive install command
# ============================================================================

echo ""
echo -e "${CYAN}Installing Claude Code skills...${RESET}"

# The Rust binary has embedded skills - just run hive install
if "$INSTALL_DIR/hive" install --skills-only 2>/dev/null; then
  echo -e "${GREEN}✓${RESET} Skills installed to ~/.claude/commands/"
else
  echo -e "${YELLOW}⚠${RESET} Skills installation skipped (run 'hive install' later)"
fi

# ============================================================================
# Verify Installation
# ============================================================================

echo ""
INSTALLED_VERSION=$("$INSTALL_DIR/hive" --version 2>/dev/null | head -1 || echo "unknown")
echo -e "${GREEN}${BOLD}✓ Hive installed successfully!${RESET}"
echo -e "  Version: $INSTALLED_VERSION"
echo ""

# ============================================================================
# Summary
# ============================================================================

echo "Quick start:"
echo -e "  ${DIM}# Initialize Hive in your project${RESET}"
echo "  hive init"
echo ""
echo -e "  ${DIM}# Create a PRD (in Claude Code)${RESET}"
echo "  /hive:prd"
echo ""
echo -e "  ${DIM}# Launch a drone${RESET}"
echo "  hive start my-feature"
echo ""
echo -e "  ${DIM}# Monitor${RESET}"
echo "  hive monitor"
echo ""
echo -e "  ${DIM}# Update to latest version${RESET}"
echo "  hive update"
echo ""
echo "Documentation: https://github.com/$REPO"
