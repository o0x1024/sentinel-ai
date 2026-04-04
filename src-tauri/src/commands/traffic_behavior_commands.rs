use std::path::{Path, PathBuf};

use arboard::Clipboard;
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

use crate::commands::traffic_analysis_commands::{CommandResponse, TrafficAnalysisState};
use crate::services::system_agents::{
    TrafficBehaviorSignalSettings, TRAFFIC_BEHAVIOR_EXTENSION_BRIDGE_PORT,
    TRAFFIC_BEHAVIOR_SIGNAL_SETTINGS_KEY,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetTrafficBehaviorSignalSettingsPayload {
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficBehaviorExtensionInstallation {
    pub bridge_url: String,
    pub extension_directory: String,
    pub directory_source: String,
    pub bundled_with_app: bool,
}

fn resolve_extension_directory(app: &tauri::AppHandle) -> Option<(PathBuf, bool)> {
    let source_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../extensions/chrome-behavior-capture");

    let mut bundled_candidates = Vec::new();
    if let Ok(resource_dir) = app.path().resource_dir() {
        bundled_candidates.push(resource_dir.join("chrome-behavior-capture"));
        bundled_candidates.push(resource_dir.join("extensions/chrome-behavior-capture"));
    }

    for candidate in bundled_candidates {
        if candidate.exists() {
            return Some((candidate, true));
        }
    }

    if source_dir.exists() {
        return Some((source_dir, false));
    }

    None
}

#[tauri::command]
pub async fn get_traffic_behavior_signal_settings(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<TrafficBehaviorSignalSettings>, String> {
    let db = state.get_db_service();
    let persisted = match db
        .load_proxy_config(TRAFFIC_BEHAVIOR_SIGNAL_SETTINGS_KEY)
        .await
    {
        Ok(Some(raw)) => serde_json::from_str::<TrafficBehaviorSignalSettings>(&raw)
            .map(|value| value.sanitized())
            .unwrap_or_default(),
        _ => TrafficBehaviorSignalSettings::default(),
    };

    let settings = {
        let shared_settings = state.get_behavior_signal_settings();
        let mut shared = shared_settings.write().await;
        if shared.browser_extension_connected
            || shared.browser_extension_last_seen_at.is_some()
            || shared.mode != TrafficBehaviorSignalSettings::default().mode
        {
            let refreshed = shared.clone().sanitized();
            *shared = refreshed.clone();
            refreshed
        } else {
            *shared = persisted.clone();
            persisted
        }
    };

    Ok(CommandResponse::ok(settings))
}

#[tauri::command]
pub async fn get_traffic_behavior_extension_installation(
    app: tauri::AppHandle,
) -> Result<CommandResponse<TrafficBehaviorExtensionInstallation>, String> {
    let bridge_url = format!("http://127.0.0.1:{TRAFFIC_BEHAVIOR_EXTENSION_BRIDGE_PORT}");
    let (extension_directory, bundled_with_app) = resolve_extension_directory(&app)
        .ok_or_else(|| "Failed to resolve browser extension directory".to_string())?;

    let installation = TrafficBehaviorExtensionInstallation {
        bridge_url,
        extension_directory: extension_directory.to_string_lossy().into_owned(),
        directory_source: if bundled_with_app {
            "bundled".to_string()
        } else {
            "workspace".to_string()
        },
        bundled_with_app,
    };

    Ok(CommandResponse::ok(installation))
}

#[tauri::command]
pub async fn read_traffic_clipboard_text() -> Result<CommandResponse<String>, String> {
    let mut clipboard = Clipboard::new()
        .map_err(|error| format!("Failed to access clipboard: {error}"))?;
    let text = clipboard
        .get_text()
        .map_err(|error| format!("Failed to read clipboard text: {error}"))?;
    Ok(CommandResponse::ok(text))
}

#[tauri::command]
pub async fn set_traffic_behavior_signal_settings(
    state: State<'_, TrafficAnalysisState>,
    payload: SetTrafficBehaviorSignalSettingsPayload,
) -> Result<CommandResponse<TrafficBehaviorSignalSettings>, String> {
    let db = state.get_db_service();
    let current = {
        let shared_settings = state.get_behavior_signal_settings();
        let current = shared_settings.read().await.clone();
        current
    };
    let settings = TrafficBehaviorSignalSettings {
        mode: payload.mode,
        browser_extension_connected: current.browser_extension_connected,
        browser_extension_last_seen_at: current.browser_extension_last_seen_at,
    }
    .sanitized();

    let raw = serde_json::to_string(&settings)
        .map_err(|error| format!("Failed to serialize behavior signal settings: {error}"))?;

    db.save_proxy_config(TRAFFIC_BEHAVIOR_SIGNAL_SETTINGS_KEY, &raw)
        .await
        .map_err(|error| format!("Failed to save behavior signal settings: {error}"))?;

    {
        let shared_settings = state.get_behavior_signal_settings();
        let mut shared = shared_settings.write().await;
        *shared = settings.clone();
    }

    Ok(CommandResponse::ok(settings))
}
