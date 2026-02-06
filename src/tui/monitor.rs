use std::collections::HashSet;
use std::path::PathBuf;

use crate::types::{DroneState, DroneStatus, Prd};

/// TUI-friendly drone information.
#[derive(Debug, Clone)]
pub struct DroneInfo {
    pub name: String,
    pub status: DroneState,
    pub completed: usize,
    pub total: usize,
    pub stories: Vec<StoryInfo>,
    pub started_at: String,
}

/// TUI-friendly story information.
#[derive(Debug, Clone)]
pub struct StoryInfo {
    pub id: String,
    pub title: String,
    pub status: StoryStatus,
}

/// Story execution state for display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoryStatus {
    Pending,
    InProgress,
    Completed,
    Error,
}

/// Reads .hive/drones/*/status.json and enriches with PRD story data.
pub fn list_drones() -> Vec<DroneInfo> {
    let hive_dir = match find_hive_dir() {
        Some(d) => d,
        None => return Vec::new(),
    };

    let drones_dir = hive_dir.join("drones");
    if !drones_dir.exists() {
        return Vec::new();
    }

    let entries = match std::fs::read_dir(&drones_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut drones = Vec::new();

    for entry in entries.flatten() {
        if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            continue;
        }

        let drone_name = entry.file_name().to_string_lossy().into_owned();
        let status_path = entry.path().join("status.json");

        let contents = match std::fs::read_to_string(&status_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let status: DroneStatus = match serde_json::from_str(&contents) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let prd_path = hive_dir.join("prds").join(&status.prd);
        let prd = load_prd(&prd_path);

        let (completed, total, stories) = match &prd {
            Some(prd) => {
                let prd_ids: HashSet<&str> = prd.stories.iter().map(|s| s.id.as_str()).collect();
                let valid_completed = status
                    .completed
                    .iter()
                    .filter(|id| prd_ids.contains(id.as_str()))
                    .count();
                let stories = build_stories(&status, prd);
                (valid_completed, prd.stories.len(), stories)
            }
            None => (status.completed.len(), status.total, Vec::new()),
        };

        drones.push(DroneInfo {
            name: drone_name,
            status: status.status,
            completed,
            total,
            stories,
            started_at: status.started,
        });
    }

    // Sort: active drones first (by state priority), then by name
    drones.sort_by(|a, b| {
        state_order(&a.status)
            .cmp(&state_order(&b.status))
            .then(a.name.cmp(&b.name))
    });

    drones
}

/// Status icon for drone state.
pub fn state_icon(state: &DroneState) -> (&'static str, ratatui::style::Color) {
    use ratatui::style::Color;
    match state {
        DroneState::Starting | DroneState::Resuming => ("\u{25cf}", Color::Yellow), // ●
        DroneState::InProgress => ("\u{25cf}", Color::Green),                       // ●
        DroneState::Completed => ("\u{2713}", Color::Green),                        // ✓
        DroneState::Error => ("\u{2717}", Color::Red),                              // ✗
        DroneState::Blocked => ("\u{25cc}", Color::Yellow),                         // ◌
        DroneState::Stopped => ("\u{25cb}", Color::DarkGray),                       // ○
    }
}

fn state_order(s: &DroneState) -> u8 {
    match s {
        DroneState::InProgress => 0,
        DroneState::Starting | DroneState::Resuming => 1,
        DroneState::Blocked => 2,
        DroneState::Error => 3,
        DroneState::Completed => 4,
        DroneState::Stopped => 5,
    }
}

fn build_stories(status: &DroneStatus, prd: &Prd) -> Vec<StoryInfo> {
    let completed_set: HashSet<&str> = status.completed.iter().map(|s| s.as_str()).collect();

    prd.stories
        .iter()
        .map(|story| {
            let story_status = if completed_set.contains(story.id.as_str()) {
                StoryStatus::Completed
            } else if status.current_story.as_deref() == Some(&story.id) {
                if status.status == DroneState::Error
                    && status.last_error_story.as_deref() == Some(&story.id)
                {
                    StoryStatus::Error
                } else {
                    StoryStatus::InProgress
                }
            } else {
                StoryStatus::Pending
            };

            StoryInfo {
                id: story.id.clone(),
                title: story.title.clone(),
                status: story_status,
            }
        })
        .collect()
}

fn load_prd(path: &std::path::Path) -> Option<Prd> {
    let contents = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&contents).ok()
}

fn find_hive_dir() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let candidate = dir.join(".hive");
        if candidate.is_dir() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}
