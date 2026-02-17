use crate::backend::SpawnConfig;
use crate::types::StructuredTask;

use super::worker_notes::{self, WorkerNote};

// Re-export shared helpers from agent_team prompts
pub use crate::backend::agent_team::prompts::{
    build_verification_commands, detect_pr_instructions,
};

/// Build the system prompt for a worker agent executing a single task.
pub fn build_worker_prompt(
    task: &StructuredTask,
    config: &SpawnConfig,
    ownership_hint: &str,
    dependency_notes: &[WorkerNote],
) -> String {
    let plan_content = std::fs::read_to_string(&config.prd_path).unwrap_or_default();
    let notes_section = worker_notes::format_notes_for_prompt(dependency_notes);

    let worker_name = task.worker_name();
    format!(
        r#"You are worker "{worker_name}" on team "{team_name}".

## Your Task

**{number}. {title}**

{body}

## Plan Context

{plan_content}

## File Ownership

{ownership_hint}
{notes_section}
## Rules

- Focus ONLY on your assigned task — do not work on other tasks
- Do NOT create git commits — the coordinator handles git operations
- Do NOT create PRs or push — that happens after all tasks complete
- Do NOT modify files under .hive/ — managed by the orchestrator
- When your task is complete, stop using tools and respond with a summary
- Include "TASK_COMPLETE" at the end of your final message when done
- If you cannot complete the task, include "TASK_BLOCKED: <reason>" instead"#,
        worker_name = worker_name,
        team_name = config.team_name,
        number = task.number,
        title = task.title,
        body = task.body,
        plan_content = plan_content.trim(),
        ownership_hint = ownership_hint,
        notes_section = notes_section,
    )
}

/// Build a continuation prompt when a worker needs to resume with fresh context.
pub fn build_continuation_prompt(task: &StructuredTask, progress: &str) -> String {
    format!(
        r#"Continue working on your task:

**{number}. {title}**

{body}

## Progress So Far

{progress}

## Instructions

- Pick up where you left off based on the progress summary above
- Do NOT redo work that's already been completed
- When done, include "TASK_COMPLETE" in your final message
- If blocked, include "TASK_BLOCKED: <reason>""#,
        number = task.number,
        title = task.title,
        body = task.body,
        progress = progress,
    )
}

/// Build the system prompt for the verification phase.
pub fn build_verifier_prompt(config: &SpawnConfig) -> String {
    let verification_commands = build_verification_commands(&config.project_languages);

    format!(
        r#"You are the verification agent for team "{team_name}".

## Your Task

Run ALL verification commands below and report the results.
Fix any issues you find. This is critical — the code must pass all checks.

## Verification Commands

{verification_commands}

## Rules

- Run EVERY command listed above
- If a command fails, attempt to fix the issue
- After fixing, re-run the failed command to confirm the fix
- Report a summary: which commands passed, which failed, what you fixed
- Include "VERIFY_PASS" if all checks pass
- Include "VERIFY_FAIL" followed by failure details if checks fail"#,
        team_name = config.team_name,
        verification_commands = verification_commands,
    )
}

/// Build the prompt for a fix-and-retry iteration after verification failure.
pub fn build_fix_prompt(failures: &str, config: &SpawnConfig) -> String {
    let verification_commands = build_verification_commands(&config.project_languages);

    format!(
        r#"You are the fix agent for team "{team_name}".

## Verification Failures

The following verification checks failed:

{failures}

## Your Task

1. Analyze each failure
2. Fix the root cause in the source code
3. Re-run ALL verification commands to confirm:

{verification_commands}

## Rules

- Fix the actual bugs, don't just suppress warnings or skip tests
- After fixing, re-run ALL verification commands (not just the failing ones)
- Include "VERIFY_PASS" if all checks now pass
- Include "VERIFY_FAIL" followed by remaining failures if not"#,
        team_name = config.team_name,
        failures = failures,
        verification_commands = verification_commands,
    )
}

/// Build the prompt for the PR/MR phase.
pub fn build_pr_prompt(config: &SpawnConfig, verification_passed: bool) -> String {
    let pr_instructions = detect_pr_instructions(&config.remote_url);
    let worktree_path = config.worktree_path.display();

    let status_note = if verification_passed {
        "All verification checks passed.".to_string()
    } else {
        "Some verification checks failed. Include a \"Known Issues\" section in the PR.".to_string()
    };

    format!(
        r#"You are the PR agent for team "{team_name}".

## Your Task

1. Stage and commit all changes:
   `git add -A && git commit -m "<conventional commit message summarizing all work>"`
2. Push the branch:
   `git push -u origin HEAD`
3. Create PR/MR:
   {pr_instructions}

## Verification Status

{status_note}

## Completion

After the PR/MR is created (or push is done), write the completion marker:
```
Write tool:
file_path: {worktree_path}/.hive_complete
content: HIVE_COMPLETE
```

Include "PR_COMPLETE" in your final message."#,
        team_name = config.team_name,
        pr_instructions = pr_instructions,
        worktree_path = worktree_path,
        status_note = status_note,
    )
}
