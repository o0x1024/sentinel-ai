//! Unified tool system type definitions (migrated)

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

// ===================== MCP related types =====================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    pub server_url: String,
    pub timeout: Duration,
    pub max_retries: u32,
    pub auth_token: Option<String>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerCapabilities {
    pub tools: bool,
    pub resources: bool,
    pub prompts: bool,
    pub logging: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransportConfig {
    Stdio,
    WebSocket { url: String },
    Http { base_url: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub version: String,
    pub description: String,
    pub transport: TransportConfig,
    pub capabilities: ServerCapabilities,
    pub tools: Vec<ToolInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolCategory {
    NetworkScanning,
    VulnerabilityScanning,
    ServiceDetection,
    CodeAnalysis,
    DataProcessing,
    SystemUtility,
    Reconnaissance,
    Scanning,
    Exploitation,
    PostExploitation,
    Reporting,
    Database,
    Analysis,
    Utility,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterType {
    String,
    Number,
    Boolean,
    Array,
    Object,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDefinition {
    pub name: String,
    pub param_type: ParameterType,
    pub description: String,
    pub required: bool,
    pub default_value: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameters {
    pub parameters: Vec<ParameterDefinition>,
    pub schema: serde_json::Value,
    pub required: Vec<String>,
    pub optional: Vec<String>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionParams {
    pub inputs: HashMap<String, Value>,
    pub context: HashMap<String, Value>,
    pub timeout: Option<Duration>,
    pub execution_id: Option<Uuid>,
}

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

// ===================== Manager config and stats =====================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolManagerConfig {
    pub max_concurrent_executions: usize,
    pub default_timeout: Duration,
    pub log_executions: bool,
}

impl Default for ToolManagerConfig {
    fn default() -> Self {
        Self { max_concurrent_executions: 10, default_timeout: Duration::from_secs(180), log_executions: true }
    }
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionRequest {
    pub tool_name: String,
    pub params: ToolExecutionParams,
    pub priority: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionRecord {
    pub execution_id: Uuid,
    pub tool_name: String,
    pub params: ToolExecutionParams,
    pub result: Option<ToolExecutionResult>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSearchQuery {
    pub query: String,
    pub category: Option<ToolCategory>,
    pub tags: Vec<String>,
    pub available_only: bool,
    pub installed_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSearchResult {
    pub tools: Vec<ToolInfo>,
    pub total_count: usize,
    pub query: ToolSearchQuery,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BatchExecutionMode {
    Parallel,
    Sequential,
    Pipeline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchExecutionRequest {
    pub requests: Vec<ToolExecutionRequest>,
    pub mode: BatchExecutionMode,
    pub stop_on_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchExecutionResult {
    pub batch_id: Uuid,
    pub results: Vec<ToolExecutionResult>,
    pub success_count: usize,
    pub failure_count: usize,
    pub total_execution_time_ms: f64,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStatistics {
    pub tool_name: String,
    pub total_executions: usize,
    pub successful_executions: usize,
    pub failed_executions: usize,
    pub average_execution_time_ms: u64,
    pub total_execution_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub structured_data: Option<Value>,
    pub artifacts: Vec<ExecutionArtifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionArtifact {
    pub name: String,
    pub path: String,
    pub content_type: String,
    pub size: f64,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration: Option<f64>,
    pub resource_usage: ResourceUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_time: f32,
    pub memory_peak: u32,
    pub network_requests: u8,
    pub disk_io: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToolSource {
    Builtin,
    External,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedToolCall {
    pub id: String,
    pub tool_name: String,
    pub parameters: HashMap<String, Value>,
    pub timeout: Option<Duration>,
    pub context: HashMap<String, Value>,
    pub retry_count: u32,
}

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

#[async_trait]
pub trait FrameworkToolAdapter: Send + Sync + std::fmt::Debug {
    fn adapter_name(&self) -> &str;
    async fn execute_tool(&self, call: UnifiedToolCall) -> Result<UnifiedToolResult>;
    async fn execute_tools_batch(&self, calls: Vec<UnifiedToolCall>) -> Vec<Result<UnifiedToolResult>>;
    async fn list_available_tools(&self) -> Vec<String>;
    async fn get_tool_info(&self, tool_name: &str) -> Option<ToolInfo>;
    async fn is_tool_available(&self, tool_name: &str) -> bool;
    async fn validate_tool_call(&self, tool_name: &str, call: &UnifiedToolCall) -> Result<()>;
}

#[async_trait]
pub trait EngineToolAdapter: Send + Sync + std::fmt::Debug {
    async fn execute_tool(&self, call: UnifiedToolCall) -> Result<UnifiedToolResult>;
    async fn list_available_tools(&self) -> Vec<String>;
    async fn get_tool_info(&self, tool_name: &str) -> Option<ToolInfo>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FrameworkType {
    PlanAndExecute,
    ReWOO,
    LLMCompiler,
    React,
}

impl std::fmt::Display for FrameworkType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameworkType::PlanAndExecute => write!(f, "plan_and_execute"),
            FrameworkType::ReWOO => write!(f, "rewoo"),
            FrameworkType::LLMCompiler => write!(f, "llm_compiler"),
            FrameworkType::React => write!(f, "react"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterConfig {
    pub framework_type: FrameworkType,
    pub cache_enabled: bool,
    pub max_concurrent_calls: usize,
    pub default_timeout: Duration,
    pub retry_policy: RetryPolicy,
}

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

#[async_trait]
pub trait UnifiedTool: Send + Sync + std::fmt::Debug {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn category(&self) -> ToolCategory;
    fn parameters(&self) -> &ToolParameters;
    fn metadata(&self) -> &ToolMetadata;
    async fn is_available(&self) -> bool { true }
    async fn is_installed(&self) -> bool { true }
    fn validate_params(&self, params: &ToolExecutionParams) -> Result<()> {
        for param_def in &self.parameters().parameters {
            if param_def.required && !params.inputs.contains_key(&param_def.name) {
                return Err(anyhow::anyhow!("Missing required parameter: {}", param_def.name));
            }
        }
        Ok(())
    }
    async fn execute(&self, params: ToolExecutionParams) -> Result<ToolExecutionResult>;
}

#[async_trait]
pub trait ToolProvider: Send + Sync + std::fmt::Debug {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn get_tools(&self) -> Result<Vec<Arc<dyn UnifiedTool>>>;
    async fn get_tool(&self, name: &str) -> Result<Option<Arc<dyn UnifiedTool>>>;
    async fn refresh(&self) -> Result<()>;
    async fn is_available(&self) -> bool { true }
}


