//! Tool commands - Tauri commands for builtin tools and workflow tools

use std::collections::HashMap;
use std::sync::Arc;

use once_cell::sync::Lazy;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use sentinel_tools::buildin_tools::shell::ShellConfig;
use sentinel_tools::buildin_tools::{
    AskUserQuestionTool, BrowserTool, CloseAgentTool, FileEditTool, FileReadTool, FileWriteTool,
    GlobTool, GrepTool, HttpRequestTool, ListAgentsTool, LspTool, OcrTool, PluginAuthoringTool,
    RouteDiscoveryTool, SearchExploitTool, ShellTool, SkillsTool, SpawnAgentTool, TasksTool,
    TenthManTool, ToolSearchTool, WaitAgentsTool,
};
use sentinel_tools::get_tool_server;
use sentinel_tools::terminal::server::TerminalServer;

use crate::agents::tool_router::{
    clear_tool_usage_records, get_tool_usage_statistics, ToolCategory, ToolMetadata, ToolRouter,
    ToolStatistics, ToolUsageStatistics,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinToolInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub version: String,
    pub enabled: bool,
    pub input_schema: Option<serde_json::Value>,
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionResult {
    pub success: bool,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

/// Global state for tool enabled/disabled status
static TOOL_STATES: Lazy<RwLock<HashMap<String, bool>>> = Lazy::new(|| {
    let mut map = HashMap::new();
    // All tools enabled by default
    map.insert(HttpRequestTool::NAME.to_string(), true);
    map.insert(AskUserQuestionTool::NAME.to_string(), true);
    map.insert(BrowserTool::NAME.to_string(), true);
    map.insert(RouteDiscoveryTool::NAME.to_string(), true);
    map.insert(ShellTool::NAME.to_string(), true);
    map.insert(GlobTool::NAME.to_string(), true);
    map.insert(GrepTool::NAME.to_string(), true);
    map.insert(FileReadTool::NAME.to_string(), true);
    map.insert(FileEditTool::NAME.to_string(), true);
    map.insert(FileWriteTool::NAME.to_string(), true);
    map.insert(LspTool::NAME.to_string(), true);
    map.insert(ToolSearchTool::NAME.to_string(), true);
    map.insert(SkillsTool::NAME.to_string(), true);
    map.insert(
        sentinel_tools::buildin_tools::WebSearchTool::NAME.to_string(),
        true,
    );
    map.insert(SearchExploitTool::NAME.to_string(), true);
    map.insert(
        sentinel_tools::buildin_tools::MemoryManagerTool::NAME.to_string(),
        true,
    );
    map.insert(OcrTool::NAME.to_string(), true);
    map.insert(TerminalServer::NAME.to_string(), true);
    map.insert(TenthManTool::NAME.to_string(), true);
    map.insert(TasksTool::NAME.to_string(), true);
    map.insert(SpawnAgentTool::NAME.to_string(), true);
    map.insert(WaitAgentsTool::NAME.to_string(), true);
    map.insert(ListAgentsTool::NAME.to_string(), true);
    map.insert(CloseAgentTool::NAME.to_string(), true);
    map.insert(PluginAuthoringTool::NAME.to_string(), true);
    RwLock::new(map)
});

/// Get all builtin tools with their status
#[tauri::command]
pub async fn get_builtin_tools_with_status() -> Result<Vec<BuiltinToolInfo>, String> {
    let tool_server = get_tool_server();
    tool_server.init_builtin_tools().await;
    let states = TOOL_STATES.read().await;
    let mut tools: Vec<BuiltinToolInfo> = tool_server
        .list_tools()
        .await
        .into_iter()
        .filter(|tool| tool.source == "builtin")
        .filter(|tool| tool.name != "sops")
        .map(|tool| BuiltinToolInfo {
            id: tool.name.clone(),
            name: tool.name.clone(),
            description: tool.description,
            category: tool.category,
            version: "1.0.0".to_string(),
            enabled: states.get(&tool.name).copied().unwrap_or(tool.enabled),
            input_schema: Some(tool.input_schema),
        })
        .collect();

    tools.sort_by(|left, right| left.name.cmp(&right.name));

    Ok(tools)
}

/// Toggle a builtin tool on/off
#[tauri::command]
pub async fn toggle_builtin_tool(tool_name: String, enabled: bool) -> Result<(), String> {
    let mut states = TOOL_STATES.write().await;
    states.insert(tool_name.clone(), enabled);
    tracing::info!("Tool '{}' toggled to {}", tool_name, enabled);
    Ok(())
}

/// Unified tool execution for builtin tools and workflow tools
#[tauri::command]
pub async fn unified_execute_tool(
    tool_name: String,
    inputs: serde_json::Value,
    _context: Option<serde_json::Value>,
    _timeout: Option<u64>,
) -> Result<ToolExecutionResult, String> {
    let start = std::time::Instant::now();

    if let Err(message) =
        sentinel_license::ensure_feature_access(sentinel_license::LicensedFeature::ToolExecution)
    {
        return Ok(ToolExecutionResult {
            success: false,
            output: None,
            error: Some(message),
            execution_time_ms: start.elapsed().as_millis() as u64,
        });
    }

    // Check tool enabled status for builtin tools
    if !tool_name.contains("::") {
        let states = TOOL_STATES.read().await;
        if !states.get(&tool_name).unwrap_or(&true) {
            return Ok(ToolExecutionResult {
                success: false,
                output: None,
                error: Some(format!("Tool '{}' is disabled", tool_name)),
                execution_time_ms: start.elapsed().as_millis() as u64,
            });
        }
    }

    // Use ToolServer for execution
    let tool_server = get_tool_server();
    tool_server.init_builtin_tools().await;

    let result = tool_server.execute(&tool_name, inputs).await;

    Ok(ToolExecutionResult {
        success: result.success,
        output: result.output,
        error: result.error,
        execution_time_ms: result.execution_time_ms,
    })
}

#[allow(dead_code)]
async fn execute_http_request(inputs: serde_json::Value) -> Result<serde_json::Value, String> {
    use sentinel_tools::buildin_tools::http_request::HttpRequestArgs;

    let args: HttpRequestArgs = serde_json::from_value(inputs)
        .map_err(|e| format!("Invalid http_request arguments: {}", e))?;

    let tool = HttpRequestTool::default();
    let result = tool
        .call(args)
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    serde_json::to_value(result).map_err(|e| format!("Failed to serialize result: {}", e))
}

#[allow(dead_code)]
async fn execute_shell(inputs: serde_json::Value) -> Result<serde_json::Value, String> {
    use sentinel_tools::buildin_tools::shell::ShellArgs;

    let args: ShellArgs =
        serde_json::from_value(inputs).map_err(|e| format!("Invalid shell arguments: {}", e))?;

    let tool = ShellTool::new();
    let result = tool
        .call(args)
        .await
        .map_err(|e| format!("Shell execution failed: {}", e))?;

    serde_json::to_value(result).map_err(|e| format!("Failed to serialize result: {}", e))
}

#[allow(dead_code)]
async fn execute_workflow_tool(
    tool_name: &str,
    _inputs: serde_json::Value,
    _timeout: Option<u64>,
) -> Result<ToolExecutionResult, String> {
    let start = std::time::Instant::now();

    // Extract workflow ID from tool_name (format: "workflow::{id}")
    let workflow_id = tool_name
        .strip_prefix("workflow::")
        .ok_or_else(|| "Invalid workflow tool name".to_string())?;

    // TODO: Load workflow definition from database and execute
    // For now, return a placeholder result
    tracing::warn!(
        "Workflow tool execution not yet fully implemented: {}",
        workflow_id
    );

    Ok(ToolExecutionResult {
        success: false,
        output: None,
        error: Some(format!("Workflow tool '{}' execution not yet implemented. Please use WorkflowStudio to run workflows.", workflow_id)),
        execution_time_ms: start.elapsed().as_millis() as u64,
    })
}

/// List all unified tools (builtin + workflow + plugin)
#[tauri::command]
pub async fn list_unified_tools() -> Result<Vec<serde_json::Value>, String> {
    let builtin_tools = get_builtin_tools_with_status().await?;

    let tools: Vec<serde_json::Value> = builtin_tools
        .into_iter()
        .map(|t| {
            serde_json::json!({
                "name": t.name,
                "description": t.description,
                "category": t.category,
                "source": "builtin",
                "available": t.enabled,
            })
        })
        .collect();

    // TODO: Add workflow tools and plugin tools

    Ok(tools)
}

/// Node catalog item for workflow studio
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCatalogItem {
    pub node_type: String,
    pub label: String,
    pub category: String,
    pub params_schema: serde_json::Value,
    pub input_ports: Vec<PortDef>,
    pub output_ports: Vec<PortDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortDef {
    pub id: String,
    pub name: String,
    pub port_type: String,
    pub required: bool,
}

/// 从插件代码获取 input_schema（仅通过运行时调用 get_input_schema）
///
/// 调用插件导出的 get_input_schema() 函数获取 schema。
/// 如果插件未导出该函数，返回默认空 schema。
async fn get_plugin_input_schema_async(
    plugin_id: &str,
    plugin_name: &str,
    code: &str,
) -> serde_json::Value {
    // 构建临时元数据
    let metadata = sentinel_plugins::PluginMetadata {
        id: plugin_id.to_string(),
        name: plugin_name.to_string(),
        version: "1.0.0".to_string(),
        author: None,
        main_category: "agent".to_string(),
        category: "tool".to_string(),
        monitor_type: None,
        default_severity: sentinel_plugins::Severity::Medium,
        tags: vec![],
        description: Some(format!("Agent tool plugin: {}", plugin_name)),
        target_asset_types: Vec::new(),
    };

    // 运行时获取 schema
    match sentinel_plugins::get_input_schema_from_code(code, metadata).await {
        Ok(schema) => schema,
        Err(e) => {
            tracing::warn!(
                "Failed to get schema from runtime for {}: {}, plugin must export get_input_schema()",
                plugin_id,
                e
            );
            // 返回默认空 schema
            serde_json::json!({
                "type": "object",
                "properties": {}
            })
        }
    }
}

/// List all available node types for workflow studio
#[tauri::command]
pub async fn list_node_catalog(
    traffic_state: tauri::State<'_, crate::commands::traffic::TrafficAnalysisState>,
) -> Result<Vec<NodeCatalogItem>, String> {
    build_node_catalog(traffic_state.inner()).await
}

/// Build node catalog for use by other commands (includes MCP and enabled plugins).
pub async fn build_node_catalog(
    traffic_state: &crate::commands::traffic::TrafficAnalysisState,
) -> Result<Vec<NodeCatalogItem>, String> {
    let mut catalog = Vec::new();

    // Trigger nodes
    catalog.push(NodeCatalogItem {
        node_type: "start".to_string(),
        label: "开始".to_string(),
        category: "trigger".to_string(),
        params_schema: serde_json::json!({"type": "object", "properties": {}}),
        input_ports: vec![],
        output_ports: vec![PortDef {
            id: "out".to_string(),
            name: "输出".to_string(),
            port_type: "Json".to_string(),
            required: false,
        }],
    });

    catalog.push(NodeCatalogItem {
        node_type: "trigger_schedule".to_string(),
        label: "定时触发".to_string(),
        category: "trigger".to_string(),
        params_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "trigger_type": {"type": "string", "enum": ["interval", "daily", "weekly"], "default": "interval"},
                "interval_seconds": {"type": "integer", "default": 60, "description": "间隔秒数（interval模式）"},
                "hour": {"type": "integer", "default": 9, "minimum": 0, "maximum": 23},
                "minute": {"type": "integer", "default": 0, "minimum": 0, "maximum": 59},
                "second": {"type": "integer", "default": 0, "minimum": 0, "maximum": 59},
                "weekdays": {"type": "string", "default": "1,2,3,4,5", "description": "周几触发，逗号分隔"}
            }
        }),
        input_ports: vec![],
        output_ports: vec![PortDef { id: "out".to_string(), name: "输出".to_string(), port_type: "Json".to_string(), required: false }],
    });

    // Control flow nodes
    catalog.push(NodeCatalogItem {
        node_type: "branch".to_string(),
        label: "条件分支".to_string(),
        category: "control".to_string(),
        params_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "expr": {"type": "string", "default": "true", "description": "条件表达式"}
            }
        }),
        input_ports: vec![PortDef {
            id: "in".to_string(),
            name: "输入".to_string(),
            port_type: "Json".to_string(),
            required: false,
        }],
        output_ports: vec![
            PortDef {
                id: "true".to_string(),
                name: "真".to_string(),
                port_type: "Json".to_string(),
                required: false,
            },
            PortDef {
                id: "false".to_string(),
                name: "假".to_string(),
                port_type: "Json".to_string(),
                required: false,
            },
        ],
    });

    catalog.push(NodeCatalogItem {
        node_type: "merge".to_string(),
        label: "合并".to_string(),
        category: "control".to_string(),
        params_schema: serde_json::json!({"type": "object", "properties": {}}),
        input_ports: vec![
            PortDef {
                id: "in1".to_string(),
                name: "输入1".to_string(),
                port_type: "Json".to_string(),
                required: false,
            },
            PortDef {
                id: "in2".to_string(),
                name: "输入2".to_string(),
                port_type: "Json".to_string(),
                required: false,
            },
        ],
        output_ports: vec![PortDef {
            id: "out".to_string(),
            name: "输出".to_string(),
            port_type: "Json".to_string(),
            required: false,
        }],
    });

    catalog.push(NodeCatalogItem {
        node_type: "retry".to_string(),
        label: "重试".to_string(),
        category: "control".to_string(),
        params_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "times": {"type": "integer", "default": 3},
                "delay_ms": {"type": "integer", "default": 500},
                "tool_name": {"type": "string"},
                "tool_params": {"type": "object"}
            }
        }),
        input_ports: vec![PortDef {
            id: "in".to_string(),
            name: "输入".to_string(),
            port_type: "Json".to_string(),
            required: false,
        }],
        output_ports: vec![PortDef {
            id: "out".to_string(),
            name: "输出".to_string(),
            port_type: "Json".to_string(),
            required: false,
        }],
    });

    // AI nodes
    catalog.push(NodeCatalogItem {
        node_type: "ai_chat".to_string(),
        label: "AI对话".to_string(),
        category: "ai".to_string(),
        params_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "provider": {"type": "string", "x-ui-widget": "ai-provider-select"},
                "model": {"type": "string", "x-ui-widget": "ai-model-select"},
                "prompt": {"type": "string", "x-ui-widget": "textarea", "description": "用户消息，支持 {{input}} 变量"},
                "system_prompt": {"type": "string", "x-ui-widget": "textarea", "description": "系统提示词"}
            }
        }),
        input_ports: vec![PortDef { id: "in".to_string(), name: "输入".to_string(), port_type: "Json".to_string(), required: false }],
        output_ports: vec![PortDef { id: "out".to_string(), name: "输出".to_string(), port_type: "Json".to_string(), required: false }],
    });

    catalog.push(NodeCatalogItem {
        node_type: "ai_agent".to_string(),
        label: "AI Agent".to_string(),
        category: "ai".to_string(),
        params_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "provider": {"type": "string", "x-ui-widget": "ai-provider-select"},
                "model": {"type": "string", "x-ui-widget": "ai-model-select"},
                "prompt": {"type": "string", "x-ui-widget": "textarea"},
                "system_prompt": {"type": "string", "x-ui-widget": "textarea"},
                "tools": {"type": "array", "x-ui-widget": "tools-multiselect", "items": {"type": "string"}}
            }
        }),
        input_ports: vec![PortDef { id: "in".to_string(), name: "输入".to_string(), port_type: "Json".to_string(), required: false }],
        output_ports: vec![PortDef { id: "out".to_string(), name: "输出".to_string(), port_type: "Json".to_string(), required: false }],
    });

    // Builtin tools as nodes
    let builtin_tools = get_builtin_tools_with_status().await?;
    for tool in builtin_tools {
        catalog.push(NodeCatalogItem {
            node_type: format!("tool::{}", tool.name),
            label: tool.name.clone(),
            category: "tool".to_string(),
            params_schema: tool
                .input_schema
                .unwrap_or(serde_json::json!({"type": "object", "properties": {}})),
            input_ports: vec![PortDef {
                id: "in".to_string(),
                name: "输入".to_string(),
                port_type: "Json".to_string(),
                required: false,
            }],
            output_ports: vec![PortDef {
                id: "out".to_string(),
                name: "输出".to_string(),
                port_type: "Json".to_string(),
                required: false,
            }],
        });
    }

    // RAG nodes
    catalog.push(NodeCatalogItem {
        node_type: "rag::ingest".to_string(),
        label: "RAG导入".to_string(),
        category: "data".to_string(),
        params_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "file_path": {"type": "string", "description": "文件路径"},
                "collection_id": {"type": "string", "description": "集合ID"},
                "metadata": {"type": "object", "description": "元数据"}
            },
            "required": ["file_path"]
        }),
        input_ports: vec![PortDef {
            id: "in".to_string(),
            name: "输入".to_string(),
            port_type: "Json".to_string(),
            required: false,
        }],
        output_ports: vec![PortDef {
            id: "out".to_string(),
            name: "输出".to_string(),
            port_type: "Json".to_string(),
            required: false,
        }],
    });

    catalog.push(NodeCatalogItem {
        node_type: "rag::query".to_string(),
        label: "RAG查询".to_string(),
        category: "data".to_string(),
        params_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "查询内容"},
                "collection_id": {"type": "string"},
                "top_k": {"type": "integer", "default": 5},
                "use_mmr": {"type": "boolean", "default": false},
                "mmr_lambda": {"type": "number", "default": 0.5}
            },
            "required": ["query"]
        }),
        input_ports: vec![PortDef {
            id: "in".to_string(),
            name: "输入".to_string(),
            port_type: "Json".to_string(),
            required: false,
        }],
        output_ports: vec![PortDef {
            id: "out".to_string(),
            name: "输出".to_string(),
            port_type: "Json".to_string(),
            required: false,
        }],
    });

    // Notification node
    catalog.push(NodeCatalogItem {
        node_type: "notify".to_string(),
        label: "通知".to_string(),
        category: "output".to_string(),
        params_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "notification_rule_id": {"type": "string", "description": "通知规则ID"},
                "title": {"type": "string", "default": "Workflow Notification"},
                "content": {"type": "string", "x-ui-widget": "textarea"},
                "use_input_as_content": {"type": "boolean", "default": false}
            }
        }),
        input_ports: vec![PortDef {
            id: "in".to_string(),
            name: "输入".to_string(),
            port_type: "Json".to_string(),
            required: false,
        }],
        output_ports: vec![PortDef {
            id: "out".to_string(),
            name: "输出".to_string(),
            port_type: "Json".to_string(),
            required: false,
        }],
    });

    // Raw data node
    catalog.push(NodeCatalogItem {
        node_type: "raw".to_string(),
        label: "Raw数据".to_string(),
        category: "data".to_string(),
        params_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "raw_type": {
                    "type": "string",
                    "enum": ["json", "text"],
                    "default": "json",
                    "description": "json 模式输出 JSON 值，text 模式原样输出文本"
                },
                "value": {
                    "type": "string",
                    "x-ui-widget": "textarea",
                    "description": "json 模式下填写合法 JSON；text 模式下原样输出"
                }
            },
            "required": ["value"]
        }),
        input_ports: vec![],
        output_ports: vec![PortDef {
            id: "out".to_string(),
            name: "输出".to_string(),
            port_type: "Json".to_string(),
            required: false,
        }],
    });

    // MCP 工具节点 - 从已连接的 MCP 服务器获取
    let mcp_tools = crate::commands::mcp_commands::mcp_get_all_tools()
        .await
        .unwrap_or_default();
    for tool in mcp_tools {
        let server_name = tool
            .get("server_name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let tool_name = tool
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let description = tool.get("description").and_then(|v| v.as_str());
        let input_schema = tool
            .get("input_schema")
            .cloned()
            .unwrap_or(serde_json::json!({"type": "object", "properties": {}}));

        catalog.push(NodeCatalogItem {
            node_type: format!("mcp::{}::{}", server_name, tool_name),
            label: format!("[{}] {}", server_name, tool_name),
            category: "mcp".to_string(),
            params_schema: input_schema,
            input_ports: vec![PortDef {
                id: "in".to_string(),
                name: "输入".to_string(),
                port_type: "Json".to_string(),
                required: false,
            }],
            output_ports: vec![PortDef {
                id: "out".to_string(),
                name: "输出".to_string(),
                port_type: "Json".to_string(),
                required: false,
            }],
        });

        tracing::debug!(
            "Added MCP tool node: mcp::{}::{} - {:?}",
            server_name,
            tool_name,
            description
        );
    }

    // Agent 插件工具节点 - 从数据库获取已启用的 Agent 工具插件
    if let Ok(plugins) = traffic_state.list_plugins_internal().await {
        // 获取数据库服务用于查询插件代码
        let db_service = Some(traffic_state.get_db_service());

        for plugin in plugins {
            // 只添加已启用的 Agent 类型插件
            if plugin.status == sentinel_traffic::PluginStatus::Enabled
                && plugin.metadata.main_category == "agent"
            {
                // 获取插件代码并通过运行时获取 schema（优先），静态解析作为 fallback
                let params_schema = if let Some(ref db) = db_service {
                    if let Ok(Some(code)) = db.get_traffic_plugin_code(&plugin.metadata.id).await {
                        // 使用运行时方法获取 schema
                        get_plugin_input_schema_async(
                            &plugin.metadata.id,
                            &plugin.metadata.name,
                            &code,
                        )
                        .await
                    } else {
                        serde_json::json!({
                            "type": "object",
                            "properties": {
                                "input": {"type": "string", "x-ui-widget": "textarea", "description": "工具输入参数"}
                            }
                        })
                    }
                } else {
                    serde_json::json!({
                        "type": "object",
                        "properties": {
                            "input": {"type": "string", "x-ui-widget": "textarea", "description": "工具输入参数"}
                        }
                    })
                };

                catalog.push(NodeCatalogItem {
                    node_type: format!("plugin::{}", plugin.metadata.id),
                    label: plugin.metadata.name.clone(),
                    category: "plugin".to_string(),
                    params_schema,
                    input_ports: vec![PortDef {
                        id: "in".to_string(),
                        name: "输入".to_string(),
                        port_type: "Json".to_string(),
                        required: false,
                    }],
                    output_ports: vec![PortDef {
                        id: "out".to_string(),
                        name: "输出".to_string(),
                        port_type: "Json".to_string(),
                        required: false,
                    }],
                });

                tracing::debug!(
                    "Added Agent plugin node: plugin::{} - {}",
                    plugin.metadata.id,
                    plugin.metadata.name
                );
            }
        }
    }

    Ok(catalog)
}

// ============================================================================
// Tool Metadata Management Commands
// ============================================================================

/// Get all tool metadata
#[tauri::command]
pub async fn get_all_tool_metadata(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Vec<ToolMetadata>, String> {
    let router = ToolRouter::new_with_all_tools(Some(db_service.inner())).await;
    Ok(router
        .list_all_tools()
        .into_iter()
        .filter(|tool| tool.id != sentinel_tools::buildin_tools::SopsTool::NAME)
        .collect())
}

/// Get tool metadata by category
#[tauri::command]
pub async fn get_tools_by_category(
    category: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Vec<ToolMetadata>, String> {
    let router = ToolRouter::new_with_all_tools(Some(db_service.inner())).await;

    let category_enum = match category.to_lowercase().as_str() {
        "network" => ToolCategory::Network,
        "security" => ToolCategory::Security,
        "data" => ToolCategory::Data,
        "ai" => ToolCategory::AI,
        "system" => ToolCategory::System,
        "mcp" => ToolCategory::MCP,
        "plugin" => ToolCategory::Plugin,
        "workflow" => ToolCategory::Workflow,
        "browser" => ToolCategory::Browser,
        "utility" => ToolCategory::Utility,
        "recon" => ToolCategory::Recon,
        "scanning" => ToolCategory::Scanning,
        "exploitation" => ToolCategory::Exploitation,
        "monitoring" => ToolCategory::Monitoring,
        "other" => ToolCategory::Other,
        _ => return Err(format!("Unknown category: {}", category)),
    };

    Ok(router.list_tools_by_category(category_enum))
}

/// Search tools by query
#[tauri::command]
pub async fn search_tools(
    query: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Vec<ToolMetadata>, String> {
    let router = ToolRouter::new_with_all_tools(Some(db_service.inner())).await;
    Ok(router.search_tools(&query))
}

/// Get tool statistics
#[tauri::command]
pub async fn get_tool_statistics(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<ToolStatistics, String> {
    let router = ToolRouter::new_with_all_tools(Some(db_service.inner())).await;
    Ok(router.get_statistics())
}

/// Get tool metadata by ID
#[tauri::command]
pub async fn get_tool_metadata(
    tool_id: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Option<ToolMetadata>, String> {
    if tool_id == SkillsTool::NAME {
        return Ok(None);
    }
    let router = ToolRouter::new_with_all_tools(Some(db_service.inner())).await;
    Ok(router.get_tool_metadata(&tool_id))
}

/// Get tool usage statistics
#[tauri::command]
pub async fn get_tool_usage_stats() -> Result<ToolUsageStatistics, String> {
    Ok(get_tool_usage_statistics().await)
}

/// Clear tool usage records
#[tauri::command]
pub async fn clear_tool_usage_stats() -> Result<(), String> {
    clear_tool_usage_records().await;
    Ok(())
}

pub mod tool_server;
pub use tool_server::{
    execute_tool_server_tool, get_tool_input_schema, get_tool_output_schema, get_tool_server_stats,
    get_tool_server_tool, init_tool_server, list_tool_server_tools, list_tools_by_source,
    refresh_all_dynamic_tools, register_mcp_tools_from_server, register_workflow_tools,
};

// ============================================================================
// Vision Explorer Credential Commands (V2 Compatible)
// ============================================================================

// Note: vision_bridge is disabled after ReAct refactoring
// The new ReAct architecture doesn't require these bridge commands
// mod vision_bridge;

mod ask_user_question;
mod shell_permission_history;
mod shell_permissions;
pub use ask_user_question::PendingAskUserQuestionRequest;
pub use shell_permission_history::{ShellPermissionHistoryEntry, ShellPermissionHistoryQuery};
pub use shell_permissions::PendingPermissionRequest;
pub use shell_permissions::PersistedShellAllowRules;

pub mod agent_config;
pub use agent_config::{
    init_agent_config, load_subagent_config_from_db, AgentConfig, SubagentConfig,
};

mod exploitdb;
mod skill_candidates;
mod skills;

pub async fn init_exploitdb_runtime_config(
    db_service: &Arc<sentinel_db::DatabaseService>,
) -> Result<(), String> {
    exploitdb::initialize_exploitdb_runtime_config(db_service).await
}

#[tauri::command]
pub async fn init_shell_permission_handler(app: tauri::AppHandle) -> Result<(), String> {
    shell_permissions::init_shell_permission_handler(app).await
}

#[tauri::command]
pub async fn init_ask_user_question_handler(app: tauri::AppHandle) -> Result<(), String> {
    ask_user_question::init_ask_user_question_handler(app).await
}

#[tauri::command]
pub async fn init_shell_background_runtime(app: tauri::AppHandle) -> Result<(), String> {
    sentinel_tools::buildin_tools::shell_background::set_shell_background_app_handle(app).await;
    Ok(())
}

#[tauri::command]
pub async fn get_shell_tool_config() -> Result<ShellConfig, String> {
    shell_permissions::get_shell_tool_config().await
}

#[tauri::command]
pub async fn set_shell_tool_config(config: ShellConfig) -> Result<(), String> {
    shell_permissions::set_shell_tool_config(config).await
}

#[tauri::command]
pub async fn respond_shell_permission(id: String, allowed: bool) -> Result<(), String> {
    shell_permissions::respond_shell_permission(id, allowed).await
}

#[tauri::command]
pub async fn allow_shell_permission_forever(
    id: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<PersistedShellAllowRules, String> {
    shell_permissions::allow_shell_permission_forever(id, db_service).await
}

pub(crate) async fn allow_shell_permission_forever_with_db(
    id: String,
    db: &sentinel_db::DatabaseService,
) -> Result<PersistedShellAllowRules, String> {
    shell_permissions::allow_shell_permission_forever_with_db(id, db).await
}

#[tauri::command]
pub async fn get_pending_shell_permissions() -> Result<Vec<PendingPermissionRequest>, String> {
    shell_permissions::get_pending_shell_permissions().await
}

#[tauri::command]
pub async fn get_shell_permission_history(
    request: ShellPermissionHistoryQuery,
) -> Result<Vec<ShellPermissionHistoryEntry>, String> {
    shell_permission_history::get_shell_permission_history(request).await
}

#[tauri::command]
pub async fn get_pending_ask_user_questions() -> Result<Vec<PendingAskUserQuestionRequest>, String>
{
    ask_user_question::get_pending_ask_user_questions().await
}

#[tauri::command]
pub async fn respond_ask_user_question(
    id: String,
    answers: HashMap<String, String>,
) -> Result<(), String> {
    ask_user_question::respond_ask_user_question(id, answers).await
}

#[tauri::command]
pub async fn reject_ask_user_question(id: String) -> Result<(), String> {
    ask_user_question::reject_ask_user_question(id).await
}

#[tauri::command]
pub async fn get_background_shell_tasks(
    execution_id: Option<String>,
) -> Result<Vec<sentinel_tools::buildin_tools::shell_background::BackgroundShellTaskRecord>, String>
{
    Ok(
        sentinel_tools::buildin_tools::shell_background::list_background_shell_tasks(
            execution_id.as_deref(),
        )
        .await,
    )
}

#[tauri::command]
pub async fn stop_background_shell_task(task_id: String) -> Result<(), String> {
    sentinel_tools::buildin_tools::shell_background::stop_background_shell_task(&task_id).await
}

#[tauri::command]
pub async fn get_agent_config(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<AgentConfig, String> {
    agent_config::get_agent_config(db_service).await
}

#[tauri::command]
pub async fn save_agent_config(
    config: AgentConfig,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<(), String> {
    agent_config::save_agent_config(config, db_service).await
}

#[tauri::command]
pub async fn list_skills(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Vec<sentinel_db::SkillSummary>, String> {
    skills::list_skills(db_service).await
}

#[tauri::command]
pub async fn list_skills_full(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Vec<sentinel_db::Skill>, String> {
    skills::list_skills_full(db_service).await
}

#[tauri::command]
pub async fn get_skill_detail(
    id: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Option<sentinel_db::SkillDetail>, String> {
    skills::get_skill_detail(id, db_service).await
}

#[tauri::command]
pub async fn get_skill(
    id: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Option<sentinel_db::Skill>, String> {
    skills::get_skill(id, db_service).await
}

#[tauri::command]
pub async fn get_skill_markdown(
    id: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<String, String> {
    skills::get_skill_markdown(id, db_service).await
}

#[tauri::command]
pub async fn create_skill(
    payload: skills::CreateSkillRequest,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<sentinel_db::Skill, String> {
    skills::create_skill(payload, db_service).await
}

#[tauri::command]
pub async fn update_skill(
    id: String,
    payload: skills::UpdateSkillRequest,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<bool, String> {
    skills::update_skill(id, payload, db_service).await
}

#[tauri::command]
pub async fn delete_skill(
    id: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<bool, String> {
    skills::delete_skill(id, db_service).await
}

#[tauri::command]
pub async fn refresh_skills_index(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<usize, String> {
    skills::refresh_skills_index(db_service).await
}

#[tauri::command]
pub async fn list_skill_candidates(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Vec<crate::skills::candidates::SkillCandidate>, String> {
    skill_candidates::list_skill_candidates(db_service).await
}

#[tauri::command]
pub async fn list_skill_candidate_suppression_rules(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Vec<crate::skills::candidates::SkillCandidateSuppressionRule>, String> {
    skill_candidates::list_skill_candidate_suppression_rules(db_service).await
}

#[tauri::command]
pub async fn delete_skill_candidate_suppression_rule(
    rule_id: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<bool, String> {
    skill_candidates::delete_skill_candidate_suppression_rule_by_id(rule_id, db_service).await
}

#[tauri::command]
pub async fn extend_skill_candidate_suppression_rule(
    rule_id: String,
    days: i64,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<crate::skills::candidates::SkillCandidateSuppressionRule, String> {
    skill_candidates::extend_skill_candidate_suppression_rule(rule_id, days, db_service).await
}

#[tauri::command]
pub async fn expire_skill_candidate_suppression_rule(
    rule_id: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<crate::skills::candidates::SkillCandidateSuppressionRule, String> {
    skill_candidates::expire_skill_candidate_suppression_rule(rule_id, db_service).await
}

#[tauri::command]
pub async fn promote_skill_candidate(
    payload: skill_candidates::PromoteSkillCandidateRequest,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<sentinel_db::Skill, String> {
    skill_candidates::promote_skill_candidate(payload, db_service).await
}

#[tauri::command]
pub async fn review_skill_candidate(
    payload: skill_candidates::ReviewSkillCandidateRequest,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<crate::skills::candidates::SkillCandidate, String> {
    skill_candidates::review_skill_candidate(payload, db_service).await
}

#[tauri::command]
pub async fn list_skill_files(
    id: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Vec<skills::SkillFileEntry>, String> {
    skills::list_skill_files(id, db_service).await
}

#[tauri::command]
pub async fn read_skill_file(
    id: String,
    path: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<String, String> {
    skills::read_skill_file(id, path, db_service).await
}

#[tauri::command]
pub async fn save_skill_file(
    id: String,
    path: String,
    content: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<bool, String> {
    skills::save_skill_file(id, path, content, db_service).await
}

#[tauri::command]
pub async fn delete_skill_file(
    id: String,
    path: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<bool, String> {
    skills::delete_skill_file(id, path, db_service).await
}

#[tauri::command]
pub async fn import_skill_file(
    id: String,
    source_path: String,
    target_path: Option<String>,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<String, String> {
    skills::import_skill_file(id, source_path, target_path, db_service).await
}

#[tauri::command]
pub async fn discover_skills_from_path(
    source_path: String,
) -> Result<Vec<skills::SkillCandidate>, String> {
    skills::discover_skills_from_path(source_path).await
}

#[tauri::command]
pub async fn discover_skills_from_git(
    url: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<(String, Vec<skills::SkillCandidate>), String> {
    skills::discover_skills_from_git(url, db_service).await
}

#[tauri::command]
pub async fn install_skills_from_path(
    source_path: String,
    skill_ids: Vec<String>,
    source_type: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Vec<String>, String> {
    skills::install_skills_from_path(source_path, skill_ids, source_type, db_service).await
}

#[tauri::command]
pub async fn list_skill_install_history(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Vec<skills::SkillInstallHistory>, String> {
    skills::list_skill_install_history(db_service).await
}

#[tauri::command]
pub async fn delete_skill_install_history(
    id: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<bool, String> {
    skills::delete_skill_install_history(id, db_service).await
}

#[tauri::command]
pub async fn get_exploitdb_settings(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<exploitdb::ExploitDbSettings, String> {
    exploitdb::get_exploitdb_settings(db_service).await
}

#[tauri::command]
pub async fn save_exploitdb_settings(
    repo_url: Option<String>,
    repo_path: Option<String>,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<exploitdb::ExploitDbSettings, String> {
    exploitdb::save_exploitdb_settings(repo_url, repo_path, db_service).await
}

#[tauri::command]
pub async fn get_exploitdb_sync_status(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<exploitdb::ExploitDbSyncStatus, String> {
    exploitdb::get_exploitdb_sync_status(db_service).await
}

#[tauri::command]
pub async fn sync_exploitdb(
    force_reindex: Option<bool>,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<exploitdb::ExploitDbSyncResponse, String> {
    exploitdb::sync_exploitdb(force_reindex, db_service).await
}

#[tauri::command]
pub async fn browse_exploitdb_entries(
    query: Option<String>,
    cve: Option<String>,
    platform: Option<String>,
    exploit_type: Option<String>,
    page: Option<u32>,
    page_size: Option<u32>,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<exploitdb::ExploitDbBrowseResponse, String> {
    exploitdb::browse_exploitdb_entries(
        query,
        cve,
        platform,
        exploit_type,
        page,
        page_size,
        db_service,
    )
    .await
}

#[tauri::command]
pub async fn get_exploitdb_entry_detail(
    edb_id: u32,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<exploitdb::ExploitDbDetailResponse, String> {
    exploitdb::get_exploitdb_entry_detail(edb_id, db_service).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn builtin_tool_menu_includes_deferred_and_browser_tools() {
        let tools = get_builtin_tools_with_status()
            .await
            .expect("builtin tools should load");

        let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_str()).collect();
        assert!(names.contains(&"tool_search"));
        assert!(names.contains(&"browser"));
        assert!(names.contains(&"route_discovery"));
        assert!(names.contains(&"file_read"));
        assert!(names.contains(&"grep"));
    }

    #[tokio::test]
    async fn toggle_builtin_tool_changes_menu_enabled_state() {
        toggle_builtin_tool("tool_search".to_string(), false)
            .await
            .expect("toggle should succeed");

        let tools = get_builtin_tools_with_status()
            .await
            .expect("builtin tools should load");
        let tool_search = tools
            .iter()
            .find(|tool| tool.name == "tool_search")
            .expect("tool_search should exist");
        assert!(!tool_search.enabled);

        toggle_builtin_tool("tool_search".to_string(), true)
            .await
            .expect("restore toggle should succeed");
    }
}
