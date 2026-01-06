# HIVE Project Instructions

## Architecture (v2)

- Go CLI tool for orchestrating multiple Claude Code agents
- Uses git worktrees for filesystem isolation (no Docker)
- AgentAPI for HTTP control of agents
- Claude Code's native sandbox (Seatbelt/Bubblewrap) for security

## Key Packages

- `internal/worktree/` - Git worktree management
- `internal/agent/` - Agent types, HTTP client, process spawner
- `internal/hub/` - Hub API server (REST + SSE)
- `cmd/` - CLI commands (spawn, agents, msg, kill, etc.)

## Commands

```bash
hive init            # Initialize Hive in current repo
hive spawn <name>    # Spawn a new agent with git worktree
hive agents          # List running agents
hive msg <agent> <m> # Send message to agent
hive conv <agent>    # Show conversation history
hive kill <agent>    # Stop an agent
hive destroy <agent> # Stop and remove worktree
hive clean           # Remove all agents and worktrees
hive hub             # Start the Hub API server
```

## Testing

```bash
make test            # Go unit tests
make test-all        # All tests
go test ./...        # Run tests directly
```

## Build

```bash
make build           # Build hive binary
make install         # Install to ~/.local/bin
```
