use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::{Command as ProcessCommand, Stdio};

use super::{ExecutionBackend, SpawnConfig, SpawnHandle};
use crate::agent_teams;

/// Agent Teams execution backend.
/// Launches a Claude Code team lead session that coordinates teammates
/// via Agent Teams native multi-agent collaboration.
pub struct AgentTeamBackend;

impl ExecutionBackend for AgentTeamBackend {
    fn spawn(&self, config: &SpawnConfig) -> Result<SpawnHandle> {
        launch_agent_team(config)
    }

    fn is_running(&self, handle: &SpawnHandle) -> bool {
        if let Some(pid) = handle.pid {
            crate::commands::common::is_process_running(pid as i32)
        } else {
            false
        }
    }

    fn stop(&self, handle: &SpawnHandle) -> Result<()> {
        // Stop the lead process (which manages teammate lifecycle)
        stop_by_worktree_match(&handle.backend_id)
    }

    fn cleanup(&self, handle: &SpawnHandle) -> Result<()> {
        // Clean up Agent Teams directories
        if let Some(team_name) = handle.backend_id.split('/').last() {
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
    let log_path = PathBuf::from(".hive/drones")
        .join(&config.drone_name)
        .join("activity.log");
    let log_file = fs::File::create(&log_path)?;

    let prd: crate::types::Prd = {
        let contents = fs::read_to_string(&config.prd_path)?;
        serde_json::from_str(&contents)
            .context("Failed to parse PRD for team lead prompt")?
    };
    let prd_text = agent_teams::format_prd_for_prompt(&prd);

    // Build story ID list for reference in prompt
    let story_ids: Vec<String> = prd
        .stories
        .iter()
        .map(|s| s.id.clone())
        .collect();

    let status_file = format!(".hive/drones/{}/status.json", config.drone_name);

    let prompt = format!(
        r#"You are the team lead for Hive drone "{drone_name}".

## FORBIDDEN TOOLS — READ THIS FIRST
- NEVER call TeamCreate — the team "{drone_name}" is ALREADY created
- NEVER call TodoWrite — it does NOT create Agent Teams tasks
- NEVER write code yourself — you are a COORDINATOR, not a developer

## YOUR MISSION
Read the PRD below. Your job is to:
1. Understand the project vision and requirements
2. Think about the best way to break down the work into tasks
3. Plan which tasks can run in parallel vs which have dependencies
4. Create tasks, spawn teammates, and coordinate until everything is done

You have full autonomy on HOW to organize the work. The PRD gives you the vision and details — you decide the task breakdown, grouping, and execution order.

## MANDATORY BOOKENDS
Regardless of how you organize the work:
- **FIRST task**: Environment setup — ensure the project compiles, dependencies are installed, the workspace is ready for development
- **LAST task**: Create a Pull Request via `gh pr create`, then verify CI passes. If CI fails, fix and retry.

## PRD — PROJECT VISION & REQUIREMENTS
{prd_text}

## HOW TO WORK

### 1. Plan your tasks
Read the PRD, explore the codebase if needed, then decide how to split the work.
Create tasks using the TaskCreate tool. For each task:
- subject: clear, concise title
- description: detailed requirements for the teammate who will implement it
- activeForm: "Implementing <title>" (shown in TUI spinner)
- metadata: {{"storyId": "<ID>"}} — maps the task to a PRD story for TUI monitoring

PRD story IDs for reference: {story_id_list}

If a task covers multiple stories, pick the primary story ID. Every story ID should appear in at least one task's metadata.

Use TaskUpdate to set blockedBy relationships between tasks.

### 2. Spawn teammates for unblocked tasks

**HARD LIMIT: Maximum {max_agents} concurrent teammates at any time.**
Never exceed this limit. If {max_agents} teammates are running, WAIT for one to finish before spawning another.
Track active teammates carefully. When one finishes, you can spawn a replacement.

For each task ready to start, spawn a teammate:
- Use the Task tool with subagent_type="general-purpose"
- Set mode="bypassPermissions" and team_name="{drone_name}"
- Give a clear name (e.g. "worker-setup", "worker-ui-shell")
- Include the FULL task requirements in the prompt
- Tell them to use TaskUpdate to mark their task completed when done
- Tell them to work in: {worktree_path}

**Model selection — be cost-conscious:**
Use the `model` parameter on each Task tool call to pick the cheapest model that can do the job:
- **"haiku"** — use for: env setup, config changes, file creation, boilerplate, simple refactors, adding dependencies, creating directory structures, running commands, writing tests for existing code
- **"sonnet"** — use for: standard feature implementation, moderate refactoring, writing tests that require understanding complex logic, bug fixes, integrating existing patterns
- **"opus"** — use ONLY for: complex architecture decisions, tasks requiring deep reasoning across many files, tricky debugging with unclear root cause, tasks where a sonnet attempt already failed

**Default to "sonnet"**. Use "haiku" aggressively for anything that doesn't require reasoning. Reserve "opus" for genuinely hard problems — most tasks do NOT need it.

Spawn up to {max_agents} unblocked tasks simultaneously in one message. Queue the rest.

### 3. Monitor and unblock
As teammates finish, check if blocked tasks are now unblocked. Spawn new teammates (respecting the {max_agents} limit).
Repeat until all tasks are done.

### 4. Update status
Maintain {status_file} throughout:
- Set "status": "in_progress" when work begins
- Update "current_story" with any active story ID
- Add completed story IDs to "completed" array
- When ALL done: set "status": "completed""#,
        drone_name = config.drone_name,
        prd_text = prd_text,
        status_file = status_file,
        worktree_path = config.worktree_path.display(),
        story_id_list = story_ids.join(", "),
        max_agents = config.max_agents,
    );

    // Generate a unique agent ID for the team lead
    let agent_id = format!("hive-lead-{}", uuid::Uuid::new_v4());

    let mut cmd = ProcessCommand::new(&config.claude_binary);
    cmd.arg("-p")
        .arg(&prompt)
        .arg("--model")
        .arg(&config.model)
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose")
        .arg("--dangerously-skip-permissions")
        .arg("--team-name")
        .arg(&config.drone_name)
        .arg("--agent-id")
        .arg(&agent_id)
        .arg("--agent-name")
        .arg("team-lead");

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
