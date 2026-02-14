use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
};
use std::collections::HashSet;
use std::path::PathBuf;

use crate::agent_teams::snapshot::SnapshotSource;
use crate::agent_teams::task_sync::TeamTaskInfo;
use crate::commands::common::truncate_with_ellipsis;
use crate::commands::monitor::cost::format_token_count;
use crate::commands::monitor::drone_actions::extract_last_activity;
use crate::commands::monitor::state::TuiState;
use crate::events::HiveEvent;

use super::tasks::{render_nested_internal, render_user_task};
use super::team_line;

/// Render the expanded detail view for a single drone.
pub fn render_expanded_drone(
    state: &TuiState,
    lines: &mut Vec<Line>,
    drone_idx: usize,
    area: Rect,
) {
    let (name, status) = &state.drones[drone_idx];

    let task_list: Vec<TeamTaskInfo> = state
        .snapshot_store
        .get(name)
        .map(|s| s.tasks.clone())
        .unwrap_or_default();

    if !task_list.is_empty() {
        let mut tasks = task_list;
        tasks.sort_by(|a, b| a.id.cmp(&b.id));

        let member_color_map =
            team_line::render_tasks_with_team(state, lines, name, &tasks, status, area);

        let user_tasks: Vec<_> = tasks.iter().filter(|t| !t.is_internal).collect();
        let internal_tasks: Vec<_> = tasks.iter().filter(|t| t.is_internal).collect();

        if !user_tasks.is_empty() {
            let matched_agents: HashSet<_> =
                user_tasks.iter().filter_map(|t| t.owner.as_ref()).collect();

            for task in &user_tasks {
                render_user_task(lines, task, &member_color_map, area);
                if let Some(ref agent) = task.owner {
                    for itask in &internal_tasks {
                        if itask.subject == *agent {
                            render_nested_internal(lines, itask, &member_color_map, area);
                        }
                    }
                }
            }

            for itask in &internal_tasks {
                if !matched_agents.contains(&itask.subject) {
                    render_nested_internal(lines, itask, &member_color_map, area);
                }
            }
        } else {
            for task in &tasks {
                render_user_task(lines, task, &member_color_map, area);
            }
        }
    } else {
        render_no_tasks_fallback(lines, name, area);
    }

    render_data_source(state, lines, name);
    render_tool_history(state, lines, name, area);
    render_cost_details(state, lines, name);
    render_last_event(state, lines, name, area);
}

fn render_no_tasks_fallback(lines: &mut Vec<Line>, name: &str, area: Rect) {
    let log_path = PathBuf::from(".hive/drones")
        .join(name)
        .join("activity.log");
    if let Ok(contents) = std::fs::read_to_string(&log_path) {
        let last_activity = extract_last_activity(&contents);
        if !last_activity.is_empty() {
            let prefix_len = 8;
            let max_width = area.width as usize;
            let activity_display =
                truncate_with_ellipsis(&last_activity, max_width.saturating_sub(prefix_len));
            lines.push(Line::from(vec![
                Span::raw("      "),
                Span::styled("◦ ", Style::default().fg(Color::DarkGray)),
                Span::styled(activity_display, Style::default().fg(Color::DarkGray)),
            ]));
        }
    }
}

fn render_data_source(state: &TuiState, lines: &mut Vec<Line>, name: &str) {
    if let Some(snapshot) = state.snapshot_store.get(name) {
        if !snapshot.tasks.is_empty() {
            let (source_label, source_color) = match snapshot.source {
                SnapshotSource::Tasks => ("Tasks", Color::Green),
                SnapshotSource::Persisted => ("Snapshot", Color::DarkGray),
            };
            lines.push(Line::from(vec![
                Span::raw("      "),
                Span::styled("src: ", Style::default().fg(Color::DarkGray)),
                Span::styled(source_label, Style::default().fg(source_color)),
            ]));
        }
    }
}

fn render_tool_history(state: &TuiState, lines: &mut Vec<Line>, name: &str, area: Rect) {
    let Some(tool_history) = state.tool_history.get(name) else {
        return;
    };
    if tool_history.is_empty() {
        return;
    }
    let len = tool_history.len();
    let start = len.saturating_sub(6);
    let tools_summary: String = tool_history
        .iter()
        .skip(start)
        .map(|r| r.tool.as_str())
        .collect::<Vec<_>>()
        .join(" → ");
    let prefix_len = 10;
    let max_width = area.width as usize;
    let tools_display =
        truncate_with_ellipsis(&tools_summary, max_width.saturating_sub(prefix_len));
    lines.push(Line::from(vec![
        Span::raw("      "),
        Span::styled("🔧 ", Style::default().fg(Color::Cyan)),
        Span::styled(tools_display, Style::default().fg(Color::DarkGray)),
    ]));
}

fn render_cost_details(state: &TuiState, lines: &mut Vec<Line>, name: &str) {
    let c = state.cost_cache.get(name).cloned().unwrap_or_default();
    if c.total_cost_usd <= 0.0 {
        return;
    }
    let cache_suffix = if c.cache_read_tokens > 0 || c.cache_creation_tokens > 0 {
        format!(
            " | Cache: {} read, {} created",
            format_token_count(c.cache_read_tokens),
            format_token_count(c.cache_creation_tokens)
        )
    } else {
        String::new()
    };
    let cost_detail = format!(
        "      Cost: ${:.2} | In: {} | Out: {}{}",
        c.total_cost_usd,
        format_token_count(c.input_tokens),
        format_token_count(c.output_tokens),
        cache_suffix,
    );
    lines.push(Line::from(vec![Span::styled(
        cost_detail,
        Style::default().fg(Color::DarkGray),
    )]));
}

fn render_last_event(state: &TuiState, lines: &mut Vec<Line>, name: &str, area: Rect) {
    let Some(event) = state.last_events.get(name) else {
        return;
    };

    let event_desc = format_event_description(event);
    let prefix_len = 8;
    let max_width = area.width as usize;
    let event_display = truncate_with_ellipsis(&event_desc, max_width.saturating_sub(prefix_len));
    lines.push(Line::from(vec![
        Span::raw("      "),
        Span::styled("⚡ ", Style::default().fg(Color::Cyan)),
        Span::styled(event_display, Style::default().fg(Color::DarkGray)),
    ]));
}

fn format_event_description(event: &HiveEvent) -> String {
    match event {
        HiveEvent::TaskCreate { subject, .. } => format!("Created: {}", subject),
        HiveEvent::TaskUpdate {
            task_id, status, ..
        } => format!("Task {} → {}", task_id, status),
        HiveEvent::Message {
            recipient, summary, ..
        } => format!("Msg → @{}: {}", recipient, summary),
        HiveEvent::TaskDone { subject, agent, .. } => {
            let suffix = agent
                .as_ref()
                .map(|a| format!(" @{}", a))
                .unwrap_or_default();
            format!("Done: {}{}", subject, suffix)
        }
        HiveEvent::Idle { agent, .. } => format!("@{} idle", agent),
        HiveEvent::Stop { .. } => "Stopped".to_string(),
        HiveEvent::Start { model, .. } => format!("Started ({})", model),
        HiveEvent::AgentSpawn { name, model, .. } => {
            let suffix = model
                .as_ref()
                .map(|m| format!(" ({})", m))
                .unwrap_or_default();
            format!("Agent: {}{}", name, suffix)
        }
        HiveEvent::SubagentStart { agent_type, .. } => {
            let suffix = agent_type
                .as_ref()
                .map(|t| format!(" ({})", t))
                .unwrap_or_default();
            format!("Subagent started{}", suffix)
        }
        HiveEvent::SubagentStop { agent_type, .. } => {
            let suffix = agent_type
                .as_ref()
                .map(|t| format!(" ({})", t))
                .unwrap_or_default();
            format!("Subagent stopped{}", suffix)
        }
        HiveEvent::ToolDone { tool, .. } => format!("Tool: {}", tool),
        HiveEvent::TodoSnapshot { todos, .. } => {
            let done = todos.iter().filter(|t| t.status == "completed").count();
            format!("Todos: {}/{}", done, todos.len())
        }
    }
}
