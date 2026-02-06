use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use serde::{Deserialize, Serialize};

use super::dialogs::ModalDialog;
use super::theme::Theme;

/// Tool approval request from Claude
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolApprovalRequest {
    pub id: String,
    pub tool_name: String,
    pub args: serde_json::Value,
    pub file_diff: Option<String>,
}

/// User response to tool approval request
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalResponse {
    Accept,
    Reject,
    AlwaysAllow,
}

/// Permission dialog state
pub struct PermissionDialogState {
    pub request: Option<ToolApprovalRequest>,
    pub response: Option<ApprovalResponse>,
}

impl PermissionDialogState {
    pub fn new() -> Self {
        Self {
            request: None,
            response: None,
        }
    }

    /// Set a new approval request
    pub fn set_request(&mut self, request: ToolApprovalRequest) {
        self.request = Some(request);
        self.response = None;
    }

    /// Clear the current request
    pub fn clear(&mut self) {
        self.request = None;
        self.response = None;
    }

    /// Check if a dialog is active
    pub fn is_active(&self) -> bool {
        self.request.is_some()
    }

    /// Handle key input for the dialog
    pub fn handle_key(&mut self, key: char) -> Option<ApprovalResponse> {
        match key {
            'y' | 'Y' => {
                self.response = Some(ApprovalResponse::Accept);
                Some(ApprovalResponse::Accept)
            }
            'n' | 'N' => {
                self.response = Some(ApprovalResponse::Reject);
                Some(ApprovalResponse::Reject)
            }
            'a' | 'A' => {
                self.response = Some(ApprovalResponse::AlwaysAllow);
                Some(ApprovalResponse::AlwaysAllow)
            }
            _ => None,
        }
    }

    /// Build the permission dialog widget using theme colors
    pub fn build_dialog(&self, theme: &Theme) -> Option<ModalDialog<'static>> {
        let request = self.request.as_ref()?;

        let mut content = Vec::new();

        // Tool name
        content.push(Line::from(vec![
            Span::styled("Tool: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                request.tool_name.clone(),
                Style::default()
                    .fg(theme.accent_warning)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        content.push(Line::from(""));

        // Arguments
        content.push(Line::from(Span::styled(
            "Arguments:",
            Style::default().add_modifier(Modifier::BOLD),
        )));

        let args_str =
            serde_json::to_string_pretty(&request.args).unwrap_or_else(|_| "{}".to_string());

        for line in args_str.lines().take(10) {
            content.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(theme.accent_primary),
            )));
        }

        // Show truncation indicator if there are more lines
        if args_str.lines().count() > 10 {
            content.push(Line::from(Span::styled(
                "  ... (truncated)",
                Style::default().fg(theme.fg_muted),
            )));
        }

        content.push(Line::from(""));

        // File diff preview if available
        if let Some(ref diff) = request.file_diff {
            content.push(Line::from(Span::styled(
                "File Changes:",
                Style::default().add_modifier(Modifier::BOLD),
            )));

            for line in diff.lines().take(5) {
                let style = if line.starts_with('+') {
                    Style::default().fg(theme.accent_success)
                } else if line.starts_with('-') {
                    Style::default().fg(theme.accent_error)
                } else {
                    Style::default().fg(theme.fg_secondary)
                };
                content.push(Line::from(Span::styled(format!("  {}", line), style)));
            }

            if diff.lines().count() > 5 {
                content.push(Line::from(Span::styled(
                    "  ... (truncated)",
                    Style::default().fg(theme.fg_muted),
                )));
            }

            content.push(Line::from(""));
        }

        // Footer with options
        let footer = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "[Y]",
                    Style::default()
                        .fg(theme.accent_success)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" Accept  "),
                Span::styled(
                    "[N]",
                    Style::default()
                        .fg(theme.accent_error)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" Reject  "),
                Span::styled(
                    "[A]",
                    Style::default()
                        .fg(theme.accent_warning)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" Always Allow"),
            ]),
        ];

        Some(
            ModalDialog::new(" Tool Approval Required ", theme)
                .content(content)
                .footer(footer)
                .width_percent(70)
                .height_percent(60),
        )
    }
}

impl Default for PermissionDialogState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approval_response_keys() {
        let mut state = PermissionDialogState::new();
        state.set_request(ToolApprovalRequest {
            id: "test-1".to_string(),
            tool_name: "Read".to_string(),
            args: serde_json::json!({"file_path": "test.rs"}),
            file_diff: None,
        });

        assert_eq!(state.handle_key('y'), Some(ApprovalResponse::Accept));

        state.response = None;
        assert_eq!(state.handle_key('n'), Some(ApprovalResponse::Reject));

        state.response = None;
        assert_eq!(state.handle_key('a'), Some(ApprovalResponse::AlwaysAllow));

        state.response = None;
        assert_eq!(state.handle_key('x'), None);
    }

    #[test]
    fn test_dialog_state() {
        let mut state = PermissionDialogState::new();
        assert!(!state.is_active());

        state.set_request(ToolApprovalRequest {
            id: "test-1".to_string(),
            tool_name: "Edit".to_string(),
            args: serde_json::json!({}),
            file_diff: None,
        });

        assert!(state.is_active());
        let theme = Theme::dark();
        assert!(state.build_dialog(&theme).is_some());

        state.clear();
        assert!(!state.is_active());
        assert!(state.build_dialog(&theme).is_none());
    }
}
