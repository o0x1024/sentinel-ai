//! Agent管理相关命令

use crate::agents::{AgentManager, AgentSelectionStrategy, MultiAgentRequest, TaskPriority};
use crate::services::database::{Database, DatabaseService};
use crate::engines::plan_and_execute::repository::PlanExecuteRepository;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{command, State};
use tokio::sync::RwLock;
use log::warn;

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

/// 工作流执行记录数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecution {
    pub execution_id: String,
    pub workflow_id: String,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub current_step: Option<String>,
    pub total_steps: u32,
    pub completed_steps: u32,
    pub progress: u32,
    pub error: Option<String>,
    pub result: Option<serde_json::Value>,
}

/// 工作流统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStatistics {
    pub total_workflows: u32,
    pub total_executions: u32,
    pub successful_executions: u32,
    pub failed_executions: u32,
    pub running_executions: u32,
}

/// 获取工作流执行列表
#[command]
pub async fn list_workflow_executions(
    manager: State<'_, GlobalAgentManager>,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<WorkflowExecution>, String> {
    // 首先尝试从数据库获取Plan-and-Execute执行记录
    let pool = db_service.get_pool().map_err(|e| format!("Failed to get database pool: {}", e))?;
    let repository = PlanExecuteRepository::new(pool.clone());
    
    let mut executions = Vec::new();
    
    // 从数据库获取执行会话
    match repository.list_execution_sessions().await {
        Ok(sessions) => {
            for session in sessions {
                let execution = WorkflowExecution {
                    execution_id: session.id.clone(),
                    workflow_id: session.plan_id.clone(),
                    status: format!("{:?}", session.status),
                    started_at: session.started_at.duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default().as_secs().to_string(),
                    completed_at: session.completed_at.map(|t| 
                        t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs().to_string()),
                    current_step: session.current_step.map(|s| format!("Step {}", s)),
                    total_steps: session.step_results.len() as u32,
                    completed_steps: session.step_results.values()
                        .filter(|r| matches!(r.status, crate::engines::types::StepExecutionStatus::Completed))
                        .count() as u32,
                    progress: session.progress as u32,
                    error: None, // 可以从metadata中提取错误信息
                    result: None,
                };
                executions.push(execution);
            }
        },
        Err(e) => {
            log::warn!("Failed to get executions from database: {}", e);
        }
    }
    
    // 从数据库获取agent_sessions执行记录
    match db_service.list_agent_sessions().await {
        Ok(sessions) => {
            for session in sessions {
                // 检查是否已经存在（避免重复）
                if executions.iter().any(|e| e.execution_id == session.session_id) {
                    continue;
                }
                
                let status = match session.status.as_str() {
                    "Created" => "Pending",
                    "Planning" => "Running", 
                    "Executing" => "Running",
                    "Completed" => "Completed",
                    "Failed" => "Failed",
                    "Cancelled" => "Cancelled",
                    _ => "Unknown",
                }.to_string();
                
                let progress = match session.status.as_str() {
                    "Completed" => 100,
                    "Failed" => 0,
                    "Executing" => 50,
                    _ => 0,
                };
                
                let execution = WorkflowExecution {
                    execution_id: session.session_id.clone(),
                    workflow_id: format!("agent-session-{}", session.agent_name),
                    status,
                    started_at: session.created_at.timestamp().to_string(),
                    completed_at: None, // AgentSessionData doesn't have completed_at field
                    current_step: Some(format!("Agent Task - {}", session.agent_name)),
                    total_steps: 1,
                    completed_steps: if session.status == "Completed" { 1 } else { 0 },
                    progress,
                    error: None, // AgentSessionData doesn't have error field
                    result: None, // AgentSessionData doesn't have result field
                };
                executions.push(execution);
            }
        },
        Err(e) => {
            log::warn!("Failed to get agent sessions: {}", e);
        }
    }
    
    let manager_guard = manager.read().await;
    
    let agent_manager = match manager_guard.as_ref() {
        Some(manager) => manager,
        None => return Err("Agent manager not initialized".to_string())
    };
    
    // 获取所有活跃会话并转换为WorkflowExecution格式
    let sessions = agent_manager.get_all_sessions().await;
    // 继续向前面已收集的 executions 追加活跃会话，避免覆盖之前的结果
    
    for (session_id, session_info) in sessions {
        let status = match session_info.status {
            crate::agents::traits::AgentSessionStatus::Created => "Pending",
            crate::agents::traits::AgentSessionStatus::Planning => "Running", 
            crate::agents::traits::AgentSessionStatus::Executing => "Running",
            crate::agents::traits::AgentSessionStatus::Completed => "Completed",
            crate::agents::traits::AgentSessionStatus::Failed => "Failed",
            crate::agents::traits::AgentSessionStatus::Cancelled => "Cancelled",
        }.to_string();
        
        // 计算进度
        let progress = match session_info.status {
            crate::agents::traits::AgentSessionStatus::Created => 0.0,
            crate::agents::traits::AgentSessionStatus::Planning => 20.0,
            crate::agents::traits::AgentSessionStatus::Executing => 60.0,
            crate::agents::traits::AgentSessionStatus::Completed => 100.0,
            crate::agents::traits::AgentSessionStatus::Failed => 0.0,
            crate::agents::traits::AgentSessionStatus::Cancelled => 0.0,
        };
        
        // 获取当前步骤描述
        let current_step = match session_info.status {
            crate::agents::traits::AgentSessionStatus::Created => Some("初始化".to_string()),
            crate::agents::traits::AgentSessionStatus::Planning => Some("任务规划中".to_string()),
            crate::agents::traits::AgentSessionStatus::Executing => Some("执行任务".to_string()),
            crate::agents::traits::AgentSessionStatus::Completed => Some("已完成".to_string()),
            crate::agents::traits::AgentSessionStatus::Failed => Some("执行失败".to_string()),
            crate::agents::traits::AgentSessionStatus::Cancelled => Some("已取消".to_string()),
        };
        
        let execution = WorkflowExecution {
            execution_id: session_id.clone(),
            workflow_id: session_info.task.description.clone(),
            status,
            // 统一为秒级时间戳字符串，前端用 parseInt * 1000 解析
            started_at: session_info.created_at.timestamp().to_string(),
            completed_at: session_info.completed_at.map(|t| t.timestamp().to_string()),
            current_step,
            total_steps: 4, // 默认4个步骤
            completed_steps: match session_info.status {
                crate::agents::traits::AgentSessionStatus::Created => 0,
                crate::agents::traits::AgentSessionStatus::Planning => 1,
                crate::agents::traits::AgentSessionStatus::Executing => 2,
                crate::agents::traits::AgentSessionStatus::Completed => 4,
                crate::agents::traits::AgentSessionStatus::Failed => 2,
                crate::agents::traits::AgentSessionStatus::Cancelled => 1,
            },
            progress: progress as u32,
            error: session_info.error.map(|e| e.to_string()),
            result: session_info.result,
        };
        
        executions.push(execution);
    }
    
    // 按创建时间排序，最新的在前
    executions.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    
    Ok(executions)
}

/// 获取工作流统计信息
#[command]
pub async fn get_workflow_statistics(
    manager: State<'_, GlobalAgentManager>,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<WorkflowStatistics, String> {
    // 首先尝试从数据库获取统计信息
    let pool = db_service.get_pool().map_err(|e| format!("Failed to get database pool: {}", e))?;
    let repository = PlanExecuteRepository::new(pool.clone());
    
    match repository.get_execution_statistics().await {
        Ok(db_stats) => {
            return Ok(WorkflowStatistics {
                total_workflows: 3, // 三种架构类型
                total_executions: db_stats.total_sessions as u32,
                successful_executions: db_stats.completed_sessions as u32,
                failed_executions: db_stats.failed_sessions as u32,
                running_executions: db_stats.running_sessions as u32,
            });
        },
        Err(e) => {
            log::warn!("Failed to get statistics from database: {}", e);
        }
    }
    
    // 如果数据库失败，回退到内存统计
    let manager_guard = manager.read().await;
    
    let agent_manager = match manager_guard.as_ref() {
        Some(manager) => manager,
        None => return Err("Agent manager not initialized".to_string())
    };
    
    let stats = agent_manager.get_statistics().await;
    let sessions = agent_manager.get_all_sessions().await;
    
    // 统计不同状态的执行数量
    let mut running_count = 0;
    for (_, session_info) in sessions {
        match session_info.status {
            crate::agents::traits::AgentSessionStatus::Planning | 
            crate::agents::traits::AgentSessionStatus::Executing => {
                running_count += 1;
            },
            _ => {}
        }
    }
    
    Ok(WorkflowStatistics {
        total_workflows: 3, // 三种架构类型
        total_executions: stats.total_tasks as u32,
        successful_executions: stats.successful_tasks as u32,
        failed_executions: stats.failed_tasks as u32,
        running_executions: running_count,
    })
}

/// 获取工作流执行详情
#[command]
pub async fn get_workflow_execution(
    execution_id: String,
    manager: State<'_, GlobalAgentManager>,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Option<WorkflowExecution>, String> {
    let executions = list_workflow_executions(manager, db_service).await?;
    Ok(executions.into_iter().find(|e| e.execution_id == execution_id))
}

/// 取消工作流执行
#[command]
pub async fn cancel_workflow_execution(
    execution_id: String,
    manager: State<'_, GlobalAgentManager>,
) -> Result<String, String> {
    cancel_agent_task(execution_id, manager).await
}

/// 删除工作流执行记录
#[command]
pub async fn delete_workflow_execution(
    execution_id: String,
    manager: State<'_, GlobalAgentManager>,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<String, String> {
    // 1) 删除数据库中的会话记录（Plan-and-Execute）
    if let Ok(pool) = db_service.get_pool() {
        let repository = PlanExecuteRepository::new(pool.clone());
        if let Err(e) = repository.delete_execution_session(&execution_id).await {
            log::warn!("Failed to delete execution session from DB: {}", e);
        }
    } else {
        log::warn!("Database pool not available when deleting execution session");
    }

    // 2) 删除agent_session记录
    use crate::services::database::Database;
    if let Err(e) = db_service.delete_agent_session(&execution_id).await {
        log::warn!("Failed to delete agent session: {}", e);
    }
    
    // 删除agent_execution_steps记录
    if let Err(e) = db_service.delete_agent_execution_steps(&execution_id).await {
        log::warn!("Failed to delete agent execution steps: {}", e);
    }

    // 3) 从内存中移除（活跃与已完成会话）
    let manager_guard = manager.read().await;
    if let Some(agent_manager) = manager_guard.as_ref() {
        {
            let mut active = agent_manager.active_sessions.write().await;
            active.remove(&execution_id);
        }
        {
            let mut completed = agent_manager.completed_sessions.write().await;
            completed.remove(&execution_id);
        }
    }

    Ok("Execution deleted successfully".to_string())
}

/// 工作流步骤详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStepDetail {
    pub step_id: String,
    pub step_name: String,
    pub status: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub duration_ms: u64,
    pub result_data: Option<serde_json::Value>,
    pub error: Option<String>,
    pub retry_count: u32,
    pub dependencies: Vec<String>,
    pub tool_result: Option<serde_json::Value>,
}

/// 工作流执行计划
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionPlan {
    pub plan_id: String,
    pub name: String,
    pub description: Option<String>,
    pub steps: Vec<WorkflowStepDetail>,
    pub total_steps: u32,
    pub completed_steps: u32,
    pub failed_steps: u32,
    pub skipped_steps: u32,
}

/// 获取工作流执行详细信息（包含步骤详情）
#[command]
pub async fn get_workflow_execution_details(
    execution_id: String,
    manager: State<'_, GlobalAgentManager>,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Option<WorkflowExecutionPlan>, String> {
    let manager_guard = manager.read().await;
    
    let agent_manager = match manager_guard.as_ref() {
        Some(manager) => manager,
        None => return Err("Agent manager not initialized".to_string())
    };
    
    // 首先尝试从数据库获取Agent执行步骤
    match db_service.get_agent_execution_steps(&execution_id).await {
        Ok(db_steps) if !db_steps.is_empty() => {
            // 从数据库获取会话信息
            let session_data = db_service.get_agent_session(&execution_id).await
                .map_err(|e| format!("Failed to get session data: {}", e))?;
            
            if let Some(session) = session_data {
                let completed_steps: u32 = db_steps
                    .iter()
                    .filter(|s| s.status == "Completed")
                    .count()
                    .try_into()
                    .unwrap_or(0);
                let failed_steps: u32 = db_steps
                    .iter()
                    .filter(|s| s.status == "Failed")
                    .count()
                    .try_into()
                    .unwrap_or(0);
                let skipped_steps: u32 = db_steps
                    .iter()
                    .filter(|s| s.status == "Skipped")
                    .count()
                    .try_into()
                    .unwrap_or(0);
                
                let execution_plan = WorkflowExecutionPlan {
                    plan_id: execution_id.clone(),
                    name: format!("Agent Execution - {}", session.agent_name),
                    description: Some(format!("Task ID: {} | Status: {}", session.task_id, session.status)),
                    steps: db_steps.clone(),
                    total_steps: db_steps.len().try_into().unwrap_or(u32::MAX),
                    completed_steps,
                    failed_steps,
                    skipped_steps,
                };
                
                return Ok(Some(execution_plan));
            }
        },
        Ok(_) => {
            // 数据库中没有步骤数据，使用原有的模拟逻辑
        },
        Err(e) => {
            warn!("Failed to get steps from database: {}", e);
            // 继续使用模拟逻辑作为fallback
        }
    }
    
    // 获取所有会话信息（fallback到内存数据）
    let all_sessions = agent_manager.get_all_sessions().await;
    
    if let Some(session_info) = all_sessions.get(&execution_id) {
        // 模拟步骤详情（因为当前Agent系统没有详细的步骤跟踪）
        let mut steps = Vec::new();
        
        // 根据任务描述推断任务类型和状态生成模拟步骤
        let description = &session_info.task.description.to_lowercase();
        let status = &session_info.status;
        
        // 基于任务描述推断类型并生成步骤，尽量模拟实际执行步骤
        let step_names = if description.contains("分析") || description.contains("analysis") {
            vec!["数据收集", "数据分析", "深度分析", "结果生成"]
        } else if description.contains("扫描") || description.contains("scan") {
            vec!["目标识别", "漏洞扫描", "深度扫描", "报告生成"]
        } else if description.contains("搜索") || description.contains("研究") || description.contains("research") || description.contains("热门") {
            // 根据日志显示的实际步骤来生成
            vec!["信息搜索", "内容分析", "补充搜索热门榜单", "汇总与去重"]
        } else {
            vec!["任务准备", "数据处理", "结果整合", "任务完成"]
        };
        
        for (i, step_name) in step_names.iter().enumerate() {
            let step_status = match status {
                crate::agents::traits::AgentSessionStatus::Completed => "Completed",
                crate::agents::traits::AgentSessionStatus::Failed => {
                    if i == 0 { "Failed" } else { "Pending" }
                },
                crate::agents::traits::AgentSessionStatus::Executing => {
                    // 对4步的情况，动态设置状态
                    match step_names.len() {
                        4 => {
                            if i < 3 { "Completed" }
                            else if i == 3 { "Running" }
                            else { "Pending" }
                        },
                        3 => {
                            if i == 0 { "Completed" }
                            else if i == 1 { "Running" } 
                            else { "Pending" }
                        },
                        _ => {
                            if i == 0 { "Completed" }
                            else if i == 1 { "Running" } 
                            else { "Pending" }
                        }
                    }
                },
                crate::agents::traits::AgentSessionStatus::Cancelled => "Cancelled",
                _ => "Pending",
            };
            
            let step_detail = WorkflowStepDetail {
                step_id: format!("step_{}", i + 1),
                step_name: step_name.to_string(),
                status: step_status.to_string(),
                started_at: Some(session_info.created_at.timestamp().to_string()),
                completed_at: session_info.completed_at.map(|t| t.timestamp().to_string()),
                duration_ms: if step_status == "Completed" { 1000 + i as u64 * 500 } else { 0 },
                result_data: session_info.result.clone(),
                error: session_info.error.clone(),
                retry_count: 0,
                dependencies: if i > 0 { vec![format!("step_{}", i)] } else { vec![] },
                tool_result: session_info.result.clone(),
            };
            steps.push(step_detail);
        }
        
        let plan = WorkflowExecutionPlan {
            plan_id: execution_id.clone(),
            name: session_info.task.description.clone(),
            description: Some(session_info.task.description.clone()),
            total_steps: steps.len() as u32,
            completed_steps: steps.iter().filter(|s| s.status == "Completed").count() as u32,
            failed_steps: steps.iter().filter(|s| s.status == "Failed").count() as u32,
            skipped_steps: steps.iter().filter(|s| s.status == "Skipped").count() as u32,
            steps,
        };
        
        return Ok(Some(plan));
    }
    
    Ok(None)
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
