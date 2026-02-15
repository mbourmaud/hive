use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use axum::extract::Query;
use axum::Json;
use serde::Deserialize;

use crate::webui::error::{ApiError, ApiResult};
use crate::webui::projects::detection::{detect_open_pr, detect_platform, run_cmd};

use super::types::{ChangedFile, FileDiff, FileStatus, GitStatus, PrSummary};

// ── Query Parameters ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct StatusQuery {
    project_path: String,
}

#[derive(Debug, Deserialize)]
pub struct DiffQuery {
    project_path: String,
    file: String,
    #[serde(default)]
    staged: bool,
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Validate that the project path exists and is a directory
fn validate_project_path(path_str: &str) -> ApiResult<PathBuf> {
    let path = PathBuf::from(path_str);

    if !path.exists() {
        return Err(ApiError::BadRequest(format!(
            "Project path does not exist: {}",
            path_str
        )));
    }

    if !path.is_dir() {
        return Err(ApiError::BadRequest(format!(
            "Project path is not a directory: {}",
            path_str
        )));
    }

    Ok(path)
}

/// Parse git status --porcelain=v1 output to determine file status
fn parse_status_line(line: &str) -> Option<(String, char, char)> {
    if line.len() < 4 {
        return None;
    }

    let staged_char = line.chars().next()?;
    let unstaged_char = line.chars().nth(1)?;
    let path = line[3..].to_string();

    Some((path, staged_char, unstaged_char))
}

/// Convert status character to FileStatus enum
fn char_to_file_status(c: char) -> FileStatus {
    match c {
        'M' => FileStatus::Modified,
        'A' => FileStatus::Added,
        'D' => FileStatus::Deleted,
        'R' => FileStatus::Renamed,
        'C' => FileStatus::Copied,
        '?' => FileStatus::Untracked,
        _ => FileStatus::Modified, // Default fallback
    }
}

/// Parse git diff --numstat output to get additions/deletions for each file
async fn get_diff_stats(
    path: &Path,
    staged: bool,
    timeout: Duration,
) -> HashMap<String, (u32, u32)> {
    let args = if staged {
        vec!["diff", "--cached", "--numstat"]
    } else {
        vec!["diff", "--numstat"]
    };

    let output = match run_cmd("git", &args, path, timeout).await {
        Some(o) => o,
        None => return HashMap::new(),
    };

    let mut stats = HashMap::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            let additions = parts[0].parse::<u32>().unwrap_or(0);
            let deletions = parts[1].parse::<u32>().unwrap_or(0);
            let filepath = parts[2].to_string();
            stats.insert(filepath, (additions, deletions));
        }
    }

    stats
}

// ── Handlers ────────────────────────────────────────────────────────────────

pub async fn git_status(Query(query): Query<StatusQuery>) -> ApiResult<Json<GitStatus>> {
    let path = validate_project_path(&query.project_path)?;
    let timeout = Duration::from_secs(3);

    // Get current branch
    let branch = run_cmd("git", &["branch", "--show-current"], &path, timeout)
        .await
        .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("Failed to get current branch")))?;

    // Get remote URL
    let remote_url = run_cmd("git", &["remote", "get-url", "origin"], &path, timeout)
        .await
        .unwrap_or_default();

    // Detect platform
    let platform = detect_platform(&remote_url);

    // Get ahead/behind counts
    let (ahead, behind) = match run_cmd(
        "git",
        &["rev-list", "--left-right", "--count", "@{upstream}...HEAD"],
        &path,
        timeout,
    )
    .await
    {
        Some(output) => {
            let parts: Vec<&str> = output.split_whitespace().collect();
            let behind_val = parts
                .first()
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0);
            let ahead_val = parts
                .get(1)
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0);
            (ahead_val, behind_val)
        }
        None => (0, 0),
    };

    // Get base branch (upstream)
    let base_branch = run_cmd(
        "git",
        &["rev-parse", "--abbrev-ref", "@{upstream}"],
        &path,
        timeout,
    )
    .await
    .and_then(|s| {
        // Strip "origin/" or other remote prefix
        s.split('/').next_back().map(|s| s.to_string())
    });

    // Get last commit
    let last_commit = run_cmd(
        "git",
        &["log", "-1", "--format=%h %s (%cr)"],
        &path,
        timeout,
    )
    .await;

    // Get status --porcelain=v1
    let status_output = run_cmd("git", &["status", "--porcelain=v1"], &path, timeout)
        .await
        .unwrap_or_default();

    // Get diff stats for staged and unstaged changes
    let staged_stats = get_diff_stats(&path, true, timeout).await;
    let unstaged_stats = get_diff_stats(&path, false, timeout).await;

    // Parse status output
    let mut staged = Vec::new();
    let mut unstaged = Vec::new();
    let mut untracked = Vec::new();

    for line in status_output.lines() {
        if let Some((filepath, staged_char, unstaged_char)) = parse_status_line(line) {
            // Handle untracked files (??   )
            if staged_char == '?' && unstaged_char == '?' {
                untracked.push(ChangedFile {
                    path: filepath.clone(),
                    status: FileStatus::Untracked,
                    additions: 0,
                    deletions: 0,
                });
                continue;
            }

            // Handle staged changes (first char != ' ' and != '?')
            if staged_char != ' ' && staged_char != '?' {
                let (additions, deletions) = staged_stats.get(&filepath).copied().unwrap_or((0, 0));
                staged.push(ChangedFile {
                    path: filepath.clone(),
                    status: char_to_file_status(staged_char),
                    additions,
                    deletions,
                });
            }

            // Handle unstaged changes (second char != ' ')
            if unstaged_char != ' ' {
                let (additions, deletions) =
                    unstaged_stats.get(&filepath).copied().unwrap_or((0, 0));
                unstaged.push(ChangedFile {
                    path: filepath.clone(),
                    status: char_to_file_status(unstaged_char),
                    additions,
                    deletions,
                });
            }
        }
    }

    // Detect open PR/MR
    let open_pr = detect_open_pr(&path, &platform, &branch)
        .await
        .map(|pr| PrSummary {
            number: pr.number,
            title: pr.title,
            url: pr.url,
            state: pr.state,
            is_draft: pr.is_draft,
        });

    Ok(Json(GitStatus {
        branch,
        base_branch,
        remote_url,
        platform,
        ahead,
        behind,
        staged,
        unstaged,
        untracked,
        open_pr,
        last_commit,
    }))
}

pub async fn git_diff(Query(query): Query<DiffQuery>) -> ApiResult<Json<FileDiff>> {
    let path = validate_project_path(&query.project_path)?;
    let timeout = Duration::from_secs(3);

    // Get the diff
    let diff_args = if query.staged {
        vec!["diff", "--cached", &query.file]
    } else {
        vec!["diff", &query.file]
    };

    let diff = run_cmd("git", &diff_args, &path, timeout)
        .await
        .unwrap_or_default();

    // Get old content (from HEAD)
    let old_content = run_cmd(
        "git",
        &["show", &format!("HEAD:{}", query.file)],
        &path,
        timeout,
    )
    .await
    .unwrap_or_default();

    // Get new content (from working copy)
    let file_path = path.join(&query.file);
    let new_content = tokio::fs::read_to_string(&file_path)
        .await
        .unwrap_or_default();

    Ok(Json(FileDiff {
        path: query.file,
        diff,
        old_content,
        new_content,
    }))
}
