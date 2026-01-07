# Hive v2 - Multi-Agent Orchestration

**Run multiple Claude Code agents in parallel.** No Docker, no Redis - just git worktrees and native processes.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Go](https://img.shields.io/badge/Go-1.21+-00ADD8?logo=go)](https://go.dev)

---

## What is Hive?

Hive lets you spawn multiple Claude Code agents, each working in an isolated git worktree. A Queen (your main Claude/OpenCode instance) orchestrates Drones via MCP.

```
┌─────────────────────────────────────────────────┐
│              Queen (Claude/OpenCode)            │
│                      │ MCP                      │
└──────────────────────┼──────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────┐
│                  Hub Server                     │
│              (REST API + SSE Events)            │
└──────────────────────┬──────────────────────────┘
                       │
       ┌───────────────┼───────────────┐
       │               │               │
┌──────▼──────┐ ┌──────▼──────┐ ┌──────▼──────┐
│   Drone 1   │ │   Drone 2   │ │   Drone 3   │
│ (worktree)  │ │ (worktree)  │ │ (worktree)  │
└─────────────┘ └─────────────┘ └─────────────┘
```

**Perfect for:**
- Fixing multiple bugs simultaneously
- Developing features in parallel
- Large-scale refactoring
- Running tests while building features

---

## Installation

### Prerequisites

- Go 1.21+
- [agentapi](https://github.com/coder/agentapi) - HTTP control for Claude
- [Claude Code CLI](https://claude.ai/download) - Anthropic's CLI

### Install AgentAPI

```bash
# macOS/Linux
curl -fsSL "https://github.com/coder/agentapi/releases/latest/download/agentapi-$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m | sed 's/x86_64/amd64/;s/aarch64/arm64/')" -o ~/go/bin/agentapi
chmod +x ~/go/bin/agentapi
```

### Install Hive

```bash
# From source
git clone https://github.com/mbourmaud/hive
cd hive
make install

# Or with go install
go install github.com/mbourmaud/hive@latest
```

---

## Quick Start

### CLI Mode

```bash
# Spawn an agent
cd your-project
hive spawn frontend

# List agents
hive agents

# Send a message
hive msg frontend "Fix the login bug in src/auth.ts"

# View conversation
hive conv frontend

# Stop agent
hive kill frontend

# Stop and remove worktree
hive destroy frontend
```

### Hub Mode (for programmatic access)

```bash
# Start the Hub API server
hive hub --port 7433

# In another terminal, use the REST API
curl -X POST http://localhost:7433/agents \
  -H "Content-Type: application/json" \
  -d '{"name": "backend", "sandbox": true}'

curl -X POST http://localhost:7433/agents/AGENT_ID/message \
  -H "Content-Type: application/json" \
  -d '{"content": "Implement the user API"}'
```

### MCP Mode (for Queen integration)

Configure in Claude's settings:
```json
{
  "mcpServers": {
    "hive": {
      "command": "hive",
      "args": ["mcp"]
    }
  }
}
```

Then Queen can use tools like:
- `manage_agent` - spawn/stop/destroy/list agents
- `send_message` - send messages to agents
- `get_conversation` - get conversation history
- `manage_task` - create/track tasks
- `get_status` - overall hive status

---

## Commands

| Command | Description |
|---------|-------------|
| `hive spawn <name>` | Spawn a new agent with git worktree |
| `hive agents` | List running agents |
| `hive msg <agent> <message>` | Send message to an agent |
| `hive conv <agent>` | Show conversation history |
| `hive kill <agent>` | Stop an agent |
| `hive destroy <agent>` | Stop and remove worktree |
| `hive clean` | Remove all agents and worktrees |
| `hive hub` | Start the Hub API server |
| `hive mcp` | Start MCP server for Queen |
| `hive status` | Show overall status |
| `hive monitor` | TUI dashboard for real-time monitoring |
| `hive monitor --web` | Web dashboard (default: port 7434) |

---

## Monitor Dashboard

Hive includes a real-time monitoring dashboard available in both TUI and Web modes.

### TUI Mode

```bash
hive monitor
```

**Navigation:**
- `Tab` - Switch between Agents/Solicitations/Tasks panels
- `↑↓` or `j/k` - Navigate within a panel
- `Enter` - Select item (open agent detail view)
- `Esc` - Go back
- `r` - Refresh data
- `q` - Quit

**Agent Actions (in detail view):**
- `K` - Kill agent
- `D` - Destroy agent (removes worktree)
- `m` - Send message

**Solicitation Actions:**
- `R` - Respond to solicitation
- `X` - Dismiss solicitation

**Task Actions:**
- `s` - Start task
- `c` - Complete task
- `x` - Cancel task

### Web Mode

```bash
hive monitor --web --port 7434
```

Open http://localhost:7434 in your browser to see:
- **Agents panel** - Click to see details, conversation, and actions
- **Solicitations panel** - Click to respond or dismiss
- **Tasks panel** - Create tasks with "+ New Task" button

---

## Architecture

### Key Design Decisions

- **No Docker** - Uses git worktrees for isolation, native processes
- **No Redis** - In-memory task management, SSE for events
- **Sandbox** - Optional `@anthropic-ai/sandbox-runtime` for security
- **AgentAPI** - HTTP control of Claude via [agentapi](https://github.com/coder/agentapi)

### Components

```
internal/
├── agent/      # Agent spawning, HTTP client
├── hub/        # REST API + SSE server
├── mcp/        # MCP server for Queen
├── worktree/   # Git worktree management
├── task/       # Task tracking
└── port/       # Port allocation
```

See [docs/architecture.md](docs/architecture.md) for details.

---

## API Endpoints

### Agents
- `GET /agents` - List all agents
- `POST /agents` - Spawn new agent
- `GET /agents/{id}` - Get agent details
- `DELETE /agents/{id}` - Stop agent
- `DELETE /agents/{id}/destroy` - Stop and remove worktree

### Messages
- `POST /agents/{id}/message` - Send message
- `GET /agents/{id}/messages` - Get conversation
- `GET /agents/{id}/status` - Get agent status

### Events
- `GET /ws` - SSE event stream

### Tasks
- `GET /tasks` - List tasks
- `POST /tasks` - Create task
- `POST /tasks/{id}/start` - Start task
- `POST /tasks/{id}/complete` - Complete task

---

## Configuration

### Environment Variables

- `HIVE_DEBUG=1` - Enable debug logging
- `ANTHROPIC_API_KEY` - API key for Claude

### Spawn Options

```bash
hive spawn myagent \
  --branch feature/my-feature \
  --base main \
  --specialty backend \
  --port 3300 \
  --no-sandbox
```

---

## Development

```bash
# Build
make build

# Test
make test

# Install locally
make install
```

---

## License

MIT License - see [LICENSE](LICENSE) for details.

---

Made with Claude Code by [@mbourmaud](https://github.com/mbourmaud)
