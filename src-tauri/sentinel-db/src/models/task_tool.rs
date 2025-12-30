use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Tool type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolType {
    Plugin,
    McpServer,
    Builtin,
    Workflow,
}

impl ToString for ToolType {
    fn to_string(&self) -> String {
        match self {
            ToolType::Plugin => "plugin".to_string(),
            ToolType::McpServer => "mcp_server".to_string(),
            ToolType::Builtin => "builtin".to_string(),
            ToolType::Workflow => "workflow".to_string(),
        }
    }
}

impl std::str::FromStr for ToolType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "plugin" => Ok(ToolType::Plugin),
            "mcp_server" => Ok(ToolType::McpServer),
            "builtin" => Ok(ToolType::Builtin),
            "workflow" => Ok(ToolType::Workflow),
            _ => Err(format!("Unknown tool type: {}", s)),
        }
    }
}

/// Tool execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ToolExecutionStatus {
    Idle,
    Running,
    Waiting,
    Completed,
    Error,
}

impl ToString for ToolExecutionStatus {
    fn to_string(&self) -> String {
        match self {
            ToolExecutionStatus::Idle => "idle".to_string(),
            ToolExecutionStatus::Running => "running".to_string(),
            ToolExecutionStatus::Waiting => "waiting".to_string(),
            ToolExecutionStatus::Completed => "completed".to_string(),
            ToolExecutionStatus::Error => "error".to_string(),
        }
    }
}

impl std::str::FromStr for ToolExecutionStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "idle" => Ok(ToolExecutionStatus::Idle),
            "running" => Ok(ToolExecutionStatus::Running),
            "waiting" => Ok(ToolExecutionStatus::Waiting),
            "completed" => Ok(ToolExecutionStatus::Completed),
            "error" => Ok(ToolExecutionStatus::Error),
            _ => Err(format!("Unknown tool execution status: {}", s)),
        }
    }
}

/// Active tool information for a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveToolInfo {
    pub log_id: String,
    pub task_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_name: Option<String>,
    pub tool_id: String,
    pub tool_name: String,
    pub tool_type: ToolType,
    pub started_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_params: Option<serde_json::Value>,
}

/// Tool statistics for a task
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolStatistics {
    pub total_executions: i64,
    pub successful_executions: i64,
    pub failed_executions: i64,
    pub total_execution_time: i64,
    pub tools_used: Vec<String>,
}

/// Task tool execution record (aggregated)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskToolExecution {
    pub id: String,
    pub task_id: String,
    pub tool_id: String,
    pub tool_name: String,
    pub tool_type: ToolType,
    pub status: ToolExecutionStatus,
    pub execution_count: i64,
    pub success_count: i64,
    pub error_count: i64,
    pub total_execution_time_ms: i64,
    pub avg_execution_time_ms: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_execution_time: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Individual tool execution log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskToolExecutionLog {
    pub id: String,
    pub task_tool_execution_id: String,
    pub task_id: String,
    pub tool_id: String,
    pub tool_name: String,
    pub tool_type: ToolType,
    pub status: ToolExecutionStatus,
    pub started_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_time_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_params: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Request to create a new tool execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskToolExecutionRequest {
    pub task_id: String,
    pub tool_id: String,
    pub tool_name: String,
    pub tool_type: ToolType,
}

/// Request to update tool execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateToolExecutionStatusRequest {
    pub task_id: String,
    pub tool_id: String,
    pub status: ToolExecutionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_time_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// Execution record for timeline display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub id: String,
    pub tool_name: String,
    pub tool_type: ToolType,
    pub status: ToolExecutionStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub execution_time_ms: Option<i64>,
    pub error_message: Option<String>,
}

impl From<TaskToolExecutionLog> for ExecutionRecord {
    fn from(log: TaskToolExecutionLog) -> Self {
        ExecutionRecord {
            id: log.id,
            tool_name: log.tool_name,
            tool_type: log.tool_type,
            status: log.status,
            started_at: log.started_at,
            completed_at: log.completed_at,
            execution_time_ms: log.execution_time_ms,
            error_message: log.error_message,
        }
    }
}
