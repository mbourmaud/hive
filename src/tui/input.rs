use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyModifiers};
use std::env;
use tui_textarea::TextArea;

use super::commands::{CommandAutocomplete, SlashCommand};
use super::file_picker::FilePicker;

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
    /// Command autocomplete state
    pub command_autocomplete: CommandAutocomplete,
    /// File picker state
    pub file_picker: FilePicker,
    /// Pending command result to be processed
    pub pending_command: Option<SlashCommand>,
}

impl InputState {
    pub fn new() -> Self {
        let mut textarea = TextArea::default();
        textarea.set_placeholder_text("Type your message here... (Ctrl+Enter to submit, @ for file, / for commands, ! for bash)");

        Self {
            textarea,
            history: Vec::new(),
            history_index: None,
            temp_input: String::new(),
            command_autocomplete: CommandAutocomplete::new(),
            file_picker: FilePicker::new(),
            pending_command: None,
        }
    }

    /// Handle keyboard events for the input area
    pub fn handle_event(&mut self, event: Event) -> Result<()> {
        if let Event::Key(key) = event {
            // Handle file picker navigation if active
            if self.file_picker.visible {
                return self.handle_file_picker_event(key);
            }

            // Handle command autocomplete if active
            if self.command_autocomplete.visible {
                return self.handle_command_autocomplete_event(key);
            }

            // Handle Ctrl+Enter for submit
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Enter {
                self.submit()?;
                return Ok(());
            }

            // Handle Tab for autocomplete acceptance
            if key.code == KeyCode::Tab && key.modifiers.is_empty() {
                let text = self.get_text();
                if text.starts_with('/') {
                    self.trigger_command_autocomplete();
                    return Ok(());
                } else if text.contains('@') {
                    self.trigger_file_picker()?;
                    return Ok(());
                }
            }

            // Handle history navigation with Up/Down (only when no autocomplete is active)
            if key.modifiers.is_empty() {
                match key.code {
                    KeyCode::Up => {
                        self.navigate_history_up();
                        return Ok(());
                    }
                    KeyCode::Down => {
                        self.navigate_history_down();
                        return Ok(());
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
                        return Ok(());
                    }
                    KeyCode::Char('e') => {
                        // Move to end of line
                        self.textarea.move_cursor(tui_textarea::CursorMove::End);
                        return Ok(());
                    }
                    KeyCode::Char('k') => {
                        // Delete from cursor to end of line
                        self.textarea.delete_line_by_end();
                        return Ok(());
                    }
                    KeyCode::Char('u') => {
                        // Delete from cursor to start of line
                        self.textarea.delete_line_by_head();
                        return Ok(());
                    }
                    _ => {}
                }
            }

            // Pass event to textarea for normal editing
            self.textarea.input(event);

            // Check if we need to trigger autocomplete
            let text = self.get_text();
            if text.starts_with('/') {
                self.trigger_command_autocomplete();
            }

            // Reset history navigation if user types something
            if !matches!(key.code, KeyCode::Up | KeyCode::Down) {
                self.history_index = None;
            }
        }

        Ok(())
    }

    /// Submit the current input
    fn submit(&mut self) -> Result<()> {
        let text = self.textarea.lines().join("\n");

        if !text.trim().is_empty() {
            // Check if it's a slash command
            if text.trim().starts_with('/') {
                if let Some(cmd) = SlashCommand::from_str(text.trim()) {
                    self.pending_command = Some(cmd);
                }
            }

            // Add to history (unless it's a slash command)
            if !text.trim().starts_with('/') {
                self.history.push(text.clone());
            }

            // TODO: Send message to chat handler or execute command/bash
            // For now, just clear the input
            self.textarea = TextArea::default();
            self.textarea.set_placeholder_text("Type your message here... (Ctrl+Enter to submit, @ for file, / for commands, ! for bash)");

            // Reset history navigation
            self.history_index = None;
            self.temp_input.clear();
        }

        Ok(())
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

    /// Trigger command autocomplete
    fn trigger_command_autocomplete(&mut self) {
        let text = self.get_text();
        if text.starts_with('/') {
            self.command_autocomplete.update(&text);
        }
    }

    /// Trigger file picker
    fn trigger_file_picker(&mut self) -> Result<()> {
        let current_dir = env::current_dir()?;
        self.file_picker.show(&current_dir)?;
        Ok(())
    }

    /// Handle keyboard events when file picker is active
    fn handle_file_picker_event(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.file_picker.hide();
            }
            KeyCode::Enter => {
                if let Some(path) = self.file_picker.accept() {
                    // Insert the file path at cursor position
                    let path_str = path.to_string_lossy().to_string();
                    self.insert_at_cursor(&path_str);
                }
            }
            KeyCode::Up => {
                self.file_picker.select_prev();
            }
            KeyCode::Down => {
                self.file_picker.select_next();
            }
            KeyCode::Char(c) => {
                // Update filter as user types
                let mut query = self.file_picker.query.clone();
                query.push(c);
                self.file_picker.update_filter(&query);
            }
            KeyCode::Backspace => {
                // Remove last character from filter
                let mut query = self.file_picker.query.clone();
                query.pop();
                self.file_picker.update_filter(&query);
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle keyboard events when command autocomplete is active
    fn handle_command_autocomplete_event(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.command_autocomplete.hide();
            }
            KeyCode::Enter | KeyCode::Tab => {
                if let Some(cmd) = self.command_autocomplete.accept() {
                    // Replace the current input with the command
                    let cmd_text = format!("/{}", cmd.name());
                    self.set_textarea_content(&cmd_text);
                    // Store the command for execution
                    self.pending_command = Some(cmd);
                }
            }
            KeyCode::Up => {
                self.command_autocomplete.select_prev();
            }
            KeyCode::Down => {
                self.command_autocomplete.select_next();
            }
            _ => {
                // Allow typing to continue filtering
                self.command_autocomplete.hide();
                self.textarea.input(Event::Key(key));
                let text = self.get_text();
                if text.starts_with('/') {
                    self.trigger_command_autocomplete();
                }
            }
        }
        Ok(())
    }

    /// Insert text at the current cursor position
    fn insert_at_cursor(&mut self, text: &str) {
        // Get current content
        let current = self.get_text();

        // Find @ symbol and replace it with the file path
        let new_content = if let Some(pos) = current.rfind('@') {
            format!("{}{}", &current[..pos], text)
        } else {
            format!("{}{}", current, text)
        };

        self.set_textarea_content(&new_content);
    }

    /// Check if input is a bash command (starts with !)
    #[allow(dead_code)]
    pub fn is_bash_command(&self) -> bool {
        let text = self.get_text();
        text.trim().starts_with('!')
    }

    /// Get the bash command (without the ! prefix)
    #[allow(dead_code)]
    pub fn get_bash_command(&self) -> Option<String> {
        let text = self.get_text();
        let trimmed = text.trim();
        trimmed.strip_prefix('!').map(|s| s.trim().to_string())
    }

    /// Execute and consume the pending command
    #[allow(dead_code)]
    pub fn take_pending_command(&mut self) -> Option<SlashCommand> {
        self.pending_command.take()
    }
}
