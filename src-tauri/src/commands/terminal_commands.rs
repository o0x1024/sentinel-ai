//! Terminal WebSocket server commands

use sentinel_tools::terminal::TerminalServer;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use once_cell::sync::Lazy;

/// Global terminal server
pub static TERMINAL_SERVER: Lazy<Arc<RwLock<Option<Arc<TerminalServer>>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

/// Terminal server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalServerConfig {
    pub host: String,
    pub port: u16,
}

impl Default for TerminalServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8765,
        }
    }
}

/// Start terminal server
#[tauri::command]
pub async fn start_terminal_server(
    config: Option<TerminalServerConfig>,
) -> Result<String, String> {
    let config = config.unwrap_or_default();
    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .map_err(|e| format!("Invalid address: {}", e))?;

    let mut server_guard = TERMINAL_SERVER.write().await;

    if server_guard.is_some() {
        return Err("Terminal server already running".to_string());
    }

    let server = Arc::new(TerminalServer::new(addr));
    *server_guard = Some(server.clone());

    // Start server in background
    tokio::spawn(async move {
        if let Err(e) = server.start().await {
            tracing::error!("Terminal server error: {}", e);
        }
    });

    Ok(format!("Terminal server started on {}", addr))
}

/// Stop terminal server
#[tauri::command]
pub async fn stop_terminal_server() -> Result<String, String> {
    let mut server_guard = TERMINAL_SERVER.write().await;

    if let Some(server) = server_guard.take() {
        server.stop().await;
        Ok("Terminal server stopped".to_string())
    } else {
        Err("Terminal server not running".to_string())
    }
}

/// Get terminal server status
#[tauri::command]
pub async fn get_terminal_server_status() -> Result<TerminalServerStatus, String> {
    let server_guard = TERMINAL_SERVER.read().await;

    if let Some(server) = server_guard.as_ref() {
        let session_count = server.manager().session_count().await;
        Ok(TerminalServerStatus {
            running: true,
            session_count,
        })
    } else {
        Ok(TerminalServerStatus {
            running: false,
            session_count: 0,
        })
    }
}

/// List terminal sessions
#[tauri::command]
pub async fn list_terminal_sessions() -> Result<Vec<sentinel_tools::terminal::SessionInfo>, String> {
    let server_guard = TERMINAL_SERVER.read().await;

    if let Some(server) = server_guard.as_ref() {
        Ok(server.manager().list_sessions().await)
    } else {
        Err("Terminal server not running".to_string())
    }
}

/// Stop a terminal session
#[tauri::command]
pub async fn stop_terminal_session(session_id: String) -> Result<String, String> {
    let server_guard = TERMINAL_SERVER.read().await;

    if let Some(server) = server_guard.as_ref() {
        server.manager().stop_session(&session_id).await?;
        Ok(format!("Session {} stopped", session_id))
    } else {
        Err("Terminal server not running".to_string())
    }
}

/// Get WebSocket URL for terminal
#[tauri::command]
pub async fn get_terminal_websocket_url() -> Result<String, String> {
    let server_guard = TERMINAL_SERVER.read().await;

    if server_guard.is_some() {
        // Default config
        let config = TerminalServerConfig::default();
        Ok(format!("ws://{}:{}", config.host, config.port))
    } else {
        Err("Terminal server not running".to_string())
    }
}

/// Terminal server status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalServerStatus {
    pub running: bool,
    pub session_count: usize,
}

/// Clean up unused containers
#[tauri::command]
pub async fn cleanup_terminal_containers() -> Result<Vec<String>, String> {
    let server_guard = TERMINAL_SERVER.read().await;

    if let Some(server) = server_guard.as_ref() {
        server.manager().cleanup_containers().await
    } else {
        Err("Terminal server not running".to_string())
    }
}

/// Get container info for all sessions
#[tauri::command]
pub async fn get_terminal_container_info() -> Result<Vec<sentinel_tools::terminal::ContainerInfo>, String> {
    let server_guard = TERMINAL_SERVER.read().await;

    if let Some(server) = server_guard.as_ref() {
        Ok(server.manager().get_container_info().await)
    } else {
        Err("Terminal server not running".to_string())
    }
}
