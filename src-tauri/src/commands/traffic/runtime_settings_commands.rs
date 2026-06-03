use sentinel_plugins::{
    get_plugin_request_queue_stats, PluginFetchPolicyKind, PluginRequestQueueStats,
    PluginRuntimeSettings,
};
use serde::{Deserialize, Serialize};
use tauri::State;

use super::TrafficAnalysisState;
use crate::commands::command_response_support::CommandResponse;

pub const TRAFFIC_PLUGIN_RUNTIME_SETTINGS_KEY: &str = "traffic_plugin_runtime_settings";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetTrafficPluginRuntimeSettingsPayload {
    pub settings: PluginRuntimeSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficPluginRuntimeQueueStatsSnapshot {
    pub active_probe: PluginRequestQueueStats,
    pub bounty_fetch: PluginRequestQueueStats,
    pub monitor_fetch: PluginRequestQueueStats,
    pub agent_fetch: PluginRequestQueueStats,
    pub plugin_test_fetch: PluginRequestQueueStats,
}

#[tauri::command]
pub async fn get_traffic_plugin_runtime_settings(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<PluginRuntimeSettings>, String> {
    let db = state.get_db_service();
    let settings = match db
        .load_proxy_config(TRAFFIC_PLUGIN_RUNTIME_SETTINGS_KEY)
        .await
    {
        Ok(Some(raw)) => serde_json::from_str::<PluginRuntimeSettings>(&raw)
            .map(|value| value.sanitized())
            .unwrap_or_default(),
        Ok(None) => PluginRuntimeSettings::default(),
        Err(error) => {
            return Err(format!(
                "Failed to load traffic plugin runtime settings: {error}"
            ));
        }
    };

    sentinel_plugins::set_plugin_runtime_settings(settings.clone());
    Ok(CommandResponse::ok(settings))
}

#[tauri::command]
pub async fn set_traffic_plugin_runtime_settings(
    state: State<'_, TrafficAnalysisState>,
    payload: SetTrafficPluginRuntimeSettingsPayload,
) -> Result<CommandResponse<PluginRuntimeSettings>, String> {
    let db = state.get_db_service();
    let settings = payload.settings.sanitized();
    let raw = serde_json::to_string(&settings)
        .map_err(|error| format!("Failed to serialize traffic plugin runtime settings: {error}"))?;

    db.save_proxy_config(TRAFFIC_PLUGIN_RUNTIME_SETTINGS_KEY, &raw)
        .await
        .map_err(|error| format!("Failed to save traffic plugin runtime settings: {error}"))?;

    sentinel_plugins::set_plugin_runtime_settings(settings.clone());
    Ok(CommandResponse::ok(settings))
}

#[tauri::command]
pub async fn get_traffic_plugin_runtime_queue_stats(
) -> Result<CommandResponse<TrafficPluginRuntimeQueueStatsSnapshot>, String> {
    Ok(CommandResponse::ok(
        TrafficPluginRuntimeQueueStatsSnapshot {
            active_probe: get_plugin_request_queue_stats(PluginFetchPolicyKind::TrafficActiveProbe),
            bounty_fetch: get_plugin_request_queue_stats(PluginFetchPolicyKind::BountyFetch),
            monitor_fetch: get_plugin_request_queue_stats(PluginFetchPolicyKind::MonitorFetch),
            agent_fetch: get_plugin_request_queue_stats(PluginFetchPolicyKind::AgentFetch),
            plugin_test_fetch: get_plugin_request_queue_stats(
                PluginFetchPolicyKind::PluginTestFetch,
            ),
        },
    ))
}
