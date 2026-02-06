use clap::{Parser, Subcommand};
use hive_lib::commands;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(name = "hive")]
#[command(about = "High-performance CLI tool for orchestrating multiple Claude Code instances")]
#[command(version = VERSION)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize Hive in the current git repository
    Init,

    /// Launch a drone to work on a PRD autonomously
    Start {
        /// Drone name
        name: String,
        /// Custom prompt to send to the drone
        prompt: Option<String>,
        /// Resume a blocked or stopped drone
        #[arg(long)]
        resume: bool,
        /// Run in current directory instead of worktree
        #[arg(long)]
        local: bool,
        /// Model to use (sonnet, opus, haiku)
        #[arg(long, default_value = "sonnet")]
        model: String,
        /// Dry run - don't launch Claude
        #[arg(long)]
        dry_run: bool,
        /// Use subagent mode (spawns Claude Task subagent instead of worktree)
        #[arg(long)]
        subagent: bool,
        /// Wait for subagent to complete before returning (implies --subagent)
        #[arg(long)]
        wait: bool,
    },

    /// Monitor drone status with auto-refreshing TUI dashboard
    Monitor {
        /// Drone name (optional)
        name: Option<String>,
        /// Simple output mode (no TUI, for scripts/CI)
        #[arg(short, long)]
        simple: bool,
    },

    /// View drone activity logs
    Logs {
        /// Drone name
        name: String,
        /// Number of lines to display
        #[arg(long)]
        lines: Option<usize>,
        /// Show logs for specific story
        #[arg(long)]
        story: Option<String>,
        /// Follow mode - continuously tail logs
        #[arg(short, long)]
        follow: bool,
    },

    /// Stop a running drone
    Stop {
        /// Drone name
        name: String,
    },

    /// [DEPRECATED] Use 'stop' instead
    #[command(hide = true)]
    Kill {
        /// Drone name
        name: String,
    },

    /// Remove worktree and clean up drone artifacts
    Clean {
        /// Drone name
        name: String,
        /// Force clean without confirmation
        #[arg(long)]
        force: bool,
    },

    /// Interactive workflow to unblock stuck drones
    Unblock {
        /// Drone name
        name: String,
        /// Non-interactive mode for CI
        #[arg(long)]
        no_interactive: bool,
    },

    /// List all drones
    List,

    /// Display version information
    Version,

    /// Self-update via GitHub releases
    Update,

    /// Manage Claude wrapper profiles
    Profile {
        #[command(subcommand)]
        command: ProfileCommands,
    },

    /// Browse Claude conversation logs
    Sessions {
        /// Drone name
        name: String,
        /// Open most recent session directly
        #[arg(long)]
        latest: bool,
    },

    /// Install Hive binary and skills
    Install {
        /// Only install skills without binary
        #[arg(long)]
        skills_only: bool,
        /// Only install binary without skills
        #[arg(long)]
        bin_only: bool,
    },

    /// Start MCP server (stdio) for Claude Code integration
    #[command(name = "mcp-server")]
    McpServer,

    /// Launch unified TUI chat interface
    Tui,
}

#[derive(Subcommand)]
enum ProfileCommands {
    /// List available profiles
    List,
    /// Create a new profile
    Create {
        /// Profile name
        name: String,
    },
    /// Activate a profile
    Use {
        /// Profile name
        name: String,
    },
    /// Delete a profile
    Delete {
        /// Profile name
        name: String,
    },
}

fn main() {
    let cli = Cli::parse();

    // Check for updates in background (non-blocking, once per day)
    commands::utils::check_for_updates_background();

    match cli.command {
        Commands::Init => {
            if let Err(e) = commands::init::run() {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Start {
            name,
            prompt,
            resume,
            local,
            model,
            dry_run,
            subagent,
            wait,
        } => {
            // --wait implies --subagent
            let use_subagent = subagent || wait;
            if let Err(e) = commands::start::run(
                name,
                prompt,
                resume,
                local,
                model,
                dry_run,
                use_subagent,
                wait,
            ) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Monitor { name, simple } => {
            if let Err(e) = commands::status::run_monitor(name, simple) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Logs {
            name,
            lines,
            story,
            follow,
        } => {
            if let Err(e) = commands::logs::run(name, lines, story, follow) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Stop { name } => {
            if let Err(e) = commands::kill_clean::kill(name) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Kill { name } => {
            eprintln!("Warning: 'hive kill' is deprecated. Use 'hive stop' instead.");
            if let Err(e) = commands::kill_clean::kill(name) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Clean { name, force } => {
            if let Err(e) = commands::kill_clean::clean(name, force) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Unblock {
            name,
            no_interactive,
        } => {
            if let Err(e) = commands::unblock::run(name, no_interactive) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::List => {
            if let Err(e) = commands::utils::list() {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Version => {
            println!("🐝 Hive v{}", VERSION);
            println!("Drone orchestration for Claude Code");
        }
        Commands::Update => {
            if let Err(e) = commands::utils::update() {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Profile { command } => {
            let result = match command {
                ProfileCommands::List => commands::profile::list(),
                ProfileCommands::Create { name } => commands::profile::create(name),
                ProfileCommands::Use { name } => commands::profile::use_profile(name),
                ProfileCommands::Delete { name } => commands::profile::delete(name),
            };

            if let Err(e) = result {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Sessions { name, latest } => {
            if let Err(e) = commands::sessions::run(name, latest) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Install {
            skills_only,
            bin_only,
        } => {
            if let Err(e) = commands::install::run(skills_only, bin_only) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::McpServer => {
            if let Err(e) = hive_lib::mcp::run_server() {
                eprintln!("MCP Server error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Tui => {
            if let Err(e) = hive_lib::tui::run_tui() {
                eprintln!("TUI error: {}", e);
                std::process::exit(1);
            }
        }
    }
}
