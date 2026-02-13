use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use super::keybinds::KeyAction;
use super::state::TuiState;

/// Event poll timeout in milliseconds for TUI
const TUI_POLL_TIMEOUT_MS: u64 = 100;

pub(crate) fn run_tui(_name: Option<String>) -> Result<()> {
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

    let mut state = TuiState::new()?;

    loop {
        state.tick()?;
        terminal.draw(|f| state.render(f))?;
        state.clear_message();

        let mut d_pressed = false;
        if event::poll(std::time::Duration::from_millis(TUI_POLL_TIMEOUT_MS))? {
            match event::read()? {
                Event::Resize(_, _) => continue,
                Event::Key(key) => {
                    if key.code == KeyCode::Char('D') {
                        d_pressed = true;
                    }
                    // Ctrl+C always quits immediately
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('c')
                    {
                        break;
                    }
                    if let KeyAction::Break = state.handle_key(key, &mut terminal)? {
                        break;
                    }
                }
                _ => {}
            }
        }

        // Cancel pending clean when D is released (no D event this tick)
        // But only cancel if the countdown hasn't already completed (3s).
        // If it already hit 3s, the clean was executed and pending_clean was
        // cleared by handle_key, so this won't fire. The grace period handles
        // the edge case where key-repeat jitter causes a single tick gap.
        if !d_pressed {
            if let Some((_, when)) = &state.pending_clean {
                if when.elapsed() < std::time::Duration::from_secs(3) {
                    // User released before 3 seconds — cancel
                    state.pending_clean = None;
                    state.set_message(
                        "Clean cancelled".to_string(),
                        ratatui::style::Color::DarkGray,
                    );
                }
                // If elapsed >= 3s but pending_clean is still Some, it means
                // we missed the D event that would have triggered the clean.
                // Execute it now instead of cancelling.
                else {
                    let name = state.pending_clean.take().unwrap().0;
                    match super::drone_actions::handle_clean_drone(&name) {
                        Ok(msg) => state.set_message(msg, ratatui::style::Color::Green),
                        Err(e) => {
                            state.set_message(format!("Error: {}", e), ratatui::style::Color::Red)
                        }
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
