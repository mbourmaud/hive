use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::fs;
use std::path::PathBuf;
use std::process::Command as ProcessCommand;

use crate::agent_teams;
use crate::backend::{self, SpawnConfig};
use crate::config;
use crate::types::{DroneState, DroneStatus, ExecutionMode, Prd};

#[derive(Debug)]
struct WorktreeInfo {
    path: PathBuf,
    branch: String,
    prunable: bool,
}

#[allow(clippy::too_many_arguments)]
pub fn run(
    name: String,
    _prompt: Option<String>,
    resume: bool,
    local: bool,
    model: String,
    dry_run: bool,
    team: bool,
    teammate_mode: String,
) -> Result<()> {
    let mode_label = if team { "team" } else { "drone" };
    println!(
        "{} Launching {} '{}'...",
        "→".bright_blue(),
        mode_label,
        name.bright_cyan()
    );

    // 1. Check if drone already exists
    let drone_dir = PathBuf::from(".hive/drones").join(&name);
    if drone_dir.exists() && !resume {
        bail!("Drone '{}' already exists. Use --resume to resume.", name);
    }

    // 2. Find PRD
    let prd_path = find_prd(&name)?;
    let prd = load_prd(&prd_path)?;
    println!("  {} Found PRD: {}", "✓".green(), prd.title);

    // 3. Determine branch and check for existing worktree
    let default_branch = format!("hive/{}", name);
    let branch = prd.target_branch.as_deref().unwrap_or(&default_branch);
    let base_branch = prd.base_branch.as_deref();

    // Log base branch info
    if let Some(base) = base_branch {
        println!("  {} Base branch: {}", "→".bright_blue(), base);
    }

    // Determine execution mode
    // --team forces Opus model for best multi-agent coordination
    let model = if team && model == "sonnet" {
        println!(
            "  {} Team mode: auto-switching to {} for multi-agent coordination",
            "→".bright_blue(),
            "opus".bright_cyan()
        );
        "opus".to_string()
    } else {
        model
    };

    let execution_mode = if team {
        ExecutionMode::AgentTeam
    } else {
        ExecutionMode::Worktree
    };

    // 4. Handle worktree creation
    let worktree_path = if local {
        std::env::current_dir()?
    } else if team {
        // Agent Teams mode: single worktree, Opus handles parallelization via Agent Teams
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
    } else if resume {
        // On resume, check if worktree already exists for this branch
        match find_existing_worktree(branch)? {
            Some(existing) => {
                if existing.prunable {
                    println!(
                        "  {} Found prunable worktree at {}",
                        "⚠".yellow(),
                        existing.path.display()
                    );
                    println!("  {} Pruning and recreating...", "→".bright_blue());

                    let output = ProcessCommand::new("git")
                        .args([
                            "worktree",
                            "remove",
                            "--force",
                            existing.path.to_str().unwrap(),
                        ])
                        .output()?;

                    if !output.status.success() {
                        bail!(
                            "Failed to remove prunable worktree: {}",
                            String::from_utf8_lossy(&output.stderr)
                        );
                    }

                    let worktree_base = config::get_worktree_base()?;
                    let project_name = get_project_name()?;
                    let new_path = worktree_base.join(&project_name).join(&name);
                    create_worktree(&new_path, branch, base_branch)?;
                    println!(
                        "  {} Created worktree at {}",
                        "✓".green(),
                        new_path.display()
                    );
                    new_path
                } else {
                    println!(
                        "  {} Found existing worktree at {}",
                        "✓".green(),
                        existing.path.display()
                    );
                    existing.path
                }
            }
            None => {
                let worktree_base = config::get_worktree_base()?;
                let project_name = get_project_name()?;
                let new_path = worktree_base.join(&project_name).join(&name);
                create_worktree(&new_path, branch, base_branch)?;
                println!(
                    "  {} Created worktree at {}",
                    "✓".green(),
                    new_path.display()
                );
                new_path
            }
        }
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

    // 6. Agent Teams: seed task list for Opus to parallelize
    if team {
        let tasks = agent_teams::translate_stories_to_tasks(&prd);
        agent_teams::seed_task_list(&name, &tasks)?;
        println!(
            "  {} Seeded {} tasks for Agent Teams",
            "✓".green(),
            tasks.len()
        );
    }

    // 7. Create drone status
    let backend_name = if team { "agent_team" } else { "native" };
    fs::create_dir_all(&drone_dir)?;
    let status = DroneStatus {
        drone: name.clone(),
        prd: prd_path.file_name().unwrap().to_string_lossy().to_string(),
        branch: branch.to_string(),
        worktree: worktree_path.to_string_lossy().to_string(),
        local_mode: local,
        execution_mode: execution_mode.clone(),
        backend: backend_name.to_string(),
        status: DroneState::Starting,
        current_story: None,
        completed: Vec::new(),
        story_times: std::collections::HashMap::new(),
        total: prd.stories.len(),
        started: chrono::Utc::now().to_rfc3339(),
        updated: chrono::Utc::now().to_rfc3339(),
        error_count: 0,
        last_error_story: None,
        blocked_reason: None,
        blocked_questions: Vec::new(),
        awaiting_human: false,
        active_agents: std::collections::HashMap::new(),
    };

    let status_path = drone_dir.join("status.json");
    let status_json = serde_json::to_string_pretty(&status)?;
    fs::write(&status_path, status_json)?;
    println!("  {} Created status.json", "✓".green());

    // Ensure inbox/outbox directories exist for inter-drone messaging
    let inbox_dir = drone_dir.join("inbox");
    let outbox_dir = drone_dir.join("outbox");
    fs::create_dir_all(&inbox_dir)?;
    fs::create_dir_all(&outbox_dir)?;

    // 8. Launch Claude via ExecutionBackend
    if dry_run {
        println!("  {} Dry run - not launching Claude", "→".yellow());
    } else {
        let backend: Box<dyn crate::backend::ExecutionBackend> = if team {
            backend::resolve_agent_team_backend()
        } else {
            backend::resolve_backend(None)
        };

        let spawn_config = SpawnConfig {
            drone_name: name.clone(),
            prd_path: prd_path.clone(),
            model: model.clone(),
            worktree_path: worktree_path.clone(),
            status_file: status_path.clone(),
            working_dir: worktree_path.clone(),
            execution_mode: execution_mode.clone(),
            wait: false,
            team_name: if team { Some(name.clone()) } else { None },
            teammate_mode: if team {
                Some(teammate_mode.clone())
            } else {
                None
            },
            worktree_assignments: None,
        };

        backend.spawn(&spawn_config)?;

        if team {
            println!(
                "  {} Launched Agent Teams lead (model: {})",
                "✓".green(),
                model.bright_cyan()
            );
        } else {
            println!(
                "  {} Launched Claude (model: {})",
                "✓".green(),
                model.bright_cyan()
            );
        }
    }

    // 9. Send notification
    let mode_emoji = if team { "🤝" } else { "🐝" };
    crate::notifications::notify(
        &format!("{} {}", mode_emoji, name),
        &format!(
            "started ({} stories, {} mode)",
            prd.stories.len(),
            execution_mode
        ),
    );

    println!(
        "\n{} {} '{}' is running!",
        "✓".green().bold(),
        if team { "Team" } else { "Drone" },
        name.bright_cyan()
    );
    println!("\nMonitor progress:");
    println!("  hive monitor {}", name);
    println!("  hive logs {}", name);

    Ok(())
}

fn find_prd(name: &str) -> Result<PathBuf> {
    let prds_dir = PathBuf::from(".hive/prds");

    if !prds_dir.exists() {
        bail!("No PRDs directory found. Run 'hive init' first.");
    }

    let mut candidates = Vec::new();

    // Search patterns: prd-<name>.json, <name>.json, <name>-prd.json
    let patterns = vec![
        prds_dir.join(format!("prd-{}.json", name)),
        prds_dir.join(format!("{}.json", name)),
        prds_dir.join(format!("{}-prd.json", name)),
    ];

    for pattern in patterns {
        if pattern.exists() {
            candidates.push(pattern);
        }
    }

    // Also search in project root for prd*.json files
    let root_dir = std::env::current_dir()?;
    if let Ok(entries) = fs::read_dir(&root_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    if filename.starts_with("prd") && filename.ends_with(".json") {
                        candidates.push(path);
                    }
                }
            }
        }
    }

    // If no candidates found, list available PRDs
    if candidates.is_empty() {
        let mut available = Vec::new();
        for entry in fs::read_dir(&prds_dir)?.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    available.push(filename.to_string());
                }
            }
        }

        if available.is_empty() {
            bail!(
                "No PRD found for drone '{}'. No PRDs available in .hive/prds/",
                name
            );
        } else {
            bail!(
                "No PRD found for drone '{}'. Available PRDs:\n  {}",
                name,
                available.join("\n  ")
            );
        }
    }

    // If only one candidate, use it
    if candidates.len() == 1 {
        return Ok(candidates.into_iter().next().unwrap());
    }

    // Multiple candidates - prompt user to select
    use dialoguer::Select;

    println!("{}", "Multiple PRD files found:".bright_yellow());
    let selection = Select::new()
        .items(
            &candidates
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>(),
        )
        .default(0)
        .interact()?;

    Ok(candidates[selection].clone())
}

fn load_prd(path: &PathBuf) -> Result<Prd> {
    let contents = fs::read_to_string(path).context("Failed to read PRD")?;
    let prd: Prd = serde_json::from_str(&contents).context("Failed to parse PRD")?;
    Ok(prd)
}

fn get_project_name() -> Result<String> {
    std::env::current_dir()?
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .context("Failed to get directory name")
}

pub fn create_worktree(
    path: &std::path::Path,
    branch: &str,
    explicit_base: Option<&str>,
) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Fetch latest from origin to ensure we have up-to-date refs
    println!("  {} Fetching latest from origin...", "→".bright_blue());
    let _ = ProcessCommand::new("git")
        .args(["fetch", "origin"])
        .output();

    // Determine the base ref for the worktree
    let base_ref = if let Some(base) = explicit_base {
        if base == "master" || base == "main" {
            let remote_ref = format!("origin/{}", base);
            let exists = ProcessCommand::new("git")
                .args(["rev-parse", "--verify", &remote_ref])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            if exists {
                remote_ref
            } else {
                base.to_string()
            }
        } else {
            base.to_string()
        }
    } else {
        get_worktree_base_ref(branch)?
    };

    let branch_exists = ProcessCommand::new("git")
        .args(["rev-parse", "--verify", branch])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let output = if branch_exists {
        ProcessCommand::new("git")
            .args(["worktree", "add", path.to_str().unwrap(), branch])
            .output()
            .context("Failed to create worktree")?
    } else {
        println!(
            "  {} Creating branch '{}' from '{}'",
            "→".bright_blue(),
            branch,
            base_ref
        );
        ProcessCommand::new("git")
            .args([
                "worktree",
                "add",
                "-b",
                branch,
                path.to_str().unwrap(),
                &base_ref,
            ])
            .output()
            .context("Failed to create worktree")?
    };

    if !output.status.success() {
        bail!(
            "Failed to create worktree: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

fn get_worktree_base_ref(branch: &str) -> Result<String> {
    let is_main_branch = branch == "master" || branch == "main";

    if is_main_branch {
        let remote_ref = format!("origin/{}", branch);
        let exists = ProcessCommand::new("git")
            .args(["rev-parse", "--verify", &remote_ref])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if exists {
            return Ok(remote_ref);
        }
        return Ok(branch.to_string());
    }

    let local_exists = ProcessCommand::new("git")
        .args(["rev-parse", "--verify", branch])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if local_exists {
        return Ok(branch.to_string());
    }

    let remote_ref = format!("origin/{}", branch);
    let remote_exists = ProcessCommand::new("git")
        .args(["rev-parse", "--verify", &remote_ref])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if remote_exists {
        return Ok(remote_ref);
    }

    for default_branch in &["origin/master", "origin/main"] {
        let exists = ProcessCommand::new("git")
            .args(["rev-parse", "--verify", default_branch])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if exists {
            return Ok(default_branch.to_string());
        }
    }

    Ok("HEAD".to_string())
}

fn create_hive_symlink(worktree: &std::path::Path) -> Result<()> {
    let hive_dir = std::env::current_dir()?.join(".hive");
    let symlink_path = worktree.join(".hive");

    if symlink_path.exists() {
        if symlink_path.is_symlink() {
            fs::remove_file(&symlink_path)?;
        } else if symlink_path.is_dir() {
            fs::remove_dir_all(&symlink_path)?;
        } else {
            fs::remove_file(&symlink_path)?;
        }
    }

    std::os::unix::fs::symlink(&hive_dir, &symlink_path)
        .context("Failed to create .hive symlink")?;

    Ok(())
}

fn list_worktrees() -> Result<Vec<WorktreeInfo>> {
    let output = ProcessCommand::new("git")
        .args(["worktree", "list", "--porcelain"])
        .output()
        .context("Failed to list worktrees")?;

    if !output.status.success() {
        bail!(
            "Failed to list worktrees: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut worktrees = Vec::new();
    let mut current_worktree = None::<WorktreeInfo>;

    for line in stdout.lines() {
        if line.starts_with("worktree ") {
            if let Some(wt) = current_worktree.take() {
                worktrees.push(wt);
            }
            let path = line.strip_prefix("worktree ").unwrap().trim();
            current_worktree = Some(WorktreeInfo {
                path: PathBuf::from(path),
                branch: String::new(),
                prunable: false,
            });
        } else if line.starts_with("branch ") {
            if let Some(ref mut wt) = current_worktree {
                let branch = line
                    .strip_prefix("branch ")
                    .unwrap()
                    .strip_prefix("refs/heads/")
                    .unwrap_or(line.strip_prefix("branch ").unwrap())
                    .trim();
                wt.branch = branch.to_string();
            }
        } else if line == "prunable" {
            if let Some(ref mut wt) = current_worktree {
                wt.prunable = true;
            }
        }
    }

    if let Some(wt) = current_worktree {
        worktrees.push(wt);
    }

    Ok(worktrees)
}

fn find_existing_worktree(branch: &str) -> Result<Option<WorktreeInfo>> {
    let worktrees = list_worktrees()?;

    for wt in worktrees {
        if wt.branch == branch {
            return Ok(Some(wt));
        }
    }

    Ok(None)
}
