# Hive Start - Launch a Drone

Launch an autonomous drone on a plan file.

## CRITICAL: Command Syntax

The `hive start` CLI uses this syntax:

```bash
hive start <NAME> [OPTIONS]
```

Where `<NAME>` **must match the plan filename** (without `.md` extension).

**Example:** For a plan file `.hive/plans/fix-auth-bug.md`, use:
```bash
hive start fix-auth-bug --model sonnet
```

## Quick Reference

| Plan File | Launch Command |
|----------|----------------|
| `.hive/plans/fix-auth-bug.md` | `hive start fix-auth-bug` |
| `.hive/plans/security-api.md` | `hive start security-api` |
| `.hive/plans/add-dark-mode.md` | `hive start add-dark-mode` |

Legacy JSON plans (`plan-<name>.json`) are also supported for backward compatibility.

## Available Options

```
hive start [OPTIONS] <NAME>

Arguments:
  <NAME>    Drone name (must match plan filename: <NAME>.md)

Options:
  --local        Run in current directory instead of worktree
  --model MODEL  Model to use: sonnet (default), opus, haiku
  --dry-run      Dry run - don't launch Claude
```

## Usage Examples

```bash
# Launch drone with default settings (sonnet model)
hive start fix-auth-bug

# Launch with specific model
hive start security-api --model opus

# Run in current directory (no worktree)
hive start quick-fix --local

# Dry run to test
hive start my-feature --dry-run
```

## Workflow

### Step 1: Check Hive Initialization

First, check if `.hive/` exists in the project:
```bash
ls -la .hive/plans/ 2>/dev/null || echo "Hive not initialized"
```

If not initialized, run `hive init`.

### Step 2: Find Available Plans

List plan files to find the correct name:
```bash
ls .hive/plans/*.md 2>/dev/null
```

Extract the drone name from the filename:
- `fix-auth-bug.md` → drone name is `fix-auth-bug`
- `security-api.md` → drone name is `security-api`

### Step 3: Launch the Drone

Use the extracted name:
```bash
hive start <drone-name> --model sonnet
```

### Step 4: Monitor Progress

After launch, show:
```
Drone launched successfully!

Monitor:  hive monitor <name>
Logs:     hive logs <name>
Stop:     hive stop <name>
```

## Important Notes

1. **The drone name MUST match the plan filename** - `<name>.md` → `hive start <name>`
2. **Default model is sonnet** - Use `--model opus` for complex tasks
3. **Worktree is created automatically** - Unless `--local` is specified
4. **Plan must exist** - The command will error if no matching plan is found

## Troubleshooting

**Error: "No plan found for drone 'X'"**
- Check the plan filename matches: `ls .hive/plans/`
- The name must match exactly (the filename without `.md` extension)
