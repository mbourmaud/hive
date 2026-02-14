use std::path::Path;

/// Guards file access per worker based on `task.files` metadata.
///
/// When a task specifies `files: [src/auth/, tests/auth/]`, the worker
/// is only allowed to modify files within those paths. This prevents
/// merge conflicts when multiple workers run concurrently.
pub struct FileOwnershipGuard {
    allowed_patterns: Vec<String>,
}

impl FileOwnershipGuard {
    /// Create a guard from the task's file patterns.
    /// Empty patterns = unrestricted access.
    pub fn new(file_patterns: &[String]) -> Self {
        Self {
            allowed_patterns: file_patterns.to_vec(),
        }
    }

    /// Check if a write to the given path is allowed.
    pub fn check_write(&self, path: &str) -> Result<(), String> {
        if self.allowed_patterns.is_empty() {
            return Ok(());
        }

        let normalized = normalize_path(path);
        for pattern in &self.allowed_patterns {
            let normalized_pattern = normalize_path(pattern);
            if matches_pattern(&normalized, &normalized_pattern) {
                return Ok(());
            }
        }

        Err(format!(
            "Write to '{}' is outside allowed files: [{}]",
            path,
            self.allowed_patterns.join(", ")
        ))
    }

    /// Generate a prompt hint describing the file ownership constraints.
    pub fn prompt_text(&self) -> String {
        if self.allowed_patterns.is_empty() {
            return "You have unrestricted file access for this task.".to_string();
        }

        let mut text =
            String::from("You are restricted to modifying ONLY these files/directories:\n");
        for pattern in &self.allowed_patterns {
            text.push_str(&format!("- `{pattern}`\n"));
        }
        text.push_str("\nDo NOT modify files outside these paths. Read access is unrestricted.");
        text
    }
}

/// Normalize a path by stripping leading `./` and trailing `/`.
fn normalize_path(path: &str) -> String {
    let p = path.strip_prefix("./").unwrap_or(path);
    p.strip_suffix('/').unwrap_or(p).to_string()
}

/// Check if a file path matches an allowed pattern.
/// Patterns can be:
/// - Exact file: `src/main.rs` matches `src/main.rs`
/// - Directory prefix: `src/auth` matches `src/auth/mod.rs`, `src/auth/handlers.rs`
/// - Glob-like: patterns ending in a known file match that file
fn matches_pattern(path: &str, pattern: &str) -> bool {
    if path == pattern {
        return true;
    }
    // Directory prefix match: pattern "src/auth" matches "src/auth/anything"
    if let Some(rest) = path.strip_prefix(pattern) {
        return rest.starts_with('/');
    }
    // Check if path is a child of pattern treated as directory
    let pattern_as_dir = format!("{pattern}/");
    path.starts_with(&pattern_as_dir)
}

/// Create ownership text for a task with given file patterns.
/// This is a convenience function combining guard creation and prompt generation.
pub fn ownership_prompt_for_files(files: &[String]) -> String {
    FileOwnershipGuard::new(files).prompt_text()
}

/// Validate that a path is safe (no directory traversal).
pub fn is_safe_path(path: &str) -> bool {
    let p = Path::new(path);
    for component in p.components() {
        if let std::path::Component::ParentDir = component {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unrestricted_access() {
        let guard = FileOwnershipGuard::new(&[]);
        assert!(guard.check_write("anything.rs").is_ok());
    }

    #[test]
    fn test_exact_file_match() {
        let guard = FileOwnershipGuard::new(&["src/main.rs".to_string()]);
        assert!(guard.check_write("src/main.rs").is_ok());
        assert!(guard.check_write("src/lib.rs").is_err());
    }

    #[test]
    fn test_directory_prefix_match() {
        let guard = FileOwnershipGuard::new(&["src/auth".to_string()]);
        assert!(guard.check_write("src/auth/mod.rs").is_ok());
        assert!(guard.check_write("src/auth/handlers.rs").is_ok());
        assert!(guard.check_write("src/backend/mod.rs").is_err());
    }

    #[test]
    fn test_trailing_slash_normalized() {
        let guard = FileOwnershipGuard::new(&["src/auth/".to_string()]);
        assert!(guard.check_write("src/auth/mod.rs").is_ok());
    }

    #[test]
    fn test_prompt_text_unrestricted() {
        let guard = FileOwnershipGuard::new(&[]);
        assert!(guard.prompt_text().contains("unrestricted"));
    }

    #[test]
    fn test_prompt_text_restricted() {
        let guard = FileOwnershipGuard::new(&["src/auth".to_string()]);
        let text = guard.prompt_text();
        assert!(text.contains("src/auth"));
        assert!(text.contains("restricted"));
    }

    #[test]
    fn test_safe_path_no_traversal() {
        assert!(is_safe_path("src/main.rs"));
        assert!(is_safe_path("./src/main.rs"));
        assert!(!is_safe_path("../../../etc/passwd"));
        assert!(!is_safe_path("src/../../etc/passwd"));
    }
}
