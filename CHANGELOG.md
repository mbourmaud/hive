# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] - 2025-01-17

### Added
- **Auto-update check** - Notifies when new version is available (cached 24h)
- **`hive update` command** - Self-update CLI and skills in one command

### Changed
- Update check runs in background (non-blocking)
- Skip update check for `help`, `version`, `update` commands

## [0.2.0] - 2025-01-17

### Added
- **Install script** - One-liner installation via curl
- **Skills for multiple editors** - Claude Code, Cursor, Amp Code, OpenCode, Gemini CLI
- **Activity logging** - Human-readable `activity.log` with emojis
- **Structured logs** - JSON logs in `status.json` for programmatic access
- **Shared `.hive/` folder** - Symlinked between queen and drones for real-time communication
- **Statusline integration** - Show drone progress in Claude Code statusline

### Changed
- Renamed command `run` to `start` for consistency with skills
- Simplified CLI from ~3500 lines to ~650 lines
- PRDs now stored in `.hive/prds/` (gitignored)
- Drone state in `.hive/drones/<name>/`

### Skills Added
- `/hive:init` - Initialize Hive in repo
- `/hive:start` - Launch a drone on PRD
- `/hive:status` - Show all drone status
- `/hive:list` - List active drones (compact)
- `/hive:logs` - View drone logs
- `/hive:kill` - Stop a running drone
- `/hive:clean` - Remove drone and worktree
- `/hive:prd` - Generate PRD interactively
- `/hive:statusline` - Configure Claude Code statusline

## [0.1.0] - 2025-01-15

### Added
- Initial release
- Basic drone orchestration via git worktrees
- PRD-driven autonomous execution
- Multi-drone support
