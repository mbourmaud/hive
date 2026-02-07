use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use crate::agent_teams::task_sync;
use crate::commands::common::{
    is_process_running, parse_timestamp, read_drone_pid,
    truncate_with_ellipsis, DEFAULT_INACTIVE_THRESHOLD_SECS, MAX_DRONE_NAME_LEN,
};
use crate::events::HiveEvent;
use crate::types::DroneState;

use super::cost::format_token_count;
use super::drone_actions::extract_last_activity;
use super::state::TuiState;
use super::views::render_messages_view;

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

        // Messages view (toggle with 'm')
        if let Some(ref drone_name) = self.messages_view {
            render_messages_view(f, area, drone_name, self.messages_scroll);
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
                Span::styled("Create a PRD with ", Style::default().fg(Color::White)),
                Span::styled(
                    "/hive:prd",
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
            let is_active_process = process_running || status.current_story.is_some();
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

            // Plan mode: count from Agent Teams tasks
            let (valid_completed, prd_story_count) = task_sync::read_team_task_states(name)
                .map(|tasks| {
                    let completed = tasks.values().filter(|t| t.status == "completed").count();
                    (completed, tasks.len())
                })
                .unwrap_or((0, 0));

            let percentage = if prd_story_count > 0 {
                (valid_completed as f32 / prd_story_count as f32 * 100.0) as u16
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

            // Teammate count for header
            let member_count = task_sync::read_team_members(name)
                .map(|m| m.len())
                .unwrap_or(0);

            // Cost display
            let cost = self.cost_cache.get(name).cloned().unwrap_or_default();
            let cost_str = if cost.total_cost_usd > 0.0 {
                format!(" ${:.2}", cost.total_cost_usd)
            } else {
                String::new()
            };
            let cost_color = if cost.total_cost_usd >= 5.0 {
                Color::Red
            } else if cost.total_cost_usd >= 1.0 {
                Color::Yellow
            } else {
                Color::Green
            };

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
                    if prd_story_count > 0 {
                        format!("{}/{}", valid_completed, prd_story_count)
                    } else {
                        "Planning...".to_string()
                    },
                    Style::default().fg(if prd_story_count == 0 {
                        Color::DarkGray
                    } else {
                        Color::White
                    }),
                ),
                Span::styled(cost_str, Style::default().fg(cost_color)),
                if member_count > 0 {
                    Span::styled(
                        format!("  🤖{}", member_count),
                        Style::default().fg(Color::DarkGray),
                    )
                } else {
                    Span::raw("")
                },
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
                self.scroll_offset =
                    selected_line.saturating_sub(content_height.saturating_sub(3));
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
        let footer_text = if let Some(msg) = &self.message {
            msg.clone()
        } else {
            " ↵ expand  m msgs  x stop  D clean  r resume  q quit".to_string()
        };

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

    fn render_expanded_drone(
        &mut self,
        lines: &mut Vec<Line>,
        drone_idx: usize,
        area: Rect,
    ) {
        let (name, _status) = &self.drones[drone_idx];

        // Read task states for Agent Teams rendering
        let task_states_result = task_sync::read_team_task_states(name);
        let has_tasks = task_states_result
            .as_ref()
            .map(|t| !t.is_empty())
            .unwrap_or(false);

        if has_tasks {
            let task_states = task_states_result.as_ref().unwrap();
            let mut tasks: Vec<_> = task_states.values().cloned().collect();
            tasks.sort_by(|a, b| a.id.cmp(&b.id));

            // Build agent index map for tasks.
            // For internal tasks, the agent name is in `subject`; for work tasks, it's in `owner`.
            let mut task_unique_agents: Vec<String> = tasks
                .iter()
                .filter_map(|t| {
                    if t.is_internal {
                        Some(t.subject.clone())
                    } else {
                        t.owner.clone()
                    }
                })
                .collect();
            task_unique_agents.sort();
            task_unique_agents.dedup();
            let task_agent_index_map: HashMap<String, usize> = task_unique_agents
                .iter()
                .enumerate()
                .map(|(idx, agent)| (agent.clone(), idx))
                .collect();

            // Show team members summary
            let members = task_sync::read_team_members(name).unwrap_or_default();
            if !members.is_empty() {
                let mut member_spans: Vec<Span> = vec![
                    Span::raw("      "),
                    Span::styled("Team: ", Style::default().fg(Color::DarkGray)),
                ];
                for (idx, m) in members.iter().enumerate() {
                    if idx > 0 {
                        member_spans
                            .push(Span::styled(", ", Style::default().fg(Color::DarkGray)));
                    }
                    member_spans.push(Span::styled(
                        m.name.clone(),
                        Style::default().fg(get_agent_color(idx)),
                    ));
                }
                lines.push(Line::from(member_spans));
            }

            for task in &tasks {
                let (task_icon, task_color) = if task.status == "completed" {
                    ("●", Color::Green)
                } else if task.status == "in_progress" {
                    ("◐", Color::DarkGray)
                } else {
                    ("○", Color::DarkGray)
                };

                // For internal teammate tasks: subject is the agent name, description is the work.
                // For work tasks: subject is the title, owner is the agent name.
                let (title, agent_name) = if task.is_internal {
                    // Extract meaningful task description, skipping boilerplate.
                    // Descriptions are often truncated (~100 chars), so fall back
                    // to the agent name (subject) when nothing useful is found.
                    let task_line = task
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
                                    !l.is_empty()
                                        && !l.starts_with("You are")
                                        && !l.starts_with("Check the task")
                                })
                                .map(|l| l.to_string())
                        })
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| task.subject.clone());
                    (task_line, Some(task.subject.clone()))
                } else {
                    (task.subject.clone(), task.owner.clone())
                };

                let agent_badge_with_color = agent_name.map(|a| {
                    let badge_text = format!(" @{}", a);
                    let agent_color = task_agent_index_map
                        .get(&a)
                        .map(|&idx| get_agent_color(idx))
                        .unwrap_or(Color::Cyan);
                    (badge_text, agent_color)
                });

                let active_form = task
                    .active_form
                    .as_ref()
                    .map(|f| format!(" ({})", f))
                    .unwrap_or_default();

                // Track and display elapsed time for in_progress tasks
                let task_key = (name.to_string(), task.id.clone());
                if task.status == "in_progress" {
                    self.task_start_times
                        .entry(task_key.clone())
                        .or_insert_with(Instant::now);
                }
                let elapsed_str = if task.status == "in_progress" {
                    self.task_start_times
                        .get(&task_key)
                        .map(|start| {
                            let secs = start.elapsed().as_secs();
                            if secs < 60 {
                                format!(" {}s", secs)
                            } else {
                                format!(" {}m{}s", secs / 60, secs % 60)
                            }
                        })
                        .unwrap_or_default()
                } else {
                    String::new()
                };

                let task_prefix_len = 8;
                let badge_len = agent_badge_with_color
                    .as_ref()
                    .map(|(text, _)| text.len())
                    .unwrap_or(0)
                    + active_form.len()
                    + elapsed_str.len();
                let task_available_width = area.width as usize;
                let max_task_title_width =
                    task_available_width.saturating_sub(task_prefix_len + badge_len + 1);

                if title.chars().count() <= max_task_title_width || max_task_title_width < 20 {
                    let mut spans = vec![
                        Span::raw("      "),
                        Span::styled(task_icon, Style::default().fg(task_color)),
                        Span::raw(" "),
                        Span::styled(title, Style::default().fg(task_color)),
                        Span::styled(
                            active_form.clone(),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ];
                    if let Some((badge_text, badge_color)) = agent_badge_with_color.as_ref() {
                        spans.push(Span::styled(
                            badge_text.clone(),
                            Style::default().fg(*badge_color),
                        ));
                    }
                    if !elapsed_str.is_empty() {
                        spans.push(Span::styled(
                            elapsed_str.clone(),
                            Style::default().fg(Color::DarkGray),
                        ));
                    }
                    lines.push(Line::from(spans));
                } else {
                    let task_title_indent = "        "; // 8 spaces
                    let wrap_width =
                        task_available_width.saturating_sub(task_prefix_len + 1);
                    let mut remaining = title.as_str();
                    let mut first_line = true;

                    while !remaining.is_empty() {
                        let char_count = remaining.chars().count();
                        let is_last = char_count <= wrap_width;
                        let (chunk, rest) = if is_last {
                            (remaining, "")
                        } else {
                            let byte_limit: usize = remaining
                                .char_indices()
                                .nth(wrap_width)
                                .map(|(i, _)| i)
                                .unwrap_or(remaining.len());
                            let break_at = remaining[..byte_limit]
                                .rfind(' ')
                                .unwrap_or(byte_limit);
                            (
                                &remaining[..break_at],
                                remaining[break_at..].trim_start(),
                            )
                        };

                        if first_line {
                            let mut spans = vec![
                                Span::raw("      "),
                                Span::styled(task_icon, Style::default().fg(task_color)),
                                Span::raw(" "),
                                Span::styled(
                                    chunk.to_string(),
                                    Style::default().fg(task_color),
                                ),
                            ];
                            if is_last {
                                spans.push(Span::styled(
                                    active_form.clone(),
                                    Style::default().fg(Color::DarkGray),
                                ));
                                if let Some((badge_text, badge_color)) =
                                    agent_badge_with_color.as_ref()
                                {
                                    spans.push(Span::styled(
                                        badge_text.clone(),
                                        Style::default().fg(*badge_color),
                                    ));
                                }
                                if !elapsed_str.is_empty() {
                                    spans.push(Span::styled(
                                        elapsed_str.clone(),
                                        Style::default().fg(Color::DarkGray),
                                    ));
                                }
                            }
                            lines.push(Line::from(spans));
                            first_line = false;
                        } else {
                            let mut spans = vec![
                                Span::raw(task_title_indent),
                                Span::styled(
                                    chunk.to_string(),
                                    Style::default().fg(task_color),
                                ),
                            ];
                            if is_last {
                                spans.push(Span::styled(
                                    active_form.clone(),
                                    Style::default().fg(Color::DarkGray),
                                ));
                                if let Some((badge_text, badge_color)) =
                                    agent_badge_with_color.as_ref()
                                {
                                    spans.push(Span::styled(
                                        badge_text.clone(),
                                        Style::default().fg(*badge_color),
                                    ));
                                }
                                if !elapsed_str.is_empty() {
                                    spans.push(Span::styled(
                                        elapsed_str.clone(),
                                        Style::default().fg(Color::DarkGray),
                                    ));
                                }
                            }
                            lines.push(Line::from(spans));
                        }
                        remaining = rest;
                    }
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

        // Show cost details in expanded view
        let cost = self.cost_cache.get(name).cloned().unwrap_or_default();
        if cost.total_cost_usd > 0.0 {
            let cost_detail = format!(
                "      Cost: ${:.2} | In: {} | Out: {}{}",
                cost.total_cost_usd,
                format_token_count(cost.input_tokens),
                format_token_count(cost.output_tokens),
                if cost.cache_read_tokens > 0 || cost.cache_creation_tokens > 0 {
                    format!(
                        " | Cache: {} read, {} created",
                        format_token_count(cost.cache_read_tokens),
                        format_token_count(cost.cache_creation_tokens)
                    )
                } else {
                    String::new()
                }
            );
            lines.push(Line::from(vec![Span::styled(
                cost_detail,
                Style::default().fg(Color::DarkGray),
            )]));
        }

        // Show last event from hooks (if available)
        if let Some(ref event) = self.last_events.get(name) {
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
                    format!("Msg → {}: {}", recipient, summary)
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
