//! 重规划引擎
//!
//! 实现 Plan-and-Execute 风格的自适应重规划能力：
//! - 执行结果评估
//! - 智能重规划触发
//! - 计划调整和优化
//! - 错误恢复策略

use super::types::*;
use super::dag_planner::DagPlanner;
use super::context_manager::ContextManager;
use crate::engines::LlmClient;
use crate::services::ai::AiService;
use crate::services::prompt_db::PromptRepository;
use crate::tools::FrameworkToolAdapter;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;

/// 重规划引擎
pub struct ReplanningEngine {
    /// AI 服务
    ai_service: Arc<AiService>,
    /// Prompt 仓库
    prompt_repo: Option<Arc<PromptRepository>>,
    /// 工具适配器
    tool_adapter: Option<Arc<dyn FrameworkToolAdapter>>,
    /// 配置
    config: ReplanningConfig,
    /// 上下文管理器
    context_manager: ContextManager,
}

/// 重规划配置
#[derive(Debug, Clone)]
pub struct ReplanningConfig {
    /// 最大重规划次数
    pub max_replan_count: u32,
    /// 重规划冷却时间 (毫秒)
    pub replan_cooldown_ms: u64,
    /// 成功阈值 (低于此值触发重规划评估)
    pub success_threshold: f32,
    /// 启用自动重规划
    pub auto_replan: bool,
    /// 重规划评估间隔 (每N个任务评估一次)
    pub evaluation_interval: u32,
    /// 启用学习模式 (从历史中学习)
    pub learning_mode: bool,
}

impl Default for ReplanningConfig {
    fn default() -> Self {
        Self {
            max_replan_count: 5,
            replan_cooldown_ms: 5000,
            success_threshold: 0.6,
            auto_replan: true,
            evaluation_interval: 3,
            learning_mode: true,
        }
    }
}

/// 重规划评估结果
#[derive(Debug, Clone)]
pub struct ReplanEvaluation {
    /// 是否需要重规划
    pub should_replan: bool,
    /// 重规划原因
    pub reason: Option<ReplanReason>,
    /// 当前进度评分 (0-1)
    pub progress_score: f32,
    /// 问题诊断
    pub diagnosis: Vec<ProblemDiagnosis>,
    /// 建议的调整
    pub suggested_adjustments: Vec<PlanAdjustment>,
}

/// 问题诊断
#[derive(Debug, Clone)]
pub struct ProblemDiagnosis {
    /// 问题类型
    pub problem_type: ProblemType,
    /// 问题描述
    pub description: String,
    /// 受影响的任务
    pub affected_tasks: Vec<String>,
    /// 严重程度 (0-1)
    pub severity: f32,
}

/// 问题类型
#[derive(Debug, Clone)]
pub enum ProblemType {
    /// 任务失败
    TaskFailure,
    /// 目标不可达
    UnreachableTarget,
    /// 资源不足
    ResourceExhausted,
    /// 循环无效操作
    IneffectiveLoop,
    /// 新发现需要处理
    NewDiscovery,
    /// 策略无效
    IneffectiveStrategy,
    /// 超时
    Timeout,
}

/// 计划调整
#[derive(Debug, Clone)]
pub struct PlanAdjustment {
    /// 调整类型
    pub adjustment_type: AdjustmentType,
    /// 描述
    pub description: String,
    /// 涉及的任务
    pub task_ids: Vec<String>,
    /// 新任务 (如果是添加类型)
    pub new_tasks: Vec<DagTask>,
}

/// 调整类型
#[derive(Debug, Clone)]
pub enum AdjustmentType {
    /// 添加新任务
    AddTasks,
    /// 移除任务
    RemoveTasks,
    /// 修改任务参数
    ModifyTasks,
    /// 调整执行顺序
    ReorderTasks,
    /// 添加条件分支
    AddCondition,
    /// 替换失败的任务
    ReplaceFailedTask,
    /// 完全重新规划
    FullReplan,
}

impl ReplanningEngine {
    pub fn new(ai_service: Arc<AiService>, config: ReplanningConfig) -> Self {
        Self {
            ai_service,
            prompt_repo: None,
            tool_adapter: None,
            config,
            context_manager: ContextManager::default(),
        }
    }

    pub fn with_prompt_repo(mut self, repo: Arc<PromptRepository>) -> Self {
        self.prompt_repo = Some(repo);
        self
    }

    pub fn with_tool_adapter(mut self, adapter: Arc<dyn FrameworkToolAdapter>) -> Self {
        self.tool_adapter = Some(adapter);
        self
    }

    /// 评估是否需要重规划
    pub async fn evaluate_replan_need(
        &self,
        current_plan: &DagPlan,
        execution_snapshot: &ExecutionSnapshot,
        recent_results: &[TaskExecutionSummary],
    ) -> Result<ReplanEvaluation> {
        log::info!("ReplanningEngine: Evaluating replan need");

        let mut diagnosis = Vec::new();
        let mut suggested_adjustments = Vec::new();

        // 1. 分析任务失败情况
        let failure_analysis = self.analyze_failures(execution_snapshot);
        diagnosis.extend(failure_analysis.diagnosis);

        // 2. 检测无效循环
        let loop_analysis = self.detect_ineffective_loops(recent_results);
        if let Some(loop_problem) = loop_analysis {
            diagnosis.push(loop_problem);
        }

        // 3. 评估进度
        let progress_score = self.calculate_progress_score(current_plan, execution_snapshot);

        // 4. 检查是否有新发现需要处理
        let discovery_analysis = self.analyze_discoveries(execution_snapshot);
        diagnosis.extend(discovery_analysis);

        // 5. 决定是否需要重规划
        let should_replan = self.should_trigger_replan(&diagnosis, progress_score);

        // 6. 如果需要重规划，生成调整建议
        if should_replan {
            suggested_adjustments = self.generate_adjustments(&diagnosis, current_plan, execution_snapshot).await?;
        }

        // 确定主要重规划原因
        let reason = if should_replan {
            diagnosis.first().map(|d| match d.problem_type {
                ProblemType::TaskFailure => ReplanReason::TaskFailed {
                    task_id: d.affected_tasks.first().cloned().unwrap_or_default(),
                    error: d.description.clone(),
                },
                ProblemType::UnreachableTarget => ReplanReason::TargetUnreachable {
                    original_target: d.description.clone(),
                },
                ProblemType::ResourceExhausted => ReplanReason::ResourceExhausted {
                    resource: d.description.clone(),
                },
                ProblemType::IneffectiveLoop => ReplanReason::IneffectiveLoop {
                    iterations: d.affected_tasks.len() as u32,
                },
                ProblemType::NewDiscovery => ReplanReason::NewDiscovery {
                    description: d.description.clone(),
                    data: serde_json::json!({}),
                },
                _ => ReplanReason::UserRequested {
                    reason: d.description.clone(),
                },
            })
        } else {
            None
        };

        Ok(ReplanEvaluation {
            should_replan,
            reason,
            progress_score,
            diagnosis,
            suggested_adjustments,
        })
    }

    /// 执行重规划
    pub async fn replan(
        &self,
        original_task: &str,
        context: &HashMap<String, serde_json::Value>,
        execution_snapshot: &ExecutionSnapshot,
        evaluation: &ReplanEvaluation,
    ) -> Result<DagPlan> {
        log::info!("ReplanningEngine: Executing replan");

        let llm_client = crate::engines::create_client(self.ai_service.as_ref());

        // 根据评估结果选择重规划策略
        let strategy = self.select_replan_strategy(evaluation);

        match strategy {
            ReplanStrategy::IncrementalAdjust => {
                self.incremental_replan(&llm_client, original_task, context, execution_snapshot, evaluation).await
            }
            ReplanStrategy::PartialReplan => {
                self.partial_replan(&llm_client, original_task, context, execution_snapshot, evaluation).await
            }
            ReplanStrategy::FullReplan => {
                self.full_replan(&llm_client, original_task, context, execution_snapshot).await
            }
        }
    }

    /// 分析失败情况
    fn analyze_failures(&self, snapshot: &ExecutionSnapshot) -> FailureAnalysis {
        let mut diagnosis = Vec::new();

        for error in &snapshot.error_history {
            let severity = self.calculate_error_severity(&error.error);
            
            diagnosis.push(ProblemDiagnosis {
                problem_type: ProblemType::TaskFailure,
                description: format!("{}: {}", error.tool_name, error.error),
                affected_tasks: vec![error.task_id.clone()],
                severity,
            });
        }

        FailureAnalysis { diagnosis }
    }

    /// 计算错误严重程度
    fn calculate_error_severity(&self, error: &str) -> f32 {
        let error_lower = error.to_lowercase();
        
        // 严重错误
        if error_lower.contains("unauthorized") 
            || error_lower.contains("forbidden")
            || error_lower.contains("not found") 
        {
            return 0.9;
        }
        
        // 中等错误
        if error_lower.contains("timeout")
            || error_lower.contains("connection")
        {
            return 0.6;
        }
        
        // 轻微错误
        if error_lower.contains("rate limit")
            || error_lower.contains("retry")
        {
            return 0.3;
        }
        
        0.5
    }

    /// 检测无效循环
    fn detect_ineffective_loops(&self, recent_results: &[TaskExecutionSummary]) -> Option<ProblemDiagnosis> {
        if recent_results.len() < 3 {
            return None;
        }

        // 检查最近的任务是否有重复模式
        let recent_tools: Vec<&str> = recent_results.iter()
            .rev()
            .take(5)
            .map(|r| r.tool_name.as_str())
            .collect();

        // 简单的重复检测
        if recent_tools.len() >= 3 {
            let first = recent_tools[0];
            let repeat_count = recent_tools.iter().filter(|&&t| t == first).count();
            
            if repeat_count >= 3 {
                return Some(ProblemDiagnosis {
                    problem_type: ProblemType::IneffectiveLoop,
                    description: format!("Tool '{}' called {} times in recent executions", first, repeat_count),
                    affected_tasks: recent_results.iter()
                        .filter(|r| r.tool_name == first)
                        .map(|r| r.task_id.clone())
                        .collect(),
                    severity: 0.7,
                });
            }
        }

        None
    }

    /// 计算进度评分
    fn calculate_progress_score(&self, plan: &DagPlan, snapshot: &ExecutionSnapshot) -> f32 {
        let total_tasks = plan.tasks.len() as f32;
        if total_tasks == 0.0 {
            return 1.0;
        }

        let completed = snapshot.completed_tasks.len() as f32;
        let failed = snapshot.error_history.len() as f32;

        // 基础进度
        let base_progress = completed / total_tasks;
        
        // 失败惩罚
        let failure_penalty = (failed / total_tasks) * 0.5;
        
        (base_progress - failure_penalty).max(0.0).min(1.0)
    }

    /// 分析新发现
    fn analyze_discoveries(&self, snapshot: &ExecutionSnapshot) -> Vec<ProblemDiagnosis> {
        let mut discoveries = Vec::new();

        // 检查是否有新发现的信息
        for (key, value) in &snapshot.gathered_info {
            // 检查是否有新的 API 端点
            if key.contains("api") || key.contains("endpoint") {
                if let Some(arr) = value.as_array() {
                    if arr.len() > 5 {
                        discoveries.push(ProblemDiagnosis {
                            problem_type: ProblemType::NewDiscovery,
                            description: format!("Discovered {} new {}", arr.len(), key),
                            affected_tasks: Vec::new(),
                            severity: 0.3,
                        });
                    }
                }
            }
        }

        discoveries
    }

    /// 判断是否应该触发重规划
    fn should_trigger_replan(&self, diagnosis: &[ProblemDiagnosis], progress_score: f32) -> bool {
        if !self.config.auto_replan {
            return false;
        }

        // 如果有严重问题
        if diagnosis.iter().any(|d| d.severity >= 0.8) {
            return true;
        }

        // 如果进度分数太低
        if progress_score < self.config.success_threshold {
            return true;
        }

        // 如果有多个中等问题
        let medium_problems = diagnosis.iter().filter(|d| d.severity >= 0.5).count();
        if medium_problems >= 2 {
            return true;
        }

        // 如果有无效循环
        if diagnosis.iter().any(|d| matches!(d.problem_type, ProblemType::IneffectiveLoop)) {
            return true;
        }

        false
    }

    /// 生成调整建议
    async fn generate_adjustments(
        &self,
        diagnosis: &[ProblemDiagnosis],
        current_plan: &DagPlan,
        snapshot: &ExecutionSnapshot,
    ) -> Result<Vec<PlanAdjustment>> {
        let mut adjustments = Vec::new();

        for problem in diagnosis {
            match problem.problem_type {
                ProblemType::TaskFailure => {
                    // 为失败的任务生成替代方案
                    adjustments.push(PlanAdjustment {
                        adjustment_type: AdjustmentType::ReplaceFailedTask,
                        description: format!("Replace failed task: {}", problem.description),
                        task_ids: problem.affected_tasks.clone(),
                        new_tasks: Vec::new(), // 将在重规划时生成
                    });
                }
                ProblemType::IneffectiveLoop => {
                    // 添加条件或跳过
                    adjustments.push(PlanAdjustment {
                        adjustment_type: AdjustmentType::AddCondition,
                        description: "Add condition to break loop".to_string(),
                        task_ids: problem.affected_tasks.clone(),
                        new_tasks: Vec::new(),
                    });
                }
                ProblemType::NewDiscovery => {
                    // 添加新任务处理发现
                    adjustments.push(PlanAdjustment {
                        adjustment_type: AdjustmentType::AddTasks,
                        description: format!("Handle discovery: {}", problem.description),
                        task_ids: Vec::new(),
                        new_tasks: Vec::new(), // 将在重规划时生成
                    });
                }
                _ => {
                    // 其他情况考虑完全重规划
                    adjustments.push(PlanAdjustment {
                        adjustment_type: AdjustmentType::FullReplan,
                        description: problem.description.clone(),
                        task_ids: Vec::new(),
                        new_tasks: Vec::new(),
                    });
                }
            }
        }

        Ok(adjustments)
    }

    /// 选择重规划策略
    fn select_replan_strategy(&self, evaluation: &ReplanEvaluation) -> ReplanStrategy {
        // 如果进度很低或有严重问题，完全重规划
        if evaluation.progress_score < 0.3 {
            return ReplanStrategy::FullReplan;
        }

        // 如果需要完全重规划的调整
        if evaluation.suggested_adjustments.iter().any(|a| 
            matches!(a.adjustment_type, AdjustmentType::FullReplan)
        ) {
            return ReplanStrategy::FullReplan;
        }

        // 如果只有少量调整，增量调整
        if evaluation.suggested_adjustments.len() <= 2 {
            return ReplanStrategy::IncrementalAdjust;
        }

        // 默认部分重规划
        ReplanStrategy::PartialReplan
    }

    /// 增量重规划 (只调整失败的部分)
    async fn incremental_replan(
        &self,
        llm_client: &LlmClient,
        original_task: &str,
        context: &HashMap<String, serde_json::Value>,
        snapshot: &ExecutionSnapshot,
        evaluation: &ReplanEvaluation,
    ) -> Result<DagPlan> {
        log::info!("ReplanningEngine: Incremental replan");

        let system_prompt = r#"你是一个任务修复专家。只修复失败的步骤，保留成功的部分。

## 输出格式
```
KEEP: 1, 2, 3  // 保留的任务ID
FIX:
4. new_tool(args...)  // 替代失败任务的新任务
5. another_tool(args...) depends: 4
```

只输出需要的修复，不要重复已成功的任务。"#;

        let failed_tasks: Vec<String> = evaluation.diagnosis.iter()
            .flat_map(|d| d.affected_tasks.clone())
            .collect();

        let user_prompt = format!(
            r#"原始任务: {}

已成功完成:
{}

失败的任务:
{}

请提供修复方案:"#,
            original_task,
            snapshot.completed_tasks.keys()
                .map(|k| format!("- {}", k))
                .collect::<Vec<_>>()
                .join("\n"),
            failed_tasks.join(", "),
        );

        let response = llm_client
            .completion(Some(system_prompt), &user_prompt)
            .await?;

        self.parse_incremental_response(&response, original_task, snapshot)
    }

    /// 部分重规划 (重规划未完成的部分)
    async fn partial_replan(
        &self,
        llm_client: &LlmClient,
        original_task: &str,
        context: &HashMap<String, serde_json::Value>,
        snapshot: &ExecutionSnapshot,
        evaluation: &ReplanEvaluation,
    ) -> Result<DagPlan> {
        log::info!("ReplanningEngine: Partial replan");

        let system_prompt = r#"你是一个任务规划专家。根据已完成的进度，重新规划剩余步骤。

## 已知信息
利用已收集的信息来优化后续计划。

## 输出格式
```
1. tool_name(arg1="val1")
2. tool_name(arg2=$1.field) depends: 1
3. join()
```

## 规则
1. 不要重复已完成的工作
2. 利用已收集的信息
3. 为之前失败的操作提供替代方案"#;

        let user_prompt = format!(
            r#"原始任务: {}

已完成的步骤和结果:
{}

收集到的信息:
{}

之前失败的操作:
{}

请规划剩余步骤:"#,
            original_task,
            serde_json::to_string_pretty(&snapshot.completed_tasks).unwrap_or_default(),
            serde_json::to_string_pretty(&snapshot.gathered_info).unwrap_or_default(),
            snapshot.error_history.iter()
                .map(|e| format!("- {} ({}): {}", e.task_id, e.tool_name, e.error))
                .collect::<Vec<_>>()
                .join("\n"),
        );

        let response = llm_client
            .completion(Some(system_prompt), &user_prompt)
            .await?;

        self.parse_plan_response(&response, original_task)
    }

    /// 完全重规划
    async fn full_replan(
        &self,
        llm_client: &LlmClient,
        original_task: &str,
        context: &HashMap<String, serde_json::Value>,
        snapshot: &ExecutionSnapshot,
    ) -> Result<DagPlan> {
        log::info!("ReplanningEngine: Full replan");

        // 使用 DagPlanner 生成新计划，但包含历史信息
        let mut enhanced_context = context.clone();
        
        // 注入历史信息
        enhanced_context.insert(
            "_completed_tasks".to_string(),
            serde_json::to_value(&snapshot.completed_tasks).unwrap_or(serde_json::json!({})),
        );
        enhanced_context.insert(
            "_gathered_info".to_string(),
            serde_json::to_value(&snapshot.gathered_info).unwrap_or(serde_json::json!({})),
        );
        enhanced_context.insert(
            "_attempted_approaches".to_string(),
            serde_json::to_value(&snapshot.attempted_approaches).unwrap_or(serde_json::json!([])),
        );

        let mut planner = DagPlanner::new(self.ai_service.clone(), LiteModeConfig::default());
        
        if let Some(repo) = &self.prompt_repo {
            planner = planner.with_prompt_repo(repo.clone());
        }
        if let Some(adapter) = &self.tool_adapter {
            planner = planner.with_tool_adapter(adapter.clone());
        }

        planner.generate_plan(original_task, &enhanced_context).await
    }

    /// 解析增量重规划响应
    fn parse_incremental_response(
        &self,
        response: &str,
        task_description: &str,
        snapshot: &ExecutionSnapshot,
    ) -> Result<DagPlan> {
        let mut plan = DagPlan::new(task_description.to_string());

        // 首先添加保留的任务（简化处理：所有已完成的任务）
        for (task_id, result) in &snapshot.completed_tasks {
            let mut task = DagTask::new(
                task_id.clone(),
                "_preserved".to_string(), // 标记为保留
                HashMap::new(),
            );
            task.status = DagTaskStatus::Completed;
            task.result = Some(result.clone());
            plan.add_task(task);
        }

        // 解析新任务
        let content = if response.contains("FIX:") {
            response.split("FIX:").nth(1).unwrap_or(response).trim()
        } else {
            response.trim()
        };

        self.parse_tasks_from_content(content, &mut plan)?;

        Ok(plan)
    }

    /// 解析计划响应
    fn parse_plan_response(&self, response: &str, task_description: &str) -> Result<DagPlan> {
        let mut plan = DagPlan::new(task_description.to_string());

        let content = if response.contains("```") {
            response
                .split("```")
                .nth(1)
                .map(|s| s.trim_start_matches("plaintext").trim_start_matches('\n'))
                .unwrap_or(response)
                .trim()
        } else {
            response.trim()
        };

        self.parse_tasks_from_content(content, &mut plan)?;

        if plan.tasks.is_empty() {
            return Err(anyhow!("Failed to parse any tasks from replan response"));
        }

        Ok(plan)
    }

    /// 从内容解析任务
    fn parse_tasks_from_content(&self, content: &str, plan: &mut DagPlan) -> Result<()> {
        let task_regex = regex::Regex::new(
            r#"(\d+)\.\s*(\w+)\s*\(([^)]*)\)(?:\s*depends:\s*([\d,\s]+))?"#
        )?;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.contains("join()") || line.starts_with("KEEP:") {
                continue;
            }

            if let Some(captures) = task_regex.captures(line) {
                let task_id = captures.get(1).map(|m| m.as_str()).unwrap_or("0");
                let tool_name = captures.get(2).map(|m| m.as_str()).unwrap_or("");
                let args_str = captures.get(3).map(|m| m.as_str()).unwrap_or("");
                let depends_str = captures.get(4).map(|m| m.as_str());

                let arguments = self.parse_arguments(args_str);
                let depends_on: Vec<String> = depends_str
                    .map(|s| {
                        s.split(',')
                            .map(|d| d.trim().to_string())
                            .filter(|d| !d.is_empty())
                            .collect()
                    })
                    .unwrap_or_default();

                let task = DagTask::new(task_id.to_string(), tool_name.to_string(), arguments)
                    .with_depends(depends_on);

                plan.add_task(task);
            }
        }

        Ok(())
    }

    /// 解析参数
    fn parse_arguments(&self, args_str: &str) -> HashMap<String, serde_json::Value> {
        let mut arguments = HashMap::new();
        let arg_regex = regex::Regex::new(
            r#"(\w+)\s*=\s*(?:"([^"]*)"|(\$[\d.]+\w*)|(\d+(?:\.\d+)?)|(\w+))"#
        ).unwrap();

        for captures in arg_regex.captures_iter(args_str) {
            let name = captures.get(1).map(|m| m.as_str()).unwrap_or("");

            let value = if let Some(quoted) = captures.get(2) {
                serde_json::Value::String(quoted.as_str().to_string())
            } else if let Some(var_ref) = captures.get(3) {
                serde_json::Value::String(var_ref.as_str().to_string())
            } else if let Some(num) = captures.get(4) {
                if let Ok(n) = num.as_str().parse::<f64>() {
                    serde_json::json!(n)
                } else {
                    serde_json::Value::String(num.as_str().to_string())
                }
            } else if let Some(word) = captures.get(5) {
                match word.as_str() {
                    "true" => serde_json::Value::Bool(true),
                    "false" => serde_json::Value::Bool(false),
                    _ => serde_json::Value::String(word.as_str().to_string()),
                }
            } else {
                serde_json::Value::Null
            };

            if !name.is_empty() {
                arguments.insert(name.to_string(), value);
            }
        }

        arguments
    }
}

/// 重规划策略
enum ReplanStrategy {
    /// 增量调整 (只修复失败部分)
    IncrementalAdjust,
    /// 部分重规划 (重新规划未完成部分)
    PartialReplan,
    /// 完全重规划
    FullReplan,
}

/// 失败分析结果
struct FailureAnalysis {
    diagnosis: Vec<ProblemDiagnosis>,
}

/// 任务执行摘要
#[derive(Debug, Clone)]
pub struct TaskExecutionSummary {
    pub task_id: String,
    pub tool_name: String,
    pub success: bool,
    pub duration_ms: u64,
    pub result_summary: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_error_severity() {
        // 测试需要一个 mock，暂时跳过
    }
}

