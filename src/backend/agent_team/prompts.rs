use crate::backend::SpawnConfig;
use crate::types::{StructuredTask, TaskType};

/// Detect the git hosting platform from the remote URL and return the exact
/// PR/MR creation instruction.
pub fn detect_pr_instructions(remote_url: &str) -> String {
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

/// Build verification commands for the detected project languages.
pub fn build_verification_commands(languages: &[String]) -> String {
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
pub fn build_task_summary(tasks: &[StructuredTask]) -> String {
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
pub fn build_structured_prompt(config: &SpawnConfig, tasks: &[StructuredTask]) -> String {
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
pub fn build_solo_prompt(config: &SpawnConfig, tasks: &[StructuredTask]) -> String {
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
