//! 智能调度器 Tauri 命令
//! 
//! 这个模块提供了智能调度器的前端接口，支持：
//! - 智能查询分析和架构选择
//! - 动态工作流创建和执行
//! - 执行状态监控和管理
//! - 历史记录查询和分析

use crate::engines::intelligent_dispatcher::IntelligentDispatcher;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;
use log::{info, error, debug};
use uuid::Uuid;

/// 智能调度器服务状态
pub type IntelligentDispatcherState = Arc<RwLock<Option<IntelligentDispatcher>>>;

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

/// 智能查询请求参数
#[derive(Debug, Deserialize)]
pub struct IntelligentQueryRequest {
    /// 用户输入
    pub user_input: String,
    /// 用户ID
    pub user_id: String,
    /// 会话ID（可选）
    pub session_id: Option<String>,
    /// 优先级（可选）
    pub priority: Option<String>,
    /// 自定义参数
    pub custom_parameters: Option<HashMap<String, serde_json::Value>>,
}

/// 智能查询响应
#[derive(Debug, Serialize)]
pub struct IntelligentQueryResponse {
    /// 请求ID
    pub request_id: String,
    /// 执行ID
    pub execution_id: String,
    /// 选择的架构
    pub selected_architecture: String,
    /// 任务类型
    pub task_type: String,
    /// 复杂度
    pub complexity: String,
    /// 选择理由
    pub reasoning: String,
    /// 置信度
    pub confidence: f32,
    /// 预估执行时长（秒）
    pub estimated_duration: Option<u64>,
    /// 工作流状态
    pub workflow_status: String,
    /// 开始时间
    pub started_at: String,
}

/// 执行状态查询请求
#[derive(Debug, Deserialize)]
pub struct ExecutionStatusRequest {
    /// 执行ID或请求ID
    pub id: String,
    /// ID类型："execution_id" 或 "request_id"
    pub id_type: String,
}

/// 执行状态响应
#[derive(Debug, Serialize)]
pub struct ExecutionStatusResponse {
    /// 执行ID
    pub execution_id: String,
    /// 请求ID
    pub request_id: String,
    /// 当前状态
    pub status: String,
    /// 进度百分比
    pub progress: f32,
    /// 当前步骤
    pub current_step: Option<String>,
    /// 已完成步骤数
    pub completed_steps: u32,
    /// 总步骤数
    pub total_steps: u32,
    /// 开始时间
    pub started_at: String,
    /// 完成时间（如果已完成）
    pub completed_at: Option<String>,
    /// 执行结果（如果已完成）
    pub result: Option<serde_json::Value>,
    /// 错误信息（如果失败）
    pub error: Option<String>,
}

/// 历史记录查询请求
#[derive(Debug, Deserialize)]
pub struct ExecutionHistoryRequest {
    /// 用户ID（可选）
    pub user_id: Option<String>,
    /// 架构类型过滤（可选）
    pub architecture: Option<String>,
    /// 状态过滤（可选）
    pub status: Option<String>,
    /// 页码
    pub page: Option<u32>,
    /// 每页大小
    pub page_size: Option<u32>,
    /// 开始时间（可选）
    pub start_time: Option<String>,
    /// 结束时间（可选）
    pub end_time: Option<String>,
}

/// 历史记录响应
#[derive(Debug, Serialize)]
pub struct ExecutionHistoryResponse {
    /// 历史记录列表
    pub records: Vec<ExecutionHistoryItem>,
    /// 总数量
    pub total: u32,
    /// 当前页
    pub page: u32,
    /// 每页大小
    pub page_size: u32,
    /// 总页数
    pub total_pages: u32,
}

/// 历史记录项
#[derive(Debug, Serialize)]
pub struct ExecutionHistoryItem {
    /// 请求ID
    pub request_id: String,
    /// 执行ID
    pub execution_id: String,
    /// 用户输入
    pub user_input: String,
    /// 选择的架构
    pub architecture: String,
    /// 任务类型
    pub task_type: String,
    /// 复杂度
    pub complexity: String,
    /// 执行状态
    pub status: String,
    /// 执行时长（秒）
    pub execution_time: Option<u64>,
    /// 成功率
    pub success_rate: Option<f32>,
    /// 开始时间
    pub started_at: String,
    /// 完成时间
    pub completed_at: Option<String>,
}

/// 智能处理用户查询
#[tauri::command]
pub async fn intelligent_process_query(
    request: IntelligentQueryRequest,
    dispatcher_state: State<'_, IntelligentDispatcherState>,
) -> Result<CommandResponse<IntelligentQueryResponse>, String> {
    info!("🚀 [智能调度器] 开始处理用户查询: {}", request.user_input);
    debug!("📋 [智能调度器] 请求参数: {:?}", request);
    
    // 获取调度器实例
    let dispatcher = {
        let state = dispatcher_state.read().await;
        match state.as_ref() {
            Some(dispatcher) => {
                info!("✅ [智能调度器] 服务已初始化，继续处理");
                dispatcher.clone()
            }
            None => {
                error!("❌ [智能调度器] 服务未初始化");
                return Ok(CommandResponse::error("智能调度器服务未初始化".to_string()));
            }
        }
    };
    
    // 执行智能查询处理
    let mut dispatcher_guard = dispatcher.write().await;
    
    match dispatcher_guard.process_query(&request.user_input).await {
        Ok(dispatch_result) => {
            info!("🎉 [智能调度器] 查询处理成功，请求ID: {}", dispatch_result.request_id);
            
            let response = IntelligentQueryResponse {
                request_id: dispatch_result.request_id.clone(),
                execution_id: dispatch_result.execution_id.clone(),
                selected_architecture: format!("{:?}", dispatch_result.decision.architecture),
                task_type: format!("{:?}", dispatch_result.decision.task_type),
                complexity: format!("{:?}", dispatch_result.decision.complexity),
                reasoning: dispatch_result.decision.reasoning.clone(),
                confidence: dispatch_result.decision.confidence,
                estimated_duration: dispatch_result.decision.estimated_duration,
                workflow_status: format!("{:?}", dispatch_result.status),
                started_at: dispatch_result.started_at.to_rfc3339(),
            };
            
            debug!("📤 [智能调度器] 响应数据: {:?}", response);
            Ok(CommandResponse::success(response))
        }
        Err(e) => {
            error!("💥 [智能调度器] 查询处理失败: {}", e);
            Ok(CommandResponse::error(format!("查询处理失败: {}", e)))
        }
    }
}

/// 获取执行状态
#[tauri::command]
pub async fn get_execution_status(
    request: ExecutionStatusRequest,
    dispatcher_state: State<'_, IntelligentDispatcherState>,
) -> Result<CommandResponse<ExecutionStatusResponse>, String> {
    info!("🔍 [智能调度器] 查询执行状态: {} ({})", request.id, request.id_type);
    
    // 获取调度器实例
    let dispatcher = {
        let state = dispatcher_state.read().await;
        match state.as_ref() {
            Some(dispatcher) => dispatcher.clone(),
            None => {
                error!("❌ [智能调度器] 服务未初始化");
                return Ok(CommandResponse::error("智能调度器服务未初始化".to_string()));
            }
        }
    };
    
    // 实现执行状态查询逻辑
    let dispatcher_guard = dispatcher.read().await;
    
    match dispatcher_guard.get_execution_status(&request.id).await {
        Ok(status) => {
            let response = ExecutionStatusResponse {
                execution_id: status.execution_id,
                request_id: status.request_id,
                status: format!("{:?}", status.status),
                progress: status.progress,
                current_step: status.current_step,
                completed_steps: status.completed_steps,
                total_steps: status.total_steps,
                started_at: status.started_at.to_rfc3339(),
                completed_at: status.completed_at.map(|t| t.to_rfc3339()),
                result: status.result,
                error: status.error,
            };
            
            info!("✅ [智能调度器] 执行状态查询成功: {}", request.id);
            Ok(CommandResponse::success(response))
        }
        Err(e) => {
            error!("💥 [智能调度器] 执行状态查询失败: {}", e);
            Ok(CommandResponse::error(format!("执行状态查询失败: {}", e)))
        }
    }
}

/// 获取执行历史
#[tauri::command]
pub async fn get_execution_history(
    request: ExecutionHistoryRequest,
    dispatcher_state: State<'_, IntelligentDispatcherState>,
) -> Result<CommandResponse<ExecutionHistoryResponse>, String> {
    info!("📚 [智能调度器] 查询执行历史");
    debug!("📋 [智能调度器] 历史查询参数: {:?}", request);
    
    // 获取调度器实例
    let dispatcher = {
        let state = dispatcher_state.read().await;
        match state.as_ref() {
            Some(dispatcher) => dispatcher.clone(),
            None => {
                error!("❌ [智能调度器] 服务未初始化");
                return Ok(CommandResponse::error("智能调度器服务未初始化".to_string()));
            }
        }
    };
    
    // 实现执行历史查询逻辑
    let dispatcher_guard = dispatcher.read().await;
    
    match dispatcher_guard.get_execution_history(
        request.user_id.as_deref(),
        request.architecture.as_deref(),
        request.status.as_deref(),
        request.page.unwrap_or(1),
        request.page_size.unwrap_or(10),
        request.start_time.as_deref(),
        request.end_time.as_deref(),
    ).await {
        Ok(history) => {
            let records: Vec<ExecutionHistoryItem> = history.records.into_iter().map(|item| {
                ExecutionHistoryItem {
                    request_id: item.request_id,
                    execution_id: item.execution_id,
                    user_input: item.user_input,
                    architecture: item.architecture,
                    task_type: item.task_type,
                    complexity: item.complexity,
                    status: item.status,
                    execution_time: item.execution_time,
                    success_rate: item.success_rate,
                    started_at: item.started_at,
                    completed_at: item.completed_at,
                }
            }).collect();
            
            let response = ExecutionHistoryResponse {
                records,
                total: history.total,
                page: request.page.unwrap_or(1),
                page_size: request.page_size.unwrap_or(10),
                total_pages: history.total_pages,
            };
            
            info!("✅ [智能调度器] 执行历史查询成功，返回 {} 条记录", response.records.len());
            Ok(CommandResponse::success(response))
        }
        Err(e) => {
            error!("💥 [智能调度器] 执行历史查询失败: {}", e);
            Ok(CommandResponse::error(format!("执行历史查询失败: {}", e)))
        }
    }
}

/// 取消执行
#[tauri::command]
pub async fn cancel_execution(
    execution_id: String,
    dispatcher_state: State<'_, IntelligentDispatcherState>,
) -> Result<CommandResponse<()>, String> {
    info!("🛑 [智能调度器] 取消执行: {}", execution_id);
    
    // 获取调度器实例
    let dispatcher = {
        let state = dispatcher_state.read().await;
        match state.as_ref() {
            Some(dispatcher) => dispatcher.clone(),
            None => {
                error!("❌ [智能调度器] 服务未初始化");
                return Ok(CommandResponse::error("智能调度器服务未初始化".to_string()));
            }
        }
    };
    
    // 实现执行取消逻辑
    let mut dispatcher_guard = dispatcher.write().await;
    
    match dispatcher_guard.cancel_execution(&execution_id).await {
        Ok(_) => {
            info!("✅ [智能调度器] 执行取消成功: {}", execution_id);
            Ok(CommandResponse::success(()))
        }
        Err(e) => {
            error!("💥 [智能调度器] 执行取消失败: {}", e);
            Ok(CommandResponse::error(format!("执行取消失败: {}", e)))
        }
    }
}

/// 获取调度器统计信息
#[tauri::command]
pub async fn get_dispatcher_statistics(
    dispatcher_state: State<'_, IntelligentDispatcherState>,
) -> Result<CommandResponse<DispatcherStatistics>, String> {
    info!("📊 [智能调度器] 获取统计信息");
    
    // 获取调度器实例
    let dispatcher = {
        let state = dispatcher_state.read().await;
        match state.as_ref() {
            Some(dispatcher) => dispatcher.clone(),
            None => {
                error!("❌ [智能调度器] 服务未初始化");
                return Ok(CommandResponse::error("智能调度器服务未初始化".to_string()));
            }
        }
    };
    
    // 实现统计信息收集
    let dispatcher_guard = dispatcher.read().await;
    
    match dispatcher_guard.get_statistics().await {
        Ok(stats) => {
            let response = DispatcherStatistics {
                total_requests: stats.total_requests,
                successful_requests: stats.successful_requests,
                failed_requests: stats.failed_requests,
                average_execution_time: stats.average_execution_time,
                architecture_usage: stats.architecture_usage,
                uptime_seconds: stats.uptime_seconds,
            };
            
            info!("✅ [智能调度器] 统计信息获取成功");
            Ok(CommandResponse::success(response))
        }
        Err(e) => {
            error!("💥 [智能调度器] 统计信息获取失败: {}", e);
            Ok(CommandResponse::error(format!("统计信息获取失败: {}", e)))
        }
    }
}

/// 调度器统计信息
#[derive(Debug, Serialize)]
pub struct DispatcherStatistics {
    /// 总请求数
    pub total_requests: u64,
    /// 成功请求数
    pub successful_requests: u64,
    /// 失败请求数
    pub failed_requests: u64,
    /// 平均执行时间（秒）
    pub average_execution_time: f64,
    /// 架构使用统计
    pub architecture_usage: HashMap<String, u64>,
    /// 运行时间（秒）
    pub uptime_seconds: u64,
}