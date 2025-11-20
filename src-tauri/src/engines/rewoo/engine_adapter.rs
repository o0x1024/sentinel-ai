//! ReWOO引擎适配器
//! 
//! 集成Planner、Worker和Solver，提供完整的ReWOO执行流程

use crate::agents::traits::*;
use crate::engines::traits::BaseExecutionEngine;
use crate::engines::rewoo::{
    rewoo_planner::ReWOOPlanner,
    rewoo_worker::ReWOOWorker,
    rewoo_solver::ReWOOSolver,
    rewoo_types::*,
};
use crate::engines::memory::get_global_memory;
use crate::services::ai::AiServiceManager;
use crate::services::prompt_db::PromptRepository;
use crate::services::database::DatabaseService;
use async_trait::async_trait;
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tracing::{info, warn, error};
use uuid::Uuid;

/// ReWOO引擎适配器
pub struct ReWooEngine {
    engine_info: EngineInfo,
    planner: ReWOOPlanner,
    worker: ReWOOWorker,
    solver: ReWOOSolver,
    config: ReWOOConfig,
    ai_service_manager: Arc<AiServiceManager>,
    prompt_repo: Arc<PromptRepository>,
    runtime_params: Option<HashMap<String, serde_json::Value>>,
    app_handle: Option<tauri::AppHandle>,
    db_service: Arc<DatabaseService>,
}

impl ReWooEngine {
    /// 使用完整依赖创建引擎适配器
    pub async fn new_with_dependencies(
        ai_service_manager: Arc<AiServiceManager>,
        config: ReWOOConfig,
        db_service: Arc<DatabaseService>,
    ) -> Result<Self> {
        info!("Initializing ReWOO Engine");
        
        // 获取prompt仓库
        let db = db_service.get_db()?;
        let prompt_repo = Arc::new(PromptRepository::new(db.pool().clone()));
        
        // 创建Worker（先创建以获取framework_adapter）
        let framework_adapter = crate::tools::get_framework_adapter(crate::tools::FrameworkType::ReWOO).await?;
        let worker = ReWOOWorker::new(framework_adapter.clone(), config.worker.clone());
        
        // 创建Planner（传递framework_adapter用于获取工具详细信息）
        let planner = ReWOOPlanner::new(
            ai_service_manager.clone(),
            prompt_repo.clone(),
            config.planner.clone(),
            framework_adapter.clone(),
        )?;
        
        // 创建Solver
        let solver = ReWOOSolver::new(
            ai_service_manager.clone(),
            prompt_repo.clone(),
            config.solver.clone(),
        )?;
        
        let engine_info = EngineInfo {
            name: "ReWOO".to_string(),
            version: "1.0.0".to_string(),
            description: "Reasoning without Observation - Efficient task execution with upfront planning".to_string(),
            supported_scenarios: vec![
                "information_retrieval".to_string(),
                "multi_step_reasoning".to_string(),
                "tool_orchestration".to_string(),
            ],
            performance_characteristics: PerformanceCharacteristics {
                token_efficiency: 90,
                execution_speed: 85,
                resource_usage: 70,
                concurrency_capability: 80,
                complexity_handling: 85,
            },
        };

        Ok(Self {
            engine_info,
            planner,
            worker,
            solver,
            config,
            ai_service_manager,
            prompt_repo,
            runtime_params: None,
            app_handle: None,
            db_service,
        })
    }
    
    /// 设置运行时参数
    pub fn set_runtime_params(&mut self, params: HashMap<String, serde_json::Value>) {
        self.runtime_params = Some(params.clone());
        self.worker.set_runtime_params(params.clone());
        self.planner.set_runtime_params(params);
    }
    
    /// 设置应用句柄
    pub fn set_app_handle(&mut self, app_handle: tauri::AppHandle) {
        self.app_handle = Some(app_handle);
    }
    
    /// 执行ReWOO流程
    pub async fn execute(&mut self, task: &AgentTask) -> Result<AgentExecutionResult> {
        // 从任务参数中提取前端消息ID
        let conversation_id = task.parameters.get("conversation_id")
            .and_then(|v| v.as_str()).map(|s| s.to_string());
        let message_id = task.parameters.get("message_id")
            .and_then(|v| v.as_str()).map(|s| s.to_string());
        let execution_id = task.parameters.get("execution_id")
            .and_then(|v| v.as_str()).map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        
        info!("ReWOO Engine: Starting execution {}", execution_id);
        info!("ReWOO: conversation_id={:?}, message_id={:?}, execution_id={:?}", 
            conversation_id, message_id, execution_id);
        
        let start_time = SystemTime::now();
        
        // 阶段1: 规划
        info!("ReWOO Engine: Phase 1 - Planning");
        
        // 发送Planning阶段开始消息
        if let Some(app) = &self.app_handle {
            crate::utils::ordered_message::emit_thinking_chunk(
                app,
                &execution_id,
                message_id.as_deref().unwrap_or(&execution_id),
                conversation_id.as_deref(),
                "开始生成执行计划...",
                Some("rewoo_planning"),
            );
        }
        
        let available_tools = self.worker.get_available_tools().await;
        let plan = self.planner.plan(
            &task.description,
            &available_tools,
            None,
            &execution_id,
        ).await?;
        
        info!("ReWOO Engine: Generated plan with {} steps", plan.steps.len());
        
        // 发送计划信息到前端（JSON格式，更可靠）
        if let Some(app) = &self.app_handle {
            // 构建JSON格式的计划信息
            let plan_json = serde_json::json!({
                "plan_summary": format!("执行完整的安全测试流程，共{}个步骤", plan.steps.len()),
                "steps": plan.steps.iter().enumerate().map(|(i, s)| {
                    serde_json::json!({
                        "id": format!("E{}", i + 1),
                        "tool": s.tool_name,
                        "description": s.description,
                        "args": s.tool_args
                    })
                }).collect::<Vec<_>>()
            });
            
            let plan_info = serde_json::to_string(&plan_json)
                .unwrap_or_else(|_| format!("生成执行计划成功，共{}个步骤", plan.steps.len()));
            
            crate::utils::ordered_message::emit_plan_info_chunk(
                app,
                &execution_id,
                message_id.as_deref().unwrap_or(&execution_id),
                conversation_id.as_deref(),
                &plan_info,
                Some("rewoo_planning"),
                None,
            );
        }
        
        // 阶段2: 执行工具
        info!("ReWOO Engine: Phase 2 - Tool Execution");
        
        // 发送工具执行阶段开始消息
        if let Some(app) = &self.app_handle {
            crate::utils::ordered_message::emit_thinking_chunk(
                app,
                &execution_id,
                message_id.as_deref().unwrap_or(&execution_id),
                conversation_id.as_deref(),
                "开始执行工具...",
                Some("rewoo_execution"),
            );
        }
        
        let mut tool_results: HashMap<String, serde_json::Value> = HashMap::new();
        let mut state = ReWOOState {
            task: task.description.clone(),
            plan_string: plan.reasoning.clone(),
            steps: plan.steps.iter().map(|s| s.step_id.clone()).collect(),
            results: HashMap::new(),
            result: String::new(),
            execution_trace: None,
        };
        
        // 解析计划步骤为PlanStep
        for (idx, rewoo_step) in plan.steps.iter().enumerate() {
            let plan_step = PlanStep {
                variable: rewoo_step.step_id.clone(),
                tool: rewoo_step.tool_name.clone(),
                args: serde_json::to_string(&rewoo_step.tool_args).unwrap_or_default(),
                reasoning: rewoo_step.description.clone(),
            };
            
            // 发送工具执行开始消息
            if let Some(app) = &self.app_handle {
                let tool_start_msg = format!(
                    "执行步骤 {}/{}: {} - {}",
                    idx + 1,
                    plan.steps.len(),
                    plan_step.tool,
                    plan_step.reasoning
                );
                crate::utils::ordered_message::emit_thinking_chunk_with_tool(
                    app,
                    &execution_id,
                    message_id.as_deref().unwrap_or(&execution_id),
                    conversation_id.as_deref(),
                    &tool_start_msg,
                    Some("rewoo_execution"),
                    Some(&plan_step.tool),
                );
            }
            
            // 替换参数中的变量引用
            let substituted_args = self.substitute_variables(&plan_step.args, &tool_results);
            
            // 执行工具
            match self.worker.execute_tool_with_retry(
                &plan_step,
                &substituted_args,
                self.config.worker.max_retries,
            ).await {
                Ok(result) => {
                    if result.success {
                        let result_value = result.json_content.unwrap_or_else(|| {
                            serde_json::Value::String(result.content.clone())
                        });
                        tool_results.insert(plan_step.variable.clone(), result_value.clone());
                        state.results.insert(plan_step.variable.clone(), result_value.clone());
                        
                        // 发送工具执行结果到前端
                        if let Some(app) = &self.app_handle {
                            let result_str = serde_json::to_string_pretty(&result_value)
                                .unwrap_or_else(|_| result.content.clone());
                            crate::utils::ordered_message::emit_tool_result_chunk(
                                app,
                                &execution_id,
                                message_id.as_deref().unwrap_or(&execution_id),
                                conversation_id.as_deref(),
                                &result_str,
                                Some("rewoo_execution"),
                                Some(&plan_step.tool),
                            );
                        }
                        
                        info!("ReWOO Engine: Step {} completed successfully", plan_step.variable);
                    } else {
                        let error_msg = format!("Tool execution failed: {:?}", result.error);
                        warn!("ReWOO Engine: Step {} failed: {}", plan_step.variable, error_msg);
                        
                        // 发送错误消息到前端
                        if let Some(app) = &self.app_handle {
                            crate::utils::ordered_message::emit_error_chunk(
                                app,
                                &execution_id,
                                message_id.as_deref().unwrap_or(&execution_id),
                                conversation_id.as_deref(),
                                &error_msg,
                                Some("rewoo_execution"),
                                Some(&plan_step.tool),
                            );
                        }
                        
                        return Err(anyhow!(error_msg));
                    }
                }
                Err(e) => {
                    error!("ReWOO Engine: Step {} error: {}", plan_step.variable, e);
                    
                    // 发送错误消息到前端
                    if let Some(app) = &self.app_handle {
                        let error_msg = format!("工具执行错误: {}", e);
                        crate::utils::ordered_message::emit_error_chunk(
                            app,
                            &execution_id,
                            message_id.as_deref().unwrap_or(&execution_id),
                            conversation_id.as_deref(),
                            &error_msg,
                            Some("rewoo_execution"),
                            Some(&plan_step.tool),
                        );
                    }
                    
                    return Err(anyhow!("Tool execution error: {}", e));
                }
            }
        }
        
        // 阶段3: 求解
        info!("ReWOO Engine: Phase 3 - Solving");
        
        // 发送求解阶段开始消息
        if let Some(app) = &self.app_handle {
            crate::utils::ordered_message::emit_thinking_chunk(
                app,
                &execution_id,
                message_id.as_deref().unwrap_or(&execution_id),
                conversation_id.as_deref(),
                "开始生成最终答案...",
                Some("rewoo_solving"),
            );
        }
        
        let final_answer = self.solver.solve(
            &task.description,
            &plan.reasoning,
            &tool_results,
            &execution_id,
        ).await?;
        
        state.result = final_answer.clone();
        
        // 存储成功的ReWOO规划蓝图到记忆系统
        let memory = get_global_memory();
        let mut memory_guard = memory.write().await;
        let plan_steps_json: Vec<serde_json::Value> = plan.steps.iter().map(|step| {
            serde_json::json!({
                "step_id": step.step_id,
                "tool_name": step.tool_name,
                "description": step.description,
                "tool_args": step.tool_args,
            })
        }).collect();
        
        let _ = memory_guard.store_rewoo_plan_blueprint(
            task.description.clone(),
            plan_steps_json,
            1.0, // 成功执行，成功率为1.0
        );
        drop(memory_guard);
        
        let execution_time = start_time.elapsed().unwrap_or_default();
        info!("ReWOO Engine: Execution completed in {:?}", execution_time);
        
        // 发送最终答案到前端（注意：Solver内部已经通过AI服务流式发送了内容）
        // 这里只需要发送元数据信息
        if let Some(app) = &self.app_handle {
            let meta_info = format!(
                "执行完成，耗时: {:.2}秒，执行步骤: {}",
                execution_time.as_secs_f64(),
                plan.steps.len()
            );
            crate::utils::ordered_message::emit_meta_chunk(
                app,
                &execution_id,
                message_id.as_deref().unwrap_or(&execution_id),
                conversation_id.as_deref(),
                &meta_info,
                None,
            );
        }
        
        Ok(AgentExecutionResult {
            id: execution_id.clone(),
            success: true,
            data: Some(serde_json::json!({
                "result": final_answer,
                "plan": plan.reasoning,
                "steps": plan.steps.len(),
                "execution_id": execution_id,
            })),
            error: None,
            execution_time_ms: execution_time.as_millis() as u64,
            resources_used: HashMap::new(),
            artifacts: Vec::new(),
        })
    }
    
    /// 替换参数中的变量引用（改进版：保持JSON结构完整性）
    fn substitute_variables(
        &self,
        args_str: &str,
        results: &HashMap<String, serde_json::Value>,
    ) -> String {
        // 尝试解析为JSON并进行智能替换
        if let Ok(mut json_value) = serde_json::from_str::<serde_json::Value>(args_str) {
            self.substitute_variables_in_json(&mut json_value, results);
            // 返回替换后的JSON字符串
            return serde_json::to_string(&json_value).unwrap_or_else(|_| args_str.to_string());
        }
        
        // 如果不是JSON，回退到简单字符串替换
        let mut substituted = args_str.to_string();
        for (var, value) in results {
            let value_str = match value {
                serde_json::Value::String(s) => s.clone(),
                _ => serde_json::to_string(value).unwrap_or_else(|_| value.to_string()),
            };
            substituted = substituted.replace(var, &value_str);
        }
        
        substituted
    }
    
    /// 递归替换JSON中的变量引用
    fn substitute_variables_in_json(
        &self,
        json_value: &mut serde_json::Value,
        results: &HashMap<String, serde_json::Value>,
    ) {
        match json_value {
            serde_json::Value::String(s) => {
                // 检查是否是变量引用
                if s.starts_with('#') && results.contains_key(s.as_str()) {
                    // 直接替换为结果值（保持类型）
                    *json_value = results[s.as_str()].clone();
                }
            }
            serde_json::Value::Array(arr) => {
                for item in arr.iter_mut() {
                    self.substitute_variables_in_json(item, results);
                }
            }
            serde_json::Value::Object(obj) => {
                for (_key, value) in obj.iter_mut() {
                    self.substitute_variables_in_json(value, results);
                }
            }
            _ => {}
        }
    }
}

impl std::fmt::Debug for ReWooEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReWooEngine")
            .field("engine_info", &self.engine_info)
            .field("config", &self.config)
            .finish()
    }
}

#[async_trait]
impl BaseExecutionEngine for ReWooEngine {
    fn get_name(&self) -> &str {
        &self.engine_info.name
    }
    
    fn get_description(&self) -> &str {
        &self.engine_info.description
    }
    
    fn get_version(&self) -> &str {
        &self.engine_info.version
    }
    
    fn get_supported_scenarios(&self) -> Vec<String> {
        self.engine_info.supported_scenarios.clone()
    }
    
    fn get_performance_characteristics(&self) -> PerformanceCharacteristics {
        self.engine_info.performance_characteristics.clone()
    }
}

// 实现 ExecutionEngine trait 以支持统一的引擎接口
#[async_trait]
impl ExecutionEngine for ReWooEngine {
    fn get_engine_info(&self) -> &EngineInfo {
        &self.engine_info
    }
    
    fn supports_task(&self, _task: &AgentTask) -> bool {
        // ReWOO 引擎支持大多数任务类型
        true
    }
    
    async fn create_plan(&self, task: &AgentTask) -> anyhow::Result<ExecutionPlan> {
        // ReWOO 使用 Planner 生成执行计划
        let available_tools = self.worker.get_available_tools().await;
        let plan = self.planner.plan(
            &task.description,
            &available_tools,
            None,
            &task.id,
        ).await.map_err(|e| anyhow::anyhow!("Failed to create ReWOO plan: {}", e))?;
        
        // 转换为 ExecutionPlan
        let steps = plan.steps.iter().enumerate().map(|(idx, step)| {
            ExecutionStep {
                id: format!("step_{}", idx + 1),
                name: step.tool_name.clone(),
                description: step.description.clone(),
                step_type: crate::agents::traits::StepType::ToolCall,
                dependencies: Vec::new(),
                parameters: step.tool_args.clone(),
            }
        }).collect();
        
        Ok(ExecutionPlan {
            id: task.id.clone(),
            name: format!("ReWOO Plan: {}", task.description),
            steps,
            estimated_duration: plan.steps.len() as u64 * 30, // 每步预估30秒
            resource_requirements: ResourceRequirements {
                cpu_cores: Some(1),
                memory_mb: Some(512),
                network_concurrency: Some(1),
                disk_space_mb: Some(100),
            },
        })
    }
    
    async fn execute_plan(&self, plan: &ExecutionPlan) -> anyhow::Result<AgentExecutionResult> {
        // 从 plan 构造 AgentTask
        let task = AgentTask {
            id: plan.id.clone(),
            user_id: "system".to_string(),
            description: plan.name.clone(),
            priority: TaskPriority::Normal,
            target: None,
            parameters: HashMap::new(),
            timeout: Some(300),
        };
        
        // 执行 ReWOO 流程 - 直接调用内部逻辑，避免 &mut self 问题
        let start_time = SystemTime::now();
        
        // 阶段1: 规划
        let available_tools = self.worker.get_available_tools().await;
        let plan_result = self.planner.plan(
            &task.description,
            &available_tools,
            None,
            &task.id,
        ).await?;
        
        // 阶段2: 执行工具
        let mut tool_results: HashMap<String, serde_json::Value> = HashMap::new();
        
        for rewoo_step in &plan_result.steps {
            let plan_step = PlanStep {
                variable: rewoo_step.step_id.clone(),
                tool: rewoo_step.tool_name.clone(),
                args: serde_json::to_string(&rewoo_step.tool_args).unwrap_or_default(),
                reasoning: rewoo_step.description.clone(),
            };
            
            // 替换参数中的变量引用
            let substituted_args = self.substitute_variables(&plan_step.args, &tool_results);
            
            // 执行工具
            match self.worker.execute_tool_with_retry(
                &plan_step,
                &substituted_args,
                self.config.worker.max_retries,
            ).await {
                Ok(result) => {
                    if result.success {
                        let result_value = result.json_content.unwrap_or_else(|| {
                            serde_json::Value::String(result.content.clone())
                        });
                        tool_results.insert(plan_step.variable.clone(), result_value);
                    } else {
                        return Err(anyhow!("Tool execution failed: {:?}", result.error));
                    }
                }
                Err(e) => {
                    return Err(anyhow!("Tool execution error: {}", e));
                }
            }
        }
        
        // 阶段3: 求解
        let final_answer = self.solver.solve(
            &task.description,
            &plan_result.reasoning,
            &tool_results,
            &task.id,
        ).await?;
        
        let execution_time = start_time.elapsed().unwrap_or_default();
        
        Ok(AgentExecutionResult {
            id: task.id.clone(),
            success: true,
            data: Some(serde_json::json!({
                "result": final_answer,
                "plan": plan_result.reasoning,
                "steps": plan_result.steps.len(),
            })),
            error: None,
            execution_time_ms: execution_time.as_millis() as u64,
            resources_used: HashMap::new(),
            artifacts: Vec::new(),
        })
    }
    
    async fn get_progress(&self, _session_id: &str) -> anyhow::Result<ExecutionProgress> {
        // ReWOO 不支持细粒度进度跟踪，返回简化的进度
        Ok(ExecutionProgress {
            completed_steps: 1,
            progress_percentage: 100.0,
            estimated_remaining_seconds: Some(0),
            current_step: Some("Completed".to_string()),
            total_steps: 1,
        })
    }
    
    async fn cancel_execution(&self, _session_id: &str) -> anyhow::Result<()> {
        // ReWOO 不支持取消执行
        log::warn!("ReWOO engine does not support cancellation");
        Ok(())
    }
}
