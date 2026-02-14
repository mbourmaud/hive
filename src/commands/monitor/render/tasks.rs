use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::collections::HashMap;

use crate::agent_teams::task_sync::TeamTaskInfo;
use crate::commands::common::{format_duration, truncate_with_ellipsis};

use super::helpers::{extract_internal_task_title, get_agent_color};

/// Get the icon, icon color, and text color for a task based on its status.
pub fn task_status_style(status: &str) -> (&'static str, Color, Color) {
    match status {
        "completed" => ("●", Color::Green, Color::DarkGray),
        "in_progress" => ("◐", Color::Yellow, Color::White),
        _ => ("○", Color::DarkGray, Color::White),
    }
}

/// Render a top-level task line (user task or internal task shown as top-level during planning).
pub fn render_user_task(
    lines: &mut Vec<Line>,
    task: &TeamTaskInfo,
    member_color_map: &HashMap<String, usize>,
    area: Rect,
) {
    let (task_icon, icon_color, text_color) = task_status_style(&task.status);

    let (title, agent_name) = if task.is_internal {
        (
            extract_internal_task_title(task),
            Some(task.subject.clone()),
        )
    } else {
        let subj = task.subject.trim_start_matches('#').trim().to_string();
        (subj, task.owner.clone())
    };

    let agent_badge_with_color = agent_name.map(|a| {
        let badge_text = format!(" @{}", a);
        let agent_color = member_color_map
            .get(&a)
            .map(|&idx| get_agent_color(idx))
            .unwrap_or(Color::Cyan);
        (badge_text, agent_color)
    });

    // For completed tasks, show duration instead of active_form
    let timing_suffix = if task.status == "completed" {
        task.created_at
            .zip(task.updated_at)
            .map(|(created, updated)| {
                let duration_ms = updated.saturating_sub(created);
                let duration = chrono::Duration::milliseconds(duration_ms as i64);
                format!(" ({})", format_duration(duration))
            })
            .unwrap_or_default()
    } else {
        String::new()
    };

    let active_form = if !timing_suffix.is_empty() {
        timing_suffix
    } else {
        task.active_form
            .as_ref()
            .map(|f| format!(" ({})", f))
            .unwrap_or_default()
    };

    let task_prefix_len = 8;
    let badge_len = agent_badge_with_color
        .as_ref()
        .map(|(text, _)| text.len())
        .unwrap_or(0)
        + active_form.len();
    let task_available_width = area.width as usize;
    let max_task_title_width = task_available_width.saturating_sub(task_prefix_len + badge_len + 1);

    if title.chars().count() <= max_task_title_width || max_task_title_width < 20 {
        render_single_line_task(
            lines,
            task_icon,
            icon_color,
            text_color,
            &title,
            &active_form,
            agent_badge_with_color.as_ref(),
        );
    } else {
        render_wrapped_task(
            lines,
            task_icon,
            icon_color,
            text_color,
            &title,
            &active_form,
            agent_badge_with_color.as_ref(),
            task_available_width,
            task_prefix_len,
            badge_len,
        );
    }
}

fn render_single_line_task(
    lines: &mut Vec<Line>,
    task_icon: &str,
    icon_color: Color,
    text_color: Color,
    title: &str,
    active_form: &str,
    agent_badge: Option<&(String, Color)>,
) {
    let mut spans = vec![
        Span::raw("      "),
        Span::styled(task_icon.to_string(), Style::default().fg(icon_color)),
        Span::raw(" "),
        Span::styled(title.to_string(), Style::default().fg(text_color)),
        Span::styled(
            active_form.to_string(),
            Style::default().fg(Color::DarkGray),
        ),
    ];
    if let Some((badge_text, badge_color)) = agent_badge {
        spans.push(Span::styled(
            badge_text.clone(),
            Style::default()
                .fg(*badge_color)
                .add_modifier(Modifier::BOLD),
        ));
    }
    lines.push(Line::from(spans));
}

#[allow(clippy::too_many_arguments)]
fn render_wrapped_task(
    lines: &mut Vec<Line>,
    task_icon: &str,
    icon_color: Color,
    text_color: Color,
    title: &str,
    active_form: &str,
    agent_badge: Option<&(String, Color)>,
    task_available_width: usize,
    task_prefix_len: usize,
    badge_len: usize,
) {
    let task_title_indent = "        "; // 8 spaces
    let wrap_width = task_available_width.saturating_sub(task_prefix_len + 1);
    let last_line_wrap_width = wrap_width.saturating_sub(badge_len);
    let mut remaining = title;
    let mut first_line = true;

    while !remaining.is_empty() {
        let char_count = remaining.chars().count();
        let is_last = char_count <= last_line_wrap_width;
        let (chunk, rest) = if is_last {
            (remaining, "")
        } else {
            let split_width = if char_count <= wrap_width {
                last_line_wrap_width
            } else {
                wrap_width
            };
            let byte_limit: usize = remaining
                .char_indices()
                .nth(split_width)
                .map(|(i, _)| i)
                .unwrap_or(remaining.len());
            let break_at = remaining[..byte_limit].rfind(' ').unwrap_or(byte_limit);
            (&remaining[..break_at], remaining[break_at..].trim_start())
        };

        let mut spans = if first_line {
            vec![
                Span::raw("      "),
                Span::styled(task_icon.to_string(), Style::default().fg(icon_color)),
                Span::raw(" "),
                Span::styled(chunk.to_string(), Style::default().fg(text_color)),
            ]
        } else {
            vec![
                Span::raw(task_title_indent),
                Span::styled(chunk.to_string(), Style::default().fg(text_color)),
            ]
        };

        if is_last {
            spans.push(Span::styled(
                active_form.to_string(),
                Style::default().fg(Color::DarkGray),
            ));
            if let Some((badge_text, badge_color)) = agent_badge {
                spans.push(Span::styled(
                    badge_text.clone(),
                    Style::default().fg(*badge_color),
                ));
            }
        }
        lines.push(Line::from(spans));
        first_line = false;
        remaining = rest;
    }
}

/// Render an internal task nested beneath its parent user task.
/// Format: `        +-- icon @agent: Task description`
pub fn render_nested_internal(
    lines: &mut Vec<Line>,
    task: &TeamTaskInfo,
    member_color_map: &HashMap<String, usize>,
    area: Rect,
) {
    let (task_icon, icon_color, text_color) = task_status_style(&task.status);

    let agent_name = &task.subject;
    let agent_color = member_color_map
        .get(agent_name)
        .map(|&idx| get_agent_color(idx))
        .unwrap_or(Color::Cyan);

    let title = extract_internal_task_title(task);

    // Prefix: "        └─ " (8 spaces + tree connector)
    let prefix = "        └─ ";
    let agent_badge = format!("@{}: ", agent_name);
    let prefix_len = prefix.len() + 2 + agent_badge.len(); // icon + space + badge
    let max_width = area.width as usize;
    let title_display = truncate_with_ellipsis(&title, max_width.saturating_sub(prefix_len));

    lines.push(Line::from(vec![
        Span::styled("        └─ ", Style::default().fg(Color::DarkGray)),
        Span::styled(task_icon, Style::default().fg(icon_color)),
        Span::raw(" "),
        Span::styled(
            agent_badge,
            Style::default()
                .fg(agent_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(title_display, Style::default().fg(text_color)),
    ]));
}
