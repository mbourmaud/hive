# Hive Rust Rewrite

High-performance CLI tool for orchestrating multiple Claude Code instances via git worktrees.

## Features

- **Concurrent drone management**: Run multiple Claude agents in parallel
- **Git worktree isolation**: Each drone works in its own worktree
- **Full TUI dashboard**: Real-time monitoring with ratatui
- **Story-driven workflow**: PRD-based task management
- **Cross-platform**: macOS (ARM64, x64) and Linux (x64)

## Build Instructions

### Prerequisites

- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- Git
- Claude CLI (`claude`)
- GitHub CLI (`gh`) for PR operations

### Development Build

```bash
# Clone the repository
git clone https://github.com/anthropics/hive.git
cd hive

# Build in debug mode
cargo build

# Run the binary
./target/debug/hive-rust --help
```

### Release Build

```bash
# Build optimized binary
cargo build --release

# Binary will be at ./target/release/hive-rust
./target/release/hive-rust --version
```

### Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test --test init_tests
```

### Linting

```bash
# Check for warnings
cargo clippy -- -D warnings

# Auto-fix issues
cargo clippy --fix
```

## Installation

```bash
# Install from source
cargo install --path .

# Or use the install script (once published)
curl -fsSL https://raw.githubusercontent.com/anthropics/hive/main/install.sh | sh
```

## Quick Start

```bash
# Initialize Hive in your repo
hive-rust init

# Launch a drone
hive-rust start my-drone

# Check status
hive-rust status

# View logs
hive-rust logs my-drone

# Stop a drone
hive-rust kill my-drone
```

## Commands

- `init` - Initialize Hive in current repo
- `start <name>` - Launch a drone
- `status` - Display drone status (use `-i` for TUI)
- `logs <name>` - View drone logs
- `kill <name>` - Stop a running drone
- `clean <name>` - Remove worktree
- `unblock <name>` - Unblock stuck drone
- `list` - List all drones
- `profile` - Manage Claude profiles
- `version` - Show version
- `update` - Self-update

## Architecture

- Built with Rust for performance and reliability
- Async runtime powered by Tokio
- TUI framework: ratatui + crossterm
- CLI parsing: clap with derive macros
- JSON handling: serde + serde_json

## License

MIT
