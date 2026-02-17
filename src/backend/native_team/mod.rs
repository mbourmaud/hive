pub mod coordinator;
pub mod events;
pub mod file_ownership;
mod phases;
pub mod prompts;
pub mod quality_gate;
pub mod scheduler;
pub mod worker;
pub mod worker_notes;

use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use anyhow::{bail, Result};

use super::{ExecutionBackend, SpawnConfig, SpawnHandle};
use crate::agent_teams;
use crate::webui::auth::credentials;
use crate::webui::chat::session::SessionStore;

use coordinator::TeamCoordinator;
use events::EventEmitter;

/// Native team execution backend.
///
/// Replaces `AgentTeamBackend` by running a deterministic Rust coordinator
/// that dispatches tasks to worker agents via `run_agentic_loop`.
/// No Claude CLI dependency — full control over prompts, models, and tools.
pub struct NativeTeamBackend;

impl ExecutionBackend for NativeTeamBackend {
    fn spawn(&self, config: &SpawnConfig) -> Result<SpawnHandle> {
        launch_native_team(config)
    }

    fn is_running(&self, handle: &SpawnHandle) -> bool {
        let complete_marker = std::path::Path::new(&handle.backend_id).join(".hive_complete");
        !complete_marker.exists()
    }

    fn stop(&self, handle: &SpawnHandle) -> Result<()> {
        // Write abort signal — coordinator polls this
        let abort_path = std::path::PathBuf::from(".hive/drones")
            .join(&handle.backend_type)
            .join(".abort");
        let _ = std::fs::write(&abort_path, "1");

        // Also try to stop the old way (if any Claude CLI processes)
        let _ = crate::backend::agent_team::stop_by_worktree_match(&handle.backend_id);
        Ok(())
    }

    fn cleanup(&self, handle: &SpawnHandle) -> Result<()> {
        if let Some(team_name) = handle.backend_id.split('/').next_back() {
            let _ = agent_teams::cleanup_team(team_name);
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "native_team"
    }

    fn is_available(&self) -> bool {
        credentials::resolve_credentials().ok().flatten().is_some()
    }
}

/// Launch the native team coordinator.
///
/// Spawns the coordinator on a non-daemon thread with its own tokio runtime.
/// Returns immediately to the caller. The process stays alive because Rust
/// waits for all non-daemon threads to exit. The PID is the current process
/// (not a child), so `hive stop` can signal us via SIGTERM.
fn launch_native_team(config: &SpawnConfig) -> Result<SpawnHandle> {
    let creds = credentials::resolve_credentials()?.ok_or_else(|| {
        anyhow::anyhow!(
            "No API credentials found. Set credentials at {:?} or configure a profile",
            credentials::credentials_path()
        )
    })?;

    if config.structured_tasks.is_empty() {
        bail!("No structured tasks in plan. Native team requires a ## Tasks section.");
    }

    let drone_dir = std::path::PathBuf::from(".hive/drones").join(&config.drone_name);

    // Touch events.ndjson so the TUI can start tailing immediately
    let events_path = drone_dir.join("events.ndjson");
    if !events_path.exists() {
        let _ = std::fs::File::create(&events_path);
    }

    let emitter = Arc::new(EventEmitter::new(
        &drone_dir,
        &config.status_file,
        &config.team_name,
    ));

    let abort_flag = Arc::new(AtomicBool::new(false));
    let session_store: SessionStore = Arc::new(tokio::sync::Mutex::new(HashMap::new()));

    let coordinator = TeamCoordinator::new(
        config.clone(),
        config.structured_tasks.clone(),
        creds,
        emitter,
        abort_flag,
        session_store,
    );

    let pid = std::process::id();
    let worktree = config.worktree_path.to_string_lossy().to_string();
    let drone_name = config.drone_name.clone();

    // Write PID immediately so `hive stop` can signal us during execution
    let _ = std::fs::write(drone_dir.join(".pid"), pid.to_string());

    // Print launch info before blocking
    println!(
        "  [hive] Native team coordinator running (pid: {pid}, tasks: {})",
        config.structured_tasks.len()
    );

    // Block: run the coordinator on this thread.
    // The start command prints status before spawn(), and the process stays
    // alive for the duration. Users run `hive start <name> &` for background.
    // `hive stop` sends SIGTERM to this PID.
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async move {
        if let Err(e) = coordinator.run().await {
            eprintln!("[hive] Native team coordinator error: {e:#}");
        }
    });

    Ok(SpawnHandle {
        pid: Some(pid),
        backend_id: worktree,
        backend_type: drone_name,
    })
}
