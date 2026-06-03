use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;

use super::TrafficAnalysisState;
use crate::commands::command_response_support::CommandResponse;
use crate::services::system_agents::{
    recommend_traffic_context_dictionary_candidates,
    RecommendTrafficContextDictionaryCandidatesResponse,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendTrafficContextDictionaryCandidatesPayload {
    pub request_ids: Vec<i64>,
    pub include_behavior_context: Option<bool>,
    pub max_candidates_per_category: Option<usize>,
}

#[tauri::command]
pub async fn recommend_traffic_context_dictionary_candidates_command(
    state: State<'_, TrafficAnalysisState>,
    payload: RecommendTrafficContextDictionaryCandidatesPayload,
) -> Result<CommandResponse<RecommendTrafficContextDictionaryCandidatesResponse>, String> {
    let request_ids = payload
        .request_ids
        .into_iter()
        .take(500)
        .collect::<Vec<_>>();
    let max_candidates_per_category = payload
        .max_candidates_per_category
        .unwrap_or(12)
        .clamp(1, 50);

    let cache = state.get_history_cache();
    let mut records = Vec::new();
    let mut skipped_request_count = 0usize;

    for request_id in request_ids {
        match cache.get_http_request_by_id(request_id).await {
            Some(record) => records.push(record),
            None => skipped_request_count += 1,
        }
    }

    let existing_settings = state.get_context_extraction_settings().read().await.clone();
    let include_behavior_context = payload.include_behavior_context.unwrap_or(true);
    let behavior_contexts = if include_behavior_context {
        let store = state.get_behavior_extension_events();
        let store = store.read().await;
        records
            .iter()
            .filter_map(|record| {
                store
                    .build_context_for_request(record)
                    .map(|context| (record.id, context))
            })
            .collect::<HashMap<_, _>>()
    } else {
        HashMap::new()
    };
    let mut response = recommend_traffic_context_dictionary_candidates(
        &records,
        Some(&behavior_contexts),
        existing_settings,
        max_candidates_per_category,
    );
    response.skipped_request_count = skipped_request_count;

    Ok(CommandResponse::ok(response))
}
