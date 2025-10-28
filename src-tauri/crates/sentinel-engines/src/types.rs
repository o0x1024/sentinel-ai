//! Engine types and structures

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use sentinel_core::{ExecutionStatus, Id, Timestamp};

/// 执行计划
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub id: Id,
    pub name: String,
    pub description: Option<String>,
    pub steps: Vec<ExecutionStep>,
    pub metadata: PlanMetadata,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// 执行步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    pub id: Id,
    pub name: String,
    pub description: Option<String>,
    pub step_type: String,
    pub parameters: serde_json::Value,
    pub dependencies: Vec<Id>,
    pub status: ExecutionStatus,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// 计划元数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlanMetadata {
    pub tags: Vec<String>,
    pub priority: String,
    pub estimated_duration: Option<u64>,
    pub properties: std::collections::HashMap<String, serde_json::Value>,
}

/// 执行会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSession {
    pub id: Id,
    pub plan_id: Id,
    pub name: String,
    pub status: ExecutionStatus,
    pub context: ExecutionContext,
    pub metadata: SessionMetadata,
    pub started_at: Option<Timestamp>,
    pub completed_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// 执行上下文
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionContext {
    pub variables: std::collections::HashMap<String, serde_json::Value>,
    pub environment: String,
    pub user_id: Option<String>,
}

/// 会话元数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionMetadata {
    pub tags: Vec<String>,
    pub properties: std::collections::HashMap<String, serde_json::Value>,
}