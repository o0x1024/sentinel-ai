//! ToolServer management commands

use std::sync::Arc;
use sentinel_db::Database;

use sentinel_tools::get_tool_server;

/// Initialize the global tool server with builtin tools
#[tauri::command]
pub async fn init_tool_server() -> Result<(), String> {
    let server = get_tool_server();
    server.init_builtin_tools().await;
    tracing::info!(
        "Tool server initialized with {} tools",
        server.tool_count().await
    );
    Ok(())
}

/// List all tools from ToolServer
#[tauri::command]
pub async fn list_tool_server_tools() -> Result<Vec<sentinel_tools::ToolInfo>, String> {
    let server = get_tool_server();
    server.init_builtin_tools().await;
    Ok(server.list_tools().await)
}

/// List tools by source type (builtin, mcp, plugin, workflow)
#[tauri::command]
pub async fn list_tools_by_source(
    source_type: String,
) -> Result<Vec<sentinel_tools::ToolInfo>, String> {
    let server = get_tool_server();
    server.init_builtin_tools().await;
    Ok(server.list_tools_by_source(&source_type).await)
}

/// Get tool info by name
#[tauri::command]
pub async fn get_tool_server_tool(tool_name: String) -> Result<Option<sentinel_tools::ToolInfo>, String> {
    let server = get_tool_server();
    server.init_builtin_tools().await;
    Ok(server.get_tool(&tool_name).await)
}

/// Execute a tool via ToolServer
#[tauri::command]
pub async fn execute_tool_server_tool(
    tool_name: String,
    args: serde_json::Value,
) -> Result<sentinel_tools::ToolResult, String> {
    // License check
    #[cfg(not(debug_assertions))]
    if !sentinel_license::is_licensed() {
        return Ok(sentinel_tools::ToolResult {
            success: false,
            tool_name: tool_name.clone(),
            output: None,
            error: Some("License required for tool execution".to_string()),
            execution_time_ms: 0,
        });
    }

    let server = get_tool_server();
    server.init_builtin_tools().await;
    Ok(server.execute(&tool_name, args).await)
}

/// Get tool server statistics
#[tauri::command]
pub async fn get_tool_server_stats() -> Result<serde_json::Value, String> {
    let server = get_tool_server();
    server.init_builtin_tools().await;

    let tools = server.list_tools().await;
    let builtin_count = tools.iter().filter(|t| t.source == "builtin").count();
    let mcp_count = tools
        .iter()
        .filter(|t| t.source.starts_with("mcp::"))
        .count();
    let plugin_count = tools
        .iter()
        .filter(|t| t.source.starts_with("plugin::"))
        .count();
    let workflow_count = tools
        .iter()
        .filter(|t| t.source.starts_with("workflow::"))
        .count();

    Ok(serde_json::json!({
        "total_tools": tools.len(),
        "builtin_tools": builtin_count,
        "mcp_tools": mcp_count,
        "plugin_tools": plugin_count,
        "workflow_tools": workflow_count,
    }))
}

/// Register MCP tools from a connected server
#[tauri::command]
pub async fn register_mcp_tools_from_server(
    server_name: String,
    tools: Vec<serde_json::Value>,
) -> Result<usize, String> {
    use sentinel_tools::mcp_adapter::{load_mcp_tools_to_server, McpToolMeta};

    let server = get_tool_server();

    // Parse tools
    let tool_metas: Vec<McpToolMeta> = tools
        .into_iter()
        .filter_map(|t| {
            let tool_name = t.get("name")?.as_str()?.to_string();
            let description = t
                .get("description")
                .and_then(|d| d.as_str())
                .map(String::from);
            let input_schema = t
                .get("input_schema")
                .cloned()
                .unwrap_or(serde_json::json!({"type": "object"}));
            let connection_id = t
                .get("connection_id")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();

            Some(McpToolMeta {
                server_name: server_name.clone(),
                connection_id,
                tool_name,
                description,
                input_schema,
            })
        })
        .collect();

    let count = tool_metas.len();
    load_mcp_tools_to_server(&server, &server_name, tool_metas).await;

    tracing::info!(
        "Registered {} MCP tools from server: {}",
        count,
        server_name
    );
    Ok(count)
}

/// Register workflow tools from database
#[tauri::command]
pub async fn register_workflow_tools(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<usize, String> {
    let server = get_tool_server();
    let db = db_service.inner().clone();

    // Load workflows marked as tools
    let workflows = db
        .list_workflow_definitions(false)
        .await
        .map_err(|e| e.to_string())?;

    let mut count = 0;
    for workflow in workflows {
        let is_tool = workflow
            .get("is_tool")
            .and_then(|v: &serde_json::Value| v.as_bool())
            .unwrap_or(false);
        if !is_tool {
            continue;
        }

        let id = workflow.get("id").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("");
        let name = workflow
            .get("name")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("Unknown");
        let description = workflow
            .get("description")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("Workflow tool");

        if id.is_empty() {
            continue;
        }

        // Extract input schema from workflow definition
        let input_schema =
            sentinel_tools::workflow_adapter::WorkflowToolAdapter::extract_input_schema(&workflow, Some(&server)).await;

        // Create executor
        let workflow_id = id.to_string();
        let executor = sentinel_tools::create_executor(move |args| {
            let wid = workflow_id.clone();
            async move {
                // Execute workflow via sentinel-workflow
                Ok(serde_json::json!({
                    "workflow_id": wid,
                    "status": "initiated",
                    "input": args,
                    "message": "Workflow execution started"
                }))
            }
        });

        server
            .register_workflow_tool(id, name, description, input_schema, executor)
            .await;
        count += 1;
    }

    tracing::info!("Registered {} workflow tools", count);
    Ok(count)
}

/// Refresh all dynamic tools (MCP, plugin, workflow)
#[tauri::command]
pub async fn refresh_all_dynamic_tools(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<serde_json::Value, String> {
    let server = get_tool_server();
    server.init_builtin_tools().await;

    // Clear existing dynamic tools
    server.clear_mcp_tools().await;
    server.clear_plugin_tools().await;
    server.clear_workflow_tools().await;

    // Reload workflow tools
    let workflow_count = register_workflow_tools(db_service.clone()).await.unwrap_or(0);

    // Note: MCP and plugin tools need to be registered via their respective commands
    // when servers connect or plugins are enabled

    Ok(serde_json::json!({
        "workflow_tools": workflow_count,
        "message": "Dynamic tools refreshed. MCP and plugin tools will be registered when connected/enabled."
    }))
}


