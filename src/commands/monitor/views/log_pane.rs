use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use std::path::PathBuf;

use super::super::log_parsing::parse_log_summary;

/// Render the split-pane log viewer.
pub(crate) fn render_log_pane(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    drone_name: &str,
    scroll: usize,
    auto_scroll: bool,
    has_focus: bool,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // header
            Constraint::Min(0),   // content
            Constraint::Length(1), // footer
        ])
        .split(area);

    // Header
    let border_color = if has_focus { Color::Cyan } else { Color::DarkGray };
    let header_lines = vec![
        Line::from(vec![
            Span::styled(
                format!(" 📋 {} ", drone_name),
                Style::default()
                    .fg(border_color)
                    .add_modifier(Modifier::BOLD),
            ),
            if auto_scroll {
                Span::styled(" [auto-scroll]", Style::default().fg(Color::DarkGray))
            } else {
                Span::raw("")
            },
        ]),
        Line::styled(
            "─".repeat(area.width as usize),
            Style::default().fg(border_color),
        ),
    ];
    f.render_widget(Paragraph::new(header_lines), chunks[0]);

    // Read log file
    let log_path = PathBuf::from(".hive/drones")
        .join(drone_name)
        .join("activity.log");

    let log_content = std::fs::read_to_string(&log_path).unwrap_or_default();
    let content_width = chunks[1].width.saturating_sub(2) as usize;

    let mut lines: Vec<Line> = Vec::new();
    for line in log_content.lines() {
        let summary = parse_log_summary(line, content_width);

        let style = if line.contains("\"error\"") || line.contains("ERROR") {
            Style::default().fg(Color::Red)
        } else if line.contains("tool_use") {
            Style::default().fg(Color::Cyan)
        } else if line.contains("\"text\"") {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::White)
        };

        lines.push(Line::from(Span::styled(format!(" {}", summary), style)));
    }

    let content_height = chunks[1].height as usize;
    let total = lines.len();

    // Auto-scroll to bottom
    let effective_scroll = if auto_scroll {
        total.saturating_sub(content_height)
    } else {
        scroll
    };

    let visible: Vec<Line> = lines
        .into_iter()
        .skip(effective_scroll)
        .take(content_height)
        .collect();
    f.render_widget(Paragraph::new(visible), chunks[1]);

    // Footer
    let footer = Paragraph::new(Line::from(vec![Span::styled(
        " Tab focus  L close",
        Style::default().fg(Color::DarkGray),
    )]));
    f.render_widget(footer, chunks[2]);
}
