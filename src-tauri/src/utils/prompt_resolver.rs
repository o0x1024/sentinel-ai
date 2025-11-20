/// 统一提示词解析器
/// 
/// 实现文档中定义的最终生效规则解析算法

use anyhow::Result;
use std::collections::HashMap;
use crate::services::prompt_db::PromptRepository;
use crate::models::prompt::{ArchitectureType, StageType};
use serde_json::Value;

#[derive(Debug, Clone)]
pub enum PromptStrategy {
    FollowGroup,
    Custom, 
    UserConfig,
}

impl From<String> for PromptStrategy {
    fn from(s: String) -> Self {
        match s.as_str() {
            "follow_group" => PromptStrategy::FollowGroup,
            "custom" => PromptStrategy::Custom,
            "user_config" => PromptStrategy::UserConfig,
            _ => PromptStrategy::FollowGroup, // 默认
        }
    }
}

/// 阶段类型枚举 - 规范阶段
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CanonicalStage {
    System,
    IntentClassifier,
    Planner,
    Executor,
    Replanner,
    Evaluator,
}

impl CanonicalStage {
    /// 映射到特定架构的阶段
    pub fn to_architecture_stage(&self, arch: &ArchitectureType) -> Option<StageType> {
        match (self, arch) {
            (CanonicalStage::Planner, ArchitectureType::ReWOO) => Some(StageType::Planner),
            (CanonicalStage::Executor, ArchitectureType::ReWOO) => Some(StageType::Worker),
            (CanonicalStage::Evaluator, ArchitectureType::ReWOO) => Some(StageType::Solver),
            
            (CanonicalStage::Planner, ArchitectureType::LLMCompiler) => Some(StageType::Planning),
            (CanonicalStage::Executor, ArchitectureType::LLMCompiler) => Some(StageType::Execution),
            (CanonicalStage::Evaluator, ArchitectureType::LLMCompiler) => Some(StageType::Evaluation),
            (CanonicalStage::Replanner, ArchitectureType::LLMCompiler) => Some(StageType::Replan),
            
            (CanonicalStage::Planner, ArchitectureType::PlanExecute) => Some(StageType::Planning),
            (CanonicalStage::Executor, ArchitectureType::PlanExecute) => Some(StageType::Execution),
            (CanonicalStage::Replanner, ArchitectureType::PlanExecute) => Some(StageType::Replan),
            
            (CanonicalStage::Planner, ArchitectureType::Travel) => Some(StageType::Observe),
            (CanonicalStage::Executor, ArchitectureType::Travel) => Some(StageType::Act),
            (CanonicalStage::Evaluator, ArchitectureType::Travel) => Some(StageType::Orient),
            (CanonicalStage::Replanner, ArchitectureType::Travel) => Some(StageType::Decide),
            
            // System和IntentClassifier不映射到具体架构阶段
            _ => None,
        }
    }
}

/// Agent提示词配置
#[derive(Debug, Clone)]
pub struct AgentPromptConfig {
    pub strategy: PromptStrategy,
    pub group_id: Option<i64>,
    pub prompt_ids: HashMap<CanonicalStage, Option<i64>>,
    pub prompt_overrides: HashMap<CanonicalStage, String>,
    pub pinned_versions: HashMap<i64, String>,
}

impl Default for AgentPromptConfig {
    fn default() -> Self {
        Self {
            strategy: PromptStrategy::FollowGroup,
            group_id: None,
            prompt_ids: HashMap::new(),
            prompt_overrides: HashMap::new(),
            pinned_versions: HashMap::new(),
        }
    }
}

/// 提示词解析器
pub struct PromptResolver {
    prompt_repo: PromptRepository,
}

impl PromptResolver {
    pub fn new(prompt_repo: PromptRepository) -> Self {
        Self { prompt_repo }
    }

    /// 解析最终生效的提示词内容
    /// 
    /// 实现文档中的最终生效规则（Resolution Order）：
    /// 1) Agent 文本覆盖
    /// 2) Agent 指定模板  
    /// 3) Agent 绑定分组
    /// 4) 用户全局配置
    /// 5) 架构默认分组
    /// 6) 阶段活动模板
    /// 7) 内置引擎默认模板
    pub async fn resolve_prompt(
        &self,
        agent_config: &AgentPromptConfig,
        architecture: ArchitectureType,
        stage: CanonicalStage,
        fallback_prompt: Option<&str>,
    ) -> Result<String> {
        // 1) Agent 文本覆盖：若存在非空文本，直接使用
        if let Some(override_text) = agent_config.prompt_overrides.get(&stage) {
            if !override_text.trim().is_empty() {
                return Ok(override_text.clone());
            }
        }

        // 2) Agent 指定模板：若存在且（可选）命中pinned_versions
        if let Some(Some(template_id)) = agent_config.prompt_ids.get(&stage) {
            if let Ok(Some(template)) = self.prompt_repo.get_template(*template_id).await {
                // TODO: 支持版本固定逻辑
                let content = self.render_variables(&template.content, &HashMap::new())?;
                return Ok(content);
            }
        }

        // 3) Agent 绑定分组：若strategy=follow_group且选定group_id
        if matches!(agent_config.strategy, PromptStrategy::FollowGroup) {
            if let Some(group_id) = agent_config.group_id {
                if let Some(arch_stage) = stage.to_architecture_stage(&architecture) {
                    let items = self.prompt_repo.list_group_items(group_id).await?;
                    if let Some(item) = items.iter().find(|item| item.stage == arch_stage) {
                        if let Ok(Some(template)) = self.prompt_repo.get_template(item.template_id).await {
                            let content = self.render_variables(&template.content, &HashMap::new())?;
                            return Ok(content);
                        }
                    }
                }
            }
        }

        // 4) 用户全局配置：查user_prompt_configs
        if let Some(arch_stage) = stage.to_architecture_stage(&architecture) {
            if let Ok(prompt) = self.prompt_repo.get_active_prompt(architecture, arch_stage).await {
                if let Some(content) = prompt {
                    let rendered = self.render_variables(&content, &HashMap::new())?;
                    return Ok(rendered);
                }
            }
        }

        // 5-6) 架构默认分组 + 阶段活动模板
        // get_active_prompt 已经包含了这两个逻辑

        // 7) 内置引擎默认模板
        if let Some(fallback) = fallback_prompt {
            return Ok(fallback.to_string());
        }

        Ok("".to_string())
    }

    /// 渲染变量
    /// 
    /// 使用{var_name}语法进行简单的字符串替换
    pub fn render_variables(&self, template: &str, context: &HashMap<String, Value>) -> Result<String> {
        let mut result = template.to_string();
        
        for (key, value) in context {
            let placeholder = format!("{{{}}}", key);
            let replacement = match value {
                Value::String(s) => s.clone(),
                _ => value.to_string(),
            };
            result = result.replace(&placeholder, &replacement);
        }
        
        Ok(result)
    }
}

impl AgentPromptConfig {
    /// 从Agent参数解析提示词配置
    pub fn parse_agent_config(parameters: &HashMap<String, Value>) -> AgentPromptConfig {
        let mut config = AgentPromptConfig::default();

        // 解析strategy
        if let Some(strategy_val) = parameters.get("prompt_strategy") {
            if let Some(strategy_str) = strategy_val.as_str() {
                config.strategy = PromptStrategy::from(strategy_str.to_string());
            }
        }

        // 解析group_id
        if let Some(group_id_val) = parameters.get("group_id") {
            config.group_id = group_id_val.as_i64();
        }

        // 解析prompt_ids
        if let Some(prompt_ids_val) = parameters.get("prompt_ids") {
            if let Some(prompt_ids_obj) = prompt_ids_val.as_object() {
                for (stage_str, id_val) in prompt_ids_obj {
                    if let Some(stage) = PromptResolver::parse_canonical_stage(stage_str) {
                        config.prompt_ids.insert(stage, id_val.as_i64());
                    }
                }
            }
        }

        // 解析prompt覆盖
        if let Some(prompts_val) = parameters.get("prompts") {
            if let Some(prompts_obj) = prompts_val.as_object() {
                for (stage_str, text_val) in prompts_obj {
                    if let Some(stage) = PromptResolver::parse_canonical_stage(stage_str) {
                        if let Some(text) = text_val.as_str() {
                            if !text.trim().is_empty() {
                                config.prompt_overrides.insert(stage, text.to_string());
                            }
                        }
                    }
                }
            }
        }

        // 解析pinned_versions
        if let Some(pinned_val) = parameters.get("pinned_versions") {
            if let Some(pinned_obj) = pinned_val.as_object() {
                for (id_str, version_val) in pinned_obj {
                    if let (Ok(id), Some(version)) = (id_str.parse::<i64>(), version_val.as_str()) {
                        config.pinned_versions.insert(id, version.to_string());
                    }
                }
            }
        }

        config
    }
}

impl PromptResolver {
    fn parse_canonical_stage(stage_str: &str) -> Option<CanonicalStage> {
        match stage_str {
            "system" => Some(CanonicalStage::System),
            "intent_classifier" => Some(CanonicalStage::IntentClassifier),
            "planner" => Some(CanonicalStage::Planner),
            "executor" => Some(CanonicalStage::Executor),
            "replanner" => Some(CanonicalStage::Replanner),
            "evaluator" => Some(CanonicalStage::Evaluator),
            _ => None,
        }
    }
}
