//! 并行执行器 - 支持 DAG 任务并行执行
//!
//! 借鉴 LLMCompiler 的并行执行能力

use super::dag_planner::DagPlanner;
use super::types::*;
use crate::tools::{FrameworkToolAdapter, UnifiedToolCall};
use crate::utils::ordered_message::{emit_message_chunk_arc, ArchitectureType, ChunkType};
use anyhow::{anyhow, Result};
use futures::future::join_all;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::{Mutex, RwLock, Semaphore};
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

/// 并行执行器
pub struct ParallelExecutor {
    /// 配置
    config: ParallelExecutionConfig,
    /// 工具适配器
    tool_adapter: Option<Arc<dyn FrameworkToolAdapter>>,
    /// 并发控制信号量
    semaphore: Arc<Semaphore>,
    /// 任务结果存储
    task_results: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    /// 取消令牌
    cancellation_token: Option<CancellationToken>,
    /// 消息发送相关
    app_handle: Option<Arc<tauri::AppHandle>>,
    execution_id: Option<String>,
    message_id: Option<String>,
    conversation_id: Option<String>,
}

impl ParallelExecutor {
    pub fn new(config: ParallelExecutionConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_concurrency));
        Self {
            config,
            tool_adapter: None,
            semaphore,
            task_results: Arc::new(RwLock::new(HashMap::new())),
            cancellation_token: None,
            app_handle: None,
            execution_id: None,
            message_id: None,
            conversation_id: None,
        }
    }

    pub fn with_tool_adapter(mut self, adapter: Arc<dyn FrameworkToolAdapter>) -> Self {
        self.tool_adapter = Some(adapter);
        self
    }

    pub fn with_cancellation_token(mut self, token: CancellationToken) -> Self {
        self.cancellation_token = Some(token);
        self
    }

    pub fn with_message_context(
        mut self,
        app_handle: Arc<tauri::AppHandle>,
        execution_id: String,
        message_id: String,
        conversation_id: Option<String>,
    ) -> Self {
        self.app_handle = Some(app_handle);
        self.execution_id = Some(execution_id);
        self.message_id = Some(message_id);
        self.conversation_id = conversation_id;
        self
    }

    /// 发送消息到前端
    fn emit_message(&self, chunk_type: ChunkType, content: &str, structured_data: Option<serde_json::Value>) {
        if let (Some(app_handle), Some(execution_id), Some(message_id)) =
            (&self.app_handle, &self.execution_id, &self.message_id)
        {
            emit_message_chunk_arc(
                app_handle,
                execution_id,
                message_id,
                self.conversation_id.as_deref(),
                chunk_type,
                content,
                false,
                Some("DagExecutor"),
                None,
                Some(ArchitectureType::Travel),
                structured_data,
            );
        }
    }

    /// 执行 DAG 计划
    pub async fn execute_dag(&self, plan: &mut DagPlan) -> Result<DagExecutionResult> {
        let start_time = Instant::now();
        let mut metrics = DagExecutionMetrics::default();
        metrics.total_tasks = plan.tasks.len() as u32;

        self.emit_message(
            ChunkType::Thinking,
            &format!("[START] Starting DAG execution with {} tasks", plan.tasks.len()),
            Some(serde_json::json!({
                "total_tasks": plan.tasks.len(),
                "max_concurrency": self.config.max_concurrency
            })),
        );

        // 清空之前的结果
        {
            let mut results = self.task_results.write().await;
            results.clear();
        }

        // 按层执行 (拓扑排序)
        let mut completed: Vec<String> = Vec::new();
        let mut failed: Vec<String> = Vec::new();
        let mut current_parallel = 0u32;

        loop {
            // 检查取消
            if let Some(token) = &self.cancellation_token {
                if token.is_cancelled() {
                    log::info!("ParallelExecutor: Execution cancelled");
                    self.emit_message(ChunkType::Error, "[CANCELLED] Execution cancelled", None);
                    break;
                }
            }

            // 获取可执行的任务
            let ready_tasks = self.get_ready_tasks(plan, &completed, &failed);

            if ready_tasks.is_empty() {
                // 检查是否所有任务都已处理
                let total_processed = completed.len() + failed.len();
                if total_processed >= plan.tasks.len() {
                    break;
                }
                // 可能有循环依赖或所有剩余任务都依赖失败的任务
                log::warn!("ParallelExecutor: No ready tasks but {} tasks remaining", 
                    plan.tasks.len() - total_processed);
                break;
            }

            // 更新最大并行数
            current_parallel = ready_tasks.len() as u32;
            if current_parallel > metrics.max_parallel {
                metrics.max_parallel = current_parallel;
            }

            self.emit_message(
                ChunkType::Content,
                &format!("⚡ Executing {} tasks in parallel", ready_tasks.len()),
                Some(serde_json::json!({
                    "parallel_count": ready_tasks.len(),
                    "completed": completed.len(),
                    "failed": failed.len()
                })),
            );

            // 提取任务数据用于并行执行
            let task_data: Vec<_> = ready_tasks
                .iter()
                .filter_map(|task_id| {
                    plan.tasks.iter().find(|t| t.id == *task_id).map(|t| {
                        (t.id.clone(), t.tool_name.clone(), t.arguments.clone())
                    })
                })
                .collect();

            // 标记任务为运行中
            for task_id in &ready_tasks {
                if let Some(task) = plan.tasks.iter_mut().find(|t| t.id == *task_id) {
                    task.status = DagTaskStatus::Running;
                    task.started_at = Some(SystemTime::now());
                }
            }

            // 并行执行任务
            let task_futures: Vec<_> = task_data
                .into_iter()
                .map(|(task_id, tool_name, arguments)| {
                    self.execute_task_by_data(task_id, tool_name, arguments)
                })
                .collect();

            let results = join_all(task_futures).await;

            // 处理结果
            for (task_id, result) in results {
                match result {
                    Ok(output) => {
                        completed.push(task_id.clone());
                        metrics.completed_tasks += 1;

                        // 存储结果供后续任务引用
                        {
                            let mut stored = self.task_results.write().await;
                            stored.insert(task_id.clone(), output.clone());
                        }

                        // 更新任务状态
                        if let Some(task) = plan.tasks.iter_mut().find(|t| t.id == task_id) {
                            task.status = DagTaskStatus::Completed;
                            task.result = Some(output);
                            task.completed_at = Some(SystemTime::now());
                        }
                    }
                    Err(e) => {
                        failed.push(task_id.clone());
                        metrics.failed_tasks += 1;

                        self.emit_message(
                            ChunkType::Error,
                            &format!("[FAILED] Task {} failed: {}", task_id, e),
                            None,
                        );

                        // 更新任务状态
                        if let Some(task) = plan.tasks.iter_mut().find(|t| t.id == task_id) {
                            task.status = DagTaskStatus::Failed;
                            task.error = Some(e.to_string());
                            task.completed_at = Some(SystemTime::now());
                        }
                    }
                }
            }
        }

        // 标记因依赖失败而跳过的任务
        for task in plan.tasks.iter_mut() {
            if task.status == DagTaskStatus::Pending || task.status == DagTaskStatus::Ready {
                task.status = DagTaskStatus::Skipped;
                metrics.skipped_tasks += 1;
            }
        }

        metrics.total_duration_ms = start_time.elapsed().as_millis() as u64;

        // 收集所有任务结果
        let task_results = self.task_results.read().await.clone();

        // 计算节省的 Token (估算: 每省略一次 LLM 调用约节省 500 tokens)
        // 传统 ReAct: 每个任务需要 Thought+Action+Observation 三次交互
        // DAG 模式: 只需要一次规划
        metrics.tokens_saved = (metrics.total_tasks.saturating_sub(1)) * 500;
        metrics.llm_calls = 1; // DAG 模式只需要一次 LLM 调用规划

        let success = failed.is_empty();

        self.emit_message(
            ChunkType::Content,
            &format!(
                "📊 DAG execution completed: {} succeeded, {} failed, {} skipped",
                metrics.completed_tasks, metrics.failed_tasks, metrics.skipped_tasks
            ),
            Some(serde_json::json!({
                "success": success,
                "metrics": metrics
            })),
        );

        Ok(DagExecutionResult {
            plan_id: plan.id.clone(),
            success,
            task_results,
            failed_tasks: failed,
            metrics,
            final_output: self.build_final_output(plan).await,
            // v3.0 增强字段
            needs_replanning: false,
            replan_reason: None,
            execution_snapshot: None,
        })
    }

    /// 获取可执行的任务 (依赖已满足)
    fn get_ready_tasks(&self, plan: &DagPlan, completed: &[String], failed: &[String]) -> Vec<String> {
        plan.tasks
            .iter()
            .filter(|task| {
                // 必须是 Pending 状态
                if task.status != DagTaskStatus::Pending {
                    return false;
                }
                // 依赖必须全部完成且没有失败
                task.depends_on.iter().all(|dep| {
                    completed.contains(dep) && !failed.contains(dep)
                })
            })
            .map(|t| t.id.clone())
            .collect()
    }

    /// 执行单个任务 (通过数据)
    async fn execute_task_by_data(
        &self,
        task_id: String,
        tool_name: String,
        arguments: HashMap<String, serde_json::Value>,
    ) -> (String, Result<serde_json::Value>) {
        log::info!("ParallelExecutor: Executing task {} - {}", task_id, tool_name);

        self.emit_message(
            ChunkType::Content,
            &format!("[TOOL] Executing: {}({})", tool_name, 
                arguments.keys().cloned().collect::<Vec<_>>().join(", ")),
            Some(serde_json::json!({
                "task_id": task_id,
                "tool_name": tool_name,
                "arguments": arguments
            })),
        );

        // 获取信号量许可
        let _permit = match self.semaphore.acquire().await {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to acquire semaphore: {}", e);
                return (task_id, Err(anyhow!("Semaphore error: {}", e)));
            }
        };

        // 解析变量引用
        let mut resolved_args = arguments.clone();
        {
            let results = self.task_results.read().await;
            DagPlanner::resolve_variable_references(&mut resolved_args, &results);
        }

        // 执行工具
        let result = self.execute_tool(&tool_name, resolved_args).await;

        match &result {
            Ok(_) => {
                self.emit_message(
                    ChunkType::ToolResult,
                    &format!("[SUCCESS] Task {} completed", task_id),
                    Some(serde_json::json!({
                        "task_id": task_id,
                        "tool_name": tool_name,
                        "success": true
                    })),
                );
            }
            Err(e) => {
                log::error!("Task {} failed: {}", task_id, e);
            }
        }

        (task_id, result)
    }

    /// 执行工具调用
    async fn execute_tool(
        &self,
        tool_name: &str,
        arguments: HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let unified_call = UnifiedToolCall {
            id: uuid::Uuid::new_v4().to_string(),
            tool_name: tool_name.to_string(),
            parameters: arguments,
            timeout: Some(Duration::from_secs(self.config.task_timeout)),
            context: HashMap::new(),
            retry_count: 0,
        };

        // 优先使用设置的 tool_adapter
        if let Some(adapter) = &self.tool_adapter {
            let result = timeout(
                Duration::from_secs(self.config.task_timeout),
                adapter.execute_tool(unified_call),
            )
            .await
            .map_err(|_| anyhow!("Tool execution timeout"))??;

            return Ok(result.output);
        }

        // 降级使用全局 adapter
        let engine_adapter = crate::tools::get_global_engine_adapter()
            .map_err(|e| anyhow!("No tool adapter available: {}", e))?;

        let result = timeout(
            Duration::from_secs(self.config.task_timeout),
            engine_adapter.execute_tool(unified_call),
        )
        .await
        .map_err(|_| anyhow!("Tool execution timeout"))??;

        Ok(result.output)
    }

    /// 构建最终输出
    async fn build_final_output(&self, plan: &DagPlan) -> Option<serde_json::Value> {
        let results = self.task_results.read().await;

        // 如果只有一个任务，直接返回其结果
        if plan.tasks.len() == 1 {
            return results.values().next().cloned();
        }

        // 合并所有任务结果
        let mut combined = serde_json::Map::new();
        for task in &plan.tasks {
            if let Some(result) = results.get(&task.id) {
                combined.insert(format!("task_{}", task.id), result.clone());
            }
        }

        Some(serde_json::Value::Object(combined))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_ready_tasks() {
        let config = ParallelExecutionConfig::default();
        let executor = ParallelExecutor::new(config);

        let mut plan = DagPlan::new("test".to_string());
        plan.add_task(DagTask::new("1".to_string(), "tool1".to_string(), HashMap::new()));
        plan.add_task(
            DagTask::new("2".to_string(), "tool2".to_string(), HashMap::new())
                .with_depends(vec!["1".to_string()]),
        );

        // 初始状态: 只有任务1可执行
        let ready = executor.get_ready_tasks(&plan, &[], &[]);
        assert_eq!(ready, vec!["1".to_string()]);

        // 任务1完成后: 任务2可执行
        let ready = executor.get_ready_tasks(&plan, &["1".to_string()], &[]);
        assert_eq!(ready, vec!["2".to_string()]);
    }
}

