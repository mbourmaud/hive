use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::{Command as ProcessCommand, Stdio};

use crate::backend::{SpawnConfig, SpawnHandle};

use super::prompts::{build_solo_prompt, build_structured_prompt};

pub fn launch_agent_team(config: &SpawnConfig) -> Result<SpawnHandle> {
    let drone_dir = PathBuf::from(".hive/drones").join(&config.drone_name);
    let log_path = drone_dir.join("activity.log");
    let log_file = fs::File::create(&log_path)?;

    // Touch events.ndjson so the TUI can start tailing immediately
    let events_path = drone_dir.join("events.ndjson");
    if !events_path.exists() {
        let _ = fs::File::create(&events_path);
    }

    let is_solo = config.mode == "agent";

    let prompt = if is_solo {
        build_solo_prompt(config, &config.structured_tasks)
    } else {
        build_structured_prompt(config, &config.structured_tasks)
    };

    // Solo mode uses the user's chosen model; agent-team mode forces Opus for the
    // team lead (Sonnet struggles with Agent Teams coordination).
    let model = if is_solo {
        config.model.as_str()
    } else {
        "opus"
    };

    let mut cmd = ProcessCommand::new(&config.claude_binary);
    cmd.arg("-p")
        .arg(&prompt)
        .arg("--model")
        .arg(model)
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose")
        .arg("--dangerously-skip-permissions");

    // Clear CLAUDECODE env var to avoid "nested session" detection
    cmd.env_remove("CLAUDECODE");

    // Only enable Agent Teams for multi-agent mode
    if !is_solo {
        cmd.env("CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS", "1");
    }

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
        .context("Failed to spawn Claude process")?;

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
