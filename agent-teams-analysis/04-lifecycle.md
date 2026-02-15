# Claude Code Agent Teams: Lifecycle

## Overview

This document traces the full lifecycle of a Claude Code agent team from creation through task execution to shutdown and cleanup. All behaviors are derived from observed data and the Claude Code tool specifications.

## Lifecycle Phases

```
TeamCreate → Task Setup → Spawn Workers → Work Loop → Shutdown → TeamDelete
```

---

## 1. Team Creation

### `TeamCreate` Tool

The team lead (the host Claude Code session) creates a team:

```
TeamCreate {
  team_name: "my-team",
  description: "Working on feature X"
}
```

This creates:
1. **Team directory**: `~/.claude/teams/my-team/`
2. **Config file**: `~/.claude/teams/my-team/config.json`
3. **Inboxes directory**: `~/.claude/teams/my-team/inboxes/`
4. **Task directory**: `~/.claude/tasks/my-team/`

The config starts with just the team lead as the sole member:

```json
{
  "name": "my-team",
  "description": "Working on feature X",
  "createdAt": 1770649635547,
  "leadAgentId": "team-lead@my-team",
  "leadSessionId": "3b22f6a2-...",
  "members": [
    {
      "agentId": "team-lead@my-team",
      "name": "team-lead",
      "agentType": "team-lead",
      "model": "opus",
      "joinedAt": 1770649635547,
      "tmuxPaneId": "",
      "cwd": "/path/to/project",
      "subscriptions": []
    }
  ]
}
```

The lead's `joinedAt` matches `createdAt` exactly (same millisecond).

### Team Name Formats

| Format | Example | When Used |
|--------|---------|-----------|
| User-chosen | `analysis-team` | Explicit name in `TeamCreate` |
| Auto-generated | `humble-chasing-goose` | No name specified (adjective-verb-animal slug) |

---

## 2. Task Setup

Before spawning workers, the team lead creates tasks via `TaskCreate`:

```
TaskCreate {
  subject: "Create CONTRIBUTORS.md",
  description: "List the project name, a placeholder contributor...",
  activeForm: "Creating CONTRIBUTORS.md"
}
```

This creates individual JSON files in `~/.claude/tasks/my-team/`:

```
~/.claude/tasks/my-team/
  .lock          # Advisory filesystem lock (0 bytes)
  1.json         # First task
  2.json         # Second task
  3.json         # Third task
```

Tasks start as `status: "pending"` with no `owner`.

### Task Assignment

The lead assigns tasks to workers via `TaskUpdate`:

```
TaskUpdate { taskId: "1", owner: "writer-1" }
```

### Task Dependencies

Tasks can declare blocking relationships:

```
TaskUpdate { taskId: "3", addBlockedBy: ["1", "2"] }
```

Task 3 cannot be claimed until tasks 1 and 2 are completed.

---

## 3. Spawning Workers

Workers are created via the `Task` tool with the `team_name` parameter:

```
Task {
  name: "writer-1",
  team_name: "my-team",
  subagent_type: "general-purpose",
  model: "haiku",
  mode: "bypassPermissions",
  prompt: "You are part of my-team. Create CONTRIBUTORS.md...",
  description: "Create CONTRIBUTORS.md"
}
```

When a worker is spawned:

1. **Config updated**: A new member is appended to `config.json`'s `members` array
2. **Internal task created**: An automatic task with `metadata._internal: true` tracks the worker
3. **Inbox created**: `~/.claude/teams/my-team/inboxes/writer-1.json` is created (empty array `[]`)
4. **Worker receives prompt**: The `prompt` field becomes the agent's initial instructions

### Worker Config Entry

```json
{
  "agentId": "writer-1@my-team",
  "name": "writer-1",
  "agentType": "general-purpose",
  "model": "haiku",
  "joinedAt": 1770649642000,
  "tmuxPaneId": "in-process",
  "cwd": "/path/to/project",
  "subscriptions": [],
  "prompt": "You are part of my-team. Create CONTRIBUTORS.md...",
  "color": "blue",
  "planModeRequired": false,
  "backendType": "in-process"
}
```

### Spawn Timing

Workers are added one at a time, 1-7 seconds apart (observed from `joinedAt` timestamps). Colors cycle through `blue`, `green`, `yellow`, `purple` in spawn order.

---

## 4. Work Loop

Once spawned, agents follow a cycle:

### Agent Turn

```
Agent wakes up (receives message or initial prompt)
  |
  v
Check TaskList → find available tasks (pending, unblocked, unowned)
  |
  v
TaskUpdate { taskId: "1", status: "in_progress" }
  |
  v
Perform work (Read, Write, Edit, Bash, Glob, Grep...)
  |
  v
TaskUpdate { taskId: "1", status: "completed" }
  |
  v
SendMessage to lead: "Task 1 complete."
  |
  v
Check TaskList for next task → repeat or go idle
```

### Idle State

After each turn, agents automatically go idle:
- An `idle_notification` message is sent to the team lead's inbox
- The agent waits for a new message to wake it up
- Idle is **normal behavior**, not an error — agents go idle between every turn

### Lead Coordination

The team lead:
1. Receives task completion messages and idle notifications
2. Can assign new tasks via `TaskUpdate`
3. Can send instructions via `SendMessage`
4. Can wake idle agents by sending them a message

### Plan Mode (Optional)

If a worker has `planModeRequired: true` in config:
1. Worker must call `ExitPlanMode` before implementing
2. This sends a `plan_approval_request` to the lead
3. Lead reviews and responds with `plan_approval_response` (approve/reject)
4. On approval, worker exits plan mode and can implement
5. On rejection, worker receives feedback and revises

---

## 5. Communication Patterns

### Worker → Lead

```
SendMessage {
  type: "message",
  recipient: "team-lead",
  content: "Task complete. CONTRIBUTORS.md has been created.",
  summary: "CONTRIBUTORS.md created"
}
```

### Lead → Worker

```
SendMessage {
  type: "message",
  recipient: "writer-1",
  content: "Good work. Please also add a license header.",
  summary: "Add license header"
}
```

### Worker → Worker (Peer DM)

Workers can message each other directly:
```
SendMessage {
  type: "message",
  recipient: "writer-2",
  content: "I've updated the shared config file, heads up.",
  summary: "Updated shared config"
}
```

When workers DM each other, a brief summary is included in the idle notification sent to the lead, giving the lead visibility without the full content.

### Broadcast (Lead → All)

```
SendMessage {
  type: "broadcast",
  content: "Critical: stop all work, blocking issue found.",
  summary: "Critical blocking issue"
}
```

Sends the same message to every teammate's inbox. Expensive — use sparingly.

---

## 6. Shutdown Flow

### Phase 1: Lead Initiates Shutdown

The lead sends shutdown requests to each worker:

```
SendMessage {
  type: "shutdown_request",
  recipient: "writer-1",
  content: "All tasks complete. Wrapping up."
}
```

This creates a message in the worker's inbox:
```json
{
  "text": "{\"type\":\"shutdown_request\",\"requestId\":\"shutdown-1770477661492@writer-1\",\"from\":\"team-lead\",\"reason\":\"All tasks complete. Wrapping up.\",\"timestamp\":\"2026-02-07T15:21:01.492Z\"}"
}
```

### Phase 2: Worker Responds

The worker receives the shutdown request and responds:

```
SendMessage {
  type: "shutdown_response",
  request_id: "shutdown-1770477661492@writer-1",
  approve: true
}
```

If approved:
- Worker sends confirmation to the lead
- Worker's process terminates

If rejected:
- Worker sends rejection with reason
- Worker continues working
- Lead can try again later

### Phase 3: Team Cleanup

After all workers have shut down, the lead calls `TeamDelete`:

```
TeamDelete {}
```

This removes:
- `~/.claude/teams/my-team/` (config, inboxes)
- `~/.claude/tasks/my-team/` (all task files)

### Observed Shutdown Sequence

From `humble-chasing-goose` timestamps:

```
15:20:46  docs-events sends "Task complete" to team-lead
15:20:49  docs-types sends idle_notification (available)
15:21:01  team-lead sends shutdown_request to docs-events
15:21:01  team-lead sends shutdown_request to docs-types
          (Workers approve and terminate)
          team-lead calls TeamDelete
          All team directories removed
```

The entire shutdown sequence takes seconds.

---

## 7. Task State Transitions

### User Task Lifecycle

```
TaskCreate (pending, no owner)
     |
     v
TaskUpdate (pending, owner: "writer-1")    ← Lead assigns
     |
     v
TaskUpdate (in_progress, owner: "writer-1") ← Worker starts
     |
     v
TaskUpdate (completed, owner: "writer-1")   ← Worker finishes
```

### Internal Task Lifecycle

```
Task tool spawns worker → internal task created (in_progress)
     |
     v
Worker runs → task stays in_progress
     |
     v
Worker shuts down → task may stay in_progress (no cleanup)
```

Internal tasks (`metadata._internal: true`) are never explicitly completed — they track agent existence, not work items.

### Status Values

| Status | Meaning |
|--------|---------|
| `pending` | Created, not yet started |
| `in_progress` | Actively being worked on |
| `completed` | Work is done |
| `deleted` | Marked as removed (file persists on disk) |

---

## 8. Edge Cases and Failure Modes

### Agent Interrupted

If an agent hits a context limit or encounters an error:
- An `idle_notification` with `idleReason: "interrupted"` is sent
- The agent goes idle but remains alive
- The lead can send a message to wake it and retry

### Agent Dies Mid-Task

If an agent's process terminates unexpectedly:
- No explicit cleanup occurs for that agent's tasks
- Tasks remain in their last state (`in_progress`)
- The lead may reassign the task to another agent
- No heartbeat mechanism — liveness is not actively monitored within Claude Code

### TeamDelete Before Shutdown

If the lead calls `TeamDelete` while workers are still running:
- The team directories are removed
- Workers lose their inbox files and task files
- Workers may error on their next attempt to read/write tasks
- This is considered an ungraceful shutdown

### Stale Teams

Teams can become stale if:
- The lead's Claude Code session ends without calling `TeamDelete`
- `config.json` persists with member entries but no live processes
- No automatic garbage collection — directories persist until manually cleaned

---

## 9. File State at Each Phase

### After TeamCreate

```
~/.claude/teams/my-team/
  config.json              ← Lead as sole member
  inboxes/                 ← Empty directory

~/.claude/tasks/my-team/
  .lock                    ← Advisory lock
```

### After Task Setup + Worker Spawn

```
~/.claude/teams/my-team/
  config.json              ← Lead + 3 workers
  inboxes/
    writer-1.json          ← Empty array []
    writer-2.json
    writer-3.json

~/.claude/tasks/my-team/
  .lock
  1.json                   ← User task (pending)
  2.json                   ← User task (pending)
  3.json                   ← User task (pending)
  4.json                   ← Internal task (writer-1 tracker)
  5.json                   ← Internal task (writer-2 tracker)
  6.json                   ← Internal task (writer-3 tracker)
```

### During Work

```
~/.claude/teams/my-team/
  inboxes/
    team-lead.json         ← Messages from workers (task completions, idle notifications)
    writer-1.json          ← Messages from lead (instructions, task assignments)
    writer-2.json
    writer-3.json

~/.claude/tasks/my-team/
  1.json                   ← status: "completed", owner: "writer-1"
  2.json                   ← status: "in_progress", owner: "writer-2"
  3.json                   ← status: "pending" (blocked by 1, 2)
```

### After Shutdown + TeamDelete

```
~/.claude/teams/my-team/    ← DELETED
~/.claude/tasks/my-team/    ← DELETED
```

Everything is removed. No persistent record of the team's existence.

---

## 10. Key Observations

1. **Pure file-based coordination** — No database, no API server, no network. All state is JSON files on the local filesystem.

2. **No heartbeat** — Claude Code does not actively monitor agent liveness. Dead agents are discovered only when their messages stop arriving or tasks stall.

3. **Immutable config, append-only messages** — Members are added to config but never removed. Messages are appended to inboxes but never edited or deleted. Cleanup is all-or-nothing via `TeamDelete`.

4. **Shared ID space** — Internal tasks (agent trackers) and user tasks share the same auto-incrementing counter. Task IDs 1-3 might be user work, 4-6 might be agent trackers.

5. **Idle is the normal state** — Agents spend most of their time idle, waiting for messages. Going idle after sending a message is expected behavior, not a problem.

6. **Two-phase shutdown** — Shutdown uses a request/response handshake. Agents can reject shutdown if they're still working. This is graceful but adds latency.

7. **No recovery after TeamDelete** — Once `TeamDelete` runs, all task files, config, and messages are gone. There is no built-in backup or undo mechanism.

8. **Timestamps are the primary ordering mechanism** — Messages, tasks, and member entries all use timestamps (ISO 8601 or epoch milliseconds) for ordering and sequencing.

## Raw Data

Captured lifecycle data in `raw/`:
- `raw/tasks/` — Task JSON files from 3 teams at various lifecycle stages
- `raw/team/` — Config files showing member rosters
- `raw/inboxes/` — Messages including shutdown handshakes
