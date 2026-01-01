# Skill: Hive Task Workflow

Complete guide for managing tasks in the Hive system.

## For Workers (Drones)

### Startup Sequence

When you start, ALWAYS do this first:

```
1. Start monitoring (FIRST!)
   Use MCP tool: hive_start_monitoring

2. Check your tasks
   Use MCP tool: hive_my_tasks

3. Report status
   🐝 Drone Ready
   - Identity: $AGENT_NAME
   - Monitoring: ✅ Active
   - Tasks: [active/queued/none]
```

### Taking Tasks

```
# Check what's assigned to you
Use MCP tool: hive_my_tasks

# Pick up the next queued task
Use MCP tool: hive_take_task
```

### Completing Tasks

**Only mark complete when CI is GREEN!**

```
# Success - task is done
Use MCP tool: hive_complete_task
Arguments: { "result": "Feature implemented. PR #123 merged. CI green." }

# Failed - something went wrong
Use MCP tool: hive_fail_task
Arguments: { "error": "Could not fix bug - requires database migration" }
```

### Logging Progress

Keep the Queen informed:

```
Use MCP tool: hive_log_activity
Arguments: { "message": "📋 Starting: Implement login form", "level": "info" }

Use MCP tool: hive_log_activity
Arguments: { "message": "🚀 SERVER RUNNING: http://localhost:3000", "level": "info" }

Use MCP tool: hive_log_activity
Arguments: { "message": "🚫 BLOCKED: Need API credentials", "level": "error" }
```

### Auto-Worker Mode (Polling)

When you have no tasks, poll every 5 seconds:

```
Use MCP tool: hive_get_monitoring_events

# If "new_task_available" event:
Use MCP tool: hive_take_task
# ... work on task ...
Use MCP tool: hive_complete_task
```

## For Queen (Orchestrator)

### Assigning Tasks

```
Use MCP tool: hive_assign_task
Arguments: {
  "worker": "drone-1",
  "title": "Fix login bug",
  "description": "Session timeout is too short. Increase to 24h. Test the fix.",
  "ticket_id": "BUG-123"
}
```

### Monitoring Drones

```
# List all drones and their status
Use MCP tool: hive_list_drones

# Get detailed status of a specific drone
Use MCP tool: hive_drone_status
Arguments: { "drone": "drone-1" }

# Read drone's activity logs
Use MCP tool: hive_get_drone_logs
Arguments: { "drone": "drone-1", "limit": 50 }
```

### Reading Config

```
Use MCP tool: hive_get_config
```

Returns the hive.yaml configuration.

## Bash Alternatives

If MCP is unavailable, use bash commands:

| MCP Tool | Bash Command |
|----------|--------------|
| `hive_my_tasks` | `my-tasks` |
| `hive_take_task` | `take-task` |
| `hive_complete_task` | `task-done` |
| `hive_fail_task` | `task-failed "message"` |
| `hive_log_activity` | `hive-log "message"` |

## Task States

| State | Meaning |
|-------|---------|
| `queued` | Waiting to be picked up |
| `active` | Currently being worked on |
| `completed` | Successfully finished |
| `failed` | Could not complete |

## Definition of Done

A task is ONLY complete when:

1. Code changes committed and pushed
2. CI/CD pipeline is GREEN
3. All tests pass
4. No linting errors
5. PR created (if required)

**If CI is red → DO NOT mark as done. Fix it first.**

## Quick Reference

```
# Worker startup
hive_start_monitoring()
hive_my_tasks()

# Work on task
hive_take_task()
# ... do the work ...
hive_log_activity(message="Progress update")
hive_complete_task(result="Done. PR #123")

# Queen assign
hive_assign_task(worker="drone-1", title="Fix bug", description="...")
hive_list_drones()
hive_get_drone_logs(drone="drone-1")
```
