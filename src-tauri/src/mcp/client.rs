use anyhow::{anyhow, Result};
use async_trait::async_trait;
use rmcp::{
    model::{
        CallToolRequestParam, CallToolResult, InitializeResult, RawContent,
        Tool,
    },
    service::{RoleClient, RunningService, ServiceExt},
    transport::{self, ConfigureCommandExt, TokioChildProcess},
};
use serde::Serialize;
use serde_json::Value;
use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::services::database::DatabaseService;
use super::types::ConnectionStatus;

#[derive(Serialize, Clone, Debug)]
pub struct ConfiguredServer {
    pub db_id: String,
    pub name: String,
    pub id: Option<String>,
    pub transport_type: String,
    pub endpoint: String,
    pub status: String,
    // For reconnecting
    pub command: String,
    pub args: Vec<String>,
}

#[async_trait]
pub trait McpSession: Send + Sync {
    /// 返回初始化结果信息
    async fn peer_info(&self) -> Option<InitializeResult>;
    /// 获取所有工具
    async fn list_all_tools(&self) -> Result<Vec<Tool>>;
    /// 调用工具
    async fn call_tool(&self, params: CallToolRequestParam) -> Result<CallToolResult>;
    /// 发送取消通知并关闭连接
    async fn cancel(&self) -> Result<()>;
}

pub struct McpSessionImpl {
    service: RwLock<Option<RunningService<RoleClient, ()>>>,
}

#[async_trait]
impl McpSession for McpSessionImpl {
    async fn peer_info(&self) -> Option<InitializeResult> {
        self.service
            .read()
            .await
            .as_ref()
            .and_then(|s| s.peer_info().cloned())
    }

    async fn list_all_tools(&self) -> Result<Vec<Tool>> {
        if let Some(service) = self.service.read().await.as_ref() {
            Ok(service.list_all_tools().await?)
        } else {
            Err(anyhow!("Session is already cancelled"))
        }
    }

    async fn call_tool(&self, params: CallToolRequestParam) -> Result<CallToolResult> {
        if let Some(service) = self.service.read().await.as_ref() {
            Ok(service.call_tool(params).await?)
        } else {
            Err(anyhow!("Session is already cancelled"))
        }
    }

    async fn cancel(&self) -> Result<()> {
        if let Some(service) = self.service.write().await.take() {
            service.cancel().await?;
        }
        Ok(())
    }
}

/// MCP 连接信息
#[derive(Clone)]
pub struct McpConnection {
    pub id: String,
    pub name: String,
    pub transport_type: String, // stdio, http, websocket
    pub endpoint: String,
    pub status: ConnectionStatus,
    pub tools: Vec<Tool>,
    pub session: Option<Arc<dyn McpSession>>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub error_count: u32,
}

impl std::fmt::Debug for McpConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpConnection")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("transport_type", &self.transport_type)
            .field("endpoint", &self.endpoint)
            .field("status", &self.status)
            .field("tools", &self.tools)
            .field(
                "session",
                &self.session.as_ref().map(|_| "Some(<dyn McpSession>)"),
            )
            .field("last_activity", &self.last_activity)
            .field("error_count", &self.error_count)
            .finish()
    }
}

/// MCP 客户端
#[derive(Debug)]
pub struct McpClient {
    connections: Arc<RwLock<HashMap<String, McpConnection>>>,
    connection_timeout: Duration,
}

impl McpClient {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            connection_timeout: Duration::from_secs(30),
        }
    }

    /// 设置连接超时
    pub fn set_connection_timeout(&mut self, timeout: Duration) {
        self.connection_timeout = timeout;
    }

    /// 连接到子进程MCP服务器
    pub async fn connect_to_child_process(
        &self,
        name: String,
        command: &str,
        args: Vec<&str>,
    ) -> Result<String> {
        info!(
            "Attempting to connect to MCP server: {} {}",
            command,
            args.join(" ")
        );

        let connection_id = uuid::Uuid::new_v4().to_string();
        let endpoint = format!("{} {}", command, args.join(" "));

        // 创建错误状态的连接以便前端可以看到
        let mut error_connection = McpConnection {
            id: connection_id.clone(),
            name: name.clone(),
            transport_type: "child_process".to_string(),
            endpoint: endpoint.clone(),
            status: ConnectionStatus::Connecting,
            tools: Vec::new(),
            session: None,
            last_activity: chrono::Utc::now(),
            error_count: 0,
        };

        // 先插入连接状态为"正在连接"
        {
            let mut connections = self.connections.write().await;
            connections.insert(connection_id.clone(), error_connection.clone());
        }

        // 尝试建立连接
        let connection_result: Result<(Arc<dyn McpSession>, Vec<Tool>)> = async {
            let transport =
                TokioChildProcess::new(tokio::process::Command::new(command).configure(|cmd| {
                    for arg in &args {
                        cmd.arg(arg);
                    }
                    cmd.stdin(std::process::Stdio::piped())
                        .stdout(std::process::Stdio::piped());
                }))?;

            let client_service = ().serve(transport).await?;

            let session: Arc<dyn McpSession> = Arc::new(McpSessionImpl {
                service: RwLock::new(Some(client_service)),
            });

            info!("Successfully initialized MCP connection: {}", name);

            let peer_info = session.peer_info().await;
            info!("MCP server information: {:?}", peer_info);

            let tools = match session.list_all_tools().await {
                Ok(list_tools_result) => {
                    info!(
                        "Successfully retrieved tool list, tool count: {}",
                        list_tools_result.len()
                    );
                    list_tools_result
                }
                Err(e) => {
                    warn!("Failed to retrieve tool list: {}", e);
                    Vec::new()
                }
            };

            Ok((session, tools))
        }.await;

        // 根据连接结果更新连接状态
        match connection_result {
            Ok((session, tools)) => {
                error_connection.status = ConnectionStatus::Connected;
                error_connection.tools = tools;
                error_connection.session = Some(session);
                error_connection.error_count = 0;
            }
            Err(e) => {
                warn!("Failed to connect to MCP server '{}': {}", name, e);
                error_connection.status = ConnectionStatus::Error(e.to_string());
                error_connection.error_count += 1;
            }
        }

        // 更新连接状态
        {
            let mut connections = self.connections.write().await;
            connections.insert(connection_id.clone(), error_connection);
        }

        // 即使连接失败也返回connection_id，以便前端可以显示错误状态
        Ok(connection_id)
    }

    /// 连接到HTTP MCP服务器
    pub async fn connect_to_http_server(&self, name: String, url: &str) -> Result<String> {
        info!("Attempting to connect to HTTP MCP server: {}", url);

        let connection_id = uuid::Uuid::new_v4().to_string();

        // 创建初始连接状态
        let mut connection = McpConnection {
            id: connection_id.clone(),
            name: name.clone(),
            transport_type: "http".to_string(),
            endpoint: url.to_string(),
            status: ConnectionStatus::Connecting,
            tools: Vec::new(),
            session: None,
            last_activity: chrono::Utc::now(),
            error_count: 0,
        };

        // 先插入连接状态为"正在连接"
        {
            let mut connections = self.connections.write().await;
            connections.insert(connection_id.clone(), connection.clone());
        }

        // 尝试建立连接
        let connection_result: Result<(Arc<dyn McpSession>, Vec<Tool>)> = async {
            let transport = transport::sse_client::SseClientTransport::start(url.to_string()).await?;
            let client_service = ().serve(transport).await?;

            let session: Arc<dyn McpSession> = Arc::new(McpSessionImpl {
                service: RwLock::new(Some(client_service)),
            });

            info!("Successfully initialized MCP connection: {}", name);

            let peer_info = session.peer_info().await;
            info!("MCP server information: {:?}", peer_info);

            let tools = match session.list_all_tools().await {
                Ok(list_tools_result) => {
                    info!(
                        "Successfully retrieved tool list, tool count: {}",
                        list_tools_result.len()
                    );
                    list_tools_result
                }
                Err(e) => {
                    warn!("Failed to retrieve tool list: {}", e);
                    Vec::new()
                }
            };

            Ok((session, tools))
        }.await;

        // 根据连接结果更新连接状态
        match connection_result {
            Ok((session, tools)) => {
                connection.status = ConnectionStatus::Connected;
                connection.tools = tools;
                connection.session = Some(session);
                connection.error_count = 0;
            }
            Err(e) => {
                warn!("Failed to connect to HTTP MCP server '{}': {}", name, e);
                connection.status = ConnectionStatus::Error(e.to_string());
                connection.error_count += 1;
            }
        }

        // 更新连接状态
        {
            let mut connections = self.connections.write().await;
            connections.insert(connection_id.clone(), connection);
        }

        Ok(connection_id)
    }

    /// 连接到NPX MCP服务器（通用JS服务器）
    pub async fn connect_to_npx_server(&self, package: &str) -> Result<String> {
        self.connect_to_child_process(
            format!("NPX {}", package),
            if cfg!(target_os = "windows") {
                "npx.cmd"
            } else {
                "npx"
            },
            vec!["-y", package],
        )
        .await
    }

    /// 连接到MCP Inspector
    pub async fn connect_to_mcp_inspector(&self) -> Result<String> {
        self.connect_to_npx_server("@modelcontextprotocol/inspector")
            .await
    }

    /// 连接到通用计数器示例
    pub async fn connect_to_counter_example(&self) -> Result<String> {
        let counter_path = if cfg!(target_os = "windows") {
            "target/release/examples/servers_counter_stdio.exe"
        } else {
            "target/release/examples/servers_counter_stdio"
        };

        self.connect_to_child_process("Counter Example".to_string(), counter_path, vec![])
            .await
    }

    /// 获取所有连接
    pub async fn get_connections(&self) -> Vec<McpConnection> {
        self.connections.read().await.values().cloned().collect()
    }

    /// 断开连接
    pub async fn disconnect(&self, connection_id: &str) -> Result<()> {
        if let Some(mut connection) = self.connections.write().await.remove(connection_id) {
            connection.status = ConnectionStatus::Disconnected;
            if let Some(session) = connection.session {
                if let Err(e) = session.cancel().await {
                    warn!("Failed to send cancel notification: {}", e);
                }
            }
            info!("MCP connection disconnected: {}", connection.name);
            Ok(())
        } else {
            Err(anyhow!("Connection not found: {}", connection_id))
        }
    }

    /// 检查服务器状态
    pub async fn check_server_status(&self, connection_id: &str) -> ConnectionStatus {
        if let Some(connection) = self.connections.read().await.get(connection_id) {
            if let ConnectionStatus::Error(_) = &connection.status {
                return connection.status.clone();
            }
            if connection.session.is_none() {
                return ConnectionStatus::Disconnected;
            }
            if (chrono::Utc::now() - connection.last_activity).num_seconds() > 300 {
                return ConnectionStatus::Disconnected;
            }
            connection.status.clone()
        } else {
            ConnectionStatus::Disconnected
        }
    }

    pub async fn get_connection_tools(&self, connection_id: &str) -> Result<Vec<Tool>> {
        let connections = self.connections.read().await;
        if let Some(connection) = connections.get(connection_id) {
            Ok(connection.tools.clone())
        } else {
            Err(anyhow!("未找到指定的连接: {}", connection_id))
        }
    }

    pub async fn call_tool(
        &self,
        connection_id: &str,
        tool_name: &str,
        parameters: Value,
    ) -> Result<CallToolResult> {
        let connections = self.connections.read().await;
        if let Some(connection) = connections.get(connection_id) {
            if let Some(session) = &connection.session {
                let params = CallToolRequestParam {
                    name: tool_name.to_string().into(),
                    arguments: parameters.as_object().cloned(),
                };
                session.call_tool(params).await
            } else {
                Err(anyhow!(
                    "Session not available for connection {}",
                    connection_id
                ))
            }
        } else {
            Err(anyhow!("Connection not found: {}", connection_id))
        }
    }

    /// 在任何可用连接上执行工具
    pub async fn execute_tool_on_any(&self, tool_name: &str, parameters: Value) -> Result<Value> {
        let connections = self.connections.read().await;
        
        tracing::info!("[MCP Client] Looking for tool '{}' across {} connections", tool_name, connections.len());
        
        let mut available_tools = Vec::new();
        let mut connection_errors = Vec::new();
        
        for (conn_id, conn) in connections.iter() {
            tracing::debug!("[MCP Client] Checking connection '{}' (status: {:?})", conn.name, conn.status);
            
            if conn.status == ConnectionStatus::Connected {
                // 记录此连接的所有工具
                for tool in &conn.tools {
                    available_tools.push(format!("{}@{}", tool.name, conn.name));
                }
                
                if conn.tools.iter().any(|t| t.name == tool_name) {
                    tracing::info!("[MCP Client] Found tool '{}' on connection '{}'", tool_name, conn.name);
                    
                    if let Some(session) = &conn.session {
                        let params = CallToolRequestParam {
                            name: tool_name.to_string().into(),
                            arguments: parameters.as_object().cloned(),
                        };

                        tracing::debug!("[MCP Client] Calling tool '{}' with params: {:?}", tool_name, params);
                        
                        match session.call_tool(params).await {
                            Ok(result) => {
                                tracing::debug!("[MCP Client] Tool call result: {:?}", result);
                                
                                if result.is_error.unwrap_or(false) {
                                    let msg = result
                                        .content
                                        .first()
                                        .and_then(|c| match &c.raw {
                                            RawContent::Text(t) => Some(t.text.clone()),
                                            _ => None,
                                        })
                                        .unwrap_or_else(|| {
                                            "Tool execution failed with no message.".to_string()
                                        });
                                    tracing::error!("[MCP Client] Tool '{}' returned error: {}", tool_name, msg);
                                    return Err(anyhow!("Tool execution error: {}", msg));
                                }

                                if let Some(raw_content) =
                                    result.content.into_iter().next().map(|c| c.raw)
                                {
                                    let final_result = match raw_content {
                                        RawContent::Text(t) => {
                                            tracing::debug!("[MCP Client] Tool '{}' returned text: {}", tool_name, t.text);
                                            serde_json::from_str(&t.text)
                                                .unwrap_or_else(|_| Value::String(t.text.clone()))
                                        },
                                        _ => {
                                            tracing::debug!("[MCP Client] Tool '{}' returned non-text content", tool_name);
                                            Value::Null
                                        },
                                    };
                                    tracing::info!("[MCP Client] Tool '{}' executed successfully", tool_name);
                                    return Ok(final_result);
                                }
                                tracing::info!("[MCP Client] Tool '{}' executed successfully with no content", tool_name);
                                return Ok(Value::Null);
                            }
                            Err(e) => {
                                let error_msg = format!("Failed to call tool '{}' on connection '{}': {}", tool_name, conn.name, e);
                                tracing::error!("[MCP Client] {}", error_msg);
                                connection_errors.push(error_msg);
                                // 继续尝试其他连接
                            }
                        }
                    } else {
                        let error_msg = format!("Connection '{}' has no active session", conn.name);
                        tracing::warn!("[MCP Client] {}", error_msg);
                        connection_errors.push(error_msg);
                    }
                } else {
                    tracing::debug!("[MCP Client] Tool '{}' not found on connection '{}'", tool_name, conn.name);
                }
            } else {
                tracing::debug!("[MCP Client] Connection '{}' is not connected (status: {:?})", conn.name, conn.status);
            }
        }
        
        // 如果没有找到工具，提供详细的错误信息
        let error_msg = if available_tools.is_empty() {
            format!("Tool '{}' not found. No tools available on any connected server.", tool_name)
        } else {
            format!(
                "Tool '{}' not found. Available tools: [{}]. Connection errors: [{}]",
                tool_name,
                available_tools.join(", "),
                connection_errors.join("; ")
            )
        };
        
        tracing::error!("[MCP Client] {}", error_msg);
        Err(anyhow!(error_msg))
    }

    /// 从配置加载连接
    pub async fn load_from_config(&self, config_path: &PathBuf) -> Result<Vec<String>> {
        let config_str = std::fs::read_to_string(config_path)?;
        let config: Value = serde_json::from_str(&config_str)?;
        let mut connection_ids = Vec::new();

        if let Some(mcp_servers) = config.get("mcpServers").and_then(Value::as_object) {
            for (name, server_config) in mcp_servers {
                let args = server_config
                    .get("args")
                    .and_then(Value::as_array)
                    .map(|arr| arr.iter().filter_map(Value::as_str).collect())
                    .unwrap_or_default();
                if let Some(command) = server_config.get("command").and_then(Value::as_str) {
                    if let Ok(id) = self
                        .connect_to_child_process(name.clone(), command, args)
                        .await
                    {
                        info!(
                            "Successfully loaded MCP server from configuration: {}",
                            name
                        );
                        connection_ids.push(id);
                    } else {
                        warn!("Failed to load MCP server from configuration: {}", name);
                    }
                }
            }
        }
        Ok(connection_ids)
    }

    pub async fn connect_to_common_servers(&self) -> Result<Vec<String>> {
        let mut ids = Vec::new();
        if let Ok(id) = self.connect_to_mcp_inspector().await {
            info!("Successfully connected to MCP Inspector");
            ids.push(id);
        } else {
            warn!("Failed to connect to MCP Inspector");
        }

        if let Ok(id) = self.connect_to_counter_example().await {
            info!("Successfully connected to Counter Example");
            ids.push(id);
        } else {
            warn!("Failed to connect to Counter Example");
        }
        Ok(ids)
    }

    /// 定期清理无效连接
    pub async fn cleanup_connections(&self) -> usize {
        let mut connections = self.connections.write().await;
        let mut to_remove = Vec::new();
        for (id, conn) in connections.iter() {
            let elapsed = chrono::Utc::now() - conn.last_activity;
            // 对于错误状态的连接，给用户10分钟时间查看错误信息
            let should_remove = elapsed.num_hours() > 1 || 
                (matches!(conn.status, ConnectionStatus::Error(_)) && elapsed.num_minutes() > 10);
            if should_remove {
                to_remove.push(id.clone());
            }
        }

        for id in &to_remove {
            if let Some(mut conn) = connections.remove(id) {
                if let Some(session) = conn.session.take() {
                    tokio::spawn(async move {
                        if let Err(e) = session.cancel().await {
                            warn!("Failed to send cancel on cleanup: {}", e);
                        }
                    });
                }
            }
        }

        let removed_count = to_remove.len();
        if removed_count > 0 {
            info!("Cleaned {} invalid MCP connections", removed_count);
        }
        removed_count
    }

    pub async fn get_all_servers_with_status(
        &self,
        db: &DatabaseService,
    ) -> Result<Vec<ConfiguredServer>> {
        // 1. Get active connections
        let active_connections = self.get_connections().await;
        let mut active_map: HashMap<String, McpConnection> = active_connections
            .into_iter()
            .map(|c| (c.name.clone(), c))
            .collect();

        let mut all_servers = Vec::new();

        // 2. Process servers from database
        let configs = db.get_all_mcp_server_configs().await?;
        for config in configs {
            let args: Vec<String> = serde_json::from_str(&config.args).unwrap_or_default();
            let endpoint = format!("{} {}", config.command, args.join(" "));

            if let Some(active_conn) = active_map.remove(&config.name) {
                // Server is in DB and is currently connected
                all_servers.push(ConfiguredServer {
                    db_id: config.id,
                    name: config.name,
                    id: Some(active_conn.id),
                    transport_type: active_conn.transport_type,
                    endpoint: active_conn.endpoint,
                    status: active_conn.status.to_string(),
                    command: config.command,
                    args,
                });
            } else {
                // Server is in DB but not connected
                all_servers.push(ConfiguredServer {
                    db_id: config.id,
                    name: config.name,
                    id: None,
                    transport_type: "child_process".to_string(),
                    endpoint,
                    status: "Disconnected".to_string(),
                    command: config.command,
                    args,
                });
            }
        }

        // 3. Add any remaining active connections not found in config (e.g., ad-hoc HTTP)
        for (name, active_conn) in active_map {
            let mut parts = active_conn.endpoint.splitn(2, ' ');
            let command = parts.next().unwrap_or("").to_string();
            let args = parts
                .next()
                .unwrap_or("")
                .split_whitespace()
                .map(String::from)
                .collect();

            all_servers.push(ConfiguredServer {
                db_id: "-1".to_string(), // 表示这是一个临时的、未保存到数据库的连接
                name,
                id: Some(active_conn.id),
                transport_type: active_conn.transport_type,
                endpoint: active_conn.endpoint,
                status: active_conn.status.to_string(),
                command,
                args,
            });
        }

        Ok(all_servers)
    }
}

/// MCP客户端管理器
#[derive(Debug)]
pub struct McpClientManager {
    client: Arc<RwLock<McpClient>>,
    db: Arc<DatabaseService>,
}

impl McpClientManager {
    pub fn new(db: Arc<DatabaseService>) -> Self {
        Self {
            client: Arc::new(RwLock::new(McpClient::new())),
            db,
        }
    }

    pub fn get_client(&self) -> Arc<RwLock<McpClient>> {
        self.client.clone()
    }

    pub async fn initialize(&self) -> Result<()> {
        let configs = self.db.get_all_mcp_server_configs().await?;
        let client = self.client.read().await;

        for config in configs {
            if config.enabled {
                let args: Vec<String> = serde_json::from_str(&config.args)?;
                let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                if let Err(e) = client
                    .connect_to_child_process(config.name.clone(), &config.command, args_refs)
                    .await
                {
                    warn!(
                        "Failed to connect to server {} when starting: {}",
                        config.name, e
                    );
                }
            }
        }

        let client_clone = self.client.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3600));
            loop {
                interval.tick().await;
                let client = client_clone.read().await;
                client.cleanup_connections().await;
            }
        });

        Ok(())
    }

    pub async fn check_servers_status(&self) -> bool {
        let client = self.client.read().await;
        let connections = client.get_connections().await;
        for conn in connections {
            if conn.status == ConnectionStatus::Connected {
                return true;
            }
        }
        false
    }

    pub async fn save_config_to_db(
        &self,
        name: &str,
        description: Option<&str>,
        command: &str,
        args: &[String],
    ) -> Result<String> {
        self.db
            .create_mcp_server_config(name, description, command, args)
            .await
    }

    /// 使用ConfigureCommandExt连接到子进程MCP服务器
    pub async fn connect_with_command(
        &self,
        name: &str,
        command: &str,
        args: Vec<String>,
    ) -> Result<String> {
        use rmcp::transport::ConfigureCommandExt;

        info!(
            "Attempting to connect to MCP server: {} {:?}",
            command, args
        );

        let client = self.client.read().await;

        let transport = rmcp::transport::TokioChildProcess::new(
            tokio::process::Command::new(command).configure(|cmd| {
                for arg in &args {
                    cmd.arg(arg);
                }
                cmd.stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped());
            }),
        )?;

        let client_service = ().serve(transport).await?;

        let session: Arc<dyn McpSession> = Arc::new(McpSessionImpl {
            service: RwLock::new(Some(client_service)),
        });

        let connection_id = uuid::Uuid::new_v4().to_string();
        info!("Successfully initialized MCP connection: {}", name);

        let peer_info = session.peer_info().await;
        info!("MCP server information: {:?}", peer_info);

        let tools = match session.list_all_tools().await {
            Ok(list_tools_result) => {
                info!(
                    "Successfully retrieved tool list, tool count: {}",
                    list_tools_result.len()
                );
                list_tools_result
            }
            Err(e) => {
                warn!("Failed to retrieve tool list: {}", e);
                Vec::new()
            }
        };

        let connection = McpConnection {
            id: connection_id.clone(),
            name: name.to_string(),
            transport_type: "child_process".to_string(),
            endpoint: format!("{} {}", command, args.join(" ")),
            status: ConnectionStatus::Connected,
            tools,
            session: Some(session),
            last_activity: chrono::Utc::now(),
            error_count: 0,
        };

        let mut connections = client.connections.write().await;
        connections.insert(connection_id.clone(), connection);

        // 如果配置不存在，则保存到数据库
        if self.db.get_mcp_server_config_by_name(name).await?.is_none() {
            if let Err(e) = self.save_config_to_db(name, None, command, &args).await {
                warn!("Failed to save MCP configuration to database: {}", e);
            }
        }

        Ok(connection_id)
    }

    pub async fn connect_to_server(
        &self,
        name: &str,
        command: &str,
        args: Vec<&str>,
    ) -> Result<String> {
        let client = self.client.read().await;
        let id = client
            .connect_to_child_process(name.to_string(), command, args.clone())
            .await?;
        drop(client);
        self.save_config_to_db(
            name,
            None,
            command,
            &args.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        )
        .await?;
        Ok(id)
    }

    pub async fn disconnect(&self, connection_id: &str) -> Result<()> {
        let client = self.client.read().await;
        client.disconnect(connection_id).await?;
        // No need to update DB on disconnect, status is dynamic
        Ok(())
    }

    pub async fn connect_to_http_server(&self, name: &str, url: &str) -> Result<String> {
        let client = self.client.read().await;
        let id = client.connect_to_http_server(name.to_string(), url).await?;
        // HTTP based connections are not persisted in config for now.
        Ok(id)
    }

    pub async fn get_all_servers_with_status(&self) -> Result<Vec<ConfiguredServer>> {
        self.client
            .read()
            .await
            .get_all_servers_with_status(&self.db)
            .await
    }

    /// 获取所有连接的状态映射（按服务器名称索引）
    pub async fn get_all_connections_status(&self) -> HashMap<String, ConfiguredServer> {
        match self.get_all_servers_with_status().await {
            Ok(servers) => servers.into_iter().map(|s| (s.name.clone(), s)).collect(),
            Err(e) => {
                warn!("Failed to get server status: {}", e);
                HashMap::new()
            }
        }
    }

    /// 从数据库获取所有服务器配置
    pub async fn get_all_server_configs_from_db(
        &self,
    ) -> anyhow::Result<Vec<crate::models::database::McpServerConfig>> {
        self.db.get_all_mcp_server_configs().await
    }

    /// 更新服务器配置
    pub async fn update_server_config(
        &self,
        payload: crate::commands::mcp::FrontendMcpConnection,
    ) -> anyhow::Result<()> {
        // 解析 args 已经是 Vec<String>
        let enabled = payload.status.to_lowercase() != "disconnected";
        self.db
            .update_mcp_server_config(
                &payload.db_id,
                &payload.name,
                payload.description.as_deref(),
                &payload.command,
                &payload.args,
                enabled,
            )
            .await
    }

    /// 重新连接失败的MCP服务器
    pub async fn retry_connection(&self, connection_id: &str) -> Result<String> {
        let client = self.client.read().await;
        let connections = client.connections.read().await;
        
        if let Some(conn) = connections.get(connection_id) {
            match &conn.status {
                ConnectionStatus::Error(_) => {
                    let name = conn.name.clone();
                    let endpoint = conn.endpoint.clone();
                    let transport_type = conn.transport_type.clone();
                    drop(connections);
                    drop(client);
                    
                    // 尝试重新连接
                    if transport_type == "http" {
                        self.connect_to_http_server(&name, &endpoint).await
                    } else {
                        // 解析命令和参数
                        let parts: Vec<&str> = endpoint.split_whitespace().collect();
                        if let Some((command, args)) = parts.split_first() {
                            self.connect_to_server(&name, command, args.to_vec()).await
                        } else {
                            Err(anyhow!("Invalid endpoint format: {}", endpoint))
                        }
                    }
                }
                _ => Err(anyhow!("Connection is not in error state"))
            }
        } else {
            Err(anyhow!("Connection not found: {}", connection_id))
        }
    }
}

impl McpClientManager {
    fn default() -> Self {
        panic!("McpClientManager must be initialized with a DatabaseService");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_connect_to_counter_example() {
        if std::env::var("RUN_INTEGRATION_TESTS").is_err() {
            return;
        }
        let client = McpClient::new();
        let result = client.connect_to_counter_example().await;
        assert!(result.is_ok());

        let id = result.unwrap();
        let tools = client.get_connection_tools(&id).await.unwrap();
        assert!(!tools.is_empty());

        let result = client
            .call_tool(&id, "increment", serde_json::json!({}))
            .await;
        assert!(result.is_ok());

        let result = client
            .call_tool(&id, "get_value", serde_json::json!({}))
            .await;
        assert!(result.is_ok());

        let result = client.disconnect(&id).await;
        assert!(result.is_ok());
    }
}
