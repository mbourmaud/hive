use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Row, Table},
    Frame,
};

use super::session_store::{Session, SessionStore};
use super::theme::Theme;

/// Action returned by the session list when the user interacts with it
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionAction {
    /// User selected a session to switch to
    Select(String),
    /// User wants to create a new session
    NewSession,
    /// User closed the session list
    Close,
}

/// Session list overlay widget for browsing and switching sessions
pub struct SessionList {
    /// Whether the overlay is visible
    pub visible: bool,
    /// Cached list of sessions
    pub sessions: Vec<Session>,
    /// Currently selected index
    pub selected_index: usize,
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
            selected_index: 0,
        }
    }

    /// Toggle visibility. When showing, refresh the session list from the store.
    pub fn toggle(&mut self, store: &SessionStore) {
        self.visible = !self.visible;
        if self.visible {
            self.sessions = store.list_sessions().unwrap_or_default();
            self.selected_index = 0;
        }
    }

    /// Show the session list (refresh from store).
    pub fn show(&mut self, store: &SessionStore) {
        self.visible = true;
        self.sessions = store.list_sessions().unwrap_or_default();
        self.selected_index = 0;
    }

    /// Handle a key event. Returns Some(action) if the user made a selection.
    pub fn handle_key(&mut self, code: KeyCode) -> Option<SessionAction> {
        match code {
            KeyCode::Esc => {
                self.visible = false;
                Some(SessionAction::Close)
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.sessions.is_empty() {
                    self.selected_index =
                        (self.selected_index + 1).min(self.sessions.len().saturating_sub(1));
                }
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected_index = self.selected_index.saturating_sub(1);
                None
            }
            KeyCode::Enter => {
                if let Some(session) = self.sessions.get(self.selected_index) {
                    let id = session.id.clone();
                    self.visible = false;
                    Some(SessionAction::Select(id))
                } else {
                    None
                }
            }
            KeyCode::Char('n') => {
                self.visible = false;
                Some(SessionAction::NewSession)
            }
            KeyCode::Char('d') => {
                // Delete selected session (returns None, caller should refresh)
                if let Some(session) = self.sessions.get(self.selected_index) {
                    let _id = session.id.clone();
                    // Removal is handled by the caller
                }
                None
            }
            _ => None,
        }
    }

    /// Render the session list as a centered overlay.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.visible {
            return;
        }

        // Center the overlay: 60% width, 50% height
        let overlay_width = ((area.width as f32) * 0.6) as u16;
        let overlay_height = ((area.height as f32) * 0.5) as u16;
        let x = area.x + (area.width.saturating_sub(overlay_width)) / 2;
        let y = area.y + (area.height.saturating_sub(overlay_height)) / 2;
        let overlay_area = Rect::new(x, y, overlay_width, overlay_height);

        // Clear background
        frame.render_widget(Clear, overlay_area);

        let block = Block::default()
            .title(" Sessions (n: new | Enter: select | Esc: close) ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_popup));

        if self.sessions.is_empty() {
            let inner = block.inner(overlay_area);
            frame.render_widget(block, overlay_area);
            let empty_msg = Line::from(Span::styled(
                "No sessions yet. Press 'n' to create one.",
                Style::default().fg(theme.fg_muted),
            ));
            frame.render_widget(ratatui::widgets::Paragraph::new(empty_msg), inner);
            return;
        }

        let header = Row::new(vec!["Title", "Msgs", "Last Updated"])
            .style(
                Style::default()
                    .fg(theme.accent_primary)
                    .add_modifier(Modifier::BOLD),
            )
            .bottom_margin(1);

        let rows: Vec<Row> = self
            .sessions
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let style = if i == self.selected_index {
                    Style::default()
                        .fg(theme.selection_fg)
                        .bg(theme.selection_bg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg_primary)
                };

                // Truncate title for display
                let title = if s.title.len() > 30 {
                    format!("{}...", &s.title[..27])
                } else {
                    s.title.clone()
                };

                // Format updated_at: show just date + time
                let updated = if s.updated_at.len() >= 16 {
                    s.updated_at[..16].to_string()
                } else {
                    s.updated_at.clone()
                };

                Row::new(vec![title, s.message_count.to_string(), updated]).style(style)
            })
            .collect();

        let widths = [
            Constraint::Percentage(50),
            Constraint::Percentage(15),
            Constraint::Percentage(35),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .block(block)
            .style(Style::default().bg(theme.bg_popup));

        frame.render_widget(table, overlay_area);
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
        assert_eq!(list.selected_index, 0);
    }

    #[test]
    fn test_handle_key_escape() {
        let mut list = SessionList::new();
        list.visible = true;
        let action = list.handle_key(KeyCode::Esc);
        assert_eq!(action, Some(SessionAction::Close));
        assert!(!list.visible);
    }

    #[test]
    fn test_handle_key_new_session() {
        let mut list = SessionList::new();
        list.visible = true;
        let action = list.handle_key(KeyCode::Char('n'));
        assert_eq!(action, Some(SessionAction::NewSession));
        assert!(!list.visible);
    }

    #[test]
    fn test_handle_key_navigate() {
        let mut list = SessionList::new();
        list.visible = true;
        list.sessions = vec![
            Session {
                id: "1".into(),
                title: "First".into(),
                created_at: String::new(),
                updated_at: String::new(),
                message_count: 0,
                last_message: None,
                is_active: false,
            },
            Session {
                id: "2".into(),
                title: "Second".into(),
                created_at: String::new(),
                updated_at: String::new(),
                message_count: 0,
                last_message: None,
                is_active: false,
            },
        ];

        assert_eq!(list.selected_index, 0);

        list.handle_key(KeyCode::Char('j'));
        assert_eq!(list.selected_index, 1);

        list.handle_key(KeyCode::Char('j'));
        assert_eq!(list.selected_index, 1); // Stays at max

        list.handle_key(KeyCode::Char('k'));
        assert_eq!(list.selected_index, 0);
    }

    #[test]
    fn test_handle_key_select() {
        let mut list = SessionList::new();
        list.visible = true;
        list.sessions = vec![Session {
            id: "abc-123".into(),
            title: "Test".into(),
            created_at: String::new(),
            updated_at: String::new(),
            message_count: 0,
            last_message: None,
            is_active: false,
        }];

        let action = list.handle_key(KeyCode::Enter);
        assert_eq!(action, Some(SessionAction::Select("abc-123".into())));
        assert!(!list.visible);
    }

    #[test]
    fn test_handle_key_enter_empty() {
        let mut list = SessionList::new();
        list.visible = true;
        // No sessions
        let action = list.handle_key(KeyCode::Enter);
        assert_eq!(action, None);
    }
}
