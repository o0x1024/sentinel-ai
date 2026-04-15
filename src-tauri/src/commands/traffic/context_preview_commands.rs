use serde::{Deserialize, Serialize};
use tauri::State;

use super::TrafficAnalysisState;
use crate::commands::command_response_support::CommandResponse;
use crate::services::system_agents::{
    preview_traffic_context_extraction_changes, TrafficContextExtractionPreviewResponse,
    TrafficContextExtractionSettings,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewTrafficContextExtractionPayload {
    pub request_ids: Vec<i64>,
    pub current_settings: TrafficContextExtractionSettings,
    pub preview_settings: TrafficContextExtractionSettings,
    pub sample_limit: Option<usize>,
}

#[tauri::command]
pub async fn preview_traffic_context_extraction_changes_command(
    state: State<'_, TrafficAnalysisState>,
    payload: PreviewTrafficContextExtractionPayload,
) -> Result<CommandResponse<TrafficContextExtractionPreviewResponse>, String> {
    let request_ids = payload
        .request_ids
        .into_iter()
        .take(500)
        .collect::<Vec<_>>();
    let sample_limit = payload.sample_limit.unwrap_or(6).clamp(1, 12);
    let cache = state.get_history_cache();
    let mut records = Vec::new();

    for request_id in request_ids {
        if let Some(record) = cache.get_http_request_by_id(request_id).await {
            records.push(record);
        }
    }

    let response = preview_traffic_context_extraction_changes(
        &records,
        &payload.current_settings.sanitized(),
        &payload.preview_settings.sanitized(),
        sample_limit,
    );

    Ok(CommandResponse::ok(response))
}
