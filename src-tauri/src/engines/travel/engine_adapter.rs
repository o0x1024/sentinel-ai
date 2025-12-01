//! Travel引擎适配器
//!
//! 实现BaseExecutionEngine trait,对接AI服务、工具调用等
//! 支持双模式执行: 精简DAG模式(Token优化) / 完整OODA模式

use super::types::*;
use super::complexity_analyzer::ComplexityAnalyzer;
use super::ooda_executor::OodaExecutor;
use super::engine_dispatcher::EngineDispatcher;
use super::dag_planner::DagPlanner;
use super::parallel_executor::ParallelExecutor;
use super::context_manager::ContextManager;
use super::resource_integration::ResourceTracker;
use crate::agents::traits::{
    AgentExecutionResult, AgentSession, AgentTask, PerformanceCharacteristics,
};
use crate::engines::traits::BaseExecutionEngine;
use crate::services::ai::AiService;
use crate::utils::ordered_message::{emit_message_chunk_arc, ArchitectureType, ChunkType};
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

/// Travel引擎
pub struct TravelEngine {
    config: TravelConfig,
    complexity_analyzer: ComplexityAnalyzer,
    ooda_executor: OodaExecutor,
    ai_service: Option<Arc<AiService>>,
    prompt_repo: Option<Arc<crate::services::prompt_db::PromptRepository>>,
    framework_adapter: Option<Arc<dyn crate::tools::FrameworkToolAdapter>>,
    app_handle: Option<tauri::AppHandle>,
    /// 上下文管理器 (Token优化)
    context_manager: ContextManager,
    /// 资源追踪器
    resource_tracker: ResourceTracker,
}

impl TravelEngine {
    /// 创建新的Travel引擎
    pub fn new(config: TravelConfig) -> Self {
        let complexity_analyzer = ComplexityAnalyzer::new(config.complexity_config.clone());
        let ooda_executor = OodaExecutor::new(config.clone());
        let context_manager = ContextManager::new(config.context_config.clone());
        let resource_tracker = ResourceTracker::new()
            .with_auto_cleanup(config.parallel_config.enable_resource_tracking);

        Self {
            config,
            complexity_analyzer,
            ooda_executor,
            ai_service: None,
            prompt_repo: None,
            framework_adapter: None,
            app_handle: None,
            context_manager,
            resource_tracker,
        }
    }

    /// 使用默认配置创建
    pub fn with_defaults() -> Self {
        Self::new(TravelConfig::default())
    }
    
    /// 判断是否应使用精简DAG模式
    fn should_use_lite_mode(&self, complexity: &TaskComplexity) -> bool {
        if !self.config.lite_mode.enabled {
            return false;
        }
        self.config.lite_mode.applicable_complexity.contains(complexity)
    }

    /// 发送消息到前端
    fn emit_message(
        &self,
        execution_id: &str,
        message_id: &str,
        conversation_id: Option<&str>,
        chunk_type: ChunkType,
        content: &str,
        structured_data: Option<serde_json::Value>,
    ) {
        if let Some(app_handle) = &self.app_handle {
            emit_message_chunk_arc(
                &Arc::new(app_handle.clone()),
                execution_id,
                message_id,
                conversation_id,
                chunk_type,
                content,
                false,
                Some("TravelEngine"),
                None,
                Some(ArchitectureType::Travel),
                structured_data,
            );
        }
    }

    /// 设置AI服务
    pub fn with_ai_service(mut self, ai_service: Arc<AiService>) -> Self {
        self.complexity_analyzer = self.complexity_analyzer.with_ai_service(ai_service.clone());
        self.ai_service = Some(ai_service);
        self.update_engine_dispatcher();
        self
    }
    
    /// 设置 PromptRepository
    pub fn with_prompt_repo(mut self, repo: Arc<crate::services::prompt_db::PromptRepository>) -> Self {
        log::info!("TravelEngine: Setting prompt repository");
        self.prompt_repo = Some(repo);
        self.update_engine_dispatcher();
        self
    }
    
    /// 设置 FrameworkToolAdapter
    pub fn with_framework_adapter(mut self, adapter: Arc<dyn crate::tools::FrameworkToolAdapter>) -> Self {
        self.framework_adapter = Some(adapter);
        self.update_engine_dispatcher();
        self
    }
    
    /// 设置 AppHandle
    pub fn with_app_handle(mut self, app: tauri::AppHandle) -> Self {
        self.app_handle = Some(app);
        self.update_engine_dispatcher();
        self
    }
    
    /// 更新 engine_dispatcher 的依赖
    fn update_engine_dispatcher(&mut self) {
        let mut dispatcher = EngineDispatcher::new();
        
        if let Some(ai_service) = &self.ai_service {
            dispatcher = dispatcher.with_ai_service(ai_service.clone());
        }
        
        if let Some(repo) = &self.prompt_repo {
            log::info!("TravelEngine: Passing prompt_repo to engine_dispatcher");
            dispatcher = dispatcher.with_prompt_repo(repo.clone());
        } else {
            log::warn!("TravelEngine: No prompt_repo available to pass to engine_dispatcher");
        }
        
        if let Some(adapter) = &self.framework_adapter {
            dispatcher = dispatcher.with_framework_adapter(adapter.clone());
        }
        
        if let Some(app) = &self.app_handle {
            dispatcher = dispatcher.with_app_handle(app.clone());
        }
        
        // 使用 std::mem::replace 来避免移动问题
        let old_executor = std::mem::replace(&mut self.ooda_executor, OodaExecutor::new(self.config.clone()));
        self.ooda_executor = old_executor.with_engine_dispatcher(dispatcher);
    }

    /// 执行Travel流程 (支持双模式)
    pub async fn execute(
        &self,
        task: &AgentTask,
        _session: &mut dyn AgentSession,
    ) -> Result<AgentExecutionResult> {
        log::info!("Travel engine executing task: {}", task.description);
        let start_time = Instant::now();

        // 1. 分析任务复杂度
        let task_complexity = self
            .complexity_analyzer
            .analyze_task_complexity(&task.description, Some(&task.parameters))
            .await?;

        log::info!("Task complexity determined: {:?}", task_complexity);

        // 2. 准备执行上下文
        let mut context = self.prepare_context(task)?;

        // 3. 提取消息相关的ID
        let execution_id = task.parameters.get("execution_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        
        let message_id = task.parameters.get("message_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        
        let conversation_id = task.parameters.get("conversation_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // 4. 清理之前的资源追踪
        self.resource_tracker.clear_all().await;

        // 5. 根据复杂度选择执行模式
        let result = if self.should_use_lite_mode(&task_complexity) {
            log::info!("Travel: Using LITE DAG mode for task (Token optimized)");
            self.emit_message(
                &execution_id,
                &message_id,
                conversation_id.as_deref(),
                ChunkType::Thinking,
                "[MODE] Using optimized DAG execution mode",
                Some(serde_json::json!({
                    "mode": "lite_dag",
                    "complexity": format!("{:?}", task_complexity)
                })),
            );
            
            self.execute_lite_mode(task, &mut context, &execution_id, &message_id, conversation_id.as_deref()).await
        } else {
            log::info!("Travel: Using FULL OODA mode for complex task");
            self.emit_message(
                &execution_id,
                &message_id,
                conversation_id.as_deref(),
                ChunkType::Thinking,
                "[MODE] Using full OODA execution mode",
                Some(serde_json::json!({
                    "mode": "full_ooda",
                    "complexity": format!("{:?}", task_complexity)
                })),
            );
            
            self.execute_full_ooda_mode(task, task_complexity, &mut context, &execution_id, &message_id, conversation_id.clone()).await
        };

        // 6. 清理资源
        if self.resource_tracker.has_resource_leak().await {
            log::warn!("Travel: Detected resource leaks, attempting cleanup");
            if let Some(adapter) = &self.framework_adapter {
                match self.resource_tracker.execute_cleanup(adapter).await {
                    Ok(report) => {
                        if report.has_leaks {
                            log::warn!("Travel: Some resources could not be cleaned: {:?}", report.leaked_resources);
                        } else {
                            log::info!("Travel: All resources cleaned successfully");
                        }
                    }
                    Err(e) => {
                        log::error!("Travel: Resource cleanup failed: {}", e);
                    }
                }
            }
        }

        let duration = start_time.elapsed().as_millis() as u64;
        log::info!("Travel: Task completed in {}ms", duration);

        result
    }

    /// 精简DAG模式执行 (Token优化)
    async fn execute_lite_mode(
        &self,
        task: &AgentTask,
        context: &mut HashMap<String, serde_json::Value>,
        execution_id: &str,
        message_id: &str,
        conversation_id: Option<&str>,
    ) -> Result<AgentExecutionResult> {
        let start_time = Instant::now();
        
        // 检查缓存
        let task_hash = ContextManager::generate_task_hash(&task.description, context);
        if let Some(cached_plan) = self.context_manager.get_cached_plan(&task_hash).await {
            log::info!("Travel Lite: Using cached plan");
            self.emit_message(
                execution_id,
                message_id,
                conversation_id,
                ChunkType::Content,
                "📦 Using cached execution plan",
                None,
            );
            
            return self.execute_dag_plan(cached_plan, context, execution_id, message_id, conversation_id).await;
        }

        // 需要 AI 服务来生成 DAG 计划
        let ai_service = self.ai_service.as_ref()
            .ok_or_else(|| anyhow::anyhow!("AI service required for DAG planning"))?;

        // 创建 DAG 规划器
        let mut planner = DagPlanner::new(ai_service.clone(), self.config.lite_mode.clone());
        
        if let Some(adapter) = &self.framework_adapter {
            planner = planner.with_tool_adapter(adapter.clone());
        }
        if let Some(repo) = &self.prompt_repo {
            planner = planner.with_prompt_repo(repo.clone());
        }

        self.emit_message(
            execution_id,
            message_id,
            conversation_id,
            ChunkType::Thinking,
            "[PLANNING] Generating DAG execution plan...",
            None,
        );

        // 生成 DAG 计划 (单次 LLM 调用)
        let plan = planner.generate_plan(&task.description, context).await?;

        self.emit_message(
            execution_id,
            message_id,
            conversation_id,
            ChunkType::PlanInfo,
            &format!("[SUCCESS] Plan generated with {} tasks", plan.tasks.len()),
            Some(serde_json::json!({
                "task_count": plan.tasks.len(),
                "tasks": plan.tasks.iter().map(|t| &t.tool_name).collect::<Vec<_>>()
            })),
        );

        // 缓存计划
        if self.config.lite_mode.enable_plan_cache {
            self.context_manager.cache_plan(
                &task_hash,
                plan.clone(),
                self.config.lite_mode.plan_cache_ttl,
            ).await;
        }

        self.execute_dag_plan(plan, context, execution_id, message_id, conversation_id).await
    }

    /// 执行 DAG 计划
    async fn execute_dag_plan(
        &self,
        mut plan: DagPlan,
        context: &mut HashMap<String, serde_json::Value>,
        execution_id: &str,
        message_id: &str,
        conversation_id: Option<&str>,
    ) -> Result<AgentExecutionResult> {
        let start_time = Instant::now();

        // 创建并行执行器
        let mut executor = ParallelExecutor::new(self.config.parallel_config.clone());
        
        if let Some(adapter) = &self.framework_adapter {
            executor = executor.with_tool_adapter(adapter.clone());
        }
        
        if let Some(app) = &self.app_handle {
            executor = executor.with_message_context(
                Arc::new(app.clone()),
                execution_id.to_string(),
                message_id.to_string(),
                conversation_id.map(|s| s.to_string()),
            );
        }

        // 执行 DAG
        let result = executor.execute_dag(&mut plan).await?;

        let duration = start_time.elapsed().as_millis() as u64;

        // 构建结果
        let success = result.success;
        let output = result.final_output.clone().unwrap_or(serde_json::json!({}));

        self.emit_message(
            execution_id,
            message_id,
            conversation_id,
            ChunkType::Content,
            &format!(
                "📊 DAG execution completed: {} succeeded, {} failed ({}ms saved ~{} tokens)",
                result.metrics.completed_tasks,
                result.metrics.failed_tasks,
                duration,
                result.metrics.tokens_saved
            ),
            Some(serde_json::json!({
                "metrics": result.metrics
            })),
        );

        Ok(AgentExecutionResult {
            id: plan.id,
            success,
            data: Some(serde_json::json!({
                "output": output,
                "mode": "lite_dag",
                "metrics": result.metrics,
                "task_results": result.task_results,
            })),
            error: if success { None } else { Some("Some tasks failed".to_string()) },
            execution_time_ms: duration,
            resources_used: HashMap::new(),
            artifacts: Vec::new(),
        })
    }

    /// 完整OODA模式执行
    async fn execute_full_ooda_mode(
        &self,
        task: &AgentTask,
        task_complexity: TaskComplexity,
        context: &mut HashMap<String, serde_json::Value>,
        execution_id: &str,
        message_id: &str,
        conversation_id: Option<String>,
    ) -> Result<AgentExecutionResult> {
        // 初始化执行轨迹
        let mut trace = TravelTrace::new(task.description.clone(), task_complexity.clone());

        // 为OodaExecutor配置消息发送
        let mut executor = OodaExecutor::new(self.config.clone());
        
        if let Some(app_handle) = &self.app_handle {
            executor = executor.with_app_handle(Arc::new(app_handle.clone()));
        }
        
        executor = executor
            .with_message_ids(execution_id.to_string(), message_id.to_string(), conversation_id.clone());
        
        // 设置dispatcher和其他依赖
        let mut dispatcher = EngineDispatcher::new();
        if let Some(ai_service) = &self.ai_service {
            dispatcher = dispatcher.with_ai_service(ai_service.clone());
        }
        if let Some(repo) = &self.prompt_repo {
            dispatcher = dispatcher.with_prompt_repo(repo.clone());
        }
        if let Some(adapter) = &self.framework_adapter {
            dispatcher = dispatcher.with_framework_adapter(adapter.clone());
        }
        if let Some(app) = &self.app_handle {
            dispatcher = dispatcher.with_app_handle(app.clone());
        }
        
        executor = executor.with_engine_dispatcher(dispatcher);

        // 执行OODA循环
        for cycle_num in 1..=self.config.max_ooda_cycles {
            log::info!("Starting OODA cycle {}/{}", cycle_num, self.config.max_ooda_cycles);

            // 检查是否应该继续循环
            if self.should_stop_cycles(&trace, context) {
                log::info!("Stopping OODA cycles: task completed or max cycles reached");
                break;
            }

            // 执行单次OODA循环
            match executor
                .execute_cycle(cycle_num, &task.description, task_complexity.clone(), context)
                .await
            {
                Ok(cycle) => {
                    let cycle_success = cycle.status == OodaCycleStatus::Completed;
                    trace.add_cycle(cycle);

                    // 更新指标
                    self.update_trace_metrics(&mut trace);

                    // 如果循环成功且任务完成,退出
                    if cycle_success && self.is_task_complete(context) {
                        log::info!("Task completed successfully after {} cycles", cycle_num);
                        break;
                    }
                }
                Err(e) => {
                    log::error!("OODA cycle {} failed: {}", cycle_num, e);
                    trace.fail(format!("Cycle {} failed: {}", cycle_num, e));
                    break;
                }
            }
        }

        // 完成轨迹
        if trace.status == TravelStatus::Running {
            if trace.ooda_cycles.len() >= self.config.max_ooda_cycles as usize {
                trace.status = TravelStatus::MaxCyclesReached;
            } else {
                let final_result = context
                    .get("execution_result")
                    .cloned()
                    .unwrap_or(serde_json::json!({}));
                trace.complete(final_result);
            }
        }

        // 转换为AgentExecutionResult
        self.trace_to_result(trace)
    }

    /// 准备执行上下文
    fn prepare_context(&self, task: &AgentTask) -> Result<HashMap<String, serde_json::Value>> {
        let mut context = HashMap::new();

        // 从任务参数中提取信息
        for (key, value) in &task.parameters {
            context.insert(key.clone(), value.clone());
        }

        // 添加目标信息
        if let Some(target) = task.parameters.get("target") {
            context.insert(
                "target_info".to_string(),
                serde_json::json!({
                    "target": target,
                    "authorized": task.parameters.get("authorized").and_then(|v| v.as_bool()).unwrap_or(false),
                }),
            );
        }

        Ok(context)
    }

    /// 判断是否应该停止循环
    fn should_stop_cycles(&self, trace: &TravelTrace, context: &HashMap<String, serde_json::Value>) -> bool {
        // 如果已经达到最大循环次数
        if trace.ooda_cycles.len() >= self.config.max_ooda_cycles as usize {
            return true;
        }

        // 如果任务已完成
        if self.is_task_complete(context) {
            return true;
        }

        // 如果上一个循环失败
        if let Some(last_cycle) = trace.ooda_cycles.last() {
            if last_cycle.status == OodaCycleStatus::Failed {
                return true;
            }
        }

        false
    }

    /// 判断任务是否完成
    fn is_task_complete(&self, context: &HashMap<String, serde_json::Value>) -> bool {
        // 检查是否有执行结果
        if let Some(result) = context.get("execution_result") {
            if let Some(status) = result.get("status").and_then(|v| v.as_str()) {
                return status == "success" || status == "completed";
            }
            // 如果有结果就认为完成
            return true;
        }
        false
    }

    /// 更新轨迹指标
    fn update_trace_metrics(&self, trace: &mut TravelTrace) {
        if let Some(last_cycle) = trace.ooda_cycles.last() {
            // 统计工具调用
            for phase in &last_cycle.phase_history {
                trace.metrics.total_tool_calls += phase.tool_calls.len() as u32;
            }

            // 统计护栏检查
            for phase in &last_cycle.phase_history {
                trace.metrics.guardrail_checks += phase.guardrail_checks.len() as u32;
                trace.metrics.guardrail_failures += phase
                    .guardrail_checks
                    .iter()
                    .filter(|c| c.result == GuardrailCheckStatus::Failed)
                    .count() as u32;
            }

            // 统计回退
            for phase in &last_cycle.phase_history {
                if phase.status == PhaseExecutionStatus::RolledBack {
                    trace.metrics.rollback_count += 1;
                }
            }
        }

        // 计算总执行时间
        if let Some(started) = trace.started_at.elapsed().ok() {
            trace.metrics.total_duration_ms = started.as_millis() as u64;
        }
    }

    /// 将TravelTrace转换为AgentExecutionResult
    fn trace_to_result(&self, trace: TravelTrace) -> Result<AgentExecutionResult> {
        let success = trace.status == TravelStatus::Completed;

        // 提取最终结果
        let output = if let Some(final_result) = &trace.final_result {
            final_result.clone()
        } else {
            serde_json::json!({
                "status": format!("{:?}", trace.status),
                "cycles": trace.ooda_cycles.len(),
                "message": "Travel execution completed",
            })
        };

        // 提取错误信息
        let error = if !success {
            Some(format!("Travel execution failed with status: {:?}", trace.status))
        } else {
            None
        };

        Ok(AgentExecutionResult {
            id: trace.trace_id.clone(),
            success,
            data: Some(serde_json::json!({
                "output": output,
                "trace_id": trace.trace_id,
                "task_complexity": format!("{:?}", trace.task_complexity),
                "total_cycles": trace.metrics.total_cycles,
                "total_tool_calls": trace.metrics.total_tool_calls,
                "guardrail_checks": trace.metrics.guardrail_checks,
                "guardrail_failures": trace.metrics.guardrail_failures,
                "rollback_count": trace.metrics.rollback_count,
                "duration_ms": trace.metrics.total_duration_ms,
                "status": format!("{:?}", trace.status),
            })),
            error,
            execution_time_ms: trace.metrics.total_duration_ms,
            resources_used: HashMap::new(),
            artifacts: Vec::new(),
        })
    }
}

// 实现BaseExecutionEngine trait
#[async_trait]
impl BaseExecutionEngine for TravelEngine {
    fn get_name(&self) -> &str {
        "Travel"
    }

    fn get_description(&self) -> &str {
        "OODA (Observe-Orient-Decide-Act) loop based security testing agent with intelligent task complexity analysis and multi-engine dispatch"
    }

    fn get_version(&self) -> &str {
        "1.0.0"
    }

    fn get_supported_scenarios(&self) -> Vec<String> {
        vec![
            "penetration_testing".to_string(),
            "vulnerability_assessment".to_string(),
            "security_scanning".to_string(),
            "threat_analysis".to_string(),
            "red_team_operations".to_string(),
            "code_audit".to_string(),
            "network_reconnaissance".to_string(),
        ]
    }

    fn get_performance_characteristics(&self) -> PerformanceCharacteristics {
        // Token效率根据配置动态调整
        let token_efficiency = if self.config.lite_mode.enabled { 85 } else { 70 };
        let execution_speed = if self.config.parallel_config.enabled { 75 } else { 60 };
        let concurrency = if self.config.parallel_config.enabled { 90 } else { 80 };
        
        PerformanceCharacteristics {
            token_efficiency,     // 85 精简模式 / 70 完整模式
            execution_speed,      // 75 并行执行 / 60 串行
            resource_usage: 70,   // 70 有资源追踪 / 60 无追踪
            concurrency_capability: concurrency, // 90 并行 / 80 串行
            complexity_handling: 95, // 优秀,专为复杂安全测试设计
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_metadata() {
        let engine = TravelEngine::with_defaults();
        assert_eq!(engine.get_name(), "Travel");
        assert!(engine
            .get_supported_scenarios()
            .contains(&"penetration_testing".to_string()));
    }

    // #[test]
    // fn test_prepare_context() {
    //     let engine = TravelEngine::with_defaults();
    //     let mut task = AgentTask {
    //         id: "test".to_string(),
    //         description: "Test task".to_string(),
    //         parameters: HashMap::new(),
    //         target: Some("localhost".to_string()),
    //         user_id: "test".to_string(),
    //         priority: TaskPriority::Normal,
    //         timeout: Some(10000),
    //     };

    //     task.parameters.insert(
    //         "target".to_string(),
    //         serde_json::Value::String("localhost".to_string()),
    //     );

    //     let context = engine.prepare_context(&task).unwrap();
    //     assert!(context.contains_key("target"));
    //     assert!(context.contains_key("target_info"));
    // }
}

