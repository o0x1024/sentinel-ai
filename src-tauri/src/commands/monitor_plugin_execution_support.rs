use std::sync::Arc;
use std::time::Instant;

use crate::services::{load_plugin_default_inputs, merge_plugin_input_defaults};
use sentinel_db::{Database, DatabaseService};
use sentinel_plugins::PluginMainCategory;
use serde_json::Value;

pub(crate) fn is_monitor_execution_plugin_category(main_category: PluginMainCategory) -> bool {
    matches!(
        main_category,
        PluginMainCategory::Agent | PluginMainCategory::Bounty
    )
}

pub(crate) fn normalize_plugin_registry_id(value: &str) -> String {
    let trimmed = value.trim();
    let without_plugin_source = trimmed.strip_prefix("plugin::").unwrap_or(trimmed);
    without_plugin_source
        .strip_prefix("plugin__")
        .unwrap_or(without_plugin_source)
        .to_string()
}

fn is_missing_required_input_value(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => true,
        Some(Value::String(value)) => value.trim().is_empty(),
        Some(Value::Array(value)) => value.is_empty(),
        Some(Value::Object(value)) => value.is_empty(),
        Some(_) => false,
    }
}

fn collect_missing_required_inputs(schema: &Value, input: &Value, path: &[String]) -> Vec<String> {
    let mut missing = Vec::new();
    let properties = schema
        .get("properties")
        .and_then(|value| value.as_object());

    if let Some(required_fields) = schema.get("required").and_then(|value| value.as_array()) {
        for required_field in required_fields {
            let Some(field_name) = required_field.as_str() else {
                continue;
            };
            let mut field_path = path.to_vec();
            field_path.push(field_name.to_string());
            if is_missing_required_input_value(input.get(field_name)) {
                missing.push(field_path.join("."));
            }
        }
    }

    let Some(properties) = properties else {
        return missing;
    };

    for (field_name, field_schema) in properties {
        let Some(child_input) = input.get(field_name) else {
            continue;
        };
        if !child_input.is_object() {
            continue;
        }

        let mut field_path = path.to_vec();
        field_path.push(field_name.clone());
        missing.extend(collect_missing_required_inputs(
            field_schema,
            child_input,
            &field_path,
        ));
    }

    missing
}

async fn validate_monitor_plugin_required_inputs(
    plugin_id: &str,
    plugin_name: &str,
    metadata: sentinel_plugins::PluginMetadata,
    code: &str,
    input: &Value,
) -> Result<(), String> {
    let schema = sentinel_plugins::get_input_schema_from_code(code, metadata)
        .await
        .map_err(|error| {
            format!(
                "Failed to load input schema for plugin '{}': {}",
                plugin_name, error
            )
        })?;
    let missing_inputs = collect_missing_required_inputs(&schema, input, &[]);
    if missing_inputs.is_empty() {
        return Ok(());
    }

    Err(format!(
        "Plugin {} missing required input: {}",
        plugin_id,
        missing_inputs.join(", ")
    ))
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

        if !is_monitor_execution_plugin_category(plugin_record.metadata.main_category) {
            return Err(format!(
                "Plugin '{}' is not a monitor execution plugin",
                normalized_plugin_id
            ));
        }

        if plugin_record.status != sentinel_plugins::PluginStatus::Enabled {
            return Err(format!("Plugin '{}' is not enabled", normalized_plugin_id));
        }

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

        let default_inputs = load_plugin_default_inputs(db.as_ref(), &normalized_plugin_id).await?;
        let resolved_input = merge_plugin_input_defaults(&default_inputs, &input);

        validate_monitor_plugin_required_inputs(
            &normalized_plugin_id,
            &plugin_record.metadata.name,
            plugin_record.metadata.clone(),
            &code,
            &resolved_input,
        )
        .await?;

        if plugin_manager
            .get_plugin(&normalized_plugin_id)
            .await
            .is_none()
        {
            let metadata = sentinel_traffic::PluginMetadata {
                id: plugin_record.metadata.id.clone(),
                name: plugin_record.metadata.name.clone(),
                version: plugin_record.metadata.version.clone(),
                author: plugin_record.metadata.author.clone(),
                main_category: plugin_record.metadata.main_category,
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
                input_mode: plugin_record.metadata.input_mode.clone(),
                seed_bindings: plugin_record.metadata.seed_bindings.clone(),
            };

            plugin_manager
                .register_plugin(normalized_plugin_id.clone(), metadata, true)
                .await
                .map_err(|error| {
                    format!(
                        "Failed to register plugin '{}': {}",
                        normalized_plugin_id, error
                    )
                })?;
            plugin_manager
                .set_plugin_code(normalized_plugin_id.clone(), code)
                .await
                .map_err(|error| {
                    format!(
                        "Failed to cache plugin '{}': {}",
                        normalized_plugin_id, error
                    )
                })?;
        } else {
            if let Err(error) = plugin_manager.enable_plugin(&normalized_plugin_id).await {
                tracing::warn!(
                    "Failed to enable monitor plugin '{}': {}",
                    normalized_plugin_id,
                    error
                );
            }
            plugin_manager
                .set_plugin_code(normalized_plugin_id.clone(), code)
                .await
                .map_err(|error| {
                    format!(
                        "Failed to refresh cached plugin '{}': {}",
                        normalized_plugin_id, error
                    )
                })?;
        }

        let run_id = input
            .get("__monitorExecution")
            .and_then(|value| value.as_object())
            .and_then(|context| {
                let task_id = context.get("task_id").and_then(|value| value.as_str())?;
                let started_at = context
                    .get("started_at")
                    .and_then(|value| value.as_str())
                    .unwrap_or("manual");
                Some(format!("monitor:{task_id}:{started_at}"))
            })
            .unwrap_or_else(|| format!("monitor:{}", uuid::Uuid::new_v4()));

        let (findings, output) = plugin_manager
            .execute_execution_plugin(
                &normalized_plugin_id,
                &resolved_input,
                "monitor_task",
                Some(run_id),
            )
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
