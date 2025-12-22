# 🐝 HIVE - Multi-Agent Claude System

Run multiple Claude Code agents in parallel with simple task coordination.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- **Multi-agent**: Queen orchestrator + up to 10 worker drones
- **Redis Queue**: Atomic task management on port 6380
- **Simple CLI**: `hive status`, `hive connect queen`, `hive connect 1`...
- **Isolated Workspaces**: Each agent has its own git clone
- **Full Tooling**: gh, glab, docker, pnpm, Playwright
- **Configurable**: Works with any project, any language

## Quick Start

```bash
# Clone Hive
git clone https://github.com/mbourmaud/hive.git
cd hive

# One-command setup (interactive wizard)
hive init

# Or non-interactive (for automation/Claude)
hive init \
  --email "you@example.com" \
  --name "Your Name" \
  --token "$CLAUDE_CODE_OAUTH_TOKEN" \
  --workspace "my-project" \
  --workers 3 \
  --no-interactive

# Check status
hive status
```

### Manual Setup (alternative)

```bash
# 1. Install CLI
make install

# 2. Setup environment
cp .env.example .env
# Edit .env with your tokens

# 3. Start HIVE
hive start 3
```

## Example: Fix 3 Bugs in Parallel

Open 4 terminals:

**Terminal 1 - Queen (orchestrator):**
```bash
hive connect queen
```

Queen will auto-check status. You tell her:
```
Fix these bugs in parallel:
- Bug 1: Login timeout
- Bug 2: Export CSV empty
- Bug 3: Email validation
```

Queen creates tasks:
```bash
hive-assign drone-1 "Fix login timeout" "Increase session timeout to 30min" "ISSUE-1234"
hive-assign drone-2 "Fix CSV export" "Handle empty data edge case" "ISSUE-1235"
hive-assign drone-3 "Fix email validation" "Update regex for plus addressing" "ISSUE-1236"
```

**Terminal 2, 3, 4 - Workers:**
```bash
hive connect 1    # Terminal 2 → drone-1
hive connect 2    # Terminal 3 → drone-2
hive connect 3    # Terminal 4 → drone-3
```

Each worker:
1. Sees task via `my-tasks` (auto-run on startup)
2. Takes it: `take-task`
3. Fixes the bug
4. Marks done: `task-done` (only when CI is GREEN!)

Queen monitors progress with `hive-status`.

**Result:** 3 bugs fixed in parallel instead of sequentially.

## Configuration

Create `.env` from `.env.example`:

```bash
# Required
GIT_USER_EMAIL=you@example.com
GIT_USER_NAME=Your Name
WORKSPACE_NAME=my-project

# Claude auth (get token from: claude setup-token)
CLAUDE_CODE_OAUTH_TOKEN=your_oauth_token

# Optional: Auto-clone your repo on first start
GIT_REPO_URL=https://github.com/user/repo.git

# Optional: GitHub/GitLab tokens for gh/glab CLI
GITHUB_TOKEN=your_github_token
# GITLAB_TOKEN=your_gitlab_token
# GITLAB_HOST=gitlab.com
```

## Commands

### HIVE CLI (host)

```bash
hive start [N]       # Start Queen + N workers (default: 2)
hive stop            # Stop all containers
hive status          # Show running agents
hive connect <id>    # Connect to agent (queen, 1, 2, 3...)
```

### Queen Commands (inside queen agent)

```bash
hive-status                           # Show HIVE status
hive-assign <drone> <title> <desc> [ticket]  # Assign task
hive-failed                           # List failed tasks
```

### Worker Commands (inside worker agent)

```bash
my-tasks              # Check my queue and active task
take-task             # Get next task from queue
task-done             # Mark task as completed (only when CI GREEN!)
task-failed "message" # Mark task as failed
```

## Architecture

```
┌──────────────────────────────────────────────┐
│              Host Machine                    │
│                                              │
│  ┌────────────────────────────────────────┐ │
│  │  Queen (Orchestrator)                  │ │
│  │  - Analyzes complex requests           │ │
│  │  - Creates subtasks                    │ │
│  │  - Monitors progress                   │ │
│  └──────────┬─────────────────────────────┘ │
│             │                                │
│      ┌──────┼──────┐                        │
│      ↓      ↓      ↓                        │
│   ┌────┐ ┌────┐ ┌────┐                      │
│   │ D1 │ │ D2 │ │ D3 │  Workers             │
│   └────┘ └────┘ └────┘                      │
│             │                                │
│      ┌──────────────┐                        │
│      │ Redis :6380  │  Task Queue           │
│      └──────────────┘                        │
└──────────────────────────────────────────────┘
```

## Use Cases

### 1. Feature Development

```bash
# Queen breaks down feature into parallel tasks
hive-assign drone-1 "Create database schema" "Add user tables and migrations"
hive-assign drone-2 "Build REST API" "Create user CRUD endpoints"
hive-assign drone-3 "Create UI components" "Build user management forms"
hive-assign drone-4 "Write tests" "Add integration tests for user flow"
```

### 2. Bug Fixing Sprint

```bash
# Multiple bugs fixed in parallel
hive-assign drone-1 "Fix #123" "Authentication timeout issue"
hive-assign drone-2 "Fix #124" "CSV export encoding"
hive-assign drone-3 "Fix #125" "Email validation regex"
```

### 3. Code Refactoring

```bash
# Refactor different modules simultaneously
hive-assign drone-1 "Refactor auth module" "Extract auth logic to service"
hive-assign drone-2 "Refactor data layer" "Migrate to Prisma ORM"
hive-assign drone-3 "Update tests" "Update tests for new structure"
```

## Best Practices

### 1. Independent Tasks First

✅ Good:
```bash
hive-assign drone-1 "Add validation"
hive-assign drone-2 "Add tests"
hive-assign drone-3 "Update docs"
```

❌ Bad:
```bash
hive-assign drone-1 "Create API"
hive-assign drone-2 "Create UI that calls API"  # Depends on drone-1!
```

### 2. One Task = One Responsibility

❌ Too broad: `"Implement payment module"`

✅ Better:
- `"Add payment database schema"`
- `"Add Stripe service"`
- `"Add payment form component"`

### 3. Clear Error Messages

```bash
# ✅ Good
task-failed "Prisma migration failed: unique constraint on email field"

# ❌ Bad
task-failed "error"
```

## Troubleshooting

**Check HIVE status:**
```bash
hive status
```

**Worker can't see tasks:**
```bash
# Inside worker container
redis-cli -h localhost -p 6380 ping
# Should return: PONG
```

**Task stuck:**
```bash
# Mark as failed to unblock
task-failed "Blocked: need clarification on requirements"

# Queen can reassign
hive-assign drone-2 "Continue task" "..."
```

## Pre-installed Tools

Each container includes:

- **claude** - Claude Code AI
- **gh** - GitHub CLI
- **glab** - GitLab CLI
- **docker** - Docker CLI (testcontainers)
- **pnpm** - Package manager
- **playwright** - Browser automation
- **node 22** - Node.js runtime

## Building from Source

```bash
make build          # Build binary
make install        # Install to /usr/local/bin
make clean          # Clean build artifacts
```

## License

MIT

## Contributing

Contributions welcome! Open an issue or PR.

## Documentation

- 📖 [MCP Setup Guide](docs/mcp-setup.md) - Configure Model Context Protocol servers
- 🏗️ [Architecture](docs/architecture.md) - How Hive works internally
- 🔧 [Troubleshooting](docs/troubleshooting.md) - Common issues and solutions
- 🐳 [Docker Images](docker/README.md) - Available Dockerfiles for different tech stacks

## Support

- 🐛 [Report a bug](https://github.com/mbourmaud/hive/issues)
- 💡 [Request a feature](https://github.com/mbourmaud/hive/issues)
