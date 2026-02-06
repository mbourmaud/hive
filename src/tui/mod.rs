// Unified TUI module for Hive
// Provides Claude Code chat interface and drone monitoring dashboard

mod app;
pub mod chat;
mod claude;
pub mod commands;
pub mod dialogs;
pub mod drone_actions;
mod file_picker;
mod input;
mod layout;
pub mod markdown;
pub mod messages;
pub mod monitor;
pub mod permissions;
pub mod session_store;
pub mod sessions;
pub mod sidebar;
mod theme;
mod ui;

pub use app::run_tui;
