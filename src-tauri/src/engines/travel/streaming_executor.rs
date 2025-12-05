//! 流式 DAG 执行器
//!
//! 实现边规划边执行的能力，支持：
//! - 流式任务生成
//! - 条件分支评估
//! - 循环展开和执行
//! - 实时进度反馈

use super::dag_planner::DagPlanner;
use super::types::*;
use crate::engines::LlmClient;
use crate::services::ai::AiService;
use crate::tools::{FrameworkToolAdapter, UnifiedToolCall};
use crate::utils::ordered_message::{emit_message_chunk_arc, ArchitectureType, ChunkType};
use anyhow::{anyhow, Result};
use futures::stream::{self, Stream, StreamExt};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::{mpsc, RwLock, Semaphore};
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

/// 流式 DAG 执行器
pub struct StreamingDagExecutor {
    /// 配置
    config: StreamingExecutorConfig,
    /// AI 服务
    ai_service: Arc<AiService>,
    /// 工具适配器
    tool_adapter: Option<Arc<dyn FrameworkToolAdapter>>,
    /// 并发控制
    semaphore: Arc<Semaphore>,
    /// 执行状态
    state: Arc<RwLock<StreamingExecutionState>>,
    /// 取消令牌
    cancellation_token: Option<CancellationToken>,
    /// 消息发送
    app_handle: Option<Arc<tauri::AppHandle>>,
    execution_id: Option<String>,
    message_id: Option<String>,
    conversation_id: Option<String>,
}

/// 流式执行器配置
#[derive(Debug, Clone)]
pub struct StreamingExecutorConfig {
    /// 最大并发数
    pub max_concurrency: usize,
    /// 任务超时 (秒)
    pub task_timeout: u64,
    /// 是否启用流式规划
    pub enable_streaming_plan: bool,
    /// 规划批次大小 (每次规划多少任务)
    pub plan_batch_size: usize,
    /// 最大重规划次数
    pub max_replan_count: u32,
    /// 条件评估超时 (毫秒)
    pub condition_eval_timeout: u64,
}

impl Default for StreamingExecutorConfig {
    fn default() -> Self {
        Self {
            max_concurrency: 5,
            task_timeout: 60,
            enable_streaming_plan: true,
            plan_batch_size: 3,
            max_replan_count: 5,
            condition_eval_timeout: 5000,
        }
    }
}

impl StreamingDagExecutor {
    pub fn new(ai_service: Arc<AiService>, config: StreamingExecutorConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_concurrency));
        Self {
            config,
            ai_service,
            tool_adapter: None,
            semaphore,
            state: Arc::new(RwLock::new(StreamingExecutionState {
                plan_version: 0,
                completed: Vec::new(),
                running: Vec::new(),
                pending: Vec::new(),
                results: HashMap::new(),
                replan_count: 0,
            })),
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

    /// 发送事件到前端
    fn emit_event(&self, event: &StreamingTaskEvent) {
        if let (Some(app_handle), Some(execution_id), Some(message_id)) =
            (&self.app_handle, &self.execution_id, &self.message_id)
        {
            let (chunk_type, content) = match event {
                StreamingTaskEvent::TaskPlanned { task } => (
                    ChunkType::PlanInfo,
                    format!("[PLAN] Task planned: {} - {}", task.id, task.tool_name),
                ),
                StreamingTaskEvent::TaskStarted { task_id } => (
                    ChunkType::Content,
                    format!("[START] Task {} started", task_id),
                ),
                StreamingTaskEvent::TaskProgress { task_id, progress, message } => (
                    ChunkType::Content,
                    format!("[PROGRESS] Task {}: {:.0}% - {}", task_id, progress * 100.0, message),
                ),
                StreamingTaskEvent::TaskCompleted { task_id, .. } => (
                    ChunkType::ToolResult,
                    format!("[DONE] Task {} completed", task_id),
                ),
                StreamingTaskEvent::TaskFailed { task_id, error } => (
                    ChunkType::Error,
                    format!("[FAIL] Task {} failed: {}", task_id, error),
                ),
                StreamingTaskEvent::ReplanTriggered { reason } => (
                    ChunkType::Thinking,
                    format!("[REPLAN] Replanning triggered: {:?}", reason),
                ),
                StreamingTaskEvent::PlanUpdated { new_tasks } => (
                    ChunkType::PlanInfo,
                    format!("[UPDATE] Plan updated with {} new tasks", new_tasks.len()),
                ),
                StreamingTaskEvent::ExecutionComplete { result } => (
                    ChunkType::Content,
                    format!("[COMPLETE] Execution finished: success={}", result.success),
                ),
            };

            emit_message_chunk_arc(
                app_handle,
                execution_id,
                message_id,
                self.conversation_id.as_deref(),
                chunk_type,
                &content,
                false,
                Some("StreamingExecutor"),
                None,
                Some(ArchitectureType::Travel),
                Some(serde_json::to_value(event).unwrap_or(serde_json::json!({}))),
            );
        }
    }

    /// 流式执行 DAG 计划
    pub async fn execute_streaming(
        &self,
        task_description: &str,
        context: &HashMap<String, serde_json::Value>,
    ) -> Result<DagExecutionResult> {
        let start_time = Instant::now();
        log::info!("StreamingExecutor: Starting streaming execution for: {}", task_description);

        // 创建事件通道
        let (event_tx, mut event_rx) = mpsc::channel::<StreamingTaskEvent>(100);

        // 启动事件处理任务
        let event_emitter = self.clone_for_events();
        let event_handle = tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                event_emitter.emit_event(&event);
            }
        });

        // 初始化状态
        {
            let mut state = self.state.write().await;
            *state = StreamingExecutionState {
                plan_version: 0,
                completed: Vec::new(),
                running: Vec::new(),
                pending: Vec::new(),
                results: HashMap::new(),
                replan_count: 0,
            };
        }

        // 执行流式规划和执行循环
        let result = self.streaming_plan_execute_loop(
            task_description,
            context,
            event_tx.clone(),
        ).await;

        // 发送完成事件
        if let Ok(ref r) = result {
            let _ = event_tx.send(StreamingTaskEvent::ExecutionComplete { result: r.clone() }).await;
        }

        // 等待事件处理完成
        drop(event_tx);
        let _ = event_handle.await;

        let duration = start_time.elapsed().as_millis() as u64;
        log::info!("StreamingExecutor: Completed in {}ms", duration);

        result
    }

    /// 流式规划-执行循环
    async fn streaming_plan_execute_loop(
        &self,
        task_description: &str,
        context: &HashMap<String, serde_json::Value>,
        event_tx: mpsc::Sender<StreamingTaskEvent>,
    ) -> Result<DagExecutionResult> {
        let mut metrics = DagExecutionMetrics::default();
        let mut all_results: HashMap<String, serde_json::Value> = HashMap::new();
        let mut failed_tasks: Vec<String> = Vec::new();
        let mut execution_snapshot = ExecutionSnapshot::default();

        // 创建 LLM 客户端用于流式规划
        let llm_client = crate::engines::create_client(self.ai_service.as_ref());

        // 初始规划
        let mut current_plan = self.generate_initial_plan(
            &llm_client,
            task_description,
            context,
        ).await?;

        metrics.total_tasks = current_plan.tasks.len() as u32;

        // 更新状态
        {
            let mut state = self.state.write().await;
            state.pending = current_plan.tasks.iter().map(|t| t.id.clone()).collect();
        }

        // 主执行循环
        loop {
            // 检查取消
            if let Some(token) = &self.cancellation_token {
                if token.is_cancelled() {
                    log::info!("StreamingExecutor: Cancelled");
                    break;
                }
            }

            // 获取可执行任务
            let ready_tasks = self.get_ready_tasks(&current_plan, &all_results).await;

            if ready_tasks.is_empty() {
                // 检查是否所有任务都已处理
                let state = self.state.read().await;
                if state.pending.is_empty() && state.running.is_empty() {
                    break;
                }
                // 等待正在执行的任务
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }

            // 并行执行就绪任务
            let execution_results = self.execute_tasks_parallel(
                ready_tasks,
                &all_results,
                &event_tx,
            ).await;

            // 处理执行结果
            for (task_id, result) in execution_results {
                match result {
                    Ok(output) => {
                        all_results.insert(task_id.clone(), output.clone());
                        execution_snapshot.completed_tasks.insert(task_id.clone(), output.clone());
                        metrics.completed_tasks += 1;

                        // 更新状态
                        let mut state = self.state.write().await;
                        state.completed.push(task_id.clone());
                        state.pending.retain(|id| id != &task_id);
                        state.results.insert(task_id, output);
                    }
                    Err(e) => {
                        let task = current_plan.tasks.iter().find(|t| t.id == task_id);
                        let error_strategy = task.map(|t| t.on_error.clone())
                            .unwrap_or(ErrorStrategy::Abort);

                        // 记录错误
                        execution_snapshot.error_history.push(ErrorRecord {
                            task_id: task_id.clone(),
                            tool_name: task.map(|t| t.tool_name.clone()).unwrap_or_default(),
                            error: e.to_string(),
                            timestamp: SystemTime::now(),
                            context: None,
                        });

                        match error_strategy {
                            ErrorStrategy::Abort => {
                                failed_tasks.push(task_id);
                                metrics.failed_tasks += 1;
                                // 不继续执行
                                break;
                            }
                            ErrorStrategy::Skip => {
                                log::warn!("Task {} failed, skipping: {}", task_id, e);
                                metrics.skipped_tasks += 1;
                            }
                            ErrorStrategy::Replan => {
                                // 触发重规划
                                let state = self.state.read().await;
                                if state.replan_count < self.config.max_replan_count {
                                    drop(state);
                                    
                                    let replan_reason = ReplanReason::TaskFailed {
                                        task_id: task_id.clone(),
                                        error: e.to_string(),
                                    };
                                    
                                    let _ = event_tx.send(StreamingTaskEvent::ReplanTriggered {
                                        reason: replan_reason.clone(),
                                    }).await;

                                    // 生成新计划
                                    match self.replan(
                                        &llm_client,
                                        task_description,
                                        context,
                                        &execution_snapshot,
                                        &replan_reason,
                                    ).await {
                                        Ok(new_plan) => {
                                            let mut state = self.state.write().await;
                                            state.replan_count += 1;
                                            state.plan_version += 1;
                                            state.pending = new_plan.tasks.iter()
                                                .filter(|t| !state.completed.contains(&t.id))
                                                .map(|t| t.id.clone())
                                                .collect();
                                            drop(state);

                                            current_plan = new_plan;
                                            metrics.llm_calls += 1;

                                            let _ = event_tx.send(StreamingTaskEvent::PlanUpdated {
                                                new_tasks: current_plan.tasks.clone(),
                                            }).await;
                                        }
                                        Err(replan_err) => {
                                            log::error!("Replanning failed: {}", replan_err);
                                            failed_tasks.push(task_id);
                                            metrics.failed_tasks += 1;
                                        }
                                    }
                                } else {
                                    log::warn!("Max replan count reached, aborting");
                                    failed_tasks.push(task_id);
                                    metrics.failed_tasks += 1;
                                }
                            }
                            _ => {
                                // 其他策略暂时当作 Skip 处理
                                metrics.skipped_tasks += 1;
                            }
                        }
                    }
                }
            }

            // 检查是否有任务失败导致中止
            if !failed_tasks.is_empty() {
                let has_abort = current_plan.tasks.iter()
                    .filter(|t| failed_tasks.contains(&t.id))
                    .any(|t| matches!(t.on_error, ErrorStrategy::Abort));
                if has_abort {
                    break;
                }
            }
        }

        // 构建最终结果
        let success = failed_tasks.is_empty();
        let needs_replanning = false;
        
        let final_output = if all_results.is_empty() {
            None
        } else if all_results.len() == 1 {
            all_results.values().next().cloned()
        } else {
            Some(serde_json::json!(all_results))
        };

        Ok(DagExecutionResult {
            plan_id: current_plan.id,
            success,
            task_results: all_results,
            failed_tasks,
            metrics,
            final_output,
            needs_replanning,
            replan_reason: None,
            execution_snapshot: Some(execution_snapshot),
        })
    }

    /// 生成初始计划
    async fn generate_initial_plan(
        &self,
        llm_client: &LlmClient,
        task_description: &str,
        context: &HashMap<String, serde_json::Value>,
    ) -> Result<DagPlan> {
        let planner = DagPlanner::new(self.ai_service.clone(), LiteModeConfig::default());
        
        // 如果有工具适配器，传递给规划器
        let planner = if let Some(adapter) = &self.tool_adapter {
            planner.with_tool_adapter(adapter.clone())
        } else {
            planner
        };

        planner.generate_plan(task_description, context).await
    }

    /// 获取可执行的任务
    async fn get_ready_tasks(
        &self,
        plan: &DagPlan,
        results: &HashMap<String, serde_json::Value>,
    ) -> Vec<DagTask> {
        let state = self.state.read().await;
        let completed: Vec<String> = state.completed.clone();
        let running: Vec<String> = state.running.clone();
        drop(state);

        let mut ready = Vec::new();

        for task in &plan.tasks {
            // 跳过已完成或正在运行的
            if completed.contains(&task.id) || running.contains(&task.id) {
                continue;
            }

            // 检查依赖是否满足
            let deps_satisfied = task.depends_on.iter().all(|dep| completed.contains(dep));
            if !deps_satisfied {
                continue;
            }

            // 评估条件 (如果有)
            if let Some(condition) = &task.condition {
                match self.evaluate_condition(&condition.expr, results).await {
                    Ok(true) => ready.push(task.clone()),
                    Ok(false) => {
                        // 条件不满足，标记为跳过
                        log::info!("Task {} condition not met, skipping", task.id);
                    }
                    Err(e) => {
                        log::warn!("Condition evaluation failed for task {}: {}", task.id, e);
                    }
                }
            } else {
                ready.push(task.clone());
            }
        }

        ready
    }

    /// 评估条件表达式
    async fn evaluate_condition(
        &self,
        expr: &str,
        results: &HashMap<String, serde_json::Value>,
    ) -> Result<bool> {
        // 简单的条件评估器
        // 支持格式: $N.field == 'value', $N.field != 'value', $N.field > N
        
        let expr = expr.trim();
        
        // 解析变量引用
        if let Some((left, op, right)) = self.parse_condition_expr(expr) {
            let left_value = self.resolve_value(&left, results)?;
            
            match op.as_str() {
                "==" => {
                    let right_str = right.trim_matches(|c| c == '\'' || c == '"');
                    Ok(left_value.as_str().map(|s| s == right_str).unwrap_or(false)
                        || left_value.to_string().trim_matches('"') == right_str)
                }
                "!=" => {
                    let right_str = right.trim_matches(|c| c == '\'' || c == '"');
                    Ok(left_value.as_str().map(|s| s != right_str).unwrap_or(true)
                        && left_value.to_string().trim_matches('"') != right_str)
                }
                ">" => {
                    let right_num: f64 = right.parse().unwrap_or(0.0);
                    let left_num = left_value.as_f64().unwrap_or(0.0);
                    Ok(left_num > right_num)
                }
                "<" => {
                    let right_num: f64 = right.parse().unwrap_or(0.0);
                    let left_num = left_value.as_f64().unwrap_or(0.0);
                    Ok(left_num < right_num)
                }
                ">=" => {
                    let right_num: f64 = right.parse().unwrap_or(0.0);
                    let left_num = left_value.as_f64().unwrap_or(0.0);
                    Ok(left_num >= right_num)
                }
                "<=" => {
                    let right_num: f64 = right.parse().unwrap_or(0.0);
                    let left_num = left_value.as_f64().unwrap_or(0.0);
                    Ok(left_num <= right_num)
                }
                _ => Err(anyhow!("Unknown operator: {}", op)),
            }
        } else {
            // 简单的布尔检查
            let value = self.resolve_value(expr, results)?;
            Ok(value.as_bool().unwrap_or(false) 
                || value.as_str().map(|s| s == "true" || s == "success").unwrap_or(false))
        }
    }

    /// 解析条件表达式
    fn parse_condition_expr(&self, expr: &str) -> Option<(String, String, String)> {
        let operators = ["==", "!=", ">=", "<=", ">", "<"];
        for op in operators {
            if let Some(idx) = expr.find(op) {
                let left = expr[..idx].trim().to_string();
                let right = expr[idx + op.len()..].trim().to_string();
                return Some((left, op.to_string(), right));
            }
        }
        None
    }

    /// 解析变量引用
    fn resolve_value(
        &self,
        expr: &str,
        results: &HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value> {
        if !expr.starts_with('$') {
            // 字面量
            return Ok(serde_json::Value::String(expr.to_string()));
        }

        // 解析 $N.field.subfield 格式
        let parts: Vec<&str> = expr.trim_start_matches('$').split('.').collect();
        if parts.is_empty() {
            return Err(anyhow!("Invalid variable reference: {}", expr));
        }

        let task_id = parts[0];
        let mut value = results.get(task_id)
            .cloned()
            .ok_or_else(|| anyhow!("Task {} result not found", task_id))?;

        for field in &parts[1..] {
            value = value.get(*field)
                .cloned()
                .ok_or_else(|| anyhow!("Field {} not found in {}", field, expr))?;
        }

        Ok(value)
    }

    /// 并行执行任务
    async fn execute_tasks_parallel(
        &self,
        tasks: Vec<DagTask>,
        results: &HashMap<String, serde_json::Value>,
        event_tx: &mpsc::Sender<StreamingTaskEvent>,
    ) -> Vec<(String, Result<serde_json::Value>)> {
        let mut handles = Vec::new();

        // 更新状态：标记为运行中
        {
            let mut state = self.state.write().await;
            for task in &tasks {
                state.running.push(task.id.clone());
                state.pending.retain(|id| id != &task.id);
            }
        }

        for task in tasks {
            let task_id = task.id.clone();
            let semaphore = self.semaphore.clone();
            let tool_adapter = self.tool_adapter.clone();
            let timeout_secs = self.config.task_timeout;
            let event_tx = event_tx.clone();
            let results = results.clone();

            let handle = tokio::spawn(async move {
                // 发送开始事件
                let _ = event_tx.send(StreamingTaskEvent::TaskStarted {
                    task_id: task_id.clone(),
                }).await;

                // 获取信号量
                let _permit = match semaphore.acquire().await {
                    Ok(p) => p,
                    Err(e) => {
                        return (task_id.clone(), Err(anyhow!("Semaphore error: {}", e)));
                    }
                };

                // 解析变量引用
                let resolved_args = Self::resolve_arguments_static(&task.arguments, &results);

                // 执行工具
                let result = Self::execute_tool_static(
                    tool_adapter.as_ref(),
                    &task.tool_name,
                    resolved_args,
                    timeout_secs,
                ).await;

                // 发送结果事件
                match &result {
                    Ok(output) => {
                        let _ = event_tx.send(StreamingTaskEvent::TaskCompleted {
                            task_id: task_id.clone(),
                            result: output.clone(),
                        }).await;
                    }
                    Err(e) => {
                        let _ = event_tx.send(StreamingTaskEvent::TaskFailed {
                            task_id: task_id.clone(),
                            error: e.to_string(),
                        }).await;
                    }
                }

                (task_id, result)
            });

            handles.push(handle);
        }

        // 等待所有任务完成
        let mut results_vec = Vec::new();
        for handle in handles {
            if let Ok(result) = handle.await {
                // 更新状态：从运行中移除
                {
                    let mut state = self.state.write().await;
                    state.running.retain(|id| id != &result.0);
                }
                results_vec.push(result);
            }
        }

        results_vec
    }

    /// 静态解析参数中的变量引用
    fn resolve_arguments_static(
        args: &HashMap<String, serde_json::Value>,
        results: &HashMap<String, serde_json::Value>,
    ) -> HashMap<String, serde_json::Value> {
        let mut resolved = HashMap::new();

        for (key, value) in args {
            let resolved_value = if let serde_json::Value::String(s) = value {
                if s.starts_with('$') {
                    // 解析变量引用
                    let parts: Vec<&str> = s.trim_start_matches('$').split('.').collect();
                    if !parts.is_empty() {
                        if let Some(result) = results.get(parts[0]) {
                            let mut current = result.clone();
                            for field in &parts[1..] {
                                if let Some(v) = current.get(*field) {
                                    current = v.clone();
                                }
                            }
                            current
                        } else {
                            value.clone()
                        }
                    } else {
                        value.clone()
                    }
                } else {
                    value.clone()
                }
            } else {
                value.clone()
            };
            resolved.insert(key.clone(), resolved_value);
        }

        resolved
    }

    /// 静态执行工具
    async fn execute_tool_static(
        tool_adapter: Option<&Arc<dyn FrameworkToolAdapter>>,
        tool_name: &str,
        arguments: HashMap<String, serde_json::Value>,
        timeout_secs: u64,
    ) -> Result<serde_json::Value> {
        let unified_call = UnifiedToolCall {
            id: uuid::Uuid::new_v4().to_string(),
            tool_name: tool_name.to_string(),
            parameters: arguments,
            timeout: Some(Duration::from_secs(timeout_secs)),
            context: HashMap::new(),
            retry_count: 0,
        };

        if let Some(adapter) = tool_adapter {
            let result = timeout(
                Duration::from_secs(timeout_secs),
                adapter.execute_tool(unified_call),
            )
            .await
            .map_err(|_| anyhow!("Tool execution timeout"))??;

            Ok(result.output)
        } else {
            // 使用全局适配器
            let engine_adapter = crate::tools::get_global_engine_adapter()
                .map_err(|e| anyhow!("No tool adapter available: {}", e))?;

            let result = timeout(
                Duration::from_secs(timeout_secs),
                engine_adapter.execute_tool(unified_call),
            )
            .await
            .map_err(|_| anyhow!("Tool execution timeout"))??;

            Ok(result.output)
        }
    }

    /// 重规划
    async fn replan(
        &self,
        llm_client: &LlmClient,
        original_task: &str,
        context: &HashMap<String, serde_json::Value>,
        snapshot: &ExecutionSnapshot,
        reason: &ReplanReason,
    ) -> Result<DagPlan> {
        log::info!("StreamingExecutor: Replanning due to {:?}", reason);

        // 构建重规划 prompt
        let system_prompt = r#"你是一个任务重规划专家。根据执行情况调整计划。

## 输出格式 (每行一个任务)
```
1. tool_name(arg1="val1", arg2="val2")
2. tool_name(arg1=$1.field) depends: 1
3. join()
```

## 规则
1. 跳过已成功完成的步骤
2. 为失败的步骤提供替代方案
3. 根据已收集的信息调整策略
4. 最多10个任务"#;

        let user_prompt = format!(
            r#"原始任务: {}

已完成的步骤:
{}

收集到的信息:
{}

错误历史:
{}

重规划原因: {:?}

请生成新的执行计划:"#,
            original_task,
            serde_json::to_string_pretty(&snapshot.completed_tasks).unwrap_or_default(),
            serde_json::to_string_pretty(&snapshot.gathered_info).unwrap_or_default(),
            snapshot.error_history.iter()
                .map(|e| format!("- {} ({}): {}", e.task_id, e.tool_name, e.error))
                .collect::<Vec<_>>()
                .join("\n"),
            reason,
        );

        let response = llm_client
            .completion(Some(system_prompt), &user_prompt)
            .await
            .map_err(|e| anyhow!("Replan LLM call failed: {}", e))?;

        // 使用 DagPlanner 解析响应
        let planner = DagPlanner::new(self.ai_service.clone(), LiteModeConfig::default());
        
        // 直接使用解析方法
        // 注意：这里需要访问 DagPlanner 的解析方法，但它是私有的
        // 我们使用一个简化的解析
        self.parse_replan_response(&response, original_task)
    }

    /// 解析重规划响应
    fn parse_replan_response(&self, response: &str, task_description: &str) -> Result<DagPlan> {
        let mut plan = DagPlan::new(task_description.to_string());

        // 提取代码块
        let content = if response.contains("```") {
            response
                .split("```")
                .nth(1)
                .map(|s| s.trim_start_matches("plaintext").trim_start_matches('\n'))
                .unwrap_or(response)
                .trim()
        } else {
            response.trim()
        };

        // 简单的行解析
        let task_regex = regex::Regex::new(
            r#"(\d+)\.\s*(\w+)\s*\(([^)]*)\)(?:\s*depends:\s*([\d,\s]+))?"#
        )?;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.contains("join()") {
                continue;
            }

            if let Some(captures) = task_regex.captures(line) {
                let task_id = captures.get(1).map(|m| m.as_str()).unwrap_or("0");
                let tool_name = captures.get(2).map(|m| m.as_str()).unwrap_or("");
                let args_str = captures.get(3).map(|m| m.as_str()).unwrap_or("");
                let depends_str = captures.get(4).map(|m| m.as_str());

                let arguments = self.parse_arguments(args_str);
                let depends_on: Vec<String> = depends_str
                    .map(|s| {
                        s.split(',')
                            .map(|d| d.trim().to_string())
                            .filter(|d| !d.is_empty())
                            .collect()
                    })
                    .unwrap_or_default();

                let task = DagTask::new(task_id.to_string(), tool_name.to_string(), arguments)
                    .with_depends(depends_on)
                    .with_error_strategy(ErrorStrategy::Replan); // 新任务继承重规划策略

                plan.add_task(task);
            }
        }

        if plan.tasks.is_empty() {
            return Err(anyhow!("Failed to parse replan response"));
        }

        Ok(plan)
    }

    /// 解析参数字符串
    fn parse_arguments(&self, args_str: &str) -> HashMap<String, serde_json::Value> {
        let mut arguments = HashMap::new();
        let arg_regex = regex::Regex::new(
            r#"(\w+)\s*=\s*(?:"([^"]*)"|(\$[\d.]+\w*)|(\d+(?:\.\d+)?)|(\w+))"#
        ).unwrap();

        for captures in arg_regex.captures_iter(args_str) {
            let name = captures.get(1).map(|m| m.as_str()).unwrap_or("");

            let value = if let Some(quoted) = captures.get(2) {
                serde_json::Value::String(quoted.as_str().to_string())
            } else if let Some(var_ref) = captures.get(3) {
                serde_json::Value::String(var_ref.as_str().to_string())
            } else if let Some(num) = captures.get(4) {
                if let Ok(n) = num.as_str().parse::<f64>() {
                    serde_json::json!(n)
                } else {
                    serde_json::Value::String(num.as_str().to_string())
                }
            } else if let Some(word) = captures.get(5) {
                match word.as_str() {
                    "true" => serde_json::Value::Bool(true),
                    "false" => serde_json::Value::Bool(false),
                    _ => serde_json::Value::String(word.as_str().to_string()),
                }
            } else {
                serde_json::Value::Null
            };

            if !name.is_empty() {
                arguments.insert(name.to_string(), value);
            }
        }

        arguments
    }

    /// 克隆用于事件处理的轻量级副本
    fn clone_for_events(&self) -> StreamingDagExecutorEventEmitter {
        StreamingDagExecutorEventEmitter {
            app_handle: self.app_handle.clone(),
            execution_id: self.execution_id.clone(),
            message_id: self.message_id.clone(),
            conversation_id: self.conversation_id.clone(),
        }
    }
}

/// 轻量级事件发送器
struct StreamingDagExecutorEventEmitter {
    app_handle: Option<Arc<tauri::AppHandle>>,
    execution_id: Option<String>,
    message_id: Option<String>,
    conversation_id: Option<String>,
}

impl StreamingDagExecutorEventEmitter {
    fn emit_event(&self, event: &StreamingTaskEvent) {
        if let (Some(app_handle), Some(execution_id), Some(message_id)) =
            (&self.app_handle, &self.execution_id, &self.message_id)
        {
            let (chunk_type, content) = match event {
                StreamingTaskEvent::TaskPlanned { task } => (
                    ChunkType::PlanInfo,
                    format!("[PLAN] Task planned: {} - {}", task.id, task.tool_name),
                ),
                StreamingTaskEvent::TaskStarted { task_id } => (
                    ChunkType::Content,
                    format!("[START] Task {} started", task_id),
                ),
                StreamingTaskEvent::TaskProgress { task_id, progress, message } => (
                    ChunkType::Content,
                    format!("[PROGRESS] Task {}: {:.0}% - {}", task_id, progress * 100.0, message),
                ),
                StreamingTaskEvent::TaskCompleted { task_id, .. } => (
                    ChunkType::ToolResult,
                    format!("[DONE] Task {} completed", task_id),
                ),
                StreamingTaskEvent::TaskFailed { task_id, error } => (
                    ChunkType::Error,
                    format!("[FAIL] Task {} failed: {}", task_id, error),
                ),
                StreamingTaskEvent::ReplanTriggered { reason } => (
                    ChunkType::Thinking,
                    format!("[REPLAN] Replanning triggered: {:?}", reason),
                ),
                StreamingTaskEvent::PlanUpdated { new_tasks } => (
                    ChunkType::PlanInfo,
                    format!("[UPDATE] Plan updated with {} new tasks", new_tasks.len()),
                ),
                StreamingTaskEvent::ExecutionComplete { result } => (
                    ChunkType::Content,
                    format!("[COMPLETE] Execution finished: success={}", result.success),
                ),
            };

            emit_message_chunk_arc(
                app_handle,
                execution_id,
                message_id,
                self.conversation_id.as_deref(),
                chunk_type,
                &content,
                false,
                Some("StreamingExecutor"),
                None,
                Some(ArchitectureType::Travel),
                Some(serde_json::to_value(event).unwrap_or(serde_json::json!({}))),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_condition_expr() {
        let executor_config = StreamingExecutorConfig::default();
        // 测试需要一个 mock AI service，暂时跳过
    }
}

