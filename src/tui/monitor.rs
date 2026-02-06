/// Hive Monitor TUI Component
/// Renders drone status, progress bars, story lists in the sidebar
use crate::commands::common::{
    duration_between, elapsed_since, format_duration, is_process_running, load_prd,
    parse_timestamp, read_drone_pid, reconcile_progress_with_prd, truncate_with_ellipsis,
    DEFAULT_INACTIVE_THRESHOLD_SECS, MAX_DRONE_NAME_LEN,
};
use crate::tui::theme::Theme;
use crate::types::{DroneState, DroneStatus, ExecutionMode, Prd};
use chrono::Utc;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

/// Render a single drone's status line (compact view)
pub fn render_drone_line(
    drone_name: &str,
    status: &DroneStatus,
    is_selected: bool,
    is_expanded: bool,
    prd: Option<&Prd>,
    _area: &Rect,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let process_running = read_drone_pid(drone_name)
        .map(is_process_running)
        .unwrap_or(false);

    // Status icon and color
    let is_active_process = process_running || status.current_story.is_some();
    let (icon, status_color) = match status.status {
        DroneState::Starting | DroneState::Resuming => ("\u{25d0}", theme.drone_starting),
        DroneState::InProgress => {
            if is_active_process {
                ("\u{25d0}", theme.drone_active)
            } else {
                ("\u{25cb}", theme.drone_starting)
            }
        }
        DroneState::Completed => ("\u{25cf}", theme.drone_completed),
        DroneState::Error => ("\u{25d0}", theme.drone_error),
        DroneState::Blocked => ("\u{25d0}", theme.drone_error),
        DroneState::Stopped => ("\u{25cb}", theme.drone_stopped),
    };

    // Use reconciled progress to filter out old completed stories
    let (valid_completed, prd_story_count) = prd
        .map(|p| reconcile_progress_with_prd(status, p))
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

    let (filled_bar, empty_bar) = if status.status == DroneState::Completed && !has_new_stories {
        ("\u{2501}".repeat(bar_width), String::new())
    } else {
        ("\u{2501}".repeat(filled), "\u{2500}".repeat(empty))
    };

    let filled_color = match status.status {
        DroneState::Completed => theme.drone_completed,
        DroneState::Blocked | DroneState::Error => theme.drone_progress_blocked,
        _ => theme.drone_active,
    };

    // Expand/collapse indicator
    let expand_indicator = if is_expanded { "\u{25bc}" } else { "\u{25b6}" };

    // Selection indicator
    let select_char = if is_selected { "\u{25b8}" } else { " " };

    // Elapsed time - stop timer if completed
    let elapsed = if status.status == DroneState::Completed {
        let last_completed = status
            .story_times
            .values()
            .filter_map(|t| t.completed.as_ref())
            .max();
        if let (Some(last), Some(start)) = (last_completed, parse_timestamp(&status.started)) {
            if let Some(end) = parse_timestamp(last) {
                format_duration(end.signed_duration_since(start))
            } else {
                elapsed_since(&status.started).unwrap_or_default()
            }
        } else {
            elapsed_since(&status.started).unwrap_or_default()
        }
    } else if status.status == DroneState::Stopped {
        if let Some(duration) = duration_between(&status.started, &status.updated) {
            format_duration(duration)
        } else {
            elapsed_since(&status.started).unwrap_or_default()
        }
    } else {
        elapsed_since(&status.started).unwrap_or_default()
    };

    // Drone header line
    let name_style = if is_selected {
        Style::default()
            .fg(theme.drone_name)
            .add_modifier(Modifier::BOLD)
    } else if status.status == DroneState::Completed {
        Style::default().fg(theme.drone_name_completed)
    } else {
        Style::default().fg(theme.drone_name)
    };

    // Use different emoji based on execution mode
    let mode_emoji = match status.execution_mode {
        ExecutionMode::Subagent => "\u{1f916}",
        ExecutionMode::Worktree => "\u{1f41d}",
        ExecutionMode::Swarm => "\u{1f41d}",
    };

    let name_display = truncate_with_ellipsis(drone_name, MAX_DRONE_NAME_LEN);

    // Backend tag
    let mode_tag = format!("[{}|{}]", status.execution_mode, status.backend);

    // Check inbox for pending messages
    let inbox_dir = PathBuf::from(".hive/drones").join(drone_name).join("inbox");
    let inbox_count = if inbox_dir.exists() {
        fs::read_dir(&inbox_dir)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
                    .count()
            })
            .unwrap_or(0)
    } else {
        0
    };
    let inbox_indicator = if inbox_count > 0 {
        format!(" \u{2709}{}", inbox_count)
    } else {
        String::new()
    };

    let count_color = if status.status == DroneState::Completed && !has_new_stories {
        theme.drone_name_completed
    } else if has_new_stories {
        theme.drone_progress_new_stories
    } else {
        theme.drone_progress_count
    };

    let header_line = Line::from(vec![
        Span::raw(format!(" {} ", select_char)),
        Span::styled(icon, Style::default().fg(status_color)),
        Span::raw(" "),
        Span::styled(expand_indicator, Style::default().fg(theme.fg_muted)),
        Span::raw(" "),
        Span::styled(format!("{} {} ", mode_emoji, name_display), name_style),
        Span::styled(filled_bar, Style::default().fg(filled_color)),
        Span::styled(empty_bar, Style::default().fg(theme.fg_muted)),
        Span::raw(" "),
        Span::styled(
            format!("{}/{}", valid_completed, prd_story_count),
            Style::default().fg(count_color),
        ),
        Span::raw("  "),
        Span::styled(mode_tag.clone(), Style::default().fg(theme.fg_muted)),
        Span::styled(
            inbox_indicator.clone(),
            Style::default().fg(theme.accent_primary),
        ),
        Span::raw("  "),
        Span::styled(elapsed, Style::default().fg(theme.fg_muted)),
    ]);

    vec![header_line]
}

/// Render expanded drone story list
pub fn render_drone_stories(
    _drone_name: &str,
    status: &DroneStatus,
    prd: &Prd,
    selected_story_index: Option<usize>,
    is_drone_selected: bool,
    area: &Rect,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    for (story_idx, story) in prd.stories.iter().enumerate() {
        let is_completed = status.completed.contains(&story.id);
        let is_current = status.current_story.as_ref() == Some(&story.id);
        let is_story_selected = is_drone_selected && selected_story_index == Some(story_idx);

        // Check if story has unsatisfied dependencies
        let has_blocked_deps = if !story.depends_on.is_empty() && !is_completed {
            story
                .depends_on
                .iter()
                .any(|dep_id| !status.completed.contains(dep_id))
        } else {
            false
        };

        let (story_icon, story_color) = if is_story_selected {
            ("\u{25b8}", theme.accent_primary)
        } else if is_completed {
            ("\u{25cf}", theme.drone_completed)
        } else if has_blocked_deps {
            ("\u{231b}", theme.accent_warning)
        } else if is_current {
            ("\u{25d0}", theme.accent_warning)
        } else {
            ("\u{25cb}", theme.fg_muted)
        };

        // Dependency info suffix
        let dep_info = if has_blocked_deps {
            let missing: Vec<&str> = story
                .depends_on
                .iter()
                .filter(|dep_id| !status.completed.contains(dep_id))
                .map(|s| s.as_str())
                .collect();
            format!(" \u{231b} waiting: {}", missing.join(", "))
        } else {
            String::new()
        };

        // Duration
        let duration_str = if let Some(timing) = status.story_times.get(&story.id) {
            if let (Some(started), Some(completed)) = (&timing.started, &timing.completed) {
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
            theme.accent_primary
        } else {
            story_color
        };

        // Calculate max title width
        let prefix_len = 26;
        let duration_len = duration_str.len();
        let available_width = area.width as usize;
        let max_title_width = available_width.saturating_sub(prefix_len + duration_len + 2);

        if story.title.len() <= max_title_width || max_title_width < 20 {
            let mut spans = vec![
                Span::styled("      ", line_style),
                Span::styled(story_icon, line_style.fg(story_color)),
                Span::raw(" "),
                Span::styled(format!("{:<16} ", story.id), line_style.fg(title_color)),
                Span::styled(story.title.clone(), line_style.fg(title_color)),
                Span::styled(duration_str.clone(), line_style.fg(theme.fg_muted)),
            ];
            if !dep_info.is_empty() {
                spans.push(Span::styled(
                    dep_info.clone(),
                    Style::default().fg(theme.accent_warning),
                ));
            }
            lines.push(Line::from(spans));
        } else {
            let truncated_title = truncate_with_ellipsis(&story.title, max_title_width);
            let mut spans = vec![
                Span::styled("      ", line_style),
                Span::styled(story_icon, line_style.fg(story_color)),
                Span::raw(" "),
                Span::styled(format!("{:<16} ", story.id), line_style.fg(title_color)),
                Span::styled(truncated_title, line_style.fg(title_color)),
                Span::styled(duration_str.clone(), line_style.fg(theme.fg_muted)),
            ];
            if !dep_info.is_empty() {
                spans.push(Span::styled(
                    dep_info.clone(),
                    Style::default().fg(theme.accent_warning),
                ));
            }
            lines.push(Line::from(spans));
        }
    }

    lines
}

/// Load PRD cache for all drones
pub fn load_prd_cache(drones: &[(String, DroneStatus)]) -> HashMap<String, Prd> {
    drones
        .iter()
        .filter_map(|(_, status)| {
            let prd_path = PathBuf::from(".hive").join("prds").join(&status.prd);
            load_prd(&prd_path).map(|prd| (status.prd.clone(), prd))
        })
        .collect()
}

/// Build display order for drones (active first, then archived)
pub fn build_display_order(
    drones: &[(String, DroneStatus)],
    prd_cache: &HashMap<String, Prd>,
) -> (Vec<usize>, usize) {
    let now = Utc::now();
    let mut display_order: Vec<usize> = Vec::new();
    let mut archived_order: Vec<usize> = Vec::new();

    for (idx, (_, status)) in drones.iter().enumerate() {
        if status.status == DroneState::Completed {
            let (valid_completed, prd_story_count) = prd_cache
                .get(&status.prd)
                .map(|prd| reconcile_progress_with_prd(status, prd))
                .unwrap_or((status.completed.len(), status.total));

            if valid_completed >= prd_story_count {
                let inactive_secs = parse_timestamp(&status.updated)
                    .map(|updated| now.signed_duration_since(updated).num_seconds())
                    .unwrap_or(0);

                if inactive_secs >= DEFAULT_INACTIVE_THRESHOLD_SECS {
                    archived_order.push(idx);
                    continue;
                }
            }
        }
        display_order.push(idx);
    }

    let active_count = display_order.len();
    display_order.extend(archived_order);
    (display_order, active_count)
}

/// Pre-expand drones based on their status
pub fn initial_expanded_drones(drones: &[(String, DroneStatus)]) -> HashSet<String> {
    drones
        .iter()
        .filter(|(_, status)| {
            matches!(
                status.status,
                DroneState::InProgress
                    | DroneState::Starting
                    | DroneState::Resuming
                    | DroneState::Blocked
                    | DroneState::Error
            )
        })
        .map(|(name, _)| name.clone())
        .collect()
}
