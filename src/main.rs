use clap::{Parser, Subcommand};
use hive_rust::commands;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(name = "hive-rust")]
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
    },

    /// Display drone status with optional TUI dashboard
    Status {
        /// Drone name (optional)
        name: Option<String>,
        /// Interactive TUI mode
        #[arg(short, long)]
        interactive: bool,
        /// Follow mode - auto-refresh
        #[arg(short, long)]
        follow: bool,
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
    },

    /// Stop a running drone
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

    match cli.command {
        Commands::Init => {
            if let Err(e) = commands::init::run() {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Start { name, prompt, resume, local, model, dry_run } => {
            if let Err(e) = commands::start::run(name, prompt, resume, local, model, dry_run) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Status { name, interactive, follow } => {
            if let Err(e) = commands::status::run(name, interactive, follow) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Logs { name, lines, story } => {
            if let Err(e) = commands::logs::run(name, lines, story) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Kill { name } => {
            println!("Kill drone '{}' - not yet implemented", name);
        }
        Commands::Clean { name, force } => {
            println!("Clean drone '{}' - not yet implemented", name);
            println!("  Force: {}", force);
        }
        Commands::Unblock { name, no_interactive } => {
            println!("Unblock drone '{}' - not yet implemented", name);
            println!("  No interactive: {}", no_interactive);
        }
        Commands::List => {
            println!("List command - not yet implemented");
        }
        Commands::Version => {
            println!("hive-rust version {}", VERSION);
        }
        Commands::Update => {
            println!("Update command - not yet implemented");
        }
        Commands::Profile { command } => {
            match command {
                ProfileCommands::List => {
                    println!("Profile list - not yet implemented");
                }
                ProfileCommands::Create { name } => {
                    println!("Profile create '{}' - not yet implemented", name);
                }
                ProfileCommands::Use { name } => {
                    println!("Profile use '{}' - not yet implemented", name);
                }
                ProfileCommands::Delete { name } => {
                    println!("Profile delete '{}' - not yet implemented", name);
                }
            }
        }
    }
}
