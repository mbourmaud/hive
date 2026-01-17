# Hive Clean - Remove Drone and Worktree

Remove a drone and its associated git worktree. Use this after a drone has completed its work or to abandon a drone.

## Usage

- `/hive:clean` - Interactive: lists drones, asks which one to clean
- `/hive:clean security` - Direct: cleans the "security" drone
- `/hive:clean security --force` - Force cleanup even if drone is running

## Arguments

| Argument | Description |
|----------|-------------|
| `<name>` | Drone name to clean |
| `--force` or `-f` | Force cleanup even if running |

## Workflow

### Step 1: List Available Drones

If no drone name provided:

```bash
for dir in .hive/drones/*/; do
  [ -d "$dir" ] || continue
  name=$(basename "$dir")

  if [ -f "$dir/status.json" ]; then
    status=$(jq -r '.status' "$dir/status.json")
    completed=$(jq -r '.completed | length' "$dir/status.json")
    total=$(jq -r '.total' "$dir/status.json")
    echo "🐝 $name ($status) - $completed/$total"
  fi
done
```

Ask user which drone to clean.

### Step 2: Check Drone Status

If drone is still running (status = "starting" or "in_progress"):
- Warn user: "Drone is still running. Use --force to clean anyway."
- Unless `--force` was provided

### Step 3: Clean the Drone

Execute:
```bash
hive clean <drone-name>
```

Or with force:
```bash
hive clean -f <drone-name>
```

### Step 4: Confirm Cleanup

Tell the user:
```
Drone "<name>" has been cleaned up.

Removed:
  - Worktree: ~/Projects/<project>-<name>/
  - Branch: hive/<name>
  - State: .hive/drones/<name>/

The drone's work has been removed. If you need to recover:
  - Check git reflog for the branch
  - Commits may still exist in the repository
```

## Examples

```
/hive:clean                    # Interactive selection
/hive:clean security           # Clean security drone
/hive:clean security --force   # Force clean running drone
```

## Notes

- This permanently removes the drone's worktree and state
- The git branch is also deleted
- Use `--force` to clean a running drone (will kill it first)
- Completed work that was committed remains in git history
- If no drones exist, show "No drones to clean"
