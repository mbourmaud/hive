use anyhow::Result;
use std::path::{Path, PathBuf};

/// File picker with fuzzy search capability
#[derive(Debug, Clone)]
pub struct FilePicker {
    /// All available files
    pub all_files: Vec<PathBuf>,
    /// Filtered files matching the current query
    pub filtered_files: Vec<PathBuf>,
    /// Currently selected index
    pub selected: usize,
    /// Current search query
    pub query: String,
    /// Whether the picker is visible
    pub visible: bool,
}

impl Default for FilePicker {
    fn default() -> Self {
        Self::new()
    }
}

impl FilePicker {
    /// Create a new file picker
    pub fn new() -> Self {
        Self {
            all_files: Vec::new(),
            filtered_files: Vec::new(),
            selected: 0,
            query: String::new(),
            visible: false,
        }
    }

    /// Show the file picker and scan for files
    pub fn show(&mut self, base_dir: &Path) -> Result<()> {
        self.scan_files(base_dir)?;
        self.filtered_files = self.all_files.clone();
        self.selected = 0;
        self.query.clear();
        self.visible = true;
        Ok(())
    }

    /// Hide the file picker
    pub fn hide(&mut self) {
        self.visible = false;
        self.query.clear();
        self.selected = 0;
    }

    /// Update the filter with a new query
    pub fn update_filter(&mut self, query: &str) {
        self.query = query.to_string();
        self.filtered_files = self.fuzzy_filter(&self.all_files, query);
        self.selected = 0;
    }

    /// Fuzzy filter files by query
    fn fuzzy_filter(&self, files: &[PathBuf], query: &str) -> Vec<PathBuf> {
        if query.is_empty() {
            return files.to_vec();
        }

        let query_lower = query.to_lowercase();
        let mut scored_files: Vec<(PathBuf, i32)> = files
            .iter()
            .filter_map(|path| {
                let path_str = path.to_string_lossy().to_lowercase();
                self.fuzzy_match(&path_str, &query_lower)
                    .map(|score| (path.clone(), score))
            })
            .collect();

        // Sort by score (higher is better)
        scored_files.sort_by(|a, b| b.1.cmp(&a.1));

        scored_files.into_iter().map(|(path, _)| path).collect()
    }

    /// Simple fuzzy matching algorithm
    /// Returns Some(score) if the query matches, None otherwise
    fn fuzzy_match(&self, text: &str, query: &str) -> Option<i32> {
        let mut score = 0;
        let mut last_match_idx = 0;
        let text_chars: Vec<char> = text.chars().collect();
        let query_chars: Vec<char> = query.chars().collect();

        for query_char in query_chars.iter() {
            let mut found = false;
            for (i, text_char) in text_chars.iter().enumerate().skip(last_match_idx) {
                if text_char == query_char {
                    score += 1;
                    // Bonus for consecutive matches
                    if i == last_match_idx {
                        score += 5;
                    }
                    last_match_idx = i + 1;
                    found = true;
                    break;
                }
            }
            if !found {
                return None;
            }
        }

        Some(score)
    }

    /// Scan files in the directory (excluding common ignore patterns)
    fn scan_files(&mut self, base_dir: &Path) -> Result<()> {
        self.all_files.clear();

        // Walk the directory tree
        self.walk_dir(base_dir, base_dir)?;

        // Sort files alphabetically
        self.all_files.sort();

        Ok(())
    }

    /// Recursively walk a directory
    fn walk_dir(&mut self, dir: &Path, base_dir: &Path) -> Result<()> {
        // Skip common ignore directories
        let dir_name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if self.should_ignore_dir(dir_name) {
            return Ok(());
        }

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();

                if path.is_dir() {
                    self.walk_dir(&path, base_dir)?;
                } else if path.is_file() {
                    // Store relative path from base_dir
                    if let Ok(rel_path) = path.strip_prefix(base_dir) {
                        self.all_files.push(rel_path.to_path_buf());
                    }
                }
            }
        }

        Ok(())
    }

    /// Check if a directory should be ignored
    fn should_ignore_dir(&self, name: &str) -> bool {
        matches!(
            name,
            ".git"
                | "node_modules"
                | "target"
                | "build"
                | "dist"
                | ".hive"
                | ".next"
                | "__pycache__"
                | ".venv"
                | "venv"
        )
    }

    /// Select the next file
    pub fn select_next(&mut self) {
        if !self.filtered_files.is_empty() {
            self.selected = (self.selected + 1) % self.filtered_files.len();
        }
    }

    /// Select the previous file
    pub fn select_prev(&mut self) {
        if !self.filtered_files.is_empty() {
            if self.selected == 0 {
                self.selected = self.filtered_files.len() - 1;
            } else {
                self.selected -= 1;
            }
        }
    }

    /// Get the currently selected file
    pub fn get_selected(&self) -> Option<&PathBuf> {
        self.filtered_files.get(self.selected)
    }

    /// Accept the selected file and return it
    pub fn accept(&mut self) -> Option<PathBuf> {
        let file = self.get_selected().cloned();
        self.hide();
        file
    }

    /// Get a slice of files to display (for pagination)
    pub fn get_display_files(&self, max_items: usize) -> &[PathBuf] {
        let start = if self.selected >= max_items {
            self.selected - max_items + 1
        } else {
            0
        };
        let end = (start + max_items).min(self.filtered_files.len());
        &self.filtered_files[start..end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_match() {
        let picker = FilePicker::new();

        // Exact matches should score high
        assert!(picker.fuzzy_match("hello.rs", "hello").is_some());

        // Consecutive character bonus
        assert!(picker.fuzzy_match("hello.rs", "hel").is_some());

        // Non-matches
        assert!(picker.fuzzy_match("hello.rs", "xyz").is_none());
    }

    #[test]
    fn test_should_ignore_dir() {
        let picker = FilePicker::new();
        assert!(picker.should_ignore_dir(".git"));
        assert!(picker.should_ignore_dir("node_modules"));
        assert!(picker.should_ignore_dir("target"));
        assert!(!picker.should_ignore_dir("src"));
    }
}
