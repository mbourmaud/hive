pub(crate) mod cost;
mod drone_actions;
mod keybinds;
mod render;
mod state;
mod tui;
mod views;

use anyhow::Result;

/// Run the monitor TUI dashboard.
pub fn run_monitor(name: Option<String>) -> Result<()> {
    tui::run_tui(name)
}
