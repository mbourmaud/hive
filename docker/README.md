# Hive Docker Images

Hive provides modular Dockerfiles for different tech stacks.

## Available Images

### Dockerfile.node (default)
**Best for:** JavaScript/TypeScript projects

**Includes:**
- Node.js 22
- pnpm 10
- Playwright (browser automation)
- Claude Code
- gh, glab
- Docker CLI

**Size:** ~1.5 GB

### Dockerfile.minimal
**Best for:** Lightweight projects, any language

**Includes:**
- Claude Code
- git, gh, glab
- Basic utilities (jq, curl, wget)

**Size:** ~500 MB

### Dockerfile.go
**Best for:** Go projects

**Includes:**
- Go 1.22
- air (hot reload)
- gopls (language server)
- golangci-lint
- delve (debugger)
- Claude Code, gh, glab

**Size:** ~1 GB

### Dockerfile.python
**Best for:** Python projects

**Includes:**
- Python 3.12
- poetry (package manager)
- black, ruff (formatters/linters)
- mypy (type checker)
- pytest, ipython
- Claude Code, gh, glab

**Size:** ~800 MB

### Dockerfile.rust
**Best for:** Rust projects

**Includes:**
- Rust 1.75
- cargo-watch, cargo-edit, cargo-audit
- Claude Code, gh, glab

**Size:** ~2 GB

## Usage

### Option 1: Environment Variable

```bash
# .env
HIVE_DOCKERFILE=docker/Dockerfile.go
```

### Option 2: docker-compose Override

```yaml
# docker-compose.override.yml
services:
  queen:
    build:
      dockerfile: docker/Dockerfile.minimal

  agent-1:
    build:
      dockerfile: docker/Dockerfile.go

  agent-2:
    build:
      dockerfile: docker/Dockerfile.python
```

### Option 3: Build Directly

```bash
# Build specific image
docker build -f docker/Dockerfile.go -t hive-go .

# Use in docker-compose
services:
  agent-1:
    image: hive-go
```

## Customization

Create your own Dockerfile:

```dockerfile
# docker/Dockerfile.custom
FROM debian:bookworm-slim

# Install your tools
RUN apt-get update && apt-get install -y \\
    your-language-here \\
    your-tools-here

# ... (follow pattern from existing Dockerfiles)
```

Then use it:

```bash
# .env
HIVE_DOCKERFILE=docker/Dockerfile.custom
```

## Size Comparison

| Image | Size | Use Case |
|-------|------|----------|
| minimal | ~500 MB | Any language, lightweight |
| node | ~1.5 GB | JavaScript/TypeScript |
| go | ~1 GB | Go projects |
| python | ~800 MB | Python projects |
| rust | ~2 GB | Rust projects |

## Pre-built Images (coming soon)

We plan to publish pre-built images to Docker Hub:

```bash
docker pull mbourmaud/hive:node
docker pull mbourmaud/hive:minimal
docker pull mbourmaud/hive:go
# etc.
```
