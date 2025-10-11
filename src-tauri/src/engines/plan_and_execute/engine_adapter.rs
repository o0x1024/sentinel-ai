//! Plan-and-Execute引擎适配器 - 实现统一的ExecutionEngine接口

use crate::agents::traits::{
    AgentTask, AgentExecutionResult, ExecutionArtifact, ArtifactType,
    ExecutionEngine, EngineInfo, PerformanceCharacteristics,
    ExecutionPlan as AgentExecutionPlan, ExecutionStep as AgentExecutionStep, 
    StepType as AgentStepType, ResourceRequirements, ExecutionProgress
};
use crate::engines::plan_and_execute::{
    planner::Planner,
    executor::{Executor, ExecutionResult},
    replanner::{Replanner, ReplanTrigger},
    memory_manager::MemoryManager,
    types::{TaskRequest, TaskStatus, ExecutionPlan, ExecutionStep, StepType, Priority, TaskType, TargetInfo, TargetType, RetryConfig, PlanAndExecuteConfig, PlanAndExecuteError},
};
use crate::services::{AiServiceManager, database::DatabaseService};

use async_trait::async_trait;
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
        
        let replanner = match Replanner::with_ai_service_manager(
            config.replanner.clone(),
            config.planner.clone(),
            prompt_repo.clone(),
            mcp_service.clone(),
            ai_service_manager.clone(),
            app_handle.clone(),
        ) {
            Ok(r) => Some(Arc::new(r)),
            Err(e) => {
                log::warn!("Failed to create replanner: {}", e);
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
        })
    }

    /// 设置运行期参数（单次执行专用）
    pub fn set_runtime_params(&mut self, params: HashMap<String, serde_json::Value>) {
        self.runtime_params = Some(params);
    }
}


#[async_trait]
impl ExecutionEngine for PlanAndExecuteEngine {
    fn get_engine_info(&self) -> &EngineInfo {
        &self.engine_info
    }
    
    fn supports_task(&self, task: &AgentTask) -> bool {
        // 检查是否支持该任务类型
        if task.description.to_lowercase().contains("scan") ||
           task.description.to_lowercase().contains("security") ||
           task.description.to_lowercase().contains("vulnerability") {
            return true;
        }
        
        // 默认支持大多数任务
        true
    }
    
    async fn create_plan(&self, task: &AgentTask) -> Result<AgentExecutionPlan> {
        // 如果有planner组件，使用它来创建计划
        if let Some(planner) = &self.planner {
            // 将AgentTask转换为TaskRequest
            let task_request = self.convert_agent_task_to_request(task).await?;
            
            // 使用planner创建计划
            match planner.create_plan(&task_request).await {
                Ok(planning_result) => {
                    // 将ExecutionPlan转换为AgentExecutionPlan
                    return self.convert_execution_plan_to_agent_plan(&planning_result.plan).await;
                }
                Err(e) => {
                    log::warn!("Planner failed, falling back to simple plan: {}", e);
                    // 回退到简单计划
                    return Err( e.into());
                }
            }
        }
        
        // 如果没有 planner，创建简单计划
        Ok(AgentExecutionPlan {
            id: Uuid::new_v4().to_string(),
            name: "Simple Plan".to_string(),
            steps: vec![],
            estimated_duration: 300,
            resource_requirements: ResourceRequirements::default(),
        })
    }
    
    async fn execute_plan(
        &self, 
        plan: &AgentExecutionPlan
    ) -> Result<AgentExecutionResult> {
        let start_time = std::time::Instant::now();
        
        // 如果有完整的Plan-and-Execute引擎，使用原生流程
        if let Some(executor) = &self.executor {
            // session.add_log(LogLevel::Info, "使用完整Plan-and-Execute引擎 (Planner->Agent->Tools->Replan)".to_string()).await?;
            
            // 将AgentExecutionPlan转换为ExecutionPlan
            match self.convert_agent_plan_to_execution_plan(plan).await {
                Ok(execution_plan) => {
                    // 创建TaskRequest用于执行
                    let task_request: TaskRequest = TaskRequest {
                        id: Uuid::new_v4().to_string(),
                        name: plan.name.clone(),
                        description: format!("执行Plan-and-Execute任务: {}", plan.name),
                        task_type: TaskType::Research,
                        target: None,
                        parameters: self.runtime_params.clone().unwrap_or_default(),
                        priority: Priority::Medium,
                        constraints: HashMap::new(),
                        metadata: HashMap::new(),
                        created_at: std::time::SystemTime::now(),
                    };
                    
                    // session.add_log(LogLevel::Info, "=== 启动Plan-and-Execute主循环 ===".to_string()).await?;
                    
                    // 使用executor执行计划（包含完整的Planner->Agent->Tools->Replan流程）
                    match executor.execute_plan(&execution_plan, &task_request).await {
                        Ok(execution_result) => {
                            // session.add_log(LogLevel::Info, format!("Plan-and-Execute执行完成: {:?}", execution_result.status)).await?;
                            return self.convert_execution_result_to_agent_result(execution_result, start_time).await;
                        }
                        Err(e) => {
                            // session.add_log(LogLevel::Error, format!("Plan-and-Execute引擎失败: {}", e)).await?;
                            log::warn!("Executor failed, falling back: {}", e);
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Plan conversion failed, falling back: {}", e);
                }
            }
        }
        
        // 如果没有完整的引擎，直接返回错误
        Err(anyhow::anyhow!("Plan-and-Execute引擎未完全初始化"))
    }
    
    async fn get_progress(&self, session_id: &str) -> Result<ExecutionProgress> {
        // Plan-and-Execute进度跟踪
        log::info!("Plan-and-Execute: 查询执行进度，会话ID: {}", session_id);
        
        // 如果有executor，尝试获取真实进度
        if let Some(executor) = &self.executor {
            if let Some(execution_state) = executor.get_execution_status().await {
                let total_steps = execution_state.current_steps.len() + 
                                 execution_state.completed_steps.len() + 
                                 execution_state.failed_steps.len();
                let completed_steps = execution_state.completed_steps.len();
                
                let progress_percentage = if total_steps > 0 {
                    ((completed_steps as f64 / total_steps as f64) * 100.0) as f32
                } else {
                    0.
                };
                
                let current_step = if !execution_state.current_steps.is_empty() {
                    Some(format!("执行中: {:?}", execution_state.current_steps.keys().next()))
                } else if execution_state.is_paused {
                    Some("执行已暂停".to_string())
                } else if execution_state.is_cancelled {
                    Some("执行已取消".to_string())
                } else if completed_steps == total_steps {
                    Some("执行完成".to_string())
                } else {
                    Some("Plan-and-Execute执行中".to_string())
                };
                
                let estimated_remaining = if progress_percentage > 0.0 && progress_percentage < 100.0 {
                    // 简单估算：假设每步骤平均60秒
                    let remaining_steps = total_steps - completed_steps;
                    Some((remaining_steps * 60) as u64)
                } else {
                    None
                };
                
                return Ok(ExecutionProgress {
                    total_steps: total_steps as u32,
                    completed_steps: completed_steps as u32,
                    current_step,
                    progress_percentage,
                    estimated_remaining_seconds: estimated_remaining,
                });
            }
        }
        
        // 无法获取进度信息
        Err(anyhow::anyhow!("无法获取执行进度：引擎未初始化或会话不存在"))
    }

    async fn cancel_execution(&self, _session_id: &str) -> Result<()> {
        // 调用执行器取消当前执行（无会话映射时执行器内部会直接置位取消标记）
        if let Some(executor) = &self.executor {
            if let Err(e) = executor.cancel().await {
                log::warn!("Plan-and-Execute cancel failed: {}", e);
            }
        } else {
            log::warn!("Plan-and-Execute executor not initialized; cancel ignored");
        }
        Ok(())
    }
}

impl PlanAndExecuteEngine {



    /// 手动触发重新规划
    pub async fn trigger_replan(
        &self,
        current_plan: &ExecutionPlan,
        task_request: &TaskRequest,
        trigger: Option<ReplanTrigger>,
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
            
            match replanner.analyze_and_replan(current_plan, &mock_execution_result, task_request, trigger).await {
                Ok(replan_result) => {
                    if replan_result.should_replan {
                        log::info!("重新规划成功: {}", replan_result.replan_reason);
                        Ok(replan_result.new_plan)
                    } else {
                        log::info!("无需重新规划: {}", replan_result.replan_reason);
                        Ok(None)
                    }
                },
                Err(e) => {
                    log::error!("重新规划失败: {}", e);
                    Err(e)
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
        task_request: &TaskRequest,
    ) -> Result<Option<ExecutionPlan>, PlanAndExecuteError> {
        if let Some(ref replanner) = self.replanner {
            log::info!("开始优化现有计划");
            
            match replanner.optimize_plan(current_plan, task_request).await {
                Ok(replan_result) => {
                    if replan_result.should_replan {
                        log::info!("计划优化成功: {}", replan_result.replan_reason);
                        Ok(replan_result.new_plan)
                    } else {
                        log::info!("计划已是最优: {}", replan_result.replan_reason);
                        Ok(None)
                    }
                },
                Err(e) => {
                    log::error!("计划优化失败: {}", e);
                    Err(e)
                }
            }
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
    pub fn create_replan_trigger_from_error(error: &str, step_id: String) -> ReplanTrigger {
        if error.contains("timeout") || error.contains("超时") {
            ReplanTrigger::ExecutionTimeout {
                expected_duration: 30000, // 30秒默认期望时间
                actual_duration: 60000,   // 假设实际用了60秒
            }
        } else if error.contains("network") || error.contains("网络") {
            ReplanTrigger::ExternalConditionChange {
                condition: "Network Connectivity".to_string(),
                old_value: "Connected".to_string(),
                new_value: "Disconnected".to_string(),
            }
        } else if error.contains("resource") || error.contains("资源") {
            ReplanTrigger::ResourceConstraint {
                resource_type: "Memory".to_string(),
                available: 256,
                required: 512,
            }
        } else {
            ReplanTrigger::StepFailure {
                step_id,
                error_message: error.to_string(),
                retry_count: 0,
            }
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
            let trigger = ReplanTrigger::QualityThreshold {
                metric: "Success Rate".to_string(),
                threshold: 0.8,
                actual: (1.0 - failed_steps.len() as f64 / current_plan.steps.len() as f64).max(0.0),
            };
            
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

        let trigger = ReplanTrigger::ExecutionTimeout {
            expected_duration,
            actual_duration: avg_step_duration,
        };

        self.trigger_replan(current_plan, task_request, Some(trigger)).await
    }

    /// 用户手动请求重新规划
    pub async fn user_request_replan(
        &self,
        current_plan: &ExecutionPlan,
        task_request: &TaskRequest,
        reason: String,
    ) -> Result<Option<ExecutionPlan>, PlanAndExecuteError> {
        let trigger = ReplanTrigger::UserRequest { reason };
        self.trigger_replan(current_plan, task_request, Some(trigger)).await
    }
}
