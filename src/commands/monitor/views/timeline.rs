use chrono::Utc;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use std::collections::HashMap;

use crate::commands::common::{parse_timestamp, truncate_with_ellipsis};
use crate::types::{DroneStatus, Prd};

/// Render the timeline/Gantt view showing story timings across all drones.
pub(crate) fn render_timeline_view(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    drones: &[(String, DroneStatus)],
    prd_cache: &HashMap<String, Prd>,
    scroll: usize,
) {
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
            Span::styled("  Timeline", Style::default().fg(Color::DarkGray)),
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
                format!("  {} drones", drones.len()),
                Style::default().fg(Color::Cyan),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(header_lines), chunks[0]);

    // Build timeline content
    let mut lines: Vec<Line> = Vec::new();
    let bar_width = (area.width as usize).saturating_sub(30).max(20);

    for (name, status) in drones {
        let start_ts = parse_timestamp(&status.started);
        let now = Utc::now();

        // Total time range for this drone
        let total_secs = start_ts
            .map(|s| now.signed_duration_since(s).num_seconds().max(1))
            .unwrap_or(1) as f64;

        lines.push(Line::from(vec![
            Span::styled(
                format!("  🐝 {:<20}", truncate_with_ellipsis(name, 20)),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        // Stories removed in plan mode - timeline not supported for tasks

        lines.push(Line::raw(""));
    }

    let content_height = chunks[1].height as usize;
    let visible: Vec<Line> = lines.into_iter().skip(scroll).take(content_height).collect();
    f.render_widget(Paragraph::new(visible), chunks[1]);

    let footer = Paragraph::new(Line::from(vec![Span::styled(
        " ↑↓ scroll  t back to dashboard  q quit",
        Style::default().fg(Color::DarkGray),
    )]));
    f.render_widget(footer, chunks[2]);
}
