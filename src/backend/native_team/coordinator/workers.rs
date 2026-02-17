use std::path::PathBuf;

use crate::types::StructuredTask;

use super::super::events::WorkerInfo;
use super::super::worker::{spawn_worker, WorkerConfig, WorkerResult};
use super::super::worker_notes;
use super::TeamCoordinator;

impl TeamCoordinator {
    /// Spawn a worker for a specific task.
    pub(super) async fn spawn_worker_for_task(&mut self, task: StructuredTask) {
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

        // Load dependency notes from previous workers
        let drone_dir = PathBuf::from(".hive/drones").join(&self.config.drone_name);
        let dep_notes = worker_notes::read_dependency_notes(&drone_dir, &task.depends_on);

        let handle = spawn_worker(WorkerConfig {
            task,
            creds: self.creds.clone(),
            model,
            cwd: self.config.working_dir.clone(),
            team_name: self.config.team_name.clone(),
            drone_name: self.config.drone_name.clone(),
            prd_path: self.config.prd_path.clone(),
            project_languages: self.config.project_languages.clone(),
            emitter: self.emitter.clone(),
            session_store: self.session_store.clone(),
            global_abort: self.abort_flag.clone(),
            dependency_notes: dep_notes,
        });

        self.workers.insert(task_number, handle);
    }

    /// Wait for any running worker to complete. Returns (WorkerResult, worker_name).
    pub(super) async fn wait_any_worker(&mut self) -> (WorkerResult, String) {
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

    /// Handle a worker result: update scheduler, emit events, fast re-dispatch.
    pub(super) async fn handle_worker_result(&mut self, result: WorkerResult, worker_name: String) {
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

            // Fast re-dispatch: immediately check for next ready task
            let ready: Vec<StructuredTask> = self
                .scheduler
                .ready_tasks()
                .iter()
                .map(|t| (*t).clone())
                .collect();
            if let Some(task) = ready.into_iter().next() {
                let num = task.number;
                self.spawn_worker_for_task(task).await;
                self.scheduler.mark_running(num);
            }
        } else {
            let error_msg = result.error.unwrap_or_else(|| "Unknown error".to_string());
            eprintln!("[hive] Worker {worker_name} failed: {error_msg}");
            self.emitter.emit_worker_error(&task_id, &error_msg);
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

    /// Map task number to pre-seeded task ID (1-based sequential for work tasks).
    pub(super) fn task_number_to_id(&self, task_number: usize) -> String {
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

    pub(super) fn update_team_config(&mut self, new_worker: &str, model: &str) {
        if !self.all_members.iter().any(|m| m.name == new_worker) {
            self.all_members.push(WorkerInfo {
                name: new_worker.to_string(),
                model: model.to_string(),
            });
        }
        let _ = self.emitter.write_team_config(&self.all_members);
    }
}
