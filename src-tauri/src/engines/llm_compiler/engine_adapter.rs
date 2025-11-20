//! LLMCompiler引擎适配器 - 实现统一的ExecutionEngine接口

use super::executor::ParallelExecutorPool;
use super::joiner::IntelligentJoiner;
use super::planner::LlmCompilerPlanner;
use super::task_fetcher::TaskFetchingUnit;
use super::types::*;
use crate::agents::traits::*;
use crate::commands::agent_commands::WorkflowStepDetail;
use crate::models::prompt::{ArchitectureType, StageType};
use crate::services::ai::{AiService, AiServiceManager};
use crate::services::database::Database; // trait for DB methods
use crate::services::database::DatabaseService;
use crate::services::prompt_db::PromptRepository;
use crate::engines::memory::get_global_memory;
use crate::tools::ToolExecutionParams;
use crate::utils::ordered_message::{emit_message_chunk, ChunkType};
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// LLMCompiler引擎适配器 - 集成原有引擎逻辑
pub struct LlmCompilerEngine {
    engine_info: EngineInfo,
    // 核心组件
    planner: Option<Arc<LlmCompilerPlanner>>,
    task_fetcher: Option<Arc<TaskFetchingUnit>>,
    executor_pool: Option<Arc<ParallelExecutorPool>>,
    joiner: Option<Arc<tokio::sync::Mutex<IntelligentJoiner>>>,
    ai_service: Option<Arc<AiService>>,
    ai_service_manager: Option<Arc<AiServiceManager>>,  // ✅ 添加AI服务管理器，用于获取各阶段模型
    config: LlmCompilerConfig,
    prompt_repo: Option<PromptRepository>,
    runtime_params: Option<std::collections::HashMap<String, serde_json::Value>>, // prompt_ids 等
    app_handle: Option<tauri::AppHandle>,
    db_service: Option<Arc<DatabaseService>>,
}

impl LlmCompilerEngine {
    /// 创建新的引擎适配器
    pub async fn new() -> Result<Self> {
        let engine_info = EngineInfo {
            name: "LLMCompiler".to_string(),
            version: "1.0.0".to_string(),
            description: "DAG-based parallel execution architecture with intelligent task scheduling and dependency resolution".to_string(),
            supported_scenarios: vec![
                "Complex Multi-step Tasks".to_string(),
                "High Concurrency Processing".to_string(),
                "Large Scale Operations".to_string(),
                "Performance Critical Tasks".to_string(),
            ],
            performance_characteristics: PerformanceCharacteristics {
                token_efficiency: 90,
                execution_speed: 95,
                resource_usage: 80,
                concurrency_capability: 95,
                complexity_handling: 90,
            },
        };

        let config = LlmCompilerConfig::default();

        Ok(Self {
            engine_info,
            planner: None,
            task_fetcher: None,
            executor_pool: None,
            joiner: None,
            ai_service: None,
            ai_service_manager: None,
            config,
            prompt_repo: None,
            runtime_params: None,
            app_handle: None,
            db_service: None,
        })
    }

    /// 使用完整依赖创建引擎适配器
    pub async fn new_with_dependencies(
        ai_service_manager: Arc<AiServiceManager>,
        config: LlmCompilerConfig,
        db_service: Arc<DatabaseService>,
    ) -> Result<Self> {
        let engine_info = EngineInfo {
            name: "LLMCompiler".to_string(),
            version: "1.0.0".to_string(),
            description: "DAG-based parallel execution architecture with intelligent task scheduling and dependency resolution".to_string(),
            supported_scenarios: vec![
                "Complex Multi-step Tasks".to_string(),
                "High Concurrency Processing".to_string(),
                "Large Scale Operations".to_string(),
                "Performance Critical Tasks".to_string(),
            ],
            performance_characteristics: PerformanceCharacteristics {
                token_efficiency: 90,
                execution_speed: 95,
                resource_usage: 80,
                concurrency_capability: 95,
                complexity_handling: 90,
            },
        };

        // 初始化各个组件
        let pool = db_service
            .get_pool()
            .map_err(|e| anyhow::anyhow!("DB pool error: {}", e))?;
        let tool_adapter = crate::tools::get_global_engine_adapter()
            .map_err(|e| anyhow::anyhow!("获取全局工具适配器失败: {}", e))?;

        // ✅ 获取各阶段的AI服务
        // Planner阶段使用Planning模型
        let planner_service = Self::get_ai_service_for_stage(&ai_service_manager, crate::services::ai::SchedulerStage::Planning).await?;
        // Joiner阶段使用Evaluation模型
        let joiner_service = Self::get_ai_service_for_stage(&ai_service_manager, crate::services::ai::SchedulerStage::Evaluation).await?;
        // 默认AI服务作为后备
        let default_service = Self::get_ai_service_from_manager(&ai_service_manager)?;

        info!("LLMCompiler: Planner using model: {}/{}", planner_service.get_config().provider, planner_service.get_config().model);
        info!("LLMCompiler: Joiner using model: {}/{}", joiner_service.get_config().provider, joiner_service.get_config().model);

        let planner = Some(Arc::new(LlmCompilerPlanner::new(
            planner_service,
            tool_adapter.clone(),
            config.clone(),
            Some(PromptRepository::new(pool.clone())),
        )));

        let task_fetcher = Some(Arc::new(TaskFetchingUnit::new(config.clone())));

        let executor_pool = Some(Arc::new(ParallelExecutorPool::new(
            tool_adapter.clone(),
            config.clone(),
        )));

        let joiner = Some(Arc::new(tokio::sync::Mutex::new(IntelligentJoiner::new(
            (*joiner_service).clone(),
            config.clone(),
            Some(PromptRepository::new(pool.clone())),
        ))));

        Ok(Self {
            engine_info,
            planner,
            task_fetcher,
            executor_pool,
            joiner,
            ai_service: Some(default_service),
            ai_service_manager: Some(ai_service_manager),  // ✅ 保存AI服务管理器
            config,
            prompt_repo: Some(PromptRepository::new(pool.clone())),
            runtime_params: None,
            app_handle: None,
            db_service: Some(db_service),
        })
    }

    pub fn set_runtime_params(
        &mut self,
        params: std::collections::HashMap<String, serde_json::Value>,
    ) {
        self.runtime_params = Some(params.clone());
        // 透传到各组件（planner/joiner/executor）
        if let Some(_planner) = &self.planner {
            // Planner目前是Arc且没有内部可变性接口，这里通过任务执行时读取engine的runtime_params
            // 如果后续Planner增加set_runtime_params，则可在此处下发
        }
        if let Some(_joiner) = &self.joiner {
            // Joiner内部会在分析时从引擎读取上下文；如需可扩展传入
        }
        // 将参数传递给执行器池
        if let Some(executor_pool) = &self.executor_pool {
            let exec_clone = executor_pool.clone();
            let params_clone = params.clone();
            tokio::spawn(async move {
                exec_clone.set_runtime_params(params_clone).await;
            });
        }
    }

    /// 设置应用句柄用于发送消息
    pub fn set_app_handle(&mut self, app_handle: tauri::AppHandle) {
        self.app_handle = Some(app_handle);
    }

    /// 发送错误消息到前端
    fn emit_error(
        &self,
        execution_id: &str,
        message_id: &str,
        conversation_id: Option<&str>,
        error_msg: &str,
    ) {
        if let Some(ref app_handle) = self.app_handle {
            let error_message = format!(
                "LLMCompiler引擎执行失败: {}\n\n如需帮助，请检查执行配置或联系技术支持。",
                error_msg
            );
            emit_message_chunk(
                app_handle,
                execution_id,
                message_id,
                conversation_id,
                ChunkType::Error,
                &error_message,
                true,
                Some("llm_compiler"),
                None,
            );
        }
    }

    /// ✅ 根据调度器阶段获取对应的AI服务
    async fn get_ai_service_for_stage(
        ai_service_manager: &Arc<AiServiceManager>,
        stage: crate::services::ai::SchedulerStage,
    ) -> Result<Arc<AiService>> {
        // 首先尝试从调度器配置获取该阶段的模型
        match ai_service_manager.get_ai_config_for_stage(stage.clone()).await {
            Ok(Some(config)) => {
                info!(
                    "LLMCompiler: Using scheduler config for stage {:?} -> provider: {}, model: {}",
                    stage, config.provider, config.model
                );
                
                // 根据provider和model创建AI服务key
                // 格式：provider::model
                let service_key = format!("{}::{}", config.provider, config.model);
                if let Some(service) = ai_service_manager.get_service(&service_key) {
                    return Ok(Arc::new(service));
                }
                
                // 如果没有找到，尝试只用provider作为key（精确匹配）
                if let Some(service) = ai_service_manager.get_service(&config.provider) {
                    return Ok(Arc::new(service));
                }
                
                // ✅ 尝试大小写不敏感匹配（遍历所有可用服务）
                let provider_lower = config.provider.to_lowercase();
                let all_services = ai_service_manager.list_services();
                for service_name in &all_services {
                    if service_name.to_lowercase() == provider_lower {
                        if let Some(service) = ai_service_manager.get_service(service_name) {
                            info!(
                                "LLMCompiler: Found service '{}' via case-insensitive match for provider '{}'",
                                service_name, config.provider
                            );
                            return Ok(Arc::new(service));
                        }
                    }
                }
                
                warn!(
                    "LLMCompiler: No service found for stage {:?} (provider: {}, model: {}), falling back to default",
                    stage, config.provider, config.model
                );
            }
            Ok(None) => {
                info!("LLMCompiler: No scheduler config for stage {:?}, using default service", stage);
            }
            Err(e) => {
                warn!(
                    "LLMCompiler: Failed to get scheduler config for stage {:?}: {}, using default service",
                    stage, e
                );
            }
        }
        
        // 如果没有配置或获取失败，回退到默认服务
        Self::get_ai_service_from_manager(ai_service_manager)
    }

    /// 从AI服务管理器获取默认AI服务
    fn get_ai_service_from_manager(
        ai_service_manager: &Arc<AiServiceManager>,
    ) -> Result<Arc<AiService>> {
        // 列出所有可用服务用于调试
        let all_services = ai_service_manager.list_services();
        info!("LLMCompiler: Available AI services: {:?}", all_services);
        
        // 首先尝试获取"default"服务
        if let Some(service) = ai_service_manager.get_service("default") {
            let config = service.get_config();
            info!(
                "LLMCompiler: Using 'default' AI service -> provider: {}, model: {}",
                config.provider, config.model
            );
            return Ok(Arc::new(service));
        }

        // 如果没有default，尝试按优先级获取
        let preferred_providers = vec![
            "deepseek",
            "openai",
            "anthropic",
            "gemini",
            "groq",
            "modelscope",
            "openrouter",
        ];
        for provider in &preferred_providers {
            if let Some(service) = ai_service_manager.get_service(provider) {
                let config = service.get_config();
                info!(
                    "LLMCompiler: Using preferred provider '{}' -> model: {}",
                    config.provider, config.model
                );
                return Ok(Arc::new(service));
            }
        }

        // 获取第一个可用的服务
        let services = ai_service_manager.list_services();
        if let Some(first_service_name) = services.first() {
            if let Some(service) = ai_service_manager.get_service(first_service_name) {
                let config = service.get_config();
                warn!(
                    "LLMCompiler: No preferred provider found, using first available service '{}' -> provider: {}, model: {}",
                    first_service_name, config.provider, config.model
                );
                warn!("Recommended: Configure a preferred AI provider in Settings > AI Configuration");
                return Ok(Arc::new(service));
            }
        }

        error!("LLMCompiler: No AI service available. Please configure at least one AI provider.");
        Err(anyhow::anyhow!(
            "No AI service available in AiServiceManager. Please configure an AI provider in Settings."
        ))
    }
}

#[async_trait]
impl ExecutionEngine for LlmCompilerEngine {
    fn get_engine_info(&self) -> &EngineInfo {
        &self.engine_info
    }

    fn supports_task(&self, task: &AgentTask) -> bool {
        // LLMCompiler适合复杂多步骤、高并发需求的任务
        if task.description.to_lowercase().contains("complex")
            || task.description.to_lowercase().contains("parallel")
            || task.description.to_lowercase().contains("concurrent")
            || task.description.to_lowercase().contains("large")
            || task.description.to_lowercase().contains("performance")
        {
            return true;
        }

        // 检查优先级是否为高或紧急
        matches!(task.priority, TaskPriority::High | TaskPriority::Critical)
    }

    async fn create_plan(&self, task: &AgentTask) -> Result<ExecutionPlan> {
        // 创建LLMCompiler风格的DAG执行计划
        let steps = vec![
            ExecutionStep {
                id: "dag_planner".to_string(),
                name: "Create Task DAG".to_string(),
                description:
                    "Generate DAG with task dependencies and parallel execution opportunities"
                        .to_string(),
                step_type: StepType::LlmCall,
                dependencies: vec![],
                parameters: [(
                    "dag_mode".to_string(),
                    serde_json::Value::String("parallel".to_string()),
                )]
                .into_iter()
                .collect(),
            },
            ExecutionStep {
                id: "task_fetcher".to_string(),
                name: "Fetch Ready Tasks".to_string(),
                description: "Identify and fetch tasks ready for execution".to_string(),
                step_type: StepType::DataProcessing,
                dependencies: vec!["dag_planner".to_string()],
                parameters: HashMap::new(),
            },
            ExecutionStep {
                id: "parallel_executor".to_string(),
                name: "Execute Tasks in Parallel".to_string(),
                description: "Execute multiple tasks concurrently based on DAG".to_string(),
                step_type: StepType::Parallel,
                dependencies: vec!["task_fetcher".to_string()],
                parameters: [(
                    "max_concurrent".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(10)),
                )]
                .into_iter()
                .collect(),
            },
            ExecutionStep {
                id: "dependency_resolver".to_string(),
                name: "Resolve Dependencies".to_string(),
                description: "Manage task dependencies and execution flow".to_string(),
                step_type: StepType::DataProcessing,
                dependencies: vec!["parallel_executor".to_string()],
                parameters: HashMap::new(),
            },
            ExecutionStep {
                id: "joiner".to_string(),
                name: "Join Results".to_string(),
                description: "Aggregate and join all task results".to_string(),
                step_type: StepType::LlmCall,
                dependencies: vec!["dependency_resolver".to_string()],
                parameters: [(
                    "join_strategy".to_string(),
                    serde_json::Value::String("intelligent".to_string()),
                )]
                .into_iter()
                .collect(),
            },
        ];

        let plan = ExecutionPlan {
            id: uuid::Uuid::new_v4().to_string(),
            name: format!("LLMCompiler: {}", task.description),
            steps,
            estimated_duration: 120, // 2分钟（并行执行）
            resource_requirements: ResourceRequirements {
                cpu_cores: Some(8),
                memory_mb: Some(2048),
                network_concurrency: Some(50),
                disk_space_mb: Some(200),
            },
        };

        Ok(plan)
    }

    async fn execute_plan(&self, plan: &ExecutionPlan) -> Result<AgentExecutionResult> {
        let _start_time = std::time::Instant::now();

        // 如果有完整的LLMCompiler组件，使用原有的工作流执行逻辑
        if let (
            Some(_planner),
            Some(_task_fetcher),
            Some(_executor_pool),
            Some(_joiner),
            Some(_ai_service),
        ) = (
            &self.planner,
            &self.task_fetcher,
            &self.executor_pool,
            &self.joiner,
            &self.ai_service,
        ) {
            match self.execute_llm_compiler_workflow(plan).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    return Err(e.into());
                }
            }
        } else {
            return Err(anyhow::anyhow!("LLMCompilerEngine execute_plan error"));
        }
    }

    async fn get_progress(&self, _session_id: &str) -> Result<ExecutionProgress> {
        // 模拟进度查询
        Ok(ExecutionProgress {
            total_steps: 5,
            completed_steps: 4,
            current_step: Some("Join Results".to_string()),
            progress_percentage: 80.0,
            estimated_remaining_seconds: Some(30),
        })
    }

    async fn cancel_execution(&self, session_id: &str) -> Result<()> {
        log::info!(
            "Cancelling LLMCompiler execution for session: {}",
            session_id
        );
        // 直接实现取消逻辑
        if let Some(task_fetcher) = &self.task_fetcher {
            task_fetcher.cancel_pending_tasks().await?;
        }
        Ok(())
    }
}

impl LlmCompilerEngine {
    /// 执行原有的LLMCompiler工作流逻辑
    async fn execute_llm_compiler_workflow(
        &self,
        _plan: &ExecutionPlan,
    ) -> Result<AgentExecutionResult> {
        let workflow_start = std::time::Instant::now();

        // ✅ 从runtime_params中获取用户的实际任务描述和前端消息ID
        let context = self.runtime_params.clone().unwrap_or_default();
        let user_query = context
            .get("task_description")
            .and_then(|v| v.as_str())
            .unwrap_or("LLMCompiler workflow execution");

        // ✅ 提取conversation_id、message_id、execution_id（用于统一消息推送）
        let conversation_id = context
            .get("conversation_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let message_id = context
            .get("message_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let execution_id = context
            .get("execution_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        info!("开始执行LLMCompiler工作流: {}", user_query);
        info!("LLMCompiler: conversation_id={:?}, message_id={:?}, execution_id={:?}", 
            conversation_id, message_id, execution_id);

        let mut execution_summary = ExecutionSummary {
            total_tasks: 0,
            successful_tasks: 0,
            failed_tasks: 0,
            total_duration_ms: 0,
            replanning_count: 0,
            key_findings: Vec::new(),
            efficiency_metrics: EfficiencyMetrics {
                average_parallelism: 0.0,
                resource_utilization: 0.0,
                task_success_rate: 0.0,
                average_task_duration_ms: 0.0,
            },
        };

        let mut all_results: Vec<TaskExecutionResult> = Vec::new();

        // 主执行循环
        for round in 1..=self.config.max_iterations {
            info!("开始执行轮次: {}/{}", round, self.config.max_iterations);

            if round > 1 {
                execution_summary.replanning_count = round - 1;
            }

            // 1. 规划阶段
            if let Some(planner) = &self.planner {
                info!("开始生成DAG执行计划...");
                
                // 发送Planning阶段开始消息
                if let Some(app) = &self.app_handle {
                    crate::utils::ordered_message::emit_thinking_chunk(
                        app,
                        execution_id.as_deref().unwrap_or("unknown"),
                        message_id.as_deref().unwrap_or("unknown"),
                        conversation_id.as_deref(),
                        "开始生成DAG执行计划...",
                        Some("llm_compiler_planning"),
                    );
                }
                
                let execution_plan = match planner.generate_dag_plan(user_query, &context).await {
                    Ok(plan) => {
                        info!("✓ DAG规划成功: {} 个任务节点", plan.nodes.len());
                        
                        // 发送计划信息到前端
                        if let Some(app) = &self.app_handle {
                            let plan_info = format!(
                                "DAG规划成功，共{}个任务节点",
                                plan.nodes.len()
                            );
                            crate::utils::ordered_message::emit_plan_info_chunk(
                                app,
                                execution_id.as_deref().unwrap_or("unknown"),
                                message_id.as_deref().unwrap_or("unknown"),
                                conversation_id.as_deref(),
                                &plan_info,
                                Some("llm_compiler_planning"),
                                None,
                            );
                        }
                        
                        plan
                    }
                    Err(e) => {
                        error!("✗ DAG规划失败: {}", e);
                        
                        // 发送错误消息到前端
                        if let Some(app) = &self.app_handle {
                            let error_msg = format!("DAG规划失败: {}", e);
                            crate::utils::ordered_message::emit_error_chunk(
                                app,
                                execution_id.as_deref().unwrap_or("unknown"),
                                message_id.as_deref().unwrap_or("unknown"),
                                conversation_id.as_deref(),
                                &error_msg,
                                Some("llm_compiler_planning"),
                                None,
                            );
                        }
                        
                        return Err(anyhow::anyhow!(
                            "LLMCompiler planning phase failed: {}. This may be due to LLM service configuration issues or network problems.",
                            e
                        ));
                    }
                };

                // 2. 任务调度阶段
                if let Some(task_fetcher) = &self.task_fetcher {
                    task_fetcher.initialize_plan(&execution_plan).await?;

                    // 3. 并行执行阶段
                    let round_results = self.execute_parallel_round().await?;

                    if round_results.is_empty() {
                        warn!("轮次 {} 没有执行任何任务", round);
                        break;
                    }

                    // 更新统计信息
                    let completed_count = round_results
                        .iter()
                        .filter(|r| r.status == TaskStatus::Completed)
                        .count();
                    let failed_count = round_results
                        .iter()
                        .filter(|r| r.status == TaskStatus::Failed)
                        .count();

                    execution_summary.successful_tasks += completed_count;
                    execution_summary.failed_tasks += failed_count;
                    execution_summary.total_tasks += round_results.len();

                    let round_duration: u64 = round_results.iter().map(|r| r.duration_ms).sum();
                    execution_summary.total_duration_ms += round_duration;

                    all_results.extend(round_results.clone());

                    info!(
                        "轮次 {} 完成: 成功 {} 个任务, 失败 {} 个任务, 耗时 {}ms",
                        round, completed_count, failed_count, round_duration
                    );

                // 4. 智能决策阶段
                if let Some(joiner) = &self.joiner {
                    info!("开始 Joiner 智能决策阶段...");
                    
                    // 发送Joiner阶段开始消息
                    if let Some(app) = &self.app_handle {
                        crate::utils::ordered_message::emit_thinking_chunk(
                            app,
                            execution_id.as_deref().unwrap_or("unknown"),
                            message_id.as_deref().unwrap_or("unknown"),
                            conversation_id.as_deref(),
                            "开始智能决策分析...",
                            Some("llm_compiler_joiner"),
                        );
                    }
                    
                    let mut joiner_guard = joiner.lock().await;
                    // ✅ 设置消息ID，确保Joiner的AI调用使用正确的ID
                    joiner_guard.set_message_ids(conversation_id.clone(), message_id.clone());
                    let decision = match joiner_guard
                        .analyze_and_decide(user_query, &execution_plan, &round_results, round)
                        .await
                    {
                        Ok(d) => {
                            debug!("Joiner 决策成功");
                            d
                        }
                        Err(e) => {
                            warn!("Joiner 决策失败: {}", e);
                            
                            // 发送错误消息到前端
                            if let Some(app) = &self.app_handle {
                                let error_msg = format!("Joiner决策失败: {}", e);
                                crate::utils::ordered_message::emit_error_chunk(
                                    app,
                                    execution_id.as_deref().unwrap_or("unknown"),
                                    message_id.as_deref().unwrap_or("unknown"),
                                    conversation_id.as_deref(),
                                    &error_msg,
                                    Some("llm_compiler_joiner"),
                                    None,
                                );
                            }
                            
                            // Joiner 失败不应导致整个流程中断，记录警告并继续
                            warn!("Joiner decision failed, continuing with available results");
                            JoinerDecision::Complete {
                                response: format!("Task execution completed with {} successful and {} failed tasks", 
                                    execution_summary.successful_tasks, 
                                    execution_summary.failed_tasks),
                                confidence: 0.7,
                                summary: execution_summary.clone(),
                            }
                        }
                    };

                    match &decision {
                        JoinerDecision::Complete { response, .. } => {
                            execution_summary
                                .key_findings
                                .push(format!("决策: 完成执行 - {}", response));
                            info!("Joiner决定完成执行: {}", response);
                            
                            // 发送决策结果到前端
                            if let Some(app) = &self.app_handle {
                                let decision_msg = format!("✓ 决策: 完成执行\n{}", response);
                                crate::utils::ordered_message::emit_meta_chunk(
                                    app,
                                    execution_id.as_deref().unwrap_or("unknown"),
                                    message_id.as_deref().unwrap_or("unknown"),
                                    conversation_id.as_deref(),
                                    &decision_msg,
                                    None,
                                );
                            }
                            
                            break;
                        }
                        JoinerDecision::Continue { feedback, .. } => {
                            execution_summary
                                .key_findings
                                .push(format!("决策: 继续执行 - {}", feedback));
                            info!("Joiner决定继续执行: {}", feedback);
                            
                            // 发送决策结果到前端
                            if let Some(app) = &self.app_handle {
                                let decision_msg = format!("→ 决策: 继续执行\n{}", feedback);
                                crate::utils::ordered_message::emit_meta_chunk(
                                    app,
                                    execution_id.as_deref().unwrap_or("unknown"),
                                    message_id.as_deref().unwrap_or("unknown"),
                                    conversation_id.as_deref(),
                                    &decision_msg,
                                    None,
                                );
                            }
                        }
                    }
                    }

                    // 检查是否还有待执行的任务
                    if !task_fetcher.has_pending_tasks().await {
                        info!("所有任务已完成，结束执行");
                        break;
                    }
                }
            }
        }

        let total_duration = workflow_start.elapsed();
        execution_summary.total_duration_ms = total_duration.as_millis() as u64;

        // 计算最终效率指标
        execution_summary.efficiency_metrics =
            self.calculate_efficiency_metrics(&execution_summary);

        info!(
            "工作流执行完成: {} 个任务, 成功 {}, 失败 {}, 耗时 {}ms",
            execution_summary.total_tasks,
            execution_summary.successful_tasks,
            execution_summary.failed_tasks,
            execution_summary.total_duration_ms
        );

        // 生成最终结果（传递正确的conversation_id和message_id）
        let final_response = self
            .generate_final_response(
                user_query, 
                &all_results, 
                &execution_summary,
                conversation_id.as_deref(),
                message_id.as_deref()
            )
            .await?;

        // 转换为AgentExecutionResult
        Ok(AgentExecutionResult {
            id: uuid::Uuid::new_v4().to_string(),
            success: true,
            data: Some(serde_json::json!({
                "engine": "llm_compiler",
                "response": final_response,
                "execution_summary": execution_summary,
                "task_results": all_results,
                "efficiency_metrics": execution_summary.efficiency_metrics
            })),
            error: None,
            execution_time_ms: execution_summary.total_duration_ms,
            resources_used: [
                (
                    "total_tasks".to_string(),
                    execution_summary.total_tasks as f64,
                ),
                (
                    "successful_tasks".to_string(),
                    execution_summary.successful_tasks as f64,
                ),
                (
                    "failed_tasks".to_string(),
                    execution_summary.failed_tasks as f64,
                ),
                (
                    "parallelism".to_string(),
                    execution_summary.efficiency_metrics.average_parallelism as f64,
                ),
            ]
            .into_iter()
            .collect(),
            artifacts: vec![ExecutionArtifact {
                artifact_type: ArtifactType::AnalysisResult,
                name: "llm_compiler_workflow_result".to_string(),
                data: serde_json::json!({
                    "execution_summary": execution_summary,
                    "task_results": all_results,
                    "final_response": final_response
                }),
                created_at: chrono::Utc::now(),
            }],
        })
    }

    /// 执行一轮并行任务
    async fn execute_parallel_round(&self) -> Result<Vec<TaskExecutionResult>> {
        let mut results = Vec::new();

        if let (Some(task_fetcher), Some(executor_pool)) = (&self.task_fetcher, &self.executor_pool)
        {
            // 获取所有就绪的任务
            let ready_tasks = task_fetcher
                .fetch_ready_tasks(self.config.max_concurrency)
                .await;

            if ready_tasks.is_empty() {
                return Ok(results);
            }

            info!("开始并行执行 {} 个就绪任务", ready_tasks.len());

            // 标记任务为执行中并启动执行
            let mut handles = Vec::new();
            for task in ready_tasks {
                let task_id = task.id.clone();
                let executor = executor_pool.clone();
                let handle = tokio::spawn(async move { executor.execute_task(task).await });

                // 标记任务开始执行
                task_fetcher
                    .mark_task_executing(task_id, handle.abort_handle())
                    .await;
                handles.push(handle);
            }

            // 等待所有任务完成
            for handle in handles {
                match handle.await {
                    Ok(result) => {
                        // ✅ 发送工具执行结果到前端（参考React架构）
                        if let Some(app) = &self.app_handle {
                            if let Some(params) = &self.runtime_params {
                                let conversation_id = params.get("conversation_id").and_then(|v| v.as_str());
                                let message_id = params.get("message_id").and_then(|v| v.as_str());
                                let execution_id = params.get("execution_id").and_then(|v| v.as_str());
                                
                                // 构造工具结果内容
                                let tool_result_content = if result.status == TaskStatus::Completed {
                                    serde_json::to_string(&result.outputs).unwrap_or_default()
                                } else {
                                    serde_json::json!({
                                        "error": result.error.clone().unwrap_or_else(|| "Unknown error".to_string()),
                                        "success": false,
                                        "task_id": result.task_id
                                    }).to_string()
                                };
                                
                                emit_message_chunk(
                                    app,
                                    execution_id.unwrap_or(&result.task_id),
                                    message_id.unwrap_or(&result.task_id),
                                    conversation_id,
                                    ChunkType::ToolResult,
                                    &tool_result_content,
                                    false,
                                    Some("llm_compiler"),
                                    None,  // 工具名称可以从result中提取，但这里暂时为None
                                );
                                
                                info!("📤 LLMCompiler: Tool result sent to frontend: task={}, status={:?}", 
                                    result.task_id, result.status);
                            }
                        }
                        
                        // 更新任务状态
                        task_fetcher.complete_task(result.clone()).await?;
                        // 落库单个任务作为步骤
                        if let (Some(db), Some(params)) = (&self.db_service, &self.runtime_params) {
                            if let Some(session_id) =
                                params.get("execution_id").and_then(|v| v.as_str())
                            {
                                let step_name = result.task_id.clone();
                                let status = match result.status {
                                    TaskStatus::Completed => "Completed",
                                    TaskStatus::Failed => "Failed",
                                    _ => "Running",
                                };
                                let _ = db
                                    .save_agent_execution_step(
                                        session_id,
                                        &WorkflowStepDetail {
                                            step_id: format!("step_{}", results.len() + 1),
                                            step_name,
                                            status: status.to_string(),
                                            started_at: None,
                                            completed_at: None,
                                            duration_ms: result.duration_ms,
                                            result_data: Some(serde_json::json!(result.outputs)),
                                            error: result.error.clone(),
                                            retry_count: 0,
                                            dependencies: vec![],
                                            tool_result: None,
                                        },
                                    )
                                    .await;
                            }
                        }
                        results.push(result);
                    }
                    Err(e) => {
                        error!("任务执行句柄错误: {}", e);
                    }
                }
            }
        }

        Ok(results)
    }

    /// 生成最终响应
    async fn generate_final_response(
        &self,
        user_query: &str,
        task_results: &[TaskExecutionResult],
        execution_summary: &ExecutionSummary,
        conversation_id: Option<&str>,
        message_id: Option<&str>,
    ) -> Result<String> {
        // 收集所有成功任务的输出
        let successful_outputs: Vec<&std::collections::HashMap<String, serde_json::Value>> =
            task_results
                .iter()
                .filter(|r| r.status == TaskStatus::Completed)
                .map(|r| &r.outputs)
                .collect();

        if successful_outputs.is_empty() {
            return Ok("抱歉，没有成功执行任何任务，无法提供有效结果。".to_string());
        }

        // 构建响应生成提示
        let mut response_prompt = if let Some(repo) = &self.prompt_repo {
            // 优先按 prompt_ids.executor 覆盖
            if let Some(rp) = &self.runtime_params {
                if let Some(pid) = rp
                    .get("prompt_ids")
                    .and_then(|v| v.get("executor"))
                    .and_then(|v| v.as_i64())
                {
                    if let Ok(Some(dynamic)) = repo.get_template(pid).await {
                        dynamic.content
                    } else if let Ok(Some(dynamic)) = repo
                        .get_active_prompt(ArchitectureType::LLMCompiler, StageType::Execution)
                        .await
                    {
                        dynamic
                    } else {
                        self.build_default_response_prompt(
                            user_query,
                            &successful_outputs,
                            execution_summary,
                        )
                    }
                } else if let Ok(Some(dynamic)) = repo
                    .get_active_prompt(ArchitectureType::LLMCompiler, StageType::Execution)
                    .await
                {
                    dynamic
                } else {
                    self.build_default_response_prompt(
                        user_query,
                        &successful_outputs,
                        execution_summary,
                    )
                }
            } else if let Ok(Some(dynamic)) = repo
                .get_active_prompt(ArchitectureType::LLMCompiler, StageType::Execution)
                .await
            {
                dynamic
            } else {
                self.build_default_response_prompt(
                    user_query,
                    &successful_outputs,
                    execution_summary,
                )
            }
        } else {
            self.build_default_response_prompt(user_query, &successful_outputs, execution_summary)
        };

        // 集成角色提示词（如果存在）
        if let Some(rp) = &self.runtime_params {
            if let Some(role_prompt) = rp.get("role_prompt").and_then(|v| v.as_str()) {
                if !role_prompt.trim().is_empty() {
                    response_prompt = if response_prompt.trim().is_empty() {
                        role_prompt.to_string()
                    } else {
                        format!("{}\n\n{}", role_prompt, response_prompt)
                    };
                    log::info!("LLM-Compiler joiner: integrated role prompt for final response");
                }
            }
        }

        // RAG augmentation for final response prompt
        log::debug!("RAG augmentation skipped in llm_compiler engine_adapter");

        // ✅ 调用AI生成最终响应，使用Execution阶段的模型
        let execution_service = if let Some(ai_service_manager) = &self.ai_service_manager {
            match Self::get_ai_service_for_stage(ai_service_manager, crate::services::ai::SchedulerStage::Execution).await {
                Ok(service) => {
                    info!("LLMCompiler: Final response using Execution stage model: {}/{}", 
                        service.get_config().provider, service.get_config().model);
                    Some(service)
                }
                Err(e) => {
                    warn!("LLMCompiler: Failed to get Execution stage service: {}, using default", e);
                    self.ai_service.clone()
                }
            }
        } else {
            self.ai_service.clone()
        };

        if let Some(ai_service) = execution_service {
            info!(
                "开始调用AI生成最终响应，提示词长度: {} 字符",
                response_prompt.len()
            );
            debug!("AI响应生成提示词: {}", response_prompt);

            // 构建最终回答的 user 提示（仅包含业务上下文，不包含系统说明）
            let outputs_block = successful_outputs
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    let pretty = serde_json::to_string_pretty(m).unwrap_or_else(|_| "{}".to_string());
                    format!("Result {}:\n{}", i + 1, pretty)
                })
                .collect::<Vec<_>>()
                .join("\n\n");

            let final_user_prompt = format!(
                "Please generate a clear, friendly final answer for the user based on the results below.\n\nUser query:\n{}\n\nExecution summary:\n- total_tasks: {}\n- successful_tasks: {}\n- failed_tasks: {}\n- total_duration_ms: {}\n\nOutputs:\n{}\n",
                user_query,
                execution_summary.total_tasks,
                execution_summary.successful_tasks,
                execution_summary.failed_tasks,
                execution_summary.total_duration_ms,
                outputs_block
            );

            match ai_service
                .send_message_stream_with_save_control(
                    Some(&final_user_prompt),        // 发送给LLM的用户消息
                    None,                             // 不重复保存用户消息
                    Some(&response_prompt),           // 作为system prompt使用（来自DB模板或默认）
                    conversation_id.map(|s| s.to_string()),
                    message_id.map(|s| s.to_string()),
                    true,
                    false,
                    Some(ChunkType::Content),
                )
                .await
            {
                Ok(ai_response) => {
                    if ai_response.trim().is_empty() {
                        warn!("AI返回了空响应，使用默认响应");
                        Ok(self.generate_default_response(task_results, execution_summary))
                    } else {
                        info!("AI响应生成成功，响应长度: {} 字符", ai_response.len());
                        Ok(ai_response)
                    }
                }
                Err(e) => {
                    error!("AI响应生成失败: {}, 使用默认响应", e);
                    Ok(self.generate_default_response(task_results, execution_summary))
                }
            }
        } else {
            Ok(self.generate_default_response(task_results, execution_summary))
        }
    }

    /// 执行DAG中的单个工具（静态方法，用于并发执行）
    async fn execute_dag_tool_static(
        tool_system: &Arc<crate::tools::ToolSystem>,
        tool_name: &str,
        target: &str,
    ) -> Result<serde_json::Value> {
        // 准备工具执行参数
        let mut tool_inputs = HashMap::new();
        tool_inputs.insert("target".to_string(), serde_json::json!(target));

        // 根据工具类型添加特定参数
        match tool_name {
            "port_scan" => {
                tool_inputs.insert("ports".to_string(), serde_json::json!("common"));
                tool_inputs.insert("threads".to_string(), serde_json::json!(100));
                // LLMCompiler使用更高并发
            }
            "rsubdomain" => {
                tool_inputs.insert("wordlist".to_string(), serde_json::json!("common"));
            }
            _ => {}
        }

        let execution_params = ToolExecutionParams {
            inputs: tool_inputs,
            context: HashMap::new(),
            timeout: Some(std::time::Duration::from_secs(120)), // LLMCompiler使用较短超时
            execution_id: Some(Uuid::new_v4()),
        };

        // 执行工具
        match tool_system.execute_tool(tool_name, execution_params).await {
            Ok(result) => Ok(if result.output.is_null() {
                serde_json::json!({"status": "completed"})
            } else {
                result.output
            }),
            Err(e) => Err(e),
        }
    }

    // 其他辅助方法
    fn build_default_response_prompt(
        &self,
        user_query: &str,
        successful_outputs: &[&std::collections::HashMap<String, serde_json::Value>],
        execution_summary: &ExecutionSummary,
    ) -> String {
        format!(
            r#"你是一个专业的安全分析专家，请基于以下LLMCompiler工作流执行结果，为用户查询生成一个完整、准确、专业的分析报告。

**用户查询**: {}

**执行统计**:
- 总任务数: {}
- 成功任务: {}
- 失败任务: {}
- 总耗时: {}ms
- 任务成功率: {:.1}%

**执行结果详情**:
{}

{}"#,
            user_query,
            execution_summary.total_tasks,
            execution_summary.successful_tasks,
            execution_summary.failed_tasks,
            execution_summary.total_duration_ms,
            if execution_summary.total_tasks > 0 {
                (execution_summary.successful_tasks as f32 / execution_summary.total_tasks as f32)
                    * 100.0
            } else {
                0.0
            },
            self.format_outputs_for_response(&successful_outputs),
            if execution_summary.failed_tasks > 0 {
                format!("\n## ❌ 执行问题\n本次执行中有 {} 个任务失败，请在报告中说明可能的原因和影响。", execution_summary.failed_tasks)
            } else {
                String::new()
            }
        )
    }

    fn format_outputs_for_response(
        &self,
        outputs: &[&std::collections::HashMap<String, serde_json::Value>],
    ) -> String {
        if outputs.is_empty() {
            return "暂无成功执行的任务输出".to_string();
        }

        outputs
            .iter()
            .enumerate()
            .map(|(i, output)| {
                let mut formatted_output = Vec::new();

                // 格式化每个输出字段
                for (key, value) in output.iter() {
                    let formatted_value = match value {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Number(n) => n.to_string(),
                        serde_json::Value::Bool(b) => b.to_string(),
                        serde_json::Value::Array(arr) => {
                            if arr.len() <= 5 {
                                format!(
                                    "[{}]",
                                    arr.iter()
                                        .map(|v| match v {
                                            serde_json::Value::String(s) => s.clone(),
                                            _ => v.to_string(),
                                        })
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                )
                            } else {
                                format!("[{} 个项目]", arr.len())
                            }
                        }
                        serde_json::Value::Object(_) => "[对象数据]".to_string(),
                        serde_json::Value::Null => "null".to_string(),
                    };

                    formatted_output.push(format!("  - {}: {}", key, formatted_value));
                }

                format!("### 任务 {}\n{}", i + 1, formatted_output.join("\n"))
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    fn generate_default_response(
        &self,
        task_results: &[TaskExecutionResult],
        execution_summary: &ExecutionSummary,
    ) -> String {
        let successful_tasks = task_results
            .iter()
            .filter(|r| r.status == TaskStatus::Completed)
            .count();
        let failed_tasks = task_results
            .iter()
            .filter(|r| r.status == TaskStatus::Failed)
            .count();

        format!(
            "执行完成！\n\n\
            📊 执行统计:\n\
            - 总任务: {}\n\
            - 成功任务: {}\n\
            - 失败任务: {}\n\
            - 总耗时: {}ms\n\n\
            📋 主要结果:\n\
            {}\n\n\
            {}",
            execution_summary.total_tasks,
            successful_tasks,
            failed_tasks,
            execution_summary.total_duration_ms,
            self.summarize_task_outputs(task_results),
            if failed_tasks > 0 {
                "⚠️ 部分任务执行失败，可能需要检查输入参数或网络连接。"
            } else {
                "✅ 所有任务执行成功！"
            }
        )
    }

    fn summarize_task_outputs(&self, task_results: &[TaskExecutionResult]) -> String {
        let mut summaries = Vec::new();

        for result in task_results {
            if result.status == TaskStatus::Completed {
                let summary = if let Some(result_value) = result.outputs.get("result") {
                    format!("- {}: {}", result.task_id, result_value)
                } else {
                    format!("- {}: 执行成功", result.task_id)
                };
                summaries.push(summary);
            } else if result.status == TaskStatus::Failed {
                let error_msg = result.error.as_deref().unwrap_or("未知错误");
                summaries.push(format!("- {}: 执行失败 - {}", result.task_id, error_msg));
            }
        }

        if summaries.is_empty() {
            "无可用结果".to_string()
        } else {
            summaries.join("\n")
        }
    }

    fn calculate_efficiency_metrics(
        &self,
        execution_summary: &ExecutionSummary,
    ) -> EfficiencyMetrics {
        let total_tasks = execution_summary.total_tasks;

        let task_success_rate = if total_tasks > 0 {
            execution_summary.successful_tasks as f32 / total_tasks as f32
        } else {
            0.0
        };

        let average_task_duration_ms = if execution_summary.successful_tasks > 0 {
            execution_summary.total_duration_ms as f32 / execution_summary.successful_tasks as f32
        } else {
            0.0
        };

        // 简化的并行效率计算（基于重规划次数+1作为轮次）
        let total_rounds = execution_summary.replanning_count + 1;
        let average_parallelism = if total_rounds > 0 {
            total_tasks as f32 / total_rounds as f32
        } else {
            0.0
        }
        .min(self.config.max_concurrency as f32);

        let resource_utilization = if self.config.max_concurrency > 0 {
            average_parallelism / self.config.max_concurrency as f32
        } else {
            0.0
        }
        .min(1.0);

        EfficiencyMetrics {
            average_parallelism,
            resource_utilization,
            task_success_rate,
            average_task_duration_ms,
        }
    }

    /// 获取引擎状态
    pub async fn get_engine_status(&self) -> EngineStatus {
        if let Some(task_fetcher) = &self.task_fetcher {
            let task_stats = task_fetcher.get_execution_stats().await;

            EngineStatus {
                is_running: task_stats.executing_tasks > 0,
                pending_tasks: task_stats.waiting_tasks + task_stats.ready_tasks,
                executing_tasks: task_stats.executing_tasks,
                completed_tasks: task_stats.completed_tasks,
                failed_tasks: task_stats.failed_tasks,
                available_capacity: self
                    .executor_pool
                    .as_ref()
                    .map_or(0, |e| e.available_permits()),
                total_capacity: self.config.max_concurrency,
            }
        } else {
            EngineStatus {
                is_running: false,
                pending_tasks: 0,
                executing_tasks: 0,
                completed_tasks: 0,
                failed_tasks: 0,
                available_capacity: self.config.max_concurrency,
                total_capacity: self.config.max_concurrency,
            }
        }
    }

    /// 取消当前执行 (内部方法)
    pub async fn cancel_current_execution(&self) -> Result<()> {
        info!("取消当前执行");
        if let Some(task_fetcher) = &self.task_fetcher {
            task_fetcher.cancel_pending_tasks().await?;
        }
        Ok(())
    }

    /// 重置引擎状态
    pub async fn reset(&self) -> Result<()> {
        info!("重置引擎状态");
        if let Some(task_fetcher) = &self.task_fetcher {
            task_fetcher.cancel_pending_tasks().await?;
        }
        Ok(())
    }

    /// 执行工作流 - 主入口点 (从原有引擎移植)
    pub async fn execute_workflow(
        &self,
        user_query: &str,
        context: Option<std::collections::HashMap<String, serde_json::Value>>,
    ) -> Result<WorkflowExecutionResult> {
        // 这个方法保持与原有引擎完全一致的逻辑
        // info!("开始执行LLMCompiler工作流: {}", user_query);

        let context = context.unwrap_or_default();
        let mut execution_summary = ExecutionSummary {
            total_tasks: 0,
            successful_tasks: 0,
            failed_tasks: 0,
            total_duration_ms: 0,
            replanning_count: 0,
            key_findings: Vec::new(),
            efficiency_metrics: EfficiencyMetrics {
                average_parallelism: 0.0,
                resource_utilization: 0.0,
                task_success_rate: 0.0,
                average_task_duration_ms: 0.0,
            },
        };

        let workflow_start = std::time::Instant::now();
        let mut current_plan: Option<DagExecutionPlan> = None;
        let mut all_results: Vec<TaskExecutionResult> = Vec::new();

        // 主执行循环
        for round in 1..=self.config.max_iterations {
            info!("开始执行轮次: {}/{}", round, self.config.max_iterations);
            // 更新总轮次（这里用replanning_count来跟踪）
            if round > 1 {
                execution_summary.replanning_count = round - 1;
            }

            // 1. 规划阶段
            let execution_plan = if let Some(ref plan) = current_plan {
                // 如果有现有计划，检查是否需要重新规划
                if self.config.enable_replanning && round > 1 {
                    info!("检查是否需要重新规划...");
                    // 这里可以添加重新规划的逻辑
                    plan.clone()
                } else {
                    plan.clone()
                }
            } else {
                // 生成初始计划
                info!("生成初始执行计划...");

                if let Some(planner) = &self.planner {
                    let plan = planner.generate_dag_plan(user_query, &context).await?;
                    current_plan = Some(plan.clone());
                    plan
                } else {
                    return Err(anyhow::anyhow!("Planner not available"));
                }
            };

            info!("执行计划包含 {} 个任务", execution_plan.nodes.len());

            // 2. 任务调度阶段
            if let Some(task_fetcher) = &self.task_fetcher {
                task_fetcher.initialize_plan(&execution_plan).await?;

                // 3. 并行执行阶段
                let round_results = self.execute_parallel_round().await?;

                if round_results.is_empty() {
                    warn!("轮次 {} 没有执行任何任务", round);
                    break;
                }

                // 更新统计信息
                let completed_count = round_results
                    .iter()
                    .filter(|r| r.status == TaskStatus::Completed)
                    .count();
                let failed_count = round_results
                    .iter()
                    .filter(|r| r.status == TaskStatus::Failed)
                    .count();

                execution_summary.successful_tasks += completed_count;
                execution_summary.failed_tasks += failed_count;
                execution_summary.total_tasks += round_results.len();

                let round_duration: u64 = round_results.iter().map(|r| r.duration_ms).sum();
                execution_summary.total_duration_ms += round_duration;

                all_results.extend(round_results.clone());

                info!(
                    "轮次 {} 完成: 成功 {} 个任务, 失败 {} 个任务, 耗时 {}ms",
                    round, completed_count, failed_count, round_duration
                );

                // 4. 智能决策阶段
                if let Some(joiner) = &self.joiner {
                    let mut joiner_guard = joiner.lock().await;
                    let decision = joiner_guard
                        .analyze_and_decide(user_query, &execution_plan, &round_results, round)
                        .await?;

                    // 记录决策信息（可以添加到key_findings中）
                    match &decision {
                        JoinerDecision::Complete { response, .. } => {
                            execution_summary
                                .key_findings
                                .push(format!("决策: 完成执行 - {}", response));
                        }
                        JoinerDecision::Continue { feedback, .. } => {
                            execution_summary
                                .key_findings
                                .push(format!("决策: 继续执行 - {}", feedback));
                        }
                    }

                    match decision {
                        JoinerDecision::Complete { response, .. } => {
                            info!("Joiner decided to complete: {}", response);
                            break;
                        }
                        JoinerDecision::Continue {
                            feedback,
                            suggested_tasks,
                            ..
                        } => {
                            info!("Joiner decided to continue: {}", feedback);

                            // 如果有建议的新任务，触发重规划
                            if !suggested_tasks.is_empty() {
                                info!(
                                    "Adding {} suggested tasks from Joiner",
                                    suggested_tasks.len()
                                );
                                for new_task in suggested_tasks {
                                    // 通过事件系统添加新任务
                                    if let Err(e) = task_fetcher.send_event(
                                        crate::engines::llm_compiler::task_fetcher::SchedulingEvent::TaskAdded {
                                            task: new_task
                                        }
                                    ) {
                                        warn!("Failed to add suggested task: {}", e);
                                    }
                                }
                                execution_summary.replanning_count += 1;
                            } else {
                                // 没有新任务但决定继续，触发重规划
                                if self.config.enable_replanning
                                    && round < self.config.max_iterations
                                {
                                    info!("Triggering replanning due to Continue decision");
                                    match self
                                        .trigger_replanning(
                                            &execution_plan,
                                            &round_results,
                                            &feedback,
                                            task_fetcher,
                                        )
                                        .await
                                    {
                                        Ok(new_tasks_added) => {
                                            if new_tasks_added > 0 {
                                                execution_summary.replanning_count += 1;
                                                info!(
                                                    "Replanning added {} new tasks",
                                                    new_tasks_added
                                                );
                                            } else {
                                                info!("Replanning did not generate new tasks");
                                            }
                                        }
                                        Err(e) => {
                                            warn!("Replanning failed: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // 检查是否还有待执行的任务
                if !task_fetcher.has_pending_tasks().await {
                    info!("所有任务已完成，结束执行");
                    break;
                }
            }
        }

        let total_duration = workflow_start.elapsed();
        execution_summary.total_duration_ms = total_duration.as_millis() as u64;

        // 计算最终效率指标
        execution_summary.efficiency_metrics =
            self.calculate_efficiency_metrics(&execution_summary);

        info!(
            "工作流执行完成: {} 个任务, 成功 {}, 失败 {}, 耗时 {}ms",
            execution_summary.total_tasks,
            execution_summary.successful_tasks,
            execution_summary.failed_tasks,
            execution_summary.total_duration_ms
        );

        // 生成最终结果（暂不传递ID，因为这个方法可能是测试/单独使用）
        let final_response = self
            .generate_final_response(user_query, &all_results, &execution_summary, None, None)
            .await?;

        Ok(WorkflowExecutionResult {
            success: true,
            response: final_response,
            execution_summary: execution_summary.clone(),
            task_results: all_results,
            efficiency_metrics: execution_summary.efficiency_metrics,
            error: None,
        })
    }

    /// 触发重规划并添加新任务
    async fn trigger_replanning(
        &self,
        original_plan: &DagExecutionPlan,
        execution_results: &[TaskExecutionResult],
        feedback: &str,
        task_fetcher: &Arc<TaskFetchingUnit>,
    ) -> Result<usize> {
        info!("Starting replanning process");

        if let Some(planner) = &self.planner {
            // 调用规划器进行重规划
            let new_plan = planner
                .replan(original_plan, execution_results, feedback)
                .await?;

            let new_tasks_count = new_plan.nodes.len();
            info!("Replanning generated {} new tasks", new_tasks_count);

            // 将新任务添加到调度器
            for new_task in new_plan.nodes {
                task_fetcher.send_event(
                    crate::engines::llm_compiler::task_fetcher::SchedulingEvent::TaskAdded {
                        task: new_task,
                    },
                )?;
            }

            // 更新依赖图（如果有新的依赖关系）
            if !new_plan.dependency_graph.is_empty() {
                task_fetcher
                    .merge_dependency_graph(&new_plan.dependency_graph)
                    .await?;
            }

            // 更新变量映射
            if !new_plan.variable_mappings.is_empty() {
                task_fetcher
                    .merge_variable_mappings(&new_plan.variable_mappings)
                    .await?;
            }

            Ok(new_tasks_count)
        } else {
            Err(anyhow::anyhow!("Planner not available for replanning"))
        }
    }
}

/// 工作流执行结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkflowExecutionResult {
    /// 是否成功
    pub success: bool,
    /// 最终响应
    pub response: String,
    /// 执行摘要
    pub execution_summary: ExecutionSummary,
    /// 任务结果列表
    pub task_results: Vec<TaskExecutionResult>,
    /// 效率指标
    pub efficiency_metrics: EfficiencyMetrics,
    /// 错误信息
    pub error: Option<String>,
}

/// 引擎状态
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EngineStatus {
    /// 是否正在运行
    pub is_running: bool,
    /// 待执行任务数
    pub pending_tasks: usize,
    /// 执行中任务数
    pub executing_tasks: usize,
    /// 已完成任务数
    pub completed_tasks: usize,
    /// 失败任务数
    pub failed_tasks: usize,
    /// 可用执行容量
    pub available_capacity: usize,
    /// 总执行容量
    pub total_capacity: usize,
}
