use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// MCP工具信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub icon: Option<String>,
    pub category: String,
    pub author: String,
    pub status: McpToolStatus,
    pub config: Value,
    pub command: Option<String>,
    pub executable_path: Option<String>,
    pub install_path: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub rating: Option<f32>,
    pub downloads: Option<String>,
}

/// 工具状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpToolStatus {
    Running,
    Stopped,
    Error,
    Installing,
    Updating,
}

/// 工具执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpExecutionResult {
    pub execution_id: String,
    pub tool_id: String,
    pub status: ExecutionStatus,
    pub result: Option<Value>,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub logs: Vec<String>,
}

/// 执行状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// 工具商店
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolStore {
    pub tools: Vec<McpStoreItem>,
    pub categories: Vec<String>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
}

/// 商店工具项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpStoreItem {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub icon: Option<String>,
    pub category: String,
    pub author: String,
    pub rating: f32,
    pub downloads: String,
    pub tags: Vec<String>,
    pub repository_url: Option<String>,
    pub documentation_url: Option<String>,
    pub license: Option<String>,
    pub size: Option<String>,
    pub dependencies: Vec<String>,
    pub platforms: Vec<String>,
}

/// 工具安装请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolInstallRequest {
    pub install_type: InstallType,
    pub source: String,
    pub name: Option<String>,
    pub version: Option<String>,
    pub config: Option<Value>,
}

/// 安装类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstallType {
    Url,
    File,
    Registry,
    Github,
    Custom,
}

/// MCP消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpMessage {
    pub id: String,
    pub method: String,
    pub params: Option<Value>,
    pub result: Option<Value>,
    pub error: Option<McpError>,
}

/// MCP错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

/// 工具配置模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolConfigTemplate {
    pub name: String,
    pub description: String,
    pub config_type: String,
    pub default_value: Value,
    pub required: bool,
    pub validation: Option<String>,
}

/// 工具日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpLogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub message: String,
    pub tool_id: String,
    pub execution_id: Option<String>,
}

/// 日志级别
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl McpTool {
    pub fn new(name: String, description: String, category: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            version: "1.0.0".to_string(),
            description,
            icon: None,
            category,
            author: "Unknown".to_string(),
            status: McpToolStatus::Stopped,
            config: Value::Object(serde_json::Map::new()),
            command: None,
            executable_path: None,
            install_path: None,
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            rating: None,
            downloads: None,
        }
    }

    pub fn is_running(&self) -> bool {
        matches!(self.status, McpToolStatus::Running)
    }

    pub fn is_custom(&self) -> bool {
        self.category == "custom"
    }
}

impl McpExecutionResult {
    pub fn new(tool_id: String) -> Self {
        Self {
            execution_id: Uuid::new_v4().to_string(),
            tool_id,
            status: ExecutionStatus::Running,
            result: None,
            error: None,
            started_at: Utc::now(),
            completed_at: None,
            logs: Vec::new(),
        }
    }

    pub fn complete_with_result(&mut self, result: Value) {
        self.status = ExecutionStatus::Completed;
        self.result = Some(result);
        self.completed_at = Some(Utc::now());
    }

    pub fn fail_with_error(&mut self, error: String) {
        self.status = ExecutionStatus::Failed;
        self.error = Some(error);
        self.completed_at = Some(Utc::now());
    }
}
