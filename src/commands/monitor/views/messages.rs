use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use std::collections::{HashMap, HashSet};

use crate::agent_teams::task_sync;

/// Agent color palette
const AGENT_COLORS: [Color; 8] = [
    Color::Cyan,
    Color::Magenta,
    Color::Yellow,
    Color::Blue,
    Color::Green,
    Color::Red,
    Color::LightCyan,
    Color::LightMagenta,
];

fn agent_color(index: usize) -> Color {
    AGENT_COLORS[index % AGENT_COLORS.len()]
}

/// A parsed message ready for display
struct DisplayMessage {
    timestamp: String,
    from: String,
    to: String,
    text: String,
    is_lead: bool,
}

/// Parse JSON message content into a clean display string
fn parse_message_text(raw: &str) -> String {
    if !raw.starts_with('{') {
        return raw.to_string();
    }
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(raw) else {
        return raw.to_string();
    };
    let msg_type = parsed.get("type").and_then(|v| v.as_str()).unwrap_or("");
    match msg_type {
        "idle_notification" => "[idle]".to_string(),
        "shutdown_request" => {
            let content = parsed
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("shutting down");
            format!("[shutdown] {}", content)
        }
        "shutdown_response" | "shutdown_approved" => {
            let approved = parsed
                .get("approve")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            if approved {
                "[shutdown approved]".to_string()
            } else {
                "[shutdown rejected]".to_string()
            }
        }
        _ => parsed
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or(raw)
            .to_string(),
    }
}

/// Render the fullscreen chat-style messages view for a specific drone
/// Returns (message_count, message_line_starts)
pub(crate) fn render_messages_view(
    f: &mut Frame,
    area: Rect,
    drone_name: &str,
    selected_index: usize,
) -> (usize, Vec<usize>) {
    // Layout: header + content + footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Messages
            Constraint::Length(1), // Footer
        ])
        .split(area);

    // Header
    let header = Paragraph::new(vec![
        Line::raw(""),
        Line::from(vec![Span::styled(
            format!("  💬 Messages — {}", drone_name),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
    ]);
    f.render_widget(header, chunks[0]);

    // Load teammates and inboxes
    let teammates: HashSet<String> = task_sync::read_team_members(drone_name)
        .unwrap_or_default()
        .into_iter()
        .map(|m| m.name)
        .collect();

    let inboxes = task_sync::read_team_inboxes(drone_name).unwrap_or_default();

    // Collect all messages
    let mut messages: Vec<DisplayMessage> = Vec::new();
    for (recipient, msgs) in &inboxes {
        for m in msgs {
            let text = parse_message_text(&m.text);
            // Skip noise: idle notifications and shutdown protocol
            if text == "[idle]" || text.starts_with("[shutdown") {
                continue;
            }
            messages.push(DisplayMessage {
                timestamp: m.timestamp.clone(),
                from: m.from.clone(),
                to: recipient.clone(),
                text,
                is_lead: !teammates.contains(m.from.as_str()),
            });
        }
    }

    // Sort chronologically (oldest first)
    messages.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

    // Build agent color index
    let mut all_agents: Vec<String> = messages
        .iter()
        .flat_map(|m| vec![m.from.clone(), m.to.clone()])
        .collect();
    all_agents.sort();
    all_agents.dedup();
    let agent_index: HashMap<String, usize> = all_agents
        .iter()
        .enumerate()
        .map(|(idx, a)| (a.clone(), idx))
        .collect();

    // Build display lines — chat bubble style
    let mut lines: Vec<Line> = Vec::new();
    let mut message_line_starts: Vec<usize> = Vec::new();
    let max_width = area.width.saturating_sub(10) as usize;

    if messages.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("No messages yet", Style::default().fg(Color::DarkGray)),
        ]));
    }

    let mut last_from: Option<String> = None;

    for (msg_idx, msg) in messages.iter().enumerate() {
        // Record the line index where this message starts
        message_line_starts.push(lines.len());
        let from_color = agent_index
            .get(&msg.from)
            .map(|&idx| agent_color(idx))
            .unwrap_or(Color::Cyan);

        // Check if this message is selected (not in auto-scroll mode)
        let is_selected = selected_index != usize::MAX && msg_idx == selected_index;

        let time_str = if msg.timestamp.len() >= 19 {
            &msg.timestamp[11..16] // HH:MM only
        } else {
            &msg.timestamp
        };

        // Show sender header when it changes (like real chat apps)
        let same_sender = last_from.as_ref() == Some(&msg.from);
        if !same_sender {
            // Spacer between different senders
            if last_from.is_some() {
                lines.push(Line::raw(""));
            }

            let lead_badge = if msg.is_lead { " 👑" } else { "" };

            let left_marker = if is_selected {
                Span::styled(
                    "▶ ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::raw("  ")
            };

            lines.push(Line::from(vec![
                left_marker.clone(),
                Span::styled(
                    format!("@{}{}", msg.from, lead_badge),
                    Style::default().fg(from_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  → @{}", msg.to),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("  {}", time_str),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        } else {
            // Same sender, just show timestamp hint for context
            let left_marker = if is_selected {
                Span::styled(
                    "▶ ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::raw("  ")
            };

            lines.push(Line::from(vec![
                left_marker,
                Span::styled(
                    format!("→ @{}  {}", msg.to, time_str),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }

        // Message body — with left border in sender's color
        // Split on newlines first for paragraph/list structure
        let body = if msg.text.is_empty() {
            "(empty)"
        } else {
            &msg.text
        };

        let left_marker_fn = |sel: bool| -> Span {
            if sel {
                Span::styled(
                    "▶ ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::raw("  ")
            }
        };

        for logical_line in body.split('\n') {
            if logical_line.trim().is_empty() {
                // Empty line → spacer with just the border
                lines.push(Line::from(vec![
                    left_marker_fn(is_selected),
                    Span::styled("│", Style::default().fg(from_color)),
                ]));
                continue;
            }

            // Detect list items
            let trimmed = logical_line.trim_start();
            let is_list = trimmed.starts_with("- ")
                || trimmed.starts_with("* ")
                || (trimmed.len() > 2
                    && trimmed
                        .chars()
                        .next()
                        .map(|c| c.is_ascii_digit())
                        .unwrap_or(false)
                    && trimmed.contains(". "));
            let indent = if is_list { "  " } else { "" };
            let effective_width = max_width.saturating_sub(indent.len());

            // Word-wrap this logical line
            let mut remaining = logical_line.trim();
            while !remaining.is_empty() {
                let char_count = remaining.chars().count();
                let (chunk, rest) = if char_count <= effective_width {
                    (remaining, "")
                } else {
                    let byte_limit: usize = remaining
                        .char_indices()
                        .nth(effective_width)
                        .map(|(i, _)| i)
                        .unwrap_or(remaining.len());
                    let break_at = remaining[..byte_limit].rfind(' ').unwrap_or(byte_limit);
                    (&remaining[..break_at], remaining[break_at..].trim_start())
                };

                lines.push(Line::from(vec![
                    left_marker_fn(is_selected),
                    Span::styled("│ ", Style::default().fg(from_color)),
                    Span::styled(
                        format!("{}{}", indent, chunk),
                        Style::default().fg(Color::White),
                    ),
                ]));
                remaining = rest;
            }
        }

        last_from = Some(msg.from.clone());
    }

    // Scrolling — convert selected_index to scroll position
    let content_height = chunks[1].height as usize;
    let total_lines = lines.len();
    let auto_scroll = total_lines.saturating_sub(content_height);

    let effective_scroll = if selected_index == usize::MAX {
        // Auto-scroll mode: scroll to bottom
        auto_scroll
    } else if selected_index < message_line_starts.len() {
        // Manual mode: show selected message at ~1/3 from top
        let selected_line = message_line_starts[selected_index];
        let target_offset = content_height / 3;
        selected_line.saturating_sub(target_offset).min(auto_scroll)
    } else {
        // Invalid index: fall back to auto-scroll
        auto_scroll
    };

    let visible_lines: Vec<Line> = lines
        .into_iter()
        .skip(effective_scroll)
        .take(content_height)
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
            .position(effective_scroll)
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
        " ↑↓ navigate  g/G first/last  q back",
        Style::default().fg(Color::DarkGray),
    )]));
    f.render_widget(footer, chunks[2]);

    (messages.len(), message_line_starts)
}
