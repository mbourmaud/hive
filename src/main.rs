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

    /// Monitor drone status with auto-refreshing TUI dashboard
    Monitor {
        /// Drone name (optional)
        name: Option<String>,
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
        Commands::Monitor { name } => {
            if let Err(e) = commands::status::run_monitor(name) {
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
        Commands::Unblock { name, no_interactive } => {
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
        Commands::Install { skills_only, bin_only } => {
            if let Err(e) = commands::install::run(skills_only, bin_only) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }
}
