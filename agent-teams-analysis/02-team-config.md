# Claude Code Agent Teams: Team Configuration & Members

## Overview

Team configuration is how Claude Code tracks which agents exist in a team, their roles, models, and capabilities. Stored as a single `config.json` file per team.

## Directory Structure

```
~/.claude/teams/<team-name>/
  config.json              # Team metadata + full member roster
  inboxes/                 # Per-agent message files (see 03-message-system.md)
    <agent-name>.json
```

## config.json Schema

```json
{
  "name": "analysis-team",
  "description": "Analyze Claude Code agent teams internals",
  "createdAt": 1770649635547,
  "leadAgentId": "team-lead@analysis-team",
  "leadSessionId": "3b22f6a2-8179-4ae8-b430-861f97e74884",
  "members": [ ... ]
}
```

### Top-Level Fields

| Field | Type | Description |
|-------|------|-------------|
| `name` | `string` | Team name (matches directory name) |
| `description` | `string` | Human-readable purpose of the team |
| `createdAt` | `number` | Epoch milliseconds when the team was created |
| `leadAgentId` | `string` | Full agent ID of the lead: `<name>@<team-name>` |
| `leadSessionId` | `string` | UUID of the team lead's Claude Code session |
| `members` | `Member[]` | Full roster including the lead |

### `leadSessionId`

This UUID identifies the Claude Code conversation/session running the team. Critical for:
- Routing messages back to the lead
- Identifying which running Claude process owns the team
- Detecting stale/dead teams

## Member Object Schema

Each member in the `members` array:

| Field | Type | Present On | Description |
|-------|------|------------|-------------|
| `agentId` | `string` | All | Globally unique: `<name>@<team-name>` |
| `name` | `string` | All | Short name (e.g. `team-lead`, `contributors-writer`) |
| `agentType` | `string` | All | Role type (see below) |
| `model` | `string` | All | Model ID (e.g. `claude-opus-4-6`, `haiku`) |
| `joinedAt` | `number` | All | Epoch ms when member was added |
| `tmuxPaneId` | `string` | All | `""` for lead, `"in-process"` for SDK-spawned workers |
| `cwd` | `string` | All | Working directory for this agent |
| `subscriptions` | `array` | All | Always `[]` (reserved for future use) |
| `prompt` | `string` | Workers only | Initial instructions/system prompt |
| `color` | `string` | Workers only | Display color (e.g. `"blue"`, `"green"`, `"yellow"`) |
| `planModeRequired` | `boolean` | Workers only | If true, agent must get lead approval before implementing |
| `backendType` | `string` | Workers only | How the agent runs (see below) |

### Why the Lead is Sparse

The team lead member lacks `prompt`, `color`, `planModeRequired`, and `backendType` because:
- It's the host Claude Code session itself
- It receives instructions from the user (no prompt needed)
- It's the coordinator, not displayed as a worker
- It IS the process (no backend needed)

## Agent Types

### `"coordinator"` / `"team-lead"`

Both refer to the team lead. The naming changed over time:

| Type | Observed In | Timeframe |
|------|-------------|-----------|
| `"coordinator"` | `humble-chasing-goose` | Earlier teams |
| `"team-lead"` | `moonlit-chasing-meerkat`, `analysis-team` | Newer teams |

Both are functionally identical — they denote the orchestrating agent.

### `"general-purpose"`

All worker agents use this type. These agents:
- Receive a prompt with their instructions
- Have access to all tools (Read, Write, Edit, Bash, Glob, Grep, etc.)
- Report back to the lead via `SendMessage`
- Run in the specified `backendType`

## Backend Types

### `"in-process"` (Only Observed Type)

All worker agents run as `"in-process"`:
- Spawned within the same process/SDK as the team lead
- Created via the `Task` tool with `team_name` parameter
- `tmuxPaneId` is set to the literal string `"in-process"`
- No separate terminal or CLI process

### Team Lead (No Backend)

The lead has no `backendType` and `tmuxPaneId: ""`. It IS the host process.

### Implied Other Backends

The `tmuxPaneId` field on all members and the `backendType` distinction suggest tmux-based backends were the original or an alternative mechanism (separate `claude` CLI processes in tmux panes). Not observed in current data.

## Team Creation Sequence

Observed from timestamps in config.json:

1. `TeamCreate` tool → writes `config.json` with lead as sole member
2. Lead's `joinedAt` == `createdAt` (same millisecond)
3. Workers added one at a time via `Task` tool with `team_name` parameter
4. Each worker gets monotonically increasing `joinedAt` (1-7 seconds apart)
5. Each worker receives `prompt`, `color`, `backendType` on creation
6. `config.json` is updated atomically as each member joins

## Observed Team Configurations

| Aspect | humble-chasing-goose | moonlit-chasing-meerkat | analysis-team |
|--------|---------------------|------------------------|---------------|
| Lead agentType | `"coordinator"` | `"team-lead"` | `"team-lead"` |
| Lead model | `sonnet` | `sonnet` | `opus` |
| Worker count | 2 | 3 | 4 |
| Worker models | `haiku` | `haiku` | `opus` |
| Colors | blue, green | blue, green, yellow | blue, green, yellow, purple |
| All in-process | Yes | Yes | Yes |

### Observations

1. **Worker models are homogeneous** — All workers in a team use the same model
2. **Colors cycle** through `blue`, `green`, `yellow`, `purple` in spawn order
3. **Lead model can differ from workers** — Sonnet lead with Haiku workers, or all Opus
4. **Auto-generated names** use `adjective-verb-animal` pattern

## Key Observations

1. **Single file, entire roster** — All team state is in one `config.json`. No separate files per member.

2. **Immutable after creation** — Members are added but never removed from config.json. Even after shutdown, member entries persist until the team directory is deleted.

3. **No heartbeat** — config.json has no "last seen" or "alive" field per member. Liveness detection must use external mechanisms (PID checking, inbox monitoring).

4. **Prompt is the contract** — The `prompt` field on workers is the complete instruction set. It's the only way the lead communicates the initial task to a worker (separate from `SendMessage`).

5. **`planModeRequired`** — When `true`, the worker must call `ExitPlanMode` and get lead approval before implementing. The lead receives a `plan_approval_request` message and responds with `plan_approval_response`.

6. **No capability restrictions in config** — The config doesn't specify which tools a worker can use. `"general-purpose"` agents get all tools. Tool restrictions are handled by the agent's `mode` parameter in the `Task` tool, not in config.json.

## Raw Data

Captured config files in `raw/team/`:
- `config-analysis-team.json` — Our analysis team (4 workers, opus)
- `config-humble-chasing-goose.json` — Earlier team (2 workers, haiku)
- `config-moonlit-chasing-meerkat.json` — Earlier team (3 workers, haiku)
