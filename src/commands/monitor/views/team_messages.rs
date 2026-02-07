use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use crate::agent_teams::task_sync;
use crate::commands::common::wrap_text;

// Show Agent Teams messages in a full-screen TUI view
pub(crate) fn show_team_messages_viewer<B: ratatui::backend::Backend>(
    terminal: &mut ratatui::Terminal<B>,
    drone_name: &str,
) -> Result<()> {
    let mut scroll_offset: usize = 0;

    loop {
        // Read inboxes
        let inboxes = task_sync::read_team_inboxes(drone_name).unwrap_or_default();
        let members = task_sync::read_team_members(drone_name).unwrap_or_default();

        // Collect all messages with recipient info, sorted by timestamp
        let mut all_msgs: Vec<(String, String, String, bool)> = Vec::new(); // (timestamp, from, text, read)
        for (recipient, msgs) in &inboxes {
            for m in msgs {
                // Parse JSON messages to get a clean display
                let display_text = if m.text.starts_with('{') {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&m.text) {
                        let msg_type = parsed.get("type").and_then(|v| v.as_str()).unwrap_or("");
                        match msg_type {
                            "idle_notification" => format!("[idle] {}", m.from),
                            "shutdown_request" => format!("[shutdown request] {}", parsed.get("content").and_then(|v| v.as_str()).unwrap_or("")),
                            "shutdown_response" => {
                                let approved = parsed.get("approve").and_then(|v| v.as_bool()).unwrap_or(false);
                                format!("[shutdown {}]", if approved { "approved" } else { "rejected" })
                            }
                            "task_completed" | "task_assignment" => {
                                let content = parsed.get("content").and_then(|v| v.as_str()).unwrap_or(&m.text);
                                content.to_string()
                            }
                            _ => {
                                parsed.get("content").and_then(|v| v.as_str())
                                    .unwrap_or(&m.text).to_string()
                            }
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
        all_msgs.sort_by(|a, b| a.0.cmp(&b.0));

        terminal.draw(|f| {
            let area = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(4),
                    Constraint::Min(0),
                    Constraint::Length(1),
                ])
                .split(area);

            // Header
            let header_lines = vec![
                Line::from(vec![
                    Span::styled("  ╦ ╦╦╦  ╦╔═╗", Style::default().fg(Color::Yellow)),
                    Span::styled("  Team Messages", Style::default().fg(Color::DarkGray)),
                ]),
                Line::from(vec![
                    Span::styled("  ╠═╣║╚╗╔╝║╣ ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format!("  🐝 {}", drone_name),
                        Style::default().fg(Color::Cyan),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("  ╩ ╩╩ ╚╝ ╚═╝", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format!("  {} msgs, {} teammates", all_msgs.len(), members.len()),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]),
            ];
            f.render_widget(Paragraph::new(header_lines), chunks[0]);

            // Messages
            let mut lines: Vec<Line> = Vec::new();

            if all_msgs.is_empty() {
                lines.push(Line::raw(""));
                lines.push(Line::styled(
                    "  No messages yet",
                    Style::default().fg(Color::DarkGray),
                ));
            } else {
                let content_width = (area.width as usize).saturating_sub(8);

                for (ts, route, text, read) in &all_msgs {
                    let time_str = if ts.len() >= 19 { &ts[11..19] } else { ts };
                    let unread_marker = if !read { "●" } else { " " };

                    // Header line: marker + time + route
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("  {} ", unread_marker),
                            Style::default().fg(if !read { Color::Cyan } else { Color::DarkGray }),
                        ),
                        Span::styled(
                            format!("{} ", time_str),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(
                            route.clone(),
                            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                        ),
                    ]));

                    // Content lines: indented and word-wrapped
                    let wrapped = wrap_text(text, content_width.max(20));
                    for chunk in &wrapped {
                        lines.push(Line::from(vec![
                            Span::raw("      "),
                            Span::styled(chunk.clone(), Style::default().fg(Color::White)),
                        ]));
                    }

                    // Blank separator between messages
                    lines.push(Line::raw(""));
                }
            }

            let content_height = chunks[1].height as usize;
            let total_lines = lines.len();

            // Auto-scroll to bottom
            if scroll_offset == 0 && total_lines > content_height {
                scroll_offset = total_lines.saturating_sub(content_height);
            }

            let visible: Vec<Line> = lines
                .into_iter()
                .skip(scroll_offset)
                .take(content_height)
                .collect();
            f.render_widget(Paragraph::new(visible), chunks[1]);

            // Scrollbar
            if total_lines > content_height {
                let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .end_symbol(None)
                    .track_symbol(Some("│"))
                    .thumb_symbol("█");
                let mut state = ScrollbarState::new(total_lines)
                    .position(scroll_offset)
                    .viewport_content_length(content_height);
                let sb_area = Rect {
                    x: chunks[1].x + chunks[1].width - 1,
                    y: chunks[1].y,
                    width: 1,
                    height: chunks[1].height,
                };
                f.render_stateful_widget(scrollbar, sb_area, &mut state);
            }

            // Footer
            let footer = Paragraph::new(Line::from(vec![Span::styled(
                " ↑↓ scroll  q back",
                Style::default().fg(Color::DarkGray),
            )]));
            f.render_widget(footer, chunks[2]);
        })?;

        // Input
        if crossterm::event::poll(std::time::Duration::from_millis(500))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Down | KeyCode::Char('j') => {
                        scroll_offset = scroll_offset.saturating_add(1);
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        scroll_offset = scroll_offset.saturating_sub(1);
                    }
                    KeyCode::PageDown => {
                        scroll_offset = scroll_offset.saturating_add(20);
                    }
                    KeyCode::PageUp => {
                        scroll_offset = scroll_offset.saturating_sub(20);
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}
