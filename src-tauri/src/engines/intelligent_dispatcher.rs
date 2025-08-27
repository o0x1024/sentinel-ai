//! 智能调度器主模块
//! 
//! 这是一个LLM驱动的动态架构选择和执行系统，负责：
//! - 智能分析用户查询特征
//! - 动态选择最适合的Agent架构
//! - 生成优化的Prompt模板
//! - 创建和执行智能工作流
//! - 任务队列管理和负载均衡

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use anyhow::Result;
use log::info;

use crate::services::ai::AiServiceManager;
use crate::services::mcp::McpService;
use crate::services::database::DatabaseService;
use crate::managers::ExecutionManager;
use serde_json::Value;

// 导入三个引擎
use crate::engines::plan_and_execute::{PlanAndExecuteEngine, PlanAndExecuteConfig};
use crate::engines::rewoo::ReWooEngine;
use crate::engines::rewoo::rewoo_types::ReWOOConfig;
use crate::engines::llm_compiler::{LlmCompilerEngine, LlmCompilerConfig};
use crate::agents::traits::{AgentTask, ExecutionEngine};

// 导入子模块
pub mod query_analyzer;
pub mod architecture_selector;
pub mod prompt_generator;
pub mod workflow_engine;
pub mod types;
pub mod task_queue;
pub mod load_balancer;

use query_analyzer::{QueryAnalyzer, QueryAnalysisResult};
use architecture_selector::{ArchitectureSelector, ArchitectureSelectionResult};
use prompt_generator::{DynamicPromptGenerator, PromptGenerationResult};
use workflow_engine::{WorkflowEngine, WorkflowDefinition, WorkflowMetadata, WorkflowStep, RetryConfig, BackoffStrategy, ErrorHandling, ErrorStrategy, NotificationConfig};
use types::{DispatchResult, DispatchDecision, AgentArchitecture, TaskType, TaskComplexity, WorkflowStatus};
use task_queue::{TaskQueue, TaskItem};
use load_balancer::{LoadBalancer, ExecutionNode};

/// 查询特征分析结果（兼容旧版本）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryFeatures {
    /// 任务类型
    pub task_type: String,
    /// 子类别
    pub sub_category: String,
    /// 并行化潜力 (high|medium|low)
    pub parallelization_potential: String,
    /// 复杂度等级 (simple|medium|complex)
    pub complexity_level: String,
    /// 时间敏感性 (high|medium|low)
    pub time_sensitivity: String,
    /// 依赖复杂度 (simple|medium|complex)
    pub dependency_complexity: String,
    /// 预估步骤数
    pub estimated_steps: u32,
    /// 资源需求 (light|medium|heavy)
    pub resource_requirements: String,
    /// 关键指标
    pub key_indicators: Vec<String>,
    /// 目标域名或IP
    pub target_domain: Option<String>,
}

/// 架构选择结果（兼容旧版本）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureSelection {
    /// 选择的架构
    pub selected_architecture: String,
    /// 置信度分数
    pub confidence_score: f32,
    /// 选择理由
    pub selection_reasoning: String,
    /// 架构配置
    pub architecture_config: ArchitectureConfig,
    /// 备选架构
    pub fallback_architecture: String,
}

/// 架构配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureConfig {
    /// 最大并行任务数
    pub max_parallel_tasks: u32,
    /// 每个任务超时时间
    pub timeout_per_task: u32,
    /// 重试策略
    pub retry_policy: String,
    /// 资源限制
    pub resource_limits: Option<ResourceConfig>,
}

/// 资源配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConfig {
    /// CPU核心数
    pub cpu_cores: u32,
    /// 内存限制(GB)
    pub memory_gb: u32,
    /// 网络并发数
    pub network_concurrent: u32,
}

/// 动态生成的Prompt集合（兼容旧版本）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicPrompts {
    /// 规划器Prompt
    pub planner: String,
    /// 执行器Prompt
    pub executor: String,
    /// 分析器Prompt
    pub analyzer: Option<String>,
    /// 工具选择器Prompt
    pub tool_selector: Option<String>,
}

/// 智能调度器
pub struct IntelligentDispatcher {
    /// AI服务管理器
    ai_service_manager: Arc<AiServiceManager>,
    /// MCP服务
    mcp_service: Arc<McpService>,
    /// 数据库服务
    database_service: Arc<DatabaseService>,
    /// 执行管理器
    execution_manager: Arc<ExecutionManager>,
    /// 工作流引擎（保留作为备用）
    workflow_engine: Arc<WorkflowEngine>,
    /// 查询分析器
    query_analyzer: QueryAnalyzer,
    /// 架构选择器
    architecture_selector: ArchitectureSelector,
    /// 动态Prompt生成器
    prompt_generator: DynamicPromptGenerator,
    /// 任务队列
    task_queue: Arc<TaskQueue>,
    /// 负载均衡器
    load_balancer: Arc<LoadBalancer>,
    /// 执行历史
    execution_history: Arc<RwLock<Vec<ExecutionRecord>>>,
}

/// 执行记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub query: String,
    pub features: QueryFeatures,
    pub selected_architecture: String,
    pub execution_time: u64,
    pub success_rate: f32,
    pub parallel_efficiency: f32,
    pub timestamp: DateTime<Utc>,
}

impl IntelligentDispatcher {
    /// 创建新的智能调度器
    pub async fn new(
        ai_service_manager: Arc<AiServiceManager>,
        mcp_service: Arc<McpService>,
        workflow_engine: Arc<WorkflowEngine>,
    ) -> Result<Self> {
        // 创建数据库服务和执行管理器（模拟，实际应该从外部传入）
        let database_service = Arc::new(DatabaseService::new());
        let execution_manager = Arc::new(ExecutionManager::new());
        
        Self::new_with_dependencies(
            ai_service_manager,
            mcp_service,
            database_service,
            execution_manager,
            workflow_engine,
        ).await
    }

    /// 使用完整依赖创建智能调度器
    pub async fn new_with_dependencies(
        ai_service_manager: Arc<AiServiceManager>,
        mcp_service: Arc<McpService>,
        database_service: Arc<DatabaseService>,
        execution_manager: Arc<ExecutionManager>,
        workflow_engine: Arc<WorkflowEngine>,
    ) -> Result<Self> {
        // 创建带有AI服务管理器的查询分析器：直接从全局适配器获取默认/可用的Provider
        use crate::ai_adapter::core::AiAdapterManager;
        let provider = AiAdapterManager::global()
            .get_provider_or_default("")
            .map_err(|e| anyhow::anyhow!("Failed to get AI provider for QueryAnalyzer: {}", e))?;

        let query_analyzer = QueryAnalyzer::new_with_service_manager(
            provider,
            ai_service_manager.clone()
        );
        
        let architecture_selector = ArchitectureSelector::new(ai_service_manager.clone());
        let prompt_generator = DynamicPromptGenerator::new();
        let task_queue = Arc::new(TaskQueue::new(None));
        let load_balancer = Arc::new(LoadBalancer::new(None));
        
        Ok(Self {
            ai_service_manager,
            mcp_service,
            database_service,
            execution_manager,
            workflow_engine,
            query_analyzer,
            architecture_selector,
            prompt_generator,
            task_queue,
            load_balancer,
            execution_history: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// 验证数据库连接是否有效
    pub fn validate_database_connection(&self) -> Result<()> {
        self.database_service.get_pool()
            .map(|_| ())
            .map_err(|e| anyhow::anyhow!("Database connection validation failed: {}", e))
    }
    
    /// 智能处理用户查询
    pub async fn process_query(&mut self, user_input: &str) -> Result<DispatchResult> {
        info!("🔍 开始智能分析用户查询: {}", user_input);
        info!("📋 执行步骤: 1.查询分析 -> 2.架构选择 -> 3.Prompt生成 -> 4.工作流创建 -> 5.工作流执行");
        
        // 1. 查询分析阶段
        info!("🔄 步骤1: 开始查询特征分析...");
        let analysis_result = self.query_analyzer.analyze_query(user_input).await
            .map_err(|e| anyhow::anyhow!("查询分析失败: {}", e))?;
        let query_features = self.convert_analysis_to_features(&analysis_result);
        info!("✅ 步骤1完成: 查询特征分析 - 任务类型: {}, 复杂度: {}, 目标域: {:?}, 预估步骤: {}", 
              query_features.task_type, 
              query_features.complexity_level,
              query_features.target_domain,
              query_features.estimated_steps);
        
        // 2. 架构选择阶段
        info!("🔄 步骤2: 开始智能架构选择...");
        let selection_result = self.architecture_selector.select_architecture(&analysis_result).await
            .map_err(|e| anyhow::anyhow!("架构选择失败: {}", e))?;
        let architecture_selection = self.convert_selection_to_legacy(&selection_result);
        info!("✅ 步骤2完成: 架构选择 - 选择架构: {}, 置信度: {:.2}, 选择理由: {}", 
              architecture_selection.selected_architecture, 
              architecture_selection.confidence_score,
              architecture_selection.selection_reasoning);
        
        // 3. 动态Prompt生成阶段
        info!("🔄 步骤3: 开始动态Prompt生成...");
        let prompt_result = self.prompt_generator.generate_prompt(
            &analysis_result,
            &selection_result,
            user_input,
            None
        ).await
            .map_err(|e| anyhow::anyhow!("Prompt生成失败: {}", e))?;
        let custom_prompts = self.convert_prompt_to_legacy(&prompt_result);
        info!("✅ 步骤3完成: 动态Prompt生成 - 规划器Prompt长度: {}, 执行器Prompt长度: {}",
              custom_prompts.planner.len(),
              custom_prompts.executor.len());
        
        // 4. 创建智能工作流
        info!("🔄 步骤4: 开始创建{}智能工作流...", architecture_selection.selected_architecture);
        let workflow = self.create_intelligent_workflow(
            &architecture_selection,
            &query_features,
            &custom_prompts,
            user_input
        ).await?;
        info!("✅ 步骤4完成: 工作流创建 - 工作流ID: {}, 步骤数: {}",
              workflow.metadata.id,
              workflow.steps.len());
        
        // 5. 直接执行引擎（不再使用抽象工作流）
        info!("🔄 步骤5: 开始直接执行选择的引擎...");
        let execution_id = self.execute_with_selected_engine(
            &architecture_selection,
            &query_features,
            &custom_prompts,
            user_input
        ).await?;
        info!("✅ 步骤5启动: 引擎执行已启动 - 执行ID: {}", execution_id);
        
        // 6. 记录执行历史
        info!("📝 记录执行历史到内存缓存...");
        self.record_execution(user_input, &query_features, &architecture_selection).await;
        
        // 7. 创建调度结果
        let result = DispatchResult {
            request_id: Uuid::new_v4().to_string(),
            execution_id: execution_id.clone(),
            decision: DispatchDecision {
                architecture: self.map_to_agent_architecture(&architecture_selection.selected_architecture),
                task_type: self.map_to_task_type(&query_features.task_type),
                complexity: self.map_to_complexity(&query_features.complexity_level),
                reasoning: architecture_selection.selection_reasoning,
                confidence: architecture_selection.confidence_score,
                estimated_duration: Some(self.estimate_duration(&query_features)),
                suggested_workflow: Some(workflow),
            },
            status: WorkflowStatus::Running,
            started_at: Utc::now(),
            completed_at: None,
            result: None,
            error: None,
        };
        
        info!("🎉 智能调度完成! 请求ID: {}, 执行ID: {}, 预估时长: {}秒",
              result.request_id,
              execution_id,
              result.decision.estimated_duration.unwrap_or(0));
        
        Ok(result)
    }

    /// 直接执行选择的引擎
    async fn execute_with_selected_engine(
        &self,
        architecture_selection: &ArchitectureSelection,
        query_features: &QueryFeatures,
        custom_prompts: &DynamicPrompts,
        user_input: &str,
    ) -> Result<String> {
        let execution_id = Uuid::new_v4().to_string();
        
        info!("🚀 开始执行{}引擎...", architecture_selection.selected_architecture);
        
        // 创建Agent任务
        let task = AgentTask {
            id: execution_id.clone(),
            user_id: "intelligent_dispatcher".to_string(),
            description: user_input.to_string(),
            priority: self.map_priority(&query_features.complexity_level),
            target: query_features.target_domain.clone(),
            parameters: {
                let mut params = std::collections::HashMap::new();
                params.insert("custom_planner_prompt".to_string(), 
                    serde_json::Value::String(custom_prompts.planner.clone()));
                params.insert("custom_executor_prompt".to_string(), 
                    serde_json::Value::String(custom_prompts.executor.clone()));
                if let Some(analyzer) = &custom_prompts.analyzer {
                    params.insert("custom_analyzer_prompt".to_string(), 
                        serde_json::Value::String(analyzer.clone()));
                }
                params.insert("task_type".to_string(), 
                    serde_json::Value::String(query_features.task_type.clone()));
                params.insert("complexity".to_string(), 
                    serde_json::Value::String(query_features.complexity_level.clone()));
                params
            },
            timeout: Some(self.estimate_timeout(&query_features) as u64),
        };

        // 根据选择的架构创建和执行对应的引擎
        match architecture_selection.selected_architecture.as_str() {
            "PlanAndExecute" => {
                self.execute_plan_and_execute_engine(task, custom_prompts).await
            },
            "ReWoo" => {
                self.execute_rewoo_engine(task, custom_prompts).await
            },
            "LlmCompiler" => {
                self.execute_llm_compiler_engine(task, custom_prompts).await
            },
            _ => {
                // 默认使用Plan-Execute
                self.execute_plan_and_execute_engine(task, custom_prompts).await
            }
        }
    }

    /// 执行Plan-and-Execute引擎
    async fn execute_plan_and_execute_engine(
        &self,
        task: AgentTask,
        _custom_prompts: &DynamicPrompts,
    ) -> Result<String> {
        info!("🔧 创建Plan-and-Execute引擎实例...");
        
        // 创建Plan-and-Execute引擎配置
        let config = PlanAndExecuteConfig::default();
        
        // 创建Plan-and-Execute引擎
        let engine = PlanAndExecuteEngine::new_with_dependencies(
            self.ai_service_manager.clone(),
            config,
            self.database_service.clone(),
        ).await?;
        
        // 创建执行计划
        let plan = engine.create_plan(&task).await?;
        
        info!("✅ Plan-and-Execute计划创建成功，步骤数: {}", plan.steps.len());
        
        let task_id = task.id.clone();
        
        // 注册到执行管理器
        let engine_instance = crate::managers::EngineInstance::PlanExecute(engine);
        self.execution_manager.register_execution(
            task_id.clone(),
            crate::managers::EngineType::PlanExecute,
            plan,
            task,
            engine_instance,
        ).await?;
        
        info!("🎯 Plan-and-Execute引擎注册成功，执行ID: {}", task_id);
        
        Ok(task_id)
    }

    /// 执行ReWOO引擎
    async fn execute_rewoo_engine(
        &self,
        task: AgentTask,
        _custom_prompts: &DynamicPrompts,
    ) -> Result<String> {
        info!("🔧 创建ReWOO引擎实例...");
        
        // 创建ReWOO引擎配置
        let config = ReWOOConfig::default();
        
        // 创建ReWOO引擎
        let engine = ReWooEngine::new_with_dependencies(
            self.ai_service_manager.clone(),
            config,
            self.database_service.clone(),
        ).await?;
        
        // 创建执行计划
        let plan = engine.create_plan(&task).await?;
        
        info!("✅ ReWOO推理计划创建成功，步骤数: {}", plan.steps.len());
        
        let task_id = task.id.clone();
        
        // 注册到执行管理器
        let engine_instance = crate::managers::EngineInstance::ReWOO(engine);
        self.execution_manager.register_execution(
            task_id.clone(),
            crate::managers::EngineType::ReWOO,
            plan,
            task,
            engine_instance,
        ).await?;
        
        info!("🎯 ReWOO引擎注册成功，执行ID: {}", task_id);
        
        Ok(task_id)
    }

    /// 执行LLMCompiler引擎
    async fn execute_llm_compiler_engine(
        &self,
        task: AgentTask,
        _custom_prompts: &DynamicPrompts,
    ) -> Result<String> {
        info!("🔧 创建LLMCompiler引擎实例...");
        
        // 创建LLMCompiler引擎配置
        let config = LlmCompilerConfig::default();
        
        // 创建LLMCompiler引擎
        let engine = LlmCompilerEngine::new_with_dependencies(
            self.ai_service_manager.clone(),
            config,
            self.database_service.clone(),
        ).await?;
        
        // 创建执行计划
        let plan = engine.create_plan(&task).await?;
        
        info!("✅ LLMCompiler并行计划创建成功，步骤数: {}", plan.steps.len());
        
        let task_id = task.id.clone();
        
        // 注册到执行管理器
        let engine_instance = crate::managers::EngineInstance::LLMCompiler(engine);
        self.execution_manager.register_execution(
            task_id.clone(),
            crate::managers::EngineType::LLMCompiler,
            plan,
            task,
            engine_instance,
        ).await?;
        
        info!("🎯 LLMCompiler引擎注册成功，执行ID: {}", task_id);
        
        Ok(task_id)
    }

    /// 映射优先级
    fn map_priority(&self, complexity: &str) -> crate::agents::traits::TaskPriority {
        use crate::agents::traits::TaskPriority;
        match complexity {
            "simple" => TaskPriority::Low,
            "medium" => TaskPriority::Normal,
            "complex" => TaskPriority::High,
            _ => TaskPriority::Normal,
        }
    }

    /// 估算超时时间
    fn estimate_timeout(&self, features: &QueryFeatures) -> u32 {
        let base_timeout = match features.complexity_level.as_str() {
            "simple" => 180,  // 3分钟
            "medium" => 600,  // 10分钟
            "complex" => 1800, // 30分钟
            _ => 300,          // 5分钟默认
        };
        
        // 根据步骤数调整
        let step_factor = (features.estimated_steps as f32 * 0.1).max(1.0);
        (base_timeout as f32 * step_factor) as u32
    }

    /// 创建智能工作流
    async fn create_intelligent_workflow(
        &self,
        architecture_selection: &ArchitectureSelection,
        query_features: &QueryFeatures,
        custom_prompts: &DynamicPrompts,
        user_input: &str,
    ) -> Result<WorkflowDefinition> {
        info!("🏗️ 根据架构类型创建工作流: {}", architecture_selection.selected_architecture);
        
        let workflow = match architecture_selection.selected_architecture.as_str() {
            "LlmCompiler" => {
                info!("🔧 创建LLMCompiler并行执行工作流...");
                self.create_llm_compiler_workflow(query_features, custom_prompts, user_input).await?
            },
            "ReWoo" => {
                info!("🔧 创建ReWOO推理链工作流...");
                self.create_rewoo_workflow(query_features, custom_prompts, user_input).await?
            },
            "PlanAndExecute" | _ => {
                info!("🔧 创建Plan-Execute基础工作流...");
                self.create_plan_execute_workflow(query_features, custom_prompts, user_input).await?
            },
        };
        
        info!("✅ 工作流创建成功: {} - {}", workflow.metadata.name, workflow.metadata.description);
        Ok(workflow)
    }

    /// 创建LLMCompiler工作流
    async fn create_llm_compiler_workflow(
        &self,
        query_features: &QueryFeatures,
        custom_prompts: &DynamicPrompts,
        user_input: &str,
    ) -> Result<WorkflowDefinition> {
        let target = query_features.target_domain.clone().unwrap_or("未指定".to_string());
        
        let workflow = WorkflowDefinition {
            metadata: WorkflowMetadata {
                id: Uuid::new_v4().to_string(),
                name: "LLMCompiler并行推理工作流".to_string(),
                version: "2.0.0".to_string(),
                description: "基于LLMCompiler架构的流式并行执行工作流".to_string(),
                author: Some("IntelligentDispatcher".to_string()),
                tags: vec!["llm-compiler".to_string(), "streaming".to_string(), "parallel".to_string()],
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            steps: vec![
                WorkflowStep {
                    id: "planner".to_string(),
                    name: "规划器".to_string(),
                    agent_type: "llm_compiler_planner".to_string(),
                    action: "stream_task_dag".to_string(),
                    inputs: {
                        let mut inputs = HashMap::new();
                        inputs.insert("user_request".to_string(), Value::String(user_input.to_string()));
                        inputs.insert("target_domain".to_string(), Value::String(target.clone()));
                        inputs.insert("custom_prompt".to_string(), Value::String(custom_prompts.planner.clone()));
                        inputs
                    },
                    outputs: {
                        let mut outputs = HashMap::new();
                        outputs.insert("task_dag_stream".to_string(), "task_dag_stream".to_string());
                        outputs.insert("task_dependencies".to_string(), "task_dependencies".to_string());
                        outputs
                    },
                    depends_on: vec![],
                    condition: None,
                    retry: Some(RetryConfig {
                        max_attempts: 3,
                        delay: 5,
                        backoff: BackoffStrategy::Exponential { multiplier: 2.0 },
                        retry_on: vec!["planning_error".to_string()],
                    }),
                    timeout: Some(300),
                    parallel: false,
                    config: Some(serde_json::json!({
                        "enable_streaming": true,
                        "dag_max_depth": 10
                    })),
                },
            ],
            variables: HashMap::new(),
            error_handling: Some(ErrorHandling {
                default_strategy: ErrorStrategy::Retry,
                step_strategies: HashMap::new(),
                on_error: None,
            }),
            notifications: Some(NotificationConfig {
                on_success: vec![],
                on_failure: vec![],
                on_progress: vec![],
            }),
        };
        
        Ok(workflow)
    }

    /// 创建ReWOO工作流
    async fn create_rewoo_workflow(
        &self,
        _query_features: &QueryFeatures,
        custom_prompts: &DynamicPrompts,
        user_input: &str,
    ) -> Result<WorkflowDefinition> {
        let workflow = WorkflowDefinition {
            metadata: WorkflowMetadata {
                id: Uuid::new_v4().to_string(),
                name: "ReWOO推理工作流".to_string(),
                version: "2.0.0".to_string(),
                description: "基于ReWOO架构的推理工作流".to_string(),
                author: Some("IntelligentDispatcher".to_string()),
                tags: vec!["rewoo".to_string(), "reasoning".to_string()],
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            steps: vec![
                WorkflowStep {
                    id: "planner".to_string(),
                    name: "规划器".to_string(),
                    agent_type: "rewoo_planner".to_string(),
                    action: "generate_task_plan".to_string(),
                    inputs: {
                        let mut inputs = HashMap::new();
                        inputs.insert("user_request".to_string(), Value::String(user_input.to_string()));
                        inputs.insert("custom_prompt".to_string(), Value::String(custom_prompts.planner.clone()));
                        inputs
                    },
                    outputs: {
                        let mut outputs = HashMap::new();
                        outputs.insert("task_list".to_string(), "task_list".to_string());
                        outputs.insert("variable_mappings".to_string(), "variable_mappings".to_string());
                        outputs
                    },
                    depends_on: vec![],
                    condition: None,
                    retry: Some(RetryConfig {
                        max_attempts: 3,
                        delay: 5,
                        backoff: BackoffStrategy::Exponential { multiplier: 2.0 },
                        retry_on: vec!["planning_error".to_string()],
                    }),
                    timeout: Some(300),
                    parallel: false,
                    config: Some(serde_json::json!({
                        "max_tasks": 10,
                        "enable_variable_substitution": true
                    })),
                },
            ],
            variables: HashMap::new(),
            error_handling: Some(ErrorHandling {
                default_strategy: ErrorStrategy::Retry,
                step_strategies: HashMap::new(),
                on_error: None,
            }),
            notifications: Some(NotificationConfig {
                on_success: vec![],
                on_failure: vec![],
                on_progress: vec![],
            }),
        };
        
        Ok(workflow)
    }

    /// 创建Plan-Execute工作流
    async fn create_plan_execute_workflow(
        &self,
        query_features: &QueryFeatures,
        custom_prompts: &DynamicPrompts,
        user_input: &str,
    ) -> Result<WorkflowDefinition> {
        let workflow = WorkflowDefinition {
            metadata: WorkflowMetadata {
                id: Uuid::new_v4().to_string(),
                name: "智能Plan-Execute工作流".to_string(),
                version: "1.0.0".to_string(),
                description: "基于LangGraph架构的计划执行工作流".to_string(),
                author: Some("IntelligentDispatcher".to_string()),
                tags: vec!["plan-execute".to_string(), "intelligent".to_string()],
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            steps: vec![
                WorkflowStep {
                    id: "planner".to_string(),
                    name: "规划器".to_string(),
                    agent_type: "plan_and_execute".to_string(),
                    action: "create_plan".to_string(),
                    inputs: {
                        let mut inputs = HashMap::new();
                        inputs.insert("user_input".to_string(), Value::String(user_input.to_string()));
                        inputs.insert("custom_prompt".to_string(), Value::String(custom_prompts.planner.clone()));
                        inputs
                    },
                    outputs: {
                        let mut outputs = HashMap::new();
                        outputs.insert("plan".to_string(), "plan".to_string());
                        outputs.insert("plan_steps".to_string(), "plan_steps".to_string());
                        outputs
                    },
                    depends_on: vec![],
                    condition: None,
                    retry: Some(RetryConfig {
                        max_attempts: 3,
                        delay: 5,
                        backoff: BackoffStrategy::Exponential { multiplier: 2.0 },
                        retry_on: vec!["timeout".to_string()],
                    }),
                    timeout: Some(300),
                    parallel: false,
                    config: Some(serde_json::json!({
                        "max_plan_steps": 10,
                        "enable_parallel_planning": query_features.parallelization_potential == "high"
                    })),
                },
            ],
            variables: HashMap::new(),
            error_handling: Some(ErrorHandling {
                default_strategy: ErrorStrategy::Retry,
                step_strategies: HashMap::new(),
                on_error: None,
            }),
            notifications: Some(NotificationConfig {
                on_success: vec![],
                on_failure: vec![],
                on_progress: vec![],
            }),
        };
        
        Ok(workflow)
    }

    /// 转换分析结果到旧版本格式
    fn convert_analysis_to_features(&self, analysis: &QueryAnalysisResult) -> QueryFeatures {
        QueryFeatures {
            task_type: analysis.task_type.clone(),
            sub_category: analysis.sub_category.clone(),
            parallelization_potential: analysis.parallelization_potential.clone(),
            complexity_level: analysis.complexity_level.clone(),
            time_sensitivity: analysis.time_sensitivity.clone(),
            dependency_complexity: analysis.dependency_complexity.clone(),
            estimated_steps: analysis.estimated_steps,
            resource_requirements: analysis.resource_requirements.clone(),
            key_indicators: analysis.key_indicators.clone(),
            target_domain: analysis.target_domain.clone(),
        }
    }

    /// 转换选择结果到旧版本格式
    fn convert_selection_to_legacy(&self, selection: &ArchitectureSelectionResult) -> ArchitectureSelection {
        ArchitectureSelection {
            selected_architecture: selection.selected_architecture.clone(),
            confidence_score: selection.confidence_score,
            selection_reasoning: selection.selection_reasoning.clone(),
            architecture_config: ArchitectureConfig {
                max_parallel_tasks: selection.architecture_config.max_parallel_tasks,
                timeout_per_task: selection.architecture_config.timeout_per_task,
                retry_policy: selection.architecture_config.retry_policy.clone(),
                resource_limits: selection.architecture_config.resource_limits.as_ref().map(|r| ResourceConfig {
                    cpu_cores: r.cpu_cores,
                    memory_gb: r.memory_gb,
                    network_concurrent: r.network_concurrent,
                }),
            },
            fallback_architecture: selection.fallback_architecture.clone(),
        }
    }

    /// 转换Prompt结果到旧版本格式
    fn convert_prompt_to_legacy(&self, prompt: &PromptGenerationResult) -> DynamicPrompts {
        DynamicPrompts {
            planner: prompt.planner_prompt.clone(),
            executor: prompt.executor_prompt.clone(),
            analyzer: prompt.analyzer_prompt.clone(),
            tool_selector: prompt.tool_selector_prompt.clone(),
        }
    }

    /// 映射到代理架构类型
    fn map_to_agent_architecture(&self, architecture: &str) -> AgentArchitecture {
        match architecture {
            "LlmCompiler" => AgentArchitecture::LlmCompiler,
            "ReWoo" => AgentArchitecture::ReWoo,
            "PlanAndExecute" => AgentArchitecture::PlanAndExecute,
            _ => AgentArchitecture::PlanAndExecute,
        }
    }

    /// 映射到任务类型
    fn map_to_task_type(&self, task_type: &str) -> TaskType {
        match task_type {
            "扫描任务" => TaskType::Scanning,
            "分析任务" => TaskType::Analysis,
            "查询任务" => TaskType::Query,
            "配置任务" => TaskType::Configuration,
            "监控任务" => TaskType::Monitoring,
            _ => TaskType::Other,
        }
    }

    /// 映射到任务复杂度
    fn map_to_complexity(&self, complexity: &str) -> TaskComplexity {
        match complexity {
            "simple" => TaskComplexity::Simple,
            "medium" => TaskComplexity::Medium,
            "complex" => TaskComplexity::Complex,
            _ => TaskComplexity::Medium,
        }
    }

    /// 预估执行时间
    fn estimate_duration(&self, features: &QueryFeatures) -> u64 {
        let base_time = features.estimated_steps as u64 * match features.complexity_level.as_str() {
            "simple" => 30,
            "medium" => 90,
            "complex" => 180,
            _ => 60,
        };
        
        // 根据并行化潜力调整
        match features.parallelization_potential.as_str() {
            "high" => base_time / 2,
            "medium" => (base_time * 3) / 4,
            _ => base_time,
        }
    }

    /// 记录执行历史
    async fn record_execution(
        &self,
        query: &str,
        features: &QueryFeatures,
        selection: &ArchitectureSelection,
    ) {
        let record = ExecutionRecord {
            query: query.to_string(),
            features: features.clone(),
            selected_architecture: selection.selected_architecture.clone(),
            execution_time: 0,
            success_rate: 0.0,
            parallel_efficiency: 0.0,
            timestamp: Utc::now(),
        };
        
        let mut history = self.execution_history.write().await;
        history.push(record);
        // 保持最近100条记录
        if history.len() > 100 {
            history.remove(0);
        }
    }

    /// 获取执行状态
    pub async fn get_execution_status(&self, execution_id: &str) -> Result<ExecutionStatus> {
        // 从工作流引擎获取执行状态
        match self.workflow_engine.get_execution_status(execution_id).await {
            Ok(status) => {
                Ok(ExecutionStatus {
                    execution_id: execution_id.to_string(),
                    request_id: execution_id.to_string(), // 简化处理
                    status: format!("{:?}", status.status),
                    progress: status.progress.unwrap_or(0),
                    current_step: status.current_step.unwrap_or("未知".to_string()),
                    completed_steps: status.completed_steps.unwrap_or(0),
                    total_steps: status.total_steps.unwrap_or(0),
                    started_at: status.started_at.to_rfc3339(),
                    completed_at: status.completed_at.map(|t| t.to_rfc3339()),
                    result: status.result.map(|v| v.to_string()),
                    error: status.error,
                })
            }
            Err(e) => Err(anyhow::anyhow!("获取执行状态失败: {}", e))
        }
    }
    
    /// 获取执行历史
    pub async fn get_execution_history(
        &self,
        _user_id: Option<&str>,
        architecture: Option<&str>,
        _status: Option<&str>,
        page: u32,
        page_size: u32,
        _start_time: Option<&str>,
        _end_time: Option<&str>,
    ) -> Result<ExecutionHistoryResult> {
        let history = self.execution_history.read().await;
        
        // 简单的过滤和分页逻辑
        let mut filtered: Vec<_> = history.iter().collect();
        
        // 根据架构过滤
        if let Some(arch) = architecture {
            filtered.retain(|record| record.selected_architecture == arch);
        }
        
        // 分页
        let total = filtered.len() as u32;
        let start = ((page - 1) * page_size) as usize;
        let end = (start + page_size as usize).min(filtered.len());
        
        let records: Vec<ExecutionHistoryRecord> = filtered[start..end].iter().map(|record| {
            ExecutionHistoryRecord {
                request_id: Uuid::new_v4().to_string(),
                execution_id: Uuid::new_v4().to_string(),
                user_input: record.query.clone(),
                architecture: record.selected_architecture.clone(),
                task_type: record.features.task_type.clone(),
                complexity: record.features.complexity_level.clone(),
                status: "completed".to_string(),
                execution_time: record.execution_time,
                success_rate: record.success_rate,
                started_at: record.timestamp.to_rfc3339(),
                completed_at: Some(record.timestamp.to_rfc3339()),
            }
        }).collect();
        
        Ok(ExecutionHistoryResult {
            records,
            total,
            total_pages: (total + page_size - 1) / page_size,
        })
    }
    
    /// 取消执行
    pub async fn cancel_execution(&mut self, execution_id: &str) -> Result<()> {
        // 调用工作流引擎取消执行
        self.workflow_engine.cancel_execution(execution_id).await
            .map_err(|e| anyhow::anyhow!("取消执行失败: {}", e))
    }
    
    /// 获取统计信息
    pub async fn get_statistics(&self) -> Result<DispatcherStats> {
        let history = self.execution_history.read().await;
        
        let total_requests = history.len() as u64;
        let successful_requests = history.iter().filter(|r| r.success_rate > 0.8).count() as u64;
        let failed_requests = total_requests - successful_requests;
        
        let average_execution_time = if !history.is_empty() {
            history.iter().map(|r| r.execution_time as f64).sum::<f64>() / history.len() as f64
        } else {
            0.0
        };
        
        let mut architecture_usage = HashMap::new();
        for record in history.iter() {
            *architecture_usage.entry(record.selected_architecture.clone()).or_insert(0) += 1;
        }
        
        Ok(DispatcherStats {
            total_requests,
            successful_requests,
            failed_requests,
            average_execution_time,
            architecture_usage,
            uptime_seconds: 0, // 简化处理
        })
    }

    /// 获取任务队列统计
    pub async fn get_task_queue_statistics(&self) -> Result<task_queue::QueueStatistics> {
        self.task_queue.get_queue_status().await
    }

    /// 获取负载均衡统计
    pub async fn get_load_balancer_statistics(&self) -> load_balancer::LoadBalancerStatistics {
        self.load_balancer.get_statistics().await
    }

    /// 注册执行节点
    pub async fn register_execution_node(&self, node: ExecutionNode) -> Result<()> {
        self.load_balancer.register_node(node).await
    }

    /// 提交任务到队列
    pub async fn submit_task(&self, task: TaskItem) -> Result<()> {
        self.task_queue.enqueue_task(task).await
    }

    /// 获取下一个任务
    pub async fn get_next_task(&self) -> Option<TaskItem> {
        self.task_queue.dequeue_task().await
    }
}

/// 执行状态结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStatus {
    pub execution_id: String,
    pub request_id: String,
    pub status: String,
    pub progress: u32,
    pub current_step: String,
    pub completed_steps: u32,
    pub total_steps: u32,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub result: Option<String>,
    pub error: Option<String>,
}

/// 执行历史结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionHistoryResult {
    pub records: Vec<ExecutionHistoryRecord>,
    pub total: u32,
    pub total_pages: u32,
}

/// 执行历史记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionHistoryRecord {
    pub request_id: String,
    pub execution_id: String,
    pub user_input: String,
    pub architecture: String,
    pub task_type: String,
    pub complexity: String,
    pub status: String,
    pub execution_time: u64,
    pub success_rate: f32,
    pub started_at: String,
    pub completed_at: Option<String>,
}

/// 调度器统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatcherStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_execution_time: f64,
    pub architecture_usage: HashMap<String, u64>,
    pub uptime_seconds: u64,
}
