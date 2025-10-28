//! Core types and structures

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// 基础ID类型
pub type Id = Uuid;

/// 时间戳类型
pub type Timestamp = DateTime<Utc>;

/// 通用配置结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub key: String,
    pub value: serde_json::Value,
    pub category: Option<String>,
    pub description: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// 执行状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Paused,
}

/// 优先级
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low,
    Normal,
    High,
    Critical,
}

/// 通用元数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Metadata {
    pub tags: Vec<String>,
    pub properties: std::collections::HashMap<String, serde_json::Value>,
}

/// 分页参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    pub page: u32,
    pub page_size: u32,
    pub total: Option<u64>,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 20,
            total: None,
        }
    }
}
