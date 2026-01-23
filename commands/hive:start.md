# Hive Start - Launch a Drone

Launch an autonomous drone on a PRD file.

## CRITICAL: Command Syntax

The `hive start` CLI uses this syntax:

```bash
hive start <NAME> [PROMPT] [OPTIONS]
```

Where `<NAME>` **must match the PRD filename** (without `prd-` prefix and `.json` suffix).

**Example:** For a PRD file `.hive/prds/prd-fix-auth-bug.json`, use:
```bash
hive start fix-auth-bug --model sonnet
```

**NOT** `--prd .hive/prds/prd-fix-auth-bug.json` (this flag doesn't exist!)

## Quick Reference

| PRD File | Launch Command |
|----------|----------------|
| `.hive/prds/prd-security-api.json` | `hive start security-api` |
| `.hive/prds/prd-fix-login-bug.json` | `hive start fix-login-bug` |
| `.hive/prds/prd-add-dark-mode.json` | `hive start add-dark-mode` |

## Available Options

```
hive start [OPTIONS] <NAME> [PROMPT]

Arguments:
  <NAME>    Drone name (must match PRD id: prd-<NAME>.json)
  [PROMPT]  Optional custom prompt to send to the drone

Options:
  --resume       Resume a blocked or stopped drone
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

# Launch with custom prompt
hive start my-feature "Focus on the authentication part first"

# Resume a stopped drone
hive start my-feature --resume

# Run in current directory (no worktree)
hive start quick-fix --local

# Dry run to test
hive start my-feature --dry-run
```

## Workflow

### Step 1: Check Hive Initialization

First, check if `.hive/` exists in the project:
```bash
ls -la .hive/prds/ 2>/dev/null || echo "Hive not initialized"
```

If not initialized, run `hive init`.

### Step 2: Find Available PRDs

List PRD files to find the correct name:
```bash
ls .hive/prds/*.json 2>/dev/null
```

Extract the drone name from the filename:
- `prd-fix-auth-bug.json` → drone name is `fix-auth-bug`
- `prd-security-api.json` → drone name is `security-api`

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
Kill:     hive kill <name>
```

## Important Notes

1. **The drone name MUST match the PRD id** - `prd-<name>.json` → `hive start <name>`
2. **Default model is sonnet** - Use `--model opus` for complex tasks
3. **Worktree is created automatically** - Unless `--local` is specified
4. **PRD must exist** - The command will error if no matching PRD is found

## Troubleshooting

**Error: "No PRD found for drone 'X'"**
- Check the PRD filename matches: `ls .hive/prds/`
- The name must match exactly (without `prd-` prefix and `.json` suffix)

**Error: "unexpected argument '--prd' found"**
- Don't use `--prd` flag - it doesn't exist
- Use: `hive start <name>` where name matches the PRD id
