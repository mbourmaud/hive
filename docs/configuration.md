# Configuration Guide

This guide covers all configuration options for Hive.

## Configuration Files

Hive uses a combination of YAML and environment files:

| File | Purpose | Committed to Git |
|------|---------|------------------|
| `hive.yaml` | Hive settings (workers, images, etc.) | ✅ Yes (recommended) |
| `hive.yaml.example` | Template for `hive.yaml` | ✅ Yes |
| `.env` | Secrets (tokens, credentials) | ❌ No (gitignored) |
| `.env.project` | Project secrets (API keys, DB) | ❌ No (gitignored) |
| `.env.example` | Template for `.env` | ✅ Yes |
| `.env.project.example` | Template for `.env.project` | ✅ Yes |

**Recommended:**
- ✅ Use `hive.yaml` for all non-secret config (version control this)
- ✅ Use `.env` for secrets only (never commit)
- ✅ Use `.env.project` for project-specific secrets

---

## YAML Configuration (hive.yaml)

**New in v0.3:** Hive now supports YAML configuration for better version control and IDE support.

### Quick Start

```bash
# Create from template
cp hive.yaml.example hive.yaml

# Or use hive init
hive init  # Creates both .env and hive.yaml
```

### Full Example

```yaml
# hive.yaml
workspace:
  name: my-project
  git_url: https://github.com/user/repo.git  # Optional

redis:
  port: 6380

agents:
  queen:
    model: opus                        # sonnet, opus, haiku
    dockerfile: docker/Dockerfile.node
    env:
      CUSTOM_QUEEN_VAR: value

  workers:
    count: 5                           # Number of workers (1-10)
    model: sonnet
    dockerfile: docker/Dockerfile.golang
    env:
      CUSTOM_WORKER_VAR: value
```

### Configuration Priority

When you run `hive start`:

1. **CLI arguments** (highest priority)
   ```bash
   hive start 8  # Uses 8 workers (ignores hive.yaml)
   ```

2. **hive.yaml** (if exists)
   ```yaml
   agents:
     workers:
       count: 5
   ```

3. **Default values** (lowest priority)
   - 2 workers, sonnet model, Node.js image

### Options

#### workspace

```yaml
workspace:
  name: my-project        # Required: workspace directory name
  git_url: ""             # Optional: auto-clone on first start
  container_prefix: ""    # Optional: prefix for container names (v1.5.0)
                          # Default: sanitized directory name (e.g., "my-project")
```

**Multi-project support (v1.5.0):** The container prefix allows running multiple Hive instances on different projects simultaneously. By default, Hive uses the project directory name (lowercase, sanitized) as the prefix.

#### redis

```yaml
redis:
  port: 6380              # Redis port (1024-65535)
```

#### agents.queen

```yaml
agents:
  queen:
    model: sonnet         # Claude model: sonnet | opus | haiku
    dockerfile: docker/Dockerfile.node
    ports:                # Optional: port mappings (v1.5.0)
      - "3000:13000"      # container:host
      - "8080:18080"
    env:                  # Optional: custom environment variables
      VAR_NAME: value
```

#### agents.workers

```yaml
agents:
  workers:
    count: 2              # Number of workers (1-10)
    model: sonnet         # Claude model
    dockerfile: docker/Dockerfile.node
    ports:                # Optional: port mappings (v1.5.0)
      - "5173:15173"      # Auto-incremented per worker
                          # drone-1: 15173, drone-2: 15174, etc.
    env:                  # Optional: custom environment variables
      VAR_NAME: value
```

#### hooks (v1.5.0)

```yaml
hooks:
  init: |                 # Shell script executed at container startup
    apt-get update && apt-get install -y postgresql-client
    npm install -g @anthropic-ai/mcp-gitlab
    pip install pandas
```

Custom init hooks let you install additional packages, configure tools, or run any setup commands when containers start.

#### mcps (v1.5.0)

```yaml
mcps:
  gitlab:
    package: "@anthropic-ai/mcp-gitlab"    # NPM package to install
    env: [GITLAB_TOKEN]                     # Required env vars (stored in .env.project)

  playwright:
    package: "@anthropic-ai/mcp-playwright"
    args: ["--headless"]                    # Optional arguments

  custom-mcp:
    command: node                           # Custom command
    args: ["/path/to/my-mcp.js"]           # Command arguments
```

Configure project-specific MCPs directly in hive.yaml instead of modifying global Claude settings.

#### playwright (v1.5.0)

```yaml
playwright:
  mode: headless          # "headless" (default) or "connect"
  browser_endpoint: ""    # WebSocket endpoint for connect mode
```

**Headless mode** (default): Browsers run inside containers using pre-installed Playwright browsers.

**Connect mode**: Attach to an external browser running on your Mac for visible debugging:

```bash
# 1. Start Chrome with remote debugging
/Applications/Google\ Chrome.app/Contents/MacOS/Google\ Chrome --remote-debugging-port=9222

# 2. Configure hive.yaml
playwright:
  mode: connect
  browser_endpoint: "ws://host.docker.internal:9222"
```

### Available Dockerfiles

```yaml
# Minimal (~500MB): Just Claude + git
dockerfile: docker/Dockerfile.minimal

# Node.js (~1.5GB): Default, full-stack web
dockerfile: docker/Dockerfile.node

# Go (~1GB): Go development
dockerfile: docker/Dockerfile.golang

# Python (~800MB): Data science
dockerfile: docker/Dockerfile.python

# Rust (~2GB): Systems programming
dockerfile: docker/Dockerfile.rust
```

### Available Models

```yaml
model: sonnet  # Fast and capable (recommended, default)
model: opus    # Most capable, slower, expensive
model: haiku   # Fastest, less capable, cheapest
```

### Validation

Hive validates your config on startup:

- ✅ `workspace.name` must not be empty
- ✅ `redis.port` must be 1024-65535
- ✅ `agents.workers.count` must be 1-10

### Examples

**Minimal config:**
```yaml
workspace:
  name: my-project
```

**Development config:**
```yaml
workspace:
  name: my-api
  git_url: https://github.com/user/my-api.git

agents:
  workers:
    count: 3
    dockerfile: docker/Dockerfile.golang
```

**Production testing:**
```yaml
agents:
  queen:
    model: opus  # Best quality for orchestration

  workers:
    count: 8     # Maximum parallelism
    model: sonnet
```

---

## Hive Configuration (.env)

Create from template:
```bash
cp .env.example .env
```

### Required Variables

#### Git Configuration
```bash
GIT_USER_EMAIL=you@example.com
GIT_USER_NAME=Your Name
```
Used for git commits in all agents.

#### Workspace
```bash
WORKSPACE_NAME=my-project
```
Name of your project workspace directory.

#### Claude Authentication
```bash
CLAUDE_CODE_OAUTH_TOKEN=your_oauth_token
```

Get token:
```bash
claude setup-token
```

### Optional Variables

#### Auto-clone Repository
```bash
GIT_REPO_URL=https://github.com/user/repo.git
```
If set, Hive will clone this repo on first start.

#### GitHub Integration
```bash
GITHUB_TOKEN=ghp_xxxxxxxxxxxx
```
Enables `gh` CLI commands in containers.

#### GitLab Integration
```bash
GITLAB_TOKEN=glpat-xxxxxxxxxxxx
GITLAB_HOST=gitlab.com
```
Enables `glab` CLI commands in containers.

#### Docker Image Selection
```bash
HIVE_DOCKERFILE=docker/Dockerfile.node
```

Available options:
- `docker/Dockerfile.minimal` (~500MB): Just Claude + git
- `docker/Dockerfile.node` (~1.5GB): Node.js 22 + pnpm + Playwright (default)
- `docker/Dockerfile.golang` (~1GB): Go 1.22 + development tools
- `docker/Dockerfile.python` (~800MB): Python 3.12 + data science libs
- `docker/Dockerfile.rust` (~2GB): Rust 1.75 + cargo tools

#### Workspace Location
```bash
HIVE_WORKSPACES=./workspaces
```
Where agent workspaces are stored on the host.

#### Tasks Location
```bash
HIVE_TASKS=./tasks
```
Where task metadata is stored.

---

## Project Secrets (.env.project)

For **your project's** secrets (API keys, database passwords, etc.):

```bash
cp .env.project.example .env.project
```

### Common Variables

#### Database
```bash
DATABASE_URL=postgresql://user:password@localhost:5432/mydb

# Or individual variables
DB_HOST=localhost
DB_PORT=5432
DB_NAME=mydb
DB_USER=myuser
DB_PASSWORD=changeme
```

#### API Keys
```bash
OPENAI_API_KEY=sk-...
AWS_ACCESS_KEY_ID=AKIA...
AWS_SECRET_ACCESS_KEY=...
STRIPE_SECRET_KEY=sk_test_...
SENDGRID_API_KEY=SG...
```

#### Authentication
```bash
JWT_SECRET=your-super-secret-jwt-key
SESSION_SECRET=your-session-secret

# OAuth
GOOGLE_CLIENT_ID=...
GOOGLE_CLIENT_SECRET=...
GITHUB_CLIENT_ID=...
GITHUB_CLIENT_SECRET=...
```

#### Application
```bash
NODE_ENV=development
API_URL=http://localhost:3000
FRONTEND_URL=http://localhost:5173
```

See [`.env.project.example`](../.env.project.example) for complete list.

### Security Best Practices

1. **Never commit `.env.project`**
   - Already in `.gitignore`
   - Contains real secrets

2. **Use `.env.project.example`**
   - Commit this to git
   - Documents required variables for your team
   - Use placeholder values, not real secrets

3. **Rotate secrets regularly**
   - API keys
   - JWT secrets
   - Database passwords

4. **Use different secrets per environment**
   - Development: weak secrets OK
   - Production: strong, unique secrets

---

## Docker Compose Overrides

For advanced customization, create `docker-compose.override.yml`:

```yaml
# docker-compose.override.yml
services:
  queen:
    environment:
      - CUSTOM_VAR=value
    volumes:
      - ./my-custom-mount:/custom

  drone-1:
    cpus: 2
    memory: 4g
```

This file is automatically loaded by Docker Compose and can override any settings.

---

## Environment Variable Priority

Variables are loaded in this order (later overrides earlier):

1. `docker-compose.yml` defaults
2. `.env` (Hive configuration)
3. `.env.project` (project secrets)
4. `docker-compose.override.yml`
5. CLI environment variables

Example:
```bash
# In .env
WORKSPACE_NAME=my-project

# Override at runtime
WORKSPACE_NAME=other-project hive start
```

---

## Per-Worker Configuration

Currently, all workers share the same configuration. To customize per worker:

1. Use `docker-compose.override.yml`:
```yaml
services:
  drone-1:
    environment:
      - PORT=3001

  drone-2:
    environment:
      - PORT=3002
```

2. Or dynamically in the worker:
```bash
# Inside drone-1
export PORT=3001
```

---

## Claude Configuration

### Shared Configuration (Selective Mounts)

Only specific subdirectories from `~/.claude/` are mounted:

```yaml
# docker-compose.yml
volumes:
  - ${HOME}/.claude/mcps:/home/agent/.claude/mcps:ro
  - ${HOME}/.claude/plugins:/home/agent/.claude/plugins:ro
  - ${HOME}/.claude/projects:/home/agent/.claude/projects
```

**Shared across all agents:**
- MCPs (`~/.claude/mcps/`) - Model Context Protocol servers
- Plugins (`~/.claude/plugins/`) - Custom Claude plugins
- Projects (`~/.claude/projects/`) - Project-specific config

**Not shared** (generated per agent):
- `settings.json` - Permissions configuration
- `~/.claude.json` - OAuth token and onboarding state
- `skills/` - Not mounted to avoid conflicts

### Per-Agent Configuration

Each agent gets these files **generated** in their container:

**`~/.claude.json`** (OAuth & onboarding):
```json
{
  "hasCompletedOnboarding": true,
  "bypassPermissionsModeAccepted": true,
  "lastOnboardingVersion": "2.0.76",
  "oauthAccount": {
    "accessToken": "${CLAUDE_CODE_OAUTH_TOKEN}"
  }
}
```

**`~/.claude/settings.json`** (permissions only):
```json
{
  "permissions": {
    "defaultMode": "bypassPermissions"
  }
}
```

**Why this approach:**
- ✅ No OAuth prompts on startup
- ✅ No theme selection wizard
- ✅ Each agent starts immediately
- ✅ Shared MCPs/plugins work everywhere
- ✅ Isolated conversation history

### Isolated Per Agent

These are mounted from `.hive/workspaces/<agent>/`:

- **Conversation history**: `history.jsonl`
- **Session state**: `session-env/`

**Why:** Each agent has independent conversations.

### Configure MCPs

MCPs must be configured on your **host machine**:

```bash
# On host (not in container)
claude mcp add playwright
claude mcp add github
```

These are then available in all agents via the shared `~/.claude/mcps/` mount.

See [mcp-setup.md](mcp-setup.md) for details.

---

## Redis Configuration

Redis is used for the task queue. Default configuration:

```yaml
# docker-compose.yml
redis:
  image: redis:7-alpine
  ports:
    - "6380:6379"  # Host:Container
```

To customize:

```yaml
# docker-compose.override.yml
services:
  redis:
    command: redis-server --maxmemory 256mb --maxmemory-policy allkeys-lru
```

---

## Network Configuration

Hive uses `network_mode: host` by default, which means:

- ✅ Agents can access services on `localhost` (PostgreSQL, Redis, etc.)
- ✅ No port mapping needed
- ✅ Simplifies service access
- ⚠️ Port conflicts possible

To use bridge networking instead:

```yaml
# docker-compose.override.yml
x-agent-common: &agent-common
  network_mode: bridge
  networks:
    - hive-network

networks:
  hive-network:
    driver: bridge
```

---

## Resource Limits

To prevent workers from consuming too much:

```yaml
# docker-compose.override.yml
services:
  drone-1:
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 4G
        reservations:
          cpus: '1'
          memory: 2G
```

---

## Logging Configuration

### Change Log Level

```yaml
# docker-compose.override.yml
services:
  queen:
    environment:
      - LOG_LEVEL=debug  # debug, info, warn, error
```

### Persist Logs

```yaml
services:
  queen:
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"
```

---

## Validation

### Validate Configuration

```bash
# Check docker-compose syntax
docker compose config

# Validate with actual .env
docker compose config | less
```

### Test Configuration

```bash
# Dry run
hive start --dry-run  # Coming soon

# Start with verbose output
docker compose up --no-start
docker compose ps
```

---

## Troubleshooting

### "env file .env not found"

**Cause**: Missing `.env` file.

**Fix**:
```bash
cp .env.example .env
# Edit .env with your values
```

### Variables not loaded

**Cause**: Containers need restart after changing `.env`.

**Fix**:
```bash
hive stop
hive start 3
```

### "Invalid value" errors

**Cause**: Syntax error in `.env`.

**Fix**:
```bash
# Check for:
# - Missing quotes around values with spaces
# - Comments on same line as value
# - Special characters not escaped

# Good:
GIT_USER_NAME="John Doe"

# Bad:
GIT_USER_NAME=John Doe  # Missing quotes
```

---

## See Also

- [FAQ](faq.md) - Common questions
- [MCP Setup](mcp-setup.md) - Configure MCPs
- [Troubleshooting](troubleshooting.md) - Fix issues
