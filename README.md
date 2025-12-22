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

### Project Secrets (.env.project)

For your **project-specific secrets** (API keys, database credentials, etc.), create a `.env.project` file:

```bash
# Copy the template
cp .env.project.example .env.project

# Edit with your project secrets
vim .env.project
```

**What goes where:**
- **`.env`** → Hive configuration (git user, workspace name, Claude token)
- **`.env.project`** → Your project secrets (database, API keys, etc.)

Example `.env.project`:
```bash
# Database
DATABASE_URL=postgresql://user:password@localhost:5432/mydb

# API Keys
OPENAI_API_KEY=sk-...
AWS_ACCESS_KEY_ID=AKIA...
STRIPE_SECRET_KEY=sk_test_...

# Auth
JWT_SECRET=your-super-secret-jwt-key
```

These variables are automatically available in **all agents** (Queen + Workers).

**Security:**
- `.env.project` is gitignored (never committed)
- Shared across all agents for consistency
- Use `.env.project.example` to document required secrets (commit this)

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

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## FAQ

### General

**Q: What is Hive?**
A: Hive is a multi-agent orchestration system that lets you run multiple Claude Code agents in parallel. Instead of working on tasks sequentially, you can break down work and execute it concurrently with a Queen (orchestrator) and Workers (executors).

**Q: How is this different from just opening multiple terminals?**
A: Hive provides:
- Task queue management (Redis-based, atomic operations)
- Isolated workspaces (each agent has its own git clone)
- Shared configuration (MCPs, skills, settings shared across agents)
- Built-in coordination commands (hive-assign, my-tasks, task-done)
- Pre-configured development environments (Node, Go, Python, Rust)

**Q: Do I need to know Redis or Docker?**
A: No! Hive handles all the infrastructure. You just need Docker installed and run `hive init`.

### Setup

**Q: What are the system requirements?**
A:
- Docker Desktop installed and running
- 8GB+ RAM (more RAM = more workers)
- 10GB+ free disk space
- macOS, Linux, or Windows (WSL2)

**Q: How do I get a Claude Code OAuth token?**
A: Run this in your terminal:
```bash
claude setup-token
```
Copy the token and use it in `hive init`.

**Q: Can I use my existing Claude Code configuration?**
A: Yes! Hive mounts your `~/.claude` directory, so all your MCPs, skills, and settings are automatically available to all agents.

**Q: What if I don't have a project repository yet?**
A: Leave `GIT_REPO_URL` empty in `.env`. Hive will create an empty workspace. You can initialize a git repo manually inside the Queen container.

**Q: How do I manage project secrets (API keys, database passwords)?**
A: Use `.env.project` for your project-specific secrets:
```bash
cp .env.project.example .env.project
# Edit with your secrets (DATABASE_URL, API keys, etc.)
```
These variables are automatically loaded in all agents. The file is gitignored for security. Use `.env.project.example` (committed) to document required secrets for your team.

### Usage

**Q: How many workers should I start?**
A: Start with 2-3 workers and scale up based on your workload:
- 2 workers: Good for learning Hive
- 3-4 workers: Typical feature development
- 5-8 workers: Large features or bug fixing sprints
- 10 workers: Maximum (large teams, parallel experiments)

Each worker uses ~1-2GB RAM.

**Q: Can I use a different programming language/stack?**
A: Yes! Set `HIVE_DOCKERFILE` in your `.env`:
```bash
HIVE_DOCKERFILE=docker/Dockerfile.go      # For Go projects
HIVE_DOCKERFILE=docker/Dockerfile.python  # For Python
HIVE_DOCKERFILE=docker/Dockerfile.rust    # For Rust
HIVE_DOCKERFILE=docker/Dockerfile.minimal # Generic (no language)
```

See [examples/](examples/) for language-specific guides.

**Q: How do I share code between workers?**
A: Each worker has its own git clone. Workers should:
1. Pull latest changes: `git pull origin main`
2. Work on separate branches
3. Push their branches when done
4. Create merge requests

The Queen can coordinate merges and resolve conflicts.

**Q: What happens if a worker gets stuck?**
A:
```bash
# Mark task as failed
task-failed "Reason for failure"

# Queen can reassign to another worker
hive-assign drone-2 "Continue task X" "..."
```

**Q: Can I stop Hive without losing my work?**
A: Yes! Your code is in `./workspaces/` on the host. Run:
```bash
hive stop  # Stops containers but keeps data
```

Later:
```bash
hive start 3  # Restart with your code intact
```

### Workflow

**Q: Should I create tasks for trivial changes?**
A: No. Use workers for substantial work that takes >15 minutes:
- ✅ Good: "Add user authentication API" (1-2 hours)
- ❌ Bad: "Fix typo in README" (1 minute)

**Q: How do I handle dependencies between tasks?**
A: Two approaches:

1. **Sequential phases:**
```bash
# Phase 1: Parallel (independent)
hive-assign drone-1 "Create database schema"
hive-assign drone-2 "Create UI mockups"
hive-assign drone-3 "Write tests"

# Wait for Phase 1 to complete...

# Phase 2: Integration (depends on Phase 1)
hive-assign drone-1 "Wire up API to DB"
hive-assign drone-2 "Connect UI to API"
```

2. **Mock dependencies:**
```bash
# Parallel (with mocks)
hive-assign drone-1 "Create real API"
hive-assign drone-2 "Create UI with mock API"  # Uses fake data

# Later: drone-2 swaps mock for real API
```

**Q: When should I use `task-done`?**
A: Only when:
- ✅ All tests pass
- ✅ Code builds without errors
- ✅ CI is green (if applicable)
- ✅ Code is committed

Never use `task-done` if there are failures. Use `task-failed` instead.

**Q: How do I review code from workers?**
A: Each worker creates a branch. Review via:
```bash
# In Queen
gh pr list                    # List all PRs
gh pr diff 123                # View diff
gh pr review 123 --approve    # Approve
```

Or use GitLab:
```bash
glab mr list
glab mr view 45
```

### Troubleshooting

**Q: "Connection refused" when running `hive connect`**
A: Check if containers are running:
```bash
hive status
# If not running:
hive start 3
```

**Q: "No space left on device"**
A: Docker is out of space. Clean up:
```bash
docker system prune -a
# Restart Hive
hive stop && hive start 3
```

**Q: Worker can't push to git**
A: Check git credentials:
```bash
hive connect 1
git config user.email  # Should show your email
git push origin my-branch  # Test push
```

If it fails, check that `GIT_USER_EMAIL` and `GIT_USER_NAME` are set in `.env`.

**Q: Changes made in one worker don't appear in another**
A: Each worker has its own git clone. Workers need to push/pull:
```bash
# Worker 1
git push origin my-feature

# Worker 2
git fetch origin
git checkout my-feature
```

**Q: How do I update Hive to the latest version?**
A:
```bash
# If installed via Homebrew
brew upgrade hive

# If installed manually
cd ~/path/to/hive
git pull origin main
make install
```

For more issues, see [docs/troubleshooting.md](docs/troubleshooting.md).

## Documentation

- 📖 [MCP Setup Guide](docs/mcp-setup.md) - Configure Model Context Protocol servers
- 🏗️ [Architecture](docs/architecture.md) - How Hive works internally
- 🔧 [Troubleshooting](docs/troubleshooting.md) - Common issues and solutions
- 🐳 [Docker Images](docker/README.md) - Available Dockerfiles for different tech stacks

## Examples

Language-specific examples with complete workflows:

- 🟢 [Node.js Monorepo](examples/nodejs-monorepo/) - Full-stack TypeScript development
- 🔵 [Go REST API](examples/golang-api/) - Microservices and gRPC
- 🟡 [Python ML Project](examples/python-ml/) - Parallel model training
- 🟠 [Rust CLI Tool](examples/rust-cli/) - Systems programming

## Support

- 🐛 [Report a bug](https://github.com/mbourmaud/hive/issues)
- 💡 [Request a feature](https://github.com/mbourmaud/hive/issues)
