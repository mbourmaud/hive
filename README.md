# 🐝 Hive - Multi-Agent Claude System

**Parallel development with Claude Code.** Run multiple AI agents simultaneously—one Queen orchestrates, workers execute tasks in parallel.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

```
┌─────────────────────────────────────────┐
│          Queen (Orchestrator)           │
│  • Analyzes requests                    │
│  • Creates subtasks                     │
│  • Monitors progress                    │
└──────────────┬──────────────────────────┘
               │
        ┌──────┼──────┐
        ↓      ↓      ↓
     ┌────┐ ┌────┐ ┌────┐
     │ W1 │ │ W2 │ │ W3 │   Workers
     └────┘ └────┘ └────┘
        │      │      │
        └──────┼──────┘
               ↓
        ┌─────────────┐
        │Redis :6380  │   Task Queue
        └─────────────┘
```

## Quick Start

**Install:**
```bash
# macOS / Linux
brew install mbourmaud/tap/hive-ai

# Or build from source
git clone https://github.com/mbourmaud/hive.git
cd hive && make install
```

**Setup in any project:**
```bash
cd your-project/
hive init    # Auto-detects git config, project type, Claude token
hive start 2 # Start Queen + 2 workers
```

That's it! `hive init` automatically:
- Detects git user.email & user.name
- Detects remote origin URL
- Detects project type (Node.js, Go, Python, Rust)
- Finds Claude token from `~/.claude`
- Creates `.hive/` folder (gitignored) with all infrastructure
- Creates `hive.yaml` config (shareable with team)

**Use:**
```bash
hive connect queen  # Open Queen terminal
hive connect 1      # Open Worker 1 terminal
hive status         # Check running containers
hive stop           # Stop all containers
hive clean          # Remove all hive files from project
```

## Example: Fix 3 Bugs in Parallel

**Terminal 1 - Queen:**
```bash
hive connect queen
```
Tell Queen:
```
Fix these bugs in parallel:
- Bug #123: Login timeout
- Bug #124: CSV export empty
- Bug #125: Email validation
```

Queen creates tasks:
```bash
hive-assign drone-1 "Fix #123" "Increase session timeout" "BUG-123"
hive-assign drone-2 "Fix #124" "Handle empty data case" "BUG-124"
hive-assign drone-3 "Fix #125" "Update regex pattern" "BUG-125"
```

**Terminal 2-4 - Workers:**
```bash
hive connect 1  # Auto-runs my-tasks, shows assigned task
# Fix the bug...
task-done       # When CI is green
```

**Result:** 3 bugs fixed in parallel instead of sequentially.

---

## Features

- ✅ **Works Anywhere**: `hive init` in any existing Git project
- ✅ **Zero Config**: Auto-detects git, project type, Claude token
- ✅ **Clean Structure**: All files in `.hive/` (gitignored), only `hive.yaml` shared
- ✅ **Multi-Agent**: 1 Queen + up to 10 workers
- ✅ **Task Queue**: Redis-based atomic task management
- ✅ **Isolated Workspaces**: Each agent has its own git worktree
- ✅ **Full Stack**: Supports Node.js, Go, Python, Rust

## Project Structure

After `hive init`, your project looks like:
```
your-project/
├── .hive/                  # All hive infrastructure (gitignored)
│   ├── docker-compose.yml
│   ├── .env                # Your secrets (token, etc.)
│   ├── docker/             # Dockerfiles
│   ├── scripts/            # Task management scripts
│   └── workspaces/         # Agent workspaces
├── hive.yaml               # Shareable config (commit this!)
└── ... your code ...
```

---

## Documentation

### Core Guides
- 📘 [**FAQ**](docs/faq.md) - Common questions and answers
- ⚙️ [**Configuration**](docs/configuration.md) - `.env` setup, secrets management
- 📋 [**Commands Reference**](docs/commands.md) - All CLI commands
- ✨ [**Best Practices**](docs/best-practices.md) - Effective parallel development

### Technical Docs
- 🏗️ [**Architecture**](docs/architecture.md) - How Hive works internally
- 🔌 [**MCP Setup**](docs/mcp-setup.md) - Configure Model Context Protocol
- 🔧 [**Troubleshooting**](docs/troubleshooting.md) - Fix common issues
- 🐳 [**Docker Images**](docker/README.md) - Available Dockerfiles

---

## Examples

Real-world examples with complete workflows:

| Language | Example | What's Included |
|----------|---------|-----------------|
| 🟢 **Node.js** | [Full-stack TypeScript](examples/nodejs-monorepo/) | User management, parallel bug fixes, refactoring |
| 🔵 **Go** | [REST API](examples/golang-api/) | CRUD handlers, search, image upload |
| 🟡 **Python** | [ML Project](examples/python-ml/) | Parallel model training, MLflow tracking |
| 🟠 **Rust** | [CLI Tool](examples/rust-cli/) | File search, parallel commands |

Each example includes:
- Complete code samples
- Task breakdown for Queen
- Timeline comparisons (sequential vs parallel)
- Troubleshooting guides

---

## Use Cases

### Feature Development
Break features into parallel tasks:
```bash
hive-assign drone-1 "Create database schema"
hive-assign drone-2 "Build REST API"
hive-assign drone-3 "Create UI components"
hive-assign drone-4 "Write tests"
```

### Bug Fixing Sprint
Fix multiple bugs simultaneously:
```bash
hive-assign drone-1 "Fix auth timeout"
hive-assign drone-2 "Fix CSV export"
hive-assign drone-3 "Fix email validation"
```

### Code Refactoring
Refactor different modules in parallel:
```bash
hive-assign drone-1 "Refactor auth module"
hive-assign drone-2 "Migrate to Prisma"
hive-assign drone-3 "Update tests"
```

---

## Requirements

- **Docker Desktop** - Installed and running
- **8GB+ RAM** - More RAM = more workers
- **10GB+ disk** - For Docker images and workspaces

**Supported OS:** macOS, Linux, Windows (WSL2)

---

## Building from Source

```bash
make build    # Build binary
make install  # Install to /usr/local/bin
make clean    # Clean build artifacts
```

---

## Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

---

## License

MIT License - see [LICENSE](LICENSE) for details.

---

## Support

- 🐛 [Report a bug](https://github.com/mbourmaud/hive/issues)
- 💡 [Request a feature](https://github.com/mbourmaud/hive/issues)
- 📖 [Read the docs](docs/)
