//! Prompt配置管理系统
//! 
//! 实现分层架构的prompt模板系统，支持：
//! - 固定核心框架 + 可配置领域模板 + 动态上下文注入
//! - 用户自定义模板
//! - 智能配置选择
//! - A/B测试框架
use super::*;
use sentinel_core::models::prompt::ArchitectureType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use anyhow::{Result, anyhow};

/// Prompt配置管理器
#[derive(Debug, Clone)]
pub struct PromptConfigManager {
    /// 配置数据
    config: PromptConfig,
    /// 自定义模板缓存
    custom_templates: HashMap<String, CustomTemplate>,
    /// A/B测试配置
    ab_test_configs: HashMap<String, ABTestConfig>,
}

/// Prompt配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PromptConfig {
    /// 代理配置文件
    pub agent_profiles: HashMap<String, AgentProfile>,
    /// 当前代理配置文件
    pub agent_profile: AgentProfile,
    /// 领域模板
    pub domain_templates: HashMap<String, DomainTemplate>,
    /// 当前领域模板
    pub domain_template: DomainTemplate,
    /// 核心框架模板
    pub core_templates: CoreTemplates,
    /// 自定义约束
    pub custom_constraints: HashMap<String, Vec<String>>,
    /// 配置版本
    pub version: String,
}

/// 代理配置文件
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentProfile {
    /// 规划器领域
    pub planner_domain: String,
    /// 执行器专业化
    pub executor_specializations: Vec<String>,
    /// 重规划触发器
    pub replanner_triggers: Vec<String>,
    /// 报告模板
    pub report_template: String,
    /// 自定义配置
    pub custom_config: Option<HashMap<String, serde_json::Value>>,
}

/// 领域模板
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DomainTemplate {
    /// 领域指令
    pub domain_instructions: String,
    /// 典型步骤
    pub typical_steps: Vec<String>,
    /// 评估标准
    pub evaluation_criteria: Vec<String>,
    /// 关键触发器
    pub critical_triggers: Vec<String>,
    /// 调整模式
    pub adjustment_patterns: Vec<AdjustmentPattern>,
}

/// 调整模式
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdjustmentPattern {
    /// 触发条件
    pub trigger: String,
    /// 调整动作
    pub action: String,
    /// 优先级
    pub priority: u32,
}

/// 核心框架模板
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CoreTemplates {
    /// 规划器核心模板
    pub planner_core: String,
    /// 执行器核心模板
    pub executor_core: String,
    /// 重规划器核心模板
    pub replanner_core: String,
    /// 报告生成器核心模板
    pub report_generator_core: String,
}

/// 自定义模板
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CustomTemplate {
    /// 模板ID
    pub id: String,
    /// 模板名称
    pub name: String,
    /// 模板描述
    pub description: String,
    /// 模板内容
    pub content: String,
    /// 模板类型
    pub template_type: TemplateType,
    /// Prompt分类
    pub category: Option<PromptCategory>,
    /// 目标架构（如果是架构特定的）
    pub target_architecture: Option<ArchitectureType>,
    /// 创建者
    pub creator: String,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 版本
    pub version: String,
    /// 标签
    pub tags: Vec<String>,
    /// 使用统计
    pub usage_stats: UsageStats,
    /// 模板变量
    pub variables: Vec<String>,
    /// 模板元数据
    pub metadata: std::collections::HashMap<String, String>,
    /// 是否为系统级模板
    pub is_system: bool,
    /// 优先级
    pub priority: u32,
}

/// 模板类型
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum TemplateType {
    /// 系统提示模板
    SystemPrompt,
    /// 意图分析模板
    IntentClassifier,
    /// 规划器模板
    #[default]
    Planner,
    /// 执行器模板
    Executor,
    /// 重规划器模板
    Replanner,
    /// 报告生成器模板
    ReportGenerator,
    /// 领域特定模板
    Domain(String),
    /// 自定义模板
    Custom,
}

/// Prompt类型 - 区分LLM架构和普通prompt
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PromptCategory {
    /// 系统级提示（跨架构通用）
    System,
    /// LLM架构特定（ReWOO、LLMCompiler、PlanExecute）
    LlmArchitecture(ArchitectureType),
    /// 应用级提示
    Application,
    /// 用户自定义
    UserDefined,
}

// ArchitectureType 从 sentinel-core 导入

/// 使用统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsageStats {
    /// 总使用次数
    pub total_uses: f64,
    /// 使用次数
    pub usage_count: f64,
    /// 成功率
    pub success_rate: f32,
    /// 平均构建时间（毫秒）
    pub avg_build_time_ms: f32,
    /// 平均执行时间
    pub avg_execution_time: f32,
    /// 用户评分
    pub user_rating: f32,
    /// 最后使用时间
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,
    /// 错误次数
    pub error_count: f64,
}

/// 模板元数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TemplateMetadata {
    /// 元数据ID
    pub id: String,
    /// 描述
    pub description: String,
    /// 标签
    pub tags: Vec<String>,
    /// 作者
    pub author: String,
    /// 创建者
    pub creator: String,
    /// 版本
    pub version: String,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 更新时间
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// 使用次数
    pub usage_count: f64,
    /// 使用统计
    pub usage_stats: UsageStats,
    /// 性能指标
    pub performance_metrics: PerformanceMetrics,
    /// 验证规则
    pub validation_rules: ValidationRules,
}

/// 性能指标
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerformanceMetrics {
    /// 平均构建时间（毫秒）
    pub avg_build_time_ms: f64,
    /// 成功率
    pub success_rate: f64,
    /// 错误次数
    pub error_count: f64,
}

/// 验证规则
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ValidationRules {
    /// 最大长度
    pub max_length: Option<usize>,
    /// 必需变量
    pub required_variables: Vec<String>,
    pub(crate) max_template_length: i32,
    pub(crate) forbidden_content: Vec<String>,
}

/// A/B测试配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ABTestConfig {
    /// 测试ID
    pub test_id: String,
    /// 测试名称
    pub name: String,
    /// 变体配置
    pub variants: Vec<TestVariant>,
    /// 流量分配
    pub traffic_allocation: HashMap<String, f32>,
    /// 测试状态
    pub status: TestStatus,
    /// 开始时间
    pub start_time: chrono::DateTime<chrono::Utc>,
    /// 结束时间
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
}

// TestVariant 定义已移至 prompt_ab_test_manager.rs

/// 变体指标
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VariantMetrics {
    /// 转换率
    pub conversion_rate: f32,
    /// 平均响应时间
    pub avg_response_time: f32,
    /// 用户满意度
    pub user_satisfaction: f32,
    /// 错误率
    pub error_rate: f32,
}

/// 测试状态
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum TestStatus {
    /// 草稿
    #[default]
    Draft,
    /// 运行中
    Running,
    /// 暂停
    Paused,
    /// 已完成
    Completed,
    /// 已取消
    Cancelled,
}

/// 配置选择上下文
#[derive(Debug, Clone)]
pub struct ConfigSelectionContext {
    /// 用户输入
    pub user_query: String,
    /// 检测到的领域
    pub detected_domain: Option<String>,
    /// 复杂度评估
    pub complexity_score: f32,
    /// 资源约束
    pub resource_constraints: Option<ResourceConstraints>,
    /// 用户偏好
    pub user_preferences: HashMap<String, serde_json::Value>,
}

/// 资源约束
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceConstraints {
    /// 最大执行时间
    pub max_execution_time: Option<f64>,
    /// 最大内存使用
    pub max_memory_mb: Option<f64>,
    /// 最大并发数
    pub max_concurrency: Option<usize>,
}

impl PromptConfigManager {
    /// 创建新的配置管理器
    pub fn new() -> Self {
        Self {
            config: Self::create_default_config(),
            custom_templates: HashMap::new(),
            ab_test_configs: HashMap::new(),
        }
    }

    /// 从文件加载配置
    pub async fn load_from_file<P: AsRef<Path>>(config_path: P) -> Result<Self> {
        let content = fs::read_to_string(config_path).await?;
        let config: PromptConfig = serde_yaml::from_str(&content)?;
        
        Ok(Self {
            config,
            custom_templates: HashMap::new(),
            ab_test_configs: HashMap::new(),
        })
    }

    /// 保存配置到文件
    pub async fn save_to_file<P: AsRef<Path>>(&self, config_path: P) -> Result<()> {
        let content = serde_yaml::to_string(&self.config)?;
        fs::write(config_path, content).await?;
        Ok(())
    }

    /// 获取最优配置
    pub async fn get_optimal_config(&self, context: &ConfigSelectionContext) -> Result<OptimalConfig> {
        // 1. 领域检测
        let domain = self.detect_domain(&context.user_query).await?
            .or(context.detected_domain.clone())
            .unwrap_or_else(|| "general".to_string());

        // 2. 复杂度评估
        let complexity = self.assess_complexity(&context.user_query).await?;

        // 3. 选择最优配置
        self.select_optimal_config(&domain, complexity, context).await
    }

    /// 检测领域
    async fn detect_domain(&self, query: &str) -> Result<Option<String>> {
        // 多策略领域检测
        let keyword_score = self.keyword_matching(query);
        let intent_score = self.intent_classification(query).await?;
        
        // 集成决策
        self.ensemble_domain_decision(keyword_score, intent_score)
    }

    /// 关键词匹配
    fn keyword_matching(&self, query: &str) -> HashMap<String, f32> {
        let mut scores = HashMap::new();
        let query_lower = query.to_lowercase();

        // 安全测试关键词
        let security_keywords = vec![
            "安全", "漏洞", "扫描", "渗透", "测试", "检测", "评估",
            "security", "vulnerability", "scan", "penetration", "test"
        ];
        
        let security_score = security_keywords.iter()
            .map(|&keyword| if query_lower.contains(keyword) { 1.0 } else { 0.0 })
            .sum::<f32>() / security_keywords.len() as f32;
        scores.insert("security_testing".to_string(), security_score);

        // 数据分析关键词
        let data_keywords = vec![
            "数据", "分析", "统计", "报告", "图表", "可视化",
            "data", "analysis", "statistics", "report", "chart", "visualization"
        ];
        
        let data_score = data_keywords.iter()
            .map(|&keyword| if query_lower.contains(keyword) { 1.0 } else { 0.0 })
            .sum::<f32>() / data_keywords.len() as f32;
        scores.insert("data_analysis".to_string(), data_score);

        scores
    }

    /// 意图分类
    async fn intent_classification(&self, _query: &str) -> Result<HashMap<String, f32>> {
        // 这里可以集成机器学习模型进行意图分类
        // 目前返回默认值
        Ok(HashMap::new())
    }

    /// 集成领域决策
    fn ensemble_domain_decision(
        &self,
        keyword_scores: HashMap<String, f32>,
        _intent_scores: HashMap<String, f32>,
    ) -> Result<Option<String>> {
        // 找到最高分的领域
        let best_domain = keyword_scores
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .filter(|(_, &score)| score > 0.3) // 阈值过滤
            .map(|(domain, _)| domain.clone());

        Ok(best_domain)
    }

    /// 评估复杂度
    async fn assess_complexity(&self, query: &str) -> Result<f32> {
        let mut complexity = 0.0;

        // 基于查询长度
        complexity += (query.len() as f32 / 100.0).min(1.0) * 0.3;

        // 基于关键词复杂度
        let complex_keywords = vec![
            "多步骤", "复杂", "深度", "全面", "详细", "综合",
            "multi-step", "complex", "deep", "comprehensive", "detailed"
        ];
        
        let keyword_complexity = complex_keywords.iter()
            .map(|&keyword| if query.to_lowercase().contains(keyword) { 1.0 } else { 0.0 })
            .sum::<f32>() / complex_keywords.len() as f32;
        
        complexity += keyword_complexity * 0.7;

        Ok(complexity.min(1.0))
    }

    /// 选择最优配置
    async fn select_optimal_config(
        &self,
        domain: &str,
        complexity: f32,
        context: &ConfigSelectionContext,
    ) -> Result<OptimalConfig> {
        // 获取领域模板
        let domain_template = self.config.domain_templates
            .get(domain)
            .cloned()
            .unwrap_or_else(|| self.create_default_domain_template());

        // 获取代理配置
        let agent_profile = self.get_best_agent_profile(domain, complexity)?;

        // 构建最优配置
        Ok(OptimalConfig {
            domain: domain.to_string(),
            complexity_score: complexity,
            agent_profile,
            domain_template,
            core_templates: self.config.core_templates.clone(),
            custom_constraints: self.config.custom_constraints
                .get(domain)
                .cloned()
                .unwrap_or_default(),
            resource_constraints: context.resource_constraints.clone(),
        })
    }

    /// 获取最佳代理配置
    fn get_best_agent_profile(&self, domain: &str, complexity: f32) -> Result<AgentProfile> {
        // 根据领域和复杂度选择最佳代理配置
        let profile_key = if complexity > 0.7 {
            format!("{}_expert", domain)
        } else {
            format!("{}_basic", domain)
        };

        self.config.agent_profiles
            .get(&profile_key)
            .or_else(|| self.config.agent_profiles.get(domain))
            .or_else(|| self.config.agent_profiles.get("default"))
            .cloned()
            .ok_or_else(|| anyhow!("未找到合适的代理配置"))
    }

    /// 创建默认领域模板
    fn create_default_domain_template(&self) -> DomainTemplate {
        DomainTemplate {
            domain_instructions: "通用任务执行指令".to_string(),
            typical_steps: vec![
                "分析需求".to_string(),
                "制定计划".to_string(),
                "执行任务".to_string(),
                "验证结果".to_string(),
            ],
            evaluation_criteria: vec![
                "任务完成度".to_string(),
                "结果准确性".to_string(),
                "执行效率".to_string(),
            ],
            critical_triggers: vec![],
            adjustment_patterns: vec![],
        }
    }

    /// 添加自定义模板
    pub async fn add_custom_template(&mut self, template: CustomTemplate) -> Result<()> {
        self.custom_templates.insert(template.id.clone(), template);
        Ok(())
    }

    /// 获取自定义模板
    pub fn get_custom_template(&self, template_id: &str) -> Option<&CustomTemplate> {
        self.custom_templates.get(template_id)
    }

    /// 创建A/B测试
    pub async fn create_ab_test(&mut self, test_config: ABTestConfig) -> Result<String> {
        let test_id = test_config.test_id.clone();
        self.ab_test_configs.insert(test_id.clone(), test_config);
        Ok(test_id)
    }

    /// 获取A/B测试配置
    pub fn get_ab_test_config(&self, test_id: &str) -> Option<&ABTestConfig> {
        self.ab_test_configs.get(test_id)
    }

    /// 检查配置是否存在
    pub async fn config_exists(&self, _config_id: &str) -> Result<bool> {
        // 目前只有一个默认配置，所以总是返回true
        Ok(true)
    }

    /// 获取配置
    pub async fn get_config(&self, _config_id: &str) -> Result<PromptConfig> {
        // 目前只有一个默认配置
        Ok(self.config.clone())
    }

    /// 保存配置
    pub async fn save_config(&self, _config_id: &str, _config: &PromptConfig) -> Result<()> {
        // TODO: 实现配置保存逻辑
        Ok(())
    }

    /// 列出所有配置
    pub async fn list_configs(&self) -> Result<Vec<String>> {
        // TODO: 实现配置列表逻辑
        Ok(vec!["default".to_string()])
    }

    /// 获取特定类型的系统prompt
    pub async fn get_system_prompt_by_type(&self, template_type: &TemplateType) -> Result<Option<String>> {
        // 查找匹配的系统级模板
        for template in self.custom_templates.values() {
            if template.template_type == *template_type && template.is_system {
                return Ok(Some(template.content.clone()));
            }
        }
        
        // 如果没找到，返回默认的模板内容
        match template_type {
            TemplateType::IntentClassifier => {
                Ok(Some(self.get_default_intent_classifier_prompt()))
            }
            _ => Ok(None)
        }
    }
    
    /// 获取默认的意图分析器prompt
    fn get_default_intent_classifier_prompt(&self) -> String {
        r#"作为一个AI意图分类器，请分析用户输入并判断意图类型。

请判断用户输入属于以下哪种类型：
1. Chat - 普通对话（问候、闲聊、简单交流）
2. Question - 知识性问答（询问概念、原理等，不需要实际执行）  
3. Task - 任务执行（需要AI助手执行具体的安全扫描、分析等操作）

判断标准：
- Chat: 问候语、感谢、简单交流等
- Question: 以"什么是"、"如何理解"等开头的概念性问题
- Task: 包含"扫描"、"检测"、"分析"、"帮我执行"等行动指令

请以JSON格式回复：
{
    "intent": "Chat|Question|Task",
    "confidence": 0.0-1.0,
    "reasoning": "分类理由",
    "requires_agent": true/false,
    "extracted_info": {"key": "value"}
}"#.to_string()
    }

    /// 验证配置
    pub fn validate_config(&self, config: &PromptConfig) -> Result<()> {
        // 检查必需字段
        if config.agent_profiles.is_empty() {
            return Err(anyhow!("代理配置文件不能为空"));
        }

        if config.core_templates.planner_core.is_empty() {
            return Err(anyhow!("规划器核心模板不能为空"));
        }

        // 验证模板格式
        for (name, template) in &config.domain_templates {
            if template.domain_instructions.is_empty() {
                return Err(anyhow!("领域模板 {} 的指令不能为空", name));
            }
        }

        Ok(())
    }

    /// 创建默认配置
    fn create_default_config() -> PromptConfig {
        let mut agent_profiles = HashMap::new();
        let mut domain_templates = HashMap::new();
        let mut custom_constraints = HashMap::new();

        // 默认代理配置
        agent_profiles.insert("security_expert".to_string(), AgentProfile {
            planner_domain: "security_testing".to_string(),
            executor_specializations: vec![
                "vulnerability_scan".to_string(),
                "compliance_check".to_string(),
            ],
            replanner_triggers: vec![
                "high_risk_found".to_string(),
                "new_attack_surface".to_string(),
            ],
            report_template: "security_assessment".to_string(),
            custom_config: None,
        });

        agent_profiles.insert("data_scientist".to_string(), AgentProfile {
            planner_domain: "data_analysis".to_string(),
            executor_specializations: vec![
                "data_processing".to_string(),
                "statistical_analysis".to_string(),
            ],
            replanner_triggers: vec![
                "data_quality_issue".to_string(),
                "significant_finding".to_string(),
            ],
            report_template: "analytical_report".to_string(),
            custom_config: None,
        });

        // 默认领域模板
        domain_templates.insert("security_testing".to_string(), DomainTemplate {
            domain_instructions: r#"
**安全检测专项要求：**
- 所有检测必须是被动的、非侵入性的
- 遵循负责任的安全研究原则
- 不进行任何可能影响服务正常运行的测试
- 覆盖OWASP Top 10安全风险
- 考虑合规性要求（如GDPR、网络安全法）
            "#.to_string(),
            typical_steps: vec![
                "信息收集".to_string(),
                "漏洞扫描".to_string(),
                "配置检查".to_string(),
                "合规评估".to_string(),
                "报告生成".to_string(),
            ],
            evaluation_criteria: vec![
                "发现漏洞数量".to_string(),
                "误报率".to_string(),
                "覆盖范围".to_string(),
                "合规性".to_string(),
            ],
            critical_triggers: vec![
                "发现高危漏洞".to_string(),
                "检测到活跃攻击".to_string(),
                "发现合规性问题".to_string(),
            ],
            adjustment_patterns: vec![
                AdjustmentPattern {
                    trigger: "发现新子域名".to_string(),
                    action: "增加子域名检测".to_string(),
                    priority: 1,
                },
                AdjustmentPattern {
                    trigger: "API泄露".to_string(),
                    action: "增加API安全检查".to_string(),
                    priority: 2,
                },
            ],
        });

        // 默认约束
        custom_constraints.insert("security_testing".to_string(), vec![
            "遵循OWASP测试指南".to_string(),
            "不进行破坏性测试".to_string(),
            "记录所有测试活动".to_string(),
        ]);

        PromptConfig {
            agent_profiles,
            agent_profile: AgentProfile::default(),
            domain_templates,
            domain_template: DomainTemplate::default(),
            core_templates: Self::create_default_core_templates(),
            custom_constraints,
            version: "1.0.0".to_string(),
        }
    }

    /// 创建默认核心模板
    fn create_default_core_templates() -> CoreTemplates {
        CoreTemplates {
            planner_core: r#"
你是一个任务规划专家。你的职责是将复杂任务分解为可执行的子步骤。

**核心规划原则：**
- 分析用户输入，理解最终目标
- 将任务分解为逻辑清晰的步骤序列  
- 每个步骤应该是具体可执行的
- 考虑步骤间的依赖关系
- 确保步骤的完整性和可验证性

**输出格式要求：**
```
PLAN:
1. [步骤描述] - [预期输出]
2. [步骤描述] - [预期输出]
...
```

{domain_specific_instructions}
{available_tools_info}
{user_query}

请制定详细的执行计划。
            "#.to_string(),
            
            executor_core: r#"
你是一个任务执行专家。你需要专注执行分配给你的特定步骤。

**执行基本原则：**
- 严格按照计划步骤执行
- 使用指定的工具完成任务
- 提供清晰、结构化的执行结果
- 遇到问题时明确报告具体困难
- 确保输出格式便于后续步骤使用

**标准输出格式：**
```
EXECUTION_RESULT:
状态: [SUCCESS/PARTIAL/FAILED]
结果: [具体执行结果]
问题: [如有问题，详细描述]
建议: [对后续步骤的建议]
```

{step_specific_instructions}
{available_tools}
{execution_context}

请执行指定任务并按格式返回结果。
            "#.to_string(),
            
            replanner_core: r#"
你是一个计划评估和调整专家。基于执行结果评估是否需要调整计划。

**评估决策树：**
1. 目标完成度评估
   - 原始目标是否已达成？
   - 结果质量是否满足要求？

2. 执行效果评估  
   - 是否发现了新的重要信息？
   - 是否遇到了预期外的问题？

3. 计划调整判断
   - 是否需要增加新的步骤？
   - 是否需要修改后续步骤？
   - 是否需要改变执行顺序？

**决策输出格式：**
- CONTINUE: 按原计划继续执行
- ADJUST: [详细的调整方案]  
- FINISHED: [最终结果总结]
- ABORT: [中止原因说明]

{domain_specific_evaluation}
{execution_results}
{original_plan}

请评估并决定下一步行动。
            "#.to_string(),
            
            report_generator_core: r#"
你是专业报告生成专家。基于所有执行结果生成结构化报告。

**标准报告结构：**
1. 执行摘要 (高管视角)
2. 关键发现 (按重要性排序)
3. 详细分析 (技术细节)
4. 风险评估 (影响和可能性)
5. 建议措施 (优先级排序)
6. 后续规划 (长期改进)

**报告质量标准：**
- 客观、准确、可验证
- 针对不同受众分层表达
- 提供具体、可操作的建议
- 包含风险量化评估

{report_domain_template}
{execution_summary}
{target_audience}

请生成专业报告。
            "#.to_string(),
        }
    }
}

/// 最优配置结果
#[derive(Debug, Clone)]
pub struct OptimalConfig {
    /// 检测到的领域
    pub domain: String,
    /// 复杂度评分
    pub complexity_score: f32,
    /// 代理配置
    pub agent_profile: AgentProfile,
    /// 领域模板
    pub domain_template: DomainTemplate,
    /// 核心模板
    pub core_templates: CoreTemplates,
    /// 自定义约束
    pub custom_constraints: Vec<String>,
    /// 资源约束
    pub resource_constraints: Option<ResourceConstraints>,
}

// 注意：prompt构建方法已移至PromptBuilder中，避免重复实现
// OptimalConfig现在只负责配置管理，不再直接构建prompt
impl OptimalConfig {
    /// 获取领域特定指令
    pub fn get_domain_instructions(&self) -> &str {
        &self.domain_template.domain_instructions
    }

    /// 获取评估标准
    pub fn get_evaluation_criteria(&self) -> Vec<String> {
        self.domain_template.evaluation_criteria.clone()
    }

    /// 获取自定义约束
    pub fn get_custom_constraints(&self) -> &[String] {
        &self.custom_constraints
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_domain_detection() {
        let manager = PromptConfigManager::new();
        let context = ConfigSelectionContext {
            user_query: "对目标网站进行安全漏洞扫描".to_string(),
            detected_domain: None,
            complexity_score: 0.5,
            resource_constraints: None,
            user_preferences: HashMap::new(),
        };

        let config = manager.get_optimal_config(&context).await.unwrap();
        assert_eq!(config.domain, "security_testing");
    }

    #[test]
    fn test_config_validation() {
        let manager = PromptConfigManager::new();
        let result = manager.validate_config(&manager.config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_prompt_building() {
        // let manager = PromptConfigManager::new();
        // let context = ConfigSelectionContext {
        //     user_query: "测试查询".to_string(),
        //     detected_domain: Some("security_testing".to_string()),
        //     complexity_score: 0.5,
        //     resource_constraints: None,
        //     user_preferences: HashMap::new(),
        // };

        // 这里需要同步版本的测试
        // let config = manager.get_optimal_config(&context).await.unwrap();
        // let prompt = config.build_planner_prompt("测试查询", "工具列表", None);
        // assert!(prompt.contains("测试查询"));
    }
}