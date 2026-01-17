#!/bin/bash
#
# hive - Drone Orchestration for Claude Code
# Launch autonomous Claude agents (drones) on PRD files
#
# Usage: hive [command] [options]
#

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m'

# Version
VERSION="0.2.0"

# Configuration
HIVE_DIR=".hive"
CONFIG_FILE="$HIVE_DIR/config.json"
PRDS_DIR="$HIVE_DIR/prds"
DRONES_DIR="$HIVE_DIR/drones"

# ============================================================================
# Helper Functions
# ============================================================================

print_info() { echo -e "${BLUE}ℹ${NC} $1"; }
print_success() { echo -e "${GREEN}✓${NC} $1"; }
print_warning() { echo -e "${YELLOW}⚠${NC} $1"; }
print_error() { echo -e "${RED}✗${NC} $1" >&2; }
print_drone() { echo -e "${CYAN}🐝${NC} $1"; }

get_timestamp() { date -u +"%Y-%m-%dT%H:%M:%SZ"; }

check_git_repo() {
    if ! git rev-parse --git-dir > /dev/null 2>&1; then
        print_error "Not a git repository"
        exit 1
    fi
}

check_dependencies() {
    local missing=()
    command -v jq &>/dev/null || missing+=("jq")
    command -v claude &>/dev/null || missing+=("claude")
    command -v git &>/dev/null || missing+=("git")

    if [ ${#missing[@]} -gt 0 ]; then
        print_error "Missing dependencies: ${missing[*]}"
        exit 1
    fi
}

get_project_root() {
    git rev-parse --show-toplevel 2>/dev/null
}

get_project_name() {
    basename "$(get_project_root)"
}

# ============================================================================
# Init Command
# ============================================================================

cmd_init() {
    check_git_repo
    check_dependencies

    print_info "Initializing Hive..."

    mkdir -p "$HIVE_DIR" "$PRDS_DIR" "$DRONES_DIR"

    if [ ! -f "$CONFIG_FILE" ]; then
        cat > "$CONFIG_FILE" << EOF
{
  "version": "$VERSION",
  "project": "$(get_project_name)",
  "created": "$(get_timestamp)"
}
EOF
    fi

    # Add to .gitignore if not already
    if [ -f .gitignore ]; then
        grep -q "^\.hive/$" .gitignore 2>/dev/null || echo ".hive/" >> .gitignore
    else
        echo ".hive/" > .gitignore
    fi

    print_success "Hive initialized in $(get_project_name)"
    print_info "Structure created:"
    print_info "  .hive/"
    print_info "  ├── config.json"
    print_info "  ├── prds/        <- Put your PRD files here"
    print_info "  └── drones/      <- Drone status files"
}

# ============================================================================
# Run Command - Launch a drone on a PRD
# ============================================================================

show_run_usage() {
    cat << EOF
${CYAN}hive run${NC} - Launch a drone on a PRD file

${YELLOW}Usage:${NC}
  hive run --prd <file> [options]

${YELLOW}Required:${NC}
  --prd <file>        PRD JSON file to execute

${YELLOW}Options:${NC}
  --name <name>       Drone name (default: derived from PRD id)
  --base <branch>     Base branch (default: main)
  --iterations <n>    Max iterations (default: 50)
  --model <model>     Claude model (default: opus)
  --help, -h          Show this help

${YELLOW}Examples:${NC}
  hive run --prd prd-security.json
  hive run --prd feature.json --name feature-auth --base develop
  hive run --prd prd.json --iterations 100 --model sonnet

${YELLOW}What it does:${NC}
  1. Creates branch hive/<name> from base
  2. Creates worktree in .hive/drones/<name>/
  3. Copies PRD to worktree
  4. Launches Claude agent in background
  5. Updates drone-status.json for tracking
EOF
}

cmd_run() {
    local prd_file=""
    local drone_name=""
    local base_branch="main"
    local iterations=50
    local model="opus"

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --prd) prd_file="$2"; shift 2 ;;
            --name) drone_name="$2"; shift 2 ;;
            --base) base_branch="$2"; shift 2 ;;
            --iterations) iterations="$2"; shift 2 ;;
            --model) model="$2"; shift 2 ;;
            --help|-h) show_run_usage; exit 0 ;;
            *) print_error "Unknown option: $1"; show_run_usage; exit 1 ;;
        esac
    done

    # Validate
    if [ -z "$prd_file" ]; then
        print_error "PRD file required (--prd <file>)"
        show_run_usage
        exit 1
    fi

    if [ ! -f "$prd_file" ]; then
        print_error "PRD file not found: $prd_file"
        exit 1
    fi

    check_git_repo
    check_dependencies

    # Initialize if needed
    [ -d "$HIVE_DIR" ] || cmd_init

    # Get drone name from PRD if not specified
    if [ -z "$drone_name" ]; then
        drone_name=$(jq -r '.id // .name // "drone"' "$prd_file" | tr '[:upper:]' '[:lower:]' | tr ' ' '-')
    fi

    local project_root=$(get_project_root)
    local project_name=$(get_project_name)
    local branch_name="hive/$drone_name"
    local external_worktree="/Users/fr162241/Projects/${project_name}-${drone_name}"
    local drone_status_dir="$HIVE_DIR/drones/$drone_name"
    local drone_status_file="$drone_status_dir/status.json"

    print_drone "Launching drone: $drone_name"
    print_info "PRD: $prd_file"
    print_info "Branch: $branch_name (from $base_branch)"

    # Check if drone already exists
    if [ -d "$external_worktree" ]; then
        print_warning "Drone $drone_name already exists"
        read -p "Remove and recreate? [y/N] " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            cmd_kill "$drone_name" 2>/dev/null || true
            git worktree remove "$external_worktree" --force 2>/dev/null || true
            git branch -D "$branch_name" 2>/dev/null || true
            rm -rf "$drone_status_dir" 2>/dev/null || true
        else
            exit 1
        fi
    fi

    # Create branch from base
    print_info "Creating branch $branch_name from $base_branch..."
    git branch "$branch_name" "$base_branch" 2>/dev/null || {
        print_warning "Branch exists, reusing..."
    }

    # Create worktree (external path for cleaner separation)
    print_info "Creating worktree at $external_worktree..."
    mkdir -p "$(dirname "$external_worktree")"
    git worktree add "$external_worktree" "$branch_name"

    # Create symlink to .hive in worktree (shared state!)
    print_info "Linking .hive to worktree (shared state)..."
    ln -sf "$project_root/$HIVE_DIR" "$external_worktree/.hive"

    # Copy PRD to .hive/prds/ if not already there
    local prd_basename=$(basename "$prd_file")
    if [ ! -f "$PRDS_DIR/$prd_basename" ]; then
        cp "$prd_file" "$PRDS_DIR/$prd_basename"
        print_info "PRD copied to .hive/prds/$prd_basename"
    fi

    # Create drone status directory and file (in shared .hive)
    mkdir -p "$drone_status_dir"
    local total_stories=$(jq '.stories | length' "$prd_file")
    cat > "$drone_status_file" << EOF
{
  "drone": "$drone_name",
  "prd": "$prd_basename",
  "branch": "$branch_name",
  "worktree": "$external_worktree",
  "status": "starting",
  "current_story": null,
  "completed": [],
  "total": $total_stories,
  "started": "$(get_timestamp)",
  "updated": "$(get_timestamp)"
}
EOF

    # Also create a symlink in worktree for backwards compatibility
    ln -sf "$project_root/$drone_status_file" "$external_worktree/drone-status.json"

    print_success "Drone $drone_name ready!"
    print_info "Worktree: $external_worktree"
    print_info "Shared .hive linked (queen can monitor)"

    # Build the drone prompt
    local drone_prompt="Tu es un drone Hive, un agent autonome qui exécute des PRDs.

**WORKING DIRECTORY**: $external_worktree
**PRD FILE**: $external_worktree/.hive/prds/$prd_basename
**STATUS FILE**: $external_worktree/.hive/drones/$drone_name/status.json
**BRANCH**: $branch_name

IMPORTANT: Toutes tes opérations doivent être dans le répertoire $external_worktree

## Ta mission

1. Lis le fichier PRD pour comprendre les stories à implémenter
2. Pour chaque story:
   - Implémente les changements demandés
   - Commit avec le message \"feat(<STORY-ID>): <description>\"
3. Après chaque story complétée, mets à jour le fichier status.json:
   {
     \"drone\": \"$drone_name\",
     \"status\": \"in_progress\",
     \"current_story\": \"<STORY-ID>\",
     \"completed\": [\"STORY-001\", ...],
     \"total\": $total_stories,
     \"updated\": \"<ISO timestamp>\"
   }
4. Quand toutes les stories sont terminées, mets status à \"completed\"

## Commence maintenant

Lis le PRD et implémente story par story. Sois autonome et méthodique."

    # Launch Claude in background
    print_info "Launching Claude agent..."

    # Create a temp file for the prompt
    local prompt_file=$(mktemp)
    echo "$drone_prompt" > "$prompt_file"

    # Launch in background with nohup
    local log_file="$drone_status_dir/drone.log"
    nohup claude --print -p "$(cat "$prompt_file")" \
        --model "$model" \
        --max-turns "$iterations" \
        --allowedTools "Bash,Read,Write,Edit,Glob,Grep,TodoWrite" \
        > "$log_file" 2>&1 &

    local pid=$!
    echo "$pid" > "$drone_status_dir/.pid"
    rm "$prompt_file"

    print_success "Drone $drone_name launched! (PID: $pid)"
    print_info "Log: $log_file"
    print_info "Status: $drone_status_file"
    echo ""
    print_info "Monitor with: hive status"
    print_info "View logs: hive logs $drone_name"
    print_info "Stop drone: hive kill $drone_name"
}

# ============================================================================
# Status Command
# ============================================================================

cmd_status() {
    check_git_repo

    local project_name=$(get_project_name)

    echo ""
    echo -e "${CYAN}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║${NC}                    ${YELLOW}👑 HIVE STATUS${NC}                            ${CYAN}║${NC}"
    echo -e "${CYAN}╠══════════════════════════════════════════════════════════════╣${NC}"

    local found_drones=0

    # Scan for drones in .hive/drones/
    if [ -d "$DRONES_DIR" ]; then
        for drone_dir in "$DRONES_DIR"/*/; do
            [ -d "$drone_dir" ] || continue

            local status_file="$drone_dir/status.json"
            [ -f "$status_file" ] || continue

            found_drones=$((found_drones + 1))

            local drone_name=$(basename "$drone_dir")
            local status=$(jq -r '.status // "unknown"' "$status_file")
            local current=$(jq -r '.current_story // "n/a"' "$status_file")
            local completed=$(jq -r '.completed | length // 0' "$status_file")
            local total=$(jq -r '.total // "?"' "$status_file")
            local worktree=$(jq -r '.worktree // ""' "$status_file")
            local pid_file="$drone_dir/.pid"
            local running="no"

            if [ -f "$pid_file" ]; then
                local pid=$(cat "$pid_file")
                if ps -p "$pid" > /dev/null 2>&1; then
                    running="yes"
                fi
            fi

            # Status icon
            local icon="⏸️"
            local color="$NC"
            case "$status" in
                "in_progress"|"starting")
                    if [ "$running" = "yes" ]; then
                        icon="🔄"
                        color="$CYAN"
                    else
                        icon="⏸️"
                        color="$YELLOW"
                    fi
                    ;;
                "completed") icon="✅"; color="$GREEN" ;;
                "error") icon="❌"; color="$RED" ;;
            esac

            echo -e "${CYAN}║${NC}  ${color}$icon 🐝 $drone_name${NC}"
            echo -e "${CYAN}║${NC}     Progress: ${GREEN}$completed${NC}/${total} stories"
            echo -e "${CYAN}║${NC}     Current:  $current"
            echo -e "${CYAN}║${NC}     Status:   $status (running: $running)"
            echo -e "${CYAN}║${NC}     Worktree: $worktree"
            echo -e "${CYAN}╠══════════════════════════════════════════════════════════════╣${NC}"
        done
    fi

    if [ $found_drones -eq 0 ]; then
        echo -e "${CYAN}║${NC}  No active drones found"
        echo -e "${CYAN}║${NC}"
        echo -e "${CYAN}║${NC}  Launch one with: hive run --prd <file.json>"
        echo -e "${CYAN}╠══════════════════════════════════════════════════════════════╣${NC}"
    fi

    echo -e "${CYAN}║${NC}  Last check: $(date '+%H:%M:%S')                                    ${CYAN}║${NC}"
    echo -e "${CYAN}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo ""
}

# ============================================================================
# Logs Command
# ============================================================================

cmd_logs() {
    local drone_name="$1"
    local follow=false

    if [ "$drone_name" = "-f" ]; then
        follow=true
        drone_name="$2"
    fi

    if [ -z "$drone_name" ]; then
        print_error "Drone name required"
        echo "Usage: hive logs [-f] <drone-name>"
        exit 1
    fi

    local log_file="$DRONES_DIR/$drone_name/drone.log"

    if [ ! -f "$log_file" ]; then
        print_error "Log file not found: $log_file"
        exit 1
    fi

    if [ "$follow" = true ]; then
        tail -f "$log_file"
    else
        tail -100 "$log_file"
    fi
}

# ============================================================================
# Kill Command
# ============================================================================

cmd_kill() {
    local drone_name="$1"

    if [ -z "$drone_name" ]; then
        print_error "Drone name required"
        echo "Usage: hive kill <drone-name>"
        exit 1
    fi

    local pid_file="$DRONES_DIR/$drone_name/.pid"

    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if ps -p "$pid" > /dev/null 2>&1; then
            kill "$pid" 2>/dev/null || true
            print_success "Killed drone $drone_name (PID: $pid)"
        else
            print_warning "Drone $drone_name was not running"
        fi
        rm -f "$pid_file"
    else
        print_warning "No PID file found for drone $drone_name"
    fi
}

# ============================================================================
# Clean Command
# ============================================================================

cmd_clean() {
    local drone_name="$1"
    local force=false

    [ "$drone_name" = "-f" ] && { force=true; drone_name="$2"; }
    [ "$drone_name" = "--force" ] && { force=true; drone_name="$2"; }

    if [ -z "$drone_name" ]; then
        print_error "Drone name required"
        echo "Usage: hive clean [-f] <drone-name>"
        exit 1
    fi

    check_git_repo

    local project_name=$(get_project_name)
    local drone_status_dir="$DRONES_DIR/$drone_name"
    local worktree_path=""
    local branch_name="hive/$drone_name"

    # Get worktree path from status file if exists
    if [ -f "$drone_status_dir/status.json" ]; then
        worktree_path=$(jq -r '.worktree // ""' "$drone_status_dir/status.json")
    fi

    # Fallback to default path
    [ -z "$worktree_path" ] && worktree_path="/Users/fr162241/Projects/${project_name}-${drone_name}"

    if [ "$force" = false ]; then
        read -p "Remove drone $drone_name and its worktree? [y/N] " -n 1 -r
        echo
        [[ ! $REPLY =~ ^[Yy]$ ]] && exit 0
    fi

    # Kill if running
    cmd_kill "$drone_name" 2>/dev/null || true

    # Remove worktree
    if [ -d "$worktree_path" ]; then
        git worktree remove "$worktree_path" --force 2>/dev/null || {
            print_warning "Could not remove worktree, deleting directory..."
            rm -rf "$worktree_path"
        }
        print_success "Removed worktree: $worktree_path"
    fi

    # Remove branch
    git branch -D "$branch_name" 2>/dev/null && print_success "Removed branch: $branch_name"

    # Remove drone status directory
    if [ -d "$drone_status_dir" ]; then
        rm -rf "$drone_status_dir"
        print_success "Removed drone status: $drone_status_dir"
    fi

    print_success "Drone $drone_name cleaned up"
}

# ============================================================================
# List Command
# ============================================================================

cmd_list() {
    check_git_repo
    local project_name=$(get_project_name)

    echo -e "${YELLOW}👑 Active drones for $project_name:${NC}"
    echo ""

    local count=0
    if [ -d "$DRONES_DIR" ]; then
        for drone_dir in "$DRONES_DIR"/*/; do
            [ -d "$drone_dir" ] || continue
            [ -f "$drone_dir/status.json" ] || continue

            local name=$(basename "$drone_dir")
            local status=$(jq -r '.status // "unknown"' "$drone_dir/status.json")
            local completed=$(jq -r '.completed | length // 0' "$drone_dir/status.json")
            local total=$(jq -r '.total // "?"' "$drone_dir/status.json")

            local status_icon=""
            case "$status" in
                "completed") status_icon="✓" ;;
                "error") status_icon="✗" ;;
                *) status_icon="" ;;
            esac

            echo -e "  🐝 ${CYAN}$name${NC} $status_icon ($completed/$total)"
            count=$((count + 1))
        done
    fi

    [ $count -eq 0 ] && echo "  No active drones"
    echo ""
}

# ============================================================================
# Version Command
# ============================================================================

cmd_version() {
    echo -e "${YELLOW}🐝 Hive${NC} v$VERSION"
    echo "Drone orchestration for Claude Code"
}

# ============================================================================
# Help Command
# ============================================================================

cmd_help() {
    cat << EOF

${YELLOW}🐝 Hive${NC} v$VERSION - Drone Orchestration for Claude Code

${CYAN}Usage:${NC}
  hive <command> [options]

${CYAN}Commands:${NC}
  ${GREEN}run${NC}      Launch a drone on a PRD file
  ${GREEN}status${NC}   Show status of all drones
  ${GREEN}list${NC}     List active drones
  ${GREEN}logs${NC}     View drone logs
  ${GREEN}kill${NC}     Stop a running drone
  ${GREEN}clean${NC}    Remove a drone and its worktree
  ${GREEN}init${NC}     Initialize Hive in current repo
  ${GREEN}version${NC}  Show version
  ${GREEN}help${NC}     Show this help

${CYAN}Quick Start:${NC}
  hive run --prd prd-feature.json
  hive status
  hive logs my-feature
  hive kill my-feature

${CYAN}Examples:${NC}
  # Launch a drone on a PRD
  hive run --prd security.json --name security --base main

  # Monitor progress
  hive status

  # View drone output
  hive logs security
  hive logs -f security  # follow mode

  # Stop and cleanup
  hive kill security
  hive clean security

EOF
}

# ============================================================================
# Main
# ============================================================================

main() {
    local command="${1:-help}"
    shift || true

    case "$command" in
        run)     cmd_run "$@" ;;
        status)  cmd_status "$@" ;;
        list)    cmd_list "$@" ;;
        logs)    cmd_logs "$@" ;;
        kill)    cmd_kill "$@" ;;
        clean)   cmd_clean "$@" ;;
        init)    cmd_init "$@" ;;
        version) cmd_version ;;
        help|--help|-h) cmd_help ;;
        *)
            print_error "Unknown command: $command"
            cmd_help
            exit 1
            ;;
    esac
}

main "$@"
