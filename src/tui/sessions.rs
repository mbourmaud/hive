use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::session_store::SessionMeta;

pub struct SessionList {
    pub visible: bool,
    pub sessions: Vec<SessionMeta>,
    pub selected: usize,
    pub list_state: ListState,
}

#[derive(Debug)]
pub enum SessionAction {
    Switch(SessionMeta),
    NewSession,
    Resume(SessionMeta),
    Close,
}

impl Default for SessionList {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionList {
    pub fn new() -> Self {
        Self {
            visible: false,
            sessions: Vec::new(),
            selected: 0,
            list_state: ListState::default(),
        }
    }

    pub fn show(&mut self, sessions: Vec<SessionMeta>) {
        self.sessions = sessions;
        self.selected = 0;
        self.list_state.select(Some(0));
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<SessionAction> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.hide();
                Some(SessionAction::Close)
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.sessions.is_empty() && self.selected + 1 < self.sessions.len() {
                    self.selected += 1;
                    self.list_state.select(Some(self.selected));
                }
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                    self.list_state.select(Some(self.selected));
                }
                None
            }
            KeyCode::Enter => {
                if let Some(session) = self.sessions.get(self.selected) {
                    let action = if session.claude_session_id.is_some() {
                        SessionAction::Resume(session.clone())
                    } else {
                        SessionAction::Switch(session.clone())
                    };
                    self.hide();
                    Some(action)
                } else {
                    None
                }
            }
            KeyCode::Char('n') => {
                self.hide();
                Some(SessionAction::NewSession)
            }
            _ => None,
        }
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let dialog_area = centered_rect(60, 70, area);
        f.render_widget(Clear, dialog_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(1), // Title
                Constraint::Length(1), // Blank
                Constraint::Min(0),    // List
                Constraint::Length(1), // Hints
            ])
            .split(dialog_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Sessions ");
        f.render_widget(block, dialog_area);

        let title = Paragraph::new(Line::from(vec![
            Span::styled(
                "Session Manager",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" ({} sessions)", self.sessions.len()),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        f.render_widget(title, chunks[0]);

        // Session list
        let items: Vec<ListItem> = self
            .sessions
            .iter()
            .map(|s| {
                let time_str = s.updated_at.format("%m/%d %H:%M").to_string();
                let resumable = if s.claude_session_id.is_some() {
                    "\u{21bb} "
                } else {
                    "  "
                };
                ListItem::new(Line::from(vec![
                    Span::styled(resumable.to_string(), Style::default().fg(Color::Green)),
                    Span::styled(truncate(&s.title, 30), Style::default().fg(Color::White)),
                    Span::styled(
                        format!("  {}", time_str),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(
                        format!("  {}msg", s.message_count),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]))
            })
            .collect();

        let list = List::new(items).highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

        f.render_stateful_widget(list, chunks[2], &mut self.list_state);

        // Hints
        let hints = Paragraph::new(Line::from(vec![
            Span::styled(" n ", Style::default().fg(Color::Black).bg(Color::DarkGray)),
            Span::styled(" New  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                " Enter ",
                Style::default().fg(Color::Black).bg(Color::DarkGray),
            ),
            Span::styled(" Open  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                " Esc ",
                Style::default().fg(Color::Black).bg(Color::DarkGray),
            ),
            Span::styled(" Close ", Style::default().fg(Color::DarkGray)),
        ]));
        f.render_widget(hints, chunks[3]);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}\u{2026}", &s[..max_len - 1])
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_list_new() {
        let list = SessionList::new();
        assert!(!list.visible);
        assert!(list.sessions.is_empty());
    }

    #[test]
    fn test_session_list_show() {
        let mut list = SessionList::new();
        let meta = SessionMeta {
            id: "test".to_string(),
            title: "Test".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            message_count: 5,
            claude_session_id: None,
        };
        list.show(vec![meta]);
        assert!(list.visible);
        assert_eq!(list.sessions.len(), 1);
        assert_eq!(list.selected, 0);
    }

    #[test]
    fn test_session_list_hide() {
        let mut list = SessionList::new();
        list.visible = true;
        list.hide();
        assert!(!list.visible);
    }
}
