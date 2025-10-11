//! Prompt构建器
//! 
//! 实现动态prompt组装和优化功能，支持：
//! - 模板变量替换
//! - 上下文注入
//! - 动态工具信息生成
//! - Prompt优化和验证
use super::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::{Result, anyhow};
use regex::Regex;

/// Prompt构建器
#[derive(Debug, Clone)]
pub struct PromptBuilder {
    /// 配置管理器
    config_manager: PromptConfigManager,
    /// 模板缓存
    #[allow(unused)]
    template_cache: HashMap<String, CompiledTemplate>,
    /// 变量解析器
    variable_resolver: VariableResolver,
}

/// 编译后的模板
#[derive(Debug, Clone)]
pub struct CompiledTemplate {
    /// 模板内容
    pub content: String,
    /// 变量列表
    pub variables: Vec<String>,
    /// 模板类型
    pub template_type: TemplateType,
    /// 编译时间
    pub compiled_at: chrono::DateTime<chrono::Utc>,
}

/// 变量解析器
#[derive(Debug, Clone)]
pub struct VariableResolver {
    /// 变量模式
    variable_pattern: Regex,
    /// 内置变量
    builtin_variables: HashMap<String, String>,
}

/// Prompt构建上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptBuildContext {
    /// 用户查询
    pub user_query: String,
    /// 目标信息
    pub target_info: Option<TargetInfo>,
    /// 可用工具
    pub available_tools: Vec<ToolInfo>,
    /// 执行上下文
    pub execution_context: Option<ExecutionContext>,
    /// 历史信息
    pub history: Vec<HistoryItem>,
    /// 自定义变量
    pub custom_variables: HashMap<String, serde_json::Value>,
    /// RAG检索上下文
    pub rag_context: Option<RagContext>,
}

/// RAG检索上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagContext {
    /// 是否启用RAG
    pub enabled: bool,
    /// 检索到的相关文档
    pub retrieved_documents: Vec<RagDocument>,
    /// 格式化的上下文文本
    pub formatted_context: String,
    /// 检索配置
    pub retrieval_config: RagRetrievalConfig,
    /// Token预算控制
    pub token_budget: Option<TokenBudget>,
}

/// RAG文档
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagDocument {
    /// 文档ID
    pub id: String,
    /// 文档内容
    pub content: String,
    /// 相似度分数
    pub score: f32,
    /// 文档元数据
    pub metadata: HashMap<String, String>,
    /// 来源信息
    pub source: String,
}

/// RAG检索配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagRetrievalConfig {
    /// 集合名称
    pub collection_name: Option<String>,
    /// 检索数量
    pub top_k: usize,
    /// 是否使用MMR
    pub use_mmr: bool,
    /// MMR Lambda参数
    pub mmr_lambda: f32,
    /// 相似度阈值
    pub similarity_threshold: f32,
}

/// Token预算控制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBudget {
    /// 最大Token数
    pub max_tokens: usize,
    /// 当前使用的Token数
    pub used_tokens: usize,
    /// 上下文优先级策略
    pub priority_strategy: ContextPriorityStrategy,
}

/// 上下文优先级策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextPriorityStrategy {
    /// 按相似度排序
    BySimilarity,
    /// 按时间排序
    ByTime,
    /// 按重要性排序
    ByImportance,
    /// 混合策略
    Hybrid,
}

/// 目标信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetInfo {
    /// 目标类型
    pub target_type: String,
    /// 目标地址
    pub address: String,
    /// 端口信息
    pub ports: Option<Vec<u16>>,
    /// 协议信息
    pub protocols: Vec<String>,
    /// 认证信息
    pub auth_info: Option<AuthInfo>,
    /// 额外属性
    pub attributes: HashMap<String, serde_json::Value>,
    /// 主机名
    pub host: String,
}

/// 认证信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthInfo {
    /// 认证类型
    pub auth_type: String,
    /// 用户名
    pub username: Option<String>,
    /// 密码（加密存储）
    pub password_hash: Option<String>,
    /// Token
    pub token: Option<String>,
    /// 证书路径
    pub cert_path: Option<String>,
}

/// 执行上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// 当前步骤
    pub current_step: String,
    /// 已完成步骤
    pub completed_steps: Vec<String>,
    /// 步骤结果
    pub step_results: HashMap<String, serde_json::Value>,
    /// 错误信息
    pub errors: Vec<ErrorInfo>,
    /// 性能指标
    pub metrics: HashMap<String, f64>,
}

/// 错误信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    /// 错误类型
    pub error_type: String,
    /// 错误消息
    pub message: String,
    /// 发生时间
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 相关步骤
    pub step_id: Option<String>,
}

/// 历史项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    /// 项目ID
    pub id: String,
    /// 项目类型
    pub item_type: String,
    /// 内容
    pub content: String,
    /// 时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 元数据
    pub metadata: HashMap<String, serde_json::Value>,
}

/// 工具信息（重新导出以避免循环依赖）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    /// 工具名称
    pub name: String,
    /// 工具描述
    pub description: String,
    /// 工具类型
    pub tool_type: String,
    /// 输入参数
    pub input_schema: serde_json::Value,
    /// 输出格式
    pub output_schema: serde_json::Value,
    /// 预估执行时间（秒）
    pub estimated_duration: f64,
    /// 资源需求
    pub resource_requirements: ResourceRequirements,
    /// 依赖关系
    pub dependencies: Vec<String>,
}

/// 资源需求（重新导出）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// 内存需求（MB）
    pub memory_mb: f64,
    /// CPU需求（百分比）
    pub cpu_percent: f32,
    /// 网络带宽需求（KB/s）
    pub network_kbps: f64,
    /// 磁盘空间需求（MB）
    pub disk_mb: f64,
}

/// Prompt构建结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptBuildResult {
    /// 构建的prompt
    pub prompt: String,
    /// 使用的模板
    pub template_id: String,
    /// 变量映射
    pub variable_mapping: HashMap<String, String>,
    /// 构建统计
    pub build_stats: BuildStats,
}

/// 构建统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildStats {
    /// 构建时间（毫秒）
    pub build_time_ms: u64,
    /// 模板长度
    pub template_length: usize,
    /// 变量数量
    pub variable_count: usize,
    /// 工具数量
    pub tool_count: usize,
}

impl PromptBuilder {
    /// 创建新的prompt构建器
    pub fn new(config_manager: PromptConfigManager) -> Self {
        Self {
            config_manager,
            template_cache: HashMap::new(),
            variable_resolver: VariableResolver::new(),
        }
    }

    /// 构建规划器prompt
    pub async fn build_planner_prompt(
        &self,
        context: &PromptBuildContext,
    ) -> Result<PromptBuildResult> {
        let start_time = std::time::Instant::now();

        // 获取最优配置
        let selection_context = ConfigSelectionContext {
            user_query: context.user_query.clone(),
            detected_domain: None,
            complexity_score: self.assess_query_complexity(&context.user_query)?,
            resource_constraints: None,
            user_preferences: HashMap::new(),
        };

        let optimal_config = self.config_manager.get_optimal_config(&selection_context).await?;

        // 准备变量
        let mut variables = HashMap::new();
        variables.insert("user_query".to_string(), format!("**用户查询：** {}", context.user_query));
        variables.insert("domain_specific_instructions".to_string(), optimal_config.domain_template.domain_instructions);
        variables.insert("available_tools_info".to_string(), self.format_tools_info(&context.available_tools)?);
        variables.insert("custom_constraints".to_string(), optimal_config.custom_constraints.join("\n- "));

        // 添加目标信息
        if let Some(target_info) = &context.target_info {
            variables.insert("target_info".to_string(), self.format_target_info(target_info)?);
        }

        // 添加历史上下文
        if !context.history.is_empty() {
            variables.insert("history_context".to_string(), self.format_history(&context.history)?);
        }

        // 添加RAG检索上下文
        if let Some(rag_context) = &context.rag_context {
            if rag_context.enabled && !rag_context.formatted_context.is_empty() {
                variables.insert("rag_context".to_string(), self.format_rag_context(rag_context)?);
            }
        }

        // 构建prompt
        let prompt = self.variable_resolver.resolve_variables(
            &optimal_config.core_templates.planner_core,
            &variables,
        )?;

        let build_time = start_time.elapsed().as_millis() as u64;

        let variable_count = variables.len();
        let template_length = optimal_config.core_templates.planner_core.len();
        let tool_count = context.available_tools.len();
        
        Ok(PromptBuildResult {
            prompt,
            template_id: "planner_core".to_string(),
            variable_mapping: variables,
            build_stats: BuildStats {
                build_time_ms: build_time,
                template_length,
                variable_count,
                tool_count,
            },
        })
    }

    /// 构建执行器prompt
    pub async fn build_executor_prompt(
        &self,
        context: &PromptBuildContext,
        step_instructions: &str,
    ) -> Result<PromptBuildResult> {
        let start_time = std::time::Instant::now();

        // 获取最优配置
        let selection_context = ConfigSelectionContext {
            user_query: context.user_query.clone(),
            detected_domain: None,
            complexity_score: self.assess_query_complexity(&context.user_query)?,
            resource_constraints: None,
            user_preferences: HashMap::new(),
        };

        let optimal_config = self.config_manager.get_optimal_config(&selection_context).await?;

        // 准备变量
        let mut variables = HashMap::new();
        variables.insert("step_specific_instructions".to_string(), step_instructions.to_string());
        variables.insert("available_tools".to_string(), self.format_tools_info(&context.available_tools)?);
        
        // 格式化执行上下文
        let execution_context_str = if let Some(exec_ctx) = &context.execution_context {
            self.format_execution_context(exec_ctx)?
        } else {
            "无执行上下文".to_string()
        };
        variables.insert("execution_context".to_string(), execution_context_str);

        // 添加RAG检索上下文
        if let Some(rag_context) = &context.rag_context {
            if rag_context.enabled && !rag_context.formatted_context.is_empty() {
                variables.insert("rag_context".to_string(), self.format_rag_context(rag_context)?);
            }
        }

        // 构建prompt
        let prompt = self.variable_resolver.resolve_variables(
            &optimal_config.core_templates.executor_core,
            &variables,
        )?;

        let build_time = start_time.elapsed().as_millis() as u64;

        let variable_count = variables.len();
        let template_length = optimal_config.core_templates.executor_core.len();
        let tool_count = context.available_tools.len();
        
        Ok(PromptBuildResult {
            prompt,
            template_id: "executor_core".to_string(),
            variable_mapping: variables,
            build_stats: BuildStats {
                build_time_ms: build_time,
                template_length,
                variable_count,
                tool_count,
            },
        })
    }

    /// 构建重规划器prompt
    pub async fn build_replanner_prompt(
        &self,
        context: &PromptBuildContext,
        execution_results: &str,
        original_plan: &str,
    ) -> Result<PromptBuildResult> {
        let start_time = std::time::Instant::now();

        // 获取最优配置
        let selection_context = ConfigSelectionContext {
            user_query: context.user_query.clone(),
            detected_domain: None,
            complexity_score: self.assess_query_complexity(&context.user_query)?,
            resource_constraints: None,
            user_preferences: HashMap::new(),
        };

        let optimal_config = self.config_manager.get_optimal_config(&selection_context).await?;

        // 准备变量
        let mut variables = HashMap::new();
        variables.insert("execution_results".to_string(), execution_results.to_string());
        variables.insert("original_plan".to_string(), original_plan.to_string());
        variables.insert("domain_specific_evaluation".to_string(), 
            optimal_config.domain_template.evaluation_criteria.join("\n- "));

        // 添加触发器信息
        let triggers = optimal_config.domain_template.critical_triggers.join("\n- ");
        variables.insert("critical_triggers".to_string(), triggers);

        // 添加RAG检索上下文
        if let Some(rag_context) = &context.rag_context {
            if rag_context.enabled && !rag_context.formatted_context.is_empty() {
                variables.insert("rag_context".to_string(), self.format_rag_context(rag_context)?);
            }
        }

        // 构建prompt
        let prompt = self.variable_resolver.resolve_variables(
            &optimal_config.core_templates.replanner_core,
            &variables,
        )?;

        let build_time = start_time.elapsed().as_millis() as u64;

        let variable_count = variables.len();
        let template_length = optimal_config.core_templates.replanner_core.len();
        let tool_count = context.available_tools.len();
        
        Ok(PromptBuildResult {
            prompt,
            template_id: "replanner_core".to_string(),
            variable_mapping: variables,
            build_stats: BuildStats {
                build_time_ms: build_time,
                template_length,
                variable_count,
                tool_count,
            },
        })
    }

    /// 构建报告生成器prompt
    pub async fn build_report_prompt(
        &self,
        context: &PromptBuildContext,
        execution_summary: &str,
        target_audience: &str,
    ) -> Result<PromptBuildResult> {
        let start_time = std::time::Instant::now();

        // 获取最优配置
        let selection_context = ConfigSelectionContext {
            user_query: context.user_query.clone(),
            detected_domain: None,
            complexity_score: self.assess_query_complexity(&context.user_query)?,
            resource_constraints: None,
            user_preferences: HashMap::new(),
        };

        let optimal_config = self.config_manager.get_optimal_config(&selection_context).await?;

        // 准备变量
        let mut variables = HashMap::new();
        variables.insert("execution_summary".to_string(), execution_summary.to_string());
        variables.insert("target_audience".to_string(), target_audience.to_string());
        variables.insert("report_domain_template".to_string(), 
            optimal_config.domain_template.domain_instructions);

        // 添加RAG检索上下文
        if let Some(rag_context) = &context.rag_context {
            if rag_context.enabled && !rag_context.formatted_context.is_empty() {
                variables.insert("rag_context".to_string(), self.format_rag_context(rag_context)?);
            }
        }

        // 构建prompt
        let prompt = self.variable_resolver.resolve_variables(
            &optimal_config.core_templates.report_generator_core,
            &variables,
        )?;

        let build_time = start_time.elapsed().as_millis() as u64;

        let variable_count = variables.len();
        let template_length = optimal_config.core_templates.report_generator_core.len();
        let tool_count = context.available_tools.len();
        
        Ok(PromptBuildResult {
            prompt,
            template_id: "report_generator_core".to_string(),
            variable_mapping: variables,
            build_stats: BuildStats {
                build_time_ms: build_time,
                template_length,
                variable_count,
                tool_count,
            },
        })
    }

    /// 评估查询复杂度
    fn assess_query_complexity(&self, query: &str) -> Result<f32> {
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

    /// 格式化工具信息
    fn format_tools_info(&self, tools: &[ToolInfo]) -> Result<String> {
        if tools.is_empty() {
            return Ok("无可用工具".to_string());
        }

        let mut formatted = String::from("**可用工具：**\n");
        
        for tool in tools {
            formatted.push_str(&format!(
                "- **{}** ({}): {}\n  - 预估时间: {}秒\n  - 资源需求: 内存{}MB, CPU{}%\n",
                tool.name,
                tool.tool_type,
                tool.description,
                tool.estimated_duration,
                tool.resource_requirements.memory_mb,
                tool.resource_requirements.cpu_percent
            ));

            if !tool.dependencies.is_empty() {
                formatted.push_str(&format!("  - 依赖: {}\n", tool.dependencies.join(", ")));
            }
        }

        Ok(formatted)
    }

    /// 格式化目标信息
    fn format_target_info(&self, target_info: &TargetInfo) -> Result<String> {
        let mut formatted = format!(
            "**目标信息：**\n- 类型: {}\n- 地址: {}\n",
            target_info.target_type,
            target_info.address
        );

        if let Some(ports) = &target_info.ports {
            formatted.push_str(&format!("- 端口: {:?}\n", ports));
        }

        if !target_info.protocols.is_empty() {
            formatted.push_str(&format!("- 协议: {}\n", target_info.protocols.join(", ")));
        }

        if let Some(auth) = &target_info.auth_info {
            formatted.push_str(&format!("- 认证类型: {}\n", auth.auth_type));
        }

        Ok(formatted)
    }

    /// 格式化执行上下文
    fn format_execution_context(&self, context: &ExecutionContext) -> Result<String> {
        let mut formatted = format!("**执行上下文：**\n- 当前步骤: {}\n", context.current_step);

        if !context.completed_steps.is_empty() {
            formatted.push_str(&format!("- 已完成步骤: {}\n", context.completed_steps.join(", ")));
        }

        if !context.errors.is_empty() {
            formatted.push_str(&format!("- 错误数量: {}\n", context.errors.len()));
            for error in &context.errors {
                formatted.push_str(&format!("  - {}: {}\n", error.error_type, error.message));
            }
        }

        if !context.metrics.is_empty() {
            formatted.push_str("- 性能指标:\n");
            for (key, value) in &context.metrics {
                formatted.push_str(&format!("  - {}: {:.2}\n", key, value));
            }
        }

        Ok(formatted)
    }

    /// 格式化历史信息
    fn format_history(&self, history: &[HistoryItem]) -> Result<String> {
        if history.is_empty() {
            return Ok("无历史记录".to_string());
        }

        let mut formatted = String::from("**历史上下文：**\n");
        
        // 只显示最近的几条记录
        let recent_history = if history.len() > 5 {
            &history[history.len()-5..]
        } else {
            history
        };

        for item in recent_history {
            formatted.push_str(&format!(
                "- [{}] {}: {}\n",
                item.timestamp.format("%H:%M:%S"),
                item.item_type,
                item.content.chars().take(100).collect::<String>()
            ));
        }

        Ok(formatted)
    }

    /// 格式化RAG检索上下文
    pub fn format_rag_context(&self, rag_context: &RagContext) -> Result<String> {
        if rag_context.retrieved_documents.is_empty() {
            return Ok("无相关上下文".to_string());
        }

        let mut formatted = String::new();
        formatted.push_str("**相关上下文：**\n");

        for (i, doc) in rag_context.retrieved_documents.iter().enumerate() {
            formatted.push_str(&format!("### 文档 {} (相似度: {:.3})\n", i + 1, doc.score));
            formatted.push_str(&format!("**来源:** {}\n", doc.source));
            
            if !doc.metadata.is_empty() {
                formatted.push_str("**元数据:** ");
                for (key, value) in &doc.metadata {
                    formatted.push_str(&format!("{}={}, ", key, value));
                }
                formatted.push_str("\n");
            }
            
            formatted.push_str(&format!("**内容:**\n{}\n\n", doc.content));
        }

        // 添加检索配置信息
        if let Some(collection) = &rag_context.retrieval_config.collection_name {
            formatted.push_str(&format!("*检索集合: {}*\n", collection));
        }
        formatted.push_str(&format!("*Top-K: {}, 相似度阈值: {:.3}*\n", 
            rag_context.retrieval_config.top_k, 
            rag_context.retrieval_config.similarity_threshold));

        Ok(formatted)
    }

    /// 验证prompt
    pub fn validate_prompt(&self, prompt: &str) -> Result<ValidationResult> {
        let mut issues = Vec::new();
        let mut warnings = Vec::new();

        // 检查prompt长度
        if prompt.len() > 8000 {
            warnings.push("Prompt长度超过8000字符，可能影响LLM性能".to_string());
        }

        if prompt.len() < 100 {
            issues.push("Prompt长度过短，可能缺少必要信息".to_string());
        }

        // 检查必要的结构
        if !prompt.contains("**") {
            warnings.push("Prompt缺少格式化标记，可能影响可读性".to_string());
        }

        // 检查变量替换
        let unresolved_vars = Regex::new(r"\{[^}]+\}").unwrap();
        if unresolved_vars.is_match(prompt) {
            issues.push("存在未解析的变量".to_string());
        }

        Ok(ValidationResult {
            is_valid: issues.is_empty(),
            issues,
            warnings,
            score: self.calculate_prompt_score(prompt),
        })
    }

    /// 计算prompt评分
    fn calculate_prompt_score(&self, prompt: &str) -> f32 {
        let mut score = 1.0;

        // 长度评分
        let length_score = if prompt.len() > 8000 {
            0.7
        } else if prompt.len() < 100 {
            0.5
        } else {
            1.0
        };
        score *= length_score;

        // 结构评分
        let structure_score = if prompt.contains("**") && prompt.contains(":\n") {
            1.0
        } else {
            0.8
        };
        score *= structure_score;

        // 完整性评分
        let completeness_score = if Regex::new(r"\{[^}]+\}").unwrap().is_match(prompt) {
            0.6 // 有未解析变量
        } else {
            1.0
        };
        score *= completeness_score;

        score
    }
}

/// 验证结果
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// 是否有效
    pub is_valid: bool,
    /// 问题列表
    pub issues: Vec<String>,
    /// 警告列表
    pub warnings: Vec<String>,
    /// 评分（0-1）
    pub score: f32,
}

impl VariableResolver {
    /// 创建新的变量解析器
    pub fn new() -> Self {
        let variable_pattern = Regex::new(r"\{([^}]+)\}").unwrap();
        let mut builtin_variables = HashMap::new();
        
        // 添加内置变量
        builtin_variables.insert("timestamp".to_string(), 
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string());
        builtin_variables.insert("date".to_string(), 
            chrono::Utc::now().format("%Y-%m-%d").to_string());
        builtin_variables.insert("time".to_string(), 
            chrono::Utc::now().format("%H:%M:%S").to_string());

        Self {
            variable_pattern,
            builtin_variables,
        }
    }

    /// 解析变量
    pub fn resolve_variables(
        &self,
        template: &str,
        variables: &HashMap<String, String>,
    ) -> Result<String> {
        let mut result = template.to_string();

        // 合并变量（自定义变量优先）
        let mut all_variables = self.builtin_variables.clone();
        all_variables.extend(variables.clone());

        // 替换变量
        for (key, value) in &all_variables {
            let pattern = format!("{{{}}}", key);
            result = result.replace(&pattern, value);
        }

        // 检查是否还有未解析的变量
        if self.variable_pattern.is_match(&result) {
            let unresolved: Vec<&str> = self.variable_pattern
                .captures_iter(&result)
                .map(|cap| cap.get(1).unwrap().as_str())
                .collect();
            
            return Err(anyhow!("未解析的变量: {:?}", unresolved));
        }

        Ok(result)
    }

    /// 提取模板中的变量
    pub fn extract_variables(&self, template: &str) -> Vec<String> {
        self.variable_pattern
            .captures_iter(template)
            .map(|cap| cap.get(1).unwrap().as_str().to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_resolver() {
        let resolver = VariableResolver::new();
        let template = "Hello {name}, today is {date}";
        
        let mut variables = HashMap::new();
        variables.insert("name".to_string(), "World".to_string());
        
        let result = resolver.resolve_variables(template, &variables).unwrap();
        assert!(result.contains("Hello World"));
        assert!(result.contains("today is"));
    }

    #[test]
    fn test_extract_variables() {
        let resolver = VariableResolver::new();
        let template = "Test {var1} and {var2} with {var1} again";
        
        let variables = resolver.extract_variables(template);
        assert_eq!(variables.len(), 3); // var1, var2, var1
        assert!(variables.contains(&"var1".to_string()));
        assert!(variables.contains(&"var2".to_string()));
    }

    #[tokio::test]
    async fn test_prompt_builder() {
        let config_manager = PromptConfigManager::new();
        let builder = PromptBuilder::new(config_manager);
        
        let context = PromptBuildContext {
            user_query: "测试查询".to_string(),
            target_info: None,
            available_tools: vec![],
            execution_context: None,
            history: vec![],
            custom_variables: HashMap::new(),
            rag_context: None,
        };

        let result = builder.build_planner_prompt(&context).await.unwrap();
        assert!(result.prompt.contains("测试查询"));
        assert!(result.build_stats.build_time_ms > 0);
    }
}