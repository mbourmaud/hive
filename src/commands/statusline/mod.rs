mod git;
mod hive;

use anyhow::Result;
use serde::Deserialize;
use std::io::{self, Read};
use std::path::Path;

use super::common;

// Re-export submodule items (used by run() and tests via `use super::*`)
#[allow(unused_imports)]
pub(crate) use git::{build_git_part, context_color, git_branch, git_icons, git_upstream};
#[allow(unused_imports)]
pub(crate) use hive::{build_line2, find_hive_root, format_drone, list_drones_at};

// ANSI escape codes
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const MAGENTA: &str = "\x1b[35m";
const CYAN: &str = "\x1b[36m";
const GRAY: &str = "\x1b[90m";
const BRIGHT_GREEN: &str = "\x1b[92m";
const BRIGHT_RED: &str = "\x1b[91m";
const LIGHT_BLUE: &str = "\x1b[94m";

const SEP: &str = " \x1b[90m\u{2502}\x1b[0m ";

#[derive(Deserialize, Default)]
struct StatuslineInput {
    workspace: Option<Workspace>,
    model: Option<Model>,
    context_window: Option<ContextWindow>,
}

#[derive(Deserialize)]
struct Workspace {
    current_dir: String,
}

#[derive(Deserialize)]
struct Model {
    display_name: String,
}

#[derive(Deserialize)]
struct ContextWindow {
    used_percentage: f64,
}

pub fn run() -> Result<()> {
    let input = read_input();
    let current_dir = input
        .workspace
        .as_ref()
        .map(|w| w.current_dir.as_str())
        .unwrap_or(".");

    let line1 = build_line1(current_dir, &input);
    let line2 = build_line2(current_dir);

    if let Some(l2) = line2 {
        println!("{}\n{}", line1, l2);
    } else {
        println!("{}", line1);
    }

    Ok(())
}

fn read_input() -> StatuslineInput {
    let mut buf = String::new();
    if io::stdin().read_to_string(&mut buf).is_ok() && !buf.trim().is_empty() {
        serde_json::from_str(&buf).unwrap_or_default()
    } else {
        StatuslineInput::default()
    }
}

fn build_line1(current_dir: &str, input: &StatuslineInput) -> String {
    let mut parts: Vec<String> = Vec::new();

    // Project name
    let project = Path::new(current_dir)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| current_dir.to_string());
    parts.push(format!("{CYAN}{BOLD}{project}{RESET}"));

    // Git branch + icons
    if let Some(branch_part) = build_git_part(current_dir) {
        parts.push(branch_part);
    }

    // Model
    if let Some(model) = &input.model {
        parts.push(format!("{LIGHT_BLUE}{}{RESET}", model.display_name));
    }

    // Context %
    if let Some(ctx) = &input.context_window {
        let color = context_color(ctx.used_percentage);
        parts.push(format!("{color}{:.0}%{RESET}", ctx.used_percentage));
    }

    parts.join(SEP)
}

#[cfg(test)]
mod tests;
