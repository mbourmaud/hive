/// Sidebar widget for the Hive TUI
/// Displays drone list with status, progress bars, and story details
use crate::tui::monitor;
use crate::tui::theme::Theme;
use crate::types::{DroneStatus, Prd};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use std::collections::{HashMap, HashSet};

pub struct SidebarState {
    pub selected_index: usize,
    pub selected_story_index: Option<usize>,
    pub expanded_drones: HashSet<String>,
    pub scroll_offset: usize,
}

impl SidebarState {
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            selected_story_index: None,
            expanded_drones: HashSet::new(),
            scroll_offset: 0,
        }
    }

    /// Returns the name of the currently selected drone.
    /// Uses `display_order` to map `selected_index` to the actual drone entry.
    pub fn selected_drone_name(
        &self,
        drones: &[(String, DroneStatus)],
        display_order: &[usize],
    ) -> Option<String> {
        if display_order.is_empty() || self.selected_index >= display_order.len() {
            return None;
        }
        let drone_idx = display_order[self.selected_index];
        drones.get(drone_idx).map(|(name, _)| name.clone())
    }
}

impl Default for SidebarState {
    fn default() -> Self {
        Self::new()
    }
}

/// Render the sidebar with drone list
#[allow(clippy::too_many_arguments)]
pub fn render_sidebar(
    f: &mut Frame,
    area: Rect,
    state: &mut SidebarState,
    drones: &[(String, DroneStatus)],
    prd_cache: &HashMap<String, Prd>,
    display_order: &[usize],
    active_count: usize,
    theme: &Theme,
) {
    let mut lines: Vec<Line> = Vec::new();
    let mut drone_line_indices: Vec<usize> = Vec::new();

    // Show placeholder when no drones
    if drones.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![Span::styled(
            "  No drones running",
            Style::default()
                .fg(theme.fg_muted)
                .add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "Press 'n' to create",
                Style::default().fg(theme.accent_warning),
            ),
        ]));
    }

    // Render ACTIVE section
    if active_count > 0 {
        lines.push(Line::from(vec![
            Span::styled(
                "  \u{1f36f} ACTIVE",
                Style::default()
                    .fg(theme.accent_warning)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" ({})", active_count),
                Style::default().fg(theme.fg_muted),
            ),
        ]));
        lines.push(Line::raw(""));
    }

    for (display_idx, &drone_idx) in display_order.iter().enumerate() {
        // Add ARCHIVED header before first archived drone
        if display_idx == active_count && active_count < display_order.len() {
            lines.push(Line::styled(
                "  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
                Style::default().fg(theme.fg_muted),
            ));
            lines.push(Line::raw(""));
            lines.push(Line::from(vec![
                Span::styled(
                    "  \u{1f43b} ARCHIVED",
                    Style::default()
                        .fg(theme.fg_muted)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" ({})", display_order.len() - active_count),
                    Style::default().fg(theme.fg_muted),
                ),
            ]));
            lines.push(Line::raw(""));
        }

        let (name, status) = &drones[drone_idx];
        drone_line_indices.push(lines.len());

        let is_selected = display_idx == state.selected_index;
        let is_expanded = state.expanded_drones.contains(name);
        let prd = prd_cache.get(&status.prd);

        // Render drone header line
        let drone_lines =
            monitor::render_drone_line(name, status, is_selected, is_expanded, prd, &area, theme);
        lines.extend(drone_lines);

        // Render stories if expanded
        if is_expanded {
            if let Some(prd) = prd {
                let story_lines = monitor::render_drone_stories(
                    name,
                    status,
                    prd,
                    state.selected_story_index,
                    is_selected,
                    &area,
                    theme,
                );
                lines.extend(story_lines);
            }
        }

        // Add separator between drones
        lines.push(Line::raw(""));
    }

    // Calculate visible area and scroll
    let content_height = area.height as usize;
    let total_lines = lines.len();

    // Ensure selected drone is visible
    if !drone_line_indices.is_empty() && state.selected_index < drone_line_indices.len() {
        let selected_line = drone_line_indices[state.selected_index];
        if selected_line < state.scroll_offset {
            state.scroll_offset = selected_line;
        } else if selected_line >= state.scroll_offset + content_height.saturating_sub(2) {
            state.scroll_offset = selected_line.saturating_sub(content_height.saturating_sub(3));
        }
    }

    // Render visible lines
    let visible_lines: Vec<Line> = lines
        .into_iter()
        .skip(state.scroll_offset)
        .take(content_height)
        .collect();

    let content = Paragraph::new(visible_lines);
    f.render_widget(content, area);

    // Scrollbar if needed
    if total_lines > content_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(Some("\u{2502}"))
            .thumb_symbol("\u{2588}");

        let mut scrollbar_state = ScrollbarState::new(total_lines)
            .position(state.scroll_offset)
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

/// Handle navigation keys in sidebar
pub fn handle_navigation(
    state: &mut SidebarState,
    key: char,
    drones: &[(String, DroneStatus)],
    prd_cache: &HashMap<String, Prd>,
    display_order: &[usize],
) {
    let current_drone_idx =
        if !display_order.is_empty() && state.selected_index < display_order.len() {
            display_order[state.selected_index]
        } else {
            0
        };

    let current_story_count = if !drones.is_empty() && current_drone_idx < drones.len() {
        let drone_name = &drones[current_drone_idx].0;
        let status = &drones[current_drone_idx].1;
        if state.expanded_drones.contains(drone_name) {
            prd_cache
                .get(&status.prd)
                .map(|p| p.stories.len())
                .unwrap_or(0)
        } else {
            0
        }
    } else {
        0
    };

    match key {
        'j' => {
            // Navigate down
            if !drones.is_empty() {
                if let Some(story_idx) = state.selected_story_index {
                    // Navigate within stories
                    if story_idx < current_story_count.saturating_sub(1) {
                        state.selected_story_index = Some(story_idx + 1);
                    }
                } else if state.selected_index < drones.len() - 1 {
                    // Navigate between drones
                    state.selected_index += 1;
                    state.selected_story_index = None;
                }
            }
        }
        'k' => {
            // Navigate up
            if let Some(story_idx) = state.selected_story_index {
                // Navigate within stories
                if story_idx > 0 {
                    state.selected_story_index = Some(story_idx - 1);
                } else {
                    // Go back to drone header
                    state.selected_story_index = None;
                }
            } else {
                // Navigate between drones
                state.selected_index = state.selected_index.saturating_sub(1);
            }
        }
        _ => {}
    }
}

/// Toggle expansion of selected drone
pub fn toggle_expansion(
    state: &mut SidebarState,
    drones: &[(String, DroneStatus)],
    display_order: &[usize],
    prd_cache: &HashMap<String, Prd>,
) {
    if drones.is_empty() {
        return;
    }

    let current_drone_idx = if state.selected_index < display_order.len() {
        display_order[state.selected_index]
    } else {
        return;
    };

    if current_drone_idx >= drones.len() {
        return;
    }

    let drone_name = &drones[current_drone_idx].0;
    let status = &drones[current_drone_idx].1;

    if state.expanded_drones.contains(drone_name) {
        // Collapse
        state.expanded_drones.remove(drone_name);
        state.selected_story_index = None;
    } else {
        // Expand
        state.expanded_drones.insert(drone_name.clone());
        // Optionally, select first story if exists
        if let Some(prd) = prd_cache.get(&status.prd) {
            if !prd.stories.is_empty() {
                // Don't auto-select story, just expand
            }
        }
    }
}
