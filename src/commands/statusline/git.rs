use std::process::Command as ProcessCommand;

use super::{GREEN, MAGENTA, RED, RESET, YELLOW};

pub fn build_git_part(current_dir: &str) -> Option<String> {
    let branch = git_branch(current_dir)?;
    let icons = git_icons(current_dir);
    let upstream = git_upstream(current_dir);

    let mut part = format!("{MAGENTA}{branch}{RESET}");
    if !icons.is_empty() {
        part.push_str(&icons);
    }
    if !upstream.is_empty() {
        part.push_str(&format!("{YELLOW}{upstream}{RESET}"));
    }
    Some(part)
}

pub fn git_branch(current_dir: &str) -> Option<String> {
    let output = ProcessCommand::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(current_dir)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if branch.is_empty() {
        None
    } else {
        Some(branch)
    }
}

pub fn git_icons(current_dir: &str) -> String {
    let output = match ProcessCommand::new("git")
        .args(["status", "--porcelain"])
        .current_dir(current_dir)
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return String::new(),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut has_untracked = false;
    let mut has_unstaged = false;
    let mut has_staged = false;

    for line in stdout.lines() {
        let bytes = line.as_bytes();
        if bytes.is_empty() {
            continue;
        }
        // Untracked: line starts with '?'
        if bytes[0] == b'?' {
            has_untracked = true;
        }
        // Staged: non-space in column 1 (index 0), excluding '?'
        if bytes[0] != b' ' && bytes[0] != b'?' {
            has_staged = true;
        }
        // Unstaged/modified: non-space in column 2 (index 1), excluding '?'
        if bytes.len() > 1 && bytes[1] != b' ' && bytes[0] != b'?' {
            has_unstaged = true;
        }
    }

    let mut icons = String::new();
    if has_untracked {
        icons.push('+');
    }
    if has_unstaged {
        icons.push('!');
    }
    if has_staged {
        icons.push('*');
    }
    icons
}

pub fn git_upstream(current_dir: &str) -> String {
    let mut result = String::new();

    // Ahead
    if let Ok(output) = ProcessCommand::new("git")
        .args(["rev-list", "--count", "@{upstream}..HEAD"])
        .current_dir(current_dir)
        .output()
    {
        if output.status.success() {
            if let Ok(n) = String::from_utf8_lossy(&output.stdout)
                .trim()
                .parse::<u32>()
            {
                if n > 0 {
                    result.push_str(&format!("\u{2191}{n}"));
                }
            }
        }
    }

    // Behind
    if let Ok(output) = ProcessCommand::new("git")
        .args(["rev-list", "--count", "HEAD..@{upstream}"])
        .current_dir(current_dir)
        .output()
    {
        if output.status.success() {
            if let Ok(n) = String::from_utf8_lossy(&output.stdout)
                .trim()
                .parse::<u32>()
            {
                if n > 0 {
                    result.push_str(&format!("\u{2193}{n}"));
                }
            }
        }
    }

    result
}

pub fn context_color(pct: f64) -> &'static str {
    if pct > 80.0 {
        RED
    } else if pct >= 50.0 {
        YELLOW
    } else {
        GREEN
    }
}
