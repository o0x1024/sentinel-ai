use std::sync::Arc;

use tauri::State;

use crate::commands::command_response_support::CommandResponse;
use crate::services::{
    run_plugin_fix_task, AiServiceManager, PluginFixTaskRequest, PluginFixTaskResult,
    SystemAgentRuntime,
};

#[tauri::command]
pub async fn fix_plugin_with_ai_task(
    runtime: State<'_, Arc<SystemAgentRuntime>>,
    ai_manager: State<'_, Arc<AiServiceManager>>,
    request: PluginFixTaskRequest,
) -> Result<CommandResponse<PluginFixTaskResult>, String> {
    let result = run_plugin_fix_task(runtime.inner(), ai_manager.inner(), request)
        .await
        .map_err(|e| e.to_string())?;
    Ok(CommandResponse::ok(result))
}

#[tauri::command]
pub async fn fix_plugin_with_system_agent(
    runtime: State<'_, Arc<SystemAgentRuntime>>,
    ai_manager: State<'_, Arc<AiServiceManager>>,
    request: PluginFixTaskRequest,
) -> Result<CommandResponse<PluginFixTaskResult>, String> {
    // Compatibility alias. New callers should use `fix_plugin_with_ai_task`.
    fix_plugin_with_ai_task(runtime, ai_manager, request).await
}
