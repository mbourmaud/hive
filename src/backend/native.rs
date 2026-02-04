use anyhow::{bail, Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::{Command as ProcessCommand, Stdio};

use super::{ExecutionBackend, SpawnConfig, SpawnHandle};
use crate::types::ExecutionMode;

/// Native execution backend using direct Claude CLI process spawning.
/// This is the original Hive execution method: spawn `claude` as a subprocess.
pub struct NativeBackend;

impl ExecutionBackend for NativeBackend {
    fn spawn(&self, config: &SpawnConfig) -> Result<SpawnHandle> {
        match (&config.execution_mode, config.wait) {
            (ExecutionMode::Worktree, _) => launch_claude_worktree(config),
            (ExecutionMode::Subagent, true) => launch_claude_subagent_sync(config),
            (ExecutionMode::Subagent, false) => launch_claude_subagent(config),
            // Swarm mode falls back to worktree for now
            (ExecutionMode::Swarm, _) => launch_claude_worktree(config),
        }
    }

    fn is_running(&self, handle: &SpawnHandle) -> bool {
        if let Some(pid) = handle.pid {
            crate::commands::common::is_process_running(pid as i32)
        } else {
            false
        }
    }

    fn stop(&self, handle: &SpawnHandle) -> Result<()> {
        stop_by_worktree_match(&handle.backend_id)
    }

    fn cleanup(&self, _handle: &SpawnHandle) -> Result<()> {
        // Cleanup is handled by kill_clean.rs at a higher level
        Ok(())
    }

    fn name(&self) -> &str {
        "native"
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

/// Launch Claude in worktree mode (traditional isolated mode).
fn launch_claude_worktree(config: &SpawnConfig) -> Result<SpawnHandle> {
    let log_path = PathBuf::from(".hive/drones")
        .join(&config.drone_name)
        .join("activity.log");
    let log_file = fs::File::create(&log_path)?;

    let prd_content = fs::read_to_string(&config.prd_path)?;
    let status_file = format!(".hive/drones/{}/status.json", config.drone_name);

    let prompt = format!(
        r#"You are a Hive drone working on this PRD. Execute each story in order.

PRD Content:
{}

## CRITICAL: Status Updates

You MUST update the status file at `{}` to track your progress. This is how the monitoring TUI knows what you're doing.

### Before starting EACH story:
Read the current status.json, then update it with:
- `"status": "in_progress"`
- `"current_story": "<story-id>"` (e.g., "US-0", "MON-001")
- `"story_times".<story-id>.started`: current ISO timestamp
- `"updated"`: current ISO timestamp

Example command to start a story:
```bash
# Read current status
cat .hive/drones/{}/status.json | jq '
  .status = "in_progress" |
  .current_story = "US-0" |
  .story_times["US-0"] = {{"started": (now | todate)}} |
  .updated = (now | todate)
' > /tmp/status.json && mv /tmp/status.json .hive/drones/{}/status.json
```

### After completing EACH story:
Update status.json with:
- Add story ID to `"completed"` array
- `"story_times".<story-id>.completed`: current ISO timestamp
- `"updated"`: current ISO timestamp

Example command to complete a story:
```bash
cat .hive/drones/{}/status.json | jq '
  .completed += ["US-0"] |
  .story_times["US-0"].completed = (now | todate) |
  .updated = (now | todate)
' > /tmp/status.json && mv /tmp/status.json .hive/drones/{}/status.json
```

### When ALL stories are done:
Set `"status": "completed"` and `"current_story": null`

### If you encounter an error:
Set `"status": "error"` and increment `"error_count"`

## Inter-drone Messages
Check `.hive/drones/{}/inbox/` before each story for messages from other drones.
If a message of type "DependencyRequest" exists, prioritize that work.
After discovering important context, write a JSON message to `.hive/drones/{}/outbox/`.

## Execution Instructions

**IMPORTANT: Before starting work, read `{}` to check which stories are already completed.**
- The `completed` array contains IDs of stories that are DONE - DO NOT redo them
- Start with the FIRST story that is NOT in the `completed` array
- If `current_story` is set and not in `completed`, resume that story first

Work through stories sequentially. After completing each story, move to the next uncompleted one automatically. Always update status.json BEFORE starting and AFTER completing each story."#,
        prd_content,
        status_file,
        config.drone_name,
        config.drone_name,
        config.drone_name,
        config.drone_name,
        config.drone_name,
        config.drone_name,
        status_file,
    );

    let child = ProcessCommand::new("claude")
        .arg("-p")
        .arg(&prompt)
        .arg("--model")
        .arg(&config.model)
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose")
        .arg("--dangerously-skip-permissions")
        .current_dir(&config.worktree_path)
        .stdin(Stdio::null())
        .stdout(log_file.try_clone()?)
        .stderr(log_file)
        .spawn()
        .context("Failed to spawn claude process")?;

    Ok(SpawnHandle {
        pid: Some(child.id()),
        backend_id: config.worktree_path.to_string_lossy().to_string(),
        backend_type: "native".to_string(),
    })
}

/// Launch Claude in subagent mode (async, no worktree).
fn launch_claude_subagent(config: &SpawnConfig) -> Result<SpawnHandle> {
    let log_path = PathBuf::from(".hive/drones")
        .join(&config.drone_name)
        .join("activity.log");
    let log_file = fs::File::create(&log_path)?;

    let prd_content = fs::read_to_string(&config.prd_path)?;
    let status_file = format!(".hive/drones/{}/status.json", config.drone_name);

    let prompt = format!(
        r#"You are a Hive subagent working on this PRD. Execute each story in order.

PRD Content:
{}

## Execution Mode: SUBAGENT

You are running in **subagent mode** - working directly in the current repository without a separate worktree.
This means:
- You work on the CURRENT branch (create a new branch if needed for the PRD)
- Use TodoWrite to track your progress through stories
- Commit changes incrementally as you complete each story

## Status Tracking

Update `{}` to track progress:

1. **Before each story**: Set `current_story` and `status: "in_progress"`
2. **After each story**: Add story ID to `completed` array
3. **When done**: Set `status: "completed"`

Use this pattern to update status:
```bash
cat {} | jq '.status = "in_progress" | .current_story = "STORY-ID" | .updated = (now | todate)' > /tmp/s.json && mv /tmp/s.json {}
```

## Inter-drone Messages
Check `.hive/drones/{}/inbox/` before each story for messages from other drones.
After discovering important context, write a JSON message to `.hive/drones/{}/outbox/`.

## Execution Instructions

1. Read `{}` to check which stories are already completed
2. Start with the FIRST story NOT in `completed` array
3. Use TodoWrite to break down each story into tasks
4. Execute each story fully before moving to the next
5. Commit your changes after each story with: `git commit -m "feat(<scope>): <story-title>"`
6. Update status.json after completing each story

## Git Workflow

Since you're in subagent mode on the main repo:
- Check current branch: `git branch --show-current`
- If not on target branch, create it: `git checkout -b <target-branch>`
- Commit changes incrementally
- DO NOT push without explicit instruction

Work through all stories. After completing each, update status and continue to the next."#,
        prd_content,
        status_file,
        status_file,
        status_file,
        config.drone_name,
        config.drone_name,
        status_file,
    );

    let child = ProcessCommand::new("claude")
        .arg("-p")
        .arg(&prompt)
        .arg("--model")
        .arg(&config.model)
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose")
        .arg("--dangerously-skip-permissions")
        .current_dir(&config.working_dir)
        .stdin(Stdio::null())
        .stdout(log_file.try_clone()?)
        .stderr(log_file)
        .spawn()
        .context("Failed to spawn claude subagent process")?;

    Ok(SpawnHandle {
        pid: Some(child.id()),
        backend_id: config.working_dir.to_string_lossy().to_string(),
        backend_type: "native".to_string(),
    })
}

/// Launch Claude in subagent mode synchronously (blocks until completion).
fn launch_claude_subagent_sync(config: &SpawnConfig) -> Result<SpawnHandle> {
    let log_path = PathBuf::from(".hive/drones")
        .join(&config.drone_name)
        .join("activity.log");
    let log_file = fs::File::create(&log_path)?;

    let prd_content = fs::read_to_string(&config.prd_path)?;
    let status_file = format!(".hive/drones/{}/status.json", config.drone_name);

    let prompt = format!(
        r#"You are a Hive subagent working on this PRD. Execute each story in order.

PRD Content:
{}

## Execution Mode: SUBAGENT (Synchronous)

You are running in **subagent mode** - working directly in the current repository.
- Work on the CURRENT branch (create a new branch if needed)
- Use TodoWrite to track progress
- Commit changes incrementally

## Status Tracking

Update `{}` to track progress:
1. **Before each story**: Set `current_story` and `status: "in_progress"`
2. **After each story**: Add story ID to `completed` array
3. **When done**: Set `status: "completed"`

## Inter-drone Messages
Check `.hive/drones/{}/inbox/` before each story for messages from other drones.
After discovering important context, write a JSON message to `.hive/drones/{}/outbox/`.

## Execution Instructions

1. Read `{}` to check completed stories
2. Start with FIRST story NOT in `completed` array
3. Execute each story fully before moving to next
4. Commit changes after each story
5. Update status.json after each story

Work through all stories sequentially."#,
        prd_content, status_file, config.drone_name, config.drone_name, status_file,
    );

    let status = ProcessCommand::new("claude")
        .arg("-p")
        .arg(&prompt)
        .arg("--model")
        .arg(&config.model)
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose")
        .arg("--dangerously-skip-permissions")
        .current_dir(&config.working_dir)
        .stdin(Stdio::null())
        .stdout(log_file.try_clone()?)
        .stderr(log_file)
        .status()
        .context("Failed to run claude subagent process")?;

    if !status.success() {
        bail!("Claude subagent exited with error: {:?}", status.code());
    }

    // Update status to completed
    let status_path = PathBuf::from(".hive/drones")
        .join(&config.drone_name)
        .join("status.json");
    if let Ok(contents) = fs::read_to_string(&status_path) {
        if let Ok(mut status_obj) = serde_json::from_str::<serde_json::Value>(&contents) {
            status_obj["status"] = serde_json::json!("completed");
            status_obj["updated"] = serde_json::json!(chrono::Utc::now().to_rfc3339());
            if let Ok(updated) = serde_json::to_string_pretty(&status_obj) {
                let _ = fs::write(&status_path, updated);
            }
        }
    }

    Ok(SpawnHandle {
        pid: None,
        backend_id: config.working_dir.to_string_lossy().to_string(),
        backend_type: "native".to_string(),
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
