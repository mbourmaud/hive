# Troubleshooting Guide

Common issues and solutions.

## Installation Issues

### `make install` fails with permission denied

**Symptom:**
```
cp: /usr/local/bin/hive: Permission denied
```

**Solution:**
```bash
sudo make install
# OR install to user directory
go build -o ~/bin/hive .
export PATH="$HOME/bin:$PATH"
```

### `go: command not found`

**Symptom:**
```
make: go: Command not found
```

**Solution:**
```bash
# Install Go 1.22+
brew install go        # macOS
# OR
sudo apt install golang-go  # Linux
```

## Docker Issues

### `Cannot connect to Docker daemon`

**Symptom:**
```
Error: Cannot connect to the Docker daemon
```

**Solution:**
```bash
# Start Docker Desktop (macOS)
open -a Docker

# OR start Docker service (Linux)
sudo systemctl start docker
```

### `Port 6380 already in use`

**Symptom:**
```
Error: bind: address already in use
```

**Solution:**
```bash
# Find what's using port 6380
lsof -i :6380

# Change port in docker-compose.yml
ports:
  - "6381:6379"  # Use 6381 instead
```

### Build fails: `no space left on device`

**Symptom:**
```
Error: failed to create: no space left on device
```

**Solution:**
```bash
# Clean up Docker
docker system prune -a --volumes

# Increase Docker Desktop disk size (macOS)
# Docker Desktop → Settings → Resources → Disk image size
```

## Configuration Issues

### `.env` not found

**Symptom:**
```
env file not found: .env
```

**Solution:**
```bash
# Use hive init
hive init

# OR manually
cp .env.example .env
# Edit .env with your tokens
```

### Claude auth fails: `Invalid OAuth token`

**Symptom:**
```
Error: Invalid OAuth token
```

**Solution:**
```bash
# Get new token
claude setup-token

# Update .env
CLAUDE_CODE_OAUTH_TOKEN=<new-token>

# Restart
hive stop && hive start 3
```

## Runtime Issues

### Worker can't see tasks

**Symptom:**
```bash
my-tasks
# Shows: No tasks
```

**Check:**
```bash
# 1. Verify Redis is running
docker ps | grep redis
# Should show: hive-redis

# 2. Check Redis connection
docker exec claude-agent-1 redis-cli -h localhost -p 6380 ping
# Should return: PONG

# 3. Check queue
docker exec hive-redis redis-cli -p 6379 LLEN hive:queue:drone-1
# Should show task count
```

**Solution:**
```bash
# Restart Hive
hive stop && hive start 3
```

### Task stuck in "active"

**Symptom:**
- Task shows in `hive-status` as active
- Worker crashed or was stopped

**Solution:**
```bash
# Option 1: Resume worker
hive start 3
hive connect 1
# Worker will resume or fail the task

# Option 2: Manual recovery (from Queen)
docker exec hive-redis redis-cli -p 6379 \
  LMOVE hive:active:drone-1 hive:queue:drone-1 RIGHT LEFT

# Then reassign
hive-assign drone-2 "Continue task" "..."
```

### MCP not working

**Symptom:**
```
"I don't have access to <tool>"
```

**Check:**
```bash
# 1. Verify MCP is configured on host
ls -la ~/.claude/mcp/

# 2. Verify shared mount
docker exec claude-queen ls -la ~/.claude/mcp/
# Should show same files

# 3. Check Claude version
docker exec claude-queen claude --version
# Should be 2.0.75+
```

**Solution:**
```bash
# Reconfigure MCP on host
claude

# Restart Hive
hive stop && hive start 3
```

### Git authentication fails

**Symptom:**
```
Permission denied (publickey)
```

**Solution:**
```bash
# 1. Verify SSH key
ls -la ~/.ssh/id_rsa
# Should exist

# 2. Add to ssh-agent
ssh-add ~/.ssh/id_rsa

# 3. Test connection
ssh -T git@github.com

# 4. Restart Hive
hive stop && hive start 3
```

## Performance Issues

### Builds are slow

**Symptom:** `docker compose build` takes 10+ minutes

**Solutions:**
```bash
# 1. Use minimal image
echo "HIVE_DOCKERFILE=docker/Dockerfile.minimal" >> .env

# 2. Pull pre-built image (future)
# docker pull mbourmaud/hive:node

# 3. Enable BuildKit cache
export DOCKER_BUILDKIT=1
docker compose build
```

### Container uses too much RAM

**Symptom:** System slow, high memory usage

**Solutions:**
```bash
# 1. Reduce worker count
hive stop
hive start 2  # Instead of 10

# 2. Use lighter image
HIVE_DOCKERFILE=docker/Dockerfile.minimal

# 3. Limit container memory
# docker-compose.override.yml
services:
  agent-1:
    mem_limit: 2g
```

## Workspace Issues

### Workspace not found

**Symptom:**
```
cd: /workspace/my-project: No such file or directory
```

**Check:**
```bash
# Verify WORKSPACE_NAME in .env
grep WORKSPACE_NAME .env

# Check mount
docker exec claude-queen ls -la /workspace/
```

**Solution:**
```bash
# Set correct workspace name
# .env
WORKSPACE_NAME=correct-name

# Restart
hive stop && hive start 3
```

### Git repo not cloned

**Symptom:** Empty `/workspace/<project>` directory

**Solution:**
```bash
# Option 1: Set GIT_REPO_URL and restart
echo "GIT_REPO_URL=https://github.com/user/repo.git" >> .env
hive stop && hive start 3

# Option 2: Manual clone
docker exec -it claude-queen bash
cd /workspace
git clone https://github.com/user/repo.git my-project
exit
```

## Debug Commands

### View logs

```bash
# All containers
docker compose logs

# Specific container
docker compose logs queen
docker compose logs agent-1

# Follow logs
docker compose logs -f agent-1

# Last 50 lines
docker compose logs --tail=50 agent-1
```

### Inspect container

```bash
# Shell into container
docker exec -it claude-queen bash

# Check environment
docker exec claude-queen env | grep CLAUDE

# Check processes
docker exec claude-queen ps aux

# Check disk space
docker exec claude-queen df -h
```

### Check Redis

```bash
# Connect to Redis
docker exec -it hive-redis redis-cli -p 6379

# View all keys
KEYS hive:*

# Get task details
LINDEX hive:active:drone-1 0

# Count completed tasks
ZCARD hive:completed

# View last 5 completed
ZRANGE hive:completed -5 -1 WITHSCORES
```

## Getting Help

If you're still stuck:

1. **Check GitHub Issues:** https://github.com/mbourmaud/hive/issues
2. **Create an Issue:**
   ```bash
   gh issue create --repo mbourmaud/hive \
     --title "Bug: <description>" \
     --body "
     **Environment:**
     - OS: macOS/Linux
     - Docker version: $(docker --version)
     - Hive version: $(cd ~/Projects/hive && git log --oneline -1)

     **Issue:**
     <describe problem>

     **Logs:**
     \`\`\`
     <paste relevant logs>
     \`\`\`
     "
   ```

3. **Provide diagnostics:**
   ```bash
   # Collect debug info
   ./scripts/debug-info.sh > debug.txt
   # Attach to issue
   ```
