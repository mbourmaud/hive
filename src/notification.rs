use std::process::Command as ProcessCommand;

/// Send a desktop notification with Hive branding.
///
/// Tries terminal-notifier first (supports icons), falls back to
/// osascript (macOS) or notify-send (Linux).
pub fn notify(title: &str, message: &str) {
    let branded_title = format!("\u{1f41d} {}", title);

    // Try terminal-notifier first (macOS, supports icons and richer UX)
    let tn_result = ProcessCommand::new("terminal-notifier")
        .args([
            "-title",
            &branded_title,
            "-message",
            message,
            "-sound",
            "Glass",
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output();

    if tn_result
        .as_ref()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return;
    }

    // Fallback to platform-native notification
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "display notification \"{}\" with title \"{}\" sound name \"Glass\"",
            message.replace('\"', "\\\"").replace('\n', " "),
            branded_title.replace('\"', "\\\""),
        );
        let _ = ProcessCommand::new("osascript")
            .arg("-e")
            .arg(&script)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }

    #[cfg(target_os = "linux")]
    {
        let _ = ProcessCommand::new("notify-send")
            .args([&branded_title, message])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }
}
