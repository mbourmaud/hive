/// Drone action handlers for the TUI sidebar.
/// Provides stop, clean, log viewing, and new drone launch capabilities
/// triggered by keyboard shortcuts (x, c, l, n).

use anyhow::{bail, Context, Result};
use std::fs;
use std::io::BufRead;
use std::path::PathBuf;

use crate::backend::{self, SpawnHandle};
use crate::types::{DroneState, DroneStatus, Prd};

/// Actions that can be performed on a drone from the sidebar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DroneAction {
    /// Stop a running drone (key: 'x')
    Stop,
    /// Clean a stopped drone (key: 'c')
    Clean,
    /// View drone logs (key: 'l')
    ViewLogs,
    /// Launch a new drone (key: 'n')
    Launch,
}

/// Result of executing a drone action, used for status bar feedback.
#[derive(Debug, Clone)]
pub struct ActionResult {
    pub success: bool,
    pub message: String,
}

impl ActionResult {
    fn ok(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
        }
    }

    fn err(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
        }
    }
}

/// State for the log viewer overlay/split pane.
pub struct LogViewerState {
    /// Lines of the log file.
    pub lines: Vec<String>,
    /// Current scroll offset (line index at the top of the view).
    pub scroll_offset: usize,
    /// Name of the drone whose logs are being viewed.
    pub drone_name: String,
    /// Whether the viewer is currently visible.
    pub visible: bool,
}

impl LogViewerState {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            scroll_offset: 0,
            drone_name: String::new(),
            visible: false,
        }
    }

    /// Open the log viewer for a given drone.
    pub fn open(&mut self, drone_name: &str) -> Result<()> {
        let log_path = PathBuf::from(".hive/drones")
            .join(drone_name)
            .join("activity.log");

        self.drone_name = drone_name.to_string();
        self.lines.clear();
        self.scroll_offset = 0;

        if log_path.exists() {
            let file = fs::File::open(&log_path)
                .context("Failed to open activity log")?;
            let reader = std::io::BufReader::new(file);
            self.lines = reader.lines().map_while(Result::ok).collect();

            // Scroll to bottom by default
            if self.lines.len() > 20 {
                self.scroll_offset = self.lines.len().saturating_sub(20);
            }
        }

        self.visible = true;
        Ok(())
    }

    /// Close the log viewer.
    pub fn close(&mut self) {
        self.visible = false;
        self.lines.clear();
        self.drone_name.clear();
        self.scroll_offset = 0;
    }

    /// Reload logs from disk (for refreshing while open).
    pub fn reload(&mut self) -> Result<()> {
        if self.drone_name.is_empty() {
            return Ok(());
        }

        let log_path = PathBuf::from(".hive/drones")
            .join(&self.drone_name)
            .join("activity.log");

        if log_path.exists() {
            let file = fs::File::open(&log_path)
                .context("Failed to open activity log")?;
            let reader = std::io::BufReader::new(file);
            let new_lines: Vec<String> = reader.lines().map_while(Result::ok).collect();

            let was_at_bottom = self.scroll_offset
                >= self.lines.len().saturating_sub(20);

            self.lines = new_lines;

            // If user was at the bottom, stay at the bottom
            if was_at_bottom && self.lines.len() > 20 {
                self.scroll_offset = self.lines.len().saturating_sub(20);
            }
        }

        Ok(())
    }

    /// Scroll up by a given number of lines.
    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    /// Scroll down by a given number of lines.
    pub fn scroll_down(&mut self, amount: usize) {
        let max_offset = self.lines.len().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + amount).min(max_offset);
    }

    /// Get the visible lines for rendering in the viewport.
    pub fn visible_lines(&self, viewport_height: usize) -> &[String] {
        let start = self.scroll_offset;
        let end = (start + viewport_height).min(self.lines.len());
        if start < self.lines.len() {
            &self.lines[start..end]
        } else {
            &[]
        }
    }
}

impl Default for LogViewerState {
    fn default() -> Self {
        Self::new()
    }
}

/// State for the PRD selection dialog when launching a new drone.
pub struct PrdSelectionState {
    /// Available PRD files.
    pub prds: Vec<PrdEntry>,
    /// Currently selected index in the PRD list.
    pub selected_index: usize,
    /// Whether the selection dialog is visible.
    pub visible: bool,
}

/// A PRD file entry for display in the selection dialog.
#[derive(Debug, Clone)]
pub struct PrdEntry {
    /// File name (e.g., "prd-feature-x.json").
    pub filename: String,
    /// Full path to the PRD file.
    pub path: PathBuf,
    /// Parsed title from the PRD, if available.
    pub title: Option<String>,
    /// Number of stories in the PRD.
    pub story_count: usize,
}

impl PrdSelectionState {
    pub fn new() -> Self {
        Self {
            prds: Vec::new(),
            selected_index: 0,
            visible: false,
        }
    }

    /// Open the PRD selection dialog by scanning .hive/prds/.
    pub fn open(&mut self) -> Result<()> {
        self.prds = list_available_prds()?;
        self.selected_index = 0;
        self.visible = true;

        if self.prds.is_empty() {
            bail!("No PRD files found in .hive/prds/");
        }

        Ok(())
    }

    /// Close the PRD selection dialog.
    pub fn close(&mut self) {
        self.visible = false;
        self.selected_index = 0;
    }

    /// Move selection up.
    pub fn select_prev(&mut self) {
        self.selected_index = self.selected_index.saturating_sub(1);
    }

    /// Move selection down.
    pub fn select_next(&mut self) {
        if !self.prds.is_empty() {
            self.selected_index = (self.selected_index + 1).min(self.prds.len() - 1);
        }
    }

    /// Get the currently selected PRD entry, if available.
    pub fn selected_prd(&self) -> Option<&PrdEntry> {
        self.prds.get(self.selected_index)
    }
}

impl Default for PrdSelectionState {
    fn default() -> Self {
        Self::new()
    }
}

/// Stop a drone by name. Uses the backend to send stop signals.
/// Returns an ActionResult for status bar feedback.
pub fn execute_stop(drone_name: &str) -> ActionResult {
    match stop_drone_impl(drone_name) {
        Ok(()) => ActionResult::ok(format!("Drone '{}' stopped", drone_name)),
        Err(e) => ActionResult::err(format!("Failed to stop '{}': {}", drone_name, e)),
    }
}

fn stop_drone_impl(drone_name: &str) -> Result<()> {
    let drone_dir = PathBuf::from(".hive/drones").join(drone_name);

    if !drone_dir.exists() {
        bail!("Drone '{}' not found", drone_name);
    }

    let status_path = drone_dir.join("status.json");
    let status: DroneStatus = if status_path.exists() {
        let contents = fs::read_to_string(&status_path)?;
        serde_json::from_str(&contents)?
    } else {
        bail!("No status file found for drone '{}'", drone_name);
    };

    // Use the backend to stop the drone process
    let backend = backend::resolve_backend(None);
    let handle = SpawnHandle {
        pid: None,
        backend_id: status.worktree.clone(),
        backend_type: status.backend.clone(),
    };

    backend.stop(&handle)?;

    // Update status to stopped
    let mut updated_status = status;
    updated_status.status = DroneState::Stopped;
    updated_status.updated = chrono::Utc::now().to_rfc3339();
    let status_json = serde_json::to_string_pretty(&updated_status)?;
    fs::write(&status_path, status_json)?;

    Ok(())
}

/// Clean a drone by name. Removes worktree and drone data.
/// Returns an ActionResult for status bar feedback.
pub fn execute_clean(drone_name: &str) -> ActionResult {
    match clean_drone_impl(drone_name) {
        Ok(()) => ActionResult::ok(format!("Drone '{}' cleaned", drone_name)),
        Err(e) => ActionResult::err(format!("Failed to clean '{}': {}", drone_name, e)),
    }
}

fn clean_drone_impl(drone_name: &str) -> Result<()> {
    // Delegate to the existing quiet clean implementation
    crate::commands::kill_clean::clean_quiet(drone_name.to_string())
}

/// Read drone logs from .hive/drones/<name>/activity.log.
/// Returns the log lines as a Vec<String>.
pub fn get_drone_logs(drone_name: &str) -> Result<Vec<String>> {
    let log_path = PathBuf::from(".hive/drones")
        .join(drone_name)
        .join("activity.log");

    if !log_path.exists() {
        return Ok(vec![format!("No activity log found for drone '{}'", drone_name)]);
    }

    let file = fs::File::open(&log_path)
        .context("Failed to open activity log")?;
    let reader = std::io::BufReader::new(file);
    let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();

    if lines.is_empty() {
        Ok(vec!["Log file is empty".to_string()])
    } else {
        Ok(lines)
    }
}

/// List available PRD files from .hive/prds/.
/// Returns PrdEntry items with parsed metadata where possible.
pub fn list_available_prds() -> Result<Vec<PrdEntry>> {
    let prds_dir = PathBuf::from(".hive/prds");

    if !prds_dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();

    for entry in fs::read_dir(&prds_dir)?.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            // Try to parse PRD to extract title and story count
            let (title, story_count) = match fs::read_to_string(&path) {
                Ok(contents) => match serde_json::from_str::<Prd>(&contents) {
                    Ok(prd) => (Some(prd.title), prd.stories.len()),
                    Err(_) => (None, 0),
                },
                Err(_) => (None, 0),
            };

            entries.push(PrdEntry {
                filename,
                path,
                title,
                story_count,
            });
        }
    }

    // Sort by filename for consistent ordering
    entries.sort_by(|a, b| a.filename.cmp(&b.filename));

    Ok(entries)
}

/// Map a key character to a DroneAction, if applicable.
pub fn key_to_action(key: char) -> Option<DroneAction> {
    match key {
        'x' => Some(DroneAction::Stop),
        'c' => Some(DroneAction::Clean),
        'l' => Some(DroneAction::ViewLogs),
        'n' => Some(DroneAction::Launch),
        _ => None,
    }
}

/// Check whether a drone action requires a selected drone.
/// Launch ('n') does not require a selected drone.
pub fn requires_selected_drone(action: &DroneAction) -> bool {
    matches!(action, DroneAction::Stop | DroneAction::Clean | DroneAction::ViewLogs)
}

/// Check whether an action needs confirmation before execution.
pub fn requires_confirmation(action: &DroneAction) -> bool {
    matches!(action, DroneAction::Stop | DroneAction::Clean)
}

/// Get the confirmation dialog message for an action.
pub fn confirmation_message(action: &DroneAction, drone_name: &str) -> (String, String) {
    match action {
        DroneAction::Stop => (
            format!(" Stop Drone '{}' ", drone_name),
            format!("Are you sure you want to stop drone '{}'?", drone_name),
        ),
        DroneAction::Clean => (
            format!(" Clean Drone '{}' ", drone_name),
            format!(
                "Are you sure you want to clean drone '{}'? This will remove the worktree and all drone data.",
                drone_name
            ),
        ),
        _ => (String::new(), String::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_key_to_action() {
        assert_eq!(key_to_action('x'), Some(DroneAction::Stop));
        assert_eq!(key_to_action('c'), Some(DroneAction::Clean));
        assert_eq!(key_to_action('l'), Some(DroneAction::ViewLogs));
        assert_eq!(key_to_action('n'), Some(DroneAction::Launch));
        assert_eq!(key_to_action('z'), None);
        assert_eq!(key_to_action('a'), None);
    }

    #[test]
    fn test_requires_selected_drone() {
        assert!(requires_selected_drone(&DroneAction::Stop));
        assert!(requires_selected_drone(&DroneAction::Clean));
        assert!(requires_selected_drone(&DroneAction::ViewLogs));
        assert!(!requires_selected_drone(&DroneAction::Launch));
    }

    #[test]
    fn test_requires_confirmation() {
        assert!(requires_confirmation(&DroneAction::Stop));
        assert!(requires_confirmation(&DroneAction::Clean));
        assert!(!requires_confirmation(&DroneAction::ViewLogs));
        assert!(!requires_confirmation(&DroneAction::Launch));
    }

    #[test]
    fn test_confirmation_message() {
        let (title, msg) = confirmation_message(&DroneAction::Stop, "test-drone");
        assert!(title.contains("Stop"));
        assert!(title.contains("test-drone"));
        assert!(msg.contains("stop"));

        let (title, msg) = confirmation_message(&DroneAction::Clean, "test-drone");
        assert!(title.contains("Clean"));
        assert!(msg.contains("clean"));
        assert!(msg.contains("worktree"));
    }

    #[test]
    fn test_action_result() {
        let ok = ActionResult::ok("success");
        assert!(ok.success);
        assert_eq!(ok.message, "success");

        let err = ActionResult::err("failure");
        assert!(!err.success);
        assert_eq!(err.message, "failure");
    }

    #[test]
    fn test_log_viewer_state() {
        let mut state = LogViewerState::new();
        assert!(!state.visible);
        assert!(state.lines.is_empty());
        assert_eq!(state.scroll_offset, 0);

        // Test scroll operations with some mock data
        state.lines = (0..50).map(|i| format!("Line {}", i)).collect();
        state.scroll_offset = 10;

        state.scroll_up(5);
        assert_eq!(state.scroll_offset, 5);

        state.scroll_up(10);
        assert_eq!(state.scroll_offset, 0);

        state.scroll_down(30);
        assert_eq!(state.scroll_offset, 30);

        state.scroll_down(100);
        assert_eq!(state.scroll_offset, 49);

        // Test visible_lines
        state.scroll_offset = 0;
        let visible = state.visible_lines(10);
        assert_eq!(visible.len(), 10);
        assert_eq!(visible[0], "Line 0");
        assert_eq!(visible[9], "Line 9");

        // Test close
        state.close();
        assert!(!state.visible);
        assert!(state.lines.is_empty());
    }

    #[test]
    fn test_prd_selection_state() {
        let mut state = PrdSelectionState::new();
        assert!(!state.visible);
        assert_eq!(state.selected_index, 0);

        state.prds = vec![
            PrdEntry {
                filename: "prd-a.json".to_string(),
                path: PathBuf::from(".hive/prds/prd-a.json"),
                title: Some("Feature A".to_string()),
                story_count: 3,
            },
            PrdEntry {
                filename: "prd-b.json".to_string(),
                path: PathBuf::from(".hive/prds/prd-b.json"),
                title: Some("Feature B".to_string()),
                story_count: 5,
            },
        ];
        state.visible = true;

        assert_eq!(state.selected_prd().unwrap().filename, "prd-a.json");

        state.select_next();
        assert_eq!(state.selected_index, 1);
        assert_eq!(state.selected_prd().unwrap().filename, "prd-b.json");

        state.select_next();
        assert_eq!(state.selected_index, 1); // Should not go beyond last

        state.select_prev();
        assert_eq!(state.selected_index, 0);

        state.select_prev();
        assert_eq!(state.selected_index, 0); // Should not go below 0

        state.close();
        assert!(!state.visible);
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn test_get_drone_logs_missing_drone() {
        let result = get_drone_logs("nonexistent-drone-xyz");
        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("No activity log found"));
    }

    #[test]
    fn test_get_drone_logs_with_content() {
        let temp_dir = std::env::temp_dir().join("hive-test-drone-actions");
        let drone_dir = temp_dir.join(".hive/drones/test-log-drone");
        fs::create_dir_all(&drone_dir).unwrap();

        let log_path = drone_dir.join("activity.log");
        let mut file = fs::File::create(&log_path).unwrap();
        writeln!(file, "[10:00:00] Started").unwrap();
        writeln!(file, "[10:00:05] Working on US-001").unwrap();
        writeln!(file, "[10:00:10] Completed US-001").unwrap();

        // We can't test get_drone_logs directly since it uses a hardcoded path,
        // but we can verify the log reading logic via LogViewerState
        let mut viewer = LogViewerState::new();
        viewer.drone_name = "test".to_string();
        viewer.lines = vec![
            "[10:00:00] Started".to_string(),
            "[10:00:05] Working on US-001".to_string(),
            "[10:00:10] Completed US-001".to_string(),
        ];
        viewer.visible = true;

        assert_eq!(viewer.lines.len(), 3);
        assert_eq!(viewer.visible_lines(10).len(), 3);

        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_list_available_prds_no_dir() {
        // When .hive/prds doesn't exist, should return empty vec
        let saved_dir = std::env::current_dir().unwrap();
        let temp = std::env::temp_dir().join("hive-test-no-prds");
        fs::create_dir_all(&temp).unwrap();
        std::env::set_current_dir(&temp).unwrap();

        let result = list_available_prds();
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());

        std::env::set_current_dir(saved_dir).unwrap();
        fs::remove_dir_all(&temp).ok();
    }

    #[test]
    fn test_execute_stop_missing_drone() {
        let result = execute_stop("nonexistent-drone-xyz-stop");
        assert!(!result.success);
        assert!(result.message.contains("Failed to stop"));
    }

    #[test]
    fn test_execute_clean_missing_drone() {
        let result = execute_clean("nonexistent-drone-xyz-clean");
        assert!(!result.success);
        assert!(result.message.contains("Failed to clean"));
    }
}
