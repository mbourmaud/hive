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
VERSION="1.10.0"

# Auto-clean configuration
INACTIVE_THRESHOLD=3600  # 60 minutes in seconds

# Configuration
HIVE_DIR=".hive"
CONFIG_FILE="$HIVE_DIR/config.json"
PRDS_DIR="$HIVE_DIR/prds"
DRONES_DIR="$HIVE_DIR/drones"

# Profile configuration
HIVE_GLOBAL_CONFIG="$HOME/.config/hive/config.json"

# Update check config
HIVE_CACHE_DIR="$HOME/.cache/hive"
HIVE_VERSION_CACHE="$HIVE_CACHE_DIR/latest_version"
HIVE_CHECK_INTERVAL=86400  # 24 hours in seconds
HIVE_REPO_URL="https://raw.githubusercontent.com/mbourmaud/hive/main"

# ============================================================================
# Helper Functions
# ============================================================================

print_info() { echo -e "${BLUE}ℹ${NC} $1"; }
print_success() { echo -e "${GREEN}✓${NC} $1"; }
print_warning() { echo -e "${YELLOW}⚠${NC} $1"; }
print_error() { echo -e "${RED}✗${NC} $1" >&2; }
print_drone() { echo -e "${CYAN}🐝${NC} $1"; }

get_timestamp() { date -u +"%Y-%m-%dT%H:%M:%SZ"; }

# ============================================================================
# Profile Management Functions
# ============================================================================

# Initialize global config with default profile
init_global_config() {
    if [ ! -f "$HIVE_GLOBAL_CONFIG" ]; then
        mkdir -p "$(dirname "$HIVE_GLOBAL_CONFIG")"
        cat > "$HIVE_GLOBAL_CONFIG" <<EOF
{
  "version": "1.0.0",
  "default_profile": "default",
  "profiles": {
    "default": {
      "claude_command": "claude",
      "description": "Default Claude CLI"
    }
  }
}
EOF
    fi
}

# Get claude command for a profile
get_claude_command() {
    local profile="${1:-}"

    init_global_config

    # If no profile specified, use default
    if [ -z "$profile" ]; then
        profile=$(jq -r '.default_profile // "default"' "$HIVE_GLOBAL_CONFIG" 2>/dev/null)
    fi

    # Get command from profile
    local cmd=$(jq -r --arg profile "$profile" '.profiles[$profile].claude_command // empty' "$HIVE_GLOBAL_CONFIG" 2>/dev/null)

    if [ -z "$cmd" ]; then
        # Profile not found, use default
        if [ "$profile" != "default" ]; then
            print_warning "Profile '$profile' not found, using 'default'"
        fi
        cmd=$(jq -r '.profiles.default.claude_command // "claude"' "$HIVE_GLOBAL_CONFIG" 2>/dev/null)
    fi

    echo "$cmd"
}

# ============================================================================
# Notification Functions (cross-platform)
# ============================================================================

# Send a desktop notification (works on macOS, Linux, and Windows/WSL)
send_notification() {
    local title="$1"
    local message="$2"
    local sound="${3:-true}"  # Play sound by default
    local icon="$HOME/.local/share/hive/bee-icon.png"

    # macOS with terminal-notifier (preferred - supports custom icon)
    if command -v terminal-notifier &>/dev/null; then
        local sound_param=""
        [ "$sound" = "true" ] && sound_param="-sound Glass"
        if [ -f "$icon" ]; then
            terminal-notifier -title "$title" -message "$message" -appIcon "$icon" $sound_param -group "hive" 2>/dev/null || true
        else
            terminal-notifier -title "$title" -message "$message" $sound_param -group "hive" 2>/dev/null || true
        fi
        return
    fi

    # macOS fallback with osascript
    if command -v osascript &>/dev/null; then
        local sound_param=""
        [ "$sound" = "true" ] && sound_param='sound name "Glass"'
        osascript -e "display notification \"$message\" with title \"$title\" $sound_param" 2>/dev/null || true
        return
    fi

    # Linux with notify-send (GNOME, KDE, etc.)
    if command -v notify-send &>/dev/null; then
        if [ -f "$icon" ]; then
            notify-send "$title" "$message" --icon="$icon" 2>/dev/null || true
        else
            notify-send "$title" "$message" --icon=dialog-information 2>/dev/null || true
        fi
        # Play sound on Linux if paplay available
        if [ "$sound" = "true" ] && command -v paplay &>/dev/null; then
            paplay /usr/share/sounds/freedesktop/stereo/complete.oga 2>/dev/null &
        fi
        return
    fi

    # Windows (WSL) using PowerShell
    if command -v powershell.exe &>/dev/null; then
        powershell.exe -Command "[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null; \$template = [Windows.UI.Notifications.ToastNotificationManager]::GetTemplateContent([Windows.UI.Notifications.ToastTemplateType]::ToastText02); \$template.SelectSingleNode('//text[@id=\"1\"]').InnerText = '$title'; \$template.SelectSingleNode('//text[@id=\"2\"]').InnerText = '$message'; [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('Hive').Show([Windows.UI.Notifications.ToastNotification]::new(\$template))" 2>/dev/null || true
        return
    fi

    # Fallback: terminal bell
    if [ "$sound" = "true" ]; then
        printf '\a'
    fi
}

# Notify drone started
notify_drone_started() {
    local drone_name="$1"
    local total_stories="$2"
    send_notification "🐝 Hive - Drone Started" "$drone_name: $total_stories stories to implement"
}

# Notify drone completed
notify_drone_completed() {
    local drone_name="$1"
    local completed="$2"
    local total="$3"
    send_notification "🎉 Hive - Drone Completed" "$drone_name: $completed/$total stories done!"
}

# Notify drone error
notify_drone_error() {
    local drone_name="$1"
    local error_msg="$2"
    send_notification "❌ Hive - Drone Error" "$drone_name: $error_msg"
}

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

# Install optional dependencies for enhanced features
install_optional_deps() {
    # gum - TUI toolkit for interactive mode
    if ! command -v gum &>/dev/null; then
        if command -v brew &>/dev/null; then
            print_info "Installing gum (TUI toolkit)..."
            brew install gum >/dev/null 2>&1 && print_success "gum installed"
        fi
    fi
}

check_gum() {
    if ! command -v gum &>/dev/null; then
        print_error "Interactive mode requires 'gum'. Install with: brew install gum"
        exit 1
    fi
}

get_project_root() {
    git rev-parse --show-toplevel 2>/dev/null
}

get_project_name() {
    basename "$(get_project_root)"
}

get_worktree_base() {
    # Priority: 1. ENV var, 2. local config.json, 3. global config, 4. default
    if [ -n "$HIVE_WORKTREE_BASE" ]; then
        echo "$HIVE_WORKTREE_BASE"
    elif [ -f "$CONFIG_FILE" ]; then
        local config_base=$(jq -r '.worktree_base // ""' "$CONFIG_FILE" 2>/dev/null)
        if [ -n "$config_base" ]; then
            echo "$config_base"
            return
        fi
    fi

    # Try global config
    if [ -f "$HIVE_GLOBAL_CONFIG" ]; then
        local global_base=$(jq -r '.worktree_base // ""' "$HIVE_GLOBAL_CONFIG" 2>/dev/null)
        if [ -n "$global_base" ]; then
            echo "$global_base"
            return
        fi
    fi

    # Default: centralized worktrees in home directory
    echo "$HOME/.hive/worktrees"
}

# ============================================================================
# Update Check Functions
# ============================================================================

# Compare semantic versions: returns 0 if $1 < $2
version_lt() {
    [ "$1" = "$2" ] && return 1
    local IFS=.
    local i ver1=($1) ver2=($2)
    for ((i=0; i<${#ver1[@]}; i++)); do
        [ -z "${ver2[i]}" ] && return 1
        [ "${ver1[i]}" -lt "${ver2[i]}" ] 2>/dev/null && return 0
        [ "${ver1[i]}" -gt "${ver2[i]}" ] 2>/dev/null && return 1
    done
    return 1
}

# Fetch latest version from GitHub (silent, non-blocking)
fetch_latest_version() {
    mkdir -p "$HIVE_CACHE_DIR"
    local remote_version
    remote_version=$(curl -sL --connect-timeout 2 --max-time 5 "$HIVE_REPO_URL/hive.sh" 2>/dev/null | grep '^VERSION=' | head -1 | cut -d'"' -f2)
    if [ -n "$remote_version" ]; then
        echo "$remote_version" > "$HIVE_VERSION_CACHE"
        touch "$HIVE_VERSION_CACHE"
    fi
}

# Check if update is available (uses cache, non-blocking)
check_for_updates() {
    # Skip update check if HIVE_NO_UPDATE_CHECK is set
    [ -n "$HIVE_NO_UPDATE_CHECK" ] && return

    mkdir -p "$HIVE_CACHE_DIR"

    local should_fetch=false

    if [ ! -f "$HIVE_VERSION_CACHE" ]; then
        should_fetch=true
    else
        local cache_age=$(($(date +%s) - $(stat -f %m "$HIVE_VERSION_CACHE" 2>/dev/null || stat -c %Y "$HIVE_VERSION_CACHE" 2>/dev/null || echo 0)))
        [ "$cache_age" -gt "$HIVE_CHECK_INTERVAL" ] && should_fetch=true
    fi

    # Fetch in background if needed (non-blocking)
    if [ "$should_fetch" = true ]; then
        (fetch_latest_version &) 2>/dev/null
    fi

    # Show update message if cache exists and version is newer
    if [ -f "$HIVE_VERSION_CACHE" ]; then
        local latest=$(cat "$HIVE_VERSION_CACHE" 2>/dev/null)
        if [ -n "$latest" ] && version_lt "$VERSION" "$latest"; then
            echo -e "${YELLOW}⚠ Update available: $VERSION → $latest${NC} (run 'hive update')"
            echo ""
        fi
    fi
}

# ============================================================================
# Init Command
# ============================================================================

cmd_init() {
    check_git_repo
    check_dependencies

    print_info "Initializing Hive..."

    # Fix circular symlink bug: .hive should be a directory, not a symlink
    if [ -L "$HIVE_DIR" ]; then
        print_warning ".hive is a symlink (should be a directory). Fixing..."
        rm "$HIVE_DIR"
    fi

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

    # Ensure global config exists (first-time setup)
    if [ ! -f "$HIVE_GLOBAL_CONFIG" ]; then
        # Default: centralized worktrees in home directory (cleaner)
        local worktree_base="$HOME/.hive/worktrees"

        echo ""
        print_info "🐝 First-time Hive Setup"
        echo -e "${CYAN}Drones will be created in separate worktrees outside your repositories.${NC}"
        echo -e "${CYAN}Default location: ${YELLOW}$worktree_base${NC}"
        echo -e "${CYAN}Structure: ${YELLOW}~/.hive/worktrees/<project>/<drone>/${NC}"
        echo ""
        read -p "$(echo -e "${YELLOW}Use default location? (Y/n): ${NC}")" -n 1 -r use_default
        echo ""

        if [[ ! $use_default =~ ^[Yy]$ ]] && [[ -n $use_default ]]; then
            read -p "$(echo -e "${YELLOW}Enter worktree base path: ${NC}")" custom_path
            if [ -n "$custom_path" ]; then
                # Expand ~ to $HOME
                worktree_base="${custom_path/#\~/$HOME}"
            fi
        fi

        # Create worktree base directory if it doesn't exist
        if [ ! -d "$worktree_base" ]; then
            echo ""
            print_warning "Directory does not exist: $worktree_base"
            read -p "$(echo -e "${YELLOW}Create it? (Y/n): ${NC}")" -n 1 -r create_dir
            echo ""

            if [[ $create_dir =~ ^[Yy]$ ]] || [[ -z $create_dir ]]; then
                mkdir -p "$worktree_base"
                print_success "Created: $worktree_base"
            else
                print_error "Aborting: worktree base directory must exist"
                exit 1
            fi
        fi

        # Create global config
        mkdir -p "$(dirname "$HIVE_GLOBAL_CONFIG")"
        cat > "$HIVE_GLOBAL_CONFIG" << EOF
{
  "worktree_base": "$worktree_base"
}
EOF
        print_success "Global worktree base configured: $worktree_base"
        print_info "You can change this later by editing: $HIVE_GLOBAL_CONFIG"
        echo ""
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
# Start Command - Launch a drone on a PRD
# ============================================================================

show_start_usage() {
    cat << EOF
${CYAN}hive start${NC} - Launch a drone on a PRD file

${YELLOW}Usage:${NC}
  hive start <prd-name> [options]
  hive start --prd <file> [options]

${YELLOW}Arguments:${NC}
  <prd-name>          PRD name (searches .hive/prds/, current dir)
                      Examples: ui-kit-refactor, valibot-migration

${YELLOW}Options:${NC}
  --prd <file>        PRD JSON file path (alternative to positional arg)
  --name <name>       Drone name (default: derived from PRD id)
  --base <branch>     Base branch (default: main)
  --iterations <n>    Max iterations (default: 15, each = full Claude session)
  --model <model>     Claude model (default: sonnet)
  --profile <name>    Claude profile to use (default: from ~/.config/hive/config.json)
  --local             Run in current directory (no worktree creation)
  --help, -h          Show this help

${YELLOW}Examples:${NC}
  hive start ui-kit-refactor                    # Finds .hive/prds/prd-ui-kit-refactor.json
  hive start valibot-migration --model opus     # Use opus model
  hive start --prd ./custom/my-prd.json         # Explicit path
  hive start fix-queries --local                # Run in current directory

${YELLOW}What it does:${NC}
  1. Creates branch hive/<name> from base (or resumes if drone exists)
  2. Creates worktree at ~/Projects/{project}-{drone}/ (unless --local)
  3. Symlinks .hive/ to worktree (shared state)
  4. Launches Claude agent in background
  5. Updates .hive/drones/<name>/status.json for tracking

${YELLOW}Note:${NC}
  If the drone already exists, it will automatically resume with the
  existing worktree. Use 'hive clean <name>' first to start fresh.

  Use --local when you're already on the target branch and want to
  run the drone in your current working directory.
EOF
}

cmd_run() {
    local prd_file=""
    local drone_name=""
    local base_branch="main"
    local iterations=15
    local model="sonnet"
    local resume=false
    local local_mode=false
    local profile=""

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --prd) prd_file="$2"; shift 2 ;;
            --name) drone_name="$2"; shift 2 ;;
            --base) base_branch="$2"; shift 2 ;;
            --iterations) iterations="$2"; shift 2 ;;
            --model) model="$2"; shift 2 ;;
            --profile) profile="$2"; shift 2 ;;
            --resume) resume=true; shift ;;
            --local) local_mode=true; shift ;;
            --help|-h) show_start_usage; exit 0 ;;
            -*)
                print_error "Unknown option: $1"
                show_start_usage
                exit 1
                ;;
            *)
                # Positional argument: PRD name or file
                if [ -z "$prd_file" ]; then
                    prd_file="$1"
                fi
                shift
                ;;
        esac
    done

    # Resolve PRD file from name if needed
    if [ -n "$prd_file" ] && [ ! -f "$prd_file" ]; then
        # Try common locations
        local prd_name="$prd_file"
        if [ -f ".hive/prds/${prd_name}.json" ]; then
            prd_file=".hive/prds/${prd_name}.json"
        elif [ -f ".hive/prds/prd-${prd_name}.json" ]; then
            prd_file=".hive/prds/prd-${prd_name}.json"
        elif [ -f "${prd_name}.json" ]; then
            prd_file="${prd_name}.json"
        elif [ -f "prd-${prd_name}.json" ]; then
            prd_file="prd-${prd_name}.json"
        else
            print_error "PRD not found: $prd_name"
            print_info "Searched: .hive/prds/${prd_name}.json, .hive/prds/prd-${prd_name}.json, ${prd_name}.json"
            exit 1
        fi
    fi

    # Validate
    if [ -z "$prd_file" ]; then
        print_error "PRD file required"
        show_start_usage
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

    # Use target_branch from PRD if specified, otherwise default to hive/<name>
    local target_branch=$(jq -r '.target_branch // empty' "$prd_file")
    local branch_name="${target_branch:-hive/$drone_name}"

    local worktree_base=$(get_worktree_base)
    local external_worktree="$worktree_base/$project_name/$drone_name"
    local drone_status_dir="$HIVE_DIR/drones/$drone_name"
    local drone_status_file="$drone_status_dir/status.json"

    # Determine workdir: current directory (--local) or external worktree
    local workdir
    if [ "$local_mode" = true ]; then
        workdir="$project_root"
    else
        workdir="$external_worktree"
    fi

    print_drone "Launching drone: $drone_name"
    print_info "PRD: $prd_file"
    if [ "$local_mode" = true ]; then
        print_info "Mode: local (current directory)"
        print_info "Workdir: $workdir"
    else
        print_info "Branch: $branch_name (from $base_branch)"
    fi

    # Handle --local mode: run in current directory, no worktree
    if [ "$local_mode" = true ]; then
        # Copy PRD to .hive/prds/ if not already there
        local prd_basename=$(basename "$prd_file")
        if [ ! -f "$PRDS_DIR/$prd_basename" ]; then
            cp "$prd_file" "$PRDS_DIR/$prd_basename"
            print_info "PRD copied to .hive/prds/$prd_basename"
        else
            cp -f "$prd_file" "$PRDS_DIR/$prd_basename" 2>/dev/null || true
            print_info "PRD updated: .hive/prds/$prd_basename"
        fi

        # Check if resuming (status file exists)
        if [ -f "$drone_status_file" ]; then
            print_drone "Resuming drone: $drone_name"
            local total_stories=$(jq '.stories | length' "$prd_file")
            jq --arg ts "$(get_timestamp)" --argjson total "$total_stories" \
                '.status = "resuming" | .updated = $ts | .total = $total' \
                "$drone_status_file" > /tmp/status.tmp && mv /tmp/status.tmp "$drone_status_file"
            resume=true
        fi
    # Check if drone already exists (worktree mode)
    elif [ -d "$external_worktree" ]; then
        # Auto-resume: reuse existing worktree
        print_drone "Resuming drone: $drone_name"
        print_info "Worktree: $external_worktree"

        # Update PRD in .hive/prds/ if provided a newer one
        local prd_basename=$(basename "$prd_file")
        # Use cp -f and ignore error if files are identical
        cp -f "$prd_file" "$PRDS_DIR/$prd_basename" 2>/dev/null || true
        print_info "PRD updated: .hive/prds/$prd_basename"

        # Ensure .hive symlink is correct
        if [ ! -L "$external_worktree/.hive" ]; then
            rm -rf "$external_worktree/.hive" 2>/dev/null

            # Verify we're not creating a circular symlink
            local hive_source="$project_root/$HIVE_DIR"
            local hive_target="$external_worktree/.hive"
            if [ "$(realpath "$hive_source" 2>/dev/null)" = "$(realpath "$hive_target" 2>/dev/null)" ]; then
                print_error "Cannot create symlink: would be circular"
                exit 1
            fi

            ln -s "$hive_source" "$hive_target"
            print_info "Fixed .hive symlink"
        fi

        # Update status to resuming
        local drone_status_dir="$HIVE_DIR/drones/$drone_name"
        local drone_status_file="$drone_status_dir/status.json"
        if [ -f "$drone_status_file" ]; then
            local total_stories=$(jq '.stories | length' "$prd_file")
            jq --arg ts "$(get_timestamp)" --argjson total "$total_stories" \
                '.status = "resuming" | .updated = $ts | .total = $total' \
                "$drone_status_file" > /tmp/status.tmp && mv /tmp/status.tmp "$drone_status_file"
        fi
        resume=true
    else
        # Create new drone with worktree

        # Create branch from base
        print_info "Creating branch $branch_name from $base_branch..."
        git branch "$branch_name" "$base_branch" 2>/dev/null || {
            print_warning "Branch exists, reusing..."
        }

        # Ensure worktree base directory exists
        if [ ! -d "$worktree_base" ]; then
            print_warning "Worktree base directory does not exist: $worktree_base"
            mkdir -p "$worktree_base"
            print_success "Created: $worktree_base"
        fi

        # Create worktree (external path for cleaner separation)
        print_info "Creating worktree at $external_worktree..."
        mkdir -p "$(dirname "$external_worktree")"
        git worktree add "$external_worktree" "$branch_name"

        # Create symlink to .hive in worktree (shared state!)
        # IMPORTANT: Remove any existing .hive first to avoid circular symlink
        # (ln -sf on a directory creates the symlink inside it, not replacing it)
        print_info "Linking .hive to worktree (shared state)..."
        rm -rf "$external_worktree/.hive" 2>/dev/null

        # Verify we're not creating a circular symlink
        local hive_source="$project_root/$HIVE_DIR"
        local hive_target="$external_worktree/.hive"
        if [ "$(realpath "$hive_source" 2>/dev/null)" = "$(realpath "$hive_target" 2>/dev/null)" ]; then
            print_error "Cannot create symlink: would be circular"
            print_error "Source: $hive_source"
            print_error "Target: $hive_target"
            exit 1
        fi

        ln -s "$hive_source" "$hive_target"

        # Copy PRD to .hive/prds/ if not already there
        local prd_basename=$(basename "$prd_file")
        if [ ! -f "$PRDS_DIR/$prd_basename" ]; then
            cp "$prd_file" "$PRDS_DIR/$prd_basename"
            print_info "PRD copied to .hive/prds/$prd_basename"
        fi
    fi

    # Create/update drone status directory and file (in shared .hive)
    local drone_status_dir="$HIVE_DIR/drones/$drone_name"
    local drone_status_file="$drone_status_dir/status.json"
    mkdir -p "$drone_status_dir"
    # Create logs directory for comprehensive logging
    mkdir -p "$drone_status_dir/logs"
    local prd_basename=$(basename "$prd_file")
    local total_stories=$(jq '.stories | length' "$prd_file")

    if [ "$resume" = true ] && [ -f "$drone_status_file" ]; then
        # Resume mode: update existing status
        local was_blocked=$(jq -r '.status // ""' "$drone_status_file")

        # If drone was blocked, clear blocking fields
        if [ "$was_blocked" = "blocked" ]; then
            print_info "Clearing blocked status..."
            rm -f "$drone_status_dir/blocked.md"
            jq --arg ts "$(get_timestamp)" --argjson total "$total_stories" --arg prd "$prd_basename" \
                '.status = "resuming" | .updated = $ts | .total = $total | .prd = $prd | .blocked_reason = null | .blocked_questions = [] | .awaiting_human = false | .error_count = 0 | .last_error_story = null' \
                "$drone_status_file" > /tmp/status.tmp && mv /tmp/status.tmp "$drone_status_file"
            print_success "Drone unblocked and ready to resume!"
        else
            jq --arg ts "$(get_timestamp)" --argjson total "$total_stories" --arg prd "$prd_basename" \
                '.status = "resuming" | .updated = $ts | .total = $total | .prd = $prd' \
                "$drone_status_file" > /tmp/status.tmp && mv /tmp/status.tmp "$drone_status_file"
        fi
        print_info "Status updated (resuming with $(jq -r '.completed | length' "$drone_status_file") stories completed)"
    else
        # New drone: create fresh status
        cat > "$drone_status_file" << EOF
{
  "drone": "$drone_name",
  "prd": "$prd_basename",
  "branch": "$branch_name",
  "worktree": "$workdir",
  "local_mode": $local_mode,
  "status": "starting",
  "current_story": null,
  "completed": [],
  "story_times": {},
  "total": $total_stories,
  "started": "$(get_timestamp)",
  "updated": "$(get_timestamp)",
  "error_count": 0,
  "last_error_story": null,
  "blocked_reason": null,
  "blocked_questions": [],
  "awaiting_human": false
}
EOF
    fi

    # Also create a symlink in worktree for backwards compatibility (not in local mode)
    if [ "$local_mode" = false ] && [ ! -L "$workdir/drone-status.json" ]; then
        rm -f "$workdir/drone-status.json" 2>/dev/null
        ln -s "$project_root/$drone_status_file" "$workdir/drone-status.json"
    fi

    if [ "$resume" = true ]; then
        print_success "Drone $drone_name resumed!"
    else
        print_success "Drone $drone_name ready!"
    fi
    if [ "$local_mode" = true ]; then
        print_info "Workdir: $workdir (local mode)"
    else
        print_info "Worktree: $workdir"
        print_info "Shared .hive linked (queen can monitor)"
    fi

    # Build the drone prompt
    local drone_prompt="# 🐝 Drone Hive - Agent Autonome

## ⚠️ RÈGLES CRITIQUES - EXÉCUTE CES COMMANDES À CHAQUE STORY

### 1. AVANT de commencer une story (remplace STORY-ID par l'ID réel):
\`\`\`bash
jq --arg story \"STORY-ID\" --arg ts \"\$(date -u +%Y-%m-%dT%H:%M:%SZ)\" '.current_story = \$story | .updated = \$ts | .story_times[\$story].started = \$ts' $workdir/.hive/drones/$drone_name/status.json > /tmp/s.tmp && mv /tmp/s.tmp $workdir/.hive/drones/$drone_name/status.json && echo \"[\$(date +%H:%M:%S)] 🔨 Début STORY-ID\" >> $workdir/.hive/drones/$drone_name/activity.log
\`\`\`

### 2. APRÈS chaque commit (remplace STORY-ID par l'ID réel):
\`\`\`bash
echo \"[\$(date +%H:%M:%S)] 💾 Commit STORY-ID\" >> $workdir/.hive/drones/$drone_name/activity.log
\`\`\`

### 3. APRÈS avoir terminé une story (remplace STORY-ID par l'ID réel):
\`\`\`bash
jq --arg story \"STORY-ID\" --arg ts \"\$(date -u +%Y-%m-%dT%H:%M:%SZ)\" '.completed += [\$story] | .updated = \$ts | .story_times[\$story].completed = \$ts' $workdir/.hive/drones/$drone_name/status.json > /tmp/s.tmp && mv /tmp/s.tmp $workdir/.hive/drones/$drone_name/status.json && echo \"[\$(date +%H:%M:%S)] ✅ STORY-ID terminée\" >> $workdir/.hive/drones/$drone_name/activity.log && C=\$(jq -r '.completed|length' $workdir/.hive/drones/$drone_name/status.json) && T=\$(jq -r '.total' $workdir/.hive/drones/$drone_name/status.json) && terminal-notifier -title \"🐝 $drone_name\" -message \"STORY-ID terminée (\$C/\$T)\" -sound Glass 2>/dev/null || osascript -e \"display notification \\\"STORY-ID terminée (\$C/\$T)\\\" with title \\\"🐝 $drone_name\\\" sound name \\\"Glass\\\"\" 2>/dev/null || true
\`\`\`

### 4. Quand TOUTES les stories sont terminées:
\`\`\`bash
jq --arg ts \"\$(date -u +%Y-%m-%dT%H:%M:%SZ)\" '.status = \"completed\" | .current_story = null | .updated = \$ts' $workdir/.hive/drones/$drone_name/status.json > /tmp/s.tmp && mv /tmp/s.tmp $workdir/.hive/drones/$drone_name/status.json && echo \"[\$(date +%H:%M:%S)] 🎉 Terminé\" >> $workdir/.hive/drones/$drone_name/activity.log
\`\`\`

**⚠️ SI TU N'EXÉCUTES PAS CES COMMANDES, LE MONITORING NE FONCTIONNE PAS.**
**⚠️ EXÉCUTE-LES SYSTÉMATIQUEMENT, PAS D'EXCEPTION.**

---

## Configuration

- **WORKDIR**: $workdir
- **PRD**: $workdir/.hive/prds/$prd_basename
- **STATUS**: $workdir/.hive/drones/$drone_name/status.json (affiche X/Y dans hive status)
- **LOG**: $workdir/.hive/drones/$drone_name/activity.log (visible via hive logs)

---

## Workflow pour chaque story

1. **Exécute commande #1** (current_story + log début)
2. Lis la story dans le PRD, notamment:
   - \`definition_of_done\`: liste des critères à remplir
   - \`verification_commands\`: commandes à exécuter pour PROUVER que c'est fait
3. Implémente les changements
4. \`git add -A && git commit -m \"feat(STORY-ID): description\"\`
5. **Exécute commande #2** (log commit)
6. **⚠️ VÉRIFIE LA DEFINITION OF DONE:**
   - Exécute CHAQUE commande dans \`verification_commands\`
   - Vérifie que le résultat correspond à \`expected\`
   - Si une vérification échoue → CORRIGE avant de continuer
7. **Exécute commande #3** (completed + log terminée) ← SEULEMENT si toutes les vérifications passent
8. Passe à la story suivante

**⚠️ RÈGLE ABSOLUE: Tu ne peux PAS marquer une story comme terminée si les verification_commands échouent.**

Quand toutes les stories sont faites → **Exécute commande #4**

---

## Ta mission

1. **D'ABORD** installe les dépendances (worktree = copie fraîche, pas de deps installées):
   - Détecte le type de projet et installe les dépendances appropriées
   - Node.js: \`pnpm install\` / \`yarn install\` / \`npm install\` (selon lockfile)
   - Python: \`pip install -r requirements.txt\` ou \`poetry install\` ou \`uv sync\`
   - Go: \`go mod download\`
   - Rust: \`cargo fetch\`
   - Autre: adapte selon le projet
2. Lis le status.json pour voir les stories déjà terminées:
   \`\`\`bash
   cat $workdir/.hive/drones/$drone_name/status.json
   \`\`\`
3. Lis le PRD: $workdir/.hive/prds/$prd_basename
4. **SAUTE les stories déjà dans 'completed'** - ne les refais PAS
5. Implémente uniquement les stories restantes dans l'ordre
6. **METS À JOUR status.json ET activity.log À CHAQUE ÉTAPE**

**⚠️ IMPORTANT: Si une story est dans 'completed', PASSE À LA SUIVANTE. Ne refais JAMAIS une story déjà terminée.**

**COMMENCE MAINTENANT.**"

    # Launch Claude in background using a loop (like Ralph)
    print_info "Launching Claude agent..."

    # Create the prompt file (persistent, not temp)
    local prompt_file="$drone_status_dir/prompt.md"
    echo "$drone_prompt" > "$prompt_file"

    # Create the launcher script that runs the loop
    local launcher_script="$drone_status_dir/launcher.sh"
    local log_file="$drone_status_dir/drone.log"
    local activity_log="$drone_status_dir/activity.log"

    cat > "$launcher_script" << 'LAUNCHER_EOF'
#!/bin/bash
set -e

DRONE_DIR="$1"
PROMPT_FILE="$2"
MODEL="$3"
MAX_ITERATIONS="$4"
WORKTREE="$5"
DRONE_NAME="$6"
CLAUDE_CMD="${7:-claude}"

LOG_FILE="$DRONE_DIR/drone.log"
STATUS_FILE="$DRONE_DIR/status.json"
ACTIVITY_LOG="$DRONE_DIR/activity.log"
LOGS_DIR="$DRONE_DIR/logs"

# ============================================================================
# Notification function (embedded in launcher for independence)
# ============================================================================
send_notification() {
    local title="$1"
    local message="$2"
    local sound="${3:-true}"
    local icon="$HOME/.local/share/hive/bee-icon.png"

    # macOS with terminal-notifier (preferred - supports custom icon)
    if command -v terminal-notifier &>/dev/null; then
        local sound_param=""
        [ "$sound" = "true" ] && sound_param="-sound Glass"
        if [ -f "$icon" ]; then
            terminal-notifier -title "$title" -message "$message" -appIcon "$icon" $sound_param -group "hive" 2>/dev/null || true
        else
            terminal-notifier -title "$title" -message "$message" $sound_param -group "hive" 2>/dev/null || true
        fi
        return
    fi

    # macOS fallback with osascript
    if command -v osascript &>/dev/null; then
        local sound_param=""
        [ "$sound" = "true" ] && sound_param='sound name "Glass"'
        osascript -e "display notification \"$message\" with title \"$title\" $sound_param" 2>/dev/null || true
        return
    fi

    # Linux with notify-send
    if command -v notify-send &>/dev/null; then
        if [ -f "$icon" ]; then
            notify-send "$title" "$message" --icon="$icon" 2>/dev/null || true
        else
            notify-send "$title" "$message" --icon=dialog-information 2>/dev/null || true
        fi
        if [ "$sound" = "true" ] && command -v paplay &>/dev/null; then
            paplay /usr/share/sounds/freedesktop/stereo/complete.oga 2>/dev/null &
        fi
        return
    fi

    # Windows (WSL)
    if command -v powershell.exe &>/dev/null; then
        powershell.exe -Command "[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null; \$t = [Windows.UI.Notifications.ToastNotificationManager]::GetTemplateContent([Windows.UI.Notifications.ToastTemplateType]::ToastText02); \$t.SelectSingleNode('//text[@id=\"1\"]').InnerText = '$title'; \$t.SelectSingleNode('//text[@id=\"2\"]').InnerText = '$message'; [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('Hive').Show([Windows.UI.Notifications.ToastNotification]::new(\$t))" 2>/dev/null || true
        return
    fi

    # Fallback: terminal bell
    [ "$sound" = "true" ] && printf '\a'
}

# ============================================================================
# Blocking function - Creates blocked.md and notifies
# ============================================================================
block_drone() {
    local reason="$1"
    local current_story="$2"

    # Update status to blocked
    jq --arg ts "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
       --arg reason "$reason" \
       --arg story "$current_story" \
       '.status = "blocked" | .blocked_reason = $reason | .awaiting_human = true | .updated = $ts' \
       "$STATUS_FILE" > /tmp/status.tmp && mv /tmp/status.tmp "$STATUS_FILE"

    # Create blocked.md
    cat > "$DRONE_DIR/blocked.md" <<BLOCKED_EOF
# Drone Blocked: $current_story

## Reason
$reason

## What Was Being Attempted
Working on story: $current_story

The drone encountered repeated errors (3+ attempts on the same story) and has automatically blocked itself to avoid wasting resources.

## Questions for Human
- Is the story definition clear and achievable?
- Are there missing dependencies or prerequisites?
- Does the codebase need changes before this story can be implemented?
- Should this story be split into smaller stories?

## To Unblock
1. Review the story in the PRD file
2. Update the PRD with clarifications or fixes
3. Run: \`hive start --resume $DRONE_NAME\` or \`hive unblock $DRONE_NAME\`

## Recent Logs
Check logs in: $LOGS_DIR/$current_story/
BLOCKED_EOF

    # Send notification
    send_notification "⚠️ Hive - Drone Blocked" "$DRONE_NAME needs input on $current_story"

    echo "🚫 Drone blocked on $current_story. See $DRONE_DIR/blocked.md for details." >> "$LOG_FILE"
}

# ============================================================================
# Drone Loop
# ============================================================================

echo "Starting drone loop: $MAX_ITERATIONS iterations max" >> "$LOG_FILE"
echo "Working directory: $WORKTREE" >> "$LOG_FILE"

# Get total stories for notification
TOTAL=$(jq -r '.total // 0' "$STATUS_FILE" 2>/dev/null)

# 🔔 Notification: Drone started
send_notification "🐝 Hive - Drone Started" "$DRONE_NAME: $TOTAL stories"

for i in $(seq 1 "$MAX_ITERATIONS"); do
    echo "" >> "$LOG_FILE"
    echo "═══════════════════════════════════════════════════════" >> "$LOG_FILE"
    echo "  Drone Iteration $i of $MAX_ITERATIONS - $(date)" >> "$LOG_FILE"
    echo "═══════════════════════════════════════════════════════" >> "$LOG_FILE"

    # Check if blocked
    if [ -f "$STATUS_FILE" ]; then
        STATUS=$(jq -r '.status // "in_progress"' "$STATUS_FILE" 2>/dev/null)
        if [ "$STATUS" = "blocked" ]; then
            echo "⚠️ Drone is blocked, stopping iteration." >> "$LOG_FILE"
            exit 0
        fi

        # Check if all stories are completed
        if [ "$STATUS" = "completed" ]; then
            echo "" >> "$LOG_FILE"
            echo "🎉 All stories completed! Drone finished at iteration $i." >> "$LOG_FILE"
            # 🔔 Notification: Drone completed
            COMPLETED=$(jq -r '.completed | length // 0' "$STATUS_FILE" 2>/dev/null)
            send_notification "🎉 Hive - Drone Completed!" "$DRONE_NAME: $COMPLETED/$TOTAL stories done"
            exit 0
        fi

        COMPLETED=$(jq -r '.completed | length // 0' "$STATUS_FILE" 2>/dev/null)
        TOTAL=$(jq -r '.total // 0' "$STATUS_FILE" 2>/dev/null)
        if [ "$COMPLETED" -ge "$TOTAL" ] && [ "$TOTAL" -gt 0 ]; then
            # Mark as completed
            jq --arg ts "$(date -u +%Y-%m-%dT%H:%M:%SZ)" '.status = "completed" | .current_story = null | .updated = $ts' "$STATUS_FILE" > /tmp/status.tmp && mv /tmp/status.tmp "$STATUS_FILE"
            echo "" >> "$LOG_FILE"
            echo "🎉 All $TOTAL stories completed! Drone finished at iteration $i." >> "$LOG_FILE"
            # 🔔 Notification: Drone completed
            send_notification "🎉 Hive - Drone Completed!" "$DRONE_NAME: $COMPLETED/$TOTAL stories done"
            exit 0
        fi
    fi

    # Get current story and create story-specific log directory
    CURRENT_STORY=$(jq -r '.current_story // "unknown"' "$STATUS_FILE" 2>/dev/null)
    if [ "$CURRENT_STORY" != "null" ] && [ "$CURRENT_STORY" != "unknown" ] && [ -n "$CURRENT_STORY" ]; then
        STORY_LOG_DIR="$LOGS_DIR/$CURRENT_STORY"
        mkdir -p "$STORY_LOG_DIR"

        # Count existing attempts for this story
        ATTEMPT_COUNT=$(ls -1 "$STORY_LOG_DIR"/attempt-*.log 2>/dev/null | wc -l | tr -d ' ')
        ATTEMPT_NUM=$((ATTEMPT_COUNT + 1))

        # Check for repeated errors (3+ attempts on same story)
        LAST_ERROR_STORY=$(jq -r '.last_error_story // ""' "$STATUS_FILE" 2>/dev/null)
        ERROR_COUNT=$(jq -r '.error_count // 0' "$STATUS_FILE" 2>/dev/null)

        if [ "$LAST_ERROR_STORY" = "$CURRENT_STORY" ] && [ "$ERROR_COUNT" -ge 3 ]; then
            block_drone "Repeated errors on story $CURRENT_STORY (${ERROR_COUNT} attempts)" "$CURRENT_STORY"
            exit 0
        fi
    else
        STORY_LOG_DIR="$LOGS_DIR"
        ATTEMPT_NUM=$i
    fi

    # Prepare log file for this attempt
    ATTEMPT_LOG="$STORY_LOG_DIR/attempt-$ATTEMPT_NUM.log"
    ATTEMPT_META="$STORY_LOG_DIR/attempt-$ATTEMPT_NUM-metadata.json"

    # Capture start time
    START_TIME=$(date -u +%Y-%m-%dT%H:%M:%SZ)
    START_EPOCH=$(date +%s)

    # Run Claude with tee to capture complete output
    cd "$WORKTREE"
    EXIT_CODE=0
    $CLAUDE_CMD --print -p "$(cat "$PROMPT_FILE")" \
        --model "$MODEL" \
        --allowedTools "Bash,Read,Write,Edit,Glob,Grep,TodoWrite" \
        2>&1 | tee -a "$ATTEMPT_LOG" >> "$LOG_FILE" || EXIT_CODE=$?

    # Capture end time and calculate duration
    END_TIME=$(date -u +%Y-%m-%dT%H:%M:%SZ)
    END_EPOCH=$(date +%s)
    DURATION=$((END_EPOCH - START_EPOCH))

    # Create metadata file
    cat > "$ATTEMPT_META" <<META_EOF
{
  "story": "$CURRENT_STORY",
  "attempt": $ATTEMPT_NUM,
  "started": "$START_TIME",
  "completed": "$END_TIME",
  "duration_seconds": $DURATION,
  "model": "$MODEL",
  "exit_code": $EXIT_CODE,
  "iteration": $i
}
META_EOF

    # Track errors for blocking logic
    if [ $EXIT_CODE -ne 0 ]; then
        if [ "$LAST_ERROR_STORY" = "$CURRENT_STORY" ]; then
            # Increment error count for same story
            jq --argjson count "$((ERROR_COUNT + 1))" '.error_count = $count' "$STATUS_FILE" > /tmp/status.tmp && mv /tmp/status.tmp "$STATUS_FILE"
        else
            # Reset error count for new story
            jq --arg story "$CURRENT_STORY" '.error_count = 1 | .last_error_story = $story' "$STATUS_FILE" > /tmp/status.tmp && mv /tmp/status.tmp "$STATUS_FILE"
        fi
    else
        # Success - reset error tracking
        jq '.error_count = 0 | .last_error_story = null' "$STATUS_FILE" > /tmp/status.tmp && mv /tmp/status.tmp "$STATUS_FILE"
    fi

    echo "Iteration $i complete. Checking status..." >> "$LOG_FILE"
    sleep 2
done

echo "" >> "$LOG_FILE"
echo "═══════════════════════════════════════════════════════" >> "$LOG_FILE"
echo "  Drone reached max iterations ($MAX_ITERATIONS)" >> "$LOG_FILE"
echo "═══════════════════════════════════════════════════════" >> "$LOG_FILE"

# 🔔 Notification: Drone paused (reached max iterations)
COMPLETED=$(jq -r '.completed | length // 0' "$STATUS_FILE" 2>/dev/null)
send_notification "⏸️ Hive - Drone Paused" "$DRONE_NAME: $COMPLETED/$TOTAL (max iterations reached)"

# Mark as paused, not error
jq --arg ts "$(date -u +%Y-%m-%dT%H:%M:%SZ)" '.updated = $ts' "$STATUS_FILE" > /tmp/status.tmp && mv /tmp/status.tmp "$STATUS_FILE"
LAUNCHER_EOF

    chmod +x "$launcher_script"

    # Get Claude command from profile
    local claude_cmd=$(get_claude_command "$profile")

    # Launch the loop in background with nohup
    nohup "$launcher_script" "$drone_status_dir" "$prompt_file" "$model" "$iterations" "$workdir" "$drone_name" "$claude_cmd" > /dev/null 2>&1 &

    local pid=$!
    echo "$pid" > "$drone_status_dir/.pid"

    print_success "Drone $drone_name launched! (PID: $pid)"
    print_info "Log: $log_file"
    print_info "Status: $drone_status_file"
    print_info "Max iterations: $iterations (each iteration = full Claude session)"
    echo ""
    print_info "Monitor with: hive status"
    print_info "View logs: hive logs $drone_name"
    print_info "Stop drone: hive kill $drone_name"
}

# ============================================================================
# Auto-Clean Function
# ============================================================================

# Check for completed drones that have been inactive for more than INACTIVE_THRESHOLD
# and offer to clean them up
check_inactive_drones() {
    local inactive_drones=()
    local now=$(date +%s)

    if [ -d "$DRONES_DIR" ]; then
        for drone_dir in "$DRONES_DIR"/*/; do
            [ -d "$drone_dir" ] || continue

            local status_file="$drone_dir/status.json"
            [ -f "$status_file" ] || continue

            local drone_name=$(basename "$drone_dir")
            local status=$(jq -r '.status // "unknown"' "$status_file")
            local pid_file="$drone_dir/.pid"
            local running="no"

            # Check if drone is running
            if [ -f "$pid_file" ]; then
                local pid=$(cat "$pid_file")
                if ps -p "$pid" > /dev/null 2>&1; then
                    running="yes"
                fi
            fi

            # Only consider completed drones that are not running
            if [ "$status" = "completed" ] && [ "$running" = "no" ]; then
                # Check last modification time of status file
                local mtime
                if [[ "$OSTYPE" == "darwin"* ]]; then
                    mtime=$(stat -f %m "$status_file" 2>/dev/null || echo 0)
                else
                    mtime=$(stat -c %Y "$status_file" 2>/dev/null || echo 0)
                fi

                local age=$((now - mtime))

                if [ $age -gt $INACTIVE_THRESHOLD ]; then
                    local age_mins=$((age / 60))
                    inactive_drones+=("$drone_name:$age_mins")
                fi
            fi
        done
    fi

    # If we found inactive drones, offer to clean them
    if [ ${#inactive_drones[@]} -gt 0 ]; then
        echo ""
        echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
        echo -e "${YELLOW}🧹 Inactive completed drones detected:${NC}"
        echo ""

        for entry in "${inactive_drones[@]}"; do
            local name="${entry%%:*}"
            local mins="${entry##*:}"
            echo -e "   • ${CYAN}$name${NC} (completed ${mins} minutes ago)"
        done

        echo ""
        # Only prompt if running interactively
        if [ -t 0 ]; then
            read -p "Clean up these drones? [y/N] " -n 1 -r
            echo
        else
            # Non-interactive: show message but don't clean automatically
            echo -e "   Run ${CYAN}hive clean <name>${NC} to remove them."
            return
        fi

        if [[ $REPLY =~ ^[Yy]$ ]]; then
            for entry in "${inactive_drones[@]}"; do
                local name="${entry%%:*}"
                echo ""
                print_info "Cleaning $name..."
                cmd_clean -f "$name"
            done
            echo ""
            print_success "All inactive drones cleaned up!"
        fi
    fi
}

# ============================================================================
# Status Command
# ============================================================================

cmd_status() {
    check_git_repo

    local follow=false
    local interactive=false
    local interval=3

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -f|--follow) follow=true; shift ;;
            -i|--interactive) interactive=true; shift ;;
            --interval) interval="$2"; shift 2 ;;
            *) shift ;;
        esac
    done

    if [ "$interactive" = true ]; then
        cmd_status_interactive
        return
    fi

    if [ "$follow" = true ]; then
        # Follow mode - continuous dashboard (notifications handled by drone itself)
        trap 'tput cnorm; echo; exit 0' INT TERM
        tput civis  # Hide cursor

        while true; do
            clear
            render_status_dashboard
            sleep "$interval"
        done
    else
        # One-shot mode
        render_status_dashboard
        check_inactive_drones
    fi
}

render_status_dashboard() {
    local now_epoch=$(date "+%s")
    local dim='\033[2m'
    local bold='\033[1m'

    echo ""
    echo -e "${YELLOW}${bold}  👑 hive${NC} v${VERSION}  ${dim}$(date '+%H:%M:%S')${NC}"
    echo ""

    local found_drones=0

    if [ -d "$DRONES_DIR" ]; then
        for drone_dir in "$DRONES_DIR"/*/; do
            [ -d "$drone_dir" ] || continue
            local status_file="$drone_dir/status.json"
            [ -f "$status_file" ] || continue

            found_drones=$((found_drones + 1))

            local drone_name=$(basename "$drone_dir")
            local prd_file=$(jq -r '.prd // ""' "$status_file")
            local status=$(jq -r '.status // "unknown"' "$status_file")
            local current=$(jq -r '.current_story // ""' "$status_file")
            local completed_json=$(jq -r '.completed // []' "$status_file")
            local completed_count=$(echo "$completed_json" | jq 'length')
            local total=$(jq -r '.total // 0' "$status_file")
            local started=$(jq -r '.started // ""' "$status_file")
            local updated=$(jq -r '.updated // ""' "$status_file")
            local pid_file="$drone_dir/.pid"
            local running="no"
            local elapsed=""

            # Check if running
            if [ -f "$pid_file" ]; then
                local pid=$(cat "$pid_file" 2>/dev/null)
                ps -p "$pid" > /dev/null 2>&1 && running="yes"
            fi

            # Calculate elapsed time (UTC)
            if [ -n "$started" ] && [ "$started" != "null" ]; then
                local start_epoch
                if [[ "$OSTYPE" == "darwin"* ]]; then
                    start_epoch=$(date -j -u -f "%Y-%m-%dT%H:%M:%SZ" "$started" "+%s" 2>/dev/null || echo 0)
                else
                    start_epoch=$(date -u -d "$started" "+%s" 2>/dev/null || echo 0)
                fi
                if [ "$start_epoch" -gt 0 ]; then
                    local diff=$((now_epoch - start_epoch))
                    local hours=$((diff / 3600))
                    local mins=$(((diff % 3600) / 60))
                    if [ $hours -gt 0 ]; then
                        elapsed="${hours}h${mins}m"
                    else
                        elapsed="${mins}m"
                    fi
                fi
            fi

            # Status indicator
            local status_icon=""
            local status_color=""
            case "$status" in
                "in_progress"|"starting"|"resuming")
                    if [ "$running" = "yes" ]; then
                        status_icon="●"
                        status_color="${GREEN}"
                    else
                        status_icon="○"
                        status_color="${YELLOW}"
                    fi
                    ;;
                "completed") status_icon="✓"; status_color="${GREEN}" ;;
                "blocked") status_icon="⚠"; status_color="${RED}" ;;
                "error") status_icon="✗"; status_color="${RED}" ;;
                *) status_icon="?"; status_color="${NC}" ;;
            esac

            # Drone header
            echo -e "  ${status_color}${status_icon}${NC} ${YELLOW}${bold}🐝 ${drone_name}${NC}  ${dim}${elapsed}${NC}"

            # Progress bar
            local bar_width=40
            local filled=$((total > 0 ? completed_count * bar_width / total : 0))
            local empty=$((bar_width - filled))
            local bar="${GREEN}"
            for ((i=0; i<filled; i++)); do bar+="━"; done
            bar+="${NC}${dim}"
            for ((i=0; i<empty; i++)); do bar+="─"; done
            bar+="${NC}"
            echo -e "    ${bar} ${GREEN}${completed_count}${NC}/${total}"
            echo ""

            # Load PRD stories and story_times
            local prd_path="$PRDS_DIR/$prd_file"
            local story_times_json=$(jq -r '.story_times // {}' "$status_file")
            if [ -f "$prd_path" ]; then
                local stories=$(jq -c '.stories[]' "$prd_path" 2>/dev/null)
                while IFS= read -r story; do
                    local story_id=$(echo "$story" | jq -r '.id')
                    local story_title=$(echo "$story" | jq -r '.title')

                    # Check if completed
                    local is_completed=$(echo "$completed_json" | jq --arg id "$story_id" 'index($id) != null')
                    local is_current=false
                    [ "$story_id" = "$current" ] && is_current=true

                    # Calculate story duration
                    local story_duration=""
                    local story_started=$(echo "$story_times_json" | jq -r --arg id "$story_id" '.[$id].started // empty')
                    local story_completed=$(echo "$story_times_json" | jq -r --arg id "$story_id" '.[$id].completed // empty')
                    if [ -n "$story_started" ]; then
                        local start_ts end_ts
                        if [[ "$OSTYPE" == "darwin"* ]]; then
                            start_ts=$(date -j -u -f "%Y-%m-%dT%H:%M:%SZ" "$story_started" "+%s" 2>/dev/null || echo 0)
                            if [ -n "$story_completed" ]; then
                                end_ts=$(date -j -u -f "%Y-%m-%dT%H:%M:%SZ" "$story_completed" "+%s" 2>/dev/null || echo 0)
                            else
                                end_ts=$now_epoch
                            fi
                        else
                            start_ts=$(date -u -d "$story_started" "+%s" 2>/dev/null || echo 0)
                            if [ -n "$story_completed" ]; then
                                end_ts=$(date -u -d "$story_completed" "+%s" 2>/dev/null || echo 0)
                            else
                                end_ts=$now_epoch
                            fi
                        fi
                        if [ "$start_ts" -gt 0 ] && [ "$end_ts" -gt 0 ]; then
                            local sdiff=$((end_ts - start_ts))
                            local smins=$((sdiff / 60))
                            local ssecs=$((sdiff % 60))
                            if [ $smins -gt 0 ]; then
                                story_duration="${smins}m${ssecs}s"
                            else
                                story_duration="${ssecs}s"
                            fi
                        fi
                    fi

                    # Display story with duration
                    if [ "$is_completed" = "true" ]; then
                        local dur_str=""
                        [ -n "$story_duration" ] && dur_str=" ${dim}(${story_duration})${NC}"
                        echo -e "    ${GREEN}✓${NC} ${dim}${story_id}${NC} ${dim}${story_title}${NC}${dur_str}"
                    elif [ "$is_current" = "true" ]; then
                        local dur_str=""
                        [ -n "$story_duration" ] && dur_str=" ${dim}(${story_duration})${NC}"
                        echo -e "    ${YELLOW}▸${NC} ${YELLOW}${story_id}${NC} ${story_title}${dur_str}"
                    else
                        echo -e "    ${dim}○${NC} ${dim}${story_id}${NC} ${dim}${story_title}${NC}"
                    fi
                done <<< "$stories"
            fi

            echo ""
        done
    fi

    if [ $found_drones -eq 0 ]; then
        echo -e "  ${dim}No active drones${NC}"
        echo ""
        echo -e "  Launch one with: ${YELLOW}hive start <prd-name>${NC}"
        echo ""
    fi

    echo -e "  ${dim}─────────────────────────────────────────────────${NC}"
    echo -e "  ${dim}logs${NC} ${CYAN}<drone>${NC}  ${dim}│${NC}  ${dim}kill${NC} ${CYAN}<drone>${NC}  ${dim}│${NC}  ${dim}clean${NC} ${CYAN}<drone>${NC}"
    echo ""
}

# ============================================================================
# Interactive Status (TUI with gum)
# ============================================================================

cmd_status_interactive() {
    check_gum

    while true; do
        clear

        # Render full dashboard (same as normal status)
        render_status_dashboard_interactive

        # Collect drone names for selection
        local drones=()
        if [ -d "$DRONES_DIR" ]; then
            for drone_dir in "$DRONES_DIR"/*/; do
                [ -d "$drone_dir" ] || continue
                [ -f "$drone_dir/status.json" ] || continue
                drones+=("$(basename "$drone_dir")")
            done
        fi

        if [ ${#drones[@]} -eq 0 ]; then
            read -p "Press Enter to exit..." -r
            break
        fi

        # Build selection options
        local options=()
        for drone in "${drones[@]}"; do
            options+=("🐝 $drone")
        done
        options+=("⟳ Auto-refresh (30s)")
        options+=("← Quit")

        # Use gum to select
        local selection
        selection=$(printf '%s\n' "${options[@]}" | gum choose --cursor "▸ " --cursor.foreground="220" --height=5)

        [ -z "$selection" ] && break
        [[ "$selection" == "← Quit" ]] && break

        # Auto-refresh mode
        if [[ "$selection" == "⟳ Auto-refresh (30s)" ]]; then
            cmd_status_auto_refresh
            continue
        fi

        # Extract drone name
        local selected_drone="${selection#🐝 }"
        [ -z "$selected_drone" ] && continue

        # Show drone actions menu
        cmd_status_drone_menu "$selected_drone"
    done
}

cmd_status_auto_refresh() {
    tput civis  # Hide cursor
    trap 'tput cnorm; return' INT TERM

    while true; do
        clear
        render_status_dashboard_interactive
        echo -e "  \033[2mAuto-refresh every 30s │ Press any key to interact\033[0m"
        echo ""

        # Wait for 30 seconds or keypress
        if read -t 30 -n 1 -s; then
            tput cnorm  # Show cursor
            return
        fi
    done
}

render_status_dashboard_interactive() {
    local now_epoch=$(date "+%s")
    local dim='\033[2m'
    local bold='\033[1m'

    echo ""
    echo -e "${YELLOW}${bold}  👑 hive${NC} v${VERSION}  ${dim}$(date '+%H:%M:%S')${NC}"
    echo ""

    local found_drones=0

    if [ -d "$DRONES_DIR" ]; then
        for drone_dir in "$DRONES_DIR"/*/; do
            [ -d "$drone_dir" ] || continue
            local status_file="$drone_dir/status.json"
            [ -f "$status_file" ] || continue

            found_drones=$((found_drones + 1))

            local drone_name=$(basename "$drone_dir")
            local prd_file=$(jq -r '.prd // ""' "$status_file")
            local status=$(jq -r '.status // "unknown"' "$status_file")
            local current=$(jq -r '.current_story // ""' "$status_file")
            local completed_json=$(jq -r '.completed // []' "$status_file")
            local completed_count=$(echo "$completed_json" | jq 'length')
            local total=$(jq -r '.total // 0' "$status_file")
            local started=$(jq -r '.started // ""' "$status_file")
            local pid_file="$drone_dir/.pid"
            local running="no"
            local elapsed=""

            # Check if running
            if [ -f "$pid_file" ]; then
                local pid=$(cat "$pid_file" 2>/dev/null)
                ps -p "$pid" > /dev/null 2>&1 && running="yes"
            fi

            # Calculate elapsed time (UTC)
            if [ -n "$started" ] && [ "$started" != "null" ]; then
                local start_epoch
                if [[ "$OSTYPE" == "darwin"* ]]; then
                    start_epoch=$(date -j -u -f "%Y-%m-%dT%H:%M:%SZ" "$started" "+%s" 2>/dev/null || echo 0)
                else
                    start_epoch=$(date -u -d "$started" "+%s" 2>/dev/null || echo 0)
                fi
                if [ "$start_epoch" -gt 0 ]; then
                    local diff=$((now_epoch - start_epoch))
                    local hours=$((diff / 3600))
                    local mins=$(((diff % 3600) / 60))
                    if [ $hours -gt 0 ]; then
                        elapsed="${hours}h${mins}m"
                    else
                        elapsed="${mins}m"
                    fi
                fi
            fi

            # Status indicator
            local status_icon=""
            local status_color=""
            case "$status" in
                "in_progress"|"starting"|"resuming")
                    if [ "$running" = "yes" ]; then
                        status_icon="●"
                        status_color="${GREEN}"
                    else
                        status_icon="○"
                        status_color="${YELLOW}"
                    fi
                    ;;
                "completed") status_icon="✓"; status_color="${GREEN}" ;;
                "blocked") status_icon="⚠"; status_color="${RED}" ;;
                "error") status_icon="✗"; status_color="${RED}" ;;
                *) status_icon="?"; status_color="${NC}" ;;
            esac

            # Drone header
            echo -e "  ${status_color}${status_icon}${NC} ${YELLOW}${bold}🐝 ${drone_name}${NC}  ${dim}${elapsed}${NC}"

            # Progress bar
            local bar_width=40
            local filled=$((total > 0 ? completed_count * bar_width / total : 0))
            local empty=$((bar_width - filled))
            local bar="${GREEN}"
            for ((i=0; i<filled; i++)); do bar+="━"; done
            bar+="${NC}${dim}"
            for ((i=0; i<empty; i++)); do bar+="─"; done
            bar+="${NC}"
            echo -e "    ${bar} ${GREEN}${completed_count}${NC}/${total}"
            echo ""

            # Load PRD stories and story_times
            local prd_path="$PRDS_DIR/$prd_file"
            local story_times_json=$(jq -r '.story_times // {}' "$status_file")
            if [ -f "$prd_path" ]; then
                local stories=$(jq -c '.stories[]' "$prd_path" 2>/dev/null)
                while IFS= read -r story; do
                    local story_id=$(echo "$story" | jq -r '.id')
                    local story_title=$(echo "$story" | jq -r '.title')

                    # Check if completed
                    local is_completed=$(echo "$completed_json" | jq --arg id "$story_id" 'index($id) != null')
                    local is_current=false
                    [ "$story_id" = "$current" ] && is_current=true

                    # Calculate story duration
                    local story_duration=""
                    local story_started=$(echo "$story_times_json" | jq -r --arg id "$story_id" '.[$id].started // empty')
                    local story_completed=$(echo "$story_times_json" | jq -r --arg id "$story_id" '.[$id].completed // empty')
                    if [ -n "$story_started" ]; then
                        local start_ts end_ts
                        if [[ "$OSTYPE" == "darwin"* ]]; then
                            start_ts=$(date -j -u -f "%Y-%m-%dT%H:%M:%SZ" "$story_started" "+%s" 2>/dev/null || echo 0)
                            if [ -n "$story_completed" ]; then
                                end_ts=$(date -j -u -f "%Y-%m-%dT%H:%M:%SZ" "$story_completed" "+%s" 2>/dev/null || echo 0)
                            else
                                end_ts=$now_epoch
                            fi
                        else
                            start_ts=$(date -u -d "$story_started" "+%s" 2>/dev/null || echo 0)
                            if [ -n "$story_completed" ]; then
                                end_ts=$(date -u -d "$story_completed" "+%s" 2>/dev/null || echo 0)
                            else
                                end_ts=$now_epoch
                            fi
                        fi
                        if [ "$start_ts" -gt 0 ] && [ "$end_ts" -gt 0 ]; then
                            local sdiff=$((end_ts - start_ts))
                            local smins=$((sdiff / 60))
                            local ssecs=$((sdiff % 60))
                            if [ $smins -gt 0 ]; then
                                story_duration="${smins}m${ssecs}s"
                            else
                                story_duration="${ssecs}s"
                            fi
                        fi
                    fi

                    # Display story with duration
                    if [ "$is_completed" = "true" ]; then
                        local dur_str=""
                        [ -n "$story_duration" ] && dur_str=" ${dim}(${story_duration})${NC}"
                        echo -e "    ${GREEN}✓${NC} ${dim}${story_id}${NC} ${dim}${story_title}${NC}${dur_str}"
                    elif [ "$is_current" = "true" ]; then
                        local dur_str=""
                        [ -n "$story_duration" ] && dur_str=" ${dim}(${story_duration})${NC}"
                        echo -e "    ${YELLOW}▸${NC} ${YELLOW}${story_id}${NC} ${story_title}${dur_str}"
                    else
                        echo -e "    ${dim}○${NC} ${dim}${story_id}${NC} ${dim}${story_title}${NC}"
                    fi
                done <<< "$stories"
            fi

            echo ""
        done
    fi

    if [ $found_drones -eq 0 ]; then
        echo -e "  ${dim}No active drones${NC}"
        echo ""
        echo -e "  Launch one with: ${YELLOW}hive start <prd-name>${NC}"
    fi

    echo ""
    echo -e "  ${dim}─────────────────────────────────────────────────${NC}"
    echo ""
}

cmd_status_drone_menu() {
    local drone_name="$1"
    local drone_dir="$DRONES_DIR/$drone_name"
    local status_file="$drone_dir/status.json"

    while true; do
        clear
        echo ""
        echo -e "${YELLOW}\033[1m  🐝 $drone_name\033[0m"
        echo ""

        # Show drone info
        local status=$(jq -r '.status // "unknown"' "$status_file")
        local current=$(jq -r '.current_story // ""' "$status_file")
        local completed_json=$(jq -r '.completed // []' "$status_file")
        local completed_count=$(echo "$completed_json" | jq 'length')
        local total=$(jq -r '.total // 0' "$status_file")
        local prd_file=$(jq -r '.prd // ""' "$status_file")

        echo -e "  Status: ${CYAN}$status${NC}"
        echo -e "  Progress: ${GREEN}$completed_count${NC}/$total"
        [ -n "$current" ] && [ "$current" != "null" ] && echo -e "  Current: ${YELLOW}$current${NC}"
        echo ""

        # Show stories
        local prd_path="$PRDS_DIR/$prd_file"
        if [ -f "$prd_path" ]; then
            local stories=$(jq -c '.stories[]' "$prd_path" 2>/dev/null)
            while IFS= read -r story; do
                local story_id=$(echo "$story" | jq -r '.id')
                local story_title=$(echo "$story" | jq -r '.title')
                local is_completed=$(echo "$completed_json" | jq --arg id "$story_id" 'index($id) != null')
                local is_current=false
                [ "$story_id" = "$current" ] && is_current=true

                if [ "$is_completed" = "true" ]; then
                    echo -e "  ${GREEN}✓${NC} \033[2m$story_id\033[0m \033[2m$story_title\033[0m"
                elif [ "$is_current" = "true" ]; then
                    echo -e "  ${YELLOW}▸${NC} ${YELLOW}$story_id${NC} $story_title"
                else
                    echo -e "  \033[2m○ $story_id $story_title\033[0m"
                fi
            done <<< "$stories"
        fi
        echo ""

        # Action menu
        local action
        action=$(gum choose --header "Action:" --cursor "▸ " --cursor.foreground="220" \
            "📜 View logs" \
            "📋 View raw logs" \
            "📊 View story logs" \
            "🛑 Kill drone" \
            "🗑  Clean drone" \
            "← Back")

        case "$action" in
            "📜 View logs")
                local log_file="$drone_dir/activity.log"
                [ ! -f "$log_file" ] && log_file="$drone_dir/drone.log"
                if [ -f "$log_file" ]; then
                    gum pager < "$log_file"
                else
                    gum style --foreground 1 "No logs found"
                    sleep 1
                fi
                ;;
            "📋 View raw logs")
                local log_file="$drone_dir/drone.log"
                if [ -f "$log_file" ]; then
                    gum pager < "$log_file"
                else
                    gum style --foreground 1 "No raw logs found"
                    sleep 1
                fi
                ;;
            "📊 View story logs")
                # Show story-specific logs
                local logs_dir="$drone_dir/logs"
                if [ -d "$logs_dir" ]; then
                    # Get list of stories with logs
                    local story_dirs=$(find "$logs_dir" -mindepth 1 -maxdepth 1 -type d -exec basename {} \;  2>/dev/null | sort)
                    if [ -n "$story_dirs" ]; then
                        local selected_story
                        selected_story=$(echo "$story_dirs" | gum choose --header "Select story:" --cursor "▸ " --cursor.foreground="220")
                        if [ -n "$selected_story" ]; then
                            local story_log_dir="$logs_dir/$selected_story"
                            # Get list of attempts
                            local attempts=$(ls -1 "$story_log_dir"/attempt-*.log 2>/dev/null | sort -V)
                            if [ -n "$attempts" ]; then
                                local selected_attempt
                                selected_attempt=$(echo "$attempts" | xargs -n 1 basename | gum choose --header "Select attempt:" --cursor "▸ " --cursor.foreground="220")
                                if [ -n "$selected_attempt" ]; then
                                    local attempt_log="$story_log_dir/$selected_attempt"
                                    local attempt_meta="${attempt_log%.log}-metadata.json"

                                    # Show metadata header if exists
                                    if [ -f "$attempt_meta" ]; then
                                        clear
                                        echo ""
                                        echo -e "${CYAN}═══════════════════════════════════════════════${NC}"
                                        echo -e "${YELLOW}Story:${NC} $selected_story"
                                        echo -e "${YELLOW}Attempt:${NC} $(basename "$selected_attempt" | sed 's/attempt-\(.*\)\.log/\1/')"
                                        if command -v jq &>/dev/null; then
                                            local started=$(jq -r '.started // ""' "$attempt_meta")
                                            local completed=$(jq -r '.completed // ""' "$attempt_meta")
                                            local duration=$(jq -r '.duration_seconds // 0' "$attempt_meta")
                                            local exit_code=$(jq -r '.exit_code // 0' "$attempt_meta")
                                            echo -e "${YELLOW}Duration:${NC} ${duration}s"
                                            echo -e "${YELLOW}Exit code:${NC} $exit_code"
                                        fi
                                        echo -e "${CYAN}═══════════════════════════════════════════════${NC}"
                                        echo ""
                                        echo "Press any key to view log..."
                                        read -n 1 -s
                                    fi

                                    if command -v bat &>/dev/null; then
                                        bat --paging=always "$attempt_log"
                                    else
                                        gum pager < "$attempt_log"
                                    fi
                                fi
                            else
                                gum style --foreground 1 "No attempt logs found for $selected_story"
                                sleep 1
                            fi
                        fi
                    else
                        gum style --foreground 1 "No story logs found"
                        sleep 1
                    fi
                else
                    gum style --foreground 1 "No logs directory found"
                    sleep 1
                fi
                ;;
            "🛑 Kill drone")
                if gum confirm "Kill drone $drone_name?"; then
                    cmd_kill "$drone_name"
                    sleep 1
                fi
                ;;
            "🗑  Clean drone")
                if gum confirm "Remove drone $drone_name and its worktree?"; then
                    cmd_clean "$drone_name"
                    sleep 1
                    break
                fi
                ;;
            "← Back"|"")
                break
                ;;
        esac
    done
}

# ============================================================================
# Logs Command
# ============================================================================

cmd_logs() {
    local drone_name=""
    local follow=false
    local raw=false

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -f|--follow) follow=true; shift ;;
            --raw) raw=true; shift ;;
            *) drone_name="$1"; shift ;;
        esac
    done

    if [ -z "$drone_name" ]; then
        print_error "Drone name required"
        echo "Usage: hive logs [-f] [--raw] <drone-name>"
        echo ""
        echo "Options:"
        echo "  -f, --follow    Follow log output"
        echo "  --raw           Show raw drone.log instead of activity.log"
        exit 1
    fi

    local drone_dir="$DRONES_DIR/$drone_name"
    local activity_log="$drone_dir/activity.log"
    local raw_log="$drone_dir/drone.log"
    local status_file="$drone_dir/status.json"

    # Choose which log to show
    local log_file="$activity_log"
    if [ "$raw" = true ] || [ ! -f "$activity_log" ]; then
        log_file="$raw_log"
    fi

    if [ ! -f "$log_file" ]; then
        print_error "Log file not found: $log_file"
        exit 1
    fi

    # Show header
    echo -e "${CYAN}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║${NC}  ${YELLOW}🐝 Drone: $drone_name${NC}"

    # Show status if available
    if [ -f "$status_file" ]; then
        local status=$(jq -r '.status // "unknown"' "$status_file")
        local completed=$(jq -r '.completed | length // 0' "$status_file")
        local total=$(jq -r '.total // "?"' "$status_file")
        local current=$(jq -r '.current_story // "none"' "$status_file")
        echo -e "${CYAN}║${NC}  Status: $status | Progress: ${GREEN}$completed${NC}/$total | Current: $current"
    fi
    echo -e "${CYAN}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo ""

    # Show logs
    if [ "$follow" = true ]; then
        echo -e "${BLUE}Following $log_file (Ctrl+C to stop)${NC}"
        echo ""
        tail -f "$log_file"
    else
        cat "$log_file"
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

    if [ -z "$drone_name" ]; then
        print_error "Drone name required"
        echo "Usage: hive clean <drone-name>"
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
    if [ -z "$worktree_path" ]; then
        local worktree_base=$(get_worktree_base)
        worktree_path="$worktree_base/$project_name/$drone_name"
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
# Unblock Command
# ============================================================================

cmd_unblock() {
    local drone_name="$1"

    if [ -z "$drone_name" ]; then
        print_error "Drone name required"
        echo "Usage: hive unblock <drone-name>"
        exit 1
    fi

    check_git_repo

    local drone_status_dir="$DRONES_DIR/$drone_name"
    local drone_status_file="$drone_status_dir/status.json"
    local blocked_file="$drone_status_dir/blocked.md"

    if [ ! -f "$drone_status_file" ]; then
        print_error "Drone not found: $drone_name"
        exit 1
    fi

    local status=$(jq -r '.status // ""' "$drone_status_file")
    if [ "$status" != "blocked" ]; then
        print_warning "Drone $drone_name is not blocked (status: $status)"
        echo "Use 'hive start --resume $drone_name' to resume a non-blocked drone."
        exit 0
    fi

    echo ""
    print_info "Drone $drone_name is blocked. Here's what happened:"
    echo ""

    # Show blocked.md if exists
    if [ -f "$blocked_file" ]; then
        if command -v bat &>/dev/null; then
            bat --style=plain --color=always "$blocked_file"
        else
            cat "$blocked_file"
        fi
        echo ""
    fi

    # Get PRD file
    local prd_file=$(jq -r '.prd // ""' "$drone_status_file")
    if [ -n "$prd_file" ] && [ -f "$PRDS_DIR/$prd_file" ]; then
        echo ""
        print_info "PRD file: $PRDS_DIR/$prd_file"
        echo ""

        # Prompt to edit PRD
        if [ -t 0 ]; then
            read -p "Would you like to edit the PRD to fix the issue? [Y/n] " -n 1 -r
            echo ""
            if [[ ! $REPLY =~ ^[Nn]$ ]]; then
                # Open PRD in editor
                ${EDITOR:-vi} "$PRDS_DIR/$prd_file"
                echo ""
                print_success "PRD updated"
            fi
        fi

        echo ""
        # Prompt to resume
        if [ -t 0 ]; then
            read -p "Ready to unblock and resume the drone? [y/N] " -n 1 -r
            echo ""
            if [[ $REPLY =~ ^[Yy]$ ]]; then
                print_info "Resuming drone $drone_name..."
                echo ""
                # Call hive start with --resume
                cmd_run --prd "$PRDS_DIR/$prd_file" --name "$drone_name" --resume
            else
                print_info "Drone remains blocked. Run 'hive start --resume $drone_name' when ready."
            fi
        else
            print_info "Run 'hive start --resume $drone_name' when ready to unblock and resume."
        fi
    else
        print_error "PRD file not found. Cannot resume without PRD."
        exit 1
    fi
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
            local status_display=""
            case "$status" in
                "completed") status_icon="✓" ;;
                "blocked") status_icon="⚠"; status_display="${RED}BLOCKED${NC}" ;;
                "error") status_icon="✗" ;;
                *) status_icon="" ;;
            esac

            if [ -n "$status_display" ]; then
                echo -e "  🐝 ${CYAN}$name${NC} $status_icon $status_display ($completed/$total)"
            else
                echo -e "  🐝 ${CYAN}$name${NC} $status_icon ($completed/$total)"
            fi
            count=$((count + 1))
        done
    fi

    [ $count -eq 0 ] && echo "  No active drones"
    echo ""
}

# ============================================================================
# Profile Command
# ============================================================================

cmd_profile() {
    local subcommand="${1:-list}"
    shift || true

    case "$subcommand" in
        list|ls)
            init_global_config
            echo ""
            echo -e "${CYAN}Claude Profiles:${NC}"
            echo ""

            local default_profile=$(jq -r '.default_profile // "default"' "$HIVE_GLOBAL_CONFIG")
            local profiles=$(jq -r '.profiles | keys[]' "$HIVE_GLOBAL_CONFIG")

            while IFS= read -r profile; do
                local cmd=$(jq -r --arg p "$profile" '.profiles[$p].claude_command' "$HIVE_GLOBAL_CONFIG")
                local desc=$(jq -r --arg p "$profile" '.profiles[$p].description // ""' "$HIVE_GLOBAL_CONFIG")

                local default_mark=""
                [ "$profile" = "$default_profile" ] && default_mark=" ${GREEN}(default)${NC}"

                echo -e "  ${YELLOW}$profile${NC}$default_mark"
                echo -e "    Command: ${CYAN}$cmd${NC}"
                [ -n "$desc" ] && echo -e "    ${dim}$desc${NC}"
                echo ""
            done <<< "$profiles"
            ;;

        add)
            local name="$1"
            local command="$2"
            local description="${3:-}"

            if [ -z "$name" ] || [ -z "$command" ]; then
                print_error "Usage: hive profile add <name> <command> [description]"
                exit 1
            fi

            init_global_config

            local profile_json=$(jq -n --arg cmd "$command" --arg desc "$description" '{claude_command: $cmd, description: $desc}')
            jq --arg name "$name" --argjson profile "$profile_json" '.profiles[$name] = $profile' "$HIVE_GLOBAL_CONFIG" > /tmp/hive_config.tmp
            mv /tmp/hive_config.tmp "$HIVE_GLOBAL_CONFIG"

            print_success "Profile '$name' added"
            ;;

        set-default)
            local name="$1"

            if [ -z "$name" ]; then
                print_error "Usage: hive profile set-default <name>"
                exit 1
            fi

            init_global_config

            # Check if profile exists
            if ! jq -e --arg name "$name" '.profiles[$name]' "$HIVE_GLOBAL_CONFIG" > /dev/null 2>&1; then
                print_error "Profile '$name' not found"
                exit 1
            fi

            jq --arg name "$name" '.default_profile = $name' "$HIVE_GLOBAL_CONFIG" > /tmp/hive_config.tmp
            mv /tmp/hive_config.tmp "$HIVE_GLOBAL_CONFIG"

            print_success "Default profile set to '$name'"
            ;;

        rm|remove)
            local name="$1"

            if [ -z "$name" ]; then
                print_error "Usage: hive profile rm <name>"
                exit 1
            fi

            if [ "$name" = "default" ]; then
                print_error "Cannot remove 'default' profile"
                exit 1
            fi

            init_global_config

            jq --arg name "$name" 'del(.profiles[$name])' "$HIVE_GLOBAL_CONFIG" > /tmp/hive_config.tmp
            mv /tmp/hive_config.tmp "$HIVE_GLOBAL_CONFIG"

            print_success "Profile '$name' removed"
            ;;

        help|--help|-h)
            cat << EOF

${CYAN}hive profile${NC} - Manage Claude profiles

${YELLOW}Usage:${NC}
  hive profile list              List all profiles
  hive profile add <name> <cmd>  Add a new profile
  hive profile set-default <name> Set default profile
  hive profile rm <name>         Remove a profile

${YELLOW}Examples:${NC}
  hive profile add ml "claude-wrapper ml" "Bedrock (work)"
  hive profile add perso "claude-wrapper perso" "MAX API (personal)"
  hive profile set-default ml
  hive profile list
  hive profile rm perso

${YELLOW}Config file:${NC}
  ~/.config/hive/config.json

EOF
            ;;

        *)
            print_error "Unknown subcommand: $subcommand"
            echo "Try 'hive profile help'"
            exit 1
            ;;
    esac
}

# ============================================================================
# Update Command
# ============================================================================

cmd_update() {
    print_info "Checking for updates..."

    # Fetch latest version
    local remote_version
    remote_version=$(curl -sL --connect-timeout 5 --max-time 10 "$HIVE_REPO_URL/hive.sh" 2>/dev/null | grep '^VERSION=' | head -1 | cut -d'"' -f2)

    if [ -z "$remote_version" ]; then
        print_error "Could not fetch latest version. Check your internet connection."
        exit 1
    fi

    # Update cache
    mkdir -p "$HIVE_CACHE_DIR"
    echo "$remote_version" > "$HIVE_VERSION_CACHE"

    if [ "$VERSION" = "$remote_version" ]; then
        print_success "Hive is already up to date (v$VERSION)"
        exit 0
    fi

    if ! version_lt "$VERSION" "$remote_version"; then
        print_success "Hive is already up to date (v$VERSION)"
        exit 0
    fi

    echo ""
    echo -e "  Current version: ${YELLOW}$VERSION${NC}"
    echo -e "  Latest version:  ${GREEN}$remote_version${NC}"
    echo ""

    read -p "Update now? [Y/n] " -n 1 -r
    echo
    [[ $REPLY =~ ^[Nn]$ ]] && exit 0

    print_info "Updating Hive..."

    # Determine install directory (where current hive is)
    local install_dir
    install_dir=$(dirname "$(command -v hive 2>/dev/null || echo "$HOME/.local/bin/hive")")

    # Download new CLI
    print_info "Downloading CLI..."
    if curl -sL -o "$install_dir/hive.tmp" "$HIVE_REPO_URL/hive.sh"; then
        chmod +x "$install_dir/hive.tmp"
        mv "$install_dir/hive.tmp" "$install_dir/hive"
        print_success "CLI updated to v$remote_version"
    else
        print_error "Failed to download CLI"
        rm -f "$install_dir/hive.tmp"
        exit 1
    fi

    # Update skills for Claude Code
    if [ -d "$HOME/.claude/commands" ]; then
        print_info "Updating Claude Code skills..."
        local skills=(
            "hive:init"
            "hive:start"
            "hive:status"
            "hive:list"
            "hive:logs"
            "hive:kill"
            "hive:clean"
            "hive:profile"
            "hive:prd"
            "hive:statusline"
        )
        for skill in "${skills[@]}"; do
            curl -sL -o "$HOME/.claude/commands/$skill.md" "$HIVE_REPO_URL/commands/$skill.md" 2>/dev/null
        done
        print_success "Claude Code skills updated (${#skills[@]} skills)"
    fi

    # Update skills for Cursor
    if [ -d "$HOME/.cursor/commands" ]; then
        print_info "Updating Cursor commands..."
        for skill in "${skills[@]}"; do
            curl -sL -o "$HOME/.cursor/commands/$skill.md" "$HIVE_REPO_URL/commands/$skill.md" 2>/dev/null
        done
        print_success "Cursor commands updated"
    fi

    # Install optional dependencies
    install_optional_deps

    echo ""
    print_success "Hive updated to v$remote_version!"
    echo ""
    echo "Changelog: https://github.com/mbourmaud/hive/blob/main/CHANGELOG.md"
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
  ${GREEN}start${NC}    Launch a drone on a PRD file
  ${GREEN}status${NC}   Show status of all drones
  ${GREEN}list${NC}     List active drones
  ${GREEN}logs${NC}     View drone logs
  ${GREEN}kill${NC}     Stop a running drone
  ${GREEN}clean${NC}    Remove a drone and its worktree
  ${GREEN}unblock${NC}  Interactively unblock a blocked drone
  ${GREEN}profile${NC}  Manage Claude profiles (wrappers, API configs)
  ${GREEN}init${NC}     Initialize Hive in current repo
  ${GREEN}update${NC}   Update Hive to latest version
  ${GREEN}version${NC}  Show version
  ${GREEN}help${NC}     Show this help

${CYAN}Status Options:${NC}
  hive status              One-shot status display
  hive status -f           Follow mode (auto-refresh)
  hive status -i           Interactive TUI (requires gum)

${CYAN}Quick Start:${NC}
  hive start --prd prd-feature.json
  hive status -i
  hive logs my-feature
  hive kill my-feature

${CYAN}Examples:${NC}
  # Launch a drone on a PRD
  hive start --prd .hive/prds/security.json --name security

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

    # Check for updates (non-blocking, cached)
    # Skip for certain commands to avoid noise
    case "$command" in
        update|version|--version|-v|help|--help|-h) ;;
        *) check_for_updates ;;
    esac

    case "$command" in
        start)   cmd_run "$@" ;;
        run)     cmd_run "$@" ;;  # alias for backwards compat
        status)  cmd_status "$@" ;;
        list)    cmd_list "$@" ;;
        logs)    cmd_logs "$@" ;;
        kill)    cmd_kill "$@" ;;
        clean)   cmd_clean "$@" ;;
        unblock) cmd_unblock "$@" ;;
        profile) cmd_profile "$@" ;;
        init)    cmd_init "$@" ;;
        update)  cmd_update "$@" ;;
        version|--version|-v) cmd_version ;;
        help|--help|-h) cmd_help ;;
        *)
            print_error "Unknown command: $command"
            cmd_help
            exit 1
            ;;
    esac
}

main "$@"
