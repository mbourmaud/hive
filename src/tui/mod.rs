mod app;
mod chat;
pub mod claude;
mod commands;
mod dialogs;
mod drone_actions;
mod file_picker;
mod input;
mod layout;
mod markdown;
pub mod messages;
pub mod monitor;
mod permissions;
mod session_store;
mod sessions;
pub mod sidebar;
mod theme;

pub use app::App;
pub use input::InputWidget;
pub use messages::{ChatMessage, ClaudeEvent};

use std::io;

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::CrosstermBackend;
use ratatui::Terminal;

pub fn run_tui() -> anyhow::Result<()> {
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let result = run_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App<'_>,
) -> anyhow::Result<()> {
    let mut refresh_counter: u8 = 0;

    while app.running {
        // Refresh drone status every ~2 seconds (20 ticks at 100ms)
        refresh_counter = refresh_counter.wrapping_add(1);
        if refresh_counter % 20 == 0 {
            app.refresh_sidebar();
        }

        // Poll Claude events if connected
        if let Some(claude) = &mut app.claude {
            let events = claude.poll_events();
            for ev in events {
                match ev {
                    messages::ClaudeEvent::AssistantText(text) => {
                        app.chat.append_to_last_assistant(&text);
                    }
                    messages::ClaudeEvent::ToolUse(info) => {
                        app.chat.add_message(ChatMessage::system(format!(
                            "Tool: {} ({})",
                            info.tool_name, info.input_preview
                        )));
                    }
                    messages::ClaudeEvent::ToolResult {
                        output_preview,
                        is_error,
                        ..
                    } => {
                        if is_error {
                            app.chat.add_message(ChatMessage::error(output_preview));
                        }
                    }
                    messages::ClaudeEvent::PermissionRequest {
                        tool_name,
                        args_preview,
                    } => {
                        app.permission_dialog = Some(permissions::create_permission_dialog(
                            &tool_name,
                            &args_preview,
                            &app.theme,
                        ));
                    }
                    messages::ClaudeEvent::Finished => {
                        app.set_status("Claude finished");
                    }
                    messages::ClaudeEvent::Error(msg) => {
                        app.chat.add_message(ChatMessage::error(msg));
                    }
                }
            }
        }

        // Clear expired status messages (after 5 seconds)
        if let Some((_, when)) = &app.status_message {
            if when.elapsed() > std::time::Duration::from_secs(5) {
                app.status_message = None;
            }
        }

        terminal.draw(|frame| {
            layout::render(frame, app);
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                handle_key(app, key);
            }
        }
    }

    Ok(())
}

fn handle_key(app: &mut App<'_>, key: event::KeyEvent) {
    // Permission dialog takes priority
    if let Some(dialog) = &app.permission_dialog {
        if let KeyCode::Char(c) = key.code {
            if let Some(choice) = dialog.handle_key(c) {
                match choice {
                    'y' | 'a' => {
                        if let Some(claude) = &mut app.claude {
                            let _ = claude.send_permission_response(true);
                        }
                    }
                    'n' => {
                        if let Some(claude) = &mut app.claude {
                            let _ = claude.send_permission_response(false);
                        }
                    }
                    _ => {}
                }
                app.permission_dialog = None;
                return;
            }
        }
        if key.code == KeyCode::Esc {
            app.permission_dialog = None;
        }
        return;
    }

    // Command palette overlay
    if app.commands.visible {
        match key.code {
            KeyCode::Esc => app.commands.hide(),
            KeyCode::Enter => {
                if let Some(cmd) = app.commands.selected_command() {
                    let name = cmd.name.clone();
                    app.commands.hide();
                    app.on_submit(name);
                }
            }
            KeyCode::Up => app.commands.select_prev(),
            KeyCode::Down => app.commands.select_next(),
            KeyCode::Backspace => {
                if app.commands.query.is_empty() {
                    app.commands.hide();
                } else {
                    app.commands.backspace();
                }
            }
            KeyCode::Char(c) => app.commands.type_char(c),
            _ => {}
        }
        return;
    }

    // File picker overlay
    if app.file_picker.visible {
        match key.code {
            KeyCode::Esc => app.file_picker.hide(),
            KeyCode::Enter => {
                if let Some(file) = app.file_picker.selected_file() {
                    let file = file.to_string();
                    app.file_picker.hide();
                    app.input.insert_text(&file);
                }
            }
            KeyCode::Up => app.file_picker.select_prev(),
            KeyCode::Down => app.file_picker.select_next(),
            KeyCode::Backspace => {
                if app.file_picker.query.is_empty() {
                    app.file_picker.hide();
                } else {
                    app.file_picker.backspace();
                }
            }
            KeyCode::Char(c) => app.file_picker.type_char(c),
            _ => {}
        }
        return;
    }

    // Session list overlay
    if app.sessions.visible {
        match key.code {
            KeyCode::Esc => app.sessions.hide(),
            KeyCode::Char('j') | KeyCode::Down => app.sessions.select_next(),
            KeyCode::Char('k') | KeyCode::Up => app.sessions.select_prev(),
            KeyCode::Enter => {
                if let Some(session) = app.sessions.selected_session() {
                    let id = session.id.clone();
                    if let Ok(data) = app.sessions.store().load_session(&id) {
                        app.chat = chat::ChatPanel::new();
                        for msg in &data.messages {
                            let chat_msg = match msg.role.as_str() {
                                "user" => ChatMessage::user(&msg.content),
                                "assistant" => ChatMessage::assistant(&msg.content),
                                "error" => ChatMessage::error(&msg.content),
                                _ => ChatMessage::system(&msg.content),
                            };
                            app.chat.add_message(chat_msg);
                        }
                        app.set_status(format!("Loaded session: {}", data.title));
                    }
                    app.sessions.hide();
                }
            }
            _ => {}
        }
        return;
    }

    // Global Ctrl shortcuts
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('c') => {
                app.quit();
                return;
            }
            KeyCode::Char('b') => {
                app.toggle_sidebar();
                return;
            }
            KeyCode::Char('t') => {
                app.toggle_theme();
                return;
            }
            KeyCode::Char('n') => {
                app.chat = chat::ChatPanel::new();
                app.set_status("New session");
                return;
            }
            KeyCode::Char('l') => {
                app.sessions.show();
                return;
            }
            _ => {}
        }
    }

    // When in Sidebar focus, forward keys to the sidebar
    if matches!(app.focus, app::Focus::Sidebar) {
        match key.code {
            KeyCode::Char('x') => {
                if let Some(drone) = app.sidebar.selected_drone() {
                    let name = drone.name.clone();
                    match drone_actions::stop_drone(&name) {
                        Ok(_) => app.set_status(format!("Stopped {}", name)),
                        Err(e) => app.set_status(format!("Stop failed: {}", e)),
                    }
                    app.refresh_sidebar();
                }
                return;
            }
            KeyCode::Char('c') => {
                if let Some(drone) = app.sidebar.selected_drone() {
                    let name = drone.name.clone();
                    match drone_actions::clean_drone(&name) {
                        Ok(_) => app.set_status(format!("Cleaned {}", name)),
                        Err(e) => app.set_status(format!("Clean failed: {}", e)),
                    }
                    app.refresh_sidebar();
                }
                return;
            }
            KeyCode::Char('l') => {
                if let Some(drone) = app.sidebar.selected_drone() {
                    let name = drone.name.clone();
                    match drone_actions::view_logs(&name) {
                        Ok(logs) => {
                            app.chat.add_message(ChatMessage::system(format!(
                                "Logs for {}:\n{}",
                                name, logs
                            )));
                        }
                        Err(e) => app.set_status(format!("Logs failed: {}", e)),
                    }
                }
                return;
            }
            _ => {}
        }
        if sidebar::handle_key(&mut app.sidebar, key) {
            return;
        }
    }

    // When in Input focus, forward keys to the input widget
    if matches!(app.focus, app::Focus::Input) {
        match key.code {
            KeyCode::Esc => {
                app.focus = app::Focus::Chat;
            }
            _ => {
                let action = app.input.handle_key(key);
                match action {
                    input::InputAction::Submit(text) => {
                        if text == "/" {
                            app.commands.show();
                        } else {
                            app.on_submit(text);
                        }
                    }
                    input::InputAction::FilePickerTrigger => {
                        app.file_picker.show();
                    }
                    input::InputAction::None => {}
                }
            }
        }
        return;
    }

    // Chat focus keys
    if matches!(app.focus, app::Focus::Chat) {
        match key.code {
            KeyCode::PageUp => {
                app.chat.scroll_up(10);
                return;
            }
            KeyCode::PageDown => {
                app.chat.scroll_down(10);
                return;
            }
            _ => {}
        }
    }

    // Global non-ctrl keys
    match key.code {
        KeyCode::Char('q') => app.quit(),
        KeyCode::Tab => app.cycle_focus(),
        KeyCode::Char('i') | KeyCode::Char('/') => {
            if matches!(app.focus, app::Focus::Chat | app::Focus::Sidebar) {
                app.focus = app::Focus::Input;
            }
        }
        _ => {}
    }
}
