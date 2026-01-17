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
VERSION="1.1.1"

# Configuration
HIVE_DIR=".hive"
CONFIG_FILE="$HIVE_DIR/config.json"
PRDS_DIR="$HIVE_DIR/prds"
DRONES_DIR="$HIVE_DIR/drones"

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
# Notification Functions (cross-platform)
# ============================================================================

# Send a desktop notification (works on macOS, Linux, and Windows/WSL)
send_notification() {
    local title="$1"
    local message="$2"
    local sound="${3:-true}"  # Play sound by default

    # macOS
    if command -v osascript &>/dev/null; then
        local sound_param=""
        [ "$sound" = "true" ] && sound_param='sound name "Glass"'
        osascript -e "display notification \"$message\" with title \"$title\" $sound_param" 2>/dev/null || true
        return
    fi

    # Linux with notify-send (GNOME, KDE, etc.)
    if command -v notify-send &>/dev/null; then
        notify-send "$title" "$message" --icon=dialog-information 2>/dev/null || true
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

get_project_root() {
    git rev-parse --show-toplevel 2>/dev/null
}

get_project_name() {
    basename "$(get_project_root)"
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
# Start Command - Launch a drone on a PRD
# ============================================================================

show_start_usage() {
    cat << EOF
${CYAN}hive start${NC} - Launch a drone on a PRD file

${YELLOW}Usage:${NC}
  hive start --prd <file> [options]

${YELLOW}Required:${NC}
  --prd <file>        PRD JSON file to execute

${YELLOW}Options:${NC}
  --name <name>       Drone name (default: derived from PRD id)
  --base <branch>     Base branch (default: main)
  --iterations <n>    Max iterations (default: 15, each = full Claude session)
  --model <model>     Claude model (default: opus)
  --help, -h          Show this help

${YELLOW}Examples:${NC}
  hive start --prd prd-security.json
  hive start --prd .hive/prds/feature.json --name feature-auth
  hive start --prd prd.json --iterations 100 --model sonnet

${YELLOW}What it does:${NC}
  1. Creates branch hive/<name> from base
  2. Creates worktree at ~/Projects/{project}-{drone}/
  3. Symlinks .hive/ to worktree (shared state)
  4. Launches Claude agent in background
  5. Updates .hive/drones/<name>/status.json for tracking
EOF
}

cmd_run() {
    local prd_file=""
    local drone_name=""
    local base_branch="main"
    local iterations=15
    local model="opus"

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --prd) prd_file="$2"; shift 2 ;;
            --name) drone_name="$2"; shift 2 ;;
            --base) base_branch="$2"; shift 2 ;;
            --iterations) iterations="$2"; shift 2 ;;
            --model) model="$2"; shift 2 ;;
            --help|-h) show_start_usage; exit 0 ;;
            *) print_error "Unknown option: $1"; show_start_usage; exit 1 ;;
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
**ACTIVITY LOG**: $external_worktree/.hive/drones/$drone_name/activity.log
**BRANCH**: $branch_name

IMPORTANT: Toutes tes opérations doivent être dans le répertoire $external_worktree

## Ta mission

1. Lis le fichier PRD pour comprendre les stories à implémenter
2. Pour chaque story:
   - Implémente les changements demandés
   - Commit avec le message \"feat(<STORY-ID>): <description>\"
   - **OBLIGATOIRE**: Mets à jour status.json IMMÉDIATEMENT après chaque story
3. Log chaque action dans activity.log

## ⚠️ MISE À JOUR status.json - OBLIGATOIRE

Tu DOIS mettre à jour status.json à ces moments précis:

### 1. Au démarrage (après lecture du PRD)
\`\`\`bash
jq --arg ts \"\$(date -u +%Y-%m-%dT%H:%M:%SZ)\" '.status = \"in_progress\" | .updated = \$ts' $external_worktree/.hive/drones/$drone_name/status.json > /tmp/status.tmp && mv /tmp/status.tmp $external_worktree/.hive/drones/$drone_name/status.json
\`\`\`

### 2. Quand tu COMMENCES une story
\`\`\`bash
jq --arg story \"STORY-ID\" --arg ts \"\$(date -u +%Y-%m-%dT%H:%M:%SZ)\" '.current_story = \$story | .updated = \$ts' $external_worktree/.hive/drones/$drone_name/status.json > /tmp/status.tmp && mv /tmp/status.tmp $external_worktree/.hive/drones/$drone_name/status.json
\`\`\`

### 3. Quand tu TERMINES une story (TRÈS IMPORTANT!)
\`\`\`bash
jq --arg story \"STORY-ID\" --arg ts \"\$(date -u +%Y-%m-%dT%H:%M:%SZ)\" '.completed += [\$story] | .updated = \$ts' $external_worktree/.hive/drones/$drone_name/status.json > /tmp/status.tmp && mv /tmp/status.tmp $external_worktree/.hive/drones/$drone_name/status.json
\`\`\`

### 4. Quand TOUTES les stories sont terminées
\`\`\`bash
jq --arg ts \"\$(date -u +%Y-%m-%dT%H:%M:%SZ)\" '.status = \"completed\" | .current_story = null | .updated = \$ts' $external_worktree/.hive/drones/$drone_name/status.json > /tmp/status.tmp && mv /tmp/status.tmp $external_worktree/.hive/drones/$drone_name/status.json
\`\`\`

## Activity Log (activity.log)

Après CHAQUE action importante, ajoute une ligne:
\`\`\`bash
echo \"[\$(date +%H:%M:%S)] <emoji> <message>\" >> $external_worktree/.hive/drones/$drone_name/activity.log
\`\`\`

Emojis:
- 🚀 Démarrage du drone
- 📖 Lecture du PRD
- 🔨 Début d'une story
- 📝 Modification d'un fichier
- ✅ Story complétée
- 💾 Commit effectué
- ⚠️ Problème rencontré
- 🎉 Toutes les stories terminées

## Workflow pour chaque story

1. Log: \`[HH:MM:SS] 🔨 Début STORY-ID: titre\`
2. Update status.json: current_story = STORY-ID
3. Implémente les changements
4. Log chaque fichier modifié: \`[HH:MM:SS] 📝 Modification: path/to/file\`
5. Commit: \`git commit -m \"feat(STORY-ID): description\"\`
6. Log: \`[HH:MM:SS] 💾 Commit: feat(STORY-ID): description\`
7. **Update status.json: ajoute STORY-ID au tableau completed**
8. Log: \`[HH:MM:SS] ✅ STORY-ID terminée\`
9. Passe à la story suivante

## Commence maintenant

1. Log: \"🚀 Drone démarré\"
2. Lis le PRD
3. Update status.json: status = in_progress
4. Log: \"📖 PRD chargé: X stories à implémenter\"
5. Implémente story par story en suivant le workflow ci-dessus

Sois autonome et méthodique. N'oublie JAMAIS de mettre à jour status.json!"

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

LOG_FILE="$DRONE_DIR/drone.log"
STATUS_FILE="$DRONE_DIR/status.json"
ACTIVITY_LOG="$DRONE_DIR/activity.log"

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
            terminal-notifier -title "$title" -message "$message" -contentImage "$icon" $sound_param -group "hive" 2>/dev/null || true
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

    # Check if all stories are completed
    if [ -f "$STATUS_FILE" ]; then
        STATUS=$(jq -r '.status // "in_progress"' "$STATUS_FILE" 2>/dev/null)
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

    # Run Claude
    cd "$WORKTREE"
    claude --print -p "$(cat "$PROMPT_FILE")" \
        --model "$MODEL" \
        --allowedTools "Bash,Read,Write,Edit,Glob,Grep,TodoWrite" \
        >> "$LOG_FILE" 2>&1 || true

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

    # Launch the loop in background with nohup
    nohup "$launcher_script" "$drone_status_dir" "$prompt_file" "$model" "$iterations" "$external_worktree" "$drone_name" > /dev/null 2>&1 &

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
  ${GREEN}init${NC}     Initialize Hive in current repo
  ${GREEN}update${NC}   Update Hive to latest version
  ${GREEN}version${NC}  Show version
  ${GREEN}help${NC}     Show this help

${CYAN}Quick Start:${NC}
  hive start --prd prd-feature.json
  hive status
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
