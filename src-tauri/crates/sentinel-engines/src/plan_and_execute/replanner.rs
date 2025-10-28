//! Replanner 组件 - 重新规划器（简化启用版）
//!
//! 负责在执行过程中动态调整计划，处理异常情况和优化执行策略。
//! 目前提供一个“规则驱动”的简化重规划：
//! - 当存在失败步骤时，基于原计划进行裁剪并追加恢复步骤，生成一个新计划。
//! - 不依赖旧的 ai_adapter；如需更智能的重规划，可后续接入 Planner + AiService。

use crate::engines::plan_and_execute::types::*;
use crate::engines::plan_and_execute::planner::{Planner, PlannerConfig};
use crate::engines::plan_and_execute::executor::ExecutionContext;
use crate::engines::plan_and_execute::executor::ExecutionResult as ExecResult;
use crate::engines::StepExecutionResult;
use crate::services::prompt_db::PromptRepository;
use crate::services::ai::AiServiceManager;
use std::sync::Arc;

/// 重新规划器（简化启用版）
#[derive(Debug)]
pub struct Replanner {
    /// 规划器实例
    planner: Arc<Planner>,
    /// 提示词仓库
    prompt_repo: Arc<PromptRepository>,
    /// 配置
    config: ReplannerConfig,
    /// 预留：AI服务管理器（未来用于真实语义嵌入）
    ai_service_manager: Option<Arc<AiServiceManager>>, 
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

/// 重新规划触发器
#[derive(Debug, Clone)]
pub enum ReplanTrigger {
    /// 步骤执行失败
    StepFailure,
    /// 执行超时
    Timeout,
    /// 手动触发
    Manual,
}

/// 重新规划结果
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
    /// 创建新的重新规划器（启用 Planner，如无 AI 服务也可工作在简化模式）
    pub async fn new(
        ai_service_manager: Arc<AiServiceManager>,
        prompt_repo: Arc<PromptRepository>,
        config: ReplannerConfig,
    ) -> Result<Self, PlanAndExecuteError> {
        let planner_config = PlannerConfig::default();
        // 优先启用带 AI 服务的 Planner，失败则回退到无 AI 的 Planner
        let planner = match crate::engines::plan_and_execute::planner::Planner::with_ai_service_manager(
            planner_config.clone(),
            Some((*prompt_repo).clone()),
            None,
            ai_service_manager.clone(),
            None,
        ) {
            Ok(p) => Arc::new(p),
            Err(e) => {
                log::warn!("Failed to create Planner with AI service: {}. Falling back to basic Planner", e);
                Arc::new(Planner::new(planner_config, Some((*prompt_repo).clone()))?)
            }
        };

        Ok(Self {
            planner,
            prompt_repo,
            config,
            ai_service_manager: Some(ai_service_manager),
        })
    }

    /// 评估是否需要重新规划（简化版）
    pub async fn should_replan(
        &self,
        _execution_context: &ExecutionContext,
        failed_steps: &[StepExecutionResult],
    ) -> Result<bool, PlanAndExecuteError> {
        if !self.config.auto_replan_enabled {
            return Ok(false);
        }
        Ok(!failed_steps.is_empty())
    }

    /// 基于原计划生成一个简化的“恢复计划”：
    /// - 移除已失败的步骤；
    /// - 追加一个简短的 AiReasoning 恢复/总结步骤；
    /// - 改名并更新时间，确保与旧计划相似度降低。
    pub async fn replan_simple(
        &self,
        original_plan: &ExecutionPlan,
        execution_result: &ExecResult,
    ) -> Result<ReplanResult, anyhow::Error> {
        let failed: std::collections::HashSet<String> = execution_result
            .failed_steps
            .iter()
            .cloned()
            .collect();

        if failed.is_empty() {
            return Ok(ReplanResult {
                should_replan: false,
                replan_reason: "No failed steps detected".to_string(),
                new_plan: None,
            });
        }

        // 裁剪失败步骤
        let mut new_steps: Vec<ExecutionStep> = original_plan
            .steps
            .iter()
            .cloned()
            .filter(|s| !failed.contains(&s.id))
            .collect();

        // 追加一个恢复/总结步骤，帮助产出最后结果
        let recovery_id = format!("step_{}_recovery", new_steps.len() + 1);
        new_steps.push(ExecutionStep {
            id: recovery_id,
            name: "Recovery reasoning".to_string(),
            description: "Analyze previous results and produce a concise recovery summary".to_string(),
            step_type: StepType::AiReasoning,
            tool_config: None,
            parameters: std::collections::HashMap::new(),
            estimated_duration: 60,
            retry_config: RetryConfig::default(),
            preconditions: vec![],
            postconditions: vec![],
        });

        if new_steps.is_empty() {
            // 所有步骤都失败，构造一个最小的恢复步骤计划
            let minimal = ExecutionPlan {
                id: uuid::Uuid::new_v4().to_string(),
                task_id: original_plan.task_id.clone(),
                name: format!("{} (replanned)", original_plan.name),
                description: format!(
                    "Replanned due to failures in original plan: {}",
                    original_plan.name
                ),
                steps: vec![ExecutionStep {
                    id: "step_1_recovery".to_string(),
                    name: "Recovery reasoning".to_string(),
                    description: "Produce a minimal viable outcome".to_string(),
                    step_type: StepType::AiReasoning,
                    tool_config: None,
                    parameters: std::collections::HashMap::new(),
                    estimated_duration: 60,
                    retry_config: RetryConfig::default(),
                    preconditions: vec![],
                    postconditions: vec![],
                }],
                estimated_duration: 60,
                created_at: std::time::SystemTime::now(),
                dependencies: std::collections::HashMap::new(),
                metadata: std::collections::HashMap::new(),
            };

            return Ok(ReplanResult {
                should_replan: true,
                replan_reason: "All steps failed; creating minimal recovery plan".to_string(),
                new_plan: Some(minimal),
            });
        }

        let new_plan = ExecutionPlan {
            id: uuid::Uuid::new_v4().to_string(),
            task_id: original_plan.task_id.clone(),
            name: format!("{} (replanned)", original_plan.name),
            description: format!(
                "Replanned after {} failed steps",
                execution_result.failed_steps.len()
            ),
            steps: new_steps,
            estimated_duration: original_plan.estimated_duration, // 简化处理
            created_at: std::time::SystemTime::now(),
            dependencies: original_plan.dependencies.clone(),
            metadata: original_plan.metadata.clone(),
        };

        Ok(ReplanResult {
            should_replan: true,
            replan_reason: "Failed steps removed and recovery step appended".to_string(),
            new_plan: Some(new_plan),
        })
    }

    /// 使用 Planner 生成真正的新计划（严格验收）
    pub async fn replan_with_planner(
        &self,
        original_plan: &ExecutionPlan,
        task_request: &crate::engines::plan_and_execute::types::TaskRequest,
        execution_result: &ExecResult,
    ) -> Result<ReplanResult, PlanAndExecuteError> {
        // 构造一个带有重规划意图的任务名称，便于Planner生成不同的方案
        let failed_list = if execution_result.failed_steps.is_empty() {
            "none".to_string()
        } else {
            execution_result.failed_steps.join(", ")
        };

        let mut replan_task = task_request.clone();
        // 注入失败上下文与工具可用性约束
        replan_task.name = format!(
            "REPLAN: {} | failed_steps: [{}] | requirements: generate a revised JSON plan to avoid previous failures, prefer allowed tools only, and include verification",
            original_plan.name, failed_list
        );
        let mut params = replan_task.parameters.clone();
        // 将失败步骤名与可用工具白名单透传给 Planner（若上游已有则保留）
        let failed_names: Vec<String> = original_plan.steps.iter()
            .filter(|s| execution_result.failed_steps.contains(&s.id))
            .map(|s| s.name.clone()).collect();
        params.insert("replan_failed_step_names".to_string(), serde_json::json!(failed_names));
        // 工具白名单沿用任务参数里的 tools_allow；若无则为空数组表示严格限制
        if !params.contains_key("tools_allow") {
            params.insert("tools_allow".to_string(), serde_json::json!([] as [String; 0]));
        }
        replan_task.parameters = params;

        // 调用 Planner 生成新计划
        let planning = self
            .planner
            .create_plan(&replan_task)
            .await
            .map_err(|e| PlanAndExecuteError::ReplanningFailed(e.to_string()))?;

        let mut new_plan = planning.plan;

        // 严格验收与断言：
        // 1) 相似度不得高于阈值；2) 步骤非空且数量合理；3) 为 ToolCall 步骤追加非空输出断言
        if !self.accept_new_plan_strict(original_plan, &new_plan).await {
            return Ok(ReplanResult {
                should_replan: false,
                replan_reason: "New plan rejected by strict acceptance checks".to_string(),
                new_plan: None,
            });
        }

        self.augment_postconditions(&mut new_plan).await;

        Ok(ReplanResult {
            should_replan: true,
            replan_reason: "Planner produced a validated new plan".to_string(),
            new_plan: Some(new_plan),
        })
    }

    async fn accept_new_plan_strict(
        &self,
        original: &ExecutionPlan,
        candidate: &ExecutionPlan,
    ) -> bool {
        // 基本检查
        if candidate.steps.is_empty() || candidate.steps.len() > 20 {
            log::warn!("Replan acceptance failed: empty or too many steps");
            return false;
        }

        // 最后一步建议为推理总结（Planner已处理，但这里再校验一次）
        if let Some(last) = candidate.steps.last() {
            if last.step_type != StepType::AiReasoning {
                log::warn!("Replan acceptance failed: last step is not AiReasoning");
                return false;
            }
        }

        // 语义相似度校验（越低越好）
        let sim = self.calculate_plan_semantic_similarity(original, candidate).await;
        if sim > self.config.replan_threshold {
            log::warn!(
                "Replan acceptance failed: similarity {:.2} exceeds threshold {:.2}",
                sim, self.config.replan_threshold
            );
            return false;
        }

        // 步骤完整性
        for step in &candidate.steps {
            if step.name.trim().is_empty() || step.description.trim().is_empty() {
                log::warn!("Replan acceptance failed: step missing name/description");
                return false;
            }
        }

        true
    }

    async fn calculate_plan_similarity(&self, p1: &ExecutionPlan, p2: &ExecutionPlan) -> f64 {
        if p1.steps.is_empty() && p2.steps.is_empty() {
            return 1.0;
        }
        if p1.steps.is_empty() || p2.steps.is_empty() {
            return 0.0;
        }
        let mut matches = 0;
        let max_steps = p1.steps.len().max(p2.steps.len());
        for i in 0..p1.steps.len().min(p2.steps.len()) {
            let s1 = &p1.steps[i];
            let s2 = &p2.steps[i];
            if s1.name == s2.name && s1.step_type == s2.step_type {
                matches += 1;
            }
        }
        matches as f64 / max_steps as f64
    }

    /// 语义级相似度：对步骤名称做轻量嵌入并计算均值余弦相似
    async fn calculate_plan_semantic_similarity(
        &self,
        p1: &ExecutionPlan,
        p2: &ExecutionPlan,
    ) -> f64 {
        if p1.steps.is_empty() && p2.steps.is_empty() {
            return 1.0;
        }
        if p1.steps.is_empty() || p2.steps.is_empty() {
            return 0.0;
        }

        // 计算每个步骤名称的嵌入（本地轻量化：字符n-gram哈希嵌入）
        let e1: Vec<Vec<f32>> = p1
            .steps
            .iter()
            .map(|s| self.compute_text_embedding(&s.name))
            .collect();
        let e2: Vec<Vec<f32>> = p2
            .steps
            .iter()
            .map(|s| self.compute_text_embedding(&s.name))
            .collect();

        let len = e1.len().min(e2.len());
        if len == 0 { return 0.0; }

        let mut sum_sim = 0.0f32;
        for i in 0..len {
            sum_sim += self.cosine_similarity(&e1[i], &e2[i]);
        }
        (sum_sim / len as f32) as f64
    }

    fn compute_text_embedding(&self, text: &str) -> Vec<f32> {
        // 轻量本地嵌入：字符3-gram哈希到固定维度向量
        const DIM: usize = 64;
        let mut vec = vec![0f32; DIM];
        let lower = text.to_lowercase();
        let chars: Vec<char> = lower.chars().collect();
        for i in 0..chars.len().saturating_sub(2) {
            let tri = [chars[i], chars[i + 1], chars[i + 2]];
            let mut h: u64 = 1469598103934665603; // FNV offset
            for c in tri {
                h ^= c as u64;
                h = h.wrapping_mul(1099511628211);
            }
            let idx = (h as usize) % DIM;
            vec[idx] += 1.0;
        }
        // 归一化
        let norm = (vec.iter().map(|v| v * v).sum::<f32>()).sqrt();
        if norm > 0.0 {
            for v in vec.iter_mut() { *v /= norm; }
        }
        vec
    }

    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() { return 0.0; }
        let mut dot = 0.0f32;
        let mut na = 0.0f32;
        let mut nb = 0.0f32;
        for i in 0..a.len() { dot += a[i] * b[i]; na += a[i]*a[i]; nb += b[i]*b[i]; }
        if na == 0.0 || nb == 0.0 { 0.0 } else { dot / (na.sqrt() * nb.sqrt()) }
    }

    async fn augment_postconditions(&self, plan: &mut ExecutionPlan) {
        for step in &mut plan.steps {
            if let StepType::ToolCall = step.step_type {
                let cond = format!("non_empty_output({})", step.name);
                if !step.postconditions.iter().any(|c| c == &cond) {
                    step.postconditions.push(cond);
                }
            }
        }
    }
}