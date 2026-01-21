# Hive Status - Show Status of All Drones

Display the status of all drones with their progress.

## Usage

- `/hive:status` - Show status of all drones

## Workflow

### Step 1: Check Hive Initialization

First, check if `.hive/` exists:

```bash
if [ ! -d ".hive" ]; then
  echo "Hive not initialized. Run: hive init"
  exit 1
fi
```

### Step 2: Get Drone Status

Execute:
```bash
hive monitor
```

Note: The `monitor` command opens an auto-refreshing TUI dashboard. Press 'q' to quit.

Or manually gather status:

```bash
for dir in .hive/drones/*/; do
  [ -d "$dir" ] || continue
  name=$(basename "$dir")

  if [ -f "$dir/status.json" ]; then
    status=$(jq -r '.status' "$dir/status.json")
    completed=$(jq -r '.completed | length' "$dir/status.json")
    total=$(jq -r '.total' "$dir/status.json")
    current=$(jq -r '.current_story // "none"' "$dir/status.json")
    prd=$(jq -r '.prd_file // "unknown"' "$dir/status.json")

    echo "🐝 $name"
    echo "   Status: $status"
    echo "   Progress: $completed/$total"
    echo "   Current: $current"
    echo "   PRD: $prd"
  fi
done
```

### Step 3: Display Results

Show a formatted table:

```
👑 Hive Status
──────────────────────────────────────────────────────────

🐝 security
   Status:   in_progress
   Progress: 4/10 stories
   Current:  SEC-005
   PRD:      .hive/prds/prd-security.json

🐝 feature
   Status:   completed ✓
   Progress: 5/5 stories
   PRD:      .hive/prds/prd-feature.json

──────────────────────────────────────────────────────────
Total: 2 drones | 1 active | 1 completed
```

### Step 4: Suggest Next Actions

Based on status, suggest:
- If drone is running: `hive logs <name>` or `hive kill <name>`
- If drone is completed: `hive clean <name>` to cleanup
- If drone failed: Check logs with `hive logs <name>`

## Examples

```
/hive:status    # Show all drone status
```

## Notes

- Shows all drones in `.hive/drones/`
- Status can be: `starting`, `in_progress`, `completed`, `failed`
- If no drones exist, show "No drones found"
