use std::sync::Arc;
use std::time::Instant;

use sentinel_db::{Database, DatabaseService};

pub(crate) fn is_monitor_execution_plugin_category(main_category: &str) -> bool {
    matches!(main_category, "agent" | "bounty")
}

pub(crate) fn normalize_plugin_registry_id(value: &str) -> String {
    let trimmed = value.trim();
    let without_plugin_source = trimmed.strip_prefix("plugin::").unwrap_or(trimmed);
    without_plugin_source
        .strip_prefix("plugin__")
        .unwrap_or(without_plugin_source)
        .to_string()
}

pub(crate) async fn execute_monitor_plugin(
    db: &Arc<DatabaseService>,
    plugin_manager: &Arc<sentinel_traffic::PluginManager>,
    plugin_id: &str,
    input: serde_json::Value,
) -> sentinel_tools::tool_server::ToolResult {
    let started_at = Instant::now();
    let normalized_plugin_id = normalize_plugin_registry_id(plugin_id);

    let result = async {
        let plugin_record = db
            .get_plugin_from_registry(&normalized_plugin_id)
            .await
            .map_err(|error| {
                format!(
                    "Failed to query plugin '{}': {}",
                    normalized_plugin_id, error
                )
            })?
            .ok_or_else(|| format!("Plugin '{}' not found", normalized_plugin_id))?;

        if !is_monitor_execution_plugin_category(&plugin_record.metadata.main_category) {
            return Err(format!(
                "Plugin '{}' is not a monitor execution plugin",
                normalized_plugin_id
            ));
        }

        if plugin_record.status != sentinel_plugins::PluginStatus::Enabled {
            return Err(format!("Plugin '{}' is not enabled", normalized_plugin_id));
        }

        if plugin_manager
            .get_plugin(&normalized_plugin_id)
            .await
            .is_none()
        {
            let code = db
                .get_plugin_code(&normalized_plugin_id)
                .await
                .map_err(|error| {
                    format!(
                        "Failed to load plugin code for '{}': {}",
                        normalized_plugin_id, error
                    )
                })?
                .ok_or_else(|| format!("Plugin '{}' has no code", normalized_plugin_id))?;

            let metadata = sentinel_traffic::PluginMetadata {
                id: plugin_record.metadata.id.clone(),
                name: plugin_record.metadata.name.clone(),
                version: plugin_record.metadata.version.clone(),
                author: plugin_record.metadata.author.clone(),
                main_category: plugin_record.metadata.main_category.clone(),
                category: plugin_record.metadata.category.clone(),
                monitor_type: plugin_record.metadata.monitor_type.clone(),
                description: plugin_record.metadata.description.clone(),
                default_severity: match plugin_record.metadata.default_severity {
                    sentinel_plugins::Severity::Critical => sentinel_traffic::Severity::Critical,
                    sentinel_plugins::Severity::High => sentinel_traffic::Severity::High,
                    sentinel_plugins::Severity::Medium => sentinel_traffic::Severity::Medium,
                    sentinel_plugins::Severity::Low => sentinel_traffic::Severity::Low,
                    sentinel_plugins::Severity::Info => sentinel_traffic::Severity::Info,
                },
                tags: plugin_record.metadata.tags.clone(),
                target_asset_types: plugin_record.metadata.target_asset_types.clone(),
            };

            plugin_manager
                .register_plugin(normalized_plugin_id.clone(), metadata, true)
                .await
                .map_err(|error| {
                    format!("Failed to register plugin '{}': {}", normalized_plugin_id, error)
                })?;
            plugin_manager
                .set_plugin_code(normalized_plugin_id.clone(), code)
                .await
                .map_err(|error| {
                    format!("Failed to cache plugin '{}': {}", normalized_plugin_id, error)
                })?;
        } else if let Err(error) = plugin_manager.enable_plugin(&normalized_plugin_id).await {
            tracing::warn!(
                "Failed to enable monitor plugin '{}': {}",
                normalized_plugin_id,
                error
            );
        }

        let (findings, output) = plugin_manager
            .execute_agent(&normalized_plugin_id, &input)
            .await
            .map_err(|error| {
                format!(
                    "Failed to execute plugin '{}': {}",
                    normalized_plugin_id, error
                )
            })?;

        Ok(output.unwrap_or_else(|| {
            serde_json::json!({
                "success": true,
                "plugin_id": normalized_plugin_id,
                "findings_count": findings.len(),
                "findings": findings
            })
        }))
    }
    .await;

    match result {
        Ok(output) => sentinel_tools::tool_server::ToolResult {
            success: true,
            tool_name: normalized_plugin_id,
            output: Some(output),
            error: None,
            execution_time_ms: started_at.elapsed().as_millis() as u64,
        },
        Err(error) => sentinel_tools::tool_server::ToolResult {
            success: false,
            tool_name: normalized_plugin_id,
            output: None,
            error: Some(error),
            execution_time_ms: started_at.elapsed().as_millis() as u64,
        },
    }
}
