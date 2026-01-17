#!/bin/bash

# Hive installer - Drone Orchestration for Claude Code
# https://github.com/mbourmaud/hive

set -e

# Colors (only if stdout is a TTY)
if [ -t 1 ]; then
  GREEN='\033[32m'
  YELLOW='\033[33m'
  CYAN='\033[36m'
  DIM='\033[2m'
  RESET='\033[0m'
else
  GREEN=''
  YELLOW=''
  CYAN=''
  DIM=''
  RESET=''
fi

REPO_URL="https://raw.githubusercontent.com/mbourmaud/hive/main"
VERSION="0.2.1"

echo ""
echo -e "${YELLOW}👑 Hive${RESET} v$VERSION - Drone Orchestration for Claude Code"
echo ""

# ============================================================================
# Install CLI
# ============================================================================

echo -e "${CYAN}Installing CLI...${RESET}"

# Determine install directory
if [ -d "$HOME/.local/bin" ]; then
  INSTALL_DIR="$HOME/.local/bin"
elif [ -d "/usr/local/bin" ] && [ -w "/usr/local/bin" ]; then
  INSTALL_DIR="/usr/local/bin"
else
  mkdir -p "$HOME/.local/bin"
  INSTALL_DIR="$HOME/.local/bin"
fi

# Download hive CLI
curl -sL -o "$INSTALL_DIR/hive" "$REPO_URL/hive.sh"
chmod +x "$INSTALL_DIR/hive"
echo -e "${GREEN}✓${RESET} CLI installed to $INSTALL_DIR/hive"

# Check if install dir is in PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
  echo ""
  echo -e "${YELLOW}⚠${RESET} Add $INSTALL_DIR to your PATH:"
  echo "  export PATH=\"\$PATH:$INSTALL_DIR\""
  echo ""
fi

# ============================================================================
# Install Skills for Claude Code
# ============================================================================

echo ""
echo -e "${CYAN}Installing Claude Code skills...${RESET}"

# Check if Claude Code is installed
if [ ! -d "$HOME/.claude" ]; then
  echo -e "${YELLOW}⚠${RESET} Claude Code not detected (~/.claude not found)"
  echo "  Install Claude Code first: https://claude.ai/code"
  echo ""
  echo -e "${GREEN}✓${RESET} CLI installed. Skills skipped."
  exit 0
fi

mkdir -p "$HOME/.claude/commands"

SKILLS=(
  "hive:init"
  "hive:start"
  "hive:status"
  "hive:list"
  "hive:logs"
  "hive:kill"
  "hive:clean"
  "hive:prd"
  "hive:statusline"
)

for skill in "${SKILLS[@]}"; do
  curl -sL -o "$HOME/.claude/commands/$skill.md" "$REPO_URL/commands/$skill.md"
done

echo -e "${GREEN}✓${RESET} ${#SKILLS[@]} skills installed to ~/.claude/commands/"

# ============================================================================
# Summary
# ============================================================================

echo ""
echo -e "${GREEN}Done!${RESET}"
echo ""
echo "Quick start:"
echo -e "  ${DIM}# Initialize Hive in your project${RESET}"
echo "  hive init"
echo ""
echo -e "  ${DIM}# Create a PRD (in Claude Code)${RESET}"
echo "  /hive:prd"
echo ""
echo -e "  ${DIM}# Launch a drone${RESET}"
echo "  hive start --prd .hive/prds/my-feature.json"
echo ""
echo -e "  ${DIM}# Monitor${RESET}"
echo "  hive status"
echo ""
echo "Documentation: https://github.com/mbourmaud/hive"
