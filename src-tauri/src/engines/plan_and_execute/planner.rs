//! Planner 组件 - 任务规划器
//! 
//! 负责将复杂任务分解为可执行的子任务，制定详细的执行计划

use crate::ai_adapter::core::AiAdapterManager;
use crate::models::prompt::{ArchitectureType, StageType};
use crate::services::prompt_db::PromptRepository;
use crate::engines::plan_and_execute::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;
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
}

impl Planner {
    /// 创建新的规划器实例
    pub fn new(config: PlannerConfig, prompt_repo: Option<PromptRepository>) -> Result<Self, PlanAndExecuteError> {
        let ai_manager = AiAdapterManager::global();
        
        // 验证AI提供商是否可用
        ai_manager.get_provider(&config.ai_provider)
            .map_err(|e| PlanAndExecuteError::ConfigError(format!("AI provider '{}' not available: {}", config.ai_provider, e)))?;
        
        Ok(Self {
            config,
            ai_manager,
            prompt_repo,
        })
    }

    /// 创建执行计划
    pub async fn create_plan(&self, task: &TaskRequest) -> Result<PlanningResult, PlanAndExecuteError> {
        log::info!("开始为任务 '{}' 创建执行计划", task.name);
        
        // 1. 分析任务复杂度
        let complexity = self.analyze_task_complexity(task).await?;
        
        // 2. 生成初始计划
        let initial_plan = self.generate_initial_plan(task, &complexity).await?;
        
        // 3. 优化计划
        let optimized_plan = self.optimize_plan(initial_plan, task).await?;
        
        // 4. 评估风险
        let risk_assessment = self.assess_risks(&optimized_plan, task).await?;
        
        // 5. 计算资源需求
        let resource_requirements = self.calculate_resource_requirements(&optimized_plan).await?;
        
        // 6. 计算置信度
        let confidence = self.calculate_confidence(&optimized_plan, &risk_assessment).await?;
        
        let result = PlanningResult {
            plan: optimized_plan,
            confidence,
            reasoning: self.generate_reasoning_explanation(task).await?,
            risk_assessment,
            resource_requirements,
        };
        
        log::info!("任务 '{}' 的执行计划创建完成，置信度: {:.2}", task.name, confidence);
        Ok(result)
    }

    /// 分析任务复杂度
    async fn analyze_task_complexity(&self, task: &TaskRequest) -> Result<TaskComplexity, PlanAndExecuteError> {
        let prompt = self.build_complexity_analysis_prompt(task).await?;
        
        let provider = self.ai_manager.get_provider(&self.config.ai_provider)
            .map_err(|e| PlanAndExecuteError::AiAdapterError(e.to_string()))?;
        
        let request = self.build_ai_request(&prompt)?;
        let response = provider.send_chat_request(&request).await
            .map_err(|e| PlanAndExecuteError::AiAdapterError(e.to_string()))?;
        
        let content = match &response.message.content {
            text => text,
        };
        self.parse_complexity_response(content)
    }

    /// 生成初始计划
    async fn generate_initial_plan(&self, task: &TaskRequest, complexity: &TaskComplexity) -> Result<ExecutionPlan, PlanAndExecuteError> {
        let prompt = self.build_planning_prompt(task, complexity).await?;
        
        let provider = self.ai_manager.get_provider(&self.config.ai_provider)
            .map_err(|e| PlanAndExecuteError::AiAdapterError(e.to_string()))?;
        
        let request = self.build_ai_request(&prompt)?;
        let response = provider.send_chat_request(&request).await
            .map_err(|e| PlanAndExecuteError::AiAdapterError(e.to_string()))?;
        
        let content = match &response.message.content {
            text => text,
        };
        self.parse_plan_response(content, task)
    }

    /// 优化计划
    async fn optimize_plan(&self, plan: ExecutionPlan, task: &TaskRequest) -> Result<ExecutionPlan, PlanAndExecuteError> {
        match self.config.planning_strategy {
            PlanningStrategy::Sequential => self.optimize_sequential_plan(plan).await,
            PlanningStrategy::Hierarchical => self.optimize_hierarchical_plan(plan).await,
            PlanningStrategy::Parallel => self.optimize_parallel_plan(plan).await,
            PlanningStrategy::Adaptive => self.optimize_adaptive_plan(plan, task).await,
        }
    }

    /// 评估风险
    async fn assess_risks(&self, plan: &ExecutionPlan, task: &TaskRequest) -> Result<RiskAssessment, PlanAndExecuteError> {
        let prompt = self.build_risk_assessment_prompt(plan, task).await?;
        
        let provider = self.ai_manager.get_provider(&self.config.ai_provider)
            .map_err(|e| PlanAndExecuteError::AiAdapterError(e.to_string()))?;
        
        let request = self.build_ai_request(&prompt)?;
        let response = provider.send_chat_request(&request).await
            .map_err(|e| PlanAndExecuteError::AiAdapterError(e.to_string()))?;
        
        let content = match &response.message.content {
            text => text,
        };
        self.parse_risk_assessment_response(content)
    }

    /// 计算资源需求
    async fn calculate_resource_requirements(&self, plan: &ExecutionPlan) -> Result<ResourceRequirements, PlanAndExecuteError> {
        let mut total_time = 0;
        let mut required_tools = Vec::new();
        let mut memory_mb = 512; // 基础内存需求
        let cpu_cores = 2; // 默认CPU需求
        let network_mbps = 10; // 默认网络需求
        
        for step in &plan.steps {
            total_time += step.estimated_duration;
            
            if let Some(tool_config) = &step.tool_config {
                if !required_tools.contains(&tool_config.tool_name) {
                    required_tools.push(tool_config.tool_name.clone());
                }
            }
            
            // 根据步骤类型调整资源需求
            match step.step_type {
                StepType::ToolCall => memory_mb += 128,
                StepType::AiReasoning => memory_mb += 256,
                StepType::DataProcessing => memory_mb += 64,
                StepType::Parallel => memory_mb += 512,
                _ => {},
            }
        }
        
        Ok(ResourceRequirements {
            estimated_time: total_time,
            required_tools,
            memory_mb,
            cpu_cores,
            network_mbps,
        })
    }

    /// 计算置信度
    async fn calculate_confidence(&self, plan: &ExecutionPlan, risk_assessment: &RiskAssessment) -> Result<f32, PlanAndExecuteError> {
        let mut confidence = 1.0;
        
        // 根据计划复杂度调整置信度
        let complexity_factor = 1.0 - (plan.steps.len() as f32 * 0.02).min(0.3);
        confidence *= complexity_factor;
        
        // 根据风险等级调整置信度
        let risk_factor = match risk_assessment.overall_risk {
            RiskLevel::Low => 0.95,
            RiskLevel::Medium => 0.85,
            RiskLevel::High => 0.70,
            RiskLevel::Critical => 0.50,
        };
        confidence *= risk_factor;
        
        // 确保置信度在合理范围内
        Ok(confidence.max(0.1).min(1.0))
    }

    /// 生成推理解释
    async fn generate_reasoning_explanation(&self, task: &TaskRequest) -> Result<String, PlanAndExecuteError> {
        Ok(format!(
            "基于任务类型 '{:?}' 和目标 '{}' 的分析，采用 '{:?}' 策略进行规划。\
             考虑了任务复杂度、资源需求和风险因素，制定了分步执行计划。",
            task.task_type, task.target.address, self.config.planning_strategy
        ))
    }

    // 辅助方法
    async fn build_complexity_analysis_prompt(&self, task: &TaskRequest) -> Result<String, PlanAndExecuteError> {
        let base = format!(
            "分析以下任务的复杂度：\n\
             任务类型: {:?}\n\
             目标: {}\n\
             描述: {}\n\
             请评估任务的复杂度等级（简单/中等/复杂/非常复杂）并说明原因。",
            task.task_type, task.target.address, task.description
        );
        self.decorate_with_dynamic_prompt(StageType::Planning, base).await
    }

    async fn build_planning_prompt(&self, task: &TaskRequest, complexity: &TaskComplexity) -> Result<String, PlanAndExecuteError> {
        let base = format!(
            "为以下任务制定详细的执行计划：\n\
             任务: {}\n\
             类型: {:?}\n\
             目标: {}\n\
             复杂度: {:?}\n\
             请分解为具体的执行步骤，包括工具调用、参数配置等。",
            task.name, task.task_type, task.target.address, complexity
        );
        self.decorate_with_dynamic_prompt(StageType::Planning, base).await
    }

    async fn build_risk_assessment_prompt(&self, plan: &ExecutionPlan, task: &TaskRequest) -> Result<String, PlanAndExecuteError> {
        let base = format!(
            "评估以下执行计划的风险：\n\
             任务: {}\n\
             步骤数: {}\n\
             目标: {}\n\
             请识别潜在风险并提供缓解策略。",
            task.name, plan.steps.len(), task.target.address
        );
        self.decorate_with_dynamic_prompt(StageType::Reflection, base).await
    }

    async fn decorate_with_dynamic_prompt(&self, stage: StageType, base: String) -> Result<String, PlanAndExecuteError> {
        if let Some(repo) = &self.prompt_repo {
            match repo.get_active_prompt(ArchitectureType::PlanExecute, stage).await {
                Ok(Some(dynamic)) => Ok(dynamic),
                _ => Ok(base),
            }
        } else {
            Ok(base)
        }
    }

    fn build_ai_request(&self, prompt: &str) -> Result<crate::ai_adapter::types::ChatRequest, PlanAndExecuteError> {
        use crate::ai_adapter::types::{ChatRequest, Message, MessageRole, MessageContent};
        
        use crate::ai_adapter::types::ChatOptions;
        
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

    // 解析响应的方法（简化实现）
    fn parse_complexity_response(&self, content: &str) -> Result<TaskComplexity, PlanAndExecuteError> {
        // 简化的解析逻辑，实际应该使用更复杂的NLP解析
        if content.to_lowercase().contains("简单") {
            Ok(TaskComplexity::Simple)
        } else if content.to_lowercase().contains("复杂") {
            Ok(TaskComplexity::Complex)
        } else {
            Ok(TaskComplexity::Medium)
        }
    }

    fn parse_plan_response(&self, content: &str, task: &TaskRequest) -> Result<ExecutionPlan, PlanAndExecuteError> {
        // 简化的解析逻辑，实际应该解析AI返回的结构化计划
        let steps = self.generate_default_steps_for_task(task)?;
        
        Ok(ExecutionPlan {
            id: Uuid::new_v4().to_string(),
            task_id: task.id.clone(),
            name: format!("{} - 执行计划", task.name),
            description: format!("为任务 '{}' 生成的执行计划", task.name),
            steps,
            estimated_duration: 1800, // 30分钟默认
            created_at: SystemTime::now(),
            dependencies: HashMap::new(),
            metadata: HashMap::new(),
        })
    }

    fn parse_risk_assessment_response(&self, content: &str) -> Result<RiskAssessment, PlanAndExecuteError> {
        // 简化的风险评估解析
        Ok(RiskAssessment {
            overall_risk: RiskLevel::Medium,
            risk_items: vec![
                RiskItem {
                    description: "网络连接可能不稳定".to_string(),
                    level: RiskLevel::Low,
                    impact: "可能导致工具调用失败".to_string(),
                    probability: 0.2,
                }
            ],
            mitigation_strategies: vec![
                "配置重试机制".to_string(),
                "设置合理的超时时间".to_string(),
            ],
        })
    }

    fn generate_default_steps_for_task(&self, task: &TaskRequest) -> Result<Vec<ExecutionStep>, PlanAndExecuteError> {
        let mut steps = Vec::new();
        
        match task.task_type {
            TaskType::SecurityScan => {
                steps.push(self.create_step("信息收集", "收集目标基本信息", StepType::ToolCall, Some("nmap".to_string()))?);
                steps.push(self.create_step("漏洞扫描", "执行漏洞扫描", StepType::ToolCall, Some("nuclei".to_string()))?);
                steps.push(self.create_step("结果分析", "分析扫描结果", StepType::AiReasoning, None)?);
            },
            TaskType::AssetDiscovery => {
                steps.push(self.create_step("子域名发现", "发现子域名", StepType::ToolCall, Some("subfinder".to_string()))?);
                steps.push(self.create_step("端口扫描", "扫描开放端口", StepType::ToolCall, Some("nmap".to_string()))?);
                steps.push(self.create_step("服务识别", "识别运行的服务", StepType::ToolCall, Some("whatweb".to_string()))?);
            },
            _ => {
                steps.push(self.create_step("默认步骤", "执行默认任务", StepType::ToolCall, None)?);
            }
        }
        
        Ok(steps)
    }

    fn create_step(&self, name: &str, description: &str, step_type: StepType, tool_name: Option<String>) -> Result<ExecutionStep, PlanAndExecuteError> {
        let tool_config = tool_name.map(|name| ToolConfig {
            tool_name: name,
            tool_version: None,
            tool_args: HashMap::new(),
            timeout: Some(300), // 5分钟默认超时
            env_vars: HashMap::new(),
        });
        
        Ok(ExecutionStep {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: description.to_string(),
            step_type,
            tool_config,
            parameters: HashMap::new(),
            estimated_duration: 300, // 5分钟默认
            retry_config: RetryConfig::default(),
            preconditions: Vec::new(),
            postconditions: Vec::new(),
        })
    }

    // 优化策略实现（简化版本）
    async fn optimize_sequential_plan(&self, plan: ExecutionPlan) -> Result<ExecutionPlan, PlanAndExecuteError> {
        // 顺序优化：确保步骤按依赖关系排序
        Ok(plan)
    }

    async fn optimize_hierarchical_plan(&self, plan: ExecutionPlan) -> Result<ExecutionPlan, PlanAndExecuteError> {
        // 分层优化：将步骤分组为不同层级
        Ok(plan)
    }

    async fn optimize_parallel_plan(&self, plan: ExecutionPlan) -> Result<ExecutionPlan, PlanAndExecuteError> {
        // 并行优化：识别可以并行执行的步骤
        Ok(plan)
    }

    async fn optimize_adaptive_plan(&self, plan: ExecutionPlan, _task: &TaskRequest) -> Result<ExecutionPlan, PlanAndExecuteError> {
        // 自适应优化：根据任务特性动态调整
        Ok(plan)
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