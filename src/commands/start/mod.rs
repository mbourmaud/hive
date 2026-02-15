mod hooks;
mod plan_loading;
mod worktree;

use anyhow::{bail, Result};
use colored::Colorize;
use std::fs;
use std::path::PathBuf;
use std::process::Command as ProcessCommand;

use crate::backend::{self, SpawnConfig};
use crate::commands::profile;
use crate::config;
use crate::types::{DroneState, DroneStatus, ExecutionMode};

// Re-export submodule items (used by run() and tests via `use super::*`)
#[allow(unused_imports)]
pub(crate) use hooks::{
    detect_project_languages, get_git_remote_url, write_hooks_config, write_hooks_config_at,
};
#[allow(unused_imports)]
pub(crate) use plan_loading::{find_plan, load_plan, parse_frontmatter};
#[allow(unused_imports)]
pub(crate) use worktree::{create_hive_symlink, create_worktree, get_project_name};

pub fn run(
    name: String,
    local: bool,
    model: String,
    max_agents: usize,
    dry_run: bool,
) -> Result<()> {
    // 0. Load active profile to get Claude binary and environment
    let active_profile = profile::load_active_profile()?;

    // 1. Auto-resume logic: if drone exists, check if process is alive
    let drone_dir = PathBuf::from(".hive/drones").join(&name);
    let is_resume = if drone_dir.exists() {
        let pid_alive = crate::commands::common::read_drone_pid(&name)
            .map(crate::commands::common::is_process_running)
            .unwrap_or(false);
        if pid_alive {
            bail!(
                "Drone '{}' is already running. Use 'hive stop {}' first.",
                name,
                name
            );
        }
        // Process is dead — auto-resume
        println!(
            "{} Resuming team '{}'...",
            "→".bright_blue(),
            name.bright_cyan()
        );
        true
    } else {
        println!(
            "{} Launching team '{}'...",
            "→".bright_blue(),
            name.bright_cyan()
        );
        false
    };

    if active_profile.name != "default" {
        println!(
            "  {} Using profile: {}",
            "→".bright_blue(),
            active_profile.name.bright_cyan()
        );
    }

    // 2. Find plan
    let prd_path = find_plan(&name)?;
    let prd = load_plan(&prd_path)?;
    println!("  {} Found plan: {}", "✓".green(), prd.title());

    // 3. Determine branch and check for existing worktree
    let default_branch = format!("hive/{}", name);
    let branch = prd.target_branch.as_deref().unwrap_or(&default_branch);
    let base_branch = prd.base_branch.as_deref();

    // Log base branch info
    if let Some(base) = base_branch {
        println!("  {} Base branch: {}", "→".bright_blue(), base);
    }

    // Show model info
    println!(
        "  {} Model: {} (max agents: {})",
        "→".bright_blue(),
        model.bright_cyan(),
        max_agents.to_string().bright_cyan()
    );

    // 4. Handle worktree creation
    let worktree_path = if local {
        std::env::current_dir()?
    } else {
        let worktree_base = config::get_worktree_base()?;
        let project_name = get_project_name()?;
        let new_path = worktree_base.join(&project_name).join(&name);

        if !new_path.exists() {
            create_worktree(&new_path, branch, base_branch)?;
            println!("  {} Created worktree", "✓".green());
        } else {
            println!("  {} Using existing worktree", "✓".green());
        }

        new_path
    };

    println!("  {} Worktree: {}", "✓".green(), worktree_path.display());

    // 5. Create .hive symlink in worktree
    if !local {
        create_hive_symlink(&worktree_path)?;
        println!("  {} Symlinked .hive", "✓".green());
    }

    // 6. Write Claude Code hooks config for event streaming
    write_hooks_config(&worktree_path, &name)?;
    println!("  {} Configured hooks", "✓".green());

    // 7. Create or update drone status
    fs::create_dir_all(&drone_dir)?;
    let status_path = drone_dir.join("status.json");

    if is_resume {
        update_drone_status_resume(&status_path)?;
        println!("  {} Updated status.json (resuming)", "✓".green());
    } else {
        create_drone_status(
            &name,
            &prd_path,
            branch,
            &worktree_path,
            local,
            &status_path,
        )?;
        println!("  {} Created status.json", "✓".green());
    }

    // Ensure inbox/outbox directories exist for inter-drone messaging
    fs::create_dir_all(drone_dir.join("inbox"))?;
    fs::create_dir_all(drone_dir.join("outbox"))?;

    // 8. Verify jq is available (required for event streaming hooks)
    if !ProcessCommand::new("jq")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        bail!("'jq' is required for event streaming but not found. Install: brew install jq");
    }

    // 9. Run environment setup (mandatory for Hive to work)
    println!("  {} Running environment setup...", "→".bright_blue());
    crate::commands::setup::run_setup(&worktree_path)?;
    println!("  {} Environment setup complete", "✓".green());

    // 10. Launch Claude via ExecutionBackend
    if !dry_run {
        let remote_url = get_git_remote_url(&worktree_path);
        let project_languages = detect_project_languages(&worktree_path);
        let spawn_config = SpawnConfig {
            drone_name: name.clone(),
            prd_path: prd_path.clone(),
            model: model.clone(),
            worktree_path: worktree_path.clone(),
            status_file: status_path.clone(),
            working_dir: worktree_path.clone(),
            wait: false,
            team_name: name.clone(),
            max_agents,
            claude_binary: active_profile.claude_wrapper.clone(),
            environment: active_profile.environment.clone(),
            structured_tasks: prd.structured_tasks.clone(),
            remote_url,
            project_languages,
            mode: "native".to_string(),
        };

        let handle = backend::resolve_backend().spawn(&spawn_config)?;

        if let Some(pid) = handle.pid {
            let _ = fs::write(drone_dir.join(".pid"), pid.to_string());
        }

        println!(
            "  {} Launched native team (model: {}, max agents: {})",
            "✓".green(),
            model.bright_cyan(),
            max_agents.to_string().bright_cyan()
        );
    } else {
        println!("  {} Dry run - not launching Claude", "→".yellow());
    }

    println!(
        "\n{} Team '{}' is running!",
        "✓".green().bold(),
        name.bright_cyan()
    );
    println!("\nMonitor progress:");
    println!("  hive monitor {}", name);
    println!("  hive logs {}", name);

    Ok(())
}

fn update_drone_status_resume(status_path: &std::path::Path) -> Result<()> {
    if let Ok(contents) = fs::read_to_string(status_path) {
        if let Ok(mut existing_status) = serde_json::from_str::<DroneStatus>(&contents) {
            existing_status.status = DroneState::Resuming;
            existing_status.updated = chrono::Utc::now().to_rfc3339();
            let _ = fs::write(status_path, serde_json::to_string_pretty(&existing_status)?);
        }
    }
    Ok(())
}

fn create_drone_status(
    name: &str,
    prd_path: &std::path::Path,
    branch: &str,
    worktree_path: &std::path::Path,
    local: bool,
    status_path: &std::path::Path,
) -> Result<()> {
    let status = DroneStatus {
        drone: name.to_string(),
        prd: prd_path.file_name().unwrap().to_string_lossy().to_string(),
        branch: branch.to_string(),
        worktree: worktree_path.to_string_lossy().to_string(),
        local_mode: local,
        execution_mode: ExecutionMode::AgentTeam,
        backend: "agent_team".to_string(),
        status: DroneState::Starting,
        current_task: None,
        completed: Vec::new(),
        story_times: Default::default(),
        total: 0,
        started: chrono::Utc::now().to_rfc3339(),
        updated: chrono::Utc::now().to_rfc3339(),
        error_count: 0,
        last_error: None,
        lead_model: Some("opus".to_string()),
        active_agents: Default::default(),
    };

    let status_json = serde_json::to_string_pretty(&status)?;
    fs::write(status_path, status_json)?;
    Ok(())
}

#[cfg(test)]
mod tests;
