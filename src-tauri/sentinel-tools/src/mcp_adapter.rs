//! MCP Tool Adapter
//!
//! Adapts MCP (Model Context Protocol) tools to the unified tool system.
//! Supports loading tools from MCP servers and executing them.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::RwLock;
    
use crate::dynamic_tool::{create_executor, DynamicToolDef, ToolExecutor, ToolSource};
use crate::tool_server::ToolServer;

/// MCP tool metadata from server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolMeta {
    pub server_name: String,
    pub connection_id: String,
    pub tool_name: String,
    pub description: Option<String>,
    pub input_schema: Value,
}

/// MCP connection info
#[derive(Debug, Clone)]
pub struct McpConnectionInfo {
    pub connection_id: String,
    pub server_name: String,
    pub command: String,

    pub args: Vec<String>,
    pub tools: Option<Vec<McpToolMeta>>,
}

/// MCP executor context - stored globally to allow tool execution
static MCP_CONNECTIONS: once_cell::sync::Lazy<RwLock<HashMap<String, McpConnectionInfo>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

/// Register MCP connection info
pub async fn register_mcp_connection(info: McpConnectionInfo) {
    let mut connections = MCP_CONNECTIONS.write().await;
    connections.insert(info.server_name.clone(), info);
}

/// Unregister MCP connection
pub async fn unregister_mcp_connection(server_name: &str) {
    let mut connections = MCP_CONNECTIONS.write().await;
    connections.remove(server_name);
}

/// Get MCP connection info
pub async fn get_mcp_connection(server_name: &str) -> Option<McpConnectionInfo> {
    let connections = MCP_CONNECTIONS.read().await;
    connections.get(server_name).cloned()
}

/// Create MCP tool executor (public for external registration)
pub fn create_mcp_tool_executor(server_name: String, tool_name: String) -> ToolExecutor {
    create_executor(move |args: Value| {
        let server = server_name.clone();
        let tool = tool_name.clone();

        async move {
            // Get connection info
            let conn_info = get_mcp_connection(&server)
                .await
                .ok_or_else(|| format!("MCP server '{}' not connected", server))?;

            // Execute via MCP protocol
            execute_mcp_tool_internal(&conn_info, &tool, args).await
        }
    })
}

/// Internal MCP tool execution using rmcp
async fn execute_mcp_tool_internal(
    conn_info: &McpConnectionInfo,
    tool_name: &str,
    args: Value,
) -> Result<Value, String> {
    use rmcp::model::{ClientCapabilities, ClientInfo, Implementation};
    use rmcp::ServiceExt;
    use tokio::process::Command as TokioCommand;

    tracing::info!(
        "Executing MCP tool: {} on server {} (command: {} {:?})",
        tool_name,
        conn_info.server_name,
        conn_info.command,
        conn_info.args
    );

    // Create new connection for this execution
    let mut cmd = TokioCommand::new(&conn_info.command);
    cmd.args(&conn_info.args);

    let transport = rmcp::transport::TokioChildProcess::new(cmd)
        .map_err(|e| format!("Failed to create MCP transport: {}", e))?;

    let client_info = ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "sentinel-ai".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            ..Default::default()
        },
    };

    let client = client_info
        .serve(transport)
        .await
        .map_err(|e| format!("Failed to connect to MCP server: {}", e))?;

    // Convert arguments
    let args_map: Option<serde_json::Map<String, Value>> = if args.is_object() {
        args.as_object().cloned()
    } else {
        None
    };

    // Call the tool
    let result = client
        .call_tool(rmcp::model::CallToolRequestParam {
            name: tool_name.to_string().into(),
            arguments: args_map,
        })
        .await
        .map_err(|e| format!("MCP tool call failed: {}", e))?;

    // Convert result to JSON
    let content_json: Vec<Value> = result
        .content
        .iter()
        .map(|c| match &c.raw {
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
        })
        .collect();

    Ok(serde_json::json!({
        "content": content_json,
        "is_error": result.is_error.unwrap_or(false)
    }))
}

/// Load and register MCP tools from a connected server
pub async fn load_mcp_tools_to_server(
    tool_server: &ToolServer,
    server_name: &str,
    tools: Vec<McpToolMeta>,
) {
    tracing::info!(
        "Loading {} MCP tools from server: {}",
        tools.len(),
        server_name
    );

    for tool_meta in tools {
        let full_name = format!("mcp__{}__{}", server_name, tool_meta.tool_name);
        let description = tool_meta
            .description
            .unwrap_or_else(|| format!("MCP tool from server {}", server_name));

        let executor =
            create_mcp_tool_executor(server_name.to_string(), tool_meta.tool_name.clone());

        tool_server
            .register_mcp_tool(
                server_name,
                &tool_meta.tool_name,
                &description,
                tool_meta.input_schema,
                executor,
            )
            .await;

        tracing::debug!("Registered MCP tool: {}", full_name);
    }
}

/// Refresh MCP tools from all connected servers
pub async fn refresh_mcp_tools(tool_server: &ToolServer) {
    // Clear existing MCP tools
    tool_server.clear_mcp_tools().await;

    // Get all connections info first (clone to release lock)
    let connections_snapshot: Vec<(String, McpConnectionInfo)> = {
        let connections = MCP_CONNECTIONS.read().await;
        connections
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    };

    for (server_name, conn_info) in connections_snapshot {
        // If we have cached tools, use them directly
        if let Some(cached_tools) = &conn_info.tools {
            tracing::info!("Using cached tools for MCP server: {}", server_name);
            for tool_meta in cached_tools {
                let executor =
                    create_mcp_tool_executor(server_name.clone(), tool_meta.tool_name.clone());

                tool_server
                    .register_mcp_tool(
                       &server_name,
                       &tool_meta.tool_name,
                       tool_meta.description.as_deref().unwrap_or_default(),
                       tool_meta.input_schema.clone(),
                       executor,
                    )
                    .await;
            }
            continue;
        }

        // Fallback to loading from server (reconnect)
        match load_tools_from_mcp_server(&conn_info).await {
            Ok(tools) => {
                for tool_meta in tools {
                    let executor =
                        create_mcp_tool_executor(server_name.clone(), tool_meta.tool_name.clone());

                    tool_server
                        .register_mcp_tool(
                            &server_name,
                            &tool_meta.tool_name,
                            &tool_meta.description.unwrap_or_default(),
                            tool_meta.input_schema,
                            executor,
                        )
                        .await;
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to load tools from MCP server {}: {}",
                    server_name,
                    e
                );
            }
        }
    }
}

/// Load tools from a single MCP server
async fn load_tools_from_mcp_server(
    conn_info: &McpConnectionInfo,
) -> Result<Vec<McpToolMeta>, String> {
    use rmcp::model::{ClientCapabilities, ClientInfo, Implementation};
    use rmcp::ServiceExt;
    use tokio::process::Command as TokioCommand;

    let mut cmd = TokioCommand::new(&conn_info.command);
    cmd.args(&conn_info.args);

    let transport = rmcp::transport::TokioChildProcess::new(cmd)
        .map_err(|e| format!("Failed to create transport: {}", e))?;

    let client_info = ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "sentinel-ai".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            ..Default::default()
        },
    };

    let client = client_info
        .serve(transport)
        .await
        .map_err(|e| format!("Failed to connect: {}", e))?;

    let tools_result = client
        .list_tools(Default::default())
        .await
        .map_err(|e| format!("Failed to list tools: {}", e))?;

    let tools: Vec<McpToolMeta> = tools_result
        .tools
        .into_iter()
        .map(|tool| McpToolMeta {
            server_name: conn_info.server_name.clone(),
            connection_id: conn_info.connection_id.clone(),
            tool_name: tool.name.to_string(),
            description: tool.description.map(|d| d.to_string()),
            input_schema: serde_json::to_value(&*tool.input_schema).unwrap_or_default(),
        })
        .collect();

    Ok(tools)
}

/// MCP tool adapter - wrapper for creating MCP tool definitions
pub struct McpToolAdapter;

impl McpToolAdapter {
    /// Create a DynamicToolDef from MCP tool metadata
    pub fn create_tool_def(meta: &McpToolMeta) -> DynamicToolDef {
        let server_name = meta.server_name.clone();
        let tool_name = meta.tool_name.clone();
        let full_name = format!("mcp__{}__{}", server_name, tool_name);

        DynamicToolDef {
            name: full_name,
            description: meta.description.clone().unwrap_or_else(|| {
                format!("MCP tool '{}' from server '{}'", tool_name, server_name)
            }),
            input_schema: meta.input_schema.clone(),
            source: ToolSource::Mcp {
                server_name: server_name.clone(),
            },
            executor: create_mcp_tool_executor(server_name, tool_name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_connection_registry() {
        let info = McpConnectionInfo {
            connection_id: "test-id".to_string(),
            server_name: "test-server".to_string(),
            command: "echo".to_string(),
            args: vec!["hello".to_string()],
            tools: None,
        };

        register_mcp_connection(info.clone()).await;

        let retrieved = get_mcp_connection("test-server").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().connection_id, "test-id");

        unregister_mcp_connection("test-server").await;
        assert!(get_mcp_connection("test-server").await.is_none());
    }
}
