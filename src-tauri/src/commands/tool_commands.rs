//! Tool commands - Tauri commands for builtin tools and workflow tools

use std::collections::HashMap;
use std::sync::Arc;

use once_cell::sync::Lazy;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use sentinel_tools::buildin_tools::{HttpRequestTool, LocalTimeTool, PortScanTool, ShellTool};
use sentinel_tools::{get_tool_server, ToolServer};

/// Builtin tool info for frontend
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
    map.insert("port_scan".to_string(), true);
    map.insert("http_request".to_string(), true);
    map.insert("local_time".to_string(), true);
    map.insert("shell".to_string(), true);
    RwLock::new(map)
});

/// Get all builtin tools with their status
#[tauri::command]
pub async fn get_builtin_tools_with_status() -> Result<Vec<BuiltinToolInfo>, String> {
    let states = TOOL_STATES.read().await;
    
    let mut tools = vec![
        BuiltinToolInfo {
            id: "port_scan".to_string(),
            name: "port_scan".to_string(),
            description: "High-performance TCP port scanner with service identification. Scans target IP for open ports.".to_string(),
            category: "network".to_string(),
            version: "1.0.0".to_string(),
            enabled: *states.get("port_scan").unwrap_or(&true),
            input_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "target": {
                        "type": "string",
                        "description": "Target IP address to scan"
                    },
                    "ports": {
                        "type": "string",
                        "description": "Port range or list (e.g., '1-1000', '80,443,8080', or 'common')",
                        "default": "common"
                    },
                    "threads": {
                        "type": "integer",
                        "description": "Number of concurrent threads (1-1000)",
                        "default": 100,
                        "minimum": 1,
                        "maximum": 1000
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Connection timeout in seconds",
                        "default": 3
                    }
                },
                "required": ["target"]
            })),
        },
        BuiltinToolInfo {
            id: "http_request".to_string(),
            name: "http_request".to_string(),
            description: "Make HTTP requests to any URL. Supports GET, POST, PUT, DELETE methods with custom headers and body.".to_string(),
            category: "network".to_string(),
            version: "1.0.0".to_string(),
            enabled: *states.get("http_request").unwrap_or(&true),
            input_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "Target URL"
                    },
                    "method": {
                        "type": "string",
                        "description": "HTTP method",
                        "default": "GET",
                        "enum": ["GET", "POST", "PUT", "DELETE", "HEAD", "PATCH"]
                    },
                    "headers": {
                        "type": "object",
                        "description": "Request headers as key-value pairs"
                    },
                    "body": {
                        "type": "string",
                        "description": "Request body (for POST, PUT, etc.)"
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Request timeout in seconds",
                        "default": 30
                    },
                    "follow_redirects": {
                        "type": "boolean",
                        "description": "Follow redirects",
                        "default": true
                    }
                },
                "required": ["url"]
            })),
        },
        BuiltinToolInfo {
            id: "local_time".to_string(),
            name: "local_time".to_string(),
            description: "Get current local or UTC time in various formats.".to_string(),
            category: "utility".to_string(),
            version: "1.0.0".to_string(),
            enabled: *states.get("local_time").unwrap_or(&true),
            input_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "timezone": {
                        "type": "string",
                        "description": "Timezone: 'local' or 'utc'",
                        "default": "local"
                    },
                    "format": {
                        "type": "string",
                        "description": "Date format string (e.g., '%Y-%m-%d %H:%M:%S')",
                        "default": "%Y-%m-%d %H:%M:%S"
                    }
                }
            })),
        },
        BuiltinToolInfo {
            id: "shell".to_string(),
            name: "shell".to_string(),
            description: "Execute shell commands. Use with caution - some commands are restricted for security.".to_string(),
            category: "system".to_string(),
            version: "1.0.0".to_string(),
            enabled: *states.get("shell").unwrap_or(&true),
            input_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Shell command to execute"
                    },
                    "cwd": {
                        "type": "string",
                        "description": "Working directory (optional)"
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Command timeout in seconds",
                        "default": 60
                    }
                },
                "required": ["command"]
            })),
        },
    ];
    
    // Add vision_explorer
    tools.push(BuiltinToolInfo {
        id: "vision_explorer".to_string(),
        name: "vision_explorer".to_string(),
        description: "Explore a website using vision capabilities to discover APIs, pages, and interactive elements.".to_string(),
        category: "ai".to_string(),
        version: "1.0.0".to_string(),
        enabled: *states.get("vision_explorer").unwrap_or(&true),
        input_schema: Some(serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to explore"
                },
                "max_iterations": {
                    "type": "integer",
                    "description": "Maximum number of exploration steps (default: 20)",
                    "default": 20
                }
            },
            "required": ["url"]
        })),
    });
    
    // Ensure it's in states
    if !states.contains_key("vision_explorer") {
        drop(states); // Release read lock
        let mut states_write = TOOL_STATES.write().await;
        states_write.entry("vision_explorer".to_string()).or_insert(true);
    }

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

    // License check for tool execution
    #[cfg(not(debug_assertions))]
    if !sentinel_license::is_licensed() {
        return Ok(ToolExecutionResult {
            success: false,
            output: None,
            error: Some("License required for tool execution".to_string()),
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

async fn execute_port_scan(inputs: serde_json::Value) -> Result<serde_json::Value, String> {
    use sentinel_tools::buildin_tools::port_scan::PortScanArgs;
    
    let args: PortScanArgs = serde_json::from_value(inputs)
        .map_err(|e| format!("Invalid port_scan arguments: {}", e))?;
    
    let tool = PortScanTool;
    let result = tool.call(args).await
        .map_err(|e| format!("Port scan failed: {}", e))?;
    
    serde_json::to_value(result)
        .map_err(|e| format!("Failed to serialize result: {}", e))
}

async fn execute_http_request(inputs: serde_json::Value) -> Result<serde_json::Value, String> {
    use sentinel_tools::buildin_tools::http_request::HttpRequestArgs;
    
    let args: HttpRequestArgs = serde_json::from_value(inputs)
        .map_err(|e| format!("Invalid http_request arguments: {}", e))?;
    
    let tool = HttpRequestTool::default();
    let result = tool.call(args).await
        .map_err(|e| format!("HTTP request failed: {}", e))?;
    
    serde_json::to_value(result)
        .map_err(|e| format!("Failed to serialize result: {}", e))
}

async fn execute_local_time(inputs: serde_json::Value) -> Result<serde_json::Value, String> {
    use sentinel_tools::buildin_tools::local_time::LocalTimeArgs;
    
    let args: LocalTimeArgs = serde_json::from_value(inputs)
        .map_err(|e| format!("Invalid local_time arguments: {}", e))?;
    
    let tool = LocalTimeTool;
    let result = tool.call(args).await
        .map_err(|e| format!("Local time failed: {}", e))?;
    
    serde_json::to_value(result)
        .map_err(|e| format!("Failed to serialize result: {}", e))
}

async fn execute_shell(inputs: serde_json::Value) -> Result<serde_json::Value, String> {
    use sentinel_tools::buildin_tools::shell::ShellArgs;
    
    let args: ShellArgs = serde_json::from_value(inputs)
        .map_err(|e| format!("Invalid shell arguments: {}", e))?;
    
    let tool = ShellTool::new();
    let result = tool.call(args).await
        .map_err(|e| format!("Shell execution failed: {}", e))?;
    
    serde_json::to_value(result)
        .map_err(|e| format!("Failed to serialize result: {}", e))
}

async fn execute_workflow_tool(
    tool_name: &str,
    inputs: serde_json::Value,
    _timeout: Option<u64>,
) -> Result<ToolExecutionResult, String> {
    let start = std::time::Instant::now();
    
    // Extract workflow ID from tool_name (format: "workflow::{id}")
    let workflow_id = tool_name.strip_prefix("workflow::")
        .ok_or_else(|| "Invalid workflow tool name".to_string())?;
    
    // TODO: Load workflow definition from database and execute
    // For now, return a placeholder result
    tracing::warn!("Workflow tool execution not yet fully implemented: {}", workflow_id);
    
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
    
    let mut tools: Vec<serde_json::Value> = builtin_tools.into_iter()
        .map(|t| serde_json::json!({
            "name": t.name,
            "description": t.description,
            "category": t.category,
            "source": "builtin",
            "available": t.enabled,
        }))
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

/// Parse plugin code to extract input_schema (delegates to unified parser)
fn parse_tool_input_schema(code: &str) -> serde_json::Value {
    sentinel_tools::plugin_adapter::PluginToolAdapter::parse_tool_input_schema(code)
}

/// List all available node types for workflow studio
#[tauri::command]
pub async fn list_node_catalog(
    passive_state: tauri::State<'_, crate::commands::passive_scan_commands::PassiveScanState>,
) -> Result<Vec<NodeCatalogItem>, String> {
    build_node_catalog(passive_state.inner()).await
}

/// Build node catalog for use by other commands (includes MCP and enabled plugins).
pub async fn build_node_catalog(
    passive_state: &crate::commands::passive_scan_commands::PassiveScanState,
) -> Result<Vec<NodeCatalogItem>, String> {
    let mut catalog = Vec::new();
    
    // Trigger nodes
    catalog.push(NodeCatalogItem {
        node_type: "start".to_string(),
        label: "开始".to_string(),
        category: "trigger".to_string(),
        params_schema: serde_json::json!({"type": "object", "properties": {}}),
        input_ports: vec![],
        output_ports: vec![PortDef { id: "out".to_string(), name: "输出".to_string(), port_type: "Json".to_string(), required: false }],
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
        input_ports: vec![PortDef { id: "in".to_string(), name: "输入".to_string(), port_type: "Json".to_string(), required: false }],
        output_ports: vec![
            PortDef { id: "true".to_string(), name: "真".to_string(), port_type: "Json".to_string(), required: false },
            PortDef { id: "false".to_string(), name: "假".to_string(), port_type: "Json".to_string(), required: false }
        ],
    });
    
    catalog.push(NodeCatalogItem {
        node_type: "merge".to_string(),
        label: "合并".to_string(),
        category: "control".to_string(),
        params_schema: serde_json::json!({"type": "object", "properties": {}}),
        input_ports: vec![
            PortDef { id: "in1".to_string(), name: "输入1".to_string(), port_type: "Json".to_string(), required: false },
            PortDef { id: "in2".to_string(), name: "输入2".to_string(), port_type: "Json".to_string(), required: false }
        ],
        output_ports: vec![PortDef { id: "out".to_string(), name: "输出".to_string(), port_type: "Json".to_string(), required: false }],
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
        input_ports: vec![PortDef { id: "in".to_string(), name: "输入".to_string(), port_type: "Json".to_string(), required: false }],
        output_ports: vec![PortDef { id: "out".to_string(), name: "输出".to_string(), port_type: "Json".to_string(), required: false }],
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
            params_schema: tool.input_schema.unwrap_or(serde_json::json!({"type": "object", "properties": {}})),
            input_ports: vec![PortDef { id: "in".to_string(), name: "输入".to_string(), port_type: "Json".to_string(), required: false }],
            output_ports: vec![PortDef { id: "out".to_string(), name: "输出".to_string(), port_type: "Json".to_string(), required: false }],
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
        input_ports: vec![PortDef { id: "in".to_string(), name: "输入".to_string(), port_type: "Json".to_string(), required: false }],
        output_ports: vec![PortDef { id: "out".to_string(), name: "输出".to_string(), port_type: "Json".to_string(), required: false }],
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
        input_ports: vec![PortDef { id: "in".to_string(), name: "输入".to_string(), port_type: "Json".to_string(), required: false }],
        output_ports: vec![PortDef { id: "out".to_string(), name: "输出".to_string(), port_type: "Json".to_string(), required: false }],
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
        input_ports: vec![PortDef { id: "in".to_string(), name: "输入".to_string(), port_type: "Json".to_string(), required: false }],
        output_ports: vec![PortDef { id: "out".to_string(), name: "输出".to_string(), port_type: "Json".to_string(), required: false }],
    });
    
    // Prompt build node
    catalog.push(NodeCatalogItem {
        node_type: "prompt::build".to_string(),
        label: "构建Prompt".to_string(),
        category: "data".to_string(),
        params_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "build_type": {"type": "string", "enum": ["Planner", "Executor", "Replanner", "ReportGenerator"], "default": "Planner"},
                "user_query": {"type": "string", "x-ui-widget": "textarea"}
            }
        }),
        input_ports: vec![PortDef { id: "in".to_string(), name: "输入".to_string(), port_type: "Json".to_string(), required: false }],
        output_ports: vec![PortDef { id: "out".to_string(), name: "输出".to_string(), port_type: "Json".to_string(), required: false }],
    });
    
    // MCP 工具节点 - 从已连接的 MCP 服务器获取
    let mcp_tools = crate::commands::mcp_commands::mcp_get_all_tools().await.unwrap_or_default();
    for tool in mcp_tools {
        let server_name = tool.get("server_name").and_then(|v| v.as_str()).unwrap_or("unknown");
        let tool_name = tool.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
        let description = tool.get("description").and_then(|v| v.as_str());
        let input_schema = tool.get("input_schema").cloned().unwrap_or(serde_json::json!({"type": "object", "properties": {}}));
        
        catalog.push(NodeCatalogItem {
            node_type: format!("mcp::{}::{}", server_name, tool_name),
            label: format!("[{}] {}", server_name, tool_name),
            category: "mcp".to_string(),
            params_schema: input_schema,
            input_ports: vec![PortDef { id: "in".to_string(), name: "输入".to_string(), port_type: "Json".to_string(), required: false }],
            output_ports: vec![PortDef { id: "out".to_string(), name: "输出".to_string(), port_type: "Json".to_string(), required: false }],
        });
        
        tracing::debug!("Added MCP tool node: mcp::{}::{} - {:?}", server_name, tool_name, description);
    }
    
    // Agent 插件工具节点 - 从数据库获取已启用的 Agent 工具插件
    if let Ok(plugins) = passive_state.list_plugins_internal().await {
        // 获取数据库服务用于查询插件代码
        let db_service = passive_state.get_db_service().await.ok();
        
        for plugin in plugins {
            // 只添加已启用的 Agent 类型插件
            if plugin.status == sentinel_passive::PluginStatus::Enabled 
                && plugin.metadata.main_category == "agent" 
            {
                // 尝试获取插件代码并解析 ToolInput 接口
                let params_schema = if let Some(ref db) = db_service {
                    if let Ok(Some(code)) = db.get_plugin_code(&plugin.metadata.id).await {
                        parse_tool_input_schema(&code)
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
                    input_ports: vec![PortDef { id: "in".to_string(), name: "输入".to_string(), port_type: "Json".to_string(), required: false }],
                    output_ports: vec![PortDef { id: "out".to_string(), name: "输出".to_string(), port_type: "Json".to_string(), required: false }],
                });
                
                tracing::debug!("Added Agent plugin node: plugin::{} - {}", plugin.metadata.id, plugin.metadata.name);
            }
        }
    }
    
    Ok(catalog)
}

// ============================================================================
// Tool Metadata Management Commands
// ============================================================================

use crate::agents::tool_router::{
    ToolRouter, ToolMetadata, ToolCategory, ToolStatistics,
    ToolUsageStatistics, get_tool_usage_statistics, clear_tool_usage_records,
};

/// Get all tool metadata
#[tauri::command]
pub async fn get_all_tool_metadata(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Vec<ToolMetadata>, String> {
    let router = ToolRouter::new_with_all_tools(Some(db_service.inner())).await;
    Ok(router.list_all_tools())
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

// ============================================================================
// ToolServer Management Commands
// ============================================================================

/// Initialize the global tool server with builtin tools
#[tauri::command]
pub async fn init_tool_server() -> Result<(), String> {
    let server = get_tool_server();
    server.init_builtin_tools().await;
    tracing::info!("Tool server initialized with {} tools", server.tool_count().await);
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
pub async fn list_tools_by_source(source_type: String) -> Result<Vec<sentinel_tools::ToolInfo>, String> {
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
    let mcp_count = tools.iter().filter(|t| t.source.starts_with("mcp::")).count();
    let plugin_count = tools.iter().filter(|t| t.source.starts_with("plugin::")).count();
    let workflow_count = tools.iter().filter(|t| t.source.starts_with("workflow::")).count();
    
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
    use sentinel_tools::mcp_adapter::{load_mcp_tools_to_server, McpToolMeta, register_mcp_connection, McpConnectionInfo};
    
    let server = get_tool_server();
    
    // Parse tools
    let tool_metas: Vec<McpToolMeta> = tools
        .into_iter()
        .filter_map(|t| {
            let tool_name = t.get("name")?.as_str()?.to_string();
            let description = t.get("description").and_then(|d| d.as_str()).map(String::from);
            let input_schema = t.get("input_schema").cloned().unwrap_or(serde_json::json!({"type": "object"}));
            let connection_id = t.get("connection_id").and_then(|c| c.as_str()).unwrap_or("").to_string();
            
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
    
    tracing::info!("Registered {} MCP tools from server: {}", count, server_name);
    Ok(count)
}

/// Register workflow tools from database
#[tauri::command]
pub async fn register_workflow_tools(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<usize, String> {
    use sentinel_tools::workflow_adapter::{load_workflows_from_db, WorkflowToolMeta};
    
    let server = get_tool_server();
    let db = db_service.inner().clone();
    
    // Load workflows marked as tools
    let workflows = db
        .list_workflow_definitions(Some(false))
        .await
        .map_err(|e| e.to_string())?;
    
    let mut count = 0;
    for workflow in workflows {
        let is_tool = workflow.get("is_tool").and_then(|v| v.as_bool()).unwrap_or(false);
        if !is_tool {
            continue;
        }
        
        let id = workflow.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let name = workflow.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown");
        let description = workflow.get("description").and_then(|v| v.as_str()).unwrap_or("Workflow tool");
        
        if id.is_empty() {
            continue;
        }
        
        // Extract input schema from workflow definition
        let input_schema = sentinel_tools::workflow_adapter::WorkflowToolAdapter::extract_input_schema(&workflow);
        
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
        
        server.register_workflow_tool(id, name, description, input_schema, executor).await;
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

// ============================================================================
// Vision Explorer Credential Commands
// ============================================================================

/// Receive login credentials from frontend for a running VisionExplorer
#[tauri::command]
pub async fn vision_explorer_receive_credentials(
    app: tauri::AppHandle,
    execution_id: String,
    username: String,
    password: String,
    verification_code: Option<String>,
    extra_fields: Option<HashMap<String, String>>,
) -> Result<(), String> {
    tracing::info!("Received credentials for VisionExplorer execution_id: {}", execution_id);
    
    crate::engines::vision_explorer::submit_credentials(
        &execution_id,
        username,
        password,
        verification_code,
        extra_fields,
    ).await?;

    // Notify frontend to close takeover form (some flows may not have an emitter available here).
    // Keep payload compatible with VisionExplorerMessageEmitter.emit_credentials_received
    use tauri::Emitter;
    let payload = serde_json::json!({
        "type": "credentials_received",
        "execution_id": execution_id,
    });
    let _ = app.emit("vision:credentials_received", payload);

    Ok(())
}

/// Send a user message to a running VisionExplorer (for mid-run guidance)
#[tauri::command]
pub async fn vision_explorer_send_user_message(
    execution_id: String,
    message: String,
) -> Result<(), String> {
    tracing::info!(
        "Received user message for VisionExplorer execution_id: {} ({} chars)",
        execution_id,
        message.len()
    );

    crate::engines::vision_explorer::submit_user_message(&execution_id, message).await
}

/// Skip login for a running VisionExplorer (user chose not to provide credentials)
#[tauri::command]
pub async fn vision_explorer_skip_login(
    app: tauri::AppHandle,
    execution_id: String,
) -> Result<(), String> {
    tracing::info!("Skip login requested for VisionExplorer execution_id: {}", execution_id);

    crate::engines::vision_explorer::skip_login(&execution_id).await?;

    // Emit credentials_received event to close takeover UI on the frontend.
    use tauri::Emitter;
    let payload = serde_json::json!({
        "type": "credentials_received",
        "execution_id": execution_id,
    });
    let _ = app.emit("vision:credentials_received", payload);

    Ok(())
}
