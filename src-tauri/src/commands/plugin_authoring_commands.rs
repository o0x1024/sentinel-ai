use std::sync::Arc;

use serde::Deserialize;
use tauri::{AppHandle, State};

use crate::commands::command_response_support::CommandResponse;
use crate::commands::monitor_config_support::validate_plugin_monitor_type;
use crate::generators::{
    parse_agent_plugin_definition, render_agent_plugin_definition, AgentPluginRenderContext,
};
use crate::services::{
    execute_plugin_authoring, AiServiceManager, PluginAuthoringRequest, PluginAuthoringResult,
    PluginMainCategory, SystemAgentRuntime,
};
use crate::TrafficAnalysisState;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderAgentPluginDefinitionRequest {
    pub definition: String,
    pub metadata: RenderAgentPluginMetadata,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderAgentPluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub category: String,
    pub main_category: String,
    pub monitor_type: Option<String>,
    #[serde(alias = "default_severity")]
    pub default_severity: String,
    pub description: String,
    pub tags_string: String,
}

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

#[tauri::command(rename_all = "camelCase")]
pub async fn render_agent_plugin_definition_command(
    request: RenderAgentPluginDefinitionRequest,
) -> Result<CommandResponse<String>, String> {
    match PluginMainCategory::parse(&request.metadata.main_category)? {
        PluginMainCategory::Agent | PluginMainCategory::Bounty => {}
        PluginMainCategory::Traffic | PluginMainCategory::Intruder => {
            return Ok(CommandResponse::ok(request.definition));
        }
    }

    let main_category = PluginMainCategory::parse(&request.metadata.main_category)?;
    let monitor_type = validate_plugin_monitor_type(
        main_category,
        request.metadata.monitor_type.clone(),
    )?;
    let definition =
        parse_agent_plugin_definition(&request.definition).map_err(|error| error.to_string())?;
    let context = AgentPluginRenderContext {
        plugin_id: request.metadata.id,
        name: request.metadata.name,
        version: request.metadata.version,
        author: request.metadata.author,
        main_category: request.metadata.main_category,
        plugin_business_category: request.metadata.category,
        monitor_type,
        default_severity: request.metadata.default_severity,
        tags: parse_tags(&request.metadata.tags_string),
        description: request.metadata.description,
    };
    let code =
        render_agent_plugin_definition(definition, &context).map_err(|error| error.to_string())?;

    Ok(CommandResponse::ok(code))
}

fn parse_tags(tags_string: &str) -> Vec<String> {
    let tags = tags_string
        .split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();

    if tags.is_empty() {
        vec!["ai-generated".to_string()]
    } else {
        tags
    }
}
