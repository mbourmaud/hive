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
        if !d_pressed && state.pending_clean.is_some() {
            state.pending_clean = None;
            state.set_message(
                "Clean cancelled".to_string(),
                ratatui::style::Color::DarkGray,
            );
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
