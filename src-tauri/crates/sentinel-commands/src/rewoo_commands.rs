//! ReWOO 引擎命令 - DISABLED (需要 Rig 重构)
//! 
//! 所有 ReWOO 相关功能都已禁用，等待 Rig 重构

use serde::{Deserialize, Serialize};
use tauri::State;
use std::sync::Mutex;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct ReWOOTestResult {
    pub success: bool,
    pub message: String,
}

/// 测试 ReWOO 引擎 - DISABLED
#[tauri::command]
pub async fn test_rewoo_engine(
    _state: State<'_, Mutex<HashMap<String, String>>>,
) -> Result<String, String> {
    Err("ReWOO engine disabled - needs Rig refactor".to_string())
}

/// 获取 ReWOO 测试结果 - DISABLED
#[tauri::command]
pub async fn get_rewoo_test_result(
    _test_id: String,
    _state: State<'_, Mutex<HashMap<String, String>>>,
) -> Result<ReWOOTestResult, String> {
    Err("ReWOO engine disabled - needs Rig refactor".to_string())
}

/// 停止 ReWOO 测试 - DISABLED
#[tauri::command]
pub async fn stop_rewoo_test(
    _test_id: String,
    _state: State<'_, Mutex<HashMap<String, String>>>,
) -> Result<String, String> {
    Err("ReWOO engine disabled - needs Rig refactor".to_string())
}

/// 清理 ReWOO 测试状态 - DISABLED
#[tauri::command]
pub async fn cleanup_rewoo_test_state(
    _state: State<'_, Mutex<HashMap<String, String>>>,
) -> Result<String, String> {
    Ok("ReWOO test state cleanup skipped (engine disabled)".to_string())
}
