use crossterm::event::KeyEvent;
use tui_textarea::{CursorMove, Input, Key, TextArea};

pub struct ChatInput {
    textarea: TextArea<'static>,
    history: Vec<String>,
    history_index: Option<usize>,
}

fn make_textarea() -> TextArea<'static> {
    let mut textarea = TextArea::default();
    textarea.set_cursor_line_style(ratatui::style::Style::default());
    textarea.set_placeholder_text("Type a message... (Ctrl+Enter to send)");
    textarea.set_block(
        ratatui::widgets::Block::default()
            .borders(ratatui::widgets::Borders::ALL)
            .title(" Input "),
    );
    textarea
}

impl ChatInput {
    pub fn new() -> Self {
        Self {
            textarea: make_textarea(),
            history: Vec::new(),
            history_index: None,
        }
    }

    /// Handle a key event. Returns Some(text) if the user submitted a message.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<String> {
        let input = Input::from(key);

        match input {
            // Ctrl+Enter submits
            Input {
                key: Key::Enter,
                ctrl: true,
                ..
            } => {
                let text = self.textarea.lines().join("\n").trim().to_string();
                if !text.is_empty() {
                    self.history.push(text.clone());
                    self.history_index = None;
                    self.textarea = make_textarea();
                    return Some(text);
                }
                None
            }
            // Up arrow navigates history when textarea is empty or at first line
            Input {
                key: Key::Up, ..
            } if self.textarea.lines().len() <= 1 && !self.history.is_empty() => {
                let new_index = match self.history_index {
                    None => self.history.len() - 1,
                    Some(i) if i > 0 => i - 1,
                    Some(i) => i,
                };
                self.history_index = Some(new_index);
                let content = self.history[new_index].clone();
                self.set_content(&content);
                None
            }
            // Down arrow goes forward in history
            Input {
                key: Key::Down, ..
            } if self.history_index.is_some() => {
                let new_index = self.history_index.unwrap() + 1;
                if new_index < self.history.len() {
                    self.history_index = Some(new_index);
                    let content = self.history[new_index].clone();
                    self.set_content(&content);
                } else {
                    self.history_index = None;
                    self.set_content("");
                }
                None
            }
            // Emacs shortcuts
            Input {
                key: Key::Char('a'),
                ctrl: true,
                ..
            } => {
                self.textarea.move_cursor(CursorMove::Head);
                None
            }
            Input {
                key: Key::Char('e'),
                ctrl: true,
                ..
            } => {
                self.textarea.move_cursor(CursorMove::End);
                None
            }
            Input {
                key: Key::Char('k'),
                ctrl: true,
                ..
            } => {
                self.textarea.delete_line_by_end();
                None
            }
            Input {
                key: Key::Char('u'),
                ctrl: true,
                ..
            } => {
                self.textarea.delete_line_by_head();
                None
            }
            // Default: pass to textarea
            input => {
                self.textarea.input(input);
                None
            }
        }
    }

    fn set_content(&mut self, content: &str) {
        self.textarea = TextArea::new(content.lines().map(String::from).collect());
        self.textarea
            .set_cursor_line_style(ratatui::style::Style::default());
        self.textarea
            .set_placeholder_text("Type a message... (Ctrl+Enter to send)");
        self.textarea.set_block(
            ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .title(" Input "),
        );
        // Move cursor to end
        self.textarea.move_cursor(CursorMove::Bottom);
        self.textarea.move_cursor(CursorMove::End);
    }

    pub fn widget(&self) -> &TextArea<'static> {
        &self.textarea
    }

    /// Check if input starts with @
    pub fn starts_with_at(&self) -> bool {
        self.textarea
            .lines()
            .first()
            .map_or(false, |l| l.starts_with('@'))
    }

    /// Get current text content
    pub fn text(&self) -> String {
        self.textarea.lines().join("\n")
    }
}
