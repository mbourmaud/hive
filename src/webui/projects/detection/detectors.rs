use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;

use super::super::types::{GitContext, PrInfo, RuntimeInfo};
use super::run_cmd;

// ── Git detection ───────────────────────────────────────────────────────────

pub async fn detect_git(path: &Path) -> Option<GitContext> {
    let timeout = Duration::from_secs(2);

    let branch = run_cmd("git", &["branch", "--show-current"], path, timeout).await?;
    let remote_url = run_cmd("git", &["remote", "get-url", "origin"], path, timeout)
        .await
        .unwrap_or_default();

    let platform = detect_platform(&remote_url);

    // Dirty count
    let status_output = run_cmd("git", &["status", "--porcelain"], path, timeout).await;
    let dirty_count = status_output
        .as_deref()
        .map(|s| s.lines().filter(|l| !l.is_empty()).count() as u32)
        .unwrap_or(0);

    // Ahead/behind
    let (ahead, behind) = match run_cmd(
        "git",
        &["rev-list", "--left-right", "--count", "@{upstream}...HEAD"],
        path,
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

    // Last commit
    let last_commit = run_cmd("git", &["log", "-1", "--format=%h %s (%cr)"], path, timeout).await;

    Some(GitContext {
        branch,
        remote_url,
        platform,
        ahead,
        behind,
        dirty_count,
        last_commit,
    })
}

pub fn detect_platform(remote_url: &str) -> String {
    let url_lower = remote_url.to_lowercase();
    if url_lower.contains("github.com") {
        "github".to_string()
    } else if url_lower.contains("gitlab") {
        "gitlab".to_string()
    } else if url_lower.contains("bitbucket") {
        "bitbucket".to_string()
    } else {
        "unknown".to_string()
    }
}

// ── Runtime detection ───────────────────────────────────────────────────────

const RUNTIME_MARKERS: &[(&str, &str)] = &[
    ("Cargo.toml", "rust"),
    ("package.json", "node"),
    ("go.mod", "go"),
    ("pyproject.toml", "python"),
    ("requirements.txt", "python"),
    ("Dockerfile", "docker"),
    ("docker-compose.yml", "docker"),
    ("compose.yml", "docker"),
];

pub async fn detect_runtimes(path: &Path) -> Vec<RuntimeInfo> {
    let timeout = Duration::from_secs(5);

    // Scan directory once for marker files
    let entries: HashSet<String> = match tokio::fs::read_dir(path).await {
        Ok(mut dir) => {
            let mut names = HashSet::new();
            while let Ok(Some(entry)) = dir.next_entry().await {
                if let Some(name) = entry.file_name().to_str() {
                    names.insert(name.to_string());
                }
            }
            names
        }
        Err(_) => return Vec::new(),
    };

    let mut runtimes: Vec<RuntimeInfo> = Vec::new();
    let mut seen_runtimes: HashSet<String> = HashSet::new();

    for (marker, runtime_name) in RUNTIME_MARKERS {
        if entries.contains(*marker) && !seen_runtimes.contains(*runtime_name) {
            seen_runtimes.insert(runtime_name.to_string());

            let version = match *runtime_name {
                "node" => run_cmd("node", &["--version"], path, timeout).await,
                "rust" => run_cmd("rustc", &["--version"], path, timeout)
                    .await
                    .map(|v| v.replace("rustc ", "")),
                "python" => run_cmd("python3", &["--version"], path, timeout)
                    .await
                    .map(|v| v.replace("Python ", "")),
                "go" => run_cmd("go", &["version"], path, timeout)
                    .await
                    .and_then(|v| {
                        v.split_whitespace()
                            .nth(2)
                            .map(|s| s.trim_start_matches("go").to_string())
                    }),
                "docker" => run_cmd("docker", &["--version"], path, timeout)
                    .await
                    .and_then(|v| {
                        v.split_whitespace()
                            .nth(2)
                            .map(|s| s.trim_end_matches(',').to_string())
                    }),
                _ => None,
            };

            // Trim version to major.minor
            let version = version.map(|v| {
                let parts: Vec<&str> = v.trim_start_matches('v').split('.').collect();
                if parts.len() >= 2 {
                    format!("{}.{}", parts[0], parts[1])
                } else {
                    v
                }
            });

            runtimes.push(RuntimeInfo {
                name: runtime_name.to_string(),
                version,
                marker_file: marker.to_string(),
            });
        }
    }

    runtimes
}

// ── Key files detection ─────────────────────────────────────────────────────

const KEY_FILES: &[&str] = &[
    "CLAUDE.md",
    ".env",
    "docker-compose.yml",
    "compose.yml",
    "Makefile",
    "justfile",
    ".github/workflows",
    ".gitlab-ci.yml",
];

pub async fn detect_key_files(path: &Path) -> Vec<String> {
    let mut found = Vec::new();

    for key_file in KEY_FILES {
        let full_path = path.join(key_file);
        if tokio::fs::metadata(&full_path).await.is_ok() {
            found.push(key_file.to_string());
        }
    }

    found
}

// ── PR/MR detection ─────────────────────────────────────────────────────────

pub async fn detect_open_pr(path: &Path, platform: &str, branch: &str) -> Option<PrInfo> {
    let timeout = Duration::from_secs(5);

    match platform {
        "github" => detect_github_pr(path, branch, timeout).await,
        "gitlab" => detect_gitlab_mr(path, branch, timeout).await,
        _ => None,
    }
}

async fn detect_github_pr(path: &Path, branch: &str, timeout: Duration) -> Option<PrInfo> {
    let output = run_cmd(
        "gh",
        &[
            "pr",
            "view",
            branch,
            "--json",
            "number,title,state,url,isDraft",
        ],
        path,
        timeout,
    )
    .await?;

    let parsed: serde_json::Value = serde_json::from_str(&output).ok()?;
    Some(PrInfo {
        number: parsed.get("number")?.as_u64()?,
        title: parsed
            .get("title")?
            .as_str()
            .unwrap_or_default()
            .to_string(),
        url: parsed.get("url")?.as_str().unwrap_or_default().to_string(),
        state: parsed
            .get("state")?
            .as_str()
            .unwrap_or_default()
            .to_lowercase(),
        is_draft: parsed
            .get("isDraft")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    })
}

async fn detect_gitlab_mr(path: &Path, branch: &str, timeout: Duration) -> Option<PrInfo> {
    let output = run_cmd(
        "glab",
        &["mr", "view", branch, "--output", "json"],
        path,
        timeout,
    )
    .await?;

    let parsed: serde_json::Value = serde_json::from_str(&output).ok()?;
    Some(PrInfo {
        number: parsed.get("iid")?.as_u64()?,
        title: parsed
            .get("title")?
            .as_str()
            .unwrap_or_default()
            .to_string(),
        url: parsed
            .get("web_url")?
            .as_str()
            .unwrap_or_default()
            .to_string(),
        state: parsed
            .get("state")?
            .as_str()
            .unwrap_or_default()
            .to_lowercase(),
        is_draft: parsed
            .get("draft")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    })
}
