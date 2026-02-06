use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::collections::HashMap;
use std::io;
use std::process::Command;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use crate::commands::common::list_drones;
use crate::types::{DroneStatus, Prd};

use super::claude::ClaudeBackend;
use super::commands::CommandResult;
use super::drone_actions::{self, LogViewerState, PrdSelectionState};
use super::input::InputState;
use super::layout::AppLayout;
use super::messages::Message;
use super::monitor;
use super::permissions::{ApprovalResponse, PermissionDialogState, ToolApprovalRequest};
use super::session_store::SessionStore;
use super::sessions::{SessionAction, SessionList};
use super::sidebar::SidebarState;
use super::theme::Theme;
use super::ui;

/// Poll timeout for event handling in milliseconds
const POLL_TIMEOUT_MS: u64 = 100;

/// Default Claude model
const DEFAULT_MODEL: &str = "claude-sonnet-4-5-20250514";

/// Duration before status messages auto-clear (in seconds)
const STATUS_MESSAGE_DURATION_SECS: u64 = 3;

/// Which pane is currently focused
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    Chat,
    Sidebar,
}

/// Main TUI application state
pub struct App {
    /// Whether the application should exit
    pub should_quit: bool,
    /// Whether the sidebar is visible
    pub sidebar_visible: bool,
    /// Currently focused pane
    pub focused_pane: FocusedPane,
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
    /// Sidebar state for drone navigation
    pub sidebar_state: SidebarState,
    /// Cached drone data
    pub drones: Vec<(String, DroneStatus)>,
    /// Cached PRD data
    pub prd_cache: HashMap<String, Prd>,
    /// Display order indices for sidebar rendering
    pub display_order: Vec<usize>,
    /// Count of active drones in display_order
    pub active_count: usize,
    /// Transient status message displayed in footer (message, timestamp)
    pub status_message: Option<(String, Instant)>,
    /// Log viewer state
    pub log_viewer: LogViewerState,
    /// PRD selection dialog state
    pub prd_selection: PrdSelectionState,
}

impl App {
    pub fn new() -> Self {
        let session_store = SessionStore::default();
        let current_session_id = session_store.create_session().ok().map(|s| s.id);

        // Load initial drone data
        let drones = list_drones().unwrap_or_default();
        let prd_cache = monitor::load_prd_cache(&drones);
        let (display_order, active_count) = monitor::build_display_order(&drones, &prd_cache);
        let expanded = monitor::initial_expanded_drones(&drones);

        let mut sidebar_state = SidebarState::new();
        sidebar_state.expanded_drones = expanded;

        Self {
            should_quit: false,
            sidebar_visible: true,
            focused_pane: FocusedPane::Chat,
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
            sidebar_state,
            drones,
            prd_cache,
            display_order,
            active_count,
            status_message: None,
            log_viewer: LogViewerState::new(),
            prd_selection: PrdSelectionState::new(),
        }
    }

    /// Set a transient status message that auto-clears.
    pub fn set_status_message(&mut self, msg: String) {
        self.status_message = Some((msg, Instant::now()));
    }

    /// Clear expired status messages.
    pub fn clear_expired_status(&mut self) {
        if let Some((_, ref created_at)) = self.status_message {
            if created_at.elapsed() > Duration::from_secs(STATUS_MESSAGE_DURATION_SECS) {
                self.status_message = None;
            }
        }
    }

    /// Refresh drone data from disk.
    pub fn refresh_drones(&mut self) {
        self.drones = list_drones().unwrap_or_default();
        self.prd_cache = monitor::load_prd_cache(&self.drones);
        let (display_order, active_count) =
            monitor::build_display_order(&self.drones, &self.prd_cache);
        self.display_order = display_order;
        self.active_count = active_count;

        // Clamp sidebar selection
        if !self.display_order.is_empty()
            && self.sidebar_state.selected_index >= self.display_order.len()
        {
            self.sidebar_state.selected_index = self.display_order.len() - 1;
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

                // If log viewer is open, handle its keys
                if self.log_viewer.visible {
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => self.log_viewer.close(),
                        KeyCode::Up | KeyCode::Char('k') => self.log_viewer.scroll_up(1),
                        KeyCode::Down | KeyCode::Char('j') => self.log_viewer.scroll_down(1),
                        KeyCode::Char('r') => {
                            let _ = self.log_viewer.reload();
                        }
                        _ => {}
                    }
                    return Ok(());
                }

                // If PRD selection is open, handle its keys
                if self.prd_selection.visible {
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => self.prd_selection.close(),
                        KeyCode::Up | KeyCode::Char('k') => self.prd_selection.select_prev(),
                        KeyCode::Down | KeyCode::Char('j') => self.prd_selection.select_next(),
                        KeyCode::Enter => {
                            if let Some(prd) = self.prd_selection.selected_prd() {
                                let msg = format!(
                                    "Selected PRD: {} (launch not yet wired)",
                                    prd.filename
                                );
                                self.set_status_message(msg);
                            }
                            self.prd_selection.close();
                        }
                        _ => {}
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
                            if !self.sidebar_visible {
                                self.focused_pane = FocusedPane::Chat;
                            }
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

                // Tab switches panes when sidebar is visible
                if key.code == KeyCode::Tab
                    && key.modifiers == KeyModifiers::NONE
                    && self.sidebar_visible
                {
                    self.focused_pane = match self.focused_pane {
                        FocusedPane::Chat => FocusedPane::Sidebar,
                        FocusedPane::Sidebar => FocusedPane::Chat,
                    };
                    return Ok(());
                }

                // Sidebar-focused key handling
                if self.focused_pane == FocusedPane::Sidebar {
                    match key.code {
                        KeyCode::Char('q') if key.modifiers == KeyModifiers::NONE => {
                            self.should_quit = true;
                        }
                        KeyCode::Char('j') | KeyCode::Down
                            if key.modifiers == KeyModifiers::NONE =>
                        {
                            super::sidebar::handle_navigation(
                                &mut self.sidebar_state,
                                'j',
                                &self.drones,
                                &self.prd_cache,
                                &self.display_order,
                            );
                        }
                        KeyCode::Char('k') | KeyCode::Up if key.modifiers == KeyModifiers::NONE => {
                            super::sidebar::handle_navigation(
                                &mut self.sidebar_state,
                                'k',
                                &self.drones,
                                &self.prd_cache,
                                &self.display_order,
                            );
                        }
                        KeyCode::Enter | KeyCode::Char(' ')
                            if key.modifiers == KeyModifiers::NONE =>
                        {
                            super::sidebar::toggle_expansion(
                                &mut self.sidebar_state,
                                &self.drones,
                                &self.display_order,
                                &self.prd_cache,
                            );
                        }
                        // Drone action: stop
                        KeyCode::Char('x') if key.modifiers == KeyModifiers::NONE => {
                            if let Some(name) = self
                                .sidebar_state
                                .selected_drone_name(&self.drones, &self.display_order)
                            {
                                let result = drone_actions::execute_stop(&name);
                                self.set_status_message(result.message);
                            }
                        }
                        // Drone action: clean
                        KeyCode::Char('c') if key.modifiers == KeyModifiers::NONE => {
                            if let Some(name) = self
                                .sidebar_state
                                .selected_drone_name(&self.drones, &self.display_order)
                            {
                                let result = drone_actions::execute_clean(&name);
                                self.set_status_message(result.message);
                            }
                        }
                        // Drone action: view logs
                        KeyCode::Char('l') if key.modifiers == KeyModifiers::NONE => {
                            if let Some(name) = self
                                .sidebar_state
                                .selected_drone_name(&self.drones, &self.display_order)
                            {
                                if let Err(e) = self.log_viewer.open(&name) {
                                    self.set_status_message(format!("Failed to open logs: {}", e));
                                }
                            }
                        }
                        // Drone action: launch new drone
                        KeyCode::Char('n') if key.modifiers == KeyModifiers::NONE => {
                            if let Err(e) = self.prd_selection.open() {
                                self.set_status_message(format!("No PRDs found: {}", e));
                            }
                        }
                        _ => {}
                    }
                    return Ok(());
                }

                // Pass other events to input state (Chat pane)
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
            Event::Resize(_, _) => {}
            _ => {}
        }
        Ok(())
    }

    /// Get keybinding hints for the footer
    pub fn get_keybindings(&self) -> Vec<(&'static str, &'static str)> {
        let mut hints = vec![
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
        ];

        if self.sidebar_visible {
            hints.push(("Tab", "switch pane"));
        }

        if self.focused_pane == FocusedPane::Chat {
            hints.push(("Ctrl+Enter", "submit"));
        }

        if self.focused_pane == FocusedPane::Sidebar {
            hints.push(("j/k", "navigate"));
            hints.push(("Enter", "expand"));
            hints.push(("x", "stop"));
            hints.push(("c", "clean"));
            hints.push(("l", "logs"));
            hints.push(("n", "new drone"));
        }

        hints
    }

    /// Handle a user prompt submission
    pub fn handle_submit(&mut self, prompt: String) -> Result<()> {
        self.messages.push(Message::user(prompt.clone()));

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

        if let Some(claude) = &self.claude {
            if let Err(e) = claude.send_input(prompt) {
                self.messages
                    .push(Message::error(format!("Failed to send prompt: {}", e)));
            }
        }

        Ok(())
    }

    fn process_pending_commands(&mut self) {
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

    fn handle_command_result(&mut self, result: CommandResult) {
        match result {
            CommandResult::NewSession => {
                self.create_new_session();
            }
            CommandResult::ResumeSession => {
                self.session_list.toggle(&self.session_store);
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

    fn create_new_session(&mut self) {
        self.messages.clear();
        match self.session_store.create_session() {
            Ok(session) => {
                self.current_session_id = Some(session.id);
                self.messages
                    .push(Message::system("New session started".to_string()));
            }
            Err(e) => {
                self.messages
                    .push(Message::error(format!("Failed to create session: {}", e)));
            }
        }
    }

    fn handle_session_action(&mut self, action: SessionAction) {
        match action {
            SessionAction::Select(id) => {
                self.current_session_id = Some(id.clone());
                self.messages.clear();
                self.messages
                    .push(Message::system(format!("Resumed session {}", id)));
                self.session_list.visible = false;
            }
            SessionAction::NewSession => {
                if let Ok(session) = self.session_store.create_session() {
                    self.current_session_id = Some(session.id.clone());
                    self.messages.clear();
                    self.messages
                        .push(Message::system("New session created".to_string()));
                }
                self.session_list.visible = false;
            }
            SessionAction::Close => {
                self.session_list.visible = false;
            }
        }
    }

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

    pub fn poll_claude_messages(&mut self) {
        if let Some(rx) = &self.claude_rx {
            while let Ok(message) = rx.try_recv() {
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
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let result = run_event_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_event_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    loop {
        app.poll_claude_messages();
        app.refresh_drones();
        app.clear_expired_status();

        terminal.draw(|f| ui::render(f, app))?;

        if event::poll(Duration::from_millis(POLL_TIMEOUT_MS))? {
            let event = event::read()?;
            app.handle_event(event)?;
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
