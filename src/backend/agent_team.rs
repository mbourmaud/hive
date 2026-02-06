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
        crate::backend::native::stop_by_worktree_match(&handle.backend_id)
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

    let prd_content = fs::read_to_string(&config.prd_path)?;

    let status_file = format!(".hive/drones/{}/status.json", config.drone_name);

    let prompt = format!(
        r#"You are the team lead for Hive drone "{drone_name}". You coordinate work using Agent Teams.

## YOUR ROLE: COORDINATE ONLY
You are a COORDINATOR. You MUST NOT write any code yourself.
Your ONLY job is to spawn teammates and monitor their progress.

## PRE-SEEDED TASKS
Tasks have ALREADY been created in the task list. Run TaskList NOW to see them.
DO NOT create new tasks with TaskCreate - they already exist.

Tasks use numeric IDs (1, 2, 3...). Each task has a "storyId" in its metadata that maps to the PRD story.
Tasks with blockedBy arrays must wait for those blocking tasks to complete first.

## STEP-BY-STEP INSTRUCTIONS

1. Run TaskList to see all tasks and their dependencies
2. Identify tasks with empty blockedBy (these can start immediately)
3. For EACH unblocked task, spawn a teammate:
   - Use the Task tool with subagent_type="general-purpose"
   - Set mode="bypassPermissions"
   - Set team_name="{drone_name}"
   - Give each teammate a clear name related to their task
   - In the prompt, include the FULL task description from the task
   - Tell each teammate to mark the task as completed via TaskUpdate when done
4. As tasks complete, check if blocked tasks are now unblocked (their blockedBy tasks are all completed)
5. Spawn new teammates for newly unblocked tasks
6. When ALL tasks are completed, update the status file and wrap up

## SPAWNING TEAMMATES - EXAMPLE
```
Task tool call:
  subagent_type: "general-purpose"
  name: "worker-1"
  mode: "bypassPermissions"
  team_name: "{drone_name}"
  prompt: "You are implementing [task subject]. [full task description]. When done, use TaskUpdate to mark task [id] as completed."
```

## PARALLELIZATION
- Spawn ALL unblocked tasks simultaneously (multiple Task tool calls in one message)
- This is critical - the whole point is PARALLEL execution
- A task is unblocked when its blockedBy list is empty or all blockedBy tasks are completed

## STATUS TRACKING
After spawning teammates and as they complete work, update {status_file}:
- Set "status": "in_progress"
- Set "current_story" to any active story ID
- As stories complete: add to "completed" array, update "story_times"
- When ALL done: set "status": "completed"

## PRD CONTENT
{prd_content}

## IMPORTANT RULES
- NEVER write code yourself
- ALWAYS spawn teammates via the Task tool
- Spawn multiple teammates in PARALLEL (multiple tool calls in one message)
- Use TaskList and TaskUpdate for coordination
- Each teammate should work in: {worktree_path}"#,
        drone_name = config.drone_name,
        prd_content = prd_content,
        status_file = status_file,
        worktree_path = config.worktree_path.display(),
    );

    // Generate a unique agent ID for the team lead
    let agent_id = format!("hive-lead-{}", uuid::Uuid::new_v4());

    let mut cmd = ProcessCommand::new("claude");
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
