//! Executor 组件 - 执行器
//! 
//! 负责按照计划逐步执行子任务，调用具体的工具和服务

use crate::engines::plan_and_execute::types::*;
use crate::engines::plan_and_execute::tool_interface::{ToolInterface, ToolCall, ToolResult};
use crate::engines::ExecutionError;
use crate::services::database::DatabaseService;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{Mutex, RwLock};
use tokio::time::timeout;
use uuid::Uuid;

/// 执行器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorConfig {
    /// 最大并发执行数
    pub max_concurrent_steps: u32,
    /// 默认超时时间（秒）
    pub default_timeout: u64,
    /// 是否启用步骤缓存
    pub enable_step_caching: bool,
    /// 执行模式
    pub execution_mode: ExecutionMode,
    /// 错误处理策略
    pub error_handling: ErrorHandlingStrategy,
}

/// 执行模式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// 严格模式：任何步骤失败都停止执行
    Strict,
    /// 容错模式：跳过失败的步骤继续执行
    Tolerant,
    /// 最佳努力模式：尽可能多地执行步骤
    BestEffort,
}

/// 错误处理策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorHandlingStrategy {
    /// 立即停止
    StopImmediately,
    /// 重试后停止
    RetryThenStop,
    /// 跳过并继续
    SkipAndContinue,
    /// 标记为失败但继续
    MarkFailedAndContinue,
}

/// 执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// 执行状态
    pub status: TaskStatus,
    /// 已完成的步骤
    pub completed_steps: Vec<String>,
    /// 失败的步骤
    pub failed_steps: Vec<String>,
    /// 跳过的步骤
    pub skipped_steps: Vec<String>,
    /// 步骤结果详情
    pub step_results: HashMap<String, StepResult>,
    /// 执行指标
    pub metrics: ExecutionMetrics,
    /// 错误信息
    pub errors: Vec<ExecutionError>,
}

/// 步骤结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    /// 步骤ID
    pub step_id: String,
    /// 执行状态
    pub status: StepStatus,
    /// 开始时间
    pub started_at: SystemTime,
    /// 结束时间
    pub completed_at: Option<SystemTime>,
    /// 执行时长（毫秒）
    pub duration_ms: u64,
    /// 结果数据
    pub result_data: Option<serde_json::Value>,
    /// 错误信息
    pub error: Option<String>,
    /// 重试次数
    pub retry_count: u32,
    /// 工具调用结果
    pub tool_result: Option<ToolResult>,
}

/// 步骤状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StepStatus {
    /// 待执行
    Pending,
    /// 执行中
    Running,
    /// 已完成
    Completed,
    /// 已失败
    Failed,
    /// 已跳过
    Skipped,
    /// 重试中
    Retrying,
    /// 已取消
    Cancelled,
}

/// 执行指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    /// 总执行时间（毫秒）
    pub total_duration_ms: u64,
    /// 成功步骤数
    pub successful_steps: u32,
    /// 失败步骤数
    pub failed_steps: u32,
    /// 跳过步骤数
    pub skipped_steps: u32,
    /// 重试总次数
    pub total_retries: u32,
    /// 平均步骤执行时间（毫秒）
    pub avg_step_duration_ms: u64,
    /// 并发执行峰值
    pub peak_concurrency: u32,
    /// 自定义指标
    pub custom_metrics: HashMap<String, f64>,
}

/// 执行上下文
#[derive(Debug)]
pub struct ExecutionContext {
    /// 任务ID
    pub task_id: String,
    /// 计划ID
    pub plan_id: String,
    /// 共享数据
    pub shared_data: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    /// 执行状态
    pub execution_state: Arc<RwLock<ExecutionState>>,
    /// 工具接口
    pub tool_interface: Arc<ToolInterface>,
}

/// 执行状态
#[derive(Debug, Clone)]
pub struct ExecutionState {
    /// 当前执行的步骤
    pub current_steps: HashMap<String, StepStatus>,
    /// 已完成的步骤
    pub completed_steps: Vec<String>,
    /// 失败的步骤
    pub failed_steps: Vec<String>,
    /// 是否暂停
    pub is_paused: bool,
    /// 是否取消
    pub is_cancelled: bool,
}

/// 执行器
#[derive(Debug)]
pub struct Executor {
    config: ExecutorConfig,
    context: Arc<Mutex<Option<ExecutionContext>>>,
    metrics: Arc<Mutex<ExecutionMetrics>>,
    db_service: Arc<DatabaseService>,
}

impl Executor {
    /// 创建新的执行器实例
    pub fn new(config: ExecutorConfig, db_service: Arc<DatabaseService>) -> Self {
        Self {
            config,
            context: Arc::new(Mutex::new(None)),
            metrics: Arc::new(Mutex::new(ExecutionMetrics::default())),
            db_service,
        }
    }

    /// 执行计划
    pub async fn execute_plan(
        &self,
        plan: &ExecutionPlan,
        task: &TaskRequest,
    ) -> Result<ExecutionResult, PlanAndExecuteError> {
        log::info!("开始执行计划: {}", plan.name);
        
        // 初始化执行上下文
        self.initialize_context(plan, task).await?;
        
        let start_time = SystemTime::now();
        let mut step_results = HashMap::new();
        let mut errors = Vec::new();
        
        // 根据执行模式执行步骤
        match self.config.execution_mode {
            ExecutionMode::Strict => {
                self.execute_strict_mode(plan, &mut step_results, &mut errors).await?
            },
            ExecutionMode::Tolerant => {
                self.execute_tolerant_mode(plan, &mut step_results, &mut errors).await?
            },
            ExecutionMode::BestEffort => {
                self.execute_best_effort_mode(plan, &mut step_results, &mut errors).await?
            },
        }
        
        // 计算最终结果
        let execution_result = self.build_execution_result(
            start_time,
            step_results,
            errors,
        ).await?;
        
        log::info!("计划执行完成: {}, 状态: {:?}", plan.name, execution_result.status);
        Ok(execution_result)
    }

    /// 执行单个步骤
    pub async fn execute_step(
        &self,
        step: &ExecutionStep,
        context: &ExecutionContext,
    ) -> Result<StepResult, PlanAndExecuteError> {
        log::debug!("开始执行步骤: {}", step.name);
        
        let start_time = SystemTime::now();
        let mut retry_count = 0;
        
        loop {
            // 检查是否暂停或取消
            self.check_execution_state(context).await?;
            
            match self.execute_step_once(step, context).await {
                Ok(result) => {
                    log::debug!("步骤执行成功: {}", step.name);
                    return Ok(StepResult {
                        step_id: step.id.clone(),
                        status: StepStatus::Completed,
                        started_at: start_time,
                        completed_at: Some(SystemTime::now()),
                        duration_ms: start_time.elapsed().unwrap_or_default().as_millis() as u64,
                        result_data: Some(result.clone()),
                        error: None,
                        retry_count,
                        tool_result: None,
                    });
                },
                Err(error) => {
                    retry_count += 1;
                    
                    if retry_count <= step.retry_config.max_retries {
                        log::warn!("步骤执行失败，准备重试 ({}/{}): {}", 
                                 retry_count, step.retry_config.max_retries, error);
                        
                        // 等待重试间隔
                        let delay = self.calculate_retry_delay(&step.retry_config, retry_count);
                        tokio::time::sleep(Duration::from_secs(delay)).await;
                        
                        continue;
                    } else {
                        log::error!("步骤执行失败，已达到最大重试次数: {}", error);
                        return Ok(StepResult {
                            step_id: step.id.clone(),
                            status: StepStatus::Failed,
                            started_at: start_time,
                            completed_at: Some(SystemTime::now()),
                            duration_ms: start_time.elapsed().unwrap_or_default().as_millis() as u64,
                            result_data: None,
                            error: Some(error.to_string()),
                            retry_count,
                            tool_result: None,
                        });
                    }
                }
            }
        }
    }

    /// 暂停执行
    pub async fn pause(&self) -> Result<(), PlanAndExecuteError> {
        let context_guard = self.context.lock().await;
        if let Some(context) = context_guard.as_ref() {
            let mut state = context.execution_state.write().await;
            state.is_paused = true;
            log::info!("执行已暂停");
        }
        Ok(())
    }

    /// 恢复执行
    pub async fn resume(&self) -> Result<(), PlanAndExecuteError> {
        let context_guard = self.context.lock().await;
        if let Some(context) = context_guard.as_ref() {
            let mut state = context.execution_state.write().await;
            state.is_paused = false;
            log::info!("执行已恢复");
        }
        Ok(())
    }

    /// 取消执行
    pub async fn cancel(&self) -> Result<(), PlanAndExecuteError> {
        let context_guard = self.context.lock().await;
        if let Some(context) = context_guard.as_ref() {
            let mut state = context.execution_state.write().await;
            state.is_cancelled = true;
            log::info!("执行已取消");
        }
        Ok(())
    }

    /// 获取执行状态
    pub async fn get_execution_status(&self) -> Option<ExecutionState> {
        let context_guard = self.context.lock().await;
        if let Some(context) = context_guard.as_ref() {
            Some(context.execution_state.read().await.clone())
        } else {
            None
        }
    }

    // 私有方法实现
    
    async fn initialize_context(
        &self,
        plan: &ExecutionPlan,
        task: &TaskRequest,
    ) -> Result<(), PlanAndExecuteError> {
        let shared_data = Arc::new(RwLock::new(HashMap::new()));
        let execution_state = Arc::new(RwLock::new(ExecutionState {
            current_steps: HashMap::new(),
            completed_steps: Vec::new(),
            failed_steps: Vec::new(),
            is_paused: false,
            is_cancelled: false,
        }));
        
        let tool_interface = Arc::new(ToolInterface::new(self.db_service.clone()).await?);
        
        let mut context = self.context.lock().await;
        *context = Some(ExecutionContext {
            task_id: task.id.clone(),
            plan_id: plan.id.clone(),
            shared_data,
            execution_state,
            tool_interface,
        });
        
        Ok(())
    }

    async fn execute_strict_mode(
        &self,
        plan: &ExecutionPlan,
        step_results: &mut HashMap<String, StepResult>,
        errors: &mut Vec<ExecutionError>,
    ) -> Result<(), PlanAndExecuteError> {
        let context_guard = self.context.lock().await;
        let context = context_guard.as_ref().unwrap();
        
        for step in &plan.steps {
            let result = self.execute_step(step, context).await?;
            
            if result.status == StepStatus::Failed {
                step_results.insert(step.id.clone(), result);
                return Err(PlanAndExecuteError::ExecutionFailed(
                    format!("步骤 '{}' 执行失败，严格模式下停止执行", step.name)
                ));
            }
            
            step_results.insert(step.id.clone(), result);
        }
        
        Ok(())
    }

    async fn execute_tolerant_mode(
        &self,
        plan: &ExecutionPlan,
        step_results: &mut HashMap<String, StepResult>,
        errors: &mut Vec<ExecutionError>,
    ) -> Result<(), PlanAndExecuteError> {
        let context_guard = self.context.lock().await;
        let context = context_guard.as_ref().unwrap();
        
        for step in &plan.steps {
            let result = self.execute_step(step, context).await?;
            
            if result.status == StepStatus::Failed {
                log::warn!("步骤 '{}' 执行失败，容错模式下跳过并继续", step.name);
                errors.push(ExecutionError {
                    error_type: crate::engines::types::ErrorType::Tool,
                    message: format!("步骤 '{}' 执行失败", step.name),
                    details: result.error.clone(),
                    error_code: None,
                    retryable: false,
                    timestamp: SystemTime::now(),
                });
            }
            
            step_results.insert(step.id.clone(), result);
        }
        
        Ok(())
    }

    async fn execute_best_effort_mode(
        &self,
        plan: &ExecutionPlan,
        step_results: &mut HashMap<String, StepResult>,
        errors: &mut Vec<ExecutionError>,
    ) -> Result<(), PlanAndExecuteError> {
        let context_guard = self.context.lock().await;
        let context = context_guard.as_ref().unwrap();
        
        // 简化实现：暂时不支持真正的并行执行，改为顺序执行
        // 避免使用 unsafe 代码和原始指针
        for step in &plan.steps {
            let result = self.execute_step(step, context).await?;
            
            if result.status == StepStatus::Failed {
                log::warn!("步骤 '{}' 执行失败，最佳努力模式下记录错误并继续", step.name);
                errors.push(ExecutionError {
                    error_type: crate::engines::types::ErrorType::Tool,
                    message: format!("步骤 '{}' 执行失败", step.name),
                    details: result.error.clone(),
                    error_code: None,
                    retryable: false,
                    timestamp: SystemTime::now(),
                });
            }
            
            step_results.insert(step.id.clone(), result);
        }
        
        Ok(())
    }

    async fn execute_step_once(
        &self,
        step: &ExecutionStep,
        context: &ExecutionContext,
    ) -> Result<serde_json::Value, PlanAndExecuteError> {
        match &step.step_type {
            StepType::ToolCall => {
                self.execute_tool_call(step, context).await
            },
            StepType::AiReasoning => {
                self.execute_ai_reasoning(step, context).await
            },
            StepType::DataProcessing => {
                self.execute_data_processing(step, context).await
            },
            StepType::Conditional => {
                self.execute_conditional(step, context).await
            },
            StepType::Parallel => {
                self.execute_parallel(step, context).await
            },
            StepType::Wait => {
                self.execute_wait(step, context).await
            },
            StepType::ManualConfirmation => {
                self.execute_manual_confirmation(step, context).await
            },
        }
    }

    async fn execute_tool_call(
        &self,
        step: &ExecutionStep,
        context: &ExecutionContext,
    ) -> Result<serde_json::Value, PlanAndExecuteError> {
        if let Some(tool_config) = &step.tool_config {
            let tool_call = ToolCall {
                id: Uuid::new_v4().to_string(),
                tool_name: tool_config.tool_name.clone(),
                parameters: step.parameters.clone(),
                timeout: Some(tool_config.timeout.unwrap_or(self.config.default_timeout)),
                retry_config: None,
                context: None,
            };
            
            let timeout_duration = Duration::from_secs(
                tool_config.timeout.unwrap_or(self.config.default_timeout)
            );
            
            match timeout(timeout_duration, context.tool_interface.call_tool(tool_call)).await {
                Ok(Ok(result)) => Ok(result.result),
                Ok(Err(error)) => Err(PlanAndExecuteError::ToolFailed(error.to_string())),
                Err(_) => Err(PlanAndExecuteError::ToolFailed("工具调用超时".to_string())),
            }
        } else {
            Err(PlanAndExecuteError::ConfigError("工具调用步骤缺少工具配置".to_string()))
        }
    }

    async fn execute_ai_reasoning(
        &self,
        step: &ExecutionStep,
        _context: &ExecutionContext,
    ) -> Result<serde_json::Value, PlanAndExecuteError> {
        // AI推理步骤的实现
        log::info!("执行AI推理步骤: {}", step.name);
        Ok(serde_json::json!({
            "reasoning_result": "AI推理完成",
            "step_name": step.name
        }))
    }

    async fn execute_data_processing(
        &self,
        step: &ExecutionStep,
        context: &ExecutionContext,
    ) -> Result<serde_json::Value, PlanAndExecuteError> {
        // 数据处理步骤的实现
        log::info!("执行数据处理步骤: {}", step.name);
        
        let shared_data = context.shared_data.read().await;
        let processed_data = serde_json::json!({
            "processed": true,
            "step_name": step.name,
            "data_count": shared_data.len()
        });
        
        Ok(processed_data)
    }

    async fn execute_conditional(
        &self,
        step: &ExecutionStep,
        _context: &ExecutionContext,
    ) -> Result<serde_json::Value, PlanAndExecuteError> {
        // 条件判断步骤的实现
        log::info!("执行条件判断步骤: {}", step.name);
        Ok(serde_json::json!({
            "condition_result": true,
            "step_name": step.name
        }))
    }

    async fn execute_parallel(
        &self,
        step: &ExecutionStep,
        _context: &ExecutionContext,
    ) -> Result<serde_json::Value, PlanAndExecuteError> {
        // 并行执行步骤的实现
        log::info!("执行并行步骤: {}", step.name);
        Ok(serde_json::json!({
            "parallel_result": "并行执行完成",
            "step_name": step.name
        }))
    }

    async fn execute_wait(
        &self,
        step: &ExecutionStep,
        _context: &ExecutionContext,
    ) -> Result<serde_json::Value, PlanAndExecuteError> {
        // 等待步骤的实现
        let wait_time = step.parameters.get("wait_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(5);
        
        log::info!("执行等待步骤: {}，等待 {} 秒", step.name, wait_time);
        tokio::time::sleep(Duration::from_secs(wait_time)).await;
        
        Ok(serde_json::json!({
            "wait_completed": true,
            "wait_seconds": wait_time,
            "step_name": step.name
        }))
    }

    async fn execute_manual_confirmation(
        &self,
        step: &ExecutionStep,
        _context: &ExecutionContext,
    ) -> Result<serde_json::Value, PlanAndExecuteError> {
        // 人工确认步骤的实现
        log::info!("执行人工确认步骤: {}", step.name);
        
        // 在实际实现中，这里应该等待用户确认
        // 现在简化为自动确认
        Ok(serde_json::json!({
            "confirmed": true,
            "step_name": step.name,
            "confirmation_time": SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
        }))
    }

    async fn check_execution_state(
        &self,
        context: &ExecutionContext,
    ) -> Result<(), PlanAndExecuteError> {
        loop {
            let state = context.execution_state.read().await;
            
            if state.is_cancelled {
                return Err(PlanAndExecuteError::ExecutionFailed("执行已被取消".to_string()));
            }
            
            if !state.is_paused {
                break;
            }
            
            drop(state);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        Ok(())
    }

    fn calculate_retry_delay(&self, retry_config: &RetryConfig, retry_count: u32) -> u64 {
        match retry_config.backoff_strategy {
            BackoffStrategy::Fixed => retry_config.retry_interval,
            BackoffStrategy::Linear => retry_config.retry_interval * retry_count as u64,
            BackoffStrategy::Exponential => {
                retry_config.retry_interval * (2_u64.pow(retry_count - 1))
            },
        }
    }

    fn can_execute_in_parallel(&self, _step: &ExecutionStep, _plan: &ExecutionPlan) -> bool {
        // 简化的并行判断逻辑
        // 实际应该检查步骤依赖关系
        false
    }

    async fn build_execution_result(
        &self,
        start_time: SystemTime,
        step_results: HashMap<String, StepResult>,
        errors: Vec<ExecutionError>,
    ) -> Result<ExecutionResult, PlanAndExecuteError> {
        let total_duration = start_time.elapsed().unwrap_or_default().as_millis() as u64;
        
        let mut completed_steps = Vec::new();
        let mut failed_steps = Vec::new();
        let mut skipped_steps = Vec::new();
        let mut successful_count = 0;
        let mut failed_count = 0;
        let mut skipped_count = 0;
        let mut total_retries = 0;
        
        for (step_id, result) in &step_results {
            total_retries += result.retry_count;
            
            match result.status {
                StepStatus::Completed => {
                    completed_steps.push(step_id.clone());
                    successful_count += 1;
                },
                StepStatus::Failed => {
                    failed_steps.push(step_id.clone());
                    failed_count += 1;
                },
                StepStatus::Skipped => {
                    skipped_steps.push(step_id.clone());
                    skipped_count += 1;
                },
                _ => {},
            }
        }
        
        let status = if failed_count > 0 {
            TaskStatus::Failed
        } else if skipped_count > 0 {
            TaskStatus::Completed // 部分完成
        } else {
            TaskStatus::Completed
        };
        
        let avg_duration = if step_results.len() > 0 {
            total_duration / step_results.len() as u64
        } else {
            0
        };
        
        let metrics = ExecutionMetrics {
            total_duration_ms: total_duration,
            successful_steps: successful_count,
            failed_steps: failed_count,
            skipped_steps: skipped_count,
            total_retries,
            avg_step_duration_ms: avg_duration,
            peak_concurrency: 1, // 简化实现
            custom_metrics: HashMap::new(),
        };
        
        // 更新内部指标
        *self.metrics.lock().await = metrics.clone();
        
        Ok(ExecutionResult {
            status,
            completed_steps,
            failed_steps,
            skipped_steps,
            step_results: step_results.into_iter().map(|(k, v)| (k, v)).collect(),
            metrics,
            errors,
        })
    }
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            max_concurrent_steps: 4,
            default_timeout: 300, // 5分钟
            enable_step_caching: true,
            execution_mode: ExecutionMode::Tolerant,
            error_handling: ErrorHandlingStrategy::RetryThenStop,
        }
    }
}

impl Default for ExecutionMetrics {
    fn default() -> Self {
        Self {
            total_duration_ms: 0,
            successful_steps: 0,
            failed_steps: 0,
            skipped_steps: 0,
            total_retries: 0,
            avg_step_duration_ms: 0,
            peak_concurrency: 0,
            custom_metrics: HashMap::new(),
        }
    }
}

// 为ExecutionContext实现Clone
impl Clone for ExecutionContext {
    fn clone(&self) -> Self {
        Self {
            task_id: self.task_id.clone(),
            plan_id: self.plan_id.clone(),
            shared_data: Arc::clone(&self.shared_data),
            execution_state: Arc::clone(&self.execution_state),
            tool_interface: Arc::clone(&self.tool_interface),
        }
    }
}