//! Replanner 组件 - 重新规划器
//! 
//! 负责在执行过程中动态调整计划，处理异常情况和优化执行策略

use crate::ai_adapter::core::AiAdapterManager;
use crate::engines::plan_and_execute::types::*;
use crate::engines::plan_and_execute::planner::{Planner, PlannerConfig, RiskLevel};
use crate::services::prompt_db::PromptRepository;
use crate::services::ai::AiServiceManager;
use crate::services::mcp::McpService;
use crate::engines::plan_and_execute::executor::{ExecutionResult, StepResult, StepStatus};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 重新规划器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplannerConfig {
    /// 是否启用自动重新规划
    pub auto_replan_enabled: bool,
    /// 重新规划触发阈值
    pub replan_threshold: ReplanThreshold,
    /// 最大重新规划次数
    pub max_replan_attempts: u32,
    /// 重新规划策略
    pub replan_strategy: ReplanStrategy,
    /// 学习模式配置
    pub learning_config: LearningConfig,
}

/// 重新规划触发阈值
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplanThreshold {
    /// 失败步骤比例阈值 (0.0-1.0)
    pub failure_rate_threshold: f64,
    /// 执行时间超出预期比例阈值 (1.0表示100%超时)
    pub timeout_ratio_threshold: f64,
    /// 连续失败步骤数阈值
    pub consecutive_failures_threshold: u32,
    /// 资源使用率阈值 (0.0-1.0)
    pub resource_usage_threshold: f64,
}

/// 重新规划策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplanStrategy {
    /// 保守策略：最小化变更
    Conservative,
    /// 激进策略：大幅调整计划
    Aggressive,
    /// 自适应策略：根据情况动态选择
    Adaptive,
    /// 学习策略：基于历史经验
    Learning,
}

/// 学习配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningConfig {
    /// 是否启用学习模式
    pub enabled: bool,
    /// 历史数据保留天数
    pub history_retention_days: u32,
    /// 学习权重
    pub learning_weight: f64,
    /// 最小样本数
    pub min_sample_size: u32,
}

/// 重新规划触发器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplanTrigger {
    /// 步骤失败
    StepFailure {
        step_id: String,
        error_message: String,
        retry_count: u32,
    },
    /// 执行超时
    ExecutionTimeout {
        expected_duration: u64,
        actual_duration: u64,
    },
    /// 资源不足
    ResourceConstraint {
        resource_type: String,
        available: f64,
        required: f64,
    },
    /// 外部条件变化
    ExternalConditionChange {
        condition: String,
        old_value: String,
        new_value: String,
    },
    /// 用户请求
    UserRequest {
        reason: String,
    },
    /// 质量阈值
    QualityThreshold {
        metric: String,
        threshold: f64,
        actual: f64,
    },
}

/// 重新规划结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplanResult {
    /// 是否需要重新规划
    pub should_replan: bool,
    /// 新的执行计划
    pub new_plan: Option<ExecutionPlan>,
    /// 重新规划原因
    pub replan_reason: String,
    /// 变更摘要
    pub changes_summary: Vec<PlanChange>,
    /// 预期改进
    pub expected_improvements: Vec<String>,
    /// 风险评估
    pub risk_assessment: RiskAssessment,
    /// 置信度 (0.0-1.0)
    pub confidence: f64,
}

/// 计划变更
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanChange {
    /// 变更类型
    pub change_type: ChangeType,
    /// 变更描述
    pub description: String,
    /// 影响的步骤
    pub affected_steps: Vec<String>,
    /// 变更原因
    pub reason: String,
}

/// 变更类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    /// 添加步骤
    AddStep,
    /// 删除步骤
    RemoveStep,
    /// 修改步骤
    ModifyStep,
    /// 重新排序
    ReorderSteps,
    /// 调整参数
    AdjustParameters,
    /// 更换工具
    ChangeTool,
    /// 修改策略
    ChangeStrategy,
}

/// 风险评估
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    /// 总体风险等级
    pub overall_risk: RiskLevel,
    /// 具体风险项
    pub risk_factors: Vec<RiskFactor>,
    /// 缓解措施
    pub mitigation_measures: Vec<String>,
}

/// 风险因子
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    /// 风险类型
    pub risk_type: String,
    /// 风险等级
    pub level: RiskLevel,
    /// 风险描述
    pub description: String,
    /// 影响范围
    pub impact: String,
    /// 发生概率 (0.0-1.0)
    pub probability: f64,
}

/// 执行分析
#[derive(Debug, Clone)]
pub struct ExecutionAnalysis {
    /// 总体成功率
    pub success_rate: f64,
    /// 平均执行时间
    pub avg_execution_time: u64,
    /// 失败模式分析
    pub failure_patterns: Vec<FailurePattern>,
    /// 性能瓶颈
    pub bottlenecks: Vec<String>,
    /// 资源使用情况
    pub resource_usage: HashMap<String, f64>,
}

/// AI辅助重新规划决策结果
#[derive(Debug, Clone)]
pub struct AiReplanDecision {
    /// 是否需要重新规划
    pub should_replan: bool,
    /// AI的推理过程
    pub reasoning: String,
    /// 决策置信度 (0.0-1.0)
    pub confidence: f64,
    /// 建议的下一步行动
    pub suggested_actions: Vec<String>,
    /// 识别的风险因素
    pub identified_risks: Vec<String>,
}

/// 恢复策略（增强错误处理）
#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryStrategy {
    /// 重试并调整参数
    RetryWithAdjustment,
    /// 使用替代方法
    AlternativeApproach,
    /// 跳过并继续后续步骤
    SkipAndContinue,

    /// 终止任务
    AbortTask,
}

/// 增强的失败分析
#[derive(Debug, Clone)]
pub struct EnhancedFailureAnalysis {
    /// 根本原因
    pub root_cause: String,
    /// 失败分类
    pub failure_category: String,
    /// 错误严重性 (1-5)
    pub severity_level: u32,
    /// 是否可恢复
    pub is_recoverable: bool,
    /// 建议的修复方法
    pub suggested_fixes: Vec<String>,
    /// 错误模式
    pub error_pattern: ErrorPattern,
    /// 影响范围
    pub impact_scope: Vec<String>,
}

/// 错误模式
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorPattern {
    /// 临时性错误（网络超时等）
    Transient,
    /// 配置错误
    Configuration,
    /// 权限错误
    Permission,
    /// 资源不足
    ResourceExhaustion,
    /// 逻辑错误
    Logic,
    /// 系统错误
    System,
    /// 未知错误
    Unknown,
}

/// 失败模式
#[derive(Debug, Clone, Serialize)]
pub struct FailurePattern {
    /// 失败类型
    pub failure_type: String,
    /// 发生频率
    pub frequency: u32,
    /// 影响的步骤类型
    pub affected_step_types: Vec<StepType>,
    /// 常见错误消息
    pub common_errors: Vec<String>,
}

/// 重新规划器
#[derive(Debug)]
pub struct Replanner {
    config: ReplannerConfig,
    planner: Planner,
    execution_history: Mutex<Vec<ExecutionResult>>,
    replan_history: Mutex<Vec<ReplanResult>>,
}

impl Replanner {
    /// 创建新的重新规划器实例
    pub fn new(config: ReplannerConfig, planner_config: PlannerConfig, prompt_repo: Option<PromptRepository>) -> Result<Self, PlanAndExecuteError> {
        let planner = Planner::new(planner_config, prompt_repo)?;
        
        Ok(Self {
            config,
            planner,
            execution_history: Mutex::new(Vec::new()),
            replan_history: Mutex::new(Vec::new()),
        })
    }

    /// 创建带有AI服务管理器的重新规划器实例（用于动态模型切换）
    pub fn with_ai_service_manager(
        config: ReplannerConfig, 
        planner_config: PlannerConfig, 
        prompt_repo: Option<PromptRepository>,
        mcp_service: Option<Arc<McpService>>,
        ai_service_manager: Arc<AiServiceManager>,
        app_handle: Option<Arc<AppHandle>>,
    ) -> Result<Self, PlanAndExecuteError> {
        let planner = Planner::with_ai_service_manager(
            planner_config, 
            prompt_repo, 
            mcp_service, 
            ai_service_manager,
            app_handle,
        )?;
        
        Ok(Self {
            config,
            planner,
            execution_history: Mutex::new(Vec::new()),
            replan_history: Mutex::new(Vec::new()),
        })
    }

    /// Replan反思层：分析执行结果并决定是否需要重新规划
    /// 核心改进：每次执行后都通过AI判断是否需要重新规划
    pub async fn analyze_and_replan(
        &self,
        current_plan: &ExecutionPlan,
        execution_result: &ExecutionResult,
        task: &TaskRequest,
        trigger: Option<ReplanTrigger>,
    ) -> Result<ReplanResult, PlanAndExecuteError> {
        log::info!("=== Replan反思层：开始分析执行结果 ===");
        log::info!("执行状态: {:?}", execution_result.status);
        log::info!("成功步骤: {}, 失败步骤: {}", 
                  execution_result.completed_steps.len(), 
                  execution_result.failed_steps.len());
        
        // 记录执行历史
        self.execution_history.lock().await.push(execution_result.clone());
        
        // 1. 先进行基础分析
        let analysis = self.analyze_execution_for_replan(execution_result).await?;
        log::info!("执行分析 - 成功率: {:.2}, 平均耗时: {}ms", 
                  analysis.success_rate, analysis.avg_execution_time);
        
        // 2. 核心改进：通过AI来判断是否需要重新规划
        let ai_decision = self.ai_assisted_replan_decision(
            current_plan, 
            execution_result, 
            task, 
            &analysis,
            trigger.as_ref()
        ).await?;
        
        log::info!("AI重新规划决策: 需要重新规划={}, 理由: {}", 
                  ai_decision.should_replan, ai_decision.reasoning);
        
        if !ai_decision.should_replan {
            log::info!("AI反思层决定：当前执行结果良好，继续执行剩余步骤");
            return Ok(ReplanResult {
                should_replan: false,
                new_plan: None,
                replan_reason: ai_decision.reasoning,
                changes_summary: Vec::new(),
                expected_improvements: Vec::new(),
                risk_assessment: RiskAssessment {
                    overall_risk: RiskLevel::Low,
                    risk_factors: Vec::new(),
                    mitigation_measures: Vec::new(),
                },
                confidence: ai_decision.confidence,
            });
        }
        
        log::info!("AI反思层决定：需要重新规划");
        
        // 3. 生成新的计划（回到Planner战略层）
        let replan_result = self.generate_improved_plan(
            current_plan,
            &analysis,
            task,
            trigger.as_ref(),
        ).await?;
        
        // 4. 使用AI的推理和置信度增强结果
        let enhanced_result = ReplanResult {
            should_replan: true,
            new_plan: replan_result.new_plan,
            replan_reason: format!("AI分析: {} | 详细分析: {}", 
                                 ai_decision.reasoning, replan_result.replan_reason),
            changes_summary: replan_result.changes_summary,
            expected_improvements: replan_result.expected_improvements,
            risk_assessment: replan_result.risk_assessment,
            confidence: (ai_decision.confidence + replan_result.confidence) / 2.0,
        };
        
        // 记录重新规划历史
        self.replan_history.lock().await.push(enhanced_result.clone());
        
        log::info!("=== Replan反思层：重新规划完成 ===");
        log::info!("新计划置信度: {:.2}", enhanced_result.confidence);
        log::info!("重新规划原因: {}", enhanced_result.replan_reason);
        
        Ok(enhanced_result)
    }

    /// 专门为Plan-and-Execute流程分析执行结果
    async fn analyze_execution_for_replan(
        &self,
        execution_result: &ExecutionResult,
    ) -> Result<ExecutionAnalysis, PlanAndExecuteError> {
        let total_steps = execution_result.step_results.len();
        let successful_steps = execution_result.completed_steps.len();
        
        let success_rate = if total_steps > 0 {
            successful_steps as f64 / total_steps as f64
        } else {
            0.0
        };
        
        log::info!("执行分析 - 总步骤: {}, 成功: {}, 成功率: {:.2}", 
                  total_steps, successful_steps, success_rate);
        
        // 分析失败模式
        let failure_patterns = self.analyze_failure_patterns(execution_result).await?;
        
        // 识别性能瓶颈
        let bottlenecks = self.identify_bottlenecks(execution_result).await?;
        
        // 分析资源使用
        let resource_usage = self.analyze_resource_usage(execution_result).await?;
        
        Ok(ExecutionAnalysis {
            success_rate,
            avg_execution_time: execution_result.metrics.avg_step_duration_ms,
            failure_patterns,
            bottlenecks,
            resource_usage,
        })
    }

    /// Plan-and-Execute重新规划决策逻辑
    async fn should_replan_decision(
        &self,
        analysis: &ExecutionAnalysis,
        trigger: Option<&ReplanTrigger>,
        current_plan: &ExecutionPlan,
    ) -> Result<bool, PlanAndExecuteError> {
        // 如果有明确的触发器，需要仔细分析
        if let Some(trigger) = trigger {
            log::info!("检测到重新规划触发器: {:?}", trigger);
            
            match trigger {
                ReplanTrigger::StepFailure { step_id, .. } => {
                    log::info!("步骤失败触发器: {}", step_id);
                    return Ok(true);
                },
                ReplanTrigger::ExecutionTimeout { .. } => {
                    log::info!("执行超时触发器");
                    return Ok(true);
                },
                _ => {
                    log::info!("其他触发器，评估是否需要重新规划");
                }
            }
        }
        
        // Plan-and-Execute反思逻辑
        
        // 1. 检查成功率阈值
        if analysis.success_rate < (1.0 - self.config.replan_threshold.failure_rate_threshold) {
            log::info!("成功率 {:.2} 低于阈值 {:.2}，需要重新规划", 
                      analysis.success_rate, 
                      1.0 - self.config.replan_threshold.failure_rate_threshold);
            return Ok(true);
        }
        
        // 2. 检查是否有关键步骤失败
        if !analysis.failure_patterns.is_empty() {
            for pattern in &analysis.failure_patterns {
                if pattern.frequency > 0 {
                    log::info!("检测到失败模式: {} (频率: {})", pattern.failure_type, pattern.frequency);
                    return Ok(true);
                }
            }
        }
        
        // 3. 检查性能瓶颈
        if !analysis.bottlenecks.is_empty() {
            log::info!("检测到性能瓶颈: {:?}", analysis.bottlenecks);
            // 如果有多个瓶颈，考虑重新规划
            if analysis.bottlenecks.len() > 1 {
                return Ok(true);
            }
        }
        
        // 4. 如果计划步骤很少且有失败，优先重新规划
        if current_plan.steps.len() <= 3 && analysis.success_rate < 0.8 {
            log::info!("简单计划出现较多失败，需要重新规划");
            return Ok(true);
        }
        
        log::info!("执行结果在可接受范围内，继续当前计划");
        Ok(false)
    }

    /// 处理实时执行异常（增强的错误处理机制）
    pub async fn handle_runtime_exception(
        &self,
        current_plan: &ExecutionPlan,
        failed_step: &StepResult,
        task: &TaskRequest,
    ) -> Result<ReplanResult, PlanAndExecuteError> {
        log::warn!("=== 增强错误处理：分析步骤失败 ===");
        log::warn!("失败步骤: '{}', 重试次数: {}", failed_step.step_id, failed_step.retry_count);
        
        let _trigger = ReplanTrigger::StepFailure {
            step_id: failed_step.step_id.clone(),
            error_message: failed_step.error.clone().unwrap_or_default(),
            retry_count: failed_step.retry_count,
        };
        
        // 1. 深度分析失败原因（增强）
        let failure_analysis = self.analyze_step_failure_enhanced(failed_step).await?;
        log::info!("失败分析: 根本原因={}, 分类={}", 
                  failure_analysis.root_cause, failure_analysis.failure_category);
        
        // 2. 根据失败类型选择策略
        let recovery_strategy = self.determine_recovery_strategy(&failure_analysis, failed_step).await;
        log::info!("选择恢复策略: {:?}", recovery_strategy);
        
        // 3. 生成针对性的恢复计划
        let recovery_plan = match recovery_strategy {
            RecoveryStrategy::RetryWithAdjustment => {
                self.generate_retry_plan(current_plan, failed_step, &failure_analysis).await?
            },
            RecoveryStrategy::AlternativeApproach => {
                self.generate_alternative_approach_plan(current_plan, failed_step, task).await?
            },
            RecoveryStrategy::SkipAndContinue => {
                self.generate_skip_plan(current_plan, failed_step).await?
            },

            RecoveryStrategy::AbortTask => {
                log::error!("错误过于严重，建议终止任务");
                return Ok(ReplanResult {
                    should_replan: false,
                    new_plan: None,
                    replan_reason: "错误过于严重，建议终止任务".to_string(),
                    changes_summary: vec![],
                    expected_improvements: vec![],
                    risk_assessment: RiskAssessment {
                        overall_risk: RiskLevel::Critical,
                        risk_factors: vec![RiskFactor {
                            risk_type: "致命错误".to_string(),
                            level: RiskLevel::Critical,
                            description: "步骤失败且无法恢复".to_string(),
                            impact: "任务无法继续".to_string(),
                            probability: 0.9,
                        }],
                        mitigation_measures: vec!["检查系统配置".to_string(), "联系技术支持".to_string()],
                    },
                    confidence: 0.95,
                });
            }
        };
        
        // 4. 构建详细的重新规划结果
        Ok(ReplanResult {
            should_replan: true,
            new_plan: Some(recovery_plan),
            replan_reason: format!("智能错误恢复: {} (策略: {:?})", 
                                 failure_analysis.root_cause, recovery_strategy),
            changes_summary: vec![PlanChange {
                change_type: ChangeType::ModifyStep,
                description: format!("应用{}恢复策略", 
                    match recovery_strategy {
                        RecoveryStrategy::RetryWithAdjustment => "重试调整",
                        RecoveryStrategy::AlternativeApproach => "替代方法",
                        RecoveryStrategy::SkipAndContinue => "跳过继续",

                        RecoveryStrategy::AbortTask => "任务终止",
                    }
                ),
                affected_steps: vec![failed_step.step_id.clone()],
                reason: format!("{}类型错误处理", failure_analysis.failure_category),
            }],
            expected_improvements: failure_analysis.suggested_fixes.clone(),
            risk_assessment: RiskAssessment {
                overall_risk: match recovery_strategy {
                    RecoveryStrategy::RetryWithAdjustment => RiskLevel::Low,
                    RecoveryStrategy::AlternativeApproach => RiskLevel::Medium,
                    RecoveryStrategy::SkipAndContinue => RiskLevel::Medium,

                    RecoveryStrategy::AbortTask => RiskLevel::Critical,
                },
                risk_factors: vec![RiskFactor {
                    risk_type: failure_analysis.failure_category.clone(),
                    level: RiskLevel::Medium,
                    description: failure_analysis.root_cause.clone(),
                    impact: "可能影响后续步骤".to_string(),
                    probability: 0.4,
                }],
                mitigation_measures: failure_analysis.suggested_fixes.clone(),
            },
            confidence: match recovery_strategy {
                RecoveryStrategy::RetryWithAdjustment => 0.8,
                RecoveryStrategy::AlternativeApproach => 0.7,
                RecoveryStrategy::SkipAndContinue => 0.6,

                RecoveryStrategy::AbortTask => 0.9,
            },
        })
    }

    /// 优化现有计划
    pub async fn optimize_plan(
        &self,
        current_plan: &ExecutionPlan,
        _task: &TaskRequest,
    ) -> Result<ReplanResult, PlanAndExecuteError> {
        log::info!("开始优化现有计划");
        
        // 分析历史执行数据
        let historical_analysis = self.analyze_historical_performance().await?;
        
        // 识别优化机会
        let optimization_opportunities = self.identify_optimization_opportunities(
            current_plan,
            &historical_analysis,
        ).await?;
        
        if optimization_opportunities.is_empty() {
            return Ok(ReplanResult {
                should_replan: false,
                new_plan: None,
                replan_reason: "当前计划已经是最优的".to_string(),
                changes_summary: Vec::new(),
                expected_improvements: Vec::new(),
                risk_assessment: RiskAssessment {
                    overall_risk: RiskLevel::Low,
                    risk_factors: Vec::new(),
                    mitigation_measures: Vec::new(),
                },
                confidence: 0.95,
            });
        }
        
        // 生成优化后的计划
        let optimized_plan = self.apply_optimizations(
            current_plan,
            &optimization_opportunities,
            // task,
        ).await?;
        
        let changes_summary = optimization_opportunities.into_iter()
            .map(|opt| PlanChange {
                change_type: ChangeType::ModifyStep,
                description: opt.description,
                affected_steps: opt.affected_steps,
                reason: "性能优化".to_string(),
            })
            .collect();
        
        Ok(ReplanResult {
            should_replan: true,
            new_plan: Some(optimized_plan),
            replan_reason: "基于历史数据的性能优化".to_string(),
            changes_summary,
            expected_improvements: vec![
                "提高执行效率".to_string(),
                "减少资源消耗".to_string(),
                "提高成功率".to_string(),
            ],
            risk_assessment: RiskAssessment {
                overall_risk: RiskLevel::Low,
                risk_factors: Vec::new(),
                mitigation_measures: Vec::new(),
            },
            confidence: 0.85,
        })
    }

    /// AI辅助重新规划决策：通过AI分析执行结果和整体任务状态
    /// 这是Plan-and-Execute架构的核心改进：让AI来判断每步执行后是否需要重新规划
    async fn ai_assisted_replan_decision(
        &self,
        current_plan: &ExecutionPlan,
        execution_result: &ExecutionResult,
        task: &TaskRequest,
        analysis: &ExecutionAnalysis,
        trigger: Option<&ReplanTrigger>,
    ) -> Result<AiReplanDecision, PlanAndExecuteError> {
        log::info!("=== AI辅助重新规划决策开始 ===");
        
        // 构建完整的上下文信息给AI分析
        let context_prompt = self.build_replan_context_prompt(
            current_plan, 
            execution_result, 
            task, 
            analysis, 
            trigger
        ).await?;
        
        // 调用AI进行分析
        let ai_response = self.call_ai_for_replan_decision(&context_prompt).await?;
        
        // 解析AI的回复
        let decision = self.parse_ai_replan_decision(&ai_response)?;
        
        log::info!("AI重新规划决策完成: 需要重新规划={}, 置信度={:.2}", 
                  decision.should_replan, decision.confidence);
        
        Ok(decision)
    }

    /// 构建重新规划决策的上下文Prompt
    async fn build_replan_context_prompt(
        &self,
        current_plan: &ExecutionPlan,
        execution_result: &ExecutionResult,
        task: &TaskRequest,
        analysis: &ExecutionAnalysis,
        trigger: Option<&ReplanTrigger>,
    ) -> Result<String, PlanAndExecuteError> {
        let execution_history = self.execution_history.lock().await;
        
        // 获取对话历史上下文
        let conversation_history = self.build_conversation_history(&execution_history).await;
        
        let trigger_info = if let Some(t) = trigger {
            format!("触发器: {:?}", t)
        } else {
            "无特定触发器".to_string()
        };
        
        let prompt = format!(r#"你是Plan-and-Execute架构中的Replanner反思层。你的任务是分析当前执行结果和整体任务状态，判断是否需要重新规划。

## 当前任务信息
任务名称: {}
任务描述: {}
任务类型: {:?}
优先级: {:?}

## 当前计划信息
计划名称: {}
计划描述: {}
总步骤数: {}
预估时长: {}秒

## 执行结果分析
执行状态: {:?}
成功步骤: {}
失败步骤: {}
跳过步骤: {}
成功率: {:.2}%
平均执行时间: {}ms

## 性能分析
失败模式: {}
性能瓶颈: {}
资源使用: {}

## 触发器信息
{}

## 对话历史上下文
{}

## 判断标准
请根据以下标准判断是否需要重新规划：
1. **上下文保持**：分析完整的对话历史和计划执行状态
2. **反馈质量**：考虑执行结果的格式和质量
3. **终止条件**：避免无限循环，检查是否应该终止
4. **错误处理**：当工具执行失败时调整策略

## 请按以下JSON格式回复：
{{
  "should_replan": true/false,
  "reasoning": "详细的推理过程，解释为什么需要或不需要重新规划",
  "confidence": 0.0-1.0,
  "suggested_actions": ["建议的具体行动"],
  "identified_risks": ["识别的风险因素"]
}}

注意：
- 如果成功率低于70%，通常需要重新规划
- 如果出现连续失败，需要调整策略
- 如果任务已基本完成，不建议重新规划
- 考虑重新规划的成本和收益
- 避免过度重新规划导致的无限循环"#,
            task.name,
            task.description,
            task.task_type,
            task.priority,
            current_plan.name,
            current_plan.description,
            current_plan.steps.len(),
            current_plan.estimated_duration,
            execution_result.status,
            execution_result.completed_steps.len(),
            execution_result.failed_steps.len(),
            execution_result.skipped_steps.len(),
            analysis.success_rate * 100.0,
            analysis.avg_execution_time,
            serde_json::to_string(&analysis.failure_patterns).unwrap_or("无".to_string()),
            analysis.bottlenecks.join(", "),
            serde_json::to_string(&analysis.resource_usage).unwrap_or("{}".to_string()),
            trigger_info,
            conversation_history
        );
        
        Ok(prompt)
    }

    /// 构建对话历史上下文
    async fn build_conversation_history(&self, execution_history: &[ExecutionResult]) -> String {
        if execution_history.is_empty() {
            return "这是首次执行，没有历史记录".to_string();
        }
        
        let mut history = String::new();
        for (i, result) in execution_history.iter().enumerate() {
            history.push_str(&format!(
                "第{}次执行: 状态={:?}, 成功步骤={}, 失败步骤={}\n",
                i + 1,
                result.status,
                result.completed_steps.len(),
                result.failed_steps.len()
            ));
        }
        
        history
    }

    /// 调用AI进行重新规划决策
    async fn call_ai_for_replan_decision(&self, prompt: &str) -> Result<String, PlanAndExecuteError> {
        // 获取AI服务管理器配置的模型
        let ai_manager = AiAdapterManager::global();
        
        // 动态解析调度阶段(Replanning)的模型，否则回退到Planner本地配置
        let (provider_name, model_name) = if let Ok(Some(cfg)) = self.planner.get_ai_config_for_stage(crate::services::ai::SchedulerStage::Replanning).await {
            log::info!("Using scheduler replanning model: {} ({})", cfg.model, cfg.provider);
            (cfg.provider, cfg.model)
        } else {
            let fb = self.planner.get_fallback_provider_and_model();
            log::warn!("Scheduler config for replanning missing, fallback to planner config: {} ({})", fb.1, fb.0);
            fb
        };

        let provider = ai_manager.get_provider_or_default(&provider_name)
            .map_err(|e| PlanAndExecuteError::AiAdapterError(e.to_string()))?;
        
        let request = crate::ai_adapter::types::ChatRequest {
            model: model_name,
            messages: vec![
                crate::ai_adapter::types::Message {
                    role: crate::ai_adapter::types::MessageRole::User,
                    content: prompt.to_string(),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                }
            ],
            options: Some(crate::ai_adapter::types::ChatOptions {
                temperature: Some(0.3), // 较低温度确保稳定决策
                max_tokens: Some(2000),
                top_p: Some(0.9),
                ..Default::default()
            }),
            tools: None,
            tool_choice: None,
            user: None,
            extra_params: None,
        };
        
        // 使用流式响应并收集结果
        let mut stream = provider.send_chat_stream(&request).await
            .map_err(|e| PlanAndExecuteError::AiAdapterError(e.to_string()))?;

        // 流式响应处理状态
        let mut content = String::new();
        let mut chunk_count = 0;
        let mut response_id = String::new();
        let mut response_model = String::new();
        let mut finish_reason = None;
        let mut usage = None;
        
        let stream_start_time = std::time::Instant::now();
        let mut last_chunk_time = stream_start_time;

        // 收集流式响应 - 参考 AiService 中的实现
        use futures::StreamExt;
        while let Some(chunk_result) = stream.stream.next().await {
            chunk_count += 1;
            let chunk_receive_time = std::time::Instant::now();
            let chunk_interval = chunk_receive_time.duration_since(last_chunk_time).as_millis();
            last_chunk_time = chunk_receive_time;
            
            log::debug!("Processing replanning chunk #{}, interval: {}ms", chunk_count, chunk_interval);
            
            // 性能监控 - 每10个chunk或间隔过长时记录日志
            if chunk_count % 10 == 0 || chunk_interval > 1000 {
                let elapsed = chunk_receive_time.duration_since(stream_start_time).as_millis();
                log::info!("🚀 Replanning stream performance: chunk #{}, total_elapsed: {}ms, chunk_interval: {}ms, chars_processed: {}", 
                          chunk_count, elapsed, chunk_interval, content.len());
            }
            
            match chunk_result {
                Ok(raw_chunk) => {
                    log::debug!("Received replanning raw chunk: id='{}', content='{}', finish_reason={:?}", 
                               raw_chunk.id, raw_chunk.content, raw_chunk.finish_reason);
                    
                    // 只有在有实际内容或完成信号时才处理
                    if !raw_chunk.content.is_empty() || raw_chunk.finish_reason.is_some() {
                        // 累积内容
                        if !raw_chunk.content.is_empty() {
                            content.push_str(&raw_chunk.content);
                            log::debug!("Replanning stream chunk received: '{}', total content length: {}", 
                                       raw_chunk.content, content.len());
                        }
                        
                        // 更新响应元数据
                        if response_id.is_empty() {
                            response_id = raw_chunk.id.clone();
                        }
                        if response_model.is_empty() {
                            response_model = raw_chunk.model.clone();
                        }
                        if raw_chunk.usage.is_some() {
                            usage = raw_chunk.usage.clone();
                        }
                        if raw_chunk.finish_reason.is_some() {
                            finish_reason = raw_chunk.finish_reason.clone();
                        }
                    } else {
                        // 检查无内容的完成情况
                        log::debug!("Empty replanning chunk content, finish_reason: {:?}", raw_chunk.finish_reason);
                        if raw_chunk.finish_reason.is_some() {
                            log::warn!("Replanning stream completed with empty content after {} chunks. Total content length: {}", 
                                      chunk_count, content.len());
                            finish_reason = raw_chunk.finish_reason.clone();
                        }
                    }
                }
                Err(e) => {
                    log::error!("Replanning stream chunk error after {} chunks: {}", chunk_count, e);
                    return Err(PlanAndExecuteError::AiAdapterError(format!("Stream error: {}", e)));
                }
            }
        }
        
        // 验证流式完成
        if content.is_empty() && finish_reason.is_none() {
            let error_msg = format!("Replanning stream ended without content or finish signal after {} chunks", chunk_count);
            log::error!("{}", error_msg);
            return Err(PlanAndExecuteError::AiAdapterError(error_msg));
        }
        
        // 处理空内容但有有效完成原因的情况
        if content.is_empty() && finish_reason.is_some() {
            log::info!("Replanning stream completed with {} chunks and empty content but valid finish_reason: {:?}", 
                      chunk_count, finish_reason);
            return Err(PlanAndExecuteError::PlanningFailed("AI returned empty replanning response".to_string()));
        }
        
        log::info!("Replanning stream completed successfully after {} chunks, total content length: {}", 
                  chunk_count, content.len());
        
        // 记录token使用情况
        if let Some(usage_info) = usage {
            log::info!("Replanning tokens used: input={}, output={}, total={}", 
                      usage_info.prompt_tokens, usage_info.completion_tokens, usage_info.total_tokens);
        }
        
        Ok(content)
    }

    /// 解析AI的重新规划决策回复
    fn parse_ai_replan_decision(&self, response: &str) -> Result<AiReplanDecision, PlanAndExecuteError> {
        // 尝试从回复中提取JSON
        let json_str = self.extract_json_from_response(response)?;
        let json: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| PlanAndExecuteError::ReplanningFailed(format!("解析AI决策JSON失败: {}", e)))?;
        
        let should_replan = json.get("should_replan")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        let reasoning = json.get("reasoning")
            .and_then(|v| v.as_str())
            .unwrap_or("AI未提供推理")
            .to_string();
        
        let confidence = json.get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);
        
        let suggested_actions = json.get("suggested_actions")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect())
            .unwrap_or_default();
        
        let identified_risks = json.get("identified_risks")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect())
            .unwrap_or_default();
        
        Ok(AiReplanDecision {
            should_replan,
            reasoning,
            confidence,
            suggested_actions,
            identified_risks,
        })
    }

    /// 从AI回复中提取JSON字符串
    fn extract_json_from_response(&self, response: &str) -> Result<String, PlanAndExecuteError> {
        // 优先匹配```json代码块
        if let Some(start_idx) = response.find("```json") {
            let rest = &response[start_idx + 7..];
            if let Some(end_idx) = rest.find("```") {
                let block = &rest[..end_idx];
                return Ok(block.trim().to_string());
            }
        }
        
        // 次选：任意```代码块
        if let Some(start_idx) = response.find("```") {
            let rest = &response[start_idx + 3..];
            if let Some(end_idx) = rest.find("```") {
                let block = &rest[..end_idx];
                let trimmed = block.trim();
                if trimmed.starts_with('{') {
                    return Ok(trimmed.to_string());
                }
            }
        }
        
        // 备用方案：扫描首个{ 和最后一个}
        if let (Some(s), Some(e)) = (response.find('{'), response.rfind('}')) {
            if e > s {
                return Ok(response[s..=e].to_string());
            }
        }
        
        Err(PlanAndExecuteError::ReplanningFailed(
            "AI回复中未找到有效的JSON格式".to_string()
        ))
    }

    /// 获取重新规划统计信息
    pub async fn get_replan_statistics(&self) -> ReplanStatistics {
        let replan_history = self.replan_history.lock().await;
        let total_replans = replan_history.len();
        let successful_replans = replan_history.iter()
            .filter(|r| r.confidence > 0.7)
            .count();
        
        let avg_confidence = if total_replans > 0 {
            replan_history.iter()
                .map(|r| r.confidence)
                .sum::<f64>() / total_replans as f64
        } else {
            0.0
        };
        
        ReplanStatistics {
            total_replans: total_replans as u32,
            successful_replans: successful_replans as u32,
            success_rate: if total_replans > 0 {
                successful_replans as f64 / total_replans as f64
            } else {
                0.0
            },
            avg_confidence,
            most_common_triggers: self.get_common_triggers().await,
        }
    }

    // 私有方法实现
    
    async fn analyze_execution(
        &self,
        execution_result: &ExecutionResult,
    ) -> Result<ExecutionAnalysis, PlanAndExecuteError> {
        let total_steps = execution_result.step_results.len();
        let successful_steps = execution_result.completed_steps.len();
        
        let success_rate = if total_steps > 0 {
            successful_steps as f64 / total_steps as f64
        } else {
            0.0
        };
        
        let avg_execution_time = execution_result.metrics.avg_step_duration_ms;
        
        // 分析失败模式
        let failure_patterns = self.analyze_failure_patterns(execution_result).await?;
        
        // 识别性能瓶颈
        let bottlenecks = self.identify_bottlenecks(execution_result).await?;
        
        // 分析资源使用
        let resource_usage = self.analyze_resource_usage(execution_result).await?;
        
        Ok(ExecutionAnalysis {
            success_rate,
            avg_execution_time,
            failure_patterns,
            bottlenecks,
            resource_usage,
        })
    }

    async fn should_replan(
        &self,
        analysis: &ExecutionAnalysis,
        trigger: Option<&ReplanTrigger>,
    ) -> Result<bool, PlanAndExecuteError> {
        // 如果有明确的触发器，直接返回true
        if trigger.is_some() {
            return Ok(true);
        }
        
        // 检查各种阈值
        if analysis.success_rate < (1.0 - self.config.replan_threshold.failure_rate_threshold) {
            log::info!("成功率低于阈值，需要重新规划");
            return Ok(true);
        }
        
        // 检查资源使用情况
        for (resource, usage) in &analysis.resource_usage {
            if *usage > self.config.replan_threshold.resource_usage_threshold {
                log::info!("资源 '{}' 使用率过高: {:.2}", resource, usage);
                return Ok(true);
            }
        }
        
        // 检查性能瓶颈
        if !analysis.bottlenecks.is_empty() {
            log::info!("发现性能瓶颈: {:?}", analysis.bottlenecks);
            return Ok(true);
        }
        
        Ok(false)
    }

    /// 生成改进的计划（Plan-and-Execute回到Planner战略层）
    async fn generate_improved_plan(
        &self,
        current_plan: &ExecutionPlan,
        analysis: &ExecutionAnalysis,
        task: &TaskRequest,
        trigger: Option<&ReplanTrigger>,
    ) -> Result<ReplanResult, PlanAndExecuteError> {
        log::info!("=== 回到Planner战略层：生成改进计划 ===");
        
        // 根据反思结果和策略选择重新规划方法
        match self.config.replan_strategy {
            ReplanStrategy::Conservative => {
                log::info!("采用保守策略重新规划");
                self.conservative_replan(current_plan, analysis, task).await
            },
            ReplanStrategy::Aggressive => {
                log::info!("采用激进策略重新规划");
                self.aggressive_replan(current_plan, analysis, task).await
            },
            ReplanStrategy::Adaptive => {
                log::info!("采用自适应策略重新规划");
                self.adaptive_replan(current_plan, analysis, task, trigger).await
            },
            ReplanStrategy::Learning => {
                log::info!("采用学习策略重新规划");
                self.learning_replan(current_plan, analysis, task).await
            },
        }
    }

    async fn generate_new_plan(
        &self,
        current_plan: &ExecutionPlan,
        analysis: &ExecutionAnalysis,
        task: &TaskRequest,
        trigger: Option<&ReplanTrigger>,
    ) -> Result<ReplanResult, PlanAndExecuteError> {
        log::info!("生成新的执行计划");
        
        // 根据策略选择重新规划方法
        match self.config.replan_strategy {
            ReplanStrategy::Conservative => {
                self.conservative_replan(current_plan, analysis, task).await
            },
            ReplanStrategy::Aggressive => {
                self.aggressive_replan(current_plan, analysis, task).await
            },
            ReplanStrategy::Adaptive => {
                self.adaptive_replan(current_plan, analysis, task, trigger).await
            },
            ReplanStrategy::Learning => {
                self.learning_replan(current_plan, analysis, task).await
            },
        }
    }

    async fn conservative_replan(
        &self,
        current_plan: &ExecutionPlan,
        analysis: &ExecutionAnalysis,
        _task: &TaskRequest,
    ) -> Result<ReplanResult, PlanAndExecuteError> {
        // 保守策略：只调整失败的步骤
        let mut new_plan = current_plan.clone();
        let mut changes = Vec::new();
        
        // 调整失败步骤的重试配置
        for step in &mut new_plan.steps {
            if analysis.failure_patterns.iter().any(|fp| 
                fp.affected_step_types.contains(&step.step_type)
            ) {
                step.retry_config.max_retries += 1;
                step.retry_config.retry_interval = (step.retry_config.retry_interval as f64 * 1.5) as u64;
                
                changes.push(PlanChange {
                    change_type: ChangeType::ModifyStep,
                    description: format!("增加步骤 '{}' 的重试次数", step.name),
                    affected_steps: vec![step.id.clone()],
                    reason: "提高容错性".to_string(),
                });
            }
        }
        
        Ok(ReplanResult {
            should_replan: true,
            new_plan: Some(new_plan),
            replan_reason: "保守策略调整".to_string(),
            changes_summary: changes,
            expected_improvements: vec!["提高步骤成功率".to_string()],
            risk_assessment: RiskAssessment {
                overall_risk: RiskLevel::Low,
                risk_factors: Vec::new(),
                mitigation_measures: Vec::new(),
            },
            confidence: 0.8,
        })
    }

    async fn aggressive_replan(
        &self,
        current_plan: &ExecutionPlan,
        _analysis: &ExecutionAnalysis,
        task: &TaskRequest,
    ) -> Result<ReplanResult, PlanAndExecuteError> {
        // 激进策略：重新生成整个计划
        let planning_result = self.planner.create_plan(task).await?;
        
        Ok(ReplanResult {
            should_replan: true,
            new_plan: Some(planning_result.plan),
            replan_reason: "激进策略重新规划".to_string(),
            changes_summary: vec![PlanChange {
                change_type: ChangeType::ChangeStrategy,
                description: "完全重新生成执行计划".to_string(),
                affected_steps: current_plan.steps.iter().map(|s| s.id.clone()).collect(),
                reason: "性能优化".to_string(),
            }],
            expected_improvements: vec![
                "大幅提高执行效率".to_string(),
                "优化资源使用".to_string(),
            ],
            risk_assessment: RiskAssessment {
                overall_risk: RiskLevel::Medium,
                risk_factors: vec![RiskFactor {
                    risk_type: "计划变更".to_string(),
                    level: RiskLevel::Medium,
                    description: "大幅变更可能引入新问题".to_string(),
                    impact: "可能影响执行稳定性".to_string(),
                    probability: 0.3,
                }],
                mitigation_measures: vec!["逐步验证新计划".to_string()],
            },
            confidence: 0.6,
        })
    }

    async fn adaptive_replan(
        &self,
        current_plan: &ExecutionPlan,
        analysis: &ExecutionAnalysis,
        task: &TaskRequest,
        _trigger: Option<&ReplanTrigger>,
    ) -> Result<ReplanResult, PlanAndExecuteError> {
        // 自适应策略：根据情况选择合适的方法
        if analysis.success_rate > 0.7 {
            // 成功率较高，使用保守策略
            self.conservative_replan(current_plan, analysis, task).await
        } else {
            // 成功率较低，使用激进策略
            self.aggressive_replan(current_plan, analysis, task).await
        }
    }

    async fn learning_replan(
        &self,
        current_plan: &ExecutionPlan,
        analysis: &ExecutionAnalysis,
        task: &TaskRequest,
    ) -> Result<ReplanResult, PlanAndExecuteError> {
        // 学习策略：基于历史数据优化
        if !self.config.learning_config.enabled || 
           self.execution_history.lock().await.len() < self.config.learning_config.min_sample_size as usize {
            // 样本不足，使用自适应策略
            return self.adaptive_replan(current_plan, analysis, task, None).await;
        }
        
        // 基于历史数据学习最佳实践
        let learned_optimizations = self.learn_from_history().await?;
        let optimized_plan = self.apply_learned_optimizations(
            current_plan,
            &learned_optimizations,
        ).await?;
        
        Ok(ReplanResult {
            should_replan: true,
            new_plan: Some(optimized_plan),
            replan_reason: "基于历史学习的优化".to_string(),
            changes_summary: learned_optimizations.into_iter()
                .map(|opt| PlanChange {
                    change_type: ChangeType::ModifyStep,
                    description: opt.description,
                    affected_steps: opt.affected_steps,
                    reason: "历史学习优化".to_string(),
                })
                .collect(),
            expected_improvements: vec![
                "基于历史经验提高成功率".to_string(),
                "优化执行时间".to_string(),
            ],
            risk_assessment: RiskAssessment {
                overall_risk: RiskLevel::Low,
                risk_factors: Vec::new(),
                mitigation_measures: Vec::new(),
            },
            confidence: 0.9,
        })
    }

    async fn analyze_step_failure(
        &self,
        failed_step: &StepResult,
    ) -> Result<FailureAnalysis, PlanAndExecuteError> {
        Ok(FailureAnalysis {
            root_cause: failed_step.error.clone().unwrap_or("未知错误".to_string()),
            failure_category: "执行错误".to_string(),
            suggested_fixes: vec![
                "增加重试次数".to_string(),
                "调整超时时间".to_string(),
                "更换工具".to_string(),
            ],
        })
    }

    async fn generate_emergency_plan(
        &self,
        current_plan: &ExecutionPlan,
        failed_step: &StepResult,
        _failure_analysis: &FailureAnalysis,
        _task: &TaskRequest,
    ) -> Result<ExecutionPlan, PlanAndExecuteError> {
        let mut emergency_plan = current_plan.clone();
        
        // 找到失败的步骤并调整
        if let Some(step) = emergency_plan.steps.iter_mut()
            .find(|s| s.id == failed_step.step_id) {
            
            // 增加重试次数
            step.retry_config.max_retries = step.retry_config.max_retries.max(3);
            
            // 调整超时时间
            if let Some(tool_config) = &mut step.tool_config {
                tool_config.timeout = tool_config.timeout.map(|t| t * 2);
            }
            
            // 添加备用参数
            step.parameters.insert(
                "emergency_mode".to_string(),
                serde_json::json!(true)
            );
        }
        
        Ok(emergency_plan)
    }

    async fn analyze_failure_patterns(
        &self,
        execution_result: &ExecutionResult,
    ) -> Result<Vec<FailurePattern>, PlanAndExecuteError> {
        let mut patterns = Vec::new();
        
        // 分析失败步骤的模式
        let failed_steps: Vec<_> = execution_result.step_results.values()
            .filter(|r| r.status == StepStatus::Failed)
            .collect();
        
        if !failed_steps.is_empty() {
            patterns.push(FailurePattern {
                failure_type: "步骤执行失败".to_string(),
                frequency: failed_steps.len() as u32,
                affected_step_types: vec![StepType::ToolCall], // 简化实现
                common_errors: failed_steps.iter()
                    .filter_map(|s| s.error.clone())
                    .collect(),
            });
        }
        
        Ok(patterns)
    }

    async fn identify_bottlenecks(
        &self,
        execution_result: &ExecutionResult,
    ) -> Result<Vec<String>, PlanAndExecuteError> {
        let mut bottlenecks = Vec::new();
        
        // 找出执行时间最长的步骤
        let avg_duration = execution_result.metrics.avg_step_duration_ms;
        
        for (step_id, result) in &execution_result.step_results {
            if result.duration_ms > avg_duration * 2 {
                bottlenecks.push(format!("步骤 '{}' 执行时间过长", step_id));
            }
        }
        
        Ok(bottlenecks)
    }

    async fn analyze_resource_usage(
        &self,
        _execution_result: &ExecutionResult,
    ) -> Result<HashMap<String, f64>, PlanAndExecuteError> {
        // 简化的资源使用分析
        let mut usage = HashMap::new();
        usage.insert("cpu".to_string(), 0.5);
        usage.insert("memory".to_string(), 0.3);
        usage.insert("network".to_string(), 0.2);
        Ok(usage)
    }

    async fn analyze_historical_performance(
        &self,
    ) -> Result<HistoricalAnalysis, PlanAndExecuteError> {
        // 分析历史执行数据
        Ok(HistoricalAnalysis {
            avg_success_rate: 0.85,
            common_failure_points: vec!["网络连接".to_string()],
            optimal_configurations: HashMap::new(),
        })
    }

    async fn identify_optimization_opportunities(
        &self,
        _current_plan: &ExecutionPlan,
        _historical_analysis: &HistoricalAnalysis,
    ) -> Result<Vec<OptimizationOpportunity>, PlanAndExecuteError> {
        // 识别优化机会
        Ok(vec![
            OptimizationOpportunity {
                description: "优化工具调用参数".to_string(),
                affected_steps: vec!["step_1".to_string()],
                expected_improvement: 0.2,
            }
        ])
    }

    async fn apply_optimizations(
        &self,
        current_plan: &ExecutionPlan,
        _opportunities: &[OptimizationOpportunity],
    ) -> Result<ExecutionPlan, PlanAndExecuteError> {
        // 应用优化
        Ok(current_plan.clone())
    }

    async fn learn_from_history(&self) -> Result<Vec<OptimizationOpportunity>, PlanAndExecuteError> {
        // 从历史中学习
        Ok(Vec::new())
    }

    async fn apply_learned_optimizations(
        &self,
        current_plan: &ExecutionPlan,
        _optimizations: &[OptimizationOpportunity],
    ) -> Result<ExecutionPlan, PlanAndExecuteError> {
        // 应用学习到的优化
        Ok(current_plan.clone())
    }

    async fn get_common_triggers(&self) -> Vec<String> {
        // 获取常见触发器
        vec!["步骤失败".to_string(), "执行超时".to_string()]
    }

    // ===== 增强的错误处理方法 =====

    /// 增强的步骤失败分析
    async fn analyze_step_failure_enhanced(
        &self,
        failed_step: &StepResult,
    ) -> Result<EnhancedFailureAnalysis, PlanAndExecuteError> {
        log::info!("进行增强的步骤失败分析");
        
        let error_message = failed_step.error.clone().unwrap_or("未知错误".to_string());
        
        // 分析错误模式
        let error_pattern = self.classify_error_pattern(&error_message).await;
        
        // 确定严重性等级
        let severity_level = self.assess_error_severity(&error_pattern, failed_step.retry_count).await;
        
        // 判断是否可恢复
        let is_recoverable = self.is_error_recoverable(&error_pattern, severity_level).await;
        
        // 生成建议修复方法
        let suggested_fixes = self.generate_fix_suggestions(&error_pattern, &error_message).await;
        
        // 分析影响范围
        let impact_scope = self.analyze_error_impact(&error_pattern).await;
        
        let root_cause = match error_pattern {
            ErrorPattern::Transient => "临时性网络或系统问题",
            ErrorPattern::Configuration => "工具配置或参数错误",
            ErrorPattern::Permission => "权限不足或认证失败",
            ErrorPattern::ResourceExhaustion => "系统资源不足",
            ErrorPattern::Logic => "执行逻辑错误",
            ErrorPattern::System => "系统级错误",
            ErrorPattern::Unknown => "未知类型错误",
        }.to_string();
        
        let failure_category = format!("{:?}类型", error_pattern);
        
        Ok(EnhancedFailureAnalysis {
            root_cause,
            failure_category,
            severity_level,
            is_recoverable,
            suggested_fixes,
            error_pattern,
            impact_scope,
        })
    }

    /// 分类错误模式
    async fn classify_error_pattern(&self, error_message: &str) -> ErrorPattern {
        let error_lower = error_message.to_lowercase();
        
        if error_lower.contains("timeout") || error_lower.contains("超时") || 
           error_lower.contains("connection") || error_lower.contains("network") {
            ErrorPattern::Transient
        } else if error_lower.contains("permission") || error_lower.contains("权限") ||
                  error_lower.contains("unauthorized") || error_lower.contains("forbidden") {
            ErrorPattern::Permission
        } else if error_lower.contains("config") || error_lower.contains("配置") ||
                  error_lower.contains("parameter") || error_lower.contains("参数") {
            ErrorPattern::Configuration
        } else if error_lower.contains("memory") || error_lower.contains("内存") ||
                  error_lower.contains("disk") || error_lower.contains("磁盘") ||
                  error_lower.contains("resource") {
            ErrorPattern::ResourceExhaustion
        } else if error_lower.contains("logic") || error_lower.contains("逻辑") ||
                  error_lower.contains("validation") || error_lower.contains("invalid") {
            ErrorPattern::Logic
        } else if error_lower.contains("system") || error_lower.contains("系统") ||
                  error_lower.contains("internal") || error_lower.contains("crash") {
            ErrorPattern::System
        } else {
            ErrorPattern::Unknown
        }
    }

    /// 评估错误严重性
    async fn assess_error_severity(&self, error_pattern: &ErrorPattern, retry_count: u32) -> u32 {
        let base_severity = match error_pattern {
            ErrorPattern::Transient => 2,
            ErrorPattern::Configuration => 3,
            ErrorPattern::Permission => 4,
            ErrorPattern::ResourceExhaustion => 4,
            ErrorPattern::Logic => 3,
            ErrorPattern::System => 5,
            ErrorPattern::Unknown => 3,
        };
        
        // 重试次数影响严重性
        let retry_penalty = (retry_count / 2).min(2);
        
        (base_severity + retry_penalty).min(5)
    }

    /// 判断错误是否可恢复
    async fn is_error_recoverable(&self, error_pattern: &ErrorPattern, severity_level: u32) -> bool {
        if severity_level >= 5 {
            return false;
        }
        
        matches!(error_pattern, 
            ErrorPattern::Transient | 
            ErrorPattern::Configuration | 
            ErrorPattern::Permission |
            ErrorPattern::Logic
        )
    }

    /// 生成修复建议
    async fn generate_fix_suggestions(&self, error_pattern: &ErrorPattern, error_message: &str) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        match error_pattern {
            ErrorPattern::Transient => {
                suggestions.push("增加重试次数和间隔".to_string());
                suggestions.push("检查网络连接".to_string());
                suggestions.push("添加指数退避重试策略".to_string());
            },
            ErrorPattern::Configuration => {
                suggestions.push("检查工具参数配置".to_string());
                suggestions.push("验证配置文件格式".to_string());
                suggestions.push("使用默认配置重试".to_string());
            },
            ErrorPattern::Permission => {
                suggestions.push("检查权限设置".to_string());
                suggestions.push("更新认证凭据".to_string());
                suggestions.push("使用其他账户重试".to_string());
            },
            ErrorPattern::ResourceExhaustion => {
                suggestions.push("释放系统资源".to_string());
                suggestions.push("减少并发执行".to_string());
                suggestions.push("分批处理数据".to_string());
            },
            ErrorPattern::Logic => {
                suggestions.push("检查输入数据格式".to_string());
                suggestions.push("调整执行逻辑".to_string());
                suggestions.push("添加数据验证".to_string());
            },
            ErrorPattern::System => {
                suggestions.push("检查系统状态".to_string());
                suggestions.push("重启相关服务".to_string());
                suggestions.push("联系系统管理员".to_string());
            },
            ErrorPattern::Unknown => {
                suggestions.push("详细分析错误日志".to_string());
                suggestions.push("尝试其他方法".to_string());
                suggestions.push("联系技术支持".to_string());
            },
        }
        
        // 基于错误消息的具体建议
        if error_message.contains("timeout") {
            suggestions.push("增加超时时间".to_string());
        }
        
        suggestions
    }

    /// 分析错误影响范围
    async fn analyze_error_impact(&self, error_pattern: &ErrorPattern) -> Vec<String> {
        match error_pattern {
            ErrorPattern::Transient => vec!["当前步骤".to_string(), "依赖此步骤的后续步骤".to_string()],
            ErrorPattern::Configuration => vec!["相同类型的所有步骤".to_string()],
            ErrorPattern::Permission => vec!["需要此权限的所有步骤".to_string()],
            ErrorPattern::ResourceExhaustion => vec!["整个执行环境".to_string()],
            ErrorPattern::Logic => vec!["当前步骤".to_string()],
            ErrorPattern::System => vec!["整个系统".to_string()],
            ErrorPattern::Unknown => vec!["影响范围未知".to_string()],
        }
    }

    /// 确定恢复策略
    async fn determine_recovery_strategy(
        &self,
        failure_analysis: &EnhancedFailureAnalysis,
        failed_step: &StepResult,
    ) -> RecoveryStrategy {
        // 基于错误严重性和可恢复性选择策略
        if !failure_analysis.is_recoverable || failure_analysis.severity_level >= 5 {
            return RecoveryStrategy::AbortTask;
        }
        
        // 基于重试次数判断
        if failed_step.retry_count >= 3 {
            match failure_analysis.error_pattern {
                ErrorPattern::Transient => RecoveryStrategy::AlternativeApproach,
                ErrorPattern::Configuration => RecoveryStrategy::RetryWithAdjustment,
                ErrorPattern::Permission => RecoveryStrategy::SkipAndContinue,
                _ => RecoveryStrategy::RetryWithAdjustment,
            }
        } else {
            match failure_analysis.error_pattern {
                ErrorPattern::Transient => RecoveryStrategy::RetryWithAdjustment,
                ErrorPattern::Configuration => RecoveryStrategy::RetryWithAdjustment,
                ErrorPattern::Permission => RecoveryStrategy::AlternativeApproach,
                ErrorPattern::ResourceExhaustion => RecoveryStrategy::AlternativeApproach,
                ErrorPattern::Logic => RecoveryStrategy::RetryWithAdjustment,
                ErrorPattern::System => RecoveryStrategy::RetryWithAdjustment,
                ErrorPattern::Unknown => RecoveryStrategy::RetryWithAdjustment,
            }
        }
    }

    /// 生成重试计划
    async fn generate_retry_plan(
        &self,
        current_plan: &ExecutionPlan,
        failed_step: &StepResult,
        failure_analysis: &EnhancedFailureAnalysis,
    ) -> Result<ExecutionPlan, PlanAndExecuteError> {
        let mut new_plan = current_plan.clone();
        
        // 找到失败的步骤并调整
        if let Some(step) = new_plan.steps.iter_mut()
            .find(|s| s.id == failed_step.step_id) {
            
            // 根据错误模式调整参数
            match failure_analysis.error_pattern {
                ErrorPattern::Transient => {
                    step.retry_config.max_retries += 2;
                    step.retry_config.retry_interval *= 2;
                    if let Some(tool_config) = &mut step.tool_config {
                        if let Some(timeout) = tool_config.timeout {
                            tool_config.timeout = Some(timeout * 2);
                        }
                    }
                },
                ErrorPattern::Configuration => {
                    // 尝试不同的工具参数
                    if let Some(tool_config) = &mut step.tool_config {
                        tool_config.tool_args.insert(
                            "alternative_mode".to_string(),
                            serde_json::json!(true)
                        );
                    }
                },
                _ => {
                    step.retry_config.max_retries += 1;
                }
            }
            
            log::info!("调整失败步骤 '{}' 的重试策略", step.name);
        }
        
        Ok(new_plan)
    }

    /// 生成替代方法计划
    async fn generate_alternative_approach_plan(
        &self,
        current_plan: &ExecutionPlan,
        failed_step: &StepResult,
        _task: &TaskRequest,
    ) -> Result<ExecutionPlan, PlanAndExecuteError> {
        let mut new_plan = current_plan.clone();
        
        // 为失败步骤创建替代方法
        if let Some(step_index) = new_plan.steps.iter()
            .position(|s| s.id == failed_step.step_id) {
            
            let original_step = new_plan.steps[step_index].clone();
            
            // 创建替代步骤
            let alternative_step = ExecutionStep {
                id: format!("{}_alternative", original_step.id),
                name: format!("{} (替代方法)", original_step.name),
                description: format!("{}的替代实现", original_step.description),
                step_type: original_step.step_type.clone(),
                tool_config: Some(ToolConfig {
                    tool_name: "alternative_tool".to_string(),
                    tool_version: None,
                    tool_args: HashMap::new(),
                    timeout: Some(300),
                    env_vars: HashMap::new(),
                }),
                parameters: original_step.parameters.clone(),
                estimated_duration: original_step.estimated_duration,
                retry_config: RetryConfig::default(),
                preconditions: Vec::new(),
                postconditions: Vec::new(),
            };
            
            // 替换原步骤
            new_plan.steps[step_index] = alternative_step;
            
            log::info!("为步骤 '{}' 生成替代方法", original_step.name);
        }
        
        Ok(new_plan)
    }

    /// 生成跳过计划
    async fn generate_skip_plan(
        &self,
        current_plan: &ExecutionPlan,
        failed_step: &StepResult,
    ) -> Result<ExecutionPlan, PlanAndExecuteError> {
        let mut new_plan = current_plan.clone();
        
        // 移除失败的步骤
        new_plan.steps.retain(|s| s.id != failed_step.step_id);
        
        log::info!("跳过失败步骤 '{}'，继续执行后续步骤", failed_step.step_id);
        
        Ok(new_plan)
    }


}

// 辅助结构体定义

#[derive(Debug, Clone)]
struct FailureAnalysis {
    root_cause: String,
    #[allow(unused)]
    failure_category: String,
    #[allow(unused)]
    suggested_fixes: Vec<String>,
}

#[derive(Debug, Clone)]
struct HistoricalAnalysis {
    #[allow(unused)]
    avg_success_rate: f64,
    #[allow(unused)]
    common_failure_points: Vec<String>,
    #[allow(unused)]
    optimal_configurations: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
struct OptimizationOpportunity {
    description: String,
    affected_steps: Vec<String>,
    #[allow(unused)]
    expected_improvement: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplanStatistics {
    pub total_replans: u32,
    pub successful_replans: u32,
    pub success_rate: f64,
    pub avg_confidence: f64,
    pub most_common_triggers: Vec<String>,
}

// 默认实现

impl Default for ReplannerConfig {
    fn default() -> Self {
        Self {
            auto_replan_enabled: true,
            replan_threshold: ReplanThreshold::default(),
            max_replan_attempts: 3,
            replan_strategy: ReplanStrategy::Adaptive,
            learning_config: LearningConfig::default(),
        }
    }
}

impl Default for ReplanThreshold {
    fn default() -> Self {
        Self {
            failure_rate_threshold: 0.3,
            timeout_ratio_threshold: 1.5,
            consecutive_failures_threshold: 3,
            resource_usage_threshold: 0.8,
        }
    }
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            history_retention_days: 30,
            learning_weight: 0.7,
            min_sample_size: 10,
        }
    }
}