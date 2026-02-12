use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

use crate::backend::{self, SpawnConfig};
use crate::commands::profile;
use crate::config;
use crate::types::{DroneState, DroneStatus, ExecutionMode, LegacyJsonPlan, Plan};

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

    // Team lead always uses Opus; show the effective model
    println!(
        "  {} Team lead: {} (teammates: {})",
        "→".bright_blue(),
        "opus".bright_cyan(),
        model.bright_cyan()
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

    // 5b. Write Claude Code hooks config for event streaming
    write_hooks_config(&worktree_path, &name)?;
    println!("  {} Configured hooks", "✓".green());

    // 6. Create or update drone status
    fs::create_dir_all(&drone_dir)?;
    let status_path = drone_dir.join("status.json");

    if is_resume {
        // Update existing status to Resuming
        if let Ok(contents) = fs::read_to_string(&status_path) {
            if let Ok(mut existing_status) = serde_json::from_str::<DroneStatus>(&contents) {
                existing_status.status = DroneState::Resuming;
                existing_status.updated = chrono::Utc::now().to_rfc3339();
                let _ = fs::write(
                    &status_path,
                    serde_json::to_string_pretty(&existing_status)?,
                );
            }
        }
        println!("  {} Updated status.json (resuming)", "✓".green());
    } else {
        let status = DroneStatus {
            drone: name.clone(),
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
        fs::write(&status_path, status_json)?;
        println!("  {} Created status.json", "✓".green());
    }

    // Ensure inbox/outbox directories exist for inter-drone messaging
    let inbox_dir = drone_dir.join("inbox");
    let outbox_dir = drone_dir.join("outbox");
    fs::create_dir_all(&inbox_dir)?;
    fs::create_dir_all(&outbox_dir)?;

    // 7. Verify jq is available (required for event streaming hooks)
    if !ProcessCommand::new("jq")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        bail!("'jq' is required for event streaming but not found. Install: brew install jq");
    }

    // 7b. ALWAYS run environment setup — mandatory for Hive to work
    println!("  {} Running environment setup...", "→".bright_blue());
    crate::commands::setup::run_setup(&worktree_path)?;
    println!("  {} Environment setup complete", "✓".green());

    // 7c. Pre-seed tasks
    let seeded = crate::agent_teams::preseed_tasks(&name, &prd.structured_tasks, &drone_dir)?;
    if !seeded.is_empty() {
        println!("  {} Pre-seeded {} tasks", "✓".green(), seeded.len());
    }

    // 8. Launch Claude via ExecutionBackend
    if dry_run {
        println!("  {} Dry run - not launching Claude", "→".yellow());
    } else {
        // Detect git remote URL for PR/MR instructions
        let remote_url = get_git_remote_url(&worktree_path);

        let backend = backend::resolve_agent_team_backend();

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
        };

        let handle = backend.spawn(&spawn_config)?;

        // Persist PID so zombie detection can check if the process is still alive
        if let Some(pid) = handle.pid {
            let pid_path = drone_dir.join(".pid");
            let _ = fs::write(&pid_path, pid.to_string());
        }

        println!(
            "  {} Launched Agent Teams lead (lead: {}, teammates: {}, max: {})",
            "✓".green(),
            "opus".bright_cyan(),
            model.bright_cyan(),
            max_agents.to_string().bright_cyan()
        );
    }

    // 8. Notification (only for completions, not start)
    // Removed start notification to reduce noise

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

fn find_plan(name: &str) -> Result<PathBuf> {
    // Search in .hive/plans/ first, fall back to .hive/prds/ for backwards compat
    // Note: prds/ is often a symlink to plans/, so only search one directory
    let plans_dir = PathBuf::from(".hive/plans");
    let prds_dir = PathBuf::from(".hive/prds");

    let search_dir = if plans_dir.exists() {
        plans_dir
    } else if prds_dir.exists() {
        prds_dir
    } else {
        bail!("No plans directory found. Run 'hive init' first.");
    };

    // Search in priority order: markdown first (preferred), then legacy JSON (backward compat)
    let candidates = [
        format!("{}.md", name),
        format!("plan-{}.md", name),
        format!("plan-{}.json", name),
        format!("{}.json", name),
        format!("prd-{}.json", name),
    ];

    for filename in &candidates {
        let path = search_dir.join(filename);
        if path.exists() {
            return Ok(path);
        }
    }

    // No candidates found — list available plans
    let mut available = Vec::new();
    for entry in fs::read_dir(&search_dir).into_iter().flatten().flatten() {
        let path = entry.path();
        let ext = path.extension().and_then(|s| s.to_str());
        if ext == Some("md") || ext == Some("json") {
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                available.push(filename.to_string());
            }
        }
    }

    if available.is_empty() {
        bail!(
            "No plan found for drone '{}'. No plans available in .hive/plans/",
            name
        );
    } else {
        bail!(
            "No plan found for drone '{}'. Available plans:\n  {}",
            name,
            available.join("\n  ")
        );
    }
}

fn load_plan(path: &Path) -> Result<Plan> {
    let contents = fs::read_to_string(path).context("Failed to read plan")?;

    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let plan = match ext {
        "md" => {
            // Markdown plan: ID from filename, content is the raw markdown
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            // Parse YAML frontmatter for target_branch/base_branch
            let (target_branch, base_branch, content) = parse_frontmatter(&contents);

            // Parse structured tasks from ## Tasks section
            let structured_tasks = crate::plan_parser::parse_tasks(&content);

            Plan {
                id,
                content,
                target_branch,
                base_branch,
                structured_tasks,
            }
        }
        "json" => {
            // Legacy JSON plan: convert to Plan
            let legacy: LegacyJsonPlan =
                serde_json::from_str(&contents).context("Failed to parse plan JSON")?;
            legacy.into()
        }
        _ => bail!("Unsupported plan file format: {}", ext),
    };

    // Validate non-empty content
    if plan.content.trim().is_empty() {
        bail!("Plan content cannot be empty");
    }

    Ok(plan)
}

/// Read the origin remote URL from a git repo. Returns empty string if unavailable.
fn get_git_remote_url(worktree_path: &Path) -> String {
    ProcessCommand::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(worktree_path)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

/// Parse optional YAML frontmatter from markdown content.
/// Returns (target_branch, base_branch, content_without_frontmatter).
fn parse_frontmatter(raw: &str) -> (Option<String>, Option<String>, String) {
    let trimmed = raw.trim_start();
    if !trimmed.starts_with("---") {
        return (None, None, raw.to_string());
    }

    // Find the closing ---
    let after_opening = &trimmed[3..];
    if let Some(end) = after_opening.find("\n---") {
        let frontmatter = &after_opening[..end];
        let rest = &after_opening[end + 4..]; // skip \n---

        let mut target_branch = None;
        let mut base_branch = None;

        for line in frontmatter.lines() {
            let line = line.trim();
            if let Some(value) = line.strip_prefix("target_branch:") {
                target_branch = Some(value.trim().to_string());
            } else if let Some(value) = line.strip_prefix("base_branch:") {
                base_branch = Some(value.trim().to_string());
            }
        }

        // Strip leading newline from rest
        let content = rest.strip_prefix('\n').unwrap_or(rest);
        (target_branch, base_branch, content.to_string())
    } else {
        (None, None, raw.to_string())
    }
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

/// Write `.claude/settings.json` in the worktree with hooks that stream events
/// to `.hive/drones/{name}/events.ndjson`.
fn write_hooks_config(worktree: &Path, drone_name: &str) -> Result<()> {
    let claude_dir = worktree.join(".claude");
    fs::create_dir_all(&claude_dir)?;
    let settings_path = claude_dir.join("settings.json");

    // Load existing settings if present
    let mut settings: serde_json::Value = if settings_path.exists() {
        let contents = fs::read_to_string(&settings_path).unwrap_or_else(|_| "{}".to_string());
        serde_json::from_str(&contents).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // Use absolute path for drone directory — $CLAUDE_PROJECT_DIR is unreliable
    let drone_dir = std::env::current_dir()?
        .join(".hive/drones")
        .join(drone_name);
    fs::create_dir_all(&drone_dir)?;
    let events_file = drone_dir
        .join("events.ndjson")
        .to_string_lossy()
        .to_string();
    let messages_file = drone_dir
        .join("messages.ndjson")
        .to_string_lossy()
        .to_string();

    // Build hook commands — each appends one line to events.ndjson via jq (lean, no persistence scripts)
    //
    // Hooks use PreToolUse (fires before execution) for task/agent/message tracking,
    // PostToolUse (fires after) for tool completion tracking, and
    // SubagentStart/SubagentStop for agent lifecycle.
    let hooks = serde_json::json!({
        "PreToolUse": [
            {
                "matcher": "Task",
                "hooks": [{
                    "type": "command",
                    "command": format!(
                        r#"cat | jq -c '{{event:"AgentSpawn",ts:(now|todate),name:(.tool_input.description // .tool_input.name // ""),model:(.tool_input.model // null),subagent_type:(.tool_input.subagent_type // null)}}' >> {}"#,
                        events_file
                    ),
                    "async": true,
                    "timeout": 5
                }]
            },
            {
                "matcher": "TodoWrite",
                "hooks": [{
                    "type": "command",
                    "command": format!(
                        r#"cat | jq -c '{{event:"TodoSnapshot",ts:(now|todate),todos:[.tool_input.todos[]? | {{content:.content,status:(.status // "pending"),activeForm:(.activeForm // null)}}]}}' >> {}"#,
                        events_file
                    ),
                    "async": true,
                    "timeout": 5
                }]
            },
            {
                "matcher": "SendMessage",
                "hooks": [{
                    "type": "command",
                    "command": format!(
                        r#"INPUT=$(cat); echo "$INPUT" | jq -c '{{event:"Message",ts:(now|todate),recipient:(.tool_input.recipient // ""),summary:(.tool_input.summary // (.tool_input.content // "" | .[0:200]))}}' >> {} && \
    echo "$INPUT" | jq -c '{{timestamp:(now|todate),from:"{}",to:(.tool_input.recipient // ""),content:(.tool_input.content // ""),summary:(.tool_input.summary // "")}}' >> {}"#,
                        events_file, drone_name, messages_file
                    ),
                    "async": true,
                    "timeout": 5
                }]
            },
            {
                "matcher": "TaskCreate",
                "hooks": [{
                    "type": "command",
                    "command": format!(
                        r#"cat | jq -c '{{event:"TaskCreate",ts:(now|todate),subject:(.tool_input.subject // ""),description:(.tool_input.description // "")}}' >> {}"#,
                        events_file
                    ),
                    "async": true,
                    "timeout": 5
                }]
            },
            {
                "matcher": "TaskUpdate",
                "hooks": [{
                    "type": "command",
                    "command": format!(
                        r#"cat | jq -c '{{event:"TaskUpdate",ts:(now|todate),task_id:(.tool_input.taskId // ""),status:(.tool_input.status // null),owner:(.tool_input.owner // null)}}' >> {}"#,
                        events_file
                    ),
                    "async": true,
                    "timeout": 5
                }]
            }
        ],
        "PostToolUse": [
            {
                "hooks": [{
                    "type": "command",
                    "command": format!(
                        r#"cat | jq -c '{{event:"ToolDone",ts:(now|todate),tool:.tool_name,tool_use_id:.tool_use_id}}' >> {}"#,
                        events_file
                    ),
                    "async": true,
                    "timeout": 5
                }]
            }
        ],
        "SubagentStart": [
            {
                "hooks": [{
                    "type": "command",
                    "command": format!(
                        r#"cat | jq -c '{{event:"SubagentStart",ts:(now|todate),agent_id:.agent_id,agent_type:.agent_type}}' >> {}"#,
                        events_file
                    ),
                    "async": true,
                    "timeout": 5
                }]
            }
        ],
        "SubagentStop": [
            {
                "hooks": [{
                    "type": "command",
                    "command": format!(
                        r#"cat | jq -c '{{event:"SubagentStop",ts:(now|todate),agent_id:.agent_id,agent_type:.agent_type}}' >> {}"#,
                        events_file
                    ),
                    "async": true,
                    "timeout": 5
                }]
            }
        ],
        "Stop": [
            {
                "hooks": [{
                    "type": "command",
                    "command": format!(
                        "echo '{{\"event\":\"Stop\",\"ts\":\"'$(date -u +%Y-%m-%dT%H:%M:%SZ)'\"}}' >> {}",
                        events_file
                    ),
                    "async": true,
                    "timeout": 5
                }]
            }
        ]
    });

    // Merge hooks into settings (overwrite hooks key)
    if let Some(obj) = settings.as_object_mut() {
        obj.insert("hooks".to_string(), hooks);
    }

    let json = serde_json::to_string_pretty(&settings)?;
    fs::write(&settings_path, json)?;

    Ok(())
}

fn create_hive_symlink(worktree: &std::path::Path) -> Result<()> {
    let hive_dir = std::env::current_dir()?.join(".hive");
    let symlink_path = worktree.join(".hive");

    if symlink_path.exists() || symlink_path.is_symlink() {
        if symlink_path.is_dir() && !symlink_path.is_symlink() {
            fs::remove_dir_all(&symlink_path)?;
        } else {
            fs::remove_file(&symlink_path)?;
        }
    }

    std::os::unix::fs::symlink(&hive_dir, &symlink_path)
        .context("Failed to create .hive symlink")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_write_hooks_config_creates_settings() {
        let dir = TempDir::new().unwrap();
        write_hooks_config(dir.path(), "test-drone").unwrap();

        let settings_path = dir.path().join(".claude").join("settings.json");
        assert!(settings_path.exists());

        let contents = fs::read_to_string(&settings_path).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&contents).unwrap();

        assert!(settings.get("hooks").is_some());
        let hooks = settings.get("hooks").unwrap();
        assert!(hooks.get("PreToolUse").is_some());
        assert!(hooks.get("PostToolUse").is_some());
        assert!(hooks.get("SubagentStart").is_some());
        assert!(hooks.get("SubagentStop").is_some());
        assert!(hooks.get("Stop").is_some());

        // Verify PreToolUse has 5 matchers: Task, TodoWrite, SendMessage, TaskCreate, TaskUpdate
        let pre_tool = hooks.get("PreToolUse").unwrap().as_array().unwrap();
        assert_eq!(pre_tool.len(), 5);

        // Verify PostToolUse has 1 entry (no matcher — captures all tools)
        let post_tool = hooks.get("PostToolUse").unwrap().as_array().unwrap();
        assert_eq!(post_tool.len(), 1);

        // Verify TodoWrite matcher exists and contains drone name
        let todo_matcher = &pre_tool[1];
        assert_eq!(todo_matcher["matcher"].as_str().unwrap(), "TodoWrite");
        let command = todo_matcher["hooks"][0]
            .get("command")
            .unwrap()
            .as_str()
            .unwrap();
        assert!(command.contains("test-drone"));
        assert!(command.contains("events.ndjson"));
    }

    #[test]
    fn test_write_hooks_config_merges_existing() {
        let dir = TempDir::new().unwrap();
        let claude_dir = dir.path().join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();

        // Write existing settings
        let existing = serde_json::json!({
            "model": "opus",
            "permissions": {"allow": ["Bash"]}
        });
        fs::write(
            claude_dir.join("settings.json"),
            serde_json::to_string_pretty(&existing).unwrap(),
        )
        .unwrap();

        write_hooks_config(dir.path(), "merge-test").unwrap();

        let contents = fs::read_to_string(claude_dir.join("settings.json")).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&contents).unwrap();

        // Original keys preserved
        assert_eq!(settings.get("model").unwrap().as_str().unwrap(), "opus");
        assert!(settings.get("permissions").is_some());

        // Hooks added
        assert!(settings.get("hooks").is_some());
    }

    #[test]
    fn test_write_hooks_config_async_and_timeout() {
        let dir = TempDir::new().unwrap();
        write_hooks_config(dir.path(), "timeout-test").unwrap();

        let contents =
            fs::read_to_string(dir.path().join(".claude").join("settings.json")).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&contents).unwrap();

        // Check PreToolUse hooks have async + timeout
        let pre_tool = settings["hooks"]["PreToolUse"].as_array().unwrap();
        for entry in pre_tool {
            let hooks = entry["hooks"].as_array().unwrap();
            for hook in hooks {
                assert!(hook.get("async").unwrap().as_bool().unwrap());
                assert_eq!(hook.get("timeout").unwrap().as_u64().unwrap(), 5);
            }
        }

        // Check PostToolUse hooks
        let post_tool = settings["hooks"]["PostToolUse"].as_array().unwrap();
        for entry in post_tool {
            let hooks = entry["hooks"].as_array().unwrap();
            for hook in hooks {
                assert!(hook.get("async").unwrap().as_bool().unwrap());
                assert_eq!(hook.get("timeout").unwrap().as_u64().unwrap(), 5);
            }
        }
    }
}
