use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyModifiers};
use tui_textarea::TextArea;

/// State for the chat input widget
pub struct InputState {
    /// The text area widget
    pub textarea: TextArea<'static>,
    /// Input history for Up/Down navigation
    pub history: Vec<String>,
    /// Current position in history (None = not navigating)
    pub history_index: Option<usize>,
    /// Temporary storage for current input when navigating history
    pub temp_input: String,
}

impl InputState {
    pub fn new() -> Self {
        let mut textarea = TextArea::default();
        textarea.set_placeholder_text("Type your message here... (Ctrl+Enter to submit, @ for file autocomplete)");
        
        Self {
            textarea,
            history: Vec::new(),
            history_index: None,
            temp_input: String::new(),
        }
    }

    /// Handle keyboard events for the input area
    /// Returns Some(message) if user submitted a message
    pub fn handle_event(&mut self, event: Event) -> Result<Option<String>> {
        if let Event::Key(key) = event {
            // Handle Ctrl+Enter for submit
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Enter {
                return self.submit();
            }

            // Handle history navigation with Up/Down
            if key.modifiers.is_empty() {
                match key.code {
                    KeyCode::Up => {
                        self.navigate_history_up();
                        return Ok(None);
                    }
                    KeyCode::Down => {
                        self.navigate_history_down();
                        return Ok(None);
                    }
                    _ => {}
                }
            }

            // Handle Emacs-style shortcuts
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                match key.code {
                    KeyCode::Char('a') => {
                        // Move to start of line
                        self.textarea.move_cursor(tui_textarea::CursorMove::Head);
                        return Ok(None);
                    }
                    KeyCode::Char('e') => {
                        // Move to end of line
                        self.textarea.move_cursor(tui_textarea::CursorMove::End);
                        return Ok(None);
                    }
                    KeyCode::Char('k') => {
                        // Delete from cursor to end of line
                        self.textarea.delete_line_by_end();
                        return Ok(None);
                    }
                    KeyCode::Char('u') => {
                        // Delete from cursor to start of line
                        self.textarea.delete_line_by_head();
                        return Ok(None);
                    }
                    _ => {}
                }
            }

            // Pass event to textarea for normal editing
            self.textarea.input(event);

            // Reset history navigation if user types something
            if !matches!(key.code, KeyCode::Up | KeyCode::Down) {
                self.history_index = None;
            }
        }

        Ok(None)
    }

    /// Submit the current input
    fn submit(&mut self) -> Result<Option<String>> {
        let text = self.textarea.lines().join("\n");

        if !text.trim().is_empty() {
            // Add to history
            self.history.push(text.clone());

            // Clear the input
            self.textarea = TextArea::default();
            self.textarea.set_placeholder_text("Type your message here... (Ctrl+Enter to submit, @ for file autocomplete)");

            // Reset history navigation
            self.history_index = None;
            self.temp_input.clear();

            return Ok(Some(text));
        }

        Ok(None)
    }

    /// Navigate up in history
    fn navigate_history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }

        match self.history_index {
            None => {
                // First time navigating, save current input
                self.temp_input = self.textarea.lines().join("\n");
                self.history_index = Some(self.history.len() - 1);
            }
            Some(idx) if idx > 0 => {
                self.history_index = Some(idx - 1);
            }
            _ => return, // Already at the oldest
        }

        if let Some(idx) = self.history_index {
            let content = self.history[idx].clone();
            self.set_textarea_content(&content);
        }
    }

    /// Navigate down in history
    fn navigate_history_down(&mut self) {
        let Some(idx) = self.history_index else {
            return;
        };

        if idx + 1 < self.history.len() {
            self.history_index = Some(idx + 1);
            let content = self.history[idx + 1].clone();
            self.set_textarea_content(&content);
        } else {
            // Restore temporary input
            self.history_index = None;
            let temp = self.temp_input.clone();
            self.set_textarea_content(&temp);
        }
    }

    /// Set the textarea content
    fn set_textarea_content(&mut self, content: &str) {
        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        self.textarea = TextArea::new(lines);
        self.textarea.set_placeholder_text("Type your message here... (Ctrl+Enter to submit, @ for file autocomplete)");
    }

    /// Get the current input text
    pub fn get_text(&self) -> String {
        self.textarea.lines().join("\n")
    }
}
