# Hive v2 - Multi-Agent Orchestration

**Run multiple Claude Code agents in parallel.** No Docker, no Redis - just git worktrees and native processes.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Go](https://img.shields.io/badge/Go-1.21+-00ADD8?logo=go)](https://go.dev)

---

## What is Hive?

Hive lets you spawn multiple Claude Code agents (Drones), each working in an isolated git worktree. A Queen (your main Claude instance) orchestrates Drones via MCP. Each Drone uses the **Ralph Loop** pattern - iterating until tasks are verified complete.

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
│ Ralph Loop  │ │ Ralph Loop  │ │ Ralph Loop  │
│ (worktree)  │ │ (worktree)  │ │ (worktree)  │
└─────────────┘ └─────────────┘ └─────────────┘
```

**Perfect for:**
- Fixing multiple bugs simultaneously
- Developing features in parallel (frontend + backend + tests)
- Large-scale refactoring with sub-agents
- Continuous iteration until all tests pass

---

## Installation

### Quick Setup (Recommended)

```bash
# Install Hive
go install github.com/mbourmaud/hive@latest

# Auto-install dependencies (agentapi, claude CLI)
hive setup

# Optional: Install desktop app (macOS)
hive install desktop
```

### Manual Installation

```bash
# From source
git clone https://github.com/mbourmaud/hive
cd hive
make install

# Check dependencies
hive setup --check
```

### What `hive setup` installs:
- **agentapi** - HTTP control layer for Claude Code
- **claude** - Claude Code CLI (via npm)

---

## Quick Start

### 1. Initialize in your project

```bash
cd your-project
hive init
```

### 2. Start the Hub

```bash
hive hub
```

### 3. Spawn a Drone

```bash
hive spawn frontend --specialty frontend
```

### 4. Send a task

```bash
hive msg frontend "Add a login form with email/password validation"
```

The Drone will:
1. Analyze the task
2. Execute with sub-agents if needed
3. Run `hive-verify` (typecheck, test, build)
4. Iterate until all checks pass
5. Commit and notify

### 5. Monitor progress

```bash
# TUI dashboard
hive monitor

# Web dashboard
hive monitor --web
# → Open http://localhost:7434

# Or use the desktop app
open '/Applications/Hive Monitor.app'
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

### Setup & Installation

| Command | Description |
|---------|-------------|
| `hive setup` | Auto-install dependencies (agentapi, claude) |
| `hive setup --check` | Check what's installed |
| `hive install desktop` | Install Hive Monitor desktop app (macOS) |
| `hive init` | Initialize Hive in current project |

### Agent Management

| Command | Description |
|---------|-------------|
| `hive spawn <name>` | Spawn a new Drone with git worktree |
| `hive agents` | List running Drones |
| `hive msg <agent> <message>` | Send message to a Drone |
| `hive conv <agent>` | Show conversation history |
| `hive logs <agent>` | Stream Drone logs in real-time |
| `hive kill <agent>` | Stop a Drone |
| `hive destroy <agent>` | Stop and remove worktree |
| `hive clean` | Remove all Drones and worktrees |

### Server & Monitoring

| Command | Description |
|---------|-------------|
| `hive hub` | Start the Hub API server |
| `hive mcp` | Start MCP server for Queen integration |
| `hive status` | Show overall status |
| `hive monitor` | TUI dashboard |
| `hive monitor --web` | Web dashboard (port 7434) |

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

## Ralph Loop Pattern

Drones execute tasks using the **Ralph Loop** pattern - a continuous iteration loop that doesn't stop until the task is verified complete.

```
RECEIVE → ANALYZE → PLAN → EXECUTE → VERIFY → (iterate if failed) → DONE
```

### Key Principles

1. **Never stop on first attempt** - always verify with `hive-verify`
2. **Parallelize with sub-agents** - spawn Task() for multi-layer work
3. **Iterate until green** - typecheck, test, build must all pass
4. **Commit atomically** - one logical change per commit

### Sub-Agent Dispatch

For full-stack tasks, Drones automatically spawn sub-agents:

```typescript
Task("contract", "Create ts-rest contract for GET /users")
Task("gateway", "Implement NestJS resolver")
Task("frontend", "Create React hook with TanStack Query")
Task("tests", "Write integration tests")
```

### Verification

Before marking complete, Drones run:

```bash
hive-verify  # Runs: typecheck → test → build
```

Only when ALL checks pass does the Drone commit and notify completion.

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
