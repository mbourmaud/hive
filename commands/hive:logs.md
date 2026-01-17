# Hive Logs - View Drone Activity

View the activity logs of a drone in a readable format.

## Usage

- `/hive:logs` - Interactive: lists drones, asks which one to view
- `/hive:logs security` - Direct: shows logs for "security" drone
- `/hive:logs security --follow` - Follow mode: watch logs in real-time

## Arguments

| Argument | Description |
|----------|-------------|
| `<name>` | Drone name |
| `--follow` or `-f` | Watch logs in real-time |
| `--raw` | Show raw drone.log instead of activity.log |

## Workflow

### Step 1: Identify Drone

If no drone name provided, list available drones:

```bash
ls -d .hive/drones/*/ 2>/dev/null | while read dir; do
  name=$(basename "$dir")
  echo "$name"
done
```

Ask user which drone to view.

### Step 2: Check Log Files

Two log files exist:
- `.hive/drones/<name>/activity.log` - Human-readable activity log
- `.hive/drones/<name>/drone.log` - Raw Claude output

Default to `activity.log`. Use `--raw` for `drone.log`.

### Step 3: Display Logs

**For activity.log:**
```bash
cat .hive/drones/<name>/activity.log
```

Output looks like:
```
[10:30:15] 🚀 Drone démarré
[10:30:20] 📖 PRD chargé: 10 stories à implémenter
[10:32:00] 🔨 Début SEC-001: Protect /api/accounts
[10:33:15] 📝 Modification: src/app/api/accounts/route.ts
[10:35:22] 💾 Commit: feat(SEC-001): Add auth to accounts API
[10:35:22] ✅ SEC-001 terminée
[10:35:30] 🔨 Début SEC-002: Protect /api/users
```

**For follow mode:**
```bash
tail -f .hive/drones/<name>/activity.log
```

### Step 4: Show Status Summary

After showing logs, display current status:

```bash
jq -r '"Status: \(.status) | Progress: \(.completed | length)/\(.total) | Current: \(.current_story // "none")"' .hive/drones/<name>/status.json
```

Output:
```
────────────────────────────────────
Status: in_progress | Progress: 4/10 | Current: SEC-005
────────────────────────────────────
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
/hive:logs                    # Interactive selection
/hive:logs security           # View security drone logs
/hive:logs security --follow  # Watch in real-time
/hive:logs security --raw     # Raw Claude output
```

## Notes

- Activity logs are human-readable with emojis
- Raw logs contain full Claude output (verbose)
- Follow mode is useful for watching drone progress live
- If activity.log doesn't exist yet, show drone.log instead
