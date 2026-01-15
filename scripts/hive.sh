#!/bin/bash
#
# hive.sh - Multi-Ralph Orchestration via Bash
# Manages multiple Claude Code (Ralph) instances in parallel git worktrees
#
# Usage: hive.sh <command> [options]
#

set -e

# Colors for terminal output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Version
VERSION="0.1.0"

# Configuration paths
HIVE_DIR=".hive"
CONFIG_FILE="$HIVE_DIR/config.json"
WORKTREES_DIR="$HIVE_DIR/worktrees"
GITIGNORE_FILE="$HIVE_DIR/.gitignore"

# ============================================================================
# Helper Functions
# ============================================================================

# Print colored message
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

# Check if we're in a git repository
check_git_repo() {
    if ! git rev-parse --git-dir > /dev/null 2>&1; then
        print_error "Not a git repository. Please run this command from within a git repo."
        exit 1
    fi
}

# Check if jq is installed
check_dependencies() {
    if ! command -v jq &> /dev/null; then
        print_error "jq is required but not installed. Please install jq first."
        exit 1
    fi
}

# Get current ISO 8601 timestamp
get_timestamp() {
    date -u +"%Y-%m-%dT%H:%M:%SZ"
}

# ============================================================================
# Init Command
# ============================================================================

cmd_init() {
    check_git_repo
    check_dependencies

    print_info "Initializing Hive in current repository..."

    # Create .hive directory if it doesn't exist
    if [ ! -d "$HIVE_DIR" ]; then
        mkdir -p "$HIVE_DIR"
        print_success "Created $HIVE_DIR directory"
    else
        print_info "$HIVE_DIR directory already exists"
    fi

    # Create worktrees directory if it doesn't exist
    if [ ! -d "$WORKTREES_DIR" ]; then
        mkdir -p "$WORKTREES_DIR"
        print_success "Created $WORKTREES_DIR directory"
    else
        print_info "$WORKTREES_DIR directory already exists"
    fi

    # Create .gitignore if it doesn't exist
    if [ ! -f "$GITIGNORE_FILE" ]; then
        echo "worktrees/" > "$GITIGNORE_FILE"
        print_success "Created $GITIGNORE_FILE"
    else
        # Ensure worktrees/ is in gitignore
        if ! grep -q "^worktrees/$" "$GITIGNORE_FILE" 2>/dev/null; then
            echo "worktrees/" >> "$GITIGNORE_FILE"
            print_info "Added worktrees/ to $GITIGNORE_FILE"
        else
            print_info "$GITIGNORE_FILE already contains worktrees/"
        fi
    fi

    # Create config.json if it doesn't exist
    if [ ! -f "$CONFIG_FILE" ]; then
        local timestamp=$(get_timestamp)
        cat > "$CONFIG_FILE" << EOF
{
  "version": "1.0.0",
  "ralphs": [],
  "branches": {},
  "lastUpdate": "$timestamp"
}
EOF
        print_success "Created $CONFIG_FILE"
    else
        # Validate existing config has required fields
        if ! jq -e '.version and .ralphs and .branches and .lastUpdate' "$CONFIG_FILE" > /dev/null 2>&1; then
            print_warning "Existing config.json may be invalid. Please check the schema."
        else
            print_info "$CONFIG_FILE already exists and is valid"
        fi
    fi

    print_success "Hive initialized successfully!"
    echo ""
    echo "Next steps:"
    echo "  1. Spawn a Ralph: hive.sh spawn <name> --create <branch>"
    echo "  2. Start the Ralph: hive.sh start <name> [prompt]"
    echo "  3. Check status: hive.sh status"
}

# ============================================================================
# Main Command Router
# ============================================================================

show_usage() {
    cat << EOF
${CYAN}hive.sh${NC} - Multi-Ralph Orchestration via Bash
Version: $VERSION

${YELLOW}Usage:${NC}
  hive.sh <command> [options]

${YELLOW}Commands:${NC}
  init              Initialize Hive in current repository
  spawn             Create a new Ralph with git worktree
  start             Start a Ralph background process
  status            Show status of all Ralphs
  logs              View Ralph's output log
  stop              Stop a running Ralph
  sync              Sync worktree with target branch
  pr                Create Pull Request from Ralph's branch
  prs               List all PRs created by Hive
  cleanup           Remove worktree after PR merge
  clean             Remove worktree without PR (abandon)
  dashboard         Live status dashboard
  help              Show this help message

${YELLOW}Examples:${NC}
  hive.sh init
  hive.sh spawn auth-feature --create feature/auth --from main
  hive.sh spawn fix-bug --attach feature/auth --scope "src/auth/*"
  hive.sh start auth-feature "Implement user authentication"
  hive.sh status
  hive.sh logs auth-feature 100
  hive.sh stop auth-feature
  hive.sh pr auth-feature --draft

Run 'hive.sh <command> --help' for more information on a specific command.
EOF
}

# Main entry point
main() {
    local command="${1:-}"
    shift || true

    case "$command" in
        init)
            cmd_init "$@"
            ;;
        spawn|start|status|logs|stop|sync|pr|prs|cleanup|clean|dashboard)
            print_error "Command '$command' not yet implemented"
            exit 1
            ;;
        help|--help|-h|"")
            show_usage
            ;;
        --version|-v)
            echo "hive.sh version $VERSION"
            ;;
        *)
            print_error "Unknown command: $command"
            echo ""
            show_usage
            exit 1
            ;;
    esac
}

main "$@"
