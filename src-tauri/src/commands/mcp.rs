use tauri::State;
use tracing::warn;
use crate::services::mcp::McpService;
use crate::mcp::types::{McpToolInfo, ToolExecutionResult, ToolCategory, TransportConfig, McpServerConfig};
use crate::models::mcp::{McpToolInstallRequest, McpStoreItem};
use serde_json::Value;
use uuid::Uuid;
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};

use crate::services::database::DatabaseService;
use crate::services::mcp::McpConnectionInfo;
use crate::mcp::{McpClientManager, McpServerManager, ConnectionStatus};
use crate::mcp::server::ServerConfig;
use std::sync::Arc;
use std::collections::HashMap;
use crate::mcp::client::ConfiguredServer;
use rmcp::model::Tool as RmcpTool;
use chrono::Utc;

/// 获取MCP工具列表
#[tauri::command]
pub async fn get_mcp_tools(state: State<'_, McpService>) -> Result<Vec<McpToolInfo>, String> {
    state.get_available_tools().await.map_err(|e| e.to_string())
}

/// 根据分类获取工具
#[tauri::command]
pub async fn get_mcp_tools_by_category(
    category: String,
    state: State<'_, McpService>
) -> Result<Vec<McpToolInfo>, String> {
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
    
    state.get_tools_by_category(tool_category).await.map_err(|e| e.to_string())
}

/// 搜索工具
#[tauri::command]
pub async fn search_mcp_tools(
    query: String,
    state: State<'_, McpService>
) -> Result<Vec<McpToolInfo>, String> {
    state.search_tools(&query).await.map_err(|e| e.to_string())
}

/// 获取单个工具信息
#[tauri::command]
pub async fn get_mcp_tool(
    tool_id: String,
    state: State<'_, McpService>
) -> Result<Option<McpToolInfo>, String> {
    state.get_tool(&tool_id).await.map_err(|e| e.to_string())
}

/// 执行MCP工具
#[tauri::command]
pub async fn execute_mcp_tool(
    tool_id: String,
    parameters: Value,
    _timeout: Option<u64>,
    mcp_service: State<'_, Mutex<McpService>>
) -> Result<Value, String> {
    let service = mcp_service.lock().await;
    service.execute_tool(&tool_id, parameters).await
        .map_err(|e| e.to_string())
}

/// 获取执行结果
#[tauri::command]
pub async fn get_mcp_execution_result(
    execution_id: String,
    state: State<'_, McpService>
) -> Result<Option<ToolExecutionResult>, String> {
    let uuid = Uuid::parse_str(&execution_id)
        .map_err(|_| "Invalid execution ID".to_string())?;
    
    state.get_execution_result(uuid).await.map_err(|e| e.to_string())
}

/// 获取连接状态
#[tauri::command]
pub async fn get_mcp_connections(state: State<'_, McpService>) -> Result<Vec<McpConnectionInfo>, String> {
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
    state: State<'_, McpService>
) -> Result<String, String> {
    let transport = match transport_type.as_str() {
        "stdio" => TransportConfig::Stdio,
        "websocket" => {
            let url = transport_config.get("url")
                .and_then(|v| v.as_str())
                .ok_or("WebSocket configuration missing URL")?;
            TransportConfig::WebSocket { url: url.to_string() }
        }
        "http" => {
            let base_url = transport_config.get("base_url")
                .and_then(|v| v.as_str())
                .ok_or("HTTP configuration missing base_url")?;
            TransportConfig::Http { base_url: base_url.to_string() }
        }
        _ => return Err("Unsupported transport type".to_string()),
    };

    let config = McpServerConfig {
        name,
        version,
        description,
        transport,
        capabilities: crate::mcp::types::ServerCapabilities {
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
    state: State<'_, McpService>
) -> Result<(), String> {
    state.remove_server(&connection_id).await.map_err(|e| e.to_string())
}

/// 初始化默认工具
#[tauri::command]
pub async fn initialize_default_mcp_tools(
    mcp_service: State<'_, Mutex<McpService>>
) -> Result<(), String> {
    let mut service = mcp_service.lock().await;
    service.initialize_mcp().await
        .map_err(|e| e.to_string())
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
pub async fn add_mcp_tool(
    _tool: Value,
    _state: State<'_, McpService>
) -> Result<(), String> {
    // 兼容性实现
    tracing::warn!("Using deprecated add_mcp_tool command");
    Ok(())
}

#[tauri::command]
pub async fn remove_mcp_tool(
    tool_id: String,
    state: State<'_, McpService>
) -> Result<(), String> {
    // 兼容性实现
    tracing::warn!("Using deprecated remove_mcp_tool command");
    Ok(())
}

#[tauri::command]
pub async fn update_mcp_tool_status(
    tool_id: String,
    status: String,
    state: State<'_, McpService>
) -> Result<(), String> {
    // 兼容性实现
    tracing::warn!("Using deprecated update_mcp_tool_status command");
    Ok(())
}

#[tauri::command]
pub async fn get_mcp_execution_status(
    execution_id: String,
    state: State<'_, McpService>
) -> Result<String, String> {
    let uuid = Uuid::parse_str(&execution_id)
        .map_err(|_| "无效的执行ID".to_string())?;
    
    if let Some(result) = state.get_execution_result(uuid).await.map_err(|e| e.to_string())? {
        Ok(format!("{:?}", result.status))
    } else {
        Err("执行记录不存在".to_string())
    }
}

#[tauri::command]
pub async fn cancel_mcp_execution(
    execution_id: String,
    state: State<'_, McpService>
) -> Result<(), String> {
    // 兼容性实现
    tracing::warn!("Using deprecated cancel_mcp_execution command");
    Ok(())
}

#[tauri::command]
pub async fn get_mcp_executions(
    state: State<'_, McpService>
) -> Result<Vec<String>, String> {
    // 兼容性实现 - 返回空列表
    Ok(Vec::new())
}

/// 检查工具安装状态
#[tauri::command]
pub async fn check_mcp_tool_installation(tool_name: String) -> Result<bool, String> {
    // 简单检查工具是否在系统PATH中可用
    let output = match tool_name.as_str() {
        "subfinder" => tokio::process::Command::new("subfinder").arg("--version").output().await,
        "nuclei" => tokio::process::Command::new("nuclei").arg("--version").output().await,
        "httpx" => tokio::process::Command::new("httpx").arg("--version").output().await,
        "nmap" => tokio::process::Command::new("nmap").arg("--version").output().await,
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
    _mcp_service: State<'_, Mutex<McpService>>
) -> Result<String, String> {
    Ok("tool_id_123".to_string())
}

#[tauri::command]
pub async fn parse_mcp_tool_output(
    output: String,
    tool_id: String,
    state: State<'_, McpService>
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
    older_than_hours: u64,
    state: State<'_, McpService>
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

/// 执行扫描模板
#[tauri::command]
pub async fn execute_scan_template(
    template_name: String, 
    target: String,
    state: State<'_, McpService>
) -> Result<Value, String> {
    let current_target = target.clone();
    
    match template_name.as_str() {
        "web_app_scan" => {
            // 1. 子域名发现
            let subfinder_result = state.execute_tool(
                "subfinder", 
                serde_json::json!({"domain": current_target})
            ).await.map_err(|e| e.to_string())?;
            
            // 2. HTTP探测
            let httpx_result = state.execute_tool(
                "httpx", 
                serde_json::json!({"url": current_target})
            ).await.map_err(|e| e.to_string())?;
            
            // 3. 漏洞扫描
            let nuclei_result = state.execute_tool(
                "nuclei", 
                serde_json::json!({"target": current_target})
            ).await.map_err(|e| e.to_string())?;
            
            Ok(serde_json::json!({
                "template": template_name,
                "target": target,
                "results": {
                    "subfinder": subfinder_result,
                    "httpx": httpx_result,
                    "nuclei": nuclei_result
                },
                "status": "completed"
            }))
        }
        "network_discovery" => {
            // 网络发现扫描
            let nmap_result = state.execute_tool(
                "nmap", 
                serde_json::json!({"target": current_target, "ports": "1-1000"})
            ).await.map_err(|e| e.to_string())?;
            
            Ok(serde_json::json!({
                "template": template_name,
                "target": target,
                "results": {
                    "nmap": nmap_result
                },
                "status": "completed"
            }))
        }
        "subdomain_enum" => {
            // 子域名枚举
            let subfinder_result = state.execute_tool(
                "subfinder", 
                serde_json::json!({"domain": current_target})
            ).await.map_err(|e| e.to_string())?;
            
            Ok(serde_json::json!({
                "template": template_name,
                "target": target,
                "results": {
                    "subfinder": subfinder_result
                },
                "status": "completed"
            }))
        }
        _ => Err(format!("未知的扫描模板: {}", template_name))
    }
}

/// 获取默认安全工具列表
#[tauri::command]
pub async fn get_default_security_tools() -> Result<Vec<Value>, String> {
    let tools = vec![
        serde_json::json!({
            "name": "subfinder",
            "description": "快速被动子域名发现工具",
            "category": "reconnaissance",
            "version": "2.6.3",
            "install_url": "https://github.com/projectdiscovery/subfinder"
        }),
        serde_json::json!({
            "name": "nuclei",
            "description": "基于模板的快速漏洞扫描器",
            "category": "scanning",
            "version": "3.0.0",
            "install_url": "https://github.com/projectdiscovery/nuclei"
        }),
        serde_json::json!({
            "name": "httpx",
            "description": "快速多用途HTTP工具包",
            "category": "reconnaissance",
            "version": "1.3.7",
            "install_url": "https://github.com/projectdiscovery/httpx"
        }),
        serde_json::json!({
            "name": "nmap",
            "description": "网络发现和安全审计工具",
            "category": "scanning",
            "version": "7.94",
            "install_url": "https://nmap.org/"
        })
    ];
    
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
    Ok(server_manager.list_tools().await)
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

#[tauri::command]
pub async fn mcp_get_connections(
    client_manager: tauri::State<'_, Arc<McpClientManager>>,
) -> Result<Vec<FrontendMcpConnection>, String> {
    let client_manager = client_manager.inner();
    let connections_status = client_manager.get_all_connections_status().await;
    let db_configs = client_manager.get_all_server_configs_from_db().await.map_err(|e| e.to_string())?;

    let mut frontend_connections = Vec::new();

    for config in db_configs {
        if let Some(status) = connections_status.get(&config.name) {
            frontend_connections.push(FrontendMcpConnection {
                db_id: config.id.to_string(),
                id: status.id.clone(),
                name: config.name.clone(),
                description: config.description.clone(),
                transport_type: status.transport_type.to_string(),
                endpoint: status.endpoint.clone(),
                status: status.status.to_string(),
                command: config.command.clone(),
                args: serde_json::from_str(&config.args).unwrap_or_else(|_| vec![]),
            });
        } else {
            frontend_connections.push(FrontendMcpConnection {
                db_id: config.id.to_string(),
                id: None,
                name: config.name.clone(),
                description: config.description.clone(),
                transport_type: "stdio".to_string(),
                endpoint: config.command.clone(),
                status: "Disconnected".to_string(),
                command: config.command.clone(),
                args: serde_json::from_str(&config.args).unwrap_or_else(|_| vec![]),
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
    command: String,
    args: Vec<String>,
) -> Result<String, String> {
    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    
    match client_manager.connect_to_server(&name, &command, args_refs).await {
        Ok(connection_id) => Ok(connection_id),
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
    let connections = client.read().await.get_connections().await;

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
    let client_arc = client_manager.get_client();
    let client = client_arc.read().await;
    match params.get("type").and_then(Value::as_str).ok_or_else(|| "Missing type parameter".to_string())? {
        "npx" => {
            let package = params
                .get("package")
                .and_then(Value::as_str)
                .ok_or_else(|| "Missing package parameter".to_string())?;
            match client.connect_to_npx_server(package).await {
                Ok(id) => Ok(id),
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
            match client.connect_to_http_server(name.to_string(), url).await {
                Ok(id) => Ok(id),
                Err(e) => Err(e.to_string()),
            }
        }
        "child_process" => {
            let name = params
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| "Missing name parameter".to_string())?;
            let command = params
                .get("command")
                .and_then(Value::as_str)
                .ok_or_else(|| "Missing command parameter".to_string())?;
            let args: Vec<&str> = params
                .get("args")
                .and_then(Value::as_array)
                .map(|arr| arr.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            match client.connect_to_child_process(name.to_string(), command, args).await {
                Ok(id) => Ok(id),
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
    let client_arc = client_manager.get_client();
    let client = client_arc.read().await;
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
    server_manager: State<'_, Arc<McpServerManager>>,
    tool_id: String,
) -> Result<(), String> {
    // TODO: 实现工具启动逻辑
    Ok(())
}

/// 停止工具
#[tauri::command]
pub async fn mcp_stop_tool(
    server_manager: State<'_, Arc<McpServerManager>>,
    tool_id: String,
) -> Result<(), String> {
    // TODO: 实现工具停止逻辑
    Ok(())
}

/// 重启工具
#[tauri::command]
pub async fn mcp_restart_tool(
    server_manager: State<'_, Arc<McpServerManager>>,
    tool_id: String,
) -> Result<(), String> {
    // TODO: 实现工具重启逻辑
    Ok(())
}

/// 卸载工具
#[tauri::command]
pub async fn mcp_uninstall_tool(
    server_manager: State<'_, Arc<McpServerManager>>,
    tool_id: String,
) -> Result<(), String> {
    // TODO: 实现工具卸载逻辑
    Ok(())
}

/// 从商店安装工具
#[tauri::command]
pub async fn mcp_install_tool(
    server_manager: State<'_, Arc<McpServerManager>>,
    tool_name: String,
    tool_version: String,
    tool_source: String,
) -> Result<String, String> {
    // TODO: 实现工具安装逻辑
    let tool_id = uuid::Uuid::new_v4().to_string();
    Ok(tool_id)
}

/// 从URL安装工具
#[tauri::command]
pub async fn mcp_install_tool_from_url(
    server_manager: State<'_, Arc<McpServerManager>>,
    url: String,
) -> Result<String, String> {
    // TODO: 实现从URL安装工具逻辑
    let tool_id = uuid::Uuid::new_v4().to_string();
    Ok(tool_id)
}

/// 从GitHub安装工具
#[tauri::command]
pub async fn mcp_install_tool_from_github(
    server_manager: State<'_, Arc<McpServerManager>>,
    url: String,
) -> Result<String, String> {
    // TODO: 实现从GitHub安装工具逻辑
    let tool_id = uuid::Uuid::new_v4().to_string();
    Ok(tool_id)
}

/// 从注册表安装工具
#[tauri::command]
pub async fn mcp_install_tool_from_registry(
    server_manager: State<'_, Arc<McpServerManager>>,
    name: String,
) -> Result<String, String> {
    // TODO: 实现从注册表安装工具逻辑
    let tool_id = uuid::Uuid::new_v4().to_string();
    Ok(tool_id)
}

/// 创建自定义工具
#[tauri::command]
pub async fn mcp_create_custom_tool(
    server_manager: State<'_, Arc<McpServerManager>>,
    name: String,
    version: String,
    description: String,
    command: String,
    config: serde_json::Value,
) -> Result<String, String> {
    // TODO: 实现创建自定义工具逻辑
    let tool_id = uuid::Uuid::new_v4().to_string();
    Ok(tool_id)
}

#[tauri::command]
pub async fn list_tools(state: State<'_, McpService>) -> Result<Vec<McpToolInfo>, String> {
    state.get_available_tools().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn execute_tool(
    tool_id: String,
    parameters: Value,
    state: State<'_, McpService>,
) -> Result<Value, String> {
    state.execute_tool(&tool_id, parameters).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_tool_info(
    tool_id: String,
    state: State<'_, McpService>,
) -> Result<Option<McpToolInfo>, String> {
    state.get_tool(&tool_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_connection_info(state: State<'_, McpService>) -> Result<Vec<McpConnectionInfo>, String> {
    state.get_connection_info().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_execution_result(
    execution_id: String,
    state: State<'_, McpService>,
) -> Result<Option<ToolExecutionResult>, String> {
    let uuid = Uuid::parse_str(&execution_id).map_err(|e| e.to_string())?;
    state.get_execution_result(uuid).await.map_err(|e| e.to_string())
}

/// 添加并连接到子进程 MCP 服务器
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
    
    let check_result = tokio::process::Command::new(check_cmd)
        .args(check_args)
        .output()
        .await;
    
    if let Err(e) = check_result {
        return Err(format!("Failed to check if command exists: {}", e));
    }
    
    let check_output = check_result.unwrap();
    if !check_output.status.success() {
        return Err(format!("Command '{}' not found: please ensure the command exists in the system PATH", command));
    }
    
    // 使用connect_with_command方法，允许更灵活地配置命令
    client_manager
        .connect_with_command(&name, &command, args)
        .await
        .map_err(|e| format!("Failed to connect to server: {}", e))
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
    client_manager.update_server_config(payload).await.map_err(|e| e.to_string())
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
    let client_guard = client.read().await;
    let tools = client_guard
        .get_connection_tools(&connection_id)
        .await
        .map_err(|e| e.to_string())?;
    
    let frontend_tools = tools.into_iter().map(FrontendTool::from).collect();
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
                return Err("For sse or http types, the parameter field must be a valid URL.".to_string());
            }
            
            client_manager
                .connect_to_http_server(&config.name, url)
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

    let config: McpServersConfig = serde_json::from_str(&json_without_comments)
        .map_err(|e| format!("Failed to parse JSON: {}. Please ensure the format is correct.", e))?;

    for (name, server_value) in config.mcp_servers {
        if let Some(obj) = server_value.as_object() {
            if obj.contains_key("command") {
                let command = obj.get("command").and_then(Value::as_str).unwrap_or("").to_string();
                if command.is_empty() { continue; }
                let args: Vec<String> = obj.get("args")
                    .and_then(Value::as_array)
                    .map(|arr| arr.iter().filter_map(Value::as_str).map(String::from).collect())
                    .unwrap_or_default();
                
                let mut final_command = command;
                if cfg!(target_os = "windows") && (final_command == "npx" || final_command == "npm") {
                    final_command.push_str(".cmd");
                }
                
                if let Err(e) = client_manager
                    .connect_with_command(&name, &final_command, args)
                    .await {
                    warn!("Failed to import stdio server '{}' from JSON: {}", name, e);
                }
            } 
            else if obj.contains_key("url") {
                let url = obj.get("url").and_then(Value::as_str).unwrap_or("");
                if !url.is_empty() {
                    if let Err(e) = client_manager
                        .connect_to_http_server(&name, url)
                        .await {
                        warn!("Failed to import sse/http server '{}' from JSON: {}", name, e);
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