use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::commands::command_response_support::CommandResponse;
use crate::services::{
    execute_plugin_authoring, AiServiceManager, PluginAuthoringRequest, PluginAuthoringResult,
    SystemAgentRuntime,
};
use crate::TrafficAnalysisState;

#[tauri::command(rename_all = "camelCase")]
pub async fn plugin_authoring_execute(
    app_handle: AppHandle,
    traffic_state: State<'_, TrafficAnalysisState>,
    ai_manager: State<'_, Arc<AiServiceManager>>,
    runtime: State<'_, Arc<SystemAgentRuntime>>,
    request: PluginAuthoringRequest,
) -> Result<CommandResponse<PluginAuthoringResult>, String> {
    let result = execute_plugin_authoring(
        &app_handle,
        &traffic_state,
        ai_manager.inner(),
        runtime.inner(),
        request,
    )
    .await
    .map_err(|error| error.to_string())?;

    Ok(CommandResponse::ok(result))
}
