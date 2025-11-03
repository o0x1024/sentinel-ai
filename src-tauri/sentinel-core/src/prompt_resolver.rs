//! Prompt 解析器相关类型和 trait

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::models::prompt::{ArchitectureType, StageType};

/// 提示词策略
#[derive(Debug, Clone, Serialize, Deserialize)]
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
            _ => PromptStrategy::FollowGroup,
        }
    }
}

/// 规范阶段类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
            (CanonicalStage::Replanner, ArchitectureType::LLMCompiler) => Some(StageType::Replan),
            
            (CanonicalStage::Planner, ArchitectureType::PlanExecute) => Some(StageType::Planning),
            (CanonicalStage::Executor, ArchitectureType::PlanExecute) => Some(StageType::Execution),
            (CanonicalStage::Replanner, ArchitectureType::PlanExecute) => Some(StageType::Replan),
            
            _ => None,
        }
    }
}

/// Agent 提示词配置
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl AgentPromptConfig {
    /// Create from context hashmap
    pub fn from_context(context: &HashMap<String, serde_json::Value>) -> Self {
        let mut config = Self::default();
        
        // Parse strategy
        if let Some(strategy) = context.get("prompt_strategy") {
            if let Some(s) = strategy.as_str() {
                config.strategy = PromptStrategy::from(s.to_string());
            }
        }
        
        // Parse group_id
        if let Some(group_id) = context.get("prompt_group_id") {
            config.group_id = group_id.as_i64();
        }
        
        // Parse prompt_ids from various fields
        if let Some(ids) = context.get("prompt_ids") {
            if let Some(obj) = ids.as_object() {
                for (key, value) in obj {
                    // Try to parse key as CanonicalStage
                    let stage = match key.as_str() {
                        "planner" | "planning" => CanonicalStage::Planner,
                        "executor" | "execution" => CanonicalStage::Executor,
                        "replanner" | "replan" => CanonicalStage::Replanner,
                        "evaluator" => CanonicalStage::Evaluator,
                        _ => continue,
                    };
                    config.prompt_ids.insert(stage, value.as_i64());
                }
            }
        }
        
        config
    }
    
    /// Create new config (backward compatibility)
    pub fn new(
        architecture: ArchitectureType,
        model_name: Option<String>,
        params: HashMap<String, serde_json::Value>,
    ) -> Self {
        let mut config = Self::from_context(&params);
        // Store architecture and model in params if needed
        config
    }
}

/// Prompt 解析器 trait
#[async_trait::async_trait]
pub trait PromptResolver: Send + Sync {
    /// 解析最终生效的提示词内容
    async fn resolve_prompt(
        &self,
        agent_config: &AgentPromptConfig,
        architecture: ArchitectureType,
        stage: CanonicalStage,
        fallback_prompt: Option<&str>,
    ) -> anyhow::Result<String>;
}

