use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::webui::error::{ApiError, ApiResult};
use crate::webui::projects::detection::run_cmd;

use super::types::FileStatus;

// ── Path Validation ─────────────────────────────────────────────────────────

/// Validate that the project path is a directory inside a git repository.
/// Canonicalizes the path to prevent path traversal attacks.
pub(super) fn validate_project_path(path_str: &str) -> ApiResult<PathBuf> {
    let path = PathBuf::from(path_str)
        .canonicalize()
        .map_err(|_| ApiError::BadRequest(format!("Invalid project path: '{path_str}'")))?;

    if !path.is_dir() {
        return Err(ApiError::BadRequest(format!(
            "Project path is not a directory: '{path_str}'"
        )));
    }

    // Verify this is actually a git repo (prevents arbitrary directory access)
    if !path.join(".git").exists() && !path.join(".git").is_file() {
        // Also check if we're inside a worktree (where .git is a file)
        let git_check = std::process::Command::new("git")
            .args(["rev-parse", "--git-dir"])
            .current_dir(&path)
            .output();
        if !git_check.map(|o| o.status.success()).unwrap_or(false) {
            return Err(ApiError::BadRequest(format!(
                "Not a git repository: '{path_str}'"
            )));
        }
    }

    Ok(path)
}

/// Validate that a file path is safe (no path traversal).
/// Returns the resolved file path that is guaranteed to be under `repo_root`.
pub(super) fn validate_file_path(repo_root: &Path, relative: &str) -> ApiResult<PathBuf> {
    // Reject obviously malicious paths
    if relative.contains("..") || relative.starts_with('/') || relative.starts_with('\\') {
        return Err(ApiError::BadRequest(format!(
            "Invalid file path: '{relative}'"
        )));
    }

    let full = repo_root.join(relative);

    // Canonicalize if it exists, otherwise just verify the joined path stays under root
    let resolved = if full.exists() {
        full.canonicalize()
            .map_err(|_| ApiError::BadRequest(format!("Cannot resolve file path: '{relative}'")))?
    } else {
        full
    };

    if !resolved.starts_with(repo_root) {
        return Err(ApiError::BadRequest(format!(
            "File path escapes repository: '{relative}'"
        )));
    }

    Ok(resolved)
}

// ── Git Output Parsing ──────────────────────────────────────────────────────

/// Parse git status --porcelain=v1 output to determine file status.
/// Format: `XY PATH` or `XY ORIG -> PATH` for renames.
pub(super) fn parse_status_line(line: &str) -> Option<(String, char, char)> {
    // Porcelain v1: first 2 chars are status codes, char 3 is always a space
    let bytes = line.as_bytes();
    if bytes.len() < 4 || bytes[2] != b' ' {
        return None;
    }

    let staged_char = bytes[0] as char;
    let unstaged_char = bytes[1] as char;
    let raw_path = &line[3..];

    // For renames/copies, format is "ORIG -> NEW" — extract the new path
    let path = if (staged_char == 'R' || staged_char == 'C') && raw_path.contains(" -> ") {
        raw_path
            .rsplit_once(" -> ")
            .map(|(_, new)| new)
            .unwrap_or(raw_path)
    } else {
        raw_path
    };

    Some((path.to_string(), staged_char, unstaged_char))
}

/// Convert status character to FileStatus enum
pub(super) fn char_to_file_status(c: char) -> FileStatus {
    match c {
        'M' => FileStatus::Modified,
        'A' => FileStatus::Added,
        'D' => FileStatus::Deleted,
        'R' => FileStatus::Renamed,
        'C' => FileStatus::Copied,
        '?' => FileStatus::Untracked,
        _ => FileStatus::Modified,
    }
}

/// Parse git diff --numstat output to get additions/deletions for each file.
/// Handles renames (`old => new` or `{prefix => suffix}`) and paths with spaces.
pub(super) async fn get_diff_stats(
    path: &Path,
    staged: bool,
    timeout: Duration,
) -> HashMap<String, (u32, u32)> {
    let args = if staged {
        vec!["diff", "--cached", "--numstat", "--"]
    } else {
        vec!["diff", "--numstat", "--"]
    };

    let output = match run_cmd("git", &args, path, timeout).await {
        Some(o) => o,
        None => return HashMap::new(),
    };

    let mut stats = HashMap::new();
    for line in output.lines() {
        // numstat format: "ADDED\tDELETED\tFILEPATH"
        let mut parts = line.splitn(3, '\t');
        let additions = parts
            .next()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        let deletions = parts
            .next()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        if let Some(filepath) = parts.next() {
            // For renames: "old => new" — use the new path
            let key = if filepath.contains(" => ") {
                filepath
                    .rsplit_once(" => ")
                    .map(|(_, new)| new.trim_end_matches('}'))
                    .unwrap_or(filepath)
            } else {
                filepath
            };
            stats.insert(key.to_string(), (additions, deletions));
        }
    }

    stats
}
