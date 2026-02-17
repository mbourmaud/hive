mod completion;
mod emit;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

use crate::types::StructuredTask;
use crate::webui::anthropic::types::{Message, MessageContent};
use crate::webui::auth::credentials::Credentials;
use crate::webui::chat::handlers::agentic::{run_agentic_loop, AgenticLoopParams};
use crate::webui::chat::session::{Effort, SessionStore};
use crate::webui::provider;
use crate::webui::tools::definitions::builtin_tool_definitions;

use super::events::EventEmitter;
use super::file_ownership::ownership_prompt_for_files;
use super::prompts::{build_continuation_prompt, build_worker_prompt};
use super::quality_gate::{self, GateResult};
use super::worker_notes::{self, WorkerNote};

pub use completion::{check_completion, extract_progress_summary};
pub use emit::{emit_cost_from_store, emit_tool_events, spawn_config_ref};

const MAX_ITERATIONS: usize = 10;

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
    pub cwd: PathBuf,
    pub team_name: String,
    pub drone_name: String,
    pub prd_path: PathBuf,
    pub project_languages: Vec<String>,
    pub emitter: Arc<EventEmitter>,
    pub session_store: SessionStore,
    pub global_abort: Arc<AtomicBool>,
    pub dependency_notes: Vec<WorkerNote>,
}

/// Spawn a worker agent for a single task.
pub fn spawn_worker(config: WorkerConfig) -> WorkerHandle {
    let task_number = config.task.number;
    let worker_name = config.task.worker_name();
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

/// The worker's main run loop.
async fn run_worker(config: WorkerConfig, abort_flag: Arc<AtomicBool>) -> Result<WorkerResult> {
    let task_number = config.task.number;
    let model_id = provider::resolve_model(&config.model, &config.creds);
    let worker_name = config.task.worker_name();

    let ownership_hint = ownership_prompt_for_files(&config.task.files);
    let system_prompt = build_worker_prompt(
        &config.task,
        &spawn_config_ref(&config),
        &ownership_hint,
        &config.dependency_notes,
    );
    let tools = builtin_tool_definitions();
    let (tx, _rx) = broadcast::channel::<String>(256);
    let gate_config = quality_gate::build_gate_config(&config.project_languages, &config.cwd);
    let drone_dir = PathBuf::from(".hive/drones").join(&config.drone_name);

    let mut continuation_context: Option<String> = None;

    for iteration in 0..MAX_ITERATIONS {
        if abort_flag.load(Ordering::Relaxed) || config.global_abort.load(Ordering::Relaxed) {
            return Ok(WorkerResult {
                task_number,
                success: false,
                error: Some("Aborted".to_string()),
            });
        }

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

        emit_cost_from_store(&config.session_store, &worker_name, &config.emitter).await;
        emit_tool_events(&config.emitter, &result_messages);

        let (complete, blocked_reason) = check_completion(&result_messages);

        if complete {
            // Run quality gate before accepting completion
            if let Some(ref gc) = gate_config {
                let task_id = task_number.to_string();
                match quality_gate::run_quality_gate(gc).await {
                    GateResult::Passed => {
                        config.emitter.emit_quality_gate(&task_id, true, "");
                    }
                    GateResult::Failed { output } => {
                        config.emitter.emit_quality_gate(&task_id, false, &output);
                        continuation_context = Some(format!(
                            "Quality gate failed. Fix these errors:\n\n{output}"
                        ));
                        continue;
                    }
                    GateResult::Timeout => {
                        config
                            .emitter
                            .emit_quality_gate(&task_id, false, "Timed out");
                        continuation_context = Some("Quality gate timed out.".into());
                        continue;
                    }
                }
            }

            // Write worker notes for downstream tasks
            let files = worker_notes::detect_files_changed(&config.cwd).await;
            let summary = completion::extract_completion_summary(&result_messages);
            let note = WorkerNote {
                task_number,
                task_title: config.task.title.clone(),
                files_changed: files,
                summary,
            };
            let _ = worker_notes::append_note(&drone_dir, &note);

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
