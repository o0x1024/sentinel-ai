//! Shell tool permission commands

use std::collections::HashMap;
use std::sync::Arc;

use once_cell::sync::Lazy;
use serde::Serialize;
use tokio::sync::RwLock;

use sentinel_tools::buildin_tools::shell::{
    get_shell_config, set_permission_handler, set_shell_config, ShellConfig, ShellPermissionHandler,
};

// Global storage for permission response channels
static SHELL_PERMISSION_SENDERS: Lazy<RwLock<HashMap<String, tokio::sync::oneshot::Sender<bool>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

// Global storage for pending permission requests (for frontend polling)
#[derive(Debug, Clone, Serialize)]
pub struct PendingPermissionRequest {
    pub id: String,
    pub command: String,
    pub timestamp: u64,
}

static PENDING_PERMISSION_REQUESTS: Lazy<RwLock<Vec<PendingPermissionRequest>>> =
    Lazy::new(|| RwLock::new(Vec::new()));

struct ShellPermissionImpl {
    app: tauri::AppHandle,
}

#[async_trait::async_trait]
impl ShellPermissionHandler for ShellPermissionImpl {
    async fn check_permission(&self, command: &str) -> bool {
        let id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = tokio::sync::oneshot::channel();

        {
            let mut senders = SHELL_PERMISSION_SENDERS.write().await;
            senders.insert(id.clone(), tx);
        }

        // Store pending request
        {
            let mut pending = PENDING_PERMISSION_REQUESTS.write().await;
            pending.push(PendingPermissionRequest {
                id: id.clone(),
                command: command.to_string(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            });
        }

        // Emit event to frontend
        use tauri::Emitter;
        tracing::info!(
            "Requesting permission for command: {} (id: {})",
            command,
            id
        );
        if let Err(e) = self.app.emit(
            "shell-permission-request",
            serde_json::json!({
                "id": id,
                "command": command
            }),
        ) {
            tracing::error!("Failed to emit permission request: {}", e);
            // Clean up pending request
            let mut pending = PENDING_PERMISSION_REQUESTS.write().await;
            pending.retain(|r| r.id != id);
            return false;
        }

        // Wait for response with timeout (e.g. 5 minutes)
        let result = match tokio::time::timeout(std::time::Duration::from_secs(300), rx).await {
            Ok(Ok(allowed)) => {
                tracing::info!("Permission response for {}: {}", id, allowed);
                allowed
            }
            Ok(Err(_)) => {
                tracing::warn!("Permission channel dropped for {}", id);
                false
            }
            Err(_) => {
                tracing::warn!("Permission request timed out for {}", id);
                false
            }
        };

        // Clean up pending request
        {
            let mut pending = PENDING_PERMISSION_REQUESTS.write().await;
            pending.retain(|r| r.id != id);
        }

        result
    }
}

/// Initialize the shell permission handler
pub async fn init_shell_permission_handler(app: tauri::AppHandle) -> Result<(), String> {
    set_permission_handler(Arc::new(ShellPermissionImpl { app })).await;
    Ok(())
}

/// Get shell tool configuration
pub async fn get_shell_tool_config() -> Result<ShellConfig, String> {
    Ok(get_shell_config().await)
}

/// Set shell tool configuration (deprecated, use save_agent_config instead)
pub async fn set_shell_tool_config(config: ShellConfig) -> Result<(), String> {
    set_shell_config(config).await;
    Ok(())
}

/// Respond to a shell permission request
pub async fn respond_shell_permission(id: String, allowed: bool) -> Result<(), String> {
    tracing::info!(
        "Responding to shell permission: id={}, allowed={}",
        id,
        allowed
    );

    // Remove from pending requests
    {
        let mut pending = PENDING_PERMISSION_REQUESTS.write().await;
        let before_len = pending.len();
        pending.retain(|r| r.id != id);
        tracing::info!(
            "Removed from pending: before={}, after={}",
            before_len,
            pending.len()
        );
    }

    let mut senders = SHELL_PERMISSION_SENDERS.write().await;
    tracing::info!(
        "Available senders: {:?}",
        senders.keys().collect::<Vec<_>>()
    );

    if let Some(tx) = senders.remove(&id) {
        let send_result = tx.send(allowed);
        tracing::info!("Sent permission response: {:?}", send_result);
        Ok(())
    } else {
        tracing::warn!("Request ID {} not found in senders map", id);
        Err(format!("Request ID {} not found or already handled", id))
    }
}

/// Get all pending shell permission requests (for frontend polling)
pub async fn get_pending_shell_permissions() -> Result<Vec<PendingPermissionRequest>, String> {
    let pending = PENDING_PERMISSION_REQUESTS.read().await;
    Ok(pending.clone())
}


