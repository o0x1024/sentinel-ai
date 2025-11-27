//! Plugin review and management commands for Tauri

use tauri::State;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use chrono::Utc;

use crate::generators::{GeneratedPlugin, PluginStatus};
use crate::services::database::DatabaseService;

/// Response for plugin review operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginReviewResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// Get plugins for review (from plugin_registry table)
#[tauri::command]
pub async fn get_plugins_for_review(
    db: State<'_, Arc<DatabaseService>>,
) -> Result<PluginReviewResponse, String> {
    log::info!("Getting plugins for review from plugin_registry");
    
    // Query from plugin_registry table
    match db.inner().get_plugins_from_registry().await {
        Ok(plugins) => {
            log::info!("Found {} plugins in registry", plugins.len());
            Ok(PluginReviewResponse {
                success: true,
                message: format!("Found {} plugins", plugins.len()),
                data: Some(serde_json::to_value(&plugins).unwrap_or(serde_json::json!([]))),
            })
        }
        Err(e) => {
            log::error!("Failed to get plugins from registry: {}", e);
            Ok(PluginReviewResponse {
                success: false,
                message: format!("Failed to get plugins: {}", e),
                data: Some(serde_json::json!([])),
            })
        }
    }
}

/// List all generated plugins (legacy, use get_plugins_for_review instead)
#[tauri::command]
pub async fn list_generated_plugins(
    db: State<'_, Arc<DatabaseService>>,
) -> Result<PluginReviewResponse, String> {
    log::info!("Listing all generated plugins");
    
    // Redirect to get_plugins_for_review
    get_plugins_for_review(db).await
}

/// Get plugin detail by ID
#[tauri::command]
pub async fn get_plugin_detail(
    plugin_id: String,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<PluginReviewResponse, String> {
    log::info!("Getting plugin detail: {}", plugin_id);
    
    // TODO: Query from database
    
    Ok(PluginReviewResponse {
        success: true,
        message: "Plugin found".to_string(),
        data: None,
    })
}

/// Approve a plugin
#[tauri::command]
pub async fn approve_plugin(
    plugin_id: String,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<PluginReviewResponse, String> {
    log::info!("Approving plugin: {}", plugin_id);
    
    // Update plugin status in database
    match db.inner().update_plugin_status(&plugin_id, "Approved").await {
        Ok(_) => {
            log::info!("Plugin {} approved successfully", plugin_id);
            Ok(PluginReviewResponse {
                success: true,
                message: "Plugin approved successfully".to_string(),
                data: Some(serde_json::json!({
                    "plugin_id": plugin_id,
                    "status": "Approved",
                    "updated_at": Utc::now().to_rfc3339(),
                })),
            })
        }
        Err(e) => {
            log::error!("Failed to approve plugin {}: {}", plugin_id, e);
            Ok(PluginReviewResponse {
                success: false,
                message: format!("Failed to approve plugin: {}", e),
                data: None,
            })
        }
    }
}

/// Reject a plugin
#[tauri::command]
pub async fn reject_plugin(
    plugin_id: String,
    reason: String,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<PluginReviewResponse, String> {
    log::info!("Rejecting plugin: {} (reason: {})", plugin_id, reason);
    
    // Update plugin status in database
    match db.inner().update_plugin_status(&plugin_id, "Rejected").await {
        Ok(_) => {
            log::info!("Plugin {} rejected successfully", plugin_id);
            Ok(PluginReviewResponse {
                success: true,
                message: "Plugin rejected".to_string(),
                data: Some(serde_json::json!({
                    "plugin_id": plugin_id,
                    "status": "Rejected",
                    "reason": reason,
                    "updated_at": Utc::now().to_rfc3339(),
                })),
            })
        }
        Err(e) => {
            log::error!("Failed to reject plugin {}: {}", plugin_id, e);
            Ok(PluginReviewResponse {
                success: false,
                message: format!("Failed to reject plugin: {}", e),
                data: None,
            })
        }
    }
}

/// Update plugin code
#[tauri::command]
pub async fn review_update_plugin_code(
    plugin_id: String,
    code: String,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<PluginReviewResponse, String> {
    log::info!("Updating plugin code: {}", plugin_id);
    
    // Update plugin code in database
    match db.inner().update_plugin_code(&plugin_id, &code).await {
        Ok(_) => {
            log::info!("Plugin {} code updated successfully", plugin_id);
            Ok(PluginReviewResponse {
                success: true,
                message: "Plugin code updated".to_string(),
                data: Some(serde_json::json!({
                    "plugin_id": plugin_id,
                    "code_length": code.len(),
                    "updated_at": Utc::now().to_rfc3339(),
                })),
            })
        }
        Err(e) => {
            log::error!("Failed to update plugin {} code: {}", plugin_id, e);
            Ok(PluginReviewResponse {
                success: false,
                message: format!("Failed to update plugin code: {}", e),
                data: None,
            })
        }
    }
}

/// Batch approve plugins
#[tauri::command]
pub async fn batch_approve_plugins(
    plugin_ids: Vec<String>,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<PluginReviewResponse, String> {
    log::info!("Batch approving {} plugins", plugin_ids.len());
    
    let mut approved_count = 0;
    let mut failed_ids: Vec<String> = vec![];
    
    for plugin_id in plugin_ids {
        match db.inner().update_plugin_status(&plugin_id, "Approved").await {
            Ok(_) => {
                approved_count += 1;
                log::info!("Plugin {} approved", plugin_id);
            }
            Err(e) => {
                log::error!("Failed to approve plugin {}: {}", plugin_id, e);
                failed_ids.push(plugin_id);
            }
        }
    }
    
    Ok(PluginReviewResponse {
        success: true,
        message: format!("Approved {} plugins", approved_count),
        data: Some(serde_json::json!({
            "approved_count": approved_count,
            "failed_ids": failed_ids,
        })),
    })
}

/// Batch reject plugins
#[tauri::command]
pub async fn batch_reject_plugins(
    plugin_ids: Vec<String>,
    reason: String,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<PluginReviewResponse, String> {
    log::info!("Batch rejecting {} plugins", plugin_ids.len());
    
    let mut rejected_count = 0;
    let mut failed_ids: Vec<String> = vec![];
    
    for plugin_id in plugin_ids {
        match db.inner().update_plugin_status(&plugin_id, "Rejected").await {
            Ok(_) => {
                rejected_count += 1;
                log::info!("Plugin {} rejected", plugin_id);
            }
            Err(e) => {
                log::error!("Failed to reject plugin {}: {}", plugin_id, e);
                failed_ids.push(plugin_id);
            }
        }
    }
    
    Ok(PluginReviewResponse {
        success: true,
        message: format!("Rejected {} plugins", rejected_count),
        data: Some(serde_json::json!({
            "rejected_count": rejected_count,
            "failed_ids": failed_ids,
            "reason": reason,
        })),
    })
}

/// Get plugin statistics
#[tauri::command]
pub async fn get_plugin_statistics(
    db: State<'_, Arc<DatabaseService>>,
) -> Result<PluginReviewResponse, String> {
    log::info!("Getting plugin statistics");
    
    // TODO: Query statistics from database
    
    Ok(PluginReviewResponse {
        success: true,
        message: "Statistics retrieved".to_string(),
        data: Some(serde_json::json!({
            "total": 0,
            "pending_review": 0,
            "approved": 0,
            "rejected": 0,
            "validation_failed": 0,
            "average_quality": 0.0,
        })),
    })
}

/// Search plugins
#[tauri::command]
pub async fn search_plugins(
    query: String,
    filters: Option<serde_json::Value>,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<PluginReviewResponse, String> {
    log::info!("Searching plugins: {}", query);
    
    // TODO: Search in database
    
    Ok(PluginReviewResponse {
        success: true,
        message: "Search completed".to_string(),
        data: Some(serde_json::json!({
            "plugins": [],
            "total": 0,
        })),
    })
}

/// Export plugin
#[tauri::command]
pub async fn export_plugin(
    plugin_id: String,
    format: String, // "ts" or "json"
    db: State<'_, Arc<DatabaseService>>,
) -> Result<PluginReviewResponse, String> {
    log::info!("Exporting plugin: {} as {}", plugin_id, format);
    
    // Get plugin from database
    match db.inner().get_plugin_from_registry(&plugin_id).await {
        Ok(plugin) => {
            log::info!("Plugin {} found for export", plugin_id);
            Ok(PluginReviewResponse {
                success: true,
                message: "Plugin exported".to_string(),
                data: Some(serde_json::to_value(&plugin).unwrap_or(serde_json::json!(null))),
            })
        }
        Err(e) => {
            log::error!("Failed to get plugin {} for export: {}", plugin_id, e);
            Ok(PluginReviewResponse {
                success: false,
                message: format!("Failed to get plugin for export: {}", e),
                data: None,
            })
        }
    }
}

/// Delete plugin
#[tauri::command]
pub async fn review_delete_plugin(
    plugin_id: String,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<PluginReviewResponse, String> {
    log::info!("Deleting plugin: {}", plugin_id);
    
    // Delete from database
    match db.inner().delete_plugin_from_registry(&plugin_id).await {
        Ok(_) => {
            log::info!("Plugin {} deleted successfully", plugin_id);
            Ok(PluginReviewResponse {
                success: true,
                message: "Plugin deleted".to_string(),
                data: Some(serde_json::json!({
                    "plugin_id": plugin_id,
                    "deleted_at": Utc::now().to_rfc3339(),
                })),
            })
        }
        Err(e) => {
            log::error!("Failed to delete plugin {}: {}", plugin_id, e);
            Ok(PluginReviewResponse {
                success: false,
                message: format!("Failed to delete plugin: {}", e),
                data: None,
            })
        }
    }
}

/// Get plugins with pagination
#[tauri::command]
pub async fn get_plugins_paginated(
    page: i64,
    page_size: i64,
    status_filter: Option<String>,
    search_text: Option<String>,
    user_id: Option<String>,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<PluginReviewResponse, String> {
    log::info!("Getting plugins paginated: page={}, page_size={}, status={:?}, search={:?}", 
        page, page_size, status_filter, search_text);
    
    match db.inner().get_plugins_paginated(
        page,
        page_size,
        status_filter.as_deref(),
        search_text.as_deref(),
        user_id.as_deref(),
    ).await {
        Ok(result) => {
            Ok(PluginReviewResponse {
                success: true,
                message: "Plugins retrieved".to_string(),
                data: Some(result),
            })
        }
        Err(e) => {
            log::error!("Failed to get plugins paginated: {}", e);
            Ok(PluginReviewResponse {
                success: false,
                message: format!("Failed to get plugins: {}", e),
                data: None,
            })
        }
    }
}

/// Toggle plugin favorite status
#[tauri::command]
pub async fn toggle_plugin_favorite(
    plugin_id: String,
    user_id: Option<String>,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<PluginReviewResponse, String> {
    log::info!("Toggling favorite for plugin: {}", plugin_id);
    
    match db.inner().toggle_plugin_favorite(&plugin_id, user_id.as_deref()).await {
        Ok(is_favorited) => {
            log::info!("Plugin {} favorite status: {}", plugin_id, is_favorited);
            Ok(PluginReviewResponse {
                success: true,
                message: if is_favorited { "Plugin favorited".to_string() } else { "Plugin unfavorited".to_string() },
                data: Some(serde_json::json!({
                    "plugin_id": plugin_id,
                    "is_favorited": is_favorited,
                })),
            })
        }
        Err(e) => {
            log::error!("Failed to toggle favorite for plugin {}: {}", plugin_id, e);
            Ok(PluginReviewResponse {
                success: false,
                message: format!("Failed to toggle favorite: {}", e),
                data: None,
            })
        }
    }
}

/// Get favorited plugins
#[tauri::command]
pub async fn get_favorited_plugins(
    user_id: Option<String>,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<PluginReviewResponse, String> {
    log::info!("Getting favorited plugins");
    
    match db.inner().get_favorited_plugins(user_id.as_deref()).await {
        Ok(plugin_ids) => {
            Ok(PluginReviewResponse {
                success: true,
                message: format!("Found {} favorited plugins", plugin_ids.len()),
                data: Some(serde_json::json!({
                    "plugin_ids": plugin_ids,
                })),
            })
        }
        Err(e) => {
            log::error!("Failed to get favorited plugins: {}", e);
            Ok(PluginReviewResponse {
                success: false,
                message: format!("Failed to get favorited plugins: {}", e),
                data: None,
            })
        }
    }
}

/// Get plugin review statistics
#[tauri::command]
pub async fn get_plugin_review_statistics(
    db: State<'_, Arc<DatabaseService>>,
) -> Result<PluginReviewResponse, String> {
    log::info!("Getting plugin review statistics");
    
    match db.inner().get_plugin_review_stats().await {
        Ok(stats) => {
            Ok(PluginReviewResponse {
                success: true,
                message: "Statistics retrieved".to_string(),
                data: Some(stats),
            })
        }
        Err(e) => {
            log::error!("Failed to get plugin review statistics: {}", e);
            Ok(PluginReviewResponse {
                success: false,
                message: format!("Failed to get statistics: {}", e),
                data: None,
            })
        }
    }
}
