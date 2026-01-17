# Hive List - List Active Drones

List all active drones (compact view).

## Usage

- `/hive:list` - List all drones

## Workflow

### Step 1: Check Hive Initialization

```bash
if [ ! -d ".hive" ]; then
  echo "Hive not initialized. Run: hive init"
  exit 1
fi
```

### Step 2: List Drones

Execute:
```bash
hive list
```

Or manually:

```bash
for dir in .hive/drones/*/; do
  [ -d "$dir" ] || continue
  name=$(basename "$dir")

  if [ -f "$dir/status.json" ]; then
    status=$(jq -r '.status' "$dir/status.json")
    completed=$(jq -r '.completed | length' "$dir/status.json")
    total=$(jq -r '.total' "$dir/status.json")

    case "$status" in
      "completed") icon="✓" ;;
      "failed")    icon="✗" ;;
      *)           icon="⋯" ;;
    esac

    echo "🐝 $name $icon ($completed/$total)"
  fi
done
```

### Step 3: Display Results

Show compact list:

```
👑 Hive Drones
────────────────────────────
🐝 security ⋯ (4/10)
🐝 feature ✓ (5/5)
🐝 refactor ✗ (2/8)
────────────────────────────
3 drones
```

## Examples

```
/hive:list    # List all drones
```

## Notes

- More compact than `/hive:status`
- Shows: name, status icon, progress
- Status icons: `⋯` (running), `✓` (completed), `✗` (failed)
- If no drones exist, show "No drones found"
