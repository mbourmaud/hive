use anyhow::{bail, Context, Result};
use colored::Colorize;
use dialoguer::Confirm;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use super::provider::{BedrockConfig, Provider};

/// Claude wrapper profile
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Profile {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub claude_wrapper: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<Vec<(String, String)>>,
    #[serde(default)]
    pub provider: Provider,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bedrock: Option<BedrockConfig>,
    pub created: String,
    pub updated: String,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            description: Some("Default Claude profile".to_string()),
            claude_wrapper: "claude".to_string(),
            environment: None,
            provider: Provider::Anthropic,
            bedrock: None,
            created: chrono::Utc::now().to_rfc3339(),
            updated: chrono::Utc::now().to_rfc3339(),
        }
    }
}

fn get_profiles_dir() -> Result<PathBuf> {
    let config_dir = dirs::config_dir().context("Failed to get config directory")?;
    let profiles_dir = config_dir.join("hive").join("profiles");
    Ok(profiles_dir)
}

fn get_active_profile_path() -> Result<PathBuf> {
    let profiles_dir = get_profiles_dir()?;
    Ok(profiles_dir.join(".active"))
}

/// List all available profiles
pub fn list() -> Result<()> {
    let profiles_dir = get_profiles_dir()?;

    if !profiles_dir.exists() {
        println!("{}", "No profiles found".yellow());
        println!("\nRun 'hive profile create <name>' to create a profile");
        return Ok(());
    }

    let active_profile = get_active_profile().ok();

    println!("{}", "Claude Profiles".bright_cyan().bold());
    println!();

    let mut profiles = Vec::new();

    for entry in fs::read_dir(&profiles_dir)? {
        let entry = entry?;
        let path = entry.path();

        // Skip .active file
        if path.file_name().and_then(|n| n.to_str()) == Some(".active") {
            continue;
        }

        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            let contents = fs::read_to_string(&path)?;
            let profile: Profile = serde_json::from_str(&contents)?;
            profiles.push(profile);
        }
    }

    if profiles.is_empty() {
        println!("{}", "No profiles found".yellow());
        return Ok(());
    }

    // Sort by name
    profiles.sort_by(|a, b| a.name.cmp(&b.name));

    for profile in profiles {
        let is_active = active_profile.as_ref() == Some(&profile.name);
        let marker = if is_active {
            "● ".green().bold()
        } else {
            "  ".into()
        };

        println!("{}{}", marker, profile.name.bright_cyan());
        if let Some(ref desc) = profile.description {
            println!("  {}", desc.bright_black());
        }
        println!("  Wrapper: {}", profile.claude_wrapper.bright_white());
        if let Some(ref env) = profile.environment {
            if !env.is_empty() {
                println!("  Environment: {} vars", env.len());
            }
        }
        println!();
    }

    Ok(())
}

/// Create a new profile
pub fn create(name: String) -> Result<()> {
    let profiles_dir = get_profiles_dir()?;
    fs::create_dir_all(&profiles_dir)?;

    let profile_path = profiles_dir.join(format!("{}.json", name));

    if profile_path.exists() {
        bail!("Profile '{}' already exists", name);
    }

    let profile = Profile {
        name: name.clone(),
        description: None,
        claude_wrapper: "claude".to_string(),
        environment: None,
        provider: Provider::Anthropic,
        bedrock: None,
        created: chrono::Utc::now().to_rfc3339(),
        updated: chrono::Utc::now().to_rfc3339(),
    };

    let contents = serde_json::to_string_pretty(&profile)?;
    fs::write(&profile_path, contents)?;

    println!("{}", format!("✓ Profile '{}' created", name).green());
    println!(
        "  Path: {}",
        profile_path.display().to_string().bright_black()
    );
    println!();
    println!("Edit the profile file to customize:");
    println!("  - description: Profile description");
    println!("  - claude_wrapper: Path to Claude wrapper script");
    println!("  - environment: Environment variables as key-value pairs");

    Ok(())
}

/// Activate a profile
pub fn use_profile(name: String) -> Result<()> {
    let profiles_dir = get_profiles_dir()?;
    let profile_path = profiles_dir.join(format!("{}.json", name));

    if !profile_path.exists() {
        bail!(
            "Profile '{}' not found. Use 'hive profile list' to see available profiles.",
            name
        );
    }

    let active_path = get_active_profile_path()?;
    fs::write(&active_path, &name)?;

    println!("{}", format!("✓ Activated profile '{}'", name).green());

    Ok(())
}

/// Delete a profile
pub fn delete(name: String) -> Result<()> {
    let profiles_dir = get_profiles_dir()?;
    let profile_path = profiles_dir.join(format!("{}.json", name));

    if !profile_path.exists() {
        bail!("Profile '{}' not found", name);
    }

    // Check if it's the active profile
    let active_profile = get_active_profile().ok();
    if active_profile.as_ref() == Some(&name) {
        println!(
            "{}",
            format!("⚠ Profile '{}' is currently active", name).yellow()
        );
        let confirm = Confirm::new()
            .with_prompt("Are you sure you want to delete it?")
            .default(false)
            .interact()?;

        if !confirm {
            println!("{}", "Cancelled".yellow());
            return Ok(());
        }

        // Clear active profile
        let active_path = get_active_profile_path()?;
        if active_path.exists() {
            fs::remove_file(&active_path)?;
        }
    }

    fs::remove_file(&profile_path)?;

    println!("{}", format!("✓ Profile '{}' deleted", name).green());

    Ok(())
}

/// Get the active profile name
pub fn get_active_profile() -> Result<String> {
    let active_path = get_active_profile_path()?;

    if !active_path.exists() {
        // Return default profile
        return Ok("default".to_string());
    }

    let name = fs::read_to_string(&active_path)?;
    Ok(name.trim().to_string())
}

/// Load the active profile
pub fn load_active_profile() -> Result<Profile> {
    let active_name = get_active_profile()?;
    load_profile(&active_name)
}

/// Load a profile by name.
pub fn load_profile(name: &str) -> Result<Profile> {
    let profiles_dir = get_profiles_dir()?;
    let profile_path = profiles_dir.join(format!("{name}.json"));

    if !profile_path.exists() {
        if name == "default" {
            return Ok(Profile::default());
        }
        bail!("Profile '{name}' not found");
    }

    let contents = fs::read_to_string(&profile_path)?;
    let profile: Profile = serde_json::from_str(&contents).context("Parsing profile")?;
    Ok(profile)
}

/// List all profiles as data (for API use, not CLI printing).
pub fn list_profiles() -> Result<Vec<Profile>> {
    let profiles_dir = get_profiles_dir()?;
    if !profiles_dir.exists() {
        return Ok(vec![Profile::default()]);
    }

    let mut profiles = Vec::new();
    for entry in fs::read_dir(&profiles_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.file_name().and_then(|n| n.to_str()) == Some(".active") {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            if let Ok(contents) = fs::read_to_string(&path) {
                if let Ok(profile) = serde_json::from_str::<Profile>(&contents) {
                    profiles.push(profile);
                }
            }
        }
    }

    if profiles.is_empty() {
        profiles.push(Profile::default());
    }
    profiles.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(profiles)
}

/// Save a profile to disk.
pub fn save_profile(profile: &Profile) -> Result<()> {
    let profiles_dir = get_profiles_dir()?;
    fs::create_dir_all(&profiles_dir)?;
    let profile_path = profiles_dir.join(format!("{}.json", profile.name));
    let contents = serde_json::to_string_pretty(profile)?;
    fs::write(&profile_path, contents)?;
    Ok(())
}
