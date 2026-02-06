pub mod agent_team;
pub mod native;

use anyhow::Result;
use std::path::PathBuf;

use crate::types::ExecutionMode;

/// Configuration for spawning a drone process.
pub struct SpawnConfig {
    pub drone_name: String,
    pub prd_path: PathBuf,
    pub model: String,
    pub worktree_path: PathBuf,
    pub status_file: PathBuf,
    pub working_dir: PathBuf,
    pub execution_mode: ExecutionMode,
    pub wait: bool,
    /// Team name for Agent Teams mode
    pub team_name: Option<String>,
    /// Teammate spawning mode: "in-process", "tmux", or "auto"
    pub teammate_mode: Option<String>,
    /// Worktree assignments for each teammate in Agent Teams mode
    pub worktree_assignments: Option<Vec<WorktreeAssignment>>,
}

/// Worktree assignment for a teammate in Agent Teams mode.
pub struct WorktreeAssignment {
    pub teammate_name: String,
    pub worktree_path: PathBuf,
    pub branch: String,
    pub story_ids: Vec<String>,
}

/// Handle returned by a backend after spawning a drone.
pub struct SpawnHandle {
    pub pid: Option<u32>,
    pub backend_id: String,
    pub backend_type: String,
}

/// Trait for execution backends that can spawn, monitor, and stop drones.
pub trait ExecutionBackend {
    /// Spawn a new drone process with the given configuration.
    fn spawn(&self, config: &SpawnConfig) -> Result<SpawnHandle>;

    /// Check if a drone is still running.
    fn is_running(&self, handle: &SpawnHandle) -> bool;

    /// Stop a running drone.
    fn stop(&self, handle: &SpawnHandle) -> Result<()>;

    /// Clean up drone artifacts (worktree, branch, etc.).
    fn cleanup(&self, handle: &SpawnHandle) -> Result<()>;

    /// Return the name of this backend.
    fn name(&self) -> &str;

    /// Check if this backend is available on the current system.
    fn is_available(&self) -> bool;
}

/// Resolve which backend to use based on configuration and execution mode.
pub fn resolve_backend(_default_backend: Option<&str>) -> Box<dyn ExecutionBackend> {
    Box::new(native::NativeBackend)
}

/// Resolve backend specifically for Agent Teams mode.
pub fn resolve_agent_team_backend() -> Box<dyn ExecutionBackend> {
    Box::new(agent_team::AgentTeamBackend)
}
