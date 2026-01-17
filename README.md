<p align="center">
  <img src="assets/logo.png" alt="Hive Logo" width="200">
</p>

<h1 align="center">Hive</h1>

<p align="center">
  <strong>Drone Orchestration for Claude Code</strong>
</p>

<p align="center">
  Launch autonomous Claude agents (drones) on PRD files. Each drone works in its own git worktree, executing stories from a PRD while you continue working.
</p>

<p align="center">
  <a href="https://opensource.org/licenses/MIT"><img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="License: MIT"></a>
  <a href="https://github.com/mbourmaud/hive/releases"><img src="https://img.shields.io/github/v/release/mbourmaud/hive" alt="Release"></a>
</p>

---

## Installation

```bash
curl -fsSL https://raw.githubusercontent.com/mbourmaud/hive/main/install.sh | bash
```

Installs:
- **CLI**: `hive` command to `~/.local/bin/`
- **Skills**: `/hive:*` commands for Claude Code
- **Icon**: Bee icon for notifications

---

## Quick Start

```bash
# Initialize Hive in your project
hive init

# Create a PRD (in Claude Code)
/hive:prd

# Launch a drone on the PRD
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

## Features

### 🔔 Desktop Notifications

Hive sends desktop notifications when:
- **🐝 Drone Started** - When a new drone begins working
- **🎉 Drone Completed** - When all stories are done
- **⏸️ Drone Paused** - When max iterations reached

**Platform Support:**
| Platform | Tool | Custom Icon |
|----------|------|-------------|
| macOS | terminal-notifier | ✅ |
| Linux | notify-send | ✅ |
| Windows/WSL | PowerShell Toast | ❌ |

### 📊 Statusline Integration

After running `/hive:statusline`, your Claude Code statusline shows:

```
project │ main │ Opus 4.5 │ 45% │ ⬢ 22
👑 Hive v1.1.0 | 🐝 security (4/10) | 🐝 feature ✓ (5/5)
```

---

## Commands

| CLI | Skill | Description |
|-----|-------|-------------|
| `hive init` | `/hive:init` | Initialize Hive in repo |
| `hive start --prd <file>` | `/hive:start` | Launch a drone on a PRD |
| `hive status` | `/hive:status` | Show status of all drones |
| `hive list` | `/hive:list` | List active drones |
| `hive logs <name>` | `/hive:logs` | View drone logs |
| `hive kill <name>` | `/hive:kill` | Stop a running drone |
| `hive clean <name>` | `/hive:clean` | Remove drone and worktree |
| `hive update` | - | Update Hive to latest version |
| - | `/hive:prd` | Generate a PRD from feature description |
| - | `/hive:statusline` | Configure statusline with drone tracking |

Run `hive <command> --help` for detailed options.

---

## Start Options

```bash
hive start --prd <file> [options]

Options:
  --prd <file>        PRD JSON file (required)
  --name <name>       Drone name (default: from PRD id)
  --base <branch>     Base branch (default: main)
  --iterations <n>    Max iterations (default: 15)
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

## How it works

1. **Init**: `hive init` creates `.hive/` folder (gitignored)
2. **PRD**: Store PRDs in `.hive/prds/`
3. **Branch**: Creates `hive/<drone-name>` from base branch
4. **Worktree**: Creates `~/Projects/{project}-{drone}/`
5. **Symlink**: Links `.hive/` to worktree (shared state!)
6. **Launch**: Starts Claude agent in background loop
7. **Track**: Drone updates `.hive/drones/<name>/status.json`
8. **Notify**: Desktop notifications on start/complete/pause
9. **Commits**: Each story = one commit with `feat(<STORY-ID>): description`

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
│           ├── activity.log       # Human-readable log
│           ├── drone.log          # Raw Claude output
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
- `terminal-notifier` (macOS, optional) - For custom notification icons

---

## Manual Installation

If you prefer not to use the install script:

```bash
# Clone
git clone https://github.com/mbourmaud/hive.git
cd hive

# Install CLI
cp hive.sh ~/.local/bin/hive
chmod +x ~/.local/bin/hive

# Install skills (Claude Code)
cp commands/*.md ~/.claude/commands/

# Install icon (for notifications)
mkdir -p ~/.local/share/hive
cp assets/logo.png ~/.local/share/hive/bee-icon.png
```

Make sure `~/.local/bin` is in your PATH.

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

### No notifications on macOS
Install terminal-notifier: `brew install terminal-notifier` and allow notifications in System Preferences.

---

## License

MIT License - see [LICENSE](LICENSE) for details.

---

<p align="center">
  Made with 👑 by <a href="https://github.com/mbourmaud">@mbourmaud</a>
</p>
