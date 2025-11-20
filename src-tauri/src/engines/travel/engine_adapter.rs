//! Travel引擎适配器
//!
//! 实现BaseExecutionEngine trait,对接AI服务、工具调用等

use super::types::*;
use super::complexity_analyzer::ComplexityAnalyzer;
use super::ooda_executor::OodaExecutor;
use super::engine_dispatcher::EngineDispatcher;
use crate::agents::traits::{
    AgentExecutionResult, AgentSession, AgentTask, PerformanceCharacteristics,
};
use crate::engines::traits::BaseExecutionEngine;
use crate::services::ai::AiService;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// Travel引擎
pub struct TravelEngine {
    config: TravelConfig,
    complexity_analyzer: ComplexityAnalyzer,
    ooda_executor: OodaExecutor,
    ai_service: Option<Arc<AiService>>,
    prompt_repo: Option<Arc<crate::services::prompt_db::PromptRepository>>,
    framework_adapter: Option<Arc<dyn crate::tools::FrameworkToolAdapter>>,
    app_handle: Option<tauri::AppHandle>,
}

impl TravelEngine {
    /// 创建新的Travel引擎
    pub fn new(config: TravelConfig) -> Self {
        let complexity_analyzer = ComplexityAnalyzer::new(config.complexity_config.clone());
        let ooda_executor = OodaExecutor::new(config.clone());

        Self {
            config,
            complexity_analyzer,
            ooda_executor,
            ai_service: None,
            prompt_repo: None,
            framework_adapter: None,
            app_handle: None,
        }
    }

    /// 使用默认配置创建
    pub fn with_defaults() -> Self {
        Self::new(TravelConfig::default())
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

    /// 执行Travel流程
    pub async fn execute(
        &self,
        task: &AgentTask,
        _session: &mut dyn AgentSession,
    ) -> Result<AgentExecutionResult> {
        log::info!("Travel engine executing task: {}", task.description);

        // 1. 分析任务复杂度
        let task_complexity = self
            .complexity_analyzer
            .analyze_task_complexity(&task.description, Some(&task.parameters))
            .await?;

        log::info!("Task complexity determined: {:?}", task_complexity);

        // 2. 初始化执行轨迹
        let mut trace = TravelTrace::new(task.description.clone(), task_complexity.clone());

        // 3. 准备执行上下文
        let mut context = self.prepare_context(task)?;

        // 4. 执行OODA循环
        for cycle_num in 1..=self.config.max_ooda_cycles {
            log::info!("Starting OODA cycle {}/{}", cycle_num, self.config.max_ooda_cycles);

            // 检查是否应该继续循环
            if self.should_stop_cycles(&trace, &context) {
                log::info!("Stopping OODA cycles: task completed or max cycles reached");
                break;
            }

            // 执行单次OODA循环
            match self
                .ooda_executor
                .execute_cycle(cycle_num, &task.description, task_complexity.clone(), &mut context)
                .await
            {
                Ok(cycle) => {
                    let cycle_success = cycle.status == OodaCycleStatus::Completed;
                    trace.add_cycle(cycle);

                    // 更新指标
                    self.update_trace_metrics(&mut trace);

                    // 如果循环成功且任务完成,退出
                    if cycle_success && self.is_task_complete(&context) {
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

        // 5. 完成轨迹
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

        // 6. 转换为AgentExecutionResult
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
        PerformanceCharacteristics {
            token_efficiency: 70, // 较好,通过智能调度减少不必要的LLM调用
            execution_speed: 60,  // 中等,OODA循环需要时间但有护栏保护
            resource_usage: 60,   // 中等,多阶段执行但有资源限制
            concurrency_capability: 80, // 良好,可以并行执行多个OODA循环
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

