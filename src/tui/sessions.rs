use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem};
use ratatui::Frame;

use super::session_store::{SavedSession, SessionStore};
use super::theme::Theme;

pub struct SessionManager {
    pub visible: bool,
    pub selected: usize,
    pub sessions: Vec<SavedSession>,
    store: SessionStore,
}

impl SessionManager {
    pub fn new() -> Self {
        let store = SessionStore::new();
        let sessions = store.list_sessions();
        Self {
            visible: false,
            selected: 0,
            sessions,
            store,
        }
    }

    pub fn show(&mut self) {
        self.sessions = self.store.list_sessions();
        self.visible = true;
        self.selected = 0;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn select_next(&mut self) {
        if !self.sessions.is_empty() {
            self.selected = (self.selected + 1).min(self.sessions.len() - 1);
        }
    }

    pub fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn selected_session(&self) -> Option<&SavedSession> {
        self.sessions.get(self.selected)
    }

    pub fn store(&self) -> &SessionStore {
        &self.store
    }

    pub fn render(&self, frame: &mut Frame, theme: &Theme) {
        let area = super::dialogs::centered_rect(60, 60, frame.area());
        frame.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border_style(true))
            .title(" Sessions ")
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.sessions.is_empty() {
            let empty = List::new(vec![ListItem::new(Line::from(Span::styled(
                "No saved sessions",
                theme.muted_style(),
            )))]);
            frame.render_widget(empty, inner);
            return;
        }

        let items: Vec<ListItem> = self
            .sessions
            .iter()
            .enumerate()
            .map(|(i, session)| {
                let is_selected = i == self.selected;
                let name_style = if is_selected {
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD | Modifier::REVERSED)
                } else {
                    Style::default().fg(theme.fg)
                };

                ListItem::new(Line::from(vec![
                    Span::styled(&session.title, name_style),
                    Span::styled(
                        format!("  ({} msgs)", session.message_count),
                        theme.muted_style(),
                    ),
                    Span::styled(format!("  {}", session.created_at), theme.muted_style()),
                ]))
            })
            .collect();

        let list = List::new(items);
        frame.render_widget(list, inner);
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}
