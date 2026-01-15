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
# Spawn Command
# ============================================================================

show_spawn_usage() {
    cat << EOF
${CYAN}hive.sh spawn${NC} - Create a new Ralph with git worktree

${YELLOW}Usage:${NC}
  hive.sh spawn <name> --create <branch> [--from <base>]
  hive.sh spawn <name> --attach <branch> [--scope <glob>]

${YELLOW}Options:${NC}
  --create <branch>   Create a new branch for this Ralph
  --attach <branch>   Attach to an existing branch
  --from <base>       Base branch to create from (default: main)
  --scope <glob>      Restrict files this Ralph can modify

${YELLOW}Examples:${NC}
  hive.sh spawn auth-feature --create feature/auth
  hive.sh spawn auth-feature --create feature/auth --from develop
  hive.sh spawn fix-bug --attach feature/auth --scope "src/auth/*"
EOF
}

cmd_spawn() {
    check_git_repo
    check_dependencies

    local name=""
    local create_branch=""
    local attach_branch=""
    local from_base="main"
    local scope=""

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --create)
                if [[ -z "${2:-}" ]]; then
                    print_error "--create requires a branch name"
                    exit 1
                fi
                create_branch="$2"
                shift 2
                ;;
            --attach)
                if [[ -z "${2:-}" ]]; then
                    print_error "--attach requires a branch name"
                    exit 1
                fi
                attach_branch="$2"
                shift 2
                ;;
            --from)
                if [[ -z "${2:-}" ]]; then
                    print_error "--from requires a branch name"
                    exit 1
                fi
                from_base="$2"
                shift 2
                ;;
            --scope)
                if [[ -z "${2:-}" ]]; then
                    print_error "--scope requires a glob pattern"
                    exit 1
                fi
                scope="$2"
                shift 2
                ;;
            --help|-h)
                show_spawn_usage
                exit 0
                ;;
            -*)
                print_error "Unknown option: $1"
                show_spawn_usage
                exit 1
                ;;
            *)
                if [[ -z "$name" ]]; then
                    name="$1"
                else
                    print_error "Unexpected argument: $1"
                    show_spawn_usage
                    exit 1
                fi
                shift
                ;;
        esac
    done

    # Validate arguments
    if [[ -z "$name" ]]; then
        print_error "Ralph name is required"
        show_spawn_usage
        exit 1
    fi

    if [[ -z "$create_branch" && -z "$attach_branch" ]]; then
        print_error "Either --create or --attach is required"
        show_spawn_usage
        exit 1
    fi

    if [[ -n "$create_branch" && -n "$attach_branch" ]]; then
        print_error "Cannot use both --create and --attach"
        exit 1
    fi

    # Ensure hive is initialized
    if [[ ! -f "$CONFIG_FILE" ]]; then
        print_error "Hive not initialized. Run 'hive.sh init' first."
        exit 1
    fi

    # Check if ralph with this name already exists
    if jq -e --arg name "$name" '.ralphs[] | select(.name == $name)' "$CONFIG_FILE" > /dev/null 2>&1; then
        print_error "Ralph '$name' already exists. Use a different name or clean up first."
        exit 1
    fi

    local worktree_path="$WORKTREES_DIR/$name"

    # Check if worktree directory already exists
    if [[ -d "$worktree_path" ]]; then
        print_error "Worktree directory '$worktree_path' already exists."
        exit 1
    fi

    if [[ -n "$create_branch" ]]; then
        spawn_with_create "$name" "$create_branch" "$from_base" "$scope"
    else
        spawn_with_attach "$name" "$attach_branch" "$scope"
    fi
}

# Spawn with --create: Create a new branch
spawn_with_create() {
    local name="$1"
    local branch="$2"
    local from_base="$3"
    local scope="$4"
    local worktree_path="$WORKTREES_DIR/$name"

    print_info "Creating Ralph '$name' with new branch '$branch' from '$from_base'..."

    # Fetch latest from origin to ensure up-to-date base
    print_info "Fetching latest from origin..."
    if ! git fetch origin 2>/dev/null; then
        print_warning "Could not fetch from origin (offline or no remote?). Proceeding with local state."
    fi

    # Check if the base branch exists (locally or remote)
    local base_ref=""
    if git show-ref --verify --quiet "refs/heads/$from_base"; then
        base_ref="$from_base"
        print_info "Using local branch '$from_base' as base"
    elif git show-ref --verify --quiet "refs/remotes/origin/$from_base"; then
        base_ref="origin/$from_base"
        print_info "Using remote branch 'origin/$from_base' as base"
    else
        print_error "Base branch '$from_base' does not exist locally or on remote."
        exit 1
    fi

    # Check if the new branch already exists
    if git show-ref --verify --quiet "refs/heads/$branch"; then
        print_error "Branch '$branch' already exists locally. Use --attach to use existing branch, or delete it first."
        exit 1
    fi

    if git show-ref --verify --quiet "refs/remotes/origin/$branch"; then
        print_error "Branch '$branch' already exists on remote. Use --attach to use existing branch."
        exit 1
    fi

    # Create the worktree with a new branch
    print_info "Creating git worktree at '$worktree_path'..."
    if ! git worktree add -b "$branch" "$worktree_path" "$base_ref" 2>&1; then
        print_error "Failed to create worktree"
        exit 1
    fi
    print_success "Created worktree at '$worktree_path'"

    # Update config.json
    local timestamp=$(get_timestamp)
    local ralph_entry=$(jq -n \
        --arg name "$name" \
        --arg branch "$branch" \
        --arg baseBranch "$from_base" \
        --arg targetBranch "$from_base" \
        --arg scope "$scope" \
        --arg worktreePath "$worktree_path" \
        --arg createdAt "$timestamp" \
        '{
            name: $name,
            branch: $branch,
            branchMode: "created",
            baseBranch: $baseBranch,
            targetBranch: $targetBranch,
            scope: (if $scope == "" then null else $scope end),
            worktreePath: $worktreePath,
            status: "spawned",
            pid: null,
            pr: null,
            createdAt: $createdAt,
            startedAt: null
        }')

    # Add ralph entry to config
    local tmp_config=$(mktemp)
    jq --argjson ralph "$ralph_entry" \
       --arg timestamp "$timestamp" \
       '.ralphs += [$ralph] | .lastUpdate = $timestamp' \
       "$CONFIG_FILE" > "$tmp_config" && mv "$tmp_config" "$CONFIG_FILE"

    # Update branches section in config
    local branch_entry=$(jq -n \
        --arg branch "$branch" \
        --arg baseBranch "$from_base" \
        --arg createdBy "$name" \
        --arg createdAt "$timestamp" \
        '{
            baseBranch: $baseBranch,
            createdBy: $createdBy,
            createdAt: $createdAt,
            ralphs: [$createdBy]
        }')

    tmp_config=$(mktemp)
    jq --arg branch "$branch" \
       --argjson entry "$branch_entry" \
       --arg timestamp "$timestamp" \
       '.branches[$branch] = $entry | .lastUpdate = $timestamp' \
       "$CONFIG_FILE" > "$tmp_config" && mv "$tmp_config" "$CONFIG_FILE"

    print_success "Ralph '$name' spawned successfully!"
    echo ""
    echo "Branch: $branch (from $from_base)"
    echo "Worktree: $worktree_path"
    if [[ -n "$scope" ]]; then
        echo "Scope: $scope"
    fi
    echo ""
    echo "Next steps:"
    echo "  1. Start the Ralph: hive.sh start $name [prompt]"
    echo "  2. Check status: hive.sh status"
}

# Spawn with --attach: Attach to existing branch (placeholder for US-003)
spawn_with_attach() {
    local name="$1"
    local branch="$2"
    local scope="$3"

    print_error "The --attach option is not yet implemented (see US-003)"
    exit 1
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
        spawn)
            cmd_spawn "$@"
            ;;
        start|status|logs|stop|sync|pr|prs|cleanup|clean|dashboard)
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
