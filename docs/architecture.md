# Hive v2 Architecture

## Overview

Hive is a Go CLI tool for orchestrating multiple Claude Code agents. Each agent runs in an isolated git worktree with optional sandboxing via `@anthropic-ai/sandbox-runtime`.

## System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Queen (Claude/OpenCode)                 │
│                              │                                  │
│                         MCP Protocol                            │
└──────────────────────────────┼──────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│                          Hub Server                             │
│                     (REST API + SSE Events)                     │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐        │
│  │  Agent   │  │   Task   │  │ Solicita │  │   Port   │        │
│  │ Manager  │  │ Manager  │  │  -tion   │  │ Registry │        │
│  └────┬─────┘  └──────────┘  └──────────┘  └──────────┘        │
└───────┼─────────────────────────────────────────────────────────┘
        │
        │ AgentAPI (HTTP)
        │
┌───────▼─────────────────────────────────────────────────────────┐
│                        Agent Spawner                            │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    srt (sandbox-runtime)                 │   │
│  │  ┌─────────────────────────────────────────────────┐    │   │
│  │  │              agentapi server                     │    │   │
│  │  │  ┌─────────────────────────────────────────┐    │    │   │
│  │  │  │    claude --dangerously-skip-permissions │    │    │   │
│  │  │  └─────────────────────────────────────────┘    │    │   │
│  │  └─────────────────────────────────────────────────┘    │   │
│  └─────────────────────────────────────────────────────────┘   │
│                              │                                  │
│                         Git Worktree                            │
│                    (isolated filesystem)                        │
└─────────────────────────────────────────────────────────────────┘
```

## Core Components

### CLI (`cmd/`)
User-facing commands:
- `hive spawn <name>` - Spawn a new agent
- `hive agents` - List running agents
- `hive msg <agent> <message>` - Send message to agent
- `hive conv <agent>` - Show conversation history
- `hive kill <agent>` - Stop an agent
- `hive destroy <agent>` - Stop and remove worktree
- `hive clean` - Remove all agents and worktrees
- `hive hub` - Start the Hub API server
- `hive mcp` - Start MCP server for Queen integration

### Hub (`internal/hub/`)
Central coordination server:
- **REST API** - CRUD for agents, tasks, solicitations, ports
- **SSE Events** - Real-time event streaming at `/ws`
- **Agent Manager** - Spawns/stops/destroys agents
- **Task Manager** - Create/track/complete tasks
- **Solicitation Manager** - Handle agent questions
- **Port Registry** - Manage port allocations

### Agent (`internal/agent/`)
Agent lifecycle management:
- **Spawner** - Creates worktree + starts agentapi process
- **Client** - HTTP communication with AgentAPI
- **Manager** - Coordinates multiple agents

### MCP Server (`internal/mcp/`)
Model Context Protocol server for Queen:
- `manage_agent` - spawn/stop/destroy/list/get
- `send_message` - send messages to agents
- `get_conversation` - get conversation history
- `manage_task` - create/start/complete/fail tasks
- `respond_solicitation` - handle agent questions
- `manage_port` - port allocation management
- `get_status` - overall hive status

### Worktree (`internal/worktree/`)
Git worktree management:
- Create isolated worktrees per agent
- Each agent gets a dedicated branch
- Full git operations supported

## Key Design Decisions

### No Docker
Uses native processes with git worktrees for isolation:
- Faster startup (no container overhead)
- Direct filesystem access
- Native performance

### Sandbox Runtime
Optional sandboxing via `@anthropic-ai/sandbox-runtime`:
- Blocks access to sensitive files (~/.ssh, ~/.aws, ~/.gnupg)
- Network restrictions to allowed domains
- Write protection for shell configs

### AgentAPI
HTTP control of Claude via [agentapi](https://github.com/coder/agentapi):
- POST /message - Send messages
- GET /messages - Get conversation
- GET /status - Agent status
- GET /events - SSE event stream

### MCP Integration
Queen controls drones via MCP protocol:
- Stdio-based JSON-RPC communication
- Tools, Resources, and Prompts
- Embedded Hub for state management

## Event Flow

1. **Queen** calls MCP tool (e.g., `manage_agent` with action `spawn`)
2. **MCP Server** calls Hub's Agent Manager
3. **Agent Manager** calls Spawner
4. **Spawner** creates worktree + starts `srt -> agentapi -> claude`
5. **Hub** broadcasts SSE event (`agent.spawned`)
6. **MCP Server** returns result to Queen

## Port Allocation

Default ports:
- Hub API: 7433
- Agent base port: 7440 (auto-increments)
- Each agent gets next available port

## File Structure

```
internal/
├── agent/          # Agent types, spawner, client
│   ├── spawner.go  # Process spawning with sandbox
│   ├── client.go   # HTTP client for AgentAPI
│   ├── manager.go  # Multi-agent coordination
│   └── sandbox-config.json  # Embedded sandbox config
├── hub/            # Hub server
│   ├── hub.go      # Main server
│   ├── api.go      # Agent handlers
│   ├── websocket.go # SSE events
│   └── handlers_*.go # Other handlers
├── mcp/            # MCP server
│   ├── server.go   # JSON-RPC handler
│   ├── adapter.go  # Hub adapter
│   └── types.go    # MCP types
├── worktree/       # Git worktree management
├── task/           # Task management
├── solicitation/   # Agent questions
├── port/           # Port registry
└── event/          # Event dispatcher
```

## Configuration

### Sandbox Config
Embedded in spawner, with placeholders:
```json
{
  "allowPty": true,
  "filesystem": {
    "denyRead": ["{{HOME}}/.ssh", "{{HOME}}/.aws", ...],
    "allowWrite": ["/tmp", "{{HOME}}"],
    "denyWrite": ["{{HOME}}/.ssh", "{{HOME}}/.gitconfig", ...]
  },
  "network": {
    "allowedDomains": ["api.anthropic.com", "github.com", ...]
  }
}
```

### Environment Variables
- `HIVE_DEBUG=1` - Enable debug logging
- `ANTHROPIC_API_KEY` - API key for Claude

## Security

- Sandbox blocks sensitive directories
- `--dangerously-skip-permissions` for autonomous mode
- Network restricted to allowed domains
- Git credentials via environment injection
