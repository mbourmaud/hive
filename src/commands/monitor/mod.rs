mod cost;
mod drone_actions;
mod keybinds;
mod render;
mod simple;
mod sparkline;
mod state;
mod tui;
mod views;

use anyhow::Result;

// ============================================================================
// View Mode
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
enum ViewMode {
    Dashboard,
    Timeline,
}

/// Run the monitor command with auto-refresh TUI by default, simple mode for scripts/CI.
pub fn run_monitor(name: Option<String>, simple: bool) -> Result<()> {
    if simple {
        simple::run_simple(name, false)
    } else {
        tui::run_tui(name)
    }
}

/// Legacy run function for backward compatibility (can be removed later).
pub fn run(name: Option<String>, interactive: bool, follow: bool) -> Result<()> {
    if interactive {
        tui::run_tui(name)
    } else {
        simple::run_simple(name, follow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_mode_enum() {
        assert_ne!(ViewMode::Dashboard, ViewMode::Timeline);
        assert_eq!(ViewMode::Dashboard, ViewMode::Dashboard);
    }
}
