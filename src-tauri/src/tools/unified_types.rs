//! 统一工具系统类型定义

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

// ============================================================================
// 核心类型定义
// ============================================================================

/// 工具分类
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolCategory {
    NetworkScanning,
    VulnerabilityScanning,
    ServiceDetection,
    CodeAnalysis,
    DataProcessing,
    SystemUtility,
    // 工具类别
    Reconnaissance,   // 侦察
    Scanning,         // 扫描
    Exploitation,     // 利用
    PostExploitation, // 后渗透
    Reporting,        // 报告
    Database,         // 数据库
    Analysis,         // 分析
    Utility,          // 实用工具
    Custom(String),
}

impl std::fmt::Display for ToolCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolCategory::NetworkScanning => write!(f, "network_scanning"),
            ToolCategory::VulnerabilityScanning => write!(f, "vulnerability_scanning"),
            ToolCategory::ServiceDetection => write!(f, "service_detection"),
            ToolCategory::CodeAnalysis => write!(f, "code_analysis"),
            ToolCategory::DataProcessing => write!(f, "data_processing"),
            ToolCategory::SystemUtility => write!(f, "system_utility"),
            // 工具类别
            ToolCategory::Reconnaissance => write!(f, "reconnaissance"),
            ToolCategory::Scanning => write!(f, "scanning"),
            ToolCategory::Exploitation => write!(f, "exploitation"),
            ToolCategory::PostExploitation => write!(f, "post_exploitation"),
            ToolCategory::Reporting => write!(f, "reporting"),
            ToolCategory::Database => write!(f, "database"),
            ToolCategory::Analysis => write!(f, "analysis"),
            ToolCategory::Utility => write!(f, "utility"),
            ToolCategory::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// 工具参数类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterType {
    String,
    Number,
    Boolean,
    Array,
    Object,
}

/// 工具参数定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDefinition {
    pub name: String,
    pub param_type: ParameterType,
    pub description: String,
    pub required: bool,
    pub default_value: Option<Value>,
}

/// 工具参数集合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameters {
    pub parameters: Vec<ParameterDefinition>,
    pub schema: serde_json::Value,
    pub required: Vec<String>,
    pub optional: Vec<String>,
}

/// 工具元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub author: String,
    pub version: String,
    pub license: String,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub tags: Vec<String>,
    pub install_command: Option<String>,
    pub requirements: Vec<String>,
}

/// 工具执行参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionParams {
    pub inputs: HashMap<String, Value>,
    pub context: HashMap<String, Value>,
    pub timeout: Option<Duration>,
    pub execution_id: Option<Uuid>,
}

/// 工具执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionResult {
    pub execution_id: Uuid,
    pub tool_name: String,
    pub tool_id: String,
    pub success: bool,
    pub output: Value,
    pub error: Option<String>,
    pub execution_time_ms: u64,
    pub metadata: HashMap<String, Value>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: ExecutionStatus,
}

/// 工具信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub category: ToolCategory,
    pub parameters: ToolParameters,
    pub metadata: ToolMetadata,
    pub available: bool,
    pub installed: bool,
    pub source: ToolSource,
}

/// 工具执行请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionRequest {
    pub tool_name: String,
    pub params: ToolExecutionParams,
    pub priority: Option<u8>,
}

/// 工具执行记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionRecord {
    pub execution_id: Uuid,
    pub tool_name: String,
    pub params: ToolExecutionParams,
    pub result: Option<ToolExecutionResult>,
    pub created_at: DateTime<Utc>,
}

/// 工具搜索查询
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSearchQuery {
    pub query: String,
    pub category: Option<ToolCategory>,
    pub tags: Vec<String>,
    pub available_only: bool,
    pub installed_only: bool,
}

/// 工具搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSearchResult {
    pub tools: Vec<ToolInfo>,
    pub total_count: usize,
    pub query: ToolSearchQuery,
}

/// 批量执行模式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BatchExecutionMode {
    Parallel,
    Sequential,
    Pipeline,
}

/// 批量执行请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchExecutionRequest {
    pub requests: Vec<ToolExecutionRequest>,
    pub mode: BatchExecutionMode,
    pub stop_on_error: bool,
}

/// 批量执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchExecutionResult {
    pub batch_id: Uuid,
    pub results: Vec<ToolExecutionResult>,
    pub success_count: usize,
    pub failure_count: usize,
    pub total_execution_time_ms: u64,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// 工具统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStatistics {
    pub tool_name: String,
    pub total_executions: usize,
    pub successful_executions: usize,
    pub failed_executions: usize,
    pub average_execution_time_ms: f64,
    pub total_execution_time_ms: u64,
}

// ============================================================================
// 核心接口定义
// ============================================================================

/// 统一工具接口
#[async_trait]
pub trait UnifiedTool: Send + Sync + std::fmt::Debug {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn category(&self) -> ToolCategory;
    fn parameters(&self) -> &ToolParameters;
    fn metadata(&self) -> &ToolMetadata;
    
    async fn is_available(&self) -> bool {
        true
    }
    
    async fn is_installed(&self) -> bool {
        true
    }
    
    fn validate_params(&self, params: &ToolExecutionParams) -> Result<()> {
        for param_def in &self.parameters().parameters {
            if param_def.required && !params.inputs.contains_key(&param_def.name) {
                return Err(anyhow!("Missing required parameter: {}", param_def.name));
            }
        }
        Ok(())
    }
    
    async fn execute(&self, params: ToolExecutionParams) -> Result<ToolExecutionResult>;
}

/// 工具提供者接口
#[async_trait]
pub trait ToolProvider: Send + Sync + std::fmt::Debug {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn get_tools(&self) -> Result<Vec<Arc<dyn UnifiedTool>>>;
    async fn get_tool(&self, name: &str) -> Result<Option<Arc<dyn UnifiedTool>>>;
    async fn refresh(&self) -> Result<()>;
    async fn is_available(&self) -> bool {
        true
    }
}

/// 工具管理器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolManagerConfig {
    pub max_concurrent_executions: usize,
    pub default_timeout: Duration,
    pub log_executions: bool,
}

impl Default for ToolManagerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_executions: 10,
            default_timeout: Duration::from_secs(300),
            log_executions: true,
        }
    }
}

// ============================================================================
// MCP相关类型定义
// ============================================================================

/// MCP配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    pub server_url: String,
    pub timeout: Duration,
    pub max_retries: u32,
    pub auth_token: Option<String>,
}

/// MCP连接状态
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

impl std::fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionStatus::Disconnected => write!(f, "Disconnected"),
            ConnectionStatus::Connecting => write!(f, "Connecting"),
            ConnectionStatus::Connected => write!(f, "Connected"),
            ConnectionStatus::Error(msg) => write!(f, "Error: {}", msg),
        }
    }
}

/// MCP服务器能力
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerCapabilities {
    pub tools: bool,
    pub resources: bool,
    pub prompts: bool,
    pub logging: bool,
}

/// 传输配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransportConfig {
    Stdio,
    WebSocket { url: String },
    Http { base_url: String },
}

/// MCP服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub version: String,
    pub description: String,
    pub transport: TransportConfig,
    pub capabilities: ServerCapabilities,
    pub tools: Vec<ToolInfo>,
}

// ============================================================================
// 执行相关类型定义
// ============================================================================

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
    pub structured_data: Option<Value>,
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
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration: Option<u64>, // milliseconds
    pub resource_usage: ResourceUsage,
}

/// 资源使用情况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_time: f32,
    pub memory_peak: u32,
    pub network_requests: u8,
    pub disk_io: u8,
}

/// 工具来源
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToolSource {
    Builtin,  // 内置工具
    External, // 外部工具
}

// ============================================================================
// 框架适配器接口定义
// ============================================================================

/// 统一工具调用请求 (框架无关)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedToolCall {
    pub id: String,
    pub tool_name: String,
    pub parameters: HashMap<String, Value>,
    pub timeout: Option<Duration>,
    pub context: HashMap<String, Value>,
    pub retry_count: u32,
}

/// 统一工具调用结果 (框架无关)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedToolResult {
    pub id: String,
    pub tool_name: String,
    pub success: bool,
    pub output: Value,
    pub error: Option<String>,
    pub execution_time_ms: u64,
    pub metadata: HashMap<String, Value>,
}

/// 框架适配器接口 - 为不同框架提供统一的工具调用抽象
#[async_trait]
pub trait FrameworkToolAdapter: Send + Sync + std::fmt::Debug {
    /// 适配器名称 (Plan&Execute, ReWOO, LLMCompiler)
    fn adapter_name(&self) -> &str;
    
    /// 执行工具调用
    async fn execute_tool(&self, call: UnifiedToolCall) -> Result<UnifiedToolResult>;
    
    /// 批量执行工具调用
    async fn execute_tools_batch(&self, calls: Vec<UnifiedToolCall>) -> Vec<Result<UnifiedToolResult>>;
    
    /// 获取可用工具列表
    async fn list_available_tools(&self) -> Vec<String>;
    
    /// 获取工具信息
    async fn get_tool_info(&self, tool_name: &str) -> Option<ToolInfo>;
    
    /// 检查工具是否可用
    async fn is_tool_available(&self, tool_name: &str) -> bool;
    
    /// 验证工具调用
    async fn validate_tool_call(&self, tool_name: &str, call: &UnifiedToolCall) -> Result<()>;
}

/// 引擎工具适配器接口 (LLM Compiler专用兼容接口)
#[async_trait]
pub trait EngineToolAdapter: Send + Sync + std::fmt::Debug {
    /// 执行工具调用 (LLM Compiler兼容接口)
    async fn execute_tool(&self, call: UnifiedToolCall) -> Result<UnifiedToolResult>;
    
    /// 获取可用工具列表
    async fn list_available_tools(&self) -> Vec<String>;
    
    /// 获取工具信息
    async fn get_tool_info(&self, tool_name: &str) -> Option<ToolInfo>;
}

/// 框架类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FrameworkType {
    PlanAndExecute,
    ReWOO,
    LLMCompiler,
}

impl std::fmt::Display for FrameworkType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameworkType::PlanAndExecute => write!(f, "plan_and_execute"),
            FrameworkType::ReWOO => write!(f, "rewoo"),
            FrameworkType::LLMCompiler => write!(f, "llm_compiler"),
        }
    }
}

/// 适配器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterConfig {
    pub framework_type: FrameworkType,
    pub cache_enabled: bool,
    pub max_concurrent_calls: usize,
    pub default_timeout: Duration,
    pub retry_policy: RetryPolicy,
}

/// 重试策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for AdapterConfig {
    fn default() -> Self {
        Self {
            framework_type: FrameworkType::PlanAndExecute,
            cache_enabled: true,
            max_concurrent_calls: 10,
            default_timeout: Duration::from_secs(300),
            retry_policy: RetryPolicy {
                max_retries: 3,
                base_delay_ms: 1000,
                max_delay_ms: 30000,
                backoff_multiplier: 2.0,
            },
        }
    }
}




