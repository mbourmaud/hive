use anyhow::Result;
use std::fs;
use std::path::PathBuf;

/// Stop a drone by name (quiet mode for TUI use)
pub fn stop_drone(name: &str) -> Result<String> {
    match crate::commands::kill_clean::kill_quiet(name.to_string()) {
        Ok(_) => Ok(format!("Drone '{}' stopped", name)),
        Err(e) => Ok(format!("Failed to stop '{}': {}", name, e)),
    }
}

/// Clean a drone by name (quiet mode for TUI use)
pub fn clean_drone(name: &str) -> Result<String> {
    match crate::commands::kill_clean::clean_quiet(name.to_string()) {
        Ok(_) => Ok(format!("Drone '{}' cleaned", name)),
        Err(e) => Ok(format!("Failed to clean '{}': {}", name, e)),
    }
}

/// Read drone activity log (last N lines)
pub fn read_drone_logs(name: &str, max_lines: usize) -> Result<Vec<String>> {
    let log_path = PathBuf::from(".hive/drones")
        .join(name)
        .join("activity.log");

    if !log_path.exists() {
        return Ok(vec![format!("No activity log found for '{}'", name)]);
    }

    let content = fs::read_to_string(&log_path)?;
    let lines: Vec<String> = content.lines().rev().take(max_lines).map(String::from).collect();
    let mut lines = lines;
    lines.reverse();
    Ok(lines)
}

/// List available PRD files
pub fn list_prds() -> Result<Vec<String>> {
    let prds_dir = PathBuf::from(".hive/prds");
    let mut prds = Vec::new();

    if prds_dir.exists() {
        for entry in fs::read_dir(&prds_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "json") {
                if let Some(stem) = path.file_stem() {
                    prds.push(stem.to_string_lossy().to_string());
                }
            }
        }
    }

    prds.sort();
    Ok(prds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_prds() {
        // This just shouldn't panic, even if .hive/prds doesn't exist
        let result = list_prds();
        assert!(result.is_ok());
    }

    #[test]
    fn test_read_nonexistent_logs() {
        let result = read_drone_logs("nonexistent_drone_xyz", 50);
        assert!(result.is_ok());
        let lines = result.unwrap();
        assert!(lines[0].contains("No activity log"));
    }
}
