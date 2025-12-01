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

    // ===== 增强的智能重规划功能 =====

    /// 分析执行结果并确定最佳重规划策略
    pub async fn analyze_and_determine_strategy(
        &self,
        original_plan: &ExecutionPlan,
        execution_result: &ExecResult,
    ) -> ReplanStrategy {
        let mut strategy = ReplanStrategy::default();
        
        // 分析失败模式
        let failure_analysis = self.analyze_failure_patterns(execution_result).await;
        
        // 根据失败分析确定策略
        match failure_analysis.primary_cause {
            FailureCause::ResourceUnavailable => {
                strategy.action = ReplanAction::ReplaceFailedTools;
                strategy.priority = ReplanPriority::High;
                strategy.suggestions.push("替换不可用的工具为备选方案".to_string());
            }
            FailureCause::Timeout => {
                strategy.action = ReplanAction::SimplifyPlan;
                strategy.priority = ReplanPriority::Medium;
                strategy.suggestions.push("简化执行计划，减少长时间运行的步骤".to_string());
            }
            FailureCause::DependencyFailure => {
                strategy.action = ReplanAction::ReorderSteps;
                strategy.priority = ReplanPriority::High;
                strategy.suggestions.push("重新排序步骤以修复依赖问题".to_string());
            }
            FailureCause::ValidationError => {
                strategy.action = ReplanAction::AddValidationSteps;
                strategy.priority = ReplanPriority::Medium;
                strategy.suggestions.push("添加额外的验证步骤".to_string());
            }
            FailureCause::Unknown => {
                strategy.action = ReplanAction::FullReplan;
                strategy.priority = ReplanPriority::Low;
                strategy.suggestions.push("需要完全重新规划".to_string());
            }
        }
        
        // 检查是否需要添加清理步骤
        if failure_analysis.has_resource_leak {
            strategy.requires_cleanup = true;
            strategy.cleanup_steps = self.generate_cleanup_steps(execution_result).await;
        }
        
        // 计算重规划置信度
        strategy.confidence = self.calculate_strategy_confidence(&failure_analysis).await;
        
        log::info!(
            "Replan strategy determined: action={:?}, priority={:?}, confidence={:.2}",
            strategy.action, strategy.priority, strategy.confidence
        );
        
        strategy
    }

    /// 分析失败模式
    async fn analyze_failure_patterns(&self, result: &ExecResult) -> FailureAnalysis {
        let mut analysis = FailureAnalysis::default();
        
        // 分析错误类型
        let mut timeout_count = 0;
        let mut tool_errors = 0;
        let mut validation_errors = 0;
        
        for error in &result.errors {
            match error.error_type {
                crate::engines::types::ErrorType::Timeout => timeout_count += 1,
                crate::engines::types::ErrorType::Tool => tool_errors += 1,
                crate::engines::types::ErrorType::Configuration => validation_errors += 1,
                _ => {}
            }
        }
        
        // 确定主要原因
        analysis.primary_cause = if timeout_count > tool_errors && timeout_count > validation_errors {
            FailureCause::Timeout
        } else if tool_errors > validation_errors {
            FailureCause::ResourceUnavailable
        } else if validation_errors > 0 {
            FailureCause::ValidationError
        } else if !result.failed_steps.is_empty() {
            FailureCause::DependencyFailure
        } else {
            FailureCause::Unknown
        };
        
        // 检查资源泄露
        analysis.has_resource_leak = self.check_potential_resource_leak(result).await;
        
        // 分析失败步骤
        analysis.failed_step_names = result.failed_steps.clone();
        analysis.failure_rate = if result.completed_steps.len() + result.failed_steps.len() > 0 {
            result.failed_steps.len() as f64 / 
            (result.completed_steps.len() + result.failed_steps.len()) as f64
        } else {
            0.0
        };
        
        analysis
    }

    /// 检查是否有潜在的资源泄露
    async fn check_potential_resource_leak(&self, result: &ExecResult) -> bool {
        // 检查是否有浏览器会话未关闭
        let has_browser_open = result.completed_steps.iter()
            .any(|s| s.contains("playwright_navigate") || s.contains("browser_open"));
        let has_browser_close = result.completed_steps.iter()
            .any(|s| s.contains("playwright_close") || s.contains("browser_close"));
        
        // 检查是否有代理未停止
        let has_proxy_start = result.completed_steps.iter()
            .any(|s| s.contains("start_passive_scan"));
        let has_proxy_stop = result.completed_steps.iter()
            .any(|s| s.contains("stop_passive_scan"));
        
        (has_browser_open && !has_browser_close) || (has_proxy_start && !has_proxy_stop)
    }

    /// 生成清理步骤
    async fn generate_cleanup_steps(&self, result: &ExecResult) -> Vec<ExecutionStep> {
        let mut cleanup_steps = Vec::new();
        
        // 检查需要清理的浏览器会话
        let has_browser_open = result.completed_steps.iter()
            .any(|s| s.contains("playwright_navigate") || s.contains("browser_open"));
        let has_browser_close = result.completed_steps.iter()
            .any(|s| s.contains("playwright_close") || s.contains("browser_close"));
        
        if has_browser_open && !has_browser_close {
            cleanup_steps.push(ExecutionStep {
                id: format!("cleanup_browser_{}", uuid::Uuid::new_v4()),
                name: "Close browser session".to_string(),
                description: "Clean up leaked browser session".to_string(),
                step_type: StepType::ToolCall,
                tool_config: Some(ToolConfig {
                    tool_name: "playwright_close".to_string(),
                    tool_version: None,
                    tool_args: std::collections::HashMap::new(),
                    timeout: Some(30),
                    env_vars: std::collections::HashMap::new(),
                }),
                parameters: std::collections::HashMap::new(),
                estimated_duration: 30,
                retry_config: RetryConfig::default(),
                preconditions: vec![],
                postconditions: vec![],
            });
        }
        
        // 检查需要清理的代理
        let has_proxy_start = result.completed_steps.iter()
            .any(|s| s.contains("start_passive_scan"));
        let has_proxy_stop = result.completed_steps.iter()
            .any(|s| s.contains("stop_passive_scan"));
        
        if has_proxy_start && !has_proxy_stop {
            cleanup_steps.push(ExecutionStep {
                id: format!("cleanup_proxy_{}", uuid::Uuid::new_v4()),
                name: "Stop passive scan proxy".to_string(),
                description: "Clean up leaked proxy session".to_string(),
                step_type: StepType::ToolCall,
                tool_config: Some(ToolConfig {
                    tool_name: "stop_passive_scan".to_string(),
                    tool_version: None,
                    tool_args: std::collections::HashMap::new(),
                    timeout: Some(30),
                    env_vars: std::collections::HashMap::new(),
                }),
                parameters: std::collections::HashMap::new(),
                estimated_duration: 30,
                retry_config: RetryConfig::default(),
                preconditions: vec![],
                postconditions: vec![],
            });
        }
        
        cleanup_steps
    }

    /// 计算策略置信度
    async fn calculate_strategy_confidence(&self, analysis: &FailureAnalysis) -> f64 {
        let mut confidence: f64 = 0.5; // 基础置信度
        
        // 根据失败原因调整
        match analysis.primary_cause {
            FailureCause::Timeout => confidence += 0.2, // 超时问题通常容易解决
            FailureCause::ResourceUnavailable => confidence += 0.1,
            FailureCause::ValidationError => confidence += 0.15,
            FailureCause::DependencyFailure => confidence += 0.1,
            FailureCause::Unknown => confidence -= 0.2,
        }
        
        // 根据失败率调整
        if analysis.failure_rate < 0.3 {
            confidence += 0.2; // 低失败率，更容易修复
        } else if analysis.failure_rate > 0.7 {
            confidence -= 0.2; // 高失败率，可能需要完全重做
        }
        
        confidence.max(0.0_f64).min(1.0_f64)
    }

    /// 应用重规划策略生成新计划
    pub async fn apply_strategy(
        &self,
        original_plan: &ExecutionPlan,
        execution_result: &ExecResult,
        strategy: &ReplanStrategy,
    ) -> Result<ReplanResult, PlanAndExecuteError> {
        log::info!("Applying replan strategy: {:?}", strategy.action);
        
        match strategy.action {
            ReplanAction::SimplifyPlan => {
                self.simplify_plan(original_plan, execution_result).await
            }
            ReplanAction::ReplaceFailedTools => {
                self.replace_failed_tools(original_plan, execution_result).await
            }
            ReplanAction::ReorderSteps => {
                self.reorder_steps(original_plan, execution_result).await
            }
            ReplanAction::AddValidationSteps => {
                self.add_validation_steps(original_plan, execution_result).await
            }
            ReplanAction::FullReplan => {
                // 对于完全重规划，我们返回None让上层调用replan_with_planner
                Ok(ReplanResult {
                    should_replan: true,
                    replan_reason: "Full replan required - use replan_with_planner".to_string(),
                    new_plan: None,
                })
            }
        }
    }

    /// 简化计划（移除不必要的步骤）
    async fn simplify_plan(
        &self,
        original_plan: &ExecutionPlan,
        result: &ExecResult,
    ) -> Result<ReplanResult, PlanAndExecuteError> {
        let failed_set: std::collections::HashSet<String> = result.failed_steps.iter().cloned().collect();
        
        // 保留成功的步骤，移除失败的步骤
        let simplified_steps: Vec<ExecutionStep> = original_plan.steps.iter()
            .filter(|s| !failed_set.contains(&s.id))
            .take(5) // 限制最多5个步骤
            .cloned()
            .collect();
        
        if simplified_steps.is_empty() {
            return Ok(ReplanResult {
                should_replan: false,
                replan_reason: "Cannot simplify: no successful steps".to_string(),
                new_plan: None,
            });
        }
        
        let new_plan = ExecutionPlan {
            id: uuid::Uuid::new_v4().to_string(),
            task_id: original_plan.task_id.clone(),
            name: format!("{} (simplified)", original_plan.name),
            description: "Simplified plan with reduced complexity".to_string(),
            steps: simplified_steps,
            estimated_duration: original_plan.estimated_duration / 2,
            created_at: std::time::SystemTime::now(),
            dependencies: std::collections::HashMap::new(),
            metadata: original_plan.metadata.clone(),
        };
        
        Ok(ReplanResult {
            should_replan: true,
            replan_reason: "Plan simplified to remove problematic steps".to_string(),
            new_plan: Some(new_plan),
        })
    }

    /// 替换失败的工具
    async fn replace_failed_tools(
        &self,
        original_plan: &ExecutionPlan,
        result: &ExecResult,
    ) -> Result<ReplanResult, PlanAndExecuteError> {
        let failed_set: std::collections::HashSet<String> = result.failed_steps.iter().cloned().collect();
        
        let mut new_steps = Vec::new();
        for step in &original_plan.steps {
            if failed_set.contains(&step.id) {
                // 对于失败的工具调用步骤，转换为AI推理步骤
                if matches!(step.step_type, StepType::ToolCall) {
                    let mut alt_step = step.clone();
                    alt_step.id = format!("{}_alt", step.id);
                    alt_step.name = format!("{} (alternative)", step.name);
                    alt_step.step_type = StepType::AiReasoning;
                    alt_step.tool_config = None;
                    alt_step.description = format!(
                        "Use AI reasoning as alternative for failed tool: {}",
                        step.description
                    );
                    new_steps.push(alt_step);
                }
            } else {
                new_steps.push(step.clone());
            }
        }
        
        let new_plan = ExecutionPlan {
            id: uuid::Uuid::new_v4().to_string(),
            task_id: original_plan.task_id.clone(),
            name: format!("{} (tools replaced)", original_plan.name),
            description: "Plan with failed tools replaced by alternatives".to_string(),
            steps: new_steps,
            estimated_duration: original_plan.estimated_duration,
            created_at: std::time::SystemTime::now(),
            dependencies: original_plan.dependencies.clone(),
            metadata: original_plan.metadata.clone(),
        };
        
        Ok(ReplanResult {
            should_replan: true,
            replan_reason: "Failed tools replaced with AI reasoning alternatives".to_string(),
            new_plan: Some(new_plan),
        })
    }

    /// 重新排序步骤
    async fn reorder_steps(
        &self,
        original_plan: &ExecutionPlan,
        result: &ExecResult,
    ) -> Result<ReplanResult, PlanAndExecuteError> {
        // 将成功的步骤放在前面
        let mut successful_steps: Vec<ExecutionStep> = original_plan.steps.iter()
            .filter(|s| result.completed_steps.contains(&s.id))
            .cloned()
            .collect();
        
        let mut other_steps: Vec<ExecutionStep> = original_plan.steps.iter()
            .filter(|s| !result.completed_steps.contains(&s.id) && !result.failed_steps.contains(&s.id))
            .cloned()
            .collect();
        
        let mut reordered = Vec::new();
        reordered.append(&mut successful_steps);
        reordered.append(&mut other_steps);
        
        // 添加恢复步骤
        reordered.push(ExecutionStep {
            id: format!("recovery_{}", uuid::Uuid::new_v4()),
            name: "Recovery analysis".to_string(),
            description: "Analyze previous results and recover".to_string(),
            step_type: StepType::AiReasoning,
            tool_config: None,
            parameters: std::collections::HashMap::new(),
            estimated_duration: 60,
            retry_config: RetryConfig::default(),
            preconditions: vec![],
            postconditions: vec![],
        });
        
        let new_plan = ExecutionPlan {
            id: uuid::Uuid::new_v4().to_string(),
            task_id: original_plan.task_id.clone(),
            name: format!("{} (reordered)", original_plan.name),
            description: "Plan with reordered steps for better execution".to_string(),
            steps: reordered,
            estimated_duration: original_plan.estimated_duration,
            created_at: std::time::SystemTime::now(),
            dependencies: std::collections::HashMap::new(),
            metadata: original_plan.metadata.clone(),
        };
        
        Ok(ReplanResult {
            should_replan: true,
            replan_reason: "Steps reordered based on execution success".to_string(),
            new_plan: Some(new_plan),
        })
    }

    /// 添加验证步骤
    async fn add_validation_steps(
        &self,
        original_plan: &ExecutionPlan,
        _result: &ExecResult,
    ) -> Result<ReplanResult, PlanAndExecuteError> {
        let mut new_steps = Vec::new();
        
        for (i, step) in original_plan.steps.iter().enumerate() {
            new_steps.push(step.clone());
            
            // 在每个工具调用后添加验证步骤
            if matches!(step.step_type, StepType::ToolCall) {
                new_steps.push(ExecutionStep {
                    id: format!("validate_{}", step.id),
                    name: format!("Validate {}", step.name),
                    description: format!("Validate output of step: {}", step.name),
                    step_type: StepType::AiReasoning,
                    tool_config: None,
                    parameters: std::collections::HashMap::new(),
                    estimated_duration: 30,
                    retry_config: RetryConfig::default(),
                    preconditions: vec![format!("completed({})", step.id)],
                    postconditions: vec![],
                });
            }
        }
        
        let new_plan = ExecutionPlan {
            id: uuid::Uuid::new_v4().to_string(),
            task_id: original_plan.task_id.clone(),
            name: format!("{} (with validation)", original_plan.name),
            description: "Plan with additional validation steps".to_string(),
            steps: new_steps,
            estimated_duration: original_plan.estimated_duration * 3 / 2, // 增加50%时间
            created_at: std::time::SystemTime::now(),
            dependencies: original_plan.dependencies.clone(),
            metadata: original_plan.metadata.clone(),
        };
        
        Ok(ReplanResult {
            should_replan: true,
            replan_reason: "Added validation steps after tool calls".to_string(),
            new_plan: Some(new_plan),
        })
    }
}

// ===== 重规划相关数据结构 =====

/// 重规划策略
#[derive(Debug, Clone, Default)]
pub struct ReplanStrategy {
    /// 重规划动作
    pub action: ReplanAction,
    /// 优先级
    pub priority: ReplanPriority,
    /// 建议
    pub suggestions: Vec<String>,
    /// 置信度
    pub confidence: f64,
    /// 是否需要清理
    pub requires_cleanup: bool,
    /// 清理步骤
    pub cleanup_steps: Vec<ExecutionStep>,
}

/// 重规划动作
#[derive(Debug, Clone, Default)]
pub enum ReplanAction {
    /// 简化计划
    SimplifyPlan,
    /// 替换失败的工具
    ReplaceFailedTools,
    /// 重新排序步骤
    ReorderSteps,
    /// 添加验证步骤
    AddValidationSteps,
    /// 完全重新规划
    #[default]
    FullReplan,
}

/// 重规划优先级
#[derive(Debug, Clone, Default)]
pub enum ReplanPriority {
    /// 高优先级
    High,
    /// 中优先级
    #[default]
    Medium,
    /// 低优先级
    Low,
}

/// 失败分析结果
#[derive(Debug, Clone, Default)]
pub struct FailureAnalysis {
    /// 主要失败原因
    pub primary_cause: FailureCause,
    /// 失败的步骤名称
    pub failed_step_names: Vec<String>,
    /// 失败率
    pub failure_rate: f64,
    /// 是否有资源泄露
    pub has_resource_leak: bool,
}

/// 失败原因
#[derive(Debug, Clone, Default)]
pub enum FailureCause {
    /// 资源不可用
    ResourceUnavailable,
    /// 超时
    Timeout,
    /// 依赖失败
    DependencyFailure,
    /// 验证错误
    ValidationError,
    /// 未知原因
    #[default]
    Unknown,
}