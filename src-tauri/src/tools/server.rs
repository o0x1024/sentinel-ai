use anyhow::{anyhow, Result};
use rmcp::handler::server::ServerHandler;
use rmcp::model::ErrorData;
use rmcp::model::{
    CallToolRequestParam, InitializeRequestParam, PaginatedRequestParam, ServerInfo, Tool,
};
use rmcp::service::{RequestContext, RoleServer, ServiceExt};
// 暂时注释掉传输层导入，直到API稳定
// use rmcp::transport::io::StdioTransport;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use tracing::{info, warn};

use super::protocol;
use super::unified_types::{ToolInfo, ToolExecutionResult};


/// Sentinel服务器处理器
#[derive(Clone)]
pub struct SentinelServerHandler {
    tool_registry: Arc<RwLock<HashMap<String, ToolInfo>>>,
    config: Arc<RwLock<ServerConfig>>,
    execution_results: Arc<RwLock<HashMap<Uuid, ToolExecutionResult>>>,
}

impl SentinelServerHandler {
    pub fn new(
        tool_registry: Arc<RwLock<HashMap<String, ToolInfo>>>,
        config: Arc<RwLock<ServerConfig>>,
        execution_results: Arc<RwLock<HashMap<Uuid, ToolExecutionResult>>>,
    ) -> Self {
        Self {
            tool_registry,
            config,
            execution_results,
        }
    }

    /// 检查工具是否有危险操作
    async fn check_tool_safety(&self, tool_name: &str) -> Result<bool> {
        let config = self.config.read().await;
        if !config.tool_annotations_enabled {
            return Ok(true);
        }

        // 根据工具名称判断是否有危险操作
        let is_dangerous = tool_name.contains("delete") 
            || tool_name.contains("remove") 
            || tool_name.contains("destroy");

        if is_dangerous {
            warn!("Executing potentially dangerous tool: {}", tool_name);
        }

        Ok(true) // 暂时允许所有工具执行
    }
}

impl ServerHandler for SentinelServerHandler {
    fn initialize(
        &self,
        _params: InitializeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl futures::Future<Output = Result<rmcp::model::InitializeResult, ErrorData>>
           + std::marker::Send
           + '_ {
        async move {
            let config = self.config.read().await;
            
            Ok(rmcp::model::InitializeResult {
                protocol_version: rmcp::model::ProtocolVersion::V_2024_11_05,
                server_info: rmcp::model::Implementation {
                    name: config.name.clone(),
                    version: config.version.clone(),
                },
                capabilities: rmcp::model::ServerCapabilities {
                    tools: Some(rmcp::model::ToolsCapability {
                        list_changed: Some(true),
                    }),
                    resources: None,
                    prompts: None,
                    logging: None,
                    experimental: None,
                    completions: None,
                },
                instructions: None,
            })
        }
    }

    fn list_tools(
        &self,
        _params: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl futures::Future<Output = Result<rmcp::model::ListToolsResult, ErrorData>>
           + std::marker::Send
           + '_ {
        async move {
            let registry = self.tool_registry.read().await;
            let tool_names: Vec<String> = registry.keys().cloned().collect();
            let mut tools = Vec::new();
            
            for name in tool_names {
                if let Some(tool_def) = registry.get(&name) {
                    tools.push(Tool {
                        name: tool_def.name.clone().into(),
                        description: Some(tool_def.description.clone().into()),
                        input_schema: Arc::new(
                            tool_def.parameters.schema.as_object()
                                .cloned()
                                .unwrap_or_default()
                        ),
                        annotations: None,
                        output_schema: None,
                    });
                }
            }

            Ok(rmcp::model::ListToolsResult {
                tools,
                next_cursor: None,
            })
        }
    }

    fn call_tool(
        &self,
        params: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl futures::Future<Output = Result<rmcp::model::CallToolResult, ErrorData>>
           + std::marker::Send
           + '_ {
        async move {
            // 检查工具安全性
            let _is_safe = self.check_tool_safety(&params.name.to_string()).await
                .map_err(|e| ErrorData::new(rmcp::model::ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;

            // 执行工具
            let _registry = self.tool_registry.read().await;
            // 简化工具执行逻辑
            let result = serde_json::json!({
                "status": "success",
                "message": format!("Tool {} executed", params.name)
            });

            // 存储执行结果
            let execution_id = Uuid::new_v4();
            let execution_result = ToolExecutionResult {
                execution_id,
                tool_id: params.name.to_string(),
                tool_name: params.name.to_string(),
                success: true,
                error: None,
                execution_time_ms: 0,
                started_at: chrono::Utc::now(),
                completed_at: Some(chrono::Utc::now()),
                status: super::unified_types::ExecutionStatus::Completed,
                output: serde_json::json!({"result": result}),
                metadata: std::collections::HashMap::new(),
            };
            
            self.execution_results.write().await.insert(execution_id, execution_result);

            let content = rmcp::model::Content::text(format!("{:?}", result));
            Ok(rmcp::model::CallToolResult {
                content: Some(vec![content]),
                structured_content: None,
                is_error: Some(false),
            })
        }
    }


}

/// Sentinel AI MCP 服务器实现
#[derive(Clone)]
pub struct SentinelMcpServer {
    // 工具注册表
    tool_registry: Arc<RwLock<HashMap<String, ToolInfo>>>,
    // 服务器信息
    server_info: ServerInfo,
    // 工具信息缓存
    tools_cache: Arc<RwLock<Vec<Tool>>>,
    // 服务器是否运行
    running: Arc<RwLock<bool>>,
    // 服务器配置
    config: Arc<RwLock<ServerConfig>>,
    // 执行结果
    execution_results: Arc<RwLock<HashMap<Uuid, ToolExecutionResult>>>,
}

/// 服务器配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServerConfig {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub log_level: String,
    pub enable_stdio: bool,
    pub enable_http: bool,
    pub http_port: u16,
    pub enable_websocket: bool,
    pub websocket_port: u16,
    pub tools_whitelist: Option<Vec<String>>,
    pub tools_blacklist: Option<Vec<String>>,
    // OAuth2.1 支持
    pub oauth_enabled: bool,
    pub oauth_client_id: Option<String>,
    pub oauth_client_secret: Option<String>,
    pub oauth_redirect_uri: Option<String>,
    pub oauth_scopes: Vec<String>,
    // 批处理支持
    pub batch_processing_enabled: bool,
    pub max_batch_size: usize,
    // 进度通知支持
    pub progress_notifications_enabled: bool,
    // 工具注解支持
    pub tool_annotations_enabled: bool,
    // 会话管理
    pub session_timeout_seconds: u64,
    pub enable_session_recovery: bool,
}

impl From<super::unified_types::McpServerConfig> for ServerConfig {
    fn from(config: super::unified_types::McpServerConfig) -> Self {
        let mut sc = ServerConfig::default();
        sc.name = config.name;
        sc.version = config.version;
        sc.description = Some(config.description);

        match config.transport {
            super::unified_types::TransportConfig::Stdio => {
                sc.enable_stdio = true;
            }
            super::unified_types::TransportConfig::WebSocket { url } => {
                sc.enable_websocket = true;
                if let Ok(parsed_url) = url::Url::parse(&url) {
                    if let Some(port) = parsed_url.port() {
                        sc.websocket_port = port;
                    }
                }
            }
            super::unified_types::TransportConfig::Http { base_url } => {
                sc.enable_http = true;
                if let Ok(parsed_url) = url::Url::parse(&base_url) {
                    if let Some(port) = parsed_url.port() {
                        sc.http_port = port;
                    }
                }
            }
        }
        sc
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            name: "Sentinel AI MCP Server".to_string(),
            version: "0.1.0".to_string(),
            description: Some("AI-powered vulnerability scanning MCP server".to_string()),
            log_level: "info".to_string(),
            enable_stdio: true,
            enable_http: false,
            http_port: 8080,
            enable_websocket: false,
            websocket_port: 8081,
            tools_whitelist: None,
            tools_blacklist: None,
            oauth_enabled: false,
            oauth_client_id: None,
            oauth_client_secret: None,
            oauth_redirect_uri: None,
            oauth_scopes: vec!["read".to_string(), "write".to_string()],
            batch_processing_enabled: true,
            max_batch_size: 10,
            progress_notifications_enabled: true,
            tool_annotations_enabled: true,
            session_timeout_seconds: 3600,
            enable_session_recovery: true,
        }
    }
}

impl ServerHandler for SentinelMcpServer {
    fn get_info(&self) -> ServerInfo {
        self.server_info.clone()
    }

    fn initialize(
        &self,
        _request: InitializeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl futures::Future<Output = Result<rmcp::model::InitializeResult, ErrorData>>
           + std::marker::Send
           + '_ {
        async move {
            Ok(rmcp::model::InitializeResult {
                protocol_version: rmcp::model::ProtocolVersion::V_2024_11_05,
                capabilities: rmcp::model::ServerCapabilities {
                    tools: Some(rmcp::model::ToolsCapability {
                        list_changed: Some(false),
                    }),
                    ..Default::default()
                },
                server_info: rmcp::model::Implementation {
                    name: "sentinel-ai".to_string(),
                    version: "1.0.0".to_string(),
                },
                instructions: None,
            })
        }
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl futures::Future<Output = Result<rmcp::model::ListToolsResult, ErrorData>>
           + std::marker::Send
           + '_ {
        async move {
            let tools = self.list_tools_as_rmcp().await.map_err(|e| {
                ErrorData::new(rmcp::model::ErrorCode::INTERNAL_ERROR, e.to_string(), None)
            })?;
            Ok(rmcp::model::ListToolsResult {
                tools,
                next_cursor: None,
            })
        }
    }

    fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl futures::Future<Output = Result<rmcp::model::CallToolResult, ErrorData>>
           + std::marker::Send
           + '_ {
        async move {
            let result = self
                .execute_tool(&request.name, request.arguments.into())
                .await
                .map_err(|e| {
                    ErrorData::new(rmcp::model::ErrorCode::INTERNAL_ERROR, e.to_string(), None)
                })?;

            let content = rmcp::model::Content::text(result.to_string());
            Ok(rmcp::model::CallToolResult {
                content: Some(vec![content]),
                structured_content: None,
                is_error: Some(false),
            })
        }
    }
}

// 手动实现Debug
impl fmt::Debug for SentinelMcpServer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SentinelMcpServer")
            .field("tool_registry", &"Arc<RwLock<HashMap<String, ToolInfo>>>")
            .field("server_info", &self.server_info)
            .field("running", &"Arc<RwLock<bool>>")
            .finish()
    }
}

impl SentinelMcpServer {
    pub fn new() -> Self {
        let config = ServerConfig::default();

        Self {
            tool_registry: Arc::new(RwLock::new(HashMap::new())),
            server_info: protocol::create_default_server_info(&config.name, &config.version),
            tools_cache: Arc::new(RwLock::new(Vec::new())),
            running: Arc::new(RwLock::new(false)),
            config: Arc::new(RwLock::new(config)),
            execution_results: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 获取工具注册表
    pub fn get_tool_registry(&self) -> Arc<RwLock<HashMap<String, ToolInfo>>> {
        self.tool_registry.clone()
    }

    /// 启动 STDIO 服务器
    pub async fn start_stdio(&mut self) -> Result<()> {
        // 检查是否已经在运行
        let running = self.running.read().await;
        if *running {
            return Err(anyhow!("MCP server is already running"));
        }
        drop(running);

        // 检查配置是否允许STDIO
        let config = self.config.read().await;
        if !config.enable_stdio {
            return Err(anyhow!("STDIO server is disabled in configuration"));
        }
        drop(config);

        // STDIO服务器功能暂未完全实现，等待rmcp API稳定
        info!("STDIO server feature is under development");
        
        // 标记服务器为运行状态（模拟）
        let mut running = self.running.write().await;
        *running = true;

        info!("MCP STDIO server placeholder started");

        Ok(())
    }

    /// 启动HTTP服务器
    pub async fn start_http(&mut self) -> Result<()> {
        // 检查是否已经在运行
        let running = self.running.read().await;
        if *running {
            return Err(anyhow!("MCP server is already running"));
        }
        drop(running);

        // 检查配置是否允许HTTP
        let config = self.config.read().await;
        if !config.enable_http {
            return Err(anyhow!("HTTP server is disabled in configuration"));
        }
        let port = config.http_port;
        drop(config);

        // HTTP服务器功能暂未完全实现
        info!("HTTP server feature is under development, port: {}", port);
        
        // 标记服务器为运行状态（模拟）
        let mut running = self.running.write().await;
        *running = true;

        info!("MCP HTTP server placeholder started on port {}", port);

        Ok(())
    }

    /// 启动WebSocket服务器
    pub async fn start_websocket(&mut self) -> Result<()> {
        // 检查是否已经在运行
        let running = self.running.read().await;
        if *running {
            return Err(anyhow!("MCP server is already running"));
        }
        drop(running);

        // 检查配置是否允许WebSocket
        let config = self.config.read().await;
        if !config.enable_websocket {
            return Err(anyhow!("WebSocket server is disabled in configuration"));
        }
        let port = config.websocket_port;
        drop(config);

        // WebSocket服务器功能暂未完全实现
        info!("WebSocket server feature is under development, port: {}", port);
        
        // 标记服务器为运行状态（模拟）
        let mut running = self.running.write().await;
        *running = true;

        info!("MCP WebSocket server placeholder started on port {}", port);

        Ok(())
    }

    /// 将内部工具转换为RMCP工具格式
    async fn list_tools_as_rmcp(&self) -> Result<Vec<Tool>> {
        let registry = self.tool_registry.read().await;
        let mut rmcp_tools = Vec::new();
        
        // 简化工具列表处理
        for (tool_name, tool_info) in registry.iter() {
            // 检查工具是否在白名单中
            let config = self.config.read().await;
            if let Some(ref whitelist) = config.tools_whitelist {
                if !whitelist.contains(tool_name) {
                    continue;
                }
            }

            // 检查工具是否在黑名单中
            if let Some(ref blacklist) = config.tools_blacklist {
                if blacklist.contains(tool_name) {
                    continue;
                }
            }
            drop(config);

            // 工具转换已简化
            let rmcp_tool = rmcp::model::Tool {
                name: tool_info.name.clone().into(),
                description: Some(tool_info.description.clone().into()),
                input_schema: std::sync::Arc::new(serde_json::Map::new()),
                output_schema: None,
                annotations: None,
            };
            rmcp_tools.push(rmcp_tool);
        }

        // 更新缓存
        let mut tools_cache = self.tools_cache.write().await;
        *tools_cache = rmcp_tools.clone();

        Ok(rmcp_tools)
    }

    /// 获取所有工具
    pub async fn list_tools(&self) -> Vec<String> {
        let registry = self.tool_registry.read().await;
        registry.keys().cloned().collect()
    }

    /// 获取工具详情
    pub async fn get_tool_details(&self) -> Result<Vec<ToolInfo>> {
        let registry = self.tool_registry.read().await;
        Ok(registry.values().cloned().collect())
    }



    /// 执行工具
    pub async fn execute_tool(&self, tool_name: &str, parameters: Value) -> Result<Value> {
        let registry = self.tool_registry.read().await;

        // 检查工具是否存在
        if !registry.contains_key(tool_name) {
            return Err(anyhow!("Tool does not exist: {}", tool_name));
        }

        // 检查工具是否在白名单中
        let config = self.config.read().await;
        if let Some(ref whitelist) = config.tools_whitelist {
            if !whitelist.contains(&tool_name.to_string()) {
                return Err(anyhow!("Tool is not in whitelist: {}", tool_name));
            }
        }

        // 检查工具是否在黑名单中
        if let Some(ref blacklist) = config.tools_blacklist {
            if blacklist.contains(&tool_name.to_string()) {
                return Err(anyhow!("Tool is in blacklist: {}", tool_name));
            }
        }
        drop(config);

        // 简化执行逻辑
        info!("Executing tool: {} with parameters: {:?}", tool_name, parameters);
        Ok(serde_json::json!({"status": "success", "tool": tool_name}))
    }

    /// 获取服务器状态
    pub async fn is_running(&self) -> bool {
        let running = self.running.read().await;
        *running
    }

    /// 停止服务器
    pub async fn stop(&mut self) -> Result<()> {
        // 标记服务器为停止状态
        let mut running = self.running.write().await;
        *running = false;

        tracing::info!("MCP server stopped");
        Ok(())
    }

    /// 设置服务器信息
    pub fn set_server_info(&mut self, name: String, version: String) {
        let mut server_info = self.server_info.clone();
        server_info.server_info.name = name.clone();
        server_info.server_info.version = version.clone();
        self.server_info = server_info;

        // 更新配置
        let config_clone = self.config.clone();
        tokio::spawn(async move {
            let mut config = config_clone.write().await;
            config.name = name;
            config.version = version;
        });
    }

    /// 更新服务器配置
    pub async fn update_config(&mut self, config: ServerConfig) -> Result<()> {
        // 检查服务器是否正在运行
        let running = self.running.read().await;
        if *running {
            return Err(anyhow!(
                "Cannot update configuration while server is running"
            ));
        }
        drop(running);

        // 更新配置
        let mut current_config = self.config.write().await;
        *current_config = config.clone();

        // 更新服务器信息
        let mut server_info = self.server_info.clone();
        server_info.server_info.name = config.name.clone();
        server_info.server_info.version = config.version.clone();
        if let Some(desc) = config.description {
            server_info.instructions = Some(desc);
        }
        self.server_info = server_info;

        tracing::info!("MCP server configuration updated");
        Ok(())
    }

    /// 获取服务器配置
    pub async fn get_config(&self) -> ServerConfig {
        let config = self.config.read().await;
        config.clone()
    }

    /// 获取执行结果（异步版本，避免GUI阻塞）
    pub async fn get_execution_result_async(&self, execution_id: &Uuid) -> Option<ToolExecutionResult> {
        let results = self.execution_results.read().await;
        results.get(execution_id).cloned()
    }

    /// 获取执行结果（同步版本，使用非阻塞try_read）
    pub fn get_execution_result(&self, execution_id: &Uuid) -> Option<ToolExecutionResult> {
        // 使用try_read避免阻塞GUI
        if let Ok(results) = self.execution_results.try_read() {
            results.get(execution_id).cloned()
        } else {
            // 如果锁被占用，返回None而不是阻塞
            tracing::warn!("Execution results lock is busy, returning None for execution_id: {}", execution_id);
            None
        }
    }
}

impl Default for SentinelMcpServer {
    fn default() -> Self {
        Self::new()
    }
}

/// MCP 服务器管理器
#[derive(Debug)]
pub struct McpServerManager {
    server: Arc<RwLock<SentinelMcpServer>>,
    config_path: PathBuf,
}

impl McpServerManager {
    pub fn new() -> Self {
        let config_path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sentinel-ai")
            .join("mcp_server_config.json");

        Self {
            server: Arc::new(RwLock::new(SentinelMcpServer::new())),
            config_path,
        }
    }

    /// 启动STDIO服务器
    pub async fn start_stdio(&self) -> Result<()> {
        let mut server = self.server.write().await;
        server.start_stdio().await
    }

    /// 停止服务器
    pub async fn stop(&self) -> Result<()> {
        let mut server = self.server.write().await;
        server.stop().await
    }

    /// 获取服务器实例
    pub async fn get_server(&self) -> Arc<RwLock<SentinelMcpServer>> {
        self.server.clone()
    }

    /// 检查服务器是否运行
    pub async fn is_running(&self) -> bool {
        let server = self.server.read().await;
        server.is_running().await
    }

    /// 获取所有工具
    pub async fn list_tools(&self) -> Vec<String> {
        let server = self.server.read().await;
        server.list_tools().await
    }

    /// 获取工具详情
    pub async fn get_tool_details(&self) -> Result<Vec<ToolInfo>> {
        let server = self.server.read().await;
        server.get_tool_details().await
    }

    /// 执行工具
    pub async fn execute_tool(&self, tool_name: &str, parameters: Value) -> Result<Value> {
        let server = self.server.read().await;
        server.execute_tool(tool_name, parameters).await
    }

    /// 加载配置
    pub async fn load_config(&self) -> Result<()> {
        // 检查配置文件是否存在
        if !self.config_path.exists() {
            // 如果不存在，创建默认配置并保存
            return self.save_config().await;
        }

        // 读取配置文件
        let config_str = std::fs::read_to_string(&self.config_path)
            .map_err(|e| anyhow!("Failed to read configuration file: {}", e))?;

        // 解析配置
        let config: ServerConfig = serde_json::from_str(&config_str)
            .map_err(|e| anyhow!("Failed to parse configuration file: {}", e))?;

        // 更新服务器配置
        let mut server = self.server.write().await;
        server.update_config(config).await?;

        tracing::debug!(
            "Loaded MCP server configuration from {:?}",
            self.config_path
        );
        Ok(())
    }

    /// 保存配置
    pub async fn save_config(&self) -> Result<()> {
        // 确保配置目录存在
        if let Some(parent) = self.config_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| anyhow!("Failed to create configuration directory: {}", e))?;
            }
        }

        // 获取当前配置
        let server = self.server.read().await;
        let config = server.get_config().await;

        // 序列化配置
        let config_str = serde_json::to_string_pretty(&config)
            .map_err(|e| anyhow!("Failed to serialize configuration: {}", e))?;

        // 写入配置文件
        let mut file = std::fs::File::create(&self.config_path)
            .map_err(|e| anyhow!("Failed to create configuration file: {}", e))?;
        file.write_all(config_str.as_bytes())
            .map_err(|e| anyhow!("Failed to write configuration file: {}", e))?;

        tracing::info!("Saved MCP server configuration to {:?}", self.config_path);
        Ok(())
    }

    /// 更新配置
    pub async fn update_config(&self, config: ServerConfig) -> Result<()> {
        // 更新服务器配置
        let mut server = self.server.write().await;
        server.update_config(config).await?;

        // 保存配置
        drop(server);
        self.save_config().await?;

        Ok(())
    }

    pub async fn start_child_process(&self, command: &str, args: &Vec<&str>) -> Result<()> {
        let server_clone = self.server.read().await.clone();
        let mut cmd = tokio::process::Command::new(command);
        for arg in args {
            cmd.arg(arg);
        }
        let transport = rmcp::transport::TokioChildProcess::new(cmd)?;
        let service = server_clone.serve(transport);
        service.await?;
        Ok(())
    }

    pub async fn add_server(&self, _config: ServerConfig) -> Result<String> {
        // Placeholder implementation
        Ok("".to_string())
    }



    pub async fn remove_server(&self, _connection_id: &str) -> Result<()> {
        // TODO: Implement server removal logic
        tracing::warn!("remove_server is not yet implemented.");
        Ok(())
    }

    pub async fn start_default_server(&self) -> Result<()> {
        // TODO: Implement default server startup logic
        tracing::info!("Starting default servers (not yet implemented).");
        self.start_stdio().await
    }
}

impl Default for McpServerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_lifecycle() {
        let manager = McpServerManager::new();

        // 启动服务器
        let result = manager.start_stdio().await;
        assert!(result.is_ok());

        // 检查服务器状态
        let running = manager.is_running().await;
        assert!(running);

        // 停止服务器
        let result = manager.stop().await;
        assert!(result.is_ok());

        // 检查服务器状态
        let running = manager.is_running().await;
        assert!(!running);
    }
}
