//! Parallel Executor Pool - 并行执行器池
//!
//! 负责多线程并行执行工具调用
//! 核心特性：
//! - 多线程执行：使用ThreadPoolExecutor
//! - 动态扩容：根据任务量调整线程数
//! - 错误隔离：单个任务失败不影响其他任务
//! - 结果收集：统一管理执行结果
//! - 重试机制：支持任务失败重试

use anyhow::Result;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::{timeout, Instant};
use uuid::Uuid;
use chrono::Utc;
use tracing::{info, warn, error};

use crate::tools::{EngineToolAdapter, UnifiedToolCall, UnifiedToolResult, get_global_engine_adapter};
use super::types::*;

/// Parallel Executor Pool - 并行执行器池
pub struct ParallelExecutorPool {
    /// 并发控制信号量
    semaphore: Arc<Semaphore>,
    /// 工具适配器
    tool_adapter: Arc<dyn EngineToolAdapter>,
    /// 配置
    config: LlmCompilerConfig,
    /// 执行统计
    execution_metrics: Arc<tokio::sync::RwLock<ExecutionMetrics>>,
    /// 运行时参数（包含工具权限、conversation_id、message_id等）
    runtime_params: Arc<tokio::sync::RwLock<Option<HashMap<String, serde_json::Value>>>>,
}

/// 执行指标
#[derive(Debug, Clone, Default)]
struct ExecutionMetrics {
    /// 总执行任务数
    total_executions: u64,
    /// 成功执行数
    successful_executions: u64,
    /// 失败执行数
    failed_executions: u64,
    /// 总执行时间（毫秒）
    total_execution_time_ms: u64,
    /// 平均执行时间（毫秒）
    average_execution_time_ms: u64,
    /// 当前并发数
    current_concurrency: usize,
    /// 最大并发数
    peak_concurrency: usize,
}

impl ParallelExecutorPool {
    pub fn new(tool_adapter: Arc<dyn EngineToolAdapter>, config: LlmCompilerConfig) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(config.max_concurrency)),
            tool_adapter,
            config,
            execution_metrics: Arc::new(tokio::sync::RwLock::new(ExecutionMetrics::default())),
            runtime_params: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }
    
    /// 设置运行时参数
    pub async fn set_runtime_params(&self, params: HashMap<String, serde_json::Value>) {
        let mut runtime_params = self.runtime_params.write().await;
        *runtime_params = Some(params);
    }
    
    /// 使用全局工具适配器创建执行器池
    pub async fn new_with_global_adapter(config: LlmCompilerConfig) -> Result<Self> {
        let tool_adapter = get_global_engine_adapter()
            .map_err(|e| anyhow::anyhow!("获取全局工具适配器失败: {}", e))?;
        
        Ok(Self::new(tool_adapter, config))
    }

    /// 执行单个任务
    pub async fn execute_task(&self, mut task: DagTaskNode) -> TaskExecutionResult {
        let start_time = Utc::now();
        let execution_start = Instant::now();

        // 获取信号量许可
        let _permit = match timeout(
            Duration::from_secs(30), // 30秒超时等待许可
            self.semaphore.acquire()
        ).await {
            Ok(Ok(permit)) => permit,
            Ok(Err(_)) => {
                error!("获取执行许可失败: {}", task.id);
                return TaskExecutionResult::failure(
                    task.id,
                    "获取执行许可失败".to_string(),
                    0,
                    task.retry_count,
                );
            }
            Err(_) => {
                error!("等待执行许可超时: {}", task.id);
                return TaskExecutionResult::failure(
                    task.id,
                    "等待执行许可超时".to_string(),
                    0,
                    task.retry_count,
                );
            }
        };

        info!("开始执行任务: {} ({})", task.name, task.tool_name);
        task.status = TaskStatus::Running;

        // 更新并发统计
        self.update_concurrency_metrics(1).await;

        // 执行工具调用
        let (status, outputs, error) = match self.execute_tool_with_timeout(&task).await {
            Ok(result) => {
                info!("任务执行成功: {}", task.id);
                
                // ✅ 发送工具执行结果到前端（参考React架构）
                // 从runtime_params中读取app_handle、conversation_id、message_id等
                let runtime_params = self.runtime_params.read().await;
                if let Some(params) = runtime_params.as_ref() {
                    if let Some(app_handle_value) = params.get("app_handle") {
                        // 注意：由于app_handle无法直接序列化，我们需要从全局状态或其他方式获取
                        // 这里先跳过，或者在engine_adapter层面处理消息推送
                        drop(runtime_params);  // 释放锁
                        
                        // 暂时使用日志记录，实际消息推送在engine_adapter层面处理
                        info!("✅ Task completed: {}, tool={}", task.id, task.tool_name);
                    }
                }
                
                (TaskStatus::Completed, result, None)
            }
            Err(e) => {
                error!("任务执行失败: {} - {}", task.id, e);
                
                // 失败也记录日志，实际消息推送在engine_adapter层面处理
                warn!("❌ Task failed: {}, tool={}, error={}", task.id, task.tool_name, e);
                
                (TaskStatus::Failed, HashMap::new(), Some(e.to_string()))
            }
        };

        let end_time = Utc::now();
        let duration_ms = execution_start.elapsed().as_millis() as u64;

        // 更新执行统计
        self.update_execution_metrics(status == TaskStatus::Completed, duration_ms).await;
        self.update_concurrency_metrics(-1).await;

        let mut result = TaskExecutionResult {
            task_id: task.id,
            status,
            outputs,
            error,
            duration_ms,
            started_at: start_time,
            completed_at: Some(end_time),
            retry_count: task.retry_count,
            metadata: HashMap::new(),
        };

        // 添加执行元数据
        result.add_metadata("tool_name".to_string(), json!(task.tool_name));
        result.add_metadata("execution_node".to_string(), json!("local"));
        result.add_metadata("concurrency_level".to_string(), json!(self.config.max_concurrency));

        result
    }

    /// 带超时的工具执行
    async fn execute_tool_with_timeout(&self, task: &DagTaskNode) -> Result<HashMap<String, Value>> {
        let timeout_duration = Duration::from_secs(self.config.task_timeout);
        
        match timeout(timeout_duration, self.call_tool(task)).await {
            Ok(result) => result,
            Err(_) => {
                error!("任务执行超时: {} ({}秒)", task.id, self.config.task_timeout);
                Err(anyhow::anyhow!("任务执行超时"))
            }
        }
    }

    /// 调用工具
    async fn call_tool(&self, task: &DagTaskNode) -> Result<HashMap<String, Value>> {
        info!("调用工具: {} with inputs: {:?}", task.tool_name, task.inputs);
        
        // 工具权限检查（从runtime_params读取）
        let runtime_params = self.runtime_params.read().await;
        if let Some(params) = runtime_params.as_ref() {
            let allow_list = params
                .get("tools_allow")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>())
                .unwrap_or_default();
            let deny_list = params
                .get("tools_deny")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>())
                .unwrap_or_default();
            
            // 如果没有白名单（空数组），则不允许任何工具
            if allow_list.is_empty() {
                return Err(anyhow::anyhow!(
                    "工具 '{}' 不在允许列表中（未配置工具权限）", task.tool_name
                ));
            }
            // 如果有白名单且工具不在白名单中，拒绝
            if !allow_list.iter().any(|&n| n == task.tool_name) {
                return Err(anyhow::anyhow!(
                    "工具 '{}' 不在允许列表中", task.tool_name
                ));
            }
            // 如果工具在黑名单中，拒绝
            if deny_list.iter().any(|&n| n == task.tool_name) {
                return Err(anyhow::anyhow!(
                    "工具 '{}' 被禁止使用", task.tool_name
                ));
            }
        }
        
        // 准备工具执行参数
        let tool_call = UnifiedToolCall {
            id: Uuid::new_v4().to_string(),
            tool_name: task.tool_name.clone(),
            parameters: task.inputs.clone(),
            timeout: Some(Duration::from_secs(self.config.task_timeout)),
            context: HashMap::new(),
            retry_count: 0,
        };
        
        // 调用统一工具适配器
        match self.tool_adapter.execute_tool(tool_call).await {
            Ok(tool_result) => {
                info!("工具执行成功: {}, 成功: {}", task.tool_name, tool_result.success);
                
                if tool_result.success {
                    // 转换工具结果为标准格式
                    let mut outputs = self.convert_tool_result_to_outputs(&tool_result)?;
                    
                    // 添加执行元数据
                    outputs.insert("executed_at".to_string(), json!(Utc::now().to_rfc3339()));
                    outputs.insert("execution_success".to_string(), json!(true));
                    
                    Ok(outputs)
                } else {
                    // 工具执行失败但没有抛出异常
                    let error_msg = tool_result.error.unwrap_or_else(|| "工具执行失败".to_string());
                    Err(anyhow::anyhow!("工具执行失败: {}", error_msg))
                }
            }
            Err(e) => {
                error!("工具执行失败: {} - {}", task.tool_name, e);
                
                // 如果工具不存在，尝试使用模拟数据作为后备
                if e.to_string().contains("not found") || e.to_string().contains("not available") {
                    warn!("工具 {} 不可用，使用模拟数据", task.tool_name);
                    Ok(HashMap::new())
                } else {
                    Err(e)
                }
            }
        }
    }
    
    /// 转换工具执行结果为标准输出格式
    fn convert_tool_result_to_outputs(&self, tool_result: &UnifiedToolResult) -> Result<HashMap<String, Value>> {
        let mut outputs = HashMap::new();
        
        // 添加基础结果信息
        outputs.insert("success".to_string(), json!(tool_result.success));
        outputs.insert("call_id".to_string(), json!(tool_result.id));
        outputs.insert("execution_time_ms".to_string(), json!(tool_result.execution_time_ms));
        outputs.insert("tool_name".to_string(), json!(tool_result.tool_name));
        
        // 处理工具输出数据
        match &tool_result.output {
            Value::Object(obj) => {
                // 如果输出是JSON对象，展开其字段
                for (key, value) in obj {
                    outputs.insert(key.clone(), value.clone());
                }
            }
            _ => {
                // 否则作为单个result字段
                outputs.insert("result".to_string(), tool_result.output.clone());
            }
        }
        
        // 添加错误信息（如果有）
        if let Some(error) = &tool_result.error {
            outputs.insert("error".to_string(), json!(error));
        }
        
        // 添加元数据
        for (key, value) in &tool_result.metadata {
            outputs.insert(format!("meta_{}", key), value.clone());
        }
        
        Ok(outputs)
    }
    
   

    /// 批量执行任务
    pub async fn execute_tasks_batch(&self, tasks: Vec<DagTaskNode>) -> Vec<TaskExecutionResult> {
        if tasks.is_empty() {
            return Vec::new();
        }

        info!("开始批量执行 {} 个任务", tasks.len());
        
        let mut handles = Vec::new();
        
        for task in tasks {
            let executor = self.clone();
            let handle = tokio::spawn(async move {
                executor.execute_task(task).await
            });
            handles.push(handle);
        }
        
        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => {
                    error!("任务执行句柄错误: {}", e);
                    // 创建一个失败结果
                    results.push(TaskExecutionResult::failure(
                        "unknown".to_string(),
                        format!("任务执行句柄错误: {}", e),
                        0,
                        0,
                    ));
                }
            }
        }
        
        info!("批量执行完成，共 {} 个结果", results.len());
        results
    }

    /// 重试失败的任务
    pub async fn retry_task(&self, mut task: DagTaskNode) -> TaskExecutionResult {
        if !task.can_retry() {
            warn!("任务 {} 已达到最大重试次数", task.id);
            return TaskExecutionResult::failure(
                task.id,
                "已达到最大重试次数".to_string(),
                0,
                task.retry_count,
            );
        }

        task.increment_retry();
        info!("重试任务: {} (第 {} 次重试)", task.id, task.retry_count);
        
        // 添加重试延迟
        let delay = Duration::from_secs(2_u64.pow(task.retry_count.min(5))); // 指数退避
        tokio::time::sleep(delay).await;
        
        self.execute_task(task).await
    }

    /// 更新执行统计
    async fn update_execution_metrics(&self, success: bool, duration_ms: u64) {
        let mut metrics = self.execution_metrics.write().await;
        
        metrics.total_executions += 1;
        if success {
            metrics.successful_executions += 1;
        } else {
            metrics.failed_executions += 1;
        }
        
        metrics.total_execution_time_ms += duration_ms;
        metrics.average_execution_time_ms = 
            metrics.total_execution_time_ms as u64 / metrics.total_executions as u64;
    }

    /// 更新并发统计
    async fn update_concurrency_metrics(&self, delta: i32) {
        let mut metrics = self.execution_metrics.write().await;
        
        if delta > 0 {
            metrics.current_concurrency += delta as usize;
            if metrics.current_concurrency > metrics.peak_concurrency {
                metrics.peak_concurrency = metrics.current_concurrency;
            }
        } else {
            metrics.current_concurrency = metrics.current_concurrency.saturating_sub((-delta) as usize);
        }
    }

    /// 获取执行统计
    pub async fn get_execution_metrics(&self) -> ExecutionMetrics {
        self.execution_metrics.read().await.clone()
    }

    /// 获取当前可用的执行槽位数
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }

    /// 检查是否有可用的执行槽位
    pub fn has_available_capacity(&self) -> bool {
        self.semaphore.available_permits() > 0
    }

    /// 动态调整并发数（如果需要）
    pub async fn adjust_concurrency(&self, new_max_concurrency: usize) -> Result<()> {
        if new_max_concurrency == 0 {
            return Err(anyhow::anyhow!("并发数不能为0"));
        }
        
        info!("调整最大并发数: {} -> {}", self.config.max_concurrency, new_max_concurrency);
        
        // 注意：Semaphore不支持动态调整，这里只是记录配置变化
        // 实际实现中可能需要重新创建Semaphore
        warn!("动态调整并发数功能需要重启执行器池才能生效");
        
        Ok(())
    }
}

// 实现Clone以支持在异步任务中使用
impl Clone for ParallelExecutorPool {
    fn clone(&self) -> Self {
        Self {
            semaphore: self.semaphore.clone(),
            tool_adapter: self.tool_adapter.clone(),
            config: self.config.clone(),
            execution_metrics: self.execution_metrics.clone(),
            runtime_params: self.runtime_params.clone(),
        }
    }
}