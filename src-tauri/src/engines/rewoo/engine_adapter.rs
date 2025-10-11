//! ReWOO引擎适配器 - 实现统一的ExecutionEngine接口

use crate::agents::traits::*;
use crate::engines::rewoo::{
    rewoo_planner::ReWOOPlanner,
    rewoo_worker::ReWOOWorker,
    rewoo_solver::ReWOOSolver,
    rewoo_types::*,
};
use crate::ai_adapter::AiProvider;
use crate::services::ai::AiServiceManager;
use crate::services::prompt_db::PromptRepository;
use crate::services::database::DatabaseService;
use async_trait::async_trait;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use uuid::Uuid;
use crate::utils::ordered_message::{ChunkType, emit_message_chunk};

/// ReWOO引擎适配器 - 集成原有引擎逻辑
pub struct ReWooEngine {
    engine_info: EngineInfo,
    planner: Option<ReWOOPlanner>,
    worker: Option<ReWOOWorker>,
    solver: Option<ReWOOSolver>,
    config: ReWOOConfig,
    sessions: HashMap<String, ReWOOSession>,
    ai_service_manager: Option<Arc<AiServiceManager>>,
    runtime_params: Option<std::collections::HashMap<String, serde_json::Value>>,
    app_handle: Option<tauri::AppHandle>,
}

impl ReWooEngine {
    /// 创建新的引擎适配器
    pub async fn new() -> Result<Self> {
        let engine_info = EngineInfo {
            name: "ReWOO".to_string(),
            version: "1.0.0".to_string(),
            description: "Reasoning without Observation architecture with variable substitution and parallel tool execution".to_string(),
            supported_scenarios: vec![
                "API Integration".to_string(),
                "Batch Processing".to_string(),
                "Tool Chain Execution".to_string(),
                "Information Retrieval".to_string(),
            ],
            performance_characteristics: PerformanceCharacteristics {
                token_efficiency: 85,
                execution_speed: 80,
                resource_usage: 50,
                concurrency_capability: 70,
                complexity_handling: 60,
            },
        };

        // 创建默认配置
        let config = ReWOOConfig {
            planner: PlannerConfig {
                model_name: "gpt-4".to_string(),
                temperature: 0.0,
                max_tokens: 1000,
                max_steps: 10,
            },
            worker: WorkerConfig {
                timeout_seconds: 30,
                max_retries: 3,
                enable_parallel: true,
            },
            solver: SolverConfig {
                model_name: "gpt-4".to_string(),
                temperature: 0.0,
                max_tokens: 2000,
            },
        };

        Ok(Self {
            engine_info,
            planner: None,
            worker: None,
            solver: None,
            config,
            sessions: HashMap::new(),
            ai_service_manager: None,
            runtime_params: None,
            app_handle: None,
        })
    }
    
    /// 使用完整依赖创建引擎适配器
    pub async fn new_with_dependencies(
        ai_service_manager: Arc<AiServiceManager>,
        config: ReWOOConfig,
        db_service: Arc<DatabaseService>,
    ) -> Result<Self> {
        let engine_info = EngineInfo {
            name: "ReWOO".to_string(),
            version: "1.0.0".to_string(),
            description: "Reasoning without Observation architecture with variable substitution and parallel tool execution".to_string(),
            supported_scenarios: vec![
                "API Integration".to_string(),
                "Batch Processing".to_string(),
                "Tool Chain Execution".to_string(),
                "Information Retrieval".to_string(),
            ],
            performance_characteristics: PerformanceCharacteristics {
                token_efficiency: 85,
                execution_speed: 80,
                resource_usage: 50,
                concurrency_capability: 70,
                complexity_handling: 60,
            },
        };

        // Prompt 仓库
        let prompt_repo = {
            let pool = db_service.get_pool().map_err(|e| anyhow::anyhow!("DB pool error: {}", e))?;
            Some(PromptRepository::new(pool.clone()))
        };
        
        // 获取ReWOO框架适配器
        let framework_adapter = crate::tools::get_framework_adapter(crate::tools::FrameworkType::ReWOO).await
            .map_err(|e| anyhow::anyhow!("获取ReWOO框架适配器失败: {}", e))?;
        
        // 从AI服务管理器获取AI provider
        let ai_provider = Self::get_ai_provider_from_service_manager(&ai_service_manager).await?;
        
        // 创建核心组件
        let planner = match ReWOOPlanner::new_with_ai_service_manager(
            Arc::clone(&ai_provider),
            framework_adapter.clone(),
            config.planner.clone(),
            prompt_repo.clone(),
            ai_service_manager.clone(),
        ) {
            Ok(p) => Some(p),
            Err(e) => {
                log::warn!("Failed to create ReWOO planner: {}", e);
                None
            }
        };
        
        let worker = Some(ReWOOWorker::new(
            framework_adapter,
            config.worker.clone(),
        ));
        
        let solver = Some(ReWOOSolver::new(
            ai_provider,
            config.solver.clone(),
            prompt_repo.clone(),
        ));

        Ok(Self {
            engine_info,
            planner,
            worker,
            solver,
            config,
            sessions: HashMap::new(),
            ai_service_manager: Some(ai_service_manager),
            runtime_params: None,
            app_handle: None,
        })
    }

    pub fn set_runtime_params(&mut self, params: std::collections::HashMap<String, serde_json::Value>) {
        self.runtime_params = Some(params.clone());
        if let Some(planner) = &mut self.planner {
            planner.set_runtime_params(params.clone());
        }
        if let Some(solver) = &mut self.solver {
            solver.set_runtime_params(params);
        }
    }

    /// 设置应用句柄用于发送消息
    pub fn set_app_handle(&mut self, app_handle: tauri::AppHandle) {
        self.app_handle = Some(app_handle);
    }

    /// 发送错误消息到前端
    fn emit_error(&self, execution_id: &str, message_id: &str, conversation_id: Option<&str>, error_msg: &str) {
        if let Some(ref app_handle) = self.app_handle {
            let error_message = format!("ReWOO引擎执行失败: {}\n\n如需帮助，请检查执行配置或联系技术支持。", error_msg);
            emit_message_chunk(
                app_handle,
                execution_id,
                message_id,
                conversation_id,
                ChunkType::Error,
                &error_message,
                true,
                Some("rewoo"),
                None,
            );
        }
    }
    
    /// 从AI服务管理器获取AI provider
    async fn get_ai_provider_from_service_manager(
        ai_service_manager: &Arc<AiServiceManager>,
    ) -> Result<Arc<dyn AiProvider>> {
        use crate::ai_adapter::providers::ProviderFactory;
        use crate::ai_adapter::types::ProviderConfig;
        
        // 首先尝试获取默认配置
        let config = if let Some(config) = ai_service_manager.get_provider_config("default").await
            .map_err(|e| anyhow::anyhow!("Failed to get provider config: {}", e))? {
            config
        } else {
            // 尝试其他提供商
            let providers = vec!["openai", "anthropic", "gemini", "groq", "modelscope", "openrouter"];
            let mut found_config = None;
            
            for provider in providers {
                if let Ok(Some(config)) = ai_service_manager.get_provider_config(provider).await {
                    found_config = Some(config);
                    break;
                }
            }
            
            found_config.ok_or_else(|| anyhow::anyhow!("No AI provider configuration found"))?
        };
        
        // 创建提供商配置
        let provider_config = ProviderConfig {
            name: config.provider.clone(),
            api_key: config.api_key.unwrap_or_default(),
            api_base: config.api_base,
            api_version: None,
            timeout: None,
            max_retries: None,
            extra_headers: None,
        };
        
        // 创建provider实例
        let provider = ProviderFactory::create(provider_config)
            .map_err(|e| anyhow::anyhow!("Failed to create AI provider: {}", e))?;
        
        Ok(provider)
    }
}

#[async_trait]
impl ExecutionEngine for ReWooEngine {
    fn get_engine_info(&self) -> &EngineInfo {
        &self.engine_info
    }
    
    fn supports_task(&self, task: &AgentTask) -> bool {
        // ReWOO适合工具链明确、API调用密集的任务
        if task.description.to_lowercase().contains("api") ||
           task.description.to_lowercase().contains("tool") ||
           task.description.to_lowercase().contains("batch") ||
           task.description.to_lowercase().contains("retrieve") {
            return true;
        }
        
        // 检查是否有明确的工具需求
        if task.parameters.contains_key("tools") || task.parameters.contains_key("tool_chain") {
            return true;
        }
        
        false
    }
    
    async fn create_plan(&self, task: &AgentTask) -> Result<ExecutionPlan> {
        // 如果有planner组件，使用它来创建ReWOO计划
        if let Some(planner) = &self.planner {
            // 创建ReWOO状态
            let mut rewoo_state = ReWOOState {
                task: task.description.clone(),
                plan_string: String::new(),
                steps: Vec::new(),
                results: HashMap::new(),
                result: String::new(),
                execution_trace: None,
            };
            
            // 使用planner生成计划
            match planner.plan(&mut rewoo_state).await {
                Ok(_) => {
                    // 将ReWOO步骤转换为ExecutionPlan
                    let steps = rewoo_state.steps.iter().enumerate().map(|(i, step)| {
                        ExecutionStep {
                            id: format!("rewoo_step_{}", i),
                            name: format!("ReWOO Step {}: {}", i + 1, step),
                            description: step.clone(),
                            step_type: StepType::ToolCall,
                            dependencies: if i == 0 { vec![] } else { vec![format!("rewoo_step_{}", i - 1)] },
                            parameters: [("rewoo_step".to_string(), serde_json::Value::String(step.clone()))]
                                .into_iter().collect(),
                        }
                    }).collect();
                    
                    let plan = ExecutionPlan {
                        id: uuid::Uuid::new_v4().to_string(),
                        name: format!("ReWOO: {}", task.description),
                        steps,
                        estimated_duration: 180, // 3分钟
                        resource_requirements: ResourceRequirements {
                            cpu_cores: Some(4),
                            memory_mb: Some(1024),
                            network_concurrency: Some(20),
                            disk_space_mb: Some(50),
                        },
                    };
                    
                    return Ok(plan);
                }
                Err(e) => {
                    log::warn!("ReWOO planner failed, falling back to simple plan: {}", e);
                }
            }
        }
        
        // 回退到简单的ReWOO计划
        self.create_simple_rewoo_plan(task).await
    }
    
    async fn execute_plan(
        &self, 
        plan: &ExecutionPlan
    ) -> Result<AgentExecutionResult> {
        let _start_time = std::time::Instant::now();
        
    
        // 如果有完整的ReWOO组件，使用原有的执行逻辑
        if let (Some(_planner), Some(_worker), Some(_solver)) = (&self.planner, &self.worker, &self.solver) {
            match self.execute_rewoo_flow(plan).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    return Err(e.into());
                }
            }
        } else {
            return Err(anyhow::anyhow!("ReWooEngine execute_plan error"));
        }
    }
    
    async fn get_progress(&self, _session_id: &str) -> Result<ExecutionProgress> {
        // 模拟进度查询
        Ok(ExecutionProgress {
            total_steps: 3,
            completed_steps: 2,
            current_step: Some("Solve Final Answer".to_string()),
            progress_percentage: 66.7,
            estimated_remaining_seconds: Some(60),
        })
    }
}

impl ReWooEngine {
    /// 执行完整的ReWOO流程（使用原有组件）
    async fn execute_rewoo_flow(
        &self,
        _plan: &ExecutionPlan,
    ) -> Result<AgentExecutionResult> {
        let session_id = Uuid::new_v4().to_string();
        let start_time = std::time::Instant::now();
        
        // 创建ReWOO会话
        let mut rewoo_session = ReWOOSession::new(session_id.clone(), "ReWOO execution task".to_string());
        
        // 执行ReWOO状态图流程
        let result = self.execute_rewoo_state_graph(&mut rewoo_session).await;
        
        // 更新会话结束时间
        if result.is_ok() {
            rewoo_session.complete();
        } else {
            rewoo_session.fail(result.as_ref().err().unwrap().to_string());
        }
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        // 转换为AgentExecutionResult
        match result {
            Ok(final_answer) => {
                Ok(AgentExecutionResult {
                    id: Uuid::new_v4().to_string(),
                    success: true,
                    data: Some(serde_json::json!({
                        "engine": "rewoo_original",
                        "session_id": session_id,
                        "final_answer": final_answer,
                        "metrics": rewoo_session.metrics,
                        "execution_strategy": "original_rewoo_engine"
                    })),
                    error: None,
                    execution_time_ms: execution_time,
                    resources_used: [
                        ("cpu_usage".to_string(), 40.0),
                        ("memory_mb".to_string(), 512.0),
                        ("tool_calls".to_string(), rewoo_session.metrics.tool_calls as f64),
                        ("success_rate".to_string(), if rewoo_session.metrics.tool_calls > 0 {
                            (rewoo_session.metrics.successful_tool_calls as f64 / rewoo_session.metrics.tool_calls as f64) * 100.0
                        } else { 0.0 }),
                    ].into_iter().collect(),
                    artifacts: vec![
                        ExecutionArtifact {
                            artifact_type: ArtifactType::LogFile,
                            name: "rewoo_execution_result".to_string(),
                            data: serde_json::json!({
                                "session": rewoo_session,
                                "final_answer": final_answer
                            }),
                            created_at: chrono::Utc::now(),
                        }
                    ],
                })
            }
            Err(e) => Err(anyhow::anyhow!("ReWOO execution failed: {}", e))
        }
    }
    
    /// 执行ReWOO状态图流程
    async fn execute_rewoo_state_graph(
        &self,
        rewoo_session: &mut ReWOOSession,
    ) -> Result<String, ReWOOError> {
        let planner = self.planner.as_ref().unwrap();
        let worker = self.worker.as_ref().unwrap();
        let solver = self.solver.as_ref().unwrap();
        
        // 1. Planning 阶段
        let planning_start = SystemTime::now();
        planner.plan(&mut rewoo_session.state).await?;
        rewoo_session.metrics.planning_time_ms = planning_start.elapsed().unwrap_or_default().as_millis() as u64;
        rewoo_session.metrics.tool_calls = rewoo_session.state.steps.len() as u32;
        
        // 2. Working 阶段 - 执行所有步骤
        let working_start = SystemTime::now();
        self.execute_rewoo_working_phase(rewoo_session, worker, planner).await?;
        rewoo_session.metrics.working_time_ms = working_start.elapsed().unwrap_or_default().as_millis() as u64;
        
        // 3. Solving 阶段
        let solving_start = SystemTime::now();
        let answer = solver.solve(&rewoo_session.state).await?;
        rewoo_session.metrics.solving_time_ms = solving_start.elapsed().unwrap_or_default().as_millis() as u64;
        
        Ok(answer)
    }
    
    /// 执行ReWOO工作阶段 - 支持依赖分析和并行执行
    async fn execute_rewoo_working_phase(
        &self,
        rewoo_session: &mut ReWOOSession,
        worker: &ReWOOWorker,
        planner: &ReWOOPlanner,
    ) -> Result<(), ReWOOError> {
        // 如果启用并行执行，使用批次模式
        if self.config.worker.enable_parallel {
            return self.execute_rewoo_working_phase_batched(rewoo_session, worker, planner).await;
        }
        
        // 否则使用原始的串行执行
        while let Some(current_step) = planner.get_current_step(&rewoo_session.state) {
            // 解析当前步骤 - 使用带推理的版本
            let parsed_step = planner.parse_step_with_plan(&current_step, &rewoo_session.state.plan_string)?;
            
            // 验证步骤，如果失败则尝试纠错
            match worker.validate_step(&parsed_step).await {
                Ok(_) => {},
                Err(e) => {
                    log::warn!("Step validation failed for {}: {}", parsed_step.variable, e);
                    
                    // 生成纠错建议
                    match planner.generate_correction_suggestions(&parsed_step, &e.to_string()).await {
                        Ok(suggestions) => {
                            if let Some(best_suggestion) = suggestions.into_iter()
                                .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal)) {
                                
                                log::info!("Applying correction suggestion for {}: {}", 
                                    best_suggestion.step_variable, best_suggestion.reason);
                                
                                // 应用纠错建议
                                planner.apply_correction(&mut rewoo_session.state, &best_suggestion)?;
                                
                                // 重新获取当前步骤（因为状态可能已更改）
                                continue;
                            }
                        }
                        Err(correction_err) => {
                            log::warn!("Failed to generate correction suggestions: {}", correction_err);
                        }
                    }
                    
                    // 如果纠错失败，返回原始错误
                    return Err(ReWOOError::ToolExecutionError(e.to_string()));
                }
            }
            
            // 替换变量
            let substituted_args = planner.substitute_variables(
                &parsed_step.args,
                &rewoo_session.state.results,
            );
            
            // 执行工具调用
            let tool_result = if self.config.worker.max_retries > 0 {
                worker.execute_tool_with_retry(
                    &parsed_step,
                    &substituted_args,
                    self.config.worker.max_retries,
                ).await?
            } else {
                worker.execute_tool(&parsed_step, &substituted_args).await?
            };
            
            // 更新状态 - 优先使用结构化 JSON，回退到字符串
            if tool_result.success {
                let result_value = tool_result.json_content
                    .unwrap_or_else(|| serde_json::Value::String(tool_result.content.clone()));
                
                rewoo_session.state.results.insert(
                    parsed_step.variable.clone(),
                    result_value,
                );
                rewoo_session.metrics.successful_tool_calls += 1;
            } else {
                return Err(ReWOOError::ToolExecutionError(
                    format!("Step {} failed: {}", 
                        parsed_step.variable,
                        tool_result.error.unwrap_or_else(|| "Unknown error".to_string())
                    )
                ));
            }
        }
        
        Ok(())
    }
    
    /// 执行ReWOO工作阶段 - 批次并行模式
    async fn execute_rewoo_working_phase_batched(
        &self,
        rewoo_session: &mut ReWOOSession,
        worker: &ReWOOWorker,
        planner: &ReWOOPlanner,
    ) -> Result<(), ReWOOError> {
        // 创建执行批次
        let batches = planner.create_execution_batches(&rewoo_session.state);
        
        log::info!("Executing {} batches in parallel mode", batches.len());
        
        for batch in batches {
            log::info!("Executing batch {} with {} steps", batch.batch_id, batch.steps.len());
            
            // 准备批次中的所有步骤
            let mut batch_tasks = Vec::new();
            
            for step_variable in &batch.steps {
                // 找到对应的步骤字符串
                if let Some(step_str) = rewoo_session.state.steps.iter()
                    .find(|s| s.contains(step_variable)) {
                    
                    // 解析步骤
                    let parsed_step = planner.parse_step_with_plan(step_str, &rewoo_session.state.plan_string)?;
                    
                    // 验证步骤，如果失败则尝试纠错
                    match worker.validate_step(&parsed_step).await {
                        Ok(_) => {},
                        Err(e) => {
                            log::warn!("Step validation failed for {}: {}", parsed_step.variable, e);
                            
                            // 在并行模式下，如果纠错失败则跳过该步骤
                            if let Ok(suggestions) = planner.generate_correction_suggestions(&parsed_step, &e.to_string()).await {
                                if let Some(best_suggestion) = suggestions.into_iter()
                                    .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal)) {
                                    
                                    log::info!("Applying correction suggestion for {}: {}", 
                                        best_suggestion.step_variable, best_suggestion.reason);
                                    
                                    // 应用纠错建议到状态
                                    if planner.apply_correction(&mut rewoo_session.state, &best_suggestion).is_ok() {
                                        // 重新解析纠正后的步骤
                                        if let Some(corrected_step_str) = rewoo_session.state.steps.iter()
                                            .find(|s| s.contains(step_variable)) {
                                            if let Ok(corrected_parsed_step) = planner.parse_step_with_plan(corrected_step_str, &rewoo_session.state.plan_string) {
                                                // 使用纠正后的步骤
                                                let corrected_substituted_args = planner.substitute_variables(
                                                    &corrected_parsed_step.args,
                                                    &rewoo_session.state.results,
                                                );
                                                batch_tasks.push((corrected_parsed_step, corrected_substituted_args));
                                                continue;
                                            }
                                        }
                                    }
                                }
                            }
                            
                            log::warn!("Skipping step {} due to validation failure: {}", step_variable, e);
                            continue;
                        }
                    }
                    
                    // 替换变量
                    let substituted_args = planner.substitute_variables(
                        &parsed_step.args,
                        &rewoo_session.state.results,
                    );
                    
                    batch_tasks.push((parsed_step, substituted_args));
                }
            }
            
            // 并行执行当前批次
            let batch_results = worker.execute_steps_parallel(batch_tasks).await;
            
            // 处理批次结果
            for (i, result) in batch_results.into_iter().enumerate() {
                let step_variable = &batch.steps[i];
                match result {
                    Ok(tool_result) => {
                        if tool_result.success {
                            let result_value = tool_result.json_content
                                .unwrap_or_else(|| serde_json::Value::String(tool_result.content.clone()));
                            
                            rewoo_session.state.results.insert(
                                step_variable.clone(),
                                result_value,
                            );
                            rewoo_session.metrics.successful_tool_calls += 1;
                        } else {
                            return Err(ReWOOError::ToolExecutionError(
                                format!("Step {} failed: {}", 
                                    step_variable,
                                    tool_result.error.unwrap_or_else(|| "Unknown error".to_string())
                                )
                            ));
                        }
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
            
            log::info!("Batch {} completed successfully", batch.batch_id);
        }
        
        Ok(())
    }
    
    /// 创建简单的ReWOO计划（回退方法）
    async fn create_simple_rewoo_plan(&self, task: &AgentTask) -> Result<ExecutionPlan> {
        let steps = vec![
            ExecutionStep {
                id: "rewoo_plan".to_string(),
                name: "Create Reasoning Plan".to_string(),
                description: "Generate reasoning plan with tool calls and variable placeholders".to_string(),
                step_type: StepType::LlmCall,
                dependencies: vec![],
                parameters: [("reasoning_type".to_string(), serde_json::Value::String("rewoo".to_string()))]
                    .into_iter().collect(),
            },
            ExecutionStep {
                id: "rewoo_worker".to_string(),
                name: "Execute Tools".to_string(),
                description: "Execute all tool calls in parallel with variable substitution".to_string(),
                step_type: StepType::Parallel,
                dependencies: vec!["rewoo_plan".to_string()],
                parameters: [("execution_mode".to_string(), serde_json::Value::String("parallel".to_string()))]
                    .into_iter().collect(),
            },
            ExecutionStep {
                id: "rewoo_solver".to_string(),
                name: "Solve Final Answer".to_string(),
                description: "Generate final answer based on tool results".to_string(),
                step_type: StepType::LlmCall,
                dependencies: vec!["rewoo_worker".to_string()],
                parameters: [("solver_type".to_string(), serde_json::Value::String("rewoo_solver".to_string()))]
                    .into_iter().collect(),
            },
        ];

        let plan = ExecutionPlan {
            id: uuid::Uuid::new_v4().to_string(),
            name: format!("ReWOO: {}", task.description),
            steps,
            estimated_duration: 180, // 3分钟
            resource_requirements: ResourceRequirements {
                cpu_cores: Some(4),
                memory_mb: Some(1024),
                network_concurrency: Some(20),
                disk_space_mb: Some(50),
            },
        };

        Ok(plan)
    }
    
    
    /// 执行ReWOO风格的工具调用（使用全局工具系统）
    async fn execute_rewoo_tool_with_system(
        &self,
        tool_system: &Arc<crate::tools::ToolSystem>,
        tool_name: &str,
        target: &str,
        session: &mut dyn AgentSession,
    ) -> Result<serde_json::Value> {
        session.add_log(LogLevel::Info, format!("ReWOO executing tool: {} with target: {}", tool_name, target)).await?;
        
        // 准备工具执行参数
        let mut tool_inputs = HashMap::new();
        tool_inputs.insert("target".to_string(), serde_json::json!(target));
        tool_inputs.insert("ports".to_string(), serde_json::json!("common"));
        tool_inputs.insert("threads".to_string(), serde_json::json!(30)); // ReWOO特色：中等并发
        
        let execution_params = crate::tools::ToolExecutionParams {
            inputs: tool_inputs,
            context: HashMap::new(),
            timeout: Some(std::time::Duration::from_secs(180)), // ReWOO特色：较短超时
            execution_id: Some(Uuid::new_v4()),
        };
        
        // 执行工具
        match tool_system.execute_tool(tool_name, execution_params).await {
            Ok(result) => {
                session.add_log(LogLevel::Info, format!("ReWOO tool {} executed successfully", tool_name)).await?;
                Ok(if result.output.is_null() { 
                    serde_json::json!({"status": "completed"}) 
                } else { 
                    result.output 
                })
            }
            Err(e) => {
                session.add_log(LogLevel::Error, format!("ReWOO tool {} execution failed: {}", tool_name, e)).await?;
                Err(e)
            }
        }
    }
    
    /// 执行ReWOO任务（兼容旧接口）
    pub async fn execute(&mut self, task: &str) -> Result<String, ReWOOError> {
        let session_id = Uuid::new_v4().to_string();
        let _start_time = SystemTime::now();
        
        // 创建执行会话
        let mut session = ReWOOSession::new(session_id.clone(), task.to_string());
        
        // 如果有完整的ReWOO组件，执行完整流程
        if let (Some(planner), Some(worker), Some(solver)) = (&self.planner, &self.worker, &self.solver) {
            // 1. Planning 阶段
            let planning_start = SystemTime::now();
            planner.plan(&mut session.state).await?;
            session.metrics.planning_time_ms = planning_start.elapsed().unwrap_or_default().as_millis() as u64;
            session.metrics.tool_calls = session.state.steps.len() as u32;
            
            // 2. Working 阶段 - 执行所有步骤
            let working_start = SystemTime::now();
            self.execute_working_phase_simple(&mut session, worker, planner).await?;
            session.metrics.working_time_ms = working_start.elapsed().unwrap_or_default().as_millis() as u64;
            
            // 3. Solving 阶段
            let solving_start = SystemTime::now();
            let answer = solver.solve(&session.state).await?;
            session.metrics.solving_time_ms = solving_start.elapsed().unwrap_or_default().as_millis() as u64;
            
            // 更新会话结束时间
            session.complete();
            
            // 保存会话
            self.sessions.insert(session_id, session);
            
            Ok(answer)
        } else {
            // 简化执行
            session.complete();
            self.sessions.insert(session_id, session);
            Ok("ReWOO execution completed with fallback".to_string())
        }
    }
    
    /// 简化的工作阶段执行
    async fn execute_working_phase_simple(
        &self,
        rewoo_session: &mut ReWOOSession,
        worker: &ReWOOWorker,
        planner: &ReWOOPlanner,
    ) -> Result<(), ReWOOError> {
        while let Some(current_step) = planner.get_current_step(&rewoo_session.state) {
            // 解析当前步骤 - 使用带推理的版本
            let parsed_step = planner.parse_step_with_plan(&current_step, &rewoo_session.state.plan_string)?;
            
            // 验证步骤
            worker.validate_step(&parsed_step).await?;
            
            // 替换变量
            let substituted_args = planner.substitute_variables(
                &parsed_step.args,
                &rewoo_session.state.results,
            );
            
            // 执行工具调用
            let tool_result = if self.config.worker.max_retries > 0 {
                worker.execute_tool_with_retry(
                    &parsed_step,
                    &substituted_args,
                    self.config.worker.max_retries,
                ).await?
            } else {
                worker.execute_tool(&parsed_step, &substituted_args).await?
            };
            
            // 更新状态 - 优先使用结构化 JSON，回退到字符串
            if tool_result.success {
                let result_value = tool_result.json_content
                    .unwrap_or_else(|| serde_json::Value::String(tool_result.content.clone()));
                
                rewoo_session.state.results.insert(
                    parsed_step.variable.clone(),
                    result_value,
                );
                rewoo_session.metrics.successful_tool_calls += 1;
            } else {
                return Err(ReWOOError::ToolExecutionError(
                    format!("Step {} failed: {}", 
                        parsed_step.variable,
                        tool_result.error.unwrap_or_else(|| "Unknown error".to_string())
                    )
                ));
            }
        }
        
        Ok(())
    }
    
    /// 获取所有会话（兼容旧接口）
    pub fn get_all_sessions(&self) -> Vec<&ReWOOSession> {
        self.sessions.values().collect()
    }
    
    /// 获取执行会话（兼容旧接口）
    pub fn get_session(&self, session_id: &str) -> Option<&ReWOOSession> {
        self.sessions.get(session_id)
    }
    
    /// 获取引擎统计信息
    pub fn get_engine_stats(&self) -> EngineStats {
        let total_sessions = self.sessions.len();
        let completed_sessions = self.sessions.values()
            .filter(|s| s.completed_at.is_some())
            .count();
        
        let total_steps: u32 = self.sessions.values()
            .map(|s| s.metrics.tool_calls)
            .sum();
        
        let completed_steps: u32 = self.sessions.values()
            .map(|s| s.metrics.successful_tool_calls)
            .sum();
        
        let failed_steps: u32 = self.sessions.values()
            .map(|s| s.metrics.tool_calls - s.metrics.successful_tool_calls)
            .sum();
        
        let avg_execution_time = if completed_sessions > 0 {
            let total_time_ms: u64 = self.sessions.values()
                .filter(|s| s.completed_at.is_some())
                .map(|s| s.metrics.total_time_ms)
                .sum();
            let total_time = std::time::Duration::from_millis(total_time_ms);
            total_time / completed_sessions as u32
        } else {
            std::time::Duration::from_secs(0)
        };
        
        EngineStats {
            total_sessions,
            completed_sessions,
            total_steps,
            completed_steps,
            failed_steps,
            avg_execution_time,
            success_rate: if total_steps > 0 {
                (completed_steps as f32 / total_steps as f32) * 100.0
            } else {
                0.0
            },
        }
    }
}

/// 引擎统计信息
#[derive(Debug, Clone)]
pub struct EngineStats {
    /// 总会话数
    pub total_sessions: usize,
    /// 已完成会话数
    pub completed_sessions: usize,
    /// 总步骤数
    pub total_steps: u32,
    /// 已完成步骤数
    pub completed_steps: u32,
    /// 失败步骤数
    pub failed_steps: u32,
    /// 平均执行时间
    pub avg_execution_time: std::time::Duration,
    /// 成功率 (0-100)
    pub success_rate: f32,
}
