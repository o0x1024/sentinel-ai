//! Plan-and-Execute引擎适配器 - 实现统一的ExecutionEngine接口

use crate::agents::traits::{
    AgentTask, AgentExecutionResult, ExecutionArtifact, ArtifactType,
    ExecutionEngine, EngineInfo, PerformanceCharacteristics,
    ExecutionPlan as AgentExecutionPlan, ExecutionStep as AgentExecutionStep, 
    StepType as AgentStepType, ResourceRequirements, ExecutionProgress
};
use async_trait::async_trait;
use crate::engines::plan_and_execute::{
    planner::Planner,
    executor::{Executor, ExecutionResult},
    replanner::{Replanner, ReplanTrigger},
    memory_manager::MemoryManager,
    types::{TaskRequest, TaskStatus, ExecutionPlan, ExecutionStep, StepType, Priority, TaskType, TargetInfo, TargetType, RetryConfig, PlanAndExecuteConfig, PlanAndExecuteError},
};
use crate::services::{AiServiceManager, database::DatabaseService};

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::AppHandle;
use uuid::Uuid;

/// Plan-and-Execute引擎适配器
#[derive(Debug)]
pub struct PlanAndExecuteEngine {
    engine_info: EngineInfo,
    planner: Option<Arc<Planner>>,
    executor: Option<Arc<Executor>>,
    replanner: Option<Arc<Replanner>>,
    memory_manager: Option<Arc<MemoryManager>>,
    /// 执行期参数（来自 AgentTask.parameters），用于在执行阶段访问，如 prompt_ids 等
    runtime_params: Option<HashMap<String, serde_json::Value>>,
    /// 数据库服务（用于保存步骤详情）
    db_service: Option<Arc<DatabaseService>>,
}

impl PlanAndExecuteEngine {
    /// 创建新的引擎适配器
    pub async fn new() -> Result<Self> {
        let engine_info = EngineInfo {
            name: "Plan-and-Execute".to_string(),
            version: "1.0.0".to_string(),
            description: "Traditional planning and execution architecture with dynamic replanning capabilities".to_string(),
            supported_scenarios: vec![
                "Security Scanning".to_string(),
                "Vulnerability Assessment".to_string(),
                "Asset Discovery".to_string(),
                "Sequential Tasks".to_string(),
            ],
            performance_characteristics: PerformanceCharacteristics {
                token_efficiency: 70,
                execution_speed: 60,
                resource_usage: 40,
                concurrency_capability: 30,
                complexity_handling: 80,
            },
        };

        // 不预创建组件，在实际使用时需要完整初始化
        Ok(Self {
            engine_info,
            planner: None,
            executor: None,
            replanner: None,
            memory_manager: None,
            runtime_params: None,
            db_service: None,
        })
    }
    
    /// 使用完整依赖创建引擎适配器
    pub async fn new_with_dependencies(
        ai_service_manager: Arc<AiServiceManager>,
        config: PlanAndExecuteConfig,
        db_service: Arc<DatabaseService>,
        app_handle: Option<Arc<AppHandle>>,
    ) -> Result<Self> {
        let engine_info = EngineInfo {
            name: "Plan-and-Execute".to_string(),
            version: "1.0.0".to_string(),
            description: "Traditional planning and execution architecture with dynamic replanning capabilities".to_string(),
            supported_scenarios: vec![
                "Security Scanning".to_string(),
                "Vulnerability Assessment".to_string(),
                "Asset Discovery".to_string(),
                "Sequential Tasks".to_string(),
            ],
            performance_characteristics: PerformanceCharacteristics {
                token_efficiency: 70,
                execution_speed: 60,
                resource_usage: 40,
                concurrency_capability: 30,
                complexity_handling: 80,
            },
        };

        // 创建各个核心组件
        let prompt_repo = {
            let pool = db_service.get_pool().map_err(|e| anyhow::anyhow!("DB pool error: {}", e))?;
            Some(crate::services::prompt_db::PromptRepository::new(pool.clone()))
        };

        let mcp_service = ai_service_manager.get_mcp_service();
        
        // 创建组件
        let planner = match Planner::with_ai_service_manager(
            config.planner.clone(), 
            prompt_repo.clone(),
            mcp_service.clone(),
            ai_service_manager.clone(),
            app_handle.clone(),
        ) {
            Ok(p) => Some(Arc::new(p)),
            Err(e) => {
                log::warn!("Failed to create planner: {}", e);
                None
            }
        };
        
        // 创建简化启用版 Replanner（使用 Planner，无需旧 ai_adapter）
        let replanner: Option<Arc<Replanner>> = match prompt_repo.clone() {
            Some(repo) => {
                match Replanner::new(ai_service_manager.clone(), Arc::new(repo), config.replanner.clone()).await {
                    Ok(r) => Some(Arc::new(r)),
                    Err(e) => {
                        log::warn!("Failed to create Replanner: {}. Continuing without replanning.", e);
                        None
                    }
                }
            }
            None => {
                log::warn!("Prompt repository not available; replanner disabled");
                None
            }
        };
        
        // 创建带有 replanner 的 executor
        let executor = Some(Arc::new(Executor::with_replanner(
            config.executor.clone(), 
            db_service.clone(), 
            Some(ai_service_manager.clone()),
            replanner.clone(),
            app_handle.clone(),
        )));
        
        let memory_manager = Some(Arc::new(MemoryManager::new(config.memory_manager.clone())));

        Ok(Self {
            engine_info,
            planner,
            executor,
            replanner,
            memory_manager,
            runtime_params: None,
            db_service: Some(db_service),
        })
    }

    /// 设置运行期参数（单次执行专用）
    pub fn set_runtime_params(&mut self, params: HashMap<String, serde_json::Value>) {
        self.runtime_params = Some(params);
    }
}



impl PlanAndExecuteEngine {



    /// 手动触发重新规划
    pub async fn trigger_replan(
        &self,
        current_plan: &ExecutionPlan,
        task_request: &TaskRequest,
        _trigger: Option<ReplanTrigger>,
    ) -> Result<Option<ExecutionPlan>, PlanAndExecuteError> {
        if let Some(ref replanner) = self.replanner {
            log::info!("手动触发重新规划");
            
            // 创建一个模拟的执行结果来触发重新规划
            let mock_execution_result = ExecutionResult {
                status: TaskStatus::Failed,
                completed_steps: vec![],
                failed_steps: vec!["user_triggered".to_string()],
                skipped_steps: vec![],
                step_results: HashMap::new(),
                metrics: crate::engines::plan_and_execute::executor::ExecutionMetrics::default(),
                errors: vec![],
                enhanced_feedback: crate::engines::plan_and_execute::executor::EnhancedExecutionFeedback::default(),
            };

            // 优先使用 Planner 驱动的重规划（失败则回退到简化重规划）
            let replan_outcome = match replanner
                .replan_with_planner(current_plan, task_request, &mock_execution_result)
                .await
            {
                Ok(r) => Ok(r),
                Err(e) => {
                    log::warn!(
                        "Planner-based manual replanning failed: {}. Falling back to simple replan.",
                        e
                    );
                    match replanner.replan_simple(current_plan, &mock_execution_result).await {
                        Ok(r2) => Ok(r2),
                        Err(e2) => Err(PlanAndExecuteError::ReplanningFailed(e2.to_string())),
                    }
                }
            };

            match replan_outcome {
                Ok(replan_result) => {
                    if replan_result.should_replan {
                        log::info!("重新规划成功: {}", replan_result.replan_reason);
                        Ok(replan_result.new_plan)
                    } else {
                        log::info!("无需重新规划: {}", replan_result.replan_reason);
                        Ok(None)
                    }
                }
                Err(e) => {
                    log::error!("重新规划失败: {}", e);
                    Err(PlanAndExecuteError::ReplanningFailed(e.to_string()))
                }
            }
        } else {
            log::warn!("未配置重新规划器，无法执行重新规划");
            Ok(None)
        }
    }

    /// 优化现有计划（性能优化）
    pub async fn optimize_plan(
        &self,
        current_plan: &ExecutionPlan,
        _task_request: &TaskRequest,
    ) -> Result<Option<ExecutionPlan>, PlanAndExecuteError> {
        if let Some(ref replanner) = self.replanner {
            log::info!("开始优化现有计划");
            
            // 优化即简化重规划：若无失败信息，这里返回 None
            log::info!("优化计划：当前使用简化replan策略，无显式失败信号时不调整");
            Ok(None)
        } else {
            log::warn!("未配置重新规划器，无法执行计划优化");
            Ok(None)
        }
    }
    
    /// 转换AgentTask到TaskRequest
    async fn convert_agent_task_to_request(&self, task: &AgentTask) -> Result<TaskRequest> {
        // 创建目标信息
        let target_info = if let Some(target_str) = &task.target {
            Some(TargetInfo {
                target_type: TargetType::Url,
                identifier: target_str.clone(),
                parameters: HashMap::new(),
                credentials: None,
                metadata: HashMap::new(),
            })
        } else {
            None
        };
        
        Ok(TaskRequest {
            id: Uuid::new_v4().to_string(),
            name: task.description.clone(),
            description: task.description.clone(),
            task_type: TaskType::Research,
            target: target_info,
            parameters: task.parameters.clone(),
            priority: Priority::Medium,
            constraints: HashMap::new(),
            metadata: HashMap::new(),
            created_at: std::time::SystemTime::now(),
        })
    }
    
    /// 将ExecutionPlan转换为AgentExecutionPlan
    async fn convert_execution_plan_to_agent_plan(&self, plan: &ExecutionPlan) -> Result<AgentExecutionPlan> {
        let mut agent_steps = Vec::new();
        
        for step in &plan.steps {
            let agent_step_type = match step.step_type {
                StepType::ToolCall => AgentStepType::ToolCall,
                StepType::AiReasoning => AgentStepType::LlmCall,
                StepType::DataProcessing => AgentStepType::DataProcessing,
                StepType::Conditional => AgentStepType::DataProcessing,
                _ => AgentStepType::DataProcessing,
            };
            
            // 准备参数，包含工具配置信息
            let mut parameters = step.parameters.clone();
            // 将执行期prompt模板ID注入到AI推理步骤参数中
            if let AgentStepType::LlmCall = agent_step_type {
                if let Some(rp) = &self.runtime_params {
                    // 支持 camelCase 兼容：prompt_ids 或 promptIds
                    let pid_obj = rp.get("prompt_ids").or_else(|| rp.get("promptIds"));
                    if let Some(pid_obj) = pid_obj {
                        if let Some(exec_id) = pid_obj.get("executor").and_then(|v| v.as_i64()) {
                            log::info!("Injecting executor prompt template id: {} for step {}", exec_id, step.name);
                            parameters.insert(
                                "prompt_template_executor_id".to_string(),
                                serde_json::Value::Number(serde_json::Number::from(exec_id)),
                            );
                        }
                    } else {
                        log::info!("No prompt_ids/promptIds provided in runtime params; using default executor prompt");
                    }
                    
                    // 将角色提示词传递给每个步骤
                    if let Some(role_prompt) = rp.get("role_prompt").and_then(|v| v.as_str()) {
                        if !role_prompt.trim().is_empty() {
                            parameters.insert(
                                "role_prompt".to_string(),
                                serde_json::Value::String(role_prompt.to_string()),
                            );
                            log::info!("Injecting role prompt to step: {}", step.name);
                        }
                    }
                }
            }
            
            // 如果有工具配置，将其保存到parameters中
            if let Some(ref tool_config) = step.tool_config {
                parameters.insert("tool_name".to_string(), serde_json::Value::String(tool_config.tool_name.clone()));
                
                // 保存工具参数
                if !tool_config.tool_args.is_empty() {
                    let args_value = serde_json::to_value(&tool_config.tool_args)
                        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
                    parameters.insert("tool_args".to_string(), args_value);
                }
                
                // 保存其他工具配置信息
                if let Some(ref version) = tool_config.tool_version {
                    parameters.insert("tool_version".to_string(), serde_json::Value::String(version.clone()));
                }
                if let Some(timeout) = tool_config.timeout {
                    parameters.insert("tool_timeout".to_string(), serde_json::Value::Number(serde_json::Number::from(timeout)));
                }
                if !tool_config.env_vars.is_empty() {
                    let env_value = serde_json::to_value(&tool_config.env_vars)
                        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
                    parameters.insert("tool_env_vars".to_string(), env_value);
                }
            }
            
            agent_steps.push(AgentExecutionStep {
                id: step.id.clone(),
                name: step.name.clone(),
                description: step.description.clone(),
                step_type: agent_step_type,
                dependencies: vec![], // ExecutionStep不包含dependencies字段
                parameters,
            });
        }
        
        Ok(AgentExecutionPlan {
            id: plan.id.clone(),
            name: plan.name.clone(),
            steps: agent_steps,
            estimated_duration: plan.estimated_duration,
            resource_requirements: ResourceRequirements {
                cpu_cores: Some(2),
                memory_mb: Some(512),
                network_concurrency: Some(5),
                disk_space_mb: Some(100),
            },
        })
    }
    
    /// 将AgentExecutionPlan转换为ExecutionPlan
    async fn convert_agent_plan_to_execution_plan(&self, plan: &AgentExecutionPlan) -> Result<ExecutionPlan> {
        let mut execution_steps = Vec::new();
        
        for step in &plan.steps {
            let step_type = match step.step_type {
                AgentStepType::ToolCall => StepType::ToolCall,
                AgentStepType::LlmCall => StepType::AiReasoning,
                AgentStepType::DataProcessing => StepType::DataProcessing,
                _ => StepType::DataProcessing,
            };
            
            // 尝试从parameters中提取工具配置
            let tool_config = if matches!(step.step_type, AgentStepType::ToolCall) {
                // 查找工具相关的参数
                let tool_name = step.parameters.get("tool_name")
                    .or_else(|| step.parameters.get("tool"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                
                if let Some(tool_name) = tool_name {
                    // 提取工具参数
                    let tool_args = step.parameters.get("tool_args")
                        .or_else(|| step.parameters.get("args"))
                        .and_then(|v| v.as_object())
                        .map(|obj| {
                            let mut args = std::collections::HashMap::new();
                            for (k, v) in obj {
                                args.insert(k.clone(), v.clone());
                            }
                            args
                        })
                        .unwrap_or_default();
                    
                    // 提取其他工具配置信息
                    let tool_version = step.parameters.get("tool_version")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    
                    let timeout = step.parameters.get("tool_timeout")
                        .and_then(|v| v.as_u64())
                        .or(Some(300)); // 默认5分钟
                    
                    let env_vars = step.parameters.get("tool_env_vars")
                        .and_then(|v| v.as_object())
                        .map(|obj| {
                            let mut vars = std::collections::HashMap::new();
                            for (k, v) in obj {
                                if let Some(s) = v.as_str() {
                                    vars.insert(k.clone(), s.to_string());
                                }
                            }
                            vars
                        })
                        .unwrap_or_default();
                    
                    Some(crate::engines::plan_and_execute::types::ToolConfig {
                        tool_name,
                        tool_version,
                        tool_args,
                        timeout,
                        env_vars,
                    })
                } else {
                    log::warn!("步骤 '{}' 标记为ToolCall但缺少工具名称", step.name);
                    None
                }
            } else {
                None
            };
            
            execution_steps.push(ExecutionStep {
                id: step.id.clone(),
                name: step.name.clone(),
                description: step.description.clone(),
                step_type,
                tool_config,
                parameters: step.parameters.clone(),
                estimated_duration: 60, // 默认60秒
                retry_config: RetryConfig::default(),
                preconditions: vec![],
                postconditions: vec![],
            });
        }
        
        Ok(ExecutionPlan {
            id: plan.id.clone(),
            task_id: plan.id.clone(), // 使用plan id作为task id
            name: plan.name.clone(),
            description: format!("Converted from agent plan: {}", plan.name),
            steps: execution_steps,
            estimated_duration: plan.estimated_duration,
            created_at: std::time::SystemTime::now(),
            dependencies: HashMap::new(),
            metadata: HashMap::new(),
        })
    }
    
    /// 将ExecutionResult转换为AgentExecutionResult
    async fn convert_execution_result_to_agent_result(
        &self, 
        result: ExecutionResult, 
        start_time: std::time::Instant
    ) -> Result<AgentExecutionResult> {
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(AgentExecutionResult {
            id: Uuid::new_v4().to_string(),
            success: matches!(result.status, TaskStatus::Completed),
            data: Some(serde_json::json!({
                "engine": "plan_execute_original",
                "status": result.status,
                "completed_steps": result.completed_steps,
                "failed_steps": result.failed_steps,
                "step_results": result.step_results,
                "execution_strategy": "original_engine"
            })),
            error: if result.errors.is_empty() {
                None
            } else {
                Some(result.errors.iter().map(|e| e.message.clone()).collect::<Vec<_>>().join("; "))
            },
            execution_time_ms: execution_time,
            resources_used: [
                ("cpu_usage".to_string(), 30.0),
                ("memory_mb".to_string(), 512.0),
                ("execution_time_ms".to_string(), execution_time as f64),
            ].into_iter().collect(),
            artifacts: vec![
                ExecutionArtifact {
                    artifact_type: ArtifactType::LogFile,
                    name: "execution_result".to_string(),
                    data: serde_json::json!({
                        "status": result.status,
                        "metrics": result.metrics,
                        "step_results": result.step_results
                    }),
                    created_at: chrono::Utc::now(),
                }
            ],
        })
    }
    
    /// 根据失败类型创建重新规划触发器
    pub fn create_replan_trigger_from_error(_error: &str, _step_id: String) -> ReplanTrigger {
        // DISABLED: Simplified trigger creation
        if _error.contains("timeout") || _error.contains("超时") {
            ReplanTrigger::Timeout
        } else {
            ReplanTrigger::StepFailure
        }
    }

    /// 批量触发重新规划（用于多个失败步骤）
    pub async fn batch_replan(
        &self,
        current_plan: &ExecutionPlan,
        task_request: &TaskRequest,
        failed_steps: &[String],
    ) -> Result<Option<ExecutionPlan>, PlanAndExecuteError> {
        if failed_steps.is_empty() {
            return Ok(None);
        }

        if let Some(ref _replanner) = self.replanner {
            log::info!("批量重新规划，涉及 {} 个失败步骤", failed_steps.len());
            
            // 创建批量失败触发器
            let trigger = ReplanTrigger::Manual; // DISABLED: simplified trigger
            
            self.trigger_replan(current_plan, task_request, Some(trigger)).await
        } else {
            log::warn!("未配置重新规划器，无法执行批量重新规划");
            Ok(None)
        }
    }

    /// 基于性能指标触发重新规划
    pub async fn trigger_performance_replan(
        &self,
        current_plan: &ExecutionPlan,
        task_request: &TaskRequest,
        avg_step_duration: u64,
        expected_duration: u64,
    ) -> Result<Option<ExecutionPlan>, PlanAndExecuteError> {
        if avg_step_duration <= expected_duration {
            return Ok(None); // 性能符合预期，无需重新规划
        }

        let trigger = ReplanTrigger::Timeout; // DISABLED: simplified trigger

        self.trigger_replan(current_plan, task_request, Some(trigger)).await
    }

    /// 用户手动请求重新规划
    pub async fn user_request_replan(
        &self,
        current_plan: &ExecutionPlan,
        task_request: &TaskRequest,
        reason: String,
    ) -> Result<Option<ExecutionPlan>, PlanAndExecuteError> {
        let trigger = ReplanTrigger::Manual; // DISABLED: simplified trigger
        self.trigger_replan(current_plan, task_request, Some(trigger)).await
    }
}

// 简化的 ExecutionEngine trait 实现
#[async_trait]
impl ExecutionEngine for PlanAndExecuteEngine {
    fn get_engine_info(&self) -> &EngineInfo {
        &self.engine_info
    }
    
    fn supports_task(&self, _task: &AgentTask) -> bool {
        // Plan-and-Execute 引擎支持大多数任务类型
        true
    }
    
    async fn create_plan(&self, task: &AgentTask) -> anyhow::Result<AgentExecutionPlan> {
        // 使用 Planner 基于 LLM 生成执行计划
        let task_req = self.convert_agent_task_to_request(task).await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        // 若未初始化 Planner，回退到简化计划
        let planner = match &self.planner {
            Some(p) => p.clone(),
            None => {
                let fallback = AgentExecutionPlan {
                    id: task.id.clone(),
                    name: format!("Plan for: {}", task.description),
                    steps: vec![AgentExecutionStep {
                        id: "step_1".to_string(),
                        name: "Execute Task".to_string(),
                        description: task.description.clone(),
                        step_type: crate::agents::traits::StepType::ToolCall,
                        dependencies: Vec::new(),
                        parameters: task.parameters.clone(),
                    }],
                    estimated_duration: 300,
                    resource_requirements: ResourceRequirements {
                        cpu_cores: Some(1),
                        memory_mb: Some(512),
                        network_concurrency: Some(1),
                        disk_space_mb: Some(100),
                    },
                };
                return Ok(fallback);
            }
        };

        let planning = planner.create_plan(&task_req).await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        // 转为 AgentExecutionPlan
        self.convert_execution_plan_to_agent_plan(&planning.plan).await
            .map_err(|e| anyhow::anyhow!(e.to_string()))
    }
    
    async fn execute_plan(&self, plan: &AgentExecutionPlan) -> anyhow::Result<AgentExecutionResult> {
        // 使用 Executor 真实执行
        let executor = self.executor.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Executor not initialized"))?;
        let exec_plan = self.convert_agent_plan_to_execution_plan(plan).await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        // 从运行期参数构造 TaskRequest（尽量还原上下文）
        let params = self.runtime_params.clone().unwrap_or_default();
        let task_req = TaskRequest {
            id: exec_plan.task_id.clone(),
            name: plan.name.clone(),
            description: plan.name.clone(),
            task_type: TaskType::Research,
            target: None,
            parameters: params,
            priority: Priority::Medium,
            constraints: std::collections::HashMap::new(),
            metadata: std::collections::HashMap::new(),
            created_at: std::time::SystemTime::now(),
        };

        let start = std::time::Instant::now();
        let result = executor.execute_plan(&exec_plan, &task_req).await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        self.convert_execution_result_to_agent_result(result, start).await
            .map_err(|e| anyhow::anyhow!(e.to_string()))
    }
    
    async fn get_progress(&self, _session_id: &str) -> anyhow::Result<ExecutionProgress> {
        // 简化的进度获取实现
        Ok(ExecutionProgress {
            completed_steps: 1,
            progress_percentage: 100.0,
            estimated_remaining_seconds: Some(0),
            current_step: Some("Completed".to_string()),
            total_steps: 1,
        })
    }
    
    async fn cancel_execution(&self, _session_id: &str) -> anyhow::Result<()> {
        // ✅ 触发executor的取消令牌
        if let Some(executor) = &self.executor {
            executor.cancel();
            log::info!("Plan-and-Execute: Cancelled execution via executor");
        } else {
            log::warn!("Plan-and-Execute: No executor available to cancel");
        }
        Ok(())
    }
}
