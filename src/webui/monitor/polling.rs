use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::agent_teams::snapshot::TaskSnapshotStore;
use crate::agent_teams::task_sync;
use crate::commands::common::cost::{parse_cost_from_log, parse_cost_from_log_at, CostSummary};
use crate::commands::common::{list_drones, list_drones_at};
use crate::config;

use super::dto::{CostInfo, DroneInfo, MemberInfo, MessageInfo, ProjectInfo, TaskInfo};
use super::liveness::{
    compute_elapsed, compute_task_duration, determine_liveness, determine_member_liveness,
};

/// Shared state for snapshot stores, keyed by project path.
pub type SnapshotStores = Mutex<HashMap<String, TaskSnapshotStore>>;

pub fn poll_all_projects(snapshot_stores: &SnapshotStores) -> Vec<ProjectInfo> {
    let mut project_paths: Vec<(String, String)> = config::load_projects_registry()
        .unwrap_or_default()
        .projects
        .into_iter()
        .map(|p| (p.path, p.name))
        .collect();

    if let Ok(cwd) = std::env::current_dir() {
        let cwd_str = cwd.to_string_lossy().to_string();
        if !project_paths.iter().any(|(p, _)| *p == cwd_str) {
            let name = cwd
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("current")
                .to_string();
            project_paths.push((cwd_str, name));
        }
    }

    let mut stores = snapshot_stores
        .lock()
        .expect("snapshot_stores lock poisoned");
    let mut projects = Vec::new();

    let cwd = std::env::current_dir().unwrap_or_default();

    for (project_path, project_name) in &project_paths {
        let root = PathBuf::from(project_path);

        if !root.join(".hive/drones").exists() {
            continue;
        }

        let is_cwd = root == cwd;
        let drones_list = if is_cwd {
            list_drones().unwrap_or_default()
        } else {
            list_drones_at(&root).unwrap_or_default()
        };

        let store = stores
            .entry(project_path.clone())
            .or_insert_with(|| TaskSnapshotStore::with_project_root(root.clone()));

        let mut drone_infos = Vec::new();

        for (name, status) in &drones_list {
            let snapshot = store.update(name);
            let cost = if is_cwd {
                parse_cost_from_log(name)
            } else {
                parse_cost_from_log_at(&root, name)
            };

            let liveness = determine_liveness(&root, name, &status.status);

            let tasks: Vec<TaskInfo> = snapshot
                .tasks
                .iter()
                .map(|t| {
                    let duration = compute_task_duration(t.created_at, t.updated_at, &t.status);
                    let blocked_by = if !t.blocked_by.is_empty() && t.status != "completed" {
                        Some(t.blocked_by.join(", "))
                    } else {
                        None
                    };
                    TaskInfo {
                        id: t.id.clone(),
                        subject: t.subject.clone(),
                        description: t.description.clone(),
                        status: t.status.clone(),
                        owner: t.owner.clone(),
                        active_form: t.active_form.clone(),
                        is_internal: t.is_internal,
                        duration,
                        retry_count: 0, // TODO(mbourmaud): read from task metadata
                        blocked_by,
                    }
                })
                .collect();

            let mut tasks = tasks;
            tasks.sort_by_key(|t| t.id.parse::<usize>().unwrap_or(usize::MAX));

            let members: Vec<MemberInfo> = snapshot
                .members
                .iter()
                .map(|m| {
                    let member_liveness = determine_member_liveness(&m.name, &tasks);
                    // Find current task assigned to this member
                    let current_task_id = tasks
                        .iter()
                        .find(|t| t.status == "in_progress" && t.owner.as_deref() == Some(&m.name))
                        .map(|t| t.id.clone());
                    MemberInfo {
                        name: m.name.clone(),
                        agent_type: m.agent_type.clone(),
                        model: m.model.clone(),
                        liveness: member_liveness,
                        current_task_id,
                    }
                })
                .collect();

            let messages = collect_messages(name);
            let elapsed = compute_elapsed(&status.started);

            drone_infos.push(DroneInfo {
                name: name.clone(),
                title: status.title.clone(),
                description: status.description.clone(),
                status: format!("{:?}", status.status).to_lowercase(),
                branch: status.branch.clone(),
                worktree: status.worktree.clone(),
                lead_model: status.lead_model.clone(),
                phase: status.phase.clone(),
                started: status.started.clone(),
                updated: status.updated.clone(),
                elapsed,
                tasks,
                members,
                messages,
                progress: snapshot.progress,
                cost: cost_to_info(&cost),
                liveness,
            });
        }

        let total_cost: f64 = drone_infos.iter().map(|d| d.cost.total_usd).sum();
        let active_count = drone_infos
            .iter()
            .filter(|d| d.liveness == "working")
            .count();

        projects.push(ProjectInfo {
            name: project_name.clone(),
            path: project_path.clone(),
            drones: drone_infos,
            total_cost,
            active_count,
        });
    }

    projects
}

fn collect_messages(drone_name: &str) -> Vec<MessageInfo> {
    let inboxes = task_sync::read_team_inboxes(drone_name).unwrap_or_default();
    let mut messages: Vec<MessageInfo> = Vec::new();

    for (recipient, inbox) in &inboxes {
        for msg in inbox {
            if msg.text.trim().is_empty() {
                continue;
            }
            if msg.text.trim_start().starts_with('{') {
                continue;
            }
            messages.push(MessageInfo {
                from: msg.from.clone(),
                to: recipient.clone(),
                text: msg.text.clone(),
                timestamp: msg.timestamp.clone(),
            });
        }
    }

    messages.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    messages
}

fn cost_to_info(cost: &CostSummary) -> CostInfo {
    CostInfo {
        total_usd: cost.total_cost_usd,
        input_tokens: cost.input_tokens,
        output_tokens: cost.output_tokens,
        cache_creation_tokens: cost.cache_creation_tokens,
        cache_read_tokens: cost.cache_read_tokens,
    }
}
