use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;

use crate::tools::{McpClientManager, McpServerManager, ConnectionStatus, McpSession};
use crate::tools::protocol::ToolDefinition;
use crate::services::database::DatabaseService;

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
#[derive(Debug, Clone)]
pub struct McpService {
    server_manager: Arc<McpServerManager>,
    client_manager: Arc<McpClientManager>,
    is_running: Arc<RwLock<bool>>,
    db_service: Arc<DatabaseService>,
}

impl McpService {
    pub fn new(client_manager: Arc<McpClientManager>, db_service: Arc<DatabaseService>) -> Self {
        Self {
            server_manager: Arc::new(McpServerManager::new()),
            client_manager,
            is_running: Arc::new(RwLock::new(false)),
            db_service,
        }
    }

    /// 获取客户端管理器的只读访问
    pub fn get_client_manager(&self) -> &Arc<McpClientManager> {
        &self.client_manager
    }

    pub fn with_server_manager(
        client_manager: Arc<McpClientManager>, 
        server_manager: Arc<McpServerManager>,
        db_service: Arc<DatabaseService>
    ) -> Self {
        Self {
            server_manager,
            client_manager,
            is_running: Arc::new(RwLock::new(false)),
            db_service,
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
                            if let Err(e) = manager
                                .start_child_process(
                                    &command,
                                    &args.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                                )
                                .await
                            {
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
    pub async fn get_available_tools(&self) -> Result<Vec<crate::tools::ToolInfo>> {
        tracing::info!("[MCP] get_available_tools called, self ptr: {:p}", self);
        let mut tool_infos = Vec::new();

        // 获取内置工具
        let server_arc = self.server_manager.get_server().await;
        let server_guard = server_arc.read().await;
        tracing::debug!("[MCP] server_guard ptr: {:p}", &*server_guard);
        let internal_tools = server_guard.list_tools().await;
        tracing::debug!("[MCP] internal_tools: {:?}", internal_tools);
        let registry_arc = server_guard.get_tool_registry();
        let reg = registry_arc.read().await;
        tracing::debug!("[MCP] registry ptr: {:p}, tool count: {}", &*reg, reg.len());

        for tool_name in internal_tools.iter() {
            if let Some(tool) = reg.get(tool_name) {
                tracing::debug!("[MCP] found tool in registry: {}", tool_name);
                tool_infos.push(crate::tools::ToolInfo {
                        id: tool_name.clone(),
                        name: tool.name.clone(),
                        description: tool.description.clone(),
                        version: "1.0.0".to_string(),
                        category: tool.category.clone(),
                        parameters: Self::parse_tool_parameters(&tool.parameters.schema),
                        metadata: tool.metadata.clone(),
                        available: true,
                        installed: true,
                        source: crate::tools::ToolSource::Builtin, // 保持内置工具标记为Builtin
                    });
            } else {
                tracing::warn!("[MCP] tool not found in registry: {}", tool_name);
            }
        }

        // 获取客户端连接的工具（主要工具来源）
        tracing::info!("[MCP] Checking client connections for tools");
        
        // 从客户端管理器获取所有连接的MCP服务器的工具
        let status_map = self.client_manager.get_all_connection_status().await;
        
        // 遍历所有连接的MCP客户端并获取工具
        for (server_name, status) in status_map {
            match status {
                ConnectionStatus::Connected => {
                    if let Some(session) = self.client_manager.get_session(&server_name).await {
                        // 获取缓存的工具
                        match session.list_tools_paginated(None).await {
                            Ok(tools_result) => {
                                tracing::info!("[MCP] Found {} tools from connected server: {}", tools_result.tools.len(), server_name);
                                
                                for rmcp_tool in tools_result.tools.iter() {
                                    // 转换为内部工具格式
                                    let tool_info = crate::tools::ToolInfo {
                                        id: rmcp_tool.name.to_string(),
                                        name: rmcp_tool.name.to_string(),
                                        description: rmcp_tool.description.as_ref().map(|d| d.to_string()).unwrap_or_else(|| "No description".to_string()),
                                        version: "1.0.0".to_string(),
                                        category: crate::tools::ToolCategory::Custom("external".to_string()),
                                        parameters: {
                                            let schema_map = rmcp_tool.input_schema.as_ref().clone();
                                            Self::parse_rmcp_tool_parameters(&serde_json::Value::Object(schema_map))
                                        },
                                        metadata: crate::tools::ToolMetadata {
                                            author: format!("MCP Server: {}", server_name),
                                            version: "1.0.0".to_string(),
                                            license: "Unknown".to_string(),
                                            homepage: None,
                                            repository: None,
                                            tags: vec!["mcp".to_string(), format!("connection:{}", server_name)],
                                            install_command: None,
                                            requirements: vec![],
                                        },
                                        available: true,
                                        installed: true,
                                        source: crate::tools::ToolSource::External,
                                    };
                                    tool_infos.push(tool_info);
                                }
                            }
                            Err(e) => {
                                tracing::warn!("[MCP] Failed to get tools from server {}: {}", server_name, e);
                            }
                        }
                    }
                }
                _ => {
                    tracing::debug!("[MCP] Skipping disconnected server: {}", server_name);
                }
            }
        }

        Ok(tool_infos)
    }

    pub async fn execute_tool(&self, tool_name: &str, parameters: Value) -> Result<Value> {
        // 移除不存在的 create_dynamic_adapter 导入
        tracing::info!("[MCP] Executing tool '{}' with parameters: {:?}", tool_name, parameters);

        // 真实执行：委托到内置服务器执行器
        let server = self.server_manager.get_server().await;
        let output = server.read().await.execute_tool(tool_name, parameters).await?;
        Ok(output)
    }

    /// 执行客户端连接的MCP工具（按连接名定向调用）
    pub async fn execute_client_tool(&self, connection_name: &str, tool_name: &str, parameters: Value) -> Result<Value> {
        use rmcp::model::{CallToolRequestParam, Content};
        tracing::info!("[MCP] Executing client tool '{}' on connection '{}' with params: {:?}", tool_name, connection_name, parameters);

        // 获取会话
        if let Some(session) = self.client_manager.get_session(connection_name).await {
            // 构造请求
            let args_map: serde_json::Map<String, Value> = parameters.as_object().cloned().unwrap_or_default();
            let req = CallToolRequestParam { name: tool_name.to_string().into(), arguments: Some(args_map) };

            // 发起调用
            let result = session.call_tool_with_progress(req).await?;

            // 优先返回 structured_content；否则返回合并的content文本
            if let Some(sc) = result.structured_content {
                return Ok(serde_json::json!({ "success": true, "output": sc }));
            }
            let text = result
                .content
                .unwrap_or_default()
                .into_iter()
                .map(|c: Content| format!("{:?}", c))
                .collect::<Vec<_>>()
                .join("\n");
            return Ok(serde_json::json!({ "success": result.is_error != Some(true), "output": text }));
        }

        Err(anyhow::anyhow!("No MCP client session for connection: {}", connection_name))
    }

    /// 动态注册新工具
    pub async fn register_tool(&self, tool_name: String, tool_description: String) -> Result<()> {
        // 简化实现，实际应该创建工具并注册
        tracing::info!("Registering tool: {} - {}", tool_name, tool_description);
        Ok(())
    }

    /// 获取工具定义
    pub async fn get_tool_definition(&self, tool_name: &str) -> Result<Option<ToolDefinition>> {
        let server = self.server_manager.get_server().await;
        let registry = server.read().await.get_tool_registry();
        let reg = registry.read().await;
        Ok(reg.get(tool_name).map(|info| ToolDefinition {
            name: info.name.clone(),
            description: Some(info.description.clone()),
            input_schema: serde_json::Value::Object(serde_json::Map::new()),
            output_content_types: Some(vec!["text/plain".to_string()]),
            tool_permissions: None,
        }))
    }

    /// 连接到外部MCP服务器
    pub async fn connect_to_external_server(&self, package: &str) -> Result<String> {
        let client_arc = self.client_manager.get_client();
        let client = client_arc;
        client.connect_to_server(package).await.map(|_| format!("Connected to {}", package))
    }

    /// 连接到子进程MCP服务器
    pub async fn connect_to_process(
        &self,
        name: String,
        _command: &str,
        _args: Vec<&str>,
    ) -> Result<String> {
        let client_arc = self.client_manager.get_client();
        let client = client_arc;
        client.connect_to_server(&name).await.map(|_| format!("Connected to {}", name))
    }

    /// 断开外部连接
    pub async fn disconnect_external(&self, connection_id: &str) -> Result<()> {
        let client_arc = self.client_manager.get_client();
        let client = client_arc;
        client.disconnect(connection_id).await
    }

    /// 获取连接状态信息
    pub async fn get_connection_info(&self) -> Result<Vec<McpConnectionInfo>> {
        let mut connections = Vec::new();

        // 内置服务器状态
        let is_running = self.is_server_running().await;
        let tools_count = self
            .server_manager
            .get_server()
            .await
            .read()
            .await
            .list_tools()
            .await
            .len();

        connections.push(McpConnectionInfo {
            name: "Built-in security tools server".to_string(),
            transport_type: "internal".to_string(),
            endpoint: "localhost".to_string(),
            status: if is_running {
                "connected".to_string()
            } else {
                "disconnected".to_string()
            },
            tools_count,
            last_activity: if is_running {
                Some(
                    chrono::Utc::now()
                        .format("%Y-%m-%d %H:%M:%S UTC")
                        .to_string(),
                )
            } else {
                None
            },
        });

        // 外部连接状态
        let client_arc = self.client_manager.get_client();
        let client = client_arc;
        let external_connections = client.get_all_connection_status().await;
        for (name, status) in external_connections {
            connections.push(McpConnectionInfo {
                name: name.clone(),
                transport_type: "unknown".to_string(),
                endpoint: "unknown".to_string(),
                status: match status {
                    ConnectionStatus::Connected => "connected".to_string(),
                    ConnectionStatus::Connecting => "connecting".to_string(),
                    ConnectionStatus::Disconnected => {
                        "disconnected".to_string()
                    }
                    ConnectionStatus::Error(e) => format!("error: {}", e),
                },
                tools_count: 0, // TODO: 实现工具计数
                last_activity: Some(
                    chrono::Utc::now()
                        .format("%Y-%m-%d %H:%M:%S UTC")
                        .to_string(),
                ),
            });
        }

        Ok(connections)
    }

    /// 测试工具连接
    pub async fn test_tool(&self, tool_name: &str) -> Result<bool> {
        let registry = self
            .server_manager
            .get_server()
            .await
            .read()
            .await
            .get_tool_registry();
        let reg = registry.read().await;

        if reg.get(tool_name).is_some() {
            return Ok(true);
        }

        // 检查外部工具
        let client_arc = self.client_manager.get_client();
        let connections = client_arc.get_all_connection_status().await;
        for (name, status) in connections {
            if matches!(status, ConnectionStatus::Connected) {
                // TODO: 实现工具检查逻辑
                tracing::debug!("Checking tools for connection: {}", name);
            }
        }

        Ok(false)
    }

    /// 获取工具使用统计
    pub async fn get_tool_stats(&self) -> Result<Value> {
        let tools = self.get_available_tools().await?;
        let connections = self.get_connection_info().await?;

        let internal_tools = tools
            .iter()
            .filter(|t| t.metadata.author.contains("Internal"))
            .count();
        let external_tools = tools
            .iter()
            .filter(|t| t.metadata.author.contains("External"))
            .count();
        let connected_servers = connections
            .iter()
            .filter(|c| c.status == "connected")
            .count();

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
    pub async fn get_tools_by_category(
        &self,
        category: crate::tools::ToolCategory,
    ) -> Result<Vec<crate::tools::ToolInfo>> {
        let all_tools = self.get_available_tools().await?;
        Ok(all_tools
            .into_iter()
            .filter(|t| t.category == category)
            .collect())
    }

    /// 搜索工具
    pub async fn search_tools(&self, query: &str) -> Result<Vec<crate::tools::ToolInfo>> {
        let all_tools = self.get_available_tools().await?;
        Ok(all_tools
            .into_iter()
            .filter(|t| t.name.contains(query) || t.description.contains(query))
            .collect())
    }

    /// 获取单个工具
    pub async fn get_tool(&self, tool_id: &str) -> Result<Option<crate::tools::ToolInfo>> {
        let all_tools = self.get_available_tools().await?;
        Ok(all_tools.into_iter().find(|t| t.id == tool_id))
    }

    /// 获取执行结果
    pub async fn get_execution_result(
        &self,
        execution_id: uuid::Uuid,
    ) -> Result<Option<crate::tools::ToolExecutionResult>> {
        let server = self.server_manager.get_server().await;
        let server_guard = server.read().await;
        Ok(server_guard.get_execution_result(&execution_id))
    }

    /// 添加服务器
    pub async fn add_server(&self, config: crate::tools::McpServerConfig) -> Result<String> {
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

    /// 保存MCP服务器状态到数据库
    pub async fn save_server_state(&self, server_name: &str, enabled: bool) -> Result<()> {
        let timestamp = Utc::now().timestamp();
        let state_field = if enabled { "last_started_at" } else { "last_stopped_at" };
        
        let query = format!(
            "INSERT OR REPLACE INTO mcp_server_state (server_name, enabled, {}, updated_at) 
             VALUES ('{}', {}, {}, {})",
            state_field, server_name, if enabled { 1 } else { 0 }, timestamp, timestamp
        );
        
        self.db_service.execute_query(&query).await
            .map_err(|e| anyhow::anyhow!("Failed to save MCP server state: {}", e))?;
        
        tracing::info!("Saved MCP server '{}' state: enabled={}", server_name, enabled);
        Ok(())
    }

    /// 从数据库恢复MCP服务器状态
    pub async fn restore_server_state(&self, server_name: &str) -> Result<bool> {
        let query = format!(
            "SELECT enabled FROM mcp_server_state WHERE server_name = '{}'",
            server_name
        );
        
        let result = self.db_service.execute_query(&query).await
            .map_err(|e| anyhow::anyhow!("Failed to restore MCP server state: {}", e))?;
        
        if let Some(row) = result.first() {
            if let Some(enabled_value) = row.get("enabled") {
                let enabled = match enabled_value {
                    serde_json::Value::Bool(b) => *b,
                    serde_json::Value::Number(n) => n.as_i64().map(|i| i != 0).unwrap_or(false),
                    serde_json::Value::String(s) => s != "0" && s.to_lowercase() != "false",
                    _ => false,
                };
                tracing::info!("Restored MCP server '{}' state: enabled={}", server_name, enabled);
                return Ok(enabled);
            }
        }
        
        // 如果没有找到记录，返回默认值false
        tracing::info!("No saved state found for MCP server '{}', defaulting to disabled", server_name);
        Ok(false)
    }

    /// 启动服务器并保存状态
    pub async fn start_server_with_state_save(&self, transport: &str, endpoint: Option<&str>) -> Result<()> {
        self.start_server(transport, endpoint).await?;
        self.save_server_state("builtin_security_tools", true).await?;
        Ok(())
    }

    /// 停止服务器并保存状态
    pub async fn stop_server_with_state_save(&self) -> Result<()> {
        self.stop_server().await?;
        self.save_server_state("builtin_security_tools", false).await?;
        Ok(())
    }

    /// 自动恢复上次的服务器状态
    pub async fn auto_restore_server_state(&self) -> Result<()> {
        let was_enabled = self.restore_server_state("builtin_security_tools").await?;
        
        if was_enabled {
            tracing::info!("Auto-starting MCP server based on previous state");
            // 使用stdio传输默认启动
            if let Err(e) = self.start_server("stdio", None).await {
                tracing::warn!("Failed to auto-start MCP server: {}", e);
                // 即使启动失败，也不返回错误，只是记录日志
            }
        } else {
            tracing::info!("MCP server was disabled in previous session, not auto-starting");
        }
        
        Ok(())
    }

    /// 获取所有服务器的保存状态
    pub async fn get_all_server_states(&self) -> Result<Vec<(String, bool, Option<i64>)>> {
        let query = "SELECT server_name, enabled, last_started_at FROM mcp_server_state";
        
        let result = self.db_service.execute_query(query).await
            .map_err(|e| anyhow::anyhow!("Failed to get server states: {}", e))?;
        
        let mut states = Vec::new();
        for row in result {
            let server_name = row.get("server_name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            
            let enabled = match row.get("enabled") {
                Some(serde_json::Value::Bool(b)) => *b,
                Some(serde_json::Value::Number(n)) => n.as_i64().map(|i| i != 0).unwrap_or(false),
                Some(serde_json::Value::String(s)) => s != "0" && s.to_lowercase() != "false",
                _ => false,
            };
            
            let last_started = row.get("last_started_at")
                .and_then(|v| v.as_i64());
            
            states.push((server_name, enabled, last_started));
        }
        
        Ok(states)
    }

    /// 解析工具参数从JSON schema
    fn parse_tool_parameters(schema: &serde_json::Value) -> crate::tools::ToolParameters {
        let mut parameters = Vec::new();
        let mut required = Vec::new();
        let mut optional = Vec::new();

        // 解析JSON Schema中的参数定义
        if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
            for (param_name, param_def) in properties {
                // 确定参数类型
                let param_type = match param_def.get("type").and_then(|t| t.as_str()) {
                    Some("string") => crate::tools::ParameterType::String,
                    Some("number") | Some("integer") => crate::tools::ParameterType::Number,
                    Some("boolean") => crate::tools::ParameterType::Boolean,
                    Some("array") => crate::tools::ParameterType::Array,
                    Some("object") => crate::tools::ParameterType::Object,
                    _ => crate::tools::ParameterType::String, // 默认为字符串
                };

                // 获取参数描述
                let description = param_def.get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("")
                    .to_string();

                // 获取默认值
                let default_value = param_def.get("default").cloned();

                // 检查是否为必需参数
                let is_required = schema.get("required")
                    .and_then(|r| r.as_array())
                    .map(|arr| arr.iter().any(|v| v.as_str() == Some(param_name)))
                    .unwrap_or(false);

                // 创建参数定义
                let param_definition = crate::tools::ParameterDefinition {
                    name: param_name.clone(),
                    param_type,
                    description,
                    required: is_required,
                    default_value,
                };

                parameters.push(param_definition);

                // 添加到必需或可选列表
                if is_required {
                    required.push(param_name.clone());
                } else {
                    optional.push(param_name.clone());
                }
            }
        }

        crate::tools::ToolParameters {
            parameters,
            schema: schema.clone(),
            required,
            optional,
        }
    }

    /// 解析RMCP工具参数
    fn parse_rmcp_tool_parameters(input_schema: &serde_json::Value) -> crate::tools::ToolParameters {
        let mut parameters = Vec::new();
        let mut required = Vec::new();
        let mut optional = Vec::new();

        if let Some(properties) = input_schema.get("properties").and_then(|p| p.as_object()) {
            let required_props = input_schema
                .get("required")
                .and_then(|r| r.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<std::collections::HashSet<_>>()
                })
                .unwrap_or_default();

            for (param_name, param_info) in properties {
                let param_type = match param_info.get("type").and_then(|t| t.as_str()) {
                    Some("string") => crate::tools::ParameterType::String,
                    Some("number") | Some("integer") => crate::tools::ParameterType::Number,
                    Some("boolean") => crate::tools::ParameterType::Boolean,
                    Some("array") => crate::tools::ParameterType::Array,
                    Some("object") => crate::tools::ParameterType::Object,
                    _ => crate::tools::ParameterType::String,
                };

                let description = param_info
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("No description")
                    .to_string();

                let default_value = param_info.get("default").cloned();
                let is_required = required_props.contains(param_name.as_str());

                let param_definition = crate::tools::ParameterDefinition {
                    name: param_name.clone(),
                    param_type,
                    description,
                    required: is_required,
                    default_value,
                };

                parameters.push(param_definition);

                // 添加到必需或可选列表
                if is_required {
                    required.push(param_name.clone());
                } else {
                    optional.push(param_name.clone());
                }
            }
        }

        crate::tools::ToolParameters {
            parameters,
            schema: input_schema.clone(),
            required,
            optional,
        }
    }
}
