use std::process::Stdio;

use anyhow::{bail, Result};
use tokio::process::Command;
use tokio::signal;

/// Run the unified dev server: Vite (HMR) + cargo-watch (Rust auto-rebuild).
pub fn run(port: u16, vite_port: u16, open: bool) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(run_dev(port, vite_port, open))
}

async fn run_dev(port: u16, vite_port: u16, open: bool) -> Result<()> {
    // Check prerequisites
    check_cargo_watch().await?;
    check_web_dir()?;

    print_banner(port, vite_port);

    // Spawn Vite dev server
    let mut vite = Command::new("npm")
        .args(["run", "dev", "--", "--port", &vite_port.to_string()])
        .current_dir("web")
        .env("HIVE_PORT", port.to_string())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to start Vite dev server: {e}"))?;

    // Spawn cargo-watch for Rust auto-rebuild
    let cargo_run_cmd = format!("run -- monitor --web --port {port}");
    let mut cargo = Command::new("cargo")
        .args(["watch", "-x", &cargo_run_cmd, "-w", "src/", "-i", "web/"])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to start cargo-watch: {e}"))?;

    if open {
        let url = format!("http://localhost:{vite_port}");
        let _ = Command::new("open").arg(&url).spawn();
    }

    // Wait for either process to exit or Ctrl+C
    tokio::select! {
        status = vite.wait() => {
            eprintln!("\nVite dev server exited: {}", status?);
            let _ = cargo.kill().await;
        }
        status = cargo.wait() => {
            eprintln!("\ncargo-watch exited: {}", status?);
            let _ = vite.kill().await;
        }
        _ = signal::ctrl_c() => {
            eprintln!("\n\nShutting down...");
            let _ = vite.kill().await;
            let _ = cargo.kill().await;
        }
    }

    Ok(())
}

async fn check_cargo_watch() -> Result<()> {
    let output = Command::new("cargo")
        .args(["watch", "--version"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;

    match output {
        Ok(status) if status.success() => Ok(()),
        _ => {
            bail!(
                "cargo-watch is not installed.\n\n\
                 Install it with:\n\
                 \n\
                   cargo install cargo-watch\n"
            );
        }
    }
}

fn check_web_dir() -> Result<()> {
    if !std::path::Path::new("web/package.json").exists() {
        bail!(
            "Cannot find web/package.json.\n\
             Run this command from the Hive project root."
        );
    }
    Ok(())
}

fn print_banner(port: u16, vite_port: u16) {
    eprintln!();
    eprintln!("  \x1b[33m\x1b[1mHive Dev Server\x1b[0m");
    eprintln!();
    eprintln!("  Frontend:  \x1b[36mhttp://localhost:{vite_port}\x1b[0m   (Vite HMR)");
    eprintln!("  API:       \x1b[36mhttp://localhost:{port}\x1b[0m   (Axum)");
    eprintln!();
    eprintln!("  Watching src/**/*.rs for changes...");
    eprintln!("  Press Ctrl+C to stop.");
    eprintln!();
}
