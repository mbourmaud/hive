use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::agent_teams::snapshot::SnapshotSource;
use crate::agent_teams::task_sync::TeamTaskInfo;
use crate::commands::common::{
    format_duration, is_process_running, parse_timestamp, read_drone_pid, truncate_with_ellipsis,
    DEFAULT_INACTIVE_THRESHOLD_SECS, MAX_DRONE_NAME_LEN,
};
use crate::events::HiveEvent;
use crate::types::DroneState;

use super::cost::format_token_count;
use super::drone_actions::extract_last_activity;
use super::state::TuiState;
use super::views::render_messages_view;
use super::views::render_tools_view;

/// Get a unique color for an agent based on their index
fn get_agent_color(agent_index: usize) -> Color {
    let palette = [
        Color::Cyan,
        Color::Magenta,
        Color::Yellow,
        Color::Blue,
        Color::Green,
        Color::Red,
        Color::LightCyan,
        Color::LightMagenta,
    ];
    palette[agent_index % palette.len()]
}

impl TuiState {
    pub fn render(&mut self, f: &mut Frame) {
        let area = f.area();

        // Tools view (toggle with 't')
        if let Some(ref drone_name) = self.tools_view {
            let tool_history = self.tool_history.get(drone_name);
            render_tools_view(f, area, drone_name, tool_history);
            return;
        }

        // Messages view (toggle with 'm')
        if let Some(ref drone_name) = self.messages_view {
            render_messages_view(f, area, drone_name, self.messages_selected_index);
            return;
        }

        // Main layout: header, content, footer
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5), // Header with ASCII art + padding
                Constraint::Min(0),    // Content
                Constraint::Length(1), // Footer
            ])
            .split(area);

        // Header with ASCII art (with top padding)
        let header_lines = vec![
            Line::raw(""), // Top padding
            Line::from(vec![
                Span::styled("  ╦ ╦╦╦  ╦╔═╗", Style::default().fg(Color::Yellow)),
                Span::styled(
                    "  Orchestrate Claude Code",
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
            Line::from(vec![
                Span::styled("  ╠═╣║╚╗╔╝║╣ ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("  v{}", env!("CARGO_PKG_VERSION")),
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
            Line::from(vec![
                Span::styled("  ╩ ╩╩ ╚╝ ╚═╝", Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!(
                        "  {} drone{}",
                        self.drones.len(),
                        if self.drones.len() != 1 { "s" } else { "" }
                    ),
                    Style::default().fg(Color::Cyan),
                ),
            ]),
        ];
        f.render_widget(Paragraph::new(header_lines), chunks[0]);

        // Build content lines
        let mut lines: Vec<Line> = Vec::new();
        let mut drone_line_indices: Vec<usize> = Vec::new();

        // Show placeholder when no drones
        if self.drones.is_empty() {
            lines.push(Line::raw(""));
            lines.push(Line::raw(""));
            lines.push(Line::from(vec![Span::styled(
                "  No drones running",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::raw(""));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("Get started:", Style::default().fg(Color::Yellow)),
            ]));
            lines.push(Line::raw(""));
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled("1. ", Style::default().fg(Color::Cyan)),
                Span::styled("Create a plan with ", Style::default().fg(Color::White)),
                Span::styled(
                    "/hive:plan",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" in Claude Code", Style::default().fg(Color::White)),
            ]));
            lines.push(Line::raw(""));
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled("2. ", Style::default().fg(Color::Cyan)),
                Span::styled("Launch a drone with ", Style::default().fg(Color::White)),
                Span::styled(
                    "hive start <name>",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::raw(""));
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled("3. ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    "Monitor progress here with ",
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    "hive monitor",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::raw(""));
            lines.push(Line::raw(""));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    "Press 'n' to create a new drone",
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }

        // Use pre-computed display_order for rendering
        let now = chrono::Utc::now();
        let active_count = self
            .display_order
            .iter()
            .take_while(|&&idx| {
                let status = &self.drones[idx].1;
                if status.status != DroneState::Completed {
                    return true;
                }
                let inactive_secs = parse_timestamp(&status.updated)
                    .map(|updated| now.signed_duration_since(updated).num_seconds())
                    .unwrap_or(0);
                inactive_secs < DEFAULT_INACTIVE_THRESHOLD_SECS
            })
            .count();

        // Render ACTIVE section
        if active_count > 0 {
            lines.push(Line::from(vec![
                Span::styled(
                    "  🍯 ACTIVE",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" ({})", active_count),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
            lines.push(Line::raw(""));
        }

        let display_order = self.display_order.clone();
        for (display_idx, &drone_idx) in display_order.iter().enumerate() {
            // Add ARCHIVED header before first archived drone
            if display_idx == active_count && active_count < self.display_order.len() {
                lines.push(Line::styled(
                    "  ─────────────────────────────────────────────────────",
                    Style::default().fg(Color::DarkGray),
                ));
                lines.push(Line::raw(""));
                lines.push(Line::from(vec![
                    Span::styled(
                        "  🐻 ARCHIVED",
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(" ({})", self.display_order.len() - active_count),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
                lines.push(Line::raw(""));
            }

            let (name, status) = &self.drones[drone_idx];
            drone_line_indices.push(lines.len());

            let is_selected = display_idx == self.selected_index;
            let is_expanded = self.expanded_drones.contains(name);
            let process_running = read_drone_pid(name)
                .map(is_process_running)
                .unwrap_or(false);

            // Auto-stop: if drone is completed but process still running, kill it
            if process_running
                && status.status == DroneState::Completed
                && !self.auto_stopped_drones.contains(name)
            {
                self.auto_stopped_drones.insert(name.clone());
                let _ = crate::commands::kill_clean::kill_quiet(name.clone());
            }

            // Status icon and color
            let is_active_process = process_running || status.current_task.is_some();
            let (icon, status_color) = match status.status {
                DroneState::Starting | DroneState::Resuming => ("◐", Color::Yellow),
                DroneState::InProgress => {
                    if is_active_process {
                        ("◐", Color::Green)
                    } else {
                        ("○", Color::Yellow)
                    }
                }
                DroneState::Completed => ("●", Color::Green),
                DroneState::Error => ("◐", Color::Red),
                DroneState::Stopped => ("○", Color::DarkGray),
                DroneState::Cleaning => ("◌", Color::DarkGray),
                DroneState::Zombie => ("\u{1f480}", Color::Magenta),
            };

            // Read progress from snapshot store (single source of truth)
            let (valid_completed, task_count) = self.snapshot_store.progress(name);

            let percentage = if task_count > 0 {
                (valid_completed as f32 / task_count as f32 * 100.0) as u16
            } else {
                0
            };

            // Build progress bar (10 chars wide - compact)
            let bar_width = 10;
            let filled = (bar_width as f32 * percentage as f32 / 100.0) as usize;
            let empty = bar_width - filled;

            let (filled_bar, empty_bar) = if status.status == DroneState::Completed {
                ("━".repeat(bar_width), String::new())
            } else {
                ("━".repeat(filled), "─".repeat(empty))
            };

            let filled_color = match status.status {
                DroneState::Completed => Color::Green,
                DroneState::Error => Color::Rgb(255, 165, 0),
                _ => Color::Green,
            };

            let expand_indicator = if is_expanded { "▼" } else { "▶" };
            let select_char = if is_selected { "▸" } else { " " };

            let elapsed = TuiState::drone_elapsed(status);

            // Drone header line
            let name_style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if status.status == DroneState::Completed {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(Color::Yellow)
            };

            let mode_emoji = "🐝";
            let name_display = truncate_with_ellipsis(name, MAX_DRONE_NAME_LEN);

            // Cost display from log-based parsing
            let c = self.cost_cache.get(name).cloned().unwrap_or_default();
            let cost_usd = c.total_cost_usd;
            let cost_str = if c.total_cost_usd > 0.0 {
                format!(" ${:.2}", c.total_cost_usd)
            } else {
                String::new()
            };
            let cost_color = if cost_usd >= 5.0 {
                Color::Red
            } else if cost_usd >= 1.0 {
                Color::Yellow
            } else {
                Color::Green
            };

            // Show lead model badge if available
            let model_badge = status
                .lead_model
                .as_ref()
                .map(|m| format!(" [{}]", m))
                .unwrap_or_default();

            let header_line = Line::from(vec![
                Span::raw(format!(" {} ", select_char)),
                Span::styled(icon, Style::default().fg(status_color)),
                Span::raw(" "),
                Span::styled(expand_indicator, Style::default().fg(Color::DarkGray)),
                Span::raw(" "),
                Span::styled(format!("{} {} ", mode_emoji, name_display), name_style),
                Span::styled(filled_bar, Style::default().fg(filled_color)),
                Span::styled(empty_bar, Style::default().fg(Color::DarkGray)),
                Span::raw(" "),
                Span::styled(
                    if task_count > 0 {
                        format!("{}/{}", valid_completed, task_count)
                    } else {
                        "Planning...".to_string()
                    },
                    Style::default().fg(if task_count == 0 {
                        Color::DarkGray
                    } else {
                        Color::White
                    }),
                ),
                Span::styled(model_badge, Style::default().fg(Color::Magenta)),
                Span::styled(cost_str, Style::default().fg(cost_color)),
                Span::raw("  "),
                Span::styled(elapsed, Style::default().fg(Color::DarkGray)),
            ]);
            lines.push(header_line);

            // Expanded: show tasks
            if is_expanded {
                self.render_expanded_drone(&mut lines, drone_idx, area);
            }

            // Add separator between drones with spacing
            lines.push(Line::raw(""));
        }

        // Calculate visible area and scroll
        let content_height = chunks[1].height as usize;
        let total_lines = lines.len();

        // Ensure selected drone is visible
        if !drone_line_indices.is_empty() && self.selected_index < drone_line_indices.len() {
            let selected_line = drone_line_indices[self.selected_index];
            if selected_line < self.scroll_offset {
                self.scroll_offset = selected_line;
            } else if selected_line >= self.scroll_offset + content_height.saturating_sub(2) {
                self.scroll_offset = selected_line.saturating_sub(content_height.saturating_sub(3));
            }
        }

        // Render visible lines
        let visible_lines: Vec<Line> = lines
            .into_iter()
            .skip(self.scroll_offset)
            .take(content_height)
            .collect();

        let content = Paragraph::new(visible_lines);
        f.render_widget(content, chunks[1]);

        // Scrollbar if needed
        if total_lines > content_height {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .track_symbol(Some("│"))
                .thumb_symbol("█");

            let mut scrollbar_state = ScrollbarState::new(total_lines)
                .position(self.scroll_offset)
                .viewport_content_length(content_height);

            let scrollbar_area = Rect {
                x: chunks[1].x + chunks[1].width - 1,
                y: chunks[1].y,
                width: 1,
                height: chunks[1].height,
            };
            f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
        }

        // Footer - shortcuts (context-dependent)
        let footer_text = self.message.clone().unwrap_or_else(|| {
            " ↵ expand  t tools  m msgs  x stop  hold D clean  r resume  q quit".to_string()
        });

        let footer = Paragraph::new(Line::from(vec![Span::styled(
            footer_text,
            Style::default().fg(if self.message.is_some() {
                self.message_color
            } else {
                Color::DarkGray
            }),
        )]));
        f.render_widget(footer, chunks[2]);
    }

    fn render_expanded_drone(&mut self, lines: &mut Vec<Line>, drone_idx: usize, area: Rect) {
        let (name, status) = &self.drones[drone_idx];

        // Read task states from snapshot store (single source of truth)
        let task_list: Vec<TeamTaskInfo> = self
            .snapshot_store
            .get(name)
            .map(|s| s.tasks.clone())
            .unwrap_or_default();

        let has_tasks = !task_list.is_empty();

        if has_tasks {
            let mut tasks = task_list;
            tasks.sort_by(|a, b| a.id.cmp(&b.id));

            // Lead model from status.json
            let lead_model = status.lead_model.as_ref().map(|m| shorten_model_name(m));

            // Build agent list: prefer team config files, fallback to snapshot agents, then task owners
            let members = self
                .snapshot_store
                .get(name)
                .map(|s| s.members.clone())
                .unwrap_or_default();

            // Extract agents from snapshot (built from events.ndjson AgentSpawn/SubagentStart/Stop)
            let snapshot_agents = self
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
                    // Fallback: extract agent names from task owners
                    let mut agent_map: HashMap<String, Option<String>> = HashMap::new();
                    for t in &tasks {
                        if let Some(ref owner) = t.owner {
                            agent_map.entry(owner.clone()).or_insert(None);
                        }
                    }
                    let mut agents: Vec<(String, Option<String>)> = agent_map.into_iter().collect();
                    agents.sort_by(|a, b| a.0.cmp(&b.0));
                    agents
                }
            } else {
                Vec::new()
            };

            // Build color map: team members first, then WS-discovered agents
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

            // Team line: lead model + agents with their models
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
            if !members.is_empty() {
                team_spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));
                for (idx, m) in members.iter().enumerate() {
                    if idx > 0 {
                        team_spans.push(Span::styled(", ", Style::default().fg(Color::DarkGray)));
                    }
                    team_spans.push(Span::styled(
                        format!("@{}", m.name),
                        Style::default()
                            .fg(get_agent_color(idx))
                            .add_modifier(Modifier::BOLD),
                    ));
                    if !m.model.is_empty() {
                        team_spans.push(Span::styled(
                            format!(" ({})", shorten_model_name(&m.model)),
                            Style::default().fg(Color::DarkGray),
                        ));
                    }
                }
            } else if !ws_agents.is_empty() {
                team_spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));
                for (idx, (agent_name, agent_model)) in ws_agents.iter().enumerate() {
                    if idx > 0 {
                        team_spans.push(Span::styled(", ", Style::default().fg(Color::DarkGray)));
                    }
                    team_spans.push(Span::styled(
                        format!("@{}", agent_name),
                        Style::default()
                            .fg(get_agent_color(idx))
                            .add_modifier(Modifier::BOLD),
                    ));
                    if let Some(ref model) = agent_model {
                        team_spans.push(Span::styled(
                            format!(" ({})", shorten_model_name(model)),
                            Style::default().fg(Color::DarkGray),
                        ));
                    }
                }
            }
            lines.push(Line::from(team_spans));

            // Split tasks into user vs internal
            let user_tasks: Vec<_> = tasks.iter().filter(|t| !t.is_internal).collect();
            let internal_tasks: Vec<_> = tasks.iter().filter(|t| t.is_internal).collect();

            if !user_tasks.is_empty() {
                // Render user tasks with nested internal tasks
                let matched_agents: HashSet<_> =
                    user_tasks.iter().filter_map(|t| t.owner.as_ref()).collect();

                for task in &user_tasks {
                    render_user_task(lines, task, &member_color_map, area);

                    // Nested internal tasks for this task's agent
                    if let Some(ref agent) = task.owner {
                        for itask in &internal_tasks {
                            if itask.subject == *agent {
                                render_nested_internal(lines, itask, &member_color_map, area);
                            }
                        }
                    }
                }

                // Orphan internals (agent not assigned to any user task)
                for itask in &internal_tasks {
                    if !matched_agents.contains(&itask.subject) {
                        render_nested_internal(lines, itask, &member_color_map, area);
                    }
                }
            } else {
                // Planning phase: only internal tasks exist, render as top-level
                for task in &tasks {
                    render_user_task(lines, task, &member_color_map, area);
                }
            }
        } else {
            // No tasks yet — show last activity from log
            let log_path = PathBuf::from(".hive/drones")
                .join(name)
                .join("activity.log");
            if let Ok(contents) = std::fs::read_to_string(&log_path) {
                let last_activity = extract_last_activity(&contents);
                if !last_activity.is_empty() {
                    let prefix_len = 8; // "      ◦ "
                    let max_width = area.width as usize;
                    let activity_display = truncate_with_ellipsis(
                        &last_activity,
                        max_width.saturating_sub(prefix_len),
                    );
                    lines.push(Line::from(vec![
                        Span::raw("      "),
                        Span::styled("◦ ", Style::default().fg(Color::DarkGray)),
                        Span::styled(activity_display, Style::default().fg(Color::DarkGray)),
                    ]));
                }
            }
        }

        // Show data source indicator
        if let Some(snapshot) = self.snapshot_store.get(name) {
            if !snapshot.tasks.is_empty() {
                let (source_label, source_color) = match snapshot.source {
                    SnapshotSource::Events => ("Events", Color::Cyan),
                    SnapshotSource::Cache => ("Cache", Color::DarkGray),
                };
                lines.push(Line::from(vec![
                    Span::raw("      "),
                    Span::styled("src: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(source_label, Style::default().fg(source_color)),
                ]));
            }
        }

        // Show recent tools from hook events (if available)
        if let Some(tool_history) = self.tool_history.get(name) {
            if !tool_history.is_empty() {
                let len = tool_history.len();
                let start = len.saturating_sub(6);
                let tools_summary: String = tool_history
                    .iter()
                    .skip(start)
                    .map(|r| r.tool.as_str())
                    .collect::<Vec<_>>()
                    .join(" → ");
                let prefix_len = 10; // "      🔧 "
                let max_width = area.width as usize;
                let tools_display =
                    truncate_with_ellipsis(&tools_summary, max_width.saturating_sub(prefix_len));
                lines.push(Line::from(vec![
                    Span::raw("      "),
                    Span::styled("🔧 ", Style::default().fg(Color::Cyan)),
                    Span::styled(tools_display, Style::default().fg(Color::DarkGray)),
                ]));
            }
        }

        // Show cost details in expanded view (from log parsing)
        let c = self.cost_cache.get(name).cloned().unwrap_or_default();
        if c.total_cost_usd > 0.0 {
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

        // Show last event from hooks (if available)
        if let Some(event) = self.last_events.get(name) {
            let event_desc = match event {
                HiveEvent::TaskCreate { subject, .. } => {
                    format!("Created: {}", subject)
                }
                HiveEvent::TaskUpdate {
                    task_id, status, ..
                } => {
                    format!("Task {} → {}", task_id, status)
                }
                HiveEvent::Message {
                    recipient, summary, ..
                } => {
                    format!("Msg → @{}: {}", recipient, summary)
                }
                HiveEvent::TaskDone { subject, agent, .. } => {
                    format!(
                        "Done: {}{}",
                        subject,
                        agent
                            .as_ref()
                            .map(|a| format!(" @{}", a))
                            .unwrap_or_default()
                    )
                }
                HiveEvent::Idle { agent, .. } => format!("@{} idle", agent),
                HiveEvent::Stop { .. } => "Stopped".to_string(),
                HiveEvent::Start { model, .. } => {
                    format!("Started ({})", model)
                }
                HiveEvent::AgentSpawn { name, model, .. } => {
                    format!(
                        "Agent: {}{}",
                        name,
                        model
                            .as_ref()
                            .map(|m| format!(" ({})", m))
                            .unwrap_or_default()
                    )
                }
                HiveEvent::SubagentStart { agent_type, .. } => {
                    format!(
                        "Subagent started{}",
                        agent_type
                            .as_ref()
                            .map(|t| format!(" ({})", t))
                            .unwrap_or_default()
                    )
                }
                HiveEvent::SubagentStop { agent_type, .. } => {
                    format!(
                        "Subagent stopped{}",
                        agent_type
                            .as_ref()
                            .map(|t| format!(" ({})", t))
                            .unwrap_or_default()
                    )
                }
                HiveEvent::ToolDone { tool, .. } => {
                    format!("Tool: {}", tool)
                }
                HiveEvent::TodoSnapshot { todos, .. } => {
                    let done = todos.iter().filter(|t| t.status == "completed").count();
                    format!("Todos: {}/{}", done, todos.len())
                }
            };
            let prefix_len = 8; // "      ⚡ "
            let max_width = area.width as usize;
            let event_display =
                truncate_with_ellipsis(&event_desc, max_width.saturating_sub(prefix_len));
            lines.push(Line::from(vec![
                Span::raw("      "),
                Span::styled("⚡ ", Style::default().fg(Color::Cyan)),
                Span::styled(event_display, Style::default().fg(Color::DarkGray)),
            ]));
        }
    }
}

/// Get the icon, icon color, and text color for a task based on its status.
fn task_status_style(status: &str) -> (&'static str, Color, Color) {
    match status {
        "completed" => ("●", Color::Green, Color::DarkGray),
        "in_progress" => ("◐", Color::Yellow, Color::White),
        _ => ("○", Color::DarkGray, Color::White),
    }
}

/// Extract a meaningful title from an internal task's description.
/// Internal tasks have the agent name as `subject` and the actual work in `description`.
fn extract_internal_task_title(task: &TeamTaskInfo) -> String {
    let title = task
        .description
        .lines()
        .find(|l| l.starts_with("Your task:") || l.starts_with("Your tasks:"))
        .map(|l| {
            l.trim_start_matches("Your task:")
                .trim_start_matches("Your tasks:")
                .trim()
                .to_string()
        })
        .filter(|s| !s.is_empty())
        .or_else(|| {
            task.description
                .lines()
                .find(|l| {
                    !l.is_empty() && !l.starts_with("You are") && !l.starts_with("Check the task")
                })
                .map(|l| l.to_string())
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| task.subject.clone());
    // Strip markdown header prefixes (e.g. "## Title" → "Title")
    title.trim_start_matches('#').trim().to_string()
}

/// Render a top-level task line (user task or internal task shown as top-level during planning).
fn render_user_task(
    lines: &mut Vec<Line>,
    task: &TeamTaskInfo,
    member_color_map: &HashMap<String, usize>,
    area: Rect,
) {
    let (task_icon, icon_color, text_color) = task_status_style(&task.status);

    let (title, agent_name) = if task.is_internal {
        (
            extract_internal_task_title(task),
            Some(task.subject.clone()),
        )
    } else {
        let subj = task.subject.trim_start_matches('#').trim().to_string();
        (subj, task.owner.clone())
    };

    let agent_badge_with_color = agent_name.map(|a| {
        let badge_text = format!(" @{}", a);
        let agent_color = member_color_map
            .get(&a)
            .map(|&idx| get_agent_color(idx))
            .unwrap_or(Color::Cyan);
        (badge_text, agent_color)
    });

    // For completed tasks, show duration instead of active_form
    let timing_suffix = if task.status == "completed" {
        task.created_at
            .zip(task.updated_at)
            .map(|(created, updated)| {
                let duration_ms = updated.saturating_sub(created);
                let duration = chrono::Duration::milliseconds(duration_ms as i64);
                format!(" ({})", format_duration(duration))
            })
            .unwrap_or_default()
    } else {
        String::new()
    };

    let active_form = if !timing_suffix.is_empty() {
        timing_suffix
    } else {
        task.active_form
            .as_ref()
            .map(|f| format!(" ({})", f))
            .unwrap_or_default()
    };

    let task_prefix_len = 8;
    let badge_len = agent_badge_with_color
        .as_ref()
        .map(|(text, _)| text.len())
        .unwrap_or(0)
        + active_form.len();
    let task_available_width = area.width as usize;
    let max_task_title_width = task_available_width.saturating_sub(task_prefix_len + badge_len + 1);

    if title.chars().count() <= max_task_title_width || max_task_title_width < 20 {
        let mut spans = vec![
            Span::raw("      "),
            Span::styled(task_icon, Style::default().fg(icon_color)),
            Span::raw(" "),
            Span::styled(title, Style::default().fg(text_color)),
            Span::styled(active_form.clone(), Style::default().fg(Color::DarkGray)),
        ];
        if let Some((badge_text, badge_color)) = agent_badge_with_color.as_ref() {
            spans.push(Span::styled(
                badge_text.clone(),
                Style::default()
                    .fg(*badge_color)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        lines.push(Line::from(spans));
    } else {
        let task_title_indent = "        "; // 8 spaces
        let wrap_width = task_available_width.saturating_sub(task_prefix_len + 1);
        let last_line_wrap_width = wrap_width.saturating_sub(badge_len);
        let mut remaining = title.as_str();
        let mut first_line = true;

        while !remaining.is_empty() {
            let char_count = remaining.chars().count();
            let is_last = char_count <= last_line_wrap_width;
            let (chunk, rest) = if is_last {
                (remaining, "")
            } else {
                let split_width = if char_count <= wrap_width {
                    last_line_wrap_width
                } else {
                    wrap_width
                };
                let byte_limit: usize = remaining
                    .char_indices()
                    .nth(split_width)
                    .map(|(i, _)| i)
                    .unwrap_or(remaining.len());
                let break_at = remaining[..byte_limit].rfind(' ').unwrap_or(byte_limit);
                (&remaining[..break_at], remaining[break_at..].trim_start())
            };

            if first_line {
                let mut spans = vec![
                    Span::raw("      "),
                    Span::styled(task_icon, Style::default().fg(icon_color)),
                    Span::raw(" "),
                    Span::styled(chunk.to_string(), Style::default().fg(text_color)),
                ];
                if is_last {
                    spans.push(Span::styled(
                        active_form.clone(),
                        Style::default().fg(Color::DarkGray),
                    ));
                    if let Some((badge_text, badge_color)) = agent_badge_with_color.as_ref() {
                        spans.push(Span::styled(
                            badge_text.clone(),
                            Style::default().fg(*badge_color),
                        ));
                    }
                }
                lines.push(Line::from(spans));
                first_line = false;
            } else {
                let mut spans = vec![
                    Span::raw(task_title_indent),
                    Span::styled(chunk.to_string(), Style::default().fg(text_color)),
                ];
                if is_last {
                    spans.push(Span::styled(
                        active_form.clone(),
                        Style::default().fg(Color::DarkGray),
                    ));
                    if let Some((badge_text, badge_color)) = agent_badge_with_color.as_ref() {
                        spans.push(Span::styled(
                            badge_text.clone(),
                            Style::default().fg(*badge_color),
                        ));
                    }
                }
                lines.push(Line::from(spans));
            }
            remaining = rest;
        }
    }
}

/// Render an internal task nested beneath its parent user task.
/// Format: `        └─ ◐ @agent: Task description`
fn render_nested_internal(
    lines: &mut Vec<Line>,
    task: &TeamTaskInfo,
    member_color_map: &HashMap<String, usize>,
    area: Rect,
) {
    let (task_icon, icon_color, text_color) = task_status_style(&task.status);

    let agent_name = &task.subject;
    let agent_color = member_color_map
        .get(agent_name)
        .map(|&idx| get_agent_color(idx))
        .unwrap_or(Color::Cyan);

    let title = extract_internal_task_title(task);

    // Prefix: "        └─ " (8 spaces + tree connector)
    let prefix = "        └─ ";
    let agent_badge = format!("@{}: ", agent_name);
    let prefix_len = prefix.len() + 2 + agent_badge.len(); // icon + space + badge
    let max_width = area.width as usize;
    let title_display = truncate_with_ellipsis(&title, max_width.saturating_sub(prefix_len));

    lines.push(Line::from(vec![
        Span::styled("        └─ ", Style::default().fg(Color::DarkGray)),
        Span::styled(task_icon, Style::default().fg(icon_color)),
        Span::raw(" "),
        Span::styled(
            agent_badge,
            Style::default()
                .fg(agent_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(title_display, Style::default().fg(text_color)),
    ]));
}

/// Shorten a Claude model name for compact display.
/// e.g. "claude-sonnet-4-5-20250929" → "sonnet-4.5"
///      "claude-opus-4-6" → "opus-4.6"
///      "claude-haiku-4-5-20251001" → "haiku-4.5"
fn shorten_model_name(model: &str) -> String {
    // Strip "claude-" prefix
    let name = model.strip_prefix("claude-").unwrap_or(model);

    // Try to extract family and version: "sonnet-4-5-20250929" → "sonnet", "4", "5"
    let parts: Vec<&str> = name.split('-').collect();
    if parts.len() >= 3 {
        let family = parts[0]; // sonnet, opus, haiku
                               // Check if parts[1] and parts[2] are version digits
        if parts[1].chars().all(|c| c.is_ascii_digit())
            && parts[2].chars().all(|c| c.is_ascii_digit())
        {
            return format!("{}-{}.{}", family, parts[1], parts[2]);
        }
    }

    // Fallback: just use the stripped name, truncated
    if name.len() > 15 {
        name[..15].to_string()
    } else {
        name.to_string()
    }
}
