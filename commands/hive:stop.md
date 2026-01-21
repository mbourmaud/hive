# Hive Stop - Stop a Running Drone

Stop a running drone. If no drone name is specified, show a list of active drones to choose from.

## Usage

- `/hive:stop` - Interactive: lists active drones, asks which one to stop
- `/hive:stop security` - Direct: stops the "security" drone immediately

## Arguments

| Argument | Description |
|----------|-------------|
| `<name>` | Drone name to stop |

## Workflow

### Step 1: Check for Running Drones

List active drones from `.hive/drones/`:

```bash
ls -d .hive/drones/*/ 2>/dev/null | while read dir; do
  name=$(basename "$dir")
  if [ -f "$dir/status.json" ]; then
    status=$(jq -r '.status' "$dir/status.json")
    done=$(jq -r '.completed | length' "$dir/status.json")
    total=$(jq -r '.total' "$dir/status.json")
    echo "$name ($status) - $done/$total"
  fi
done
```

### Step 2: If No Drone Specified

If the user didn't specify a drone name in the command, ask them:

**"Which drone do you want to stop?"**

Show options based on active drones (status = "starting" or "in_progress"):
- List each drone with its progress: `🐝 security (4/10)`
- Only show drones that are actually running

### Step 3: Stop the Drone

Execute:
```bash
hive stop <drone-name>
```

### Step 4: Confirm

Tell the user:
```
Drone "<name>" has been stopped.

To resume later, you can restart with:
  hive start --resume <name>

To clean up completely:
  hive clean <name>
```

## Examples

```
/hive:stop                # Interactive selection
/hive:stop security       # Stop security drone directly
```

## Notes

- Only show drones with status "starting" or "in_progress" in the selection
- If no drones are running, tell the user "No active drones to stop"
- The drone's progress is preserved in `.hive/drones/<name>/status.json`
- Stopping a drone does NOT delete the worktree or branch
- Use `hive clean` to remove worktree and branch after stopping
