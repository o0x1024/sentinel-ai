//! Plan-and-Execute 架构 Tauri 命令
//! 
//! 这个模块提供了 Plan-and-Execute 架构的前端接口，支持：
//! - 引擎启动和停止
//! - 任务调度和执行
//! - 任务状态监控
//! - 任务历史查询
//! - 引擎统计信息

use crate::engines::plan_and_execute::{
    engine::{PlanAndExecuteEngine, PlanAndExecuteConfig, EngineConfig, EngineStatus},
    types::*,
    planner::PlannerConfig,
    executor::ExecutorConfig,
    replanner::ReplannerConfig,
    memory_manager::MemoryManagerConfig,
    tool_interface::ToolInterfaceConfig,
};
use crate::services::database::DatabaseService;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Plan-and-Execute 引擎状态
pub type PlanExecuteEngineState = Arc<RwLock<Option<PlanAndExecuteEngine>>>;

/// 命令响应包装器
#[derive(Debug, Serialize)]
pub struct CommandResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: u64,
    pub request_id: String,
}

impl<T> CommandResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            request_id: Uuid::new_v4().to_string(),
        }
    }
    
    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            request_id: Uuid::new_v4().to_string(),
        }
    }
}

/// 引擎状态响应
#[derive(Debug, Serialize)]
pub struct EngineStatusResponse {
    pub is_running: bool,
    pub is_paused: bool,
    pub active_tasks: u32,
    pub total_tasks_processed: u64,
    pub uptime_seconds: u64,
    pub last_activity: Option<String>,
    pub performance_metrics: PerformanceMetrics,
}

/// 性能指标
#[derive(Debug, Serialize)]
pub struct PerformanceMetrics {
    pub cpu_usage: f32,
    pub memory_usage: u64,
    pub average_task_duration: f64,
    pub success_rate: f32,
}

/// 任务调度请求
#[derive(Debug, Deserialize)]
pub struct DispatchTaskRequest {
    pub name: String,
    pub description: String,
    pub task_type: String, // 对应 TaskType 枚举
    pub target: TargetInfoRequest,
    pub priority: String, // 对应 Priority 枚举
    pub parameters: Option<HashMap<String, serde_json::Value>>,
    pub constraints: Option<HashMap<String, serde_json::Value>>,
    pub metadata: Option<HashMap<String, String>>,
}

/// 目标信息请求
#[derive(Debug, Deserialize)]
pub struct TargetInfoRequest {
    pub target_type: String, // 对应 TargetType 枚举
    pub address: String,
    pub port: Option<u16>,
    pub protocol: Option<String>,
    pub credentials: Option<HashMap<String, String>>,
    pub metadata: Option<HashMap<String, String>>,
}

/// 任务调度响应
#[derive(Debug, Serialize)]
pub struct DispatchTaskResponse {
    pub task_id: String,
    pub status: String,
    pub message: String,
    pub estimated_duration: Option<u64>,
    pub created_at: String,
}

/// 任务状态响应
#[derive(Debug, Serialize)]
pub struct TaskStatusResponse {
    pub task_id: String,
    pub status: String,
    pub progress: f32,
    pub current_step: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
    pub metrics: Option<TaskMetricsResponse>,
}

/// 任务指标响应
#[derive(Debug, Serialize)]
pub struct TaskMetricsResponse {
    pub execution_time_ms: u64,
    pub steps_completed: u32,
    pub steps_total: u32,
    pub memory_used_mb: u64,
    pub cpu_time_ms: u64,
    pub network_requests: u32,
    pub errors_count: u32,
}

/// 任务结果响应
#[derive(Debug, Serialize)]
pub struct TaskResultResponse {
    pub task_id: String,
    pub status: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub execution_time_ms: Option<u64>,
    pub result_data: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub metrics: Option<TaskMetricsResponse>,
    pub reports: Vec<TaskReportResponse>,
}

/// 任务报告响应
#[derive(Debug, Serialize)]
pub struct TaskReportResponse {
    pub report_id: String,
    pub report_type: String,
    pub title: String,
    pub content: String,
    pub format: String,
    pub created_at: String,
    pub attachments: Vec<ReportAttachmentResponse>,
}

/// 报告附件响应
#[derive(Debug, Serialize)]
pub struct ReportAttachmentResponse {
    pub filename: String,
    pub content_type: String,
    pub size_bytes: u64,
    pub data: Vec<u8>,
}

/// 任务列表响应
#[derive(Debug, Serialize)]
pub struct TaskListResponse {
    pub tasks: Vec<TaskSummary>,
    pub total: u32,
    pub active: u32,
    pub completed: u32,
    pub failed: u32,
}

/// 任务摘要
#[derive(Debug, Serialize)]
pub struct TaskSummary {
    pub task_id: String,
    pub name: String,
    pub task_type: String,
    pub status: String,
    pub priority: String,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub progress: f32,
}

/// 引擎统计响应
#[derive(Debug, Serialize)]
pub struct EngineStatisticsResponse {
    pub total_tasks: u64,
    pub successful_tasks: u64,
    pub failed_tasks: u64,
    pub average_execution_time: f64,
    pub uptime_seconds: u64,
    pub task_type_distribution: HashMap<String, u64>,
    pub priority_distribution: HashMap<String, u64>,
    pub hourly_task_count: Vec<u32>,
}

/// 启动 Plan-and-Execute 引擎
#[tauri::command]
pub async fn start_plan_execute_engine(
    engine_state: State<'_, PlanExecuteEngineState>,
    ai_service_manager: State<'_, Arc<crate::services::AiServiceManager>>,
    database_service: State<'_, Arc<DatabaseService>>,
) -> Result<CommandResponse<String>, String> {
    info!("🚀 [Plan-Execute] 启动引擎");
    
    let mut state = engine_state.write().await;
    
    if state.is_some() {
        info!("⚠️ [Plan-Execute] 引擎已经在运行中");
        return Ok(CommandResponse::success("引擎已经在运行中".to_string()));
    }
    
    // 创建默认配置
    let config = PlanAndExecuteConfig {
        name: "default".to_string(),
        version: "1.0.0".to_string(),
        planner_config: PlannerConfig::default(),
        executor_config: ExecutorConfig::default(),
        replanner_config: ReplannerConfig::default(),
        memory_config: MemoryManagerConfig::default(),
        tool_config: ToolInterfaceConfig::default(),
        engine_config: EngineConfig::default(),
    };
    
    // 创建一个临时的AiAdapterManager实例
    let ai_adapter_manager = Arc::new(crate::ai_adapter::core::AiAdapterManager::new());
    
    match PlanAndExecuteEngine::new(
        config,
        ai_adapter_manager,
        ai_service_manager.inner().clone(),
        database_service.inner().clone(),
    ).await {
        Ok(mut engine) => {
            match engine.start().await {
                Ok(_) => {
                    *state = Some(engine);
                    info!("✅ [Plan-Execute] 引擎启动成功");
                    Ok(CommandResponse::success("引擎启动成功".to_string()))
                }
                Err(e) => {
                    error!("💥 [Plan-Execute] 引擎启动失败: {}", e);
                    Ok(CommandResponse::error(format!("引擎启动失败: {}", e)))
                }
            }
        }
        Err(e) => {
            error!("💥 [Plan-Execute] 引擎创建失败: {}", e);
            Ok(CommandResponse::error(format!("引擎创建失败: {}", e)))
        }
    }
}

/// 停止 Plan-and-Execute 引擎
#[tauri::command]
pub async fn stop_plan_execute_engine(
    engine_state: State<'_, PlanExecuteEngineState>,
) -> Result<CommandResponse<String>, String> {
    info!("🛑 [Plan-Execute] 停止引擎");
    
    let mut state = engine_state.write().await;
    
    match state.take() {
        Some(mut engine) => {
            match engine.stop().await {
                Ok(_) => {
                    info!("✅ [Plan-Execute] 引擎停止成功");
                    Ok(CommandResponse::success("引擎停止成功".to_string()))
                }
                Err(e) => {
                    error!("💥 [Plan-Execute] 引擎停止失败: {}", e);
                    Ok(CommandResponse::error(format!("引擎停止失败: {}", e)))
                }
            }
        }
        None => {
            info!("⚠️ [Plan-Execute] 引擎未运行");
            Ok(CommandResponse::success("引擎未运行".to_string()))
        }
    }
}

/// 获取引擎状态
#[tauri::command]
pub async fn get_plan_execute_engine_status(
    engine_state: State<'_, PlanExecuteEngineState>,
) -> Result<CommandResponse<EngineStatusResponse>, String> {
    debug!("🔍 [Plan-Execute] 获取引擎状态");
    
    let state = engine_state.read().await;
    
    match state.as_ref() {
        Some(engine) => {
            let status = engine.get_status().await;
            let response = EngineStatusResponse {
                is_running: matches!(status, EngineStatus::Running),
                is_paused: false, // 需要从引擎获取暂停状态
                active_tasks: 0, // 需要从引擎获取活跃任务数
                total_tasks_processed: 0, // 需要从引擎指标获取
                uptime_seconds: 0, // 需要从引擎指标获取
                last_activity: None, // 需要从引擎指标获取
                performance_metrics: PerformanceMetrics {
                    cpu_usage: 0.0, // TODO: 实现实际的性能监控
                    memory_usage: 0,
                    average_task_duration: 0.0,
                    success_rate: 0.0,
                },
            };
            Ok(CommandResponse::success(response))
        }
        None => {
            Ok(CommandResponse::error("引擎未运行".to_string()))
        }
    }
}

/// 调度任务
#[tauri::command]
pub async fn dispatch_plan_execute_task(
    request: DispatchTaskRequest,
    engine_state: State<'_, PlanExecuteEngineState>,
) -> Result<CommandResponse<DispatchTaskResponse>, String> {
    info!("📋 [Plan-Execute] 调度任务: {}", request.name);
    debug!("📋 [Plan-Execute] 任务参数: {:?}", request);
    
    let state = engine_state.read().await;
    
    match state.as_ref() {
        Some(engine) => {
            // 构建任务请求
            let task_request = TaskRequest {
                id: Uuid::new_v4().to_string(),
                name: request.name.clone(),
                description: request.description,
                task_type: parse_task_type(&request.task_type)?,
                target: TargetInfo {
                    target_type: parse_target_type(&request.target.target_type)?,
                    address: request.target.address,
                    port: request.target.port,
                    port_range: None,
                    protocols: request.target.protocol.map(|p| vec![p]).unwrap_or_default(),
                    credentials: request.target.credentials,
                    metadata: request.target.metadata.unwrap_or_default().into_iter()
                        .map(|(k, v)| (k, serde_json::Value::String(v)))
                        .collect(),
                },
                parameters: request.parameters.unwrap_or_default(),
                priority: parse_priority(&request.priority)?,
                constraints: request.constraints.unwrap_or_default(),
                metadata: request.metadata.unwrap_or_default().into_iter().map(|(k, v)| (k, serde_json::Value::String(v))).collect(),
                created_at: std::time::SystemTime::now(),
            };
            
            match engine.execute_task(task_request).await {
                Ok(task_id) => {
                    info!("✅ [Plan-Execute] 任务调度成功: {}", task_id);
                    let response = DispatchTaskResponse {
                        task_id: task_id.clone(),
                        status: "Queued".to_string(),
                        message: "任务已成功加入执行队列".to_string(),
                        estimated_duration: None, // TODO: 实现预估时间
                        created_at: Utc::now().to_rfc3339(),
                    };
                    Ok(CommandResponse::success(response))
                }
                Err(e) => {
                    error!("💥 [Plan-Execute] 任务调度失败: {}", e);
                    Ok(CommandResponse::error(format!("任务调度失败: {}", e)))
                }
            }
        }
        None => {
            Ok(CommandResponse::error("引擎未运行".to_string()))
        }
    }
}

/// 获取任务状态
#[tauri::command]
pub async fn get_plan_execute_task_status(
    task_id: String,
    engine_state: State<'_, PlanExecuteEngineState>,
) -> Result<CommandResponse<TaskStatusResponse>, String> {
    debug!("🔍 [Plan-Execute] 获取任务状态: {}", task_id);
    
    let state = engine_state.read().await;
    
    match state.as_ref() {
        Some(engine) => {
            let status_option = engine.get_task_status(&task_id).await;
            match status_option {
                Some(status) => {
                    let response = TaskStatusResponse {
                        task_id: task_id.clone(),
                        status: format!("{:?}", status),
                        progress: 0.0, // TaskStatus枚举没有progress字段
                        current_step: None, // TaskStatus枚举没有current_step字段
                        started_at: None, // TaskStatus枚举没有started_at字段
                        completed_at: None, // TaskStatus枚举没有completed_at字段
                        error_message: None, // TaskStatus枚举没有error_message字段
                        metrics: None, // TaskStatus枚举没有metrics字段
                    };
                    Ok(CommandResponse::success(response))
                }
                None => {
                    Ok(CommandResponse::error(format!("任务不存在: {}", task_id)))
                }
            }
        }
        None => {
            Ok(CommandResponse::error("引擎未运行".to_string()))
        }
    }
}

/// 获取任务结果
#[tauri::command]
pub async fn get_plan_execute_task_result(
    task_id: String,
    engine_state: State<'_, PlanExecuteEngineState>,
) -> Result<CommandResponse<TaskResultResponse>, String> {
    info!("📊 [Plan-Execute] 获取任务结果: {}", task_id);
    
    let state = engine_state.read().await;
    
    match state.as_ref() {
        Some(engine) => {
            match engine.get_task_result(&task_id).await {
                Some(result) => {
                    let response = TaskResultResponse {
                        task_id: result.task_id,
                        status: format!("{:?}", result.status),
                        started_at: Some(result.started_at.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs().to_string()),
                        completed_at: result.completed_at.map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs().to_string()),
                        execution_time_ms: result.completed_at.map(|end| {
                            end.duration_since(result.started_at).unwrap_or_default().as_millis() as u64
                        }),
                        result_data: Some(result.result_data),
                        error_message: result.error,
                        metrics: Some(TaskMetricsResponse {
                            execution_time_ms: result.metrics.total_duration_ms,
                            steps_completed: result.metrics.successful_steps as u32,
                            steps_total: result.metrics.total_steps as u32,
                            memory_used_mb: result.metrics.memory_usage_bytes / 1024 / 1024,
                            cpu_time_ms: result.metrics.cpu_time_ms,
                            network_requests: result.metrics.network_requests as u32,
                            errors_count: result.metrics.failed_steps as u32,
                        }),
                        reports: result.reports.into_iter().map(|r| TaskReportResponse {
                            report_id: r.id,
                            report_type: format!("{:?}", r.report_type),
                            title: r.title,
                            content: r.content,
                            format: format!("{:?}", r.format),
                            created_at: r.generated_at.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs().to_string(),
                            attachments: r.attachments.into_iter().map(|a| ReportAttachmentResponse {
                                filename: a.name,
                                content_type: a.content_type,
                                size_bytes: a.size,
                                data: Vec::from_iter(a.path.as_str().bytes()),
                            }).collect(),
                        }).collect(),
                    };
                    Ok(CommandResponse::success(response))
                }
                None => {
                    Ok(CommandResponse::error("任务结果不存在".to_string()))
                }
            }
        }
        None => {
            Ok(CommandResponse::error("引擎未运行".to_string()))
        }
    }
}

/// 取消任务
#[tauri::command]
pub async fn cancel_plan_execute_task(
    task_id: String,
    engine_state: State<'_, PlanExecuteEngineState>,
) -> Result<CommandResponse<String>, String> {
    info!("🛑 [Plan-Execute] 取消任务: {}", task_id);
    
    let state = engine_state.read().await;
    
    match state.as_ref() {
        Some(engine) => {
            match engine.cancel_task(&task_id).await {
                Ok(_) => {
                    info!("✅ [Plan-Execute] 任务取消成功: {}", task_id);
                    Ok(CommandResponse::success("任务取消成功".to_string()))
                }
                Err(e) => {
                    error!("💥 [Plan-Execute] 任务取消失败: {}", e);
                    Ok(CommandResponse::error(format!("任务取消失败: {}", e)))
                }
            }
        }
        None => {
            Ok(CommandResponse::error("引擎未运行".to_string()))
        }
    }
}

/// 获取活跃任务列表
#[tauri::command]
pub async fn get_plan_execute_active_tasks(
    engine_state: State<'_, PlanExecuteEngineState>,
) -> Result<CommandResponse<TaskListResponse>, String> {
    debug!("📋 [Plan-Execute] 获取活跃任务列表");
    
    let state = engine_state.read().await;
    
    match state.as_ref() {
        Some(engine) => {
            let tasks = engine.get_active_tasks().await;
            let task_summaries: Vec<TaskSummary> = tasks.into_iter().map(|task_id| {
                TaskSummary {
                    task_id: task_id.clone(),
                    name: "Active Task".to_string(),
                    task_type: "Unknown".to_string(),
                    status: "Active".to_string(),
                    priority: "Normal".to_string(),
                    created_at: chrono::Utc::now().to_rfc3339(),
                    started_at: None,
                    completed_at: None,
                    progress: 0.0,
                }
            }).collect();
                    
            let response = TaskListResponse {
                total: task_summaries.len() as u32,
                active: task_summaries.len() as u32,
                completed: 0,
                failed: 0,
                tasks: task_summaries,
            };
            
            Ok(CommandResponse::success(response))
         }
         None => {
             Ok(CommandResponse::error("引擎未运行".to_string()))
         }
     }
 }

/// 获取任务历史
#[tauri::command]
pub async fn get_plan_execute_task_history(
    limit: Option<u32>,
    offset: Option<u32>,
    engine_state: State<'_, PlanExecuteEngineState>,
) -> Result<CommandResponse<TaskListResponse>, String> {
    debug!("📚 [Plan-Execute] 获取任务历史");
    
    let state = engine_state.read().await;
    
    match state.as_ref() {
        Some(engine) => {
            let limit_usize = limit.map(|l| l as usize);
            let tasks = engine.get_task_history(limit_usize).await;
            let task_summaries: Vec<TaskSummary> = tasks.into_iter().map(|session| {
                TaskSummary {
                    task_id: session.id.clone(),
                    name: session.request.name.clone(),
                    task_type: format!("{:?}", session.request.task_type),
                    status: format!("{:?}", session.status),
                    priority: format!("{:?}", session.request.priority),
                    created_at: session.started_at.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs().to_string(),
                    started_at: Some(session.started_at.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs().to_string()),
                    completed_at: session.completed_at.map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs().to_string()),
                    progress: match session.status {
                        TaskStatus::Completed => 100.0,
                        TaskStatus::Failed | TaskStatus::Cancelled => 0.0,
                        _ => 50.0,
                    },
                }
            }).collect();
                    
            let completed = task_summaries.iter().filter(|t| t.status == "Completed").count() as u32;
            let failed = task_summaries.iter().filter(|t| t.status == "Failed" || t.status == "Cancelled").count() as u32;
            
            let response = TaskListResponse {
                total: task_summaries.len() as u32,
                active: 0,
                completed,
                failed,
                tasks: task_summaries,
            };
            
            Ok(CommandResponse::success(response))
         }
         None => {
             Ok(CommandResponse::error("引擎未运行".to_string()))
         }
     }
 }

// 辅助函数：解析任务类型
fn parse_task_type(task_type: &str) -> Result<TaskType, String> {
    match task_type.to_lowercase().as_str() {
        "security_scan" => Ok(TaskType::SecurityScan),
        "vulnerability_assessment" => Ok(TaskType::VulnerabilityAssessment),
        "penetration_test" => Ok(TaskType::PenetrationTest),
        "compliance_check" => Ok(TaskType::ComplianceCheck),
        "threat_hunting" => Ok(TaskType::ThreatHunting),
        "incident_response" => Ok(TaskType::IncidentResponse),
        "forensic_analysis" => Ok(TaskType::ForensicAnalysis),
        "risk_assessment" => Ok(TaskType::RiskAssessment),
        _ => Err(format!("未知的任务类型: {}", task_type)),
    }
}

// 辅助函数：解析目标类型
fn parse_target_type(target_type: &str) -> Result<TargetType, String> {
    match target_type.to_lowercase().as_str() {
        "host" => Ok(TargetType::Host),
        "network" => Ok(TargetType::Network),
        "domain" => Ok(TargetType::Domain),
        "url" => Ok(TargetType::Url),
        "application" => Ok(TargetType::Application),
        "database" => Ok(TargetType::Database),
        "api" => Ok(TargetType::Api),
        "file" => Ok(TargetType::File),
        _ => Err(format!("未知的目标类型: {}", target_type)),
    }
}

// 辅助函数：解析优先级
fn parse_priority(priority: &str) -> Result<Priority, String> {
    match priority.to_lowercase().as_str() {
        "low" => Ok(Priority::Low),
        "normal" => Ok(Priority::Normal),
        "high" => Ok(Priority::High),
        "critical" => Ok(Priority::Critical),
        _ => Err(format!("未知的优先级: {}", priority)),
    }
}