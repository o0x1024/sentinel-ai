//! Replanner 组件 - 重新规划器 - DISABLED (ai_adapter removed)
//! 
//! 负责在执行过程中动态调整计划，处理异常情况和优化执行策略

// DISABLED: use crate::ai_adapter::core::AiAdapterManager;
use crate::engines::plan_and_execute::types::*;
use crate::engines::plan_and_execute::planner::{Planner, PlannerConfig};
use crate::engines::plan_and_execute::executor::ExecutionContext;
use crate::engines::StepExecutionResult;
use crate::services::prompt_db::PromptRepository;
use crate::services::ai::AiServiceManager;
use std::sync::Arc;

/// 重新规划器 - DISABLED
#[derive(Debug)]
pub struct Replanner {
    /// 规划器实例
    planner: Arc<Planner>,
    /// 提示词仓库
    prompt_repo: Arc<PromptRepository>,
    /// 配置
    config: ReplannerConfig,
}

/// 重新规划器配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReplannerConfig {
    /// 最大重新规划次数
    pub max_replan_attempts: usize,
    /// 重新规划触发阈值
    pub replan_threshold: f64,
    /// 是否启用自动重新规划
    pub auto_replan_enabled: bool,
}

/// 重新规划触发器 - DISABLED
#[derive(Debug, Clone)]
pub enum ReplanTrigger {
    /// 步骤执行失败
    StepFailure,
    /// 执行超时
    Timeout,
    /// 手动触发
    Manual,
}

/// 重新规划结果 - DISABLED
#[derive(Debug, Clone)]
pub struct ReplanResult {
    pub should_replan: bool,
    pub replan_reason: String,
    pub new_plan: Option<crate::engines::plan_and_execute::ExecutionPlan>,
}

impl Default for ReplannerConfig {
    fn default() -> Self {
        Self {
            max_replan_attempts: 3,
            replan_threshold: 0.7,
            auto_replan_enabled: true,
        }
    }
}

impl Replanner {
    /// 创建新的重新规划器 - DISABLED
    pub async fn new(
        _ai_service_manager: Arc<AiServiceManager>,
        prompt_repo: Arc<PromptRepository>,
        config: ReplannerConfig, 
    ) -> Result<Self, PlanAndExecuteError> {
        let planner_config = PlannerConfig::default();
        let planner = Arc::new(Planner::new(planner_config, Some((*prompt_repo).clone()))?);
        
        Ok(Self {
            planner,
            prompt_repo,
            config,
        })
    }

    /// 评估是否需要重新规划 - DISABLED
    pub async fn should_replan(
        &self,
        _execution_context: &ExecutionContext,
        _failed_steps: &[StepExecutionResult],
    ) -> Result<bool, PlanAndExecuteError> {
        // DISABLED: Always return false
        Ok(false)
    }

    /// 执行重新规划 - DISABLED
    pub async fn replan(
        &self,
        _original_plan: &ExecutionPlan,
        _execution_context: &ExecutionContext,
        _failed_steps: &[StepExecutionResult],
        _replan_reason: &str,
    ) -> Result<ExecutionPlan, PlanAndExecuteError> {
        Err(PlanAndExecuteError::ReplanningFailed(
            "Replanner disabled - ai_adapter removed, needs Rig refactor".to_string()
        ))
    }

    /// 调用AI进行重新规划决策 - DISABLED
    async fn call_ai_for_replan_decision(&self, _prompt: &str) -> Result<String, PlanAndExecuteError> {
        Err(PlanAndExecuteError::ReplanningFailed(
            "Replanner disabled - ai_adapter removed, needs Rig refactor".to_string()
        ))
    }
}