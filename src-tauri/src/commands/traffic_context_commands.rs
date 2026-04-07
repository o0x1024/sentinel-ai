use serde::{Deserialize, Serialize};
use tauri::State;

use crate::commands::traffic_analysis_commands::{CommandResponse, TrafficAnalysisState};
use crate::services::system_agents::{
    TrafficContextExtractionSettings, TRAFFIC_CONTEXT_EXTRACTION_SETTINGS_KEY,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetTrafficContextExtractionSettingsPayload {
    pub settings: TrafficContextExtractionSettings,
}

#[tauri::command]
pub async fn get_traffic_context_extraction_settings(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<TrafficContextExtractionSettings>, String> {
    let db = state.get_db_service();
    let settings = match db
        .load_proxy_config(TRAFFIC_CONTEXT_EXTRACTION_SETTINGS_KEY)
        .await
    {
        Ok(Some(raw)) => serde_json::from_str::<TrafficContextExtractionSettings>(&raw)
            .map(|value| value.sanitized())
            .unwrap_or_default(),
        Ok(None) => TrafficContextExtractionSettings::default(),
        Err(error) => {
            return Err(format!(
                "Failed to load traffic context extraction settings: {error}"
            ));
        }
    };

    {
        let shared_settings = state.get_context_extraction_settings();
        let mut shared = shared_settings.write().await;
        *shared = settings.clone();
    }

    Ok(CommandResponse::ok(settings))
}

#[tauri::command]
pub async fn set_traffic_context_extraction_settings(
    state: State<'_, TrafficAnalysisState>,
    payload: SetTrafficContextExtractionSettingsPayload,
) -> Result<CommandResponse<TrafficContextExtractionSettings>, String> {
    let db = state.get_db_service();
    let settings = payload.settings.sanitized();
    let raw = serde_json::to_string(&settings)
        .map_err(|error| format!("Failed to serialize traffic context settings: {error}"))?;

    db.save_proxy_config(TRAFFIC_CONTEXT_EXTRACTION_SETTINGS_KEY, &raw)
        .await
        .map_err(|error| format!("Failed to save traffic context settings: {error}"))?;

    {
        let shared_settings = state.get_context_extraction_settings();
        let mut shared = shared_settings.write().await;
        *shared = settings.clone();
    }

    Ok(CommandResponse::ok(settings))
}
