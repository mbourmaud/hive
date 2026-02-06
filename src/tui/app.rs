use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Duration;

use super::claude::ClaudeBackend;
use super::input::InputState;
use super::layout::AppLayout;
use super::messages::Message;
use super::theme::Theme;
use super::ui;

/// Poll timeout for event handling in milliseconds
const POLL_TIMEOUT_MS: u64 = 100;

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
    /// Chat message history
    pub messages: Vec<Message>,
    /// Current theme
    pub theme: Theme,
}

impl App {
    pub fn new() -> Self {
        Self {
            should_quit: false,
            sidebar_visible: true,
            input_state: InputState::new(),
            layout: AppLayout::new(),
            claude: None,
            messages: Vec::new(),
            theme: Theme::default(),
        }
    }

    /// Handle keyboard events
    pub fn handle_event(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Key(key) => {
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
                        _ => {}
                    }
                }

                // Handle 'q' for quit
                if key.code == KeyCode::Char('q') && !key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.should_quit = true;
                    return Ok(());
                }

                // Pass other events to input state
                self.input_state.handle_event(event)?;
            }
            Event::Resize(_, _) => {
                // Terminal was resized, layout will be recalculated on next render
            }
            _ => {}
        }
        Ok(())
    }

    /// Get keybinding hints for the footer
    pub fn get_keybindings(&self) -> Vec<(&'static str, &'static str)> {
        vec![
            ("q", "quit"),
            ("Ctrl+C", "quit"),
            ("Ctrl+B", if self.sidebar_visible { "hide sidebar" } else { "show sidebar" }),
            ("Ctrl+Enter", "submit"),
            ("Ctrl+A/E", "start/end of line"),
            ("Ctrl+K/U", "delete to end/start"),
            ("Up/Down", "history"),
        ]
    }

    /// Handle a user prompt submission
    pub fn handle_submit(&mut self, prompt: String) -> Result<()> {
        // Add user message to history
        self.messages.push(Message::User(prompt.clone()));

        // Spawn Claude backend if not already running
        if self.claude.is_none() {
            match ClaudeBackend::spawn() {
                Ok(backend) => {
                    self.claude = Some(backend);
                }
                Err(e) => {
                    self.messages.push(Message::Error(format!("Failed to start Claude: {}", e)));
                    return Ok(());
                }
            }
        }

        // Send prompt to Claude
        if let Some(claude) = &self.claude {
            if let Err(e) = claude.send_prompt(&prompt) {
                self.messages.push(Message::Error(format!("Failed to send prompt: {}", e)));
            }
        }

        Ok(())
    }

    /// Poll for new messages from Claude
    pub fn poll_claude_messages(&mut self) {
        if let Some(claude) = &mut self.claude {
            // Collect all pending messages
            while let Some(message) = claude.try_recv() {
                self.messages.push(message);
            }

            // Check if process is still running
            if !claude.is_running() {
                self.messages.push(Message::Error("Claude process terminated".to_string()));
                self.claude = None;
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
