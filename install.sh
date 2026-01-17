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
VERSION="0.2.0"
INSTALLED_CLI=0
INSTALLED_SKILLS=0

echo ""
echo "${YELLOW}👑 Hive${RESET} v$VERSION - Drone Orchestration for Claude Code"
echo ""

# ============================================================================
# Install CLI
# ============================================================================

echo "${CYAN}Installing CLI...${RESET}"

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
printf "${GREEN}✓${RESET} CLI installed to $INSTALL_DIR/hive\n"
INSTALLED_CLI=1

# Check if install dir is in PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
  echo ""
  printf "${YELLOW}⚠${RESET} Add $INSTALL_DIR to your PATH:\n"
  echo "  export PATH=\"\$PATH:$INSTALL_DIR\""
  echo ""
fi

# ============================================================================
# Install Skills (Claude Code, Cursor, etc.)
# ============================================================================

echo ""
echo "${CYAN}Installing skills...${RESET}"

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

# Claude Code
if [ -d "$HOME/.claude" ]; then
  mkdir -p "$HOME/.claude/commands"
  for skill in "${SKILLS[@]}"; do
    curl -sL -o "$HOME/.claude/commands/$skill.md" "$REPO_URL/commands/$skill.md"
  done
  printf "${GREEN}✓${RESET} Claude Code (${#SKILLS[@]} skills)\n"
  INSTALLED_SKILLS=$((INSTALLED_SKILLS + 1))
fi

# Cursor (1.6+)
if [ -d "$HOME/.cursor" ]; then
  mkdir -p "$HOME/.cursor/commands"
  for skill in "${SKILLS[@]}"; do
    curl -sL -o "$HOME/.cursor/commands/$skill.md" "$REPO_URL/commands/$skill.md"
  done
  printf "${GREEN}✓${RESET} Cursor (${#SKILLS[@]} commands)\n"
  INSTALLED_SKILLS=$((INSTALLED_SKILLS + 1))
fi

# Amp Code
if [ -d "$HOME/.amp" ]; then
  mkdir -p "$HOME/.config/amp/commands"
  for skill in "${SKILLS[@]}"; do
    curl -sL -o "$HOME/.config/amp/commands/$skill.md" "$REPO_URL/commands/$skill.md"
  done
  printf "${GREEN}✓${RESET} Amp Code (${#SKILLS[@]} skills)\n"
  INSTALLED_SKILLS=$((INSTALLED_SKILLS + 1))
fi

# OpenCode
if command -v opencode &> /dev/null || [ -d "$HOME/.config/opencode" ]; then
  mkdir -p "$HOME/.config/opencode/command"
  for skill in "${SKILLS[@]}"; do
    curl -sL -o "$HOME/.config/opencode/command/$skill.md" "$REPO_URL/commands/$skill.md"
  done
  printf "${GREEN}✓${RESET} OpenCode (${#SKILLS[@]} commands)\n"
  INSTALLED_SKILLS=$((INSTALLED_SKILLS + 1))
fi

# Gemini CLI
if command -v gemini &> /dev/null || [ -d "$HOME/.gemini" ]; then
  mkdir -p "$HOME/.gemini/commands"
  # For Gemini, we only install the main commands as TOML
  # (prd and statusline are Claude-specific)
  for skill in "hive:init" "hive:start" "hive:status" "hive:list" "hive:logs" "hive:kill" "hive:clean"; do
    TOML_FILE="$HOME/.gemini/commands/$skill.toml"
    CONTENT=$(curl -sL "$REPO_URL/commands/$skill.md" | sed '1,/^---$/d' | sed '1,/^---$/d')
    cat > "$TOML_FILE" << TOMLEOF
description = "Hive: $skill"
prompt = """
$CONTENT
"""
TOMLEOF
  done
  printf "${GREEN}✓${RESET} Gemini CLI (7 commands)\n"
  INSTALLED_SKILLS=$((INSTALLED_SKILLS + 1))
fi

# ============================================================================
# Summary
# ============================================================================

echo ""

if [ $INSTALLED_CLI -eq 0 ] && [ $INSTALLED_SKILLS -eq 0 ]; then
  echo "Installation failed."
  exit 1
fi

echo "${GREEN}Done!${RESET}"
echo ""
echo "Quick start:"
echo "  ${DIM}# Initialize Hive in your project${RESET}"
echo "  hive init"
echo ""
echo "  ${DIM}# Create a PRD (in Claude Code)${RESET}"
echo "  /hive:prd"
echo ""
echo "  ${DIM}# Launch a drone${RESET}"
echo "  hive start --prd .hive/prds/my-feature.json"
echo ""
echo "  ${DIM}# Monitor${RESET}"
echo "  hive status"
echo ""
echo "Documentation: https://github.com/mbourmaud/hive"
