use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PlanInfo {
    pub name: String,
    pub path: String,
}

#[tauri::command]
pub fn stop_drone(name: String) -> Result<String, String> {
    hive_lib::commands::kill_clean::kill_quiet(name.clone())
        .map(|_| format!("Drone '{}' stopped", name))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clean_drone(name: String) -> Result<String, String> {
    hive_lib::commands::kill_clean::clean_quiet(name.clone())
        .map(|_| format!("Drone '{}' cleaned", name))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_plans() -> Result<Vec<PlanInfo>, String> {
    let plans_dir = std::path::PathBuf::from(".hive/plans");
    if !plans_dir.exists() {
        return Ok(vec![]);
    }
    let mut plans = Vec::new();
    let entries = std::fs::read_dir(&plans_dir).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "md" || ext == "json") {
            let name = path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            plans.push(PlanInfo {
                name,
                path: path.to_string_lossy().to_string(),
            });
        }
    }
    Ok(plans)
}

#[tauri::command]
pub fn start_drone(name: String, plan: String, model: String, mode: String) -> Result<String, String> {
    // TODO: integrate with hive_lib::commands::start
    Err(format!("start_drone not yet implemented for '{}' (plan={}, model={}, mode={})", name, plan, model, mode))
}
