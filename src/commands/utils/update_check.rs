use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::time::{Duration, SystemTime};

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

    // Don't notify if assets aren't uploaded yet (CI still building)
    let has_platform_assets = release["assets"]
        .as_array()
        .map(|assets| {
            assets.iter().any(|a| {
                a["name"]
                    .as_str()
                    .map(|n| n.starts_with("hive-"))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);

    if !has_platform_assets {
        return Ok(());
    }

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
