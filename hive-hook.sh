#!/bin/bash
# Hive postToolUse hook - sends notification when drone completes
# Called after each Bash tool use

# Read hook input from stdin
input=$(cat)

# Get current working directory from hook context
cwd=$(echo "$input" | jq -r '.cwd // empty')
[ -z "$cwd" ] && cwd=$(pwd)

# Check if we're in a Hive drone worktree
hive_dir="$cwd/.hive/drones"
[ -d "$hive_dir" ] || exit 0

# Get drone name from worktree path (format: project-dronename)
drone_name=$(basename "$cwd" | sed 's/.*-//')

# Check if status.json exists for this drone
status_file="$hive_dir/$drone_name/status.json"
[ -f "$status_file" ] || exit 0

# Read current status
status=$(jq -r '.status // empty' "$status_file" 2>/dev/null)
completed=$(jq -r '.completed | length // 0' "$status_file" 2>/dev/null)
total=$(jq -r '.total // 0' "$status_file" 2>/dev/null)

# Check if just completed (all stories done)
if [ "$status" = "completed" ] || { [ "$completed" -ge "$total" ] && [ "$total" -gt 0 ]; }; then
    # Check if we already sent notification (use a marker file)
    notified_file="$hive_dir/$drone_name/.notified"
    if [ ! -f "$notified_file" ]; then
        # Send notification
        if command -v terminal-notifier &>/dev/null; then
            terminal-notifier -title "🎉 Hive - Drone Completed!" \
                -message "$drone_name: $completed/$total stories done" \
                -contentImage "$HOME/.local/share/hive/bee-icon.png" \
                -sound Glass \
                -group "hive-$drone_name" 2>/dev/null
        elif command -v osascript &>/dev/null; then
            osascript -e "display notification \"$drone_name: $completed/$total stories done\" with title \"🎉 Hive - Drone Completed!\" sound name \"Glass\"" 2>/dev/null
        elif command -v notify-send &>/dev/null; then
            notify-send "🎉 Hive - Drone Completed!" "$drone_name: $completed/$total stories done" 2>/dev/null
        fi
        # Mark as notified
        touch "$notified_file"
    fi
fi
