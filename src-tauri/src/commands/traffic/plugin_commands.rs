use sentinel_db::{Database, DatabaseService};
use sentinel_plugins::HttpTransaction;
use sentinel_traffic::{PluginMetadata, PluginRecord, PluginStatus};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use super::analysis_state_support::{resolve_plugin_registry_id, TrafficAnalysisState};
use crate::commands::command_response_support::CommandResponse;
use crate::commands::monitor_config_support::validate_plugin_monitor_type;
use crate::events::{emit_plugin_changed, PluginChangedEvent};
use crate::services::{
    ensure_plugin_allowed_for_current_tier, ensure_plugin_catalog_write_access,
    ensure_plugin_delete_access, filter_plugins_for_current_tier, load_plugin_default_inputs,
    merge_plugin_input_defaults, save_plugin_default_inputs, IntruderPluginCategory,
    PluginMainCategory,
};
use crate::utils::plugin_registry_cleanup::cleanup_removed_agent_plugins;

pub(crate) fn is_agent_tool_plugin_main_category(main_category: PluginMainCategory) -> bool {
    matches!(main_category, PluginMainCategory::Agent)
}

pub(crate) fn is_execution_plugin_main_category(main_category: PluginMainCategory) -> bool {
    matches!(
        main_category,
        PluginMainCategory::Agent | PluginMainCategory::Bounty | PluginMainCategory::Intruder
    )
}

pub(crate) fn is_traffic_scan_plugin_main_category(main_category: PluginMainCategory) -> bool {
    matches!(main_category, PluginMainCategory::Traffic)
}

fn ensure_intruder_plugin_kind(
    plugin: &PluginRecord,
    plugin_id: &str,
    expected_category: IntruderPluginCategory,
) -> Result<(), String> {
    let main_category = plugin.metadata.main_category;
    if main_category != PluginMainCategory::Intruder {
        return Err(format!("Plugin '{plugin_id}' is not an intruder plugin"));
    }

    let actual_category = plugin.metadata.category.intruder_category()?;
    if actual_category != expected_category {
        let capability = match expected_category {
            IntruderPluginCategory::PayloadGenerator => "payload generation",
            IntruderPluginCategory::PayloadProcessor => "payload processing",
            IntruderPluginCategory::RequestProcessor => "request processing",
        };
        return Err(format!(
            "Plugin '{}' does not implement intruder {}",
            plugin_id, capability
        ));
    }

    Ok(())
}

pub(crate) async fn refresh_active_agent_plugin_tools(
    db: &DatabaseService,
) -> Result<usize, String> {
    cleanup_removed_agent_plugins(db).await?;

    let tool_server = sentinel_tools::get_tool_server();
    tool_server.init_builtin_tools().await;
    sentinel_tools::plugin_adapter::refresh_plugin_tools(&tool_server).await;

    let active_plugins = db
        .get_active_agent_plugins()
        .await
        .map_err(|e| format!("Failed to query active agent plugins: {}", e))?;

    let mut plugin_metas = Vec::new();
    for plugin in active_plugins {
        let plugin_id = plugin.metadata.id.clone();
        if plugin_id.is_empty() {
            continue;
        }

        let code = db
            .get_plugin_code(&plugin_id)
            .await
            .map_err(|e| format!("Failed to load plugin code for {}: {}", plugin_id, e))?;

        let (input_schema, output_schema) = if let Some(code_str) = &code {
            let input_schema =
                sentinel_tools::plugin_adapter::PluginToolAdapter::get_input_schema_runtime(
                    code_str,
                    plugin.metadata.clone(),
                )
                .await;
            let output_schema =
                sentinel_tools::plugin_adapter::PluginToolAdapter::get_output_schema_runtime_optional(
                    code_str,
                    plugin.metadata.clone(),
                )
                .await;
            (input_schema, output_schema)
        } else {
            (
                serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
                None,
            )
        };
        let default_input = load_plugin_default_inputs(db, &plugin_id).await?;

        plugin_metas.push(sentinel_tools::plugin_adapter::PluginToolMeta {
            plugin_id,
            name: plugin.metadata.name.clone(),
            description: plugin
                .metadata
                .description
                .as_deref()
                .unwrap_or("Agent plugin tool")
                .to_string(),
            input_schema,
            output_schema,
            default_input,
            code,
        });
    }

    let count = plugin_metas.len();
    if count > 0 {
        sentinel_tools::plugin_adapter::load_plugin_tools_to_server(&tool_server, plugin_metas)
            .await;
    }

    Ok(count)
}

pub(crate) fn resolved_explicit_plugin_monitor_type(existing: Option<&PluginRecord>) -> Option<String> {
    existing
        .and_then(|record| record.metadata.monitor_type.clone())
        .filter(|value| !value.trim().is_empty())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntruderPluginSummary {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntruderPayloadGenerationResult {
    pub payloads: Vec<String>,
    pub output: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntruderPayloadProcessorResult {
    pub payload: Option<String>,
    pub skipped: bool,
    pub output: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntruderRequestProcessorResult {
    pub raw_request: String,
    pub output: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchToggleResult {
    pub enabled_count: usize,
    pub disabled_count: usize,
    pub failed_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestPluginResult {
    pub success: bool,
    pub message: Option<String>,
    pub findings: Option<Vec<TestFinding>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFinding {
    pub title: String,
    pub description: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedRunStat {
    pub run_index: u32,
    pub duration_ms: u128,
    pub findings: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedTestResult {
    pub plugin_id: String,
    pub success: bool,
    pub total_runs: u32,
    pub concurrency: u32,
    pub total_duration_ms: u128,
    pub avg_duration_ms: f64,
    pub total_findings: usize,
    pub unique_findings: usize,
    pub findings: Vec<TestFinding>,
    pub runs: Vec<AdvancedRunStat>,
    pub message: Option<String>,
    pub error: Option<String>,
    pub outputs: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTestResult {
    pub success: bool,
    pub message: Option<String>,
    pub output: Option<serde_json::Value>,
    pub execution_time_ms: u128,
    pub error: Option<String>,
}

fn extract_intruder_payloads(output: &serde_json::Value) -> Option<Vec<String>> {
    let payloads_value = output
        .get("data")
        .and_then(|data| data.get("payloads"))
        .or_else(|| output.get("payloads"))?;

    let payloads = payloads_value
        .as_array()?
        .iter()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect::<Vec<_>>();

    Some(payloads)
}

fn extract_intruder_payload(output: &serde_json::Value) -> Option<Option<String>> {
    if output
        .get("data")
        .and_then(|data| data.get("skip"))
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
        || output
            .get("skip")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
    {
        return Some(None);
    }

    output
        .get("data")
        .and_then(|data| data.get("payload"))
        .or_else(|| output.get("payload"))
        .map(|value| value.as_str().map(str::to_string))
}

fn extract_intruder_raw_request(output: &serde_json::Value) -> Option<String> {
    output
        .get("data")
        .and_then(|data| data.get("rawRequest"))
        .or_else(|| output.get("rawRequest"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

#[tauri::command]
pub async fn reload_plugin_in_pipeline(
    state: State<'_, TrafficAnalysisState>,
    plugin_id: String,
) -> Result<CommandResponse<String>, String> {
    tracing::info!("Reload plugin request: {}", plugin_id);

    if !*state.is_running.read().await {
        return Ok(CommandResponse::err("流量分析未运行".to_string()));
    }

    if let Some(scan_tx) = state.scan_tx.read().await.as_ref() {
        if let Err(e) = scan_tx.send(sentinel_traffic::ScanTask::ReloadPlugin(plugin_id.clone())) {
            tracing::error!("Failed to send reload task for plugin {}: {}", plugin_id, e);
            return Ok(CommandResponse::err(format!("发送重载任务失败: {}", e)));
        }

        tracing::info!("Sent reload task for plugin: {}", plugin_id);
        Ok(CommandResponse::ok(format!(
            "插件 {} 重载任务已发送",
            plugin_id
        )))
    } else {
        Ok(CommandResponse::err("扫描通道不可用".to_string()))
    }
}

#[tauri::command]
pub async fn enable_plugin(
    app: AppHandle,
    state: State<'_, TrafficAnalysisState>,
    plugin_id: String,
) -> Result<CommandResponse<()>, String> {
    ensure_plugin_allowed_for_current_tier(&plugin_id)?;

    let db = state.get_db_service();

    let (main_category, _) = match db.get_plugin_summary(&plugin_id).await {
        Ok(Some(info)) => info,
        Ok(None) => {
            return Ok(CommandResponse::err(format!(
                "Plugin not found: {}",
                plugin_id
            )))
        }
        Err(e) => {
            return Ok(CommandResponse::err(format!(
                "Failed to query plugin: {}",
                e
            )))
        }
    };
    let main_category = PluginMainCategory::parse(&main_category)?;

    if let Err(e) = db.update_plugin_enabled(&plugin_id, true).await {
        return Ok(CommandResponse::err(format!(
            "Failed to enable plugin: {}",
            e
        )));
    }

    tracing::info!("Plugin enabled in database: {}", plugin_id);

    if is_agent_tool_plugin_main_category(main_category) {
        if let Err(e) = refresh_active_agent_plugin_tools(db.as_ref()).await {
            tracing::warn!(
                "Failed to refresh active agent tools after enabling plugin {}: {}",
                plugin_id,
                e
            );
        }
    }

    let plugin_name = db
        .get_plugin_name(&plugin_id)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| plugin_id.clone());

    if is_traffic_scan_plugin_main_category(main_category) {
        let scan_tx = state.scan_tx.read().await;
        if let Some(ref tx) = *scan_tx {
            if let Err(e) = tx.send(sentinel_traffic::ScanTask::ReloadPlugin(plugin_id.clone())) {
                tracing::warn!("Failed to send reload plugin task for {}: {}", plugin_id, e);
            }
        }
    }

    emit_plugin_changed(
        &app,
        PluginChangedEvent {
            plugin_id,
            enabled: true,
            name: plugin_name,
        },
    );

    Ok(CommandResponse::ok(()))
}

#[tauri::command]
pub async fn disable_plugin(
    app: AppHandle,
    state: State<'_, TrafficAnalysisState>,
    plugin_id: String,
) -> Result<CommandResponse<()>, String> {
    ensure_plugin_allowed_for_current_tier(&plugin_id)?;

    let db = state.get_db_service();

    let (main_category, _) = match db.get_plugin_summary(&plugin_id).await {
        Ok(Some(info)) => info,
        Ok(None) => {
            return Ok(CommandResponse::err(format!(
                "Plugin not found: {}",
                plugin_id
            )))
        }
        Err(e) => {
            return Ok(CommandResponse::err(format!(
                "Failed to query plugin: {}",
                e
            )))
        }
    };
    let main_category = PluginMainCategory::parse(&main_category)?;

    if let Err(e) = db.update_plugin_enabled(&plugin_id, false).await {
        return Ok(CommandResponse::err(format!(
            "Failed to disable plugin: {}",
            e
        )));
    }

    tracing::info!("Plugin disabled in database: {}", plugin_id);

    if is_agent_tool_plugin_main_category(main_category) {
        if let Err(e) = refresh_active_agent_plugin_tools(db.as_ref()).await {
            tracing::warn!(
                "Failed to refresh active agent tools after disabling plugin {}: {}",
                plugin_id,
                e
            );
        }
    }

    let plugin_name = db
        .get_plugin_name(&plugin_id)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| plugin_id.clone());

    if is_traffic_scan_plugin_main_category(main_category) {
        let scan_tx = state.scan_tx.read().await;
        if let Some(ref tx) = *scan_tx {
            if let Err(e) = tx.send(sentinel_traffic::ScanTask::RemovePlugin(plugin_id.clone())) {
                tracing::warn!("Failed to send remove plugin task for {}: {}", plugin_id, e);
            }
        }
    }

    emit_plugin_changed(
        &app,
        PluginChangedEvent {
            plugin_id,
            enabled: false,
            name: plugin_name,
        },
    );

    Ok(CommandResponse::ok(()))
}

#[tauri::command]
pub async fn list_plugins(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<Vec<PluginRecord>>, String> {
    let plugins = filter_plugins_for_current_tier(state.list_plugins_internal().await?, |plugin| {
        plugin.metadata.id.as_str()
    });
    Ok(CommandResponse::ok(plugins))
}

#[tauri::command(rename_all = "camelCase")]
pub async fn get_plugin_default_input_config(
    state: State<'_, TrafficAnalysisState>,
    plugin_id: String,
) -> Result<CommandResponse<serde_json::Value>, String> {
    ensure_plugin_allowed_for_current_tier(&plugin_id)?;

    let db = state.get_db_service();
    let resolved_plugin_id = resolve_plugin_registry_id(db.as_ref(), &plugin_id)
        .await?
        .unwrap_or_else(|| plugin_id.clone());
    let config = load_plugin_default_inputs(db.as_ref(), &resolved_plugin_id).await?;

    Ok(CommandResponse::ok(config))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetPluginDefaultInputConfigPayload {
    pub plugin_id: String,
    pub config: serde_json::Value,
}

#[tauri::command(rename_all = "camelCase")]
pub async fn set_plugin_default_input_config(
    state: State<'_, TrafficAnalysisState>,
    payload: SetPluginDefaultInputConfigPayload,
) -> Result<CommandResponse<serde_json::Value>, String> {
    ensure_plugin_allowed_for_current_tier(&payload.plugin_id)?;

    let db = state.get_db_service();
    let resolved_plugin_id = resolve_plugin_registry_id(db.as_ref(), &payload.plugin_id)
        .await?
        .unwrap_or_else(|| payload.plugin_id.clone());
    let saved =
        save_plugin_default_inputs(db.as_ref(), &resolved_plugin_id, &payload.config).await?;

    refresh_active_agent_plugin_tools(db.as_ref()).await?;

    Ok(CommandResponse::ok(saved))
}

#[tauri::command(rename_all = "camelCase")]
pub async fn intruder_list_plugins(
    state: State<'_, TrafficAnalysisState>,
    category: Option<String>,
) -> Result<CommandResponse<Vec<IntruderPluginSummary>>, String> {
    let category_filter = category
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let plugins = state
        .list_plugins_internal()
        .await?
        .into_iter()
        .filter(|plugin| plugin.status == PluginStatus::Enabled)
        .filter(|plugin| matches!(plugin.metadata.main_category, PluginMainCategory::Intruder))
        .filter(|plugin| {
            category_filter
                .map(|expected| plugin.metadata.category.as_str() == expected)
                .unwrap_or(true)
        })
        .map(|plugin| IntruderPluginSummary {
            id: plugin.metadata.id,
            name: plugin.metadata.name,
            category: plugin.metadata.category.to_string(),
            description: plugin.metadata.description,
        })
        .collect::<Vec<_>>();

    Ok(CommandResponse::ok(plugins))
}

#[tauri::command(rename_all = "camelCase")]
pub async fn intruder_generate_payloads(
    state: State<'_, TrafficAnalysisState>,
    plugin_id: String,
    input: serde_json::Value,
) -> Result<CommandResponse<IntruderPayloadGenerationResult>, String> {
    let db = state.get_db_service();
    let resolved_plugin_id = resolve_plugin_registry_id(db.as_ref(), &plugin_id)
        .await?
        .unwrap_or_else(|| plugin_id.clone());
    let Some(plugin) = db
        .get_plugin_from_registry(&resolved_plugin_id)
        .await
        .map_err(|e| format!("Failed to query plugin '{}': {}", resolved_plugin_id, e))?
    else {
        return Ok(CommandResponse::err(format!(
            "Intruder plugin '{}' was not found",
            resolved_plugin_id
        )));
    };

    if let Err(error) = ensure_intruder_plugin_kind(
        &plugin,
        &resolved_plugin_id,
        IntruderPluginCategory::PayloadGenerator,
    ) {
        return Ok(CommandResponse::err(error));
    }

    if plugin.status != sentinel_plugins::PluginStatus::Enabled {
        return Ok(CommandResponse::err(format!(
            "Plugin '{}' is not enabled",
            resolved_plugin_id
        )));
    }

    let (_, output) = state
        .execute_agent_plugin(&resolved_plugin_id, &input)
        .await?;
    let Some(payloads) = output.as_ref().and_then(extract_intruder_payloads) else {
        return Ok(CommandResponse::err(format!(
            "Plugin '{}' did not return data.payloads",
            resolved_plugin_id
        )));
    };

    Ok(CommandResponse::ok(IntruderPayloadGenerationResult {
        payloads,
        output,
    }))
}

#[tauri::command(rename_all = "camelCase")]
pub async fn intruder_process_payload(
    state: State<'_, TrafficAnalysisState>,
    plugin_id: String,
    input: serde_json::Value,
) -> Result<CommandResponse<IntruderPayloadProcessorResult>, String> {
    let db = state.get_db_service();
    let resolved_plugin_id = resolve_plugin_registry_id(db.as_ref(), &plugin_id)
        .await?
        .unwrap_or_else(|| plugin_id.clone());
    let Some(plugin) = db
        .get_plugin_from_registry(&resolved_plugin_id)
        .await
        .map_err(|e| format!("Failed to query plugin '{}': {}", resolved_plugin_id, e))?
    else {
        return Ok(CommandResponse::err(format!(
            "Intruder plugin '{}' was not found",
            resolved_plugin_id
        )));
    };

    if let Err(error) = ensure_intruder_plugin_kind(
        &plugin,
        &resolved_plugin_id,
        IntruderPluginCategory::PayloadProcessor,
    ) {
        return Ok(CommandResponse::err(error));
    }

    if plugin.status != sentinel_plugins::PluginStatus::Enabled {
        return Ok(CommandResponse::err(format!(
            "Plugin '{}' is not enabled",
            resolved_plugin_id
        )));
    }

    let (_, output) = state
        .execute_agent_plugin(&resolved_plugin_id, &input)
        .await?;
    let Some(result) = output.as_ref().and_then(extract_intruder_payload) else {
        return Ok(CommandResponse::err(format!(
            "Plugin '{}' did not return payload or skip",
            resolved_plugin_id
        )));
    };

    Ok(CommandResponse::ok(IntruderPayloadProcessorResult {
        skipped: result.is_none(),
        payload: result,
        output,
    }))
}

#[tauri::command(rename_all = "camelCase")]
pub async fn intruder_transform_request(
    state: State<'_, TrafficAnalysisState>,
    plugin_id: String,
    input: serde_json::Value,
) -> Result<CommandResponse<IntruderRequestProcessorResult>, String> {
    let db = state.get_db_service();
    let resolved_plugin_id = resolve_plugin_registry_id(db.as_ref(), &plugin_id)
        .await?
        .unwrap_or_else(|| plugin_id.clone());
    let Some(plugin) = db
        .get_plugin_from_registry(&resolved_plugin_id)
        .await
        .map_err(|e| format!("Failed to query plugin '{}': {}", resolved_plugin_id, e))?
    else {
        return Ok(CommandResponse::err(format!(
            "Intruder plugin '{}' was not found",
            resolved_plugin_id
        )));
    };

    if let Err(error) = ensure_intruder_plugin_kind(
        &plugin,
        &resolved_plugin_id,
        IntruderPluginCategory::RequestProcessor,
    ) {
        return Ok(CommandResponse::err(error));
    }

    if plugin.status != sentinel_plugins::PluginStatus::Enabled {
        return Ok(CommandResponse::err(format!(
            "Plugin '{}' is not enabled",
            resolved_plugin_id
        )));
    }

    let (_, output) = state
        .execute_agent_plugin(&resolved_plugin_id, &input)
        .await?;
    let Some(raw_request) = output.as_ref().and_then(extract_intruder_raw_request) else {
        return Ok(CommandResponse::err(format!(
            "Plugin '{}' did not return rawRequest",
            resolved_plugin_id
        )));
    };

    Ok(CommandResponse::ok(IntruderRequestProcessorResult {
        raw_request,
        output,
    }))
}

#[tauri::command]
pub async fn batch_enable_plugins(
    app: AppHandle,
    state: State<'_, TrafficAnalysisState>,
    plugin_ids: Vec<String>,
) -> Result<CommandResponse<BatchToggleResult>, String> {
    let db = state.get_db_service();

    let mut enabled_count: usize = 0;
    let mut failed_ids: Vec<String> = Vec::new();

    for plugin_id in plugin_ids.iter() {
        if ensure_plugin_allowed_for_current_tier(plugin_id).is_err() {
            failed_ids.push(plugin_id.clone());
            continue;
        }

        if db.update_plugin_enabled(plugin_id, true).await.is_err() {
            failed_ids.push(plugin_id.clone());
            continue;
        }

        enabled_count += 1;
        let plugin_name = db
            .get_plugin_name(plugin_id)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| plugin_id.clone());

        emit_plugin_changed(
            &app,
            PluginChangedEvent {
                plugin_id: plugin_id.clone(),
                enabled: true,
                name: plugin_name,
            },
        );
    }

    if enabled_count > 0 {
        if let Err(e) = refresh_active_agent_plugin_tools(db.as_ref()).await {
            tracing::warn!(
                "Failed to refresh active agent tools after batch enable: {}",
                e
            );
        }
    }

    Ok(CommandResponse::ok(BatchToggleResult {
        enabled_count,
        disabled_count: 0,
        failed_ids,
    }))
}

#[tauri::command]
pub async fn batch_disable_plugins(
    app: AppHandle,
    state: State<'_, TrafficAnalysisState>,
    plugin_ids: Vec<String>,
) -> Result<CommandResponse<BatchToggleResult>, String> {
    let db = state.get_db_service();

    let mut disabled_count: usize = 0;
    let mut failed_ids: Vec<String> = Vec::new();

    for plugin_id in plugin_ids.iter() {
        if ensure_plugin_allowed_for_current_tier(plugin_id).is_err() {
            failed_ids.push(plugin_id.clone());
            continue;
        }

        if db.update_plugin_enabled(plugin_id, false).await.is_err() {
            failed_ids.push(plugin_id.clone());
            continue;
        }

        disabled_count += 1;
        let plugin_name = db
            .get_plugin_name(plugin_id)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| plugin_id.clone());

        emit_plugin_changed(
            &app,
            PluginChangedEvent {
                plugin_id: plugin_id.clone(),
                enabled: false,
                name: plugin_name,
            },
        );
    }

    if disabled_count > 0 {
        if let Err(e) = refresh_active_agent_plugin_tools(db.as_ref()).await {
            tracing::warn!(
                "Failed to refresh active agent tools after batch disable: {}",
                e
            );
        }
    }

    Ok(CommandResponse::ok(BatchToggleResult {
        enabled_count: 0,
        disabled_count,
        failed_ids,
    }))
}

#[tauri::command]
pub async fn create_plugin_in_db(
    app: AppHandle,
    state: State<'_, TrafficAnalysisState>,
    metadata: serde_json::Value,
    plugin_code: String,
) -> Result<CommandResponse<String>, String> {
    ensure_plugin_catalog_write_access()?;

    let db = state.get_db_service();

    let plugin: sentinel_traffic::PluginMetadata =
        serde_json::from_value(metadata).map_err(|e| format!("Invalid plugin metadata: {}", e))?;
    let plugin = PluginMetadata {
        monitor_type: validate_plugin_monitor_type(
            plugin.main_category,
            plugin.monitor_type.clone(),
        )?,
        ..plugin
    };

    let plugin_id = plugin.id.clone();
    let plugin_name = plugin.name.clone();

    use sentinel_db::TrafficPluginMetadata;
    let traffic_plugin = TrafficPluginMetadata {
        id: plugin.id.clone(),
        name: plugin.name.clone(),
        version: plugin.version.clone(),
        author: plugin.author.clone(),
        main_category: plugin.main_category.to_string(),
        category: plugin.category.to_string(),
        description: plugin.description.clone(),
        default_severity: format!("{}", plugin.default_severity),
        tags: plugin.tags.clone(),
    };

    db.register_traffic_plugin_with_code(&traffic_plugin, &plugin_code)
        .await
        .map_err(|e| format!("Failed to create plugin in database: {}", e))?;
    let metadata_json = serde_json::to_value(&plugin)
        .map_err(|e| format!("Failed to serialize plugin metadata: {}", e))?;
    db.update_plugin(&metadata_json, &plugin_code)
        .await
        .map_err(|e| format!("Failed to persist plugin metadata: {}", e))?;
    db.update_plugin_enabled(&plugin_id, true)
        .await
        .map_err(|e| format!("Failed to enable plugin after create: {}", e))?;

    tracing::info!("Plugin created/updated in database: {}", plugin_id);

    let plugin_manager = state.get_plugin_manager();
    if is_execution_plugin_main_category(plugin.main_category) {
        let runtime_metadata = PluginMetadata {
            id: plugin.id.clone(),
            name: plugin.name.clone(),
            version: plugin.version.clone(),
            author: plugin.author.clone(),
            main_category: plugin.main_category.clone(),
            category: plugin.category.clone(),
            description: plugin.description.clone(),
            monitor_type: plugin.monitor_type.clone(),
            default_severity: plugin.default_severity,
            tags: plugin.tags.clone(),
            target_asset_types: plugin.target_asset_types.clone(),
            input_mode: plugin.input_mode.clone(),
            seed_bindings: plugin.seed_bindings.clone(),
        };

        let _ = plugin_manager
            .register_plugin(plugin_id.clone(), runtime_metadata, true)
            .await;
    }

    if let Err(e) = plugin_manager
        .set_plugin_code(plugin_id.clone(), plugin_code.clone())
        .await
    {
        tracing::warn!("Failed to update plugin code cache after create: {}", e);
    } else {
        tracing::info!("Plugin code cache updated after create: {}", plugin_id);
    }

    emit_plugin_changed(
        &app,
        PluginChangedEvent {
            plugin_id: plugin_id.clone(),
            enabled: true,
            name: plugin_name,
        },
    );

    Ok(CommandResponse::ok(plugin_id))
}

#[tauri::command]
pub async fn update_plugin(
    app: AppHandle,
    state: State<'_, TrafficAnalysisState>,
    metadata: serde_json::Value,
    plugin_code: String,
) -> Result<CommandResponse<()>, String> {
    let db = state.get_db_service();

    let plugin_id = metadata
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("Missing plugin id")?
        .to_string();

    let plugin_name = metadata
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or(&plugin_id)
        .to_string();

    let plugin: sentinel_traffic::PluginMetadata =
        serde_json::from_value(metadata).map_err(|e| format!("Invalid plugin metadata: {}", e))?;
    let plugin = PluginMetadata {
        monitor_type: validate_plugin_monitor_type(
            plugin.main_category,
            plugin.monitor_type.clone(),
        )?,
        ..plugin
    };
    let plugin_description = plugin.description.clone().unwrap_or_default();
    let main_category = plugin.main_category.clone();
    let plugin_category = plugin.category.clone();

    use sentinel_db::TrafficPluginMetadata;
    let traffic_plugin = TrafficPluginMetadata {
        id: plugin.id.clone(),
        name: plugin.name.clone(),
        version: plugin.version.clone(),
        author: plugin.author.clone(),
        main_category: plugin.main_category.to_string(),
        category: plugin.category.to_string(),
        description: plugin.description.clone(),
        default_severity: format!("{}", plugin.default_severity),
        tags: plugin.tags.clone(),
    };

    db.update_traffic_plugin(&traffic_plugin, &plugin_code)
        .await
        .map_err(|e| format!("Failed to update plugin: {}", e))?;
    let metadata_json = serde_json::to_value(&plugin)
        .map_err(|e| format!("Failed to serialize plugin metadata: {}", e))?;
    db.update_plugin(&metadata_json, &plugin_code)
        .await
        .map_err(|e| format!("Failed to persist plugin metadata: {}", e))?;

    tracing::info!("Plugin updated in database: {}", plugin_id);

    let plugin_manager = state.get_plugin_manager();
    if let Err(e) = plugin_manager
        .set_plugin_code(plugin_id.clone(), plugin_code.clone())
        .await
    {
        tracing::warn!("Failed to update plugin code cache: {}", e);
    } else {
        tracing::info!("Plugin code cache updated: {}", plugin_id);
    }

    if is_traffic_scan_plugin_main_category(main_category) && *state.is_running.read().await {
        if let Some(scan_tx) = state.scan_tx.read().await.as_ref() {
            if let Err(e) =
                scan_tx.send(sentinel_traffic::ScanTask::ReloadPlugin(plugin_id.clone()))
            {
                tracing::error!("Failed to send reload task for plugin {}: {}", plugin_id, e);
            } else {
                tracing::info!("Sent hot-reload task for traffic plugin: {}", plugin_id);
            }
        }
    }

    if is_agent_tool_plugin_main_category(main_category) {
        let tool_server = sentinel_tools::tool_server::get_tool_server();
        let sanitized_id = plugin_id.replace(|c: char| !c.is_alphanumeric() && c != '_', "_");
        let tool_name = format!("plugin__{}", sanitized_id);

        tool_server.unregister_tool(&tool_name).await;

        let plugin_metadata = sentinel_plugins::PluginMetadata {
            id: plugin_id.clone(),
            name: plugin_name.clone(),
            version: "1.0.0".to_string(),
            author: None,
            main_category: main_category.clone(),
            category: plugin_category.clone(),
            monitor_type: None,
            default_severity: sentinel_plugins::Severity::Medium,
            tags: vec![],
            description: Some(plugin_description.clone()),
            target_asset_types: Vec::new(),
            input_mode: None,
            seed_bindings: Vec::new(),
        };
        let input_schema =
            sentinel_tools::plugin_adapter::PluginToolAdapter::get_input_schema_runtime(
                &plugin_code,
                plugin_metadata.clone(),
            )
            .await;
        let output_schema =
            sentinel_tools::plugin_adapter::PluginToolAdapter::get_output_schema_runtime_optional(
                &plugin_code,
                plugin_metadata,
            )
            .await;

        let executor = sentinel_tools::dynamic_tool::create_executor({
            let pid = plugin_id.clone();
            move |args: serde_json::Value| {
                let plugin_id = pid.clone();
                async move {
                    if let Some(ctx) =
                        sentinel_tools::plugin_adapter::get_plugin_context(&plugin_id).await
                    {
                        tracing::info!("Executing plugin: {} (id: {})", ctx.name, ctx.plugin_id);
                        Ok(serde_json::json!({
                            "plugin_id": ctx.plugin_id,
                            "plugin_name": ctx.name,
                            "input": args,
                            "status": "executed"
                        }))
                    } else {
                        Err(format!("Plugin '{}' not registered", plugin_id))
                    }
                }
            }
        });

        tool_server
            .register_plugin_tool(
                &plugin_id,
                &plugin_name,
                &plugin_description,
                input_schema,
                output_schema,
                executor,
            )
            .await;

        tracing::info!("Plugin tool re-registered to ToolServer: {}", tool_name);
    }

    emit_plugin_changed(
        &app,
        PluginChangedEvent {
            plugin_id: plugin_id.clone(),
            enabled: true,
            name: plugin_name,
        },
    );

    Ok(CommandResponse::ok(()))
}

#[tauri::command]
pub async fn get_plugin_code(
    state: State<'_, TrafficAnalysisState>,
    plugin_id: String,
) -> Result<CommandResponse<Option<String>>, String> {
    ensure_plugin_allowed_for_current_tier(&plugin_id)?;

    let db = state.get_db_service();

    let code = db
        .get_traffic_plugin_code(&plugin_id)
        .await
        .map_err(|e| format!("Failed to get plugin code: {}", e))?;

    Ok(CommandResponse::ok(code))
}

#[tauri::command]
pub async fn get_plugin_by_id(
    state: State<'_, TrafficAnalysisState>,
    plugin_id: String,
) -> Result<CommandResponse<Option<serde_json::Value>>, String> {
    ensure_plugin_allowed_for_current_tier(&plugin_id)?;

    let db = state.get_db_service();

    let plugin = db
        .get_traffic_plugin_by_id(&plugin_id)
        .await
        .map_err(|e| format!("Failed to get plugin: {}", e))?;

    Ok(CommandResponse::ok(plugin))
}

#[tauri::command]
pub async fn test_plugin(
    state: State<'_, TrafficAnalysisState>,
    plugin_id: String,
) -> Result<CommandResponse<TestPluginResult>, String> {
    ensure_plugin_allowed_for_current_tier(&plugin_id)?;

    let db = state.get_db_service();

    let plugin_record = db
        .get_plugin_from_registry(&plugin_id)
        .await
        .map_err(|e| format!("Failed to query plugin: {}", e))?;

    if let Some(record) = plugin_record {
        let main_category = record.metadata.main_category.clone();
        let enabled = record.status == sentinel_plugins::PluginStatus::Enabled;

        if is_execution_plugin_main_category(main_category) {
            return Ok(CommandResponse::ok(TestPluginResult {
                success: false,
                message: Some("该插件属于执行型插件类别，请使用 Agent 测试入口".to_string()),
                findings: None,
                error: Some("WrongTestEndpoint".to_string()),
            }));
        }

        let plugin_manager = state.get_plugin_manager();
        if plugin_manager.get_plugin(&plugin_id).await.is_none() {
            let code_opt = db
                .get_plugin_code(&plugin_id)
                .await
                .map_err(|e| format!("Failed to load plugin code: {}", e))?;

            if let Some(code) = code_opt {
                let metadata = db
                    .get_plugin_from_registry(&plugin_id)
                    .await
                    .map_err(|e| format!("Failed to query plugin metadata: {}", e))?
                    .ok_or_else(|| format!("Plugin metadata not found for id {}", plugin_id))?
                    .metadata;

                let severity = match metadata.default_severity {
                    sentinel_plugins::Severity::Critical => {
                        sentinel_traffic::types::Severity::Critical
                    }
                    sentinel_plugins::Severity::High => sentinel_traffic::types::Severity::High,
                    sentinel_plugins::Severity::Medium => sentinel_traffic::types::Severity::Medium,
                    sentinel_plugins::Severity::Low => sentinel_traffic::types::Severity::Low,
                    sentinel_plugins::Severity::Info => sentinel_traffic::types::Severity::Info,
                };

                let traffic_metadata = PluginMetadata {
                    id: metadata.id.clone(),
                    name: metadata.name.clone(),
                    version: metadata.version.clone(),
                    author: metadata.author.clone(),
                    main_category: metadata.main_category.clone(),
                    category: metadata.category.clone(),
                    monitor_type: metadata.monitor_type.clone(),
                    description: metadata.description.clone(),
                    default_severity: severity,
                    tags: metadata.tags.clone(),
                    target_asset_types: metadata.target_asset_types.clone(),
                    input_mode: metadata.input_mode.clone(),
                    seed_bindings: metadata.seed_bindings.clone(),
                };

                let _ = plugin_manager
                    .register_plugin(plugin_id.clone(), traffic_metadata, enabled)
                    .await;
                let _ = plugin_manager
                    .set_plugin_code(plugin_id.clone(), code)
                    .await;
            }
        }

        let original_status = plugin_manager
            .get_plugin(&plugin_id)
            .await
            .map(|p| p.status);
        if !enabled {
            let _ = plugin_manager.enable_plugin(&plugin_id).await;
        }

        use sentinel_traffic::RequestContext;
        let mut headers = std::collections::HashMap::new();
        headers.insert("User-Agent".to_string(), "Sentinel-Test/1.0".to_string());

        let request_ctx = RequestContext {
            id: uuid::Uuid::new_v4().to_string(),
            method: "GET".to_string(),
            url: "https://example.com/test".to_string(),
            http_version: Some("HTTP/1.1".to_string()),
            headers,
            body: vec![],
            content_type: Some("text/plain".to_string()),
            query_params: std::collections::HashMap::new(),
            is_https: true,
            timestamp: chrono::Utc::now(),
            was_edited: false,
            edited_method: None,
            edited_url: None,
            edited_headers: None,
            edited_body: None,
        };

        let transaction = HttpTransaction {
            request: request_ctx.clone(),
            response: None,
        };
        let findings_result = plugin_manager
            .scan_transaction(&plugin_id, &transaction)
            .await;

        if !enabled {
            if let Some(status) = original_status {
                if status == PluginStatus::Disabled {
                    let _ = plugin_manager.disable_plugin(&plugin_id).await;
                }
            }
        }

        match findings_result {
            Ok(foundings) => {
                let mapped: Vec<TestFinding> = foundings
                    .into_iter()
                    .map(|f| TestFinding {
                        title: f.title,
                        description: f.description,
                        severity: f.severity.to_string(),
                    })
                    .collect();

                return Ok(CommandResponse::ok(TestPluginResult {
                    success: true,
                    message: Some(format!(
                        "插件 '{}' (v{}) 执行测试完成。发现数量: {}。",
                        record.metadata.name,
                        record.metadata.version,
                        mapped.len()
                    )),
                    findings: Some(mapped),
                    error: None,
                }));
            }
            Err(e) => {
                return Ok(CommandResponse::ok(TestPluginResult {
                    success: false,
                    message: Some(format!("插件执行失败: {}", e)),
                    findings: None,
                    error: Some("ExecutionError".to_string()),
                }));
            }
        }
    }

    Ok(CommandResponse::err(format!(
        "Plugin not found: {}",
        plugin_id
    )))
}

#[tauri::command(rename_all = "camelCase")]
pub async fn test_plugin_advanced(
    state: State<'_, TrafficAnalysisState>,
    plugin_id: String,
    url: Option<String>,
    method: Option<String>,
    headers: Option<std::collections::HashMap<String, String>>,
    body: Option<String>,
    runs: Option<u32>,
    concurrency: Option<u32>,
) -> Result<CommandResponse<AdvancedTestResult>, String> {
    ensure_plugin_allowed_for_current_tier(&plugin_id)?;

    let runs = runs.unwrap_or(1).max(1);
    let concurrency = concurrency.unwrap_or(1).max(1);
    let db = state.get_db_service();

    let plugin_record = match db.get_plugin_from_registry(&plugin_id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return Ok(CommandResponse::ok(AdvancedTestResult {
                plugin_id,
                success: false,
                total_runs: runs,
                concurrency,
                total_duration_ms: 0,
                avg_duration_ms: 0.0,
                total_findings: 0,
                unique_findings: 0,
                findings: vec![],
                runs: vec![],
                message: Some("Plugin not found".to_string()),
                error: Some("PluginNotFound".to_string()),
                outputs: None,
            }));
        }
        Err(e) => return Err(format!("Failed to query plugin: {}", e)),
    };

    let plugin_manager = state.get_plugin_manager();
    if plugin_manager.get_plugin(&plugin_id).await.is_none() {
        let code_opt = db
            .get_plugin_code(&plugin_id)
            .await
            .map_err(|e| format!("Failed to load plugin code: {}", e))?;

        if let Some(code) = code_opt {
            let metadata = PluginMetadata {
                id: plugin_id.clone(),
                name: plugin_record.metadata.name,
                version: plugin_record.metadata.version,
                author: plugin_record.metadata.author,
                main_category: plugin_record.metadata.main_category,
                category: plugin_record.metadata.category,
                monitor_type: plugin_record.metadata.monitor_type,
                description: plugin_record.metadata.description,
                default_severity: match plugin_record.metadata.default_severity {
                    sentinel_plugins::Severity::Critical => sentinel_traffic::Severity::Critical,
                    sentinel_plugins::Severity::High => sentinel_traffic::Severity::High,
                    sentinel_plugins::Severity::Medium => sentinel_traffic::Severity::Medium,
                    sentinel_plugins::Severity::Low => sentinel_traffic::Severity::Low,
                    sentinel_plugins::Severity::Info => sentinel_traffic::Severity::Info,
                },
                tags: plugin_record.metadata.tags,
                target_asset_types: plugin_record.metadata.target_asset_types,
                input_mode: plugin_record.metadata.input_mode,
                seed_bindings: plugin_record.metadata.seed_bindings,
            };

            let _ = plugin_manager
                .register_plugin(plugin_id.clone(), metadata, true)
                .await;
            let _ = plugin_manager
                .set_plugin_code(plugin_id.clone(), code)
                .await;
        }
    }

    use sentinel_traffic::RequestContext;
    let plugin_manager = state.get_plugin_manager();

    let parsed_headers = headers.unwrap_or_else(|| {
        let mut m = std::collections::HashMap::new();
        m.insert("User-Agent".to_string(), "Sentinel-AdvTest/1.0".to_string());
        m
    });

    let req_url = url.unwrap_or_else(|| "https://example.com/test".to_string());
    let req_method = method.unwrap_or_else(|| "GET".to_string());
    let body_bytes = body.map(|b| b.into_bytes()).unwrap_or_default();

    let indices: Vec<u32> = (0..runs).collect();
    let start_all = std::time::Instant::now();
    let mut run_stats: Vec<AdvancedRunStat> = Vec::with_capacity(runs as usize);
    let mut all_findings: Vec<TestFinding> = Vec::new();

    use futures::{stream, StreamExt};
    let results = stream::iter(indices.into_iter())
        .map(|i| {
            let plugin_manager = plugin_manager.clone();
            let plugin_id = plugin_id.clone();
            let headers_map = parsed_headers.clone();
            let req_url = req_url.clone();
            let req_method = req_method.clone();
            let body_bytes = body_bytes.clone();
            async move {
                let run_start = std::time::Instant::now();
                let ctx = RequestContext {
                    id: uuid::Uuid::new_v4().to_string(),
                    method: req_method.clone(),
                    url: req_url.clone(),
                    http_version: Some("HTTP/1.1".to_string()),
                    headers: headers_map.clone(),
                    body: body_bytes.clone(),
                    content_type: None,
                    query_params: std::collections::HashMap::new(),
                    is_https: req_url.starts_with("https://"),
                    timestamp: chrono::Utc::now(),
                    was_edited: false,
                    edited_method: None,
                    edited_url: None,
                    edited_headers: None,
                    edited_body: None,
                };
                let transaction = HttpTransaction {
                    request: ctx.clone(),
                    response: None,
                };
                match plugin_manager
                    .scan_transaction(&plugin_id, &transaction)
                    .await
                {
                    Ok(foundings) => {
                        let mapped: Vec<TestFinding> = foundings
                            .into_iter()
                            .map(|f| TestFinding {
                                title: f.title,
                                description: f.description,
                                severity: f.severity.to_string(),
                            })
                            .collect();
                        let dur = run_start.elapsed().as_millis();
                        (i, Ok((dur, mapped)))
                    }
                    Err(e) => {
                        let dur = run_start.elapsed().as_millis();
                        (i, Err((dur, e.to_string())))
                    }
                }
            }
        })
        .buffer_unordered(concurrency as usize)
        .collect::<Vec<(u32, Result<(u128, Vec<TestFinding>), (u128, String)>)>>()
        .await;

    for (idx, res) in results {
        match res {
            Ok((dur, findings)) => {
                run_stats.push(AdvancedRunStat {
                    run_index: idx,
                    duration_ms: dur,
                    findings: findings.len(),
                    error: None,
                });
                all_findings.extend(findings);
            }
            Err((dur, err)) => {
                run_stats.push(AdvancedRunStat {
                    run_index: idx,
                    duration_ms: dur,
                    findings: 0,
                    error: Some(err),
                });
            }
        }
    }

    let total_duration_ms = start_all.elapsed().as_millis();
    let avg_duration_ms = if run_stats.is_empty() {
        0.0
    } else {
        (run_stats.iter().map(|r| r.duration_ms).sum::<u128>() as f64) / (run_stats.len() as f64)
    };

    use std::collections::HashSet;
    let mut uniq = HashSet::new();
    let mut unique_list: Vec<TestFinding> = Vec::new();
    for f in &all_findings {
        let key = format!("{}|{}|{}", f.title, f.severity, f.description);
        if uniq.insert(key) {
            unique_list.push(f.clone());
        }
    }

    Ok(CommandResponse::ok(AdvancedTestResult {
        plugin_id,
        success: run_stats.iter().all(|r| r.error.is_none()),
        total_runs: runs,
        concurrency,
        total_duration_ms,
        avg_duration_ms,
        total_findings: all_findings.len(),
        unique_findings: unique_list.len(),
        findings: unique_list,
        runs: run_stats,
        message: Some("高级测试完成".to_string()),
        error: None,
        outputs: None,
    }))
}

#[tauri::command(rename_all = "camelCase")]
pub async fn test_agent_plugin(
    state: State<'_, TrafficAnalysisState>,
    plugin_id: String,
    inputs: Option<serde_json::Value>,
) -> Result<CommandResponse<AgentTestResult>, String> {
    ensure_plugin_allowed_for_current_tier(&plugin_id)?;

    let db = state.get_db_service();

    let plugin_record = match db.get_plugin_from_registry(&plugin_id).await {
        Ok(Some(record)) => record,
        Ok(None) => {
            return Ok(CommandResponse::ok(AgentTestResult {
                success: false,
                message: Some(format!("插件 '{}' 不存在", plugin_id)),
                output: None,
                execution_time_ms: 0,
                error: Some("PluginNotFound".to_string()),
            }));
        }
        Err(e) => return Err(format!("Failed to query plugin: {}", e)),
    };

    if !matches!(
        plugin_record.metadata.main_category,
        PluginMainCategory::Agent | PluginMainCategory::Bounty | PluginMainCategory::Intruder
    ) {
        return Ok(CommandResponse::ok(AgentTestResult {
            success: false,
            message: Some("该插件不是执行型插件，请使用流量分析测试入口".to_string()),
            output: None,
            execution_time_ms: 0,
            error: Some("WrongPluginType".to_string()),
        }));
    }

    let code = db
        .get_plugin_code(&plugin_id)
        .await
        .map_err(|e| format!("Failed to query plugin code: {}", e))?;

    let code = match code {
        Some(c) => c,
        None => {
            return Ok(CommandResponse::ok(AgentTestResult {
                success: false,
                message: Some("插件代码不存在".to_string()),
                output: None,
                execution_time_ms: 0,
                error: Some("NoPluginCode".to_string()),
            }));
        }
    };

    let severity = crate::services::parse_plugin_severity(
        &format!("{:?}", plugin_record.metadata.default_severity).to_lowercase(),
    )
    .map_err(|e| format!("Failed to parse plugin severity: {}", e))?;
    let metadata = crate::services::build_plugin_metadata(
        plugin_record.metadata.id.clone(),
        plugin_record.metadata.name.clone(),
        plugin_record.metadata.main_category.to_string(),
        plugin_record.metadata.category.to_string(),
        plugin_record.metadata.description.clone(),
        plugin_record.metadata.monitor_type.clone(),
        severity,
    )
    .map_err(|e| format!("Failed to build plugin metadata: {}", e))?;

    let default_input = load_plugin_default_inputs(db.as_ref(), &plugin_id).await?;
    let resolved_inputs = match inputs {
        Some(value) => Some(merge_plugin_input_defaults(&default_input, &value)),
        None => Some(default_input),
    };

    let result = crate::services::test_plugin_code(metadata, code, resolved_inputs)
        .await
        .map_err(|e| format!("Plugin test failed: {}", e))?;

    Ok(CommandResponse::ok(AgentTestResult {
        success: result.success,
        message: result.message,
        output: result.output,
        execution_time_ms: result.execution_time_ms,
        error: result.error,
    }))
}

#[tauri::command(rename_all = "camelCase")]
pub async fn get_plugin_input_schema(
    state: State<'_, TrafficAnalysisState>,
    plugin_id: String,
) -> Result<CommandResponse<serde_json::Value>, String> {
    ensure_plugin_allowed_for_current_tier(&plugin_id)?;

    let db = state.get_db_service();
    let resolved_plugin_id = resolve_plugin_registry_id(db.as_ref(), &plugin_id)
        .await?
        .unwrap_or_else(|| plugin_id.clone());
    let plugin_record = db
        .get_plugin_from_registry(&resolved_plugin_id)
        .await
        .map_err(|e| format!("Failed to query plugin metadata: {}", e))?;

    let code = db
        .get_plugin_code(&resolved_plugin_id)
        .await
        .map_err(|e| format!("Failed to query plugin code: {}", e))?;

    let code = match code {
        Some(c) => c,
        None => {
            return Ok(CommandResponse::ok(serde_json::json!({
                "type": "object",
                "properties": {}
            })));
        }
    };

    let metadata = plugin_record
        .map(|record| record.metadata)
        .unwrap_or_else(|| sentinel_plugins::PluginMetadata {
            id: resolved_plugin_id.clone(),
            name: resolved_plugin_id.clone(),
            version: "1.0.0".to_string(),
            author: None,
            main_category: sentinel_plugins::PluginMainCategory::Agent,
            category: "tool".into(),
            monitor_type: None,
            default_severity: sentinel_plugins::Severity::Medium,
            tags: vec![],
            description: None,
            target_asset_types: Vec::new(),
            input_mode: None,
            seed_bindings: Vec::new(),
        });

    let schema = sentinel_tools::plugin_adapter::PluginToolAdapter::get_input_schema_runtime(
        &code, metadata,
    )
    .await;

    Ok(CommandResponse::ok(schema))
}

#[tauri::command(rename_all = "camelCase")]
pub async fn get_plugin_output_schema(
    state: State<'_, TrafficAnalysisState>,
    plugin_id: String,
) -> Result<CommandResponse<serde_json::Value>, String> {
    ensure_plugin_allowed_for_current_tier(&plugin_id)?;

    let db = state.get_db_service();
    let plugin_record = db
        .get_plugin_from_registry(&plugin_id)
        .await
        .map_err(|e| format!("Failed to query plugin metadata: {}", e))?;

    let code = db
        .get_plugin_code(&plugin_id)
        .await
        .map_err(|e| format!("Failed to query plugin code: {}", e))?;

    let code = match code {
        Some(c) => c,
        None => {
            return Ok(CommandResponse::ok(serde_json::json!({
                "type": "object",
                "properties": {
                    "success": {"type": "boolean"},
                    "data": {"type": "object"},
                    "error": {"type": "string"}
                }
            })));
        }
    };

    let metadata = plugin_record
        .map(|record| record.metadata)
        .unwrap_or_else(|| sentinel_plugins::PluginMetadata {
            id: plugin_id.clone(),
            name: plugin_id.clone(),
            version: "1.0.0".to_string(),
            author: None,
            main_category: sentinel_plugins::PluginMainCategory::Agent,
            category: "tool".into(),
            monitor_type: None,
            default_severity: sentinel_plugins::Severity::Medium,
            tags: vec![],
            description: None,
            target_asset_types: Vec::new(),
            input_mode: None,
            seed_bindings: Vec::new(),
        });

    let schema = match sentinel_plugins::get_output_schema_from_code(&code, metadata).await {
        Ok(s) => s,
        Err(e) => {
            log::warn!(
                "Failed to get output schema for plugin {}: {}",
                plugin_id,
                e
            );
            serde_json::json!({
                "type": "object",
                "properties": {
                    "success": {"type": "boolean"},
                    "data": {"type": "object"},
                    "error": {"type": "string"}
                }
            })
        }
    };

    Ok(CommandResponse::ok(schema))
}

#[tauri::command]
pub async fn delete_plugin(
    app: AppHandle,
    state: State<'_, TrafficAnalysisState>,
    plugin_id: String,
) -> Result<CommandResponse<()>, String> {
    ensure_plugin_delete_access()?;
    ensure_plugin_allowed_for_current_tier(&plugin_id)?;

    let db = state.get_db_service();
    let plugin_name = db
        .get_plugin_name(&plugin_id)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| plugin_id.clone());

    db.update_traffic_plugin_enabled(&plugin_id, false)
        .await
        .map_err(|e| format!("Failed to disable plugin before deletion: {}", e))?;

    db.delete_traffic_plugin(&plugin_id)
        .await
        .map_err(|e| format!("Failed to delete plugin: {}", e))?;

    if let Err(e) = refresh_active_agent_plugin_tools(db.as_ref()).await {
        tracing::warn!(
            "Failed to refresh active agent tools after deleting plugin {}: {}",
            plugin_id,
            e
        );
    }

    emit_plugin_changed(
        &app,
        PluginChangedEvent {
            plugin_id: plugin_id.clone(),
            enabled: false,
            name: plugin_name,
        },
    );

    tracing::info!("Plugin deleted: {}", plugin_id);
    Ok(CommandResponse::ok(()))
}
