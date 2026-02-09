# Hive Logs - View Drone Activity

View the activity log for a drone. Shows the team conversation and tool usage from the Agent Teams session.

## Usage

- `/hive:logs` - Interactive: lists drones, lets you pick one
- `/hive:logs <drone>` - Show activity log for a drone

## Arguments

| Argument | Description |
|----------|-------------|
| `<drone>` | Drone name |
| `--follow` or `-f` | Watch logs in real-time |

## Workflow

### Step 1: Identify Drone

If no drone name provided, list available drones:

```bash
ls -d .hive/drones/*/ 2>/dev/null | while read dir; do
  name=$(basename "$dir")
  status=$(jq -r '.status // "unknown"' "$dir/status.json" 2>/dev/null)
  echo "$name ($status)"
done
```

Ask user which drone to view.

### Step 2: Display Activity Log

The activity log contains the full Claude Code stream-json output from the team lead session.

```bash
cat .hive/drones/<name>/activity.log
```

**For follow mode:**
```bash
tail -f .hive/drones/<name>/activity.log
```

### Step 3: Show Status Summary

Display current drone status:

```bash
jq -r '"Status: \(.status) | Current: \(.current_task // "none")"' .hive/drones/<name>/status.json
```

## CLI Alternative

```bash
hive logs <name>        # Last 100 lines
hive logs -f <name>     # Follow mode
```

## Examples

```
/hive:logs                    # Interactive: select drone
/hive:logs security           # View activity log for security drone
/hive:logs security --follow  # Watch activity.log in real-time
```

## Notes

- Activity logs contain Claude Code stream-json output
- Use `hive monitor` TUI for the best real-time experience (press Enter to expand a drone, 'm' for messages)
- If no activity log exists yet, the drone may still be starting up
