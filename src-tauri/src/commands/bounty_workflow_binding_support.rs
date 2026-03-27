use chrono::Utc;
use sentinel_db::DatabaseService;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWorkflowBindingRequest {
    pub scope_id: Option<String>,
    pub is_enabled: bool,
    pub auto_run_on_change: bool,
    pub trigger_conditions: Option<serde_json::Value>,
    pub schedule_cron: Option<String>,
}

#[tauri::command]
pub async fn bounty_update_workflow_binding(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
    request: UpdateWorkflowBindingRequest,
) -> Result<bool, String> {
    let mut binding = db_service
        .get_bounty_workflow_binding(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Workflow binding not found".to_string())?;

    binding.scope_id = request.scope_id.and_then(|scope_id| {
        let trimmed = scope_id.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    });
    binding.is_enabled = request.is_enabled;
    binding.auto_run_on_change = request.auto_run_on_change;
    binding.trigger_conditions_json = request
        .trigger_conditions
        .map(|conditions| serde_json::to_string(&conditions).unwrap_or_default());
    binding.schedule_cron = request.schedule_cron.and_then(|schedule_cron| {
        let trimmed = schedule_cron.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    });
    binding.updated_at = Utc::now().to_rfc3339();

    db_service
        .update_bounty_workflow_binding(&binding)
        .await
        .map_err(|e| e.to_string())
}
