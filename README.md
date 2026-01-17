# 👑 Hive

**Drone Orchestration for Claude Code**

Launch autonomous Claude agents (drones) on PRD files. Each drone works in its own git worktree, executing stories from a PRD while you continue working.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

---

## Quick Start

```bash
# Install
make install

# Initialize Hive in your project
hive init

# Launch a drone on a PRD
hive start --prd .hive/prds/prd-security.json

# Monitor progress
hive status

# View logs
hive logs security

# Cleanup when done
hive clean security
```

---

## What it does

```
┌─────────────────────────────────────────────────────────────┐
│  👑 Queen (main branch)                                     │
│  You + Claude working on features                           │
│  .hive/ folder with shared state                            │
├─────────────────────────────────────────────────────────────┤
│  🐝 Drone: security (hive/security branch)                  │
│  Autonomously implementing SEC-001 → SEC-010                │
│  .hive/ symlinked from queen                                │
├─────────────────────────────────────────────────────────────┤
│  🐝 Drone: feature-x (hive/feature-x branch)                │
│  Autonomously implementing FEAT-001 → FEAT-005              │
│  .hive/ symlinked from queen                                │
└─────────────────────────────────────────────────────────────┘
```

**Perfect for:**
- Executing PRD stories in the background
- Running multiple parallel implementations
- Large-scale refactoring with isolated workspaces
- Autonomous code generation from specifications

---

## Commands

| Command | Description |
|---------|-------------|
| `hive start --prd <file>` | Launch a drone on a PRD |
| `hive status` | Show status of all drones |
| `hive list` | List active drones |
| `hive logs <name>` | View drone logs |
| `hive logs -f <name>` | Follow drone logs |
| `hive kill <name>` | Stop a running drone |
| `hive clean <name>` | Remove drone and worktree |
| `hive init` | Initialize Hive in repo |
| `hive help` | Show help |

Run `hive <command> --help` for detailed options.

---

## Start Options

```bash
hive start --prd <file> [options]

Options:
  --prd <file>        PRD JSON file (required)
  --name <name>       Drone name (default: from PRD id)
  --base <branch>     Base branch (default: main)
  --iterations <n>    Max turns (default: 50)
  --model <model>     Claude model (default: opus)
```

### Examples

```bash
# Simple - uses PRD id as drone name
hive start --prd .hive/prds/prd-security.json

# Custom name and base branch
hive start --prd feature.json --name auth-feature --base develop

# More iterations for complex PRDs
hive start --prd big-refactor.json --iterations 100

# Use faster model
hive start --prd small-task.json --model sonnet
```

---

## PRD Format

PRDs are stored in `.hive/prds/`:

```json
{
  "id": "security-api-protection",
  "title": "Secure API Routes",
  "description": "Add authentication to all API routes",
  "stories": [
    {
      "id": "SEC-001",
      "title": "Protect /api/accounts/*",
      "description": "Add requireAuth() to account routes",
      "acceptance_criteria": [
        "GET /api/accounts returns 401 if not authenticated",
        "POST /api/accounts returns 401 if not authenticated"
      ],
      "files": [
        "src/app/api/accounts/route.ts"
      ]
    },
    {
      "id": "SEC-002",
      "title": "Protect /api/users/*",
      "description": "Add requireAuth() to user routes",
      "acceptance_criteria": ["..."],
      "files": ["..."]
    }
  ]
}
```

---

## Claude Code Integration

Use these skills in Claude Code:

| Skill | Description |
|-------|-------------|
| `/hive:start` | Interactive drone launch wizard |
| `/hive:prd` | Generate a PRD from feature description |
| `/hive:statusline` | Configure statusline with drone tracking |

### Statusline

After running `/hive:statusline`, your statusline shows:

```
project │ main │ Opus 4.5 │ 45% │ ⬢ 22
👑 Hive v0.2.0 | 🐝 security (4/10) | 🐝 feature ✓ (5/5)
```

---

## How it works

1. **Init**: `hive init` creates `.hive/` folder (gitignored)
2. **PRD**: Store PRDs in `.hive/prds/`
3. **Branch**: Creates `hive/<drone-name>` from base branch
4. **Worktree**: Creates `~/Projects/{project}-{drone}/`
5. **Symlink**: Links `.hive/` to worktree (shared state!)
6. **Launch**: Starts Claude agent in background
7. **Track**: Drone updates `.hive/drones/<name>/status.json`
8. **Commits**: Each story = one commit with `feat(<STORY-ID>): description`

---

## File Structure

```
your-project/                      # 👑 Queen (main repo)
├── .hive/                         # Shared state (gitignored)
│   ├── config.json
│   ├── prds/                      # PRD files here
│   │   └── prd-security.json
│   └── drones/                    # Drone state
│       └── security/
│           ├── status.json        # Progress tracking
│           ├── drone.log          # Output log
│           └── .pid               # Process ID

~/Projects/
├── your-project/                  # Queen
├── your-project-security/         # 🐝 Drone worktree
│   ├── .hive -> ../your-project/.hive  # Symlink!
│   └── (project files)
└── your-project-feature/          # 🐝 Another drone
    └── .hive -> ../your-project/.hive
```

The `.hive/` symlink enables **queen ↔ drone communication**:
- Queen can monitor drone progress in real-time
- Drones share PRDs from the same source
- Status updates are immediately visible

---

## Requirements

- `bash` - Shell interpreter
- `jq` - JSON processor
- `git` - Version control
- `claude` - Claude Code CLI

---

## Installation

```bash
# Clone
git clone https://github.com/mbourmaud/hive.git
cd hive

# Install to ~/.local/bin
make install
```

Make sure `~/.local/bin` is in your PATH.

### Manual Installation

```bash
cp hive.sh ~/.local/bin/hive
chmod +x ~/.local/bin/hive
```

---

## Troubleshooting

### "Not a git repository"
Make sure you're in a git repository before running hive commands.

### "jq is required"
Install jq: `brew install jq` (macOS) or `apt install jq` (Ubuntu).

### Drone process died
Check logs with `hive logs <name>`. Restart with `hive start --prd ...`.

### Worktree conflicts
Use `hive clean -f <name>` to force cleanup, then retry.

---

## License

MIT License - see [LICENSE](LICENSE) for details.

---

Made with 👑 by [@mbourmaud](https://github.com/mbourmaud)
