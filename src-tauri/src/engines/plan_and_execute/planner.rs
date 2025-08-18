//! Planner 组件 - 通用任务规划器
//! 
//! 基于 Prompt 驱动的通用规划逻辑，不针对特定任务类型做特殊处理

use crate::ai_adapter::core::AiAdapterManager;
use crate::ai_adapter::types::{ChatRequest, Message, MessageRole, ChatOptions};
use crate::services::prompt_db::PromptRepository;
use crate::services::mcp::McpService;
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
}

impl Planner {
    /// 创建新的规划器实例
    pub fn new(config: PlannerConfig, prompt_repo: Option<PromptRepository>) -> Result<Self, PlanAndExecuteError> {
        let ai_manager = AiAdapterManager::global();
        
        // 验证AI提供商是否可用
        ai_manager.get_provider(&config.ai_provider)
            .map_err(|e| PlanAndExecuteError::ConfigError(format!("AI provider '{}' not available: {}", config.ai_provider, e)))?;
        
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
        })
    }

    /// 创建带有MCP服务的规划器实例
    pub fn with_mcp_service(
        config: PlannerConfig, 
        prompt_repo: Option<PromptRepository>,
        mcp_service: Option<Arc<McpService>>
    ) -> Result<Self, PlanAndExecuteError> {
        let ai_manager = AiAdapterManager::global();
        
        // 验证AI提供商是否可用
        ai_manager.get_provider(&config.ai_provider)
            .map_err(|e| PlanAndExecuteError::ConfigError(format!("AI provider '{}' not available: {}", config.ai_provider, e)))?;
        
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
        })
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
        r#"你是一个Plan-and-Execute规划器。基于以下任务信息，生成一个由清晰、可执行步骤组成的计划。

任务名称: {task_name}
任务描述: {task_description}
任务类型: {task_type}
{target_info}
优先级: {priority}
参数: {parameters}

可用工具（名称与参数签名）：
{tools}

重要：请严格以JSON返回，键为"steps"，每个步骤为一个对象，格式如下：
{
  "steps": [
    {
      "name": "步骤名称",
      "description": "做什么、为什么",
      "step_type": "ToolCall|AiReasoning|DataProcessing|Conditional|Parallel|Wait|ManualConfirmation",
      "tool": {
        "name": "可用工具名称",
        "args": { "param1": "value", "param2": 123 }
      }
    }
  ]
}

规则：
- 如需使用工具，必须提供tool.name与tool.args(JSON对象)，参数名需与工具签名匹配；
- 不使用工具时，可省略tool字段或将step_type设置为AiReasoning等；
- 步骤数量建议3-10；
- 仅返回JSON，不要附加其它文本。"#.to_string()
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
        let provider = self.ai_manager.get_provider(&self.config.ai_provider)
            .map_err(|e| PlanAndExecuteError::AiAdapterError(e.to_string()))?;
        
        let request = self.build_ai_request(prompt)?;
        let response = provider.send_chat_request(&request).await
            .map_err(|e| PlanAndExecuteError::AiAdapterError(e.to_string()))?;
        
        Ok(response.message.content.clone())
    }

    /// 解析规划响应
    fn parse_planning_response(&self, response: &str) -> Result<ExecutionPlan, PlanAndExecuteError> {
        // 优先尝试JSON解析（包含可执行的工具信息）
        if let Ok(plan) = self.parse_planning_response_json(response) {
            return Ok(plan);
        }

        // 回退：解析为纯文本步骤
        let steps = self.extract_steps_from_response(response)?;
        let steps_count = steps.len();
        
        let plan = ExecutionPlan {
            id: Uuid::new_v4().to_string(),
            task_id: Uuid::new_v4().to_string(),
            name: "Generated Plan".to_string(),
            description: "AI generated execution plan".to_string(),
            steps: steps.into_iter().enumerate().map(|(i, step_desc)| {
                ExecutionStep {
                    id: format!("step_{}", i + 1),
                    name: format!("Step {}", i + 1),
                    description: step_desc,
                    step_type: StepType::AiReasoning,
                    tool_config: None,
                    parameters: HashMap::new(),
                    estimated_duration: 60,
                    retry_config: RetryConfig::default(),
                    preconditions: Vec::new(),
                    postconditions: Vec::new(),
                }
            }).collect(),
            estimated_duration: (steps_count as u64) * 60,
            created_at: SystemTime::now(),
            dependencies: HashMap::new(),
            metadata: HashMap::new(),
        };
        
        Ok(plan)
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
            let mut step_type_str = step.get("step_type").and_then(|v| v.as_str()).unwrap_or("AiReasoning");
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
            if let Some(tool_obj) = step.get("tool").and_then(|v| v.as_object()) {
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

                    // 如果包含工具但未显式声明为ToolCall，则强制为ToolCall
                    if !matches!(step_type, StepType::ToolCall) {
                        step_type = StepType::ToolCall;
                        step_type_str = "ToolCall";
                    }
                }
            }

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
        // 回退：扫描首个{ 和最后一个}
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
            ai_provider: "deepseek".to_string(),
            model_config: ModelConfig {
                model_name: "deepseek-chat".to_string(),
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