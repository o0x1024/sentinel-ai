use anyhow::Result;
use serde_json::Value;

use crate::commands::mcp_commands::McpConnection;

#[derive(Clone)]
pub struct McpService;

impl McpService {
    pub fn new() -> Self {
        Self
    }

    pub async fn get_connection_info(&self) -> Result<Vec<McpConnection>> {
        // Retrieve active connections from the global state
        let connections = crate::commands::mcp_commands::get_active_mcp_connections().await;
        Ok(connections)
    }

    pub async fn execute_client_tool(
        &self,
        conn_name: &str,
        tool_name: &str,
        params: Value,
    ) -> Result<Value> {
        // Get connection ID first
        let conn_id = crate::commands::mcp_commands::get_connection_id_by_name(conn_name)
            .await
            .ok_or_else(|| anyhow::anyhow!("MCP server '{}' is not connected", conn_name))?;
            
        // Execute tool through the command function
        let result = crate::commands::mcp_commands::mcp_call_tool(
            conn_id,
            tool_name.to_string(),
            params
        )
        .await
        .map_err(|e| anyhow::anyhow!("MCP tool execution failed: {}", e))?;
        
        Ok(result)
    }
}
