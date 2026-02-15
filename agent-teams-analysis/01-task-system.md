# Claude Code Agent Teams: Task System

## Overview

Claude Code's Agent Teams feature uses a file-based task management system stored under `~/.claude/tasks/`. Tasks are how the team lead coordinates work across teammate agents.

## Directory Structure

```
~/.claude/tasks/<team-name>/
  .lock                    # Advisory lock file (always 0 bytes)
  .highwatermark           # Only in UUID-based solo sessions (not teams)
  1.json                   # Task file (ID = 1)
  2.json                   # Task file (ID = 2)
  ...
  tasks.json               # (Optional) Consolidated task list from team lead
```

Each team gets its own subdirectory. The `<team-name>` matches the name passed to `TeamCreate`.

### Team Name Formats

| Format | Example | When Used |
|--------|---------|-----------|
| User-chosen | `analysis-team` | Explicit name in `TeamCreate` |
| Auto-generated | `humble-chasing-goose` | No name specified (adjective-verb-animal slug) |
| UUID | `3b22f6a2-8179-...` | Solo Claude Code sessions (no team, just `TaskCreate`) |

## Task JSON Schema (Individual Files)

Each `N.json` file contains a single task object:

```json
{
  "id": "1",
  "subject": "Analyze task file structure",
  "description": "Research how tasks are stored and managed...",
  "activeForm": "Analyzing task file structure",
  "status": "in_progress",
  "blocks": [],
  "blockedBy": [],
  "owner": "task-analyst",
  "metadata": null,
  "createdAt": 1739012345678,
  "updatedAt": 1739012567890
}
```

### Field Reference

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | `string` | Yes | Numeric string matching filename (`"1"`, `"2"`, etc.) |
| `subject` | `string` | Yes | Brief task title. For work tasks: imperative form. For internal tasks: agent name |
| `description` | `string` | Yes | Full task description. Truncated to ~100 chars in internal tasks |
| `status` | `string` | No | `"pending"`, `"in_progress"`, `"completed"`, or `"deleted"` |
| `owner` | `string?` | No | Agent name assigned to this task (e.g. `"task-analyst"`) |
| `activeForm` | `string?` | No | Present continuous phrase shown while in_progress (e.g. `"Running tests"`) |
| `blocks` | `string[]` | No | Task IDs that cannot start until THIS task completes |
| `blockedBy` | `string[]` | No | Task IDs that must complete before THIS task can start |
| `metadata` | `object?` | No | Arbitrary JSON. Key convention: `{ "_internal": true }` for teammate trackers |
| `createdAt` | `number?` | No | Epoch milliseconds when task was created |
| `updatedAt` | `number?` | No | Epoch milliseconds when task was last modified |

**Serialization notes:**
- Uses `camelCase` field names (`blockedBy`, `activeForm`, `createdAt`)
- Absent optional fields are omitted entirely (not `null`)
- `id` is always a string even though it represents a number

## Internal vs User Tasks

The `metadata._internal` flag creates two fundamentally different task types that share the same ID space:

### User Tasks (Work Items)

```json
{
  "id": "1",
  "subject": "Create CONTRIBUTORS.md",
  "description": "List the project name, a placeholder contributor...",
  "activeForm": "Creating CONTRIBUTORS.md",
  "status": "in_progress",
  "owner": "contributors-writer"
}
```

- No `metadata` or `_internal` is false/absent
- `subject` describes the work to be done
- `description` has full requirements
- `activeForm` present with gerund phrase
- Created by the team lead via `TaskCreate` tool

### Internal Tasks (Teammate Trackers)

```json
{
  "id": "5",
  "subject": "task-analyst",
  "description": "You are part of the analysis-team. Your task is to analyze how Claude Code...",
  "status": "in_progress",
  "metadata": { "_internal": true }
}
```

- `metadata._internal` is `true`
- `subject` is the **agent name** (not a task title)
- `description` is the agent's initial prompt (truncated to ~100 chars in the file)
- Created automatically when a teammate is spawned via the `Task` tool
- Tracks agent existence and state — not visible as "work items"
- `owner` and `activeForm` are absent

### Shared ID Counter

Internal and user tasks share the same auto-incrementing ID:
- IDs 1-4: user work items
- IDs 5-8: internal teammate trackers

IDs never reuse, always increment.

## tasks.json Consolidated Format

An alternative format where the team lead writes all tasks into a single file:

```json
{
  "tasks": [
    {
      "id": 1,
      "title": "Implement login page",
      "description": "Build the login form with validation",
      "status": "in_progress",
      "owner": "teammate-1",
      "blockedBy": []
    }
  ]
}
```

### Differences from Individual Task Files

| Aspect | N.json (individual) | tasks.json (consolidated) |
|--------|---------------------|---------------------------|
| ID type | String (`"1"`) | Number or String |
| Title field | `subject` | `title` (aliased as `subject`) |
| Dependencies | `blockedBy` | `blockedBy` or `dependencies` (aliased) |
| Owner names | Real agent names | May use generic `teammate-N` |
| `activeForm` | Present | Not present |
| `metadata` | Present | Not present |
| Timestamps | `createdAt`/`updatedAt` | Not present |

**Note:** In practice, `tasks.json` was never observed in live data. Team leads consistently use individual `TaskCreate` calls instead.

## Task Lifecycle

### Status Transitions

```
(created) --> pending --> in_progress --> completed
                                     \-> deleted
              pending --> deleted
```

### Typical Flow (Claude Code Tools)

1. **`TaskCreate`** — Creates `N.json` with status `"pending"`, no owner
2. **`TaskUpdate`** (set `owner`) — Team lead assigns task to an agent
3. **`TaskUpdate`** (set `status: "in_progress"`) — Agent starts working
4. Agent performs work (reading files, writing code, etc.)
5. **`TaskUpdate`** (set `status: "completed"`) — Agent marks task done
6. **`TaskList`** — Agent checks for next available unblocked task

### Blocking Dependencies

- A task with non-empty `blockedBy` cannot be claimed until all blocking tasks complete
- The system recommends working on tasks in ID order (lowest first)
- `blocks` and `blockedBy` are bidirectional: if task 2 blocks task 3, then task 3's `blockedBy` contains `"2"` and task 2's `blocks` contains `"3"`

### Internal Task Lifecycle

Simpler: created as `in_progress` when agent spawns, stays `in_progress` until agent shuts down.

## Key Observations

1. **No tasks.json in practice** — Despite being supported, team leads consistently create individual files via `TaskCreate` instead of a consolidated file.

2. **Shared ID space** — Internal teammate tracking tasks and user work tasks share the same incrementing counter, interleaved.

3. **Description truncation** — Internal task descriptions are truncated to ~100 chars. The full prompt is delivered separately to the agent via the `Task` tool's `prompt` parameter.

4. **`.highwatermark` only in solo sessions** — UUID-based directories (non-team) track the next ID in `.highwatermark`. Named team directories don't have this file.

5. **`.lock` is advisory** — Always 0 bytes. Uses filesystem-level locking (flock/fcntl), not content-based.

6. **Deleted tasks stay on disk** — Setting `status: "deleted"` marks the task but the file remains. No garbage collection.

7. **`teammate-N` owner resolution** — `tasks.json` may use generic owner names like `teammate-1` which map to real agent names via the internal task ID-to-subject mapping.

8. **File-based, not database** — All state is individual JSON files on disk. Claude Code uses no database, no API server — pure filesystem.
