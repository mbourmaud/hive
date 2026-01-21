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
2. Scan for drone directories: `${HIVE_WORKTREE_BASE:-$HOME/Projects}/${project_name}-*/`
3. Read status from `drone-status.json` or `ralph-status.json`
4. Build the drone line with crown + version + drones
5. Only show line 2 if there are active drones AND hive is installed

Key code for drone display:
```bash
# Check if process is running
pid_file="${drone_dir}.pid"
is_running="no"
if [ -f "$pid_file" ]; then
  pid=$(cat "$pid_file" 2>/dev/null)
  ps -p "$pid" >/dev/null 2>&1 && is_running="yes"
fi

# For each drone, format based on status:
if [ "$d_status" = "in_progress" ] || [ "$d_status" = "starting" ]; then
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

- Drones are detected by looking for `{project}-{drone}/drone-status.json` in `${HIVE_WORKTREE_BASE:-$HOME/Projects}/`
- Also supports legacy `ralph-status.json` files
- The statusline refreshes on each user message
- Line 2 only appears if both hive is installed AND drones exist

## Related Skills

- `/hive:start` - Launch a new drone
- `/hive:prd` - Create a PRD for drones
