use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::collections::HashMap;

use crate::agent_teams::task_sync::{TeamMember, TeamTaskInfo};
use crate::commands::common::truncate_with_ellipsis;
use crate::commands::monitor::state::TuiState;

use super::helpers::{get_agent_color, shorten_model_name};

/// Build the agent color map and render the team line + agent badges for an expanded drone.
pub fn render_tasks_with_team(
    state: &TuiState,
    lines: &mut Vec<Line>,
    name: &str,
    tasks: &[TeamTaskInfo],
    status: &crate::types::DroneStatus,
    area: Rect,
) -> HashMap<String, usize> {
    let lead_model = status.lead_model.as_ref().map(|m| shorten_model_name(m));

    let members = state
        .snapshot_store
        .get(name)
        .map(|s| s.members.clone())
        .unwrap_or_default();

    let snapshot_agents = state
        .snapshot_store
        .get(name)
        .map(|s| &s.agents)
        .filter(|a| !a.is_empty());

    let ws_agents: Vec<(String, Option<String>)> = if members.is_empty() {
        if let Some(agents) = snapshot_agents {
            agents
                .iter()
                .map(|a| (a.name.clone(), a.model.clone()))
                .collect()
        } else {
            build_agents_from_tasks(tasks)
        }
    } else {
        Vec::new()
    };

    let member_color_map: HashMap<String, usize> = if !members.is_empty() {
        members
            .iter()
            .enumerate()
            .map(|(idx, m)| (m.name.clone(), idx))
            .collect()
    } else {
        ws_agents
            .iter()
            .enumerate()
            .map(|(idx, (aname, _))| (aname.clone(), idx))
            .collect()
    };

    render_team_line(lines, &lead_model, &members, &ws_agents, area);
    member_color_map
}

fn build_agents_from_tasks(tasks: &[TeamTaskInfo]) -> Vec<(String, Option<String>)> {
    let mut agent_map: HashMap<String, Option<String>> = HashMap::new();
    for t in tasks {
        if let Some(ref owner) = t.owner {
            agent_map.entry(owner.clone()).or_insert(None);
        }
    }
    let mut agents: Vec<(String, Option<String>)> = agent_map.into_iter().collect();
    agents.sort_by(|a, b| a.0.cmp(&b.0));
    agents
}

fn render_team_line(
    lines: &mut Vec<Line>,
    lead_model: &Option<String>,
    members: &[TeamMember],
    ws_agents: &[(String, Option<String>)],
    area: Rect,
) {
    let mut team_spans: Vec<Span> = vec![
        Span::raw("      "),
        Span::styled("Lead", Style::default().fg(Color::DarkGray)),
    ];
    if let Some(ref model) = lead_model {
        team_spans.push(Span::styled(
            format!(" ({})", model),
            Style::default().fg(Color::Magenta),
        ));
    }

    let agent_entries: Vec<(String, Color, Option<String>)> = if !members.is_empty() {
        members
            .iter()
            .enumerate()
            .map(|(idx, m)| {
                let model_str = if !m.model.is_empty() {
                    Some(shorten_model_name(&m.model))
                } else {
                    None
                };
                (m.name.clone(), get_agent_color(idx), model_str)
            })
            .collect()
    } else {
        ws_agents
            .iter()
            .enumerate()
            .map(|(idx, (aname, amodel))| {
                let model_str = amodel.as_ref().map(|m| shorten_model_name(m));
                (aname.clone(), get_agent_color(idx), model_str)
            })
            .collect()
    };

    if !agent_entries.is_empty() {
        team_spans.push(Span::styled(
            format!(" | {} agents", agent_entries.len()),
            Style::default().fg(Color::DarkGray),
        ));
        lines.push(Line::from(team_spans));
        render_agent_badges(lines, &agent_entries, area);
    } else {
        lines.push(Line::from(team_spans));
    }
}

fn render_agent_badges(
    lines: &mut Vec<Line>,
    agent_entries: &[(String, Color, Option<String>)],
    area: Rect,
) {
    let max_agent_name_len = 20;
    let max_line_width = area.width as usize;
    let indent = "        "; // 8 spaces
    let mut current_spans: Vec<Span> = vec![Span::raw(indent.to_string())];
    let mut current_len = indent.len();

    for (i, (name, color, model_str)) in agent_entries.iter().enumerate() {
        let display_name = truncate_with_ellipsis(name, max_agent_name_len);
        let badge = format!("@{}", display_name);
        let model_part = model_str
            .as_ref()
            .map(|m| format!(" ({})", m))
            .unwrap_or_default();
        let sep = if i > 0 { "  " } else { "" };
        let entry_len = sep.len() + badge.len() + model_part.len();

        if current_len + entry_len > max_line_width && current_len > indent.len() {
            lines.push(Line::from(current_spans));
            current_spans = vec![Span::raw(indent.to_string())];
            current_len = indent.len();
        } else if i > 0 {
            current_spans.push(Span::styled("  ", Style::default().fg(Color::DarkGray)));
            current_len += 2;
        }

        current_spans.push(Span::styled(
            badge.clone(),
            Style::default().fg(*color).add_modifier(Modifier::BOLD),
        ));
        current_len += badge.len();

        if let Some(ref m) = model_str {
            let part = format!(" ({})", m);
            current_spans.push(Span::styled(
                part.clone(),
                Style::default().fg(Color::DarkGray),
            ));
            current_len += part.len();
        }
    }

    if current_len > indent.len() {
        lines.push(Line::from(current_spans));
    }
}
