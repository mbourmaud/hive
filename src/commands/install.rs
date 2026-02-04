use anyhow::{Context, Result};
use colored::Colorize;
use rust_embed::RustEmbed;
use serde_json::Value;
use std::fs;
use std::path::Path;

#[derive(RustEmbed)]
#[folder = "commands/"]
struct Skills;

pub fn run(skills_only: bool, bin_only: bool) -> Result<()> {
    let home = dirs::home_dir().context("Could not find home directory")?;

    // Install binary unless --skills-only
    if !skills_only {
        install_binary(&home)?;
    }

    // Install skills and MCP server unless --bin-only
    if !bin_only {
        install_skills(&home)?;
        install_mcp_server(&home)?;
    }

    println!("\n{} Hive installation complete!", "✓".green().bold());

    if !bin_only {
        println!("\nRun {} to get started.", "hive init".cyan().bold());
    }

    Ok(())
}

fn install_binary(home: &Path) -> Result<()> {
    let bin_dir = home.join(".local").join("bin");
    fs::create_dir_all(&bin_dir).context("Failed to create ~/.local/bin directory")?;

    let current_exe = std::env::current_exe().context("Failed to get current executable path")?;

    let target_path = bin_dir.join("hive");

    // Copy the binary
    fs::copy(&current_exe, &target_path).context("Failed to copy binary to ~/.local/bin/hive")?;

    // Make it executable on Unix systems
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&target_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&target_path, perms)?;
    }

    println!(
        "{} Binary installed to {}",
        "✓".green().bold(),
        target_path.display().to_string().cyan()
    );

    // Check if ~/.local/bin is in PATH
    if let Ok(path) = std::env::var("PATH") {
        if !path.split(':').any(|p| p == bin_dir.to_str().unwrap_or("")) {
            println!("\n{} Add ~/.local/bin to your PATH:", "⚠".yellow().bold());
            println!("  echo 'export PATH=\"$HOME/.local/bin:$PATH\"' >> ~/.bashrc");
            println!("  echo 'export PATH=\"$HOME/.local/bin:$PATH\"' >> ~/.zshrc");
        }
    }

    Ok(())
}

fn install_skills(home: &Path) -> Result<()> {
    let skills_dir = home.join(".claude").join("commands");
    fs::create_dir_all(&skills_dir).context("Failed to create ~/.claude/commands directory")?;

    let mut installed_count = 0;

    // Iterate through embedded skills
    for file in Skills::iter() {
        if file.ends_with(".md") {
            let content =
                Skills::get(&file).context(format!("Failed to get embedded skill: {}", file))?;

            let target_path = skills_dir.join(file.as_ref());

            fs::write(&target_path, content.data.as_ref())
                .context(format!("Failed to write skill file: {}", file))?;

            installed_count += 1;
        }
    }

    println!(
        "{} {} skills installed to {}",
        "✓".green().bold(),
        installed_count,
        skills_dir.display().to_string().cyan()
    );

    Ok(())
}

fn install_mcp_server(home: &Path) -> Result<()> {
    let settings_path = home.join(".claude").join("settings.json");

    // Read existing settings or create a new object
    let mut settings: Value = if settings_path.exists() {
        let content =
            fs::read_to_string(&settings_path).context("Failed to read ~/.claude/settings.json")?;
        serde_json::from_str(&content).context("Failed to parse ~/.claude/settings.json")?
    } else {
        serde_json::json!({})
    };

    // Ensure mcpServers object exists
    if settings.get("mcpServers").is_none() {
        settings["mcpServers"] = serde_json::json!({});
    }

    let mcp_servers = settings["mcpServers"]
        .as_object_mut()
        .context("mcpServers is not an object")?;

    // Check if hive MCP server is already registered
    if mcp_servers.contains_key("hive") {
        println!(
            "{} MCP server already registered in {}",
            "✓".green().bold(),
            "~/.claude/settings.json".cyan()
        );
        return Ok(());
    }

    // Add hive MCP server
    mcp_servers.insert(
        "hive".to_string(),
        serde_json::json!({
            "command": "hive",
            "args": ["mcp-server"]
        }),
    );

    // Write back settings with pretty formatting
    let formatted =
        serde_json::to_string_pretty(&settings).context("Failed to serialize settings")?;
    fs::write(&settings_path, formatted).context("Failed to write ~/.claude/settings.json")?;

    println!(
        "{} MCP server registered in {}",
        "✓".green().bold(),
        "~/.claude/settings.json".cyan()
    );

    Ok(())
}
