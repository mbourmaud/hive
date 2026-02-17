//! Gather fresh project context (git state) with a 30s TTL cache.
//! Injected into the system prompt before each API call so Claude
//! always has current branch/dirty-file info without wasting tool calls.

use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tokio::process::Command;

const TTL: Duration = Duration::from_secs(30);

struct CachedContext {
    text: String,
    created: Instant,
}

static CACHE: Mutex<Option<CachedContext>> = Mutex::new(None);

/// Return a compact `<project_context>` block (< 200 tokens), or empty string on error.
pub async fn gather_project_context(cwd: &Path) -> String {
    // Check cache (lock held only briefly)
    {
        let guard = CACHE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(ref cached) = *guard {
            if cached.created.elapsed() < TTL {
                return cached.text.clone();
            }
        }
    }

    // Run git commands outside the lock
    let text = match build_context(cwd).await {
        Some(ctx) => ctx,
        None => return String::new(),
    };

    // Store in cache
    {
        let mut guard = CACHE.lock().unwrap_or_else(|e| e.into_inner());
        *guard = Some(CachedContext {
            text: text.clone(),
            created: Instant::now(),
        });
    }

    text
}

/// Invalidate the cache (e.g. after a commit).
#[cfg(test)]
pub fn invalidate_cache() {
    let mut guard = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    *guard = None;
}

async fn build_context(cwd: &Path) -> Option<String> {
    let branch = run_git(cwd, &["rev-parse", "--abbrev-ref", "HEAD"]).await?;
    let status = run_git(cwd, &["status", "--porcelain"])
        .await
        .unwrap_or_default();
    let log = run_git(cwd, &["log", "--oneline", "-5"])
        .await
        .unwrap_or_default();
    let diff_stat = run_git(cwd, &["diff", "--stat", "--no-color"])
        .await
        .unwrap_or_default();

    let mut out = String::with_capacity(512);
    out.push_str("\n\n<project_context>\n");
    out.push_str(&format!("Branch: {branch}\n"));

    if !status.is_empty() {
        let lines: Vec<&str> = status.lines().collect();
        out.push_str(&format!("Dirty files ({}):\n", lines.len()));
        for line in lines.iter().take(15) {
            out.push_str(&format!("  {line}\n"));
        }
        if lines.len() > 15 {
            out.push_str(&format!("  ... and {} more\n", lines.len() - 15));
        }
    } else {
        out.push_str("Working tree: clean\n");
    }

    if !diff_stat.is_empty() {
        // Last line of diff --stat is the summary (e.g. "3 files changed, 42 insertions(+)")
        if let Some(summary) = diff_stat.lines().last() {
            out.push_str(&format!("Uncommitted changes: {summary}\n"));
        }
    }

    if !log.is_empty() {
        out.push_str("Recent commits:\n");
        for line in log.lines().take(5) {
            out.push_str(&format!("  {line}\n"));
        }
    }

    out.push_str("</project_context>");
    Some(out)
}

async fn run_git(cwd: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_gather_returns_string_in_git_repo() {
        invalidate_cache();
        let cwd = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let ctx = gather_project_context(&cwd).await;
        assert!(ctx.contains("<project_context>"));
        assert!(ctx.contains("Branch:"));
    }

    #[tokio::test]
    async fn test_gather_returns_empty_outside_git() {
        invalidate_cache();
        let ctx = gather_project_context(Path::new("/tmp")).await;
        assert!(ctx.is_empty());
    }
}
