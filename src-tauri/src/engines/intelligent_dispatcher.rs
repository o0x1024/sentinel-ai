//! 智能调度器模块
//! 
//! 这是一个LLM驱动的动态架构选择和执行系统，负责：
//! - 智能分析用户查询特征
//! - 动态选择最适合的Agent架构
//! - 生成优化的Prompt模板
//! - 创建和执行智能工作流

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use anyhow::Result;
use log::{info, warn};


use crate::services::ai::AiServiceManager;
use crate::services::mcp::McpService;
use crate::ai_adapter::core::AiAdapterManager;
use serde_json::{Value, Map};

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
    /// 工作流引擎
    workflow_engine: Arc<WorkflowEngine>,
    /// 查询分析器
    query_analyzer: QueryAnalyzer,
    /// 架构选择器
    architecture_selector: ArchitectureSelector,
    /// 动态Prompt生成器
    prompt_generator: DynamicPromptGenerator,
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
        // 创建带有AI服务管理器的查询分析器，以便使用配置的意图分析模型
        use crate::ai_adapter::providers::deepseek::DeepSeekProvider;
        use crate::ai_adapter::types::ProviderConfig;
        
        // 从数据库获取DeepSeek配置
        let ai_config = match ai_service_manager.get_provider_config("deepseek").await {
            Ok(Some(config)) => config,
            Ok(None) => {
                tracing::warn!("No DeepSeek configuration found in database, using default");
                crate::services::ai::AiConfig {
                    provider: "deepseek".to_string(),
                    model: "deepseek-chat".to_string(),
                    api_key: None,
                    api_base: Some("https://api.deepseek.com".to_string()),
                    organization: None,
                    temperature: Some(0.7),
                    max_tokens: Some(4096),
                }
            },
            Err(e) => {
                tracing::error!("Failed to get DeepSeek configuration: {}", e);
                return Err(anyhow::anyhow!("Failed to initialize IntelligentDispatcher: {}", e));
            }
        };
        
        let config = ProviderConfig {
            name: ai_config.provider,
            api_key: ai_config.api_key,
            api_base: ai_config.api_base,
            api_version: None,
            organization: ai_config.organization,
            project: None,
            timeout: None,
            retry_strategy: None,
            extra_headers: None,
            extra_params: None,
        };
        
        let provider = DeepSeekProvider::new(config).unwrap();
        let query_analyzer = QueryAnalyzer::new_with_service_manager(
            Box::new(provider),
            ai_service_manager.clone()
        );
        
        let architecture_selector = ArchitectureSelector::new(ai_service_manager.clone());
        let prompt_generator = DynamicPromptGenerator::new();
        
        Ok(Self {
            ai_service_manager,
            mcp_service,
            registry,
            workflow_engine,
            query_analyzer,
            architecture_selector,
            prompt_generator,
            execution_history: Arc::new(RwLock::new(Vec::new())),
        })
    }
    
    /// 智能处理用户查询
    pub async fn process_query(&mut self, user_input: String) -> Result<DispatchResult> {
        info!("🔍 开始智能分析用户查询: {}", user_input);
        info!("📋 执行步骤: 1.查询分析 -> 2.架构选择 -> 3.Prompt生成 -> 4.工作流创建 -> 5.工作流执行");
        
        // 1. 查询分析阶段
        info!("🔄 步骤1: 开始查询特征分析...");
        let analysis_result = self.query_analyzer.analyze_query(&user_input).await
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
            &user_input,
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
            &user_input
        ).await?;
        info!("✅ 步骤4完成: 工作流创建 - 工作流ID: {}, 步骤数: {}",
              workflow.metadata.id,
              workflow.steps.len());
        
        // 5. 执行工作流
        info!("🔄 步骤5: 开始执行智能工作流...");
        let execution_id = self.workflow_engine.execute_workflow(&workflow, None).await?;
        info!("✅ 步骤5启动: 工作流执行已启动 - 执行ID: {}", execution_id);
        
        // 6. 记录执行历史
        info!("📝 记录执行历史到内存缓存...");
        self.record_execution(&user_input, &query_features, &architecture_selection).await;
        
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
                description: "基于LLMCompiler架构的流式并行执行工作流，包含Planner、Task Fetching Unit、Joiner三个核心组件".to_string(),
                author: Some("IntelligentDispatcher".to_string()),
                tags: vec!["llm-compiler".to_string(), "streaming".to_string(), "parallel".to_string(), "dag".to_string()],
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            steps: vec![
                // 1. Planner模块 - 流式生成任务DAG
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
                        inputs.insert("query_features".to_string(), serde_json::to_value(query_features)?); 
                        inputs.insert("max_parallel_tasks".to_string(), Value::Number(query_features.estimated_steps.into()));
                        inputs.insert("complexity_level".to_string(), Value::String(query_features.complexity_level.clone()));
                        inputs
                    },
                    outputs: {
                        let mut outputs = HashMap::new();
                        outputs.insert("task_dag_stream".to_string(), "task_dag_stream".to_string());
                        outputs.insert("task_dependencies".to_string(), "task_dependencies".to_string());
                        outputs.insert("variable_mappings".to_string(), "variable_mappings".to_string());
                        outputs.insert("execution_graph".to_string(), "execution_graph".to_string());
                        outputs
                    },
                    depends_on: vec![],
                    condition: None,
                    retry: Some(RetryConfig {
                        max_attempts: 3,
                        delay: 5,
                        backoff: BackoffStrategy::Exponential { multiplier: 2.0 },
                        retry_on: vec!["planning_error".to_string(), "dag_generation_error".to_string()],
                    }),
                    timeout: Some(300),
                    parallel: false,
                    config: Some(serde_json::json!({
                        "enable_streaming": true,
                        "dag_max_depth": 10,
                        "enable_variable_substitution": true,
                        "planning_strategy": "streaming_decomposition",
                        "output_parser_mode": "eager"
                    })),
                },
                
                // 2. Task Fetching Unit模块 - 并行调度执行
                WorkflowStep {
                    id: "task_fetching_unit".to_string(),
                    name: "任务获取单元".to_string(),
                    agent_type: "llm_compiler_scheduler".to_string(),
                    action: "schedule_and_execute_tasks".to_string(),
                    inputs: {
                        let mut inputs = HashMap::new();
                        inputs.insert("task_dag_stream".to_string(), Value::String("{{planner.task_dag_stream}}".to_string()));
                        inputs.insert("task_dependencies".to_string(), Value::String("{{planner.task_dependencies}}".to_string()));
                        inputs.insert("variable_mappings".to_string(), Value::String("{{planner.variable_mappings}}".to_string()));
                        inputs.insert("execution_graph".to_string(), Value::String("{{planner.execution_graph}}".to_string()));
                        inputs.insert("custom_prompt".to_string(), Value::String(custom_prompts.executor.clone()));
                        inputs
                    },
                    outputs: {
                        let mut outputs = HashMap::new();
                        outputs.insert("task_results".to_string(), "task_results".to_string());
                        outputs.insert("execution_history".to_string(), "execution_history".to_string());
                        outputs.insert("variable_values".to_string(), "variable_values".to_string());
                        outputs.insert("parallel_execution_stats".to_string(), "parallel_execution_stats".to_string());
                        outputs.insert("dependency_resolution_log".to_string(), "dependency_resolution_log".to_string());
                        outputs
                    },
                    depends_on: vec!["planner".to_string()],
                    condition: None,
                    retry: Some(RetryConfig {
                        max_attempts: 2,
                        delay: 3,
                        backoff: BackoffStrategy::Fixed,
                        retry_on: vec!["task_execution_error".to_string(), "dependency_error".to_string()],
                    }),
                    timeout: Some(1800), // 30分钟执行超时
                    parallel: true, // 核心并行执行模块
                    config: Some(serde_json::json!({
                        "max_concurrent_tasks": if query_features.parallelization_potential == "high" { 8 } else { 4 },
                        "enable_eager_scheduling": true,
                        "dependency_check_interval": 100, // ms
                        "enable_variable_substitution": true,
                        "task_timeout": 300,
                        "enable_result_streaming": true,
                        "scheduler_strategy": "dependency_aware"
                    })),
                },
                
                // 3. Joiner模块 - 动态重规划或完成
                WorkflowStep {
                    id: "joiner".to_string(),
                    name: "连接器".to_string(),
                    agent_type: "llm_compiler_joiner".to_string(),
                    action: "decide_replan_or_complete".to_string(),
                    inputs: {
                        let mut inputs = HashMap::new();
                        inputs.insert("user_request".to_string(), Value::String(user_input.to_string()));
                        inputs.insert("task_results".to_string(), Value::String("{{task_fetching_unit.task_results}}".to_string()));
                        inputs.insert("execution_history".to_string(), Value::String("{{task_fetching_unit.execution_history}}".to_string()));
                        inputs.insert("variable_values".to_string(), Value::String("{{task_fetching_unit.variable_values}}".to_string()));
                        inputs.insert("execution_graph".to_string(), Value::String("{{planner.execution_graph}}".to_string()));
                        inputs.insert("parallel_execution_stats".to_string(), Value::String("{{task_fetching_unit.parallel_execution_stats}}".to_string()));
                        inputs.insert("custom_prompt".to_string(), Value::String(custom_prompts.analyzer.clone().unwrap_or_else(|| "基于执行历史决定是否重规划或完成".to_string())));
                        inputs
                    },
                    outputs: {
                        let mut outputs = HashMap::new();
                        outputs.insert("decision".to_string(), "decision".to_string()); // "complete" or "replan"
                        outputs.insert("final_answer".to_string(), "final_answer".to_string());
                        outputs.insert("replan_instructions".to_string(), "replan_instructions".to_string());
                        outputs.insert("completion_confidence".to_string(), "completion_confidence".to_string());
                        outputs.insert("execution_summary".to_string(), "execution_summary".to_string());
                        outputs
                    },
                    depends_on: vec!["task_fetching_unit".to_string()],
                    condition: None,
                    retry: Some(RetryConfig {
                        max_attempts: 2,
                        delay: 2,
                        backoff: BackoffStrategy::Fixed,
                        retry_on: vec!["decision_error".to_string(), "completion_error".to_string()],
                    }),
                    timeout: Some(300),
                    parallel: false,
                    config: Some(serde_json::json!({
                        "completion_threshold": 0.8,
                        "enable_dynamic_replanning": true,
                        "max_replan_iterations": 3,
                        "decision_strategy": "graph_history_based",
                        "enable_confidence_scoring": true
                    })),
                },
                
                // 4. 条件重规划步骤（可选）
                WorkflowStep {
                    id: "replan".to_string(),
                    name: "重新规划".to_string(),
                    agent_type: "llm_compiler_planner".to_string(),
                    action: "replan_from_progress".to_string(),
                    inputs: {
                        let mut inputs = HashMap::new();
                        inputs.insert("user_request".to_string(), Value::String(user_input.to_string()));
                        inputs.insert("replan_instructions".to_string(), Value::String("{{joiner.replan_instructions}}".to_string()));
                        inputs.insert("execution_history".to_string(), Value::String("{{task_fetching_unit.execution_history}}".to_string()));
                        inputs.insert("variable_values".to_string(), Value::String("{{task_fetching_unit.variable_values}}".to_string()));
                        inputs.insert("custom_prompt".to_string(), Value::String(custom_prompts.planner.clone()));
                        inputs
                    },
                    outputs: {
                        let mut outputs = HashMap::new();
                        outputs.insert("updated_task_dag".to_string(), "updated_task_dag".to_string());
                        outputs.insert("new_dependencies".to_string(), "new_dependencies".to_string());
                        outputs.insert("replan_success".to_string(), "replan_success".to_string());
                        outputs
                    },
                    depends_on: vec!["joiner".to_string()],
                    condition: Some("{{joiner.decision}} == 'replan'".to_string()),
                    retry: Some(RetryConfig {
                        max_attempts: 2,
                        delay: 3,
                        backoff: BackoffStrategy::Fixed,
                        retry_on: vec!["replan_error".to_string()],
                    }),
                    timeout: Some(300),
                    parallel: false,
                    config: Some(serde_json::json!({
                        "enable_incremental_planning": true,
                        "preserve_completed_tasks": true,
                        "replan_strategy": "progressive_enhancement"
                    })),
                },
            ],
            variables: {
                let mut variables = HashMap::new();
                variables.insert("global_execution_state".to_string(), Value::Object(serde_json::Map::new()));
                variables.insert("task_variable_registry".to_string(), Value::Object(serde_json::Map::new()));
                variables.insert("replan_counter".to_string(), Value::Number(0.into()));
                variables
            },
            error_handling: Some(ErrorHandling {
                default_strategy: ErrorStrategy::Retry,
                step_strategies: {
                    let mut strategies = HashMap::new();
                    strategies.insert("task_fetching_unit".to_string(), ErrorStrategy::Continue);
                    strategies.insert("joiner".to_string(), ErrorStrategy::Retry);
                    strategies
                },
                on_error: Some("graceful_degradation_with_partial_results".to_string()),
            }),
            notifications: Some(NotificationConfig {
                on_success: vec![
                    NotificationTarget {
                        target_type: "workflow_completion".to_string(),
                        config: serde_json::json!({
                            "message": "LLMCompiler工作流执行成功",
                            "channel": "info",
                            "include_stats": true
                        }),
                    }
                ],
                on_failure: vec![
                    NotificationTarget {
                        target_type: "error_analysis".to_string(),
                        config: serde_json::json!({
                            "message": "LLMCompiler工作流执行失败",
                            "channel": "error",
                            "log_file": "llm_compiler_errors.log",
                            "include_dag_state": true
                        }),
                    }
                ],
                on_progress: vec![
                    NotificationTarget {
                        target_type: "parallel_progress".to_string(),
                        config: serde_json::json!({
                            "message": "LLMCompiler并行执行进度",
                            "channel": "progress",
                            "update_interval": 5000
                        }),
                    }
                ],
            }),
        };
        
        Ok(workflow)
    }
    
    /// 创建ReWOO工作流
    async fn create_rewoo_workflow(
        &self,
        query_features: &QueryFeatures,
        custom_prompts: &DynamicPrompts,
        user_input: &str,
    ) -> Result<WorkflowDefinition> {
        let workflow = WorkflowDefinition {
            metadata: WorkflowMetadata {
                id: Uuid::new_v4().to_string(),
                name: "ReWOO推理工作流".to_string(),
                version: "2.0.0".to_string(),
                description: "基于ReWOO架构的推理工作流，包含Planner、Worker、Solver三个核心模块".to_string(),
                author: Some("IntelligentDispatcher".to_string()),
                tags: vec!["rewoo".to_string(), "reasoning".to_string(), "planner".to_string(), "worker".to_string(), "solver".to_string()],
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            steps: vec![
                // 1. Planner模块 - 生成带变量替换的计划
                WorkflowStep {
                    id: "planner".to_string(),
                    name: "规划器".to_string(),
                    agent_type: "rewoo_planner".to_string(),
                    action: "generate_task_plan".to_string(),
                    inputs: {
                        let mut inputs = HashMap::new();
                        inputs.insert("user_request".to_string(), Value::String(user_input.to_string()));
                        inputs.insert("custom_prompt".to_string(), Value::String(custom_prompts.planner.clone()));
                        inputs.insert("query_features".to_string(), serde_json::to_value(query_features)?);
                        inputs
                    },
                    outputs: {
                        let mut outputs = HashMap::new();
                        outputs.insert("task_list".to_string(), "task_list".to_string());
                        outputs.insert("variable_mappings".to_string(), "variable_mappings".to_string());
                        outputs.insert("execution_plan".to_string(), "execution_plan".to_string());
                        outputs.insert("task_dependencies".to_string(), "task_dependencies".to_string());
                        outputs
                    },
                    depends_on: vec![],
                    condition: None,
                    retry: Some(RetryConfig {
                        max_attempts: 3,
                        delay: 5,
                        backoff: BackoffStrategy::Exponential { multiplier: 2.0 },
                        retry_on: vec!["planning_error".to_string(), "timeout".to_string()],
                    }),
                    timeout: Some(300),
                    parallel: false,
                    config: Some(serde_json::json!({
                        "max_tasks": 10,
                        "enable_variable_substitution": true,
                        "planning_strategy": "decomposition",
                        "task_complexity_threshold": query_features.complexity_level
                    })),
                },
                
                // 2. Worker模块 - 执行工具并收集结果
                WorkflowStep {
                    id: "worker".to_string(),
                    name: "工作器".to_string(),
                    agent_type: "rewoo_worker".to_string(),
                    action: "execute_tasks".to_string(),
                    inputs: {
                        let mut inputs = HashMap::new();
                        inputs.insert("task_list".to_string(), Value::String("{{planner.task_list}}".to_string()));
                        inputs.insert("variable_mappings".to_string(), Value::String("{{planner.variable_mappings}}".to_string()));
                        inputs.insert("execution_plan".to_string(), Value::String("{{planner.execution_plan}}".to_string()));
                        inputs.insert("task_dependencies".to_string(), Value::String("{{planner.task_dependencies}}".to_string()));
                        inputs.insert("custom_prompt".to_string(), Value::String(custom_prompts.executor.clone()));
                        inputs
                    },
                    outputs: {
                        let mut outputs = HashMap::new();
                        outputs.insert("task_results".to_string(), "task_results".to_string());
                        outputs.insert("tool_outputs".to_string(), "tool_outputs".to_string());
                        outputs.insert("execution_status".to_string(), "execution_status".to_string());
                        outputs.insert("variable_values".to_string(), "variable_values".to_string());
                        outputs.insert("intermediate_results".to_string(), "intermediate_results".to_string());
                        outputs
                    },
                    depends_on: vec!["planner".to_string()],
                    condition: None,
                    retry: Some(RetryConfig {
                        max_attempts: 2,
                        delay: 3,
                        backoff: BackoffStrategy::Fixed,
                        retry_on: vec!["tool_error".to_string(), "execution_error".to_string()],
                    }),
                    timeout: Some(1800), // 30分钟执行超时
                    parallel: query_features.parallelization_potential == "high",
                    config: Some(serde_json::json!({
                        "enable_tool_calling": true,
                        "max_concurrent_tasks": if query_features.parallelization_potential == "high" { 5 } else { 2 },
                        "enable_variable_substitution": true,
                        "tool_timeout": 300,
                        "enable_result_caching": true
                    })),
                },
                
                // 3. Solver模块 - 基于工具输出生成最终答案
                WorkflowStep {
                    id: "solver".to_string(),
                    name: "求解器".to_string(),
                    agent_type: "rewoo_solver".to_string(),
                    action: "solve_and_respond".to_string(),
                    inputs: {
                        let mut inputs = HashMap::new();
                        inputs.insert("user_request".to_string(), Value::String(user_input.to_string()));
                        inputs.insert("task_results".to_string(), Value::String("{{worker.task_results}}".to_string()));
                        inputs.insert("tool_outputs".to_string(), Value::String("{{worker.tool_outputs}}".to_string()));
                        inputs.insert("variable_values".to_string(), Value::String("{{worker.variable_values}}".to_string()));
                        inputs.insert("intermediate_results".to_string(), Value::String("{{worker.intermediate_results}}".to_string()));
                        inputs.insert("execution_plan".to_string(), Value::String("{{planner.execution_plan}}".to_string()));
                        inputs.insert("custom_prompt".to_string(), Value::String(custom_prompts.analyzer.clone().unwrap_or_else(|| "基于工具输出生成最终答案".to_string())));
                        inputs
                    },
                    outputs: {
                        let mut outputs = HashMap::new();
                        outputs.insert("final_answer".to_string(), "final_answer".to_string());
                        outputs.insert("reasoning_chain".to_string(), "reasoning_chain".to_string());
                        outputs.insert("confidence_score".to_string(), "confidence_score".to_string());
                        outputs.insert("solution_quality".to_string(), "solution_quality".to_string());
                        outputs.insert("response_metadata".to_string(), "response_metadata".to_string());
                        outputs
                    },
                    depends_on: vec!["worker".to_string()],
                    condition: None,
                    retry: Some(RetryConfig {
                        max_attempts: 2,
                        delay: 2,
                        backoff: BackoffStrategy::Fixed,
                        retry_on: vec!["reasoning_error".to_string(), "synthesis_error".to_string()],
                    }),
                    timeout: Some(300),
                    parallel: false,
                    config: Some(serde_json::json!({
                        "enable_reasoning_validation": true,
                        "confidence_threshold": 0.7,
                        "enable_answer_synthesis": true,
                        "max_reasoning_depth": 5,
                        "enable_quality_assessment": true
                    })),
                },
            ],
            variables: {
                let mut variables = HashMap::new();
                variables.insert("execution_context".to_string(), Value::Object(serde_json::Map::new()));
                variables.insert("global_state".to_string(), Value::Object(serde_json::Map::new()));
                variables
            },
            error_handling: Some(ErrorHandling {
                default_strategy: ErrorStrategy::Retry,
                step_strategies: HashMap::new(),
                on_error: Some("graceful_degradation".to_string()),
            }),
            notifications: Some(NotificationConfig {
                on_success: vec![
                    NotificationTarget {
                        target_type: "workflow_status".to_string(),
                        config: serde_json::json!({
                            "message": "ReWOO工作流执行成功",
                            "channel": "info"
                        }),
                    }
                ],
                on_failure: vec![
                    NotificationTarget {
                        target_type: "error_log".to_string(),
                        config: serde_json::json!({
                            "message": "ReWOO工作流执行失败",
                            "channel": "error",
                            "log_file": "rewoo_errors.log"
                        }),
                    }
                ],
                on_progress: vec![
                    NotificationTarget {
                        target_type: "progress_update".to_string(),
                        config: serde_json::json!({
                            "message": "ReWOO工作流进度更新",
                            "channel": "progress"
                        }),
                    }
                ],
            }),
        };
        
        Ok(workflow)
    }
    
    /// 创建Plan-Execute工作流
    /// 基于LangGraph架构设计，包含planner、agent、replan节点和条件边缘
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
                description: "基于LangGraph架构的计划执行工作流，支持动态重规划".to_string(),
                author: Some("IntelligentDispatcher".to_string()),
                tags: vec!["plan-execute".to_string(), "langgraph".to_string(), "intelligent".to_string()],
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            steps: vec![
                // 1. Planner节点 - 创建初始计划
                WorkflowStep {
                    id: "planner".to_string(),
                    name: "规划器".to_string(),
                    agent_type: "plan_and_execute".to_string(),
                    action: "create_plan".to_string(),
                    inputs: {
                        let mut inputs = HashMap::new();
                        inputs.insert("user_input".to_string(), Value::String(user_input.to_string()));
                        inputs.insert("custom_prompt".to_string(), Value::String(custom_prompts.planner.clone()));
                        inputs.insert("query_features".to_string(), serde_json::to_value(query_features)?);
                        inputs
                    },
                    outputs: {
                        let mut outputs = HashMap::new();
                        outputs.insert("plan".to_string(), "plan".to_string());
                        outputs.insert("plan_steps".to_string(), "plan_steps".to_string());
                        outputs.insert("current_step_index".to_string(), "current_step_index".to_string());
                        outputs
                    },
                    depends_on: vec![],
                    condition: None,
                    retry: Some(RetryConfig {
                        max_attempts: 3,
                        delay: 5,
                        backoff: BackoffStrategy::Exponential { multiplier: 2.0 },
                        retry_on: vec!["timeout".to_string(), "network_error".to_string()],
                    }),
                    timeout: Some(300),
                    parallel: false,
                    config: Some(serde_json::json!({
                        "max_plan_steps": 10,
                        "enable_parallel_planning": query_features.parallelization_potential == "high"
                    })),
                },
                
                // 2. Agent节点 - 执行计划步骤
                WorkflowStep {
                    id: "agent".to_string(),
                    name: "执行代理".to_string(),
                    agent_type: "plan_and_execute".to_string(),
                    action: "execute_step".to_string(),
                    inputs: {
                        let mut inputs = HashMap::new();
                        inputs.insert("plan".to_string(), Value::String("{{planner.plan}}".to_string()));
                        inputs.insert("current_step".to_string(), Value::String("{{planner.plan_steps[planner.current_step_index]}}".to_string()));
                        inputs.insert("execution_context".to_string(), Value::String("{{execution_context}}".to_string()));
                        inputs.insert("custom_prompt".to_string(), Value::String(custom_prompts.executor.clone()));
                        inputs
                    },
                    outputs: {
                        let mut outputs = HashMap::new();
                        outputs.insert("step_result".to_string(), "step_result".to_string());
                        outputs.insert("execution_status".to_string(), "execution_status".to_string());
                        outputs.insert("step_outputs".to_string(), "step_outputs".to_string());
                        outputs.insert("next_step_index".to_string(), "next_step_index".to_string());
                        outputs
                    },
                    depends_on: vec!["planner".to_string()],
                    condition: None,
                    retry: Some(RetryConfig {
                        max_attempts: 2,
                        delay: 3,
                        backoff: BackoffStrategy::Fixed,
                        retry_on: vec!["execution_error".to_string(), "tool_error".to_string()],
                    }),
                    timeout: Some(1800), // 30分钟执行超时
                    parallel: query_features.parallelization_potential == "high",
                    config: Some(serde_json::json!({
                        "enable_tool_calling": true,
                        "max_tool_calls_per_step": 5,
                        "enable_parallel_execution": query_features.parallelization_potential == "high"
                    })),
                },
                
                // 3. Replan节点 - 重新规划和决策
                WorkflowStep {
                    id: "replan".to_string(),
                    name: "重规划器".to_string(),
                    agent_type: "plan_and_execute".to_string(),
                    action: "replan_and_decide".to_string(),
                    inputs: {
                        let mut inputs = HashMap::new();
                        inputs.insert("original_plan".to_string(), Value::String("{{planner.plan}}".to_string()));
                        inputs.insert("execution_results".to_string(), Value::String("{{agent.step_result}}".to_string()));
                        inputs.insert("current_step_index".to_string(), Value::String("{{agent.next_step_index}}".to_string()));
                        inputs.insert("execution_status".to_string(), Value::String("{{agent.execution_status}}".to_string()));
                        inputs.insert("user_input".to_string(), Value::String(user_input.to_string()));
                        inputs.insert("custom_prompt".to_string(), Value::String(custom_prompts.planner.clone()));
                        inputs
                    },
                    outputs: {
                        let mut outputs = HashMap::new();
                        outputs.insert("should_continue".to_string(), "should_continue".to_string());
                        outputs.insert("updated_plan".to_string(), "updated_plan".to_string());
                        outputs.insert("next_action".to_string(), "next_action".to_string());
                        outputs.insert("completion_status".to_string(), "completion_status".to_string());
                        outputs.insert("final_result".to_string(), "final_result".to_string());
                        outputs
                    },
                    depends_on: vec!["agent".to_string()],
                    condition: None,
                    retry: Some(RetryConfig {
                        max_attempts: 2,
                        delay: 2,
                        backoff: BackoffStrategy::Fixed,
                        retry_on: vec!["decision_error".to_string()],
                    }),
                    timeout: Some(180),
                    parallel: false,
                    config: Some(serde_json::json!({
                        "max_replan_attempts": 3,
                        "enable_adaptive_planning": true,
                        "decision_threshold": 0.8
                    })),
                },
                
                // 4. 条件分支节点 - 根据replan结果决定下一步
                WorkflowStep {
                    id: "decision_branch".to_string(),
                    name: "决策分支".to_string(),
                    agent_type: "workflow_controller".to_string(),
                    action: "conditional_branch".to_string(),
                    inputs: {
                        let mut inputs = HashMap::new();
                        inputs.insert("should_continue".to_string(), Value::String("{{replan.should_continue}}".to_string()));
                        inputs.insert("next_action".to_string(), Value::String("{{replan.next_action}}".to_string()));
                        inputs.insert("completion_status".to_string(), Value::String("{{replan.completion_status}}".to_string()));
                        inputs
                    },
                    outputs: {
                        let mut outputs = HashMap::new();
                        outputs.insert("branch_decision".to_string(), "branch_decision".to_string());
                        outputs.insert("workflow_status".to_string(), "workflow_status".to_string());
                        outputs
                    },
                    depends_on: vec!["replan".to_string()],
                    // 条件逻辑：如果should_continue为true，返回到agent节点；否则结束
                    condition: Some("{{replan.should_continue}} == true ? 'continue' : 'end'".to_string()),
                    retry: None,
                    timeout: Some(30),
                    parallel: false,
                    config: Some(serde_json::json!({
                        "continue_target": "agent",
                        "end_target": "__end__",
                        "max_iterations": 20
                    })),
                },
            ],
            variables: {
                let mut variables = HashMap::new();
                variables.insert("max_iterations".to_string(), Value::Number(serde_json::Number::from(20)));
                variables.insert("current_iteration".to_string(), Value::Number(serde_json::Number::from(0)));
                variables.insert("execution_context".to_string(), serde_json::json!({
                    "complexity_level": query_features.complexity_level,
                    "parallelization_potential": query_features.parallelization_potential,
                    "estimated_duration": self.estimate_duration(query_features)
                }));
                variables
            },
            error_handling: Some(ErrorHandling {
                default_strategy: ErrorStrategy::Retry,
                step_strategies: {
                    let mut strategies = HashMap::new();
                    strategies.insert("planner".to_string(), ErrorStrategy::Retry);
                    strategies.insert("agent".to_string(), ErrorStrategy::Continue);
                    strategies.insert("replan".to_string(), ErrorStrategy::Stop);
                    strategies.insert("decision_branch".to_string(), ErrorStrategy::Stop);
                    strategies
                },
                on_error: Some("log_and_notify".to_string()),
            }),
            notifications: Some(NotificationConfig {
                on_success: vec![NotificationTarget {
                    target_type: "log".to_string(),
                    config: serde_json::json!({"level": "info", "message": "Plan-Execute工作流执行成功"}),
                }],
                on_failure: vec![NotificationTarget {
                    target_type: "log".to_string(),
                    config: serde_json::json!({"level": "error", "message": "Plan-Execute工作流执行失败"}),
                }],
                on_progress: vec![NotificationTarget {
                    target_type: "log".to_string(),
                    config: serde_json::json!({"level": "debug", "message": "Plan-Execute工作流执行进度更新"}),
                }],
            }),
        };
        
        Ok(workflow)
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
                    result: status.result,
                    error: status.error,
                })
            }
            Err(e) => Err(anyhow::anyhow!("获取执行状态失败: {}", e))
        }
    }
    
    /// 获取执行历史
    pub async fn get_execution_history(
        &self,
        user_id: Option<&str>,
        architecture: Option<&str>,
        status: Option<&str>,
        page: u32,
        page_size: u32,
        start_time: Option<&str>,
        end_time: Option<&str>,
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
        
        let total_requests = history.len() as u32;
        let successful_requests = history.iter().filter(|r| r.success_rate > 0.8).count() as u32;
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
    pub total_requests: u32,
    pub successful_requests: u32,
    pub failed_requests: u32,
    pub average_execution_time: f64,
    pub architecture_usage: HashMap<String, u32>,
    pub uptime_seconds: u64,
}