# Hive Statusline Configuration

Configure Claude Code's statusline to display active Hive drones.

## What this does

This skill modifies your `~/.claude/settings.json` to add drone tracking to the statusline. After running, your statusline will show:

**Line 1:** Normal statusline
```
imanisa-finance │ main │ Opus 4.5 │ 45% │ ⬢ 22
```

**Line 2:** Hive dashboard (only if drones exist)
```
👑 Hive v1.1.0 │ 🐝 security ✓ (10/10) │ 🐝 feature (5/10)
```

## Drone Status Icons

- `🐝 name (5/10)` - In progress, running (yellow)
- `🐝 name ⏸ (5/10)` - In progress, paused (light gray) - process not running
- `🐝 name ✓ (10/10)` - Completed (yellow + green check)
- `🐝 name ✗ (5/10)` - Error (yellow + red cross)
- `🐝 name ⏹ (5/10)` - Stopped/zombie (light gray)

## Instructions

Read the user's current statusline configuration from `~/.claude/settings.json`, then modify it to add drone tracking on a second line.

### Step 1: Read current settings

```bash
cat ~/.claude/settings.json
```

### Step 2: Understand the current statusline

The statusline is configured in `settings.json` under:
```json
{
  "statusLine": {
    "type": "command",
    "command": "..."
  }
}
```

### Step 3: Add drone tracking

The drone line logic should:

1. Get hive version: `hive version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+'`
2. Find the current project's `.hive` directory
3. Read status from `.hive/drones/*/status.json` files
4. Check PID files at `.hive/drones/{name}/.pid`
5. Build the drone line with crown + version + drones
6. Only show line 2 if there are active drones AND hive is installed

Key code for drone display:
```bash
# Find .hive directory in project
hive_dir="$CLAUDE_PROJECT_DIR/.hive"

if [ -d "$hive_dir/drones" ]; then
  for status_file in "$hive_dir"/drones/*/status.json; do
    [ -f "$status_file" ] || continue

    drone_dir="$(dirname "$status_file")"
    d_name=$(jq -r '.drone' "$status_file" 2>/dev/null)
    d_status=$(jq -r '.status' "$status_file" 2>/dev/null)
    d_done=$(jq '.completed | length' "$status_file" 2>/dev/null)
    d_total=$(jq -r '.total' "$status_file" 2>/dev/null)

    # Check if process is running
    pid_file="$drone_dir/.pid"
    is_running="no"
    if [ -f "$pid_file" ]; then
      pid=$(cat "$pid_file" 2>/dev/null)
      ps -p "$pid" >/dev/null 2>&1 && is_running="yes"
    fi

    # Format based on status and PID state
    if [ "$d_status" = "in_progress" ] || [ "$d_status" = "starting" ] || [ "$d_status" = "resuming" ]; then
      if [ "$is_running" = "yes" ]; then
        # 🐝 name (done/total) - yellow (running)
        drone_line="${drone_line}$(printf \"\\033[33m🐝 %s (%s/%s)\\033[0m\" \"$d_name\" \"$d_done\" \"$d_total\")"
      else
        # 🐝 name ⏸ (done/total) - light gray (paused)
        drone_line="${drone_line}$(printf \"\\033[37m🐝 %s ⏸ (%s/%s)\\033[0m\" \"$d_name\" \"$d_done\" \"$d_total\")"
      fi
    elif [ "$d_status" = "completed" ]; then
      # 🐝 name ✓ (done/total) - yellow with green check
      drone_line="${drone_line}$(printf \"\\033[33m🐝 %s \\033[92m✓\\033[0m \\033[90m(%s/%s)\\033[0m\" \"$d_name\" \"$d_done\" \"$d_total\")"
    elif [ "$d_status" = "error" ]; then
      # 🐝 name ✗ (done/total) - yellow with red cross
      drone_line="${drone_line}$(printf \"\\033[33m🐝 %s \\033[91m✗\\033[0m \\033[90m(%s/%s)\\033[0m\" \"$d_name\" \"$d_done\" \"$d_total\")"
    elif [ "$d_status" = "stopped" ] || [ "$d_status" = "zombie" ]; then
      # 🐝 name ⏹ (done/total) - light gray (stopped)
      drone_line="${drone_line}$(printf \"\\033[37m🐝 %s ⏹ (%s/%s)\\033[0m\" \"$d_name\" \"$d_done\" \"$d_total\")"
    fi

    # Add separator if not first drone
    [ -n "$drone_line" ] && drone_line="${drone_line}${sep2}"
  done
fi
```

Final output with crown:
```bash
if [ -n "$hive_ver" ] && [ -n "$drone_line" ]; then
  out="$out\n$(printf \"\\033[33m👑 Hive v%s\\033[0m\" \"$hive_ver\")${sep2}${drone_line}"
fi
```

### Step 4: Save and confirm

After editing, save the file and tell the user to restart Claude Code or open a new session for changes to take effect.

## Example Output

```
imanisa-finance │ main │ Opus 4.5 │ 45% │ ⬢ 22
👑 Hive v1.1.0 │ 🐝 security ✓ (10/10) │ 🐝 feature (5/10) │ 🐝 refactor ⏸ (3/8)
```

## Notes

- Drones are detected from `.hive/drones/*/status.json` in the current project directory
- Status fields: `.drone` (name), `.status` (state), `.completed` (array of done tasks), `.total` (total count)
- PID files are at `.hive/drones/{name}/.pid` (not in worktree directories)
- The statusline refreshes on each user message
- Line 2 only appears if both hive is installed AND drones exist

## Related Skills

- `/hive:start` - Launch a new drone
- `/hive:plan` - Create a plan for drones
