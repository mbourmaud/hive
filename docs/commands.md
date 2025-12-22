# Command Reference

Complete reference for all Hive commands.

## Host Commands (hive CLI)

Run these on your **host machine** (not inside containers).

### hive init

Initialize Hive with interactive wizard or flags.

```bash
hive init [flags]
```

**Interactive mode** (default):
```bash
hive init
# Prompts for: email, name, token, workspace, workers
```

**Non-interactive mode**:
```bash
hive init \
  --email "you@example.com" \
  --name "Your Name" \
  --token "$CLAUDE_CODE_OAUTH_TOKEN" \
  --workspace "my-project" \
  --workers 3 \
  --no-interactive
```

**Flags:**
- `--email string`: Git user email
- `--name string`: Git user name
- `--token string`: Claude Code OAuth token
- `--workspace string`: Workspace name
- `--workers int`: Number of workers (default: 2)
- `--no-interactive`: Skip prompts (use flags)

**What it does:**
1. Creates `.env` with your configuration
2. Builds the Hive CLI binary
3. Installs to `/usr/local/bin/hive`
4. Starts Hive containers

---

### hive start

Start Hive containers (Queen + Workers + Redis).

```bash
hive start [N]
```

**Arguments:**
- `N`: Number of workers (default: 2, max: 10)

**Examples:**
```bash
hive start        # Start with 2 workers
hive start 5      # Start with 5 workers
hive start 10     # Start with maximum workers
```

**What it does:**
1. Validates `.env` exists
2. Starts Redis container
3. Starts Queen container
4. Starts N worker containers
5. Waits for all containers to be healthy

**Flags** (coming soon):
- `--build`: Force rebuild images
- `--detach`: Run in background

---

### hive stop

Stop all Hive containers.

```bash
hive stop
```

**What it does:**
1. Stops all containers (Queen, Workers, Redis)
2. Keeps workspace data in `./workspaces/`
3. Keeps task data in `./tasks/`

**Notes:**
- Does **not** delete data
- Your code is safe in `./workspaces/`
- To delete everything: `docker compose down -v`

---

### hive status

Show status of all Hive containers.

```bash
hive status
```

**Output:**
```
📊 HIVE Status
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Active Workers: 3/3
Queue Size: 0
Failed Tasks: 0

Containers:
✓ claude-queen     Running
✓ claude-drone-1   Running
✓ claude-drone-2   Running
✓ claude-drone-3   Running
✓ redis            Running

Tasks:
→ drone-1: Fix login timeout [IN_PROGRESS]
✓ drone-2: Add CSV export [DONE]
✓ drone-3: Email validation [DONE]
```

---

### hive connect

Connect to an agent's Claude Code session.

```bash
hive connect <agent-id>
```

**Arguments:**
- `agent-id`: `queen`, `1`, `2`, `3`, etc.

**Examples:**
```bash
hive connect queen    # Connect to Queen
hive connect 1        # Connect to Worker 1
hive connect 5        # Connect to Worker 5
```

**What it does:**
1. Attaches to the agent's container
2. Starts Claude Code session
3. Loads agent-specific instructions (CLAUDE.md)

**Keyboard shortcuts:**
- `Ctrl+D` or `exit`: Disconnect (container keeps running)
- `Ctrl+C`: Interrupt current command

---

## Queen Commands

Run these **inside the Queen** container.

### hive-status

Show detailed status of all workers and tasks.

```bash
hive-status
```

**Output:**
```
📊 HIVE Status
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Active Workers: 3/3
Queue Size: 2
Failed Tasks: 1

Tasks by Status:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
IN_PROGRESS:
  → drone-1: Fix login timeout (TICKET-123)
    Started: 10 minutes ago

QUEUED:
  • Add pagination API (TICKET-124)
  • Update documentation (TICKET-125)

FAILED:
  ✗ drone-2: CSV export (TICKET-126)
    Reason: Type error in export service
    Failed: 5 minutes ago

COMPLETED:
  ✓ drone-3: Email validation (TICKET-127)
    Completed: 15 minutes ago
```

---

### hive-assign

Assign a task to a worker.

```bash
hive-assign <worker> <title> <description> [ticket-id]
```

**Arguments:**
- `worker`: `drone-1`, `drone-2`, etc.
- `title`: Short task title
- `description`: Detailed description
- `ticket-id`: Optional Jira/GitHub issue ID

**Examples:**
```bash
# Minimal
hive-assign drone-1 "Fix login bug" "Increase session timeout"

# With ticket
hive-assign drone-2 \
  "Add user pagination" \
  "Implement cursor-based pagination on /users endpoint. Add tests." \
  "PROJ-456"

# Multi-line description
hive-assign drone-3 \
  "Refactor auth service" \
  "Extract auth logic to separate service.
   Update all controllers to use new service.
   Add unit tests for service methods." \
  "TECH-789"
```

**What it does:**
1. Creates task in Redis queue
2. Assigns to specific worker
3. Publishes notification
4. Worker sees task via `my-tasks`

**Best practices:**
- Clear, actionable titles
- Detailed descriptions with acceptance criteria
- Include ticket ID for traceability
- Verify task appears in `my-tasks`

---

### hive-failed

List all failed tasks.

```bash
hive-failed
```

**Output:**
```
Failed Tasks:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✗ drone-2: CSV export (TICKET-126)
  Reason: Type error in export service
  Failed: 5 minutes ago

✗ drone-4: Database migration (TECH-101)
  Reason: Prisma schema conflict
  Failed: 1 hour ago
```

**Actions:**
- Investigate failure reason
- Fix underlying issue
- Reassign task:
  ```bash
  hive-assign drone-2 "Retry CSV export" "Fix type error from previous attempt" "TICKET-126"
  ```

---

## Worker Commands

Run these **inside Worker** containers.

### my-tasks

Check assigned tasks and current active task.

```bash
my-tasks
```

**Auto-run:** This command runs automatically when you connect to a worker.

**Output (with task):**
```
📋 My Tasks (drone-1)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Active Task:
→ Fix login timeout (TICKET-123)
  Description: Increase session timeout to 30min in auth.config.ts
  Assigned: 5 minutes ago

Status: IN_PROGRESS
Next: Complete the task, then run 'task-done'
```

**Output (no task):**
```
📋 My Tasks (drone-1)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
No active task

Run 'take-task' to get next task from queue
```

---

### take-task

Get the next task from the queue.

```bash
take-task
```

**What it does:**
1. Pops next task from Redis queue (FIFO)
2. Assigns it to this worker
3. Displays task details
4. Sets status to `IN_PROGRESS`

**Output:**
```
✓ Task assigned: Add user pagination

Description: Implement cursor-based pagination on /users endpoint
Ticket: PROJ-456

Next steps:
1. Read the requirements
2. Implement the feature
3. Write tests
4. Run 'task-done' when CI is green
```

**Notes:**
- Only one task at a time per worker
- If you already have an active task, you'll get an error
- First complete or fail current task

---

### task-done

Mark current task as completed.

```bash
task-done
```

**⚠️ IMPORTANT:** Only use when:
- ✅ All tests pass
- ✅ Code builds without errors
- ✅ CI is green
- ✅ Code is committed

**What it does:**
1. Marks task as `COMPLETED`
2. Records completion time
3. Publishes notification
4. Frees worker for next task

**Output:**
```
✓ Task completed: Add user pagination

Summary:
- Started: 15 minutes ago
- Duration: 15 minutes
- Ticket: PROJ-456

Run 'take-task' to get next task
```

---

### task-failed

Mark current task as failed with reason.

```bash
task-failed "<reason>"
```

**Arguments:**
- `reason`: Clear explanation of why it failed

**Examples:**
```bash
task-failed "Type error in UserService.ts line 45"
task-failed "Blocked: need API documentation from backend team"
task-failed "Tests failing: jest expects 200 but got 500"
task-failed "Cannot reproduce bug in local environment"
```

**What it does:**
1. Marks task as `FAILED`
2. Records failure reason
3. Publishes notification
4. Frees worker for next task
5. Queen can see failure via `hive-failed`

**When to use:**
- ❌ Task cannot be completed
- ❌ Blocked by external dependency
- ❌ Tests fail and you can't fix them
- ❌ Requirements are unclear

**When NOT to use:**
- ✅ Tests pass → Use `task-done`
- ⏸️ Need help → Ask Queen, don't fail yet

---

## Redis Scripts

Low-level Redis commands for debugging.

### Enqueue Task

```bash
# Inside any container
/workspace/scripts/redis/task-enqueue.sh \
  "drone-1" \
  "Task title" \
  "Task description" \
  "TICKET-123"
```

### Check Task Status

```bash
/workspace/scripts/redis/task-status.sh drone-1
```

### Direct Redis Access

```bash
redis-cli -h localhost -p 6380

# List all keys
KEYS *

# Get task
GET task:drone-1

# Check queue
LRANGE task:queue 0 -1

# Clear queue (dangerous!)
DEL task:queue
```

---

## Git Commands

Hive pre-configures git in all containers.

### Check Configuration

```bash
git config --list | grep user
```

### Common Workflow

```bash
# Create branch
git checkout -b feature/my-feature

# Make changes
# ...

# Commit
git add .
git commit -m "feat: add feature X"

# Push
git push origin feature/my-feature

# Create PR
gh pr create --title "Add feature X" --body "..."
```

---

## GitHub CLI (gh)

Available if `GITHUB_TOKEN` is set.

```bash
# List PRs
gh pr list

# View PR
gh pr view 123

# Review PR
gh pr review 123 --approve

# Merge PR
gh pr merge 123 --squash
```

---

## GitLab CLI (glab)

Available if `GITLAB_TOKEN` is set.

```bash
# List MRs
glab mr list

# View MR
glab mr view 45

# Approve MR
glab mr approve 45

# Merge MR
glab mr merge 45
```

---

## Docker Commands

### Run Commands in Containers

```bash
# On host
docker exec -it claude-queen bash
docker exec -it claude-drone-1 sh -c "npm test"
```

### View Logs

```bash
docker compose logs queen
docker compose logs drone-1 --follow
docker compose logs redis
```

### Restart Container

```bash
docker compose restart queen
docker compose restart drone-1
```

---

## Keyboard Shortcuts

### Inside Claude Code

- `Ctrl+C`: Interrupt current command
- `Ctrl+D`: Exit Claude session (keeps container running)
- `Ctrl+L`: Clear screen
- `Ctrl+R`: Search command history
- `↑`/`↓`: Navigate command history

### When Attached to Container

- `Ctrl+P` then `Ctrl+Q`: Detach (container keeps running)
- `Ctrl+D`: Exit shell
- `Ctrl+C`: Stop current process

---

## See Also

- [FAQ](faq.md) - Common questions
- [Best Practices](best-practices.md) - Tips for effective use
- [Troubleshooting](troubleshooting.md) - Fix issues
