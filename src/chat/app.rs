use anyhow::Result;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::Paragraph,
    Frame, Terminal,
};
use std::io;
use std::sync::mpsc;
use std::time::Duration;

use super::claude::{ClaudeEvent, ClaudeProcess};
use super::commands::{self, SlashCommand};
use super::dialogs::{default_models, Dialog, DialogAction, SessionListItem};
use super::input::{InputAction, InputEditor};
use super::keybinds::{KeyAction, KeyHandler};
use super::messages::MessageDisplay;
use super::provider::ProviderConfig;
use super::session::{PersistedMessage, SessionManager};
use super::sidebar::{Sidebar, SidebarSessionInfo};
use super::theme;

/// Event poll timeout in milliseconds for chat TUI
const TUI_POLL_TIMEOUT_MS: u64 = 50;

#[derive(Debug, Clone, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Error,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct ChatApp {
    // Core state
    pub should_quit: bool,
    pub sidebar_visible: bool,

    // Messages
    pub messages: Vec<ChatMessage>,
    pub message_display: MessageDisplay,

    // Input editor
    pub input: InputEditor,

    // Session persistence
    pub session_id: Option<String>,
    pub session_manager: SessionManager,

    // Sidebar
    pub sidebar_width: u16,
    pub sidebar: Sidebar,

    // Leader key handler (Ctrl+X prefix system)
    pub key_handler: KeyHandler,

    // Claude subprocess
    pub claude: ClaudeProcess,
    pub event_rx: Option<mpsc::Receiver<ClaudeEvent>>,

    // Overlay dialogs
    pub active_dialog: Option<Dialog>,

    // Provider configuration
    pub provider: ProviderConfig,
}

impl ChatApp {
    pub fn new() -> Self {
        let provider = ProviderConfig::load();
        let model = provider.default_model.clone();
        let working_dir = std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let mut session_manager = SessionManager::new().unwrap_or_else(|e| {
            eprintln!("Warning: failed to initialize session manager: {}", e);
            // Fallback: create a minimal session manager using current dir
            SessionManager::new().expect("Session manager initialization failed")
        });

        let (session_id, messages) = match session_manager.resume_or_create(&model) {
            Ok((metadata, persisted_msgs)) => {
                let msgs: Vec<ChatMessage> = persisted_msgs
                    .iter()
                    .map(|pm| pm.to_chat_message())
                    .collect();
                (metadata.claude_session_id, msgs)
            }
            Err(_) => (None, Vec::new()),
        };

        let mut claude = ClaudeProcess::new(model, working_dir);
        if let Some(ref sid) = session_id {
            claude.set_session_id(sid.clone());
        }

        Self {
            should_quit: false,
            sidebar_visible: false,
            messages,
            message_display: MessageDisplay::new(),
            input: InputEditor::new(),
            session_id,
            session_manager,
            sidebar_width: theme::SIDEBAR_WIDTH,
            sidebar: Sidebar::new(),
            key_handler: KeyHandler::new(),
            claude,
            event_rx: None,
            active_dialog: None,
            provider,
        }
    }

    pub fn tick(&mut self) -> Result<()> {
        // Drain events from Claude subprocess if active
        if let Some(rx) = self.event_rx.take() {
            let mut disconnected = false;
            loop {
                match rx.try_recv() {
                    Ok(event) => self.handle_claude_event(event),
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }
            if disconnected {
                self.message_display.set_streaming(false);
            } else {
                self.event_rx = Some(rx);
            }
        }
        self.sidebar.tick()?;
        Ok(())
    }

    fn handle_claude_event(&mut self, event: ClaudeEvent) {
        match event {
            ClaudeEvent::Init {
                session_id,
                model: _,
            } => {
                self.session_id = Some(session_id.clone());
                self.claude.set_session_id(session_id.clone());
                // Persist the Claude CLI session ID for --resume
                if let Some(ref mut meta) = self.session_manager.current().cloned() {
                    meta.claude_session_id = Some(session_id);
                    let _ = self.session_manager.save_metadata(meta);
                }
            }
            ClaudeEvent::ContentBlockDelta { delta, .. } => {
                self.message_display.set_streaming(true);
                if let Some(last) = self.messages.last_mut() {
                    if last.role == MessageRole::Assistant {
                        last.content.push_str(&delta);
                        return;
                    }
                }
                self.messages.push(ChatMessage {
                    role: MessageRole::Assistant,
                    content: delta,
                    timestamp: chrono::Utc::now(),
                });
            }
            ClaudeEvent::ContentBlockStart { content_type, .. } => {
                self.message_display.set_streaming(true);
                if content_type.starts_with("tool_use:") {
                    let tool_name = content_type.strip_prefix("tool_use:").unwrap_or("unknown");
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: format!("[Using tool: {}]", tool_name),
                        timestamp: chrono::Utc::now(),
                    });
                }
            }
            ClaudeEvent::Result {
                session_id,
                input_tokens,
                output_tokens,
                cost_usd,
                ..
            } => {
                self.message_display.set_streaming(false);
                if !session_id.is_empty() {
                    self.session_id = Some(session_id.clone());
                    self.claude.set_session_id(session_id);
                }
                // Update token usage in session
                let _ = self
                    .session_manager
                    .update_usage(input_tokens, output_tokens, cost_usd);
                // Auto-title the session from the first user message
                if let Some(meta) = self.session_manager.current().cloned() {
                    if meta.title == "New Chat" {
                        let persisted_msgs: Vec<PersistedMessage> = self
                            .messages
                            .iter()
                            .map(PersistedMessage::from_chat_message)
                            .collect();
                        let title = SessionManager::auto_title(&persisted_msgs);
                        if title != "New Chat" {
                            let mut updated = meta;
                            updated.title = title;
                            let _ = self.session_manager.save_metadata(&updated);
                        }
                    }
                }
            }
            ClaudeEvent::Error { message } => {
                self.message_display.set_streaming(false);
                self.messages.push(ChatMessage {
                    role: MessageRole::Error,
                    content: message,
                    timestamp: chrono::Utc::now(),
                });
            }
            ClaudeEvent::ContentBlockStop { .. } => {
                // Persist the last assistant message when a text block completes
                if let Some(last) = self.messages.last() {
                    if last.role == MessageRole::Assistant {
                        if let Some(meta) = self.session_manager.current() {
                            let session_id = meta.id.clone();
                            let persisted = PersistedMessage::from_chat_message(last);
                            let _ = self.session_manager.append_message(&session_id, &persisted);
                        }
                    }
                }
            }
        }
    }

    pub fn handle_key(&mut self, key: event::KeyEvent) -> Result<()> {
        // If a dialog is active, route all keys to it first
        if let Some(ref mut dialog) = self.active_dialog {
            match dialog.handle_key(key) {
                DialogAction::Continue => return Ok(()),
                DialogAction::Close => {
                    self.active_dialog = None;
                    return Ok(());
                }
                DialogAction::SelectSession(session_id) => {
                    self.active_dialog = None;
                    self.load_session(&session_id);
                    return Ok(());
                }
                DialogAction::SelectModel(model_id) => {
                    self.active_dialog = None;
                    self.switch_model(&model_id);
                    return Ok(());
                }
            }
        }

        match self.key_handler.handle_key(key) {
            KeyAction::Quit => {
                self.should_quit = true;
            }
            KeyAction::Interrupt => {
                if self.event_rx.is_some() {
                    // Interrupt Claude
                    let _ = self.claude.interrupt();
                    self.event_rx = None;
                    self.message_display.set_streaming(false);
                } else {
                    self.should_quit = true;
                }
            }
            KeyAction::NewSession => {
                let model = self.provider.default_model.clone();
                if let Ok(meta) = self.session_manager.create_session(&model) {
                    self.messages.clear();
                    self.session_id = meta.claude_session_id.clone();
                    self.claude = ClaudeProcess::new(
                        model,
                        std::env::current_dir()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string(),
                    );
                    self.event_rx = None;
                }
            }
            KeyAction::ToggleSidebar => {
                self.sidebar_visible = !self.sidebar_visible;
            }
            KeyAction::SessionList => {
                self.open_session_list();
            }
            KeyAction::ModelPicker => {
                self.active_dialog = Some(Dialog::ModelPicker {
                    models: default_models(),
                    selected: 0,
                });
            }
            KeyAction::DroneList => {
                // Show sidebar with drones section
                self.sidebar_visible = true;
            }
            KeyAction::ScrollUp => {
                self.message_display.page_up(20);
            }
            KeyAction::ScrollDown => {
                self.message_display.page_down(20);
            }
            KeyAction::PageUp => {
                self.message_display.page_up(20);
            }
            KeyAction::PageDown => {
                self.message_display.page_down(20);
            }
            KeyAction::PassToInput(key_event) => {
                match self.input.handle_key(key_event) {
                    InputAction::Submit(text) => {
                        // Check for slash commands before sending to Claude
                        if text.starts_with('/') {
                            self.input.clear(true);
                            if let Some(cmd) = commands::parse_command(&text) {
                                self.handle_command(cmd);
                            } else {
                                self.messages.push(ChatMessage {
                                    role: MessageRole::Error,
                                    content: format!(
                                        "Unknown command: {}",
                                        text.split_whitespace().next().unwrap_or(&text)
                                    ),
                                    timestamp: chrono::Utc::now(),
                                });
                            }
                            self.message_display.scroll_to_bottom();
                        } else {
                            let user_msg = ChatMessage {
                                role: MessageRole::User,
                                content: text.clone(),
                                timestamp: chrono::Utc::now(),
                            };
                            if let Some(meta) = self.session_manager.current() {
                                let session_id = meta.id.clone();
                                let persisted = PersistedMessage::from_chat_message(&user_msg);
                                let _ =
                                    self.session_manager.append_message(&session_id, &persisted);
                            }
                            self.messages.push(user_msg);
                            self.input.clear(true);
                            self.message_display.scroll_to_bottom();
                            match self.claude.send_message(&text) {
                                Ok(rx) => {
                                    self.event_rx = Some(rx);
                                }
                                Err(e) => {
                                    self.messages.push(ChatMessage {
                                        role: MessageRole::Error,
                                        content: format!("Failed to send: {}", e),
                                        timestamp: chrono::Utc::now(),
                                    });
                                }
                            }
                        }
                    }
                    InputAction::Continue | InputAction::Noop => {}
                }
            }
            KeyAction::None => {}
        }
        Ok(())
    }

    fn open_session_list(&mut self) {
        let current_id = self.session_manager.current().map(|m| m.id.clone());
        let sessions = self.session_manager.list_sessions().unwrap_or_default();
        let items: Vec<SessionListItem> = sessions
            .iter()
            .map(|s| SessionListItem {
                id: s.id.clone(),
                title: s.title.clone(),
                model: s.model.clone(),
                message_count: s.message_count,
                cost: s.total_cost_usd,
                is_current: Some(&s.id) == current_id.as_ref(),
            })
            .collect();
        self.active_dialog = Some(Dialog::SessionList {
            sessions: items,
            selected: 0,
        });
    }

    fn load_session(&mut self, session_id: &str) {
        match self.session_manager.load_session(session_id) {
            Ok((meta, persisted_msgs)) => {
                self.messages = persisted_msgs
                    .iter()
                    .map(|pm| pm.to_chat_message())
                    .collect();
                self.session_id = meta.claude_session_id.clone();
                let model = meta.model.clone();
                self.claude = ClaudeProcess::new(
                    model,
                    std::env::current_dir()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                );
                if let Some(ref sid) = self.session_id {
                    self.claude.set_session_id(sid.clone());
                }
                self.event_rx = None;
                self.message_display.scroll_to_bottom();
            }
            Err(e) => {
                self.messages.push(ChatMessage {
                    role: MessageRole::Error,
                    content: format!("Failed to load session: {}", e),
                    timestamp: chrono::Utc::now(),
                });
            }
        }
    }

    fn switch_model(&mut self, model_id: &str) {
        self.claude = ClaudeProcess::new(
            model_id.to_string(),
            std::env::current_dir()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
        );
        if let Some(ref sid) = self.session_id {
            self.claude.set_session_id(sid.clone());
        }
        let display = self.provider.display_name(model_id);
        self.messages.push(ChatMessage {
            role: MessageRole::System,
            content: format!("Switched to model: {}", display),
            timestamp: chrono::Utc::now(),
        });
    }

    fn handle_command(&mut self, cmd: SlashCommand) {
        match cmd {
            SlashCommand::New => {
                let model = self.provider.default_model.clone();
                if let Ok(meta) = self.session_manager.create_session(&model) {
                    self.messages.clear();
                    self.session_id = meta.claude_session_id.clone();
                    self.claude = ClaudeProcess::new(
                        model,
                        std::env::current_dir()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string(),
                    );
                    self.event_rx = None;
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: "New session created.".to_string(),
                        timestamp: chrono::Utc::now(),
                    });
                }
            }
            SlashCommand::Help => {
                self.active_dialog = Some(Dialog::Help);
            }
            SlashCommand::Clear => {
                self.messages.clear();
            }
            SlashCommand::Sessions => {
                self.open_session_list();
            }
            SlashCommand::Model(name) => match name {
                Some(n) => {
                    let model_id = super::provider::resolve_model_id(&n);
                    self.switch_model(&model_id);
                }
                None => {
                    self.active_dialog = Some(Dialog::ModelPicker {
                        models: default_models(),
                        selected: 0,
                    });
                }
            },
            SlashCommand::Status => {
                let drones = crate::commands::common::list_drones().unwrap_or_default();
                if drones.is_empty() {
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: "No active drones.".to_string(),
                        timestamp: chrono::Utc::now(),
                    });
                } else {
                    let mut text = String::from("## Active Drones\n\n");
                    for (name, status) in &drones {
                        text.push_str(&format!(
                            "  {} -- {} ({}/{})\n",
                            name,
                            status.status,
                            status.completed.len(),
                            status.total
                        ));
                    }
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: text,
                        timestamp: chrono::Utc::now(),
                    });
                }
            }
            SlashCommand::Stop(name) => {
                match crate::commands::kill_clean::kill_quiet(name.clone()) {
                    Ok(_) => {
                        self.messages.push(ChatMessage {
                            role: MessageRole::System,
                            content: format!("Drone '{}' stopped.", name),
                            timestamp: chrono::Utc::now(),
                        });
                    }
                    Err(e) => {
                        self.messages.push(ChatMessage {
                            role: MessageRole::Error,
                            content: format!("Failed to stop '{}': {}", name, e),
                            timestamp: chrono::Utc::now(),
                        });
                    }
                }
            }
            SlashCommand::Monitor => {
                self.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: "Use 'hive monitor' in a separate terminal for the full dashboard."
                        .to_string(),
                    timestamp: chrono::Utc::now(),
                });
            }
            SlashCommand::Plan(_)
            | SlashCommand::Start(_)
            | SlashCommand::Compact
            | SlashCommand::Share => {
                self.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: "This command is not yet implemented.".to_string(),
                    timestamp: chrono::Utc::now(),
                });
            }
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Horizontal split: main area + optional sidebar
        let horizontal_constraints = if self.sidebar_visible {
            vec![Constraint::Min(1), Constraint::Length(self.sidebar_width)]
        } else {
            vec![Constraint::Min(1)]
        };

        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(horizontal_constraints)
            .split(area);

        let main_area = horizontal[0];

        // Vertical split within main area: messages + input
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(theme::INPUT_HEIGHT)])
            .split(main_area);

        let messages_area = vertical[0];
        let input_area = vertical[1];

        // Render messages area using MessageDisplay
        self.message_display
            .render(frame, messages_area, &self.messages);

        // Render input area using InputEditor
        self.input.render(frame, input_area, true);

        // Show leader key indicator
        if self.key_handler.is_leader_active() && input_area.width >= 14 {
            let indicator = Paragraph::new(" Ctrl+X... ").style(
                Style::default()
                    .fg(theme::SECONDARY)
                    .bg(ratatui::style::Color::DarkGray),
            );
            let indicator_area = Rect::new(
                input_area.x + input_area.width.saturating_sub(14),
                input_area.y,
                14,
                1,
            );
            frame.render_widget(indicator, indicator_area);
        }

        // Render sidebar if visible
        if self.sidebar_visible && horizontal.len() > 1 {
            let sidebar_area = horizontal[1];

            let (title, model, duration, input_tokens, output_tokens, total_cost_usd) =
                if let Some(meta) = self.session_manager.current() {
                    let dur = crate::commands::common::elapsed_since(&meta.created_at)
                        .unwrap_or_else(|| "0s".to_string());
                    (
                        meta.title.clone(),
                        meta.model.clone(),
                        dur,
                        meta.input_tokens,
                        meta.output_tokens,
                        meta.total_cost_usd,
                    )
                } else {
                    (
                        "New Chat".to_string(),
                        "claude-sonnet-4-5-20250929".to_string(),
                        "0s".to_string(),
                        0,
                        0,
                        0.0,
                    )
                };

            let session_info = SidebarSessionInfo {
                title,
                model,
                duration,
                input_tokens,
                output_tokens,
                total_cost_usd,
                is_streaming: self.event_rx.is_some(),
            };

            self.sidebar.render(frame, sidebar_area, &session_info);
        }

        // Render dialog overlay on top of everything
        if let Some(ref dialog) = self.active_dialog {
            dialog.render(frame, area);
        }
    }
}

pub fn run_chat() -> Result<()> {
    // Install panic hook to restore terminal before printing panic info
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

    let mut app = ChatApp::new();

    loop {
        app.tick()?;
        terminal.draw(|f| {
            app.render(f);
        })?;

        if event::poll(Duration::from_millis(TUI_POLL_TIMEOUT_MS))? {
            if let Event::Key(key) = event::read()? {
                app.handle_key(key)?;
            }
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
