#[derive(Debug, Clone)]
pub enum SlashCommand {
    /// /new - Start a new session
    New,
    /// /sessions - List and switch sessions
    Sessions,
    /// /model <name> - Switch model
    Model(Option<String>),
    /// /plan <prompt> - Create a Hive plan (stub)
    Plan(String),
    /// /start <name> - Launch a drone (stub)
    Start(String),
    /// /monitor - Open full monitor view
    Monitor,
    /// /status - Show all drones
    Status,
    /// /stop <name> - Stop a drone
    Stop(String),
    /// /help - Show commands and keybinds
    Help,
    /// /compact - Compact session context (stub)
    Compact,
    /// /share - Export session (stub)
    Share,
    /// /clear - Clear current messages
    Clear,
}

/// Parse a slash command from input text.
/// Returns None if the text doesn't start with /
pub fn parse_command(input: &str) -> Option<SlashCommand> {
    let input = input.trim();
    if !input.starts_with('/') {
        return None;
    }

    let parts: Vec<&str> = input.splitn(2, ' ').collect();
    let cmd = parts[0].to_lowercase();
    let args = parts
        .get(1)
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    match cmd.as_str() {
        "/new" => Some(SlashCommand::New),
        "/sessions" => Some(SlashCommand::Sessions),
        "/model" => Some(SlashCommand::Model(if args.is_empty() {
            None
        } else {
            Some(args)
        })),
        "/plan" => {
            if args.is_empty() {
                None
            } else {
                Some(SlashCommand::Plan(args))
            }
        }
        "/start" => {
            if args.is_empty() {
                None
            } else {
                Some(SlashCommand::Start(args))
            }
        }
        "/monitor" => Some(SlashCommand::Monitor),
        "/status" => Some(SlashCommand::Status),
        "/stop" => {
            if args.is_empty() {
                None
            } else {
                Some(SlashCommand::Stop(args))
            }
        }
        "/help" => Some(SlashCommand::Help),
        "/compact" => Some(SlashCommand::Compact),
        "/share" => Some(SlashCommand::Share),
        "/clear" => Some(SlashCommand::Clear),
        _ => None,
    }
}

/// Get all available commands for autocomplete/help
pub fn all_commands() -> Vec<(&'static str, &'static str)> {
    vec![
        ("/new", "Start a new session"),
        ("/sessions", "List and switch sessions"),
        ("/model", "Switch model (sonnet/opus/haiku)"),
        ("/plan <prompt>", "Create a Hive plan"),
        ("/start <name>", "Launch a drone"),
        ("/monitor", "Open full monitor view"),
        ("/status", "Show all drones"),
        ("/stop <name>", "Stop a drone"),
        ("/help", "Show commands and keybinds"),
        ("/clear", "Clear current messages"),
        ("/compact", "Compact session context"),
        ("/share", "Export session"),
    ]
}

/// Generate help text to show inline
pub fn help_text() -> String {
    let mut help = String::new();
    help.push_str("## Commands\n\n");
    for (cmd, desc) in all_commands() {
        help.push_str(&format!("  {:<20} {}\n", cmd, desc));
    }
    help.push_str("\n## Keybinds\n\n");
    help.push_str("  Ctrl+X             Leader key prefix\n");
    help.push_str("  <leader>n          New session\n");
    help.push_str("  <leader>l          Session list\n");
    help.push_str("  <leader>b          Toggle sidebar\n");
    help.push_str("  <leader>m          Model picker\n");
    help.push_str("  <leader>d          Drone list\n");
    help.push_str("  Ctrl+C             Interrupt / Quit\n");
    help.push_str("  Escape             Close / Quit\n");
    help.push_str("  Enter              Send message\n");
    help.push_str("  Shift+Enter        New line\n");
    help.push_str("  PageUp/Down        Scroll messages\n");
    help.push_str("  Ctrl+U/D           Half-page scroll\n");
    help
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_new() {
        assert!(matches!(parse_command("/new"), Some(SlashCommand::New)));
    }

    #[test]
    fn test_parse_model_with_arg() {
        match parse_command("/model sonnet") {
            Some(SlashCommand::Model(Some(m))) => assert_eq!(m, "sonnet"),
            _ => panic!("Expected Model with arg"),
        }
    }

    #[test]
    fn test_parse_model_no_arg() {
        assert!(matches!(
            parse_command("/model"),
            Some(SlashCommand::Model(None))
        ));
    }

    #[test]
    fn test_parse_help() {
        assert!(matches!(parse_command("/help"), Some(SlashCommand::Help)));
    }

    #[test]
    fn test_parse_unknown() {
        assert!(parse_command("/unknown_cmd").is_none());
    }

    #[test]
    fn test_parse_not_command() {
        assert!(parse_command("hello world").is_none());
    }

    #[test]
    fn test_parse_stop_requires_arg() {
        assert!(parse_command("/stop").is_none());
    }

    #[test]
    fn test_parse_stop_with_arg() {
        match parse_command("/stop my-drone") {
            Some(SlashCommand::Stop(name)) => assert_eq!(name, "my-drone"),
            _ => panic!("Expected Stop"),
        }
    }
}
