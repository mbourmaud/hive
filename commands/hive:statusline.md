# Hive Statusline Configuration

Configure Claude Code's statusline to display Hive drone status using the native `hive statusline` command.

## What this does

Sets your `~/.claude/settings.json` statusline to use the native Hive statusline command. This replaces the previous bash one-liner with a fast, native Rust implementation.

**Line 1:** Project context
```
project │ branch+icons │ model │ context%
```

**Line 2:** Hive dashboard (only if active drones exist)
```
🐝 hive vX.Y.Z │ 🐝 drone1 5/10 2m 30s │ 🐝 drone2 ✓ 10/10 5m 0s
```

## Drone Status Icons

- `🐝 name N/M elapsed` - In progress, running (yellow)
- `🐝 name ⏸ N/M elapsed` - In progress, paused (gray) - process not running
- `🐝 name ✓ N/M elapsed` - Completed (yellow + green check)
- `🐝 name ✗ N/M` - Error (yellow + red cross)

## Instructions

### Step 1: Read current settings

Read `~/.claude/settings.json` to check the current statusline configuration.

### Step 2: Update statusline config

Set the statusline configuration to:

```json
{
  "statusLine": {
    "type": "command",
    "command": "hive statusline"
  }
}
```

This replaces any existing statusline configuration. The `hive statusline` command:
- Reads JSON from stdin (workspace, model, context_window) provided by Claude Code
- Outputs formatted ANSI-colored text to stdout
- Shows git branch, status icons, model, context usage
- Shows active drone status on a second line (only when drones exist)

### Step 3: Save and confirm

Save the file and tell the user to restart Claude Code or open a new session for changes to take effect.

## Notes

- Requires `hive` to be installed and in PATH
- The statusline refreshes on each user message
- Line 2 only appears if active drones exist in the current project's `.hive/drones/`
- Completed drones are hidden after 1 hour

## Related Skills

- `/hive:start` - Launch a new drone
- `/hive:plan` - Create a plan for drones
