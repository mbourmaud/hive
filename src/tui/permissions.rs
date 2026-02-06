use ratatui::style::Style;

use super::dialogs::ModalDialog;
use super::theme::Theme;

#[allow(dead_code)]
pub fn create_permission_dialog(tool_name: &str, args_preview: &str, theme: &Theme) -> ModalDialog {
    let body = format!(
        "Claude wants to use a tool:\n\n\
         Tool: {}\n\n\
         Arguments:\n{}\n",
        tool_name, args_preview
    );

    ModalDialog::new("Permission Request", body)
        .with_option("Accept", 'y', Style::default().fg(theme.success))
        .with_option("Reject", 'n', Style::default().fg(theme.error))
        .with_option("Always Allow", 'a', Style::default().fg(theme.warning))
}
