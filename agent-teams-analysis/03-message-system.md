# Claude Code Agent Teams: Message System

## Overview

Claude Code's Agent Teams uses a file-based inbox system for inter-agent communication. Each agent has its own inbox file where all messages sent to them are stored. The system supports both plain text messages and JSON-encoded protocol messages (idle notifications, shutdown requests, task assignments).

## Directory Structure

```
~/.claude/teams/<team-name>/inboxes/
    <recipient-agent-name>.json     # One file per recipient
```

Each team has its own `inboxes/` subdirectory inside `~/.claude/teams/<team-name>/`. The inbox directory is created when the team is set up via `TeamCreate`.

### Observed Teams and Their Inbox Files

| Team | Inbox Files |
|------|-------------|
| `humble-chasing-goose` | `team-lead.json`, `docs-events.json`, `docs-types.json` |
| `moonlit-chasing-meerkat` | `team-lead.json`, `doc-writer-1.json` |
| `analysis-team` | `task-analyst.json` |

## Inbox File Structure

- **One JSON file per recipient agent**: Named `<agent-name>.json` matching the `name` field in `config.json`
- Contains **all messages sent TO that agent**, regardless of sender
- Stored as a **JSON array** of message objects

This means:
- `team-lead.json` contains all messages sent **to** the team lead (from workers)
- `docs-events.json` contains all messages sent **to** the `docs-events` agent (from lead or peers)

## Message JSON Schema

Each message in the inbox array:

```json
{
  "from": "<sender-agent-name>",
  "text": "<message-content>",
  "summary": "<short-summary>",
  "timestamp": "<ISO-8601-datetime>",
  "color": "<color-name>",
  "read": true
}
```

### Field Reference

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `from` | string | Yes | Sender agent name (e.g. `"team-lead"`, `"doc-writer-1"`) |
| `text` | string | Yes | Message content — plain text or JSON-encoded (see section 5) |
| `summary` | string | No | Short summary for UI preview. Only present on plain-text messages |
| `timestamp` | string | Yes | ISO 8601 UTC timestamp with milliseconds (e.g. `"2026-02-07T15:35:15.573Z"`) |
| `color` | string | No | Sender's assigned color from config (e.g. `"blue"`, `"green"`) |
| `read` | boolean | Yes | Whether the recipient has processed this message |

### No Explicit `to` Field

The recipient is implicit — determined by which inbox file the message is stored in. `team-lead.json` means the recipient is `team-lead`.

## Message Types

Messages fall into several categories based on their content:

### 1. Plain Text Messages (Regular Communication)

Standard inter-agent communication. The `text` field contains human-readable text. These messages include a `summary` field.

```json
{
  "from": "doc-writer-1",
  "text": "Both files already have comprehensive module-level documentation...",
  "summary": "Documentation already complete",
  "timestamp": "2026-02-07T15:35:15.573Z",
  "color": "blue",
  "read": true
}
```

Sent by agents via the `SendMessage` tool with `type: "message"`.

### 2. Idle Notification (JSON-encoded)

Sent automatically when an agent's turn ends. The `text` field contains a JSON string.

```json
{
  "from": "docs-types",
  "text": "{\"type\":\"idle_notification\",\"from\":\"docs-types\",\"timestamp\":\"2026-02-07T15:20:49.498Z\",\"idleReason\":\"available\"}",
  "timestamp": "2026-02-07T15:20:49.498Z",
  "color": "blue",
  "read": true
}
```

Inner JSON schema:
```json
{
  "type": "idle_notification",
  "from": "<agent-name>",
  "timestamp": "<ISO-8601>",
  "idleReason": "available" | "interrupted"
}
```

Key details:
- Sent automatically by Claude Code (not explicitly by the agent)
- `idleReason: "available"` = agent finished its turn normally
- `idleReason: "interrupted"` = agent was interrupted (e.g. context limit, error)
- No `summary` field on these messages

### 3. Shutdown Request (JSON-encoded)

Sent by the team lead to terminate a worker agent via `SendMessage` with `type: "shutdown_request"`.

```json
{
  "from": "team-lead",
  "text": "{\"type\":\"shutdown_request\",\"requestId\":\"shutdown-1770477661492@docs-events\",\"from\":\"team-lead\",\"reason\":\"Work is complete. Shutting down the team. Thank you!\",\"timestamp\":\"2026-02-07T15:21:01.492Z\"}",
  "timestamp": "2026-02-07T15:21:01.492Z",
  "read": true
}
```

Inner JSON schema:
```json
{
  "type": "shutdown_request",
  "requestId": "shutdown-<epoch-ms>@<recipient-name>",
  "from": "<sender-name>",
  "reason": "<human-readable-reason>",
  "timestamp": "<ISO-8601>"
}
```

The `requestId` format `shutdown-<epoch>@<agent>` encodes the timestamp and target agent, providing a unique ID for the shutdown handshake.

### 4. Shutdown Response (JSON-encoded)

Worker's response to a shutdown request. Sent via `SendMessage` with `type: "shutdown_response"`.

```json
{
  "from": "docs-events",
  "text": "{\"type\":\"shutdown_response\",\"requestId\":\"shutdown-1770477661492@docs-events\",\"approved\":true}",
  "timestamp": "2026-02-07T15:21:02.123Z",
  "read": true
}
```

Inner JSON schema:
```json
{
  "type": "shutdown_response",
  "requestId": "<matching-request-id>",
  "approved": true | false
}
```

If `approved: false`, the agent includes a `content` field explaining why it rejected shutdown.

### 5. Task Assignment (JSON-encoded)

Sent when assigning or notifying about a task. Observed in `analysis-team` inboxes.

```json
{
  "from": "task-analyst",
  "text": "{\"type\":\"task_assignment\",\"taskId\":\"1\",\"subject\":\"Analyze task file structure and lifecycle\",\"description\":\"...\",\"assignedBy\":\"task-analyst\",\"timestamp\":\"2026-02-09T15:08:11.039Z\"}",
  "timestamp": "2026-02-09T15:08:11.039Z",
  "color": "blue",
  "read": false
}
```

Inner JSON schema:
```json
{
  "type": "task_assignment",
  "taskId": "<task-id>",
  "subject": "<task-title>",
  "description": "<full-description>",
  "assignedBy": "<agent-name>",
  "timestamp": "<ISO-8601>"
}
```

### 6. Plan Approval Request (JSON-encoded)

Sent by a worker with `planModeRequired: true` after calling `ExitPlanMode`.

Inner JSON schema:
```json
{
  "type": "plan_approval_request",
  "requestId": "<unique-id>",
  "from": "<agent-name>",
  "plan": "<plan-content>"
}
```

The team lead responds with `plan_approval_response` via `SendMessage`.

### Summary of All Known Message Types

| `type` field | Direction | Trigger |
|---|---|---|
| *(none — plain text)* | Any → Any | `SendMessage` with `type: "message"` |
| `idle_notification` | Worker → Lead | Automatic when agent's turn ends |
| `shutdown_request` | Lead → Worker | `SendMessage` with `type: "shutdown_request"` |
| `shutdown_response` | Worker → Lead | `SendMessage` with `type: "shutdown_response"` |
| `task_assignment` | Any → Any | Automatic on task creation/assignment |
| `plan_approval_request` | Worker → Lead | `ExitPlanMode` (when `planModeRequired`) |
| `plan_approval_response` | Lead → Worker | `SendMessage` with `type: "plan_approval_response"` |

## Double-Encoding Pattern

The most important architectural observation. The `text` field uses **two encoding modes**:

### Plain Text
Regular messages have `text` as a plain human-readable string. These also include a `summary` field.

```json
{
  "text": "Thanks for checking! Those files were already completed in a previous commit.",
  "summary": "Good - those are already done"
}
```

### JSON-Encoded (Double-Encoded)
Protocol/system messages embed a JSON object as a **string** in the `text` field. This creates double-encoding: the outer message is JSON, and the `text` value is a JSON string that must be parsed separately.

```json
{
  "text": "{\"type\":\"idle_notification\",\"from\":\"docs-types\",...}"
}
```

### How to Distinguish

1. If `text` does not start with `{` → plain text
2. If `text` starts with `{` → attempt JSON parse
3. If parse succeeds → check the `type` field to determine message type
4. If parse fails → treat as plain text (could be a message that happens to start with `{`)

JSON-encoded messages typically do **not** have a `summary` field. Plain text messages typically **do**.

## The `read` Flag

- Messages are created with `read: false`
- Set to `read: true` after the recipient agent processes the message
- Managed entirely by Claude Code, not by external tools
- Observable state: unread messages indicate the recipient hasn't processed them yet

## Message Routing

### How Messages Are Sent

When an agent calls `SendMessage`:
1. Claude Code identifies the recipient from the `recipient` field
2. Looks up the recipient's inbox file: `~/.claude/teams/<team>/inboxes/<recipient>.json`
3. Reads the current JSON array, appends the new message object, writes back
4. The recipient sees the new message on its next turn (or immediately if idle)

### Delivery Guarantees

- Messages are file-based — no network, no queue, no database
- Writing is atomic (read → append → write) with filesystem locking
- Messages are persisted immediately — no in-memory buffering
- The team lead is automatically notified when idle teammates send messages (messages are queued and delivered when the lead's turn ends)

## Example Messages from Live Data

### Worker Reporting Task Completion

From `humble-chasing-goose/inboxes/team-lead.json`:
```json
{
  "from": "docs-events",
  "text": "Task complete. The file already has a module-level doc comment at lines 1-3. The documentation is appropriate and describes the module's purpose. Both cargo build and cargo clippy pass successfully.",
  "summary": "Task complete - events.rs docs verified",
  "timestamp": "2026-02-07T15:20:46.348Z",
  "color": "green",
  "read": true
}
```

### Team Lead Responding to Worker

From `moonlit-chasing-meerkat/inboxes/doc-writer-1.json`:
```json
{
  "from": "team-lead",
  "text": "Thanks for checking! Those files were already completed in a previous commit. Stand by while the other teammates finish their work.",
  "summary": "Good - those are already done",
  "timestamp": "2026-02-07T15:35:24.484Z",
  "read": false
}
```

### Idle Notification

From `humble-chasing-goose/inboxes/team-lead.json`:
```json
{
  "from": "docs-types",
  "text": "{\"type\":\"idle_notification\",\"from\":\"docs-types\",\"timestamp\":\"2026-02-07T15:20:49.498Z\",\"idleReason\":\"available\"}",
  "timestamp": "2026-02-07T15:20:49.498Z",
  "color": "blue",
  "read": true
}
```

### Shutdown Request

From `humble-chasing-goose/inboxes/docs-events.json`:
```json
{
  "from": "team-lead",
  "text": "{\"type\":\"shutdown_request\",\"requestId\":\"shutdown-1770477661492@docs-events\",\"from\":\"team-lead\",\"reason\":\"Work is complete. Shutting down the team. Thank you!\",\"timestamp\":\"2026-02-07T15:21:01.492Z\"}",
  "timestamp": "2026-02-07T15:21:01.492Z",
  "read": true
}
```

## Key Observations

1. **Double-encoding is the norm for protocol messages** — Any consumer must detect and parse nested JSON in the `text` field to understand message types.

2. **No explicit `to` field in inbox files** — The recipient is the filename. This is elegant but means you can't determine the recipient from the message object alone.

3. **`read` flag is managed by Claude Code** — External tools can observe it but should not modify it.

4. **Idle notifications are automatic and frequent** — Every time an agent's turn ends, an idle notification is sent. This is normal behavior, not an error state.

5. **Shutdown is a two-phase handshake** — Request (lead → worker) then response (worker → lead). The `requestId` ties them together.

6. **Messages survive until team deletion** — Inbox files persist as long as `~/.claude/teams/<team>/` exists. When `TeamDelete` is called, everything is removed.

7. **Color field is informational** — The sender's color from `config.json` is copied into each message for rendering convenience. Not all messages have it (notably, messages from the team lead often lack `color`).

8. **No message editing or deletion** — Messages are append-only. Once written, they persist until the team is deleted.

## Raw Data

Captured inbox files in `raw/inboxes/`:
- `humble-chasing-goose/` — team-lead.json, docs-events.json, docs-types.json
- `moonlit-chasing-meerkat/` — team-lead.json, doc-writer-1.json
- `analysis-team/` — task-analyst.json
