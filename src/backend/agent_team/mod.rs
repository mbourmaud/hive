mod launch;
pub(crate) mod prompts;

use anyhow::Result;
use std::process::{Command as ProcessCommand, Stdio};

use super::{ExecutionBackend, SpawnConfig, SpawnHandle};
use crate::agent_teams;

pub use launch::stop_by_worktree_match;

/// Agent Teams execution backend.
/// Launches a Claude Code team lead session that coordinates teammates
/// via Agent Teams native multi-agent collaboration.
pub struct AgentTeamBackend;

impl ExecutionBackend for AgentTeamBackend {
    fn spawn(&self, config: &SpawnConfig) -> Result<SpawnHandle> {
        launch::launch_agent_team(config)
    }

    fn is_running(&self, handle: &SpawnHandle) -> bool {
        handle
            .pid
            .map(|pid| crate::commands::common::is_process_running(pid as i32))
            .unwrap_or(false)
    }

    fn stop(&self, handle: &SpawnHandle) -> Result<()> {
        // Stop the lead process (which manages teammate lifecycle)
        stop_by_worktree_match(&handle.backend_id)
    }

    fn cleanup(&self, handle: &SpawnHandle) -> Result<()> {
        // Clean up Agent Teams directories
        if let Some(team_name) = handle.backend_id.split('/').next_back() {
            let _ = agent_teams::cleanup_team(team_name);
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "agent_team"
    }

    fn is_available(&self) -> bool {
        // Check if `claude` CLI is available
        ProcessCommand::new("claude")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}
