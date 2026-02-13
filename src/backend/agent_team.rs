use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::{Command as ProcessCommand, Stdio};

use super::{ExecutionBackend, SpawnConfig, SpawnHandle};
use crate::agent_teams;
use crate::types::{StructuredTask, TaskType};

/// Detect the git hosting platform from the remote URL and return the exact
/// PR/MR creation instruction. This avoids the LLM guessing (and using `gh`
/// on GitLab repos, which always fails).
fn detect_pr_instructions(remote_url: &str) -> String {
    let url_lower = remote_url.to_lowercase();
    if url_lower.contains("github.com") {
        "Create a Pull Request: `gh pr create --fill`\n**IMPORTANT: This is a GitHub repo. Use `gh` only, NEVER `glab`.**".to_string()
    } else if url_lower.contains("gitlab") {
        "Create a Merge Request: `glab mr create --fill --yes`\n**IMPORTANT: This is a GitLab repo. Use `glab` only, NEVER `gh`.**".to_string()
    } else if url_lower.contains("bitbucket") {
        "Push the branch. Do NOT attempt to create a PR via CLI (Bitbucket CLI is not available)."
            .to_string()
    } else if remote_url.is_empty() {
        "No git remote detected. Push the branch only, skip PR/MR creation.".to_string()
    } else {
        format!("Push the branch only. The remote `{}` is not a recognized platform — skip PR/MR creation.", remote_url)
    }
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

/// Build verification commands for the detected project languages.
fn build_verification_commands(languages: &[String]) -> String {
    if languages.is_empty() {
        return "- Run any available linting/testing commands you can detect from the project"
            .to_string();
    }

    let mut cmds = Vec::new();
    for lang in languages {
        match lang.as_str() {
            "rust" => {
                cmds.push("- `cargo build` — must compile");
                cmds.push("- `cargo test` — all tests must pass");
                cmds.push("- `cargo clippy -- -D warnings` — no warnings");
            }
            "node" => {
                cmds.push("- `npm test` (or yarn/pnpm equivalent) — all tests must pass");
                cmds.push("- `npm run lint` (if available) — no lint errors");
                cmds.push("- `npm run build` (if available) — must build");
            }
            "go" => {
                cmds.push("- `go build ./...` — must compile");
                cmds.push("- `go test ./...` — all tests must pass");
                cmds.push("- `go vet ./...` — no issues");
            }
            "python" => {
                cmds.push("- `python -m pytest` (if pytest available) — all tests must pass");
                cmds.push("- `ruff check .` or `flake8` (if available) — no lint errors");
                cmds.push("- `mypy .` (if configured) — no type errors");
            }
            _ => {}
        }
    }
    cmds.join("\n")
}

/// Build a task summary table from structured tasks (shared by both prompts).
fn build_task_summary(tasks: &[StructuredTask]) -> String {
    let work_tasks: Vec<&StructuredTask> = tasks
        .iter()
        .filter(|t| t.task_type == TaskType::Work)
        .collect();

    let mut task_summary = String::new();
    for task in &work_tasks {
        task_summary.push_str(&format!("- **{}. {}**", task.number, task.title));
        if let Some(ref model) = task.model {
            task_summary.push_str(&format!(" (model: {})", model));
        }
        if task.parallel {
            task_summary.push_str(" [parallel]");
        }
        if !task.depends_on.is_empty() {
            let deps: Vec<String> = task.depends_on.iter().map(|d| d.to_string()).collect();
            task_summary.push_str(&format!(" [depends_on: {}]", deps.join(", ")));
        }
        if !task.files.is_empty() {
            task_summary.push_str(&format!(" [files: {}]", task.files.join(", ")));
        }
        task_summary.push('\n');
        if !task.body.is_empty() {
            for line in task.body.lines() {
                task_summary.push_str(&format!("  {}\n", line));
            }
        }
    }
    task_summary
}

/// Build the prompt for a structured plan in agent-team mode (5-phase completion loop).
/// The team lead creates tasks via TaskCreate, spawns teammates, and coordinates.
fn build_structured_prompt(config: &SpawnConfig, tasks: &[StructuredTask]) -> String {
    let pr_instructions = detect_pr_instructions(&config.remote_url);
    let worktree_path = config.worktree_path.display();
    let verification_commands = build_verification_commands(&config.project_languages);
    let task_summary = build_task_summary(tasks);

    format!(
        r#"You are the team lead for team "{drone_name}".

You MUST complete ALL 5 phases below. Do NOT stop early. Do NOT skip any phase.

---

## Phase 1: DISPATCH

Read the task list below. For each work task, call TaskCreate to add it to the task list.
Then spawn teammates (max {max_agents} concurrent) and assign tasks via TaskUpdate (set status="in_progress" + owner).

1. Call TaskCreate for each task listed below
2. Spawn teammates and assign tasks via TaskUpdate
3. Respect the `model` field from task metadata when spawning teammates
4. Respect `depends_on` ordering — blocked tasks wait for dependencies
5. Tasks marked `parallel` can run concurrently

### Tasks
{task_summary}

## Phase 2: MONITOR

While tasks are in progress:
1. Check TaskList every 30 seconds to track progress
2. If a teammate is idle for >5 minutes without completing their task, send a status check message
3. If a teammate appears stuck (idle >10 minutes after status check), reassign the task to a new teammate
4. When a teammate reports completion, mark the task completed via TaskUpdate (status="completed")
5. Continue until ALL tasks show status="completed"

## Phase 3: VERIFY (MANDATORY — DO NOT SKIP)

After ALL work tasks are completed, you MUST verify the code works.
Run these commands yourself (you are allowed to run Bash commands for verification):

{verification_commands}

### Fix-and-retry loop (max 3 attempts):
If ANY verification command fails:
1. Analyze the failure output
2. Create a fix task: spawn a teammate to fix the specific issues
3. Wait for the fix to complete
4. Re-run ALL verification commands
5. Repeat up to 3 times total

After 3 failed attempts: proceed to Phase 4 anyway, but note the remaining issues in the PR description.

## Phase 4: PR/MR

1. Stage and commit all changes: `git add -A && git commit -m "<conventional commit message>"`
2. Push the branch: `git push -u origin HEAD`
3. {pr_instructions}

If verification passed: create a clean PR with summary of changes.
If verification failed after 3 retries: create the PR but include a "Known Issues" section listing remaining failures.

## Phase 5: SIGNAL COMPLETION

After the PR/MR is created (or push is done), write the completion marker:
```
Write tool:
file_path: {worktree_path}/.hive_complete
content: HIVE_COMPLETE
```

---

## Rules
- PURE DISPATCHER for Phase 1-2: do NOT write code yourself during dispatch/monitor
- You ARE allowed to run Bash commands in Phase 3 (verification) and Phase 4 (git/PR)
- Do NOT run environment setup — already done
- Do NOT modify any files under .hive/ — those are managed by the orchestrator
- NEVER skip Phase 3 (verification) — this is the most important phase
- NEVER write .hive_complete before Phase 4 is done"#,
        drone_name = config.drone_name,
        task_summary = task_summary.trim(),
        max_agents = config.max_agents,
        verification_commands = verification_commands,
        pr_instructions = pr_instructions,
        worktree_path = worktree_path,
    )
}

/// Build the prompt for solo agent mode (no teammates, direct execution).
fn build_solo_prompt(config: &SpawnConfig, tasks: &[StructuredTask]) -> String {
    let pr_instructions = detect_pr_instructions(&config.remote_url);
    let worktree_path = config.worktree_path.display();
    let verification_commands = build_verification_commands(&config.project_languages);
    let task_summary = build_task_summary(tasks);

    // Include the full plan content for context
    let plan_content = std::fs::read_to_string(&config.prd_path).unwrap_or_default();

    format!(
        r#"You are drone "{drone_name}". Complete the following tasks in order.
Work solo — do NOT spawn teammates. Do NOT use TeamCreate or the Task tool for spawning agents.

You MUST complete ALL 4 phases below. Do NOT stop early. Do NOT skip any phase.

---

## Plan

{plan_content}

## Phase 1: EXECUTE

Complete each task below in order. Work through them sequentially.

### Tasks
{task_summary}

## Phase 2: VERIFY (MANDATORY — DO NOT SKIP)

After ALL tasks are completed, you MUST verify the code works.
Run these commands:

{verification_commands}

### Fix-and-retry loop (max 3 attempts):
If ANY verification command fails:
1. Analyze the failure output
2. Fix the specific issues yourself
3. Re-run ALL verification commands
4. Repeat up to 3 times total

After 3 failed attempts: proceed to Phase 3 anyway, but note the remaining issues in the PR description.

## Phase 3: PR/MR

1. Stage and commit all changes: `git add -A && git commit -m "<conventional commit message>"`
2. Push the branch: `git push -u origin HEAD`
3. {pr_instructions}

If verification passed: create a clean PR with summary of changes.
If verification failed after 3 retries: create the PR but include a "Known Issues" section listing remaining failures.

## Phase 4: SIGNAL COMPLETION

After the PR/MR is created (or push is done), write the completion marker:
```
Write tool:
file_path: {worktree_path}/.hive_complete
content: HIVE_COMPLETE
```

---

## Rules
- Work SOLO — do NOT spawn teammates or create teams
- Do NOT run environment setup — already done
- Do NOT modify any files under .hive/ — those are managed by the orchestrator
- NEVER skip Phase 2 (verification) — this is the most important phase
- NEVER write .hive_complete before Phase 3 is done"#,
        drone_name = config.drone_name,
        plan_content = plan_content.trim(),
        task_summary = task_summary.trim(),
        verification_commands = verification_commands,
        pr_instructions = pr_instructions,
        worktree_path = worktree_path,
    )
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
