use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskPriority { Low, Normal, High, Critical }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel { Debug, Info, Warn, Error }

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AgentTask {
    pub id: String,
    pub description: String,
    pub target: Option<String>,
    pub parameters: String, // Store as JSON string for sqlx
    pub user_id: String,
    pub priority: String, // Store as string for sqlx
    pub timeout: Option<i64>, // Store as i64 for sqlx
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SessionLog {
    pub level: String, // Store as string for sqlx
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AgentExecutionResult {
    pub id: String,
    pub success: bool,
    pub data: Option<String>, // Store as JSON string for sqlx
    pub error: Option<String>,
    pub execution_time_ms: i64, // Store as i64 for sqlx
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AgentSessionData {
    pub id: String, // In DB it might be 'id' instead of 'session_id'
    pub task_id: String,
    pub status: String,
    pub agent_name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
