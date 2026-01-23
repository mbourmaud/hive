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
# This version is automatically updated by CI when a release is fully built
STABLE_VERSION="2.4.1"

echo ""
echo -e "${YELLOW}"
cat << 'EOF'
██╗  ██╗██╗██╗   ██╗███████╗
██║  ██║██║██║   ██║██╔════╝
███████║██║██║   ██║█████╗
██╔══██║██║╚██╗ ██╔╝██╔══╝
██║  ██║██║ ╚████╔╝ ███████╗
╚═╝  ╚═╝╚═╝  ╚═══╝  ╚══════╝
EOF
echo -e "${RESET}"
echo -e "${YELLOW}Drone Orchestration for Claude Code${RESET}"
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
# Use Stable Version
# ============================================================================

VERSION="$STABLE_VERSION"
echo -e "${GREEN}Installing version:${RESET} v$VERSION"

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

# ============================================================================
# Install Skills via hive install command
# ============================================================================

echo ""
echo -e "${CYAN}Installing components...${RESET}"

# The Rust binary has embedded skills - just run hive install quietly
SKILLS_OUTPUT=$("$INSTALL_DIR/hive" install --skills-only 2>&1)
SKILLS_EXIT=$?

# ============================================================================
# Installation Summary
# ============================================================================

echo ""
echo -e "${GREEN}${BOLD}✓ Installation complete!${RESET}"
echo -e "  ${GREEN}✓${RESET} Binary installed to $INSTALL_DIR/hive"
if [ $SKILLS_EXIT -eq 0 ]; then
  SKILL_COUNT=$(echo "$SKILLS_OUTPUT" | grep -oE '[0-9]+ skills' | grep -oE '[0-9]+' || echo "8")
  echo -e "  ${GREEN}✓${RESET} ${SKILL_COUNT} skills installed to ~/.claude/commands/"
else
  echo -e "  ${YELLOW}⚠${RESET} Skills installation skipped (run 'hive install' later)"
fi

# Check if install dir is in PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
  echo -e "${YELLOW}⚠${RESET} Add $INSTALL_DIR to your PATH:"
  echo "export PATH=\"\$PATH:$INSTALL_DIR\""
  echo ""
fi

# Get clean version
VERSION=$("$INSTALL_DIR/hive" --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1 || echo "unknown")
echo -e "${BOLD}Getting Started:${RESET}"
echo -e "  ${CYAN}1.${RESET} cd into your project"
echo -e "  ${CYAN}2.${RESET} hive init"
echo -e "  ${CYAN}3.${RESET} Create a PRD with ${CYAN}/hive:prd${RESET} in Claude Code"
echo -e "  ${CYAN}4.${RESET} hive start my-drone"
echo -e "  ${CYAN}5.${RESET} Monitor PRD progress with ${CYAN}hive monitor${RESET}"
echo ""
echo -e "${BOLD}Monitoring:${RESET}"
echo -e "${DIM}Live dashboard:${RESET} hive monitor"
echo -e "${DIM}Statusline:${RESET} ${CYAN}/hive:statusline${RESET} in Claude Code"
echo -e "${DIM}View logs:${RESET} hive logs <name>"
echo ""
echo -e "${BOLD}Maintenance:${RESET}"
echo -e "${DIM}Update:${RESET} hive update"
echo ""
echo -e "${DIM}Hive ${VERSION} • https://github.com/$REPO${RESET}"
echo ""
