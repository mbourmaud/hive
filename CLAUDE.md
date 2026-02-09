# HIVE Project Instructions

## Overview

Rust CLI for orchestrating multiple Claude Code instances (drones) via git worktrees. Features a TUI dashboard for monitoring and managing drones.

## Commands

```bash
hive init                              # Initialize Hive in current repo
hive start <name>                      # Launch a drone with a plan
hive monitor                           # TUI dashboard for all drones
hive logs <name>                       # View drone activity log
hive stop <name>                       # Stop a running drone
hive clean <name>                      # Remove drone & worktree
hive list                              # Quick list of all drones
hive update                            # Self-update to latest version
```

## Dependencies

- git
- gh CLI (for PR operations)
- claude CLI

## Installation

```bash
curl -fsSL https://raw.githubusercontent.com/mbourmaud/hive/main/install.sh | bash
```

## Project Structure

```
.hive/              # Created by 'hive init'
  config.json       # Configuration
  plans/            # Plan files
  drones/           # Drone status and logs
    <name>/
      status.json   # Real-time progress
      activity.log  # Activity feed
src/                # Rust source code
tests/              # Test suite
```
