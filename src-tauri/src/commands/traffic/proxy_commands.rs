use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use super::analysis_commands::{start_traffic_analysis_internal, stop_traffic_analysis_internal};
use super::TrafficAnalysisState;
use crate::commands::command_response_support::CommandResponse;
use sentinel_traffic::ProxyConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyListenerConfig {
    pub host: String,
    pub port: u16,
    pub mitm_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryPersistenceConfig {
    pub enable_auto_persistence: bool,
    pub auto_persistence_threshold: usize,
    pub auto_persistence_interval: u64,
}

impl Default for HistoryPersistenceConfig {
    fn default() -> Self {
        Self {
            enable_auto_persistence: true,
            auto_persistence_threshold: 1000,
            auto_persistence_interval: 300,
        }
    }
}

fn default_listener_config(port: u16) -> ProxyConfig {
    ProxyConfig {
        start_port: port,
        max_port_attempts: 1,
        mitm_enabled: true,
        max_request_body_size: 2 * 1024 * 1024,
        max_response_body_size: 2 * 1024 * 1024,
        mitm_bypass_fail_threshold: 3,
        upstream_proxy: None,
        exclude_self_traffic: true,
        scope_include_rules: Vec::new(),
        scope_exclude_rules: Vec::new(),
        match_replace_rules: Vec::new(),
    }
}

#[tauri::command]
pub async fn start_proxy_listener(
    app: AppHandle,
    state: State<'_, TrafficAnalysisState>,
    host: String,
    port: u16,
    index: usize,
) -> Result<CommandResponse<String>, String> {
    tracing::info!(
        "Starting proxy listener on {}:{} (index: {})",
        host,
        port,
        index
    );

    if *state.is_running.read().await {
        tracing::info!("Proxy already running, listener request acknowledged");
        return Ok(CommandResponse::ok(format!(
            "Listener {}:{} is already running",
            host, port
        )));
    }

    let db = state.get_db_service();
    let mut config = match db.load_proxy_config("proxy_config").await {
        Ok(Some(config_json)) => match serde_json::from_str::<ProxyConfig>(&config_json) {
            Ok(config) => {
                tracing::info!(
                    "Loaded proxy configuration from database for listener: {:?}",
                    config
                );
                config
            }
            Err(e) => {
                tracing::warn!("Failed to deserialize config, using default: {}", e);
                default_listener_config(port)
            }
        },
        Ok(None) => {
            tracing::info!("No saved configuration found, using default");
            default_listener_config(port)
        }
        Err(e) => {
            tracing::warn!("Failed to load config from database, using default: {}", e);
            default_listener_config(port)
        }
    };

    config.start_port = port;
    config.max_port_attempts = 1;

    match start_traffic_analysis_internal(&app, &state, Some(config)).await {
        Ok(_) => {
            tracing::info!("Proxy listener started successfully on {}:{}", host, port);
            Ok(CommandResponse::ok(format!(
                "Listener started on {}:{}",
                host, port
            )))
        }
        Err(e) => {
            tracing::error!("Failed to start proxy listener: {}", e);
            Ok(CommandResponse::err(e))
        }
    }
}

#[tauri::command]
pub async fn stop_proxy_listener(
    app: AppHandle,
    state: State<'_, TrafficAnalysisState>,
    index: usize,
) -> Result<CommandResponse<String>, String> {
    tracing::info!("Stopping proxy listener at index: {}", index);

    match stop_traffic_analysis_internal(&app, &state).await {
        Ok(_) => {
            tracing::info!("Proxy listener stopped successfully");
            Ok(CommandResponse::ok("Listener stopped".to_string()))
        }
        Err(e) => {
            tracing::error!("Failed to stop proxy listener: {}", e);
            Ok(CommandResponse::err(e))
        }
    }
}

#[tauri::command]
pub async fn save_proxy_config(
    state: State<'_, TrafficAnalysisState>,
    config: ProxyConfig,
) -> Result<CommandResponse<()>, String> {
    tracing::info!("Saving proxy configuration: {:?}", config);

    let db = state.get_db_service();
    let mut config_to_persist = config.clone();
    config_to_persist.scope_include_rules = Vec::new();
    config_to_persist.scope_exclude_rules = Vec::new();
    let config_json = serde_json::to_string(&config_to_persist).map_err(|e| {
        tracing::error!("Failed to serialize config: {}", e);
        format!("Failed to serialize config: {}", e)
    })?;

    db.save_proxy_config("proxy_config", &config_json)
        .await
        .map_err(|e| {
            tracing::error!("Failed to save config to database: {}", e);
            format!("Failed to save config: {}", e)
        })?;

    {
        let mut exclude_self = state.exclude_self_traffic.write().await;
        *exclude_self = config.exclude_self_traffic;
        tracing::info!(
            "Updated exclude_self_traffic to: {}",
            config.exclude_self_traffic
        );
    }
    {
        let mut scope_include_rules = state.scope_include_rules.write().await;
        *scope_include_rules = config.scope_include_rules.clone();
        tracing::info!(
            "Updated traffic scope include rules to: {:?}",
            config.scope_include_rules
        );
    }
    {
        let mut scope_exclude_rules = state.scope_exclude_rules.write().await;
        *scope_exclude_rules = config.scope_exclude_rules.clone();
        tracing::info!(
            "Updated traffic scope exclude rules to: {:?}",
            config.scope_exclude_rules
        );
    }
    {
        let mut match_replace_rules = state.match_replace_rules.write().await;
        *match_replace_rules = config.match_replace_rules.clone();
        tracing::info!(
            "Updated traffic match-replace rules to: {}",
            match_replace_rules.len()
        );
    }

    tracing::info!("Proxy configuration saved successfully");
    Ok(CommandResponse::ok(()))
}

#[tauri::command]
pub async fn get_proxy_config(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<ProxyConfig>, String> {
    tracing::info!("Getting proxy configuration");

    let db = state.get_db_service();
    let config = match db.load_proxy_config("proxy_config").await {
        Ok(Some(config_json)) => match serde_json::from_str::<ProxyConfig>(&config_json) {
            Ok(config) => {
                tracing::debug!("Loaded proxy configuration from database: {:?}", config);
                config
            }
            Err(e) => {
                tracing::warn!("Failed to deserialize config, using default: {}", e);
                ProxyConfig::default()
            }
        },
        Ok(None) => {
            tracing::info!("No saved configuration found, using default");
            ProxyConfig::default()
        }
        Err(e) => {
            tracing::warn!("Failed to load config from database, using default: {}", e);
            ProxyConfig::default()
        }
    };

    Ok(CommandResponse::ok(config))
}

#[tauri::command]
pub async fn set_proxy_auto_start(
    state: State<'_, TrafficAnalysisState>,
    enabled: bool,
) -> Result<CommandResponse<()>, String> {
    tracing::info!("Setting proxy auto-start to: {}", enabled);

    let db = state.get_db_service();
    db.save_proxy_config("proxy_auto_start_enabled", &enabled.to_string())
        .await
        .map_err(|e| {
            tracing::error!("Failed to save proxy auto-start config: {}", e);
            format!("Failed to save config: {}", e)
        })?;

    tracing::info!("Proxy auto-start configuration saved successfully");
    Ok(CommandResponse::ok(()))
}

#[tauri::command]
pub async fn get_proxy_auto_start(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<bool>, String> {
    let db = state.get_db_service();

    let enabled = match db.load_proxy_config("proxy_auto_start_enabled").await {
        Ok(Some(value)) => value.parse::<bool>().unwrap_or(false),
        _ => false,
    };

    Ok(CommandResponse::ok(enabled))
}

#[tauri::command]
pub async fn set_traffic_analysis_plugin_enabled(
    state: State<'_, TrafficAnalysisState>,
    enabled: bool,
) -> Result<CommandResponse<()>, String> {
    tracing::info!("Setting traffic analysis plugin scanning to: {}", enabled);

    {
        let mut plugin_scanning = state.plugin_scanning_enabled.write().await;
        *plugin_scanning = enabled;
    }

    let db = state.get_db_service();
    db.save_proxy_config("traffic_analysis_plugin_enabled", &enabled.to_string())
        .await
        .map_err(|e| {
            tracing::error!(
                "Failed to save traffic analysis plugin enabled config: {}",
                e
            );
            format!("Failed to save config: {}", e)
        })?;

    tracing::info!("Traffic analysis plugin enabled configuration saved successfully");
    Ok(CommandResponse::ok(()))
}

#[tauri::command]
pub async fn get_traffic_analysis_plugin_enabled(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<bool>, String> {
    let db = state.get_db_service();

    let enabled = match db
        .load_proxy_config("traffic_analysis_plugin_enabled")
        .await
    {
        Ok(Some(value)) => value.parse::<bool>().unwrap_or(true),
        _ => true,
    };

    Ok(CommandResponse::ok(enabled))
}

#[tauri::command]
pub async fn set_history_persistence_config(
    state: State<'_, TrafficAnalysisState>,
    config: HistoryPersistenceConfig,
) -> Result<CommandResponse<()>, String> {
    tracing::info!("Setting history persistence config: {:?}", config);

    let db = state.get_db_service();
    let json =
        serde_json::to_string(&config).map_err(|e| format!("Failed to serialize config: {}", e))?;

    db.save_proxy_config("history_persistence_config", &json)
        .await
        .map_err(|e| format!("Failed to save config: {}", e))?;

    tracing::info!("History persistence config saved successfully");
    Ok(CommandResponse::ok(()))
}

#[tauri::command]
pub async fn get_history_persistence_config(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<HistoryPersistenceConfig>, String> {
    let db = state.get_db_service();

    let config = match db.load_proxy_config("history_persistence_config").await {
        Ok(Some(json)) => {
            serde_json::from_str::<HistoryPersistenceConfig>(&json).unwrap_or_default()
        }
        _ => HistoryPersistenceConfig::default(),
    };

    Ok(CommandResponse::ok(config))
}
