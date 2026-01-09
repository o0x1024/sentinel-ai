pub mod ai;
pub mod asset;
pub mod attachment;
pub mod database;
pub mod dictionary;
pub mod mcp;
pub mod scan;
pub mod scan_session;
pub mod vulnerability;
pub mod security_testing;
pub mod workflow_graph;

// 通用类型定义
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 通用响应结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
            timestamp: Utc::now(),
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            message: Some(message),
            timestamp: Utc::now(),
        }
    }
}

/// 分页信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    pub page: u32,
    pub per_page: u32,
    pub total: f64,
    pub total_pages: u32,
}

/// 分页响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: Pagination,
}
