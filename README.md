<p align="center">
  <img src="assets/logo.png" alt="Hive Logo" width="180">
</p>

<h1 align="center">Hive</h1>

<p align="center">
  <strong>Let Your Bees Do the Work! 🍯</strong>
</p>

<p align="center">
  Launch autonomous Claude agents that buzz through your plans while you sip coffee.
</p>

<p align="center">
  <a href="https://opensource.org/licenses/MIT"><img src="https://img.shields.io/badge/License-MIT-F5A623.svg" alt="License: MIT"></a>
  <a href="https://github.com/mbourmaud/hive/releases"><img src="https://img.shields.io/github/v/release/mbourmaud/hive?color=F5A623" alt="Release"></a>
  <a href="https://github.com/mbourmaud/hive/actions"><img src="https://github.com/mbourmaud/hive/workflows/CI/badge.svg" alt="CI"></a>
</p>

---

## 📦 Install

### Homebrew (macOS/Linux)

```bash
brew tap mbourmaud/tap
brew install hive-ai
hive init  # Initialize + install Claude Code skills
```

### Direct Download

```bash
# macOS (Apple Silicon)
curl -fsSL https://github.com/mbourmaud/hive/releases/latest/download/hive-darwin-arm64.tar.gz | tar xz
sudo mv hive /usr/local/bin/

# macOS (Intel)
curl -fsSL https://github.com/mbourmaud/hive/releases/latest/download/hive-darwin-amd64.tar.gz | tar xz
sudo mv hive /usr/local/bin/

# Linux (x86_64)
curl -fsSL https://github.com/mbourmaud/hive/releases/latest/download/hive-linux-amd64.tar.gz | tar xz
sudo mv hive /usr/local/bin/

# Then initialize
hive init
```

### From Source

```bash
git clone https://github.com/mbourmaud/hive.git
cd hive
cargo build --release
sudo cp target/release/hive /usr/local/bin/
hive init
```

> **Note:** `hive init` automatically installs Claude Code skills and the MCP server. **Restart Claude Code** after initialization to load the `/hive:*` commands.

---

## 🔄 Upgrading from v2.x

**Hive v3.0.0** is a major simplification. Key changes:

| v2.x | v3.0.0 |
|------|--------|
| `/hive:prd` | `/hive:plan` |
| `hive start --prd <file>` | `hive start <name>` (auto-detects plan) |
| `.hive/prds/` | `.hive/plans/` |
| Story-based workflow | Task-based workflow |
| `hive kill` | `hive stop` |
| `hive sessions` | `hive list` |
| `hive start --resume` | Removed (just use `hive start <name>`) |
| `hive unblock` | Removed (drones don't block) |
| Blocking workflow | Removed entirely |

**Migration steps:**
1. Update Hive: `brew upgrade hive-ai` or download latest release
2. Rename `.hive/prds/` to `.hive/plans/`
3. Update your plans (remove stories, use task-based format - see PLAN_GUIDE.md)
4. Use new command syntax

---

## 🐝 How to Bee Productive

| Step | Command |
|------|---------|
| **1. Initialize Your Hive** | `hive init` |
| **2. Create a Plan** <sup>`IN CLAUDE CODE`</sup> | `/hive:plan` |
| **3. Launch Your Drones!** <sup>`IN CLAUDE CODE`</sup> | `/hive:start` |
| **4. Be the Queen** <sup>`IN CLAUDE CODE`</sup> | `/hive:status` |
| **5. Harvest the Honey!** | Review, PR, merge 🍯 |

---

## 📊 Statusline - Track Your Drones

Run `/hive:statusline` in Claude Code to enable drone tracking in your statusline:

```
project │ main │ Opus 4.5 │ 45% │ ⬢ 22
👑 Hive v3.0.0 | 🐝 security 🔄 | 🐝 ui-refactor ✓
```

See all your drones at a glance:
- **🐝 name 🔄** - Drone currently working
- **🐝 name ✓** - Drone completed
- **🐝 name ⚠** - Drone needs attention

---

## 🎯 Commands

| In Claude Code | CLI | What it does |
|----------------|-----|--------------|
| `/hive:init` | `hive init` | Set up the hive in your project |
| `/hive:plan` | - | Generate a plan from description |
| `/hive:start` | `hive start <name>` | Launch a drone |
| `/hive:status` | `hive monitor` | TUI dashboard for all drones |
| `/hive:logs` | `hive logs <name>` | View drone activity |
| `/hive:stop` | `hive stop <name>` | Stop a running drone |
| `/hive:clean` | `hive clean <name>` | Remove drone & worktree |
| `/hive:statusline` | - | Configure statusline |

---

## 🔔 Desktop Notifications

Get notified when drones start, complete, or need attention:

| Event | Notification |
|-------|--------------|
| 🐝 Started | "security: drone started" |
| 🎉 Completed | "security: completed!" |
| ⚠️ Needs attention | "security: needs review" |

Works on macOS, Linux, and Windows/WSL.

---

## 🏗️ How It Works

```
┌──────────────────────────────────────────────────────┐
│  👑 Queen (your main branch)                         │
│  You + Claude working on features                    │
│  .hive/ folder with shared state                     │
├──────────────────────────────────────────────────────┤
│  🐝 Drone: security                                  │
│  Branch: hive/security                               │
│  Working through tasks autonomously                  │
├──────────────────────────────────────────────────────┤
│  🐝 Drone: ui-refactor                               │
│  Branch: hive/ui-refactor                            │
│  Working through tasks autonomously                  │
└──────────────────────────────────────────────────────┘
```

Each drone:
- Gets its own **git worktree** (isolated workspace) in `~/.hive/worktrees/<project>/<drone>/`
- Works on its own **branch** (`hive/<name>`)
- **Commits** progress regularly with descriptive messages
- Updates **status.json** in real-time
- Shares `.hive/` state with main project via symlink

---

## 📁 File Structure

```
your-project/                        # 👑 Queen
├── .hive/                           # Shared state
│   ├── plans/                       # Your plan files
│   │   └── security.json
│   └── drones/                      # Drone status
│       └── security/
│           ├── status.json          # Real-time progress
│           └── activity.log         # What it's doing

~/.hive/worktrees/                   # Global worktree base
└── your-project/                    # Per-project directory
    └── security/                    # 🐝 Drone worktree
        ├── .hive -> /path/to/your-project/.hive  # Symlinked!
        └── (your code being modified)
```

---

## 📊 Real-Time Monitoring

Track drone progress with live updates:

**Activity logs** - Each drone maintains a real-time activity log:
```
.hive/drones/security/
  ├── status.json      # Current progress and state
  └── activity.log     # Live activity feed
```

**Status tracking** includes:
- Current task being worked on
- Overall progress
- Duration and timestamps
- Model being used

**View in TUI**:
```bash
hive monitor
# Select drone → View logs, stop, clean
```

---

## ⚙️ Configuration

### Model Selection

By default, drones use **Sonnet** for cost-efficiency. You can change the model:

```bash
# Use Opus for complex tasks
hive start my-drone --model opus

# Use Haiku for simple/fast tasks
hive start my-drone --model haiku

# Default (Sonnet)
hive start my-drone
```

**Recommended usage:**
- **Sonnet** (default): Best balance of speed/cost/quality for most tasks
- **Opus**: Complex architectural changes, nuanced refactoring
- **Haiku**: Simple repetitive tasks, quick fixes

### Worktree Location

On first `hive init`, you'll be prompted to choose where drone worktrees are created:

**Default**: `~/.hive/worktrees/` (recommended - keeps everything centralized and clean)

**Custom**: You can specify any directory, or set via environment variable:
```bash
export HIVE_WORKTREE_BASE="/custom/path"
```

**Priority**:
1. `HIVE_WORKTREE_BASE` environment variable
2. Local `.hive/config.json` (per-project override)
3. Global `~/.config/hive/config.json`
4. Default: `~/.hive/worktrees/`

To change the global default later:
```bash
# Edit global config
vim ~/.config/hive/config.json

# Or set environment variable permanently
echo 'export HIVE_WORKTREE_BASE="$HOME/custom/path"' >> ~/.bashrc
```

### Color Output

Hive automatically detects terminal color support. To control colors:

**Disable colors**:
```bash
export NO_COLOR=1        # Standard way (https://no-color.org/)
# or
export HIVE_NO_COLOR=1   # Hive-specific
```

**Force colors** (when auto-detection fails):
```bash
export HIVE_FORCE_COLOR=1
```

**Troubleshooting**: If you see raw ANSI codes like `\033[0;32m` instead of colors:
```bash
# Set proper terminal type
export TERM=xterm-256color

# Or disable colors
export NO_COLOR=1
```

---

## 📋 Requirements

- `git`
- [Claude Code](https://claude.ai/code) CLI

---

## 🔧 Development

### Build from source

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Run with debug output
RUST_LOG=debug cargo run -- monitor
```

### Project Structure

```
src/
├── main.rs           # CLI entry point (clap)
├── lib.rs            # Library exports
├── types.rs          # Shared types (DroneStatus, Prd, etc.)
└── commands/
    ├── init.rs       # hive init
    ├── start.rs      # hive start
    ├── status.rs     # hive monitor (TUI)
    ├── logs.rs       # hive logs
    ├── kill_clean.rs # hive stop/clean
    └── ...
```

---

<p align="center">
  Made with 🍯 by <a href="https://github.com/mbourmaud">@mbourmaud</a><br>
  <sub>MIT License • Buzz responsibly</sub>
</p>
Testing auto-update
