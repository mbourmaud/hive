use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
    Frame,
};

#[derive(Debug, Clone)]
pub struct SlashCommand {
    pub name: &'static str,
    pub description: &'static str,
}

const SLASH_COMMANDS: &[SlashCommand] = &[
    SlashCommand {
        name: "/new",
        description: "Create a new session",
    },
    SlashCommand {
        name: "/resume",
        description: "Resume a previous session",
    },
    SlashCommand {
        name: "/clear",
        description: "Clear chat history",
    },
    SlashCommand {
        name: "/help",
        description: "Show available commands",
    },
    SlashCommand {
        name: "/model",
        description: "Switch AI model",
    },
    SlashCommand {
        name: "/sessions",
        description: "List all sessions",
    },
];

/// Result of processing a slash command
#[derive(Debug)]
pub enum CommandResult {
    NewSession,
    ShowSessions,
    ClearChat,
    ShowHelp(Vec<String>),
    Unknown(String),
}

pub fn execute_command(input: &str) -> CommandResult {
    let cmd = input.trim();
    match cmd {
        "/new" => CommandResult::NewSession,
        "/resume" | "/sessions" => CommandResult::ShowSessions,
        "/clear" => CommandResult::ClearChat,
        "/help" => {
            let help_lines: Vec<String> = SLASH_COMMANDS
                .iter()
                .map(|c| format!("  {}  - {}", c.name, c.description))
                .collect();
            CommandResult::ShowHelp(help_lines)
        }
        _ => CommandResult::Unknown(cmd.to_string()),
    }
}

pub struct CommandPalette {
    pub visible: bool,
    pub filter: String,
    pub selected: usize,
    pub list_state: ListState,
    filtered_commands: Vec<&'static SlashCommand>,
}

impl CommandPalette {
    pub fn new() -> Self {
        Self {
            visible: false,
            filter: String::new(),
            selected: 0,
            list_state: ListState::default(),
            filtered_commands: Vec::new(),
        }
    }

    pub fn show(&mut self, initial_filter: &str) {
        self.visible = true;
        self.filter = initial_filter.to_string();
        self.selected = 0;
        self.update_filter();
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.filter.clear();
        self.filtered_commands.clear();
    }

    pub fn update_filter(&mut self) {
        let filter_lower = self.filter.to_lowercase();
        self.filtered_commands = SLASH_COMMANDS
            .iter()
            .filter(|c| {
                c.name.contains(&filter_lower) || c.description.to_lowercase().contains(&filter_lower)
            })
            .collect();
        if self.selected >= self.filtered_commands.len() {
            self.selected = self.filtered_commands.len().saturating_sub(1);
        }
        self.list_state.select(Some(self.selected));
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.list_state.select(Some(self.selected));
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.filtered_commands.len() {
            self.selected += 1;
            self.list_state.select(Some(self.selected));
        }
    }

    pub fn confirm(&self) -> Option<&'static str> {
        self.filtered_commands.get(self.selected).map(|c| c.name)
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        if !self.visible || self.filtered_commands.is_empty() {
            return;
        }

        let height = (self.filtered_commands.len() as u16 + 2).min(10);
        let width = 40u16.min(area.width);

        // Position popup above the input area (bottom-left)
        let popup = Rect {
            x: area.x + 1,
            y: area.y.saturating_sub(height + 1),
            width,
            height,
        };

        f.render_widget(Clear, popup);

        let items: Vec<ListItem> = self
            .filtered_commands
            .iter()
            .map(|c| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{:<12}", c.name),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(c.description, Style::default().fg(Color::DarkGray)),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(" Commands "),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );

        f.render_stateful_widget(list, popup, &mut self.list_state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_new() {
        assert!(matches!(execute_command("/new"), CommandResult::NewSession));
    }

    #[test]
    fn test_execute_clear() {
        assert!(matches!(execute_command("/clear"), CommandResult::ClearChat));
    }

    #[test]
    fn test_execute_help() {
        if let CommandResult::ShowHelp(lines) = execute_command("/help") {
            assert!(!lines.is_empty());
        } else {
            panic!("Expected ShowHelp");
        }
    }

    #[test]
    fn test_execute_unknown() {
        assert!(matches!(
            execute_command("/foobar"),
            CommandResult::Unknown(_)
        ));
    }

    #[test]
    fn test_command_palette_filter() {
        let mut palette = CommandPalette::new();
        palette.show("/n");
        assert!(!palette.filtered_commands.is_empty());
        // Should include /new
        assert!(palette
            .filtered_commands
            .iter()
            .any(|c| c.name == "/new"));
    }

    #[test]
    fn test_command_palette_navigation() {
        let mut palette = CommandPalette::new();
        palette.show("/");
        assert_eq!(palette.selected, 0);
        palette.move_down();
        assert_eq!(palette.selected, 1);
        palette.move_up();
        assert_eq!(palette.selected, 0);
    }
}
