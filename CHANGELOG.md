# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.5.0] - 2026-01-19

### Added
- **Interactive TUI mode** - `hive status -i` with navigation using gum
- **Drone action menu** - View logs, kill, clean directly from TUI
- **Log viewer** - Integrated pager with gum for viewing logs
- **Auto-install gum** - Optional dependency installed on update (via brew)

### Changed
- Simplified follow mode (removed redundant notification handling)
- Updated help with status options

## [1.4.1] - 2026-01-19

### Fixed
- **Bash 3.x compatibility** - Replaced `declare -A` with temp files for macOS default bash

## [1.4.0] - 2026-01-19

### Added
- **New `hive status` dashboard** - Clean minimal design with story todolist
- **Follow mode** - `hive status --follow` with auto-refresh every 3 seconds
- **Story checklist** - Visual indicators: ✓ completed, ▸ in progress, ○ pending
- **Desktop notifications** - Notify on story completion in follow mode

### Fixed
- **UTC timezone bug** - Elapsed time now correctly calculated from UTC timestamps

### Changed
- Removed heavy ASCII borders for cleaner look
- Progress bar uses `━` for filled and `─` for empty
- Compact layout with dimmed completed items

## [1.3.4] - 2026-01-18

### Added
- **Story completion notifications** - Desktop notification when each story is completed with drone name and progress (X/Y)

## [1.3.3] - 2026-01-18

### Changed
- **Reinforced drone prompt** - Numbered commands (#1, #2, #3, #4) with clearer instructions
- **Mandatory logging** - Explicit warnings that monitoring breaks without status updates
- **Combined commands** - jq + echo in single commands to ensure both run together

## [1.3.2] - 2026-01-18

### Changed
- **Improved drone prompt** - Critical status.json update command now at the very top
- **Clearer workflow** - Simplified step-by-step instructions for status tracking
- **No confirmation on clean** - `hive clean` now removes directly without asking

## [1.3.1] - 2026-01-18

### Changed
- **Cleaner timer format** - Changed from `⏱ 1h30m` to `(5/5 - 1h30m)` inline format
- **Bold hive name** - Both "hive" and version are now bold in status header

## [1.3.0] - 2026-01-18

### Added
- **Elapsed time display** - Shows elapsed time next to each drone in `hive status` and statusline
- **Auto-clean suggestion** - Prompts to clean completed drones inactive for 60+ minutes
- **Version in bold** - `hive v1.3.0 status` header with bold version number

### Changed
- Statusline now shows elapsed time for each drone
- `hive` lowercase in status header

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
