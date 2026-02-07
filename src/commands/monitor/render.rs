use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::agent_teams::task_sync;
use crate::commands::common::{
    duration_between, elapsed_since, format_duration, is_process_running, parse_timestamp,
    read_drone_pid, reconcile_progress_with_prd, truncate_with_ellipsis,
    DEFAULT_INACTIVE_THRESHOLD_SECS, MAX_DRONE_NAME_LEN,
};
use crate::events::HiveEvent;
use crate::types::DroneState;

use super::cost::format_token_count;
use super::drone_actions::{extract_last_activity, extract_task_title};
use super::sparkline::{get_sparkline_data, render_sparkline};
use super::state::TuiState;
use super::views::{render_blocked_detail_view, render_timeline_view};
use super::ViewMode;

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

        // Timeline view (toggle with 't')
        if self.view_mode == ViewMode::Timeline {
            render_timeline_view(f, area, &self.drones, &self.prd_cache, self.timeline_scroll);
            return;
        }

        // Check if we're showing the blocked detail view
        if let Some(ref blocked_drone_name) = self.blocked_view {
            if let Some((_, status)) = self
                .drones
                .iter()
                .find(|(name, _)| name == blocked_drone_name)
            {
                render_blocked_detail_view(f, area, blocked_drone_name, status, &self.prd_cache);
                return;
            }
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
                let (valid_completed, prd_story_count) = self
                    .prd_cache
                    .get(&status.prd)
                    .map(|prd| reconcile_progress_with_prd(status, prd))
                    .unwrap_or((status.completed.len(), status.total));
                if valid_completed < prd_story_count {
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

        for (display_idx, &drone_idx) in self.display_order.iter().enumerate() {
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

            let _is_archived = display_idx >= active_count;
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
                DroneState::Blocked => ("◐", Color::Red),
                DroneState::Stopped => ("○", Color::DarkGray),
                DroneState::Cleaning => ("◌", Color::DarkGray),
            };

            // Use reconciled progress to filter out old completed stories
            let (valid_completed, prd_story_count) = self
                .prd_cache
                .get(&status.prd)
                .map(|prd| reconcile_progress_with_prd(status, prd))
                .unwrap_or((status.completed.len(), status.total));
            let has_new_stories = prd_story_count > status.total;

            let percentage = if prd_story_count > 0 {
                (valid_completed as f32 / prd_story_count as f32 * 100.0) as u16
            } else {
                0
            };

            // Build progress bar (10 chars wide - compact)
            let bar_width = 10;
            let filled = (bar_width as f32 * percentage as f32 / 100.0) as usize;
            let empty = bar_width - filled;

            let (filled_bar, empty_bar) =
                if status.status == DroneState::Completed && !has_new_stories {
                    ("━".repeat(bar_width), String::new())
                } else {
                    ("━".repeat(filled), "─".repeat(empty))
                };

            let filled_color = match status.status {
                DroneState::Completed => Color::Green,
                DroneState::Blocked | DroneState::Error => Color::Rgb(255, 165, 0),
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

            // Check inbox for pending messages
            let inbox_count = task_sync::read_team_inboxes(name)
                .map(|inboxes| {
                    inboxes
                        .values()
                        .flat_map(|v| v.iter())
                        .filter(|m| !m.read)
                        .count()
                })
                .unwrap_or(0);
            let inbox_indicator = if inbox_count > 0 {
                format!(" ✉{}", inbox_count)
            } else {
                String::new()
            };

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

            // Activity sparkline
            let sparkline_data = get_sparkline_data(&self.activity_history, name);
            let sparkline_str = render_sparkline(&sparkline_data);
            let has_recent_activity = sparkline_data.iter().rev().take(2).any(|&v| v > 0);
            let sparkline_color = if has_recent_activity {
                Color::Cyan
            } else {
                Color::DarkGray
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
                    Style::default().fg(
                        if prd_story_count == 0
                            || (status.status == DroneState::Completed && !has_new_stories)
                        {
                            Color::DarkGray
                        } else if has_new_stories {
                            Color::Cyan
                        } else {
                            Color::White
                        },
                    ),
                ),
                Span::styled(cost_str, Style::default().fg(cost_color)),
                Span::styled(inbox_indicator.clone(), Style::default().fg(Color::Cyan)),
                if member_count > 0 {
                    Span::styled(
                        format!("  🤖{}", member_count),
                        Style::default().fg(Color::DarkGray),
                    )
                } else {
                    Span::raw("")
                },
                Span::raw("  "),
                Span::styled(sparkline_str, Style::default().fg(sparkline_color)),
                Span::raw("  "),
                Span::styled(elapsed, Style::default().fg(Color::DarkGray)),
            ]);
            lines.push(header_line);

            // Expanded: show stories
            if is_expanded {
                self.render_expanded_drone(
                    &mut lines,
                    drone_idx,
                    area,
                    has_new_stories,
                    prd_story_count,
                );
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
        } else if self.selected_story_index.is_some() {
            " i info  ↑↓ navigate  ← back  q back".to_string()
        } else {
            " ↵ expand  t timeline  b blocked  x stop  D clean  q quit".to_string()
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
        &self,
        lines: &mut Vec<Line>,
        drone_idx: usize,
        area: Rect,
        has_new_stories: bool,
        prd_story_count: usize,
    ) {
        let (name, status) = &self.drones[drone_idx];

        // Build maps of story_id -> agent_name and story_id -> model
        let task_states_result = task_sync::read_team_task_states(name);
        let (agent_map, model_map): (HashMap<String, String>, HashMap<String, String>) = {
            let mut map = HashMap::new();
            let mut mmap = HashMap::new();

            // Method 1: from task states (storyId in metadata)
            if let Ok(ref task_states) = task_states_result {
                for t in task_states.values() {
                    if t.status != "in_progress" {
                        continue;
                    }
                    if let Some(ref story_id) = t.story_id {
                        if let Some(ref owner) = t.owner {
                            map.insert(story_id.clone(), owner.clone());
                        }
                        if let Some(ref model) = t.model {
                            mmap.insert(story_id.clone(), model.clone());
                        }
                    }
                }
            }

            // Method 2: from teammate names matching story IDs
            if map.is_empty() {
                if let Ok(ref task_states) = task_states_result {
                    for t in task_states.values() {
                        if t.status != "in_progress" {
                            continue;
                        }
                        let subj = t.subject.to_lowercase();
                        if let Some(prd) = self.prd_cache.get(&status.prd) {
                            for story in &prd.stories {
                                let sid = story.id.to_lowercase().replace("-", "");
                                if subj.contains(&sid) {
                                    map.insert(story.id.clone(), t.subject.clone());
                                    if let Some(ref model) = t.model {
                                        mmap.insert(story.id.clone(), model.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Method 3: from active_agents in status.json
            for (agent, story_id) in &status.active_agents {
                map.entry(story_id.clone())
                    .or_insert_with(|| agent.clone());
            }

            (map, mmap)
        };

        // Build agent index map for unique colors
        let mut unique_agents: Vec<String> = agent_map.values().cloned().collect();
        unique_agents.sort();
        unique_agents.dedup();
        let agent_index_map: HashMap<String, usize> = unique_agents
            .iter()
            .enumerate()
            .map(|(idx, agent)| (agent.clone(), idx))
            .collect();

        if let Some(prd) = self.prd_cache.get(&status.prd) {
            for (story_idx, story) in prd.stories.iter().enumerate() {
                let is_completed = status.completed.contains(&story.id);
                let is_current = status.current_story.as_ref() == Some(&story.id);
                let is_agent_active = agent_map.contains_key(&story.id);
                let is_story_selected = self.display_order.iter().position(|&i| i == drone_idx)
                    .map(|di| di == self.selected_index)
                    .unwrap_or(false)
                    && self.selected_story_index == Some(story_idx);

                let has_blocked_deps =
                    if !story.depends_on.is_empty() && !is_completed {
                        story
                            .depends_on
                            .iter()
                            .any(|dep_id| !status.completed.contains(dep_id))
                    } else {
                        false
                    };

                let (story_icon, story_color) = if is_story_selected {
                    ("▸", Color::Cyan)
                } else if is_completed {
                    ("●", Color::Green)
                } else if has_blocked_deps {
                    ("⏳", Color::DarkGray)
                } else if is_current || is_agent_active {
                    ("◐", Color::DarkGray)
                } else {
                    ("○", Color::DarkGray)
                };

                let dep_info = if has_blocked_deps {
                    " 🔒".to_string()
                } else {
                    String::new()
                };

                let duration_str = if let Some(timing) = status.story_times.get(&story.id) {
                    if let (Some(started), Some(completed)) =
                        (&timing.started, &timing.completed)
                    {
                        if let Some(dur) = duration_between(started, completed) {
                            format!(" {}", format_duration(dur))
                        } else {
                            String::new()
                        }
                    } else if let Some(started) = &timing.started {
                        if let Some(elapsed) = elapsed_since(started) {
                            format!(" {}", elapsed)
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };

                let line_style = if is_story_selected {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let title_color = if is_story_selected {
                    Color::Cyan
                } else {
                    story_color
                };

                let prefix_len = 26;
                let duration_len = duration_str.len();
                let available_width = area.width as usize;
                let max_title_width =
                    available_width.saturating_sub(prefix_len + duration_len + 2);

                let agent_badge_with_color = agent_map.get(&story.id).map(|a| {
                    let model_str = model_map
                        .get(&story.id)
                        .map(|m| format!(" {}", m))
                        .unwrap_or_default();
                    let badge_text = format!(" @{}{}", a, model_str);
                    let agent_color = agent_index_map
                        .get(a)
                        .map(|&idx| get_agent_color(idx))
                        .unwrap_or(Color::Cyan);
                    (badge_text, agent_color)
                });

                if story.title.len() <= max_title_width || max_title_width < 20 {
                    let mut spans = vec![
                        Span::styled("      ", line_style),
                        Span::styled(story_icon, line_style.fg(story_color)),
                        Span::raw(" "),
                        Span::styled(
                            format!("{:<16} ", story.id),
                            line_style.fg(title_color),
                        ),
                        Span::styled(story.title.clone(), line_style.fg(title_color)),
                        Span::styled(
                            duration_str.clone(),
                            line_style.fg(Color::DarkGray),
                        ),
                    ];
                    if let Some((ref badge, color)) = agent_badge_with_color {
                        spans.push(Span::styled(
                            badge.clone(),
                            Style::default().fg(color),
                        ));
                    }
                    if !dep_info.is_empty() {
                        spans.push(Span::styled(
                            dep_info.clone(),
                            Style::default().fg(Color::Yellow),
                        ));
                    }
                    lines.push(Line::from(spans));
                } else {
                    let title_indent = "                         "; // 25 spaces
                    let mut remaining = story.title.as_str();
                    let mut first_line = true;

                    while !remaining.is_empty() {
                        let char_count = remaining.chars().count();
                        let (chunk, rest) = if char_count <= max_title_width {
                            (remaining, "")
                        } else {
                            let byte_limit: usize = remaining
                                .char_indices()
                                .nth(max_title_width)
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
                            lines.push(Line::from(vec![
                                Span::styled("      ", line_style),
                                Span::styled(story_icon, line_style.fg(story_color)),
                                Span::raw(" "),
                                Span::styled(
                                    format!("{:<16} ", story.id),
                                    line_style.fg(title_color),
                                ),
                                Span::styled(chunk.to_string(), line_style.fg(title_color)),
                                if rest.is_empty() {
                                    Span::styled(
                                        duration_str.clone(),
                                        line_style.fg(Color::DarkGray),
                                    )
                                } else {
                                    Span::raw("")
                                },
                            ]));
                            first_line = false;
                        } else {
                            lines.push(Line::from(vec![
                                Span::styled(title_indent, line_style),
                                Span::styled(chunk.to_string(), line_style.fg(title_color)),
                                if rest.is_empty() {
                                    Span::styled(
                                        duration_str.clone(),
                                        line_style.fg(Color::DarkGray),
                                    )
                                } else {
                                    Span::raw("")
                                },
                            ]));
                        }
                        remaining = rest;
                    }
                }
            }
        }

        // For plan-only PRDs (no stories), show Agent Teams tasks instead
        let prd_has_stories = self
            .prd_cache
            .get(&status.prd)
            .map(|p| !p.stories.is_empty())
            .unwrap_or(false);
        if !prd_has_stories {
            let has_tasks = task_states_result
                .as_ref()
                .map(|t| !t.is_empty())
                .unwrap_or(false);

            if has_tasks {
                let task_states = task_states_result.as_ref().unwrap();
                let mut tasks: Vec<_> = task_states.values().collect();
                tasks.sort_by(|a, b| a.id.cmp(&b.id));

                // Build agent index map for tasks
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

                for task in &tasks {
                    let (task_icon, task_color) = if task.status == "completed" {
                        ("●", Color::Green)
                    } else if task.status == "in_progress" {
                        ("◐", Color::DarkGray)
                    } else {
                        ("○", Color::DarkGray)
                    };

                    let (title, agent_name) = if task.is_internal {
                        let title = extract_task_title(&task.description);
                        (title, Some(task.subject.clone()))
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

                    let task_prefix_len = 8;
                    let badge_len = agent_badge_with_color
                        .as_ref()
                        .map(|(text, _)| text.len())
                        .unwrap_or(0)
                        + active_form.len();
                    let task_available_width = area.width as usize;
                    let max_task_title_width =
                        task_available_width.saturating_sub(task_prefix_len + badge_len + 1);

                    if title.chars().count() <= max_task_title_width
                        || max_task_title_width < 20
                    {
                        let mut spans = vec![
                            Span::raw("      "),
                            Span::styled(task_icon, Style::default().fg(task_color)),
                            Span::raw(" "),
                            Span::styled(title, Style::default().fg(task_color)),
                            Span::styled(active_form.clone(), Style::default().fg(Color::DarkGray)),
                        ];
                        if let Some((badge_text, badge_color)) = agent_badge_with_color.as_ref() {
                            spans.push(Span::styled(
                                badge_text.clone(),
                                Style::default().fg(*badge_color),
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
                                    Span::styled(
                                        task_icon,
                                        Style::default().fg(task_color),
                                    ),
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
                                }
                                lines.push(Line::from(spans));
                            }
                            remaining = rest;
                        }
                    }
                }
            } else if status.total > 0 {
                for story_id in &status.completed {
                    let timing = status.story_times.get(story_id);
                    let duration_str = timing
                        .and_then(|t| {
                            let started = t.started.as_deref()?;
                            let completed = t.completed.as_deref()?;
                            let dur = duration_between(started, completed)?;
                            Some(format!(" {}", format_duration(dur)))
                        })
                        .unwrap_or_default();

                    lines.push(Line::from(vec![
                        Span::raw("      "),
                        Span::styled("● ", Style::default().fg(Color::Green)),
                        Span::styled(story_id.clone(), Style::default().fg(Color::Green)),
                        Span::styled(duration_str, Style::default().fg(Color::DarkGray)),
                    ]));
                }
            } else {
                let log_path = PathBuf::from(".hive/drones")
                    .join(name)
                    .join("activity.log");
                if let Ok(contents) = std::fs::read_to_string(&log_path) {
                    let last_activity = extract_last_activity(&contents);
                    if !last_activity.is_empty() {
                        lines.push(Line::from(vec![
                            Span::raw("      "),
                            Span::styled("◦ ", Style::default().fg(Color::DarkGray)),
                            Span::styled(last_activity, Style::default().fg(Color::DarkGray)),
                        ]));
                    }
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
                HiveEvent::Idle { agent, .. } => format!("Idle: @{}", agent),
                HiveEvent::Stop { .. } => "Stopped".to_string(),
                HiveEvent::Start { model, .. } => {
                    format!("Started ({})", model)
                }
            };
            lines.push(Line::from(vec![
                Span::raw("      "),
                Span::styled("⚡ ", Style::default().fg(Color::Cyan)),
                Span::styled(event_desc, Style::default().fg(Color::DarkGray)),
            ]));
        }

        // Show team messages inline (last 2-3 messages)
        let inboxes = task_sync::read_team_inboxes(name).unwrap_or_default();
        if !inboxes.is_empty() {
            // Collect all messages with recipient info, sorted by timestamp
            let mut all_msgs: Vec<(String, String, String, bool)> = Vec::new();
            for (recipient, msgs) in &inboxes {
                for m in msgs {
                    // Parse JSON messages to get a clean display
                    let display_text = if m.text.starts_with('{') {
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&m.text) {
                            let msg_type = parsed.get("type").and_then(|v| v.as_str()).unwrap_or("");
                            match msg_type {
                                "idle_notification" => format!("[idle] {}", m.from),
                                "shutdown_request" => format!(
                                    "[shutdown request] {}",
                                    parsed
                                        .get("content")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                ),
                                "shutdown_response" => {
                                    let approved = parsed
                                        .get("approve")
                                        .and_then(|v| v.as_bool())
                                        .unwrap_or(false);
                                    format!("[shutdown {}]", if approved { "approved" } else { "rejected" })
                                }
                                "task_completed" | "task_assignment" => {
                                    let content = parsed
                                        .get("content")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or(&m.text);
                                    content.to_string()
                                }
                                _ => parsed
                                    .get("content")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or(&m.text)
                                    .to_string(),
                            }
                        } else {
                            m.text.clone()
                        }
                    } else {
                        m.text.clone()
                    };

                    all_msgs.push((
                        m.timestamp.clone(),
                        format!("{} → {}", m.from, recipient),
                        display_text,
                        m.read,
                    ));
                }
            }
            all_msgs.sort_by(|a, b| b.0.cmp(&a.0)); // Sort descending (most recent first)

            // Show last 2-3 messages
            let recent_msgs: Vec<_> = all_msgs.iter().take(3).collect();
            if !recent_msgs.is_empty() {
                for (ts, route, text, read) in recent_msgs {
                    let time_str = if ts.len() >= 19 { &ts[11..19] } else { ts };
                    let unread_marker = if !read { "●" } else { " " };

                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("      {} ", unread_marker),
                            Style::default().fg(if !read { Color::Cyan } else { Color::DarkGray }),
                        ),
                        Span::styled(
                            format!("{} ", time_str),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(route.clone(), Style::default().fg(Color::Cyan)),
                    ]));

                    // Truncate long messages to fit on one line
                    let max_msg_len = 80;
                    let display_msg = if text.len() > max_msg_len {
                        format!("{}...", &text[..max_msg_len])
                    } else {
                        text.clone()
                    };

                    lines.push(Line::from(vec![
                        Span::raw("          "),
                        Span::styled(display_msg, Style::default().fg(Color::White)),
                    ]));
                }
            }
        }

        // Show blocked indicator (press 'b' for details)
        if status.status == DroneState::Blocked {
            let orange = Color::Rgb(255, 165, 0);
            lines.push(Line::from(vec![
                Span::raw("      "),
                Span::styled(
                    "⚠ BLOCKED",
                    Style::default().fg(orange).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " - press 'b' for details",
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }

        // Show new stories indicator (auto-resume pending)
        if has_new_stories {
            let new_count = prd_story_count - status.total;
            lines.push(Line::from(vec![
                Span::raw("      "),
                Span::styled(
                    format!(
                        "✨ {} new stor{}",
                        new_count,
                        if new_count == 1 { "y" } else { "ies" }
                    ),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " - auto-resuming...",
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }
    }
}
