use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::fs;

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

    if !should_update(current_version, latest_version) {
        println!("{}", "✓ You are already on the latest version".green());
        return Ok(());
    }

    let current_parts = parse_version(current_version);
    let latest_parts = parse_version(latest_version);

    println!(
        "{}",
        format!(
            "New version available: {} -> {}",
            current_version, latest_version
        )
        .bright_yellow()
    );

    // Check for breaking changes
    let release_body = release["body"].as_str().unwrap_or("");
    if is_breaking_change(&current_parts, &latest_parts, release_body)
        && !confirm_breaking_update(release_body)?
    {
        println!("Update cancelled.");
        return Ok(());
    }

    let asset_name = platform_asset_name()?;
    let assets = release["assets"]
        .as_array()
        .context("Missing assets in release")?;

    let asset = match assets
        .iter()
        .find(|a| a["name"].as_str() == Some(asset_name))
    {
        Some(a) => a,
        None => {
            print_asset_not_ready(latest_version, asset_name, REPO);
            return Ok(());
        }
    };

    let download_url = asset["browser_download_url"]
        .as_str()
        .context("Missing download URL")?;

    download_and_install(&client, download_url, asset_name, latest_version, REPO)?;

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

fn parse_version(v: &str) -> Vec<u32> {
    v.split('.').filter_map(|s| s.parse().ok()).collect()
}

fn should_update(current_version: &str, latest_version: &str) -> bool {
    if current_version == latest_version {
        return false;
    }

    let current_parts = parse_version(current_version);
    let latest_parts = parse_version(latest_version);

    for i in 0..current_parts.len().max(latest_parts.len()) {
        let current_part = current_parts.get(i).copied().unwrap_or(0);
        let latest_part = latest_parts.get(i).copied().unwrap_or(0);

        if current_part > latest_part {
            return false;
        } else if current_part < latest_part {
            return true;
        }
    }

    false
}

fn is_breaking_change(current_parts: &[u32], latest_parts: &[u32], release_body: &str) -> bool {
    let current_major = current_parts.first().copied().unwrap_or(0);
    let latest_major = latest_parts.first().copied().unwrap_or(0);
    latest_major > current_major || release_body.contains("BREAKING")
}

fn confirm_breaking_update(release_body: &str) -> Result<bool> {
    println!(
        "\n{}",
        "⚠  This release contains BREAKING CHANGES:".red().bold()
    );
    let preview = if release_body.len() > 500 {
        format!("{}...", &release_body[..500])
    } else {
        release_body.to_string()
    };
    if !preview.is_empty() {
        println!("{}", preview.bright_white());
    }
    println!();

    let confirmed = dialoguer::Confirm::new()
        .with_prompt("Proceed with update?")
        .default(false)
        .interact()?;

    Ok(confirmed)
}

fn platform_asset_name() -> Result<&'static str> {
    if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        Ok("hive-darwin-arm64.tar.gz")
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "x86_64") {
        Ok("hive-darwin-amd64.tar.gz")
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        Ok("hive-linux-amd64.tar.gz")
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
        Ok("hive-linux-arm64.tar.gz")
    } else {
        bail!("Unsupported platform for auto-update. Please download manually from GitHub.");
    }
}

fn print_asset_not_ready(latest_version: &str, asset_name: &str, repo: &str) {
    println!("\n{}", "⚠  Binary not available yet".yellow().bold());
    println!(
        "Release {} exists but the {} asset hasn't been uploaded yet.",
        latest_version.bright_white(),
        asset_name.bright_white()
    );
    println!("CI is probably still building. Try again in a few minutes.\n");
    println!(
        "Build status: {}",
        format!("https://github.com/{}/actions", repo).bright_cyan()
    );
}

fn download_and_install(
    client: &reqwest::blocking::Client,
    download_url: &str,
    asset_name: &str,
    latest_version: &str,
    repo: &str,
) -> Result<()> {
    println!("{}", format!("Downloading {}...", asset_name).bright_cyan());

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
            repo,
            "--pattern",
            asset_name,
            "--dir",
            temp_dir.to_str().unwrap(),
        ])
        .output();

    match gh_output {
        Ok(output) if output.status.success() => {}
        _ => {
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

    let current_exe = std::env::current_exe().context("Failed to get current executable path")?;
    if !temp_archive.exists() {
        bail!("Downloaded archive not found at {}", temp_archive.display());
    }
    println!("{}", "Extracting archive...".bright_cyan());

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

    let extracted_binary = temp_dir.join("hive");
    if !extracted_binary.exists() {
        bail!(
            "Extracted binary not found at {}",
            extracted_binary.display()
        );
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&extracted_binary)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&extracted_binary, perms)?;
    }

    fs::rename(&extracted_binary, &current_exe).context("Failed to replace current binary")?;

    let _ = fs::remove_dir_all(&temp_dir);

    println!("{}", "✓ Binary updated successfully!".green().bold());

    Ok(())
}

/// Update skills by calling the install command
fn update_skills() -> Result<()> {
    use crate::commands::install;
    install::run(true, false)?;
    Ok(())
}
