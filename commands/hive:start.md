# Hive Start - Launch a Drone

Launch an autonomous drone on a PRD file.

## Usage

- `/hive:start` - Interactive wizard (asks all questions)
- `/hive:start --prd .hive/prds/security.json` - Direct with PRD
- `/hive:start --prd security.json --iterations 100 --model sonnet` - Full options

## Arguments

Parse arguments from the command if provided:

| Argument | Description | Default |
|----------|-------------|---------|
| `--prd <file>` | PRD JSON file | (ask user) |
| `--name <name>` | Drone name | (from PRD id) |
| `--base <branch>` | Base branch | main |
| `--iterations <n>` | Max API turns | 50 |
| `--model <model>` | Claude model (opus/sonnet) | opus |

**Examples:**
```
/hive:start --prd .hive/prds/prd-security.json
/hive:start --prd security.json --name sec --iterations 100
/hive:start --prd feature.json --model sonnet --base develop
```

## Workflow

### Step 0: Parse Arguments

Check if user provided arguments. Extract:
- `--prd` → prd_file
- `--name` → drone_name
- `--base` → base_branch
- `--iterations` → iterations
- `--model` → model

For any argument NOT provided, ask the user interactively.

### Step 1: Check Hive Initialization

First, check if `.hive/` exists in the project. If not, run `hive init`.

### Step 2: Select PRD File (if not provided)

If `--prd` was provided, use it directly. Otherwise:

Search for PRD files in `.hive/prds/` and project root:
```bash
find .hive/prds -name "*.json" 2>/dev/null
find . -maxdepth 1 -name "prd*.json" -o -name "*-prd.json" 2>/dev/null
```

Ask the user to select which PRD to use.

### Step 3: Execution Mode (if not obvious)

If all args provided, assume worktree mode. Otherwise ask:

**"Where should the drone work?"**

Options:
- **New worktree (Recommended)** - Isolated environment
- **Current branch** - Work in current directory (risky)

### Step 4: Drone Name (if not provided)

If `--name` not provided, suggest from PRD's `id` field:
```bash
jq -r '.id // .name // "drone"' <prd-file>
```

### Step 5: Base Branch (if not provided)

If `--base` not provided, ask:

**"Which branch should the drone start from?"**

Options:
- **main (Recommended)**
- **develop**
- **Current branch**
- Other

### Step 6: Iterations (if not provided)

If `--iterations` not provided, ask:

**"How many iterations?"**

Options:
- **50 (Recommended)** - 5-10 stories
- **25** - Small PRDs
- **100** - Large PRDs
- **Unlimited** - Sets to 999

### Step 7: Model (if not provided)

If `--model` not provided, ask:

**"Which Claude model?"**

Options:
- **opus (Recommended)** - Best quality
- **sonnet** - Faster, cheaper

### Step 8: Confirmation

If ANY argument was missing (interactive mode), show summary:
```
Ready to launch drone:

  PRD:        .hive/prds/prd-security.json (10 stories)
  Drone:      security
  Base:       main
  Iterations: 50
  Model:      opus

Proceed? [Y/n]
```

If ALL arguments were provided, skip confirmation and launch directly.

### Step 9: Launch

Execute:
```bash
hive start --prd <file> --name <name> --base <branch> --iterations <n> --model <model>
```

### Step 10: Post-launch Info

```
Drone launched successfully!

Monitor:  hive status
Logs:     hive logs <name>
Stop:     hive kill <name>

Statusline will show: 👑 Hive v0.2.0 | 🐝 security (0/10)
```

## Quick Launch Examples

```
# Full interactive (no args)
/hive:start

# Just specify PRD, rest is interactive
/hive:start --prd .hive/prds/prd-auth.json

# Quick launch with all options (no questions asked)
/hive:start --prd security.json --name sec --base main --iterations 50 --model opus

# Fast mode with sonnet
/hive:start --prd small-task.json --model sonnet --iterations 25
```

## File Structure

After launch:
```
your-project/                      # Queen
├── .hive/
│   ├── prds/
│   │   └── prd-security.json
│   └── drones/
│       └── security/
│           ├── status.json
│           ├── drone.log
│           └── .pid

~/Projects/your-project-security/  # Drone worktree
├── .hive -> ../your-project/.hive # Symlink!
└── (project files)
```

## Notes

- If all args provided → launch immediately without questions
- If some args missing → only ask for missing ones
- Always validate PRD file exists before launching
- Check `hive` and `claude` CLI are available
