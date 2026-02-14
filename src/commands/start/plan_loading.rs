use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::types::{LegacyJsonPlan, Plan};

pub fn find_plan(name: &str) -> Result<PathBuf> {
    // Search in .hive/plans/ first, fall back to .hive/prds/ for backwards compat
    // Note: prds/ is often a symlink to plans/, so only search one directory
    let plans_dir = PathBuf::from(".hive/plans");
    let prds_dir = PathBuf::from(".hive/prds");

    let search_dir = if plans_dir.exists() {
        plans_dir
    } else if prds_dir.exists() {
        prds_dir
    } else {
        bail!("No plans directory found. Run 'hive init' first.");
    };

    // Search in priority order: markdown first (preferred), then legacy JSON (backward compat)
    let candidates = [
        format!("{}.md", name),
        format!("plan-{}.md", name),
        format!("plan-{}.json", name),
        format!("{}.json", name),
        format!("prd-{}.json", name),
    ];

    for filename in &candidates {
        let path = search_dir.join(filename);
        if path.exists() {
            return Ok(path);
        }
    }

    // No candidates found — list available plans
    let mut available = Vec::new();
    for entry in fs::read_dir(&search_dir).into_iter().flatten().flatten() {
        let path = entry.path();
        let ext = path.extension().and_then(|s| s.to_str());
        if ext == Some("md") || ext == Some("json") {
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                available.push(filename.to_string());
            }
        }
    }

    if available.is_empty() {
        bail!(
            "No plan found for drone '{}'. No plans available in .hive/plans/",
            name
        );
    } else {
        bail!(
            "No plan found for drone '{}'. Available plans:\n  {}",
            name,
            available.join("\n  ")
        );
    }
}

pub fn load_plan(path: &Path) -> Result<Plan> {
    let contents = fs::read_to_string(path).context("Failed to read plan")?;

    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let plan = match ext {
        "md" => {
            // Markdown plan: ID from filename, content is the raw markdown
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            // Parse YAML frontmatter for target_branch/base_branch
            let (target_branch, base_branch, content) = parse_frontmatter(&contents);

            // Parse structured tasks from ## Tasks section
            let structured_tasks = crate::plan_parser::parse_tasks(&content);

            Plan {
                id,
                content,
                target_branch,
                base_branch,
                structured_tasks,
            }
        }
        "json" => {
            // Legacy JSON plan: convert to Plan
            let legacy: LegacyJsonPlan =
                serde_json::from_str(&contents).context("Failed to parse plan JSON")?;
            legacy.into()
        }
        _ => bail!("Unsupported plan file format: {}", ext),
    };

    // Validate non-empty content
    if plan.content.trim().is_empty() {
        bail!("Plan content cannot be empty");
    }

    Ok(plan)
}

/// Parse optional YAML frontmatter from markdown content.
/// Returns (target_branch, base_branch, content_without_frontmatter).
pub fn parse_frontmatter(raw: &str) -> (Option<String>, Option<String>, String) {
    let trimmed = raw.trim_start();
    if !trimmed.starts_with("---") {
        return (None, None, raw.to_string());
    }

    // Find the closing ---
    let after_opening = &trimmed[3..];
    if let Some(end) = after_opening.find("\n---") {
        let frontmatter = &after_opening[..end];
        let rest = &after_opening[end + 4..]; // skip \n---

        let mut target_branch = None;
        let mut base_branch = None;

        for line in frontmatter.lines() {
            let line = line.trim();
            if let Some(value) = line.strip_prefix("target_branch:") {
                target_branch = Some(value.trim().to_string());
            } else if let Some(value) = line.strip_prefix("base_branch:") {
                base_branch = Some(value.trim().to_string());
            }
        }

        // Strip leading newline from rest
        let content = rest.strip_prefix('\n').unwrap_or(rest);
        (target_branch, base_branch, content.to_string())
    } else {
        (None, None, raw.to_string())
    }
}
