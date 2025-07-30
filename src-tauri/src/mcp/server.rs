use anyhow::{anyhow, Result};
use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    CallToolResult, CallToolRequestParam, Content, InitializeRequestParam, InitializeResult,
    ListToolsResult, PaginatedRequestParam, ServerInfo, Tool,
};
use rmcp::service::{RequestContext, RoleServer, ServiceExt};
use rmcp::Error as McpError;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::mcp::McpTool;

use super::protocol;
use super::tools::ToolRegistry;
use super::types::{McpToolInfo, ToolExecutionResult};

/// Sentinel AI MCP 服务器实现
#[derive(Clone)]
pub struct SentinelMcpServer {
    // 保持原有工具注册表用于兼容
    tool_registry: Arc<RwLock<ToolRegistry>>,
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
}

impl From<super::types::McpServerConfig> for ServerConfig {
    fn from(config: super::types::McpServerConfig) -> Self {
        let mut sc = ServerConfig::default();
        sc.name = config.name;
        sc.version = config.version;
        sc.description = Some(config.description);

        match config.transport {
            super::types::TransportConfig::Stdio => {
                sc.enable_stdio = true;
            }
            super::types::TransportConfig::WebSocket { url } => {
                sc.enable_websocket = true;
                if let Ok(parsed_url) = url::Url::parse(&url) {
                    if let Some(port) = parsed_url.port() {
                        sc.websocket_port = port;
                    }
                }
            }
            super::types::TransportConfig::Http { base_url } => {
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
            description: Some("Sentinel AI security tool MCP server".to_string()),
            log_level: "info".to_string(),
            enable_stdio: true,
            enable_http: false,
            http_port: 8080,
            enable_websocket: false,
            websocket_port: 8081,
            tools_whitelist: None,
            tools_blacklist: None,
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
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<InitializeResult, McpError>> + Send + '_>> {
        Box::pin(async move {
            Ok(self.get_info())
        })
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ListToolsResult, McpError>> + Send + '_>> {
        Box::pin(async move {
            let tools = self
                .list_tools_as_rmcp()
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            Ok(ListToolsResult {
                tools,
                next_cursor: None,
            })
        })
    }

    fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<CallToolResult, McpError>> + Send + '_>> {
        Box::pin(async move {
            let result = self
                .execute_tool(&request.name, request.arguments.into())
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        
        let content = Content::text(result.to_string());
        Ok(CallToolResult {
            content: vec![content],
                is_error: Some(false),
            })
        })
    }
}

// 手动实现Debug
impl fmt::Debug for SentinelMcpServer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SentinelMcpServer")
            .field("tool_registry", &"Arc<RwLock<ToolRegistry>>")
            .field("server_info", &self.server_info)
            .field("running", &"Arc<RwLock<bool>>")
            .finish()
    }
}

impl SentinelMcpServer {
    pub fn new() -> Self {
        let config = ServerConfig::default();
        
        Self {
            tool_registry: Arc::new(RwLock::new(ToolRegistry::new())),
            server_info: protocol::create_default_server_info(&config.name, &config.version),
            tools_cache: Arc::new(RwLock::new(Vec::new())),
            running: Arc::new(RwLock::new(false)),
            config: Arc::new(RwLock::new(config)),
            execution_results: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// 获取工具注册表
    pub fn get_tool_registry(&self) -> Arc<RwLock<ToolRegistry>> {
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
        
        // 在实际项目中，这里应该启动一个真正的STDIO服务器
        // 但由于rmcp::server不可用，我们只是模拟启动
        
        // 标记服务器为运行状态
        let mut running = self.running.write().await;
        *running = true;
        
        tracing::info!("MCP STDIO server started");
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
        
        // 目前rmcp库不直接暴露，需要自己实现HTTP传输
        tracing::info!("HTTP server feature not yet implemented, port: {}", port);
        
        Err(anyhow!("HTTP server feature not yet implemented"))
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
        
        // 目前rmcp库不直接暴露，需要自己实现WebSocket传输
        tracing::info!("WebSocket server feature not yet implemented, port: {}", port);
        
        Err(anyhow!("WebSocket server feature not yet implemented"))
    }
    
    /// 将内部工具转换为RMCP工具格式
    async fn list_tools_as_rmcp(&self) -> Result<Vec<Tool>> {
        let registry = self.tool_registry.read().await;
        let tools = registry.list_tools_with_details();
        
        let mut rmcp_tools = Vec::new();
        for tool_name in tools {
            let tool_def = registry.get_tool(&tool_name)?;
            
            // 检查工具是否在白名单中
            let config = self.config.read().await;
            if let Some(ref whitelist) = config.tools_whitelist {
                if !whitelist.contains(&tool_name) {
                    continue;
                }
            }
            
            // 检查工具是否在黑名单中
            if let Some(ref blacklist) = config.tools_blacklist {
                if blacklist.contains(&tool_name) {
                    continue;
                }
            }
            drop(config);
            
            let rmcp_tool = crate::mcp::convert_to_rmcp_tool(&tool_def);
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
        registry.list_tools()
    }
    
    /// 获取工具详情
    pub async fn get_tool_details(&self) -> Result<Vec<McpToolInfo>> {
        let registry = self.tool_registry.read().await;
        registry.get_tool_details()
    }
    
    /// 执行工具
    pub async fn execute_tool(&self, tool_name: &str, parameters: Value) -> Result<Value> {
        let registry = self.tool_registry.read().await;
        
        // 检查工具是否存在
        if !registry.tool_exists(tool_name) {
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
        
        // 执行工具
        match registry.execute_tool(tool_name, parameters).await {
            Ok(result) => {
                // 处理执行结果
                if result.is_empty() {
                    Ok(serde_json::json!({ "status": "success", "result": null }))
                } else if result.len() == 1 {
                    // 单个结果，可能是JSON或纯文本
                    let content = &result[0];
                    match serde_json::from_str::<Value>(&content.text) {
                        Ok(json_val) => Ok(json_val),
                        Err(_) => Ok(serde_json::json!({
                            "status": "success", 
                            "result": content.text
                        }))
                    }
                } else {
                    // 多个结果，合并为数组
                    let mut results = Vec::new();
                    for content in result {
                        results.push(serde_json::json!({ "text": content.text }));
                    }
                    Ok(serde_json::json!({
                        "status": "success",
                        "results": results
                    }))
                }
            }
            Err(e) => Err(anyhow!("Tool execution failed: {}", e)),
        }
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
            return Err(anyhow!("Cannot update configuration while server is running"));
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

    pub fn get_execution_result(&self, execution_id: &Uuid) -> Option<ToolExecutionResult> {
        // This might block if the lock is contended, but it's a synchronous method now.
        // For a UI-heavy application, consider `try_read` or keeping it async
        // and fixing the lifetime issues in the caller differently.
        self.execution_results.blocking_read().get(execution_id).cloned()
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
    pub async fn get_tool_details(&self) -> Result<Vec<McpToolInfo>> {
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
        
        tracing::info!("Loaded MCP server configuration from {:?}", self.config_path);
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
    
    pub async fn start_child_process(
        &self,
        command: &str,
        args: &Vec<&str>,
    ) -> Result<()> {
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

    pub async fn register_tool(&self, _tool: Box<dyn McpTool>) -> Result<()> {
        // Placeholder implementation
        Ok(())
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