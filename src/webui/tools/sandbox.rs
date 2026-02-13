use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

/// Resolve a file path relative to the session cwd, ensuring it's valid.
/// Allows absolute paths that exist, and resolves relative paths against cwd.
pub fn validate_path(path: &str, cwd: &Path) -> Result<PathBuf> {
    let resolved = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        cwd.join(path)
    };

    // Canonicalize to resolve .. and symlinks (only if parent exists for new files)
    let canonical = if resolved.exists() {
        resolved.canonicalize().unwrap_or_else(|_| resolved.clone())
    } else {
        // For new files, canonicalize the parent directory
        if let Some(parent) = resolved.parent() {
            if parent.exists() {
                let canonical_parent = parent
                    .canonicalize()
                    .unwrap_or_else(|_| parent.to_path_buf());
                let file_name = resolved.file_name().unwrap_or_default();
                canonical_parent.join(file_name)
            } else {
                resolved
            }
        } else {
            resolved
        }
    };

    Ok(canonical)
}

/// Check if a bash command is potentially dangerous and should be blocked.
pub fn check_dangerous_command(command: &str) -> Result<()> {
    let dangerous_patterns = [
        "rm -rf /",
        "rm -rf /*",
        "mkfs.",
        "dd if=/dev/zero",
        ":(){ :|:& };:",
        "> /dev/sda",
    ];

    let lower = command.to_lowercase();
    for pattern in &dangerous_patterns {
        if lower.contains(pattern) {
            bail!("Blocked dangerous command pattern: {pattern}");
        }
    }

    Ok(())
}
