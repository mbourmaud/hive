use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use std::collections::VecDeque;

use super::super::state::ToolRecord;
use crate::commands::common::truncate_with_ellipsis;

/// Render the full-screen tool call log for a drone.
/// Shows the last N tool calls captured by PostToolUse hooks.
pub fn render_tools_view(
    f: &mut Frame,
    area: Rect,
    drone_name: &str,
    tool_history: Option<&VecDeque<ToolRecord>>,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Content
            Constraint::Length(1), // Footer
        ])
        .split(area);

    // Header
    let has_tools = tool_history.map(|h| !h.is_empty()).unwrap_or(false);
    let status_indicator = if has_tools {
        Span::styled(
            format!(" ({} calls)", tool_history.unwrap().len()),
            Style::default().fg(Color::DarkGray),
        )
    } else {
        Span::styled(" (no data)", Style::default().fg(Color::DarkGray))
    };

    let header = Paragraph::new(vec![
        Line::raw(""),
        Line::from(vec![
            Span::styled(
                format!("  🔧 Tool Calls — {}", drone_name),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            status_indicator,
        ]),
    ]);
    f.render_widget(header, chunks[0]);

    // Content: tool call list
    let mut lines: Vec<Line> = Vec::new();

    if let Some(tool_history) = tool_history {
        if tool_history.is_empty() {
            lines.push(Line::raw(""));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    "No tool calls recorded yet. Waiting for hook events...",
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        } else {
            // Column header
            lines.push(Line::from(vec![Span::styled(
                "  Tool              ID",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::styled(
                "  ─────────────────────────────────────────────────────────────────────────",
                Style::default().fg(Color::DarkGray),
            ));

            let max_id_width = area.width as usize - 22; // tool(18) + padding

            for record in tool_history.iter() {
                let tool_name = truncate_with_ellipsis(&record.tool, 16);
                let id_display = truncate_with_ellipsis(&record.tool_use_id, max_id_width.min(40));

                // Color-code by tool type
                let tool_color = match record.tool.as_str() {
                    "Bash" => Color::Red,
                    "Read" | "Glob" | "Grep" => Color::Cyan,
                    "Write" | "Edit" => Color::Yellow,
                    "Task" => Color::Magenta,
                    _ => Color::White,
                };

                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!("{:<16}", tool_name),
                        Style::default().fg(tool_color),
                    ),
                    Span::raw("  "),
                    Span::styled(id_display, Style::default().fg(Color::DarkGray)),
                ]));
            }
        }
    } else {
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "No tool call data available for this drone.",
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    }

    // Render content with scrollbar
    let content_height = chunks[1].height as usize;
    let total_lines = lines.len();

    // Auto-scroll to bottom (most recent tools)
    let scroll_offset = total_lines.saturating_sub(content_height);

    let visible_lines: Vec<Line> = lines
        .into_iter()
        .skip(scroll_offset)
        .take(content_height)
        .collect();

    f.render_widget(Paragraph::new(visible_lines), chunks[1]);

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
        " t close  q quit",
        Style::default().fg(Color::DarkGray),
    )]));
    f.render_widget(footer, chunks[2]);
}
