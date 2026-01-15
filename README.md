# Hive - Multi-Ralph Orchestration

**Run multiple Claude Code instances in parallel.** A single bash script for orchestrating multiple Claude Code (Ralph) instances via git worktrees.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

---

## What is Hive?

Hive lets you spawn multiple Claude Code agents (Ralphs), each working in an isolated git worktree. Each Ralph runs autonomously, iterating until tasks are verified complete.

```
┌─────────────────────────────────────────────────────┐
│              Your Project Repository                │
├─────────────────────────────────────────────────────┤
│                                                     │
│  ┌───────────┐  ┌───────────┐  ┌───────────┐       │
│  │  Ralph 1  │  │  Ralph 2  │  │  Ralph 3  │       │
│  │ (worktree)│  │ (worktree)│  │ (worktree)│       │
│  │ feature/A │  │ feature/B │  │ feature/C │       │
│  └───────────┘  └───────────┘  └───────────┘       │
│                                                     │
│  Orchestrated by hive.sh                           │
│                                                     │
└─────────────────────────────────────────────────────┘
```

**Perfect for:**
- Fixing multiple bugs simultaneously
- Developing features in parallel (frontend + backend + tests)
- Large-scale refactoring with isolated workspaces
- Continuous iteration until all tests pass

---

## Prerequisites

- **bash** - Shell interpreter (included on macOS/Linux)
- **jq** - JSON processor ([install](https://jqlang.github.io/jq/download/))
- **git** - Version control
- **gh** - GitHub CLI ([install](https://cli.github.com/))
- **claude** - Claude Code CLI

---

## Installation

```bash
# Clone the repository
git clone https://github.com/mbourmaud/hive
cd hive

# Install hive to ~/.local/bin
make install
```

This copies `hive.sh` to `~/.local/bin/hive`. Make sure `~/.local/bin` is in your PATH.

### Manual Installation

```bash
# Or just copy the script directly
cp hive.sh ~/.local/bin/hive
chmod +x ~/.local/bin/hive
```

---

## Quick Start

### 1. Initialize in your project

```bash
cd your-project
hive init
```

### 2. Create a Ralph

```bash
# Create a new branch for the Ralph
hive spawn my-feature --create feature/my-feature

# Or attach to an existing branch
hive spawn my-feature --attach feature/existing-branch
```

### 3. Start the Ralph

```bash
hive start my-feature "Implement user authentication"
```

### 4. Monitor progress

```bash
# Check status of all Ralphs
hive status

# Watch live logs
hive logs my-feature --follow

# Live dashboard with auto-refresh
hive dashboard
```

### 5. Create a Pull Request

```bash
hive pr my-feature
```

### 6. Cleanup after merge

```bash
hive cleanup my-feature
```

---

## Commands

| Command | Description |
|---------|-------------|
| `hive init` | Initialize Hive in current repository |
| `hive spawn <name> --create <branch>` | Create new Ralph with new branch |
| `hive spawn <name> --attach <branch>` | Attach Ralph to existing branch |
| `hive start <name> [prompt]` | Start Ralph background process |
| `hive status` | Show status of all Ralphs |
| `hive logs <name> [lines]` | View Ralph's output log |
| `hive stop <name>` | Stop a running Ralph |
| `hive sync <name>` | Sync worktree with target branch |
| `hive pr <name> [--draft]` | Create Pull Request |
| `hive prs` | List all PRs created by Hive |
| `hive cleanup <name>` | Remove worktree after PR merge |
| `hive clean <name>` | Remove worktree (abandon work) |
| `hive dashboard` | Live status dashboard |

Run `hive <command> --help` for detailed information on each command.

---

## Example Workflows

### Single Feature Development

```bash
# Initialize Hive in your repo
hive init

# Create a Ralph for a new feature
hive spawn auth-feature --create feature/auth --from main

# Start Ralph with a task
hive start auth-feature "Implement JWT authentication"

# Monitor progress
hive status
hive logs auth-feature --follow

# When done, create a PR
hive pr auth-feature

# After PR is merged, cleanup
hive cleanup auth-feature
```

### Parallel Feature Development

```bash
# Create multiple Ralphs for different features
hive spawn frontend --create feature/ui-redesign
hive spawn backend --create feature/api-v2
hive spawn tests --create feature/e2e-tests

# Start all Ralphs
hive start frontend "Redesign the dashboard UI"
hive start backend "Implement REST API v2"
hive start tests "Add end-to-end test coverage"

# Monitor all with dashboard
hive dashboard
```

### Collaborative Work on Same Branch

```bash
# First Ralph creates the branch
hive spawn lead --create feature/big-feature

# Second Ralph attaches with scoped access
hive spawn helper --attach feature/big-feature --scope "src/utils/*"

# Start both with different tasks
hive start lead "Implement main feature logic"
hive start helper "Create utility functions"
```

### Continue Existing Work

```bash
# Attach to an existing remote branch
hive spawn continue-work --attach feature/existing-branch
hive start continue-work "Complete the remaining tasks"
```

---

## Project Structure

After running `hive init`, the following structure is created:

```
your-project/
├── .hive/
│   ├── config.json      # Configuration and state
│   └── worktrees/       # Git worktrees for each Ralph
│       ├── ralph-1/
│       ├── ralph-2/
│       └── ...
└── ...
```

---

## How It Works

1. **Git Worktrees**: Each Ralph works in an isolated git worktree, allowing parallel development on different branches without conflicts.

2. **Background Processes**: Ralphs run as background processes using `nohup`, surviving terminal sessions.

3. **State Management**: All state is stored in `.hive/config.json`, tracking Ralph status, PIDs, branches, and PR information.

4. **GitHub Integration**: Uses `gh` CLI for PR creation and status tracking.

---

## Tips

- Use `hive dashboard` for a live view of all Ralphs
- Use `--scope` when attaching multiple Ralphs to the same branch to avoid conflicts
- Run `hive sync <name>` regularly to keep worktrees up-to-date with the target branch
- Use `hive clean <name>` to abandon work without creating a PR

---

## Troubleshooting

### "Not a git repository"
Make sure you're in a git repository before running `hive init`.

### "jq is required but not installed"
Install jq: `brew install jq` (macOS) or `apt install jq` (Ubuntu).

### "gh CLI not authenticated"
Run `gh auth login` to authenticate with GitHub.

### Ralph process died unexpectedly
Check the logs with `hive logs <name>` to see what went wrong. You can restart with `hive start <name>`.

---

## License

MIT License - see [LICENSE](LICENSE) for details.

---

Made with Claude Code by [@mbourmaud](https://github.com/mbourmaud)
