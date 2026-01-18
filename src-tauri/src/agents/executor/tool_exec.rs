//! Tool execution helpers.

use anyhow::Result;

use sentinel_tools::get_tool_server;

/// Execute builtin tool.
pub async fn execute_builtin_tool(
    tool_name: &str,
    arguments: &serde_json::Value,
) -> Result<String> {
    let tool_server = get_tool_server();
    tool_server.init_builtin_tools().await;

    let result = tool_server.execute(tool_name, arguments.clone()).await;

    if result.success {
        Ok(result
            .output
            .map(|v| serde_json::to_string_pretty(&v).unwrap_or_default())
            .unwrap_or_else(|| "Tool executed successfully".to_string()))
    } else {
        Err(anyhow::anyhow!(
            "Tool execution failed: {}",
            result.error.unwrap_or_else(|| "Unknown error".to_string())
        ))
    }
}

/// Execute workflow tool.
pub async fn execute_workflow_tool(
    workflow_id: &str,
    arguments: &serde_json::Value,
) -> Result<String> {
    let tool_name = format!("workflow::{}", workflow_id);
    let tool_server = get_tool_server();

    let result = tool_server.execute(&tool_name, arguments.clone()).await;

    if result.success {
        Ok(result
            .output
            .map(|v| serde_json::to_string_pretty(&v).unwrap_or_default())
            .unwrap_or_else(|| "Workflow executed successfully".to_string()))
    } else {
        Err(anyhow::anyhow!(
            "Workflow execution failed: {}",
            result.error.unwrap_or_else(|| "Unknown error".to_string())
        ))
    }
}

/// Execute MCP tool.
pub async fn execute_mcp_tool(
    server_name: &str,
    tool_name: &str,
    arguments: &serde_json::Value,
) -> Result<String> {
    let full_name = format!("mcp::{}::{}", server_name, tool_name);
    let tool_server = get_tool_server();

    let result = tool_server.execute(&full_name, arguments.clone()).await;

    if result.success {
        Ok(result
            .output
            .map(|v| serde_json::to_string_pretty(&v).unwrap_or_default())
            .unwrap_or_else(|| "MCP tool executed successfully".to_string()))
    } else {
        Err(anyhow::anyhow!(
            "MCP tool execution failed: {}",
            result.error.unwrap_or_else(|| "Unknown error".to_string())
        ))
    }
}

/// Execute plugin tool.
pub async fn execute_plugin_tool(
    plugin_id: &str,
    arguments: &serde_json::Value,
) -> Result<String> {
    let tool_name = format!("plugin::{}", plugin_id);
    let tool_server = get_tool_server();

    let result = tool_server.execute(&tool_name, arguments.clone()).await;

    if result.success {
        Ok(result
            .output
            .map(|v| serde_json::to_string_pretty(&v).unwrap_or_default())
            .unwrap_or_else(|| "Plugin executed successfully".to_string()))
    } else {
        Err(anyhow::anyhow!(
            "Plugin execution failed: {}",
            result.error.unwrap_or_else(|| "Unknown error".to_string())
        ))
    }
}

