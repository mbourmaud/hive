use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};
use std::fs;
use std::path::PathBuf;

use super::super::log_parsing::{parse_log_summary, pretty_print_json};

/// Log viewer poll timeout in milliseconds
const LOG_POLL_TIMEOUT_MS: u64 = 500;

// Show logs viewer in TUI with line selection and JSON pretty-print
pub(crate) fn show_logs_viewer<B: ratatui::backend::Backend>(
    terminal: &mut ratatui::Terminal<B>,
    drone_name: &str,
) -> Result<()> {
    // Read log file
    let log_path = PathBuf::from(".hive")
        .join("drones")
        .join(drone_name)
        .join("activity.log");

    let log_content = fs::read_to_string(&log_path).unwrap_or_else(|_| "No logs found".to_string());

    let log_lines: Vec<String> = log_content.lines().map(|s| s.to_string()).collect();
    let total_lines = log_lines.len();
    let mut selected_line: usize = total_lines.saturating_sub(1); // Start at last line
    let mut scroll_offset: usize = total_lines.saturating_sub(20);
    let mut detail_view: Option<String> = None; // Pretty-printed JSON for detail view
    let mut detail_scroll: usize = 0;

    loop {
        // Reload log file to get updates
        let log_content =
            fs::read_to_string(&log_path).unwrap_or_else(|_| "No logs found".to_string());
        let log_lines: Vec<String> = log_content.lines().map(|s| s.to_string()).collect();
        let total_lines = log_lines.len();

        // Clamp selected line
        if total_lines > 0 && selected_line >= total_lines {
            selected_line = total_lines - 1;
        }

        terminal.draw(|f| {
            let area = f.area();

            // If showing detail view (pretty-printed JSON)
            if let Some(ref detail) = detail_view {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(4), // Header
                        Constraint::Min(0),    // Content
                        Constraint::Length(1), // Footer
                    ])
                    .split(area);

                // Use different emoji based on execution mode
                let mode_emoji = "🐝";

                // Header with HIVE ASCII art
                let header_lines = vec![
                    Line::from(vec![
                        Span::styled("  ╦ ╦╦╦  ╦╔═╗", Style::default().fg(Color::Yellow)),
                        Span::styled("  Log Detail", Style::default().fg(Color::DarkGray)),
                    ]),
                    Line::from(vec![
                        Span::styled("  ╠═╣║╚╗╔╝║╣ ", Style::default().fg(Color::Yellow)),
                        Span::styled(
                            format!("  {} {}", mode_emoji, drone_name),
                            Style::default().fg(Color::Cyan),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled("  ╩ ╩╩ ╚╝ ╚═╝", Style::default().fg(Color::Yellow)),
                        Span::styled(
                            format!("  Line {}/{}", selected_line + 1, total_lines),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]),
                ];
                f.render_widget(Paragraph::new(header_lines), chunks[0]);

                // Detail content with word wrap
                let content_width = chunks[1].width.saturating_sub(4) as usize;

                // Wrap lines to fit screen width
                let wrapped_lines: Vec<(String, Style)> = detail
                    .lines()
                    .flat_map(|line| {
                        let style = if line.contains("\"error\"") || line.contains("ERROR") {
                            Style::default().fg(Color::Red)
                        } else if line.contains("\"type\"") {
                            Style::default().fg(Color::Cyan)
                        } else if line.contains("\"text\"") || line.contains("\"name\"") {
                            Style::default().fg(Color::Green)
                        } else if line.trim().starts_with("\"") {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default().fg(Color::White)
                        };

                        // Wrap long lines
                        if line.len() > content_width {
                            let mut wrapped = Vec::new();
                            let mut remaining = line;
                            while !remaining.is_empty() {
                                let (chunk, rest) = if remaining.len() > content_width {
                                    remaining.split_at(content_width)
                                } else {
                                    (remaining, "")
                                };
                                wrapped.push((chunk.to_string(), style));
                                remaining = rest;
                            }
                            wrapped
                        } else {
                            vec![(line.to_string(), style)]
                        }
                    })
                    .collect();

                let detail_total = wrapped_lines.len();
                let content_height = chunks[1].height as usize;

                let visible_detail: Vec<Line> = wrapped_lines
                    .iter()
                    .skip(detail_scroll)
                    .take(content_height)
                    .map(|(line, style)| Line::from(Span::styled(format!("  {}", line), *style)))
                    .collect();

                f.render_widget(Paragraph::new(visible_detail), chunks[1]);

                // Scrollbar for detail view
                if detail_total > content_height {
                    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                        .begin_symbol(None)
                        .end_symbol(None)
                        .track_symbol(Some("│"))
                        .thumb_symbol("█");

                    let mut scrollbar_state = ScrollbarState::new(detail_total)
                        .position(detail_scroll)
                        .viewport_content_length(content_height);

                    let scrollbar_area = Rect {
                        x: chunks[1].x + chunks[1].width - 1,
                        y: chunks[1].y,
                        width: 1,
                        height: chunks[1].height,
                    };
                    f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
                }

                // Footer
                let footer = Paragraph::new(Line::from(vec![Span::styled(
                    " ↑↓ scroll  q/Esc back to list",
                    Style::default().fg(Color::DarkGray),
                )]));
                f.render_widget(footer, chunks[2]);

                return;
            }

            // Main log list view
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(4), // Header with ASCII art
                    Constraint::Min(0),    // Content
                    Constraint::Length(1), // Footer
                ])
                .split(area);

            let mode_emoji = "🐝";

            // Header with HIVE ASCII art
            let header_lines = vec![
                Line::from(vec![
                    Span::styled("  ╦ ╦╦╦  ╦╔═╗", Style::default().fg(Color::Yellow)),
                    Span::styled("  Activity Logs", Style::default().fg(Color::DarkGray)),
                ]),
                Line::from(vec![
                    Span::styled("  ╠═╣║╚╗╔╝║╣ ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format!("  {} {}", mode_emoji, drone_name),
                        Style::default().fg(Color::Cyan),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("  ╩ ╩╩ ╚╝ ╚═╝", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format!("  {} entries", total_lines),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]),
            ];
            f.render_widget(Paragraph::new(header_lines), chunks[0]);

            // Content - log lines with selection
            let content_height = chunks[1].height as usize;
            let content_width = chunks[1].width.saturating_sub(4) as usize;

            // Ensure selected line is visible
            if selected_line < scroll_offset {
                scroll_offset = selected_line;
            } else if selected_line >= scroll_offset + content_height {
                scroll_offset = selected_line.saturating_sub(content_height - 1);
            }

            let visible_lines: Vec<Line> = log_lines
                .iter()
                .enumerate()
                .skip(scroll_offset)
                .take(content_height)
                .map(|(idx, line)| {
                    let is_selected = idx == selected_line;

                    // Parse JSON to get summary
                    let summary = parse_log_summary(line, content_width);

                    let base_style = if line.contains("\"error\"") || line.contains("ERROR") {
                        Style::default().fg(Color::Red)
                    } else if line.contains("tool_use") || line.contains("\"name\"") {
                        Style::default().fg(Color::Cyan)
                    } else if line.contains("\"text\"") {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::White)
                    };

                    let style = if is_selected {
                        base_style.bg(Color::DarkGray).add_modifier(Modifier::BOLD)
                    } else {
                        base_style
                    };

                    let prefix = if is_selected { "▸ " } else { "  " };
                    Line::from(Span::styled(format!("{}{}", prefix, summary), style))
                })
                .collect();

            f.render_widget(Paragraph::new(visible_lines), chunks[1]);

            // Scrollbar
            if total_lines > content_height {
                let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .end_symbol(None)
                    .track_symbol(Some("│"))
                    .thumb_symbol("█");

                let mut scrollbar_state = ScrollbarState::new(total_lines)
                    .position(scroll_offset)
                    .viewport_content_length(content_height);

                let scrollbar_area = Rect {
                    x: chunks[1].x + chunks[1].width - 1,
                    y: chunks[1].y,
                    width: 1,
                    height: chunks[1].height,
                };
                f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
            }

            // Footer
            let footer = Paragraph::new(Line::from(vec![Span::styled(
                " ↑↓/jk navigate  ↵ expand  G end  g start  q back",
                Style::default().fg(Color::DarkGray),
            )]));
            f.render_widget(footer, chunks[2]);
        })?;

        // Handle input
        if event::poll(std::time::Duration::from_millis(LOG_POLL_TIMEOUT_MS))? {
            if let Event::Key(key) = event::read()? {
                let content_height = terminal.size()?.height.saturating_sub(5) as usize;

                if detail_view.is_some() {
                    // Detail view controls
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                            detail_view = None;
                            detail_scroll = 0;
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            detail_scroll += 1;
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            detail_scroll = detail_scroll.saturating_sub(1);
                        }
                        KeyCode::PageDown => {
                            detail_scroll += content_height;
                        }
                        KeyCode::PageUp => {
                            detail_scroll = detail_scroll.saturating_sub(content_height);
                        }
                        KeyCode::Char('g') => {
                            detail_scroll = 0;
                        }
                        KeyCode::Char('G') => {
                            // Go to end - will be clamped in render
                            detail_scroll = usize::MAX / 2;
                        }
                        _ => {}
                    }
                } else {
                    // List view controls
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => break,
                        KeyCode::Char('j') | KeyCode::Down => {
                            if total_lines > 0 && selected_line < total_lines - 1 {
                                selected_line += 1;
                            }
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            selected_line = selected_line.saturating_sub(1);
                        }
                        KeyCode::Char('g') => {
                            selected_line = 0;
                            scroll_offset = 0;
                        }
                        KeyCode::Char('G') => {
                            if total_lines > 0 {
                                selected_line = total_lines - 1;
                                scroll_offset = total_lines.saturating_sub(content_height);
                            }
                        }
                        KeyCode::PageDown => {
                            selected_line =
                                (selected_line + content_height).min(total_lines.saturating_sub(1));
                        }
                        KeyCode::PageUp => {
                            selected_line = selected_line.saturating_sub(content_height);
                        }
                        KeyCode::Enter => {
                            // Pretty-print selected line
                            if total_lines > 0 && selected_line < log_lines.len() {
                                let line = &log_lines[selected_line];
                                detail_view = Some(pretty_print_json(line));
                                detail_scroll = 0;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(())
}
