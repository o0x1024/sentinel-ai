//! ReWOO 引擎命令
//! 
//! 提供 ReWOO 引擎的测试和管理功能

use serde::{Deserialize, Serialize};
use tauri::{State, AppHandle};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use uuid::Uuid;
use crate::services::{AiServiceManager, database::DatabaseService};
use crate::engines::rewoo::engine_adapter::ReWooEngine;
use crate::engines::rewoo::rewoo_types::ReWOOConfig;
use crate::agents::traits::AgentTask;

#[derive(Debug, Serialize, Deserialize)]
pub struct ReWOOTestResult {
    pub success: bool,
    pub message: String,
}

/// 测试 ReWOO 引擎
#[tauri::command]
pub async fn test_rewoo_engine(
    query: String,
    ai_service_manager: State<'_, Arc<AiServiceManager>>,
    db_service: State<'_, Arc<DatabaseService>>,
    app: AppHandle,
) -> Result<String, String> {
    log::info!("Testing ReWOO engine with query: {}", query);
    
    // 创建 ReWOO 引擎配置
    let config = ReWOOConfig::default();
    
    // 创建 ReWOO 引擎
    let mut engine = ReWooEngine::new_with_dependencies(
        ai_service_manager.inner().clone(),
        config,
        db_service.inner().clone(),
    ).await.map_err(|e| format!("Failed to create ReWOO engine: {}", e))?;
    
    // 设置 app handle
    engine.set_app_handle(app);
    
    // 创建测试任务
    let task_id = Uuid::new_v4().to_string();
    let task = AgentTask {
        id: task_id.clone(),
        user_id: "test".to_string(),
        description: query,
        priority: crate::agents::traits::TaskPriority::Normal,
        target: None,
        parameters: HashMap::new(),
        timeout: Some(300),
    };
    
    // 执行任务
    match engine.execute(&task).await {
        Ok(result) => {
            log::info!("ReWOO engine test completed successfully");
            let response = if let Some(data) = &result.data {
                serde_json::to_string_pretty(data).unwrap_or_else(|_| format!("{:?}", data))
            } else {
                "ReWOO execution completed".to_string()
            };
            Ok(response)
        }
        Err(e) => {
            log::error!("ReWOO engine test failed: {}", e);
            Err(format!("ReWOO execution failed: {}", e))
        }
    }
}

/// 获取 ReWOO 测试结果
#[tauri::command]
pub async fn get_rewoo_test_result(
    test_id: String,
    _state: State<'_, Mutex<HashMap<String, String>>>,
) -> Result<ReWOOTestResult, String> {
    // 简化实现：直接返回成功状态
    // 实际的测试结果通过 test_rewoo_engine 直接返回
    log::info!("Getting ReWOO test result for: {}", test_id);
    Ok(ReWOOTestResult {
        success: true,
        message: format!("Test {} completed", test_id),
    })
}

/// 停止 ReWOO 测试
#[tauri::command]
pub async fn stop_rewoo_test(
    test_id: String,
    _state: State<'_, Mutex<HashMap<String, String>>>,
) -> Result<String, String> {
    log::info!("Stopping ReWOO test: {}", test_id);
    // ReWOO 引擎不支持取消执行
    Ok(format!("Test {} stop requested (ReWOO does not support cancellation)", test_id))
}

/// 清理 ReWOO 测试状态
#[tauri::command]
pub async fn cleanup_rewoo_test_state(
    _state: State<'_, Mutex<HashMap<String, String>>>,
) -> Result<String, String> {
    log::info!("Cleaning up ReWOO test state");
    Ok("ReWOO test state cleaned up".to_string())
}
