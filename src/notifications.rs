use std::process::Command;

/// Send a desktop notification
/// Falls back to terminal bell if notification systems are unavailable
pub fn notify(title: &str, message: &str) {
    if send_notification(title, message).is_err() {
        // Fallback to terminal bell
        print!("\x07");
    }
}

#[cfg(target_os = "macos")]
fn send_notification(title: &str, message: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Try terminal-notifier first (if installed)
    if Command::new("terminal-notifier")
        .args(["-title", title, "-message", message, "-sound", "Glass"])
        .status()
        .is_ok()
    {
        return Ok(());
    }

    // Fallback to osascript (always available on macOS)
    Command::new("osascript")
        .args([
            "-e",
            &format!(
                "display notification \"{}\" with title \"{}\" sound name \"Glass\"",
                message.replace('"', "\\\""),
                title.replace('"', "\\\"")
            ),
        ])
        .status()?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn send_notification(title: &str, message: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Check if we're in WSL
    if std::path::Path::new("/proc/sys/fs/binfmt_misc/WSLInterop").exists() {
        // WSL - use PowerShell for Windows notifications
        Command::new("powershell.exe")
            .args([
                "-Command",
                &format!(
                    "[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null; \
                     [Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime] | Out-Null; \
                     $template = @\"<toast><visual><binding template='ToastText02'><text id='1'>{}</text><text id='2'>{}</text></binding></visual></toast>\"@; \
                     $xml = New-Object Windows.Data.Xml.Dom.XmlDocument; \
                     $xml.LoadXml($template); \
                     $toast = New-Object Windows.UI.Notifications.ToastNotification $xml; \
                     [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('Hive').Show($toast)",
                    title.replace('"', "`\""),
                    message.replace('"', "`\"")
                ),
            ])
            .status()?;
    } else {
        // Regular Linux - use notify-send
        Command::new("notify-send")
            .args([title, message])
            .status()?;
    }

    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn send_notification(_title: &str, _message: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Unsupported platform
    Err("Notifications not supported on this platform".into())
}
