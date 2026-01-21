use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use crate::types::{DroneState, DroneStatus};

/// List all drones with compact output
pub fn list() -> Result<()> {
    let drones = list_drones()?;

    if drones.is_empty() {
        println!("{}", "No drones found".yellow());
        println!("\nRun 'hive init' to initialize Hive");
        return Ok(());
    }

    // Honey theme header with crown emoji
    println!("{}", "👑 Hive Drones".yellow().bold());
    println!();

    // Header
    println!(
        "{:<20} {:<15} {:<10}",
        "NAME".bright_black(),
        "STATUS".bright_black(),
        "PROGRESS".bright_black()
    );
    println!("{}", "─".repeat(50).bright_black());

    for (name, status) in drones {
        let status_str = match status.status {
            DroneState::Starting => "starting".yellow(),
            DroneState::Resuming => "resuming".yellow(),
            DroneState::InProgress => "in_progress".green(),
            DroneState::Completed => "completed".bright_green().bold(),
            DroneState::Error => "error".red().bold(),
            DroneState::Blocked => "blocked".red().bold(),
            DroneState::Stopped => "stopped".bright_black(),
        };

        let progress = if status.total > 0 {
            format!("{}/{}", status.completed.len(), status.total)
        } else {
            "0/0".to_string()
        };

        let percentage = if status.total > 0 {
            (status.completed.len() as f32 / status.total as f32 * 100.0) as u32
        } else {
            0
        };

        println!(
            "{:<20} {:<15} {:<10}",
            format!("🐝 {}", name).yellow().bold(),
            status_str,
            format!("{} ({}%)", progress, percentage).bright_white()
        );
    }

    Ok(())
}

fn list_drones() -> Result<Vec<(String, DroneStatus)>> {
    let hive_dir = PathBuf::from(".hive");
    let drones_dir = hive_dir.join("drones");

    if !drones_dir.exists() {
        return Ok(Vec::new());
    }

    let mut drones = Vec::new();

    for entry in fs::read_dir(&drones_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let drone_name = entry.file_name().to_string_lossy().to_string();
        let status_path = entry.path().join("status.json");

        if status_path.exists() {
            let contents = fs::read_to_string(&status_path)
                .context(format!("Failed to read status for drone '{}'", drone_name))?;
            let status: DroneStatus = serde_json::from_str(&contents)
                .context(format!("Failed to parse status for drone '{}'", drone_name))?;
            drones.push((drone_name, status));
        }
    }

    // Sort by updated timestamp (most recent first)
    drones.sort_by(|a, b| b.1.updated.cmp(&a.1.updated));

    Ok(drones)
}

/// Check for updates silently (called on every command)
pub fn check_for_updates_background() {
    // Only check once per day
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return,
    };

    let cache_dir = home.join(".cache").join("hive");
    let _ = fs::create_dir_all(&cache_dir);
    let last_check_file = cache_dir.join("last_update_check");

    // Check if we checked recently (within 24 hours)
    if let Ok(metadata) = fs::metadata(&last_check_file) {
        if let Ok(modified) = metadata.modified() {
            if let Ok(elapsed) = SystemTime::now().duration_since(modified) {
                if elapsed < Duration::from_secs(86400) {
                    // Checked less than 24h ago, skip
                    return;
                }
            }
        }
    }

    // Update last check time
    let _ = fs::write(&last_check_file, "");

    // Check for updates in background (don't block)
    std::thread::spawn(|| {
        let _ = check_and_notify_update();
    });
}

fn check_and_notify_update() -> Result<()> {
    const REPO: &str = "mbourmaud/hive";
    let current_version = env!("CARGO_PKG_VERSION");

    let client = reqwest::blocking::Client::builder()
        .user_agent("hive")
        .timeout(Duration::from_secs(5))
        .build()?;

    let url = format!("https://api.github.com/repos/{}/releases/latest", REPO);
    let response = client.get(&url).send()?;

    if !response.status().is_success() {
        return Ok(());
    }

    let release: serde_json::Value = response.json()?;
    let latest_version = release["tag_name"]
        .as_str()
        .unwrap_or("")
        .trim_start_matches('v');

    // Simple version comparison
    if current_version < latest_version {
        eprintln!(
            "\n{}",
            format!(
                "💡 New Hive version available: {} → {}",
                current_version, latest_version
            )
            .yellow()
        );
        eprintln!("{}", format!("   Run {} to update", "hive update".cyan()));
    }

    Ok(())
}

/// Self-update via GitHub releases
pub fn update() -> Result<()> {
    const REPO: &str = "mbourmaud/hive";

    println!("{}", "🔄 Checking for updates...".bright_cyan());

    // Get current version
    let current_version = env!("CARGO_PKG_VERSION");
    println!("Current version: {}", current_version.bright_white());

    // Fetch latest release info from GitHub API
    let url = format!("https://api.github.com/repos/{}/releases/latest", REPO);

    println!("{}", "Fetching latest release info...".bright_black());

    let client = reqwest::blocking::Client::builder()
        .user_agent("hive")
        .build()?;

    let response = client
        .get(&url)
        .send()
        .context("Failed to fetch release info")?;

    if !response.status().is_success() {
        bail!("Failed to fetch release info: {}", response.status());
    }

    let release: serde_json::Value = response.json().context("Failed to parse release info")?;

    let latest_version = release["tag_name"]
        .as_str()
        .context("Missing tag_name in release")?
        .trim_start_matches('v');

    println!("Latest version: {}", latest_version.bright_white());

    // Compare versions
    if current_version >= latest_version {
        println!("{}", "✓ You are already on the latest version".green());
        return Ok(());
    }

    println!(
        "{}",
        format!(
            "New version available: {} -> {}",
            current_version, latest_version
        )
        .bright_yellow()
    );

    // Detect platform
    let platform = if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        "aarch64-apple-darwin"
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "x86_64") {
        "x86_64-apple-darwin"
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        "x86_64-unknown-linux-gnu"
    } else {
        bail!("Unsupported platform for auto-update. Please download manually from GitHub.");
    };

    // Find the matching asset
    let assets = release["assets"]
        .as_array()
        .context("Missing assets in release")?;

    let binary_name = format!("hive-{}", platform);
    let asset = assets
        .iter()
        .find(|a| a["name"].as_str().is_some_and(|n| n.contains(platform)))
        .context(format!("No binary found for platform '{}'", platform))?;

    let download_url = asset["browser_download_url"]
        .as_str()
        .context("Missing download URL")?;

    println!(
        "{}",
        format!("Downloading {}...", binary_name).bright_cyan()
    );

    // Download the binary
    let response = client
        .get(download_url)
        .send()
        .context("Failed to download binary")?;

    if !response.status().is_success() {
        bail!("Failed to download binary: {}", response.status());
    }

    let binary_data = response.bytes().context("Failed to read binary data")?;

    // Get current executable path
    let current_exe = std::env::current_exe().context("Failed to get current executable path")?;

    // Write to temporary file
    let temp_file = current_exe.with_extension("new");
    fs::write(&temp_file, &binary_data).context("Failed to write new binary")?;

    // Make executable (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&temp_file)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&temp_file, perms)?;
    }

    // Replace current binary
    fs::rename(&temp_file, &current_exe).context("Failed to replace current binary")?;

    println!("{}", "✓ Binary updated successfully!".green().bold());

    // Update skills automatically
    println!("\n{}", "Updating skills...".bright_cyan());
    if let Err(e) = update_skills() {
        eprintln!("{} Failed to update skills: {}", "⚠".yellow(), e);
        eprintln!(
            "Run {} to update skills manually",
            "hive install --skills-only".cyan()
        );
    } else {
        println!("{}", "✓ Skills updated successfully!".green().bold());
    }

    println!("\n{}", "Update complete!".green().bold());
    println!("Hive {} is now ready to use.", latest_version.bright_cyan());

    Ok(())
}

/// Update skills by calling the install command
fn update_skills() -> Result<()> {
    use crate::commands::install;
    install::run(true, false)?; // skills_only=true, bin_only=false to just update skills
    Ok(())
}
