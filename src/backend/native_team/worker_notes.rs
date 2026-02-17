use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

const MAX_NOTES_IN_PROMPT: usize = 5;
const MAX_SUMMARY_LEN: usize = 500;
const NOTES_FILE: &str = "worker-notes.json";

/// A note left by a worker about its completed task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerNote {
    pub task_number: usize,
    pub task_title: String,
    pub files_changed: Vec<String>,
    pub summary: String,
}

/// Append a note to the drone's worker-notes.json file.
pub fn append_note(drone_dir: &Path, note: &WorkerNote) -> Result<()> {
    let path = drone_dir.join(NOTES_FILE);
    let mut notes = read_all_notes(drone_dir);
    notes.push(note.clone());
    let json = serde_json::to_string_pretty(&notes)?;
    std::fs::write(&path, json)?;
    Ok(())
}

/// Read all worker notes from the drone directory.
fn read_all_notes(drone_dir: &Path) -> Vec<WorkerNote> {
    let path = drone_dir.join(NOTES_FILE);
    let contents = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    serde_json::from_str(&contents).unwrap_or_default()
}

/// Read notes for tasks that this task depends on.
pub fn read_dependency_notes(drone_dir: &Path, depends_on: &[usize]) -> Vec<WorkerNote> {
    if depends_on.is_empty() {
        return Vec::new();
    }
    let all = read_all_notes(drone_dir);
    all.into_iter()
        .filter(|note| depends_on.contains(&note.task_number))
        .take(MAX_NOTES_IN_PROMPT)
        .collect()
}

/// Detect files changed in the working directory using git.
pub async fn detect_files_changed(cwd: &Path) -> Vec<String> {
    let output = tokio::process::Command::new("git")
        .args(["diff", "--name-only", "HEAD"])
        .current_dir(cwd)
        .output()
        .await;

    match output {
        Ok(o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            stdout
                .lines()
                .filter(|l| !l.is_empty())
                .map(String::from)
                .collect()
        }
        _ => Vec::new(),
    }
}

/// Format worker notes for inclusion in a system prompt.
pub fn format_notes_for_prompt(notes: &[WorkerNote]) -> String {
    if notes.is_empty() {
        return String::new();
    }

    let mut out = String::from("\n## Notes from Completed Dependencies\n");

    for note in notes.iter().take(MAX_NOTES_IN_PROMPT) {
        out.push_str(&format!(
            "\n**Task {}: {}**\n",
            note.task_number, note.task_title
        ));
        if !note.files_changed.is_empty() {
            out.push_str(&format!(
                "Files changed: {}\n",
                note.files_changed.join(", ")
            ));
        }
        let summary = if note.summary.len() > MAX_SUMMARY_LEN {
            &note.summary[..MAX_SUMMARY_LEN]
        } else {
            &note.summary
        };
        if !summary.is_empty() {
            out.push_str(&format!("Summary: {summary}\n"));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_append_and_read_notes() {
        let dir = tempfile::tempdir().unwrap();
        let note = WorkerNote {
            task_number: 1,
            task_title: "Add auth".to_string(),
            files_changed: vec!["src/auth.rs".to_string()],
            summary: "Added JWT middleware".to_string(),
        };
        append_note(dir.path(), &note).unwrap();

        let notes = read_all_notes(dir.path());
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].task_number, 1);
    }

    #[test]
    fn test_read_dependency_notes() {
        let dir = tempfile::tempdir().unwrap();
        for i in 1..=3 {
            let note = WorkerNote {
                task_number: i,
                task_title: format!("Task {i}"),
                files_changed: vec![],
                summary: format!("Did task {i}"),
            };
            append_note(dir.path(), &note).unwrap();
        }

        let deps = read_dependency_notes(dir.path(), &[1, 3]);
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].task_number, 1);
        assert_eq!(deps[1].task_number, 3);
    }

    #[test]
    fn test_read_dependency_notes_empty() {
        let dir = tempfile::tempdir().unwrap();
        let deps = read_dependency_notes(dir.path(), &[]);
        assert!(deps.is_empty());
    }

    #[test]
    fn test_format_notes_empty() {
        assert!(format_notes_for_prompt(&[]).is_empty());
    }

    #[test]
    fn test_format_notes_with_data() {
        let notes = vec![WorkerNote {
            task_number: 1,
            task_title: "Add auth".to_string(),
            files_changed: vec!["src/auth.rs".to_string()],
            summary: "Added JWT".to_string(),
        }];
        let out = format_notes_for_prompt(&notes);
        assert!(out.contains("Task 1: Add auth"));
        assert!(out.contains("src/auth.rs"));
        assert!(out.contains("Added JWT"));
    }
}
