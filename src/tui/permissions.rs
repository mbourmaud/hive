use std::collections::HashSet;

use super::dialogs::{DialogAction, PermissionDialog};

pub struct PermissionManager {
    always_allowed: HashSet<String>,
    pub active_dialog: Option<PermissionDialog>,
}

impl PermissionManager {
    pub fn new() -> Self {
        Self {
            always_allowed: HashSet::new(),
            active_dialog: None,
        }
    }

    /// Check if a tool needs approval. Returns true if dialog should be shown.
    pub fn needs_approval(&self, tool_name: &str) -> bool {
        !self.always_allowed.contains(tool_name)
    }

    /// Show approval dialog for a tool
    pub fn request_approval(&mut self, tool_name: String, args_summary: String) {
        if self.always_allowed.contains(&tool_name) {
            return; // Auto-approved
        }
        self.active_dialog = Some(PermissionDialog::new(tool_name, args_summary));
    }

    /// Handle dialog result. Returns true if approved, false if rejected.
    pub fn handle_action(&mut self, action: DialogAction) -> bool {
        let approved = match &action {
            DialogAction::Accept => true,
            DialogAction::Reject => false,
            DialogAction::AlwaysAllow => {
                if let Some(ref dialog) = self.active_dialog {
                    self.always_allowed.insert(dialog.tool_name.clone());
                }
                true
            }
        };
        self.active_dialog = None;
        approved
    }

    pub fn has_active_dialog(&self) -> bool {
        self.active_dialog.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_manager_needs_approval_for_all() {
        let manager = PermissionManager::new();
        assert!(manager.needs_approval("Read"));
        assert!(manager.needs_approval("Write"));
        assert!(manager.needs_approval("Bash"));
    }

    #[test]
    fn test_no_active_dialog_initially() {
        let manager = PermissionManager::new();
        assert!(!manager.has_active_dialog());
    }

    #[test]
    fn test_request_approval_creates_dialog() {
        let mut manager = PermissionManager::new();
        manager.request_approval("Read".to_string(), "/tmp/file.txt".to_string());
        assert!(manager.has_active_dialog());
    }

    #[test]
    fn test_accept_clears_dialog() {
        let mut manager = PermissionManager::new();
        manager.request_approval("Read".to_string(), "args".to_string());
        let approved = manager.handle_action(DialogAction::Accept);
        assert!(approved);
        assert!(!manager.has_active_dialog());
        // Accept does NOT add to always_allowed
        assert!(manager.needs_approval("Read"));
    }

    #[test]
    fn test_reject_clears_dialog() {
        let mut manager = PermissionManager::new();
        manager.request_approval("Read".to_string(), "args".to_string());
        let approved = manager.handle_action(DialogAction::Reject);
        assert!(!approved);
        assert!(!manager.has_active_dialog());
    }

    #[test]
    fn test_always_allow_adds_to_set() {
        let mut manager = PermissionManager::new();
        manager.request_approval("Read".to_string(), "args".to_string());
        let approved = manager.handle_action(DialogAction::AlwaysAllow);
        assert!(approved);
        assert!(!manager.has_active_dialog());
        assert!(!manager.needs_approval("Read"));
    }

    #[test]
    fn test_always_allowed_tool_skips_dialog() {
        let mut manager = PermissionManager::new();
        // First, always-allow Read
        manager.request_approval("Read".to_string(), "args".to_string());
        manager.handle_action(DialogAction::AlwaysAllow);

        // Now request approval again - should not create dialog
        manager.request_approval("Read".to_string(), "other args".to_string());
        assert!(!manager.has_active_dialog());
    }
}
