# Hive Init - Initialize Hive in Repository

Initialize Hive in the current git repository. Creates the `.hive/` folder structure.

## Usage

- `/hive:init` - Initialize Hive in current repo

## Workflow

### Step 1: Check Prerequisites

Verify we're in a git repository:

```bash
if ! git rev-parse --git-dir > /dev/null 2>&1; then
  echo "Error: Not a git repository"
  exit 1
fi
```

Check if Hive is already initialized:

```bash
# Check if .hive exists as a directory (correct)
if [ -d ".hive" ] && [ ! -L ".hive" ]; then
  echo "Hive is already initialized in this repository"
  exit 0
fi

# If .hive is a symlink (wrong!), remove it
if [ -L ".hive" ]; then
  echo "Warning: .hive is a symlink (should be a directory). Fixing..."
  rm ".hive"
fi
```

### Step 2: Initialize Hive

Execute:
```bash
hive init
```

Or manually create structure:

```bash
mkdir -p .hive/prds .hive/drones

cat > .hive/config.json << EOF
{
  "version": "0.2.0",
  "project": "$(basename $(pwd))",
  "created": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
}
EOF

# Add to .gitignore
if [ -f .gitignore ]; then
  grep -q "^\.hive/$" .gitignore || echo ".hive/" >> .gitignore
else
  echo ".hive/" > .gitignore
fi
```

### Step 3: Confirm

Tell the user:
```
👑 Hive initialized!

Structure created:
  .hive/
  ├── config.json
  ├── prds/        # Store your PRD files here
  └── drones/      # Drone state (auto-managed)

Next steps:
  1. Create a PRD:     /hive:prd
  2. Launch a drone:   /hive:start --prd .hive/prds/your-prd.json
  3. Monitor:          /hive:status
```

## Examples

```
/hive:init    # Initialize Hive
```

## Notes

- Creates `.hive/` folder (gitignored)
- Safe to run multiple times (idempotent)
- Requires being in a git repository
- PRD files go in `.hive/prds/`
