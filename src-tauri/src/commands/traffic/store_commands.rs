use sentinel_db::Database;
use sentinel_plugins::{PluginCategory, PluginMainCategory, PluginMetadata, Severity};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use super::plugin_commands::{
    is_agent_tool_plugin_main_category, refresh_active_agent_plugin_tools,
    resolved_store_plugin_monitor_type,
};
use super::TrafficAnalysisState;
use crate::commands::command_response_support::CommandResponse;
use crate::events::{emit_plugin_changed, PluginChangedEvent};
use crate::services::ensure_plugin_catalog_write_access;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorePluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub main_category: String,
    pub category: String,
    pub description: String,
    pub default_severity: String,
    pub tags: Vec<String>,
    pub download_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorePluginListResponse {
    pub success: bool,
    pub plugins: Vec<StorePluginInfo>,
    pub error: Option<String>,
}

fn plugin_download_candidates(download_url: &str) -> Vec<String> {
    vec![download_url.to_string()]
}

async fn download_plugin_source(
    client: &reqwest::Client,
    download_url: &str,
) -> Result<String, String> {
    let candidates = plugin_download_candidates(download_url);
    let mut last_error = None;

    for candidate in candidates {
        match client.get(&candidate).send().await {
            Ok(resp) if resp.status().is_success() => {
                return resp
                    .text()
                    .await
                    .map_err(|e| format!("Failed to read response: {}", e));
            }
            Ok(resp) => {
                let error = format!("HTTP {}", resp.status());
                if resp.status() == reqwest::StatusCode::NOT_FOUND {
                    tracing::warn!(
                        "Plugin download candidate not found: {} ({})",
                        candidate,
                        error
                    );
                    last_error = Some(error);
                    continue;
                }
                return Err(error);
            }
            Err(e) => return Err(e.to_string()),
        }
    }

    Err(last_error.unwrap_or_else(|| "HTTP 404 Not Found".to_string()))
}

#[tauri::command]
pub async fn fetch_store_plugins(repo_url: String) -> Result<StorePluginListResponse, String> {
    tracing::info!("Fetching store plugins from: {}", repo_url);

    let parts: Vec<&str> = repo_url.trim_end_matches('/').split('/').collect();

    if parts.len() < 2 {
        return Ok(StorePluginListResponse {
            success: false,
            plugins: vec![],
            error: Some("Invalid repository URL".to_string()),
        });
    }

    let owner = parts[parts.len() - 2];
    let repo = parts[parts.len() - 1];
    let api_url = format!(
        "https://raw.githubusercontent.com/{}/{}/main/plugins.json",
        owner, repo
    );

    tracing::info!("Fetching plugin manifest from: {}", api_url);

    let builder = reqwest::Client::builder().timeout(std::time::Duration::from_secs(30));
    let builder = sentinel_core::global_proxy::apply_proxy_to_client(builder).await;
    let client = builder
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client.get(&api_url).send().await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let text = resp.text().await.map_err(|e| e.to_string())?;

                match serde_json::from_str::<serde_json::Value>(&text) {
                    Ok(manifest) => {
                        let mut plugins = Vec::new();

                        if let Some(plugin_list) =
                            manifest.get("plugins").and_then(|v| v.as_array())
                        {
                            tracing::info!("Found {} plugins in manifest", plugin_list.len());
                            for plugin_value in plugin_list {
                                match serde_json::from_value::<StorePluginInfo>(
                                    plugin_value.clone(),
                                ) {
                                    Ok(mut plugin) => {
                                        if plugin.download_url.is_empty() {
                                            plugin.download_url = format!(
                                                "https://raw.githubusercontent.com/{}/{}/main/plugins/{}.ts",
                                                owner, repo, plugin.id
                                            );
                                        }
                                        plugins.push(plugin);
                                    }
                                    Err(e) => {
                                        let plugin_id = plugin_value
                                            .get("id")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("unknown");
                                        tracing::warn!(
                                            "Failed to parse plugin '{}': {}",
                                            plugin_id,
                                            e
                                        );
                                    }
                                }
                            }
                        }

                        tracing::info!("Fetched {} plugins from store", plugins.len());

                        Ok(StorePluginListResponse {
                            success: true,
                            plugins,
                            error: None,
                        })
                    }
                    Err(e) => {
                        tracing::error!("Failed to parse plugin manifest: {}", e);
                        Ok(StorePluginListResponse {
                            success: false,
                            plugins: vec![],
                            error: Some(format!("Failed to parse manifest: {}", e)),
                        })
                    }
                }
            } else {
                tracing::error!("Failed to fetch manifest: HTTP {}", resp.status());
                Ok(StorePluginListResponse {
                    success: false,
                    plugins: vec![],
                    error: Some(format!("HTTP error: {}", resp.status())),
                })
            }
        }
        Err(e) => {
            tracing::error!("Failed to fetch manifest: {}", e);
            Ok(StorePluginListResponse {
                success: false,
                plugins: vec![],
                error: Some(format!("Network error: {}", e)),
            })
        }
    }
}

#[tauri::command]
pub async fn fetch_plugin_code(download_url: String) -> Result<serde_json::Value, String> {
    tracing::info!("Fetching plugin code from: {}", download_url);

    let builder = reqwest::Client::builder().timeout(std::time::Duration::from_secs(30));
    let builder = sentinel_core::global_proxy::apply_proxy_to_client(builder).await;
    let client = builder
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    match download_plugin_source(&client, &download_url).await {
        Ok(code) => Ok(serde_json::json!({
            "success": true,
            "code": code
        })),
        Err(error) => Ok(serde_json::json!({
            "success": false,
            "error": error
        })),
    }
}

#[tauri::command]
pub async fn install_store_plugin(
    app: AppHandle,
    state: State<'_, TrafficAnalysisState>,
    plugin: StorePluginInfo,
) -> Result<CommandResponse<String>, String> {
    ensure_plugin_catalog_write_access()?;

    tracing::info!("Installing store plugin: {} ({})", plugin.name, plugin.id);

    let builder = reqwest::Client::builder().timeout(std::time::Duration::from_secs(30));
    let builder = sentinel_core::global_proxy::apply_proxy_to_client(builder).await;
    let client = builder
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let plugin_code = match download_plugin_source(&client, &plugin.download_url).await {
        Ok(code) => code,
        Err(error) => {
            return Ok(CommandResponse::err(format!(
                "Failed to download plugin: {}",
                error
            )));
        }
    };

    let severity = match plugin.default_severity.to_lowercase().as_str() {
        "critical" => Severity::Critical,
        "high" => Severity::High,
        "medium" => Severity::Medium,
        "low" => Severity::Low,
        "info" => Severity::Info,
        _ => Severity::Medium,
    };

    let metadata = PluginMetadata {
        id: plugin.id.clone(),
        name: plugin.name.clone(),
        version: plugin.version,
        author: Some(plugin.author),
        category: PluginCategory::parse_for_main_category(
            PluginMainCategory::parse(&plugin.main_category)?,
            &plugin.category,
        )?,
        main_category: PluginMainCategory::parse(&plugin.main_category)?,
        monitor_type: None,
        description: Some(plugin.description),
        default_severity: severity,
        tags: plugin.tags,
        target_asset_types: Vec::new(),
    };

    let db = state.get_db_service();
    let existing_plugin = db
        .get_plugin_from_registry(&plugin.id)
        .await
        .map_err(|e| format!("Failed to read existing plugin metadata: {}", e))?;

    use sentinel_db::TrafficPluginMetadata;
    let traffic_metadata = TrafficPluginMetadata {
        id: metadata.id.clone(),
        name: metadata.name.clone(),
        version: metadata.version.clone(),
        author: metadata.author.clone(),
        main_category: metadata.main_category.to_string(),
        category: metadata.category.to_string(),
        description: metadata.description.clone(),
        default_severity: format!("{}", metadata.default_severity),
        tags: metadata.tags.clone(),
    };

    db.register_traffic_plugin_with_code(&traffic_metadata, &plugin_code)
        .await
        .map_err(|e| format!("Failed to install plugin: {}", e))?;

    db.update_plugin_enabled(&plugin.id, true)
        .await
        .map_err(|e| format!("Failed to enable installed plugin: {}", e))?;

    let metadata_json = serde_json::to_value(&PluginMetadata {
        monitor_type: resolved_store_plugin_monitor_type(
            existing_plugin.as_ref(),
            &metadata.id,
            metadata.main_category,
            &metadata.category,
        ),
        target_asset_types: existing_plugin
            .as_ref()
            .map(|record| record.metadata.target_asset_types.clone())
            .unwrap_or_default(),
        ..metadata.clone()
    })
    .map_err(|e| format!("Failed to serialize plugin metadata: {}", e))?;
    db.update_plugin(&metadata_json, &plugin_code)
        .await
        .map_err(|e| format!("Failed to persist plugin metadata: {}", e))?;

    tracing::info!("Plugin installed: {}", plugin.id);

    let plugin_manager = state.get_plugin_manager();
    if let Err(e) = plugin_manager
        .set_plugin_code(plugin.id.clone(), plugin_code)
        .await
    {
        tracing::warn!("Failed to update plugin cache: {}", e);
    }

    if is_agent_tool_plugin_main_category(metadata.main_category) {
        let refreshed = refresh_active_agent_plugin_tools(db.as_ref()).await?;
        tracing::info!(
            "Refreshed {} active agent plugin tools after installing {}",
            refreshed,
            plugin.id
        );
    }

    emit_plugin_changed(
        &app,
        PluginChangedEvent {
            plugin_id: plugin.id.clone(),
            enabled: true,
            name: plugin.name.clone(),
        },
    );

    Ok(CommandResponse::ok(plugin.id))
}

#[tauri::command]
pub async fn update_store_plugin(
    app: AppHandle,
    state: State<'_, TrafficAnalysisState>,
    plugin: StorePluginInfo,
) -> Result<CommandResponse<String>, String> {
    ensure_plugin_catalog_write_access()?;

    tracing::info!("Updating store plugin: {} ({})", plugin.name, plugin.id);

    let builder = reqwest::Client::builder().timeout(std::time::Duration::from_secs(30));
    let builder = sentinel_core::global_proxy::apply_proxy_to_client(builder).await;
    let client = builder
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let plugin_code = match download_plugin_source(&client, &plugin.download_url).await {
        Ok(code) => code,
        Err(error) => {
            return Ok(CommandResponse::err(format!(
                "Failed to download plugin: {}",
                error
            )));
        }
    };

    let severity = match plugin.default_severity.to_lowercase().as_str() {
        "critical" => Severity::Critical,
        "high" => Severity::High,
        "medium" => Severity::Medium,
        "low" => Severity::Low,
        "info" => Severity::Info,
        _ => Severity::Medium,
    };

    let metadata = PluginMetadata {
        id: plugin.id.clone(),
        name: plugin.name.clone(),
        version: plugin.version,
        author: Some(plugin.author),
        category: PluginCategory::parse_for_main_category(
            PluginMainCategory::parse(&plugin.main_category)?,
            &plugin.category,
        )?,
        main_category: PluginMainCategory::parse(&plugin.main_category)?,
        monitor_type: None,
        description: Some(plugin.description),
        default_severity: severity,
        tags: plugin.tags,
        target_asset_types: Vec::new(),
    };

    let db = state.get_db_service();
    let existing_plugin = db
        .get_plugin_from_registry(&plugin.id)
        .await
        .map_err(|e| format!("Failed to read existing plugin metadata: {}", e))?;

    use sentinel_db::TrafficPluginMetadata;
    let traffic_metadata = TrafficPluginMetadata {
        id: metadata.id.clone(),
        name: metadata.name.clone(),
        version: metadata.version.clone(),
        author: metadata.author.clone(),
        main_category: metadata.main_category.to_string(),
        category: metadata.category.to_string(),
        description: metadata.description.clone(),
        default_severity: format!("{}", metadata.default_severity),
        tags: metadata.tags.clone(),
    };

    db.update_traffic_plugin(&traffic_metadata, &plugin_code)
        .await
        .map_err(|e| format!("Failed to update plugin: {}", e))?;

    let metadata_json = serde_json::to_value(&PluginMetadata {
        monitor_type: resolved_store_plugin_monitor_type(
            existing_plugin.as_ref(),
            &metadata.id,
            metadata.main_category,
            &metadata.category,
        ),
        target_asset_types: existing_plugin
            .as_ref()
            .map(|record| record.metadata.target_asset_types.clone())
            .unwrap_or_default(),
        ..metadata.clone()
    })
    .map_err(|e| format!("Failed to serialize plugin metadata: {}", e))?;
    db.update_plugin(&metadata_json, &plugin_code)
        .await
        .map_err(|e| format!("Failed to persist plugin metadata: {}", e))?;

    tracing::info!("Plugin updated: {}", plugin.id);

    let plugin_manager = state.get_plugin_manager();
    if let Err(e) = plugin_manager
        .set_plugin_code(plugin.id.clone(), plugin_code)
        .await
    {
        tracing::warn!("Failed to update plugin cache: {}", e);
    }

    if is_agent_tool_plugin_main_category(metadata.main_category) {
        let refreshed = refresh_active_agent_plugin_tools(db.as_ref()).await?;
        tracing::info!(
            "Refreshed {} active agent plugin tools after updating {}",
            refreshed,
            plugin.id
        );
    }

    emit_plugin_changed(
        &app,
        PluginChangedEvent {
            plugin_id: plugin.id.clone(),
            enabled: true,
            name: plugin.name.clone(),
        },
    );

    Ok(CommandResponse::ok(plugin.id))
}
