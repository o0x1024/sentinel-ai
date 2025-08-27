//! Planner 组件 - 通用任务规划器
//! 
//! 基于 Prompt 驱动的通用规划逻辑，不针对特定任务类型做特殊处理

use crate::ai_adapter::core::AiAdapterManager;
use crate::ai_adapter::types::{ChatRequest, Message, MessageRole, ChatOptions};
use crate::services::prompt_db::PromptRepository;
use crate::services::mcp::McpService;
use crate::services::ai::{AiServiceManager, SchedulerStage};
use crate::models::prompt::{ArchitectureType, StageType};
use crate::engines::plan_and_execute::types::*;
use crate::tools::{ToolInfo};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::SystemTime;
use std::sync::Arc;

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
    ai_manager: &'static AiAdapterManager,
    prompt_repo: Option<PromptRepository>,
    framework_adapter: Option<Arc<dyn crate::tools::FrameworkToolAdapter>>,
    mcp_service: Option<Arc<McpService>>,
    ai_service_manager: Option<Arc<AiServiceManager>>,
}

impl Planner {
    /// 创建新的规划器实例
    pub fn new(config: PlannerConfig, prompt_repo: Option<PromptRepository>) -> Result<Self, PlanAndExecuteError> {
        let ai_manager = AiAdapterManager::global();
        
        // 验证AI提供商是否可用（仅在非空时校验；空由调度器/默认回退处理）
        if !config.ai_provider.is_empty() {
            if let Err(e) = ai_manager.get_provider_or_default(&config.ai_provider) {
                log::warn!(
                    "AI provider '{}' not registered at init: {}. Will resolve at runtime via scheduler or fallback.",
                    config.ai_provider, e
                );
            }
        }
        
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
            ai_manager,
            prompt_repo,
            framework_adapter,
            mcp_service: None,
            ai_service_manager: None,
        })
    }

    /// 创建带有MCP服务的规划器实例
    pub fn with_mcp_service(
        config: PlannerConfig, 
        prompt_repo: Option<PromptRepository>,
        mcp_service: Option<Arc<McpService>>
    ) -> Result<Self, PlanAndExecuteError> {
        let ai_manager = AiAdapterManager::global();
        
        // 验证AI提供商是否可用（仅在非空时校验；空由调度器/默认回退处理）
        if !config.ai_provider.is_empty() {
            if let Err(e) = ai_manager.get_provider_or_default(&config.ai_provider) {
                log::warn!(
                    "AI provider '{}' not registered at init: {}. Will resolve at runtime via scheduler or fallback.",
                    config.ai_provider, e
                );
            }
        }
        
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
            ai_manager,
            prompt_repo,
            framework_adapter,
            mcp_service,
            ai_service_manager: None,
        })
    }

    /// 创建带有AI服务管理器的规划器实例（用于动态模型切换）
    pub fn with_ai_service_manager(
        config: PlannerConfig, 
        prompt_repo: Option<PromptRepository>,
        mcp_service: Option<Arc<McpService>>,
        ai_service_manager: Arc<AiServiceManager>,
    ) -> Result<Self, PlanAndExecuteError> {
        let ai_manager = AiAdapterManager::global();
        
        // 验证AI提供商是否可用（仅在非空时校验；空由调度器/默认回退处理）
        if !config.ai_provider.is_empty() {
            if let Err(e) = ai_manager.get_provider(&config.ai_provider) {
                log::warn!(
                    "AI provider '{}' not registered at init: {}. Will resolve at runtime via scheduler or fallback.",
                    config.ai_provider, e
                );
            }
        }
        
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
            ai_manager,
            prompt_repo,
            framework_adapter,
            mcp_service,
            ai_service_manager: Some(ai_service_manager),
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
        
        // 1. 构建通用规划 Prompt
        let planning_prompt = self.build_generic_planning_prompt(task).await?;
        
        // 2. 调用 AI 进行规划
        let planning_response = self.call_ai_for_planning(&planning_prompt).await?;
        
        // 3. 解析规划结果
        let plan = self.parse_planning_response(&planning_response)?;
        
        // 4. 计算基本指标
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
        Ok(result)
    }

    /// 构建通用规划 Prompt
    async fn build_generic_planning_prompt(&self, task: &TaskRequest) -> Result<String, PlanAndExecuteError> {
        // 从数据库获取 Prompt 模板，或者使用默认模板
        let prompt_template = if let Some(repo) = &self.prompt_repo {
            repo.get_active_prompt(ArchitectureType::PlanExecute, StageType::Planning)
                .await
                .unwrap_or_else(|_| None)
                .unwrap_or_else(|| self.get_default_planning_template())
        } else {
            self.get_default_planning_template()
        };

        // 获取可用工具信息
        let tools_block = self.build_tools_information().await;

        // 构建具体的 Prompt
        let target_info = if let Some(target) = &task.target {
            format!("目标: {} ({})", target.identifier, format!("{:?}", target.target_type))
        } else {
            "无特定目标".to_string()
        };

        let parameters_str = if !task.parameters.is_empty() {
            serde_json::to_string_pretty(&task.parameters)
                .unwrap_or_else(|_| "无参数".to_string())
        } else {
            "无参数".to_string()
        };

        let prompt = prompt_template
            .replace("{task_name}", &task.name)
            .replace("{task_description}", &task.description)
            .replace("{task_type}", &format!("{:?}", task.task_type))
            .replace("{target_info}", &target_info)
            .replace("{parameters}", &parameters_str)
            .replace("{priority}", &format!("{:?}", task.priority))
            .replace("{tools}", &tools_block);

        Ok(prompt)
    }

    /// 获取默认规划模板
    fn get_default_planning_template(&self) -> String {
        r#"你是一个Plan-and-Execute规划器的战略层(Planner)。你的任务是将复杂问题分解为清晰的可执行步骤。

Plan-and-Execute架构流程：
1. Planner（战略层）：分析任务，生成初始计划
2. Agent（执行层）：选择工具，执行具体步骤
3. Tools（工具层）：调用具体工具，返回结果
4. Replan（反思层）：评估结果，决定是否需要新计划

任务信息：
任务名称: {task_name}
任务描述: {task_description}
任务类型: {task_type}
{target_info}
优先级: {priority}
参数: {parameters}

可用工具（名称与参数签名）：
{tools}

**重要规划原则**：
- 每个步骤应该是原子操作（一个步骤完成一个明确的子任务）
- 步骤之间应该有逻辑依赖关系
- 考虑可能的失败情况和重新规划需求
- 优先使用ToolCall步骤来获取外部数据
- 使用AiReasoning步骤来分析和综合信息

严格按以下JSON格式返回计划：
{
  "steps": [
    {
      "name": "步骤名称",
      "description": "清晰描述这一步要做什么，为什么需要这一步",
      "step_type": "ToolCall|AiReasoning|DataProcessing|Conditional|Wait",
      "tool": {
        "name": "工具名称",
        "args": { "参数名": "参数值" }
      }
    }
  ]
}

步骤类型说明：
- ToolCall: 调用外部工具获取数据，必须提供tool字段
- AiReasoning: AI分析推理，用于处理获取的数据
- DataProcessing: 数据处理和转换
- Conditional: 条件判断
- Wait: 等待一段时间

注意：
1. 每个ToolCall步骤必须指定具体的tool.name和tool.args
2. 工具参数必须与工具签名匹配
3. 步骤数量建议2-6个，保持合理
4. 只返回JSON，不要其他文本
5. 确保步骤逻辑顺序正确"#.to_string()
    }

    /// 构建工具信息块
    async fn build_tools_information(&self) -> String {
        let mut all_tools: Vec<ToolInfo> = Vec::new();
        
        // 从框架适配器获取工具
        if let Some(framework_adapter) = &self.framework_adapter {
            let available_tools = framework_adapter.list_available_tools().await;
            for tool_name in available_tools {
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
            "(no tools available)".to_string()
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

    /// 调用 AI 进行规划
    async fn call_ai_for_planning(&self, prompt: &str) -> Result<String, PlanAndExecuteError> {
        // 尝试获取调度策略配置的规划模型
        let ai_config = self.get_planning_ai_config().await?;
        
        let (provider_name, model_name) = if let Some(config) = ai_config {
            log::info!("使用调度策略配置的规划模型: {} ({})", config.model, config.provider);
            (config.provider, config.model)
        } else {
            log::info!("使用默认规划模型: {} ({})", self.config.model_config.model_name, self.config.ai_provider);
            (self.config.ai_provider.clone(), self.config.model_config.model_name.clone())
        };
        
        let provider = self.ai_manager.get_provider_or_default(&provider_name)
            .map_err(|e| PlanAndExecuteError::AiAdapterError(e.to_string()))?;
        
        let request = self.build_ai_request_with_model(prompt, &model_name)?;
        
        // 使用流式响应并收集结果
        let mut stream = provider.send_chat_stream(&request).await
            .map_err(|e| PlanAndExecuteError::AiAdapterError(e.to_string()))?;

        // 流式响应处理状态
        let mut content = String::new();
        let mut response_id = String::new();
        

        // 收集流式响应 - 参考 AiService 中的实现
        use futures::StreamExt;
        while let Some(chunk_result) = stream.stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    //需要处理报错 EOF while parsing a value at line 1 column 0    
                    if !chunk.content.is_empty() {
                        let choice = serde_json::from_str::<serde_json::Value>(&chunk.content)?;
                        // 取content的值  INFO sentinel_ai_lib::engines::intelligent_dispatcher::query_analyzer: 107: choice: Object {"choices": Array [Object {"delta": Object {"content": String(""), "role": String("assistant")}, "finish_reason": Null, "index": Number(0)}], "created": Number(1756283183), "id": String("chatcmpl-68aec12f1e91f778fae1cc59"), "model": String("moonshot-v1-8k"), "object": String("chat.completion.chunk"), "system_fingerprint": String("fpv0_ff52a3ef")}
                        let content_str = choice["choices"][0]["delta"]["content"].as_str().unwrap_or("");
                        content.push_str(content_str);
                    }
                }
                Err(e) => return Err(PlanAndExecuteError::AiAdapterError(e.to_string())),
            }
        }
        
        Ok(content)
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
        log::info!("开始解析规划响应JSON");
        let json_str = Self::extract_json_string(response)
            .ok_or_else(|| PlanAndExecuteError::PlanningFailed("No JSON content found in response".to_string()))?;

        log::info!("提取的JSON字符串: {}", json_str);

        let json: Value = serde_json::from_str(&json_str)
            .map_err(|e| PlanAndExecuteError::PlanningFailed(format!("Invalid JSON plan: {}", e)))?;
        
        log::info!("JSON解析成功，开始解析steps数组");

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

        let plan = ExecutionPlan {
            id: Uuid::new_v4().to_string(),
            task_id: Uuid::new_v4().to_string(),
            name: json.get("name").and_then(|v| v.as_str()).unwrap_or("Generated Plan").to_string(),
            description: json.get("description").and_then(|v| v.as_str()).unwrap_or("AI generated execution plan").to_string(),
            steps: built_steps,
            estimated_duration: json.get("estimated_duration").and_then(|v| v.as_u64()).unwrap_or(60 *  (json.get("steps").and_then(|s| s.as_array()).map(|a| a.len()).unwrap_or(1) as u64)),
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

    /// 构建 AI 请求（支持自定义模型）
    fn build_ai_request_with_model(&self, prompt: &str, model_name: &str) -> Result<ChatRequest, PlanAndExecuteError> {
        let request = ChatRequest {
            model: model_name.to_string(),
            messages: vec![
                Message {
                    role: MessageRole::User,
                    content: prompt.to_string(),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                }
            ],
            options: Some(ChatOptions {
                temperature: Some(self.config.model_config.temperature),
                max_tokens: Some(self.config.model_config.max_tokens),
                top_p: Some(self.config.model_config.top_p),
                ..Default::default()
            }),
            tools: None,
            tool_choice: None,
            user: None,
            extra_params: None,
        };
        
        log::debug!("构建AI请求，模型: {}, 温度: {}", 
                   model_name, self.config.model_config.temperature);
        Ok(request)
    }

    /// 构建 AI 请求
    fn build_ai_request(&self, prompt: &str) -> Result<ChatRequest, PlanAndExecuteError> {
        Ok(ChatRequest {
            model: self.config.model_config.model_name.clone(),
            messages: vec![Message {
                role: MessageRole::User,
                content: prompt.to_string(),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            }],
            tools: None,
            tool_choice: None,
            user: None,
            extra_params: None,
            options: Some(ChatOptions {
                temperature: Some(self.config.model_config.temperature),
                max_tokens: Some(self.config.model_config.max_tokens),
                top_p: Some(self.config.model_config.top_p),
                frequency_penalty: None,
                presence_penalty: None,
                stop: None,
                stream: Some(false),
            }),
        })
    }



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