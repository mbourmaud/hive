pub(crate) mod cost;
mod drone_actions;
mod keybinds;
mod render;
mod state;
mod tui;
mod views;

use anyhow::Result;

/// Run the monitor TUI dashboard, or web-only mode with `--web`.
pub fn run_monitor(name: Option<String>, web: bool) -> Result<()> {
    if web {
        tui::run_web_only()
    } else {
        tui::run_tui(name)
    }
}
