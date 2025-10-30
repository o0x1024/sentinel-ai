//! Planner 组件 - 通用任务规划器
//! 
//! 基于 Prompt 驱动的通用规划逻辑，不针对特定任务类型做特殊处理

use crate::services::prompt_db::PromptRepository;
use crate::services::mcp::McpService;
use crate::services::ai::{AiService, AiServiceManager, SchedulerStage};
use crate::engines::plan_and_execute::types::*;
use crate::tools::{ToolInfo};
use crate::utils::ordered_message::ChunkType;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Manager};
use std::collections::HashMap;
use std::time::SystemTime;
use std::sync::Arc;
use sentinel_rag::models::AssistantRagRequest;


use uuid::Uuid;

/// 规划器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerConfig {
    /// AI提供商名称
    pub ai_provider: String,
    /// 规划模型配置
    pub model_config: ModelConfig,
    /// 规划策略
    pub planning_strategy: PlanningStrategy,
    /// 最大规划深度
    pub max_planning_depth: u32,
    /// 规划超时时间（秒）
    pub planning_timeout: u64,
    /// 是否启用并行规划
    pub enable_parallel_planning: bool,
}

/// 模型配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// 模型名称
    pub model_name: String,
    /// 温度参数
    pub temperature: f32,
    /// 最大token数
    pub max_tokens: u32,
    /// top_p参数
    pub top_p: f32,
}

/// 规划策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlanningStrategy {
    /// 顺序规划
    Sequential,
    /// 分层规划
    Hierarchical,
    /// 并行规划
    Parallel,
    /// 自适应规划
    Adaptive,
}

/// 规划结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningResult {
    /// 执行计划
    pub plan: ExecutionPlan,
    /// 规划置信度 (0.0-1.0)
    pub confidence: f32,
    /// 规划推理过程
    pub reasoning: String,
    /// 风险评估
    pub risk_assessment: RiskAssessment,
    /// 资源需求
    pub resource_requirements: ResourceRequirements,
}

/// 风险评估
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    /// 整体风险等级
    pub overall_risk: RiskLevel,
    /// 具体风险项
    pub risk_items: Vec<RiskItem>,
    /// 缓解措施
    pub mitigation_strategies: Vec<String>,
}

/// 风险等级
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// 风险项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskItem {
    /// 风险描述
    pub description: String,
    /// 风险等级
    pub level: RiskLevel,
    /// 影响范围
    pub impact: String,
    /// 发生概率
    pub probability: f32,
}

/// 资源需求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// 预估执行时间（秒）
    pub estimated_time: u64,
    /// 所需工具列表
    pub required_tools: Vec<String>,
    /// 内存需求（MB）
    pub memory_mb: u64,
    /// CPU需求（核心数）
    pub cpu_cores: u32,
    /// 网络带宽需求（Mbps）
    pub network_mbps: u32,
}

/// 计划器
#[derive(Debug)]
pub struct Planner {
    config: PlannerConfig,
    prompt_repo: Option<PromptRepository>,
    framework_adapter: Option<Arc<dyn crate::tools::FrameworkToolAdapter>>,
    mcp_service: Option<Arc<McpService>>,
    ai_service_manager: Option<Arc<AiServiceManager>>,
    app_handle: Option<Arc<AppHandle>>, // 用于向前端发送计划阶段消息
}

impl Planner {
    /// 创建新的规划器实例
    pub fn new(config: PlannerConfig, prompt_repo: Option<PromptRepository>) -> Result<Self, PlanAndExecuteError> {
        // 尝试获取Plan & Execute框架适配器
        let framework_adapter = match crate::tools::get_global_adapter_manager() {
            Ok(adapter_manager) => {
                let adapter = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        adapter_manager.get_framework_adapter(crate::tools::FrameworkType::PlanAndExecute).await
                    })
                });
                Some(adapter)
            },
            Err(e) => {
                log::warn!("Failed to get framework adapter: {}", e);
                None
            }
        };
        
        Ok(Self {
            config,
            prompt_repo,
            framework_adapter,
            mcp_service: None,
            ai_service_manager: None,
            app_handle: None,
        })
    }

    /// 创建带有MCP服务的规划器实例
    pub fn with_mcp_service(
        config: PlannerConfig, 
        prompt_repo: Option<PromptRepository>,
        mcp_service: Option<Arc<McpService>>
    ) -> Result<Self, PlanAndExecuteError> {
        // 尝试获取Plan & Execute框架适配器
        let framework_adapter = match crate::tools::get_global_adapter_manager() {
            Ok(adapter_manager) => {
                let adapter = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        adapter_manager.get_framework_adapter(crate::tools::FrameworkType::PlanAndExecute).await
                    })
                });
                Some(adapter)
            },
            Err(e) => {
                log::warn!("Failed to get framework adapter: {}", e);
                None
            }
        };
        
        Ok(Self {
            config,
            prompt_repo,
            framework_adapter,
            mcp_service,
            ai_service_manager: None,
            app_handle: None,
        })
    }

    /// 创建带有AI服务管理器的规划器实例（用于动态模型切换）
    pub fn with_ai_service_manager(
        config: PlannerConfig, 
        prompt_repo: Option<PromptRepository>,
        mcp_service: Option<Arc<McpService>>,
        ai_service_manager: Arc<AiServiceManager>,
        app_handle: Option<Arc<AppHandle>>,
    ) -> Result<Self, PlanAndExecuteError> {
        // 尝试获取Plan & Execute框架适配器
        let framework_adapter = match crate::tools::get_global_adapter_manager() {
            Ok(_adapter_manager) => {
                let adapter = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        crate::tools::get_framework_adapter(crate::tools::FrameworkType::PlanAndExecute).await
                    })
                });
                match adapter {
                    Ok(adapter) => Some(adapter),
                    Err(e) => {
                        log::warn!("Failed to get framework adapter: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                log::warn!("Failed to get framework adapter: {}", e);
                None
            }
        };
        
        Ok(Self {
            config,
            prompt_repo,
            framework_adapter,
            mcp_service,
            ai_service_manager: Some(ai_service_manager),
            app_handle,
        })
    }

    /// 获取当前规划阶段应使用的AI配置
    async fn get_planning_ai_config(&self) -> Result<Option<crate::services::ai::AiConfig>, PlanAndExecuteError> {
        if let Some(ref ai_service_manager) = self.ai_service_manager {
            match ai_service_manager.get_ai_config_for_stage(SchedulerStage::Planning).await {
                Ok(config) => Ok(config),
                Err(e) => {
                    log::warn!("Failed to get AI config for planning stage: {}, using default", e);
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    /// 对外暴露：根据调度阶段获取AI配置
    pub async fn get_ai_config_for_stage(&self, stage: SchedulerStage) -> Result<Option<crate::services::ai::AiConfig>, PlanAndExecuteError> {
        if let Some(ref ai_service_manager) = self.ai_service_manager {
            match ai_service_manager.get_ai_config_for_stage(stage.clone()).await {
                Ok(config) => Ok(config),
                Err(e) => {
                    log::warn!("Failed to get AI config for stage {:?}: {}", stage, e);
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    /// 对外暴露：获取本地配置中的备用 provider 与 model
    pub fn get_fallback_provider_and_model(&self) -> (String, String) {
        (
            self.config.ai_provider.clone(),
            self.config.model_config.model_name.clone(),
        )
    }

    /// 创建执行计划 - 基于通用 Prompt 模板
    pub async fn create_plan(&self, task: &TaskRequest) -> Result<PlanningResult, PlanAndExecuteError> {
        log::info!("开始为任务 '{}' 创建执行计划", task.name);

        // 1. 构建规划所需的 system 与 user 提示词
        let (system_prompt, user_prompt) = self.build_generic_planning_prompts(task).await?;

        // 2. 从任务参数中获取前端传入的会话/消息ID/执行ID
        let conversation_id = task
            .parameters
            .get("conversation_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let message_id = task
            .parameters
            .get("message_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let execution_id = task
            .parameters
            .get("execution_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| message_id.clone());

        // 3. 调用AI生成计划，失败时最多重试2次（合计3次尝试）
        let mut last_err: Option<PlanAndExecuteError> = None;
        for attempt in 1..=3 {
            match self
                .call_ai_for_planning(&system_prompt, &user_prompt, conversation_id.clone(), message_id.clone(), task)
                .await
            {
                Ok(planning_response) => {
                    match self.parse_planning_response(&planning_response) {
                        Ok(plan) => {
                            // 计算基本指标
                            let confidence = self.calculate_basic_confidence(&plan).await?;
                            let risk_assessment = self.assess_generic_risks(&plan).await?;
                            let resource_requirements = self.calculate_basic_resources(&plan).await?;
                            tracing::info!("plan:{:?}", plan);

                            let result = PlanningResult {
                                plan,
                                confidence,
                                reasoning: planning_response.clone(),
                                risk_assessment,
                                resource_requirements,
                            };
                            log::info!("任务 '{}' 的执行计划创建完成，置信度: {:.2}", task.name, confidence);
                            return Ok(result);
                        }
                        Err(e) => {
                            log::warn!("规划结果解析失败(尝试 {}/3): {}", attempt, e);
                            last_err = Some(e);
                            self.emit_planning_content(&execution_id, &message_id, conversation_id.as_deref(), &format!(
                                "计划生成解析失败(第{}次): {}",
                                attempt, last_err.as_ref().unwrap()
                            ));
                        }
                    }
                }
                Err(e) => {
                    log::warn!("调用AI生成计划失败(尝试 {}/3): {}", attempt, e);
                    last_err = Some(e);
                    self.emit_planning_content(&execution_id, &message_id, conversation_id.as_deref(), &format!(
                        "计划生成失败(第{}次): {}",
                        attempt, last_err.as_ref().unwrap()
                    ));
                }
            }

            if attempt < 3 {
                // 简单固定退避：500ms * attempt
                tokio::time::sleep(std::time::Duration::from_millis(500 * attempt)).await;
            }
        }

        // 最终失败：向前端输出错误消息并返回一个包含错误信息的计划
        let error_message = if let Some(err) = &last_err {
            format!("计划生成失败: {}", err)
        } else {
            "计划生成失败: 未知错误".to_string()
        };
        
        log::warn!("规划最终失败，返回错误消息: {}", error_message);
        
        // 发送错误消息到前端
        self.emit_planning_error_final(&execution_id, &message_id, conversation_id.as_deref(), &error_message);
        
        // 返回一个包含错误信息的计划，而不是抛出错误
        let error_plan = ExecutionPlan {
            id: format!("error_plan_{}", chrono::Utc::now().timestamp()),
            task_id: task.id.clone(),
            name: "错误计划".to_string(),
            description: error_message.clone(),
            steps: vec![],
            estimated_duration: 0,
            created_at: SystemTime::now(),
            dependencies: HashMap::new(),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("error".to_string(), serde_json::Value::String(error_message.clone()));
                meta.insert("is_error_plan".to_string(), serde_json::Value::Bool(true));
                meta
            },
        };
        
        Ok(PlanningResult {
            plan: error_plan,
            confidence: 0.0,
            reasoning: error_message.clone(),
            risk_assessment: RiskAssessment {
                overall_risk: RiskLevel::High,
                risk_items: vec![RiskItem {
                    description: "计划生成失败".to_string(),
                    level: RiskLevel::High,
                    impact: "无法执行任务".to_string(),
                    probability: 1.0,
                }],
                mitigation_strategies: vec!["请检查输入参数并重试".to_string()],
            },
            resource_requirements: ResourceRequirements {
                estimated_time: 0,
                required_tools: vec![],
                memory_mb: 0,
                cpu_cores: 0,
                network_mbps: 0,
            },
        })
    }

    fn emit_planning_content(&self, execution_id: &Option<String>, message_id: &Option<String>, conversation_id: Option<&str>, msg: &str) {
        if let (Some(app), Some(exec_id), Some(msg_id)) = (&self.app_handle, execution_id.as_ref(), message_id.as_ref()) {
            crate::utils::ordered_message::emit_message_chunk_arc(
                app,
                exec_id,
                msg_id,
                conversation_id,
                crate::utils::ordered_message::ChunkType::Content,
                msg,
                false,
                Some("planner"),
                None,
            );
        }
    }

    fn emit_planning_error_final(&self, execution_id: &Option<String>, message_id: &Option<String>, conversation_id: Option<&str>, msg: &str) {
        if let (Some(app), Some(exec_id), Some(msg_id)) = (&self.app_handle, execution_id.as_ref(), message_id.as_ref()) {
            crate::utils::ordered_message::emit_message_chunk_arc(
                app,
                exec_id,
                msg_id,
                conversation_id,
                crate::utils::ordered_message::ChunkType::Error,
                msg,
                false,
                Some("planner"),
                None,
            );
        }
    }

    /// 构建通用规划的 system 与 user 提示词（使用统一提示词系统）
    async fn build_generic_planning_prompts(&self, task: &TaskRequest) -> Result<(String, String), PlanAndExecuteError> {
        use crate::utils::prompt_resolver::{PromptResolver, CanonicalStage, AgentPromptConfig};
        use crate::models::prompt::ArchitectureType;
        use std::collections::HashMap;

        // 获取可用工具信息（放入 system 提示），支持根据Agent工具策略过滤
        let tools_block = self.build_tools_information(task).await;

        // 解析并获取 system 模板（仅渲染通用变量，避免注入用户任务内容）
        let system_template = if let Some(repo) = &self.prompt_repo {
            let resolver = PromptResolver::new(repo.clone());
            let agent_config = AgentPromptConfig::parse_agent_config(&task.parameters);
            resolver
                .resolve_prompt(
                    &agent_config,
                    ArchitectureType::PlanExecute,
                    CanonicalStage::Planner,
                    Some(&"".to_string()),
                )
                .await
                .unwrap_or_else(|_| "".to_string())
        } else {
           "".to_string()
        };

        // 仅渲染通用上下文（如工具清单）到 system 提示
        let mut system_ctx = HashMap::new();
        system_ctx.insert("tools".to_string(), serde_json::Value::String(tools_block));
        let mut system_prompt = if let Some(repo) = &self.prompt_repo {
            let resolver = PromptResolver::new(repo.clone());
            resolver
                .render_variables(&system_template, &system_ctx)
                .unwrap_or(system_template)
        } else {
            system_template.replace("{tools}", system_ctx.get("tools").unwrap().as_str().unwrap())
        };

        // 集成角色提示词（如果存在）
        if let Some(role_prompt) = task.parameters.get("role_prompt").and_then(|v| v.as_str()) {
            if !role_prompt.trim().is_empty() {
                system_prompt = if system_prompt.trim().is_empty() {
                    role_prompt.to_string()
                } else {
                    format!("{}\n\n{}", role_prompt, system_prompt)
                };
                log::info!("Plan-Execute planner: integrated role prompt");
            }
        }

        let user_prompt = format!(
            "{}",
            task.name.clone()
        );

        // RAG augmentation for planning stage (global toggle)
        if let Ok(rag_service) = crate::commands::rag_commands::get_global_rag_service().await {
            if rag_service.get_config().augmentation_enabled {
                use tokio::time::{timeout, Duration};
                
                // 获取激活的集合ID，与AI助手模式保持一致（通过 AppHandle 获取 DatabaseService）
                let active_collection_id: Option<String> = if let Some(ref app_handle) = self.app_handle {
                    let db_service = app_handle.state::<Arc<crate::services::database::DatabaseService>>();
                    match db_service.get_rag_collections().await {
                        Ok(cols) => cols.into_iter().find(|c| c.is_active).map(|c| c.id),
                        Err(_) => None,
                    }
                } else {
                    None
                };
                
                let rag_request = AssistantRagRequest {
                    query: task.name.clone(),
                    collection_id: active_collection_id,
                    conversation_history: None,
                    top_k: Some(5),
                    use_mmr: Some(true),
                    mmr_lambda: Some(0.7),
                    similarity_threshold: Some(0.65),
                    reranking_enabled: Some(false),
                    model_provider: None,
                    model_name: None,
                    max_tokens: None,
                    temperature: None,
                    system_prompt: None,                };
                if let Ok(Ok((knowledge_context, _))) = timeout(
                    Duration::from_millis(1200),
                    rag_service.query_for_assistant(&rag_request),
                )
                .await
                {
                    if !knowledge_context.trim().is_empty() {
                        log::info!("Augmenting planner system prompt with RAG context from active collection");
                        system_prompt.push_str("\n\n[KNOWLEDGE CONTEXT]\n");
                        system_prompt.push_str(&knowledge_context);
                    } else {
                        log::info!("No relevant knowledge found in active collection for planning");
                    }
                }
            }
        }

        Ok((system_prompt, user_prompt))
    }

    /// 构建工具信息块
    async fn build_tools_information(&self, task: &TaskRequest) -> String {
        use std::collections::HashSet;
        // 读取Agent传入的工具白名单/黑名单
        let (allow, deny): (HashSet<String>, HashSet<String>) = {
            let params = &task.parameters;
            let allow = params
                .get("tools_allow")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_else(HashSet::new);
            let deny = params
                .get("tools_deny")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_else(HashSet::new);
            (allow, deny)
        };
        let mut all_tools: Vec<ToolInfo> = Vec::new();
        
        // 从框架适配器获取工具
        if let Some(framework_adapter) = &self.framework_adapter {
            let available_tools = framework_adapter.list_available_tools().await;
            for tool_name in available_tools {
                // 过滤白名单/黑名单
                // 如果有白名单且工具不在白名单中，跳过
                if !allow.is_empty() && !allow.contains(&tool_name) {
                    continue;
                }
                // 如果没有白名单（空数组），则不允许任何工具
                if allow.is_empty() {
                    continue;
                }
                // 如果工具在黑名单中，跳过
                if deny.contains(&tool_name) {
                    continue;
                }
                if let Some(tool_info) = framework_adapter.get_tool_info(&tool_name).await {
                    all_tools.push(tool_info);
                }
            }
        }
        
        // 框架适配器已经包含了所有工具（内置工具 + MCP工具）
        // 不需要单独处理MCP工具，因为它们已经通过全局工具系统集成到框架适配器中
        log::info!("所有工具（包括MCP工具）已通过框架适配器统一获取");
        
        // 去重工具（按名称）
        let mut unique_tools: HashMap<String, ToolInfo> = HashMap::new();
        for tool in all_tools {
            unique_tools.entry(tool.name.clone()).or_insert(tool);
        }
        
        let tool_infos: Vec<&ToolInfo> = unique_tools.values().collect();
        
        if tool_infos.is_empty() {
            log::warn!("没有找到任何可用工具");
            "没有找到任何可用工具".to_string()
        } else {
            log::info!("构建工具信息，共 {} 个工具", tool_infos.len());
            let mut tool_lines: Vec<String> = Vec::new();
            for info in &tool_infos {
                // 构建工具参数签名
                let mut parts: Vec<String> = Vec::new();
                for param in &info.parameters.parameters {
                    let param_type = match param.param_type {
                        crate::tools::ParameterType::String => "string",
                        crate::tools::ParameterType::Number => "number",
                        crate::tools::ParameterType::Boolean => "boolean",
                        crate::tools::ParameterType::Array => "array",
                        crate::tools::ParameterType::Object => "object",
                    };
                    let param_str = if param.required {
                        format!("{}: {}", param.name, param_type)
                    } else {
                        format!("{}?: {}", param.name, param_type)
                    };
                    parts.push(param_str);
                }
                
                let signature = if parts.is_empty() {
                    String::new()
                } else {
                    parts.join(", ")
                };
                
                tool_lines.push(format!("- {}({}) - {}", 
                    info.name, signature, info.description));
            }
            tool_lines.join("\n")
        }
    }

    /// 调用 AI 进行规划（区分 system 与 user）
    async fn call_ai_for_planning(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        conversation_id: Option<String>, 
        message_id: Option<String>,
        task: &TaskRequest,
    ) -> Result<String, PlanAndExecuteError> {
        // 尝试获取模型配置配置的规划模型
        let ai_config = self.get_planning_ai_config().await?;
        
        let (provider_name, model_name) = if let Some(config) = ai_config {
            log::info!("使用模型配置配置的规划模型: {} ( {})", config.model, config.provider);
            (config.provider, config.model)
        } else {
            // 允许Agent传入覆盖（llm.default）
            if let Some(params) = task.parameters.get("llm").and_then(|v| v.get("default")) {
                let provider_str = params.get("provider").and_then(|v| v.as_str()).unwrap_or("");
                let model_str = params.get("model").and_then(|v| v.as_str()).unwrap_or("");
                
                // 跳过 "auto" 配置，让调度器配置生效
                if model_str != "auto" && !model_str.trim().is_empty() {
                    let p = if provider_str != "auto" && !provider_str.trim().is_empty() {
                        Some(provider_str.to_string())
                    } else {
                        None
                    };
                    log::info!("使用Agent覆盖规划模型: {} ({:?})", model_str, p);
                    (p.unwrap_or_else(|| self.config.ai_provider.clone()), model_str.to_string())
                } else {
                    log::info!("Agent LLM config is 'auto' or empty, using scheduler/default config");
                    if self.config.model_config.model_name.trim().is_empty() {
                        return Err(PlanAndExecuteError::ConfigError(
                            "无法找到可用的规划器模型配置。请在调度器设置中配置规划器模型，或在Agent配置中设置LLM覆盖。".to_string()
                        ));
                    }
                    (self.config.ai_provider.clone(), self.config.model_config.model_name.clone())
                }
            } else {
                log::info!("使用默认规划模型: {} ({})", self.config.model_config.model_name, self.config.ai_provider);
                if self.config.model_config.model_name.trim().is_empty() {
                    return Err(PlanAndExecuteError::ConfigError(
                        "无法找到可用的规划器模型配置。请在调度器设置中配置规划器模型，或在Agent配置中设置LLM覆盖。".to_string()
                    ));
                }
                (self.config.ai_provider.clone(), self.config.model_config.model_name.clone())
            }
        };
        
        // 获取AI服务：直接按配置构建一次性服务（使用 Rig）
        let ai_service = if let Some(ref ai_service_manager) = self.ai_service_manager {
            match ai_service_manager.get_provider_config(&provider_name).await {
                Ok(Some(cfg)) => {
                    let mut dc = cfg;
                    dc.model = model_name.clone();
                    let app_handle = self.app_handle.as_ref().map(|a| a.as_ref().clone());
                    // 构建时使用 AI Service Manager 的底层 DB
                    if let Some(ref mgr) = self.ai_service_manager {
                        AiService::new(dc, mgr.get_db_arc(), app_handle, self.mcp_service.clone())
                    } else {
                        return Err(PlanAndExecuteError::AiAdapterError("AI服务管理器未初始化".to_string()));
                    }
                }
                Ok(None) => {
                    return Err(PlanAndExecuteError::AiAdapterError(format!(
                        "找不到提供商配置: {}", provider_name
                    )));
                }
                Err(e) => {
                    return Err(PlanAndExecuteError::AiAdapterError(format!(
                        "读取提供商配置失败: {}", e
                    )));
                }
            }
        } else {
            return Err(PlanAndExecuteError::AiAdapterError(
                "AI服务管理器未初始化".to_string()
            ));
        };
        
        // 使用流式消息API发送请求（分别传递 system 与 user 提示），并显式传入前端提供的 IDs
        let result = ai_service
            .send_message_stream(
                Some(user_prompt),       // 用户提示
                Some(system_prompt),     // 系统提示
                conversation_id,         // 前端会话ID
                message_id,              // 前端消息ID
                false,
                false,
                Some(ChunkType::PlanInfo),
            )
            .await
            .map_err(|e| PlanAndExecuteError::AiAdapterError(e.to_string()))?;
        
        log::info!("AI响应内容: {}", result);
        Ok(result)
    }

    /// 解析规划响应
    fn parse_planning_response(&self, response: &str) -> Result<ExecutionPlan, PlanAndExecuteError> {
        // 优先尝试JSON解析（包含可执行的工具信息）
        if let Ok(plan) = self.parse_planning_response_json(response) {
            return Ok(plan);
        }
        Err(PlanAndExecuteError::PlanningFailed("No JSON content found in response".to_string()))
    }

    /// JSON解析：从LLM返回的结构化计划中构建可执行计划
    fn parse_planning_response_json(&self, response: &str) -> Result<ExecutionPlan, PlanAndExecuteError> {
        let json_str = Self::extract_json_string(response)
            .ok_or_else(|| PlanAndExecuteError::PlanningFailed("No JSON content found in response".to_string()))?;


        let json: Value = serde_json::from_str(&json_str)
            .map_err(|e| PlanAndExecuteError::PlanningFailed(format!("Invalid JSON plan: {}", e)))?;
        

        let steps_val = json.get("steps").ok_or_else(||
            PlanAndExecuteError::PlanningFailed("Missing 'steps' field in JSON plan".to_string())
        )?;

        let steps_arr = steps_val.as_array().ok_or_else(||
            PlanAndExecuteError::PlanningFailed("Field 'steps' must be an array".to_string())
        )?;

        let mut built_steps: Vec<ExecutionStep> = Vec::new();
        for (i, step) in steps_arr.iter().enumerate() {
            let name = step.get("name").and_then(|v| v.as_str()).unwrap_or(&format!("Step {}", i + 1)).to_string();
            let description = step.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let step_type_str = step.get("step_type").and_then(|v| v.as_str()).unwrap_or("AiReasoning");
            let mut step_type = match step_type_str {
                "ToolCall" => StepType::ToolCall,
                "AiReasoning" => StepType::AiReasoning,
                "DataProcessing" => StepType::DataProcessing,
                "Conditional" => StepType::Conditional,
                "Parallel" => StepType::Parallel,
                "Wait" => StepType::Wait,
                "ManualConfirmation" => StepType::ManualConfirmation,
                _ => StepType::AiReasoning,
            };

            // 读取可选工具信息
            let mut tool_config: Option<ToolConfig> = None;
            log::debug!("步骤 '{}' 的tool字段: {:?}", name, step.get("tool"));
            if let Some(tool_obj) = step.get("tool").and_then(|v| v.as_object()) {
                log::debug!("解析步骤 '{}' 的工具配置: {:?}", name, tool_obj);
                log::debug!("工具对象的name字段: {:?}", tool_obj.get("name"));
                if let Some(tool_name) = tool_obj.get("name").and_then(|v| v.as_str()) {
                    // 解析args对象
                    let args_map = tool_obj.get("args").and_then(|v| v.as_object()).cloned().unwrap_or_default();
                    let mut tool_args: HashMap<String, serde_json::Value> = HashMap::new();
                    for (k, v) in args_map.into_iter() { tool_args.insert(k, v); }
                    tool_config = Some(ToolConfig {
                        tool_name: tool_name.to_string(),
                        tool_version: None,
                        tool_args,
                        timeout: Some(300),
                        env_vars: HashMap::new(),
                    });
                    log::debug!("成功为步骤 '{}' 创建工具配置: {}", name, tool_name);

                    // 只有在非Wait类型且包含真实工具时才强制为ToolCall
                    if !matches!(step_type, StepType::ToolCall | StepType::Wait) {
                        log::debug!("步骤 '{}' 类型从 {:?} 强制更改为 ToolCall", name, step_type);
                        step_type = StepType::ToolCall;
                    }
                    
                    // 如果是Wait类型但包含Wait工具配置，则清除工具配置
                    if matches!(step_type, StepType::Wait) && tool_name == "Wait" {
                        log::debug!("步骤 '{}' 是Wait类型，清除工具配置，使用内置Wait处理", name);
                        tool_config = None;
                    }
                } else {
                    log::debug!("步骤 '{}' 的工具对象没有name字段: {:?}", name, tool_obj);
                }
            } else {
                log::debug!("步骤 '{}' 没有工具配置或工具不是对象类型", name);
            }
            
            log::info!("步骤 '{}': 类型={:?}, 工具配置={}", name, step_type, if tool_config.is_some() { "有" } else { "无" });

            built_steps.push(ExecutionStep {
                id: format!("step_{}", i + 1),
                name,
                description,
                step_type,
                tool_config,
                parameters: HashMap::new(),
                estimated_duration: step.get("estimated_duration").and_then(|v| v.as_u64()).unwrap_or(60),
                retry_config: RetryConfig::default(),
                preconditions: Vec::new(),
                postconditions: Vec::new(),
            });
        }

        if built_steps.is_empty() {
            return Err(PlanAndExecuteError::PlanningFailed("No steps parsed from JSON plan".to_string()));
        }

        // 如果计划的最后一步不是 AiReasoning，则追加一个 AiReasoning 步骤
        if let Some(last) = built_steps.last() {
            if !matches!(last.step_type, StepType::AiReasoning) {
                let next_idx = built_steps.len() + 1;
                log::info!(
                    "最后一步不是AiReasoning，自动追加总结步骤: step_{}",
                    next_idx
                );
                built_steps.push(ExecutionStep {
                    id: format!("step_{}", next_idx),
                    name: "总结".to_string(),
                    description: "用最简短的文字和语言总结一下".to_string(),
                    step_type: StepType::AiReasoning,
                    tool_config: None,
                    parameters: HashMap::new(),
                    estimated_duration: 60,
                    retry_config: RetryConfig::default(),
                    preconditions: Vec::new(),
                    postconditions: Vec::new(),
                });
            }
        }

        let plan = ExecutionPlan {
            id: Uuid::new_v4().to_string(),
            task_id: Uuid::new_v4().to_string(),
            name: json.get("name").and_then(|v| v.as_str()).unwrap_or("生成计划").to_string(),
            description: json.get("description").and_then(|v| v.as_str()).unwrap_or("AI generated execution plan").to_string(),
            steps: built_steps.clone(),
            // 若JSON未提供estimated_duration，则按最终步骤数量估算（包含可能追加的最终AiReasoning步骤）
            estimated_duration: json
                .get("estimated_duration")
                .and_then(|v| v.as_u64())
                .unwrap_or(60 * (built_steps.len() as u64)),
            created_at: SystemTime::now(),
            dependencies: HashMap::new(),
            metadata: HashMap::new(),
        };

        // 验证创建的计划中每个步骤的工具配置
        log::info!("验证创建的ExecutionPlan:");
        for step in &plan.steps {
            log::info!("  步骤 '{}': 类型={:?}, 工具配置={}", 
                      step.name, step.step_type, 
                      if let Some(ref config) = step.tool_config { 
                          format!("有({})", config.tool_name) 
                      } else { 
                          "无".to_string() 
                      });
        }

        Ok(plan)
    }

    /// 从响应文本中提取JSON字符串（支持```json代码块或首尾大括号）
    fn extract_json_string(response: &str) -> Option<String> {
        // 优先匹配```json代码块
        if let Some(start_idx) = response.find("```json") {
            let rest = &response[start_idx + 7..];
            if let Some(end_idx) = rest.find("```") {
                let block = &rest[..end_idx];
                let trimmed = block.trim();
                // 再次裁剪到首个{ 与末尾}
                if let (Some(s), Some(e)) = (trimmed.find('{'), trimmed.rfind('}')) {
                    if e > s { return Some(trimmed[s..=e].to_string()); }
                }
                return Some(trimmed.to_string());
            }
        }
        // 次选：任意```代码块
        if let Some(start_idx) = response.find("```") {
            let rest = &response[start_idx + 3..];
            if let Some(end_idx) = rest.find("```") {
                let block = &rest[..end_idx];
                let trimmed = block.trim();
                if let (Some(s), Some(e)) = (trimmed.find('{'), trimmed.rfind('}')) {
                    if e > s { return Some(trimmed[s..=e].to_string()); }
                }
                return Some(trimmed.to_string());
            }
        }
        // 备用方案：扫描首个{ 和最后一个}
        if let (Some(s), Some(e)) = (response.find('{'), response.rfind('}')) {
            if e > s {
                return Some(response[s..=e].to_string());
            }
        }
        None
    }

    /// 从响应中提取步骤
    fn extract_steps_from_response(&self, response: &str) -> Result<Vec<String>, PlanAndExecuteError> {
        let mut steps = Vec::new();
        
        for line in response.lines() {
            let trimmed = line.trim();
            // 匹配形如 "1. 步骤描述" 的行
            if let Some(captures) = regex::Regex::new(r"^\d+\.\s*(.+)$")
                .unwrap()
                .captures(trimmed) {
                if let Some(step_desc) = captures.get(1) {
                    steps.push(step_desc.as_str().to_string());
                }
            }
        }
        
        if steps.is_empty() {
            // 如果没有找到编号格式，尝试按行分割
            for line in response.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && trimmed.len() > 10 { // 过滤太短的行
                    steps.push(trimmed.to_string());
                }
            }
        }
        
        if steps.is_empty() {
            return Err(PlanAndExecuteError::PlanningFailed(
                "Unable to extract steps from AI response".to_string()
            ));
        }
        
        Ok(steps)
    }

    /// 计算基本置信度
    async fn calculate_basic_confidence(&self, plan: &ExecutionPlan) -> Result<f32, PlanAndExecuteError> {
        // 简单的置信度计算：基于步骤数量和描述质量
        let step_count = plan.steps.len() as f32;
        let mut confidence: f32 = 0.5; // 基础置信度
        
        // 步骤数量合理性
        if step_count >= 3.0 && step_count <= 10.0 {
            confidence += 0.2;
        } else if step_count > 10.0 {
            confidence -= 0.1;
        }
        
        // 步骤描述质量
        let avg_description_length = plan.steps.iter()
            .map(|s| s.description.len() as f32)
            .sum::<f32>() / step_count;
        
        if avg_description_length > 20.0 {
            confidence += 0.2;
        }
        
        Ok(confidence.min(1.0).max(0.1))
    }

    /// 评估通用风险
    async fn assess_generic_risks(&self, plan: &ExecutionPlan) -> Result<RiskAssessment, PlanAndExecuteError> {
        let risk_level = if plan.steps.len() > 8 {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };

        Ok(RiskAssessment {
            overall_risk: risk_level,
            risk_items: vec![
                RiskItem {
                    description: "Execution complexity".to_string(),
                    level: RiskLevel::Low,
                    impact: "Potential delays in execution".to_string(),
                    probability: 0.3,
                }
            ],
            mitigation_strategies: vec![
                "Monitor execution progress closely".to_string(),
                "Have backup plans for critical steps".to_string(),
            ],
        })
    }

    /// 计算基本资源需求
    async fn calculate_basic_resources(&self, plan: &ExecutionPlan) -> Result<ResourceRequirements, PlanAndExecuteError> {
        Ok(ResourceRequirements {
            estimated_time: plan.estimated_duration,
            required_tools: vec!["ai_reasoning".to_string()],
            memory_mb: 512,
            cpu_cores: 1,
            network_mbps: 10,
        })
    }

    // 这些方法不再需要，因为我们现在使用AiService的send_message_stream方法



}

/// 任务复杂度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskComplexity {
    Simple,
    Medium,
    Complex,
    VeryComplex,
}

impl Default for PlannerConfig {
    fn default() -> Self {
        Self {
            ai_provider: "".to_string(),
            model_config: ModelConfig {
                model_name: "".to_string(),
                temperature: 0.7,
                max_tokens: 4000,
                top_p: 0.9,
            },
            planning_strategy: PlanningStrategy::Adaptive,
            max_planning_depth: 5,
            planning_timeout: 300,
            enable_parallel_planning: true,
        }
    }
}
