mod detectors;
mod events;

use std::path::Path;
use std::time::Duration;

use super::types::ProjectContext;

pub use detectors::{
    detect_git, detect_key_files, detect_open_pr, detect_platform, detect_runtimes,
};
pub use events::detect_with_events;

/// Run a shell command with a timeout, returning stdout if successful.
pub(crate) async fn run_cmd(
    cmd: &str,
    args: &[&str],
    cwd: &Path,
    timeout: Duration,
) -> Option<String> {
    let result = tokio::time::timeout(
        timeout,
        tokio::process::Command::new(cmd)
            .args(args)
            .current_dir(cwd)
            .output(),
    )
    .await;

    match result {
        Ok(Ok(output)) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if stdout.is_empty() {
                None
            } else {
                Some(stdout)
            }
        }
        _ => None,
    }
}

/// Full detection: git, runtimes, key files, open PR — combined.
pub async fn detect_all(path: &Path) -> ProjectContext {
    let (git, runtimes, key_files) = tokio::join!(
        detect_git(path),
        detect_runtimes(path),
        detect_key_files(path)
    );

    let open_pr = if let Some(ref git_ctx) = git {
        detect_open_pr(path, &git_ctx.platform, &git_ctx.branch).await
    } else {
        None
    };

    ProjectContext {
        git,
        runtimes,
        key_files,
        open_pr,
    }
}
