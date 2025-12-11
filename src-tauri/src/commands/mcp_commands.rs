//! MCP Commands - Tauri commands for MCP server management
//!
//! Uses rmcp library to connect to MCP servers via stdio.

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::process::Command as TokioCommand;
use tokio::sync::RwLock;

use sentinel_core::models::database::McpServerConfig;
use sentinel_db::DatabaseService;

use rmcp::model::{ClientCapabilities, ClientInfo, Implementation};
use rmcp::ServiceExt;

/// MCP connection info for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConnection {
    pub db_id: String,
    pub id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub transport_type: String,
    pub endpoint: String,
    pub status: String,
    pub command: String,
    pub args: Vec<String>,
}

/// MCP tool info for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolInfo {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

/// Active MCP connection state (without holding the client)
#[derive(Debug, Clone)]
struct ActiveMcpConnection {
    pub connection_id: String,
    pub name: String,
    pub status: String,
    pub command: String,
    pub args: Vec<String>,
    pub tools: Vec<McpToolInfo>,
    pub process_id: Option<u32>,
}

/// Global state for active MCP connections
static ACTIVE_CONNECTIONS: Lazy<RwLock<HashMap<String, ActiveMcpConnection>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Create rmcp client info
fn create_client_info() -> ClientInfo {
    ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "sentinel-ai".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            ..Default::default()
        },
    }
}

/// Get all MCP server configurations from database and merge with active connection status
#[tauri::command]
pub async fn mcp_get_connections(
    db: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<McpConnection>, String> {
    // Get all configs from database
    let configs = db
        .get_all_mcp_server_configs()
        .await
        .map_err(|e| format!("Failed to get MCP configs: {}", e))?;

    // Get active connections
    let active = ACTIVE_CONNECTIONS.read().await;

    // Map to frontend format
    let connections: Vec<McpConnection> = configs
        .into_iter()
        .map(|config| {
            // Parse args from JSON string
            let args: Vec<String> = serde_json::from_str(&config.args).unwrap_or_default();

            // Check if there's an active connection for this server
            let (id, status) = if let Some(active_conn) = active.get(&config.name) {
                (
                    Some(active_conn.connection_id.clone()),
                    active_conn.status.clone(),
                )
            } else {
                (None, "Disconnected".to_string())
            };

            McpConnection {
                db_id: config.id,
                id,
                name: config.name,
                description: config.description,
                transport_type: config.connection_type,
                endpoint: config.url,
                status,
                command: config.command,
                args,
            }
        })
        .collect();

    Ok(connections)
}

/// Get connection status map
#[tauri::command]
pub async fn mcp_get_connection_status() -> Result<HashMap<String, String>, String> {
    let active = ACTIVE_CONNECTIONS.read().await;
    tracing::info!(
        "mcp_get_connection_status: active connections count: {}",
        active.len()
    );
    let status_map: HashMap<String, String> = active
        .iter()
        .map(|(name, conn)| {
            tracing::info!("  Active connection: {} -> {}", name, conn.status);
            (name.clone(), conn.status.clone())
        })
        .collect();
    Ok(status_map)
}

/// Helper to get connection ID by server name (used by internal services)
pub async fn get_connection_id_by_name(name: &str) -> Option<String> {
    let active = ACTIVE_CONNECTIONS.read().await;
    active.get(name).map(|c| c.connection_id.clone())
}

/// Get all active MCP connections (used by internal services like VisionExplorer)
pub async fn get_active_mcp_connections() -> Vec<McpConnection> {
    let active = ACTIVE_CONNECTIONS.read().await;
    active
        .values()
        .map(|conn| McpConnection {
            db_id: String::new(), // Not available from active connection
            id: Some(conn.connection_id.clone()),
            name: conn.name.clone(),
            description: None,
            transport_type: "stdio".to_string(),
            endpoint: String::new(),
            status: conn.status.clone(),
            command: conn.command.clone(),
            args: conn.args.clone(),
        })
        .collect()
}

/// Connect to an MCP server via stdio (child process)
#[tauri::command]
pub async fn add_child_process_mcp_server(
    name: String,
    command: String,
    args: Vec<String>,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<String, String> {
    tracing::info!(
        "Connecting to MCP server: {} (command: {} {:?})",
        name,
        command,
        args
    );

    // Check if already connected
    {
        let active = ACTIVE_CONNECTIONS.read().await;
        if active.contains_key(&name) {
            return Err(format!("Server {} is already connected", name));
        }
    }

    // Generate a connection ID
    let connection_id = uuid::Uuid::new_v4().to_string();

    // Create the transport using TokioCommand
    let mut cmd = TokioCommand::new(&command);
    cmd.args(&args);

    let transport = rmcp::transport::TokioChildProcess::new(cmd)
        .map_err(|e| format!("Failed to create transport: {}", e))?;

    // Get process ID before we move the transport
    let process_id = transport.id();

    // Connect using rmcp client
    let client_info = create_client_info();
    let client = client_info
        .serve(transport)
        .await
        .map_err(|e| format!("Failed to connect to MCP server: {}", e))?;

    // Get server info
    let server_info = client.peer_info();
    tracing::info!("Connected to MCP server: {:?}", server_info);

    // List tools from the server
    let tools_result = client
        .list_tools(Default::default())
        .await
        .map_err(|e| format!("Failed to list tools: {}", e))?;

    let tools: Vec<McpToolInfo> = tools_result
        .tools
        .into_iter()
        .map(|tool| McpToolInfo {
            name: tool.name.to_string(),
            description: tool.description.map(|d| d.to_string()),
            input_schema: serde_json::to_value(&*tool.input_schema).unwrap_or_default(),
        })
        .collect();

    tracing::info!("MCP server {} has {} tools", name, tools.len());
    for tool in &tools {
        tracing::info!("  Tool: {} - {:?}", tool.name, tool.description);
    }

    // Store the active connection state (client will be dropped but we keep the info)
    let active_conn = ActiveMcpConnection {
        connection_id: connection_id.clone(),
        name: name.clone(),
        status: "Connected".to_string(),
        command: command.clone(),
        args: args.clone(),
        tools: tools.clone(),
        process_id,
    };

    {
        let mut active = ACTIVE_CONNECTIONS.write().await;
        active.insert(name.clone(), active_conn);
    }

    // Convert tools to McpToolMeta for caching
    let tool_metas: Vec<sentinel_tools::mcp_adapter::McpToolMeta> = tools
        .iter()
        .map(|t| sentinel_tools::mcp_adapter::McpToolMeta {
            server_name: name.clone(),
            connection_id: connection_id.clone(),
            tool_name: t.name.clone(),
            description: t.description.clone(),
            input_schema: t.input_schema.clone(),
        })
        .collect();

    // 同时注册到 mcp_adapter 的全局状态，以便 refresh_mcp_tools 能正确工作
    sentinel_tools::mcp_adapter::register_mcp_connection(
        sentinel_tools::mcp_adapter::McpConnectionInfo {
            connection_id: connection_id.clone(),
            server_name: name.clone(),
            command: command.clone(),
            args: args.clone(),
            tools: Some(tool_metas),
        },
    )
    .await;

    // 将工具注册到全局 ToolServer
    let tool_server = sentinel_tools::get_tool_server();
    for tool in &tools {
        let input_schema = tool.input_schema.clone();
        let executor =
            sentinel_tools::mcp_adapter::create_mcp_tool_executor(name.clone(), tool.name.clone());
        tool_server
            .register_mcp_tool(
                &name,
                &tool.name,
                tool.description.as_deref().unwrap_or("MCP tool"),
                input_schema,
                executor,
            )
            .await;
        tracing::debug!(
            "Registered MCP tool to ToolServer: mcp::{}::{}",
            name,
            tool.name
        );
    }

    // Update auto_connect in database
    if let Ok(Some(config)) = db.get_mcp_server_config_by_name(&name).await {
        if let Err(e) = db.update_mcp_server_auto_connect(&config.id, true).await {
            tracing::warn!("Failed to update auto_connect for server {}: {}", name, e);
        }
    }

    // Note: We intentionally drop the client here.
    // For tool execution, we'll reconnect each time (stateless approach).
    // This is simpler and avoids lifetime/ownership issues.
    drop(client);

    tracing::info!("MCP server {} connected with id: {}", name, connection_id);

    Ok(connection_id)
}

/// Disconnect from an MCP server
#[tauri::command]
pub async fn mcp_disconnect_server(
    connection_id: String,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    let mut active = ACTIVE_CONNECTIONS.write().await;

    // Find and remove by connection_id
    let name_to_remove = active
        .iter()
        .find(|(_, conn)| conn.connection_id == connection_id)
        .map(|(name, _)| name.clone());

    if let Some(name) = name_to_remove {
        active.remove(&name);

        // Update auto_connect in database
        if let Ok(Some(config)) = db.get_mcp_server_config_by_name(&name).await {
            if let Err(e) = db.update_mcp_server_auto_connect(&config.id, false).await {
                tracing::warn!("Failed to update auto_connect for server {}: {}", name, e);
            }
        }

        tracing::info!("MCP server disconnected: {} (id: {})", name, connection_id);
        Ok(())
    } else {
        Err(format!("Connection {} not found", connection_id))
    }
}

/// Set auto_connect for an MCP server
#[tauri::command]
pub async fn mcp_set_auto_connect(
    db_id: String,
    auto_connect: bool,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    db.update_mcp_server_auto_connect(&db_id, auto_connect)
        .await
        .map_err(|e| format!("Failed to update auto_connect: {}", e))?;

    tracing::info!(
        "Set auto_connect={} for MCP server: {}",
        auto_connect,
        db_id
    );
    Ok(())
}

/// Auto-connect MCP servers that have auto_connect=true
pub async fn mcp_auto_connect_servers(db: Arc<DatabaseService>, app: AppHandle) {
    tracing::info!("Auto-connecting MCP servers...");

    let configs = match db.get_auto_connect_mcp_servers().await {
        Ok(configs) => configs,
        Err(e) => {
            tracing::error!("Failed to get auto-connect MCP servers: {}", e);
            return;
        }
    };

    for config in configs {
        let args: Vec<String> = serde_json::from_str(&config.args).unwrap_or_default();

        // Check if already connected
        {
            let active = ACTIVE_CONNECTIONS.read().await;
            if active.contains_key(&config.name) {
                tracing::info!("MCP server {} is already connected, skipping", config.name);
                continue;
            }
        }

        tracing::info!(
            "Auto-connecting MCP server: {} (command: {} {:?})",
            config.name,
            config.command,
            args
        );

        // Create the transport using TokioCommand
        let mut cmd = TokioCommand::new(&config.command);
        cmd.args(&args);

        let transport = match rmcp::transport::TokioChildProcess::new(cmd) {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("Failed to create transport for {}: {}", config.name, e);
                continue;
            }
        };

        let process_id = transport.id();
        let client_info = create_client_info();

        match client_info.serve(transport).await {
            Ok(client) => {
                // List tools
                let tools = match client.list_tools(Default::default()).await {
                    Ok(result) => result
                        .tools
                        .into_iter()
                        .map(|tool| McpToolInfo {
                            name: tool.name.to_string(),
                            description: tool.description.map(|d| d.to_string()),
                            input_schema: serde_json::to_value(&*tool.input_schema)
                                .unwrap_or_default(),
                        })
                        .collect(),
                    Err(e) => {
                        tracing::warn!("Failed to list tools for {}: {}", config.name, e);
                        Vec::new()
                    }
                };

                let connection_id = uuid::Uuid::new_v4().to_string();
                let active_conn = ActiveMcpConnection {
                    connection_id: connection_id.clone(),
                    name: config.name.clone(),
                    status: "Connected".to_string(),
                    command: config.command.clone(),
                    args: args.clone(),
                    tools: tools.clone(),
                    process_id,
                };

                {
                    let mut active = ACTIVE_CONNECTIONS.write().await;
                    active.insert(config.name.clone(), active_conn);
                }

                // Convert tools to McpToolMeta for caching
                let tool_metas: Vec<sentinel_tools::mcp_adapter::McpToolMeta> = tools
                    .iter()
                    .map(|t| sentinel_tools::mcp_adapter::McpToolMeta {
                        server_name: config.name.clone(),
                        connection_id: connection_id.clone(),
                        tool_name: t.name.clone(),
                        description: t.description.clone(),
                        input_schema: t.input_schema.clone(),
                    })
                    .collect();

                // 同时注册到 mcp_adapter 的全局状态
                sentinel_tools::mcp_adapter::register_mcp_connection(
                    sentinel_tools::mcp_adapter::McpConnectionInfo {
                        connection_id: connection_id.clone(),
                        server_name: config.name.clone(),
                        command: config.command.clone(),
                        args: args.clone(),
                        tools: Some(tool_metas),
                    },
                )
                .await;

                // 将工具注册到全局 ToolServer
                let tool_server = sentinel_tools::get_tool_server();
                for tool in &tools {
                    let input_schema = tool.input_schema.clone();
                    let executor = sentinel_tools::mcp_adapter::create_mcp_tool_executor(
                        config.name.clone(),
                        tool.name.clone(),
                    );
                    tool_server
                        .register_mcp_tool(
                            &config.name,
                            &tool.name,
                            tool.description.as_deref().unwrap_or("MCP tool"),
                            input_schema,
                            executor,
                        )
                        .await;
                    tracing::debug!(
                        "Registered MCP tool to ToolServer: mcp::{}::{}",
                        config.name,
                        tool.name
                    );
                }

                drop(client);
                tracing::info!("Auto-connected MCP server: {}", config.name);

                // Notify frontend to update status
                let _ = app.emit(
                    "mcp:tools-changed",
                    serde_json::json!({
                        "action": "server_connected",
                        "serverName": config.name.clone()
                    }),
                );
            }
            Err(e) => {
                tracing::error!("Failed to auto-connect MCP server {}: {}", config.name, e);
            }
        }
    }

    tracing::info!("Auto-connect MCP servers completed");
}

/// Delete MCP server configuration from database
#[tauri::command]
pub async fn mcp_delete_server_config(
    db_id: String,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    db.delete_mcp_server_config(&db_id)
        .await
        .map_err(|e| format!("Failed to delete MCP config: {}", e))?;

    tracing::info!("Deleted MCP server config: {}", db_id);
    Ok(())
}

/// Update MCP server configuration
#[tauri::command]
pub async fn mcp_update_server_config(
    payload: McpConnection,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    db.update_mcp_server_config(
        &payload.db_id,
        &payload.name,
        payload.description.as_deref(),
        &payload.command,
        &payload.args,
        true, // enabled by default when updating
    )
    .await
    .map_err(|e| format!("Failed to update MCP config: {}", e))?;

    tracing::info!("Updated MCP server config: {}", payload.name);
    Ok(())
}

/// Get tools from a connected MCP server
#[tauri::command]
pub async fn mcp_get_connection_tools(connection_id: String) -> Result<Vec<McpToolInfo>, String> {
    tracing::info!("Getting tools for connection: {}", connection_id);

    let active = ACTIVE_CONNECTIONS.read().await;

    // Find connection by id
    let connection = active
        .iter()
        .find(|(_, conn)| conn.connection_id == connection_id)
        .map(|(_, conn)| conn);

    match connection {
        Some(conn) => {
            tracing::info!(
                "Found {} tools for connection {}",
                conn.tools.len(),
                connection_id
            );
            Ok(conn.tools.clone())
        }
        None => {
            tracing::warn!("Connection {} not found", connection_id);
            Err(format!("Connection {} not found", connection_id))
        }
    }
}

/// Call a tool on a connected MCP server (reconnects for each call)
#[tauri::command]
pub async fn mcp_call_tool(
    connection_id: String,
    tool_name: String,
    arguments: serde_json::Value,
) -> Result<serde_json::Value, String> {
    tracing::info!("Calling tool {} on connection {}", tool_name, connection_id);

    // Get connection info
    let conn_info = {
        let active = ACTIVE_CONNECTIONS.read().await;
        active
            .iter()
            .find(|(_, conn)| conn.connection_id == connection_id)
            .map(|(_, conn)| (conn.command.clone(), conn.args.clone()))
    };

    let (command, args) =
        conn_info.ok_or_else(|| format!("Connection {} not found", connection_id))?;

    // Reconnect to execute the tool
    let mut cmd = TokioCommand::new(&command);
    cmd.args(&args);

    let transport = rmcp::transport::TokioChildProcess::new(cmd)
        .map_err(|e| format!("Failed to create transport: {}", e))?;

    let client_info = create_client_info();
    let client = client_info
        .serve(transport)
        .await
        .map_err(|e| format!("Failed to connect to MCP server: {}", e))?;

    // Convert arguments to the format expected by rmcp
    let args_map: Option<serde_json::Map<String, serde_json::Value>> = if arguments.is_object() {
        arguments.as_object().cloned()
    } else {
        None
    };

    // Call the tool
    let result = client
        .call_tool(rmcp::model::CallToolRequestParam {
            name: tool_name.clone().into(),
            arguments: args_map,
        })
        .await
        .map_err(|e| format!("Failed to call tool: {}", e))?;

    // Convert result to JSON
    let content_json: Vec<serde_json::Value> = result
        .content
        .iter()
        .map(|c| {
            // Access the raw content through Annotated wrapper
            match &c.raw {
                rmcp::model::RawContent::Text(text) => serde_json::json!({
                    "type": "text",
                    "text": text.text
                }),
                rmcp::model::RawContent::Image(img) => serde_json::json!({
                    "type": "image",
                    "data": img.data,
                    "mime_type": img.mime_type
                }),
                rmcp::model::RawContent::Audio(audio) => serde_json::json!({
                    "type": "audio",
                    "data": audio.data,
                    "mime_type": audio.mime_type
                }),
                rmcp::model::RawContent::Resource(res) => serde_json::json!({
                    "type": "resource",
                    "resource": res.resource
                }),
                rmcp::model::RawContent::ResourceLink(link) => serde_json::json!({
                    "type": "resource_link",
                    "uri": link.uri
                }),
            }
        })
        .collect();

    let response = serde_json::json!({
        "content": content_json,
        "is_error": result.is_error.unwrap_or(false)
    });

    Ok(response)
}

/// Test an MCP server tool (alias for mcp_call_tool)
#[tauri::command]
pub async fn mcp_test_server_tool(
    connection_id: String,
    tool_name: String,
    args: serde_json::Value,
) -> Result<serde_json::Value, String> {
    mcp_call_tool(connection_id, tool_name, args).await
}

/// Quick create MCP server
#[tauri::command]
pub async fn quick_create_mcp_server(
    config: QuickCreateConfig,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<String, String> {
    // Parse params to get command and args
    let params = config.params.trim();
    let parts: Vec<&str> = params.split_whitespace().collect();

    let (command, args) = if parts.is_empty() {
        (config.name.clone(), vec![])
    } else {
        (
            parts[0].to_string(),
            parts[1..].iter().map(|s| s.to_string()).collect(),
        )
    };

    let id = db
        .create_mcp_server_config(
            &config.name,
            if config.description.is_empty() {
                None
            } else {
                Some(&config.description)
            },
            &command,
            &args,
        )
        .await
        .map_err(|e| format!("Failed to create MCP config: {}", e))?;

    tracing::info!("Created MCP server config: {} (id: {})", config.name, id);
    Ok(id)
}

#[derive(Debug, Clone, Deserialize)]
pub struct QuickCreateConfig {
    pub enabled: bool,
    pub name: String,
    pub description: String,
    #[serde(rename = "type")]
    pub server_type: String,
    pub params: String,
    #[serde(rename = "envVars")]
    pub env_vars: String,
    pub timeout: u64,
    #[serde(rename = "providerName")]
    pub provider_name: String,
    #[serde(rename = "providerWebsite")]
    pub provider_website: String,
    #[serde(rename = "logoUrl")]
    pub logo_url: String,
}

/// Import MCP servers from JSON config
#[tauri::command]
pub async fn import_mcp_servers_from_json(
    json_config: String,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<usize, String> {
    let config: serde_json::Value =
        serde_json::from_str(&json_config).map_err(|e| format!("Invalid JSON: {}", e))?;

    let mut count = 0;

    // Handle mcpServers format (from Claude Desktop config)
    if let Some(servers) = config.get("mcpServers").and_then(|v| v.as_object()) {
        for (name, server_config) in servers {
            let command = server_config
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let args: Vec<String> = server_config
                .get("args")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            if !command.is_empty() {
                let _ = db
                    .create_mcp_server_config(name, Some("Imported from JSON"), &command, &args)
                    .await;
                count += 1;
            }
        }
    }

    tracing::info!("Imported {} MCP servers from JSON", count);
    Ok(count)
}

/// Cleanup duplicate MCP servers
#[tauri::command]
pub async fn cleanup_duplicate_mcp_servers(
    db: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<String>, String> {
    let configs = db
        .get_all_mcp_server_configs()
        .await
        .map_err(|e| format!("Failed to get configs: {}", e))?;

    let mut seen_names: HashMap<String, String> = HashMap::new();
    let mut removed: Vec<String> = vec![];

    for config in configs {
        if let Some(existing_id) = seen_names.get(&config.name) {
            // This is a duplicate, remove the older one
            let _ = db.delete_mcp_server_config(existing_id).await;
            removed.push(config.name.clone());
        }
        seen_names.insert(config.name.clone(), config.id);
    }

    tracing::info!("Cleaned up {} duplicate MCP servers", removed.len());
    Ok(removed)
}

/// Get all available MCP tools from all connected servers
#[tauri::command]
pub async fn mcp_get_all_tools() -> Result<Vec<serde_json::Value>, String> {
    let active = ACTIVE_CONNECTIONS.read().await;

    let mut all_tools = Vec::new();
    for (server_name, conn) in active.iter() {
        for tool in &conn.tools {
            all_tools.push(serde_json::json!({
                "server_name": server_name,
                "connection_id": conn.connection_id,
                "name": tool.name,
                "description": tool.description,
                "input_schema": tool.input_schema,
            }));
        }
    }

    Ok(all_tools)
}
