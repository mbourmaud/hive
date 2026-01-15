# HIVE Project Instructions

## Overview

Pure bash script for orchestrating multiple Claude Code (Ralph) instances via git worktrees. No build step required.

## Commands

```bash
hive.sh init                              # Initialize Hive in current repo
hive.sh spawn <name> --create <branch>    # Create new Ralph with new branch
hive.sh spawn <name> --attach <branch>    # Attach Ralph to existing branch
hive.sh start <name> [prompt]             # Start Ralph background process
hive.sh status                            # Show status of all Ralphs
hive.sh logs <name> [lines]               # View Ralph's output log
hive.sh stop <name>                       # Stop a running Ralph
hive.sh sync <name>                       # Sync worktree with target branch
hive.sh pr <name> [--draft]               # Create Pull Request
hive.sh prs                               # List all PRs created by Hive
hive.sh cleanup <name>                    # Remove worktree after PR merge
hive.sh clean <name>                      # Remove worktree (abandon work)
hive.sh dashboard                         # Live status dashboard
```

## Dependencies

- bash
- jq
- git
- gh CLI (for PR operations)
- claude CLI

## Installation

```bash
make install    # Copies hive.sh to ~/.local/bin/hive
```

## Project Structure

```
.hive/              # Created by 'hive.sh init'
  config.json       # Configuration and state
  worktrees/        # Git worktrees for each Ralph
hive.sh             # Main script
Makefile            # Install/uninstall targets
```
