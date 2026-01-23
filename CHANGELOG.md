# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.5.2] - 2026-01-23

### Added
- **Base Branch Support**: PRDs can now specify `base_branch` to control where worktree is created from
  - For `master`/`main`, always uses `origin/master` or `origin/main` (up-to-date remote)
  - For other branches, uses local branch by default
  - Fetches from origin before creating worktree to ensure up-to-date refs
  - Fixes issue where drones included unwanted commits from stale local branches

### Changed
- Worktree creation now shows the base ref being used: `Creating branch 'X' from 'origin/master'`

## [2.5.1] - 2026-01-23

### Changed
- **Auto-Resume**: Monitor now automatically resumes drones when new stories are detected
  - No manual intervention required - drones resume as soon as new stories appear
  - Shows `✨ X new stories - auto-resuming...` indicator during auto-resume
  - Tracks which drones have been auto-resumed to avoid duplicate launches
  - Manual 'r' shortcut still available as fallback

## [2.5.0] - 2026-01-23

### Added
- **Hot Reload PRD Stories**: Monitor now detects when new stories are added to a PRD
  - Shows `✨ X new stories` indicator when PRD has more stories than status.total
  - Progress bar and counter now use PRD as source of truth (not cached status.total)
  - New 'r' keyboard shortcut to resume drone with updated PRD
  - Automatically updates status.json with new story count on resume

### Fixed
- **TUI Display**: Fixed spacing issues in monitor
  - Drone name padding increased from 16 to 30 characters (fixes long names like `fix-buildings-listing-archived-error`)
  - Story ID padding increased from 10 to 16 characters with trailing space (fixes IDs like `FIX-ARCHIVED-001`)
  - Progress bar no longer appears full when new stories are pending

## [2.4.1] - 2026-01-23

### Fixed
- **`/hive:start` skill**: Fixed documentation to match actual CLI syntax
  - Corrected: `hive start <NAME>` where NAME matches the PRD id (not `--prd` flag)
  - Added troubleshooting section for common errors
  - Added quick reference table: PRD filename → launch command

### Added
- **`/hive:prd` skill**: Added standard MR/CI story (Step 7)
  - New step asks user if they want a story for creating MR and monitoring CI
  - Provides `MR-001` template with definition of done for pipeline management
  - Adapts commands based on Git platform (GitLab `glab` / GitHub `gh`)

## [1.9.0] - 2026-01-20

### Added
- **Comprehensive Logging System**: All Claude invocations now logged with complete metadata
  - Story-specific logs in `.hive/drones/<name>/logs/<STORY-ID>/`
  - Attempt-by-attempt logging with `attempt-N.log` and `attempt-N-metadata.json`
  - Metadata includes: duration, exit code, timestamps, model, iteration number
  - `/hive:logs` slash command updated for interactive log viewing
  - TUI log navigation in `hive status -i` with "📊 View story logs" option

- **Human-in-the-Loop Blocking System**: Automatic drone blocking on repeated errors
  - Drones auto-block after 3+ failed attempts on same story
  - Creates `blocked.md` with context and troubleshooting questions
  - Desktop notifications when drone blocks: "⚠️ Hive - Drone Blocked"
  - `hive start --resume <name>` clears blocked status and continues
  - `hive unblock <name>` interactive command to review and unblock
  - Blocked drones highlighted in RED in all status displays

- **Enhanced Status Tracking**: New fields in `status.json`
  - `error_count`: Track consecutive errors on same story
  - `last_error_story`: Identify which story is causing issues
  - `blocked_reason`: Human-readable explanation of blocking
  - `blocked_questions`: Suggested clarifications needed
  - `awaiting_human`: Boolean flag for blocked state

### Changed
- Enhanced `hive status` to show blocked drones with ⚠️ in red
- Enhanced `hive status -i` TUI with story logs viewer
- Improved error handling and retry logic in launcher script
- Updated `/hive:logs` slash command with story-specific log viewing
- Launcher script now uses `tee` to capture complete Claude output
- Status indicator now includes "resuming" state

### Fixed
- Better log file organization for debugging
- Improved tracking of drone execution state
- More accurate error counting for blocking logic

## [1.8.0] - 2026-01-19

### Added
- **Local mode (`--local`)** - Run drone in current directory without creating worktree
  - Use when already on target branch: `hive start my-prd --local`
  - Skips branch creation and worktree setup
  - Ideal for working on existing branches

### Changed
- **Custom app icon for notifications** - Uses `-appIcon` instead of `-contentImage` for cleaner macOS notifications
- Unified notification functions with custom icon support across all platforms

## [1.7.2] - 2026-01-19

### Added
- **Auto-install dependencies** - Drone detects project type and installs deps at startup (Node.js, Python, Go, Rust, etc.)
- **Support for `target_branch` in PRD** - PRD can specify existing branch instead of creating `hive/<name>`

## [1.7.1] - 2026-01-19

### Changed
- **Default model changed to Sonnet** - Drones now use `sonnet` by default (was `opus`)
- Use `--model opus` for complex tasks requiring higher reasoning

## [1.7.0] - 2026-01-19

### Added
- **Definition of Done validation** - PRD stories now support `definition_of_done` and `verification_commands` fields
- **Verification commands** - Drone MUST execute verification commands before marking story as complete
- **Interactive PRD workflow** - `/hive:prd` now asks user to validate DoD for each story

### Changed
- Drone prompt now requires verification_commands to pass before completing a story
- PRD skill updated with interactive DoD validation workflow

## [1.6.1] - 2026-01-19

### Fixed
- **Resume skips completed stories** - Drone now reads status.json first and skips already completed stories instead of replaying them

## [1.6.0] - 2026-01-19

### Added
- **Story duration tracking** - Each story now tracks start/end time in `story_times` field
- **Duration display** - Dashboard shows duration for each story (e.g., `✓ STORY-001 Title (4m52s)`)
- In-progress stories show elapsed time, completed stories show total duration

### Changed
- Auto-refresh interval changed from 3s to 30s

## [1.5.2] - 2026-01-19

### Added
- **Auto-refresh in interactive mode** - Select "⟳ Auto-refresh" to enable auto-refresh, press any key to return to interactive menu

## [1.5.1] - 2026-01-19

### Changed
- **Interactive mode shows full dashboard** - Same detailed view as `hive status` (progress bar, stories, elapsed time)
- Simplified drone selection menu at bottom

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
