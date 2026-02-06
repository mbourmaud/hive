use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    text::{Line, Span},
    widgets::Paragraph,
    Terminal,
};
use std::io;

use super::chat::ChatPanel;
use super::claude::{ClaudeEvent, ClaudeProcess};
use super::commands::{self, CommandPalette, CommandResult};
use super::dialogs::DialogAction;
use super::file_picker::FilePicker;
use super::input::ChatInput;
use super::layout::AppLayout;
use super::messages::{ChatMessage, ToolStatus};
use super::permissions::PermissionManager;
use super::session_store::{SessionMeta, SessionStore};
use super::sessions::{SessionAction, SessionList};
use super::sidebar::SidebarState;
use super::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActivePane {
    Sidebar,
    Chat,
}

pub struct TuiApp {
    sidebar_visible: bool,
    should_quit: bool,
    active_pane: ActivePane,
    messages: Vec<ChatMessage>,
    claude_process: Option<ClaudeProcess>,
    sidebar_state: SidebarState,
    chat_input: ChatInput,
    chat_panel: ChatPanel,
    permission_manager: PermissionManager,
    session_store: Option<SessionStore>,
    session_list: SessionList,
    current_session: Option<SessionMeta>,
    command_palette: CommandPalette,
    file_picker: FilePicker,
    theme: Theme,
}

impl TuiApp {
    pub fn new() -> Self {
        let session_store = SessionStore::new().ok();
        Self {
            sidebar_visible: true,
            should_quit: false,
            active_pane: ActivePane::Chat,
            messages: Vec::new(),
            claude_process: None,
            sidebar_state: SidebarState::new(),
            chat_input: ChatInput::new(),
            chat_panel: ChatPanel::new(),
            permission_manager: PermissionManager::new(),
            session_store,
            session_list: SessionList::new(),
            current_session: None,
            command_palette: CommandPalette::new(),
            file_picker: FilePicker::new(),
            theme: Theme::dark(),
        }
    }

    /// Send a prompt to Claude Code, spawning a new process if needed
    pub fn send_prompt(&mut self, prompt: &str) {
        self.messages.push(ChatMessage::user(prompt.to_string()));

        // Auto-set session title from first user message
        if let Some(ref mut session) = self.current_session {
            if session.title == "New Chat" {
                let title: String = prompt.chars().take(50).collect();
                session.title = title;
                if let Some(ref store) = self.session_store {
                    let _ = store.save(session);
                }
            }
        }

        let session_id = self
            .current_session
            .as_ref()
            .and_then(|s| s.claude_session_id.as_deref());

        match ClaudeProcess::spawn(prompt, session_id) {
            Ok(process) => {
                self.claude_process = Some(process);
            }
            Err(e) => {
                self.messages
                    .push(ChatMessage::error(format!("Failed to spawn Claude: {}", e)));
            }
        }
    }

    /// Poll for events from the Claude subprocess and add them to messages
    fn poll_claude_events(&mut self) {
        let mut events = Vec::new();

        if let Some(ref process) = self.claude_process {
            while let Some(event) = process.try_recv() {
                events.push(event);
            }
        }

        for event in events {
            match event {
                ClaudeEvent::AssistantText(text) => {
                    self.messages.push(ChatMessage::assistant(text));
                }
                ClaudeEvent::ToolUse {
                    tool_name,
                    args_summary,
                } => {
                    if self.permission_manager.needs_approval(&tool_name) {
                        self.permission_manager
                            .request_approval(tool_name.clone(), args_summary.clone());
                    }
                    self.messages.push(ChatMessage::ToolUse {
                        tool_name,
                        args_summary,
                        status: ToolStatus::Running,
                        timestamp: chrono::Utc::now(),
                    });
                }
                ClaudeEvent::ToolResult {
                    tool_name,
                    success,
                    output,
                } => {
                    self.messages.push(ChatMessage::ToolResult {
                        tool_name,
                        success,
                        output_preview: if output.len() > 200 {
                            format!("{}...", &output[..200])
                        } else {
                            output
                        },
                        timestamp: chrono::Utc::now(),
                    });
                }
                ClaudeEvent::ProcessExit(code) => {
                    self.messages.push(ChatMessage::system(format!(
                        "Claude process exited with code {}.",
                        code
                    )));
                    self.claude_process = None;
                }
                ClaudeEvent::Error(msg) => {
                    self.messages.push(ChatMessage::error(msg));
                }
            }
        }

        // Check if process has exited
        if let Some(ref mut process) = self.claude_process {
            if !process.is_running() {
                self.messages
                    .push(ChatMessage::system("Claude process finished.".to_string()));
                self.claude_process = None;
            }
        }
    }

    fn new_session(&mut self) {
        // Kill current process if any
        if let Some(ref mut process) = self.claude_process {
            process.kill();
        }
        self.claude_process = None;

        // Save current session if exists
        if let (Some(ref store), Some(ref mut session)) =
            (&self.session_store, &mut self.current_session)
        {
            session.message_count = self.messages.len();
            session.updated_at = chrono::Utc::now();
            let _ = store.save(session);
        }

        // Create new session
        let meta = SessionMeta::new("New Chat");
        if let Some(ref store) = self.session_store {
            let _ = store.save(&meta);
        }
        self.current_session = Some(meta);
        self.messages.clear();
        self.chat_panel = ChatPanel::new();
        self.messages
            .push(ChatMessage::system("New session started.".to_string()));
    }

    fn show_session_list(&mut self) {
        if let Some(ref store) = self.session_store {
            if let Ok(sessions) = store.list() {
                self.session_list.show(sessions);
            }
        }
    }

    fn handle_session_action(&mut self, action: SessionAction) {
        match action {
            SessionAction::NewSession => self.new_session(),
            SessionAction::Switch(meta) => {
                // Save current session
                if let (Some(ref store), Some(ref mut session)) =
                    (&self.session_store, &mut self.current_session)
                {
                    session.message_count = self.messages.len();
                    session.updated_at = chrono::Utc::now();
                    let _ = store.save(session);
                }

                // Switch to selected session
                self.current_session = Some(meta.clone());
                self.messages.clear();
                self.chat_panel = ChatPanel::new();
                self.messages.push(ChatMessage::system(format!(
                    "Switched to session: {}",
                    meta.title
                )));
            }
            SessionAction::Resume(meta) => {
                if let Some(ref mut process) = self.claude_process {
                    process.kill();
                }
                self.claude_process = None;

                self.current_session = Some(meta.clone());
                self.messages.clear();
                self.chat_panel = ChatPanel::new();

                if let Some(ref sid) = meta.claude_session_id {
                    self.messages.push(ChatMessage::system(format!(
                        "Resuming session: {}",
                        meta.title
                    )));
                    match ClaudeProcess::spawn("", Some(sid)) {
                        Ok(process) => {
                            self.claude_process = Some(process);
                        }
                        Err(e) => {
                            self.messages.push(ChatMessage::error(format!(
                                "Failed to resume: {}",
                                e
                            )));
                        }
                    }
                }
            }
            SessionAction::Close => {}
        }
    }

    fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) {
        // If session list is visible, handle session keys only
        if self.session_list.visible {
            if let Some(action) = self.session_list.handle_key(key) {
                self.handle_session_action(action);
            }
            return;
        }

        // If permission dialog is active, handle dialog keys only
        if self.permission_manager.has_active_dialog() {
            self.handle_dialog_key(key);
            return;
        }

        // App-level shortcuts that always apply
        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                self.should_quit = true;
                return;
            }
            (KeyModifiers::CONTROL, KeyCode::Char('b')) => {
                self.sidebar_visible = !self.sidebar_visible;
                if !self.sidebar_visible && self.active_pane == ActivePane::Sidebar {
                    self.active_pane = ActivePane::Chat;
                }
                return;
            }
            (KeyModifiers::CONTROL, KeyCode::Char('n')) => {
                self.new_session();
                return;
            }
            (KeyModifiers::CONTROL, KeyCode::Char('l')) => {
                self.show_session_list();
                return;
            }
            (KeyModifiers::CONTROL, KeyCode::Char('t')) => {
                self.theme = self.theme.toggle();
                return;
            }
            (_, KeyCode::Tab) => {
                if self.sidebar_visible {
                    self.active_pane = match self.active_pane {
                        ActivePane::Sidebar => ActivePane::Chat,
                        ActivePane::Chat => ActivePane::Sidebar,
                    };
                }
                return;
            }
            _ => {}
        }

        // Pane-specific handling
        match self.active_pane {
            ActivePane::Chat => {
                // Chat scroll keys
                match key.code {
                    KeyCode::PageUp => {
                        self.chat_panel.page_up();
                        return;
                    }
                    KeyCode::PageDown => {
                        self.chat_panel.page_down();
                        return;
                    }
                    KeyCode::Home => {
                        self.chat_panel.scroll_to_top();
                        return;
                    }
                    KeyCode::End => {
                        self.chat_panel.scroll_to_bottom();
                        return;
                    }
                    _ => {}
                }
                if let Some(text) = self.chat_input.handle_key(key) {
                    if text.starts_with('/') {
                        self.handle_slash_command(&text);
                    } else {
                        self.send_prompt(&text);
                    }
                }
            }
            ActivePane::Sidebar => match key.code {
                KeyCode::Char('j') | KeyCode::Down => self.sidebar_state.move_down(),
                KeyCode::Char('k') | KeyCode::Up => self.sidebar_state.move_up(),
                KeyCode::Enter => self.sidebar_state.toggle_expand(),
                KeyCode::Char('q') => self.should_quit = true,
                KeyCode::Char('x') => {
                    // Stop selected drone
                    if let Some((name, _)) = self.sidebar_state.selected_drone() {
                        let name = name.clone();
                        match super::drone_actions::stop_drone(&name) {
                            Ok(msg) => self.messages.push(ChatMessage::system(msg)),
                            Err(e) => self.messages.push(ChatMessage::error(e.to_string())),
                        }
                    }
                }
                KeyCode::Char('c') => {
                    // Clean selected drone
                    if let Some((name, _)) = self.sidebar_state.selected_drone() {
                        let name = name.clone();
                        match super::drone_actions::clean_drone(&name) {
                            Ok(msg) => self.messages.push(ChatMessage::system(msg)),
                            Err(e) => self.messages.push(ChatMessage::error(e.to_string())),
                        }
                    }
                }
                KeyCode::Char('l') => {
                    // View drone logs
                    if let Some((name, _)) = self.sidebar_state.selected_drone() {
                        let name = name.clone();
                        match super::drone_actions::read_drone_logs(&name, 50) {
                            Ok(lines) => {
                                self.messages.push(ChatMessage::system(
                                    format!("--- Logs for '{}' ---", name),
                                ));
                                for line in lines {
                                    self.messages.push(ChatMessage::system(line));
                                }
                                self.messages.push(ChatMessage::system(
                                    "--- End of logs ---".to_string(),
                                ));
                            }
                            Err(e) => self.messages.push(ChatMessage::error(e.to_string())),
                        }
                    }
                }
                _ => {}
            },
        }
    }

    fn handle_slash_command(&mut self, input: &str) {
        match commands::execute_command(input) {
            CommandResult::NewSession => self.new_session(),
            CommandResult::ShowSessions => self.show_session_list(),
            CommandResult::ClearChat => {
                self.messages.clear();
                self.chat_panel = ChatPanel::new();
                self.messages
                    .push(ChatMessage::system("Chat cleared.".to_string()));
            }
            CommandResult::ShowHelp(lines) => {
                self.messages
                    .push(ChatMessage::system("Available commands:".to_string()));
                for line in lines {
                    self.messages.push(ChatMessage::system(line));
                }
            }
            CommandResult::Unknown(cmd) => {
                self.messages
                    .push(ChatMessage::error(format!("Unknown command: {}", cmd)));
            }
        }
    }

    fn handle_dialog_key(&mut self, key: crossterm::event::KeyEvent) {
        let action = match key.code {
            KeyCode::Char('y') => Some(DialogAction::Accept),
            KeyCode::Char('n') => Some(DialogAction::Reject),
            KeyCode::Char('a') => Some(DialogAction::AlwaysAllow),
            KeyCode::Enter => {
                if let Some(ref dialog) = self.permission_manager.active_dialog {
                    Some(dialog.confirm())
                } else {
                    None
                }
            }
            KeyCode::Esc => Some(DialogAction::Reject),
            KeyCode::Left => {
                if let Some(ref mut dialog) = self.permission_manager.active_dialog {
                    dialog.prev_option();
                }
                None
            }
            KeyCode::Right | KeyCode::Tab => {
                if let Some(ref mut dialog) = self.permission_manager.active_dialog {
                    dialog.next_option();
                }
                None
            }
            _ => None,
        };

        if let Some(action) = action {
            let approved = self.permission_manager.handle_action(action);
            if approved {
                self.messages.push(ChatMessage::system(
                    "Permission granted.".to_string(),
                ));
            } else {
                self.messages.push(ChatMessage::system(
                    "Permission denied.".to_string(),
                ));
            }
        }
    }

    fn render(&mut self, f: &mut ratatui::Frame) {
        let layout = AppLayout::compute(f.area(), self.sidebar_visible);

        // Render sidebar if visible
        if let Some(sidebar_area) = layout.sidebar {
            let is_focused = self.active_pane == ActivePane::Sidebar;
            self.sidebar_state.render(f, sidebar_area, is_focused);
        }

        // Split main panel into messages area (top) and input area (bottom, 5 rows)
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(5)])
            .split(layout.main_panel);
        let messages_area = main_chunks[0];
        let input_area = main_chunks[1];

        // Render main chat panel with messages (markdown rendering + scrolling)
        let is_chat_focused = self.active_pane == ActivePane::Chat;
        self.chat_panel.render(f, messages_area, &self.messages, is_chat_focused);

        // Render chat input widget
        f.render_widget(self.chat_input.widget(), input_area);

        // Render footer with keybinding hints (themed)
        let hints = self.get_keybinding_hints();
        let hint_spans: Vec<Span> = hints
            .iter()
            .enumerate()
            .flat_map(|(i, (key, desc))| {
                let mut spans = vec![
                    Span::styled(format!(" {} ", key), self.theme.hint_key_style()),
                    Span::styled(format!(" {} ", desc), self.theme.hint_desc_style()),
                ];
                if i < hints.len() - 1 {
                    spans.push(Span::raw(" "));
                }
                spans
            })
            .collect();
        let footer = Paragraph::new(Line::from(hint_spans));
        f.render_widget(footer, layout.footer);

        // Render permission dialog on top if active
        if let Some(ref dialog) = self.permission_manager.active_dialog {
            dialog.render(f, f.area());
        }

        // Render session list overlay if visible
        self.session_list.render(f, f.area());

        // Render command palette popup if visible
        self.command_palette.render(f, input_area);

        // Render file picker popup if visible
        self.file_picker.render(f, input_area);
    }

    fn get_keybinding_hints(&self) -> Vec<(&str, &str)> {
        if self.permission_manager.has_active_dialog() {
            return vec![
                ("y", "Accept"),
                ("n", "Reject"),
                ("a", "Always Allow"),
                ("Enter", "Confirm"),
                ("Esc", "Reject"),
            ];
        }

        if self.session_list.visible {
            return vec![
                ("j/k", "Navigate"),
                ("Enter", "Open"),
                ("n", "New"),
                ("Esc", "Close"),
            ];
        }

        let mut hints = vec![
            ("Ctrl+C", "Quit"),
            ("Ctrl+B", "Sidebar"),
            ("Ctrl+N", "New"),
            ("Ctrl+L", "Sessions"),
            ("Ctrl+T", "Theme"),
            ("Tab", "Pane"),
        ];
        if self.active_pane == ActivePane::Chat {
            hints.push(("Ctrl+Enter", "Send"));
        } else if self.active_pane == ActivePane::Sidebar {
            hints.push(("j/k", "Nav"));
            hints.push(("Enter", "Expand"));
            hints.push(("x", "Stop"));
            hints.push(("c", "Clean"));
            hints.push(("l", "Logs"));
        }
        hints
    }
}

/// Main TUI application entry point
pub fn run_tui() -> Result<()> {
    // Install panic hook to restore terminal
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

    let mut app = TuiApp::new();

    loop {
        // Refresh sidebar drone data each iteration
        app.sidebar_state.refresh();

        // Poll Claude events each iteration
        app.poll_claude_events();

        terminal.draw(|f| app.render(f))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                app.handle_key_event(key);
            }
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
