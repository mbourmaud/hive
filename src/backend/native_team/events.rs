use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde_json::json;

use crate::agent_teams::{team_dir, team_tasks_dir, AgentTeamTask};
use crate::events::HiveEvent;
use crate::types::{DroneState, DroneStatus};

/// Writes filesystem artifacts for TUI compatibility.
///
/// Emits HiveEvents to `events.ndjson`, updates `status.json`,
/// writes team config, and updates individual task files.
pub struct EventEmitter {
    events_path: PathBuf,
    status_path: PathBuf,
    team_name: String,
}

impl EventEmitter {
    pub fn new(drone_dir: &Path, status_path: &Path, team_name: &str) -> Self {
        Self {
            events_path: drone_dir.join("events.ndjson"),
            status_path: status_path.to_path_buf(),
            team_name: team_name.to_string(),
        }
    }

    /// Append a HiveEvent to events.ndjson.
    pub fn emit(&self, event: &HiveEvent) {
        let Ok(line) = serde_json::to_string(event) else {
            return;
        };
        let Ok(mut file) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.events_path)
        else {
            return;
        };
        let _ = writeln!(file, "{line}");
    }

    /// Update the DroneStatus JSON file.
    pub fn update_status(&self, status: &DroneStatus) {
        let Ok(json) = serde_json::to_string_pretty(status) else {
            return;
        };
        let _ = fs::write(&self.status_path, json);
    }

    pub fn emit_start(&self, model: &str) {
        self.emit(&HiveEvent::Start {
            ts: now(),
            model: model.to_string(),
        });
    }

    pub fn emit_stop(&self) {
        self.emit(&HiveEvent::Stop { ts: now() });
    }

    pub fn emit_worker_start(&self, worker_name: &str, model: &str) {
        self.emit(&HiveEvent::SubagentStart {
            ts: now(),
            agent_id: worker_name.to_string(),
            agent_type: Some("worker".to_string()),
        });
        self.emit(&HiveEvent::AgentSpawn {
            ts: now(),
            name: worker_name.to_string(),
            model: Some(model.to_string()),
            subagent_type: Some("worker".to_string()),
        });
    }

    pub fn emit_worker_done(&self, task_id: &str, subject: &str, worker: &str) {
        self.emit(&HiveEvent::TaskDone {
            ts: now(),
            task_id: task_id.to_string(),
            subject: subject.to_string(),
            agent: Some(worker.to_string()),
        });
        self.emit(&HiveEvent::SubagentStop {
            ts: now(),
            agent_id: worker.to_string(),
            agent_type: Some("worker".to_string()),
        });
    }

    pub fn emit_tool_done(&self, tool: &str, tool_use_id: Option<&str>) {
        self.emit(&HiveEvent::ToolDone {
            ts: now(),
            tool: tool.to_string(),
            tool_use_id: tool_use_id.map(str::to_string),
        });
    }

    pub fn emit_task_update(&self, task_id: &str, status: &str, owner: Option<&str>) {
        self.emit(&HiveEvent::TaskUpdate {
            ts: now(),
            task_id: task_id.to_string(),
            status: status.to_string(),
            owner: owner.map(str::to_string),
        });
    }

    /// Write/update the team config at `~/.claude/teams/{team}/config.json`.
    pub fn write_team_config(&self, workers: &[WorkerInfo]) -> Result<()> {
        let team_path = team_dir(&self.team_name);
        fs::create_dir_all(&team_path)?;

        let members: Vec<serde_json::Value> = workers
            .iter()
            .map(|w| {
                json!({
                    "name": w.name,
                    "agentType": "worker",
                    "model": w.model,
                })
            })
            .collect();

        let config = json!({
            "name": self.team_name,
            "members": members,
        });

        fs::write(
            team_path.join("config.json"),
            serde_json::to_string_pretty(&config)?,
        )?;
        Ok(())
    }

    /// Update a single task file in `~/.claude/tasks/{team}/{id}.json`.
    pub fn update_task_file(
        &self,
        task_id: &str,
        status: &str,
        owner: Option<&str>,
        active_form: Option<&str>,
    ) {
        let task_path = team_tasks_dir(&self.team_name).join(format!("{task_id}.json"));
        let Ok(contents) = fs::read_to_string(&task_path) else {
            return;
        };
        let Ok(mut task) = serde_json::from_str::<AgentTeamTask>(&contents) else {
            return;
        };

        task.status = status.to_string();
        if let Some(o) = owner {
            task.owner = Some(o.to_string());
        }
        if let Some(af) = active_form {
            task.active_form = Some(af.to_string());
        }
        task.updated_at = Some(epoch_millis());

        if let Ok(json) = serde_json::to_string_pretty(&task) {
            let _ = fs::write(&task_path, json);
        }
    }

    /// Update DroneStatus to set the current phase/state.
    pub fn set_drone_state(&self, state: DroneState) {
        let Ok(contents) = fs::read_to_string(&self.status_path) else {
            return;
        };
        let Ok(mut status) = serde_json::from_str::<DroneStatus>(&contents) else {
            return;
        };
        status.status = state;
        status.updated = now();
        self.update_status(&status);
    }

    /// Update DroneStatus to persist the current coordinator phase.
    pub fn set_drone_phase(&self, phase: &str) {
        let Ok(contents) = fs::read_to_string(&self.status_path) else {
            return;
        };
        let Ok(mut status) = serde_json::from_str::<DroneStatus>(&contents) else {
            return;
        };
        status.phase = Some(phase.to_string());
        status.updated = now();
        self.update_status(&status);
    }

    pub fn emit_quality_gate(&self, task_id: &str, passed: bool, output: &str) {
        self.emit(&HiveEvent::QualityGateResult {
            ts: now(),
            task_id: task_id.to_string(),
            passed,
            output: output.to_string(),
        });
    }

    pub fn emit_worker_error(&self, task_id: &str, error: &str) {
        self.emit(&HiveEvent::WorkerError {
            ts: now(),
            task_id: task_id.to_string(),
            error_message: error.to_string(),
        });
    }

    pub fn emit_phase_transition(&self, from: &str, to: &str) {
        self.emit(&HiveEvent::PhaseTransition {
            ts: now(),
            from_phase: from.to_string(),
            to_phase: to.to_string(),
        });
    }
}

/// Info about a worker for team config.
pub struct WorkerInfo {
    pub name: String,
    pub model: String,
}

impl EventEmitter {
    /// Append a cost record to cost.ndjson in the drone directory.
    /// Each line: `{"input_tokens":N,"output_tokens":N,"cache_read":N,"cache_create":N}`
    /// The polling code sums all lines to get the total.
    pub fn emit_cost(&self, usage: &crate::webui::anthropic::types::UsageStats) {
        let cost_path = self
            .events_path
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("cost.ndjson");
        let Ok(line) = serde_json::to_string(&serde_json::json!({
            "input_tokens": usage.input_tokens,
            "output_tokens": usage.output_tokens,
            "cache_read": usage.cache_read_input_tokens,
            "cache_create": usage.cache_creation_input_tokens,
        })) else {
            return;
        };
        let Ok(mut file) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&cost_path)
        else {
            return;
        };
        let _ = writeln!(file, "{line}");
    }
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn epoch_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
