# HIVE Project Instructions

## Build System - IMPORTANT

The `make build` command syncs embedded files FROM ROOT to `internal/embed/files/`:

```
entrypoint.sh        -> internal/embed/files/entrypoint.sh
start-worker.sh      -> internal/embed/files/start-worker.sh
worker-daemon.py     -> internal/embed/files/worker-daemon.py
backends.py          -> internal/embed/files/backends.py
tools.py             -> internal/embed/files/tools.py
docker/              -> internal/embed/files/docker/
scripts/             -> internal/embed/files/scripts/
templates/           -> internal/embed/files/templates/
.env.example         -> internal/embed/files/.env.example
```

**ALWAYS edit the ROOT files**, not the ones in `internal/embed/files/`. The build will overwrite those.

## Architecture

- Go CLI tool for orchestrating multi-agent Claude Code instances
- Uses Docker containers for isolation
- Git worktrees for parallel workspaces
- Redis for task queue and agent communication

## Key Files

- `entrypoint.sh` - Container entry point (edit ROOT file!)
- `docker/Dockerfile.node` - Agent container image
- `internal/compose/generator.go` - Docker Compose generation
- `cmd/` - CLI commands (init, start, stop, connect, etc.)

## Testing

```bash
make test          # Go unit tests
make test-smoke    # Quick sanity checks
make test-docker   # Docker integration
make test-git      # Git/worktree tests
make test-all      # Everything
```
