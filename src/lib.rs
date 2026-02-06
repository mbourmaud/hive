/// The current version of Hive, sourced from Cargo.toml
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod backend;
pub mod commands;
pub mod communication;
pub mod config;
pub mod mcp;
pub mod notifications;
pub mod types;
