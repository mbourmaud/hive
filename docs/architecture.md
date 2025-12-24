# Hive Architecture

Deep dive into how Hive's multi-agent system works.

## System Overview

```
┌──────────────────────────────────────────────┐
│              Host Machine                    │
│  ~/.claude/ (SHARED)                         │
│  ├── mcp/          ← MCPs                    │
│  ├── skills/       ← Custom skills           │
│  └── claude.json   ← Settings                │
│                                              │
│  ┌────────────────────────────────────────┐ │
│  │  Queen (Orchestrator)                  │ │
│  │  - Analyzes complex requests           │ │
│  │  - Breaks into parallelizable tasks    │ │
│  │  - Assigns to workers via Redis        │ │
│  │  - Monitors progress                   │ │
│  │  - history.jsonl (isolated)            │ │
│  └──────────┬─────────────────────────────┘ │
│             │                                │
│      ┌──────┼──────┐                        │
│      ↓      ↓      ↓                        │
│   ┌────┐ ┌────┐ ┌────┐                      │
│   │ W1 │ │ W2 │ │ W3 │  Workers             │
│   │ ·  │ │ ·  │ │ ·  │  - Take tasks        │
│   │ ·  │ │ ·  │ │ ·  │  - Execute           │
│   │ ·  │ │ ·  │ │ ·  │  - Report done       │
│   └────┘ └────┘ └────┘  - Isolated history │
│             │                                │
│      ┌──────────────┐                        │
│      │ Redis :6380  │  Task Queue           │
│      │              │  - Atomic ops         │
│      │              │  - Pub/Sub            │
│      └──────────────┘                        │
└──────────────────────────────────────────────┘
```

## Components

### 1. Queen (Orchestrator)

**Container:** `claude-queen`
**Model:** Opus (configurable)
**Working Dir:** `/workspace/${WORKSPACE_NAME}`

**Responsibilities:**
- Analyze user requests
- Break into independent subtasks
- Assign tasks to workers via Redis
- Monitor task queue
- Handle failures/reassignments
- Merge results

**Commands:**
```bash
hive-status    # Check queue status
hive-assign    # Assign task to worker
hive-failed    # List failed tasks
```

**Special Instructions:** `templates/CLAUDE-QUEEN.md`

### 2. Workers (Drones)

**Containers:** `claude-agent-1` ... `claude-agent-10`
**Model:** Sonnet (configurable)
**Working Dir:** `/workspace/${WORKSPACE_NAME}`

**Responsibilities:**
- Poll for assigned tasks
- Execute tasks independently
- Commit & push changes
- Wait for CI to pass
- Report completion

**Commands:**
```bash
my-tasks       # Check my queue
take-task      # Get next task
task-done      # Mark complete (CI must be GREEN!)
task-failed    # Mark failed with error
```

**Special Instructions:** `templates/CLAUDE-WORKER.md`

### 3. Redis (Task Queue)

**Container:** `hive-redis`
**Port:** `6380` (avoids conflict with app Redis on 6379)
**Persistence:** Append-only file (AOF)

**Data Structure:**
```
Keys:
├── hive:queue:<drone-id>      LIST    # Pending tasks per worker
├── hive:active:<drone-id>     LIST    # Currently executing task
├── hive:completed             ZSET    # Completed tasks (score=timestamp)
├── hive:failed                ZSET    # Failed tasks (score=timestamp)
├── hive:task:<task-id>        HASH    # Task details
└── hive:events                PUBSUB  # Task notifications
```

## Task Lifecycle

### 1. Creation (Queen)

```bash
hive-assign drone-1 "Fix login bug" "Details..." "PROJ-123"
```

**What happens:**
```
1. Generate task ID: task-drone-1-1735000000
2. Create task JSON:
   {
     "id": "task-drone-1-1735000000",
     "drone": "drone-1",
     "title": "Fix login bug",
     "description": "Details...",
     "jira_ticket": "PROJ-123",
     "branch": "feature/PROJ-123-fix-login-bug",
     "status": "pending",
     "created_at": "2025-12-23T10:00:00Z"
   }
3. LPUSH hive:queue:drone-1 <task-json>
4. PUBLISH hive:events "task_queued:drone-1"
```

### 2. Execution (Worker)

```bash
take-task
```

**What happens:**
```
1. RPOPLPUSH hive:queue:drone-1 hive:active:drone-1
2. Parse task JSON
3. Checkout branch (or create)
4. Execute task
5. Run tests/lint
6. Commit & push
7. Wait for CI → GREEN
```

### 3. Completion (Worker)

```bash
task-done
```

**What happens:**
```
1. LPOP hive:active:drone-1
2. ZADD hive:completed <timestamp> <task-json>
3. PUBLISH hive:events "task_completed:drone-1"
```

### 4. Failure (Worker)

```bash
task-failed "Error message"
```

**What happens:**
```
1. LPOP hive:active:drone-1
2. Add error_message to task JSON
3. ZADD hive:failed <timestamp> <task-json>
4. PUBLISH hive:events "task_failed:drone-1"
```

## Configuration Sharing

### Shared (All Agents)

**Selective Mounts:**
- `${HOME}/.claude/mcps:/home/agent/.claude/mcps:ro`
- `${HOME}/.claude/plugins:/home/agent/.claude/plugins:ro`
- `${HOME}/.claude/projects:/home/agent/.claude/projects`

**Contains:**
- `mcps/` - Model Context Protocol servers
- `plugins/` - Custom Claude plugins
- `projects/` - Project-specific config

**Why:** Configure MCPs/plugins once, use everywhere

**Not Shared:**
- `settings.json` - Generated per agent with permissions only
- `~/.claude.json` - Generated per agent with OAuth token and onboarding flags
- `skills/` - Not mounted to avoid conflicts

### Isolated (Per Agent)

**Mounts:**
- `.hive/workspaces/<agent>/history.jsonl:/home/agent/.claude/history.jsonl`
- `.hive/workspaces/<agent>/session-env:/home/agent/.claude/session-env`

**Contains:**
- Conversation history
- Session state

**Why:** Each agent has independent conversations

### Authentication

**OAuth Token Persistence:**

Each agent gets `~/.claude.json` created with:
```json
{
  "hasCompletedOnboarding": true,
  "bypassPermissionsModeAccepted": true,
  "lastOnboardingVersion": "2.0.76",
  "oauthAccount": {
    "accessToken": "${CLAUDE_CODE_OAUTH_TOKEN}"
  }
}
```

This bypasses:
- OAuth login prompts
- Theme selection wizard
- Onboarding steps

**Why:** Agents start immediately without manual setup

## Workspace Isolation

### Git Worktrees

Each agent has its own **git worktree** (not a clone):

```
.hive/workspaces/
├── queen/              # Git worktree (detached)
│   ├── .git            # Worktree metadata
│   ├── src/
│   └── ... project files ...
├── drone-1/            # Git worktree (detached)
│   ├── .git
│   └── ... project files ...
├── drone-2/
│   └── ...
```

**Created during `hive init`:**
```bash
git worktree add --detach .hive/workspaces/queen main
git worktree add --detach .hive/workspaces/drone-1 main
git worktree add --detach .hive/workspaces/drone-2 main
```

**Advantages over clones:**
- ✅ Share `.git/` directory (faster, less disk space)
- ✅ `--detach` allows multiple worktrees on same branch
- ✅ No need to fetch/push between workspaces
- ✅ Automatic cleanup with `git worktree remove`

**Why detached mode:**
- Normal worktrees can't share the same branch
- Detached worktrees bypass this restriction
- Agents can work simultaneously on `main` branch
- No branch locking or conflicts

**Cleanup:**
```bash
hive clean  # Automatically removes all worktrees
```

### Workspace Structure

Each worktree contains:
```
.hive/workspaces/queen/
├── .git                # Worktree-specific git metadata
├── history.jsonl       # Claude conversation history
├── session-env/        # Claude session state
└── ... project files ...
```

**Why:**
- Parallel work without conflicts
- Each agent works in isolation
- Shared repository state
- No duplicate `.git/` storage

## Networking

**Mode:** `network_mode: host`

**Why:**
- Agents can access host services (DB, Redis, etc.)
- Simpler than bridge networking
- No port mapping needed

**Access:**
- Queen: `localhost:6380` → Redis
- Workers: `localhost:6380` → Redis
- All: `localhost:5432` → PostgreSQL (if running on host)

## Security

### Secrets Management

**Environment Variables:**
- `CLAUDE_CODE_OAUTH_TOKEN` - Claude auth
- `GITHUB_TOKEN` - GitHub CLI
- `GITLAB_TOKEN` - GitLab CLI
- `JIRA_API_TOKEN` - Jira CLI

**Mounted Secrets:**
- `~/.ssh` (read-only) - Git SSH keys
- `~/.aws` (read-only) - AWS credentials

### Permissions

**Claude Bypass:**
```env
CLAUDE_DANGEROUSLY_SKIP_PERMISSIONS=1
CLAUDE_CODE_BYPASS_PERMISSIONS_ACCEPTED=1
```

**Why:** Agents need to work autonomously without prompts

**Docker Socket:**
```yaml
volumes:
  - /var/run/docker.sock:/var/run/docker.sock
```

**Why:** Testcontainers, `docker` commands

**Group:** `group_add: ["0"]`

**Why:** Access Docker socket

## Scalability

### Current Limits

- **Max workers:** 10
- **Reason:** Docker Compose service definitions

### Horizontal Scaling (Future)

Replace Docker Compose with Kubernetes:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: hive-workers
spec:
  replicas: 50  # Scale to 50 workers
  template:
    spec:
      containers:
      - name: worker
        image: hive-agent
        env:
        - name: AGENT_ID
          valueFrom:
            fieldRef:
              fieldPath: metadata.name
```

## Performance

### Build Time

- **First build:** 5-10 min (downloads all tools)
- **Rebuild:** 1-2 min (cached layers)
- **Image pull (future):** 30 sec

### Startup Time

- **Cold start:** 30 sec (build + start)
- **Warm start:** 5 sec (containers exist)

### Task Throughput

**Sequential (1 agent):**
- 3 bugs = 3 hours

**Parallel (3 agents):**
- 3 bugs = 1 hour ✅

**Speedup:** ~3x (linear with workers, assuming independent tasks)

## Monitoring

### Check Status

```bash
hive status           # CLI status
hive-status           # Queen: queue status
docker compose ps     # Container status
docker compose logs   # All logs
```

### Redis Inspection

```bash
# Connect to Redis
docker exec -it hive-redis redis-cli -p 6379

# Check queue length
LLEN hive:queue:drone-1

# View task
LINDEX hive:active:drone-1 0 | jq

# Completed tasks (last 10)
ZRANGE hive:completed -10 -1 WITHSCORES
```

## Failure Recovery

### Worker Crashes

**Scenario:** Worker container crashes mid-task

**Recovery:**
```
1. Task remains in hive:active:drone-1
2. Restart worker: hive start 3
3. Worker sees active task on startup
4. Worker resumes or marks failed
```

### Redis Crashes

**Scenario:** Redis container crashes

**Recovery:**
```
1. Data persists (AOF enabled)
2. Restart: docker compose up -d redis
3. Tasks reload from disk
4. Workers reconnect automatically
```

### Network Partition

**Scenario:** Worker loses Redis connection

**Recovery:**
```
1. Worker keeps working on local task
2. Can't report done → user notices stuck task
3. Manual intervention: task-failed or reassign
```

## Best Practices

### Task Design

✅ **Do:**
- Independent tasks (no dependencies)
- Single responsibility
- Clear acceptance criteria
- Include ticket ID

❌ **Don't:**
- Sequential dependencies
- Vague descriptions
- Missing context

### Resource Management

- **Limit workers** based on CPU/RAM
- **Use minimal image** when possible
- **Stop workers** when not in use: `hive stop`

### Git Hygiene

- **One task = one branch**
- **Push frequently** (enables recovery)
- **Wait for CI** before marking done
- **Squash commits** before merge

## Future Enhancements

- [ ] Kubernetes deployment (scale to 100+ workers)
- [ ] Task dependencies (DAG execution)
- [ ] Priority queues
- [ ] Worker pools (different skill sets)
- [ ] Web UI for monitoring
- [ ] Metrics (Prometheus/Grafana)
- [ ] Auto-scaling based on queue depth
