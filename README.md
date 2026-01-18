<p align="center">
  <img src="assets/logo.png" alt="Hive Logo" width="180">
</p>

<h1 align="center">Hive</h1>

<p align="center">
  <strong>Let Your Bees Do the Work! 🍯</strong>
</p>

<p align="center">
  Launch autonomous Claude agents that buzz through your PRDs while you sip coffee.
</p>

<p align="center">
  <a href="https://opensource.org/licenses/MIT"><img src="https://img.shields.io/badge/License-MIT-F5A623.svg" alt="License: MIT"></a>
  <a href="https://github.com/mbourmaud/hive/releases"><img src="https://img.shields.io/github/v/release/mbourmaud/hive?color=F5A623" alt="Release"></a>
</p>

---

## 📦 Install

```bash
curl -fsSL https://raw.githubusercontent.com/mbourmaud/hive/main/install.sh | bash
```

---

## 🐝 How to Bee Productive

| Step | Command |
|------|---------|
| **1. Initialize Your Hive** | `hive init` |
| **2. Create a PRD** <sup>`IN CLAUDE CODE`</sup> | `/hive:prd` |
| **3. Launch Your Drones!** <sup>`IN CLAUDE CODE`</sup> | `/hive:start` |
| **4. Be the Queen** <sup>`IN CLAUDE CODE`</sup> | `/hive:status` |
| **5. Harvest the Honey!** | Review, PR, merge 🍯 |

---

## 📊 Statusline - Track Your Drones

Run `/hive:statusline` in Claude Code to enable drone tracking in your statusline:

```
project │ main │ Opus 4.5 │ 45% │ ⬢ 22
👑 Hive v1.2.0 | 🐝 security (4/10) | 🐝 ui-refactor ✓
```

See all your drones at a glance:
- **🐝 name (X/Y)** - Drone in progress (X stories done out of Y)
- **🐝 name ✓** - Drone completed all stories
- **🔄** - Drone currently running

---

## 🎯 Commands

| In Claude Code | CLI | What it does |
|----------------|-----|--------------|
| `/hive:init` | `hive init` | Set up the hive in your project |
| `/hive:prd` | - | Generate a PRD from description |
| `/hive:start` | `hive start --prd <file>` | Launch a drone |
| `/hive:status` | `hive status` | See all drones status |
| `/hive:logs` | `hive logs <name>` | View drone activity |
| `/hive:kill` | `hive kill <name>` | Stop a drone |
| `/hive:clean` | `hive clean <name>` | Remove drone & worktree |
| `/hive:statusline` | - | Configure statusline |

---

## 🔔 Desktop Notifications

Get notified when drones start, complete, or pause:

| Event | Notification |
|-------|--------------|
| 🐝 Started | "security: 10 stories" |
| 🎉 Completed | "security: 10/10 done!" |
| ⏸️ Paused | "security: 7/10 (max iterations)" |

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
│  Implementing SEC-001 → SEC-010 autonomously         │
├──────────────────────────────────────────────────────┤
│  🐝 Drone: ui-refactor                               │
│  Branch: hive/ui-refactor                            │
│  Implementing UI-001 → UI-025 autonomously           │
└──────────────────────────────────────────────────────┘
```

Each drone:
- Gets its own **git worktree** (isolated workspace)
- Works on its own **branch** (`hive/<name>`)
- **Commits** each story: `feat(SEC-001): description`
- Updates **status.json** in real-time

---

## 📁 File Structure

```
your-project/                        # 👑 Queen
├── .hive/                           # Shared state
│   ├── prds/                        # Your PRD files
│   │   └── prd-security.json
│   └── drones/                      # Drone status
│       └── security/
│           ├── status.json          # Progress: 4/10
│           └── activity.log         # What it's doing

~/Projects/your-project-security/    # 🐝 Drone worktree
├── .hive -> ../your-project/.hive   # Symlinked!
└── (your code being modified)
```

---

## 📋 Requirements

- `bash`, `git`, `jq`
- [Claude Code](https://claude.ai/code) CLI

---

<p align="center">
  Made with 🍯 by <a href="https://github.com/mbourmaud">@mbourmaud</a><br>
  <sub>MIT License • Buzz responsibly</sub>
</p>
