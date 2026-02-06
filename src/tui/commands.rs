use anyhow::Result;

/// Represents a slash command that can be executed in the chat
#[derive(Debug, Clone, PartialEq)]
pub enum SlashCommand {
    /// Create a new chat session
    New,
    /// Resume a previous chat session
    Resume,
    /// Show help information
    Help,
    /// Clear the current chat history
    Clear,
}

impl SlashCommand {
    /// Parse a string into a slash command
    pub fn from_str(s: &str) -> Option<Self> {
        let s = s.trim_start_matches('/').to_lowercase();
        match s.as_str() {
            "new" => Some(SlashCommand::New),
            "resume" => Some(SlashCommand::Resume),
            "help" => Some(SlashCommand::Help),
            "clear" => Some(SlashCommand::Clear),
            _ => None,
        }
    }

    /// Get the command name (without the slash)
    pub fn name(&self) -> &'static str {
        match self {
            SlashCommand::New => "new",
            SlashCommand::Resume => "resume",
            SlashCommand::Help => "help",
            SlashCommand::Clear => "clear",
        }
    }

    /// Get a description of the command
    pub fn description(&self) -> &'static str {
        match self {
            SlashCommand::New => "Create a new chat session",
            SlashCommand::Resume => "Resume a previous chat session",
            SlashCommand::Help => "Show help information",
            SlashCommand::Clear => "Clear the current chat history",
        }
    }

    /// Get all available commands
    pub fn all() -> Vec<SlashCommand> {
        vec![
            SlashCommand::New,
            SlashCommand::Resume,
            SlashCommand::Help,
            SlashCommand::Clear,
        ]
    }

    /// Filter commands by prefix
    pub fn filter_by_prefix(prefix: &str) -> Vec<SlashCommand> {
        let prefix = prefix.trim_start_matches('/').to_lowercase();
        if prefix.is_empty() {
            return Self::all();
        }

        Self::all()
            .into_iter()
            .filter(|cmd| cmd.name().starts_with(&prefix))
            .collect()
    }

    /// Execute the command
    pub fn execute(&self) -> Result<CommandResult> {
        match self {
            SlashCommand::New => Ok(CommandResult::NewSession),
            SlashCommand::Resume => Ok(CommandResult::ResumeSession),
            SlashCommand::Help => Ok(CommandResult::ShowHelp(help_text())),
            SlashCommand::Clear => Ok(CommandResult::ClearChat),
        }
    }
}

/// Result of executing a slash command
#[derive(Debug, Clone)]
pub enum CommandResult {
    /// Create a new session
    NewSession,
    /// Resume a previous session
    ResumeSession,
    /// Show help text with content
    ShowHelp(String),
    /// Clear chat history
    ClearChat,
}

/// Generate the help text for the /help command
pub fn help_text() -> String {
    let text = "\
=== Hive Unified TUI - Help ===

Slash Commands:
  /new       Create a new chat session
  /resume    Resume a previous chat session
  /help      Show this help information
  /clear     Clear the current chat history

Special Prefixes:
  @          Trigger file picker (fuzzy search)
  !<cmd>     Execute a bash command

Keybindings:
  Ctrl+Enter   Submit message
  Ctrl+C       Quit
  Ctrl+B       Toggle sidebar
  Ctrl+T       Toggle theme (dark/light)
  Ctrl+A       Move cursor to start of line
  Ctrl+E       Move cursor to end of line
  Ctrl+K       Delete from cursor to end of line
  Ctrl+U       Delete from cursor to start of line
  Up/Down      Navigate input history
  Tab          Accept autocomplete suggestion
  Esc          Close popup (autocomplete/file picker)";
    text.to_string()
}

/// State for the command autocomplete popup
#[derive(Debug, Clone)]
pub struct CommandAutocomplete {
    /// The current filter prefix
    pub prefix: String,
    /// Filtered commands matching the prefix
    pub commands: Vec<SlashCommand>,
    /// Selected index in the commands list
    pub selected: usize,
    /// Whether the autocomplete is visible
    pub visible: bool,
}

impl CommandAutocomplete {
    pub fn new() -> Self {
        Self {
            prefix: String::new(),
            commands: Vec::new(),
            selected: 0,
            visible: false,
        }
    }

    /// Update the autocomplete with a new prefix
    pub fn update(&mut self, prefix: &str) {
        self.prefix = prefix.to_string();
        self.commands = SlashCommand::filter_by_prefix(prefix);
        self.selected = 0;
        self.visible = !self.commands.is_empty();
    }

    /// Select the next command
    pub fn select_next(&mut self) {
        if !self.commands.is_empty() {
            self.selected = (self.selected + 1) % self.commands.len();
        }
    }

    /// Select the previous command
    pub fn select_prev(&mut self) {
        if !self.commands.is_empty() {
            if self.selected == 0 {
                self.selected = self.commands.len() - 1;
            } else {
                self.selected -= 1;
            }
        }
    }

    /// Get the currently selected command
    pub fn get_selected(&self) -> Option<&SlashCommand> {
        self.commands.get(self.selected)
    }

    /// Accept the selected command and return it
    pub fn accept(&mut self) -> Option<SlashCommand> {
        let cmd = self.get_selected().cloned();
        self.hide();
        cmd
    }

    /// Hide the autocomplete
    pub fn hide(&mut self) {
        self.visible = false;
        self.prefix.clear();
        self.commands.clear();
        self.selected = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slash_command_from_str() {
        assert_eq!(SlashCommand::from_str("/new"), Some(SlashCommand::New));
        assert_eq!(SlashCommand::from_str("new"), Some(SlashCommand::New));
        assert_eq!(SlashCommand::from_str("/help"), Some(SlashCommand::Help));
        assert_eq!(SlashCommand::from_str("/invalid"), None);
    }

    #[test]
    fn test_filter_by_prefix() {
        let commands = SlashCommand::filter_by_prefix("/n");
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0], SlashCommand::New);

        let commands = SlashCommand::filter_by_prefix("/");
        assert_eq!(commands.len(), 4);
    }

    #[test]
    fn test_autocomplete() {
        let mut ac = CommandAutocomplete::new();
        ac.update("/n");
        assert!(ac.visible);
        assert_eq!(ac.commands.len(), 1);
        assert_eq!(ac.commands[0], SlashCommand::New);
    }
}
