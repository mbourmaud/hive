# 🐝 Hive

**Drone Orchestration for Claude Code**

Launch autonomous Claude agents (drones) on PRD files. Each drone works in its own git worktree, executing stories from a PRD while you continue working.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

---

## Quick Start

```bash
# Install
make install

# Launch a drone on a PRD
hive run --prd prd-security.json

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
│  Your terminal (main branch)                                │
│  You + Claude working on features                           │
├─────────────────────────────────────────────────────────────┤
│  🐝 Drone: security (hive/security branch)                  │
│  Autonomously implementing SEC-001 → SEC-010                │
├─────────────────────────────────────────────────────────────┤
│  🐝 Drone: feature-x (hive/feature-x branch)                │
│  Autonomously implementing FEAT-001 → FEAT-005              │
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
| `hive run --prd <file>` | Launch a drone on a PRD |
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

## Run Options

```bash
hive run --prd <file> [options]

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
hive run --prd prd-security.json

# Custom name and base branch
hive run --prd feature.json --name auth-feature --base develop

# More iterations for complex PRDs
hive run --prd big-refactor.json --iterations 100

# Use faster model
hive run --prd small-task.json --model sonnet
```

---

## PRD Format

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

## Statusline Integration

Add drone tracking to your Claude Code statusline:

```
/hive-statusline
```

This displays active drones in your statusline:

```
project │ main │ Opus │ 🐝 security (4/10) │ 🐝 feature (2/5)
```

---

## How it works

1. **Branch Creation**: Creates `hive/<drone-name>` from base branch
2. **Worktree**: Creates isolated worktree at `~/Projects/{project}-{drone}/`
3. **PRD Copy**: Copies PRD to worktree as `prd.json`
4. **Claude Launch**: Starts Claude agent in background with PRD instructions
5. **Status Tracking**: Drone updates `drone-status.json` as it completes stories
6. **Commits**: Each story = one commit with `feat(<STORY-ID>): description`

---

## File Structure

```
your-project/
├── .hive/
│   └── config.json      # Hive configuration
└── prd-*.json           # Your PRD files

~/Projects/
├── your-project/            # Main repo (you work here)
├── your-project-security/   # Drone worktree
│   ├── prd.json
│   ├── drone-status.json
│   ├── drone.log
│   └── .drone.pid
└── your-project-feature/    # Another drone
```

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
Check logs with `hive logs <name>`. Restart with `hive run --prd ...`.

### Worktree conflicts
Use `hive clean -f <name>` to force cleanup, then retry.

---

## License

MIT License - see [LICENSE](LICENSE) for details.

---

Made with 🐝 by [@mbourmaud](https://github.com/mbourmaud)
