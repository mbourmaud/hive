use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::process::Command;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use super::claude::ClaudeBackend;
use super::commands::CommandResult;
use super::input::InputState;
use super::layout::AppLayout;
use super::messages::Message;
use super::permissions::{ApprovalResponse, PermissionDialogState, ToolApprovalRequest};
use super::session_store::SessionStore;
use super::sessions::{SessionAction, SessionList};
use super::theme::Theme;
use super::ui;

/// Poll timeout for event handling in milliseconds
const POLL_TIMEOUT_MS: u64 = 100;

/// Default Claude model
const DEFAULT_MODEL: &str = "claude-sonnet-4-5-20250514";

/// Main TUI application state
pub struct App {
    /// Whether the application should exit
    pub should_quit: bool,
    /// Whether the sidebar is visible
    pub sidebar_visible: bool,
    /// Input state for chat widget
    pub input_state: InputState,
    /// Layout manager
    pub layout: AppLayout,
    /// Claude backend (spawned on first prompt)
    claude: Option<ClaudeBackend>,
    /// Receiver for messages from Claude backend
    claude_rx: Option<Receiver<Message>>,
    /// Chat message history
    pub messages: Vec<Message>,
    /// Current theme
    pub theme: Theme,
    /// Permission dialog state for tool approvals
    pub permission_state: PermissionDialogState,
    /// Session persistence store
    pub session_store: SessionStore,
    /// Session list overlay
    pub session_list: SessionList,
    /// Current active session ID
    pub current_session_id: Option<String>,
}

impl App {
    pub fn new() -> Self {
        let session_store = SessionStore::default();
        // Create initial session
        let current_session_id = session_store.create_session().ok().map(|s| s.id);
        Self {
            should_quit: false,
            sidebar_visible: true,
            input_state: InputState::new(),
            layout: AppLayout::new(),
            claude: None,
            claude_rx: None,
            messages: Vec::new(),
            theme: Theme::default(),
            permission_state: PermissionDialogState::new(),
            session_store,
            session_list: SessionList::new(),
            current_session_id,
        }
    }

    /// Handle keyboard events
    pub fn handle_event(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Key(key) => {
                // If permission dialog is active, intercept all key events
                if self.permission_state.is_active() {
                    if let KeyCode::Char(c) = key.code {
                        if let Some(response) = self.permission_state.handle_key(c) {
                            let tool_name = self
                                .permission_state
                                .request
                                .as_ref()
                                .map(|r| r.tool_name.clone())
                                .unwrap_or_default();
                            let msg = match response {
                                ApprovalResponse::Accept => {
                                    format!("Approved tool: {}", tool_name)
                                }
                                ApprovalResponse::Reject => {
                                    format!("Rejected tool: {}", tool_name)
                                }
                                ApprovalResponse::AlwaysAllow => {
                                    format!("Always allowed tool: {}", tool_name)
                                }
                            };
                            self.messages.push(Message::system(msg));
                            self.permission_state.clear();
                        }
                    }
                    return Ok(());
                }

                // If session list is visible, route keys to it
                if self.session_list.visible {
                    if let Some(action) = self.session_list.handle_key(key.code) {
                        self.handle_session_action(action);
                    }
                    return Ok(());
                }

                // Global shortcuts
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    match key.code {
                        KeyCode::Char('c') => {
                            self.should_quit = true;
                            return Ok(());
                        }
                        KeyCode::Char('b') => {
                            self.sidebar_visible = !self.sidebar_visible;
                            return Ok(());
                        }
                        KeyCode::Char('t') => {
                            self.theme = self.theme.toggle();
                            return Ok(());
                        }
                        KeyCode::Char('n') => {
                            self.create_new_session();
                            return Ok(());
                        }
                        KeyCode::Char('l') => {
                            self.session_list.toggle(&self.session_store);
                            return Ok(());
                        }
                        _ => {}
                    }
                }

                // Handle 'q' for quit
                if key.code == KeyCode::Char('q') && !key.modifiers.contains(KeyModifiers::CONTROL)
                {
                    self.should_quit = true;
                    return Ok(());
                }

                // Pass other events to input state
                self.input_state.handle_event(event)?;

                // Process any pending slash commands
                self.process_pending_commands();

                // Process any pending submitted messages (regular or bash)
                if let Some(text) = self.input_state.take_pending_message() {
                    let trimmed = text.trim().to_string();
                    if let Some(stripped) = trimmed.strip_prefix('!') {
                        let cmd = stripped.trim().to_string();
                        if !cmd.is_empty() {
                            self.execute_bash_command(&cmd);
                        }
                    } else {
                        self.handle_submit(trimmed)?;
                    }
                }
            }
            Event::Resize(_, _) => {
                // Terminal was resized, layout will be recalculated on next render
            }
            _ => {}
        }
        Ok(())
    }

    /// Get keybinding hints for the footer
    #[allow(dead_code)]
    pub fn get_keybindings(&self) -> Vec<(&'static str, &'static str)> {
        vec![
            ("q", "quit"),
            ("Ctrl+C", "quit"),
            (
                "Ctrl+B",
                if self.sidebar_visible {
                    "hide sidebar"
                } else {
                    "show sidebar"
                },
            ),
            ("Ctrl+T", "toggle theme"),
            ("Ctrl+Enter", "submit"),
            ("Ctrl+A/E", "start/end of line"),
            ("Ctrl+K/U", "delete to end/start"),
            ("Up/Down", "history"),
        ]
    }

    /// Handle a user prompt submission
    pub fn handle_submit(&mut self, prompt: String) -> Result<()> {
        // Add user message to history
        self.messages.push(Message::user(prompt.clone()));

        // Persist to session store
        if let Some(ref session_id) = self.current_session_id {
            let _ = self.session_store.add_message(session_id, "user", &prompt);
        }

        // Spawn Claude backend if not already running
        if self.claude.is_none() {
            let mut backend = ClaudeBackend::new();
            match backend.spawn(DEFAULT_MODEL) {
                Ok(rx) => {
                    self.claude = Some(backend);
                    self.claude_rx = Some(rx);
                }
                Err(e) => {
                    self.messages
                        .push(Message::error(format!("Failed to start Claude: {}", e)));
                    return Ok(());
                }
            }
        }

        // Send prompt to Claude
        if let Some(claude) = &self.claude {
            if let Err(e) = claude.send_input(prompt) {
                self.messages
                    .push(Message::error(format!("Failed to send prompt: {}", e)));
            }
        }

        Ok(())
    }

    /// Process any pending slash command from the input state
    pub fn process_pending_commands(&mut self) {
        if let Some(cmd) = self.input_state.take_pending_command() {
            match cmd.execute() {
                Ok(result) => self.handle_command_result(result),
                Err(e) => {
                    self.messages
                        .push(Message::error(format!("Command failed: {}", e)));
                }
            }
        }
    }

    /// Handle a command result by applying its effect
    fn handle_command_result(&mut self, result: CommandResult) {
        match result {
            CommandResult::NewSession => {
                self.create_new_session();
            }
            CommandResult::ResumeSession => {
                self.session_list.show(&self.session_store);
            }
            CommandResult::ShowHelp(text) => {
                self.messages.push(Message::assistant(text));
            }
            CommandResult::ClearChat => {
                self.messages.clear();
                self.messages
                    .push(Message::system("Chat cleared".to_string()));
            }
        }
    }

    /// Create a new session: save current, kill claude, clear messages.
    fn create_new_session(&mut self) {
        // Save current session before switching
        self.save_current_session();

        // Kill current Claude process
        if let Some(ref mut claude) = self.claude {
            let _ = claude.kill();
        }
        self.claude = None;
        self.claude_rx = None;

        // Create new session
        match self.session_store.create_session() {
            Ok(session) => {
                self.current_session_id = Some(session.id);
                self.messages.clear();
                self.messages
                    .push(Message::system("New session started.".to_string()));
            }
            Err(e) => {
                self.messages
                    .push(Message::error(format!("Failed to create session: {}", e)));
            }
        }
    }

    /// Switch to an existing session by ID.
    fn switch_to_session(&mut self, session_id: &str) {
        // Don't switch to the same session
        if self.current_session_id.as_deref() == Some(session_id) {
            return;
        }

        // Save current session
        self.save_current_session();

        // Kill current Claude process
        if let Some(ref mut claude) = self.claude {
            let _ = claude.kill();
        }
        self.claude = None;
        self.claude_rx = None;

        // Load the selected session
        match self.session_store.get_session(session_id) {
            Ok(Some(session)) => {
                let title = session.title.clone();
                self.current_session_id = Some(session.id.clone());
                self.messages.clear();
                self.messages.push(Message::system(format!(
                    "Resumed session: {}. Use /resume to reconnect to Claude.",
                    title
                )));
                // Mark as active
                let _ = self.session_store.set_active(session_id);
            }
            Ok(None) => {
                self.messages
                    .push(Message::error("Session not found.".to_string()));
            }
            Err(e) => {
                self.messages
                    .push(Message::error(format!("Failed to load session: {}", e)));
            }
        }
    }

    /// Save current session metadata to the store.
    fn save_current_session(&self) {
        let Some(ref session_id) = self.current_session_id else {
            return;
        };
        // Update message count in the session
        for msg in &self.messages {
            if let Message::User { ref content, .. } = msg {
                let _ = self.session_store.add_message(session_id, "user", content);
            }
        }
    }

    /// Handle a session action from the session list overlay.
    fn handle_session_action(&mut self, action: SessionAction) {
        match action {
            SessionAction::NewSession => self.create_new_session(),
            SessionAction::Select(id) => self.switch_to_session(&id),
            SessionAction::Close => {}
        }
    }

    /// Execute a bash command and push the output as a system message
    fn execute_bash_command(&mut self, cmd: &str) {
        self.messages.push(Message::system(format!("$ {}", cmd)));

        match Command::new("sh").arg("-c").arg(cmd).output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                let mut result_lines: Vec<&str> = Vec::new();
                const MAX_LINES: usize = 50;

                if !stdout.is_empty() {
                    for line in stdout.lines().take(MAX_LINES) {
                        result_lines.push(line);
                    }
                }

                if !stderr.is_empty() {
                    for line in stderr
                        .lines()
                        .take(MAX_LINES.saturating_sub(result_lines.len()))
                    {
                        result_lines.push(line);
                    }
                }

                let total_lines = stdout.lines().count() + stderr.lines().count();
                let mut combined = result_lines.join("\n");

                if total_lines > MAX_LINES {
                    combined.push_str(&format!(
                        "\n... ({} lines truncated)",
                        total_lines - MAX_LINES
                    ));
                }

                if combined.is_empty() {
                    combined = "(no output)".to_string();
                }

                if output.status.success() {
                    self.messages.push(Message::system(combined));
                } else {
                    self.messages.push(Message::error(format!(
                        "Exit code {}: {}",
                        output.status.code().unwrap_or(-1),
                        combined
                    )));
                }
            }
            Err(e) => {
                self.messages
                    .push(Message::error(format!("Failed to execute command: {}", e)));
            }
        }
    }

    /// Poll for new messages from Claude
    pub fn poll_claude_messages(&mut self) {
        if let Some(rx) = &self.claude_rx {
            // Collect all pending messages
            while let Ok(message) = rx.try_recv() {
                // Check if this is a ToolUse message that needs approval
                if let Message::ToolUse {
                    ref tool_name,
                    ref args_summary,
                    ..
                } = message
                {
                    self.permission_state.set_request(ToolApprovalRequest {
                        id: uuid::Uuid::new_v4().to_string(),
                        tool_name: tool_name.clone(),
                        args: serde_json::from_str(args_summary)
                            .unwrap_or(serde_json::Value::String(args_summary.clone())),
                        file_diff: None,
                    });
                }
                self.messages.push(message);
            }
        }

        if let Some(claude) = &mut self.claude {
            // Check if process is still running
            if !claude.is_running() {
                self.messages
                    .push(Message::error("Claude process terminated".to_string()));
                self.claude = None;
                self.claude_rx = None;
            }
        }
    }
}

/// Main entry point for the TUI application
pub fn run_tui() -> Result<()> {
    // Install panic hook to restore terminal on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new();

    // Main event loop
    let result = run_event_loop(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

/// Run the main event loop
fn run_event_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    loop {
        // Poll for Claude messages
        app.poll_claude_messages();

        // Render UI
        terminal.draw(|f| ui::render(f, app))?;

        // Handle events with timeout
        if event::poll(Duration::from_millis(POLL_TIMEOUT_MS))? {
            let event = event::read()?;
            app.handle_event(event)?;
        }

        // Check if we should quit
        if app.should_quit {
            break;
        }
    }

    Ok(())
}
