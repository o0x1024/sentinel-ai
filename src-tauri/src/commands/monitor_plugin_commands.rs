use crate::commands::monitor_config_support::normalize_monitor_type;
use crate::commands::monitor_plugin_execution_support::{
    is_monitor_execution_plugin_category, normalize_plugin_registry_id,
};
use sentinel_db::{Database, DatabaseService};
use sentinel_plugins::MonitorSeedBinding;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorPluginInfo {
    pub id: String,
    pub name: String,
    pub category: String,
    pub monitor_type: String,
    pub input_mode: Option<String>,
    pub description: Option<String>,
    pub seed_bindings: Vec<MonitorSeedBinding>,
    pub is_available: bool,
}

async fn resolve_monitor_type_from_metadata(
    db_service: &Arc<DatabaseService>,
    tool_name: &str,
) -> Result<Option<String>, String> {
    let normalized_name = normalize_plugin_registry_id(tool_name);
    let plugin_record = db_service
        .get_plugin_from_registry(&normalized_name)
        .await
        .map_err(|e| e.to_string())?;

    let Some(plugin_record) = plugin_record else {
        return Ok(None);
    };

    Ok(plugin_record
        .metadata
        .monitor_type
        .as_deref()
        .and_then(normalize_monitor_type)
        .map(str::to_string))
}

#[tauri::command]
pub async fn monitor_get_available_plugins(
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<MonitorPluginInfo>, String> {
    let all_plugins = db_service
        .get_plugins_from_registry(Some("default"))
        .await
        .map_err(|error| format!("Failed to list plugins: {}", error))?;

    tracing::info!(
        "Loading available plugins for monitoring, total plugins: {}",
        all_plugins.len()
    );

    let mut plugins = Vec::new();

    for plugin in all_plugins {
        if !is_monitor_execution_plugin_category(plugin.metadata.main_category) {
            continue;
        }

        tracing::debug!(
            "Checking monitor plugin: id={}, main_category={}, category={}, status={:?}",
            plugin.metadata.id,
            plugin.metadata.main_category,
            plugin.metadata.category,
            plugin.status
        );

        let Some(monitor_type) =
            resolve_monitor_type_from_metadata(db_service.inner(), &plugin.metadata.id).await?
        else {
            continue;
        };

        plugins.push(MonitorPluginInfo {
            id: plugin.metadata.id.clone(),
            name: plugin.metadata.name.clone(),
            category: plugin.metadata.category.to_string(),
            monitor_type,
            input_mode: plugin.metadata.input_mode.clone(),
            description: plugin.metadata.description.clone(),
            seed_bindings: plugin.metadata.seed_bindings.clone(),
            is_available: plugin.status == sentinel_plugins::PluginStatus::Enabled,
        });
    }

    tracing::info!("Found {} monitor plugins", plugins.len());

    Ok(plugins)
}

#[tauri::command]
pub async fn monitor_test_plugin(
    db_service: State<'_, Arc<DatabaseService>>,
    plugin_id: String,
) -> Result<bool, String> {
    let normalized_plugin_id = normalize_plugin_registry_id(&plugin_id);
    let plugin = db_service
        .get_plugin_from_registry(&normalized_plugin_id)
        .await
        .map_err(|error| {
            format!(
                "Failed to query plugin '{}': {}",
                normalized_plugin_id, error
            )
        })?;

    Ok(plugin
        .map(|record| {
            is_monitor_execution_plugin_category(record.metadata.main_category)
                && record.status == sentinel_plugins::PluginStatus::Enabled
        })
        .unwrap_or(false))
}
