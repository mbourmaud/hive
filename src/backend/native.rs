use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::{Command as ProcessCommand, Stdio};

use super::{ExecutionBackend, SpawnConfig, SpawnHandle};

/// Native execution backend using direct Claude CLI process spawning.
/// This is the original Hive execution method: spawn `claude` as a subprocess.
pub struct NativeBackend;

impl ExecutionBackend for NativeBackend {
    fn spawn(&self, config: &SpawnConfig) -> Result<SpawnHandle> {
        launch_claude_worktree(config)
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
