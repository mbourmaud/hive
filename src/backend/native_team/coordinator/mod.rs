mod workers;

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
use super::scheduler::TaskScheduler;
use super::worker::WorkerHandle;

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

impl Phase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Phase::Dispatch => "dispatch",
            Phase::Monitor => "monitor",
            Phase::Verify => "verify",
            Phase::Pr => "pr",
            Phase::Complete => "complete",
            Phase::Failed => "failed",
        }
    }
}

/// State machine driving the full lifecycle of a native agent team.
pub struct TeamCoordinator {
    pub(super) config: SpawnConfig,
    pub(super) scheduler: TaskScheduler,
    pub(super) workers: HashMap<usize, WorkerHandle>,
    /// All workers that have ever been part of the team (accumulated).
    pub(super) all_members: Vec<WorkerInfo>,
    pub(super) emitter: Arc<EventEmitter>,
    pub(super) abort_flag: Arc<AtomicBool>,
    pub(super) creds: Credentials,
    pub(super) session_store: SessionStore,
    pub(super) phase: Phase,
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
        self.transition_phase(Phase::Monitor);
        if let Err(e) = self.run_monitor_loop().await {
            eprintln!("[hive] Monitor loop error: {e:#}");
            self.transition_phase(Phase::Failed);
        } else if self.scheduler.has_failures() {
            eprintln!("[hive] Some tasks failed permanently, skipping verify/PR");
            self.transition_phase(Phase::Failed);
        }

        if self.is_aborted() {
            self.finish(false);
            return Ok(());
        }

        // === VERIFY + PR phases ===
        if self.phase != Phase::Failed {
            self.transition_phase(Phase::Verify);
            let passed = super::phases::run_verify_phase(
                &self.config,
                &self.creds,
                self.session_store.clone(),
            )
            .await;

            self.transition_phase(Phase::Pr);
            super::phases::run_pr_phase(
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
            self.handle_worker_result(result, worker_name).await;
        }

        Ok(())
    }

    fn transition_phase(&mut self, new_phase: Phase) {
        let old = self.phase.as_str().to_string();
        self.phase = new_phase;
        self.emitter.emit_phase_transition(&old, new_phase.as_str());
        self.emitter.set_drone_phase(new_phase.as_str());
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
}
