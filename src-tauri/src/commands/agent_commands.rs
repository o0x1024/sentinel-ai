//! Agent管理相关命令

use crate::agents::{AgentManager, AgentSelectionStrategy, MultiAgentRequest, TaskPriority};
// 已删除 plan_and_execute 独立模块，能力内嵌到泛化的 ReAct 引擎中
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{command, State};
use tokio::sync::RwLock;

/// 全局Agent管理器
pub type GlobalAgentManager = Arc<RwLock<Option<AgentManager>>>;

/// 多Agent执行请求（前端接口）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecutionRequest {
    /// 用户输入
    pub user_input: String,
    /// 目标
    pub target: Option<String>,
    /// 执行上下文
    pub context: HashMap<String, serde_json::Value>,
    /// 会话ID
    pub conversation_id: Option<String>,
    /// 用户ID
    pub user_id: String,
    /// 架构选择策略
    pub architecture: Option<String>,
    /// 优先级
    pub priority: Option<String>,
}

/// 执行响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecutionResponse {
    /// 会话ID
    pub session_id: String,
    /// 选择的架构
    pub selected_architecture: String,
    /// 估计执行时间
    pub estimated_duration: u64,
    /// 状态
    pub status: String,
}

/// 初始化Agent管理器
#[command]
pub async fn initialize_agent_manager(
    manager: State<'_, GlobalAgentManager>,
    db_service: State<'_, Arc<crate::services::database::DatabaseService>>,
    ai_service_manager: State<'_, Arc<crate::services::AiServiceManager>>,
) -> Result<String, String> {
    let mut manager_guard = manager.write().await;
    
    if manager_guard.is_some() {
        return Ok("Agent manager already initialized".to_string());
    }
    
    let agent_manager = AgentManager::new((*db_service).clone());
    
    // 尝试使用完整依赖初始化
    match agent_manager.initialize_with_dependencies(
        (*ai_service_manager).clone(),
        (*db_service).clone(),
    ).await {
        Ok(_) => {
            *manager_guard = Some(agent_manager);
            Ok("Agent manager initialized successfully with full dependencies".to_string())
        }
        Err(e) => {
            log::warn!("Failed to initialize with dependencies: {}, trying fallback", e);
            Err(format!("Failed to initialize with dependencies: {}", e))
        }
    }
}

/// 列出所有可用的Agent
#[command]
pub async fn list_agents(
    manager: State<'_, GlobalAgentManager>,
) -> Result<Vec<String>, String> {
    let manager_guard = manager.read().await;
    
    match manager_guard.as_ref() {
        Some(agent_manager) => {
            Ok(agent_manager.list_agents().await)
        }
        None => Err("Agent manager not initialized".to_string())
    }
}

/// 列出所有可用的执行引擎
#[command]
pub async fn list_agent_architectures(
    manager: State<'_, GlobalAgentManager>,
) -> Result<Vec<String>, String> {
    let manager_guard = manager.read().await;
    
    match manager_guard.as_ref() {
        Some(agent_manager) => {
            Ok(agent_manager.list_engines().await)
        }
        None => Err("Agent manager not initialized".to_string())
    }
}

/// 分发多Agent任务
#[command]
pub async fn dispatch_multi_agent_task(
    request: AgentExecutionRequest,
    manager: State<'_, GlobalAgentManager>,
) -> Result<AgentExecutionResponse, String> {
    let manager_guard = manager.read().await;
    
    let agent_manager = match manager_guard.as_ref() {
        Some(manager) => manager,
        None => return Err("Agent manager not initialized".to_string())
    };
    
    // 转换优先级
    let priority = match request.priority.as_deref() {
        Some("low") => TaskPriority::Low,
        Some("high") => TaskPriority::High,
        Some("critical") => TaskPriority::Critical,
        _ => TaskPriority::Normal,
    };
    
    // 转换架构选择策略
    let selection_strategy = match request.architecture.as_deref() {
        Some("plan_execute") => AgentSelectionStrategy::Specific("plan_execute_agent".to_string()),
        Some("rewoo") => AgentSelectionStrategy::Specific("rewoo_agent".to_string()),
        Some("llm_compiler") => AgentSelectionStrategy::Specific("llm_compiler_agent".to_string()),
        Some("performance") => AgentSelectionStrategy::PerformanceOptimized,
        Some("resource") => AgentSelectionStrategy::ResourceOptimized,
        _ => AgentSelectionStrategy::Auto,
    };
    
    let multi_agent_request = MultiAgentRequest {
        user_input: request.user_input,
        target: request.target,
        context: request.context,
        selection_strategy,
        priority,
        user_id: request.user_id,
    };
    
    match agent_manager.dispatch_task(multi_agent_request).await {
        Ok(session_id) => {
            Ok(AgentExecutionResponse {
                session_id,
                selected_architecture: request.architecture.unwrap_or("auto".to_string()),
                estimated_duration: 300, // 5分钟默认值
                status: "dispatched".to_string(),
            })
        }
        Err(e) => Err(format!("Failed to dispatch task: {}", e))
    }
}

/// 获取任务状态
#[command]
pub async fn get_agent_task_status(
    session_id: String,
    manager: State<'_, GlobalAgentManager>,
) -> Result<Option<String>, String> {
    let manager_guard = manager.read().await;
    
    let agent_manager = match manager_guard.as_ref() {
        Some(manager) => manager,
        None => return Err("Agent manager not initialized".to_string())
    };
    
    match agent_manager.get_session_status(&session_id).await {
        Some(status) => Ok(Some(format!("{:?}", status))),
        None => Ok(None)
    }
}

/// 取消任务
#[command]
pub async fn cancel_agent_task(
    session_id: String,
    manager: State<'_, GlobalAgentManager>,
) -> Result<String, String> {
    let manager_guard = manager.read().await;
    
    let agent_manager = match manager_guard.as_ref() {
        Some(manager) => manager,
        None => return Err("Agent manager not initialized".to_string())
    };
    
    match agent_manager.cancel_task(&session_id).await {
        Ok(_) => Ok("Task cancelled successfully".to_string()),
        Err(e) => Err(format!("Failed to cancel task: {}", e))
    }
}

/// 获取Agent系统统计信息
#[command]
pub async fn get_agent_system_stats(
    manager: State<'_, GlobalAgentManager>,
) -> Result<serde_json::Value, String> {
    let manager_guard = manager.read().await;
    
    let agent_manager = match manager_guard.as_ref() {
        Some(manager) => manager,
        None => return Err("Agent manager not initialized".to_string())
    };
    
    let stats = agent_manager.get_statistics().await;
    
    Ok(serde_json::json!({
        "total_agents": 3, // 固定值，因为我们有3个引擎
        "total_tasks": stats.total_tasks,
        "successful_tasks": stats.successful_tasks,
        "failed_tasks": stats.failed_tasks,
        "active_sessions": stats.active_sessions,
        "overall_success_rate": if stats.total_tasks > 0 {
            stats.successful_tasks as f64 / stats.total_tasks as f64
        } else {
            0.0
        },
        "average_execution_time": stats.average_execution_time_ms / 1000.0 // 转换为秒
    }))
}

/// 获取调度统计信息（兼容原API）
#[command]
pub async fn get_dispatch_statistics(
    manager: State<'_, GlobalAgentManager>,
) -> Result<serde_json::Value, String> {
    let manager_guard = manager.read().await;
    
    let agent_manager = match manager_guard.as_ref() {
        Some(manager) => manager,
        None => return Err("Agent manager not initialized".to_string())
    };
    
    let stats = agent_manager.get_statistics().await;
    
    Ok(serde_json::json!({
        "total_dispatches": stats.total_tasks,
        "successful_dispatches": stats.successful_tasks,
        "failed_dispatches": stats.failed_tasks,
        "average_duration": stats.average_execution_time_ms / 1000.0
    }))
}


/// 获取Agent任务执行日志
#[command]
pub async fn get_agent_task_logs(
    session_id: String,
    manager: State<'_, GlobalAgentManager>,
) -> Result<Vec<serde_json::Value>, String> {
    let manager_guard = manager.read().await;
    
    let agent_manager = match manager_guard.as_ref() {
        Some(manager) => manager,
        None => return Err("Agent manager not initialized".to_string())
    };
    
    // 获取会话日志
    let active_sessions = agent_manager.active_sessions.read().await;
    match active_sessions.get(&session_id) {
        Some(session) => {
            let logs = session.get_logs();
            let json_logs: Vec<serde_json::Value> = logs.iter().map(|log| {
                serde_json::json!({
                    "level": format!("{:?}", log.level),
                    "message": log.message,
                    "timestamp": log.timestamp.to_rfc3339(),
                })
            }).collect();
            Ok(json_logs)
        }
        None => Ok(Vec::new()) // 会话不存在或已结束，返回空日志
    }
}

/// 添加测试会话数据（仅用于调试）
#[command]
pub async fn add_test_session_data(
    manager: State<'_, GlobalAgentManager>,
) -> Result<String, String> {
    let manager_guard = manager.read().await;
    
    let agent_manager = match manager_guard.as_ref() {
        Some(manager) => manager,
        None => return Err("Agent manager not initialized".to_string())
    };
    
    // 添加一些测试的已完成会话数据
    let mut completed_sessions = agent_manager.completed_sessions.write().await;
    
    let test_task = crate::agents::traits::AgentTask {
        id: "test-task-1".to_string(),
        description: "测试任务：查看B站今天的热门视频".to_string(),
        target: Some("bilibili.com".to_string()),
        parameters: std::collections::HashMap::new(),
        user_id: "test-user".to_string(),
        priority: crate::agents::traits::TaskPriority::Normal,
        timeout: Some(300),
    };
    
    let session_info = crate::agents::manager::SessionInfo {
        task: test_task,
        status: crate::agents::traits::AgentSessionStatus::Completed,
        created_at: chrono::Utc::now() - chrono::Duration::minutes(5),
        completed_at: Some(chrono::Utc::now()),
        error: None,
        result: Some(serde_json::json!({
            "success": true,
            "message": "成功获取了B站热门视频列表"
        })),
    };
    
    completed_sessions.insert("test-session-1".to_string(), session_info);
    
    Ok("Test session data added successfully".to_string())
}
