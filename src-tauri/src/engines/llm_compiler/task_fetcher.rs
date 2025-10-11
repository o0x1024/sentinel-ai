//! Task Fetching Unit - 智能任务调度单元
//!
//! 负责DAG任务的智能调度和依赖管理
//! 核心功能：
//! - 依赖解析：分析任务间的依赖关系
//! - 并行调度：将无依赖的任务并行执行
//! - 阻塞管理：等待依赖完成后再执行
//! - 变量替换：将$1等占位符替换为实际结果

use anyhow::Result;
use serde_json::Value;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc, Notify};
use tokio::task::AbortHandle;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn, debug};

use super::types::*;

/// 调度事件类型
#[derive(Debug, Clone)]
pub enum SchedulingEvent {
    /// 任务完成事件
    TaskCompleted {
        task_id: String,
        result: TaskExecutionResult,
    },
    /// 任务失败事件
    TaskFailed {
        task_id: String,
        error: String,
        retry_count: u32,
    },
    /// 新任务添加事件
    TaskAdded {
        task: DagTaskNode,
    },
    /// 停止调度事件
    Shutdown,
}

/// Task Fetching Unit - 智能任务调度单元
pub struct TaskFetchingUnit {
    /// 等待队列（等待依赖的任务）
    waiting_queue: Arc<RwLock<VecDeque<DagTaskNode>>>,
    /// 就绪队列（可立即执行的任务）
    ready_queue: Arc<RwLock<VecDeque<DagTaskNode>>>,
    /// 正在执行的任务
    executing_tasks: Arc<RwLock<HashMap<String, AbortHandle>>>,
    /// 已完成的任务结果
    completed_tasks: Arc<RwLock<HashMap<String, TaskExecutionResult>>>,
    /// 失败的任务
    failed_tasks: Arc<RwLock<HashMap<String, TaskExecutionResult>>>,
    /// 依赖关系图
    dependency_graph: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// 变量解析上下文
    variable_context: Arc<RwLock<VariableResolutionContext>>,
    /// 配置
    config: LlmCompilerConfig,
    /// 事件发送器
    event_sender: mpsc::UnboundedSender<SchedulingEvent>,
    /// 事件接收器
    event_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<SchedulingEvent>>>>,
    /// 取消令牌
    cancellation_token: CancellationToken,
    /// 就绪通知器
    ready_notify: Arc<Notify>,
    /// 工具调用缓存
    tool_cache: Arc<RwLock<crate::engines::llm_compiler::types::ToolCallCache>>,
}

impl TaskFetchingUnit {
    pub fn new(config: LlmCompilerConfig) -> Self {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        
        // 创建缓存管理器，最大1000个条目，默认TTL 5分钟
        let tool_cache = crate::engines::llm_compiler::types::ToolCallCache::new(1000, 300);
        
        Self {
            waiting_queue: Arc::new(RwLock::new(VecDeque::new())),
            ready_queue: Arc::new(RwLock::new(VecDeque::new())),
            executing_tasks: Arc::new(RwLock::new(HashMap::new())),
            completed_tasks: Arc::new(RwLock::new(HashMap::new())),
            failed_tasks: Arc::new(RwLock::new(HashMap::new())),
            dependency_graph: Arc::new(RwLock::new(HashMap::new())),
            variable_context: Arc::new(RwLock::new(VariableResolutionContext::new())),
            config,
            event_sender,
            event_receiver: Arc::new(RwLock::new(Some(event_receiver))),
            cancellation_token: CancellationToken::new(),
            ready_notify: Arc::new(Notify::new()),
            tool_cache: Arc::new(RwLock::new(tool_cache)),
        }
    }

    /// 初始化DAG计划
    pub async fn initialize_plan(&self, plan: &DagExecutionPlan) -> Result<()> {
        info!("初始化DAG计划: {} (版本: {})", plan.name, plan.version);

        // 清空所有队列
        self.clear_all_queues().await;

        // 设置依赖关系图
        {
            let mut deps = self.dependency_graph.write().await;
            *deps = plan.dependency_graph.clone();
        }

        // 设置变量映射
        {
            let mut context = self.variable_context.write().await;
            context.variable_mappings = plan.variable_mappings.clone();
            // 设置全局变量
            for (key, value) in &plan.global_config {
                context.global_variables.insert(key.clone(), value.clone());
            }
        }

        // 分析任务依赖并分配到相应队列
        let mut waiting_queue = self.waiting_queue.write().await;
        let mut ready_queue = self.ready_queue.write().await;

        for mut node in plan.nodes.clone() {
            if node.dependencies.is_empty() {
                // 无依赖任务直接进入就绪队列
                node.status = TaskStatus::Ready;
                ready_queue.push_back(node);
            } else {
                // 有依赖任务进入等待队列
                node.status = TaskStatus::Pending;
                waiting_queue.push_back(node);
            }
        }

        // 按优先级排序就绪队列
        let mut ready_tasks: Vec<_> = ready_queue.drain(..).collect();
        ready_tasks.sort_by_key(|task| task.priority);
        ready_queue.extend(ready_tasks);

        info!(
            "DAG初始化完成: {} 个就绪任务, {} 个等待任务",
            ready_queue.len(),
            waiting_queue.len()
        );

        // 通知就绪任务可用
        if !ready_queue.is_empty() {
            self.ready_notify.notify_waiters();
        }

        Ok(())
    }

    /// 启动事件驱动调度循环
    pub async fn start_event_driven_scheduling(&self) -> Result<()> {
        info!("Starting event-driven scheduling loop");
        
        // 取出事件接收器
        let mut receiver = {
            let mut receiver_guard = self.event_receiver.write().await;
            receiver_guard.take().ok_or_else(|| anyhow::anyhow!("Event receiver already taken"))?
        };

        loop {
            tokio::select! {
                // 处理调度事件
                event = receiver.recv() => {
                    match event {
                        Some(SchedulingEvent::TaskCompleted { task_id, result }) => {
                            self.handle_task_completion(task_id, result).await?;
                        }
                        Some(SchedulingEvent::TaskFailed { task_id, error, retry_count }) => {
                            self.handle_task_failure(task_id, error, retry_count).await?;
                        }
                        Some(SchedulingEvent::TaskAdded { task }) => {
                            self.handle_new_task(task).await?;
                        }
                        Some(SchedulingEvent::Shutdown) => {
                            info!("Received shutdown signal, stopping scheduling loop");
                            break;
                        }
                        None => {
                            warn!("Event channel closed, stopping scheduling loop");
                            break;
                        }
                    }
                }
                // 检查取消
                _ = self.cancellation_token.cancelled() => {
                    info!("Scheduling loop cancelled");
                    break;
                }
            }
        }

        Ok(())
    }

    /// 处理任务完成事件
    async fn handle_task_completion(&self, task_id: String, result: TaskExecutionResult) -> Result<()> {
        info!("Handling task completion: {}", task_id);
        
        // 从执行队列中移除
        {
            let mut executing = self.executing_tasks.write().await;
            executing.remove(&task_id);
        }

        // 保存成功结果
        {
            let mut completed = self.completed_tasks.write().await;
            completed.insert(task_id.clone(), result.clone());
        }
        
        // 更新变量解析上下文
        {
            let mut context = self.variable_context.write().await;
            context.add_task_result(result);
        }
        
        // 立即检查并移动新就绪的任务
        self.update_waiting_tasks().await?;
        
        // 通知有新的就绪任务
        self.ready_notify.notify_waiters();
        
        Ok(())
    }

    /// 处理任务失败事件
    async fn handle_task_failure(&self, task_id: String, error: String, retry_count: u32) -> Result<()> {
        warn!("Handling task failure: {} - {} (retry: {})", task_id, error, retry_count);
        
        // 从执行队列中移除
        {
            let mut executing = self.executing_tasks.write().await;
            executing.remove(&task_id);
        }

        // 检查是否应该重试
        if retry_count < self.config.max_task_retries {
            info!("Task {} will be retried (attempt {}/{})", task_id, retry_count + 1, self.config.max_task_retries);
            
            // 将任务重新加入等待队列进行重试
            if let Some(task) = self.prepare_task_for_retry(&task_id, retry_count + 1).await? {
                // 计算退避延迟
                let backoff_delay = self.calculate_backoff_delay(retry_count + 1);
                
                // 延迟后重新调度任务
                let event_sender = self.event_sender.clone();
                let task_for_retry = task.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(backoff_delay).await;
                    let _ = event_sender.send(SchedulingEvent::TaskAdded { task: task_for_retry });
                });
                
                info!("Task {} scheduled for retry after {:?} delay", task_id, backoff_delay);
            } else {
                warn!("Failed to prepare task {} for retry", task_id);
                self.mark_task_as_finally_failed(task_id, error, retry_count).await?;
            }
        } else {
            // 达到最大重试次数，最终失败
            info!("Task {} reached max retries, marking as finally failed", task_id);
            self.mark_task_as_finally_failed(task_id, error, retry_count).await?;
        }
        
        Ok(())
    }

    /// 标记任务为最终失败
    async fn mark_task_as_finally_failed(&self, task_id: String, error: String, retry_count: u32) -> Result<()> {
        let failure_result = TaskExecutionResult::failure(task_id.clone(), error, 0, retry_count);
        
        // 保存失败结果
        {
            let mut failed = self.failed_tasks.write().await;
            failed.insert(task_id.clone(), failure_result);
        }
        
        // 触发失败传播策略
        self.handle_failure_propagation(&task_id).await?;
        
        Ok(())
    }

    /// 准备任务重试
    async fn prepare_task_for_retry(&self, task_id: &str, new_retry_count: u32) -> Result<Option<DagTaskNode>> {
        // 尝试从各个队列中查找原始任务
        let mut original_task = None;
        
        // 检查等待队列
        {
            let waiting_queue = self.waiting_queue.read().await;
            if let Some(task) = waiting_queue.iter().find(|t| t.id == task_id) {
                original_task = Some(task.clone());
            }
        }
        
        // 检查就绪队列
        if original_task.is_none() {
            let ready_queue = self.ready_queue.read().await;
            if let Some(task) = ready_queue.iter().find(|t| t.id == task_id) {
                original_task = Some(task.clone());
            }
        }
        
        // 如果找不到原始任务，尝试从失败记录中重建
        if original_task.is_none() {
            if let Some(retry_task) = self.reconstruct_task_from_failure(task_id).await? {
                original_task = Some(retry_task);
            }
        }
        
        if let Some(mut task) = original_task {
            // 更新重试计数
            task.retry_count = new_retry_count;
            task.status = TaskStatus::Pending;
            
            // 重置任务到初始状态但保留重试信息
            task.created_at = chrono::Utc::now();
            
            // 添加重试元数据
            task.add_tag(format!("retry_{}", new_retry_count));
            
            Ok(Some(task))
        } else {
            warn!("Could not find or reconstruct task {} for retry", task_id);
            Ok(None)
        }
    }

    /// 从失败记录重建任务（用于重试）
    async fn reconstruct_task_from_failure(&self, task_id: &str) -> Result<Option<DagTaskNode>> {
        // 这是一个简化的实现，实际中可能需要存储更多的任务元数据
        warn!("Task reconstruction for retry not fully implemented for task {}", task_id);
        Ok(None)
    }

    /// 计算指数退避延迟
    fn calculate_backoff_delay(&self, retry_attempt: u32) -> std::time::Duration {
        // 指数退避：基础延迟 * 2^(retry_attempt - 1)
        // 加入抖动以避免雷群效应
        let base_delay_ms = 1000; // 1秒基础延迟
        let max_delay_ms = 60000; // 最大60秒
        
        let exponential_delay = base_delay_ms * (2_u64.pow(retry_attempt.saturating_sub(1)));
        let capped_delay = exponential_delay.min(max_delay_ms);
        
        // 添加简单的抖动（基于时间戳的简单随机化）
        let jitter_range = capped_delay / 4;
        let time_based_jitter = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default().as_nanos() % (jitter_range * 2) as u128) as u64;
        let jitter = time_based_jitter.saturating_sub(jitter_range);
        let final_delay = capped_delay.saturating_add(jitter);
        
        std::time::Duration::from_millis(final_delay)
    }

    /// 处理新任务添加事件
    async fn handle_new_task(&self, mut task: DagTaskNode) -> Result<()> {
        info!("Handling new task: {}", task.name);
        
        if task.dependencies.is_empty() {
            // 无依赖任务直接进入就绪队列
            task.status = TaskStatus::Ready;
            let mut ready_queue = self.ready_queue.write().await;
            ready_queue.push_back(task);
            self.ready_notify.notify_waiters();
        } else {
            // 有依赖任务进入等待队列
            task.status = TaskStatus::Pending;
            let mut waiting_queue = self.waiting_queue.write().await;
            waiting_queue.push_back(task);
        }
        
        Ok(())
    }

    /// 处理失败传播
    async fn handle_failure_propagation(&self, failed_task_id: &str) -> Result<()> {
        info!("Processing failure propagation for task: {} (strategy: {:?})", failed_task_id, self.config.failure_strategy);
        
        match self.config.failure_strategy {
            crate::engines::llm_compiler::types::FailurePropagationStrategy::FailFast => {
                self.handle_fail_fast_propagation(failed_task_id).await
            }
            crate::engines::llm_compiler::types::FailurePropagationStrategy::BestEffort => {
                self.handle_best_effort_propagation(failed_task_id).await
            }
            crate::engines::llm_compiler::types::FailurePropagationStrategy::Fallback => {
                self.handle_fallback_propagation(failed_task_id).await
            }
            crate::engines::llm_compiler::types::FailurePropagationStrategy::Continue => {
                self.handle_continue_propagation(failed_task_id).await
            }
        }
    }

    /// 快速失败策略：取消所有依赖失败任务的任务
    async fn handle_fail_fast_propagation(&self, failed_task_id: &str) -> Result<()> {
        let dependent_tasks = self.get_dependent_tasks(failed_task_id).await;
        
        for dependent_task_id in dependent_tasks {
            self.cancel_dependent_task(&dependent_task_id, failed_task_id, "FailFast strategy").await?;
        }
        
        Ok(())
    }

    /// 尽力而为策略：修改任务依赖，移除失败的依赖项
    async fn handle_best_effort_propagation(&self, failed_task_id: &str) -> Result<()> {
        let dependent_tasks = self.get_dependent_tasks(failed_task_id).await;
        
        for dependent_task_id in dependent_tasks {
            self.remove_dependency(&dependent_task_id, failed_task_id).await?;
            info!("Removed failed dependency {} from task {} (BestEffort strategy)", failed_task_id, dependent_task_id);
        }
        
        // 检查是否有任务因为依赖移除而变成就绪状态
        self.update_waiting_tasks().await?;
        
        Ok(())
    }

    /// 回退策略：尝试使用默认值或替代方案
    async fn handle_fallback_propagation(&self, failed_task_id: &str) -> Result<()> {
        let dependent_tasks = self.get_dependent_tasks(failed_task_id).await;
        
        for dependent_task_id in dependent_tasks {
            // 为依赖任务提供回退值
            if let Some(fallback_value) = self.generate_fallback_value(failed_task_id).await {
                self.inject_fallback_value(&dependent_task_id, failed_task_id, fallback_value).await?;
                info!("Injected fallback value for failed dependency {} in task {}", failed_task_id, dependent_task_id);
            } else {
                // 如果无法生成回退值，降级到尽力而为策略
                self.remove_dependency(&dependent_task_id, failed_task_id).await?;
                warn!("No fallback value available for {}, removing dependency", failed_task_id);
            }
        }
        
        self.update_waiting_tasks().await?;
        Ok(())
    }

    /// 继续执行策略：忽略失败，不影响其他任务
    async fn handle_continue_propagation(&self, _failed_task_id: &str) -> Result<()> {
        info!("Continue strategy: ignoring failure, no propagation");
        Ok(())
    }

    /// 获取依赖指定任务的所有任务ID
    async fn get_dependent_tasks(&self, failed_task_id: &str) -> Vec<String> {
        let dependency_graph = self.dependency_graph.read().await;
        dependency_graph
            .iter()
            .filter(|(_, deps)| deps.contains(&failed_task_id.to_string()))
            .map(|(task_id, _)| task_id.clone())
            .collect()
    }

    /// 取消依赖任务
    async fn cancel_dependent_task(&self, dependent_task_id: &str, failed_task_id: &str, reason: &str) -> Result<()> {
        warn!("Task {} depends on failed task {}, cancelling ({})", dependent_task_id, failed_task_id, reason);
        
        // 从等待队列中移除并标记为取消
        let mut waiting_queue = self.waiting_queue.write().await;
        if let Some(pos) = waiting_queue.iter().position(|task| task.id == dependent_task_id) {
            let mut task = waiting_queue.remove(pos).unwrap();
            task.status = TaskStatus::Cancelled;
            
            let cancelled_result = TaskExecutionResult {
                task_id: dependent_task_id.to_string(),
                status: TaskStatus::Cancelled,
                outputs: HashMap::new(),
                error: Some(format!("Cancelled due to dependency failure: {} ({})", failed_task_id, reason)),
                duration_ms: 0,
                started_at: chrono::Utc::now(),
                completed_at: Some(chrono::Utc::now()),
                retry_count: 0,
                metadata: HashMap::new(),
            };
            
            let mut failed = self.failed_tasks.write().await;
            failed.insert(dependent_task_id.to_string(), cancelled_result);
        }
        
        Ok(())
    }

    /// 移除任务的特定依赖项
    async fn remove_dependency(&self, task_id: &str, dependency_to_remove: &str) -> Result<()> {
        // 从依赖图中移除依赖
        let mut dependency_graph = self.dependency_graph.write().await;
        if let Some(deps) = dependency_graph.get_mut(task_id) {
            deps.retain(|dep| dep != dependency_to_remove);
        }
        
        // 从等待队列中的任务对象移除依赖
        let mut waiting_queue = self.waiting_queue.write().await;
        if let Some(task) = waiting_queue.iter_mut().find(|task| task.id == task_id) {
            task.dependencies.retain(|dep| dep != dependency_to_remove);
        }
        
        Ok(())
    }

    /// 生成回退值
    async fn generate_fallback_value(&self, _failed_task_id: &str) -> Option<Value> {
        // 这里可以实现更复杂的回退值生成逻辑
        // 例如：空结果、默认值、或基于任务类型的合理回退
        Some(Value::Object(serde_json::Map::new()))
    }

    /// 为任务注入回退值
    async fn inject_fallback_value(&self, task_id: &str, failed_dependency: &str, fallback_value: Value) -> Result<()> {
        // 将回退值添加到变量解析上下文中
        let mut context = self.variable_context.write().await;
        
        // 创建一个虚拟的成功结果
        let fallback_result = TaskExecutionResult {
            task_id: failed_dependency.to_string(),
            status: TaskStatus::Completed,
            outputs: if let Value::Object(obj) = fallback_value {
                obj.into_iter().collect()
            } else {
                HashMap::from([("fallback_value".to_string(), fallback_value)])
            },
            error: None,
            duration_ms: 0,
            started_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
            retry_count: 0,
            metadata: HashMap::from([("is_fallback".to_string(), Value::Bool(true))]),
        };
        
        context.add_task_result(fallback_result);
        info!("Injected fallback value for {} in task {}", failed_dependency, task_id);
        
        Ok(())
    }

    /// 发送事件到调度循环
    pub fn send_event(&self, event: SchedulingEvent) -> Result<()> {
        self.event_sender.send(event)
            .map_err(|e| anyhow::anyhow!("Failed to send scheduling event: {}", e))
    }

    /// 获取取消令牌
    pub fn get_cancellation_token(&self) -> CancellationToken {
        self.cancellation_token.clone()
    }

    /// 等待就绪任务通知
    pub async fn wait_for_ready_tasks(&self) {
        self.ready_notify.notified().await;
    }

    /// 清空所有队列
    async fn clear_all_queues(&self) {
        self.waiting_queue.write().await.clear();
        self.ready_queue.write().await.clear();
        self.executing_tasks.write().await.clear();
        self.completed_tasks.write().await.clear();
        self.failed_tasks.write().await.clear();
    }

    /// 获取下一个可执行的任务
    pub async fn fetch_next_task(&self) -> Option<DagTaskNode> {
        let mut ready_queue = self.ready_queue.write().await;
        
        // 优先获取高优先级任务
        if let Some(task) = ready_queue.pop_front() {
            debug!("获取就绪任务: {} (优先级: {})", task.name, task.priority);
            Some(task)
        } else {
            None
        }
    }

    /// 获取多个可执行任务（用于批量并行执行）
    pub async fn fetch_ready_tasks(&self, max_count: usize) -> Vec<DagTaskNode> {
        let mut ready_queue = self.ready_queue.write().await;
        let mut tasks = Vec::new();
        
        for _ in 0..max_count {
            if let Some(task) = ready_queue.pop_front() {
                tasks.push(task);
            } else {
                break;
            }
        }
        
        if !tasks.is_empty() {
            info!("批量获取 {} 个就绪任务", tasks.len());
        }
        
        tasks
    }

    /// 标记任务开始执行
    pub async fn mark_task_executing(&self, task_id: String, abort_handle: AbortHandle) {
        let mut executing = self.executing_tasks.write().await;
        executing.insert(task_id.clone(), abort_handle);
        info!("Task started executing: {}", task_id);
    }

    /// 标记任务完成并更新依赖
    pub async fn complete_task(&self, result: TaskExecutionResult) -> Result<()> {
        info!("Task completed: {} (status: {:?})", result.task_id, result.status);

        // 根据执行状态发送相应事件
        match result.status {
            TaskStatus::Completed => {
                self.send_event(SchedulingEvent::TaskCompleted {
                    task_id: result.task_id.clone(),
                    result,
                })?;
            }
            TaskStatus::Failed => {
                self.send_event(SchedulingEvent::TaskFailed {
                    task_id: result.task_id.clone(),
                    error: result.error.unwrap_or_else(|| "Unknown error".to_string()),
                    retry_count: result.retry_count,
                })?;
            }
            _ => {
                warn!("Received non-terminal task result: {} (status: {:?})", result.task_id, result.status);
            }
        }

        Ok(())
    }

    /// 更新等待队列，将满足依赖的任务移到就绪队列
    async fn update_waiting_tasks(&self) -> Result<()> {
        let mut waiting_queue = self.waiting_queue.write().await;
        let mut ready_queue = self.ready_queue.write().await;
        let completed = self.completed_tasks.read().await;

        let mut newly_ready = Vec::new();
        let mut still_waiting = VecDeque::new();

        while let Some(mut task) = waiting_queue.pop_front() {
            // 检查所有依赖是否已完成
            let all_deps_completed = task.dependencies.iter()
                .all(|dep_id| completed.contains_key(dep_id));

            if all_deps_completed {
                // 解析变量引用
                self.resolve_task_variables(&mut task).await?;
                task.status = TaskStatus::Ready;
                newly_ready.push(task);
            } else {
                still_waiting.push_back(task);
            }
        }

        // 更新队列
        *waiting_queue = still_waiting;
        
        // 按优先级排序新就绪的任务
        newly_ready.sort_by_key(|task| task.priority);
        
        for task in newly_ready {
            ready_queue.push_back(task);
        }

        if !ready_queue.is_empty() {
            debug!("更新后就绪队列有 {} 个任务", ready_queue.len());
        }

        Ok(())
    }

    /// 解析任务的变量引用并验证参数
    /// 支持多种变量引用格式：
    /// - $1, $2 等简单引用
    /// - ${task_id.outputs.field} 等表达式引用
    /// - 直接路径引用（task_id.outputs.field）
    async fn resolve_task_variables(&self, task: &mut DagTaskNode) -> Result<()> {
        let context = self.variable_context.read().await;
        
        // 解析输入参数中的变量引用
        for (key, value) in task.inputs.iter_mut() {
            match value {
                Value::String(s) => {
                    let s_clone = s.clone();
                    if let Some(resolved_value) = self.resolve_string_variable(&context, &s_clone) {
                        *value = resolved_value;
                        debug!("Resolved variable in {}: {} -> {:?}", key, s_clone, value);
                    }
                }
                Value::Array(arr) => {
                    // 递归解析数组中的变量
                    for item in arr.iter_mut() {
                        if let Value::String(s) = item {
                            let s_clone = s.clone();
                            if let Some(resolved_value) = self.resolve_string_variable(&context, &s_clone) {
                                *item = resolved_value;
                                debug!("Resolved array variable: {} -> {:?}", s_clone, item);
                            }
                        }
                    }
                }
                Value::Object(obj) => {
                    // 递归解析对象中的变量
                    for (_, obj_value) in obj.iter_mut() {
                        if let Value::String(s) = obj_value {
                            let s_clone = s.clone();
                            if let Some(resolved_value) = self.resolve_string_variable(&context, &s_clone) {
                                *obj_value = resolved_value;
                                debug!("Resolved object variable: {} -> {:?}", s_clone, obj_value);
                            }
                        }
                    }
                }
                _ => {} // 其他类型不处理
            }
        }
        
        // Schema 验证（如果有可用的工具 Schema）
        if let Some(tool_schema) = self.get_tool_schema(&task.tool_name).await {
            let validation_result = context.validate_tool_parameters(&tool_schema, &task.inputs);
            
            if !validation_result.is_valid {
                warn!("Parameter validation failed for tool {}: {:?}", task.tool_name, validation_result.errors);
                
                // 尝试使用修正后的参数
                if let Some(corrected_params) = validation_result.corrected_params {
                    info!("Using corrected parameters for tool {}", task.tool_name);
                    task.inputs = corrected_params;
                } else {
                    // 验证失败且无法修正，记录错误但继续执行（或者可以选择失败）
                    warn!("Cannot correct parameters for tool {}, proceeding with original parameters", task.tool_name);
                }
            } else {
                // 使用验证并可能修正后的参数
                if let Some(corrected_params) = validation_result.corrected_params {
                    task.inputs = corrected_params;
                }
                debug!("Parameter validation passed for tool {}", task.tool_name);
            }
        }
        
        // 清空变量引用列表（已解析）
        task.variable_refs.clear();
        
        Ok(())
    }

    /// 获取工具的 Schema 定义
    async fn get_tool_schema(&self, tool_name: &str) -> Option<crate::engines::llm_compiler::types::ToolSchema> {
        // 这里应该从工具注册中心或配置中获取 Schema
        // 为了演示，我创建一些常用工具的 Schema
        self.create_default_tool_schema(tool_name)
    }

    /// 创建默认工具 Schema（演示用）
    fn create_default_tool_schema(&self, tool_name: &str) -> Option<crate::engines::llm_compiler::types::ToolSchema> {
        use crate::engines::llm_compiler::types::*;
        
        match tool_name {
            "port_scanner" | "port_scan" => {
                Some(ToolSchema {
                    name: tool_name.to_string(),
                    description: "Scan network ports on target".to_string(),
                    parameters: vec![
                        ToolParameterSchema {
                            name: "target".to_string(),
                            param_type: ParameterType::String,
                            required: true,
                            description: Some("Target host or IP address".to_string()),
                            default_value: None,
                            constraints: Some(ParameterConstraints {
                                min_length: Some(1),
                                max_length: Some(255),
                                pattern: Some("*".to_string()), // 简化的域名/IP模式
                                ..Default::default()
                            }),
                        },
                        ToolParameterSchema {
                            name: "ports".to_string(),
                            param_type: ParameterType::Enum(vec!["common".to_string(), "all".to_string(), "top1000".to_string()]),
                            required: false,
                            description: Some("Port scan range".to_string()),
                            default_value: Some(Value::String("common".to_string())),
                            constraints: None,
                        },
                        ToolParameterSchema {
                            name: "threads".to_string(),
                            param_type: ParameterType::Integer,
                            required: false,
                            description: Some("Number of concurrent threads".to_string()),
                            default_value: Some(Value::Number(serde_json::Number::from(50))),
                            constraints: Some(ParameterConstraints {
                                min_value: Some(1.0),
                                max_value: Some(1000.0),
                                ..Default::default()
                            }),
                        },
                    ],
                    output_schema: None,
                })
            }
            "dns_scanner" => {
                Some(ToolSchema {
                    name: tool_name.to_string(),
                    description: "Perform DNS scanning and enumeration".to_string(),
                    parameters: vec![
                        ToolParameterSchema {
                            name: "target".to_string(),
                            param_type: ParameterType::String,
                            required: true,
                            description: Some("Target domain".to_string()),
                            default_value: None,
                            constraints: Some(ParameterConstraints {
                                min_length: Some(1),
                                max_length: Some(255),
                                pattern: Some("*.*".to_string()), // 简化的域名模式
                                ..Default::default()
                            }),
                        },
                        ToolParameterSchema {
                            name: "record_types".to_string(),
                            param_type: ParameterType::Array(Box::new(ParameterType::String)),
                            required: false,
                            description: Some("DNS record types to query".to_string()),
                            default_value: Some(Value::Array(vec![
                                Value::String("A".to_string()),
                                Value::String("AAAA".to_string()),
                                Value::String("MX".to_string()),
                                Value::String("NS".to_string()),
                            ])),
                            constraints: None,
                        },
                    ],
                    output_schema: None,
                })
            }
            "rsubdomain" => {
                Some(ToolSchema {
                    name: tool_name.to_string(),
                    description: "Subdomain enumeration tool".to_string(),
                    parameters: vec![
                        ToolParameterSchema {
                            name: "target".to_string(),
                            param_type: ParameterType::String,
                            required: true,
                            description: Some("Target domain".to_string()),
                            default_value: None,
                            constraints: Some(ParameterConstraints {
                                min_length: Some(1),
                                max_length: Some(255),
                                ..Default::default()
                            }),
                        },
                        ToolParameterSchema {
                            name: "wordlist".to_string(),
                            param_type: ParameterType::Enum(vec!["common".to_string(), "large".to_string(), "custom".to_string()]),
                            required: false,
                            description: Some("Wordlist to use for enumeration".to_string()),
                            default_value: Some(Value::String("common".to_string())),
                            constraints: None,
                        },
                        ToolParameterSchema {
                            name: "timeout".to_string(),
                            param_type: ParameterType::Integer,
                            required: false,
                            description: Some("Timeout in seconds".to_string()),
                            default_value: Some(Value::Number(serde_json::Number::from(30))),
                            constraints: Some(ParameterConstraints {
                                min_value: Some(1.0),
                                max_value: Some(300.0),
                                ..Default::default()
                            }),
                        },
                    ],
                    output_schema: None,
                })
            }
            _ => None, // 未知工具不提供 Schema
        }
    }

    /// 解析字符串中的变量引用
    fn resolve_string_variable(&self, context: &crate::engines::llm_compiler::types::VariableResolutionContext, s: &str) -> Option<Value> {
        // 处理 ${expression} 格式
        if s.starts_with("${") && s.ends_with('}') {
            let expression = &s[2..s.len()-1];
            return context.resolve_variable(expression);
        }
        
        // 处理 $variable 格式
        if s.starts_with('$') {
            return context.resolve_variable(s);
        }
        
        // 处理模板字符串（包含多个变量）
        if s.contains('$') {
            return Some(Value::String(self.resolve_template_string(context, s)));
        }
        
        // 检查是否为直接路径引用（不以$开头但符合路径格式）
        if context.validate_variable_path(s) && context.can_resolve_path(s) {
            return context.resolve_variable(s);
        }
        
        None
    }

    /// 解析模板字符串中的多个变量
    fn resolve_template_string(&self, context: &crate::engines::llm_compiler::types::VariableResolutionContext, template: &str) -> String {
        let mut result = template.to_string();
        
        // 查找所有 ${...} 模式
        let mut start = 0;
        while let Some(begin) = result[start..].find("${") {
            let begin_pos = start + begin;
            if let Some(end) = result[begin_pos..].find('}') {
                let end_pos = begin_pos + end;
                let var_expr = &result[begin_pos + 2..end_pos];
                
                if let Some(resolved_value) = context.resolve_variable(var_expr) {
                    let replacement = match resolved_value {
                        Value::String(s) => s,
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        _ => serde_json::to_string(&resolved_value).unwrap_or_else(|_| "null".to_string()),
                    };
                    
                    result.replace_range(begin_pos..=end_pos, &replacement);
                    start = begin_pos + replacement.len();
                } else {
                    start = end_pos + 1;
                }
            } else {
                break;
            }
        }
        
        // 查找所有简单的 $variable 模式（简化版，不使用 regex）
        result = self.replace_simple_variables(context, &result);
        
        result
    }

    /// 替换简单的 $variable 模式（不使用正则表达式）
    fn replace_simple_variables(&self, context: &crate::engines::llm_compiler::types::VariableResolutionContext, text: &str) -> String {
        let mut result = text.to_string();
        let mut i = 0;
        
        while i < result.len() {
            if result.chars().nth(i) == Some('$') {
                // 找到变量开始位置
                let start = i;
                i += 1;
                
                // 查找变量名结束位置
                let mut end = i;
                while end < result.len() {
                    let ch = result.chars().nth(end).unwrap();
                    if ch.is_alphanumeric() || ch == '_' {
                        end += 1;
                    } else {
                        break;
                    }
                }
                
                if end > i {
                    // 提取变量名
                    let var_ref = &result[start..end];
                    if let Some(resolved_value) = context.resolve_variable(var_ref) {
                        let replacement = match resolved_value {
                            Value::String(s) => s,
                            Value::Number(n) => n.to_string(),
                            Value::Bool(b) => b.to_string(),
                            _ => serde_json::to_string(&resolved_value).unwrap_or_else(|_| "null".to_string()),
                        };
                        
                        result.replace_range(start..end, &replacement);
                        i = start + replacement.len();
                    } else {
                        i = end;
                    }
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }
        
        result
    }

    /// 获取执行统计
    pub async fn get_execution_stats(&self) -> ExecutionStats {
        let waiting_count = self.waiting_queue.read().await.len();
        let ready_count = self.ready_queue.read().await.len();
        let executing_count = self.executing_tasks.read().await.len();
        let completed_count = self.completed_tasks.read().await.len();
        let failed_count = self.failed_tasks.read().await.len();
        let total_count = waiting_count + ready_count + executing_count + completed_count + failed_count;

        ExecutionStats {
            waiting_tasks: waiting_count,
            ready_tasks: ready_count,
            executing_tasks: executing_count,
            completed_tasks: completed_count,
            failed_tasks: failed_count,
            total_tasks: total_count,
        }
    }

    /// 获取所有已完成的任务结果
    pub async fn get_completed_results(&self) -> Vec<TaskExecutionResult> {
        self.completed_tasks.read().await.values().cloned().collect()
    }

    /// 获取所有失败的任务结果
    pub async fn get_failed_results(&self) -> Vec<TaskExecutionResult> {
        self.failed_tasks.read().await.values().cloned().collect()
    }

    /// 检查是否还有任务需要执行
    pub async fn has_pending_tasks(&self) -> bool {
        let stats = self.get_execution_stats().await;
        stats.waiting_tasks > 0 || stats.ready_tasks > 0 || stats.executing_tasks > 0
    }

    /// 取消所有等待和就绪的任务
    pub async fn cancel_pending_tasks(&self) -> Result<()> {
        info!("Cancelling all pending tasks");
        
        // 触发取消令牌
        self.cancellation_token.cancel();
        
        // 清空等待队列和就绪队列
        self.waiting_queue.write().await.clear();
        self.ready_queue.write().await.clear();
        
        // 取消正在执行的任务
        let executing_tasks = self.executing_tasks.write().await;
        for (task_id, abort_handle) in executing_tasks.iter() {
            warn!("Cancelling executing task: {}", task_id);
            abort_handle.abort();
        }
        
        // 发送停机事件到调度循环
        let _ = self.send_event(SchedulingEvent::Shutdown);
        
        Ok(())
    }

    /// 获取任务依赖关系图的拓扑排序
    pub async fn get_topological_order(&self) -> Result<Vec<String>> {
        let dependency_graph = self.dependency_graph.read().await;
        self.topological_sort(&dependency_graph)
    }

    /// 拓扑排序算法
    fn topological_sort(&self, graph: &HashMap<String, Vec<String>>) -> Result<Vec<String>> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut adj_list: HashMap<String, Vec<String>> = HashMap::new();
        
        // 构建邻接表和入度表
        for (node, deps) in graph {
            in_degree.entry(node.clone()).or_insert(0);
            for dep in deps {
                adj_list.entry(dep.clone()).or_insert_with(Vec::new).push(node.clone());
                *in_degree.entry(node.clone()).or_insert(0) += 1;
            }
        }
        
        // Kahn算法
        let mut queue: VecDeque<String> = VecDeque::new();
        let mut result: Vec<String> = Vec::new();
        
        // 找到所有入度为0的节点
        for (node, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(node.clone());
            }
        }
        
        while let Some(node) = queue.pop_front() {
            result.push(node.clone());
            
            // 减少相邻节点的入度
            if let Some(neighbors) = adj_list.get(&node) {
                for neighbor in neighbors {
                    if let Some(degree) = in_degree.get_mut(neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(neighbor.clone());
                        }
                    }
                }
            }
        }
        
        // 检查是否有环
        if result.len() != in_degree.len() {
            return Err(anyhow::anyhow!("检测到循环依赖"));
        }
        
        Ok(result)
    }

    /// 获取可以并行执行的任务组
    pub async fn get_parallel_task_groups(&self) -> Result<Vec<Vec<String>>> {
        let dependency_graph = self.dependency_graph.read().await;
        let topo_order = self.topological_sort(&dependency_graph)?;
        
        let mut groups: Vec<Vec<String>> = Vec::new();
        let mut processed: HashSet<String> = HashSet::new();
        
        for task_id in topo_order.clone() {
            if processed.contains(&task_id) {
                continue;
            }
            
            // 找到所有可以与当前任务并行执行的任务
            let mut parallel_group = vec![task_id.clone()];
            processed.insert(task_id.clone());
            
            // 检查其他未处理的任务是否可以并行
            for other_task in &topo_order {
                if processed.contains(other_task) {
                    continue;
                }
                
                // 检查是否有依赖关系
                if !self.has_dependency_path(&dependency_graph, &task_id, other_task) &&
                   !self.has_dependency_path(&dependency_graph, other_task, &task_id) {
                    parallel_group.push(other_task.clone());
                    processed.insert(other_task.clone());
                }
            }
            
            groups.push(parallel_group);
        }
        
        Ok(groups)
    }

    /// 检查两个任务之间是否存在依赖路径
    fn has_dependency_path(
        &self,
        graph: &HashMap<String, Vec<String>>,
        from: &str,
        to: &str,
    ) -> bool {
        let mut visited: HashSet<String> = HashSet::new();
        let mut stack: Vec<String> = vec![from.to_string()];
        
        while let Some(current) = stack.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());
            
            if current == to {
                return true;
            }
            
            if let Some(deps) = graph.get(&current) {
                for dep in deps {
                    if !visited.contains(dep) {
                        stack.push(dep.clone());
                    }
                }
            }
        }
        
        false
    }

    /// 合并依赖图
    pub async fn merge_dependency_graph(&self, new_dependencies: &HashMap<String, Vec<String>>) -> Result<()> {
        let mut deps = self.dependency_graph.write().await;
        
        for (task_id, new_deps) in new_dependencies {
            deps.insert(task_id.clone(), new_deps.clone());
        }
        
        info!("Merged {} new dependency relationships", new_dependencies.len());
        Ok(())
    }

    /// 合并变量映射
    pub async fn merge_variable_mappings(&self, new_mappings: &HashMap<String, String>) -> Result<()> {
        let mut context = self.variable_context.write().await;
        
        for (var_ref, mapping) in new_mappings {
            context.variable_mappings.insert(var_ref.clone(), mapping.clone());
        }
        
        info!("Merged {} new variable mappings", new_mappings.len());
        Ok(())
    }

    /// 检查工具调用缓存
    pub async fn check_tool_cache(&self, task: &DagTaskNode) -> Option<TaskExecutionResult> {
        let mut cache = self.tool_cache.write().await;
        
        // 检查是否应该使用缓存
        if !cache.should_cache(&task.tool_name, &task.inputs) {
            return None;
        }
        
        let cache_key = cache.generate_cache_key(&task.tool_name, &task.inputs);
        
        if let Some(cached_result) = cache.get(&cache_key) {
            info!("Cache hit for tool {} with task {}", task.tool_name, task.id);
            
            // 创建基于缓存的新结果，但使用当前任务ID
            let mut result = cached_result;
            result.task_id = task.id.clone();
            result.started_at = chrono::Utc::now();
            result.completed_at = Some(chrono::Utc::now());
            result.duration_ms = 1; // 缓存命中，几乎没有延迟
            
            // 添加缓存标记
            result.metadata.insert("from_cache".to_string(), Value::Bool(true));
            result.metadata.insert("cache_key".to_string(), Value::String(serde_json::to_string(&cache_key).unwrap_or_default()));
            
            Some(result)
        } else {
            debug!("Cache miss for tool {} with task {}", task.tool_name, task.id);
            None
        }
    }

    /// 将结果添加到缓存
    pub async fn cache_tool_result(&self, task: &DagTaskNode, result: &TaskExecutionResult) {
        let mut cache = self.tool_cache.write().await;
        
        // 只缓存成功的结果
        if result.status != TaskStatus::Completed {
            return;
        }
        
        // 检查是否应该缓存
        if !cache.should_cache(&task.tool_name, &task.inputs) {
            return;
        }
        
        let cache_key = cache.generate_cache_key(&task.tool_name, &task.inputs);
        let ttl = cache.get_tool_ttl(&task.tool_name);
        
        // 创建一个不包含任务特定信息的通用结果
        let mut generic_result = result.clone();
        generic_result.task_id = "cached".to_string(); // 使用通用ID
        generic_result.metadata.remove("task_specific_data"); // 移除任务特定元数据
        
        cache.put(cache_key, generic_result, Some(ttl));
        info!("Cached result for tool {} (TTL: {}s)", task.tool_name, ttl);
    }

    /// 获取缓存统计
    pub async fn get_cache_stats(&self) -> crate::engines::llm_compiler::types::CacheStats {
        let cache = self.tool_cache.read().await;
        cache.get_stats().clone()
    }

    /// 清理过期缓存
    pub async fn cleanup_cache(&self) {
        let mut cache = self.tool_cache.write().await;
        let old_size = cache.size();
        cache.cleanup_expired();
        let new_size = cache.size();
        
        if old_size != new_size {
            info!("Cache cleanup: removed {} expired entries ({} -> {})", old_size - new_size, old_size, new_size);
        }
    }

    /// 获取缓存命中率
    pub async fn get_cache_hit_rate(&self) -> f64 {
        let cache = self.tool_cache.read().await;
        cache.hit_rate()
    }

    /// 获取任务的详细状态信息
    pub async fn get_task_status(&self, task_id: &str) -> Option<TaskStatus> {
        // 检查各个队列中的任务状态
        
        // 检查等待队列
        let waiting_queue = self.waiting_queue.read().await;
        if waiting_queue.iter().any(|task| task.id == task_id) {
            return Some(TaskStatus::Pending);
        }
        
        // 检查就绪队列
        let ready_queue = self.ready_queue.read().await;
        if ready_queue.iter().any(|task| task.id == task_id) {
            return Some(TaskStatus::Ready);
        }
        
        // 检查执行中任务
        let executing_tasks = self.executing_tasks.read().await;
        if executing_tasks.contains_key(task_id) {
            return Some(TaskStatus::Running);
        }
        
        // 检查已完成任务
        let completed_tasks = self.completed_tasks.read().await;
        if completed_tasks.contains_key(task_id) {
            return Some(TaskStatus::Completed);
        }
        
        // 检查失败任务
        let failed_tasks = self.failed_tasks.read().await;
        if failed_tasks.contains_key(task_id) {
            return Some(TaskStatus::Failed);
        }
        
        None
    }
}