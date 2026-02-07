use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use std::collections::HashMap;

use crate::commands::common::wrap_text;
use crate::types::{DroneStatus, Prd};

// Render the blocked detail view
pub(crate) fn render_blocked_detail_view(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    drone_name: &str,
    status: &DroneStatus,
    prd_cache: &HashMap<String, Prd>,
) {
    let orange = Color::Rgb(255, 165, 0);

    // Layout: header (4) + subheader (2) + content + footer (1)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Header with ASCII art
            Constraint::Length(2), // Subheader with drone info
            Constraint::Min(0),    // Content
            Constraint::Length(1), // Footer
        ])
        .split(area);

    // Header with ASCII art (same as main view)
    let header_lines = vec![
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
                "  BLOCKED DRONE",
                Style::default().fg(orange).add_modifier(Modifier::BOLD),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(header_lines), chunks[0]);

    // Subheader: drone name + blocked story
    let blocked_story = status.current_story.as_deref().unwrap_or("Unknown");
    let story_title = ""; // Stories removed in plan mode

    let mode_emoji = "🐝";

    let subheader_lines = vec![
        Line::from(vec![
            Span::styled("  ⚠ ", Style::default().fg(orange)),
            Span::styled(
                format!("{} {}", mode_emoji, drone_name),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                blocked_story,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {}", story_title),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::styled(
            "  ─────────────────────────────────────────────────────────────",
            Style::default().fg(Color::DarkGray),
        ),
    ];
    f.render_widget(Paragraph::new(subheader_lines), chunks[1]);

    // Content: blocked reason + questions
    let mut content_lines: Vec<Line> = Vec::new();
    content_lines.push(Line::raw(""));

    // Blocked reason
    if let Some(ref reason) = status.blocked_reason {
        content_lines.push(Line::from(vec![Span::styled(
            "  REASON",
            Style::default().fg(orange).add_modifier(Modifier::BOLD),
        )]));
        content_lines.push(Line::raw(""));

        // Word-wrap the reason text
        let max_width = (area.width as usize).saturating_sub(6).min(80);
        let wrapped = wrap_text(reason, max_width);
        for line in wrapped {
            content_lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(line, Style::default().fg(Color::White)),
            ]));
        }
    }

    // Questions
    if !status.blocked_questions.is_empty() {
        content_lines.push(Line::raw(""));
        content_lines.push(Line::from(vec![Span::styled(
            "  QUESTIONS",
            Style::default().fg(orange).add_modifier(Modifier::BOLD),
        )]));
        content_lines.push(Line::raw(""));

        let max_width = (area.width as usize).saturating_sub(8).min(78);
        for (i, question) in status.blocked_questions.iter().enumerate() {
            content_lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(format!("{}. ", i + 1), Style::default().fg(Color::Yellow)),
            ]));

            let wrapped = wrap_text(question, max_width);
            for (j, line) in wrapped.iter().enumerate() {
                if j == 0 {
                    // First line: just append to the number
                    if let Some(last_line) = content_lines.last_mut() {
                        last_line.spans.push(Span::styled(
                            line.clone(),
                            Style::default().fg(Color::White),
                        ));
                    }
                } else {
                    // Continuation lines: indent
                    content_lines.push(Line::from(vec![
                        Span::raw("     "),
                        Span::styled(line.clone(), Style::default().fg(Color::White)),
                    ]));
                }
            }
            content_lines.push(Line::raw(""));
        }
    }

    // Show last errors from log if any
    if status.error_count > 0 {
        content_lines.push(Line::raw(""));
        content_lines.push(Line::from(vec![Span::styled(
            format!("  ERRORS ({})", status.error_count),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )]));
        content_lines.push(Line::raw(""));
        content_lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "Press 'l' to view full logs",
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    }

    f.render_widget(Paragraph::new(content_lines), chunks[2]);

    // Footer
    let footer = Paragraph::new(Line::from(vec![Span::styled(
        " l logs  q back",
        Style::default().fg(Color::DarkGray),
    )]));
    f.render_widget(footer, chunks[3]);
}
