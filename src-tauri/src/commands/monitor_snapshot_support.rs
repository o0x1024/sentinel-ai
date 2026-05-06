use std::sync::Arc;

use chrono::Utc;
use sentinel_bounty::services::{ChangeMonitorConfig, MonitorPluginConfig, MonitorTask};
use sentinel_db::{DatabaseService, SurfaceDiscoveryRunRow, SurfaceObservationRow};
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::commands::monitor_config_support::infer_monitor_type_for_plugin;

fn normalized_plugin_id(plugin_id: &str) -> &str {
    plugin_id.strip_prefix("plugin__").unwrap_or(plugin_id)
}

fn plugin_configs_mut<'a>(
    config: &'a mut ChangeMonitorConfig,
    monitor_type: &str,
) -> Option<&'a mut Vec<MonitorPluginConfig>> {
    match monitor_type {
        "dns" => Some(&mut config.dns_plugins),
        "ip" => Some(&mut config.ip_plugins),
        "cert" => Some(&mut config.cert_plugins),
        "content" => Some(&mut config.content_plugins),
        "api" => Some(&mut config.api_plugins),
        "port" => Some(&mut config.port_plugins),
        "service" => Some(&mut config.service_plugins),
        "web" => Some(&mut config.web_plugins),
        "risk" => Some(&mut config.risk_plugins),
        _ => None,
    }
}

fn upsert_previous_snapshots(
    plugin: &mut MonitorPluginConfig,
    snapshots: Map<String, Value>,
) -> bool {
    let next_value = Value::Object(snapshots);
    let mut params = plugin
        .plugin_params
        .as_object()
        .cloned()
        .unwrap_or_default();
    if params.get("previousSnapshots") == Some(&next_value) {
        return false;
    }
    params.insert("previousSnapshots".to_string(), next_value);
    plugin.plugin_params = Value::Object(params);
    true
}

fn persist_snapshots_for_plugin(
    plugins: &mut [MonitorPluginConfig],
    target_plugin_id: &str,
    snapshots: &Map<String, Value>,
) -> bool {
    for plugin in plugins {
        if normalized_plugin_id(&plugin.plugin_id) == target_plugin_id {
            return upsert_previous_snapshots(plugin, snapshots.clone());
        }
    }
    false
}

pub(crate) fn persist_monitor_plugin_snapshots_to_task(
    task: &mut MonitorTask,
    plugin: &MonitorPluginConfig,
    output: &Value,
) -> bool {
    let snapshots = output
        .get("data")
        .unwrap_or(output)
        .get("snapshots")
        .and_then(Value::as_object)
        .cloned();

    let Some(snapshots) = snapshots else {
        return false;
    };

    let normalized_id = normalized_plugin_id(&plugin.plugin_id).to_string();
    if let Some(monitor_type) = infer_monitor_type_for_plugin(&normalized_id, "") {
        if let Some(plugins) = plugin_configs_mut(&mut task.config, monitor_type) {
            if persist_snapshots_for_plugin(plugins, &normalized_id, &snapshots) {
                return true;
            }
        }
    }

    for monitor_type in [
        "dns", "ip", "cert", "content", "api", "port", "service", "web", "risk",
    ] {
        if let Some(plugins) = plugin_configs_mut(&mut task.config, monitor_type) {
            if persist_snapshots_for_plugin(plugins, &normalized_id, &snapshots) {
                return true;
            }
        }
    }

    false
}

fn string_value(object: &Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| object.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub(crate) async fn persist_api_monitor_inventory_output(
    db_service: &Arc<DatabaseService>,
    task: &MonitorTask,
    plugin: &MonitorPluginConfig,
    output: &Value,
    trigger_source: &str,
    schedule_id: Option<&str>,
) -> Result<usize, String> {
    if normalized_plugin_id(&plugin.plugin_id) != "api_monitor" {
        return Ok(0);
    }

    let data = output.get("data").unwrap_or(output);
    let results = data
        .get("results")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if results.is_empty() {
        return Ok(0);
    }

    let started_at = Utc::now().to_rfc3339();
    let run_id = Uuid::new_v4().to_string();
    let run = SurfaceDiscoveryRunRow {
        id: run_id.clone(),
        program_id: task.program_id.clone(),
        trigger_source: trigger_source.to_string(),
        schedule_id: schedule_id.map(str::to_string),
        workflow_id: None,
        workflow_template_id: None,
        plugin_id: Some(normalized_plugin_id(&plugin.plugin_id).to_string()),
        status: "running".to_string(),
        seed_count: None,
        observation_count: Some(0),
        imported_asset_count: Some(0),
        changed_asset_count: Some(0),
        error_message: None,
        started_at: started_at.clone(),
        completed_at: None,
        metadata_json: Some(
            serde_json::json!({
                "task_id": task.id,
                "task_name": task.name,
                "execution_mode": trigger_source,
                "summary": data.get("summary").cloned().unwrap_or(Value::Null),
            })
            .to_string(),
        ),
    };

    db_service
        .create_surface_discovery_run(&run)
        .await
        .map_err(|error| error.to_string())?;

    let mut observation_count = 0i32;
    let mut changed_target_count = 0i32;
    let mut first_error: Option<String> = None;

    for result in results {
        let Some(object) = result.as_object() else {
            continue;
        };
        let Some(base_url) = string_value(object, &["baseUrl"]) else {
            continue;
        };

        let added_count = object
            .get("addedApiEndpoints")
            .and_then(Value::as_array)
            .map(|value| value.len())
            .unwrap_or(0);
        let removed_count = object
            .get("removedApiEndpoints")
            .and_then(Value::as_array)
            .map(|value| value.len())
            .unwrap_or(0);
        if added_count > 0 || removed_count > 0 {
            changed_target_count += 1;
        }

        let error_message = string_value(object, &["error"]);
        if first_error.is_none() && error_message.is_some() {
            first_error = error_message.clone();
        }

        let observed_at = object
            .get("snapshot")
            .and_then(Value::as_object)
            .and_then(|snapshot| string_value(snapshot, &["lastChecked"]))
            .unwrap_or_else(|| started_at.clone());

        let observation = SurfaceObservationRow {
            id: Uuid::new_v4().to_string(),
            run_id: run_id.clone(),
            program_id: task.program_id.clone(),
            artifact_type: "api_snapshot".to_string(),
            object_key: Some(base_url),
            payload_json: result.to_string(),
            source_plugin: Some(normalized_plugin_id(&plugin.plugin_id).to_string()),
            confidence_score: Some(if error_message.is_some() { 0.3 } else { 0.95 }),
            observed_at,
            normalized: false,
            metadata_json: Some(
                serde_json::json!({
                    "task_id": task.id,
                    "task_name": task.name,
                    "execution_mode": trigger_source,
                    "added_endpoints_count": added_count,
                    "removed_endpoints_count": removed_count,
                })
                .to_string(),
            ),
        };

        db_service
            .create_surface_observation(&observation)
            .await
            .map_err(|error| error.to_string())?;
        observation_count += 1;
    }

    db_service
        .update_surface_discovery_run(
            &run_id,
            if first_error.is_some() && observation_count == 0 {
                "failed"
            } else if first_error.is_some() {
                "completed_with_errors"
            } else {
                "completed"
            },
            Some(observation_count),
            Some(0),
            Some(changed_target_count),
            first_error.as_deref(),
            Some(&Utc::now().to_rfc3339()),
        )
        .await
        .map_err(|error| error.to_string())?;

    Ok(observation_count as usize)
}
