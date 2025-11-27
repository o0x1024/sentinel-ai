use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskPriority { Low, Normal, High, Critical }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel { Debug, Info, Warn, Error }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub id: String,
    pub description: String,
    pub target: Option<String>,
    pub parameters: HashMap<String, serde_json::Value>,
    pub user_id: String,
    pub priority: TaskPriority,
    pub timeout: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionLog {
    pub level: LogLevel,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecutionResult {
    pub id: String,
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
    pub resources_used: Vec<String>,
    pub artifacts: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionData {
    pub session_id: String,
    pub task_id: String,
    pub status: String,
    pub agent_name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

