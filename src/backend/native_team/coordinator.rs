use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::Result;

use crate::agent_teams::preseed_tasks;
use crate::backend::SpawnConfig;
use crate::types::{DroneState, StructuredTask};
use crate::webui::auth::credentials::Credentials;
use crate::webui::chat::session::SessionStore;

use super::events::{EventEmitter, WorkerInfo};
use super::phases;
use super::scheduler::TaskScheduler;
use super::worker::{spawn_worker, WorkerConfig, WorkerHandle, WorkerResult};

/// Phases of the team coordinator's lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Dispatch,
    Monitor,
    Verify,
    Pr,
    Complete,
    Failed,
}

/// State machine driving the full lifecycle of a native agent team.
pub struct TeamCoordinator {
    config: SpawnConfig,
    scheduler: TaskScheduler,
    workers: HashMap<usize, WorkerHandle>,
    /// All workers that have ever been part of the team (accumulated).
    all_members: Vec<WorkerInfo>,
    emitter: Arc<EventEmitter>,
    abort_flag: Arc<AtomicBool>,
    creds: Credentials,
    session_store: SessionStore,
    phase: Phase,
}

impl TeamCoordinator {
    pub fn new(
        config: SpawnConfig,
        tasks: Vec<StructuredTask>,
        creds: Credentials,
        emitter: Arc<EventEmitter>,
        abort_flag: Arc<AtomicBool>,
        session_store: SessionStore,
    ) -> Self {
        // Scheduler is created lazily in run() after preseeding
        let scheduler = TaskScheduler::new(tasks, config.max_agents, &HashSet::new());
        Self {
            config,
            scheduler,
            workers: HashMap::new(),
            all_members: Vec::new(),
            emitter,
            abort_flag,
            creds,
            session_store,
            phase: Phase::Dispatch,
        }
    }

    /// Run the full coordinator lifecycle.
    pub async fn run(mut self) -> Result<()> {
        self.emitter.emit_start(&self.config.model);
        self.emitter.set_drone_state(DroneState::InProgress);

        // Pre-seed tasks to filesystem so TUI sees them immediately.
        // Returns seeded tasks — completed ones are preserved from previous runs.
        let drone_dir = std::path::PathBuf::from(".hive/drones").join(&self.config.drone_name);
        let seeded = preseed_tasks(
            &self.config.team_name,
            &self.config.structured_tasks,
            &drone_dir,
        )
        .unwrap_or_default();

        // Collect already-completed task numbers for resume support
        let completed_numbers: HashSet<usize> = seeded
            .iter()
            .filter(|t| t.status == "completed")
            .filter_map(|t| {
                t.metadata
                    .as_ref()
                    .and_then(|m| m["plan_number"].as_u64())
                    .map(|n| n as usize)
            })
            .collect();

        if !completed_numbers.is_empty() {
            eprintln!(
                "[hive] Resuming: {} tasks already completed, skipping",
                completed_numbers.len()
            );
        }

        // Rebuild scheduler with completed state from previous run
        self.scheduler = TaskScheduler::new(
            self.config.structured_tasks.clone(),
            self.config.max_agents,
            &completed_numbers,
        );

        // Write initial team config (empty, updated as workers spawn)
        let _ = self.emitter.write_team_config(&[]);

        // === DISPATCH + MONITOR loop ===
        self.phase = Phase::Monitor;
        if let Err(e) = self.run_monitor_loop().await {
            eprintln!("[hive] Monitor loop error: {e:#}");
            self.phase = Phase::Failed;
        } else if self.scheduler.has_failures() {
            eprintln!("[hive] Some tasks failed permanently, skipping verify/PR");
            self.phase = Phase::Failed;
        }

        if self.is_aborted() {
            self.finish(false);
            return Ok(());
        }

        // === VERIFY + PR phases ===
        if self.phase != Phase::Failed {
            self.phase = Phase::Verify;
            let passed =
                phases::run_verify_phase(&self.config, &self.creds, self.session_store.clone())
                    .await;

            self.phase = Phase::Pr;
            phases::run_pr_phase(
                &self.config,
                &self.creds,
                self.session_store.clone(),
                passed,
            )
            .await;
        }

        // === COMPLETE ===
        self.finish(self.phase != Phase::Failed);
        Ok(())
    }

    /// The main dispatch + monitor loop (Ralph pattern).
    async fn run_monitor_loop(&mut self) -> Result<()> {
        while !self.scheduler.all_completed() {
            if self.is_aborted() {
                self.abort_all_workers();
                return Ok(());
            }

            // Collect ready tasks (cloned to avoid borrow conflict)
            let ready: Vec<StructuredTask> = self
                .scheduler
                .ready_tasks()
                .iter()
                .map(|t| (*t).clone())
                .collect();
            for task in ready {
                let task_number = task.number;
                self.spawn_worker_for_task(task).await;
                self.scheduler.mark_running(task_number);
            }

            if self.workers.is_empty() {
                if self.scheduler.has_failures() {
                    eprintln!("[hive] All remaining tasks have unmet deps or failures");
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                continue;
            }

            let (result, worker_name) = self.wait_any_worker().await;
            self.handle_worker_result(result, worker_name);
        }

        Ok(())
    }

    /// Spawn a worker for a specific task.
    async fn spawn_worker_for_task(&mut self, task: StructuredTask) {
        let task_number = task.number;
        let task_id = self.task_number_to_id(task_number);
        let worker_name = task.worker_name();
        let model = task
            .model
            .clone()
            .unwrap_or_else(|| self.config.model.clone());

        self.emitter.update_task_file(
            &task_id,
            "in_progress",
            Some(&worker_name),
            Some(&format!("Working on: {}", task.title)),
        );
        self.emitter
            .emit_task_update(&task_id, "in_progress", Some(&worker_name));
        self.emitter.emit_worker_start(&worker_name, &model);
        self.update_team_config(&worker_name, &model);

        let handle = spawn_worker(WorkerConfig {
            task,
            creds: self.creds.clone(),
            model,
            cwd: self.config.working_dir.clone(),
            team_name: self.config.team_name.clone(),
            prd_path: self.config.prd_path.clone(),
            emitter: self.emitter.clone(),
            session_store: self.session_store.clone(),
            global_abort: self.abort_flag.clone(),
        });

        self.workers.insert(task_number, handle);
    }

    /// Wait for any running worker to complete. Returns (WorkerResult, worker_name).
    async fn wait_any_worker(&mut self) -> (WorkerResult, String) {
        loop {
            let finished = self
                .workers
                .iter()
                .find(|(_, h)| h.join_handle.is_finished())
                .map(|(k, _)| *k);

            if let Some(key) = finished {
                let handle = self.workers.remove(&key).unwrap();
                let worker_name = handle.worker_name.clone();
                let result = match handle.join_handle.await {
                    Ok(Ok(result)) => result,
                    Ok(Err(e)) => WorkerResult {
                        task_number: key,
                        success: false,
                        error: Some(format!("{e:#}")),
                    },
                    Err(e) => WorkerResult {
                        task_number: key,
                        success: false,
                        error: Some(format!("Worker panicked: {e}")),
                    },
                };
                return (result, worker_name);
            }

            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    fn handle_worker_result(&mut self, result: WorkerResult, worker_name: String) {
        let task_id = self.task_number_to_id(result.task_number);

        if result.success {
            self.scheduler.mark_completed(result.task_number);
            self.emitter
                .update_task_file(&task_id, "completed", None, None);
            let subject = self
                .scheduler
                .get_task(result.task_number)
                .map(|t| t.title.clone())
                .unwrap_or_default();
            self.emitter
                .emit_worker_done(&task_id, &subject, &worker_name);
        } else {
            let error_msg = result.error.unwrap_or_else(|| "Unknown error".to_string());
            eprintln!("[hive] Worker {worker_name} failed: {error_msg}");
            self.scheduler.mark_failed(result.task_number);

            if self.scheduler.requeue(result.task_number) {
                eprintln!(
                    "[hive] Retrying task {} (worker-{})",
                    task_id, result.task_number
                );
                self.emitter
                    .update_task_file(&task_id, "pending", None, None);
            } else {
                eprintln!(
                    "[hive] Task {} (worker-{}) exceeded max retries, marking as failed",
                    task_id, result.task_number
                );
                self.emitter
                    .update_task_file(&task_id, "completed", None, None);
            }
        }
    }

    fn abort_all_workers(&mut self) {
        for handle in self.workers.values() {
            handle.abort_flag.store(true, Ordering::Relaxed);
        }
    }

    fn is_aborted(&self) -> bool {
        self.abort_flag.load(Ordering::Relaxed)
    }

    fn finish(&self, success: bool) {
        let state = if success {
            DroneState::Completed
        } else {
            DroneState::Error
        };
        self.emitter.set_drone_state(state);
        self.emitter.emit_stop();
        let _ = crate::agent_teams::auto_complete_tasks(&self.config.team_name);
    }

    /// Map task number to pre-seeded task ID (1-based sequential for work tasks).
    fn task_number_to_id(&self, task_number: usize) -> String {
        let work_tasks: Vec<_> = self
            .config
            .structured_tasks
            .iter()
            .filter(|t| t.task_type == crate::types::TaskType::Work)
            .collect();
        work_tasks
            .iter()
            .enumerate()
            .find(|(_, t)| t.number == task_number)
            .map(|(idx, _)| (idx + 1).to_string())
            .unwrap_or_else(|| task_number.to_string())
    }

    fn update_team_config(&mut self, new_worker: &str, model: &str) {
        // Accumulate: only add if not already tracked
        if !self.all_members.iter().any(|m| m.name == new_worker) {
            self.all_members.push(WorkerInfo {
                name: new_worker.to_string(),
                model: model.to_string(),
            });
        }
        let _ = self.emitter.write_team_config(&self.all_members);
    }
}
