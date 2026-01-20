# Hive Logs - View Drone Execution Logs

View comprehensive logs of drone execution including all Claude invocations, story-specific attempts, and metadata.

## Usage

- `/hive:logs` - Interactive: lists drones and stories, lets you browse
- `/hive:logs <drone>` - Show all stories with logs for a drone
- `/hive:logs <drone> <story>` - Show all attempts for a specific story
- `/hive:logs <drone> <story> --attempt N` - Show specific attempt log

## Arguments

| Argument | Description |
|----------|-------------|
| `<drone>` | Drone name |
| `<story>` | Story ID (e.g., SEC-001) |
| `--attempt N` | Show specific attempt number |
| `--follow` or `-f` | Watch logs in real-time (activity log only) |
| `--raw` | Show raw drone.log instead of activity.log |

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

### Step 2: Choose Log Type

Three log types available:

1. **Activity log**: `.hive/drones/<name>/activity.log` - Human-readable activity summary
2. **Raw log**: `.hive/drones/<name>/drone.log` - Complete Claude output across all iterations
3. **Story logs**: `.hive/drones/<name>/logs/<STORY-ID>/` - Per-story, per-attempt detailed logs

### Step 3: Display Story-Specific Logs (NEW in v1.9.0)

Story logs are organized as:
```
.hive/drones/<name>/logs/
  ├── SEC-001/
  │   ├── attempt-1.log           # First attempt at SEC-001
  │   ├── attempt-1-metadata.json # Duration, exit code, timestamps
  │   ├── attempt-2.log           # Second attempt (if first failed)
  │   └── attempt-2-metadata.json
  └── SEC-002/
      ├── attempt-1.log
      └── attempt-1-metadata.json
```

**List stories with logs:**
```bash
ls -d .hive/drones/<name>/logs/*/ 2>/dev/null | while read dir; do
  story=$(basename "$dir")
  attempts=$(ls -1 "$dir"/attempt-*.log 2>/dev/null | wc -l)
  echo "$story ($attempts attempts)"
done
```

**View a specific attempt:**
```bash
# Show metadata first
cat .hive/drones/<name>/logs/SEC-001/attempt-1-metadata.json

# Then show log
cat .hive/drones/<name>/logs/SEC-001/attempt-1.log
```

**Metadata format:**
```json
{
  "story": "SEC-001",
  "attempt": 1,
  "started": "2026-01-20T10:30:00Z",
  "completed": "2026-01-20T10:35:22Z",
  "duration_seconds": 322,
  "model": "claude-sonnet-4.5",
  "exit_code": 0,
  "iteration": 3
}
```

### Step 4: Display Classic Logs

**For activity.log:**
```bash
cat .hive/drones/<name>/activity.log
```

**For follow mode:**
```bash
tail -f .hive/drones/<name>/activity.log
```

### Step 5: Show Status Summary

Display current status and blocked reason if blocked:

```bash
jq -r 'if .status == "blocked" then
  "Status: \(.status) ⚠️ BLOCKED\nReason: \(.blocked_reason)\nProgress: \(.completed | length)/\(.total)\nCurrent: \(.current_story // "none")"
else
  "Status: \(.status) | Progress: \(.completed | length)/\(.total) | Current: \(.current_story // "none")"
end' .hive/drones/<name>/status.json
```

## Structured Logs from status.json

You can also show structured logs from `status.json`:

```bash
jq -r '.logs[]? | "[\(.time)] \(.event): \(.story // "") \(.message // "")"' .hive/drones/<name>/status.json
```

## CLI Alternative

The user can also use the CLI directly:
```bash
hive logs <name>        # Last 100 lines
hive logs -f <name>     # Follow mode
```

## Examples

```
/hive:logs                         # Interactive: select drone → story → attempt
/hive:logs security                # List all stories with logs for security drone
/hive:logs security SEC-001        # View all attempts for SEC-001 story
/hive:logs security SEC-001 --attempt 2  # View specific attempt
/hive:logs security --follow       # Watch activity.log in real-time
/hive:logs security --raw          # Raw Claude output
```

## Notes

- **Story logs** (v1.9.0+): Per-story, per-attempt logs with metadata (duration, exit code)
- **Activity logs**: Human-readable with emojis
- **Raw logs**: Complete Claude output (verbose)
- **Follow mode**: Useful for watching drone progress live
- **Blocked drones**: Check logs to understand why drone blocked itself
- If activity.log doesn't exist yet, show drone.log instead

## TUI Alternative

Use the interactive TUI for the best experience:
```bash
hive status -i
# Select drone → "📊 View story logs"
```

This provides an interactive browsing experience with metadata display.
