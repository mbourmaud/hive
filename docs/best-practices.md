# Best Practices

Guidelines for effective parallel development with Hive.

## Task Design

### 1. Independent Tasks First

Design tasks that can run in parallel without dependencies.

✅ **Good** (independent):
```bash
hive-assign drone-1 "Add user validation"
hive-assign drone-2 "Add product validation"
hive-assign drone-3 "Add order validation"
# All can run simultaneously
```

❌ **Bad** (sequential dependency):
```bash
hive-assign drone-1 "Create user API"
hive-assign drone-2 "Create UI that calls user API"  # Blocked!
# drone-2 cannot start until drone-1 is done
```

**Solution:**
Use mocks or work in phases:
```bash
# Phase 1: Parallel
hive-assign drone-1 "Create user API"
hive-assign drone-2 "Create UI with mock API"

# Phase 2: Integration (after Phase 1)
hive-assign drone-2 "Replace mock with real API"
```

---

### 2. Atomic Tasks

Each task should be a complete unit of work.

✅ **Good** (atomic):
```bash
hive-assign drone-1 \
  "Add user pagination API" \
  "Implement GET /users?page=1&limit=10 with cursor-based pagination. Include total count. Write integration tests." \
  "PROJ-456"
```

❌ **Too broad**:
```bash
hive-assign drone-1 "Implement user management" "..." "PROJ-456"
# Too vague, unclear scope
```

❌ **Too narrow**:
```bash
hive-assign drone-1 "Add import statement" "Import UserService at line 1" "PROJ-457"
# Too trivial, not worth a task
```

**Guidelines:**
- Task should take 30min - 3 hours
- Clear acceptance criteria
- Testable outcome
- Self-contained (one feature/bug)

---

### 3. Clear Descriptions

Provide detailed, actionable descriptions.

✅ **Good**:
```bash
hive-assign drone-1 \
  "Fix login timeout bug" \
  "Increase session timeout from 5min to 30min in auth.config.ts.
   Update SESSION_TTL constant.
   Add test: user stays logged in after 15 minutes.
   Verify: run 'npm test auth' - must pass." \
  "BUG-789"
```

❌ **Bad**:
```bash
hive-assign drone-1 "Fix login" "Fix it" "BUG-789"
# Too vague, worker doesn't know what to do
```

**Include:**
- What to change
- Where to change it
- How to verify success
- Acceptance criteria

---

## Workflow Patterns

### Pattern 1: Feature Development

Break features into layers that can be parallelized.

```bash
# Example: User Management Feature

# Layer 1: Data (parallel)
hive-assign drone-1 "Create User database schema"
hive-assign drone-2 "Create UserDTO types"

# Layer 2: Business Logic (after Layer 1)
hive-assign drone-1 "Implement UserService CRUD"
hive-assign drone-2 "Add user validation logic"

# Layer 3: API (after Layer 2)
hive-assign drone-1 "Create user REST endpoints"
hive-assign drone-2 "Add authentication middleware"

# Layer 4: UI (parallel with Layer 3)
hive-assign drone-3 "Create user list component"
hive-assign drone-4 "Create user form component"

# Layer 5: Tests (parallel)
hive-assign drone-1 "Write API integration tests"
hive-assign drone-2 "Write UI component tests"
hive-assign drone-3 "Write E2E user flow tests"
```

---

### Pattern 2: Bug Fixing Sprint

Fix multiple independent bugs in parallel.

```bash
# Triage bugs first (in Queen)
# Identify independent bugs

# Assign in parallel
hive-assign drone-1 "Fix #123: Login timeout" "..."
hive-assign drone-2 "Fix #124: CSV export empty" "..."
hive-assign drone-3 "Fix #125: Email validation" "..."
hive-assign drone-4 "Fix #126: Date picker timezone" "..."

# All workers fix bugs simultaneously
# Result: 4 bugs fixed in parallel instead of sequentially
```

---

### Pattern 3: Experimentation

Try multiple approaches in parallel.

```bash
# Example: Choose best caching strategy

hive-assign drone-1 \
  "Implement Redis caching" \
  "Add Redis cache for /users endpoint. Benchmark performance."

hive-assign drone-2 \
  "Implement in-memory caching" \
  "Add LRU cache for /users endpoint. Benchmark performance."

hive-assign drone-3 \
  "Implement HTTP caching" \
  "Add Cache-Control headers. Benchmark performance."

# Compare results, choose best approach
# Delete branches of rejected solutions
```

---

### Pattern 4: Refactoring

Refactor different modules simultaneously.

```bash
hive-assign drone-1 \
  "Refactor auth module" \
  "Extract authentication logic to AuthService. Update tests."

hive-assign drone-2 \
  "Refactor data layer" \
  "Migrate TypeORM to Prisma. Update all queries. Tests must pass."

hive-assign drone-3 \
  "Refactor error handling" \
  "Standardize error responses. Add ErrorHandler middleware."
```

**Caution:**
- Coordinate to avoid merge conflicts
- Each worker on different module
- Frequent rebases from main

---

## Code Quality

### 1. Test Before task-done

**Never** mark a task complete if tests fail.

✅ **Good workflow**:
```bash
# Inside worker
npm test           # ✓ All tests pass
npm run build      # ✓ Build succeeds
task-done          # ✓ Mark complete
```

❌ **Bad workflow**:
```bash
npm test           # ✗ 2 tests failing
task-done          # ❌ DON'T DO THIS!
```

**If tests fail:**
```bash
# Option 1: Fix the issue
npm test           # ✗ Failed
# ... fix the code ...
npm test           # ✓ Pass
task-done

# Option 2: Task failed
task-failed "Test failing: UserService.create returns 500 instead of 201"
```

---

### 2. Commit Frequently

Commit small, atomic changes.

✅ **Good**:
```bash
git commit -m "feat(api): add user validation"
# ... more work ...
git commit -m "test(api): add user validation tests"
# ... more work ...
git commit -m "docs(api): update API documentation"
```

❌ **Bad**:
```bash
# Work for 2 hours without commits
git add .
git commit -m "stuff"  # Huge commit, unclear what changed
```

**Benefits:**
- Easy to revert mistakes
- Clear history
- Better code reviews
- Easier to resolve conflicts

---

### 3. Follow Conventions

Use consistent commit messages and code style.

**Commit format:**
```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**Examples:**
```bash
feat(api): add user pagination
fix(auth): increase session timeout to 30min
refactor(db): migrate to Prisma ORM
test(users): add integration tests for CRUD
docs(readme): update installation instructions
chore(deps): update dependencies
```

---

## Coordination

### 1. Queen Monitors Progress

Queen should regularly check worker status.

```bash
# In Queen
hive-status

# Check every 15-30 minutes
# Identify:
# - Blocked workers
# - Failed tasks
# - Idle workers
```

---

### 2. Workers Communicate Issues

Don't struggle in silence.

❌ **Bad** (silent failure):
```bash
# Worker stuck for 30 minutes trying to fix issue
# No communication to Queen
```

✅ **Good** (communicate):
```bash
# Worker (after 10 minutes stuck)
task-failed "Blocked: need database schema from drone-1"

# Or ping Queen via git/PR comments
```

---

### 3. Coordinate Merges

To avoid conflicts:

**Strategy A: Feature Branches**
```bash
# Each worker on own branch
drone-1: feature/auth
drone-2: feature/pagination
drone-3: feature/search

# Merge to main when done
# Conflicts are rare
```

**Strategy B: Frequent Rebases**
```bash
# Inside worker
git fetch origin
git rebase origin/main
# Resolve conflicts early and often
```

---

## Performance

### 1. Don't Overload Workers

Match worker count to available resources.

```bash
# Check system resources
docker stats

# If containers are slow:
# - Reduce worker count
# - Use lighter Docker image (minimal instead of node)
# - Close other applications
```

**Guidelines:**
- Each worker: ~1-2GB RAM
- Leave 4GB for host OS
- Example: 16GB RAM → max 6 workers

---

### 2. Stagger Heavy Operations

Don't run intensive tasks simultaneously.

❌ **Bad** (simultaneous builds):
```bash
# All workers build at same time
drone-1: npm run build  # Heavy CPU
drone-2: npm run build  # Heavy CPU
drone-3: npm run build  # Heavy CPU
# System grinds to a halt
```

✅ **Good** (staggered):
```bash
# Workers build sequentially or stagger start
drone-1: npm run build   # Starts now
drone-2: npm test        # Light task first
drone-3: npm run lint    # Light task first

# Later, drone-2 and drone-3 build when drone-1 is done
```

---

### 3. Use Appropriate Docker Image

Choose the smallest image that works.

```bash
# Minimal (500MB): Basic projects
HIVE_DOCKERFILE=docker/Dockerfile.minimal

# Node (1.5GB): Full-stack web apps
HIVE_DOCKERFILE=docker/Dockerfile.node

# Go (1GB): Go projects
HIVE_DOCKERFILE=docker/Dockerfile.golang

# Rust (2GB): Rust projects
HIVE_DOCKERFILE=docker/Dockerfile.rust
```

**Impact:**
- Faster startup
- Less disk space
- Better performance

---

## Security

### 1. Never Commit Secrets

**Always check before committing:**

```bash
# Before git add
grep -r "API_KEY" .
grep -r "password" .
grep -r "secret" .

# Use pre-commit hooks
# Add secrets to .env.project (gitignored)
```

---

### 2. Rotate Secrets

Change secrets regularly:

```bash
# Rotate every 90 days
# 1. Generate new secret
# 2. Update .env.project
# 3. Restart containers: hive stop && hive start
# 4. Update production
```

---

### 3. Use Different Secrets Per Environment

```bash
# .env.project (development)
JWT_SECRET=dev-secret-weak-is-ok

# Production (use secrets manager)
JWT_SECRET=prod-strong-random-secret-32-chars-min
```

---

## Troubleshooting

### 1. Worker Stuck

**Symptoms:**
- Task shows `IN_PROGRESS` for >1 hour
- Worker not responding

**Diagnosis:**
```bash
# On host
hive connect 1

# Inside container
ps aux              # Check running processes
docker stats        # Check resource usage
```

**Solutions:**
```bash
# Option 1: Mark failed and reassign
task-failed "Worker stuck, reassigning"
# Queen assigns to another worker

# Option 2: Restart worker
# On host
docker compose restart drone-1
```

---

### 2. Merge Conflicts

**Prevention:**
- Work on different files/modules
- Rebase frequently
- Communicate with team

**Resolution:**
```bash
# Inside worker with conflict
git fetch origin
git rebase origin/main

# Resolve conflicts manually
git add .
git rebase --continue

# Push (may need force push on feature branch)
git push --force-with-lease
```

---

### 3. Task Queue Stuck

**Symptoms:**
- `take-task` returns nothing
- Queue appears frozen

**Diagnosis:**
```bash
# Inside any container
redis-cli -h localhost -p 6380

# Check queue
LRANGE task:queue 0 -1

# Check for corrupted tasks
KEYS task:*
```

**Fix:**
```bash
# Clear queue (⚠️ deletes all tasks)
redis-cli -h localhost -p 6380 DEL task:queue

# Reassign tasks from Queen
```

---

## Scaling

### When to Add More Workers

Add workers when:
- ✅ You have many independent tasks queued
- ✅ Current workers are busy with long-running tasks
- ✅ System resources allow (RAM, CPU)

**Don't** add workers when:
- ❌ Tasks depend on each other (serial execution needed)
- ❌ System is already slow (resource constrained)
- ❌ Only 1-2 tasks remaining

---

### Optimal Worker Count

```
Optimal Workers = min(
  Available Tasks,
  Available RAM / 2GB,
  Available CPU Cores
)
```

**Examples:**
- 8GB RAM → max 2-3 workers
- 16GB RAM → max 6-8 workers
- 32GB RAM → max 10 workers (Hive max)

---

## Examples by Language

See language-specific best practices:

- [Node.js](../examples/nodejs-monorepo/) - Monorepo, TypeScript, testing
- [Go](../examples/golang-api/) - REST API, gRPC, concurrency
- [Python](../examples/python-ml/) - ML workflows, experiment tracking
- [Rust](../examples/rust-cli/) - CLI tools, performance optimization

---

## Hive-lite: Parallel Work with hive.sh

When using `hive.sh` for multi-Ralph orchestration via git worktrees, follow these best practices for coordinating parallel work.

### Branch Sharing Strategies

**Strategy A: One Ralph per Branch (Recommended)**

The safest approach - each Ralph works on its own branch.

```bash
# Ralph 1: Auth feature
hive.sh spawn ralph-auth --create feature/auth --from main

# Ralph 2: Search feature
hive.sh spawn ralph-search --create feature/search --from main

# Ralph 3: UI improvements
hive.sh spawn ralph-ui --create feature/ui --from main

# All work independently, merge via PRs
```

**Strategy B: Multiple Ralphs on Same Branch (With Scope)**

When you need multiple workers on the same branch, use scope restrictions.

```bash
# First Ralph: Backend work on auth branch
hive.sh spawn ralph-backend --create feature/auth --from main
hive.sh start ralph-backend "Implement auth API endpoints"

# Second Ralph: Frontend work on SAME branch (scoped)
hive.sh spawn ralph-frontend --attach feature/auth --scope "src/components/*"
hive.sh start ralph-frontend "Implement login form UI"

# Scopes prevent file conflicts:
# - ralph-backend: can modify any file
# - ralph-frontend: restricted to src/components/*
```

**Scope Pattern Examples:**
```bash
--scope "src/api/*"           # Only API files
--scope "src/components/**/*" # All component files (recursive)
--scope "tests/**/*.spec.ts"  # Only test files
--scope "*.go"                # Only Go files in root
--scope "internal/auth/*"     # Only auth module
```

### Coordination Guidelines

**Before Creating a PR:**

1. **Check branch status**: `hive.sh status` shows which Ralphs share branches
2. **Stop other Ralphs**: If multiple Ralphs are on your branch, stop them first
3. **Sync if needed**: `hive.sh sync <name>` pulls latest changes

```bash
# Check who's on your branch
hive.sh status

# If ralph-frontend is still running on your branch:
hive.sh stop ralph-frontend

# Then create your PR
hive.sh pr ralph-backend
```

**Warning System:**

The `hive.sh pr` command warns you when:
- Multiple Ralphs are attached to the same branch
- Other Ralphs are **actively running** (stronger warning)
- Shows `[ACTIVE]` indicator for running processes

```
[WARN] Multiple Ralphs (2) are attached to branch 'feature/auth':
  - ralph-backend (status: stopped)
  - ralph-frontend (status: running) [ACTIVE]

[ERROR] WARNING: 1 other Ralph(s) are ACTIVELY RUNNING on this branch!
[WARN] Creating a PR while other Ralphs are running may cause conflicts.
[INFO] Consider stopping them first with: hive.sh stop <name>
```

### Conflict Prevention

**When Scopes Overlap:**

If two Ralphs might edit the same files:
1. Stop one Ralph before the other continues
2. Use `hive.sh sync <name>` to pull latest changes
3. Let one Ralph complete before starting the other on same files

**After PR Merge:**

When a PR is merged, other Ralphs on that base branch may have stale code:

```bash
# Ralph-B's PR was merged to main

# Ralph-A needs to sync with updated main
hive.sh sync ralph-a

# If conflicts occur, hive.sh shows instructions
```

### Cleanup Patterns

**After Successful Work:**
```bash
# PR was merged - clean up completely
hive.sh cleanup ralph-auth  # Removes worktree, branches, config entry
```

**Abandoning Work:**
```bash
# Work didn't pan out - discard all changes
hive.sh clean ralph-experiment  # Removes everything, warns about lost commits
```

**Before Creating New Work:**
```bash
# Check for orphaned Ralphs
hive.sh status

# Clean up any stuck/dead Ralphs
hive.sh clean ralph-old --force
```

### Dashboard Monitoring

Use the live dashboard to monitor multiple Ralphs:

```bash
hive.sh dashboard          # Auto-refresh every 5 seconds
hive.sh dashboard -i 10    # Refresh every 10 seconds
```

The dashboard shows:
- Real-time status of all Ralphs
- Branch sharing warnings (⚠ indicator)
- PR status from GitHub
- Last activity from logs

### Anti-Patterns to Avoid

❌ **Don't run multiple Ralphs on same branch without scope:**
```bash
# BAD - both can edit any file, conflicts likely
hive.sh spawn ralph-1 --attach feature/auth
hive.sh spawn ralph-2 --attach feature/auth
```

❌ **Don't create PR while other Ralphs are running:**
```bash
# BAD - running Ralph may commit after PR is created
hive.sh pr ralph-1  # Creates PR
# Meanwhile ralph-2 is still committing to the branch
```

❌ **Don't forget to sync after merges:**
```bash
# BAD - Ralph has stale code after main was updated
hive.sh start ralph-old  # Working on outdated base
```

✅ **Do use explicit scopes:**
```bash
# GOOD - clear separation of concerns
hive.sh spawn ralph-api --attach feature/big --scope "src/api/*"
hive.sh spawn ralph-ui --attach feature/big --scope "src/ui/*"
```

✅ **Do stop Ralphs before PR:**
```bash
# GOOD - clean state before PR
hive.sh stop ralph-helper
hive.sh pr ralph-main
```

---

## See Also

- [Commands Reference](commands.md) - All available commands
- [FAQ](faq.md) - Common questions
- [Troubleshooting](troubleshooting.md) - Fix issues
