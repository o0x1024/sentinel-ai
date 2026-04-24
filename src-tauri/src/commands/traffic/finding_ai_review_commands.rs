use std::sync::Arc;

use tauri::State;

use crate::commands::command_response_support::CommandResponse;
use crate::services::finding_ai_review::{run_finding_ai_review, FindingAiReviewResult};
use crate::services::AiServiceManager;

use super::TrafficAnalysisState;

#[tauri::command]
pub async fn review_finding_with_ai(
    state: State<'_, TrafficAnalysisState>,
    ai_manager: State<'_, Arc<AiServiceManager>>,
    finding_id: String,
) -> Result<CommandResponse<FindingAiReviewResult>, String> {
    let db_service = state.get_db_service();
    let result = run_finding_ai_review(db_service.as_ref(), ai_manager.inner().as_ref(), &finding_id)
        .await
        .map_err(|error| error.to_string())?;
    Ok(CommandResponse::ok(result))
}
