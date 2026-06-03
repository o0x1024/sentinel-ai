use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
pub struct PathReport {
    pub data_dir: String,
    pub config_path: String,
    pub logs_dir: String,
    pub runs_dir: String,
}

pub fn resolve_paths() -> PathReport {
    let data_dir = sentinel_state_dir();

    PathReport {
        config_path: data_dir.join("arena-config.json").display().to_string(),
        logs_dir: data_dir.join("logs").display().to_string(),
        runs_dir: data_dir.join("runs").display().to_string(),
        data_dir: data_dir.display().to_string(),
    }
}

pub fn sentinel_state_dir() -> PathBuf {
    if let Ok(explicit) = std::env::var("SENTINEL_STATE_DIR") {
        let trimmed = explicit.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("sentinel-ai")
}
