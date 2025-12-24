# Frequently Asked Questions (FAQ)

## General

### What is Hive?

Hive is a multi-agent orchestration system that lets you run multiple Claude Code agents in parallel. Instead of working on tasks sequentially, you can break down work and execute it concurrently with a Queen (orchestrator) and Workers (executors).

### How is this different from just opening multiple terminals?

Hive provides:
- **Task queue management**: Redis-based, atomic operations
- **Isolated workspaces**: Each agent has its own git clone
- **Shared configuration**: MCPs, skills, settings shared across agents
- **Built-in coordination**: Commands like `hive-assign`, `my-tasks`, `task-done`
- **Pre-configured environments**: Node, Go, Python, Rust Docker images

Without Hive, you'd need to manually:
- Coordinate task distribution
- Set up Redis for task queue
- Configure each terminal separately
- Manage workspace isolation
- Track task status manually

### Do I need to know Redis or Docker?

No! Hive handles all the infrastructure. You just need:
- Docker Desktop installed and running
- Run `hive init`

Everything else (Redis, task queue, container orchestration) is automated.

---

## Setup

### What are the system requirements?

- **Docker Desktop**: Installed and running
- **RAM**: 8GB+ (more RAM = more workers)
  - Each worker uses ~1-2GB RAM
  - Recommended: 16GB for 4+ workers
- **Disk**: 10GB+ free space
- **OS**: macOS, Linux, or Windows (WSL2)

### How do I get a Claude Code OAuth token?

Run this in your terminal:
```bash
claude setup-token
```

This opens a browser for authentication and saves your token. Copy the token and use it in `hive init` or in your `.env` file.

### Can I use my existing Claude Code configuration?

Yes! Hive mounts your `~/.claude` directory, so:
- All your MCPs are available to all agents
- Skills are shared across agents
- Settings are consistent
- Only conversation history is isolated per agent

This means you configure MCPs once on your host machine, and they work in all containers.

### What if I don't have a project repository yet?

Leave `GIT_REPO_URL` empty in `.env`. Hive will create an empty workspace. You can:
1. Initialize a git repo manually inside the Queen container
2. Clone a repo later
3. Work without version control (not recommended)

### How do I manage project secrets (API keys, database passwords)?

Use `.env.project` for your project-specific secrets:

```bash
# Copy the template
cp .env.project.example .env.project

# Edit with your secrets
vim .env.project
```

**What goes where:**
- **`.env`**: Hive configuration (git user, workspace name, Claude token)
- **`.env.project`**: Project secrets (DATABASE_URL, API keys, JWT_SECRET, etc.)

These variables are automatically loaded in all agents. The file is gitignored for security. Use `.env.project.example` (committed to git) to document required secrets for your team.

---

## Usage

### How many workers should I start?

Start with 2-3 workers and scale based on your workload:

| Workers | Use Case | RAM Required |
|---------|----------|--------------|
| 2 | Learning Hive, small tasks | 8GB |
| 3-4 | Typical feature development | 12GB |
| 5-8 | Large features, bug sprints | 16GB+ |
| 10 | Maximum (parallel experiments) | 24GB+ |

Each worker uses ~1-2GB RAM depending on the Docker image (minimal < node < rust).

### Can I use a different programming language/stack?

Yes! Set `HIVE_DOCKERFILE` in your `.env`:

```bash
# For Go projects
HIVE_DOCKERFILE=docker/Dockerfile.golang

# For Python projects
HIVE_DOCKERFILE=docker/Dockerfile.python

# For Rust projects
HIVE_DOCKERFILE=docker/Dockerfile.rust

# Generic (no language-specific tools)
HIVE_DOCKERFILE=docker/Dockerfile.minimal
```

See [examples/](../examples/) for language-specific guides.

### How do I share code between workers?

Each worker has its own git clone. To share code:

**Worker 1:**
```bash
# Make changes
git add .
git commit -m "feat: add feature X"
git push origin feature-x
```

**Worker 2:**
```bash
# Get latest code
git fetch origin
git checkout feature-x
# Or: git pull origin main
```

The Queen can coordinate merges and resolve conflicts.

### What happens if a worker gets stuck?

Mark the task as failed and reassign:

**Worker (stuck):**
```bash
task-failed "Blocked: need API documentation"
```

**Queen (reassign):**
```bash
hive-assign drone-2 "Continue task X" "Use mock API for now" "TICKET-123"
```

Or investigate:
```bash
# In another terminal
hive connect 1
# Debug the issue
```

### Can I stop Hive without losing my work?

Yes! Your code is in `./workspaces/` on the host:

```bash
# Stop containers (keeps data)
hive stop

# Later, restart
hive start 3
# Your code is still there
```

To completely remove everything:
```bash
docker compose down -v  # ⚠️ Deletes volumes
rm -rf workspaces/      # ⚠️ Deletes workspace data
```

---

## Workflow

### Should I create tasks for trivial changes?

No. Use workers for substantial work (>15 minutes):

✅ **Good tasks:**
- "Add user authentication API" (1-2 hours)
- "Implement search with filters" (1 hour)
- "Add integration tests for payment flow" (1 hour)

❌ **Bad tasks:**
- "Fix typo in README" (1 minute)
- "Add console.log for debugging" (30 seconds)
- "Update package version" (2 minutes)

For trivial changes, do them directly without assigning tasks.

### How do I handle dependencies between tasks?

Two approaches:

#### 1. Sequential Phases

```bash
# Phase 1: Independent tasks (parallel)
hive-assign drone-1 "Create database schema"
hive-assign drone-2 "Create UI mockups"
hive-assign drone-3 "Write API tests"

# Wait for Phase 1 to complete...

# Phase 2: Integration (depends on Phase 1)
hive-assign drone-1 "Wire up API to database"
hive-assign drone-2 "Connect UI to API"
```

#### 2. Mock Dependencies

```bash
# All parallel (use mocks)
hive-assign drone-1 "Create real API"
hive-assign drone-2 "Create UI with mock API"  # Uses fake data

# Later: drone-2 swaps mock for real API
```

### When should I use `task-done`?

Only when **all conditions are met**:
- ✅ All tests pass
- ✅ Code builds without errors
- ✅ CI is green (if applicable)
- ✅ Code is committed and pushed

**Never** use `task-done` if there are failures. Use `task-failed "reason"` instead.

### How do I review code from workers?

Each worker creates a branch. Review via GitHub/GitLab CLI:

**GitHub:**
```bash
# In Queen
gh pr list
gh pr diff 123
gh pr review 123 --approve
```

**GitLab:**
```bash
glab mr list
glab mr view 45
glab mr approve 45
```

Or use the web UI.

---

## Troubleshooting

### "Connection refused" when running `hive connect`

**Cause:** Containers are not running.

**Fix:**
```bash
hive status
# If not running:
hive start 3
```

### "No space left on device"

**Cause:** Docker is out of disk space.

**Fix:**
```bash
# Clean up Docker
docker system prune -a

# Restart Hive
hive stop
hive start 3
```

### Worker can't push to git

**Cause:** Git credentials not configured.

**Fix:**
```bash
hive connect 1

# Check config
git config user.email
git config user.name

# If empty, check .env file
# GIT_USER_EMAIL and GIT_USER_NAME must be set
```

### Changes made in one worker don't appear in another

**Cause:** Each worker has its own git clone. Changes must be pushed/pulled.

**Fix:**
```bash
# Worker 1 (made changes)
git push origin my-feature

# Worker 2 (needs changes)
git fetch origin
git checkout my-feature
# Or: git pull origin main
```

This is **by design** for workspace isolation.

### MCP not working in containers

**Cause:** MCP not installed on host machine.

**Fix:**
```bash
# On your host machine (not in container)
claude mcp add <mcp-name>

# Configure MCP in ~/.claude/settings.json
```

MCPs are shared from `~/.claude`, so install them on the host.

### How do I update Hive to the latest version?

**Step 1: Update Hive CLI**

If installed via Homebrew:
```bash
brew upgrade hive
```

If installed manually:
```bash
cd ~/path/to/hive
git pull origin main
make install
```

**Step 2: Update containers (non-destructive)**

```bash
cd ~/your-project
hive update
```

This rebuilds containers while preserving all data:
- ✅ Workspaces and code changes
- ✅ Conversation history
- ✅ Redis task queue
- ✅ Package caches

**Options:**
```bash
hive update                    # Smart rebuild (uses cache)
hive update --rebuild          # Force rebuild from scratch
hive update --pull             # Pull latest base images
hive update --rebuild --pull   # Complete refresh
```

**Old method (destructive):**
```bash
hive clean  # ⚠️ Destroys all data
hive init   # Start from scratch
```

Only use `hive clean && hive init` if you want to start fresh or have corrupted volumes.

---

## Performance

### Hive is slow to start

**Causes:**
1. Docker images need to be pulled/built (first time only)
2. Large workspace being cloned

**Solutions:**
```bash
# Pre-build images
docker compose build

# Use smaller workspace (clone only what you need)
# Use shallow clone in .env:
GIT_REPO_URL=https://github.com/user/repo.git --depth=1
```

### Workers are running out of memory

**Causes:**
1. Too many workers for available RAM
2. Heavy processes (builds, tests) running simultaneously

**Solutions:**
```bash
# Reduce worker count
hive stop
hive start 2  # Instead of 5

# Use lighter Docker image
HIVE_DOCKERFILE=docker/Dockerfile.minimal

# Stagger heavy operations (don't run all tests at once)
```

### Redis connection errors

**Cause:** Redis container not healthy.

**Fix:**
```bash
# Check Redis status
docker compose ps redis

# Check Redis logs
docker compose logs redis

# Restart Redis
docker compose restart redis
```

---

## Advanced

### Can I use a custom Docker image?

Yes! Create your own Dockerfile:

```dockerfile
# docker/Dockerfile.custom
FROM ubuntu:22.04

# Install your tools
RUN apt-get update && apt-get install -y \
    your-tools-here

# Install Claude Code (required)
RUN npm install -g @anthropic-ai/claude
```

Then:
```bash
# In .env
HIVE_DOCKERFILE=docker/Dockerfile.custom
```

### Can I run Hive on a remote server?

Yes, but you'll need to:
1. Forward ports for `docker exec` (or use SSH)
2. Mount remote directories
3. Configure network access

See [docs/remote-setup.md](remote-setup.md) (coming soon).

### Can I use Hive with other AI models?

Currently, Hive is designed for Claude Code. Support for other models would require:
- Different CLI tool
- Different auth mechanism
- Modified CLAUDE.md instructions

This is not currently supported.

---

## See Also

- [Troubleshooting Guide](troubleshooting.md) - Detailed error solutions
- [Architecture](architecture.md) - How Hive works internally
- [Configuration](configuration.md) - All configuration options
- [Best Practices](best-practices.md) - Tips for effective parallel development
