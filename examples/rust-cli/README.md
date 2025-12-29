# Example: Rust CLI Tool

This example shows how to use Hive to develop a Rust CLI application with parallel development.

## Project Structure

```
my-cli/
├── src/
│   ├── main.rs
│   ├── commands/       # CLI commands
│   ├── config/         # Configuration
│   ├── utils/          # Utilities
│   └── lib.rs
├── tests/              # Integration tests
└── Cargo.toml
```

## Setup

1. **Configure Hive for Rust:**

```bash
# .env
WORKSPACE_NAME=my-cli
HIVE_DOCKERFILE=docker/Dockerfile.rust
GIT_REPO_URL=https://github.com/user/my-cli.git
```

2. **Start Hive with 3 workers:**

```bash
hive init --workspace my-cli --workers 3 -y
```

## Example Workflow: Build a File Search CLI

### Step 1: Queen Plans Features

```bash
hive connect queen
```

Tell Queen:
```
Build a fast file search CLI with:
- find command (search by name pattern)
- grep command (search file contents)
- stats command (file statistics)
- All commands should be parallel and fast
```

Queen creates tasks:
```bash
hive-assign drone-1 "Implement find command" "Search files by glob pattern, display results with colors" "CLI-101"
hive-assign drone-2 "Implement grep command" "Search file contents with regex, show matches with context" "CLI-102"
hive-assign drone-3 "Implement stats command" "Calculate file counts, sizes, types with progress bar" "CLI-103"
```

### Step 2: Workers Implement Commands

**Terminal 2 - Drone 1 (Find Command):**
```bash
hive connect 1
take-task
```

```rust
// src/commands/find.rs
use clap::Args;
use glob::glob;
use colored::Colorize;
use std::path::PathBuf;

#[derive(Args)]
pub struct FindArgs {
    /// Glob pattern to search for
    pattern: String,

    /// Directory to search in
    #[arg(short, long, default_value = ".")]
    path: PathBuf,

    /// Case insensitive search
    #[arg(short, long)]
    ignore_case: bool,
}

pub fn run(args: FindArgs) -> anyhow::Result<()> {
    let pattern = if args.ignore_case {
        args.pattern.to_lowercase()
    } else {
        args.pattern.clone()
    };

    let search_pattern = args.path.join(&pattern);
    let pattern_str = search_pattern.to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid pattern"))?;

    let entries: Vec<_> = glob(pattern_str)?
        .filter_map(Result::ok)
        .collect();

    if entries.is_empty() {
        println!("{}", "No files found".yellow());
        return Ok(());
    }

    println!("{} {} files:", "Found".green(), entries.len());
    for entry in entries {
        let metadata = entry.metadata()?;
        let size = metadata.len();
        let modified = metadata.modified()?;

        println!(
            "  {} ({} bytes, modified: {:?})",
            entry.display().to_string().cyan(),
            size,
            modified
        );
    }

    Ok(())
}
```

Tests:
```bash
cargo test find --features test-utils
```

When done:
```bash
task-done
```

**Terminal 3 - Drone 2 (Grep Command):**
```bash
hive connect 2
take-task
```

```rust
// src/commands/grep.rs
use clap::Args;
use regex::Regex;
use walkdir::WalkDir;
use std::fs::File;
use std::io::{BufRead, BufReader};
use colored::Colorize;

#[derive(Args)]
pub struct GrepArgs {
    /// Regex pattern to search for
    pattern: String,

    /// Directory to search in
    #[arg(short, long, default_value = ".")]
    path: String,

    /// Context lines to show
    #[arg(short = 'C', long, default_value = "2")]
    context: usize,

    /// Case insensitive
    #[arg(short, long)]
    ignore_case: bool,
}

pub fn run(args: GrepArgs) -> anyhow::Result<()> {
    let regex = if args.ignore_case {
        Regex::new(&format!("(?i){}", args.pattern))?
    } else {
        Regex::new(&args.pattern)?
    };

    for entry in WalkDir::new(&args.path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let file = File::open(entry.path())?;
        let reader = BufReader::new(file);
        let lines: Vec<_> = reader.lines().collect();

        for (line_num, line) in lines.iter().enumerate() {
            if let Ok(line_content) = line {
                if regex.is_match(line_content) {
                    println!(
                        "{}:{}",
                        entry.path().display().to_string().green(),
                        (line_num + 1).to_string().yellow()
                    );

                    // Show context
                    let start = line_num.saturating_sub(args.context);
                    let end = (line_num + args.context + 1).min(lines.len());

                    for i in start..end {
                        if let Ok(context_line) = &lines[i] {
                            let prefix = if i == line_num {
                                "→".red()
                            } else {
                                " ".normal()
                            };
                            println!("  {} {}", prefix, context_line);
                        }
                    }
                    println!();
                }
            }
        }
    }

    Ok(())
}
```

Tests:
```bash
cargo test grep --features test-utils
```

When done:
```bash
task-done
```

**Terminal 4 - Drone 3 (Stats Command):**
```bash
hive connect 3
take-task
```

```rust
// src/commands/stats.rs
use clap::Args;
use walkdir::WalkDir;
use std::collections::HashMap;
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Args)]
pub struct StatsArgs {
    /// Directory to analyze
    #[arg(default_value = ".")]
    path: String,

    /// Show detailed breakdown
    #[arg(short, long)]
    detailed: bool,
}

pub fn run(args: StatsArgs) -> anyhow::Result<()> {
    // Count total files first
    let total_files = WalkDir::new(&args.path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .count();

    let pb = ProgressBar::new(total_files as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")?
    );

    let mut stats = HashMap::new();
    let mut total_size = 0u64;

    for entry in WalkDir::new(&args.path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let ext = entry.path()
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("no extension")
            .to_string();

        let size = entry.metadata()?.len();
        total_size += size;

        let entry_stats = stats.entry(ext).or_insert((0, 0u64));
        entry_stats.0 += 1;
        entry_stats.1 += size;

        pb.inc(1);
    }

    pb.finish_with_message("Done!");

    println!("\n📊 Statistics for {}\n", args.path);
    println!("Total files: {}", total_files);
    println!("Total size: {} bytes ({:.2} MB)\n", total_size, total_size as f64 / 1_000_000.0);

    if args.detailed {
        println!("Breakdown by extension:");
        let mut sorted: Vec<_> = stats.iter().collect();
        sorted.sort_by(|a, b| b.1.1.cmp(&a.1.1));

        for (ext, (count, size)) in sorted {
            println!(
                "  .{:<10} {:>6} files  {:>10} bytes ({:>6.2} MB)",
                ext,
                count,
                size,
                *size as f64 / 1_000_000.0
            );
        }
    }

    Ok(())
}
```

Tests:
```bash
cargo test stats --features test-utils
```

When done:
```bash
task-done
```

### Step 3: Integration

Wire all commands together:

```rust
// src/main.rs
use clap::{Parser, Subcommand};

mod commands;

#[derive(Parser)]
#[command(name = "mycli")]
#[command(about = "Fast file search and analysis", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Find files by pattern
    Find(commands::find::FindArgs),
    /// Search file contents
    Grep(commands::grep::GrepArgs),
    /// File statistics
    Stats(commands::stats::StatsArgs),
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Find(args) => commands::find::run(args),
        Commands::Grep(args) => commands::grep::run(args),
        Commands::Stats(args) => commands::stats::run(args),
    }
}
```

## Common Tasks

### Add New Command

```bash
hive-assign drone-1 "Add watch command" "Watch directory for changes and notify" "CLI-104"
```

### Optimize Performance

```bash
hive-assign drone-1 "Parallelize grep" "Use rayon to search files in parallel" "CLI-105"
hive-assign drone-2 "Add file caching" "Cache file metadata for faster stats" "CLI-106"
```

### Cross-Platform Build

```bash
hive-assign drone-1 "Build for Linux" "cargo build --release --target x86_64-unknown-linux-gnu" "CLI-107"
hive-assign drone-2 "Build for macOS" "cargo build --release --target aarch64-apple-darwin" "CLI-108"
hive-assign drone-3 "Build for Windows" "cargo build --release --target x86_64-pc-windows-gnu" "CLI-109"
```

## Best Practices

### 1. Error Handling

```rust
// ✅ Good
use anyhow::{Context, Result};

fn process_file(path: &Path) -> Result<()> {
    let content = fs::read_to_string(path)
        .context("Failed to read file")?;
    Ok(())
}

// ❌ Bad
fn process_file(path: &Path) {
    let content = fs::read_to_string(path).unwrap(); // Panics!
}
```

### 2. Testing

```bash
# Run all tests
cargo test

# Run with coverage
cargo tarpaulin --out Html

# Integration tests
cargo test --test integration
```

### 3. Before task-done

```bash
# Format
cargo fmt

# Lint
cargo clippy -- -D warnings

# Build release
cargo build --release

# Test
cargo test
```

## Troubleshooting

### Build Errors

```bash
# Clean and rebuild
cargo clean
cargo build

# Update dependencies
cargo update
```

### Test Failures

```bash
# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_find -- --nocapture
```

## Timeline

**Sequential (1 developer):**
- 3 commands × 2 hours = 6 hours

**Parallel (Hive with 3 workers):**
- 3 commands in parallel = 2 hours

**Time saved: 67%**
