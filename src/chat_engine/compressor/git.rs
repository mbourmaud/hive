//! Compressors for git command output (status, diff, log).

use regex::Regex;
use std::sync::LazyLock;

static GIT_LOG_HASH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^commit ([a-f0-9]{40})").unwrap());

static GIT_LOG_AUTHOR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^Author:\s+(.+)$").unwrap());

static GIT_LOG_DATE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^Date:\s+(.+)$").unwrap());

/// Try to compress git output. Returns None if the output doesn't match.
pub fn try_compress(content: &str) -> Option<String> {
    if content.starts_with("On branch ") || content.starts_with("HEAD detached at ") {
        return Some(compress_git_status(content));
    }
    if content.contains("diff --git ") {
        return Some(compress_git_diff(content));
    }
    if GIT_LOG_HASH.is_match(content) {
        return Some(compress_git_log(content));
    }
    None
}

fn compress_git_status(output: &str) -> String {
    let mut branch = String::new();
    let mut tracking_info = String::new();
    let mut modified: Vec<&str> = Vec::new();
    let mut untracked: Vec<&str> = Vec::new();
    let mut staged: Vec<&str> = Vec::new();
    let mut deleted: Vec<&str> = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("On branch ") {
            branch = rest.to_string();
        } else if trimmed.starts_with("HEAD detached at ") {
            branch = trimmed.to_string();
        } else if trimmed.contains("up to date") {
            tracking_info = " (up to date)".to_string();
        } else if (trimmed.contains("ahead of") || trimmed.contains("behind"))
            && tracking_info.is_empty()
        {
            tracking_info = extract_ahead_behind(trimmed);
        } else if let Some(file) = trimmed.strip_prefix("modified:") {
            modified.push(file.trim());
        } else if let Some(file) = trimmed.strip_prefix("new file:") {
            staged.push(file.trim());
        } else if let Some(file) = trimmed.strip_prefix("deleted:") {
            deleted.push(file.trim());
        } else if !trimmed.is_empty()
            && !trimmed.starts_with('(')
            && !trimmed.starts_with("Changes")
            && !trimmed.starts_with("Untracked")
            && !trimmed.starts_with("Your branch")
            && !trimmed.starts_with("no changes")
        {
            // Untracked files are listed without a prefix
            if output.contains("Untracked files:") && is_likely_file_path(trimmed) {
                untracked.push(trimmed);
            }
        }
    }

    let mut result = format!("branch: {branch}{tracking_info}");
    if !staged.is_empty() {
        result.push_str(&format!("\nA {}", staged.join(", ")));
    }
    if !modified.is_empty() {
        result.push_str(&format!("\nM {}", modified.join(", ")));
    }
    if !deleted.is_empty() {
        result.push_str(&format!("\nD {}", deleted.join(", ")));
    }
    if !untracked.is_empty() {
        result.push_str(&format!("\n? {}", untracked.join(", ")));
    }
    result
}

fn extract_ahead_behind(line: &str) -> String {
    // Extract "ahead of 'origin/main' by 3" or "behind 'origin/main' by 2" patterns
    if line.contains("ahead") && line.contains("behind") {
        return " (diverged)".to_string();
    }
    if line.contains("ahead") {
        return " (ahead)".to_string();
    }
    " (behind)".to_string()
}

fn is_likely_file_path(s: &str) -> bool {
    s.contains('/') || s.contains('.') || s.ends_with('/')
}

fn compress_git_diff(output: &str) -> String {
    let mut result = String::new();
    let mut current_file: Option<&str> = None;
    let mut hunks_for_file = 0;
    let max_hunks_per_file: usize = 3;

    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            // New file — extract b/path
            if let Some(b_path) = rest.split(" b/").nth(1) {
                current_file = Some(b_path);
                hunks_for_file = 0;
                result.push_str(&format!("\n--- {b_path} ---\n"));
            }
        } else if line.starts_with("@@") {
            hunks_for_file += 1;
            if hunks_for_file <= max_hunks_per_file {
                result.push_str(line);
                result.push('\n');
            } else if hunks_for_file == max_hunks_per_file + 1 {
                let file_name = current_file.unwrap_or("file");
                result.push_str(&format!("  ... (more hunks in {file_name})\n"));
            }
        } else if line.starts_with('+') || line.starts_with('-') {
            // Keep added/removed lines within hunk limit
            if hunks_for_file <= max_hunks_per_file
                && !line.starts_with("+++")
                && !line.starts_with("---")
            {
                result.push_str(line);
                result.push('\n');
            }
        }
        // Drop context lines (lines starting with space) to save tokens
    }

    let trimmed = result.trim();
    if trimmed.is_empty() {
        return output.to_string();
    }
    trimmed.to_string()
}

fn compress_git_log(output: &str) -> String {
    let mut entries: Vec<String> = Vec::new();
    let mut current_hash = String::new();
    let mut current_author = String::new();
    let mut current_date = String::new();
    let mut current_subject = String::new();

    for line in output.lines() {
        if let Some(caps) = GIT_LOG_HASH.captures(line) {
            // Flush previous entry
            if !current_hash.is_empty() {
                entries.push(format_log_entry(
                    &current_hash,
                    &current_subject,
                    &current_author,
                    &current_date,
                ));
            }
            current_hash = caps[1][..8].to_string();
            current_author.clear();
            current_date.clear();
            current_subject.clear();
        } else if let Some(caps) = GIT_LOG_AUTHOR.captures(line) {
            current_author = caps[1].trim().to_string();
            // Strip email if present
            if let Some(name_end) = current_author.find(" <") {
                current_author = current_author[..name_end].to_string();
            }
        } else if let Some(caps) = GIT_LOG_DATE.captures(line) {
            current_date = caps[1].trim().to_string();
        } else {
            let trimmed = line.trim();
            if !trimmed.is_empty() && current_subject.is_empty() {
                current_subject = trimmed.to_string();
            }
        }
    }

    // Flush last entry
    if !current_hash.is_empty() {
        entries.push(format_log_entry(
            &current_hash,
            &current_subject,
            &current_author,
            &current_date,
        ));
    }

    if entries.is_empty() {
        return output.to_string();
    }
    entries.join("\n")
}

fn format_log_entry(hash: &str, subject: &str, author: &str, date: &str) -> String {
    format!("{hash} {subject} ({author}, {date})")
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_compress_git_status() {
        let input = "\
On branch feat/my-feature
Your branch is up to date with 'origin/feat/my-feature'.

Changes not staged for commit:
  (use \"git add <file>...\" to update what will be committed)
  (use \"git restore <file>...\" to discard changes in working directory)
        modified:   src/main.rs
        modified:   src/lib.rs
        modified:   src/utils/helper.rs

Untracked files:
  (use \"git add <file>...\" to include in what will be committed)
        src/new_file.rs
        tests/test_new.rs
";
        let compressed = compress_git_status(input);
        assert!(compressed.contains("branch: feat/my-feature"));
        assert!(compressed.contains("(up to date)"));
        assert!(compressed.contains("M src/main.rs"));
        assert!(compressed.contains("? src/new_file.rs"));
        assert!(compressed.len() < input.len());
    }

    #[test]
    fn test_compress_git_diff() {
        let input = "\
diff --git a/src/main.rs b/src/main.rs
index abc123..def456 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -10,6 +10,7 @@ fn main() {
     let x = 1;
+    let y = 2;
     let z = 3;
";
        let compressed = compress_git_diff(input);
        assert!(compressed.contains("--- src/main.rs ---"));
        assert!(compressed.contains("+    let y = 2;"));
        // Context lines (starting with space) should be stripped
        assert!(!compressed.contains("     let x = 1;"));
    }

    #[test]
    fn test_compress_git_log() {
        let input = "\
commit abcdef1234567890abcdef1234567890abcdef12
Author: John Doe <john@example.com>
Date:   Mon Jan 1 12:00:00 2024 +0000

    feat: add new feature

commit 1234567890abcdef1234567890abcdef12345678
Author: Jane Smith <jane@example.com>
Date:   Sun Dec 31 10:00:00 2023 +0000

    fix: resolve bug
";
        let compressed = compress_git_log(input);
        assert!(compressed.contains("abcdef12 feat: add new feature (John Doe,"));
        assert!(compressed.contains("12345678 fix: resolve bug (Jane Smith,"));
        assert!(compressed.len() < input.len());
    }
}
