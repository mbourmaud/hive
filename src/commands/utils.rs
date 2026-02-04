use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::fs;
use std::time::{Duration, SystemTime};

use super::common::reconcile_progress;
use crate::types::{DroneState, DroneStatus, ExecutionMode};

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

        // Reconcile progress with actual PRD (filters out old completed stories)
        let (valid_completed, total_stories) = reconcile_progress(&status);

        let progress = if total_stories > 0 {
            format!("{}/{}", valid_completed, total_stories)
        } else {
            "0/0".to_string()
        };

        let percentage = if total_stories > 0 {
            (valid_completed as f32 / total_stories as f32 * 100.0) as u32
        } else {
            0
        };

        // Use different emoji for subagent mode
        let mode_emoji = match status.execution_mode {
            ExecutionMode::Subagent => "🤖",
            ExecutionMode::Worktree => "🐝",
            ExecutionMode::Swarm => "🐝",
        };

        println!(
            "{:<20} {:<15} {:<10}",
            format!("{} {}", mode_emoji, name).yellow().bold(),
            status_str,
            format!("{} ({}%)", progress, percentage).bright_white()
        );
    }

    Ok(())
}

fn list_drones() -> Result<Vec<(String, DroneStatus)>> {
    super::common::list_drones()
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
        eprintln!("   Run {} to update", "hive update".cyan());
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
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(std::time::Duration::from_secs(30))
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

    // Compare versions (simple lexicographic comparison works for most cases)
    // Note: This doesn't handle all semver edge cases but works for our versioning scheme
    if current_version == latest_version {
        println!("{}", "✓ You are already on the latest version".green());
        return Ok(());
    }

    // Parse versions for proper comparison
    let parse_version =
        |v: &str| -> Vec<u32> { v.split('.').filter_map(|s| s.parse().ok()).collect() };

    let current_parts = parse_version(current_version);
    let latest_parts = parse_version(latest_version);

    // Compare version parts
    for i in 0..current_parts.len().max(latest_parts.len()) {
        let current_part = current_parts.get(i).copied().unwrap_or(0);
        let latest_part = latest_parts.get(i).copied().unwrap_or(0);

        if current_part > latest_part {
            println!("{}", "✓ You are already on the latest version".green());
            return Ok(());
        } else if current_part < latest_part {
            break;
        }
    }

    println!(
        "{}",
        format!(
            "New version available: {} -> {}",
            current_version, latest_version
        )
        .bright_yellow()
    );

    // Detect platform and map to asset naming convention
    let asset_name = if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        "hive-darwin-arm64.tar.gz"
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "x86_64") {
        "hive-darwin-amd64.tar.gz"
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        "hive-linux-amd64.tar.gz"
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
        "hive-linux-arm64.tar.gz"
    } else {
        bail!("Unsupported platform for auto-update. Please download manually from GitHub.");
    };

    // Find the matching asset
    let assets = release["assets"]
        .as_array()
        .context("Missing assets in release")?;

    let asset = assets
        .iter()
        .find(|a| a["name"].as_str() == Some(asset_name))
        .context(format!("No binary found for platform '{}'", asset_name))?;

    let download_url = asset["browser_download_url"]
        .as_str()
        .context("Missing download URL")?;

    println!("{}", format!("Downloading {}...", asset_name).bright_cyan());

    // Create temporary directory for download
    let temp_dir = std::env::temp_dir().join(format!("hive-update-{}", latest_version));
    fs::create_dir_all(&temp_dir).context("Failed to create temp directory")?;

    let temp_archive = temp_dir.join(asset_name);

    // Use gh CLI to download (more reliable than reqwest for GitHub releases)
    let gh_output = std::process::Command::new("gh")
        .args([
            "release",
            "download",
            &format!("v{}", latest_version),
            "--repo",
            REPO,
            "--pattern",
            asset_name,
            "--dir",
            temp_dir.to_str().unwrap(),
        ])
        .output();

    match gh_output {
        Ok(output) if output.status.success() => {
            // gh download succeeded
        }
        _ => {
            // Fallback to direct download with reqwest
            println!(
                "{}",
                "gh CLI not available, using direct download...".bright_black()
            );
            let response = client
                .get(download_url)
                .send()
                .with_context(|| format!("Failed to download archive from {}", download_url))?;

            if !response.status().is_success() {
                bail!("Failed to download archive: {}", response.status());
            }

            let archive_data = response.bytes().context("Failed to read archive data")?;
            fs::write(&temp_archive, &archive_data).context("Failed to write archive")?;
        }
    }

    // Get current executable path
    let current_exe = std::env::current_exe().context("Failed to get current executable path")?;

    // Verify archive was downloaded
    if !temp_archive.exists() {
        bail!("Downloaded archive not found at {}", temp_archive.display());
    }

    println!("{}", "Extracting archive...".bright_cyan());

    // Extract using tar command
    let output = std::process::Command::new("tar")
        .args([
            "-xzf",
            temp_archive.to_str().unwrap(),
            "-C",
            temp_dir.to_str().unwrap(),
        ])
        .output()
        .context("Failed to extract archive")?;

    if !output.status.success() {
        bail!(
            "Failed to extract archive: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Find the extracted binary
    let extracted_binary = temp_dir.join("hive");
    if !extracted_binary.exists() {
        bail!(
            "Extracted binary not found at {}",
            extracted_binary.display()
        );
    }

    // Make executable (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&extracted_binary)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&extracted_binary, perms)?;
    }

    // Replace current binary
    fs::rename(&extracted_binary, &current_exe).context("Failed to replace current binary")?;

    // Clean up temp directory
    let _ = fs::remove_dir_all(&temp_dir);

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
