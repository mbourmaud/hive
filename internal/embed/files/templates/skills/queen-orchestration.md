# Skill: Queen Orchestration

Guide for the Queen to effectively coordinate drones and manage the Hive.

## Startup Sequence

When you start as Queen, do this:

```
1. Check Hive status
   Use bash: hive-status

2. List available drones
   Use MCP tool: hive_list_drones

3. Report summary
   👑 Queen Ready
   - Drones available: N
   - Tasks in queue: M
   - Current status: [summary]
```

## Task Assignment Strategy

### Breaking Down Work

When given a large task, break it into parallel subtasks:

```
User: "Build user authentication system"

Break into:
1. Database schema (drone-1)
2. API endpoints (drone-2)
3. Frontend forms (drone-3)
4. Tests (drone-4)
```

### Assigning Tasks

```
Use MCP tool: hive_assign_task
Arguments: {
  "worker": "drone-1",
  "title": "Create auth database schema",
  "description": "Create users table with email, password_hash, created_at. Add migration. Test with seed data.",
  "ticket_id": "AUTH-001"
}
```

**Good task descriptions:**
- Clear deliverable
- Specific files/components to modify
- Testing requirements
- Definition of done

### Task Dependencies

If tasks depend on each other:

1. Assign independent tasks first
2. Monitor for completion
3. Assign dependent tasks when ready

```
# drone-1: Build API (no dependencies)
# drone-2: Build frontend (depends on API)

# Assign drone-1 first
hive_assign_task(worker="drone-1", title="Build auth API", ...)

# Wait for drone-1 to complete
# Check status periodically
hive_drone_status(drone="drone-1")

# When done, assign drone-2
hive_assign_task(worker="drone-2", title="Build auth frontend", ...)
```

## Monitoring Drones

### Check All Drones

```
Use MCP tool: hive_list_drones
```

Shows:
- Active/idle status
- Current task
- Last activity

### Check Specific Drone

```
Use MCP tool: hive_drone_status
Arguments: { "drone": "drone-1" }
```

### Read Drone Logs

```
Use MCP tool: hive_get_drone_logs
Arguments: { "drone": "drone-1", "limit": 20 }
```

Look for:
- Progress updates
- `🚀 SERVER RUNNING` - drone started a dev server
- `🚫 BLOCKED` - drone needs help
- `✅` - task completed

## Handling Problems

### Drone is Blocked

If you see `BLOCKED` in logs:

1. Read the blocker reason
2. Provide guidance or re-assign
3. Consider splitting the task

### Drone Failed Task

When a drone marks task as failed:

1. Read the error reason
2. Decide: retry with same drone or different drone
3. Possibly break task into smaller pieces

### Drone Not Responding

If drone hasn't logged in >10 minutes:

1. Check if container is running: `docker ps`
2. Check drone logs: `hive logs drone-1`
3. Consider restarting: `docker restart hive-drone-1`

## Communication with User

### Progress Updates

Keep user informed:

```
📊 Progress Update:
- drone-1: Working on API endpoints (60%)
- drone-2: Tests complete, creating PR
- drone-3: Blocked - needs API spec clarification
```

### When Tasks Complete

```
✅ All tasks complete!

Summary:
- API: PR #123 merged
- Frontend: PR #124 ready for review
- Tests: All passing

Next steps: [recommendations]
```

## Best Practices

### 1. Equal Distribution

Spread tasks evenly across drones. Don't overload one drone.

### 2. Clear Descriptions

Include in every task:
- What to build/fix
- Where (files/components)
- How to test
- When it's done

### 3. Regular Monitoring

Check drone status every few minutes. Don't let blockers sit.

### 4. Server Awareness

When you see `🚀 SERVER RUNNING`:
- Note the port
- This means the drone's app is accessible for testing
- User can run `hive expose <port>` to access it

### 5. CI/CD Enforcement

Remind drones: **CI GREEN = DONE**

If a drone marks task complete with red CI, ask them to fix it.

## Quick Reference

```
# Startup
hive-status
hive_list_drones()

# Assign work
hive_assign_task(worker="drone-1", title="...", description="...", ticket_id="...")

# Monitor
hive_drone_status(drone="drone-1")
hive_get_drone_logs(drone="drone-1", limit=20)

# Bulk status
hive-status  # bash command for overview
```

## Task Template

```
Title: [Short action phrase]
Description: |
  ## Goal
  [What to achieve]

  ## Files
  - src/components/Login.tsx
  - src/api/auth.ts

  ## Requirements
  - [ ] Implement login form
  - [ ] Add validation
  - [ ] Connect to API

  ## Testing
  - Test with Playwright/iOS Simulator
  - Take screenshots

  ## Done when
  - PR created and CI green
Ticket: PROJ-123
```
