use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

use crate::types::StructuredTask;
use crate::webui::anthropic::model::resolve_model;
use crate::webui::anthropic::types::{ContentBlock, Message, MessageContent};
use crate::webui::auth::credentials::Credentials;
use crate::webui::chat::handlers::agentic::{run_agentic_loop, AgenticLoopParams};
use crate::webui::chat::session::{Effort, SessionStore};
use crate::webui::tools::definitions::builtin_tool_definitions;

use super::events::EventEmitter;
use super::file_ownership::ownership_prompt_for_files;
use super::prompts::{build_continuation_prompt, build_worker_prompt};

const MAX_ITERATIONS: usize = 10;
const TASK_COMPLETE_SIGNAL: &str = "TASK_COMPLETE";
const TASK_BLOCKED_SIGNAL: &str = "TASK_BLOCKED";

/// Handle returned when a worker is spawned.
pub struct WorkerHandle {
    pub task_number: usize,
    pub worker_name: String,
    pub join_handle: JoinHandle<Result<WorkerResult>>,
    pub abort_flag: Arc<AtomicBool>,
}

/// Result of a worker completing its task.
pub struct WorkerResult {
    pub task_number: usize,
    pub success: bool,
    pub error: Option<String>,
}

/// Configuration for spawning a worker.
pub struct WorkerConfig {
    pub task: StructuredTask,
    pub creds: Credentials,
    pub model: String,
    pub cwd: std::path::PathBuf,
    pub team_name: String,
    pub prd_path: std::path::PathBuf,
    pub emitter: Arc<EventEmitter>,
    pub session_store: SessionStore,
    pub global_abort: Arc<AtomicBool>,
}

/// Spawn a worker agent for a single task.
///
/// Returns a `WorkerHandle` with a `JoinHandle` that resolves when the
/// worker completes (or fails) its task.
pub fn spawn_worker(config: WorkerConfig) -> WorkerHandle {
    let task_number = config.task.number;
    let worker_name = format!("worker-{}", task_number);
    let abort_flag = Arc::new(AtomicBool::new(false));
    let abort_clone = abort_flag.clone();

    let join_handle = tokio::spawn(async move { run_worker(config, abort_clone).await });

    WorkerHandle {
        task_number,
        worker_name,
        join_handle,
        abort_flag,
    }
}

/// The worker's main run loop (Ralph pattern).
///
/// Runs the agentic loop, and if the task isn't complete after a full
/// conversation, resets context with a progress summary and continues.
async fn run_worker(config: WorkerConfig, abort_flag: Arc<AtomicBool>) -> Result<WorkerResult> {
    let task_number = config.task.number;
    let model_id = resolve_model(&config.model).to_string();
    let worker_name = format!("worker-{task_number}");

    // Build initial system prompt with file ownership constraints
    let ownership_hint = ownership_prompt_for_files(&config.task.files);
    let system_prompt =
        build_worker_prompt(&config.task, &spawn_config_ref(&config), &ownership_hint);
    let tools = builtin_tool_definitions();

    // Create a broadcast channel for this worker (events are used for tool tracking)
    let (tx, _rx) = broadcast::channel::<String>(256);

    let mut continuation_context: Option<String> = None;

    for iteration in 0..MAX_ITERATIONS {
        if abort_flag.load(Ordering::Relaxed) || config.global_abort.load(Ordering::Relaxed) {
            return Ok(WorkerResult {
                task_number,
                success: false,
                error: Some("Aborted".to_string()),
            });
        }

        // Build messages: either initial user message or continuation
        let messages = build_messages(iteration, &config.task, continuation_context.as_deref());

        let params = AgenticLoopParams {
            creds: &config.creds,
            model: &model_id,
            messages,
            system_prompt: Some(system_prompt.clone()),
            tools: Some(tools.clone()),
            cwd: &config.cwd,
            tx: &tx,
            session_id: &worker_name,
            abort_flag: &abort_flag,
            store: config.session_store.clone(),
            effort: Effort::High,
            max_turns: Some(25),
            mcp_pool: None,
        };

        let result_messages = run_agentic_loop(params).await?;

        // Emit ToolDone events for TUI tool history
        emit_tool_events(&config.emitter, &result_messages);

        // Check for completion signals in assistant messages
        let (complete, blocked_reason) = check_completion(&result_messages);

        if complete {
            return Ok(WorkerResult {
                task_number,
                success: true,
                error: None,
            });
        }

        if let Some(reason) = blocked_reason {
            return Ok(WorkerResult {
                task_number,
                success: false,
                error: Some(reason),
            });
        }

        // Not done — extract progress and continue with fresh context
        continuation_context = Some(extract_progress_summary(&result_messages));
    }

    // Exhausted max iterations — treat as success (best effort)
    Ok(WorkerResult {
        task_number,
        success: true,
        error: None,
    })
}

/// Build the user messages for an agentic loop iteration.
fn build_messages(
    iteration: usize,
    task: &StructuredTask,
    continuation: Option<&str>,
) -> Vec<Message> {
    let content = if iteration == 0 {
        format!(
            "Complete this task:\n\n**{}. {}**\n\n{}",
            task.number, task.title, task.body
        )
    } else {
        let progress = continuation.unwrap_or("No progress summary available.");
        build_continuation_prompt(task, progress)
    };

    vec![Message {
        role: "user".to_string(),
        content: MessageContent::Text(content),
    }]
}

/// Check assistant messages for completion/blocked signals.
fn check_completion(messages: &[Message]) -> (bool, Option<String>) {
    for msg in messages.iter().rev() {
        if msg.role != "assistant" {
            continue;
        }
        let text = extract_text(msg);
        if text.contains(TASK_COMPLETE_SIGNAL) {
            return (true, None);
        }
        if let Some(idx) = text.find(TASK_BLOCKED_SIGNAL) {
            let reason = text[idx + TASK_BLOCKED_SIGNAL.len()..].trim().to_string();
            return (false, Some(reason));
        }
    }

    // If the last assistant message has no tool_use, consider it done
    // (agent stopped using tools = finished)
    if let Some(last) = messages.last() {
        if last.role == "assistant" && !has_tool_use(last) {
            return (true, None);
        }
    }

    (false, None)
}

/// Extract text content from a message.
fn extract_text(msg: &Message) -> String {
    match &msg.content {
        MessageContent::Text(t) => t.clone(),
        MessageContent::Blocks(blocks) => blocks
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

/// Check if a message contains tool_use blocks.
fn has_tool_use(msg: &Message) -> bool {
    match &msg.content {
        MessageContent::Text(_) => false,
        MessageContent::Blocks(blocks) => blocks
            .iter()
            .any(|b| matches!(b, ContentBlock::ToolUse { .. })),
    }
}

/// Extract a progress summary from conversation messages.
fn extract_progress_summary(messages: &[Message]) -> String {
    let mut summary = String::new();
    for msg in messages {
        if msg.role != "assistant" {
            continue;
        }
        let text = extract_text(msg);
        if !text.is_empty() {
            // Take last 500 chars of each assistant message for summary
            let trimmed = if text.len() > 500 {
                &text[text.len() - 500..]
            } else {
                &text
            };
            summary.push_str(trimmed);
            summary.push('\n');
        }
    }
    if summary.len() > 2000 {
        summary.truncate(2000);
    }
    summary
}

/// Emit ToolDone events for each tool_use in the messages.
fn emit_tool_events(emitter: &EventEmitter, messages: &[Message]) {
    for msg in messages {
        if msg.role != "assistant" {
            continue;
        }
        if let MessageContent::Blocks(blocks) = &msg.content {
            for block in blocks {
                if let ContentBlock::ToolUse { id, name, .. } = block {
                    emitter.emit_tool_done(name, Some(id));
                }
            }
        }
    }
}

/// Build a minimal SpawnConfig reference for prompt building.
fn spawn_config_ref(config: &WorkerConfig) -> crate::backend::SpawnConfig {
    crate::backend::SpawnConfig {
        drone_name: config.team_name.clone(),
        prd_path: config.prd_path.clone(),
        model: config.model.clone(),
        worktree_path: config.cwd.clone(),
        status_file: std::path::PathBuf::new(),
        working_dir: config.cwd.clone(),
        wait: false,
        team_name: config.team_name.clone(),
        max_agents: 0,
        claude_binary: String::new(),
        environment: None,
        structured_tasks: vec![],
        remote_url: String::new(),
        mode: String::new(),
        project_languages: vec![],
    }
}
