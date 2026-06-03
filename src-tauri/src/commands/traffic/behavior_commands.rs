use std::fs;
use std::path::{Path, PathBuf};

use arboard::Clipboard;
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

use super::TrafficAnalysisState;
use crate::commands::command_response_support::CommandResponse;
use crate::services::system_agents::{
    BrowserShellFrame, BrowserShellSession, BrowserShellWriteRequest,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyTrafficBehaviorExtensionResult {
    pub copied_directory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficBrowserShellFrameList {
    pub session_id: String,
    pub frames: Vec<BrowserShellFrame>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueTrafficBrowserShellWritePayload {
    pub session_id: String,
    pub input_text: String,
    #[serde(default = "default_requires_approval")]
    pub requires_approval: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespondTrafficBrowserShellWritePayload {
    pub request_id: String,
    pub allowed: bool,
}

fn default_requires_approval() -> bool {
    true
}

fn resolve_extension_directory(app: &tauri::AppHandle) -> Option<(PathBuf, bool)> {
    let source_dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../extensions/chrome-behavior-capture");

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

fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst)
        .map_err(|error| format!("Failed to create destination directory: {error}"))?;

    for entry in
        fs::read_dir(src).map_err(|error| format!("Failed to read source directory: {error}"))?
    {
        let entry = entry.map_err(|error| format!("Failed to read directory entry: {error}"))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("Failed to read entry type: {error}"))?;
        let from = entry.path();
        let to = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            fs::copy(&from, &to).map_err(|error| {
                format!(
                    "Failed to copy '{}' to '{}': {error}",
                    from.display(),
                    to.display()
                )
            })?;
        }
    }

    Ok(())
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
pub async fn copy_traffic_behavior_extension_to_directory(
    app: tauri::AppHandle,
    target_directory: String,
) -> Result<CommandResponse<CopyTrafficBehaviorExtensionResult>, String> {
    let (extension_directory, _) = resolve_extension_directory(&app)
        .ok_or_else(|| "Failed to resolve browser extension directory".to_string())?;

    let target_root = PathBuf::from(target_directory.trim());
    if target_directory.trim().is_empty() {
        return Ok(CommandResponse::err(
            "Target directory is required".to_string(),
        ));
    }

    if !target_root.exists() {
        return Ok(CommandResponse::err(
            "Target directory does not exist".to_string(),
        ));
    }

    if !target_root.is_dir() {
        return Ok(CommandResponse::err(
            "Target path is not a directory".to_string(),
        ));
    }

    let destination = target_root.join("chrome-behavior-capture");
    if destination.exists() {
        fs::remove_dir_all(&destination).map_err(|error| {
            format!(
                "Failed to replace existing extension directory '{}': {error}",
                destination.display()
            )
        })?;
    }

    copy_dir_all(&extension_directory, &destination)?;

    Ok(CommandResponse::ok(CopyTrafficBehaviorExtensionResult {
        copied_directory: destination.to_string_lossy().into_owned(),
    }))
}

#[tauri::command]
pub async fn read_traffic_clipboard_text() -> Result<CommandResponse<String>, String> {
    let mut clipboard =
        Clipboard::new().map_err(|error| format!("Failed to access clipboard: {error}"))?;
    let text = clipboard
        .get_text()
        .map_err(|error| format!("Failed to read clipboard text: {error}"))?;
    Ok(CommandResponse::ok(text))
}

#[tauri::command]
pub async fn list_traffic_browser_shell_sessions(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<Vec<BrowserShellSession>>, String> {
    let store = state.get_browser_shell_store();
    let sessions = store.read().await.list_sessions();
    Ok(CommandResponse::ok(sessions))
}

#[tauri::command]
pub async fn get_traffic_browser_shell_frames(
    state: State<'_, TrafficAnalysisState>,
    session_id: String,
    limit: Option<usize>,
) -> Result<CommandResponse<TrafficBrowserShellFrameList>, String> {
    let normalized_session_id = session_id.trim();
    if normalized_session_id.is_empty() {
        return Ok(CommandResponse::err("Session id is required"));
    }

    let store = state.get_browser_shell_store();
    let frames = store
        .read()
        .await
        .list_frames(normalized_session_id, limit.unwrap_or(50));
    Ok(CommandResponse::ok(TrafficBrowserShellFrameList {
        session_id: normalized_session_id.to_string(),
        frames,
    }))
}

#[tauri::command]
pub async fn queue_traffic_browser_shell_write(
    state: State<'_, TrafficAnalysisState>,
    payload: QueueTrafficBrowserShellWritePayload,
) -> Result<CommandResponse<BrowserShellWriteRequest>, String> {
    let store = state.get_browser_shell_store();
    let request = store.write().await.enqueue_write(
        &payload.session_id,
        &payload.input_text,
        payload.requires_approval,
    );
    match request {
        Ok(data) => Ok(CommandResponse::ok(data)),
        Err(error) => Ok(CommandResponse::err(error)),
    }
}

#[tauri::command]
pub async fn respond_traffic_browser_shell_write(
    state: State<'_, TrafficAnalysisState>,
    payload: RespondTrafficBrowserShellWritePayload,
) -> Result<CommandResponse<BrowserShellWriteRequest>, String> {
    let store = state.get_browser_shell_store();
    let request = store
        .write()
        .await
        .respond_write_request(&payload.request_id, payload.allowed);
    match request {
        Ok(data) => Ok(CommandResponse::ok(data)),
        Err(error) => Ok(CommandResponse::err(error)),
    }
}

#[tauri::command]
pub async fn list_traffic_browser_shell_write_requests(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<Vec<BrowserShellWriteRequest>>, String> {
    let store = state.get_browser_shell_store();
    let requests = store.read().await.list_write_requests();
    Ok(CommandResponse::ok(requests))
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
