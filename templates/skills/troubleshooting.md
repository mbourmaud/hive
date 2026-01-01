# Skill: Troubleshooting

Common issues and how to fix them.

## Connection Issues

### Redis Connection Refused

**Symptom**: `Error: connect ECONNREFUSED redis:6379`

**Fix**:
```bash
# Check Redis is running
redis-cli -h redis -a "$REDIS_PASSWORD" PING

# If not responding, check container
docker ps | grep redis

# Restart if needed
docker restart hive-redis
```

### MCP Tools Not Working

**Symptom**: MCP tools return errors or don't respond

**Fix**:
```bash
# Check MCP server is connected
# List available tools in Claude

# Verify Redis connection (MCP uses Redis)
redis-cli -h redis -a "$REDIS_PASSWORD" PING
```

## Server/Port Issues

### Server Not Accessible

**Symptom**: `curl http://localhost:PORT` fails or browser can't connect

**Checks**:
```bash
# 1. Is the server running?
pgrep -f "vite\|next\|expo"

# 2. Is it binding to 0.0.0.0?
# Server must bind to 0.0.0.0, not 127.0.0.1

# 3. Check port mapping
echo $HIVE_EXPOSED_PORTS
```

**Fix for Vite**:
```typescript
// vite.config.ts
server: {
  host: '0.0.0.0',
  port: 5173
}
```

**Fix for Expo**:
```bash
npx expo start --host 0.0.0.0 --port 8081
```

### Wrong Port in URL

**Symptom**: App loads but shows wrong bundler URL

**For Expo**, set `EXPO_PACKAGER_PROXY_URL`:
```bash
# In .hive/.env
EXPO_PACKAGER_PROXY_URL=http://localhost:18081
```

## Git Issues

### Can't Push to Remote

**Symptom**: `git push` fails with authentication error

**Fix**:
```bash
# For HTTPS, use token
git remote set-url origin https://TOKEN@github.com/user/repo.git

# For SSH, check key
ssh -T git@github.com
```

### Worktree Conflicts

**Symptom**: Can't checkout branch, dirty worktree

**Fix**:
```bash
# Stash changes
git stash

# Or reset (careful!)
git reset --hard HEAD
```

## iOS Simulator Issues

### Expo Go Not Installed

**Symptom**: `ios_open_url` does nothing

**Fix**:
```
Use MCP tool: ios_install_expo_go
Arguments: { "device": "iPhone 15" }
```

### Simulator Not Booting

**Symptom**: `ios_boot_device` fails

**Fix**:
```bash
# On host machine
xcrun simctl list devices
xcrun simctl boot "iPhone 15"

# Or try via MCP
ios_get_status()  # Check Xcode status
ios_list_devices()  # See available devices
```

### App Not Loading in Simulator

**Symptom**: Expo Go opens but app doesn't load

**Checks**:
1. Metro running? `curl http://localhost:8081`
2. Correct URL? `exp://localhost:18081` (HOST port)
3. `EXPO_PACKAGER_PROXY_URL` set?

## Playwright Issues

### Browser Can't Navigate

**Symptom**: `browser_navigate` times out

**Checks**:
1. Server running? `curl http://localhost:3000`
2. Correct port? Use HOST port (e.g., 13000 not 3000)
3. URL format: `http://localhost:13000`

### Element Not Found

**Symptom**: `browser_click` can't find element

**Fix**:
```
# Always snapshot first
Use MCP tool: browser_snapshot

# Use the element names/refs from snapshot
# Try different selectors: text, ref, role
```

## Task Issues

### No Tasks Available

**Symptom**: `hive_my_tasks` shows empty

**Checks**:
1. Queen assigned tasks? Ask Queen
2. Correct drone name? Check `$AGENT_NAME`
3. Redis connected? `redis-cli PING`

### Task Stuck as Active

**Symptom**: Can't take new task, old one stuck

**Fix**:
```bash
# Mark current task as failed
task-failed "Stuck task reset"

# Or via MCP
hive_fail_task(error="Stuck task reset")

# Then take new task
hive_take_task()
```

## Container Issues

### Out of Memory

**Symptom**: Container crashes, OOM errors

**Fix**:
```bash
# In .hive/.env
NODE_MAX_OLD_SPACE_SIZE=8192  # Increase to 8GB
```

### Container Won't Start

**Symptom**: `docker-compose up` fails

**Fix**:
```bash
# Check logs
docker logs hive-drone-1

# Rebuild
hive update --rebuild

# Clean start
hive clean && hive init
```

## Quick Diagnostic Commands

```bash
# Check everything
redis-cli -h redis -a "$REDIS_PASSWORD" PING  # Redis OK?
echo $HIVE_EXPOSED_PORTS                       # Port mappings
my-tasks                                        # Tasks assigned?
pgrep -f "vite\|next\|expo"                    # Server running?

# Reset state
task-failed "Reset"                            # Clear stuck task
git stash                                      # Clear git state
```

## Getting Help

If you're stuck:

1. **Log it**: `hive-log "🚫 BLOCKED: [reason]" error`
2. **Ask Queen**: Queen can help or reassign
3. **Check docs**: `cat ~/HIVE-CAPABILITIES.md`
4. **Read skills**: `cat ~/skills/*.md`
