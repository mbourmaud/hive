use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::Frame;
use tui_textarea::TextArea;

pub struct InputWidget<'a> {
    pub textarea: TextArea<'a>,
    history: Vec<String>,
    history_index: Option<usize>,
    saved_input: String,
}

pub enum InputAction {
    Submit(String),
    FilePickerTrigger,
    None,
}

impl<'a> InputWidget<'a> {
    pub fn new() -> Self {
        let mut textarea = TextArea::default();
        textarea.set_cursor_line_style(ratatui::style::Style::default());
        textarea.set_placeholder_text(
            "Type a message... (Ctrl+S to send, @ for files, / for commands)",
        );
        Self {
            textarea,
            history: Vec::new(),
            history_index: None,
            saved_input: String::new(),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> InputAction {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match (ctrl, key.code) {
            // Submit: Ctrl+S or Ctrl+Enter
            (true, KeyCode::Char('s')) | (true, KeyCode::Enter) => {
                let text = self.text().to_string();
                if text.trim().is_empty() {
                    return InputAction::None;
                }
                self.history.push(text.clone());
                self.history_index = None;
                self.textarea.select_all();
                self.textarea.cut();
                InputAction::Submit(text)
            }
            // Emacs: Ctrl+A -> beginning of line
            (true, KeyCode::Char('a')) => {
                self.textarea.move_cursor(tui_textarea::CursorMove::Head);
                InputAction::None
            }
            // Emacs: Ctrl+E -> end of line
            (true, KeyCode::Char('e')) => {
                self.textarea.move_cursor(tui_textarea::CursorMove::End);
                InputAction::None
            }
            // Emacs: Ctrl+K -> kill to end of line
            (true, KeyCode::Char('k')) => {
                self.textarea.delete_line_by_end();
                InputAction::None
            }
            // Emacs: Ctrl+U -> kill to beginning of line
            (true, KeyCode::Char('u')) => {
                self.textarea.delete_line_by_head();
                InputAction::None
            }
            // History: Up arrow
            (false, KeyCode::Up) => {
                if !self.history.is_empty() {
                    if self.history_index.is_none() {
                        self.saved_input = self.text().to_string();
                        self.history_index = Some(self.history.len() - 1);
                    } else if let Some(idx) = self.history_index {
                        if idx > 0 {
                            self.history_index = Some(idx - 1);
                        }
                    }
                    if let Some(idx) = self.history_index {
                        self.set_text(&self.history[idx].clone());
                    }
                }
                InputAction::None
            }
            // History: Down arrow
            (false, KeyCode::Down) => {
                if let Some(idx) = self.history_index {
                    if idx + 1 < self.history.len() {
                        self.history_index = Some(idx + 1);
                        self.set_text(&self.history[idx + 1].clone());
                    } else {
                        self.history_index = None;
                        let saved = self.saved_input.clone();
                        self.set_text(&saved);
                    }
                }
                InputAction::None
            }
            // @ trigger file picker
            (false, KeyCode::Char('@')) => {
                self.textarea.input(key);
                InputAction::FilePickerTrigger
            }
            // Default: pass to textarea
            _ => {
                self.textarea.input(key);
                InputAction::None
            }
        }
    }

    pub fn text(&self) -> String {
        self.textarea.lines().join("\n")
    }

    pub fn set_text(&mut self, text: &str) {
        self.textarea.select_all();
        self.textarea.cut();
        self.textarea.insert_str(text);
    }

    pub fn set_focus_style(&mut self, focused: bool) {
        let color = if focused {
            ratatui::style::Color::Cyan
        } else {
            ratatui::style::Color::DarkGray
        };
        self.textarea.set_block(
            ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .border_style(ratatui::style::Style::default().fg(color))
                .title(" Input "),
        );
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_widget(&self.textarea, area);
    }

    pub fn insert_text(&mut self, text: &str) {
        self.textarea.insert_str(text);
    }
}

impl<'a> Default for InputWidget<'a> {
    fn default() -> Self {
        Self::new()
    }
}
