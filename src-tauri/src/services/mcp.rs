use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json::Value;

use crate::mcp::{
    McpServerManager, McpClientManager, ConnectionStatus,
    McpTool, ToolDefinition
};

// 工具信息类型已移动到 types 模块

/// MCP 连接信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConnectionInfo {
    pub name: String,
    pub transport_type: String,
    pub endpoint: String,
    pub status: String,
    pub tools_count: usize,
    pub last_activity: Option<String>,
}

/// MCP 服务 - 集成服务器和客户端管理
#[derive(Debug)]
pub struct McpService {
    server_manager: Arc<McpServerManager>,
    client_manager: Arc<McpClientManager>,
    is_running: Arc<RwLock<bool>>,
}

impl McpService {
    pub fn new(client_manager: Arc<McpClientManager>) -> Self {
        Self {
            server_manager: Arc::new(McpServerManager::new()),
            client_manager,
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// 启动 MCP 服务器
    pub async fn start_server(&self, transport: &str, endpoint: Option<&str>) -> Result<()> {
        let mut running = self.is_running.write().await;
        if *running {
            return Err(anyhow::anyhow!("MCP server is already running"));
        }
        
        match transport {
            "stdio" => {
                // 在后台启动 STDIO 服务器
                let manager = self.server_manager.clone();
                let running_flag = self.is_running.clone();
                
                tokio::spawn(async move {
                    *running_flag.write().await = true;
                    
                    if let Err(e) = manager.start_stdio().await {
                        eprintln!("STDIO MCP server failed to start: {}", e);
                    }
                    
                    *running_flag.write().await = false;
                });
            }
            "child_process" => {
                // 启动子进程服务器
                if let Some(endpoint) = endpoint {
                    let parts: Vec<&str> = endpoint.split_whitespace().collect();
                    if !parts.is_empty() {
                        let command = parts[0].to_string();
                        let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();
                        
                        let manager = self.server_manager.clone();
                        tokio::spawn(async move {
                            if let Err(e) = manager.start_child_process(&command, &args.iter().map(|s| s.as_str()).collect::<Vec<_>>()).await {
                                eprintln!("Child process MCP server failed to start: {}", e);
                            }
                        });
                    }
                }
                *running = true;
            }
            _ => {
                *running = true;
                tracing::info!("MCP server started (mode: {})", transport);
            }
        }
        
        Ok(())
    }

    /// 停止 MCP 服务器
    pub async fn stop_server(&self) -> Result<()> {
        let mut running = self.is_running.write().await;
        *running = false;
        tracing::info!("MCP server stopped");
        Ok(())
    }

    /// 检查服务器状态
    pub async fn is_server_running(&self) -> bool {
        *self.is_running.read().await
    }

    /// 获取所有可用工具（包括内置和外部连接的工具）
    pub async fn get_available_tools(&self) -> Result<Vec<crate::mcp::types::McpToolInfo>> {
        let mut tool_infos = Vec::new();
        
        // 获取内置工具
        let server_arc = self.server_manager.get_server().await;
        let server_guard = server_arc.read().await;
        let internal_tools = server_guard.list_tools().await;
        let registry_arc = server_guard.get_tool_registry();
        let reg = registry_arc.read().await;
        
        for tool_name in internal_tools {
            if let Some(tool) = reg.get_tool(&tool_name).ok() {
                tool_infos.push(crate::mcp::types::McpToolInfo {
                    id: tool_name,
                    name: tool.name,
                    description: tool.description,
                    version: "1.0.0".to_string(),
                    category: crate::mcp::ToolCategory::Utility,
                    parameters: crate::mcp::types::ToolParameters {
                        schema: tool.input_schema,
                        required: vec![],
                        optional: vec![],
                    },
                    metadata: crate::mcp::types::ToolMetadata {
                        author: "Sentinel AI Internal".to_string(),
                        license: "MIT".to_string(),
                        homepage: None,
                        repository: None,
                        tags: vec!["security".to_string()],
                        install_command: None,
                        requirements: vec![],
                    },
                });
            }
        }
        
        // 获取外部连接的工具
        let client_arc = self.client_manager.get_client();
        let client = client_arc.read().await;
        let connections = client.get_connections().await;
        for connection in connections {
            for tool in connection.tools {
                tool_infos.push(crate::mcp::types::McpToolInfo {
                    id: tool.name.to_string(),
                    name: tool.name.to_string(),
                    description: tool.description.unwrap_or_default().to_string(),
                    version: "1.0.0".to_string(),
                    category: crate::mcp::ToolCategory::Utility,
                    parameters: crate::mcp::types::ToolParameters {
                        schema: serde_json::Value::Object((*tool.input_schema).clone()),
                        required: vec![],
                        optional: vec![],
                    },
                    metadata: crate::mcp::types::ToolMetadata {
                        author: format!("External ({})", connection.name),
                        license: "Unknown".to_string(),
                        homepage: None,
                        repository: None,
                        tags: vec!["external".to_string()],
                        install_command: None,
                        requirements: vec![],
                    },
                });
            }
        }
        
        Ok(tool_infos)
    }

    /// 执行工具（优先使用内置工具，然后尝试外部工具）
    pub async fn execute_tool(&self, tool_name: &str, parameters: Value) -> Result<Value> {
        // 首先尝试执行内置工具
        let server = self.server_manager.get_server().await;
        match server.write().await.execute_tool(tool_name, parameters.clone()).await {
            Ok(result) => return Ok(result),
            Err(_) => {
                // 内置工具不存在或执行失败，尝试外部工具
                tracing::info!("Built-in tool '{}' not available, trying external tools", tool_name);
            }
        }

        // 尝试在任何已连接的客户端上执行工具
        let client = self.client_manager.get_client();
        let client = client.read().await;

        match client.execute_tool_on_any(tool_name, parameters).await {
            Ok(result) => Ok(result),
            Err(e) => Err(anyhow::anyhow!("Tool '{}' is not available or failed to execute on built-in and external servers: {}", tool_name, e)),
        }
    }

    /// 动态注册新工具
    pub async fn register_tool(&self, tool: Box<dyn McpTool>) -> Result<()> {
        self.server_manager.register_tool(tool).await
    }

    /// 获取工具定义
    pub async fn get_tool_definition(&self, tool_name: &str) -> Result<Option<ToolDefinition>> {
        let server = self.server_manager.get_server().await;
        let registry = server.read().await.get_tool_registry();
        let reg = registry.read().await;
        Ok(reg.get_tool(tool_name).ok())
    }

    /// 连接到外部MCP服务器
    pub async fn connect_to_external_server(&self, package: &str) -> Result<String> {
        let client_arc = self.client_manager.get_client();
        let client = client_arc.read().await;
        client.connect_to_npx_server(package).await
    }

    /// 连接到子进程MCP服务器
    pub async fn connect_to_process(&self, name: String, command: &str, args: Vec<&str>) -> Result<String> {
        let client_arc = self.client_manager.get_client();
        let client = client_arc.read().await;
        client.connect_to_child_process(name, command, args).await
    }

    /// 断开外部连接
    pub async fn disconnect_external(&self, connection_id: &str) -> Result<()> {
        let client_arc = self.client_manager.get_client();
        let client = client_arc.read().await;
        client.disconnect(connection_id).await
    }

    /// 获取连接状态信息
    pub async fn get_connection_info(&self) -> Result<Vec<McpConnectionInfo>> {
        let mut connections = Vec::new();
        
        // 内置服务器状态
        let is_running = self.is_server_running().await;
        let tools_count = self.server_manager.get_server().await.read().await.list_tools().await.len();
        
        connections.push(McpConnectionInfo {
            name: "Built-in security tools server".to_string(),
            transport_type: "internal".to_string(),
            endpoint: "localhost".to_string(),
            status: if is_running { "connected".to_string() } else { "disconnected".to_string() },
            tools_count,
            last_activity: if is_running { 
                Some(chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string()) 
            } else { 
                None 
            },
        });
        
        // 外部连接状态
        let client_arc = self.client_manager.get_client();
        let client = client_arc.read().await;
        let external_connections = client.get_connections().await;
        for conn in external_connections {
            connections.push(McpConnectionInfo {
                name: conn.name,
                transport_type: conn.transport_type,
                endpoint: conn.endpoint,
                status: match conn.status {
                    crate::mcp::client::ConnectionStatus::Connected => "connected".to_string(),
                    crate::mcp::client::ConnectionStatus::Connecting => "connecting".to_string(),
                    crate::mcp::client::ConnectionStatus::Disconnected => "disconnected".to_string(),
                    crate::mcp::client::ConnectionStatus::Error(e) => format!("error: {}", e),
                },
                tools_count: conn.tools.len(),
                last_activity: Some(chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string()),
            });
        }
        
        Ok(connections)
    }

    /// 测试工具连接
    pub async fn test_tool(&self, tool_name: &str) -> Result<bool> {
        let registry = self.server_manager.get_server().await.read().await.get_tool_registry();
        let reg = registry.read().await;
        
        if reg.get_tool(tool_name).is_ok() {
            return Ok(true);
        }
        
        // 检查外部工具
        let client_arc = self.client_manager.get_client();
        let connections = client_arc.read().await.get_connections().await;
        for conn in connections {
            if matches!(conn.status, crate::mcp::client::ConnectionStatus::Connected) {
                for tool in &conn.tools {
                    if tool.name == tool_name {
                        return Ok(true);
                    }
                }
            }
        }
        
        Ok(false)
    }

    /// 获取工具使用统计
    pub async fn get_tool_stats(&self) -> Result<Value> {
        let tools = self.get_available_tools().await?;
        let connections = self.get_connection_info().await?;
        
        let internal_tools = tools.iter().filter(|t| t.metadata.author.contains("Internal")).count();
        let external_tools = tools.iter().filter(|t| t.metadata.author.contains("External")).count();
        let connected_servers = connections.iter().filter(|c| c.status == "connected").count();
        
        Ok(serde_json::json!({
            "total_tools": tools.len(),
            "internal_tools": internal_tools,
            "external_tools": external_tools,
            "enabled_tools": tools.len(), // 所有工具都视为启用
            "connected_servers": connected_servers,
            "total_servers": connections.len(),
            "categories": {
                "Security": internal_tools,
                "External": external_tools
            },
            "last_updated": chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string()
        }))
    }

    /// 根据分类获取工具
    pub async fn get_tools_by_category(&self, category: crate::mcp::ToolCategory) -> Result<Vec<crate::mcp::types::McpToolInfo>> {
        let all_tools = self.get_available_tools().await?;
        Ok(all_tools.into_iter().filter(|t| t.category == category).collect())
    }

    /// 搜索工具
    pub async fn search_tools(&self, query: &str) -> Result<Vec<crate::mcp::types::McpToolInfo>> {
        let all_tools = self.get_available_tools().await?;
        Ok(all_tools.into_iter().filter(|t| t.name.contains(query) || t.description.contains(query)).collect())
    }

    /// 获取单个工具
    pub async fn get_tool(&self, tool_id: &str) -> Result<Option<crate::mcp::types::McpToolInfo>> {
        let all_tools = self.get_available_tools().await?;
        Ok(all_tools.into_iter().find(|t| t.id == tool_id))
    }

    /// 获取执行结果
    pub async fn get_execution_result(&self, execution_id: uuid::Uuid) -> Result<Option<crate::mcp::ToolExecutionResult>> {
        let server = self.server_manager.get_server().await;
        let server_guard = server.read().await;
        Ok(server_guard.get_execution_result(&execution_id))
    }

    /// 添加服务器
    pub async fn add_server(&self, config: crate::mcp::McpServerConfig) -> Result<String> {
        self.server_manager.add_server(config.into()).await
    }

    /// 移除服务器
    pub async fn remove_server(&self, connection_id: &str) -> Result<()> {
        self.server_manager.remove_server(connection_id).await
    }

    /// 初始化MCP服务（连接到常用的外部服务器）
    pub async fn initialize_mcp(&self) -> Result<()> {
        self.client_manager.initialize().await
    }
} 