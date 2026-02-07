//! Hive - A CLI tool for orchestrating multiple Claude Code instances via git worktrees.
//!
//! This library provides the core functionality for managing "drones" (Claude Code instances)
//! with a TUI dashboard for monitoring and control.
//!
//! # Modules
//!
//! - `agent_teams`: Agent team orchestration and monitoring
//! - `backend`: Backend execution and process management
//! - `commands`: CLI command implementations (init, start, monitor, etc.)
//! - `config`: Configuration file handling and management
//! - `events`: Event system for drone communication
//! - `mcp`: Model Context Protocol integration
//! - `types`: Shared types and data structures

pub mod agent_teams;
pub mod backend;
pub mod commands;
pub mod config;
pub mod events;
pub mod mcp;
pub mod types;
