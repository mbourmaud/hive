use std::path::Path;

use tokio::sync::mpsc;

use super::super::types::{DetectionEvent, ProjectContext};
use super::detectors::{detect_git, detect_key_files, detect_open_pr, detect_runtimes};

/// Send a `StepStarted` event.
async fn emit_started(tx: &mpsc::Sender<DetectionEvent>, step: &str, label: &str) {
    let _ = tx
        .send(DetectionEvent::StepStarted {
            step: step.to_string(),
            label: label.to_string(),
        })
        .await;
}

/// Send a `StepCompleted` or `StepFailed` event based on the result.
async fn emit_result(
    tx: &mpsc::Sender<DetectionEvent>,
    step: &str,
    label: &str,
    result: Result<serde_json::Value, String>,
) {
    let event = match result {
        Ok(value) => DetectionEvent::StepCompleted {
            step: step.to_string(),
            label: label.to_string(),
            result: value,
        },
        Err(error) => DetectionEvent::StepFailed {
            step: step.to_string(),
            label: label.to_string(),
            error,
        },
    };
    let _ = tx.send(event).await;
}

pub async fn detect_with_events(path: &Path, tx: mpsc::Sender<DetectionEvent>) {
    let path = path.to_path_buf();

    // Git detection
    emit_started(&tx, "git", "Scanning git repository").await;
    let git = detect_git(&path).await;
    let git_result = match &git {
        Some(ctx) => Ok(serde_json::json!({
            "branch": ctx.branch,
            "platform": ctx.platform,
            "dirty_count": ctx.dirty_count,
            "ahead": ctx.ahead,
            "behind": ctx.behind,
        })),
        None => Err("Not a git repository".to_string()),
    };
    emit_result(&tx, "git", "Scanning git repository", git_result).await;

    // Runtimes detection
    emit_started(&tx, "runtimes", "Detecting runtimes & versions").await;
    let runtimes = detect_runtimes(&path).await;
    let runtime_summary: Vec<String> = runtimes
        .iter()
        .map(|r| match r.version {
            Some(ref v) => format!("{} {v}", r.name),
            None => r.name.clone(),
        })
        .collect();
    emit_result(
        &tx,
        "runtimes",
        "Detecting runtimes & versions",
        Ok(serde_json::json!(runtime_summary)),
    )
    .await;

    // Key files detection
    emit_started(&tx, "key_files", "Finding configuration files").await;
    let key_files = detect_key_files(&path).await;
    emit_result(
        &tx,
        "key_files",
        "Finding configuration files",
        Ok(serde_json::json!(key_files)),
    )
    .await;

    // PR detection
    emit_started(&tx, "pr", "Checking for open PR/MR").await;
    let open_pr = if let Some(ref git_ctx) = git {
        detect_open_pr(&path, &git_ctx.platform, &git_ctx.branch).await
    } else {
        None
    };
    let pr_result = match &open_pr {
        Some(pr) => serde_json::json!({
            "number": pr.number,
            "title": pr.title,
            "state": pr.state,
        }),
        None => serde_json::Value::Null,
    };
    emit_result(&tx, "pr", "Checking for open PR/MR", Ok(pr_result)).await;

    // All complete
    let context = ProjectContext {
        git,
        runtimes,
        key_files,
        open_pr,
    };
    let _ = tx.send(DetectionEvent::AllComplete { context }).await;
}
