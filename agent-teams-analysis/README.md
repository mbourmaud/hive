# Claude Code Agent Teams — Internal Analysis

A deep analysis of how Claude Code manages agent teams internally. Produced by running an actual agent team (4 analyst agents in parallel), examining live data structures, and then refining the documentation to focus purely on Claude Code's internal mechanisms.

## Analysis Documents

| File | Topic |
|------|-------|
| [01-task-system.md](01-task-system.md) | Task files, JSON schema, lifecycle, internal vs user tasks, blocking dependencies |
| [02-team-config.md](02-team-config.md) | Team config, member roster, agent types, backend types, `planModeRequired` |
| [03-message-system.md](03-message-system.md) | Inbox system, message types (plain text, idle, shutdown, task assignment), double-encoding pattern |
| [04-lifecycle.md](04-lifecycle.md) | Full team lifecycle: creation, task setup, work loop, shutdown, cleanup |

## Key Directories

| Path | Purpose |
|------|---------|
| `~/.claude/tasks/<team>/` | Task files (`1.json`, `2.json`, `.lock`) |
| `~/.claude/teams/<team>/config.json` | Team config with full member roster |
| `~/.claude/teams/<team>/inboxes/<agent>.json` | Per-agent inbox messages |

## Raw Data

The `raw/` folder contains actual files captured from live agent teams:

```
raw/
  tasks/                         # Task JSON files from multiple teams
    analysis-team/               # Our own analysis team's tasks
    humble-chasing-goose/        # Previous team run
    moonlit-chasing-meerkat/     # Previous team run
  team/                          # Team config.json files
  inboxes/                       # Inbox message JSON files per team
```

## How This Was Produced

1. Ran `TeamCreate` to create a 4-agent analysis team in Claude Code
2. Each agent researched one aspect (tasks, config, messages, lifecycle)
3. Agents read live data from `~/.claude/` directories
4. Raw data was copied to `raw/` for preservation
5. Analysis markdown was written by the agents, then refined for clarity
