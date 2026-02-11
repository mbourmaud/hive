use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, Stdio};

use super::{ExecutionBackend, SpawnConfig, SpawnHandle};
use crate::agent_teams;

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

/// Agent Teams execution backend.
/// Launches a Claude Code team lead session that coordinates teammates
/// via Agent Teams native multi-agent collaboration.
pub struct AgentTeamBackend;

impl ExecutionBackend for AgentTeamBackend {
    fn spawn(&self, config: &SpawnConfig) -> Result<SpawnHandle> {
        launch_agent_team(config)
    }

    fn is_running(&self, handle: &SpawnHandle) -> bool {
        handle
            .pid
            .map(|pid| crate::commands::common::is_process_running(pid as i32))
            .unwrap_or(false)
    }

    fn stop(&self, handle: &SpawnHandle) -> Result<()> {
        // Stop the lead process (which manages teammate lifecycle)
        stop_by_worktree_match(&handle.backend_id)
    }

    fn cleanup(&self, handle: &SpawnHandle) -> Result<()> {
        // Clean up Agent Teams directories
        if let Some(team_name) = handle.backend_id.split('/').next_back() {
            let _ = agent_teams::cleanup_team(team_name);
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "agent_team"
    }

    fn is_available(&self) -> bool {
        // Check if `claude` CLI is available
        ProcessCommand::new("claude")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

fn launch_agent_team(config: &SpawnConfig) -> Result<SpawnHandle> {
    let drone_dir = PathBuf::from(".hive/drones").join(&config.drone_name);
    let log_path = drone_dir.join("activity.log");
    let log_file = fs::File::create(&log_path)?;

    // Touch events.ndjson so the TUI can start tailing immediately
    let events_path = drone_dir.join("events.ndjson");
    if !events_path.exists() {
        let _ = fs::File::create(&events_path);
    }

    // Read plan content — markdown is passed directly as the project requirements
    let plan_content = fs::read_to_string(&config.prd_path).context("Failed to read plan file")?;

    // Pass the git remote URL so the lead knows which platform it's on
    let remote_url = get_git_remote_url(&config.worktree_path);

    let prompt = format!(
        r#"You are coordinating work on this project.

## Project Requirements
{plan_content}

## Working directory
{worktree_path}

## Git remote
{remote_url}

## Instructions
- Create an agent team named "{drone_name}" to implement this plan
- Use delegate mode — coordinate only, do not write code yourself
- Before delegating work, create tasks in the task list (using TaskCreate) to break down the plan into concrete, trackable work items.
- Use sonnet for teammates by default, haiku for simple tasks
- Maximum {max_agents} concurrent teammates
- Do NOT modify any files under .hive/ — those are managed by the orchestrator

## Environment Setup (FIRST STEP)
Before assigning any tasks, verify the project builds:
1. Detect project type (package.json, Cargo.toml, go.mod, etc.)
2. Install dependencies if needed (npm install, pnpm install, cargo build, etc.)
3. Run code generation if applicable (prisma generate, protoc, etc.)
4. Verify the project compiles/type-checks — fix any issues before delegating work

## PR/MR Creation (LAST STEP)
After all tasks are completed:
1. Run linting/formatting (cargo fmt, prettier, etc.)
2. Run tests relevant to changed files
3. Commit all changes with a conventional commit message
4. Push the branch
5. Create a PR/MR with a description summarizing the changes. **Detect the platform from the git remote URL above** and use the correct CLI:
   - GitHub (github.com) → `gh pr create`
   - GitLab (gitlab) → `glab mr create`
   - Bitbucket (bitbucket) → manual or `bb pr create`
   - Other/self-hosted → commit and push only, skip PR creation
   **CRITICAL: Using the wrong CLI (e.g. `gh` on GitLab) will fail. Check the remote URL first.**

## CRITICAL: Task Progress Tracking
You MUST keep task status up to date — this drives the progress dashboard.
- When you assign a task to a teammate: call TaskUpdate with status "in_progress" and set the owner
- When a teammate reports completion: call TaskUpdate with status "completed" IMMEDIATELY, before doing anything else
- NEVER skip TaskUpdate calls — the monitoring dashboard relies on accurate task status
- After marking a task completed, check TaskList for the next pending task to assign

## Completion Signal
CRITICAL: Once ALL tasks are completed AND the PR is successfully created, you MUST signal completion to the orchestrator:
1. Use the Write tool to create a file named '.hive_complete' in the working directory
2. The file content should be the single word: HIVE_COMPLETE
3. This signals the orchestrator that the drone can be safely stopped

Example:
```
Write tool:
file_path: {worktree_path}/.hive_complete
content: HIVE_COMPLETE
```"#,
        plan_content = plan_content,
        worktree_path = config.worktree_path.display(),
        remote_url = remote_url,
        drone_name = config.drone_name,
        max_agents = config.max_agents,
    );

    // Force Opus for team lead — Sonnet struggles with Agent Teams coordination
    // (skips TaskCreate/TaskUpdate, poor task delegation). Falls back to user's
    // chosen model only if opus is explicitly unavailable.
    let team_lead_model = "opus";

    let mut cmd = ProcessCommand::new(&config.claude_binary);
    cmd.arg("-p")
        .arg(&prompt)
        .arg("--model")
        .arg(team_lead_model)
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose")
        .arg("--dangerously-skip-permissions");

    // Enable Agent Teams experimental flag
    cmd.env("CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS", "1");

    // Apply profile environment variables
    if let Some(ref env_vars) = config.environment {
        for (key, value) in env_vars {
            cmd.env(key, value);
        }
    }

    let child = cmd
        .current_dir(&config.worktree_path)
        .stdin(Stdio::null())
        .stdout(log_file.try_clone()?)
        .stderr(log_file)
        .spawn()
        .context("Failed to spawn Claude Agent Teams lead process")?;

    Ok(SpawnHandle {
        pid: Some(child.id()),
        backend_id: config.worktree_path.to_string_lossy().to_string(),
        backend_type: "agent_team".to_string(),
    })
}

/// Stop a drone by matching its worktree path in `ps aux` output.
/// Sends SIGTERM, waits 2 seconds, then SIGKILL if still running.
pub fn stop_by_worktree_match(worktree_path: &str) -> Result<()> {
    let ps_output = ProcessCommand::new("ps")
        .args(["aux"])
        .output()
        .context("Failed to run ps command")?;

    let ps_str = String::from_utf8_lossy(&ps_output.stdout);

    let mut pids = Vec::new();
    for line in ps_str.lines() {
        if line.contains("claude") && line.contains(worktree_path) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 1 {
                if let Ok(pid) = parts[1].parse::<i32>() {
                    pids.push(pid);
                }
            }
        }
    }

    for pid in &pids {
        let _ = ProcessCommand::new("kill")
            .args(["-TERM", &pid.to_string()])
            .output();

        std::thread::sleep(std::time::Duration::from_secs(2));

        let still_running = ProcessCommand::new("ps")
            .args(["-p", &pid.to_string()])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if still_running {
            let _ = ProcessCommand::new("kill")
                .args(["-KILL", &pid.to_string()])
                .output();
        }
    }

    Ok(())
}
