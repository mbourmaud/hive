use std::process::Command;

pub fn stop_drone(name: &str) -> anyhow::Result<String> {
    let output = Command::new("hive").args(["stop", name]).output()?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn clean_drone(name: &str) -> anyhow::Result<String> {
    let output = Command::new("hive")
        .args(["clean", name, "--force"])
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn view_logs(name: &str) -> anyhow::Result<String> {
    let output = Command::new("hive")
        .args(["logs", name, "--lines", "50"])
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
