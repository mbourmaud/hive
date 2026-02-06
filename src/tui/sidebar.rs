use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::types::{DroneState, DroneStatus};

use super::monitor::DroneSnapshot;

pub struct SidebarState {
    pub selected_index: usize,
    pub expanded_drone: Option<String>,
    pub list_state: ListState,
    snapshot: DroneSnapshot,
}

impl Default for SidebarState {
    fn default() -> Self {
        Self::new()
    }
}

impl SidebarState {
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            expanded_drone: None,
            list_state: ListState::default(),
            snapshot: DroneSnapshot { drones: Vec::new() },
        }
    }

    pub fn refresh(&mut self) {
        if let Ok(snap) = DroneSnapshot::refresh() {
            self.snapshot = snap;
            // Clamp selected index
            if !self.snapshot.drones.is_empty() && self.selected_index >= self.snapshot.drones.len()
            {
                self.selected_index = self.snapshot.drones.len() - 1;
            }
        }
        self.list_state.select(Some(self.selected_index));
    }

    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.list_state.select(Some(self.selected_index));
        }
    }

    pub fn move_down(&mut self) {
        if self.selected_index + 1 < self.snapshot.drones.len() {
            self.selected_index += 1;
            self.list_state.select(Some(self.selected_index));
        }
    }

    pub fn toggle_expand(&mut self) {
        if let Some((name, _)) = self.snapshot.drones.get(self.selected_index) {
            if self.expanded_drone.as_ref() == Some(name) {
                self.expanded_drone = None;
            } else {
                self.expanded_drone = Some(name.clone());
            }
        }
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect, is_focused: bool) {
        let border_style = if is_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        // Header with Hive logo + drone list below
        let inner_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        // Hive logo header
        let header = Paragraph::new(Line::from(vec![
            Span::styled(
                "  HIVE ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("({})", self.snapshot.drones.len()),
                Style::default().fg(Color::DarkGray),
            ),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(" Drones "),
        );
        f.render_widget(header, inner_layout[0]);

        // Drone list
        let items: Vec<ListItem> = self
            .snapshot
            .drones
            .iter()
            .enumerate()
            .flat_map(|(idx, (name, status))| {
                let mut items = vec![render_drone_item(name, status, idx == self.selected_index)];

                // If this drone is expanded, show its stories
                if self.expanded_drone.as_ref() == Some(name) {
                    for story_id in &status.completed {
                        items.push(ListItem::new(Line::from(vec![
                            Span::raw("  "),
                            Span::styled("\u{2713} ", Style::default().fg(Color::Green)),
                            Span::styled(
                                truncate(story_id, 15),
                                Style::default().fg(Color::DarkGray),
                            ),
                        ])));
                    }
                    if let Some(ref current) = status.current_story {
                        items.push(ListItem::new(Line::from(vec![
                            Span::raw("  "),
                            Span::styled("\u{25b8} ", Style::default().fg(Color::Yellow)),
                            Span::styled(truncate(current, 15), Style::default().fg(Color::Yellow)),
                        ])));
                    }
                }

                items
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        f.render_stateful_widget(list, inner_layout[1], &mut self.list_state);
    }

    pub fn selected_drone(&self) -> Option<&(String, DroneStatus)> {
        self.snapshot.drones.get(self.selected_index)
    }
}

fn render_drone_item(name: &str, status: &DroneStatus, _is_selected: bool) -> ListItem<'static> {
    let status_icon = match status.status {
        DroneState::Starting | DroneState::Resuming => "\u{25cc}",
        DroneState::InProgress => "\u{25cf}",
        DroneState::Completed => "\u{2713}",
        DroneState::Error => "\u{2717}",
        DroneState::Blocked => "\u{26a0}",
        DroneState::Stopped => "\u{25cb}",
    };

    let status_color = match status.status {
        DroneState::Starting | DroneState::Resuming => Color::Yellow,
        DroneState::InProgress => Color::Green,
        DroneState::Completed => Color::Green,
        DroneState::Error => Color::Red,
        DroneState::Blocked => Color::Red,
        DroneState::Stopped => Color::DarkGray,
    };

    let progress = format!("{}/{}", status.completed.len(), status.total);
    let display_name = truncate(name, 12);

    ListItem::new(Line::from(vec![
        Span::styled(
            format!(" {} ", status_icon),
            Style::default().fg(status_color),
        ),
        Span::styled(
            format!("{:<12}", display_name),
            Style::default().fg(Color::Cyan),
        ),
        Span::styled(
            format!(" {}", progress),
            Style::default().fg(Color::DarkGray),
        ),
    ]))
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}\u{2026}", &s[..max_len - 1])
    } else {
        s.to_string()
    }
}
