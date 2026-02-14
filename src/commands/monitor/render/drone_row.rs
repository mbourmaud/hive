use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use crate::commands::common::{truncate_with_ellipsis, MAX_DRONE_NAME_LEN};
use crate::types::DroneState;

use super::super::state::TuiState;

/// Build the header line for a single drone row in the main list.
pub fn build_drone_header_line(
    state: &TuiState,
    drone_idx: usize,
    is_selected: bool,
    is_expanded: bool,
    process_running: bool,
) -> Line<'static> {
    let (name, status) = &state.drones[drone_idx];

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

    let (valid_completed, task_count) = state.snapshot_store.progress(name);
    let percentage = if task_count > 0 {
        (valid_completed as f32 / task_count as f32 * 100.0) as u16
    } else {
        0
    };

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

    let name_style = if is_selected {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else if status.status == DroneState::Completed {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::Yellow)
    };

    let name_display = truncate_with_ellipsis(name, MAX_DRONE_NAME_LEN);

    let c = state.cost_cache.get(name).cloned().unwrap_or_default();
    let cost_usd = c.total_cost_usd;
    let cost_str = if cost_usd > 0.0 {
        format!(" ${:.2}", cost_usd)
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

    let model_badge = status
        .lead_model
        .as_ref()
        .map(|m| format!(" [{}]", m))
        .unwrap_or_default();

    Line::from(vec![
        Span::raw(format!(" {} ", select_char)),
        Span::styled(icon, Style::default().fg(status_color)),
        Span::raw(" "),
        Span::styled(expand_indicator, Style::default().fg(Color::DarkGray)),
        Span::raw(" "),
        Span::styled(format!("🐝 {} ", name_display), name_style),
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
    ])
}

pub fn render_header(f: &mut Frame, area: Rect, drone_count: usize) {
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
                    drone_count,
                    if drone_count != 1 { "s" } else { "" }
                ),
                Style::default().fg(Color::Cyan),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(header_lines), area);
}

pub fn render_empty_placeholder(lines: &mut Vec<Line>) {
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

pub fn render_active_header(lines: &mut Vec<Line>, active_count: usize) {
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
}

pub fn render_archived_header(lines: &mut Vec<Line>, archived_count: usize) {
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
            format!(" ({})", archived_count),
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    lines.push(Line::raw(""));
}

pub fn render_scrollbar(
    f: &mut Frame,
    area: Rect,
    total_lines: usize,
    content_height: usize,
    scroll_offset: usize,
) {
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
            x: area.x + area.width - 1,
            y: area.y,
            width: 1,
            height: area.height,
        };
        f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }
}
