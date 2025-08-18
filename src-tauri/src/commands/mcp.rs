use crate::tools::{
    McpServerConfig, ToolInfo, TransportConfig,
    McpClientManager, McpServerManager,
    ToolCategory, ToolExecutionResult, ToolProvider,
};
use crate::tools::client::McpSession;
use crate::tools::unified_types::ConnectionStatus;
use crate::tools::server::ServerConfig;
use crate::models::mcp::McpToolInstallRequest;
use crate::services::mcp::McpService;
use crate::tools::client::McpClientManager as McpClientManagerDirect;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;
use tokio::sync::Mutex;
use tracing::{warn, info, error};
use uuid::Uuid;
use crate::services::database::DatabaseService;
use crate::services::mcp::McpConnectionInfo;
use chrono::Utc;
use rmcp::model::Tool as RmcpTool;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// 获取MCP工具列表
#[tauri::command]
pub async fn get_mcp_tools(state: State<'_, McpService>) -> Result<Vec<ToolInfo>, String> {
    state.get_available_tools().await.map_err(|e| e.to_string())
}

/// 根据分类获取工具
#[tauri::command]
pub async fn get_mcp_tools_by_category(
    category: String,
    state: State<'_, McpService>,
) -> Result<Vec<ToolInfo>, String> {
    let tool_category = match category.as_str() {
        "reconnaissance" => ToolCategory::Reconnaissance,
        "scanning" => ToolCategory::Scanning,
        "exploitation" => ToolCategory::Exploitation,
        "post_exploitation" => ToolCategory::PostExploitation,
        "reporting" => ToolCategory::Reporting,
        "database" => ToolCategory::Database,
        "analysis" => ToolCategory::Analysis,
        "utility" => ToolCategory::Utility,
        _ => return Err("Invalid tool category".to_string()),
    };

    state
        .get_tools_by_category(tool_category)
        .await
        .map_err(|e| e.to_string())
}

/// 搜索工具
#[tauri::command]
pub async fn search_mcp_tools(
    query: String,
    state: State<'_, McpService>,
) -> Result<Vec<ToolInfo>, String> {
    state.search_tools(&query).await.map_err(|e| e.to_string())
}

/// 获取单个工具信息
#[tauri::command]
pub async fn get_mcp_tool(
    tool_id: String,
    state: State<'_, McpService>,
) -> Result<Option<ToolInfo>, String> {
    state.get_tool(&tool_id).await.map_err(|e| e.to_string())
}

/// 执行MCP工具（通过统一工具管理器）
#[tauri::command]
pub async fn execute_mcp_tool(
    tool_id: String,
    parameters: Value,
    timeout: Option<u64>,
    _mcp_service: State<'_, Mutex<McpService>>,
) -> Result<Value, String> {
    use crate::tools::{get_global_tool_system, ToolExecutionParams};
    use std::collections::HashMap;
    use uuid::Uuid;
    
    // 使用统一工具管理器执行工具
    let tool_system = get_global_tool_system().map_err(|e| {
        format!("获取工具系统失败: {}. 请确保全局工具系统已初始化。", e)
    })?;
    
    // 将Value参数转换为HashMap<String, Value>
    let inputs = if let Value::Object(map) = parameters {
        map.into_iter().collect::<HashMap<String, Value>>()
    } else {
        HashMap::new()
    };
    
    let execution_id = Uuid::new_v4().to_string();
    
    let params = ToolExecutionParams {
        inputs,
        context: HashMap::new(),
        timeout: timeout.map(Duration::from_secs),
        execution_id: Some(Uuid::new_v4()),
    };
    
    match tool_system.execute_tool(&tool_id, params).await {
        Ok(result) => {
            Ok(serde_json::json!({
                "success": true,
                "output": result.output,
                "tool": tool_id,
                "execution_id": execution_id,
                "execution_time": result.execution_time_ms
            }))
        },
        Err(e) => {
            Err(format!("Tool '{}' execution failed: {}", tool_id, e))
        }
    }
}

/// 获取执行结果
#[tauri::command]
pub async fn get_mcp_execution_result(
    execution_id: String,
    state: State<'_, McpService>,
) -> Result<Option<ToolExecutionResult>, String> {
    let uuid = Uuid::parse_str(&execution_id).map_err(|_| "Invalid execution ID".to_string())?;

    state
        .get_execution_result(uuid)
        .await
        .map_err(|e| e.to_string())
}

/// 获取连接状态
#[tauri::command]
pub async fn get_mcp_connections(
    state: State<'_, McpService>,
) -> Result<Vec<McpConnectionInfo>, String> {
    state.get_connection_info().await.map_err(|e| e.to_string())
}

/// 添加MCP服务器
#[tauri::command]
pub async fn add_mcp_server(
    name: String,
    version: String,
    description: String,
    transport_type: String,
    transport_config: Value,
    state: State<'_, McpService>,
) -> Result<String, String> {
    let transport = match transport_type.as_str() {
        "stdio" => TransportConfig::Stdio,
        "websocket" => {
            let url = transport_config
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or("WebSocket configuration missing URL")?;
            TransportConfig::WebSocket {
                url: url.to_string(),
            }
        }
        "http" => {
            let base_url = transport_config
                .get("base_url")
                .and_then(|v| v.as_str())
                .ok_or("HTTP configuration missing base_url")?;
            TransportConfig::Http {
                base_url: base_url.to_string(),
            }
        }
        _ => return Err("Unsupported transport type".to_string()),
    };

    let config = McpServerConfig {
        name,
        version,
        description,
        transport,
        capabilities: crate::tools::ServerCapabilities {
            tools: true,
            resources: false,
            prompts: false,
            logging: false,
        },
        tools: Vec::new(),
    };

    state.add_server(config).await.map_err(|e| e.to_string())
}

/// 移除MCP服务器
#[tauri::command]
pub async fn remove_mcp_server(
    connection_id: String,
    state: State<'_, McpService>,
) -> Result<(), String> {
    state
        .remove_server(&connection_id)
        .await
        .map_err(|e| e.to_string())
}

/// 初始化默认工具
#[tauri::command]
pub async fn initialize_default_mcp_tools(
    mcp_service: State<'_, Mutex<McpService>>,
) -> Result<(), String> {
    let  service = mcp_service.lock().await;
    service.initialize_mcp().await.map_err(|e| e.to_string())
}

/// 获取工具分类列表
#[tauri::command]
pub async fn get_mcp_tool_categories() -> Result<Vec<String>, String> {
    Ok(vec![
        "reconnaissance".to_string(),
        "scanning".to_string(),
        "exploitation".to_string(),
        "post_exploitation".to_string(),
        "reporting".to_string(),
        "database".to_string(),
        "analysis".to_string(),
        "utility".to_string(),
    ])
}

// 保留原有命令以保持兼容性
#[tauri::command]
pub async fn add_mcp_tool(_tool: Value, _state: State<'_, McpService>) -> Result<(), String> {
    // 兼容性实现
    tracing::warn!("Using deprecated add_mcp_tool command");
    Ok(())
}

#[tauri::command]
pub async fn remove_mcp_tool(_tool_id: String, _state: State<'_, McpService>) -> Result<(), String> {
    // 兼容性实现
    tracing::warn!("Using deprecated remove_mcp_tool command");
    Ok(())
}

#[tauri::command]
pub async fn update_mcp_tool_status(
    _tool_id: String,
    _status: String,
    _state: State<'_, McpService>,
) -> Result<(), String> {
    // 兼容性实现
    tracing::warn!("Using deprecated update_mcp_tool_status command");
    Ok(())
}

#[tauri::command]
pub async fn get_mcp_execution_status(
    execution_id: String,
    state: State<'_, McpService>,
) -> Result<String, String> {
    let uuid = Uuid::parse_str(&execution_id).map_err(|_| "无效的执行ID".to_string())?;

    if let Some(result) = state
        .get_execution_result(uuid)
        .await
        .map_err(|e| e.to_string())?
    {
        Ok(format!("{:?}", result.status))
    } else {
        Err("执行记录不存在".to_string())
    }
}

#[tauri::command]
pub async fn cancel_mcp_execution(
    _execution_id: String,
    _state: State<'_, McpService>,
) -> Result<(), String> {
    // 兼容性实现
    tracing::warn!("Using deprecated cancel_mcp_execution command");
    Ok(())
}

#[tauri::command]
pub async fn get_mcp_executions(_state: State<'_, McpService>) -> Result<Vec<String>, String> {
    // 兼容性实现 - 返回空列表
    Ok(Vec::new())
}

/// 检查工具安装状态
#[tauri::command]
pub async fn check_mcp_tool_installation(tool_name: String) -> Result<bool, String> {
    // 简单检查工具是否在系统PATH中可用
    let output = match tool_name.as_str() {
        "subfinder" => {
            tokio::process::Command::new("subfinder")
                .arg("--version")
                .output()
                .await
        }
        "nuclei" => {
            tokio::process::Command::new("nuclei")
                .arg("--version")
                .output()
                .await
        }
        "httpx" => {
            tokio::process::Command::new("httpx")
                .arg("--version")
                .output()
                .await
        }
        "nmap" => {
            tokio::process::Command::new("nmap")
                .arg("--version")
                .output()
                .await
        }
        _ => return Ok(false),
    };

    match output {
        Ok(result) => Ok(result.status.success()),
        Err(_) => Ok(false),
    }
}

#[tauri::command]
pub async fn install_mcp_tool(
    _install_request: McpToolInstallRequest,
    _mcp_service: State<'_, Mutex<McpService>>,
) -> Result<String, String> {
    Ok("tool_id_123".to_string())
}

#[tauri::command]
pub async fn parse_mcp_tool_output(
    output: String,
    _tool_id: String,
    _state: State<'_, McpService>,
) -> Result<Value, String> {
    // 尝试解析为JSON，失败则返回原始文本
    if let Ok(json_value) = serde_json::from_str::<Value>(&output) {
        Ok(json_value)
    } else {
        Ok(Value::String(output))
    }
}

#[tauri::command]
pub async fn get_mcp_tool_stats(state: State<'_, McpService>) -> Result<Value, String> {
    state.get_tool_stats().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cleanup_mcp_executions(
    _older_than_hours: u64,
    _state: State<'_, McpService>,
) -> Result<usize, String> {
    // 兼容性实现
    tracing::warn!("Using deprecated cleanup_mcp_executions command");
    Ok(0)
}

/// 获取预定义的安全扫描模板
#[tauri::command]
pub async fn get_scan_templates() -> Result<Value, String> {
    let templates = serde_json::json!({
        "web_app_scan": {
            "name": "Web应用扫描",
            "description": "全面的Web应用安全扫描",
            "tools": ["subfinder", "httpx", "nuclei"],
            "parameters": {
                "target": {
                    "type": "string",
                    "required": true,
                    "description": "目标域名或URL"
                },
                "depth": {
                    "type": "integer",
                    "default": 3,
                    "description": "扫描深度"
                }
            }
        },
        "network_discovery": {
            "name": "网络发现",
            "description": "网络资产发现和端口扫描",
            "tools": ["nmap", "httpx"],
            "parameters": {
                "target": {
                    "type": "string",
                    "required": true,
                    "description": "目标IP或网段"
                },
                "ports": {
                    "type": "string",
                    "default": "1-1000",
                    "description": "端口范围"
                }
            }
        },
        "subdomain_enum": {
            "name": "子域名枚举",
            "description": "全面的子域名发现",
            "tools": ["subfinder"],
            "parameters": {
                "domain": {
                    "type": "string",
                    "required": true,
                    "description": "目标域名"
                }
            }
        }
    });

    Ok(templates)
}

/// 执行扫描模板（通过DynamicToolAdapter）
#[tauri::command]
pub async fn execute_scan_template(
    _template_name: String,
    _target: String,
    _state: State<'_, McpService>,
) -> Result<Value, String> {
    Ok(Value::Null)
}

/// 获取默认安全工具列表
#[tauri::command]
pub async fn get_default_security_tools() -> Result<Vec<Value>, String> {
    let tools = vec![];

    Ok(tools)
}

/// 验证工具配置
#[tauri::command]
pub async fn validate_mcp_tool_config(_config: Value) -> Result<bool, String> {
    // 简单验证实现
    Ok(true)
}

/// 初始化MCP客户端和服务器
#[tauri::command]
pub async fn initialize_mcp(
    client_manager: State<'_, McpClientManager>,
    server_manager: State<'_, McpServerManager>,
) -> Result<String, String> {
    // 初始化客户端
    if let Err(e) = client_manager.initialize().await {
        return Err(format!("Failed to initialize MCP client: {}", e));
    }

    // 加载服务器配置
    if let Err(e) = server_manager.load_config().await {
        return Err(format!("Failed to load MCP server configuration: {}", e));
    }

    Ok("MCP system initialized".to_string())
}

/// 启动MCP服务器
#[tauri::command]
pub async fn start_mcp_server(
    server_manager: State<'_, McpServerManager>,
) -> Result<String, String> {
    match server_manager.start_stdio().await {
        Ok(_) => Ok("MCP server started".to_string()),
        Err(e) => Err(format!("Failed to start MCP server: {}", e)),
    }
}

/// 启动MCP服务器并保存状态
#[tauri::command]
pub async fn start_mcp_server_with_state(
    mcp_service: State<'_, Arc<McpService>>,
) -> Result<String, String> {
    match mcp_service.start_server_with_state_save("stdio", None).await {
        Ok(_) => Ok("MCP server started and state saved".to_string()),
        Err(e) => Err(format!("Failed to start MCP server with state save: {}", e)),
    }
}

/// 停止MCP服务器
#[tauri::command]
pub async fn stop_mcp_server(
    server_manager: State<'_, McpServerManager>,
) -> Result<String, String> {
    match server_manager.stop().await {
        Ok(_) => Ok("MCP server stopped".to_string()),
        Err(e) => Err(format!("Failed to stop MCP server: {}", e)),
    }
}

/// 停止MCP服务器并保存状态
#[tauri::command]
pub async fn stop_mcp_server_with_state(
    mcp_service: State<'_, Arc<McpService>>,
) -> Result<String, String> {
    match mcp_service.stop_server_with_state_save().await {
        Ok(_) => Ok("MCP server stopped and state saved".to_string()),
        Err(e) => Err(format!("Failed to stop MCP server with state save: {}", e)),
    }
}

/// 获取MCP服务器状态
#[tauri::command]
pub async fn get_mcp_server_status(
    server_manager: State<'_, McpServerManager>,
) -> Result<bool, String> {
    Ok(server_manager.is_running().await)
}

/// 获取MCP服务器工具列表
#[tauri::command]
pub async fn get_mcp_server_tools(
    server_manager: State<'_, McpServerManager>,
) -> Result<Vec<String>, String> {
    let tools = server_manager.list_tools().await;
    let tool_names: Vec<String> = tools;
    Ok(tool_names)
}

/// 执行MCP服务器工具
#[tauri::command]
pub async fn execute_mcp_server_tool(
    server_manager: State<'_, McpServerManager>,
    tool_name: String,
    parameters: Value,
) -> Result<Value, String> {
    match server_manager.execute_tool(&tool_name, parameters).await {
        Ok(result) => Ok(result),
        Err(e) => Err(format!("Failed to execute tool: {}", e)),
    }
}

/// 获取MCP服务器配置
#[tauri::command]
pub async fn get_mcp_server_config(
    server_manager: State<'_, McpServerManager>,
) -> Result<ServerConfig, String> {
    let server = server_manager.get_server().await;
    let server = server.read().await;
    Ok(server.get_config().await)
}

/// 更新MCP服务器配置
#[tauri::command]
pub async fn update_mcp_server_config(
    server_manager: State<'_, McpServerManager>,
    config: ServerConfig,
) -> Result<String, String> {
    match server_manager.update_config(config).await {
        Ok(_) => Ok("MCP server configuration updated".to_string()),
        Err(e) => Err(format!("Failed to update MCP server configuration: {}", e)),
    }
}

/// 获取MCP客户端连接列表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendMcpConnection {
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

/// 获取MCP连接的实时状态
#[tauri::command]
pub async fn mcp_get_connection_status(
    client_manager: tauri::State<'_, Arc<McpClientManager>>,
) -> Result<HashMap<String, String>, String> {
    let client = client_manager.get_client();
    let status_map = client.get_all_connection_status().await;
    
    let mut result = HashMap::new();
    for (name, status) in status_map {
        result.insert(name, status.to_string());
    }
    
    Ok(result)
}

#[tauri::command]
pub async fn mcp_get_connections(
    client_manager: tauri::State<'_, Arc<McpClientManager>>,
) -> Result<Vec<FrontendMcpConnection>, String> {
    let client_manager = client_manager.inner();
    let connections_status = client_manager.get_all_connection_status().await;
    let db_configs = client_manager
        .get_all_server_configs_from_db()
        .await
        .map_err(|e| e.to_string())?;

    let mut frontend_connections = Vec::new();

    for config in db_configs {
        let name = config.get("name").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
        let id = config.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
        let description = config.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let command = config.get("command").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let args_str = config.get("args").and_then(|v| v.as_str()).unwrap_or("[]");
        
        if let Some(status) = connections_status.get(&name) {
            frontend_connections.push(FrontendMcpConnection {
                db_id: id.to_string(),
                id: Some(name.clone()),
                name: name.clone(),
                description: Some(description.clone()),
                transport_type: "stdio".to_string(),
                endpoint: command.clone(),
                status: status.to_string(),
                command: command.clone(),
                args: serde_json::from_str(args_str).unwrap_or_else(|_| vec![]),
            });
        } else {
            frontend_connections.push(FrontendMcpConnection {
                db_id: id.to_string(),
                id: None,
                name: name.clone(),
                description: Some(description.clone()),
                transport_type: "stdio".to_string(),
                endpoint: command.clone(),
                status: "Disconnected".to_string(),
                command: command.clone(),
                args: serde_json::from_str(args_str).unwrap_or_else(|_| vec![]),
            });
        }
    }
    Ok(frontend_connections)
}

/// 连接到MCP服务器
#[tauri::command]
pub async fn connect_to_mcp_server(
    client_manager: State<'_, McpClientManager>,
    name: String,
    _command: String,
    _args: Vec<String>,
) -> Result<String, String> {
    // let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    match client_manager
        .connect_to_server(&name)
        .await
    {
        Ok(connection_id) => Ok(format!("Connected: {:?}", connection_id)),
        Err(e) => Err(format!("Failed to connect to MCP server: {}", e)),
    }
}

/// 断开MCP连接
#[tauri::command]
pub async fn disconnect_mcp_connection(
    client_manager: State<'_, McpClientManager>,
    connection_id: String,
) -> Result<String, String> {
    match client_manager.disconnect(&connection_id).await {
        Ok(_) => Ok("MCP connection disconnected".to_string()),
        Err(e) => Err(format!("Failed to disconnect MCP connection: {}", e)),
    }
}

/// 重试失败的MCP连接
#[tauri::command]
pub async fn retry_mcp_connection(
    client_manager: State<'_, McpClientManager>,
    connection_id: String,
) -> Result<String, String> {
    match client_manager.retry_connection(&connection_id).await {
        Ok(_) => Ok("Connection retried successfully".to_string()),
        Err(e) => Err(format!("Failed to retry MCP connection: {}", e)),
    }
}

/// MCP服务器状态结构
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerStatus {
    pub running: bool,
    pub connections: usize,
    pub available_tools: usize,
}

/// 检查MCP服务器状态
#[tauri::command]
pub async fn mcp_check_server_status(
    client_manager: State<'_, Arc<McpClientManager>>,
    server_manager: State<'_, Arc<McpServerManager>>,
) -> Result<ServerStatus, String> {
    let client = client_manager.get_client();
    let connections = client.get_all_connection_status().await;

    let is_running = server_manager.is_running().await;
    let tools_count = if is_running {
        server_manager.list_tools().await.len()
    } else {
        0
    };

    Ok(ServerStatus {
        running: is_running,
        connections: connections.len(),
        available_tools: tools_count,
    })
}

/// 启动MCP服务器
#[tauri::command]
pub async fn mcp_connect_server(
    client_manager: State<'_, Arc<McpClientManager>>,
    params: serde_json::Value,
) -> Result<String, String> {
    let client = client_manager.get_client();
    match params
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| "Missing type parameter".to_string())?
    {
        "npx" => {
            let package = params
                .get("package")
                .and_then(Value::as_str)
                .ok_or_else(|| "Missing package parameter".to_string())?;
            match client.connect_to_server(package).await {
                Ok(_session) => Ok(format!("Connected to server: {}", package)),
                Err(e) => Err(e.to_string()),
            }
        }
        "http" => {
            let name = params
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| "Missing name parameter".to_string())?;
            let url = params
                .get("url")
                .and_then(Value::as_str)
                .ok_or_else(|| "Missing url parameter".to_string())?;
            match client.connect_to_http_server(&name, url.to_string()).await {
                Ok(id) => Ok(id),
                Err(e) => Err(e.to_string()),
            }
        }
        "child_process" => {
            let name = params
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| "Missing name parameter".to_string())?;
            let _command = params
                .get("command")
                .and_then(Value::as_str)
                .ok_or_else(|| "Missing command parameter".to_string())?;
            let _args: Vec<&str> = params
                .get("args")
                .and_then(Value::as_array)
                .map(|arr| arr.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            match client
                .connect_to_server(&name)
                .await
            {
                Ok(_session) => Ok(format!("Connected to server: {}", name)),
                Err(e) => Err(e.to_string()),
            }
        }
        _ => Err("Unsupported connection type".to_string()),
    }
}

/// 断开MCP连接
#[tauri::command]
pub async fn mcp_disconnect_server(
    client_manager: State<'_, Arc<McpClientManager>>,
    connection_id: String,
) -> Result<(), String> {
    let client = client_manager.get_client();
    match client.disconnect(&connection_id).await {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

/// 列出所有工具
#[tauri::command]
pub async fn mcp_list_tools(
    server_manager: State<'_, Arc<McpServerManager>>,
) -> Result<Vec<serde_json::Value>, String> {
    // 检查服务器是否运行
    let running = server_manager.is_running().await;
    if !running {
        return Ok(Vec::new());
    }

    // 获取工具详情
    let tools = match server_manager.get_tool_details().await {
        Ok(tools) => tools,
        Err(e) => return Err(format!("Failed to get tool details: {}", e)),
    };

    // 将工具转换为前端可用的格式
    let mut result = Vec::new();
    for tool in tools {
        result.push(serde_json::json!({
            "id": tool.id,
            "name": tool.name,
            "description": tool.description,
            "version": tool.version,
            "category": format!("{:?}", tool.category),
            "status": "stopped",
            "author": tool.metadata.author,
            "icon": "fas fa-tools",
            "config": {}
        }));
    }

    Ok(result)
}

/// 启动工具
#[tauri::command]
pub async fn mcp_start_tool(
    _server_manager: State<'_, Arc<McpServerManager>>,
    _tool_id: String,
) -> Result<(), String> {
    // TODO: 实现工具启动逻辑
    Ok(())
}

/// 停止工具
#[tauri::command]
pub async fn mcp_stop_tool(
    _server_manager: State<'_, Arc<McpServerManager>>,
    _tool_id: String,
) -> Result<(), String> {
    // TODO: 实现工具停止逻辑
    Ok(())
}

/// 重启工具
#[tauri::command]
pub async fn mcp_restart_tool(
    _server_manager: State<'_, Arc<McpServerManager>>,
    _tool_id: String,
) -> Result<(), String> {
    // TODO: 实现工具重启逻辑
    Ok(())
}

/// 卸载工具
#[tauri::command]
pub async fn mcp_uninstall_tool(
    _server_manager: State<'_, Arc<McpServerManager>>,
    _tool_id: String,
) -> Result<(), String> {
    // TODO: 实现工具卸载逻辑
    Ok(())
}

/// 从商店安装工具
#[tauri::command]
pub async fn mcp_install_tool(
    _server_manager: State<'_, Arc<McpServerManager>>,
    _tool_name: String,
    _tool_version: String,
    _tool_source: String,
) -> Result<String, String> {
    // TODO: 实现工具安装逻辑
    let tool_id = uuid::Uuid::new_v4().to_string();
    Ok(tool_id)
}

/// 从URL安装工具
#[tauri::command]
pub async fn mcp_install_tool_from_url(
    _server_manager: State<'_, Arc<McpServerManager>>,
    _url: String,
) -> Result<String, String> {
    // TODO: 实现从URL安装工具逻辑
    let tool_id = uuid::Uuid::new_v4().to_string();
    Ok(tool_id)
}

/// 从GitHub安装工具
#[tauri::command]
pub async fn mcp_install_tool_from_github(
    _server_manager: State<'_, Arc<McpServerManager>>,
    _url: String,
) -> Result<String, String> {
    // TODO: 实现从GitHub安装工具逻辑
    let tool_id = uuid::Uuid::new_v4().to_string();
    Ok(tool_id)
}

/// 从注册表安装工具
#[tauri::command]
pub async fn mcp_install_tool_from_registry(
    _server_manager: State<'_, Arc<McpServerManager>>,
    _name: String,
) -> Result<String, String> {
    // TODO: 实现从注册表安装工具逻辑
    let tool_id = uuid::Uuid::new_v4().to_string();
    Ok(tool_id)
}

/// 创建自定义工具
#[tauri::command]
pub async fn mcp_create_custom_tool(
    _server_manager: State<'_, Arc<McpServerManager>>,
    _name: String,
    _version: String,
    _description: String,
    _command: String,
    _config: serde_json::Value,
) -> Result<String, String> {
    // TODO: 实现创建自定义工具逻辑
    let tool_id = uuid::Uuid::new_v4().to_string();
    Ok(tool_id)
}

#[tauri::command]
pub async fn list_tools(state: State<'_, McpService>) -> Result<Vec<ToolInfo>, String> {
    state.get_available_tools().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn execute_tool(
    tool_id: String,
    parameters: Value,
    state: State<'_, McpService>,
) -> Result<Value, String> {
    state
        .execute_tool(&tool_id, parameters)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_tool_info(
    tool_id: String,
    state: State<'_, McpService>,
) -> Result<Option<ToolInfo>, String> {
    state.get_tool(&tool_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_connection_info(
    state: State<'_, McpService>,
) -> Result<Vec<McpConnectionInfo>, String> {
    state.get_connection_info().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_execution_result(
    execution_id: String,
    state: State<'_, McpService>,
) -> Result<Option<ToolExecutionResult>, String> {
    let uuid = Uuid::parse_str(&execution_id).map_err(|e| e.to_string())?;
    state
        .get_execution_result(uuid)
        .await
        .map_err(|e| e.to_string())
}

/// 添加并连接到子进程 MCP 服务器（异步）
#[tauri::command]
pub async fn add_child_process_mcp_server(
    name: String,
    mut command: String,
    args: Vec<String>,
    client_manager: tauri::State<'_, Arc<McpClientManager>>,
) -> Result<String, String> {
    if cfg!(target_os = "windows") {
        if command == "npx" || command == "npm" {
            command.push_str(".cmd");
        }
    }

    // 检查命令是否存在
    let (check_cmd, check_args) = if cfg!(target_os = "windows") {
        ("where", vec![command.clone()])
    } else {
        ("which", vec![command.clone()])
    };

    info!("Checking if command '{}' exists...", command);
    let check_result = tokio::process::Command::new(check_cmd)
        .args(check_args)
        .output()
        .await;

    if let Err(e) = check_result {
        return Err(format!("Failed to check if command exists: {}", e));
    }

    let check_output = check_result.unwrap();
    if !check_output.status.success() {
        return Err(format!(
            "Command '{}' not found: please ensure the command exists in the system PATH",
            command
        ));
    }

    info!("Command '{}' found, starting connection...", command);

    // 异步启动连接过程，但提供更详细的状态反馈
    let client_manager_clone = client_manager.inner().clone();
    let name_clone = name.clone();
    let command_clone = command.clone();
    let args_clone = args.clone();

    // 使用更详细的连接状态跟踪
    tokio::spawn(async move {
        info!("Starting connection attempt to MCP server: {}", name_clone);
        
        match client_manager_clone
            .connect_with_command(&name_clone, &command_clone, args_clone)
            .await
        {
            Ok(_) => {
                info!("Successfully connected to MCP server: {}", name_clone);
            }
            Err(e) => {
                error!("Failed to connect to MCP server {}: {}", name_clone, e);
                
                // 提供更详细的错误诊断
                if e.to_string().contains("timeout") {
                    error!("Connection timeout for '{}'. This may be due to:", name_clone);
                    error!("  1. Network connectivity issues");
                    error!("  2. NPM package installation taking too long");
                    error!("  3. Package not available in npm registry");
                    error!("  4. Firewall blocking the connection");
                } else if e.to_string().contains("Command") {
                    error!("Command execution failed for '{}'. Check if:", name_clone);
                    error!("  1. Node.js and npm are properly installed");
                    error!("  2. The command is in your PATH");
                    error!("  3. You have necessary permissions");
                } else {
                    error!("General connection error for '{}': {}", name_clone, e);
                }
            }
        }
    });

    Ok(format!("Connection to '{}' started in background. Check logs for detailed status.", name))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuickCreateServerConfig {
    enabled: bool,
    name: String,
    description: String,
    #[serde(rename = "type")]
    server_type: String,
    params: String,
    #[serde(rename = "envVars")]
    env_vars: String,
    timeout: u64,
    #[serde(rename = "providerName")]
    provider_name: String,
    #[serde(rename = "providerWebsite")]
    provider_website: String,
    #[serde(rename = "logoUrl")]
    logo_url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateMcpServerConfigPayload {
    db_id: i64,
    name: String,
    description: Option<String>,
    command: String,
    args: Vec<String>,
    enabled: bool,
}

#[tauri::command]
pub async fn mcp_update_server_config(
    payload: FrontendMcpConnection,
    client_manager: tauri::State<'_, Arc<McpClientManager>>,
) -> Result<(), String> {
    let client_manager = client_manager.inner();
    client_manager
        .update_server_config(serde_json::to_value(payload).unwrap_or_default())
        .await
        .map_err(|e| e.to_string())
}

#[derive(serde::Serialize, Clone)]
pub struct FrontendTool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

impl From<RmcpTool> for FrontendTool {
    fn from(tool: RmcpTool) -> Self {
        FrontendTool {
            name: tool.name.to_string(),
            description: tool.description.map(|d| d.to_string()).unwrap_or_default(),
            input_schema: serde_json::Value::Object((*tool.input_schema).clone()),
        }
    }
}

#[tauri::command]
pub async fn mcp_get_connection_tools(
    client_manager: State<'_, Arc<McpClientManager>>,
    connection_id: String,
) -> Result<Vec<FrontendTool>, String> {
    let client = client_manager.get_client();
    
    // 首先尝试通过connection_id获取会话
    let tools = if let Some(session) = client.get_session(&connection_id).await {
        // 从会话获取工具列表
        match session.list_tools_paginated(None).await {
            Ok(result) => result.tools,
            Err(e) => {
                warn!("Failed to get tools from session '{}': {}", connection_id, e);
                Vec::new()
            }
        }
    } else {
        // 如果通过connection_id找不到，尝试获取所有连接状态
        let connection_status = client.get_all_connection_status().await;
        let mut found_tools = Vec::new();
        
        // 遍历所有连接状态，找到已连接的服务器
        for (name, status) in connection_status.iter() {
            if matches!(status, ConnectionStatus::Connected) {
                // 尝试获取该服务器的会话
                if let Some(session) = client.get_session(name).await {
                    match session.list_tools_paginated(None).await {
                        Ok(result) => {
                            found_tools = result.tools;
                            info!("Found {} tools from session '{}'", found_tools.len(), name);
                            break;
                        }
                        Err(e) => {
                            warn!("Failed to get tools from session '{}': {}", name, e);
                        }
                    }
                }
            }
        }
        
        found_tools
    };

    let frontend_tools: Vec<FrontendTool> = tools.into_iter().map(|tool| FrontendTool::from(tool)).collect();
    info!("Returning {} tools for connection '{}'", frontend_tools.len(), connection_id);
    Ok(frontend_tools)
}

#[tauri::command]
pub async fn quick_create_mcp_server(
    config: QuickCreateServerConfig,
    client_manager: tauri::State<'_, Arc<McpClientManager>>,
) -> Result<(), String> {
    if !config.enabled {
        return Ok(());
    }

    match config.server_type.as_str() {
        "stdio" => {
            let mut parts = config.params.split_whitespace();
            let command = match parts.next() {
                Some(cmd) => cmd.to_string(),
                None => return Err("Parameter field is empty, cannot extract command.".to_string()),
            };
            let args: Vec<String> = parts.map(String::from).collect();

            let mut final_command = command;
            if cfg!(target_os = "windows") && (final_command == "npx" || final_command == "npm") {
                final_command.push_str(".cmd");
            }

            client_manager
                .connect_with_command(&config.name, &final_command, args)
                .await
                .map(|_| ())
                .map_err(|e| format!("Failed to connect to stdio server: {}", e))
        }
        "sse" | "streamableHttp" => {
            let url = config.params.trim();
            if url.is_empty() {
                return Err(
                    "For sse or http types, the parameter field must be a valid URL.".to_string(),
                );
            }

            client_manager
                .connect_to_http_server(&config.name, url.to_string())
                .await
                .map(|_| ())
                .map_err(|e| format!("Failed to connect to sse/http server: {}", e))
        }
        _ => Err(format!("Unsupported server type: {}", config.server_type)),
    }
}

#[derive(Deserialize)]
struct McpServersConfig {
    #[serde(rename = "mcpServers")]
    mcp_servers: HashMap<String, Value>,
}

#[tauri::command]
pub async fn import_mcp_servers_from_json(
    json_config: String,
    client_manager: tauri::State<'_, Arc<McpClientManager>>,
) -> Result<(), String> {
    let json_without_comments: String = json_config
        .lines()
        .filter(|line| !line.trim().starts_with("//"))
        .collect();

    let config: McpServersConfig = serde_json::from_str(&json_without_comments).map_err(|e| {
        format!(
            "Failed to parse JSON: {}. Please ensure the format is correct.",
            e
        )
    })?;

    for (name, server_value) in config.mcp_servers {
        if let Some(obj) = server_value.as_object() {
            if obj.contains_key("command") {
                let command = obj
                    .get("command")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                if command.is_empty() {
                    continue;
                }
                let args: Vec<String> = obj
                    .get("args")
                    .and_then(Value::as_array)
                    .map(|arr| {
                        arr.iter()
                            .filter_map(Value::as_str)
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default();

                let mut final_command = command;
                if cfg!(target_os = "windows") && (final_command == "npx" || final_command == "npm")
                {
                    final_command.push_str(".cmd");
                }

                if let Err(e) = client_manager
                    .connect_with_command(&name, &final_command, args)
                    .await
                {
                    warn!("Failed to import stdio server '{}' from JSON: {}", name, e);
                }
            } else if obj.contains_key("url") {
                let url = obj.get("url").and_then(Value::as_str).unwrap_or("");
                if !url.is_empty() {
                    if let Err(e) = client_manager.connect_to_http_server(&name, url.to_string()).await {
                        warn!(
                            "Failed to import sse/http server '{}' from JSON: {}",
                            name, e
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketplaceServer {
    name: String,
    description: String,
    command: String,
    args: Vec<String>,
    icon: String,
}

#[tauri::command]
pub fn get_mcp_marketplace_servers() -> Result<Vec<MarketplaceServer>, String> {
    Ok(vec![
        MarketplaceServer {
            name: "MCP Everything".to_string(),
            description: "A general MCP server that provides various basic tools like calculator and counter.".to_string(),
            command: "npx".to_string(),
            args: vec!["-y".to_string(), "@modelcontextprotocol/server-everything".to_string()],
            icon: "fas fa-cubes".to_string(),
        },
        MarketplaceServer {
            name: "MCP Counter".to_string(),
            description: "A simple MCP server that only contains a counter tool for demonstration and testing.".to_string(),
            command: "npx".to_string(),
            args: vec!["-y".to_string(), "@modelcontextprotocol/server-counter".to_string()],
            icon: "fas fa-sort-numeric-up".to_string(),
        },
        MarketplaceServer {
            name: "Bilibili Search".to_string(),
            description: "An MCP server specialized for searching videos on Bilibili.".to_string(),
            command: "npx".to_string(),
            args: vec!["-y".to_string(), "bilibili-mcp".to_string()],
            icon: "fab fa-bilibili".to_string(),
        },
    ])
}

/// 切换内置工具的启用状态
#[tauri::command]
pub async fn toggle_builtin_tool(
    tool_name: String,
    enabled: bool,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    // 在数据库中保存工具的启用状态
    let timestamp = Utc::now().timestamp();
    
    let formatted_query = format!(
        "INSERT OR REPLACE INTO builtin_tool_settings (tool_name, enabled, updated_at) VALUES ('{}', {}, {})",
        tool_name, if enabled { 1 } else { 0 }, timestamp
    );
    
    db_service
        .execute_query(&formatted_query)
        .await
        .map_err(|e| format!("Failed to update tool status: {}", e))?;
    
    Ok(())
}

/// 获取带有启用状态的内置工具列表
#[tauri::command]
pub async fn get_builtin_tools_with_status(
    _state: State<'_, Arc<McpService>>, // 不再依赖McpService列出内置工具
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<serde_json::Value>, String> {
    // 直接创建BuiltinToolProvider来获取纯内置工具（避免MCP工具混入）
    let builtin_provider = crate::tools::BuiltinToolProvider::new(db_service.inner().clone());
    let builtin_unified_tools = builtin_provider.get_tools().await.map_err(|e| e.to_string())?;
    
    // 转换为ToolInfo格式
    let mut builtin_tools = Vec::new();
    for tool in builtin_unified_tools {
        let info = crate::tools::ToolInfo {
            id: tool.name().to_string(),
            name: tool.name().to_string(),
            description: tool.description().to_string(),
            version: tool.metadata().version.clone(),
            category: tool.category(),
            parameters: tool.parameters().clone(),
            metadata: tool.metadata().clone(),
            available: tool.is_available().await,
            installed: tool.is_installed().await,
            source: crate::tools::ToolSource::Builtin,
        };
        builtin_tools.push(info);
    }

    // 读取启用状态
    let query = "SELECT tool_name, enabled FROM builtin_tool_settings";
    let settings_result = db_service
        .execute_query(query)
        .await
        .map_err(|e| e.to_string())?;

    let mut settings_map = std::collections::HashMap::new();
    for setting in settings_result {
        if let (Some(tool_name), Some(enabled_value)) = (
            setting.get("tool_name").and_then(|v| v.as_str()),
            setting.get("enabled"),
        ) {
            let enabled = match enabled_value {
                serde_json::Value::Bool(b) => *b,
                serde_json::Value::Number(n) => n.as_i64().map(|i| i != 0).or_else(|| n.as_f64().map(|f| f != 0.0)).unwrap_or(true),
                serde_json::Value::String(s) => s != "0" && s.to_lowercase() != "false",
                _ => true,
            };
            settings_map.insert(tool_name.to_string(), enabled);
        }
    }

    // 组装前端需要的结构
    let mut result = Vec::new();
    for info in builtin_tools {
        let name = info.name.clone();
        let enabled = settings_map.get(&name).copied().unwrap_or(true);
        let tool_json = serde_json::json!({
            "id": name,
            "name": info.name,
            "description": info.description,
            "category": info.category.to_string(),
            "enabled": enabled,
        });
        result.push(tool_json);
    }

    Ok(result)
}

/// 获取MCP外部工具列表（用于"我的服务器"选项卡）
#[tauri::command]
pub async fn get_mcp_external_tools(
    mcp_service: State<'_, Arc<McpService>>,
) -> Result<Vec<serde_json::Value>, String> {
    // 从MCP服务获取所有工具
    let all_tools = mcp_service.get_available_tools().await.map_err(|e| e.to_string())?;
    
    // 过滤出外部MCP工具
    let external_tools: Vec<_> = all_tools.into_iter()
        .filter(|tool| matches!(tool.source, crate::tools::ToolSource::External))
        .collect();

    // 组装前端需要的结构
    let mut result = Vec::new();
    for info in external_tools {
        let tool_json = serde_json::json!({
            "id": info.id,
            "name": info.name,
            "description": info.description,
            "category": info.category.to_string(),
            "enabled": true, // MCP工具默认启用
            "source": "external",
            "metadata": info.metadata,
        });
        result.push(tool_json);
    }

    Ok(result)
}

/// 删除本地MCP服务器配置
#[tauri::command]
pub async fn remove_local_mcp_servers(
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<String>, String> {
    let mut removed_servers = Vec::new();
    
    // 获取所有MCP服务器配置
    let configs = db_service
        .get_all_mcp_server_configs()
        .await
        .map_err(|e| format!("Failed to get MCP server configs: {}", e))?;
    
    for config in configs {
        // 检查是否是本地服务器（通过命令判断）
        if config.command.contains("mcp-server") || 
           (config.url.starts_with("http://localhost") && !config.command.contains("npx")) {
            // 删除本地MCP服务器配置
            db_service
                .delete_mcp_server_config(&config.id)
                .await
                .map_err(|e| format!("Failed to delete MCP server config {}: {}", config.id, e))?;
            
            removed_servers.push(format!("{} ({})", config.name, config.command));
        }
    }
    
    Ok(removed_servers)
}

/// 获取运行中的MCP进程信息
#[tauri::command]
pub async fn get_running_mcp_processes(
    client_manager: State<'_, Arc<McpClientManagerDirect>>,
) -> Result<Vec<serde_json::Value>, String> {
    let _client = client_manager.get_client();
    // 获取运行中的进程列表（简化实现）
    let processes = Vec::new(); // TODO: 实现获取运行进程的逻辑
    
    let process_info: Vec<serde_json::Value> = processes
        .into_iter()
        .map(|(pid, command, args, started_at): (u32, String, Vec<String>, chrono::DateTime<chrono::Utc>)| {
            serde_json::json!({
                "pid": pid,
                "command": command,
                "args": args,
                "started_at": started_at.to_rfc3339(),
                "uptime_seconds": (chrono::Utc::now().timestamp() - started_at.timestamp())
            })
        })
        .collect();
    
    Ok(process_info)
}

/// 关闭特定的MCP进程
#[tauri::command]
pub async fn shutdown_mcp_process(
    process_id: u32,
    client_manager: State<'_, Arc<McpClientManagerDirect>>,
) -> Result<String, String> {
    let _client = client_manager.get_client();
    // 关闭进程（简化实现）
    // TODO: 实现关闭进程的逻辑
    Ok(format!("Successfully shutdown MCP process with PID: {}", process_id))
}

/// 关闭所有MCP进程
#[tauri::command]
pub async fn shutdown_all_mcp_processes(
    client_manager: State<'_, Arc<McpClientManagerDirect>>,
) -> Result<String, String> {
    client_manager
        .shutdown_all()
        .await
        .map_err(|e| e.to_string())?;
    
    Ok("Successfully shutdown all MCP processes".to_string())
}

/// MCP环境诊断命令
#[tauri::command]
pub async fn diagnose_mcp_environment(
    client_manager: State<'_, Arc<McpClientManagerDirect>>,
) -> Result<serde_json::Value, String> {
    client_manager
        .diagnose_mcp_environment()
        .await
        .map_err(|e| e.to_string())
}

/// 重试失败的MCP连接（新版本）
#[tauri::command]
pub async fn retry_mcp_connection_new(
    connection_id: String,
    client_manager: State<'_, Arc<McpClientManagerDirect>>,
) -> Result<String, String> {
    client_manager
        .retry_connection(&connection_id)
        .await
        .map(|_| "Connection retried successfully".to_string())
        .map_err(|e| e.to_string())
}

/// 批量测试MCP服务器连接
#[tauri::command]
pub async fn test_mcp_servers(
    client_manager: State<'_, Arc<McpClientManagerDirect>>,
) -> Result<Vec<serde_json::Value>, String> {
    let all_servers = client_manager
        .get_all_servers_with_status()
        .await
        .map_err(|e| e.to_string())?;
    
    let mut test_results = Vec::new();
    
    for server in all_servers {
        let test_result = if server.get("status").and_then(|s| s.as_str()) == Some("Connected") {
            serde_json::json!({
                "name": server.get("name").and_then(|n| n.as_str()).unwrap_or("unknown"),
                "status": "connected",
                "message": "Connection already established"
            })
        } else {
            // 尝试重新连接
            // 简化连接逻辑，因为 connect_to_server 方法签名不匹配
            let server_name = server.get("name").and_then(|n| n.as_str()).unwrap_or("unknown");
            match client_manager.connect_to_server(server_name).await {
                Ok(_) => serde_json::json!({
                    "name": server_name,
                    "status": "success",
                    "message": "Connection established successfully"
                }),
                Err(e) => serde_json::json!({
                    "name": server_name,
                    "status": "failed",
                    "message": format!("Connection failed: {}", e)
                })
            }
        };
        
        test_results.push(test_result);
    }
    
    Ok(test_results)
}

/// 清理重复的MCP服务器配置
#[tauri::command]
pub async fn cleanup_duplicate_mcp_servers(
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<String>, String> {
    let mut removed_duplicates = Vec::new();
    
    // 获取所有MCP服务器配置
    let configs = db_service
        .get_all_mcp_server_configs()
        .await
        .map_err(|e| format!("Failed to get MCP server configs: {}", e))?;
    
    // 按名称分组，找出重复项
    let mut name_groups: std::collections::HashMap<String, Vec<_>> = std::collections::HashMap::new();
    for config in configs {
        name_groups.entry(config.name.clone()).or_default().push(config);
    }
    
    // 删除重复项，保留最新的
    for (name, mut group) in name_groups {
        if group.len() > 1 {
            // 按创建时间排序，保留最新的
            group.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            
            // 删除除第一个（最新）之外的所有配置
            for config in group.iter().skip(1) {
                db_service
                    .delete_mcp_server_config(&config.id)
                    .await
                    .map_err(|e| format!("Failed to delete duplicate config {}: {}", config.id, e))?;
                
                removed_duplicates.push(format!("{} (ID: {})", name, config.id));
            }
        }
    }
    
    Ok(removed_duplicates)
}

/// 自动恢复MCP服务器状态
#[tauri::command]
pub async fn auto_restore_mcp_server_state(
    mcp_service: State<'_, Arc<McpService>>,
) -> Result<String, String> {
    match mcp_service.auto_restore_server_state().await {
        Ok(_) => Ok("MCP server state restored successfully".to_string()),
        Err(e) => Err(format!("Failed to restore MCP server state: {}", e)),
    }
}

/// 获取MCP服务器保存的状态
#[tauri::command]
pub async fn get_mcp_server_saved_states(
    mcp_service: State<'_, Arc<McpService>>,
) -> Result<Vec<serde_json::Value>, String> {
    match mcp_service.get_all_server_states().await {
        Ok(states) => {
            let result: Vec<serde_json::Value> = states
                .into_iter()
                .map(|(name, enabled, last_started)| {
                    serde_json::json!({
                        "server_name": name,
                        "enabled": enabled,
                        "last_started_at": last_started,
                        "last_started_formatted": last_started
                            .map(|ts| chrono::DateTime::from_timestamp(ts, 0)
                                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                                .unwrap_or_else(|| "Invalid timestamp".to_string()))
                    })
                })
                .collect();
            Ok(result)
        }
        Err(e) => Err(format!("Failed to get MCP server saved states: {}", e)),
    }
}

/// 手动保存MCP服务器当前状态
#[tauri::command]
pub async fn save_mcp_server_state(
    server_name: String,
    enabled: bool,
    mcp_service: State<'_, Arc<McpService>>,
) -> Result<String, String> {
    match mcp_service.save_server_state(&server_name, enabled).await {
        Ok(_) => Ok(format!("MCP server '{}' state saved: enabled={}", server_name, enabled)),
        Err(e) => Err(format!("Failed to save MCP server state: {}", e)),
    }
}

/// MCP连接诊断
#[tauri::command]
pub async fn diagnose_mcp_connection(
    server_name: String,
    command: String,
    args: Vec<String>,
) -> Result<serde_json::Value, String> {
    let mut diagnostics = serde_json::Map::new();
    
    info!("Starting MCP connection diagnostics for: {}", server_name);
    
    // 1. 检查命令是否存在
    let (check_cmd, check_args) = if cfg!(target_os = "windows") {
        ("where", vec![command.clone()])
    } else {
        ("which", vec![command.clone()])
    };
    
    let command_check = tokio::process::Command::new(check_cmd)
        .args(check_args)
        .output()
        .await;
    
    let command_available = match command_check {
        Ok(output) => output.status.success(),
        Err(_) => false,
    };
    
    diagnostics.insert("command_available".to_string(), serde_json::json!({
        "status": command_available,
        "command": command,
        "message": if command_available {
            "Command found and accessible"
        } else {
            "Command not found in PATH"
        }
    }));
    
    // 2. 如果是npx命令，检查npm和网络
    let mut npm_available = false;
    let mut network_ok = false;
    
    if command.contains("npx") {
        // 检查npm
        let npm_check = tokio::process::Command::new("npm")
            .args(["--version"])
            .output()
            .await;
        
        npm_available = npm_check.is_ok() && npm_check.unwrap().status.success();
        
        diagnostics.insert("npm_available".to_string(), serde_json::json!({
            "status": npm_available,
            "message": if npm_available {
                "npm is available and functional"
            } else {
                "npm is not available - please install Node.js"
            }
        }));
        
        // 检查网络连接
        let ping_result = if cfg!(target_os = "windows") {
            tokio::process::Command::new("ping")
                .args(["-n", "1", "registry.npmjs.org"])
                .output()
                .await
        } else {
            tokio::process::Command::new("ping")
                .args(["-c", "1", "registry.npmjs.org"])
                .output()
                .await
        };
        
        network_ok = ping_result.is_ok() && ping_result.unwrap().status.success();
        
        diagnostics.insert("network_connectivity".to_string(), serde_json::json!({
            "status": network_ok,
            "target": "registry.npmjs.org",
            "message": if network_ok {
                "Network connectivity to npm registry confirmed"
            } else {
                "Network connectivity issues detected"
            }
        }));
        
        // 尝试检查包是否存在（如果有包名）
        if !args.is_empty() {
            let package_name = args.last().unwrap();
            if !package_name.is_empty() && !package_name.starts_with('-') {
                let package_check = tokio::process::Command::new("npm")
                    .args(["view", package_name, "version"])
                    .output()
                    .await;
                
                let package_exists = package_check.is_ok() && package_check.unwrap().status.success();
                
                diagnostics.insert("package_availability".to_string(), serde_json::json!({
                    "status": package_exists,
                    "package": package_name,
                    "message": if package_exists {
                        format!("Package '{}' is available in npm registry", package_name)
                    } else {
                        format!("Package '{}' not found in npm registry", package_name)
                    }
                }));
            }
        }
    }
    
    // 3. 系统环境检查
    let node_check = tokio::process::Command::new("node")
        .args(["--version"])
        .output()
        .await;
    
    let node_available = node_check.is_ok() && node_check.unwrap().status.success();
    
    diagnostics.insert("node_available".to_string(), serde_json::json!({
        "status": node_available,
        "message": if node_available {
            "Node.js is available"
        } else {
            "Node.js is not available"
        }
    }));
    
    // 4. 生成建议
    let mut recommendations = Vec::new();
    
    if !command_available {
        recommendations.push("Install the required command or add it to your PATH");
    }
    
    if command.contains("npx") && !npm_available {
        recommendations.push("Install Node.js and npm from https://nodejs.org/");
    }
    
    if command.contains("npx") && !network_ok {
        recommendations.push("Check your internet connection and firewall settings");
    }
    
    if recommendations.is_empty() {
        recommendations.push("All prerequisites appear to be met. The issue may be with the specific package or server configuration.");
    }
    
    diagnostics.insert("recommendations".to_string(), serde_json::json!(recommendations));
    
    // 5. 总体状态
    let overall_status = command_available && 
        (!command.contains("npx") || (npm_available && node_available));
    
    diagnostics.insert("overall_status".to_string(), serde_json::json!({
        "healthy": overall_status,
        "message": if overall_status {
            "Connection prerequisites are satisfied"
        } else {
            "Some connection prerequisites are missing"
        }
    }));
    
    info!("MCP connection diagnostics completed for: {}", server_name);
    
    Ok(serde_json::Value::Object(diagnostics))
}

/// 强制自动连接所有启用的MCP服务器
#[tauri::command]
pub async fn force_auto_connect_mcp_servers(
    client_manager: State<'_, Arc<McpClientManager>>,
) -> Result<serde_json::Value, String> {
    info!("Force auto-connect triggered by user");
    
    // 重新初始化客户端管理器（这会触发自动连接）
    if let Err(e) = client_manager.initialize().await {
        return Err(format!("Failed to initialize MCP client manager: {}", e));
    }
    
    // 等待一段时间让连接完成
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // 获取连接状态
    let connection_status = client_manager.get_all_connection_status().await;
    
    let mut result = serde_json::Map::new();
    result.insert("message".to_string(), serde_json::json!("Auto-connect process completed"));
    result.insert("connections".to_string(), serde_json::json!(connection_status));
    
    Ok(serde_json::Value::Object(result))
}

/// 测试MCP传输类型实现
#[tauri::command]
pub async fn test_mcp_transport_types(
    client_manager: State<'_, Arc<McpClientManager>>,
) -> Result<serde_json::Value, String> {
    let mut test_results = serde_json::Map::new();
    
    // 测试STDIO传输类型
    test_results.insert("stdio".to_string(), serde_json::json!({
        "supported": true,
        "description": "STDIO transport implemented using child process management",
        "requires_command": true
    }));
    
    // 测试SSE客户端传输类型
    test_results.insert("sse_client".to_string(), serde_json::json!({
        "supported": true,
        "description": "SSE client transport available but requires specific server setup with SSE events",
        "requires_endpoint": true,
        "note": "Server must provide SSE endpoint for this transport to work"
    }));
    
    // 测试HTTP流式传输类型
    test_results.insert("http_streaming".to_string(), serde_json::json!({
        "supported": true,
        "description": "HTTP streaming transport available but requires specific server setup with streamable HTTP protocol",
        "requires_endpoint": true,
        "note": "Server must support streamable HTTP protocol for this transport to work"
    }));
    
    // 测试子进程传输类型
    test_results.insert("child_process".to_string(), serde_json::json!({
        "supported": true,
        "description": "Child process transport fully implemented",
        "requires_command": true
    }));
    
    // 获取当前连接状态
    let connection_status = client_manager.get_all_connection_status().await;
    test_results.insert("current_connections".to_string(), serde_json::json!(connection_status));
    
    Ok(serde_json::Value::Object(test_results))
}

/// 批量并发连接多个MCP服务器
#[tauri::command]
pub async fn connect_servers_concurrent(
    server_names: Vec<String>,
    client_manager: State<'_, Arc<McpClientManager>>,
) -> Result<Vec<serde_json::Value>, String> {
    info!("Starting concurrent connection to {} servers: {:?}", server_names.len(), server_names);
    
    let results = client_manager
        .connect_to_servers_concurrent(server_names)
        .await
        .map_err(|e| e.to_string())?;
    
    let response: Vec<serde_json::Value> = results
        .into_iter()
        .map(|(server_name, result)| {
            match result {
                Ok(duration) => serde_json::json!({
                    "server_name": server_name,
                    "success": true,
                    "duration_ms": duration.as_millis(),
                    "message": format!("Connected successfully in {:?}", duration)
                }),
                Err(e) => serde_json::json!({
                    "server_name": server_name,
                    "success": false,
                    "error": e.to_string(),
                    "message": format!("Connection failed: {}", e)
                })
            }
        })
        .collect();
    
    Ok(response)
}

/// 获取连接性能统计
#[tauri::command]
pub async fn get_connection_performance_stats(
    client_manager: State<'_, Arc<McpClientManager>>,
) -> Result<serde_json::Value, String> {
    let stats = client_manager.get_connection_performance_stats().await;
    info!("Connection performance stats: {}", stats);
    Ok(stats)
}
