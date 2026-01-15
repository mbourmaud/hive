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
        start)
            cmd_start "$@"
            ;;
        status)
            cmd_status "$@"
            ;;
        logs|stop|sync|pr|prs|cleanup|clean|dashboard)
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
