//! 代理配置测试模块 - DISABLED (ai_adapter removed)

use tauri::State;
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use crate::services::database::DatabaseService;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProxyTestResult {
    pub success: bool,
    pub message: String,
    pub proxy_config: Option<serde_json::Value>, // Simplified
    pub response_time_ms: Option<f64>,
}

/// 测试代理配置动态更新功能 - DISABLED
#[tauri::command]
pub async fn test_proxy_dynamic_update(
    _db: State<'_, Arc<DatabaseService>>,
) -> Result<ProxyTestResult, String> {
    Ok(ProxyTestResult {
        success: false,
        message: "Proxy testing disabled - ai_adapter removed".to_string(),
        proxy_config: None,
        response_time_ms: None,
    })
}

/// 测试代理连接 - DISABLED
#[tauri::command]
pub async fn test_proxy_connection(
    _proxy_host: String,
    _proxy_port: u16,
    _proxy_username: Option<String>,
    _proxy_password: Option<String>,
    _test_url: Option<String>,
) -> Result<ProxyTestResult, String> {
    Ok(ProxyTestResult {
        success: false,
        message: "Proxy testing disabled - ai_adapter removed".to_string(),
        proxy_config: None,
        response_time_ms: None,
    })
}

/// 获取当前代理配置 - DISABLED
#[tauri::command]
pub async fn get_current_proxy_config() -> Result<Option<serde_json::Value>, String> {
    Ok(None)
}