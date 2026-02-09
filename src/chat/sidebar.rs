use std::collections::{HashMap, HashSet};

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::agent_teams::snapshot::TaskSnapshotStore;
use crate::events::EventReader;
use crate::types::DroneState;

use super::theme;

/// Info passed from the app to the sidebar for rendering.
pub struct SidebarSessionInfo {
    pub title: String,
    pub model: String,
    pub duration: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_cost_usd: f64,
    pub is_streaming: bool,
}

pub struct Sidebar {
    /// Drone event readers (for incremental updates)
    event_readers: HashMap<String, EventReader>,
    /// Task snapshot store (monotonic progress tracking)
    snapshot_store: TaskSnapshotStore,
    /// Cached cost per drone
    #[allow(dead_code)]
    cost_cache: HashMap<String, f64>,
    /// Section collapse state
    #[allow(dead_code)]
    collapsed_sections: HashSet<String>,
}

impl Default for Sidebar {
    fn default() -> Self {
        Self::new()
    }
}

impl Sidebar {
    pub fn new() -> Self {
        Self {
            event_readers: HashMap::new(),
            snapshot_store: TaskSnapshotStore::new(),
            cost_cache: HashMap::new(),
            collapsed_sections: HashSet::new(),
        }
    }

    /// Update drone data (called from app tick).
    pub fn tick(&mut self) -> anyhow::Result<()> {
        let drones = crate::commands::common::list_drones().unwrap_or_default();

        for (name, _status) in &drones {
            // Update snapshot store
            self.snapshot_store.update(name);

            // Initialize event reader if needed
            if !self.event_readers.contains_key(name) {
                self.event_readers
                    .insert(name.clone(), EventReader::new(name));
            }

            // Read new events (for cost tracking etc)
            if let Some(reader) = self.event_readers.get_mut(name) {
                let _events = reader.read_new();
            }
        }

        // Clean up event readers for removed drones
        self.event_readers
            .retain(|k, _| drones.iter().any(|(name, _)| name == k));

        Ok(())
    }

    /// Render the sidebar into the given area.
    pub fn render(&self, f: &mut Frame, area: Rect, session_info: &SidebarSessionInfo) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::BORDER))
            .title(" Sidebar ")
            .title_style(Style::default().fg(theme::ACCENT));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let mut lines: Vec<Line> = Vec::new();

        // Section 1: Session Info
        lines.push(Line::from(Span::styled(
            " Session Info".to_string(),
            Style::default()
                .fg(theme::PRIMARY)
                .add_modifier(Modifier::BOLD),
        )));
        let separator =
            " ".to_string() + &"\u{2500}".repeat((inner.width as usize).saturating_sub(3));
        lines.push(Line::from(Span::styled(
            separator.clone(),
            Style::default().fg(theme::DIM),
        )));
        lines.push(Line::from(vec![
            Span::styled("  Title: ".to_string(), Style::default().fg(theme::DIM)),
            Span::styled(
                session_info.title.clone(),
                Style::default().fg(Color::White),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Model: ".to_string(), Style::default().fg(theme::DIM)),
            Span::styled(
                session_info.model.clone(),
                Style::default().fg(Color::White),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Duration: ".to_string(), Style::default().fg(theme::DIM)),
            Span::styled(
                session_info.duration.clone(),
                Style::default().fg(Color::White),
            ),
        ]));
        lines.push(Line::raw(""));

        // Section 2: Context
        lines.push(Line::from(Span::styled(
            " Context".to_string(),
            Style::default()
                .fg(theme::PRIMARY)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            separator.clone(),
            Style::default().fg(theme::DIM),
        )));
        lines.push(Line::from(vec![
            Span::styled("  Input:  ".to_string(), Style::default().fg(theme::DIM)),
            Span::styled(
                format_tokens(session_info.input_tokens),
                Style::default().fg(Color::White),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Output: ".to_string(), Style::default().fg(theme::DIM)),
            Span::styled(
                format_tokens(session_info.output_tokens),
                Style::default().fg(Color::White),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Cost:   ".to_string(), Style::default().fg(theme::DIM)),
            Span::styled(
                format!("${:.2}", session_info.total_cost_usd),
                Style::default().fg(theme::SECONDARY),
            ),
        ]));

        // Token usage bar
        let total = session_info.input_tokens + session_info.output_tokens;
        let max_tokens: u64 = 200_000;
        let bar_width = (inner.width as usize).saturating_sub(4);
        let filled = ((total as f64 / max_tokens as f64) * bar_width as f64) as usize;
        let filled = filled.min(bar_width);
        let empty = bar_width.saturating_sub(filled);

        lines.push(Line::from(vec![
            Span::raw("  ".to_string()),
            Span::styled(
                "\u{2588}".repeat(filled),
                Style::default().fg(theme::PRIMARY),
            ),
            Span::styled("\u{2591}".repeat(empty), Style::default().fg(theme::DIM)),
        ]));
        lines.push(Line::from(Span::styled(
            format!("  {}k/{}k", total / 1000, max_tokens / 1000),
            Style::default().fg(theme::DIM),
        )));
        lines.push(Line::raw(""));

        // Section 3: Active Drones
        self.render_drones(&mut lines, &separator);

        // Section 4: Modified Files (stub)
        lines.push(Line::from(Span::styled(
            " Modified Files".to_string(),
            Style::default()
                .fg(theme::PRIMARY)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            separator,
            Style::default().fg(theme::DIM),
        )));
        lines.push(Line::from(Span::styled(
            "  (tracked via tool events)".to_string(),
            Style::default().fg(theme::DIM),
        )));

        let paragraph = Paragraph::new(lines);
        f.render_widget(paragraph, inner);
    }

    /// Toggle a section's collapsed state.
    #[allow(dead_code)]
    pub fn toggle_section(&mut self, section: &str) {
        if self.collapsed_sections.contains(section) {
            self.collapsed_sections.remove(section);
        } else {
            self.collapsed_sections.insert(section.to_string());
        }
    }

    fn render_drones(&self, lines: &mut Vec<Line>, separator: &str) {
        let drones = crate::commands::common::list_drones().unwrap_or_default();

        lines.push(Line::from(Span::styled(
            format!(" Drones ({})", drones.len()),
            Style::default()
                .fg(theme::PRIMARY)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            separator.to_string(),
            Style::default().fg(theme::DIM),
        )));

        if drones.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No active drones".to_string(),
                Style::default().fg(theme::DIM),
            )));
            lines.push(Line::raw(""));
            return;
        }

        for (name, status) in &drones {
            let (completed, total) = self.snapshot_store.progress(name);

            let (icon, color) = match status.status {
                DroneState::InProgress => ("\u{25D0}", Color::Green),
                DroneState::Completed => ("\u{25CF}", Color::Green),
                DroneState::Error => ("\u{25D0}", Color::Red),
                DroneState::Starting | DroneState::Resuming => ("\u{25D0}", Color::Yellow),
                _ => ("\u{25CB}", Color::DarkGray),
            };

            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", icon), Style::default().fg(color)),
                Span::styled(
                    format!("{:<16}", truncate_str(name, 16)),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{}/{}", completed, total),
                    Style::default().fg(theme::DIM),
                ),
            ]));

            // Show tasks from snapshot
            if let Some(snapshot) = self.snapshot_store.get(name) {
                for task in &snapshot.tasks {
                    let (task_icon, task_color) = match task.status.as_str() {
                        "completed" => ("\u{25CF}", Color::Green),
                        "in_progress" => ("\u{25D0}", Color::Yellow),
                        _ => ("\u{25CB}", Color::DarkGray),
                    };

                    let mut task_spans = vec![
                        Span::styled(
                            format!("    {} ", task_icon),
                            Style::default().fg(task_color),
                        ),
                        Span::styled(
                            truncate_str(&task.subject, 25),
                            Style::default().fg(Color::White),
                        ),
                    ];

                    if let Some(ref owner) = task.owner {
                        task_spans.push(Span::raw(" ".to_string()));
                        task_spans.push(Span::styled(
                            format!("@{}", owner),
                            Style::default().fg(theme::ACCENT),
                        ));
                    }

                    lines.push(Line::from(task_spans));
                }
            }

            lines.push(Line::raw(""));
        }
    }
}

fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M tokens", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}k tokens", tokens as f64 / 1_000.0)
    } else {
        format!("{} tokens", tokens)
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        format!(
            "{}...",
            s.chars()
                .take(max_len.saturating_sub(3))
                .collect::<String>()
        )
    }
}
