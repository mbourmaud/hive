mod drone_row;
mod expanded;
pub(crate) mod helpers;
mod tasks;
mod team_line;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::commands::common::{
    is_process_running, parse_timestamp, read_drone_pid, DEFAULT_INACTIVE_THRESHOLD_SECS,
};
use crate::types::DroneState;

use super::state::TuiState;
use super::views::render_messages_view;
use super::views::render_tools_view;

use drone_row::{
    build_drone_header_line, render_active_header, render_archived_header,
    render_empty_placeholder, render_header, render_scrollbar,
};

impl TuiState {
    pub fn render(&mut self, f: &mut Frame) {
        let area = f.area();

        // Tools view (toggle with 't')
        if let Some(ref drone_name) = self.tools_view {
            let tool_history = self.tool_history.get(drone_name);
            render_tools_view(f, area, drone_name, tool_history);
            return;
        }

        // Messages view (toggle with 'm')
        if let Some(ref drone_name) = self.messages_view {
            render_messages_view(f, area, drone_name, self.messages_selected_index);
            return;
        }

        // Main layout: header, content, footer
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5), // Header with ASCII art + padding
                Constraint::Min(0),    // Content
                Constraint::Length(1), // Footer
            ])
            .split(area);

        render_header(f, chunks[0], self.drones.len());

        let mut lines: Vec<Line> = Vec::new();
        let mut drone_line_indices: Vec<usize> = Vec::new();

        if self.drones.is_empty() {
            render_empty_placeholder(&mut lines);
        }

        let (active_count, display_order) = self.build_active_count_and_order();

        render_active_header(&mut lines, active_count);
        self.render_drone_list(
            &mut lines,
            &mut drone_line_indices,
            &display_order,
            active_count,
            area,
        );

        // Scroll + render visible lines
        let content_height = chunks[1].height as usize;
        let total_lines = lines.len();

        self.ensure_selected_visible(&drone_line_indices, content_height);

        let visible_lines: Vec<Line> = lines
            .into_iter()
            .skip(self.scroll_offset)
            .take(content_height)
            .collect();

        f.render_widget(Paragraph::new(visible_lines), chunks[1]);
        render_scrollbar(
            f,
            chunks[1],
            total_lines,
            content_height,
            self.scroll_offset,
        );
        self.render_footer(f, chunks[2]);
    }

    fn build_active_count_and_order(&self) -> (usize, Vec<usize>) {
        let now = chrono::Utc::now();
        let active_count = self
            .display_order
            .iter()
            .take_while(|&&idx| {
                let status = &self.drones[idx].1;
                if status.status != DroneState::Completed {
                    return true;
                }
                let inactive_secs = parse_timestamp(&status.updated)
                    .map(|updated| now.signed_duration_since(updated).num_seconds())
                    .unwrap_or(0);
                inactive_secs < DEFAULT_INACTIVE_THRESHOLD_SECS
            })
            .count();
        (active_count, self.display_order.clone())
    }

    fn render_drone_list(
        &mut self,
        lines: &mut Vec<Line>,
        drone_line_indices: &mut Vec<usize>,
        display_order: &[usize],
        active_count: usize,
        area: Rect,
    ) {
        for (display_idx, &drone_idx) in display_order.iter().enumerate() {
            if display_idx == active_count && active_count < self.display_order.len() {
                render_archived_header(lines, self.display_order.len() - active_count);
            }

            drone_line_indices.push(lines.len());
            self.render_drone_row(lines, drone_idx, display_idx, area);
        }
    }

    fn render_drone_row(
        &mut self,
        lines: &mut Vec<Line>,
        drone_idx: usize,
        display_idx: usize,
        area: Rect,
    ) {
        let (name, status) = &self.drones[drone_idx];
        let is_selected = display_idx == self.selected_index;
        let is_expanded = self.expanded_drones.contains(name);
        let process_running = read_drone_pid(name)
            .map(is_process_running)
            .unwrap_or(false);

        // Auto-stop: if drone is completed but process still running
        if process_running
            && status.status == DroneState::Completed
            && !self.auto_stopped_drones.contains(name)
        {
            self.auto_stopped_drones.insert(name.clone());
            let _ = crate::commands::kill_clean::kill_quiet(name.clone());
        }

        let header_line =
            build_drone_header_line(self, drone_idx, is_selected, is_expanded, process_running);
        lines.push(header_line);

        if is_expanded {
            expanded::render_expanded_drone(self, lines, drone_idx, area);
        }

        lines.push(Line::raw(""));
    }

    fn ensure_selected_visible(&mut self, drone_line_indices: &[usize], content_height: usize) {
        if !drone_line_indices.is_empty() && self.selected_index < drone_line_indices.len() {
            let selected_line = drone_line_indices[self.selected_index];
            if selected_line < self.scroll_offset {
                self.scroll_offset = selected_line;
            } else if selected_line >= self.scroll_offset + content_height.saturating_sub(2) {
                self.scroll_offset = selected_line.saturating_sub(content_height.saturating_sub(3));
            }
        }
    }

    fn render_footer(&self, f: &mut Frame, area: Rect) {
        let footer_text = self.message.clone().unwrap_or_else(|| {
            " ↵ expand  t tools  m msgs  x stop  hold D clean  r resume  q quit".to_string()
        });

        let footer = Paragraph::new(Line::from(vec![Span::styled(
            footer_text,
            Style::default().fg(if self.message.is_some() {
                self.message_color
            } else {
                Color::DarkGray
            }),
        )]));
        f.render_widget(footer, area);
    }
}
