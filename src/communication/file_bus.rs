use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use super::{DroneMessage, MessageBus, MessageTarget};

/// File-based message bus implementation.
///
/// Messages are stored as JSON files:
/// - Inbox: `.hive/drones/<name>/inbox/<message-id>.json`
/// - Outbox: `.hive/drones/<name>/outbox/<message-id>.json`
pub struct FileBus {
    hive_dir: PathBuf,
}

impl FileBus {
    pub fn new() -> Self {
        Self {
            hive_dir: PathBuf::from(".hive"),
        }
    }

    fn inbox_dir(&self, drone_name: &str) -> PathBuf {
        self.hive_dir.join("drones").join(drone_name).join("inbox")
    }

    fn outbox_dir(&self, drone_name: &str) -> PathBuf {
        self.hive_dir.join("drones").join(drone_name).join("outbox")
    }

    /// Get all active drone names by reading the drones directory.
    fn active_drones(&self) -> Result<Vec<String>> {
        let drones_dir = self.hive_dir.join("drones");
        if !drones_dir.exists() {
            return Ok(Vec::new());
        }

        let mut names = Vec::new();
        for entry in fs::read_dir(&drones_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                names.push(name);
            }
        }
        Ok(names)
    }

    /// Read all message files from a directory.
    fn read_messages_from_dir(&self, dir: &PathBuf) -> Result<Vec<DroneMessage>> {
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut messages = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let contents = fs::read_to_string(&path)
                    .with_context(|| format!("Failed to read message: {}", path.display()))?;
                if let Ok(msg) = serde_json::from_str::<DroneMessage>(&contents) {
                    messages.push(msg);
                }
            }
        }

        // Sort by timestamp
        messages.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        Ok(messages)
    }
}

impl Default for FileBus {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageBus for FileBus {
    fn send(&self, message: &DroneMessage) -> Result<()> {
        // Determine recipient drone name
        let recipient = match &message.to {
            MessageTarget::Drone(name) => name.clone(),
            MessageTarget::Orchestrator => "_orchestrator".to_string(),
            MessageTarget::AllDrones => {
                // For broadcast, use the broadcast method instead
                return self.broadcast(message);
            }
        };

        // Write to recipient's inbox
        let inbox = self.inbox_dir(&recipient);
        fs::create_dir_all(&inbox)?;
        let msg_path = inbox.join(format!("{}.json", message.id));
        let json = serde_json::to_string_pretty(message)?;
        fs::write(&msg_path, &json)?;

        // Write to sender's outbox
        let outbox = self.outbox_dir(&message.from);
        fs::create_dir_all(&outbox)?;
        let outbox_path = outbox.join(format!("{}.json", message.id));
        fs::write(&outbox_path, &json)?;

        Ok(())
    }

    fn receive(&self, drone_name: &str) -> Result<Vec<DroneMessage>> {
        let inbox = self.inbox_dir(drone_name);
        let messages = self.read_messages_from_dir(&inbox)?;

        // Delete consumed messages
        if inbox.exists() {
            for entry in fs::read_dir(&inbox)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    let _ = fs::remove_file(&path);
                }
            }
        }

        Ok(messages)
    }

    fn peek(&self, drone_name: &str) -> Result<Vec<DroneMessage>> {
        let inbox = self.inbox_dir(drone_name);
        self.read_messages_from_dir(&inbox)
    }

    fn broadcast(&self, message: &DroneMessage) -> Result<()> {
        let drones = self.active_drones()?;
        for drone in &drones {
            // Don't send to self
            if drone == &message.from {
                continue;
            }

            let inbox = self.inbox_dir(drone);
            fs::create_dir_all(&inbox)?;
            let msg_path = inbox.join(format!("{}.json", message.id));
            let json = serde_json::to_string_pretty(message)?;
            fs::write(&msg_path, json)?;
        }

        // Write to sender's outbox
        let outbox = self.outbox_dir(&message.from);
        fs::create_dir_all(&outbox)?;
        let outbox_path = outbox.join(format!("{}.json", message.id));
        let json = serde_json::to_string_pretty(message)?;
        fs::write(&outbox_path, json)?;

        Ok(())
    }

    fn list_outbox(&self, drone_name: &str) -> Result<Vec<DroneMessage>> {
        let outbox = self.outbox_dir(drone_name);
        self.read_messages_from_dir(&outbox)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::communication::MessageType;
    use tempfile::TempDir;

    fn create_test_bus(tmp: &TempDir) -> FileBus {
        let hive_dir = tmp.path().join(".hive");
        fs::create_dir_all(hive_dir.join("drones/drone-a/inbox")).unwrap();
        fs::create_dir_all(hive_dir.join("drones/drone-a/outbox")).unwrap();
        fs::create_dir_all(hive_dir.join("drones/drone-b/inbox")).unwrap();
        fs::create_dir_all(hive_dir.join("drones/drone-b/outbox")).unwrap();

        FileBus { hive_dir }
    }

    fn make_message(from: &str, to: &str) -> DroneMessage {
        DroneMessage {
            id: format!(
                "msg-{}-{}",
                from,
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
            ),
            from: from.to_string(),
            to: MessageTarget::Drone(to.to_string()),
            timestamp: chrono::Utc::now().to_rfc3339(),
            message_type: MessageType::ContextShare,
            payload: serde_json::json!({"info": "test context"}),
        }
    }

    #[test]
    fn test_send_and_receive() {
        let tmp = TempDir::new().unwrap();
        let bus = create_test_bus(&tmp);

        let msg = make_message("drone-a", "drone-b");
        bus.send(&msg).unwrap();

        // Peek should show message without consuming
        let peeked = bus.peek("drone-b").unwrap();
        assert_eq!(peeked.len(), 1);
        assert_eq!(peeked[0].from, "drone-a");

        // Receive should consume
        let received = bus.receive("drone-b").unwrap();
        assert_eq!(received.len(), 1);
        assert_eq!(received[0].from, "drone-a");

        // After receive, inbox should be empty
        let empty = bus.peek("drone-b").unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn test_broadcast() {
        let tmp = TempDir::new().unwrap();
        let bus = create_test_bus(&tmp);

        let msg = DroneMessage {
            id: "broadcast-1".to_string(),
            from: "drone-a".to_string(),
            to: MessageTarget::AllDrones,
            timestamp: chrono::Utc::now().to_rfc3339(),
            message_type: MessageType::CompletionNotification,
            payload: serde_json::json!({"story": "US-001"}),
        };

        bus.broadcast(&msg).unwrap();

        // drone-b should have the message
        let b_msgs = bus.peek("drone-b").unwrap();
        assert_eq!(b_msgs.len(), 1);

        // drone-a should NOT have the message (don't send to self)
        let a_msgs = bus.peek("drone-a").unwrap();
        assert!(a_msgs.is_empty());

        // drone-a's outbox should have the message
        let outbox = bus.list_outbox("drone-a").unwrap();
        assert_eq!(outbox.len(), 1);
    }

    #[test]
    fn test_outbox_tracking() {
        let tmp = TempDir::new().unwrap();
        let bus = create_test_bus(&tmp);

        let msg = make_message("drone-a", "drone-b");
        bus.send(&msg).unwrap();

        let outbox = bus.list_outbox("drone-a").unwrap();
        assert_eq!(outbox.len(), 1);
        assert_eq!(outbox[0].from, "drone-a");
    }
}
