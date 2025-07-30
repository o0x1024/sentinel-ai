use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

/// 工具分类
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToolCategory {
    Reconnaissance,    // 侦察
    Scanning,         // 扫描
    Exploitation,     // 利用
    PostExploitation, // 后渗透
    Reporting,        // 报告
    Database,         // 数据库
    Analysis,         // 分析
    Utility,          // 实用工具
}

/// 工具参数定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameters {
    pub schema: serde_json::Value,
    pub required: Vec<String>,
    pub optional: Vec<String>,
}

/// 工具元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub author: String,
    pub license: String,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub tags: Vec<String>,
    pub install_command: Option<String>,
    pub requirements: Vec<String>,
}

/// MCP工具信息（用于UI显示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub category: ToolCategory,
    pub parameters: ToolParameters,
    pub metadata: ToolMetadata,
}

/// 工具执行请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionRequest {
    pub tool_id: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub timeout: Option<u64>,
    pub priority: ExecutionPriority,
}

/// 执行优先级
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// 工具执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionResult {
    pub execution_id: Uuid,
    pub tool_id: String,
    pub status: ExecutionStatus,
    pub output: ExecutionOutput,
    pub metadata: ExecutionMetadata,
}

/// 执行状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Timeout,
}

/// 执行输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub structured_data: Option<serde_json::Value>,
    pub artifacts: Vec<ExecutionArtifact>,
}

/// 执行产物
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionArtifact {
    pub name: String,
    pub path: String,
    pub content_type: String,
    pub size: u64,
    pub checksum: String,
}

/// 执行元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub duration: Option<u64>, // milliseconds
    pub resource_usage: ResourceUsage,
}

/// 资源使用情况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_percent: Option<f64>,
    pub memory_mb: Option<u64>,
    pub disk_io_mb: Option<u64>,
    pub network_io_mb: Option<u64>,
}

/// 传输配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransportConfig {
    Stdio,
    WebSocket { url: String },
    Http { base_url: String },
}

/// 批量执行请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchExecutionRequest {
    pub requests: Vec<ToolExecutionRequest>,
    pub mode: BatchMode,
    pub max_concurrent: Option<u32>,
}

/// 批量执行模式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BatchMode {
    Parallel,     // 并行执行
    Sequential,   // 顺序执行
    Pipeline,     // 管道执行（前一个的输出作为后一个的输入）
}

/// 执行进度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionProgress {
    pub execution_id: Uuid,
    pub tool_id: String,
    pub progress_percent: f64,
    pub current_step: String,
    pub total_steps: Option<u32>,
    pub estimated_remaining_ms: Option<u64>,
}

impl Default for ExecutionPriority {
    fn default() -> Self {
        Self::Normal
    }
}

impl Default for ToolCategory {
    fn default() -> Self {
        Self::Utility
    }
}

/// MCP连接状态
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

/// 工具发现结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDiscoveryResult {
    pub tools: Vec<McpToolInfo>,
    pub total_count: usize,
    pub categories: Vec<ToolCategory>,
}

/// MCP服务器能力
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerCapabilities {
    pub tools: bool,
    pub resources: bool,
    pub prompts: bool,
    pub logging: bool,
}

/// MCP 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub version: String,
    pub description: String,
    pub transport: TransportConfig,
    pub capabilities: ServerCapabilities,
    pub tools: Vec<McpToolInfo>,
}

// 以下是内部MCP工具实现的类型定义

/// 工具定义
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub category: ToolCategory,
    pub input_schema: Value,
    pub metadata: ToolMetadata,
}

/// 工具输入
pub struct ToolInput {
    pub arguments: Value,
}

/// 工具内容
pub struct ToolContent {
    pub text: String,
}

/// 工具调用错误
pub type CallToolError = anyhow::Error;

/// 工具调用结果
pub type CallToolResult = Result<Vec<ToolContent>, CallToolError>;

/// MCP工具trait
#[async_trait]
pub trait McpTool: Send + Sync {
    /// 获取工具定义
    fn definition(&self) -> ToolDefinition;
    
    /// 调用工具
    async fn call(&self, input: ToolInput) -> CallToolResult;
} 