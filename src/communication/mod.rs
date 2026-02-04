pub mod file_bus;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Target for a drone message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageTarget {
    /// Send to a specific drone by name.
    Drone(String),
    /// Broadcast to all active drones.
    AllDrones,
    /// Send to the orchestrator (Hive itself).
    Orchestrator,
}

/// A message exchanged between drones.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DroneMessage {
    pub id: String,
    pub from: String,
    pub to: MessageTarget,
    pub timestamp: String,
    pub message_type: MessageType,
    pub payload: serde_json::Value,
}

/// Types of inter-drone messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    /// Drone reports a status change.
    StatusUpdate,
    /// Drone is blocked and needs help.
    BlockedNotification,
    /// Drone has completed all stories.
    CompletionNotification,
    /// Drone requests another drone to complete a dependency first.
    DependencyRequest,
    /// Drone shares discovered context (e.g., API patterns, config).
    ContextShare,
    /// Drone needs human intervention.
    HumanEscalation,
}

/// Trait for message bus implementations.
pub trait MessageBus: Send + Sync {
    /// Send a message to a specific drone's inbox.
    fn send(&self, message: &DroneMessage) -> Result<()>;

    /// Receive (and consume) all messages from a drone's inbox.
    fn receive(&self, drone_name: &str) -> Result<Vec<DroneMessage>>;

    /// Peek at messages without consuming them.
    fn peek(&self, drone_name: &str) -> Result<Vec<DroneMessage>>;

    /// Broadcast a message to all active drones.
    fn broadcast(&self, message: &DroneMessage) -> Result<()>;

    /// List messages in a drone's outbox (sent messages).
    fn list_outbox(&self, drone_name: &str) -> Result<Vec<DroneMessage>>;
}
