pub mod agent_team;

use anyhow::Result;
use std::path::PathBuf;

/// Configuration for spawning a drone process.
pub struct SpawnConfig {
    pub drone_name: String,
    pub prd_path: PathBuf,
    pub model: String,
    pub worktree_path: PathBuf,
    pub status_file: PathBuf,
    pub working_dir: PathBuf,
    pub wait: bool,
    /// Team name for Agent Teams mode
    pub team_name: String,
    /// Maximum number of concurrent teammates the team lead can spawn
    pub max_agents: usize,
    /// Worktree assignments for each teammate in Agent Teams mode
    pub worktree_assignments: Option<Vec<WorktreeAssignment>>,
    /// Claude binary to use (e.g., "claude" or "claude-ml")
    pub claude_binary: String,
    /// Environment variables to set when spawning Claude
    pub environment: Option<Vec<(String, String)>>,
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

/// Resolve the Agent Teams backend.
pub fn resolve_agent_team_backend() -> Box<dyn ExecutionBackend> {
    Box::new(agent_team::AgentTeamBackend)
}
