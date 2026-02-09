use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::theme;

#[derive(Debug, Clone, PartialEq)]
pub enum InputAction {
    /// Message submitted (Enter pressed)
    Submit(String),
    /// Continue editing (any other key)
    Continue,
    /// Nothing happened (unhandled key)
    Noop,
}

pub struct InputEditor {
    /// Lines of text (each line is a String)
    lines: Vec<String>,
    /// Cursor row (line index)
    cursor_row: usize,
    /// Cursor column (char index within line)
    cursor_col: usize,
    /// Input history
    history: Vec<String>,
    /// Current history index (None = not browsing history)
    history_index: Option<usize>,
    /// Saved input when browsing history
    saved_input: Option<String>,
}

impl Default for InputEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl InputEditor {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            history: Vec::new(),
            history_index: None,
            saved_input: None,
        }
    }

    /// Handle a key event. Returns InputAction.
    pub fn handle_key(&mut self, key: KeyEvent) -> InputAction {
        match (key.modifiers, key.code) {
            // Emacs keybinds
            (m, KeyCode::Char('a')) if m.contains(KeyModifiers::CONTROL) => {
                self.cursor_col = 0;
                InputAction::Continue
            }
            (m, KeyCode::Char('e')) if m.contains(KeyModifiers::CONTROL) => {
                self.cursor_col = self.current_line().chars().count();
                InputAction::Continue
            }
            (m, KeyCode::Char('k')) if m.contains(KeyModifiers::CONTROL) => {
                self.kill_to_end_of_line();
                InputAction::Continue
            }
            (m, KeyCode::Char('u')) if m.contains(KeyModifiers::CONTROL) => {
                self.kill_to_start_of_line();
                InputAction::Continue
            }
            (m, KeyCode::Char('w')) if m.contains(KeyModifiers::CONTROL) => {
                self.delete_word_backward();
                InputAction::Continue
            }

            // Shift+Enter => insert newline
            (m, KeyCode::Enter) if m.contains(KeyModifiers::SHIFT) => {
                self.insert_newline();
                InputAction::Continue
            }

            // Enter => submit
            (_, KeyCode::Enter) => {
                let text = self.text();
                if text.trim().is_empty() {
                    return InputAction::Noop;
                }
                InputAction::Submit(text)
            }

            // Word jump with Ctrl+Left/Right
            (m, KeyCode::Left) if m.contains(KeyModifiers::CONTROL) => {
                self.word_left();
                InputAction::Continue
            }
            (m, KeyCode::Right) if m.contains(KeyModifiers::CONTROL) => {
                self.word_right();
                InputAction::Continue
            }

            // Arrow keys
            (_, KeyCode::Left) => {
                self.move_left();
                InputAction::Continue
            }
            (_, KeyCode::Right) => {
                self.move_right();
                InputAction::Continue
            }
            (_, KeyCode::Up) => {
                self.move_up_or_history();
                InputAction::Continue
            }
            (_, KeyCode::Down) => {
                self.move_down_or_history();
                InputAction::Continue
            }

            // Home/End
            (_, KeyCode::Home) => {
                self.cursor_col = 0;
                InputAction::Continue
            }
            (_, KeyCode::End) => {
                self.cursor_col = self.current_line().chars().count();
                InputAction::Continue
            }

            // Backspace
            (_, KeyCode::Backspace) => {
                self.backspace();
                InputAction::Continue
            }

            // Delete
            (_, KeyCode::Delete) => {
                self.delete_char();
                InputAction::Continue
            }

            // Regular character input
            (m, KeyCode::Char(c)) if !m.contains(KeyModifiers::CONTROL) => {
                self.insert_char(c);
                InputAction::Continue
            }

            _ => InputAction::Noop,
        }
    }

    /// Get the current text as a single string
    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    /// Clear the input and optionally save to history
    pub fn clear(&mut self, save_to_history: bool) {
        if save_to_history {
            let text = self.text();
            if !text.trim().is_empty() {
                self.history.push(text);
            }
        }
        self.lines = vec![String::new()];
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.history_index = None;
        self.saved_input = None;
    }

    /// Check if input is empty
    pub fn is_empty(&self) -> bool {
        self.lines.len() == 1 && self.lines[0].is_empty()
    }

    /// Get cursor position for rendering (x, y relative to inner area)
    pub fn cursor_position(&self, _area_width: u16) -> (u16, u16) {
        let x = self.cursor_col as u16;
        let y = self.cursor_row as u16;
        (x, y)
    }

    /// Render the input editor into the given area
    pub fn render(&self, f: &mut Frame, area: Rect, focused: bool) {
        let border_color = if focused {
            theme::SECONDARY
        } else {
            theme::BORDER
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(" Input ")
            .title_style(Style::default().fg(theme::SECONDARY));

        if self.is_empty() {
            let placeholder =
                Paragraph::new("Type a message... (Enter to send, Shift+Enter for newline)")
                    .style(Style::default().fg(theme::DIM))
                    .block(block);
            f.render_widget(placeholder, area);
        } else {
            let text_content: Vec<ratatui::text::Line> = self
                .lines
                .iter()
                .map(|line| ratatui::text::Line::from(line.as_str()))
                .collect();

            let paragraph = Paragraph::new(text_content)
                .style(Style::default().fg(Color::White))
                .block(block);
            f.render_widget(paragraph, area);
        }

        // Show cursor if focused (account for border: +1 on x and y)
        if focused {
            let (cx, cy) = self.cursor_position(area.width.saturating_sub(2));
            let cursor_x = area.x + 1 + cx;
            let cursor_y = area.y + 1 + cy;
            f.set_cursor_position((cursor_x, cursor_y));
        }
    }

    // -- Private helpers --

    fn current_line(&self) -> &str {
        &self.lines[self.cursor_row]
    }

    fn current_line_char_count(&self) -> usize {
        self.lines[self.cursor_row].chars().count()
    }

    /// Convert char index to byte index for the current line
    fn char_to_byte(&self, char_idx: usize) -> usize {
        self.lines[self.cursor_row]
            .char_indices()
            .nth(char_idx)
            .map(|(i, _)| i)
            .unwrap_or(self.lines[self.cursor_row].len())
    }

    fn insert_char(&mut self, c: char) {
        let byte_idx = self.char_to_byte(self.cursor_col);
        self.lines[self.cursor_row].insert(byte_idx, c);
        self.cursor_col += 1;
    }

    fn insert_newline(&mut self) {
        let byte_idx = self.char_to_byte(self.cursor_col);
        let rest = self.lines[self.cursor_row][byte_idx..].to_string();
        self.lines[self.cursor_row].truncate(byte_idx);
        self.cursor_row += 1;
        self.lines.insert(self.cursor_row, rest);
        self.cursor_col = 0;
    }

    fn backspace(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
            let byte_idx = self.char_to_byte(self.cursor_col);
            // Find the byte length of the character at this position
            let ch = self.lines[self.cursor_row][byte_idx..]
                .chars()
                .next()
                .unwrap();
            self.lines[self.cursor_row].replace_range(byte_idx..byte_idx + ch.len_utf8(), "");
        } else if self.cursor_row > 0 {
            // Merge with previous line
            let current = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.current_line_char_count();
            self.lines[self.cursor_row].push_str(&current);
        }
    }

    fn delete_char(&mut self) {
        let char_count = self.current_line_char_count();
        if self.cursor_col < char_count {
            let byte_idx = self.char_to_byte(self.cursor_col);
            let ch = self.lines[self.cursor_row][byte_idx..]
                .chars()
                .next()
                .unwrap();
            self.lines[self.cursor_row].replace_range(byte_idx..byte_idx + ch.len_utf8(), "");
        } else if self.cursor_row + 1 < self.lines.len() {
            // Merge next line into current
            let next = self.lines.remove(self.cursor_row + 1);
            self.lines[self.cursor_row].push_str(&next);
        }
    }

    fn move_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.current_line_char_count();
        }
    }

    fn move_right(&mut self) {
        let char_count = self.current_line_char_count();
        if self.cursor_col < char_count {
            self.cursor_col += 1;
        } else if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.cursor_col = 0;
        }
    }

    fn move_up_or_history(&mut self) {
        if self.cursor_row > 0 {
            // Move up a line
            self.cursor_row -= 1;
            let char_count = self.current_line_char_count();
            if self.cursor_col > char_count {
                self.cursor_col = char_count;
            }
        } else {
            // On first line: browse history
            self.history_previous();
        }
    }

    fn move_down_or_history(&mut self) {
        if self.cursor_row + 1 < self.lines.len() {
            // Move down a line
            self.cursor_row += 1;
            let char_count = self.current_line_char_count();
            if self.cursor_col > char_count {
                self.cursor_col = char_count;
            }
        } else {
            // On last line: browse history forward
            self.history_next();
        }
    }

    fn word_left(&mut self) {
        if self.cursor_col == 0 {
            // Move to end of previous line
            if self.cursor_row > 0 {
                self.cursor_row -= 1;
                self.cursor_col = self.current_line_char_count();
            }
            return;
        }

        let line = self.current_line().to_string();
        let chars: Vec<char> = line.chars().collect();
        let mut pos = self.cursor_col;

        // Skip whitespace backward
        while pos > 0 && chars[pos - 1].is_whitespace() {
            pos -= 1;
        }
        // Skip word characters backward
        while pos > 0 && !chars[pos - 1].is_whitespace() {
            pos -= 1;
        }

        self.cursor_col = pos;
    }

    fn word_right(&mut self) {
        let char_count = self.current_line_char_count();
        if self.cursor_col >= char_count {
            // Move to start of next line
            if self.cursor_row + 1 < self.lines.len() {
                self.cursor_row += 1;
                self.cursor_col = 0;
            }
            return;
        }

        let line = self.current_line().to_string();
        let chars: Vec<char> = line.chars().collect();
        let mut pos = self.cursor_col;

        // Skip word characters forward
        while pos < chars.len() && !chars[pos].is_whitespace() {
            pos += 1;
        }
        // Skip whitespace forward
        while pos < chars.len() && chars[pos].is_whitespace() {
            pos += 1;
        }

        self.cursor_col = pos;
    }

    fn kill_to_end_of_line(&mut self) {
        let byte_idx = self.char_to_byte(self.cursor_col);
        self.lines[self.cursor_row].truncate(byte_idx);
    }

    fn kill_to_start_of_line(&mut self) {
        let byte_idx = self.char_to_byte(self.cursor_col);
        self.lines[self.cursor_row] = self.lines[self.cursor_row][byte_idx..].to_string();
        self.cursor_col = 0;
    }

    fn delete_word_backward(&mut self) {
        if self.cursor_col == 0 {
            return;
        }

        let line = self.current_line().to_string();
        let chars: Vec<char> = line.chars().collect();
        let mut pos = self.cursor_col;

        // Skip whitespace backward
        while pos > 0 && chars[pos - 1].is_whitespace() {
            pos -= 1;
        }
        // Skip word characters backward
        while pos > 0 && !chars[pos - 1].is_whitespace() {
            pos -= 1;
        }

        let start_byte = line
            .char_indices()
            .nth(pos)
            .map(|(i, _)| i)
            .unwrap_or(line.len());
        let end_byte = self.char_to_byte(self.cursor_col);

        self.lines[self.cursor_row].replace_range(start_byte..end_byte, "");
        self.cursor_col = pos;
    }

    fn history_previous(&mut self) {
        if self.history.is_empty() {
            return;
        }

        match self.history_index {
            None => {
                // Save current input, load last history entry
                self.saved_input = Some(self.text());
                self.history_index = Some(self.history.len() - 1);
                self.load_history_entry(self.history.len() - 1);
            }
            Some(idx) if idx > 0 => {
                self.history_index = Some(idx - 1);
                self.load_history_entry(idx - 1);
            }
            _ => {}
        }
    }

    fn history_next(&mut self) {
        if let Some(idx) = self.history_index {
            if idx + 1 < self.history.len() {
                self.history_index = Some(idx + 1);
                self.load_history_entry(idx + 1);
            } else {
                // Restore saved input
                self.history_index = None;
                if let Some(saved) = self.saved_input.take() {
                    self.set_text(&saved);
                }
            }
        }
    }

    fn load_history_entry(&mut self, idx: usize) {
        let entry = self.history[idx].clone();
        self.set_text(&entry);
    }

    fn set_text(&mut self, text: &str) {
        self.lines = if text.is_empty() {
            vec![String::new()]
        } else {
            text.split('\n').map(|s| s.to_string()).collect()
        };
        self.cursor_row = self.lines.len() - 1;
        self.cursor_col = self.current_line_char_count();
    }
}
