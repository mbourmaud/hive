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

# Convert string to uppercase (bash 3.2 compatible)
to_upper() {
    echo "$1" | tr '[:lower:]' '[:upper:]'
}

# ============================================================================
# Init Command
# ============================================================================

show_init_usage() {
    cat << EOF
${CYAN}hive.sh init${NC} - Initialize Hive in current repository

${YELLOW}Usage:${NC}
  hive.sh init

${YELLOW}Options:${NC}
  --help, -h  Show this help message

${YELLOW}Description:${NC}
  Initializes the Hive directory structure for managing multiple Ralph
  instances. Creates the following:
  - .hive/             Main Hive directory
  - .hive/config.json  Configuration and state
  - .hive/worktrees/   Directory for git worktrees
  - .hive/.gitignore   Excludes worktrees from git

${YELLOW}Examples:${NC}
  cd my-project
  hive.sh init
EOF
}

cmd_init() {
    # Handle --help flag
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --help|-h)
                show_init_usage
                exit 0
                ;;
            *)
                print_error "Unknown option: $1"
                show_init_usage
                exit 1
                ;;
        esac
    done

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

# Spawn with --attach: Attach to existing branch
spawn_with_attach() {
    local name="$1"
    local branch="$2"
    local scope="$3"
    local worktree_path="$WORKTREES_DIR/$name"

    print_info "Creating Ralph '$name' attached to existing branch '$branch'..."

    # Fetch latest from origin
    print_info "Fetching latest from origin..."
    if ! git fetch origin 2>/dev/null; then
        print_warning "Could not fetch from origin (offline or no remote?). Proceeding with local state."
    fi

    # Check if branch exists (locally or remote)
    local branch_ref=""
    local branch_exists_locally=false
    local branch_exists_remote=false

    if git show-ref --verify --quiet "refs/heads/$branch"; then
        branch_exists_locally=true
    fi

    if git show-ref --verify --quiet "refs/remotes/origin/$branch"; then
        branch_exists_remote=true
    fi

    if [[ "$branch_exists_locally" == "false" && "$branch_exists_remote" == "false" ]]; then
        print_error "Branch '$branch' does not exist locally or on remote. Use --create to create a new branch."
        exit 1
    fi

    # Determine the base branch for this existing branch (try to detect from remote tracking)
    local base_branch="main"
    if [[ "$branch_exists_remote" == "true" ]]; then
        # Try to detect base branch from merge-base with main/master
        for candidate in main master develop; do
            if git show-ref --verify --quiet "refs/remotes/origin/$candidate" 2>/dev/null; then
                base_branch="$candidate"
                break
            fi
        done
    fi

    # Check if this branch is already checked out in another worktree
    local existing_worktree=""
    existing_worktree=$(git worktree list --porcelain | awk -v branch="refs/heads/$branch" '
        /^worktree / { wt = substr($0, 10) }
        /^branch / { if (substr($0, 8) == branch) print wt }
    ')

    if [[ -n "$existing_worktree" ]]; then
        # Branch is already checked out - share the existing worktree
        print_warning "Branch '$branch' is already checked out at '$existing_worktree'"
        print_info "Creating Ralph '$name' to share existing worktree..."
        worktree_path="$existing_worktree"

        # Pull latest changes in the existing worktree
        print_info "Pulling latest changes for branch '$branch'..."
        if ! (cd "$worktree_path" && git pull origin "$branch" 2>/dev/null); then
            print_warning "Could not pull from origin (offline or no remote tracking?). Using local state."
        fi
    else
        # Create worktree
        print_info "Creating git worktree at '$worktree_path'..."

        if [[ "$branch_exists_locally" == "true" ]]; then
            # Branch exists locally, use it directly
            if ! git worktree add "$worktree_path" "$branch" 2>&1; then
                print_error "Failed to create worktree"
                exit 1
            fi
        else
            # Branch only exists on remote, track it
            if ! git worktree add --track -b "$branch" "$worktree_path" "origin/$branch" 2>&1; then
                print_error "Failed to create worktree"
                exit 1
            fi
        fi
        print_success "Created worktree at '$worktree_path'"

        # Pull latest changes in the worktree
        print_info "Pulling latest changes for branch '$branch'..."
        if ! (cd "$worktree_path" && git pull origin "$branch" 2>/dev/null); then
            print_warning "Could not pull from origin (offline or no remote tracking?). Using local state."
        fi
    fi

    # Update config.json
    local timestamp=$(get_timestamp)
    local ralph_entry=$(jq -n \
        --arg name "$name" \
        --arg branch "$branch" \
        --arg baseBranch "$base_branch" \
        --arg targetBranch "$base_branch" \
        --arg scope "$scope" \
        --arg worktreePath "$worktree_path" \
        --arg createdAt "$timestamp" \
        '{
            name: $name,
            branch: $branch,
            branchMode: "attached",
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

    # Update or create branches section in config
    # Check if branch entry exists
    if jq -e --arg branch "$branch" '.branches[$branch]' "$CONFIG_FILE" > /dev/null 2>&1; then
        # Branch entry exists, add this Ralph to the ralphs array
        tmp_config=$(mktemp)
        jq --arg branch "$branch" \
           --arg name "$name" \
           --arg timestamp "$timestamp" \
           '.branches[$branch].ralphs += [$name] | .branches[$branch].ralphs |= unique | .lastUpdate = $timestamp' \
           "$CONFIG_FILE" > "$tmp_config" && mv "$tmp_config" "$CONFIG_FILE"
    else
        # Create new branch entry (for externally created branches)
        local branch_entry=$(jq -n \
            --arg branch "$branch" \
            --arg baseBranch "$base_branch" \
            --arg createdBy "external" \
            --arg createdAt "$timestamp" \
            --arg name "$name" \
            '{
                baseBranch: $baseBranch,
                createdBy: $createdBy,
                createdAt: $createdAt,
                ralphs: [$name]
            }')

        tmp_config=$(mktemp)
        jq --arg branch "$branch" \
           --argjson entry "$branch_entry" \
           --arg timestamp "$timestamp" \
           '.branches[$branch] = $entry | .lastUpdate = $timestamp' \
           "$CONFIG_FILE" > "$tmp_config" && mv "$tmp_config" "$CONFIG_FILE"
    fi

    print_success "Ralph '$name' spawned successfully!"
    echo ""
    echo "Branch: $branch (attached)"
    echo "Worktree: $worktree_path"
    if [[ -n "$scope" ]]; then
        echo "Scope: $scope"
    fi

    # Show warning if other Ralphs are already on this branch
    local ralph_count=$(jq --arg branch "$branch" '.branches[$branch].ralphs | length' "$CONFIG_FILE")
    if [[ "$ralph_count" -gt 1 ]]; then
        echo ""
        print_warning "Multiple Ralphs ($ralph_count) are now attached to branch '$branch'."
        print_warning "Consider using --scope to partition work and avoid conflicts."
    fi

    echo ""
    echo "Next steps:"
    echo "  1. Start the Ralph: hive.sh start $name [prompt]"
    echo "  2. Check status: hive.sh status"
}

# ============================================================================
# Start Command
# ============================================================================

show_start_usage() {
    cat << EOF
${CYAN}hive.sh start${NC} - Start a Ralph background process

${YELLOW}Usage:${NC}
  hive.sh start <name> [prompt]

${YELLOW}Arguments:${NC}
  name        Name of the Ralph to start (must be spawned first)
  prompt      Optional custom prompt for Ralph (default: autonomous work prompt)

${YELLOW}Options:${NC}
  --help, -h  Show this help message

${YELLOW}Examples:${NC}
  hive.sh start auth-feature
  hive.sh start auth-feature "Implement user authentication with JWT"
  hive.sh start fix-bug "Fix the login validation bug in src/auth/login.ts"
EOF
}

cmd_start() {
    check_git_repo
    check_dependencies

    local name=""
    local prompt=""

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --help|-h)
                show_start_usage
                exit 0
                ;;
            -*)
                print_error "Unknown option: $1"
                show_start_usage
                exit 1
                ;;
            *)
                if [[ -z "$name" ]]; then
                    name="$1"
                elif [[ -z "$prompt" ]]; then
                    prompt="$1"
                else
                    print_error "Too many arguments"
                    show_start_usage
                    exit 1
                fi
                shift
                ;;
        esac
    done

    # Validate arguments
    if [[ -z "$name" ]]; then
        print_error "Ralph name is required"
        show_start_usage
        exit 1
    fi

    # Ensure hive is initialized
    if [[ ! -f "$CONFIG_FILE" ]]; then
        print_error "Hive not initialized. Run 'hive.sh init' first."
        exit 1
    fi

    # Check if ralph exists
    local ralph_entry
    ralph_entry=$(jq --arg name "$name" '.ralphs[] | select(.name == $name)' "$CONFIG_FILE" 2>/dev/null || true)
    if [[ -z "$ralph_entry" ]]; then
        print_error "Ralph '$name' does not exist. Spawn it first with 'hive.sh spawn'."
        exit 1
    fi

    # Get ralph info
    local worktree_path
    local status
    local scope
    local branch
    local existing_pid

    worktree_path=$(echo "$ralph_entry" | jq -r '.worktreePath')
    status=$(echo "$ralph_entry" | jq -r '.status')
    scope=$(echo "$ralph_entry" | jq -r '.scope // empty')
    branch=$(echo "$ralph_entry" | jq -r '.branch')
    existing_pid=$(echo "$ralph_entry" | jq -r '.pid // empty')

    # Check if already running
    if [[ "$status" == "running" ]] && [[ -n "$existing_pid" ]]; then
        # Verify PID is still running
        if kill -0 "$existing_pid" 2>/dev/null; then
            print_error "Ralph '$name' is already running (PID: $existing_pid)"
            print_info "Use 'hive.sh stop $name' to stop it first, or 'hive.sh logs $name' to view output."
            exit 1
        else
            print_warning "Ralph '$name' was marked as running but process (PID: $existing_pid) is dead."
            print_info "Updating status and restarting..."
        fi
    fi

    # Verify worktree exists
    if [[ ! -d "$worktree_path" ]]; then
        print_error "Worktree directory '$worktree_path' does not exist."
        print_info "The worktree may have been removed. Consider cleaning up with 'hive.sh clean $name'."
        exit 1
    fi

    # Check if claude CLI is available
    if ! command -v claude &> /dev/null; then
        print_error "claude CLI is not installed or not in PATH."
        print_info "Please install Claude Code first: https://claude.ai/code"
        exit 1
    fi

    print_info "Starting Ralph '$name' on branch '$branch'..."

    # Build the prompt
    local final_prompt=""
    if [[ -n "$prompt" ]]; then
        final_prompt="$prompt"
    else
        # Default autonomous work prompt
        final_prompt="You are Ralph, an autonomous coding agent. Work on the tasks in this repository.

Review the codebase, identify what needs to be done, and implement it. Commit your changes as you go.

Work autonomously until the task is complete or you need human input."
    fi

    # Add scope restriction if set
    if [[ -n "$scope" ]]; then
        final_prompt="$final_prompt

IMPORTANT: You are restricted to modifying only files matching the pattern: $scope
Do not modify files outside this scope."
    fi

    # Create log file directory if needed (worktree should exist)
    local log_file="$worktree_path/ralph-output.log"

    # Initialize log file with header
    cat > "$log_file" << EOF
# Ralph Output Log
# Ralph: $name
# Branch: $branch
# Started: $(get_timestamp)
# Worktree: $worktree_path
$(if [[ -n "$scope" ]]; then echo "# Scope: $scope"; fi)
---

EOF

    # Launch claude in background
    print_info "Launching Claude Code in background..."
    print_info "Log file: $log_file"

    # Create a PID file to capture the process ID
    local pid_file="$worktree_path/.ralph-pid"

    # Change to worktree directory and run claude in background
    # Using nohup to keep it running after terminal closes
    # The subshell writes the PID to a file for reliable capture
    (
        cd "$worktree_path"
        nohup claude --dangerously-skip-permissions -p "$final_prompt" >> "$log_file" 2>&1 &
        echo $! > "$pid_file"
    )

    # Give it a moment to start
    sleep 1

    # Read the PID from the file
    local pid=""
    if [[ -f "$pid_file" ]]; then
        pid=$(cat "$pid_file" 2>/dev/null || true)
        rm -f "$pid_file"
    fi

    # Update config.json with PID and status
    local timestamp=$(get_timestamp)
    local tmp_config=$(mktemp)

    if [[ -n "$pid" ]]; then
        jq --arg name "$name" \
           --argjson pid "$pid" \
           --arg status "running" \
           --arg timestamp "$timestamp" \
           '(.ralphs[] | select(.name == $name)) |= . + {pid: $pid, status: $status, startedAt: $timestamp} | .lastUpdate = $timestamp' \
           "$CONFIG_FILE" > "$tmp_config" && mv "$tmp_config" "$CONFIG_FILE"

        print_success "Ralph '$name' started successfully!"
        echo ""
        echo "PID: $pid"
        echo "Branch: $branch"
        echo "Log file: $log_file"
        if [[ -n "$scope" ]]; then
            echo "Scope: $scope"
        fi
    else
        # Started but couldn't get PID - still update status
        jq --arg name "$name" \
           --arg status "running" \
           --arg timestamp "$timestamp" \
           '(.ralphs[] | select(.name == $name)) |= . + {pid: null, status: $status, startedAt: $timestamp} | .lastUpdate = $timestamp' \
           "$CONFIG_FILE" > "$tmp_config" && mv "$tmp_config" "$CONFIG_FILE"

        print_warning "Ralph '$name' started but could not capture PID."
        print_info "The process may still be running. Check logs for activity."
        echo ""
        echo "Branch: $branch"
        echo "Log file: $log_file"
    fi

    echo ""
    echo "Next steps:"
    echo "  1. Monitor progress: hive.sh logs $name"
    echo "  2. Check status: hive.sh status"
    echo "  3. Stop Ralph: hive.sh stop $name"
}

# ============================================================================
# Status Command
# ============================================================================

show_status_usage() {
    cat << EOF
${CYAN}hive.sh status${NC} - Show status of all Ralphs

${YELLOW}Usage:${NC}
  hive.sh status

${YELLOW}Options:${NC}
  --help, -h  Show this help message

${YELLOW}Output includes:${NC}
  - Ralph name and status (color-coded)
  - Branch and target branch
  - Scope restrictions (if any)
  - Branch sharing warnings
  - PR status (if created)
  - Last activity for running Ralphs

${YELLOW}Status colors:${NC}
  ${GREEN}●${NC} running/completed - Active or finished
  ${YELLOW}●${NC} spawned/stopped   - Ready or paused
  ${RED}●${NC} failed            - Error state
EOF
}

cmd_status() {
    check_git_repo
    check_dependencies

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --help|-h)
                show_status_usage
                exit 0
                ;;
            -*)
                print_error "Unknown option: $1"
                show_status_usage
                exit 1
                ;;
            *)
                print_error "Unexpected argument: $1"
                show_status_usage
                exit 1
                ;;
        esac
    done

    # Ensure hive is initialized
    if [[ ! -f "$CONFIG_FILE" ]]; then
        print_error "Hive not initialized. Run 'hive.sh init' first."
        exit 1
    fi

    # Get all ralphs from config
    local ralph_count
    ralph_count=$(jq '.ralphs | length' "$CONFIG_FILE")

    if [[ "$ralph_count" -eq 0 ]]; then
        echo ""
        print_info "No Ralphs have been spawned yet."
        echo ""
        echo "Get started with:"
        echo "  hive.sh spawn <name> --create <branch>"
        echo ""
        exit 0
    fi

    echo ""
    echo -e "${CYAN}═══════════════════════════════════════════════════════════════════${NC}"
    echo -e "${CYAN}                         HIVE STATUS DASHBOARD                       ${NC}"
    echo -e "${CYAN}═══════════════════════════════════════════════════════════════════${NC}"
    echo ""

    # Track which branches have multiple Ralphs for warning display
    local branch_ralphs=()

    # Iterate through all Ralphs
    local i=0
    while [[ $i -lt $ralph_count ]]; do
        local ralph_json
        ralph_json=$(jq --argjson i "$i" '.ralphs[$i]' "$CONFIG_FILE")

        # Extract ralph fields
        local name branch branch_mode status pid scope worktree_path pr_json target_branch started_at
        name=$(echo "$ralph_json" | jq -r '.name')
        branch=$(echo "$ralph_json" | jq -r '.branch')
        branch_mode=$(echo "$ralph_json" | jq -r '.branchMode')
        status=$(echo "$ralph_json" | jq -r '.status')
        pid=$(echo "$ralph_json" | jq -r '.pid // empty')
        scope=$(echo "$ralph_json" | jq -r '.scope // empty')
        worktree_path=$(echo "$ralph_json" | jq -r '.worktreePath')
        pr_json=$(echo "$ralph_json" | jq '.pr')
        target_branch=$(echo "$ralph_json" | jq -r '.targetBranch')
        started_at=$(echo "$ralph_json" | jq -r '.startedAt // empty')

        # Verify PID liveness and update status if needed
        local actual_status="$status"
        local status_changed=false

        if [[ "$status" == "running" ]] && [[ -n "$pid" ]]; then
            if ! kill -0 "$pid" 2>/dev/null; then
                # Process is dead, update status
                actual_status="stopped"
                status_changed=true

                # Update config.json
                local timestamp=$(get_timestamp)
                local tmp_config=$(mktemp)
                jq --arg name "$name" \
                   --arg status "stopped" \
                   --arg timestamp "$timestamp" \
                   '(.ralphs[] | select(.name == $name)) |= . + {status: $status, pid: null} | .lastUpdate = $timestamp' \
                   "$CONFIG_FILE" > "$tmp_config" && mv "$tmp_config" "$CONFIG_FILE"
            fi
        fi

        # Determine status color and symbol
        local status_color status_symbol
        case "$actual_status" in
            running)
                status_color="${GREEN}"
                status_symbol="●"
                ;;
            completed|pr_created)
                status_color="${GREEN}"
                status_symbol="✓"
                ;;
            spawned|stopped)
                status_color="${YELLOW}"
                status_symbol="○"
                ;;
            failed)
                status_color="${RED}"
                status_symbol="✗"
                ;;
            *)
                status_color="${NC}"
                status_symbol="?"
                ;;
        esac

        # Print Ralph header
        echo -e "${status_color}${status_symbol}${NC} ${CYAN}${name}${NC}"
        echo -e "  Status:   ${status_color}${actual_status}${NC}$(if [[ "$status_changed" == true ]]; then echo -e " ${YELLOW}(process died)${NC}"; fi)"
        echo -e "  Branch:   ${branch} (${branch_mode}) → ${target_branch}"

        # Show scope if set
        if [[ -n "$scope" ]]; then
            echo -e "  Scope:    ${YELLOW}${scope}${NC}"
        fi

        # Show worktree path
        echo -e "  Worktree: ${worktree_path}"

        # Show PID if running
        if [[ "$actual_status" == "running" ]] && [[ -n "$pid" ]]; then
            echo -e "  PID:      ${pid}"
        fi

        # Show started timestamp
        if [[ -n "$started_at" ]]; then
            echo -e "  Started:  ${started_at}"
        fi

        # Show PR status if exists
        if [[ "$pr_json" != "null" ]] && [[ -n "$pr_json" ]]; then
            local pr_number pr_url pr_state
            pr_number=$(echo "$pr_json" | jq -r '.number // empty')
            pr_url=$(echo "$pr_json" | jq -r '.url // empty')
            pr_state=$(echo "$pr_json" | jq -r '.state // "unknown"')

            if [[ -n "$pr_number" ]]; then
                # Try to get current PR status from GitHub
                local gh_pr_state=""
                local gh_check_status=""
                local gh_review_status=""

                if command -v gh &> /dev/null; then
                    # Get PR state
                    gh_pr_state=$(gh pr view "$pr_number" --json state -q '.state' 2>/dev/null || echo "")

                    if [[ -n "$gh_pr_state" ]]; then
                        pr_state="$gh_pr_state"

                        # Get CI check status
                        gh_check_status=$(gh pr view "$pr_number" --json statusCheckRollup -q '.statusCheckRollup[0].conclusion // .statusCheckRollup[0].status // "pending"' 2>/dev/null || echo "")

                        # Get review status
                        gh_review_status=$(gh pr view "$pr_number" --json reviewDecision -q '.reviewDecision // "PENDING"' 2>/dev/null || echo "")
                    fi
                fi

                # Color code PR state
                local pr_state_color
                case "$pr_state" in
                    OPEN|open)
                        pr_state_color="${GREEN}"
                        ;;
                    MERGED|merged)
                        pr_state_color="${CYAN}"
                        ;;
                    CLOSED|closed)
                        pr_state_color="${RED}"
                        ;;
                    *)
                        pr_state_color="${NC}"
                        ;;
                esac

                echo -e "  PR:       #${pr_number} ${pr_state_color}${pr_state}${NC}"

                if [[ -n "$gh_check_status" ]]; then
                    local check_color
                    case "$gh_check_status" in
                        SUCCESS|success)
                            check_color="${GREEN}"
                            ;;
                        FAILURE|failure)
                            check_color="${RED}"
                            ;;
                        PENDING|pending|IN_PROGRESS|in_progress)
                            check_color="${YELLOW}"
                            ;;
                        *)
                            check_color="${NC}"
                            ;;
                    esac
                    echo -e "  CI:       ${check_color}${gh_check_status}${NC}"
                fi

                if [[ -n "$gh_review_status" ]] && [[ "$gh_review_status" != "null" ]]; then
                    local review_color
                    case "$gh_review_status" in
                        APPROVED|approved)
                            review_color="${GREEN}"
                            ;;
                        CHANGES_REQUESTED|changes_requested)
                            review_color="${RED}"
                            ;;
                        *)
                            review_color="${YELLOW}"
                            ;;
                    esac
                    echo -e "  Review:   ${review_color}${gh_review_status}${NC}"
                fi

                if [[ -n "$pr_url" ]]; then
                    echo -e "  PR URL:   ${pr_url}"
                fi
            fi
        fi

        # Show last line of log for running Ralphs
        if [[ "$actual_status" == "running" ]]; then
            local log_file="$worktree_path/ralph-output.log"
            if [[ -f "$log_file" ]]; then
                # Get last non-empty, non-header line
                local last_line
                last_line=$(grep -v '^#' "$log_file" | grep -v '^---' | grep -v '^$' | tail -1 2>/dev/null || echo "")
                if [[ -n "$last_line" ]]; then
                    # Truncate if too long
                    if [[ ${#last_line} -gt 60 ]]; then
                        last_line="${last_line:0:57}..."
                    fi
                    echo -e "  Activity: ${BLUE}${last_line}${NC}"
                fi
            fi
        fi

        echo ""
        i=$((i + 1))
    done

    # Show branch relationships section
    echo -e "${CYAN}───────────────────────────────────────────────────────────────────${NC}"
    echo -e "${CYAN}BRANCH RELATIONSHIPS${NC}"
    echo ""

    # Get all branches with their Ralphs
    local branches
    branches=$(jq -r '.branches | to_entries[] | "\(.key)|\(.value.ralphs | join(","))"' "$CONFIG_FILE" 2>/dev/null || echo "")

    if [[ -n "$branches" ]]; then
        while IFS='|' read -r branch_name ralph_list; do
            local ralph_array
            IFS=',' read -ra ralph_array <<< "$ralph_list"
            local ralph_count_on_branch=${#ralph_array[@]}

            if [[ $ralph_count_on_branch -gt 1 ]]; then
                echo -e "  ${YELLOW}⚠${NC}  ${branch_name}: ${ralph_list}"
                echo -e "      ${YELLOW}Warning: Multiple Ralphs ($ralph_count_on_branch) sharing this branch${NC}"
            else
                echo -e "  ${GREEN}✓${NC}  ${branch_name}: ${ralph_list}"
            fi
        done <<< "$branches"
    else
        echo "  No branches tracked"
    fi

    echo ""
    echo -e "${CYAN}───────────────────────────────────────────────────────────────────${NC}"
    local running_count stopped_count pr_count
    running_count=$(jq '[.ralphs[] | select(.status == "running")] | length' "$CONFIG_FILE")
    stopped_count=$(jq '[.ralphs[] | select(.status == "spawned" or .status == "stopped")] | length' "$CONFIG_FILE")
    pr_count=$(jq '[.ralphs[] | select(.pr != null)] | length' "$CONFIG_FILE")

    echo -e "  Total: ${ralph_count} Ralph(s) | ${GREEN}Running: ${running_count}${NC} | ${YELLOW}Idle: ${stopped_count}${NC} | PRs: ${pr_count}"
    echo -e "  Last update: $(jq -r '.lastUpdate' "$CONFIG_FILE")"
    echo ""
}

# ============================================================================
# Logs Command
# ============================================================================

show_logs_usage() {
    cat << EOF
${CYAN}hive.sh logs${NC} - View Ralph's output log

${YELLOW}Usage:${NC}
  hive.sh logs <name> [lines]

${YELLOW}Arguments:${NC}
  name        Name of the Ralph whose logs to view
  lines       Number of lines to show (default: 50)

${YELLOW}Options:${NC}
  --follow, -f  Follow the log output (live tailing)
  --help, -h    Show this help message

${YELLOW}Examples:${NC}
  hive.sh logs auth-feature          # Show last 50 lines
  hive.sh logs auth-feature 100      # Show last 100 lines
  hive.sh logs auth-feature --follow # Live tail the log
  hive.sh logs auth-feature -f       # Same as --follow
EOF
}

cmd_logs() {
    check_git_repo
    check_dependencies

    local name=""
    local lines=50
    local follow=false

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --follow|-f)
                follow=true
                shift
                ;;
            --help|-h)
                show_logs_usage
                exit 0
                ;;
            -*)
                print_error "Unknown option: $1"
                show_logs_usage
                exit 1
                ;;
            *)
                if [[ -z "$name" ]]; then
                    name="$1"
                elif [[ "$1" =~ ^[0-9]+$ ]]; then
                    lines="$1"
                else
                    print_error "Unexpected argument: $1"
                    show_logs_usage
                    exit 1
                fi
                shift
                ;;
        esac
    done

    # Validate arguments
    if [[ -z "$name" ]]; then
        print_error "Ralph name is required"
        show_logs_usage
        exit 1
    fi

    # Ensure hive is initialized
    if [[ ! -f "$CONFIG_FILE" ]]; then
        print_error "Hive not initialized. Run 'hive.sh init' first."
        exit 1
    fi

    # Check if ralph exists
    local ralph_entry
    ralph_entry=$(jq --arg name "$name" '.ralphs[] | select(.name == $name)' "$CONFIG_FILE" 2>/dev/null || true)
    if [[ -z "$ralph_entry" ]]; then
        print_error "Ralph '$name' does not exist."
        exit 1
    fi

    # Get worktree path
    local worktree_path
    worktree_path=$(echo "$ralph_entry" | jq -r '.worktreePath')

    # Build log file path
    local log_file="$worktree_path/ralph-output.log"

    # Check if log file exists
    if [[ ! -f "$log_file" ]]; then
        print_error "No log file found for Ralph '$name'."
        print_info "The Ralph may not have been started yet."
        print_info "Expected log file: $log_file"
        exit 1
    fi

    # Get ralph status for context
    local status
    status=$(echo "$ralph_entry" | jq -r '.status')

    if [[ "$follow" == true ]]; then
        # Live tail mode
        print_info "Following log for Ralph '$name' (status: $status)"
        print_info "Press Ctrl+C to stop"
        echo ""
        tail -f "$log_file"
    else
        # Show last N lines
        print_info "Showing last $lines lines for Ralph '$name' (status: $status)"
        print_info "Log file: $log_file"
        echo ""
        echo -e "${CYAN}───────────────────────────────────────────────────────────────────${NC}"
        tail -n "$lines" "$log_file"
        echo -e "${CYAN}───────────────────────────────────────────────────────────────────${NC}"
        echo ""

        # Show hint about follow mode if Ralph is running
        if [[ "$status" == "running" ]]; then
            print_info "Tip: Use 'hive.sh logs $name --follow' for live tailing"
        fi
    fi
}

# ============================================================================
# Stop Command
# ============================================================================

show_stop_usage() {
    cat << EOF
${CYAN}hive.sh stop${NC} - Stop a running Ralph process

${YELLOW}Usage:${NC}
  hive.sh stop <name>

${YELLOW}Arguments:${NC}
  name        Name of the Ralph to stop

${YELLOW}Options:${NC}
  --help, -h  Show this help message

${YELLOW}Behavior:${NC}
  - Sends SIGTERM to gracefully stop the Ralph process
  - Waits up to 5 seconds for graceful shutdown
  - Sends SIGKILL if process doesn't stop gracefully
  - Updates status to 'stopped' in config
  - Preserves worktree and branch for later continuation
  - No-op if Ralph is already stopped

${YELLOW}Examples:${NC}
  hive.sh stop auth-feature
  hive.sh stop fix-bug
EOF
}

cmd_stop() {
    check_git_repo
    check_dependencies

    local name=""

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --help|-h)
                show_stop_usage
                exit 0
                ;;
            -*)
                print_error "Unknown option: $1"
                show_stop_usage
                exit 1
                ;;
            *)
                if [[ -z "$name" ]]; then
                    name="$1"
                else
                    print_error "Unexpected argument: $1"
                    show_stop_usage
                    exit 1
                fi
                shift
                ;;
        esac
    done

    # Validate arguments
    if [[ -z "$name" ]]; then
        print_error "Ralph name is required"
        show_stop_usage
        exit 1
    fi

    # Ensure hive is initialized
    if [[ ! -f "$CONFIG_FILE" ]]; then
        print_error "Hive not initialized. Run 'hive.sh init' first."
        exit 1
    fi

    # Check if ralph exists
    local ralph_entry
    ralph_entry=$(jq --arg name "$name" '.ralphs[] | select(.name == $name)' "$CONFIG_FILE" 2>/dev/null || true)
    if [[ -z "$ralph_entry" ]]; then
        print_error "Ralph '$name' does not exist."
        exit 1
    fi

    # Get ralph info
    local status pid branch
    status=$(echo "$ralph_entry" | jq -r '.status')
    pid=$(echo "$ralph_entry" | jq -r '.pid // empty')
    branch=$(echo "$ralph_entry" | jq -r '.branch')

    # Check if already stopped
    if [[ "$status" != "running" ]]; then
        print_info "Ralph '$name' is not running (status: $status)."
        print_info "No action needed."
        exit 0
    fi

    # Check if PID exists and is valid
    if [[ -z "$pid" ]]; then
        print_warning "Ralph '$name' was marked as running but has no PID."
        print_info "Updating status to 'stopped'..."

        # Update config.json
        local timestamp=$(get_timestamp)
        local tmp_config=$(mktemp)
        jq --arg name "$name" \
           --arg status "stopped" \
           --arg timestamp "$timestamp" \
           '(.ralphs[] | select(.name == $name)) |= . + {status: $status, pid: null} | .lastUpdate = $timestamp' \
           "$CONFIG_FILE" > "$tmp_config" && mv "$tmp_config" "$CONFIG_FILE"

        print_success "Ralph '$name' status updated to 'stopped'."
        exit 0
    fi

    # Check if process is still running
    if ! kill -0 "$pid" 2>/dev/null; then
        print_warning "Ralph '$name' process (PID: $pid) is no longer running."
        print_info "Updating status to 'stopped'..."

        # Update config.json
        local timestamp=$(get_timestamp)
        local tmp_config=$(mktemp)
        jq --arg name "$name" \
           --arg status "stopped" \
           --arg timestamp "$timestamp" \
           '(.ralphs[] | select(.name == $name)) |= . + {status: $status, pid: null} | .lastUpdate = $timestamp' \
           "$CONFIG_FILE" > "$tmp_config" && mv "$tmp_config" "$CONFIG_FILE"

        print_success "Ralph '$name' status updated to 'stopped'."
        exit 0
    fi

    print_info "Stopping Ralph '$name' (PID: $pid) on branch '$branch'..."

    # Send SIGTERM for graceful shutdown
    print_info "Sending SIGTERM for graceful shutdown..."
    kill -TERM "$pid" 2>/dev/null || true

    # Wait up to 5 seconds for graceful shutdown
    local wait_count=0
    local max_wait=5
    while [[ $wait_count -lt $max_wait ]]; do
        if ! kill -0 "$pid" 2>/dev/null; then
            print_success "Ralph '$name' stopped gracefully."
            break
        fi
        sleep 1
        wait_count=$((wait_count + 1))
        print_info "Waiting for process to stop... ($wait_count/$max_wait)"
    done

    # If still running, send SIGKILL
    if kill -0 "$pid" 2>/dev/null; then
        print_warning "Process didn't stop gracefully. Sending SIGKILL..."
        kill -KILL "$pid" 2>/dev/null || true
        sleep 1

        if kill -0 "$pid" 2>/dev/null; then
            print_error "Failed to kill process (PID: $pid). It may require manual intervention."
            print_info "Try: kill -9 $pid"
            exit 1
        else
            print_success "Ralph '$name' force-stopped with SIGKILL."
        fi
    fi

    # Update config.json
    local timestamp=$(get_timestamp)
    local tmp_config=$(mktemp)
    jq --arg name "$name" \
       --arg status "stopped" \
       --arg timestamp "$timestamp" \
       '(.ralphs[] | select(.name == $name)) |= . + {status: $status, pid: null} | .lastUpdate = $timestamp' \
       "$CONFIG_FILE" > "$tmp_config" && mv "$tmp_config" "$CONFIG_FILE"

    print_success "Ralph '$name' stopped successfully!"
    echo ""
    echo "Branch '$branch' and worktree preserved for later continuation."
    echo ""
    echo "Next steps:"
    echo "  1. Restart Ralph: hive.sh start $name"
    echo "  2. Check status: hive.sh status"
    echo "  3. Create PR: hive.sh pr $name"
}

# ============================================================================
# Sync Command
# ============================================================================

show_sync_usage() {
    cat << EOF
${CYAN}hive.sh sync${NC} - Sync worktree with target branch

${YELLOW}Usage:${NC}
  hive.sh sync <name>

${YELLOW}Arguments:${NC}
  name        Name of the Ralph whose worktree to sync

${YELLOW}Options:${NC}
  --help, -h  Show this help message

${YELLOW}Behavior:${NC}
  - Fetches latest changes from origin
  - Merges targetBranch (usually main) into Ralph's branch
  - Reports merge conflicts if any occur
  - Pauses Ralph if running, then resumes after sync
  - Use after another Ralph's PR is merged to get latest changes

${YELLOW}Examples:${NC}
  hive.sh sync auth-feature    # Merge main into auth-feature's branch
  hive.sh sync fix-bug         # Sync fix-bug worktree with its target
EOF
}

cmd_sync() {
    check_git_repo
    check_dependencies

    local name=""

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --help|-h)
                show_sync_usage
                exit 0
                ;;
            -*)
                print_error "Unknown option: $1"
                show_sync_usage
                exit 1
                ;;
            *)
                if [[ -z "$name" ]]; then
                    name="$1"
                else
                    print_error "Unexpected argument: $1"
                    show_sync_usage
                    exit 1
                fi
                shift
                ;;
        esac
    done

    # Validate arguments
    if [[ -z "$name" ]]; then
        print_error "Ralph name is required"
        show_sync_usage
        exit 1
    fi

    # Ensure hive is initialized
    if [[ ! -f "$CONFIG_FILE" ]]; then
        print_error "Hive not initialized. Run 'hive.sh init' first."
        exit 1
    fi

    # Check if ralph exists
    local ralph_entry
    ralph_entry=$(jq --arg name "$name" '.ralphs[] | select(.name == $name)' "$CONFIG_FILE" 2>/dev/null || true)
    if [[ -z "$ralph_entry" ]]; then
        print_error "Ralph '$name' does not exist."
        exit 1
    fi

    # Get ralph info
    local worktree_path branch target_branch status pid
    worktree_path=$(echo "$ralph_entry" | jq -r '.worktreePath')
    branch=$(echo "$ralph_entry" | jq -r '.branch')
    target_branch=$(echo "$ralph_entry" | jq -r '.targetBranch')
    status=$(echo "$ralph_entry" | jq -r '.status')
    pid=$(echo "$ralph_entry" | jq -r '.pid // empty')

    # Verify worktree exists
    if [[ ! -d "$worktree_path" ]]; then
        print_error "Worktree directory '$worktree_path' does not exist."
        print_info "The worktree may have been removed. Consider cleaning up with 'hive.sh clean $name'."
        exit 1
    fi

    print_info "Syncing Ralph '$name' worktree with '$target_branch'..."

    # Track if we paused a running Ralph
    local was_running=false
    local original_pid=""

    # If Ralph is running, pause it first
    if [[ "$status" == "running" ]] && [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
        print_warning "Ralph '$name' is currently running. Pausing for sync..."
        was_running=true
        original_pid="$pid"

        # Send SIGSTOP to pause (not kill) the process
        kill -STOP "$pid" 2>/dev/null || true
        print_info "Ralph process paused (PID: $pid)"
    fi

    # Change to worktree directory for git operations
    cd "$worktree_path"

    # Fetch latest from origin
    print_info "Fetching latest from origin..."
    if ! git fetch origin 2>&1; then
        print_warning "Could not fetch from origin. Proceeding with local state."
    fi

    # Check for uncommitted changes
    local has_changes=false
    if ! git diff --quiet 2>/dev/null || ! git diff --cached --quiet 2>/dev/null; then
        has_changes=true
        print_warning "Worktree has uncommitted changes."
        print_info "Stashing changes before merge..."

        if ! git stash push -m "hive-sync-$(date +%s)" 2>&1; then
            print_error "Failed to stash changes."

            # Resume Ralph if we paused it
            if [[ "$was_running" == true ]] && [[ -n "$original_pid" ]]; then
                kill -CONT "$original_pid" 2>/dev/null || true
                print_info "Ralph process resumed"
            fi
            exit 1
        fi
        print_success "Changes stashed"
    fi

    # Attempt to merge target branch
    print_info "Merging 'origin/$target_branch' into '$branch'..."

    local merge_result=0
    local merge_output
    merge_output=$(git merge "origin/$target_branch" -m "Merge $target_branch into $branch (hive sync)" 2>&1) || merge_result=$?

    if [[ $merge_result -ne 0 ]]; then
        # Check if it's a conflict
        if git ls-files --unmerged | grep -q .; then
            print_error "Merge conflicts detected!"
            echo ""
            echo -e "${YELLOW}Conflicting files:${NC}"
            git ls-files --unmerged | cut -f2 | sort -u | while read -r file; do
                echo "  - $file"
            done
            echo ""
            echo -e "${YELLOW}To resolve conflicts:${NC}"
            echo "  1. cd $worktree_path"
            echo "  2. Edit the conflicting files to resolve conflicts"
            echo "  3. git add <resolved-files>"
            echo "  4. git commit"
            echo ""
            echo -e "${YELLOW}To abort the merge:${NC}"
            echo "  cd $worktree_path && git merge --abort"

            # Pop stash if we stashed changes (but warn about potential conflicts)
            if [[ "$has_changes" == true ]]; then
                echo ""
                print_warning "Your uncommitted changes are stashed. After resolving merge conflicts:"
                echo "  git stash pop"
            fi

            # Resume Ralph if we paused it
            if [[ "$was_running" == true ]] && [[ -n "$original_pid" ]]; then
                print_info "Ralph process left paused due to conflicts."
                print_info "Resume manually after resolving: kill -CONT $original_pid"
            fi

            exit 1
        else
            # Some other merge error
            print_error "Merge failed: $merge_output"

            # Resume Ralph if we paused it
            if [[ "$was_running" == true ]] && [[ -n "$original_pid" ]]; then
                kill -CONT "$original_pid" 2>/dev/null || true
                print_info "Ralph process resumed"
            fi
            exit 1
        fi
    fi

    print_success "Merged successfully!"

    # Pop stash if we stashed changes
    if [[ "$has_changes" == true ]]; then
        print_info "Restoring stashed changes..."
        if ! git stash pop 2>&1; then
            print_warning "Failed to restore stashed changes. They remain in stash."
            print_info "To restore manually: cd $worktree_path && git stash pop"
        else
            print_success "Stashed changes restored"
        fi
    fi

    # Resume Ralph if we paused it
    if [[ "$was_running" == true ]] && [[ -n "$original_pid" ]]; then
        if kill -0 "$original_pid" 2>/dev/null; then
            kill -CONT "$original_pid" 2>/dev/null || true
            print_success "Ralph process resumed (PID: $original_pid)"
        else
            print_warning "Ralph process (PID: $original_pid) is no longer running."

            # Update config.json since process died
            local timestamp=$(get_timestamp)
            local tmp_config=$(mktemp)
            jq --arg name "$name" \
               --arg status "stopped" \
               --arg timestamp "$timestamp" \
               '(.ralphs[] | select(.name == $name)) |= . + {status: $status, pid: null} | .lastUpdate = $timestamp' \
               "$CONFIG_FILE" > "$tmp_config" && mv "$tmp_config" "$CONFIG_FILE"
        fi
    fi

    # Update lastUpdate in config
    cd - > /dev/null
    local timestamp=$(get_timestamp)
    local tmp_config=$(mktemp)
    jq --arg timestamp "$timestamp" \
       '.lastUpdate = $timestamp' \
       "$CONFIG_FILE" > "$tmp_config" && mv "$tmp_config" "$CONFIG_FILE"

    print_success "Sync completed for Ralph '$name'!"
    echo ""
    echo "Branch '$branch' is now up to date with '$target_branch'."
    echo ""
    echo "Next steps:"
    if [[ "$status" != "running" ]] || [[ "$was_running" == false ]]; then
        echo "  1. Start Ralph: hive.sh start $name"
    fi
    echo "  2. Check status: hive.sh status"
}

# ============================================================================
# PR Command
# ============================================================================

show_pr_usage() {
    cat << EOF
${CYAN}hive.sh pr${NC} - Create Pull Request from Ralph's branch

${YELLOW}Usage:${NC}
  hive.sh pr <name> [--draft] [--to <target>]

${YELLOW}Arguments:${NC}
  name        Name of the Ralph whose branch to create a PR from

${YELLOW}Options:${NC}
  --draft       Create a draft PR (work-in-progress)
  --to <target> Override target branch (default: Ralph's targetBranch)
  --help, -h    Show this help message

${YELLOW}Behavior:${NC}
  - Commits any uncommitted changes in worktree
  - Pushes branch to origin
  - Creates PR using GitHub CLI (gh)
  - Generates PR title and body from Ralph info
  - Stores PR number and URL in config.json
  - Updates Ralph status to 'pr_created'
  - Warns if other Ralphs are active on the same branch

${YELLOW}Examples:${NC}
  hive.sh pr auth-feature              # Create PR to default target
  hive.sh pr auth-feature --draft      # Create a draft PR
  hive.sh pr fix-bug --to develop      # Create PR targeting develop branch
EOF
}

cmd_pr() {
    check_git_repo
    check_dependencies

    local name=""
    local draft=false
    local target_override=""

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --draft)
                draft=true
                shift
                ;;
            --to)
                if [[ -z "${2:-}" ]]; then
                    print_error "--to requires a branch name"
                    exit 1
                fi
                target_override="$2"
                shift 2
                ;;
            --help|-h)
                show_pr_usage
                exit 0
                ;;
            -*)
                print_error "Unknown option: $1"
                show_pr_usage
                exit 1
                ;;
            *)
                if [[ -z "$name" ]]; then
                    name="$1"
                else
                    print_error "Unexpected argument: $1"
                    show_pr_usage
                    exit 1
                fi
                shift
                ;;
        esac
    done

    # Validate arguments
    if [[ -z "$name" ]]; then
        print_error "Ralph name is required"
        show_pr_usage
        exit 1
    fi

    # Ensure hive is initialized
    if [[ ! -f "$CONFIG_FILE" ]]; then
        print_error "Hive not initialized. Run 'hive.sh init' first."
        exit 1
    fi

    # Check if gh CLI is available
    if ! command -v gh &> /dev/null; then
        print_error "GitHub CLI (gh) is not installed or not in PATH."
        print_info "Please install it: https://cli.github.com/"
        exit 1
    fi

    # Check if gh is authenticated
    if ! gh auth status &> /dev/null; then
        print_error "GitHub CLI is not authenticated."
        print_info "Run 'gh auth login' to authenticate."
        exit 1
    fi

    # Check if ralph exists
    local ralph_entry
    ralph_entry=$(jq --arg name "$name" '.ralphs[] | select(.name == $name)' "$CONFIG_FILE" 2>/dev/null || true)
    if [[ -z "$ralph_entry" ]]; then
        print_error "Ralph '$name' does not exist."
        exit 1
    fi

    # Get ralph info
    local worktree_path branch target_branch status pr_json
    worktree_path=$(echo "$ralph_entry" | jq -r '.worktreePath')
    branch=$(echo "$ralph_entry" | jq -r '.branch')
    target_branch=$(echo "$ralph_entry" | jq -r '.targetBranch')
    status=$(echo "$ralph_entry" | jq -r '.status')
    pr_json=$(echo "$ralph_entry" | jq '.pr')

    # Use target override if provided
    if [[ -n "$target_override" ]]; then
        target_branch="$target_override"
    fi

    # Check if PR already exists
    if [[ "$pr_json" != "null" ]] && [[ -n "$pr_json" ]]; then
        local existing_pr_number existing_pr_url
        existing_pr_number=$(echo "$pr_json" | jq -r '.number // empty')
        existing_pr_url=$(echo "$pr_json" | jq -r '.url // empty')

        if [[ -n "$existing_pr_number" ]]; then
            print_warning "A PR already exists for Ralph '$name'."
            print_info "PR #$existing_pr_number: $existing_pr_url"
            print_info "To view the PR status, run: hive.sh prs"
            exit 0
        fi
    fi

    # Verify worktree exists
    if [[ ! -d "$worktree_path" ]]; then
        print_error "Worktree directory '$worktree_path' does not exist."
        print_info "The worktree may have been removed."
        exit 1
    fi

    # Check if other Ralphs are active on the same branch
    local branch_ralphs
    branch_ralphs=$(jq --arg branch "$branch" '.branches[$branch].ralphs // []' "$CONFIG_FILE")
    local ralph_count
    ralph_count=$(echo "$branch_ralphs" | jq 'length')

    if [[ "$ralph_count" -gt 1 ]]; then
        # Count how many OTHER Ralphs are actively running on this branch
        local running_others=0
        local running_names=""

        while read -r ralph_name; do
            if [[ "$ralph_name" != "$name" ]]; then
                local ralph_status ralph_pid
                ralph_status=$(jq -r --arg n "$ralph_name" '.ralphs[] | select(.name == $n) | .status' "$CONFIG_FILE")
                ralph_pid=$(jq -r --arg n "$ralph_name" '.ralphs[] | select(.name == $n) | .pid // empty' "$CONFIG_FILE")

                # Verify if process is actually running
                if [[ "$ralph_status" == "running" ]] && [[ -n "$ralph_pid" ]] && kill -0 "$ralph_pid" 2>/dev/null; then
                    running_others=$((running_others + 1))
                    running_names="$running_names $ralph_name"
                fi
            fi
        done < <(echo "$branch_ralphs" | jq -r '.[]')

        print_warning "Multiple Ralphs ($ralph_count) are attached to branch '$branch':"
        while read -r ralph_name; do
            local ralph_status ralph_pid is_active=""
            ralph_status=$(jq -r --arg n "$ralph_name" '.ralphs[] | select(.name == $n) | .status' "$CONFIG_FILE")
            ralph_pid=$(jq -r --arg n "$ralph_name" '.ralphs[] | select(.name == $n) | .pid // empty' "$CONFIG_FILE")

            # Check if process is actually alive
            if [[ "$ralph_status" == "running" ]] && [[ -n "$ralph_pid" ]] && kill -0 "$ralph_pid" 2>/dev/null; then
                is_active=" ${RED}[ACTIVE]${NC}"
            fi
            echo -e "  - $ralph_name (status: $ralph_status)$is_active"
        done < <(echo "$branch_ralphs" | jq -r '.[]')
        echo ""

        if [[ $running_others -gt 0 ]]; then
            print_error "WARNING: $running_others other Ralph(s) are ACTIVELY RUNNING on this branch!"
            print_warning "Creating a PR while other Ralphs are running may cause conflicts."
            print_info "Consider stopping them first with: hive.sh stop <name>"
            echo ""
            read -p "Continue anyway? This is risky! [y/N] " -n 1 -r
        else
            print_warning "Consider cleaning up inactive Ralphs before creating a PR."
            read -p "Continue anyway? [y/N] " -n 1 -r
        fi
        echo ""
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            print_info "Aborted."
            exit 0
        fi
    fi

    print_info "Creating Pull Request for Ralph '$name'..."
    print_info "Branch: $branch → $target_branch"

    # Change to worktree directory
    cd "$worktree_path"

    # Check for uncommitted changes and commit them
    local has_changes=false
    if ! git diff --quiet 2>/dev/null || ! git diff --cached --quiet 2>/dev/null; then
        has_changes=true
        print_info "Found uncommitted changes. Committing..."

        # Stage all changes
        git add -A

        # Create a commit
        local commit_msg="WIP: Changes from Ralph '$name'

Automatic commit by hive.sh before PR creation.

🤖 Generated by Hive"

        if git commit -m "$commit_msg" 2>&1; then
            print_success "Changes committed"
        else
            print_warning "No changes to commit (may be empty)"
        fi
    fi

    # Check for untracked files
    if [[ -n "$(git ls-files --others --exclude-standard)" ]]; then
        print_info "Found untracked files. Adding..."
        git add -A
        if git diff --cached --quiet 2>/dev/null; then
            print_info "No new files to commit"
        else
            local commit_msg="Add new files from Ralph '$name'

🤖 Generated by Hive"
            git commit -m "$commit_msg" 2>&1 || true
            print_success "New files committed"
        fi
    fi

    # Push branch to origin
    print_info "Pushing branch '$branch' to origin..."
    if ! git push -u origin "$branch" 2>&1; then
        print_error "Failed to push branch to origin."
        exit 1
    fi
    print_success "Branch pushed to origin"

    # Generate PR title
    # Convert branch name to a readable title
    # e.g., "feature/auth-system" -> "Feature: Auth System"
    local pr_title
    pr_title=$(echo "$branch" | sed 's|^feature/||; s|^fix/||; s|^bugfix/||; s|^hotfix/||' | sed 's/-/ /g; s/_/ /g' | awk '{for(i=1;i<=NF;i++) $i=toupper(substr($i,1,1)) tolower(substr($i,2))}1')
    pr_title="[$name] $pr_title"

    # Generate PR body with summary of work
    local commit_count
    commit_count=$(git rev-list --count "origin/$target_branch..$branch" 2>/dev/null || echo "0")

    local commit_summary=""
    if [[ "$commit_count" -gt 0 ]]; then
        commit_summary=$(git log --oneline "origin/$target_branch..$branch" 2>/dev/null | head -10)
    fi

    local pr_body
    pr_body="## Summary

Pull request created by Ralph \`$name\` using Hive.

**Branch:** \`$branch\`
**Target:** \`$target_branch\`
**Commits:** $commit_count

## Changes

\`\`\`
$commit_summary
\`\`\`

## Files Changed

$(git diff --stat "origin/$target_branch..$branch" 2>/dev/null | tail -20 || echo "Unable to determine")

---
🤖 Generated by Hive"

    # Build gh pr create command
    local gh_args=("pr" "create")
    gh_args+=("--title" "$pr_title")
    gh_args+=("--body" "$pr_body")
    gh_args+=("--base" "$target_branch")
    gh_args+=("--head" "$branch")

    if [[ "$draft" == true ]]; then
        gh_args+=("--draft")
        print_info "Creating draft PR..."
    else
        print_info "Creating PR..."
    fi

    # Create the PR
    local pr_output
    pr_output=$(gh "${gh_args[@]}" 2>&1)
    local pr_result=$?

    if [[ $pr_result -ne 0 ]]; then
        # Check if PR already exists on GitHub
        if echo "$pr_output" | grep -q "already exists"; then
            print_warning "A PR already exists for this branch on GitHub."
            # Try to get the existing PR info
            local existing_pr_info
            existing_pr_info=$(gh pr view "$branch" --json number,url 2>/dev/null || echo "")
            if [[ -n "$existing_pr_info" ]]; then
                local existing_number existing_url
                existing_number=$(echo "$existing_pr_info" | jq -r '.number')
                existing_url=$(echo "$existing_pr_info" | jq -r '.url')
                print_info "Existing PR: #$existing_number"
                print_info "URL: $existing_url"

                # Update config with existing PR info
                local timestamp=$(get_timestamp)
                local pr_info=$(jq -n \
                    --argjson number "$existing_number" \
                    --arg url "$existing_url" \
                    --arg state "open" \
                    --arg createdAt "$timestamp" \
                    '{number: $number, url: $url, state: $state, createdAt: $createdAt}')

                local tmp_config=$(mktemp)
                jq --arg name "$name" \
                   --argjson pr "$pr_info" \
                   --arg status "pr_created" \
                   --arg timestamp "$timestamp" \
                   '(.ralphs[] | select(.name == $name)) |= . + {pr: $pr, status: $status} | .lastUpdate = $timestamp' \
                   "$CONFIG_FILE" > "$tmp_config" && mv "$tmp_config" "$CONFIG_FILE"

                print_success "Config updated with existing PR info."
            fi
            exit 0
        fi

        print_error "Failed to create PR: $pr_output"
        exit 1
    fi

    # Extract PR URL from output (last line usually contains the URL)
    local pr_url
    pr_url=$(echo "$pr_output" | grep -E 'https://github.com' | tail -1)

    # Extract PR number from URL
    local pr_number
    pr_number=$(echo "$pr_url" | grep -oE '[0-9]+$')

    if [[ -z "$pr_number" ]]; then
        # Try to get PR info via gh
        local pr_info
        pr_info=$(gh pr view "$branch" --json number,url 2>/dev/null || echo "")
        if [[ -n "$pr_info" ]]; then
            pr_number=$(echo "$pr_info" | jq -r '.number')
            pr_url=$(echo "$pr_info" | jq -r '.url')
        fi
    fi

    print_success "Pull Request created successfully!"
    echo ""
    if [[ -n "$pr_number" ]]; then
        echo "PR #$pr_number: $pr_url"
    else
        echo "PR URL: $pr_url"
    fi

    # Update config.json with PR info
    cd - > /dev/null
    local timestamp=$(get_timestamp)

    local pr_info
    if [[ -n "$pr_number" ]]; then
        pr_info=$(jq -n \
            --argjson number "$pr_number" \
            --arg url "$pr_url" \
            --arg state "open" \
            --arg createdAt "$timestamp" \
            --argjson draft "$draft" \
            '{number: $number, url: $url, state: $state, draft: $draft, createdAt: $createdAt}')
    else
        pr_info=$(jq -n \
            --arg url "$pr_url" \
            --arg state "open" \
            --arg createdAt "$timestamp" \
            --argjson draft "$draft" \
            '{number: null, url: $url, state: $state, draft: $draft, createdAt: $createdAt}')
    fi

    local tmp_config=$(mktemp)
    jq --arg name "$name" \
       --argjson pr "$pr_info" \
       --arg status "pr_created" \
       --arg timestamp "$timestamp" \
       '(.ralphs[] | select(.name == $name)) |= . + {pr: $pr, status: $status} | .lastUpdate = $timestamp' \
       "$CONFIG_FILE" > "$tmp_config" && mv "$tmp_config" "$CONFIG_FILE"

    print_success "Config updated with PR info."

    echo ""
    echo "Next steps:"
    echo "  1. View PR status: hive.sh prs"
    echo "  2. After PR merge: hive.sh cleanup $name"
    if [[ "$draft" == true ]]; then
        echo "  3. When ready, mark PR as ready: gh pr ready $pr_number"
    fi
}

# ============================================================================
# PRs Command
# ============================================================================

show_prs_usage() {
    cat << EOF
${CYAN}hive.sh prs${NC} - List all PRs created by Hive

${YELLOW}Usage:${NC}
  hive.sh prs

${YELLOW}Options:${NC}
  --help, -h  Show this help message

${YELLOW}Output includes:${NC}
  - Ralph name and branch
  - PR number and URL
  - PR state (open/merged/closed)
  - CI check status (passing/failing/pending)
  - Review status (approved/changes requested/pending)

${YELLOW}Status colors:${NC}
  ${GREEN}●${NC} open/passing/approved   - Good state
  ${YELLOW}●${NC} pending                 - Waiting
  ${RED}●${NC} closed/failing/changes  - Needs attention
  ${CYAN}●${NC} merged                  - Completed
EOF
}

cmd_prs() {
    check_git_repo
    check_dependencies

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --help|-h)
                show_prs_usage
                exit 0
                ;;
            -*)
                print_error "Unknown option: $1"
                show_prs_usage
                exit 1
                ;;
            *)
                print_error "Unexpected argument: $1"
                show_prs_usage
                exit 1
                ;;
        esac
    done

    # Ensure hive is initialized
    if [[ ! -f "$CONFIG_FILE" ]]; then
        print_error "Hive not initialized. Run 'hive.sh init' first."
        exit 1
    fi

    # Check if gh CLI is available
    local gh_available=true
    if ! command -v gh &> /dev/null; then
        print_warning "GitHub CLI (gh) not installed. Showing cached PR info only."
        gh_available=false
    elif ! gh auth status &> /dev/null 2>&1; then
        print_warning "GitHub CLI not authenticated. Showing cached PR info only."
        gh_available=false
    fi

    # Get all ralphs with PRs
    local ralphs_with_prs
    ralphs_with_prs=$(jq '[.ralphs[] | select(.pr != null)]' "$CONFIG_FILE")
    local pr_count
    pr_count=$(echo "$ralphs_with_prs" | jq 'length')

    if [[ "$pr_count" -eq 0 ]]; then
        echo ""
        print_info "No Pull Requests have been created yet."
        echo ""
        echo "Create a PR with:"
        echo "  hive.sh pr <ralph-name>"
        echo ""
        exit 0
    fi

    echo ""
    echo -e "${CYAN}═══════════════════════════════════════════════════════════════════${NC}"
    echo -e "${CYAN}                         HIVE PULL REQUESTS                         ${NC}"
    echo -e "${CYAN}═══════════════════════════════════════════════════════════════════${NC}"
    echo ""

    # Track summary counts
    local open_count=0
    local merged_count=0
    local closed_count=0
    local checks_passing=0
    local checks_failing=0
    local checks_pending=0

    # Iterate through all Ralphs with PRs
    local i=0
    while [[ $i -lt $pr_count ]]; do
        local ralph_json
        ralph_json=$(echo "$ralphs_with_prs" | jq --argjson i "$i" '.[$i]')

        # Extract ralph fields
        local name branch pr_json target_branch status
        name=$(echo "$ralph_json" | jq -r '.name')
        branch=$(echo "$ralph_json" | jq -r '.branch')
        target_branch=$(echo "$ralph_json" | jq -r '.targetBranch')
        status=$(echo "$ralph_json" | jq -r '.status')
        pr_json=$(echo "$ralph_json" | jq '.pr')

        # Extract cached PR info
        local pr_number pr_url pr_state pr_draft pr_created_at
        pr_number=$(echo "$pr_json" | jq -r '.number // empty')
        pr_url=$(echo "$pr_json" | jq -r '.url // empty')
        pr_state=$(echo "$pr_json" | jq -r '.state // "unknown"')
        pr_draft=$(echo "$pr_json" | jq -r '.draft // false')
        pr_created_at=$(echo "$pr_json" | jq -r '.createdAt // empty')

        # Variables for live GitHub data
        local gh_pr_state=""
        local gh_check_status=""
        local gh_review_status=""
        local gh_mergeable=""
        local gh_additions=""
        local gh_deletions=""
        local gh_changed_files=""

        # Fetch live PR status from GitHub if available
        if [[ "$gh_available" == true ]] && [[ -n "$pr_number" ]]; then
            # Get comprehensive PR info in a single call
            local gh_info
            gh_info=$(gh pr view "$pr_number" --json state,statusCheckRollup,reviewDecision,mergeable,additions,deletions,changedFiles,isDraft 2>/dev/null || echo "")

            if [[ -n "$gh_info" ]]; then
                gh_pr_state=$(echo "$gh_info" | jq -r '.state // empty')
                pr_draft=$(echo "$gh_info" | jq -r '.isDraft // false')
                gh_mergeable=$(echo "$gh_info" | jq -r '.mergeable // empty')
                gh_additions=$(echo "$gh_info" | jq -r '.additions // empty')
                gh_deletions=$(echo "$gh_info" | jq -r '.deletions // empty')
                gh_changed_files=$(echo "$gh_info" | jq -r '.changedFiles // empty')

                # Get overall check status (first check's conclusion or status)
                gh_check_status=$(echo "$gh_info" | jq -r '
                    if (.statusCheckRollup | length) > 0 then
                        if (.statusCheckRollup | map(select(.conclusion == "FAILURE" or .conclusion == "failure")) | length) > 0 then "FAILURE"
                        elif (.statusCheckRollup | map(select(.conclusion == "SUCCESS" or .conclusion == "success")) | length) == (.statusCheckRollup | length) then "SUCCESS"
                        elif (.statusCheckRollup | map(select(.status == "IN_PROGRESS" or .status == "in_progress" or .status == "PENDING" or .status == "pending" or .status == "QUEUED" or .status == "queued")) | length) > 0 then "PENDING"
                        else "PENDING"
                        end
                    else "NO_CHECKS"
                    end
                ' 2>/dev/null || echo "")

                # Get review decision
                gh_review_status=$(echo "$gh_info" | jq -r '.reviewDecision // empty')

                # Update state from GitHub
                if [[ -n "$gh_pr_state" ]]; then
                    pr_state="$gh_pr_state"
                fi
            fi
        fi

        # Normalize state values to uppercase for comparison
        local pr_state_upper
        pr_state_upper=$(to_upper "$pr_state")
        local gh_check_status_upper
        gh_check_status_upper=$(to_upper "$gh_check_status")

        # Update summary counts based on state
        case "$pr_state_upper" in
            OPEN)
                open_count=$((open_count + 1))
                ;;
            MERGED)
                merged_count=$((merged_count + 1))
                ;;
            CLOSED)
                closed_count=$((closed_count + 1))
                ;;
        esac

        # Update check counts
        case "$gh_check_status_upper" in
            SUCCESS)
                checks_passing=$((checks_passing + 1))
                ;;
            FAILURE)
                checks_failing=$((checks_failing + 1))
                ;;
            PENDING|IN_PROGRESS|QUEUED)
                checks_pending=$((checks_pending + 1))
                ;;
        esac

        # Determine PR state color and symbol
        local state_color state_symbol
        case "$pr_state_upper" in
            OPEN)
                state_color="${GREEN}"
                state_symbol="●"
                ;;
            MERGED)
                state_color="${CYAN}"
                state_symbol="✓"
                ;;
            CLOSED)
                state_color="${RED}"
                state_symbol="✗"
                ;;
            *)
                state_color="${NC}"
                state_symbol="?"
                ;;
        esac

        # Print Ralph/PR header
        echo -e "${state_color}${state_symbol}${NC} ${CYAN}${name}${NC} → ${branch}"
        echo -e "  PR:       #${pr_number:-unknown} ${state_color}${pr_state}${NC}$(if [[ "$pr_draft" == "true" ]]; then echo -e " ${YELLOW}(draft)${NC}"; fi)"
        echo -e "  Target:   ${target_branch}"

        # Show URL
        if [[ -n "$pr_url" ]]; then
            echo -e "  URL:      ${pr_url}"
        fi

        # Show CI check status
        if [[ -n "$gh_check_status" ]] && [[ "$gh_check_status" != "NO_CHECKS" ]]; then
            local check_color check_symbol
            case "$gh_check_status_upper" in
                SUCCESS)
                    check_color="${GREEN}"
                    check_symbol="✓"
                    ;;
                FAILURE)
                    check_color="${RED}"
                    check_symbol="✗"
                    ;;
                PENDING|IN_PROGRESS|QUEUED)
                    check_color="${YELLOW}"
                    check_symbol="○"
                    ;;
                *)
                    check_color="${NC}"
                    check_symbol="?"
                    ;;
            esac
            echo -e "  CI:       ${check_color}${check_symbol} ${gh_check_status}${NC}"
        elif [[ "$gh_check_status" == "NO_CHECKS" ]]; then
            echo -e "  CI:       ${YELLOW}No checks configured${NC}"
        fi

        # Show review status
        if [[ -n "$gh_review_status" ]] && [[ "$gh_review_status" != "null" ]]; then
            local review_color review_symbol
            local gh_review_status_upper
            gh_review_status_upper=$(to_upper "$gh_review_status")
            case "$gh_review_status_upper" in
                APPROVED)
                    review_color="${GREEN}"
                    review_symbol="✓"
                    ;;
                CHANGES_REQUESTED)
                    review_color="${RED}"
                    review_symbol="✗"
                    ;;
                REVIEW_REQUIRED)
                    review_color="${YELLOW}"
                    review_symbol="○"
                    ;;
                *)
                    review_color="${YELLOW}"
                    review_symbol="○"
                    ;;
            esac
            echo -e "  Review:   ${review_color}${review_symbol} ${gh_review_status}${NC}"
        fi

        # Show mergeable status for open PRs
        if [[ "$pr_state_upper" == "OPEN" ]] && [[ -n "$gh_mergeable" ]] && [[ "$gh_mergeable" != "null" ]]; then
            local merge_color
            local gh_mergeable_upper
            gh_mergeable_upper=$(to_upper "$gh_mergeable")
            case "$gh_mergeable_upper" in
                MERGEABLE)
                    merge_color="${GREEN}"
                    ;;
                CONFLICTING)
                    merge_color="${RED}"
                    ;;
                *)
                    merge_color="${YELLOW}"
                    ;;
            esac
            echo -e "  Merge:    ${merge_color}${gh_mergeable}${NC}"
        fi

        # Show stats if available
        if [[ -n "$gh_additions" ]] && [[ -n "$gh_deletions" ]] && [[ -n "$gh_changed_files" ]]; then
            echo -e "  Changes:  ${GREEN}+${gh_additions}${NC} ${RED}-${gh_deletions}${NC} in ${gh_changed_files} file(s)"
        fi

        # Show created timestamp
        if [[ -n "$pr_created_at" ]]; then
            echo -e "  Created:  ${pr_created_at}"
        fi

        echo ""
        i=$((i + 1))
    done

    # Show summary
    echo -e "${CYAN}───────────────────────────────────────────────────────────────────${NC}"
    echo -e "  Total PRs: ${pr_count}"
    echo -e "  State:   ${GREEN}Open: ${open_count}${NC} | ${CYAN}Merged: ${merged_count}${NC} | ${RED}Closed: ${closed_count}${NC}"

    if [[ "$gh_available" == true ]]; then
        local checks_total=$((checks_passing + checks_failing + checks_pending))
        if [[ $checks_total -gt 0 ]]; then
            echo -e "  CI:      ${GREEN}Passing: ${checks_passing}${NC} | ${RED}Failing: ${checks_failing}${NC} | ${YELLOW}Pending: ${checks_pending}${NC}"
        fi
    fi

    echo ""
}

# ============================================================================
# Cleanup Command
# ============================================================================

show_cleanup_usage() {
    cat << EOF
${CYAN}hive.sh cleanup${NC} - Remove worktree after PR merge

${YELLOW}Usage:${NC}
  hive.sh cleanup <name>

${YELLOW}Arguments:${NC}
  name        Name of the Ralph to clean up

${YELLOW}Options:${NC}
  --force     Skip PR merge check and force cleanup
  --help, -h  Show this help message

${YELLOW}Behavior:${NC}
  - Checks if PR is merged (warns and prompts if not)
  - Stops Ralph if still running
  - Removes git worktree
  - Deletes local branch
  - Deletes remote branch (if PR was merged)
  - Removes Ralph entry from config.json
  - Removes branch entry from config.json (if no other Ralphs use it)

${YELLOW}Examples:${NC}
  hive.sh cleanup auth-feature       # Clean up after PR merge
  hive.sh cleanup fix-bug --force    # Force cleanup without PR check
EOF
}

cmd_cleanup() {
    check_git_repo
    check_dependencies

    local name=""
    local force=false

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --force)
                force=true
                shift
                ;;
            --help|-h)
                show_cleanup_usage
                exit 0
                ;;
            -*)
                print_error "Unknown option: $1"
                show_cleanup_usage
                exit 1
                ;;
            *)
                if [[ -z "$name" ]]; then
                    name="$1"
                else
                    print_error "Unexpected argument: $1"
                    show_cleanup_usage
                    exit 1
                fi
                shift
                ;;
        esac
    done

    # Validate arguments
    if [[ -z "$name" ]]; then
        print_error "Ralph name is required"
        show_cleanup_usage
        exit 1
    fi

    # Ensure hive is initialized
    if [[ ! -f "$CONFIG_FILE" ]]; then
        print_error "Hive not initialized. Run 'hive.sh init' first."
        exit 1
    fi

    # Check if ralph exists
    local ralph_entry
    ralph_entry=$(jq --arg name "$name" '.ralphs[] | select(.name == $name)' "$CONFIG_FILE" 2>/dev/null || true)
    if [[ -z "$ralph_entry" ]]; then
        print_error "Ralph '$name' does not exist."
        exit 1
    fi

    # Get ralph info
    local worktree_path branch status pid pr_json branch_mode
    worktree_path=$(echo "$ralph_entry" | jq -r '.worktreePath')
    branch=$(echo "$ralph_entry" | jq -r '.branch')
    status=$(echo "$ralph_entry" | jq -r '.status')
    pid=$(echo "$ralph_entry" | jq -r '.pid // empty')
    pr_json=$(echo "$ralph_entry" | jq '.pr')
    branch_mode=$(echo "$ralph_entry" | jq -r '.branchMode')

    print_info "Cleaning up Ralph '$name'..."

    # Check PR status
    local pr_merged=false
    local pr_number=""

    if [[ "$pr_json" != "null" ]] && [[ -n "$pr_json" ]]; then
        pr_number=$(echo "$pr_json" | jq -r '.number // empty')

        if [[ -n "$pr_number" ]] && command -v gh &> /dev/null; then
            # Get current PR state from GitHub
            local gh_pr_state
            gh_pr_state=$(gh pr view "$pr_number" --json state -q '.state' 2>/dev/null || echo "")

            if [[ -n "$gh_pr_state" ]]; then
                local gh_pr_state_upper
                gh_pr_state_upper=$(to_upper "$gh_pr_state")

                if [[ "$gh_pr_state_upper" == "MERGED" ]]; then
                    pr_merged=true
                    print_success "PR #$pr_number is merged."
                elif [[ "$gh_pr_state_upper" == "CLOSED" ]]; then
                    print_warning "PR #$pr_number is closed but not merged."
                elif [[ "$gh_pr_state_upper" == "OPEN" ]]; then
                    print_warning "PR #$pr_number is still open (not merged)."
                fi
            fi
        fi
    else
        print_warning "No PR has been created for Ralph '$name'."
    fi

    # Warn and prompt if PR is not merged (unless --force)
    if [[ "$pr_merged" == false ]] && [[ "$force" == false ]]; then
        echo ""
        print_warning "The PR has not been merged. Are you sure you want to clean up?"
        print_warning "This will remove the worktree and delete the branch."
        echo ""
        read -p "Continue anyway? [y/N] " -n 1 -r
        echo ""
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            print_info "Aborted."
            exit 0
        fi
    fi

    # Stop Ralph if running
    if [[ "$status" == "running" ]] && [[ -n "$pid" ]]; then
        if kill -0 "$pid" 2>/dev/null; then
            print_info "Stopping Ralph process (PID: $pid)..."
            kill -TERM "$pid" 2>/dev/null || true

            # Wait up to 5 seconds for graceful shutdown
            local wait_count=0
            while kill -0 "$pid" 2>/dev/null && [[ $wait_count -lt 5 ]]; do
                sleep 1
                wait_count=$((wait_count + 1))
            done

            # Force kill if still running
            if kill -0 "$pid" 2>/dev/null; then
                print_warning "Process did not stop gracefully. Force killing..."
                kill -KILL "$pid" 2>/dev/null || true
            fi

            print_success "Ralph process stopped."
        fi
    fi

    # Remove git worktree
    if [[ -d "$worktree_path" ]]; then
        print_info "Removing git worktree at '$worktree_path'..."

        # First, try to remove the worktree properly
        if git worktree remove "$worktree_path" --force 2>/dev/null; then
            print_success "Worktree removed."
        else
            # If that fails, try to remove the directory and prune
            print_warning "Standard worktree removal failed. Attempting force removal..."
            rm -rf "$worktree_path" 2>/dev/null || true
            git worktree prune 2>/dev/null || true
            print_success "Worktree directory removed and pruned."
        fi
    else
        print_info "Worktree directory does not exist (already removed?)."
        git worktree prune 2>/dev/null || true
    fi

    # Delete local branch
    print_info "Deleting local branch '$branch'..."
    if git show-ref --verify --quiet "refs/heads/$branch" 2>/dev/null; then
        if git branch -D "$branch" 2>/dev/null; then
            print_success "Local branch deleted."
        else
            print_warning "Could not delete local branch (may be checked out elsewhere)."
        fi
    else
        print_info "Local branch does not exist (already deleted?)."
    fi

    # Delete remote branch if PR was merged
    if [[ "$pr_merged" == true ]]; then
        print_info "Deleting remote branch 'origin/$branch'..."
        if git push origin --delete "$branch" 2>/dev/null; then
            print_success "Remote branch deleted."
        else
            print_info "Remote branch may already be deleted or doesn't exist."
        fi
    else
        print_info "Skipping remote branch deletion (PR not merged)."
        print_info "To delete remote branch manually: git push origin --delete $branch"
    fi

    # Remove Ralph entry from config.json
    print_info "Updating config.json..."
    local timestamp=$(get_timestamp)
    local tmp_config=$(mktemp)

    jq --arg name "$name" \
       --arg timestamp "$timestamp" \
       '.ralphs = [.ralphs[] | select(.name != $name)] | .lastUpdate = $timestamp' \
       "$CONFIG_FILE" > "$tmp_config" && mv "$tmp_config" "$CONFIG_FILE"

    print_success "Ralph '$name' removed from config."

    # Remove Ralph from branch entry and clean up if no more Ralphs use this branch
    local branch_ralphs_count
    branch_ralphs_count=$(jq --arg branch "$branch" '.branches[$branch].ralphs | length // 0' "$CONFIG_FILE" 2>/dev/null || echo "0")

    if [[ "$branch_ralphs_count" -gt 0 ]]; then
        # Remove this Ralph from the branch's ralphs array
        tmp_config=$(mktemp)
        jq --arg branch "$branch" \
           --arg name "$name" \
           --arg timestamp "$timestamp" \
           '.branches[$branch].ralphs = [.branches[$branch].ralphs[] | select(. != $name)] | .lastUpdate = $timestamp' \
           "$CONFIG_FILE" > "$tmp_config" && mv "$tmp_config" "$CONFIG_FILE"

        # Check if any Ralphs remain on this branch
        local remaining_ralphs
        remaining_ralphs=$(jq --arg branch "$branch" '.branches[$branch].ralphs | length // 0' "$CONFIG_FILE" 2>/dev/null || echo "0")

        if [[ "$remaining_ralphs" -eq 0 ]]; then
            # Remove the branch entry entirely
            tmp_config=$(mktemp)
            jq --arg branch "$branch" \
               --arg timestamp "$timestamp" \
               'del(.branches[$branch]) | .lastUpdate = $timestamp' \
               "$CONFIG_FILE" > "$tmp_config" && mv "$tmp_config" "$CONFIG_FILE"
            print_success "Branch '$branch' removed from config (no more Ralphs)."
        else
            print_info "Branch '$branch' still has $remaining_ralphs other Ralph(s) attached."
        fi
    fi

    print_success "Cleanup completed for Ralph '$name'!"
    echo ""
    echo "Summary:"
    echo "  - Worktree removed: $worktree_path"
    echo "  - Local branch deleted: $branch"
    if [[ "$pr_merged" == true ]]; then
        echo "  - Remote branch deleted: origin/$branch"
    fi
    echo "  - Config entries removed"
    echo ""
    echo "Run 'hive.sh status' to see remaining Ralphs."
}

# ============================================================================
# Clean Command (Abandon without PR)
# ============================================================================

show_clean_usage() {
    cat << EOF
${CYAN}hive.sh clean${NC} - Remove worktree without PR (abandon)

${YELLOW}Usage:${NC}
  hive.sh clean <name>

${YELLOW}Arguments:${NC}
  name        Name of the Ralph to abandon

${YELLOW}Options:${NC}
  --force     Skip confirmation prompts
  --help, -h  Show this help message

${YELLOW}Behavior:${NC}
  - Stops Ralph if still running
  - Removes git worktree forcefully
  - Deletes local and remote branch
  - Removes Ralph entry from config.json
  - Removes branch entry from config.json (if no other Ralphs use it)
  - Prompts for confirmation if commits exist that haven't been merged

${YELLOW}Examples:${NC}
  hive.sh clean abandoned-feature     # Abandon work, confirm if commits exist
  hive.sh clean experiment --force    # Force abandon without prompts
EOF
}

cmd_clean() {
    check_git_repo
    check_dependencies

    local name=""
    local force=false

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --force)
                force=true
                shift
                ;;
            --help|-h)
                show_clean_usage
                exit 0
                ;;
            -*)
                print_error "Unknown option: $1"
                show_clean_usage
                exit 1
                ;;
            *)
                if [[ -z "$name" ]]; then
                    name="$1"
                else
                    print_error "Unexpected argument: $1"
                    show_clean_usage
                    exit 1
                fi
                shift
                ;;
        esac
    done

    # Validate arguments
    if [[ -z "$name" ]]; then
        print_error "Ralph name is required"
        show_clean_usage
        exit 1
    fi

    # Ensure hive is initialized
    if [[ ! -f "$CONFIG_FILE" ]]; then
        print_error "Hive not initialized. Run 'hive.sh init' first."
        exit 1
    fi

    # Check if ralph exists
    local ralph_entry
    ralph_entry=$(jq --arg name "$name" '.ralphs[] | select(.name == $name)' "$CONFIG_FILE" 2>/dev/null || true)
    if [[ -z "$ralph_entry" ]]; then
        print_error "Ralph '$name' does not exist."
        exit 1
    fi

    # Get ralph info
    local worktree_path branch status pid target_branch
    worktree_path=$(echo "$ralph_entry" | jq -r '.worktreePath')
    branch=$(echo "$ralph_entry" | jq -r '.branch')
    status=$(echo "$ralph_entry" | jq -r '.status')
    pid=$(echo "$ralph_entry" | jq -r '.pid // empty')
    target_branch=$(echo "$ralph_entry" | jq -r '.targetBranch // "main"')

    print_info "Abandoning Ralph '$name'..."
    print_info "  Branch: $branch"
    print_info "  Worktree: $worktree_path"

    # Check if there are commits that haven't been merged
    local commit_count=0
    local has_unpushed=false

    if [[ -d "$worktree_path" ]]; then
        # Fetch latest to compare properly
        git fetch origin 2>/dev/null || true

        # Count commits on this branch that aren't on target branch
        if git show-ref --verify --quiet "refs/heads/$branch" 2>/dev/null || \
           git show-ref --verify --quiet "refs/remotes/origin/$branch" 2>/dev/null; then
            commit_count=$(cd "$worktree_path" && git rev-list --count "origin/$target_branch..$branch" 2>/dev/null || echo "0")
        fi

        # Check for unpushed commits (local commits not on remote)
        if git show-ref --verify --quiet "refs/heads/$branch" 2>/dev/null; then
            if git show-ref --verify --quiet "refs/remotes/origin/$branch" 2>/dev/null; then
                local unpushed
                unpushed=$(cd "$worktree_path" && git rev-list --count "origin/$branch..$branch" 2>/dev/null || echo "0")
                if [[ "$unpushed" -gt 0 ]]; then
                    has_unpushed=true
                fi
            else
                # Remote branch doesn't exist, so all local commits are unpushed
                has_unpushed=true
            fi
        fi
    fi

    # Warn and confirm if work exists (unless --force)
    if [[ "$force" == false ]] && [[ "$commit_count" -gt 0 ]]; then
        echo ""
        print_warning "Branch '$branch' has $commit_count commit(s) that will be lost!"
        if [[ "$has_unpushed" == true ]]; then
            print_warning "Some commits have NOT been pushed to remote."
        fi

        # Show the commits that will be lost
        if [[ -d "$worktree_path" ]]; then
            echo ""
            echo -e "${CYAN}Commits that will be abandoned:${NC}"
            (cd "$worktree_path" && git log --oneline "origin/$target_branch..$branch" 2>/dev/null | head -10) || true
            if [[ "$commit_count" -gt 10 ]]; then
                echo "  ... and $((commit_count - 10)) more"
            fi
        fi

        echo ""
        print_warning "This action is IRREVERSIBLE. The work will be permanently lost."
        echo ""
        read -p "Are you sure you want to abandon this work? [y/N] " -n 1 -r
        echo ""
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            print_info "Aborted. Work preserved."
            echo ""
            echo "Alternatives:"
            echo "  - Create a PR first: hive.sh pr $name"
            echo "  - Clean up after PR merge: hive.sh cleanup $name"
            exit 0
        fi
    fi

    # Stop Ralph if running
    if [[ "$status" == "running" ]] && [[ -n "$pid" ]]; then
        if kill -0 "$pid" 2>/dev/null; then
            print_info "Stopping Ralph process (PID: $pid)..."
            kill -TERM "$pid" 2>/dev/null || true

            # Wait up to 5 seconds for graceful shutdown
            local wait_count=0
            while kill -0 "$pid" 2>/dev/null && [[ $wait_count -lt 5 ]]; do
                sleep 1
                wait_count=$((wait_count + 1))
            done

            # Force kill if still running
            if kill -0 "$pid" 2>/dev/null; then
                print_warning "Process did not stop gracefully. Force killing..."
                kill -KILL "$pid" 2>/dev/null || true
            fi

            print_success "Ralph process stopped."
        fi
    fi

    # Remove git worktree
    if [[ -d "$worktree_path" ]]; then
        print_info "Removing git worktree at '$worktree_path'..."

        # First, try to remove the worktree properly
        if git worktree remove "$worktree_path" --force 2>/dev/null; then
            print_success "Worktree removed."
        else
            # If that fails, try to remove the directory and prune
            print_warning "Standard worktree removal failed. Attempting force removal..."
            rm -rf "$worktree_path" 2>/dev/null || true
            git worktree prune 2>/dev/null || true
            print_success "Worktree directory removed and pruned."
        fi
    else
        print_info "Worktree directory does not exist (already removed?)."
        git worktree prune 2>/dev/null || true
    fi

    # Delete local branch
    print_info "Deleting local branch '$branch'..."
    if git show-ref --verify --quiet "refs/heads/$branch" 2>/dev/null; then
        if git branch -D "$branch" 2>/dev/null; then
            print_success "Local branch deleted."
        else
            print_warning "Could not delete local branch (may be checked out elsewhere)."
        fi
    else
        print_info "Local branch does not exist (already deleted?)."
    fi

    # Delete remote branch (always, since we're abandoning)
    print_info "Deleting remote branch 'origin/$branch'..."
    if git push origin --delete "$branch" 2>/dev/null; then
        print_success "Remote branch deleted."
    else
        print_info "Remote branch may not exist or already deleted."
    fi

    # Remove Ralph entry from config.json
    print_info "Updating config.json..."
    local timestamp=$(get_timestamp)
    local tmp_config=$(mktemp)

    jq --arg name "$name" \
       --arg timestamp "$timestamp" \
       '.ralphs = [.ralphs[] | select(.name != $name)] | .lastUpdate = $timestamp' \
       "$CONFIG_FILE" > "$tmp_config" && mv "$tmp_config" "$CONFIG_FILE"

    print_success "Ralph '$name' removed from config."

    # Remove Ralph from branch entry and clean up if no more Ralphs use this branch
    local branch_ralphs_count
    branch_ralphs_count=$(jq --arg branch "$branch" '.branches[$branch].ralphs | length // 0' "$CONFIG_FILE" 2>/dev/null || echo "0")

    if [[ "$branch_ralphs_count" -gt 0 ]]; then
        # Remove this Ralph from the branch's ralphs array
        tmp_config=$(mktemp)
        jq --arg branch "$branch" \
           --arg name "$name" \
           --arg timestamp "$timestamp" \
           '.branches[$branch].ralphs = [.branches[$branch].ralphs[] | select(. != $name)] | .lastUpdate = $timestamp' \
           "$CONFIG_FILE" > "$tmp_config" && mv "$tmp_config" "$CONFIG_FILE"

        # Check if any Ralphs remain on this branch
        local remaining_ralphs
        remaining_ralphs=$(jq --arg branch "$branch" '.branches[$branch].ralphs | length // 0' "$CONFIG_FILE" 2>/dev/null || echo "0")

        if [[ "$remaining_ralphs" -eq 0 ]]; then
            # Remove the branch entry entirely
            tmp_config=$(mktemp)
            jq --arg branch "$branch" \
               --arg timestamp "$timestamp" \
               'del(.branches[$branch]) | .lastUpdate = $timestamp' \
               "$CONFIG_FILE" > "$tmp_config" && mv "$tmp_config" "$CONFIG_FILE"
            print_success "Branch '$branch' removed from config (no more Ralphs)."
        else
            print_info "Branch '$branch' still has $remaining_ralphs other Ralph(s) attached."
        fi
    fi

    print_success "Ralph '$name' has been abandoned!"
    echo ""
    echo "Summary:"
    echo "  - Worktree removed: $worktree_path"
    echo "  - Local branch deleted: $branch"
    echo "  - Remote branch deleted: origin/$branch"
    echo "  - Config entries removed"
    if [[ "$commit_count" -gt 0 ]]; then
        echo "  - $commit_count commit(s) abandoned"
    fi
    echo ""
    echo "Run 'hive.sh status' to see remaining Ralphs."
}

# ============================================================================
# Dashboard Command
# ============================================================================

show_dashboard_usage() {
    cat << EOF
${CYAN}hive.sh dashboard${NC} - Live status dashboard with auto-refresh

${YELLOW}Usage:${NC}
  hive.sh dashboard [options]

${YELLOW}Options:${NC}
  --interval, -i <seconds>  Refresh interval (default: 5)
  --help, -h                Show this help message

${YELLOW}Description:${NC}
  Displays a continuously updating status dashboard that shows:
  - All Ralphs with their current status
  - Branch relationships
  - PR status and CI checks
  - Aggregated progress across all Ralphs

  Press Ctrl+C to exit.

${YELLOW}Examples:${NC}
  hive.sh dashboard           # Refresh every 5 seconds
  hive.sh dashboard -i 10     # Refresh every 10 seconds
EOF
}

# Display dashboard content (called by cmd_dashboard in a loop)
display_dashboard() {
    local show_header="$1"

    # Get all ralphs from config
    local ralph_count
    ralph_count=$(jq '.ralphs | length' "$CONFIG_FILE")

    if [[ "$show_header" == "true" ]]; then
        echo ""
        echo -e "${CYAN}═══════════════════════════════════════════════════════════════════${NC}"
        echo -e "${CYAN}                      HIVE LIVE DASHBOARD                           ${NC}"
        echo -e "${CYAN}═══════════════════════════════════════════════════════════════════${NC}"
        echo -e "  ${BLUE}Refreshing every ${REFRESH_INTERVAL} seconds | Press Ctrl+C to exit${NC}"
        echo -e "  ${BLUE}Last refresh: $(date '+%Y-%m-%d %H:%M:%S')${NC}"
        echo ""
    fi

    if [[ "$ralph_count" -eq 0 ]]; then
        print_info "No Ralphs have been spawned yet."
        echo ""
        echo "Get started with:"
        echo "  hive.sh spawn <name> --create <branch>"
        echo ""
        return 0
    fi

    # Track aggregated stats
    local total_running=0
    local total_stopped=0
    local total_spawned=0
    local total_pr_created=0
    local total_commits=0

    # Iterate through all Ralphs
    local i=0
    while [[ $i -lt $ralph_count ]]; do
        local ralph_json
        ralph_json=$(jq --argjson i "$i" '.ralphs[$i]' "$CONFIG_FILE")

        # Extract ralph fields
        local name branch branch_mode status pid scope worktree_path pr_json target_branch started_at
        name=$(echo "$ralph_json" | jq -r '.name')
        branch=$(echo "$ralph_json" | jq -r '.branch')
        branch_mode=$(echo "$ralph_json" | jq -r '.branchMode')
        status=$(echo "$ralph_json" | jq -r '.status')
        pid=$(echo "$ralph_json" | jq -r '.pid // empty')
        scope=$(echo "$ralph_json" | jq -r '.scope // empty')
        worktree_path=$(echo "$ralph_json" | jq -r '.worktreePath')
        pr_json=$(echo "$ralph_json" | jq '.pr')
        target_branch=$(echo "$ralph_json" | jq -r '.targetBranch')
        started_at=$(echo "$ralph_json" | jq -r '.startedAt // empty')

        # Verify PID liveness and update status if needed
        local actual_status="$status"
        local status_changed=false

        if [[ "$status" == "running" ]] && [[ -n "$pid" ]]; then
            if ! kill -0 "$pid" 2>/dev/null; then
                # Process is dead, update status
                actual_status="stopped"
                status_changed=true

                # Update config.json
                local timestamp=$(get_timestamp)
                local tmp_config=$(mktemp)
                jq --arg name "$name" \
                   --arg status "stopped" \
                   --arg timestamp "$timestamp" \
                   '(.ralphs[] | select(.name == $name)) |= . + {status: $status, pid: null} | .lastUpdate = $timestamp' \
                   "$CONFIG_FILE" > "$tmp_config" && mv "$tmp_config" "$CONFIG_FILE"
            fi
        fi

        # Track stats
        case "$actual_status" in
            running) total_running=$((total_running + 1)) ;;
            stopped) total_stopped=$((total_stopped + 1)) ;;
            spawned) total_spawned=$((total_spawned + 1)) ;;
            pr_created) total_pr_created=$((total_pr_created + 1)) ;;
        esac

        # Count commits on this Ralph's branch
        if [[ -d "$worktree_path" ]]; then
            local branch_commits
            branch_commits=$(cd "$worktree_path" && git rev-list --count "origin/${target_branch}..${branch}" 2>/dev/null || echo "0")
            total_commits=$((total_commits + branch_commits))
        fi

        # Determine status color and symbol
        local status_color status_symbol
        case "$actual_status" in
            running)
                status_color="${GREEN}"
                status_symbol="●"
                ;;
            completed|pr_created)
                status_color="${GREEN}"
                status_symbol="✓"
                ;;
            spawned|stopped)
                status_color="${YELLOW}"
                status_symbol="○"
                ;;
            failed)
                status_color="${RED}"
                status_symbol="✗"
                ;;
            *)
                status_color="${NC}"
                status_symbol="?"
                ;;
        esac

        # Print Ralph in compact format for dashboard
        echo -ne "${status_color}${status_symbol}${NC} ${CYAN}${name}${NC}"
        echo -ne " | ${actual_status}"
        if [[ "$status_changed" == true ]]; then
            echo -ne " ${YELLOW}(died)${NC}"
        fi
        echo -ne " | ${branch}"
        if [[ -n "$scope" ]]; then
            echo -ne " ${YELLOW}[${scope}]${NC}"
        fi

        # Show PR status inline if exists
        if [[ "$pr_json" != "null" ]] && [[ -n "$pr_json" ]]; then
            local pr_number pr_state
            pr_number=$(echo "$pr_json" | jq -r '.number // empty')
            pr_state=$(echo "$pr_json" | jq -r '.state // "unknown"')

            if [[ -n "$pr_number" ]]; then
                # Try to get current PR status from GitHub
                if command -v gh &> /dev/null; then
                    local gh_info
                    gh_info=$(gh pr view "$pr_number" --json state,statusCheckRollup 2>/dev/null || echo "")
                    if [[ -n "$gh_info" ]]; then
                        pr_state=$(echo "$gh_info" | jq -r '.state // "unknown"')
                        local ci_status
                        ci_status=$(echo "$gh_info" | jq -r '
                            if .statusCheckRollup == null or (.statusCheckRollup | length) == 0 then "none"
                            elif any(.statusCheckRollup[]; .conclusion == "FAILURE") then "FAILURE"
                            elif all(.statusCheckRollup[]; .conclusion == "SUCCESS") then "SUCCESS"
                            else "PENDING"
                            end
                        ' 2>/dev/null || echo "unknown")

                        local pr_color ci_color
                        case "$pr_state" in
                            OPEN) pr_color="${GREEN}" ;;
                            MERGED) pr_color="${CYAN}" ;;
                            CLOSED) pr_color="${RED}" ;;
                            *) pr_color="${NC}" ;;
                        esac
                        case "$ci_status" in
                            SUCCESS) ci_color="${GREEN}" ;;
                            FAILURE) ci_color="${RED}" ;;
                            PENDING) ci_color="${YELLOW}" ;;
                            *) ci_color="${NC}" ;;
                        esac

                        echo -ne " | PR#${pr_number} ${pr_color}${pr_state}${NC}"
                        if [[ "$ci_status" != "none" ]]; then
                            echo -ne " CI:${ci_color}${ci_status}${NC}"
                        fi
                    else
                        echo -ne " | PR#${pr_number} ${pr_state}"
                    fi
                else
                    echo -ne " | PR#${pr_number} ${pr_state}"
                fi
            fi
        fi

        # Show last activity for running Ralphs
        if [[ "$actual_status" == "running" ]]; then
            local log_file="$worktree_path/ralph-output.log"
            if [[ -f "$log_file" ]]; then
                local last_line
                last_line=$(grep -v '^#' "$log_file" | grep -v '^---' | grep -v '^$' | tail -1 2>/dev/null || echo "")
                if [[ -n "$last_line" ]]; then
                    # Truncate if too long
                    if [[ ${#last_line} -gt 40 ]]; then
                        last_line="${last_line:0:37}..."
                    fi
                    echo -ne " | ${BLUE}${last_line}${NC}"
                fi
            fi
        fi

        echo ""
        i=$((i + 1))
    done

    # Branch relationships (compact)
    echo ""
    echo -e "${CYAN}───────────────────────────────────────────────────────────────────${NC}"
    echo -e "${CYAN}BRANCHES${NC}"

    local branches
    branches=$(jq -r '.branches | to_entries[] | "\(.key)|\(.value.ralphs | join(","))"' "$CONFIG_FILE" 2>/dev/null || echo "")

    if [[ -n "$branches" ]]; then
        while IFS='|' read -r branch_name ralph_list; do
            local ralph_array
            IFS=',' read -ra ralph_array <<< "$ralph_list"
            local ralph_count_on_branch=${#ralph_array[@]}

            if [[ $ralph_count_on_branch -gt 1 ]]; then
                echo -e "  ${YELLOW}⚠${NC} ${branch_name}: ${ralph_list} (shared)"
            else
                echo -e "  ${GREEN}✓${NC} ${branch_name}: ${ralph_list}"
            fi
        done <<< "$branches"
    else
        echo "  No branches tracked"
    fi

    # Aggregated summary
    echo ""
    echo -e "${CYAN}═══════════════════════════════════════════════════════════════════${NC}"
    echo -e "${CYAN}AGGREGATED PROGRESS${NC}"
    echo -e "  Ralphs:  ${ralph_count} total | ${GREEN}${total_running} running${NC} | ${YELLOW}${total_stopped} idle${NC} | ${GREEN}${total_pr_created} PRs${NC}"
    echo -e "  Commits: ${total_commits} across all branches"

    # Count PRs by state if we have any
    local open_prs=0
    local merged_prs=0
    local closed_prs=0

    local pr_ralphs
    pr_ralphs=$(jq -r '.ralphs[] | select(.pr != null) | .pr.number' "$CONFIG_FILE" 2>/dev/null || echo "")
    if [[ -n "$pr_ralphs" ]]; then
        while read -r pr_num; do
            if [[ -n "$pr_num" ]] && command -v gh &> /dev/null; then
                local state
                state=$(gh pr view "$pr_num" --json state -q '.state' 2>/dev/null || echo "")
                case "$state" in
                    OPEN) open_prs=$((open_prs + 1)) ;;
                    MERGED) merged_prs=$((merged_prs + 1)) ;;
                    CLOSED) closed_prs=$((closed_prs + 1)) ;;
                esac
            fi
        done <<< "$pr_ralphs"

        echo -e "  PRs:     ${GREEN}${open_prs} open${NC} | ${CYAN}${merged_prs} merged${NC} | ${RED}${closed_prs} closed${NC}"
    fi

    echo ""
}

# Global variable for refresh interval (used by display_dashboard)
REFRESH_INTERVAL=5

cmd_dashboard() {
    check_git_repo
    check_dependencies

    local interval=5

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --interval|-i)
                if [[ -z "${2:-}" ]]; then
                    print_error "--interval requires a number of seconds"
                    exit 1
                fi
                if ! [[ "$2" =~ ^[0-9]+$ ]]; then
                    print_error "Interval must be a positive integer"
                    exit 1
                fi
                interval="$2"
                shift 2
                ;;
            --help|-h)
                show_dashboard_usage
                exit 0
                ;;
            -*)
                print_error "Unknown option: $1"
                show_dashboard_usage
                exit 1
                ;;
            *)
                print_error "Unexpected argument: $1"
                show_dashboard_usage
                exit 1
                ;;
        esac
    done

    # Ensure hive is initialized
    if [[ ! -f "$CONFIG_FILE" ]]; then
        print_error "Hive not initialized. Run 'hive.sh init' first."
        exit 1
    fi

    # Set global for display function
    REFRESH_INTERVAL="$interval"

    # Set up signal trap for clean exit
    trap 'echo ""; print_info "Dashboard stopped."; exit 0' INT TERM

    # Main dashboard loop
    while true; do
        # Clear screen
        clear

        # Display dashboard content
        display_dashboard "true"

        # Sleep for refresh interval
        sleep "$interval"
    done
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
  --version, -v     Show version number

${YELLOW}Quick Start:${NC}
  hive.sh init
  hive.sh spawn my-feature --create feature/my-feature
  hive.sh start my-feature "Implement the new feature"
  hive.sh status

${YELLOW}Example Workflows:${NC}

  ${CYAN}1. Single Feature Development${NC}
     # Initialize Hive in your repo
     hive.sh init

     # Create a Ralph for a new feature
     hive.sh spawn auth-feature --create feature/auth --from main

     # Start Ralph with a task
     hive.sh start auth-feature "Implement JWT authentication"

     # Monitor progress
     hive.sh status
     hive.sh logs auth-feature --follow

     # When done, create a PR
     hive.sh pr auth-feature

     # After PR is merged, cleanup
     hive.sh cleanup auth-feature

  ${CYAN}2. Parallel Feature Development${NC}
     # Create multiple Ralphs for different features
     hive.sh spawn frontend --create feature/ui-redesign
     hive.sh spawn backend --create feature/api-v2
     hive.sh spawn tests --create feature/e2e-tests

     # Start all Ralphs
     hive.sh start frontend "Redesign the dashboard UI"
     hive.sh start backend "Implement REST API v2"
     hive.sh start tests "Add end-to-end test coverage"

     # Monitor all with dashboard
     hive.sh dashboard

  ${CYAN}3. Collaborative Work on Same Branch${NC}
     # First Ralph creates the branch
     hive.sh spawn lead --create feature/big-feature

     # Second Ralph attaches with scoped access
     hive.sh spawn helper --attach feature/big-feature --scope "src/utils/*"

     # Start both with different tasks
     hive.sh start lead "Implement main feature logic"
     hive.sh start helper "Create utility functions"

  ${CYAN}4. Continue Existing Work${NC}
     # Attach to an existing remote branch
     hive.sh spawn continue-work --attach feature/existing-branch
     hive.sh start continue-work "Complete the remaining tasks"

Run 'hive.sh <command> --help' for detailed information on a specific command.
EOF
}

show_version() {
    cat << EOF
hive.sh version $VERSION

Multi-Ralph Orchestration via Bash
Manages multiple Claude Code (Ralph) instances in parallel git worktrees.
EOF
}

cmd_help() {
    # Handle --help on the help command itself
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --help|-h)
                cat << EOF
${CYAN}hive.sh help${NC} - Show usage information

${YELLOW}Usage:${NC}
  hive.sh help
  hive.sh help <command>
  hive.sh <command> --help

${YELLOW}Description:${NC}
  Shows usage information for hive.sh or a specific command.
  Running 'hive.sh help <command>' is equivalent to 'hive.sh <command> --help'.
EOF
                exit 0
                ;;
            *)
                # Treat as a command name to get help for
                exec "$0" "$1" --help
                ;;
        esac
    done
    show_usage
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
        start)
            cmd_start "$@"
            ;;
        status)
            cmd_status "$@"
            ;;
        logs)
            cmd_logs "$@"
            ;;
        stop)
            cmd_stop "$@"
            ;;
        sync)
            cmd_sync "$@"
            ;;
        pr)
            cmd_pr "$@"
            ;;
        prs)
            cmd_prs "$@"
            ;;
        cleanup)
            cmd_cleanup "$@"
            ;;
        clean)
            cmd_clean "$@"
            ;;
        dashboard)
            cmd_dashboard "$@"
            ;;
        help|--help|-h)
            cmd_help "$@"
            ;;
        version|--version|-v)
            show_version
            ;;
        "")
            show_usage
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
