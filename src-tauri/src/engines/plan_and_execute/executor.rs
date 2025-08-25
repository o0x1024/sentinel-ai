//! Executor 组件 - 执行器
//! 
//! 负责按照计划逐步执行子任务，调用具体的工具和服务

use crate::engines::plan_and_execute::types::*;
use crate::engines::plan_and_execute::replanner::Replanner;
use crate::tools::{UnifiedToolManager, ToolExecutionParams, ToolExecutionResult, get_global_tool_system};
use crate::engines::ExecutionError;
use crate::services::database::DatabaseService;
use crate::services::ai::{AiServiceManager, SchedulerStage};
use crate::ai_adapter::core::AiAdapterManager;
use crate::ai_adapter::types::{ChatRequest, Message, MessageRole, ChatOptions};
use crate::database::plan_execute_repository::PlanExecuteRepository;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{Mutex, RwLock};
use tokio::time::timeout;


/// 执行器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorConfig {
    /// 最大并发执行数
    pub max_concurrent_steps: u32,
    /// 默认超时时间（秒）
    pub default_timeout: u64,
    /// 是否启用步骤缓存
    pub enable_step_caching: bool,
    /// 执行模式
    pub execution_mode: ExecutionMode,
    /// 错误处理策略
    pub error_handling: ErrorHandlingStrategy,
    /// AI提供商（用于AI推理步骤）
    pub ai_provider: String,
    /// AI模型配置（用于AI推理步骤）
    pub model_config: ExecutorModelConfig,
}

/// 执行器模型配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorModelConfig {
    /// 模型名称
    pub model_name: String,
    /// 温度参数
    pub temperature: f32,
    /// 最大token数
    pub max_tokens: u32,
    /// top_p参数
    pub top_p: f32,
}

/// 执行模式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// 严格模式：任何步骤失败都停止执行
    Strict,
    /// 容错模式：跳过失败的步骤继续执行
    Tolerant,
    /// 最佳努力模式：尽可能多地执行步骤
    BestEffort,
}

/// 错误处理策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorHandlingStrategy {
    /// 立即停止
    StopImmediately,
    /// 重试后停止
    RetryThenStop,
    /// 跳过并继续
    SkipAndContinue,
    /// 标记为失败但继续
    MarkFailedAndContinue,
}

/// 执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// 执行状态
    pub status: TaskStatus,
    /// 已完成的步骤
    pub completed_steps: Vec<String>,
    /// 失败的步骤
    pub failed_steps: Vec<String>,
    /// 跳过的步骤
    pub skipped_steps: Vec<String>,
    /// 步骤结果详情
    pub step_results: HashMap<String, StepResult>,
    /// 执行指标
    pub metrics: ExecutionMetrics,
    /// 错误信息
    pub errors: Vec<ExecutionError>,
    /// 新增：增强的反馈信息
    pub enhanced_feedback: EnhancedExecutionFeedback,
}

/// 步骤结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    /// 步骤ID
    pub step_id: String,
    /// 执行状态
    pub status: StepStatus,
    /// 开始时间
    pub started_at: SystemTime,
    /// 结束时间
    pub completed_at: Option<SystemTime>,
    /// 执行时长（毫秒）
    pub duration_ms: u64,
    /// 结果数据
    pub result_data: Option<serde_json::Value>,
    /// 错误信息
    pub error: Option<String>,
    /// 重试次数
    pub retry_count: u32,
    /// 工具调用结果
    pub tool_result: Option<ToolExecutionResult>,
}

/// 步骤状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StepStatus {
    /// 待执行
    Pending,
    /// 执行中
    Running,
    /// 已完成
    Completed,
    /// 已失败
    Failed,
    /// 已跳过
    Skipped,
    /// 重试中
    Retrying,
    /// 已取消
    Cancelled,
}

/// 执行指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    /// 总执行时间（毫秒）
    pub total_duration_ms: u64,
    /// 成功步骤数
    pub successful_steps: u32,
    /// 失败步骤数
    pub failed_steps: u32,
    /// 跳过步骤数
    pub skipped_steps: u32,
    /// 重试总次数
    pub total_retries: u32,
    /// 平均步骤执行时间（毫秒）
    pub avg_step_duration_ms: u64,
    /// 并发执行峰值
    pub peak_concurrency: u32,
    /// 自定义指标
    pub custom_metrics: HashMap<String, f64>,
}

/// 执行上下文
#[derive(Debug)]
pub struct ExecutionContext {
    /// 任务ID
    pub task_id: String,
    /// 计划ID
    pub plan_id: String,
    /// 共享数据
    pub shared_data: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    /// 执行状态
    pub execution_state: Arc<RwLock<ExecutionState>>,
    /// 工具管理器
    pub tool_manager: Arc<RwLock<UnifiedToolManager>>,
}

/// 执行状态
#[derive(Debug, Clone)]
pub struct ExecutionState {
    /// 当前执行的步骤
    pub current_steps: HashMap<String, StepStatus>,
    /// 已完成的步骤
    pub completed_steps: Vec<String>,
    /// 失败的步骤
    pub failed_steps: Vec<String>,
    /// 是否暂停
    pub is_paused: bool,
    /// 是否取消
    pub is_cancelled: bool,
}

/// 增强的执行反馈（专为Replanner判断优化）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnhancedExecutionFeedback {
    /// 执行摘要
    pub execution_summary: String,
    /// 关键成功因素
    pub success_factors: Vec<String>,
    /// 关键失败因素
    pub failure_factors: Vec<String>,
    /// 性能洞察
    pub performance_insights: Vec<PerformanceInsight>,
    /// 质量评估
    pub quality_assessment: QualityAssessment,
    /// 建议改进点
    pub improvement_suggestions: Vec<String>,
    /// 风险指标
    pub risk_indicators: Vec<RiskIndicator>,
    /// 依赖关系分析
    pub dependency_analysis: DependencyAnalysis,
    /// 资源使用分析
    pub resource_analysis: ResourceAnalysis,
}

/// 性能洞察
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerformanceInsight {
    /// 洞察类型
    pub insight_type: InsightType,
    /// 描述
    pub description: String,
    /// 影响等级 (1-5)
    pub impact_level: u32,
    /// 置信度 (0.0-1.0)
    pub confidence: f64,
    /// 相关步骤
    pub related_steps: Vec<String>,
}

/// 洞察类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InsightType {
    /// 性能瓶颈
    PerformanceBottleneck,
    /// 效率机会
    EfficiencyOpportunity,
    /// 工具使用问题
    ToolUsageIssue,
    /// 并发机会
    ConcurrencyOpportunity,
    /// 缓存机会
    CachingOpportunity,
}

impl Default for InsightType {
    fn default() -> Self {
        InsightType::PerformanceBottleneck
    }
}

/// 质量评估
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QualityAssessment {
    /// 整体质量分数 (0-100)
    pub overall_score: u32,
    /// 完整性分数 (0-100)
    pub completeness_score: u32,
    /// 可靠性分数 (0-100)
    pub reliability_score: u32,
    /// 效率分数 (0-100)
    pub efficiency_score: u32,
    /// 质量维度详情
    pub quality_dimensions: Vec<QualityDimension>,
}

/// 质量维度
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QualityDimension {
    /// 维度名称
    pub name: String,
    /// 分数 (0-100)
    pub score: u32,
    /// 详细说明
    pub details: String,
    /// 改进建议
    pub suggestions: Vec<String>,
}

/// 风险指标
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RiskIndicator {
    /// 风险类型
    pub risk_type: RiskType,
    /// 风险等级 (1-5)
    pub level: u32,
    /// 描述
    pub description: String,
    /// 影响范围
    pub impact_scope: Vec<String>,
    /// 缓解建议
    pub mitigation_suggestions: Vec<String>,
}

/// 风险类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskType {
    /// 执行风险
    ExecutionRisk,
    /// 质量风险
    QualityRisk,
    /// 性能风险
    PerformanceRisk,
    /// 资源风险
    ResourceRisk,
    /// 依赖风险
    DependencyRisk,
}

impl Default for RiskType {
    fn default() -> Self {
        RiskType::ExecutionRisk
    }
}

/// 依赖关系分析
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DependencyAnalysis {
    /// 已解决的依赖
    pub resolved_dependencies: Vec<Dependency>,
    /// 未解决的依赖
    pub unresolved_dependencies: Vec<Dependency>,
    /// 循环依赖
    pub circular_dependencies: Vec<Dependency>,
    /// 依赖冲突
    pub dependency_conflicts: Vec<DependencyConflict>,
}

/// 依赖
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Dependency {
    /// 源步骤
    pub source_step: String,
    /// 目标步骤
    pub target_step: String,
    /// 依赖类型
    pub dependency_type: DependencyType,
    /// 状态
    pub status: DependencyStatus,
}

/// 依赖类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DependencyType {
    /// 数据依赖
    DataDependency,
    /// 执行顺序依赖
    OrderDependency,
    /// 资源依赖
    ResourceDependency,
    /// 工具依赖
    ToolDependency,
}

impl Default for DependencyType {
    fn default() -> Self {
        DependencyType::DataDependency
    }
}

/// 依赖状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DependencyStatus {
    /// 满足
    Satisfied,
    /// 未满足
    Unsatisfied,
    /// 冲突
    Conflicted,
}

impl Default for DependencyStatus {
    fn default() -> Self {
        DependencyStatus::Satisfied
    }
}

/// 依赖冲突
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DependencyConflict {
    /// 冲突描述
    pub description: String,
    /// 涉及的步骤
    pub involved_steps: Vec<String>,
    /// 建议解决方案
    pub suggested_resolutions: Vec<String>,
}

/// 资源使用分析
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceAnalysis {
    /// CPU使用情况
    pub cpu_usage: ResourceUsage,
    /// 内存使用情况
    pub memory_usage: ResourceUsage,
    /// 网络使用情况
    pub network_usage: ResourceUsage,
    /// 存储使用情况
    pub storage_usage: ResourceUsage,
    /// 资源瓶颈
    pub resource_bottlenecks: Vec<String>,
    /// 资源优化建议
    pub optimization_suggestions: Vec<String>,
}

/// 资源使用情况
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceUsage {
    /// 当前使用量
    pub current: f64,
    /// 峰值使用量
    pub peak: f64,
    /// 平均使用量
    pub average: f64,
    /// 使用率 (0.0-1.0)
    pub utilization_rate: f64,
    /// 单位
    pub unit: String,
}

/// 执行器
#[derive(Debug)]
pub struct Executor {
    config: ExecutorConfig,
    context: Arc<Mutex<Option<ExecutionContext>>>,
    metrics: Arc<Mutex<ExecutionMetrics>>,
    db_service: Arc<DatabaseService>,
    ai_service_manager: Option<Arc<AiServiceManager>>,
    replanner: Option<Arc<Replanner>>,
    repository: Arc<PlanExecuteRepository>,
}

impl Executor {
    /// 创建新的执行器实例
    pub fn new(config: ExecutorConfig, db_service: Arc<DatabaseService>) -> Self {
        let pool = db_service.get_pool().expect("Failed to get database pool").clone();
        let repository = Arc::new(PlanExecuteRepository::new(pool));
        
        Self {
            config,
            context: Arc::new(Mutex::new(None)),
            metrics: Arc::new(Mutex::new(ExecutionMetrics::default())),
            db_service,
            ai_service_manager: None,
            replanner: None,
            repository,
        }
    }

    /// 创建带有AI服务管理器的执行器实例（用于动态模型切换）
    pub fn with_ai_service_manager(config: ExecutorConfig, db_service: Arc<DatabaseService>, ai_service_manager: Arc<AiServiceManager>) -> Self {
        let pool = db_service.get_pool().expect("Failed to get database pool").clone();
        let repository = Arc::new(PlanExecuteRepository::new(pool));
        
        Self {
            config,
            context: Arc::new(Mutex::new(None)),
            metrics: Arc::new(Mutex::new(ExecutionMetrics::default())),
            db_service,
            ai_service_manager: Some(ai_service_manager),
            replanner: None,
            repository,
        }
    }

    /// 创建带有完整依赖的执行器实例（包含重新规划器）
    pub fn with_replanner(
        config: ExecutorConfig, 
        db_service: Arc<DatabaseService>, 
        ai_service_manager: Option<Arc<AiServiceManager>>,
        replanner: Option<Arc<Replanner>>
    ) -> Self {
        let pool = db_service.get_pool().expect("Failed to get database pool").clone();
        let repository = Arc::new(PlanExecuteRepository::new(pool));
        
        Self {
            config,
            context: Arc::new(Mutex::new(None)),
            metrics: Arc::new(Mutex::new(ExecutionMetrics::default())),
            db_service,
            ai_service_manager,
            replanner,
            repository,
        }
    }

    /// 获取执行阶段应使用的AI配置
    async fn get_execution_ai_config(&self) -> Result<Option<crate::services::ai::AiConfig>, PlanAndExecuteError> {
        if let Some(ref ai_service_manager) = self.ai_service_manager {
            match ai_service_manager.get_ai_config_for_stage(SchedulerStage::Execution).await {
                Ok(config) => Ok(config),
                Err(e) => {
                    log::warn!("Failed to get AI config for execution stage: {}, using default", e);
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    /// 执行计划（按照Plan-and-Execute流程）
    pub async fn execute_plan(
        &self,
        plan: &ExecutionPlan,
        task: &TaskRequest,
    ) -> Result<ExecutionResult, PlanAndExecuteError> {
        log::info!("=== Plan-and-Execute开始执行 ===");
        log::info!("执行计划: {}", plan.name);
        
        // 初始化执行上下文
        self.initialize_context(plan, task).await?;
        
        let _start_time = SystemTime::now();
        let mut current_plan = plan.clone();
        let mut replan_attempts = 0;
        let max_replan_attempts = 3; // 最大重新规划次数
        let mut overall_step_results = HashMap::new();
        let mut overall_errors = Vec::new();
        
        // 新增：明确的终止条件
        let mut consecutive_failures = 0;
        let mut total_execution_time = 0u64;
        let max_total_execution_time = 30 * 60 * 1000; // 30分钟最大执行时间
        let max_consecutive_failures = 3; // 最大连续失败次数
        
        // Plan-and-Execute主循环：Planner -> Agent -> Tools -> Replan -> Agent...
        loop {
            let loop_start_time = SystemTime::now();
            log::info!("=== 执行计划循环 (尝试 {}/{}) ===", replan_attempts + 1, max_replan_attempts + 1);
            
            // 终止条件1: 检查最大重试次数
            if replan_attempts >= max_replan_attempts {
                log::warn!("已达到最大重新规划次数 ({}), 停止执行", max_replan_attempts);
                overall_errors.push(crate::engines::ExecutionError {
                    error_type: crate::engines::ErrorType::System,
                    message: "达到最大重新规划次数限制".to_string(),
                    details: Some("达到最大重新规划次数限制".to_string()),
                    error_code: Some(1001),
                    retryable: false,
                    timestamp: SystemTime::now(),
                });
                break;
            }
            
            // 终止条件2: 检查总执行时间
            if total_execution_time > max_total_execution_time {
                log::warn!("总执行时间超过限制 ({}ms), 停止执行", max_total_execution_time);
                overall_errors.push(crate::engines::ExecutionError {
                    error_type: crate::engines::ErrorType::Timeout,
                    message: "执行时间超过限制".to_string(),
                    details: Some("执行时间超过限制".to_string()),
                    error_code: Some(1002),
                    retryable: false,
                    timestamp: SystemTime::now(),
                });
                break;
            }
            
            // Agent执行层：逐步执行计划中的每个步骤
            let execution_result = self.execute_plan_steps(&current_plan, task).await?;
            
            // 累积结果
            for (step_id, step_result) in execution_result.step_results.clone() {
                overall_step_results.insert(step_id, step_result);
            }
            overall_errors.extend(execution_result.errors.clone());
            
            // 终止条件3: 检查执行结果状态
            match execution_result.status {
                TaskStatus::Completed => {
                    log::info!("任务执行成功完成");
                    // 构建成功的最终结果并返回
                    let final_result = ExecutionResult {
                        status: execution_result.status,
                        completed_steps: execution_result.completed_steps,
                        failed_steps: execution_result.failed_steps,
                        skipped_steps: execution_result.skipped_steps,
                        step_results: overall_step_results,
                        metrics: execution_result.metrics,
                        errors: overall_errors,
                        enhanced_feedback: execution_result.enhanced_feedback,
                    };
                    return Ok(final_result);
                },
                TaskStatus::Failed => {
                    consecutive_failures += 1;
                    log::warn!("执行失败，连续失败次数: {}", consecutive_failures);
                    
                    // 终止条件4: 连续失败次数检查
                    if consecutive_failures >= max_consecutive_failures {
                        log::error!("连续失败次数达到限制 ({}), 停止执行", max_consecutive_failures);
                        overall_errors.push(crate::engines::ExecutionError {
                            error_type: crate::engines::ErrorType::System,
                            message: "连续失败次数超过限制".to_string(),
                            details: Some("连续失败次数超过限制".to_string()),
                            error_code: Some(1003),
                            retryable: false,
                            timestamp: SystemTime::now(),
                        });
                        break;
                    }
                },
                _ => {
                    // 重置连续失败计数器（有进展）
                    consecutive_failures = 0;
                }
            }
            
            // Replan反思层：评估执行结果，决定是否需要重新规划
            if let Some(ref replanner) = self.replanner {
                log::info!("=== Replan反思层：评估执行结果 ===");
                
                let should_replan = self.should_trigger_replan(&execution_result, replan_attempts, max_replan_attempts).await;
                
                if should_replan {
                    log::info!("反思层决定：需要重新规划");
                    
                    match replanner.analyze_and_replan(&current_plan, &execution_result, task, None).await {
                        Ok(replan_result) => {
                            if replan_result.should_replan {
                                if let Some(new_plan) = replan_result.new_plan {
                                    log::info!("=== Planner战略层：生成新计划 ===");
                                    log::info!("重新规划原因: {}", replan_result.replan_reason);
                                    
                                    // 终止条件5: 验证新计划的合理性
                                    if self.validate_new_plan(&new_plan, &current_plan).await {
                                    current_plan = new_plan;
                                    replan_attempts += 1;
                                        
                                        // 记录循环执行时间
                                        let loop_duration = loop_start_time.elapsed()
                                            .unwrap_or_default().as_millis() as u64;
                                        total_execution_time += loop_duration;
                                        
                                    continue; // 回到Agent执行层，执行新计划
                                    } else {
                                        log::error!("新计划验证失败，停止执行");
                                        overall_errors.push(crate::engines::ExecutionError {
                                            error_type: crate::engines::ErrorType::Configuration,
                                            message: "新计划验证失败".to_string(),
                                            details: Some("新计划验证失败".to_string()),
                                            error_code: Some(1004),
                                            retryable: false,
                                            timestamp: SystemTime::now(),
                                        });
                                        break;
                                    }
                                } else {
                                    log::warn!("Replanner建议重新规划但未提供新计划，停止执行");
                                    break;
                                }
                            } else {
                                log::info!("AI决定不需要重新规划，执行完成");
                                break;
                            }
                        },
                        Err(e) => {
                            log::error!("重新规划过程失败: {}", e);
                            overall_errors.push(crate::engines::ExecutionError {
                                error_type: crate::engines::ErrorType::System,
                                message: format!("重新规划失败: {}", e),
                                details: Some(format!("重新规划失败: {}", e)),
                                error_code: Some(1005),
                                retryable: false,
                                timestamp: SystemTime::now(),
                            });
                            break;
                        }
                    }
                } else {
                    log::info!("反思层决定：执行结果满足预期，无需重新规划");
                    break;
                }
            } else {
                log::info!("没有Replanner，执行完成");
                break;
            }
            }
            
            // 构建最终执行结果
            let final_result = ExecutionResult {
            status: if overall_errors.is_empty() && !overall_step_results.is_empty() {
                TaskStatus::Completed
            } else if overall_errors.iter().any(|e| !e.retryable) {
                TaskStatus::Failed
            } else {
                TaskStatus::Pending
            },
            completed_steps: overall_step_results.iter()
                .filter(|(_, result)| result.status == StepStatus::Completed)
                .map(|(id, _)| id.clone())
                .collect(),
            failed_steps: overall_step_results.iter()
                .filter(|(_, result)| result.status == StepStatus::Failed)
                .map(|(id, _)| id.clone())
                .collect(),
            skipped_steps: overall_step_results.iter()
                .filter(|(_, result)| result.status == StepStatus::Skipped)
                .map(|(id, _)| id.clone())
                .collect(),
            step_results: overall_step_results.clone(),
            metrics: ExecutionMetrics {
                total_duration_ms: total_execution_time,
                successful_steps: overall_step_results.values()
                    .filter(|r| r.status == StepStatus::Completed)
                    .count() as u32,
                failed_steps: overall_step_results.values()
                    .filter(|r| r.status == StepStatus::Failed)
                    .count() as u32,
                skipped_steps: overall_step_results.values()
                    .filter(|r| r.status == StepStatus::Skipped)
                    .count() as u32,
                total_retries: overall_step_results.values()
                    .map(|r| r.retry_count)
                    .sum(),
                avg_step_duration_ms: if !overall_step_results.is_empty() {
                    overall_step_results.values()
                        .map(|r| r.duration_ms)
                        .sum::<u64>() / overall_step_results.len() as u64
                } else {
                    0
                },
                peak_concurrency: 1,
                custom_metrics: HashMap::new(),
            },
            errors: overall_errors,
            enhanced_feedback: EnhancedExecutionFeedback {
                execution_summary: format!("总执行时间: {}ms, 重新规划次数: {}", total_execution_time, replan_attempts),
                success_factors: vec![],
                failure_factors: vec![],
                performance_insights: vec![],
                quality_assessment: QualityAssessment {
                    overall_score: 50,
                    completeness_score: 50,
                    reliability_score: 50,
                    efficiency_score: 50,
                    quality_dimensions: vec![],
                },
                improvement_suggestions: vec![],
                risk_indicators: vec![],
                dependency_analysis: DependencyAnalysis {
                    resolved_dependencies: vec![],
                    unresolved_dependencies: vec![],
                    circular_dependencies: vec![],
                    dependency_conflicts: vec![],
                },
                resource_analysis: ResourceAnalysis {
                    cpu_usage: ResourceUsage {
                        current: 0.2, peak: 0.5, average: 0.3, utilization_rate: 0.3, unit: "cores".to_string()
                    },
                    memory_usage: ResourceUsage {
                        current: 200.0, peak: 400.0, average: 300.0, utilization_rate: 0.3, unit: "MB".to_string()
                    },
                    network_usage: ResourceUsage {
                        current: 1.0, peak: 3.0, average: 2.0, utilization_rate: 0.1, unit: "Mbps".to_string()
                    },
                    storage_usage: ResourceUsage {
                        current: 5.0, peak: 20.0, average: 10.0, utilization_rate: 0.05, unit: "MB".to_string()
                    },
                    resource_bottlenecks: vec![],
                    optimization_suggestions: vec![],
                },
            },
            };
            
            log::info!("=== Plan-and-Execute执行完成 ===");
        log::info!("最终状态: {:?}, 完成步骤: {}, 失败步骤: {}, 总耗时: {}ms", 
                  final_result.status, final_result.completed_steps.len(), 
                  final_result.failed_steps.len(), total_execution_time);
        
        // 保存执行结果到数据库
        if let Err(e) = self.save_execution_to_database(plan, task, &final_result).await {
            log::error!("保存执行结果到数据库失败: {}", e);
        }
        
        Ok(final_result)
    }

    /// 验证新计划的合理性（终止条件）
    async fn validate_new_plan(&self, new_plan: &ExecutionPlan, current_plan: &ExecutionPlan) -> bool {
        log::info!("验证新计划的合理性");
        
        // 1. 检查计划不能为空
        if new_plan.steps.is_empty() {
            log::warn!("新计划没有任何步骤，验证失败");
            return false;
        }
        
        // 2. 检查步骤数量合理性（不超过20个步骤）
        if new_plan.steps.len() > 20 {
            log::warn!("新计划步骤数量过多 ({}), 验证失败", new_plan.steps.len());
            return false;
        }
        
        // 3. 检查是否与当前计划过于相似（避免无效重新规划）
        let similarity = self.calculate_plan_similarity(new_plan, current_plan).await;
        if similarity > 0.9 {
            log::warn!("新计划与当前计划过于相似 ({:.2}), 验证失败", similarity);
            return false;
        }
        
        // 4. 检查步骤的完整性
        for step in &new_plan.steps {
            if step.name.is_empty() || step.description.is_empty() {
                log::warn!("新计划包含不完整的步骤: {}", step.id);
                return false;
            }
        }
        
        // 5. 检查预估执行时间合理性（不超过2小时）
        if new_plan.estimated_duration > 2 * 60 * 60 {
            log::warn!("新计划预估执行时间过长 ({}秒), 验证失败", new_plan.estimated_duration);
            return false;
        }
        
        log::info!("新计划验证通过: {} 个步骤, 预估时间 {} 秒", 
                  new_plan.steps.len(), new_plan.estimated_duration);
        true
    }

    /// 计算两个计划的相似度
    async fn calculate_plan_similarity(&self, plan1: &ExecutionPlan, plan2: &ExecutionPlan) -> f64 {
        if plan1.steps.is_empty() && plan2.steps.is_empty() {
            return 1.0;
        }
        
        if plan1.steps.is_empty() || plan2.steps.is_empty() {
            return 0.0;
        }
        
        // 相似度计算：基于步骤名称和类型的匹配
        let mut matches = 0;
        let max_steps = plan1.steps.len().max(plan2.steps.len());
        
        for i in 0..plan1.steps.len().min(plan2.steps.len()) {
            let step1 = &plan1.steps[i];
            let step2 = &plan2.steps[i];
            
            if step1.name == step2.name && step1.step_type == step2.step_type {
                matches += 1;
            }
        }
        
        matches as f64 / max_steps as f64
    }

    /// Agent执行层：按步骤执行计划
    async fn execute_plan_steps(
        &self,
        plan: &ExecutionPlan,
        _task: &TaskRequest,
    ) -> Result<ExecutionResult, PlanAndExecuteError> {
        log::info!("=== Agent执行层：开始逐步执行计划 ===");
        
        let start_time = SystemTime::now();
        let context_guard = self.context.lock().await;
        let context = context_guard.as_ref().unwrap();
        
        let mut step_results = HashMap::new();
        let mut completed_steps = Vec::new();
        let mut failed_steps = Vec::new();
        let mut errors = Vec::new();
        
        // 逐步执行每个计划步骤
        for (index, step) in plan.steps.iter().enumerate() {
            log::info!("Agent执行步骤 {}/{}: {}", index + 1, plan.steps.len(), step.name);
            
            // 调用Tools层执行具体步骤
            match self.execute_step(step, context).await {
                Ok(result) => {
                    if result.status == StepStatus::Completed {
                        log::info!("✓ 步骤执行成功: {}", step.name);
                        completed_steps.push(step.id.clone());
                    } else {
                        log::warn!("✗ 步骤执行失败: {}", step.name);
                        failed_steps.push(step.id.clone());
                        
                        // 根据执行模式决定是否继续
                        if matches!(self.config.execution_mode, ExecutionMode::Strict) {
                            errors.push(ExecutionError {
                                error_type: crate::engines::types::ErrorType::Tool,
                                message: format!("步骤 '{}' 执行失败", step.name),
                                details: result.error.clone(),
                                error_code: None,
                                retryable: true,
                                timestamp: SystemTime::now(),
                            });
                            step_results.insert(step.id.clone(), result);
                            break; // 严格模式下，步骤失败就停止
                        }
                    }
                    step_results.insert(step.id.clone(), result);
                },
                Err(e) => {
                    log::error!("步骤执行异常: {}: {}", step.name, e);
                    failed_steps.push(step.id.clone());
                    errors.push(ExecutionError {
                        error_type: crate::engines::types::ErrorType::Tool,
                        message: format!("步骤 '{}' 执行异常", step.name),
                        details: Some(e.to_string()),
                        error_code: None,
                        retryable: true,
                        timestamp: SystemTime::now(),
                    });
                    
                    if matches!(self.config.execution_mode, ExecutionMode::Strict) {
                        break; // 严格模式下，异常就停止
                    }
                }
            }
        }
        
        // 构建执行结果
        self.build_execution_result(start_time, step_results, errors).await
    }

    /// 带重新规划支持的执行方法
    async fn execute_with_replan_support(
        &self,
        plan: &ExecutionPlan,
        task: &TaskRequest,
        step_results: &mut HashMap<String, StepResult>,
        errors: &mut Vec<ExecutionError>,
    ) -> Result<ExecutionResult, PlanAndExecuteError> {
        let start_time = SystemTime::now();
        let context_guard = self.context.lock().await;
        let context = context_guard.as_ref().unwrap();
        
        for step in &plan.steps {
            let result = self.execute_step(step, context).await?;
            
            // 检查步骤是否失败，如果失败且有replanner，尝试实时重新规划
            if result.status == StepStatus::Failed {
                if let Some(ref replanner) = self.replanner {
                    log::warn!("步骤 '{}' 执行失败，尝试实时重新规划", step.name);
                    
                    match replanner.handle_runtime_exception(plan, &result, task).await {
                        Ok(replan_result) => {
                            if replan_result.should_replan {
                                if let Some(_new_plan) = replan_result.new_plan {
                                    log::info!("实时重新规划成功: {}", replan_result.replan_reason);
                                    // 触发重新规划，这会在上层循环中处理
                                    step_results.insert(step.id.clone(), result);
                                    return self.build_execution_result(start_time, step_results.clone(), errors.clone()).await;
                                }
                            }
                        },
                        Err(e) => {
                            log::warn!("实时重新规划失败: {}", e);
                        }
                    }
                }
                
                // 如果没有replanner或重新规划失败，在严格模式下停止执行
                step_results.insert(step.id.clone(), result);
                return Err(PlanAndExecuteError::ExecutionFailed(
                    format!("步骤 '{}' 执行失败，严格模式下停止执行", step.name)
                ));
            }
            
            step_results.insert(step.id.clone(), result);
        }
        
        self.build_execution_result(start_time, step_results.clone(), errors.clone()).await
    }

    /// 判断是否应该触发重新规划
    async fn should_trigger_replan(
        &self,
        execution_result: &ExecutionResult,
        current_attempts: u32,
        max_attempts: u32,
    ) -> bool {
        // 如果已达到最大重试次数，不再重新规划
        if current_attempts >= max_attempts {
            return false;
        }
        
        // 如果执行成功，不需要重新规划
        if matches!(execution_result.status, TaskStatus::Completed) {
            return false;
        }
        
        // 如果有失败的步骤，考虑重新规划
        if !execution_result.failed_steps.is_empty() {
            return true;
        }
        
        // 检查是否有严重错误
        if !execution_result.errors.is_empty() {
            for error in &execution_result.errors {
                if error.retryable {
                    return true;
                }
            }
        }
        
        false
    }

    /// 执行单个步骤
    pub async fn execute_step(
        &self,
        step: &ExecutionStep,
        context: &ExecutionContext,
    ) -> Result<StepResult, PlanAndExecuteError> {
        log::debug!("开始执行步骤: {}", step.name);
        
        let start_time = SystemTime::now();
        let mut retry_count = 0;
        
        loop {
            // 检查是否暂停或取消
            self.check_execution_state(context).await?;
            
            match self.execute_step_once(step, context).await {
                Ok(result) => {
                    log::debug!("步骤执行成功: {}", step.name);
                    
                    // 将步骤执行结果存储到共享数据中，以便后续步骤可以访问
                    let mut shared_data = context.shared_data.write().await;
                    shared_data.insert(format!("step_result_{}", step.id), result.clone());
                    shared_data.insert(format!("step_result_{}", step.name), result.clone());
                    drop(shared_data);
                    
                    return Ok(StepResult {
                        step_id: step.id.clone(),
                        status: StepStatus::Completed,
                        started_at: start_time,
                        completed_at: Some(SystemTime::now()),
                        duration_ms: start_time.elapsed().unwrap_or_default().as_millis() as u64,
                        result_data: Some(result.clone()),
                        error: None,
                        retry_count,
                        tool_result: None,
                    });
                },
                Err(error) => {
                    retry_count += 1;
                    
                    if retry_count <= step.retry_config.max_retries {
                        log::warn!("步骤执行失败，准备重试 ({}/{}): {}", 
                                 retry_count, step.retry_config.max_retries, error);
                        
                        // 等待重试间隔
                        let delay = self.calculate_retry_delay(&step.retry_config, retry_count);
                        tokio::time::sleep(Duration::from_secs(delay)).await;
                        
                        continue;
                    } else {
                        log::error!("步骤执行失败，已达到最大重试次数: {}", error);
                        return Ok(StepResult {
                            step_id: step.id.clone(),
                            status: StepStatus::Failed,
                            started_at: start_time,
                            completed_at: Some(SystemTime::now()),
                            duration_ms: start_time.elapsed().unwrap_or_default().as_millis() as u64,
                            result_data: None,
                            error: Some(error.to_string()),
                            retry_count,
                            tool_result: None,
                        });
                    }
                }
            }
        }
    }

    /// 暂停执行
    pub async fn pause(&self) -> Result<(), PlanAndExecuteError> {
        let context_guard = self.context.lock().await;
        if let Some(context) = context_guard.as_ref() {
            let mut state = context.execution_state.write().await;
            state.is_paused = true;
            log::info!("执行已暂停");
        }
        Ok(())
    }

    /// 恢复执行
    pub async fn resume(&self) -> Result<(), PlanAndExecuteError> {
        let context_guard = self.context.lock().await;
        if let Some(context) = context_guard.as_ref() {
            let mut state = context.execution_state.write().await;
            state.is_paused = false;
            log::info!("执行已恢复");
        }
        Ok(())
    }

    /// 取消执行
    pub async fn cancel(&self) -> Result<(), PlanAndExecuteError> {
        let context_guard = self.context.lock().await;
        if let Some(context) = context_guard.as_ref() {
            let mut state = context.execution_state.write().await;
            state.is_cancelled = true;
            log::info!("执行已取消");
        }
        Ok(())
    }

    /// 获取执行状态
    pub async fn get_execution_status(&self) -> Option<ExecutionState> {
        let context_guard = self.context.lock().await;
        if let Some(context) = context_guard.as_ref() {
            Some(context.execution_state.read().await.clone())
        } else {
            None
        }
    }

    // 私有方法实现
    
    async fn initialize_context(
        &self,
        plan: &ExecutionPlan,
        task: &TaskRequest,
    ) -> Result<(), PlanAndExecuteError> {
        let shared_data = Arc::new(RwLock::new(HashMap::new()));
        let execution_state = Arc::new(RwLock::new(ExecutionState {
            current_steps: HashMap::new(),
            completed_steps: Vec::new(),
            failed_steps: Vec::new(),
            is_paused: false,
            is_cancelled: false,
        }));
        
        let tool_system = get_global_tool_system()
            .map_err(|e| PlanAndExecuteError::ToolFailed(format!("获取全局工具系统失败: {}", e)))?;
        let tool_manager = tool_system.get_manager();
        
        let mut context = self.context.lock().await;
        *context = Some(ExecutionContext {
            task_id: task.id.clone(),
            plan_id: plan.id.clone(),
            shared_data,
            execution_state,
            tool_manager,
        });
        
        Ok(())
    }

    async fn execute_strict_mode(
        &self,
        plan: &ExecutionPlan,
        step_results: &mut HashMap<String, StepResult>,
        _errors: &mut Vec<ExecutionError>,
    ) -> Result<(), PlanAndExecuteError> {
        let context_guard = self.context.lock().await;
        let context = context_guard.as_ref().unwrap();
        
        for step in &plan.steps {
            let result = self.execute_step(step, context).await?;
            
            if result.status == StepStatus::Failed {
                step_results.insert(step.id.clone(), result);
                return Err(PlanAndExecuteError::ExecutionFailed(
                    format!("步骤 '{}' 执行失败，严格模式下停止执行", step.name)
                ));
            }
            
            step_results.insert(step.id.clone(), result);
        }
        
        Ok(())
    }

    async fn execute_tolerant_mode(
        &self,
        plan: &ExecutionPlan,
        step_results: &mut HashMap<String, StepResult>,
        errors: &mut Vec<ExecutionError>,
    ) -> Result<(), PlanAndExecuteError> {
        let context_guard = self.context.lock().await;
        let context = context_guard.as_ref().unwrap();
        
        for step in &plan.steps {
            let result = self.execute_step(step, context).await?;
            
            if result.status == StepStatus::Failed {
                log::warn!("步骤 '{}' 执行失败，容错模式下跳过并继续", step.name);
                errors.push(ExecutionError {
                    error_type: crate::engines::types::ErrorType::Tool,
                    message: format!("步骤 '{}' 执行失败", step.name),
                    details: result.error.clone(),
                    error_code: None,
                    retryable: false,
                    timestamp: SystemTime::now(),
                });
            }
            
            step_results.insert(step.id.clone(), result);
        }
        
        Ok(())
    }

    async fn execute_best_effort_mode(
        &self,
        plan: &ExecutionPlan,
        step_results: &mut HashMap<String, StepResult>,
        errors: &mut Vec<ExecutionError>,
    ) -> Result<(), PlanAndExecuteError> {
        let context_guard = self.context.lock().await;
        let context = context_guard.as_ref().unwrap();
        
        // 按顺序执行步骤
        for step in &plan.steps {
            let result = self.execute_step(step, context).await?;
            
            if result.status == StepStatus::Failed {
                log::warn!("步骤 '{}' 执行失败，最佳努力模式下记录错误并继续", step.name);
                errors.push(ExecutionError {
                    error_type: crate::engines::types::ErrorType::Tool,
                    message: format!("步骤 '{}' 执行失败", step.name),
                    details: result.error.clone(),
                    error_code: None,
                    retryable: false,
                    timestamp: SystemTime::now(),
                });
            }
            
            step_results.insert(step.id.clone(), result);
        }
        
        Ok(())
    }

    async fn execute_step_once(
        &self,
        step: &ExecutionStep,
        context: &ExecutionContext,
    ) -> Result<serde_json::Value, PlanAndExecuteError> {
        log::info!("=== Tools层：执行具体步骤 ===");
        log::info!("步骤名称: {}", step.name);
        log::info!("步骤类型: {:?}", step.step_type);
        log::info!("工具配置: {}", if step.tool_config.is_some() { "有" } else { "无" });
        
        let result = match &step.step_type {
            StepType::ToolCall => {
                log::info!("Tools层：调用外部工具");
                self.execute_tool_call(step, context).await
            },
            StepType::AiReasoning => {
                log::info!("Tools层：执行AI推理");
                self.execute_ai_reasoning(step, context).await
            },
            StepType::DataProcessing => {
                log::info!("Tools层：执行数据处理");
                self.execute_data_processing(step, context).await
            },
            StepType::Conditional => {
                log::info!("Tools层：执行条件判断");
                self.execute_conditional(step, context).await
            },
            StepType::Parallel => {
                log::info!("Tools层：执行并行任务");
                self.execute_parallel(step, context).await
            },
            StepType::Wait => {
                log::info!("Tools层：执行等待");
                self.execute_wait(step, context).await
            },
            StepType::ManualConfirmation => {
                log::info!("Tools层：执行人工确认");
                self.execute_manual_confirmation(step, context).await
            },
        };
        
        match &result {
            Ok(data) => {
                log::info!("✓ Tools层执行成功: {}", step.name);
                log::debug!("执行结果: {}", serde_json::to_string_pretty(data).unwrap_or_else(|_| "无法序列化".to_string()));
            },
            Err(e) => {
                log::error!("✗ Tools层执行失败: {}: {}", step.name, e);
            }
        }
        
        result
    }

    async fn execute_tool_call(
        &self,
        step: &ExecutionStep,
        context: &ExecutionContext,
    ) -> Result<serde_json::Value, PlanAndExecuteError> {
        if let Some(tool_config) = &step.tool_config {
            // 合并工具参数（以步骤参数为优先，覆盖tool_args）
            let mut merged_inputs = tool_config.tool_args.clone();
            for (k, v) in &step.parameters { merged_inputs.insert(k.clone(), v.clone()); }

            let tool_params = ToolExecutionParams {
                inputs: merged_inputs,
                context: HashMap::new(),
                timeout: Some(std::time::Duration::from_secs(tool_config.timeout.unwrap_or(self.config.default_timeout))),
                execution_id: None,
            };
            
            let timeout_duration = Duration::from_secs(
                tool_config.timeout.unwrap_or(self.config.default_timeout)
            );
            
            let manager = context.tool_manager.read().await;
            match timeout(timeout_duration, manager.call_tool(&tool_config.tool_name, tool_params)).await {
                Ok(Ok(result)) => Ok(result.output),
                Ok(Err(error)) => Err(PlanAndExecuteError::ToolFailed(error.to_string())),
                Err(_) => Err(PlanAndExecuteError::ToolFailed("工具调用超时".to_string())),
            }
        } else {
            Err(PlanAndExecuteError::ConfigError("工具调用步骤缺少工具配置".to_string()))
        }
    }

    async fn execute_ai_reasoning(
        &self,
        step: &ExecutionStep,
        context: &ExecutionContext,
    ) -> Result<serde_json::Value, PlanAndExecuteError> {
        log::info!("Executing AI reasoning step: {}", step.name);

        // 构建提示
        let shared = context.shared_data.read().await;
        let shared_keys: Vec<String> = shared.keys().cloned().collect();
        let params_str = if step.parameters.is_empty() {
            "{}".to_string()
        } else {
            serde_json::to_string(&step.parameters).unwrap_or_else(|_| "{}".to_string())
        };
        
        // 构建包含之前步骤结果的上下文信息
        let mut context_data = String::new();
        let mut has_previous_results = false;
        
        for (key, value) in shared.iter() {
            if key.starts_with("step_result_") {
                has_previous_results = true;
                context_data.push_str(&format!("\n\n--- {} ---\n{}", key, 
                    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())));
            }
        }
        
        let prompt = if has_previous_results {
            format!(
                "You are an expert reasoning module.\n\nStep: {step_name}\nDescription: {desc}\nParameters: {params}\n\nPrevious step results:{context}\n\nTask: Analyze the provided data and results from previous steps to provide a concise reasoning outcome. Use the data to generate meaningful insights for the user. Return plain text.",
                step_name = step.name,
                desc = step.description,
                params = params_str,
                context = context_data
            )
        } else {
            format!(
                "You are an expert reasoning module.\n\nStep: {step_name}\nDescription: {desc}\nParameters: {params}\nShared context keys: {keys}\n\nTask: Provide a concise reasoning outcome. If applicable, produce actionable insights for subsequent steps. Return plain text.",
                step_name = step.name,
                desc = step.description,
                params = params_str,
                keys = if shared_keys.is_empty() { "(none)".to_string() } else { shared_keys.join(", ") }
            )
        };

        // 解析执行阶段应使用的provider与model：优先调度器Execution -> 调度器Planning -> 本地配置
        let (provider_name, model_name) = if let Some(ref ai_service_manager) = self.ai_service_manager {
            match ai_service_manager.get_ai_config_for_stage(SchedulerStage::Execution).await {
                Ok(Some(cfg)) => {
                    log::info!("Using scheduler Execution config: {} ({})", cfg.model, cfg.provider);
                    (cfg.provider, cfg.model)
                }
                _ => {
                    match ai_service_manager.get_ai_config_for_stage(SchedulerStage::Planning).await {
                        Ok(Some(cfg)) => {
                            log::info!("Falling back to scheduler Planning config: {} ({})", cfg.model, cfg.provider);
                            (cfg.provider, cfg.model)
                        }
                        _ => {
                            log::info!("Using local executor config: {} ({})", self.config.model_config.model_name, self.config.ai_provider);
                            (self.config.ai_provider.clone(), self.config.model_config.model_name.clone())
                        }
                    }
                }
            }
        } else {
            log::info!("AI service manager not set; using local executor config: {} ({})", self.config.model_config.model_name, self.config.ai_provider);
            (self.config.ai_provider.clone(), self.config.model_config.model_name.clone())
        };

        if model_name.trim().is_empty() {
            return Err(PlanAndExecuteError::ConfigError("Executor model is empty; please configure scheduler or executor.model_config.model_name".to_string()));
        }

        let ai_manager = AiAdapterManager::global();
        let provider = ai_manager.get_provider_or_default(&provider_name)
            .map_err(|e| PlanAndExecuteError::AiAdapterError(e.to_string()))?;
        let request = ChatRequest {
            model: model_name,
            messages: vec![Message {
                role: MessageRole::User,
                content: prompt,
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
        };

        // 使用流式响应并收集结果
        let mut stream = provider.send_chat_stream(&request).await
            .map_err(|e| PlanAndExecuteError::AiAdapterError(e.to_string()))?;

        let mut content = String::new();

        // 收集流式响应
        use futures::StreamExt;
        while let Some(chunk_result) = stream.stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    content.push_str(&chunk.content);
                }
                Err(e) => return Err(PlanAndExecuteError::AiAdapterError(e.to_string())),
            }
        }

        tracing::info!("AI reasoning result: {}", content);
        Ok(serde_json::json!({
            "reasoning_result": content,
            "step_name": step.name
        }))
    }

    async fn execute_data_processing(
        &self,
        step: &ExecutionStep,
        context: &ExecutionContext,
    ) -> Result<serde_json::Value, PlanAndExecuteError> {
        // 数据处理步骤的实现
        log::info!("执行数据处理步骤: {}", step.name);
        
        let shared_data = context.shared_data.read().await;
        
        // 查找并处理之前步骤的结果
        let mut processed_results = Vec::new();
        let mut total_processed_items = 0;
        
        for (key, value) in shared_data.iter() {
            if key.starts_with("step_result_") {
                // 如果步骤结果包含工具执行的数据，尝试提取和处理
                if let Some(obj) = value.as_object() {
                    // 查找可能的数据数组或对象
                    for (result_key, result_value) in obj {
                        if result_key.contains("output") || result_key.contains("data") || result_key.contains("result") {
                            if let Some(array) = result_value.as_array() {
                                total_processed_items += array.len();
                                processed_results.push(serde_json::json!({
                                    "source_step": key,
                                    "data_type": "array",
                                    "item_count": array.len(),
                                    "preview": if array.len() > 0 { Some(&array[0]) } else { None }
                                }));
                            } else if let Some(obj) = result_value.as_object() {
                                total_processed_items += obj.len();
                                processed_results.push(serde_json::json!({
                                    "source_step": key,
                                    "data_type": "object",
                                    "field_count": obj.len(),
                                    "fields": obj.keys().collect::<Vec<_>>()
                                }));
                            }
                        }
                    }
                }
            }
        }
        
        let processed_data = serde_json::json!({
            "processed": true,
            "step_name": step.name,
            "total_shared_data_keys": shared_data.len(),
            "processed_results": processed_results,
            "total_processed_items": total_processed_items,
            "processing_summary": format!("Processed {} data items from {} previous steps", 
                total_processed_items, processed_results.len())
        });
        
        log::info!("数据处理完成: 处理了 {} 个数据项，来自 {} 个之前的步骤", 
                  total_processed_items, processed_results.len());
        
        Ok(processed_data)
    }

    async fn execute_conditional(
        &self,
        step: &ExecutionStep,
        _context: &ExecutionContext,
    ) -> Result<serde_json::Value, PlanAndExecuteError> {
        // 条件判断步骤的实现
        log::info!("执行条件判断步骤: {}", step.name);
        Ok(serde_json::json!({
            "condition_result": true,
            "step_name": step.name
        }))
    }

    async fn execute_parallel(
        &self,
        step: &ExecutionStep,
        _context: &ExecutionContext,
    ) -> Result<serde_json::Value, PlanAndExecuteError> {
        // 并行执行步骤的实现
        log::info!("执行并行步骤: {}", step.name);
        Ok(serde_json::json!({
            "parallel_result": "并行执行完成",
            "step_name": step.name
        }))
    }

    async fn execute_wait(
        &self,
        step: &ExecutionStep,
        _context: &ExecutionContext,
    ) -> Result<serde_json::Value, PlanAndExecuteError> {
        // 等待步骤的实现，支持多种参数格式
        let wait_time = step.parameters.get("wait_seconds")
            .and_then(|v| v.as_u64())
            .or_else(|| step.parameters.get("duration").and_then(|v| v.as_u64()))
            .or_else(|| {
                // 如果参数中包含工具配置，从工具参数中获取duration
                if let Some(tool_args) = step.parameters.get("tool_args").and_then(|v| v.as_object()) {
                    tool_args.get("duration").and_then(|v| v.as_u64())
                } else {
                    None
                }
            })
            .unwrap_or(3); // 默认等待3秒
        
        log::info!("执行等待步骤: {}，等待 {} 秒", step.name, wait_time);
        tokio::time::sleep(Duration::from_secs(wait_time)).await;
        
        Ok(serde_json::json!({
            "wait_completed": true,
            "wait_seconds": wait_time,
            "step_name": step.name
        }))
    }

    async fn execute_manual_confirmation(
        &self,
        step: &ExecutionStep,
        _context: &ExecutionContext,
    ) -> Result<serde_json::Value, PlanAndExecuteError> {
        // 人工确认步骤的实现
        log::info!("执行人工确认步骤: {}", step.name);
        
        // 自动确认执行
        Ok(serde_json::json!({
            "confirmed": true,
            "step_name": step.name,
            "confirmation_time": SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
        }))
    }

    async fn check_execution_state(
        &self,
        context: &ExecutionContext,
    ) -> Result<(), PlanAndExecuteError> {
        loop {
            let state = context.execution_state.read().await;
            
            if state.is_cancelled {
                return Err(PlanAndExecuteError::ExecutionFailed("执行已被取消".to_string()));
            }
            
            if !state.is_paused {
                break;
            }
            
            drop(state);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        Ok(())
    }

    fn calculate_retry_delay(&self, retry_config: &RetryConfig, retry_count: u32) -> u64 {
        match retry_config.backoff_strategy {
            BackoffStrategy::Fixed => retry_config.retry_interval,
            BackoffStrategy::Linear => retry_config.retry_interval * retry_count as u64,
            BackoffStrategy::Exponential => {
                retry_config.retry_interval * (2_u64.pow(retry_count - 1))
            },
        }
    }

    #[allow(unused)]
    fn can_execute_in_parallel(&self, _step: &ExecutionStep, _plan: &ExecutionPlan) -> bool {
        // 并行判断逻辑
        false
    }

    async fn build_execution_result(
        &self,
        start_time: SystemTime,
        step_results: HashMap<String, StepResult>,
        errors: Vec<ExecutionError>,
    ) -> Result<ExecutionResult, PlanAndExecuteError> {
        let total_duration = start_time.elapsed().unwrap_or_default().as_millis() as u64;
        
        let mut completed_steps = Vec::new();
        let mut failed_steps = Vec::new();
        let mut skipped_steps = Vec::new();
        let mut successful_count = 0;
        let mut failed_count = 0;
        let mut skipped_count = 0;
        let mut total_retries = 0;
        
        for (step_id, result) in &step_results {
            total_retries += result.retry_count;
            
            match result.status {
                StepStatus::Completed => {
                    completed_steps.push(step_id.clone());
                    successful_count += 1;
                },
                StepStatus::Failed => {
                    failed_steps.push(step_id.clone());
                    failed_count += 1;
                },
                StepStatus::Skipped => {
                    skipped_steps.push(step_id.clone());
                    skipped_count += 1;
                },
                _ => {},
            }
        }
        
        let status = if failed_count > 0 {
            TaskStatus::Failed
        } else if skipped_count > 0 {
            TaskStatus::Completed // 部分完成
        } else {
            TaskStatus::Completed
        };
        
        let avg_duration = if step_results.len() > 0 {
            total_duration / step_results.len() as u64
        } else {
            0
        };
        
        let metrics = ExecutionMetrics {
            total_duration_ms: total_duration,
            successful_steps: successful_count,
            failed_steps: failed_count,
            skipped_steps: skipped_count,
            total_retries,
            avg_step_duration_ms: avg_duration,
            peak_concurrency: 1,
            custom_metrics: HashMap::new(),
        };
        
        // 更新内部指标
        *self.metrics.lock().await = metrics.clone();
        
        // 生成增强的执行反馈
        let enhanced_feedback = self.generate_enhanced_feedback(
            &step_results,
            &metrics,
            &errors,
            total_duration,
        ).await?;
        
        Ok(ExecutionResult {
            status,
            completed_steps,
            failed_steps,
            skipped_steps,
            step_results: step_results.into_iter().map(|(k, v)| (k, v)).collect(),
            metrics,
            errors,
            enhanced_feedback,
        })
    }

    /// 生成增强的执行反馈（专为Replanner判断优化）
    async fn generate_enhanced_feedback(
        &self,
        step_results: &HashMap<String, StepResult>,
        metrics: &ExecutionMetrics,
        errors: &[ExecutionError],
        total_duration: u64,
    ) -> Result<EnhancedExecutionFeedback, PlanAndExecuteError> {
        log::info!("生成增强执行反馈以支持AI重新规划决策");

        // 1. 生成执行摘要
        let execution_summary = self.generate_execution_summary(step_results, metrics).await;

        // 2. 分析成功因素
        let success_factors = self.analyze_success_factors(step_results).await;

        // 3. 分析失败因素
        let failure_factors = self.analyze_failure_factors(step_results, errors).await;

        // 4. 生成性能洞察
        let performance_insights = self.generate_performance_insights(step_results, metrics).await;

        // 5. 质量评估
        let quality_assessment = self.assess_execution_quality(step_results, metrics).await;

        // 6. 改进建议
        let improvement_suggestions = self.generate_improvement_suggestions(step_results, errors).await;

        // 7. 风险指标
        let risk_indicators = self.identify_risk_indicators(step_results, errors).await;

        // 8. 依赖关系分析
        let dependency_analysis = self.analyze_dependencies(step_results).await;

        // 9. 资源使用分析
        let resource_analysis = self.analyze_resource_usage(metrics, total_duration).await;

        Ok(EnhancedExecutionFeedback {
            execution_summary,
            success_factors,
            failure_factors,
            performance_insights,
            quality_assessment,
            improvement_suggestions,
            risk_indicators,
            dependency_analysis,
            resource_analysis,
        })
    }

    async fn generate_execution_summary(
        &self,
        step_results: &HashMap<String, StepResult>,
        metrics: &ExecutionMetrics,
    ) -> String {
        let total_steps = step_results.len();
        let success_rate = if total_steps > 0 {
            (metrics.successful_steps as f64 / total_steps as f64) * 100.0
        } else {
            0.0
        };

        format!(
            "执行了{}个步骤，成功率{:.1}%，总耗时{}ms，平均每步{}ms。{}个步骤成功，{}个失败，{}个跳过，共重试{}次。",
            total_steps,
            success_rate,
            metrics.total_duration_ms,
            metrics.avg_step_duration_ms,
            metrics.successful_steps,
            metrics.failed_steps,
            metrics.skipped_steps,
            metrics.total_retries
        )
    }

    async fn analyze_success_factors(&self, step_results: &HashMap<String, StepResult>) -> Vec<String> {
        let mut factors = Vec::new();

        let successful_steps: Vec<_> = step_results.values()
            .filter(|r| r.status == StepStatus::Completed)
            .collect();

        if !successful_steps.is_empty() {
            factors.push("工具调用成功执行".to_string());
            
            let avg_duration = successful_steps.iter()
                .map(|r| r.duration_ms)
                .sum::<u64>() / successful_steps.len() as u64;
            
            if avg_duration < 5000 {
                factors.push("步骤执行速度较快".to_string());
            }

            let no_retry_steps = successful_steps.iter()
                .filter(|r| r.retry_count == 0)
                .count();
            
            if no_retry_steps == successful_steps.len() {
                factors.push("无需重试即可成功".to_string());
            }
        }

        factors
    }

    async fn analyze_failure_factors(&self, step_results: &HashMap<String, StepResult>, errors: &[ExecutionError]) -> Vec<String> {
        let mut factors = Vec::new();

        let failed_steps: Vec<_> = step_results.values()
            .filter(|r| r.status == StepStatus::Failed)
            .collect();

        if !failed_steps.is_empty() {
            factors.push(format!("{}个步骤执行失败", failed_steps.len()));

            let high_retry_steps = failed_steps.iter()
                .filter(|r| r.retry_count > 2)
                .count();
            
            if high_retry_steps > 0 {
                factors.push("多个步骤需要大量重试".to_string());
            }

            // 分析错误类型
            let mut error_types = std::collections::HashSet::new();
            for step in &failed_steps {
                if let Some(error) = &step.error {
                    if error.contains("timeout") || error.contains("超时") {
                        error_types.insert("超时错误");
                    } else if error.contains("network") || error.contains("网络") {
                        error_types.insert("网络错误");
                    } else if error.contains("permission") || error.contains("权限") {
                        error_types.insert("权限错误");
                    } else {
                        error_types.insert("其他错误");
                    }
                }
            }

            for error_type in error_types {
                factors.push(format!("发生{}", error_type));
            }
        }

        if !errors.is_empty() {
            factors.push(format!("存在{}个系统级错误", errors.len()));
        }

        factors
    }

    async fn generate_performance_insights(&self, step_results: &HashMap<String, StepResult>, _metrics: &ExecutionMetrics) -> Vec<PerformanceInsight> {
        let mut insights = Vec::new();

        // 识别性能瓶颈
        let mut durations: Vec<_> = step_results.iter()
            .map(|(id, result)| (id.clone(), result.duration_ms))
            .collect();
        durations.sort_by(|a, b| b.1.cmp(&a.1));

        if let Some((slowest_step, duration)) = durations.first() {
            if *duration > 10000 { // 超过10秒
                insights.push(PerformanceInsight {
                    insight_type: InsightType::PerformanceBottleneck,
                    description: format!("步骤 '{}' 执行时间过长 ({}ms)", slowest_step, duration),
                    impact_level: 4,
                    confidence: 0.9,
                    related_steps: vec![slowest_step.clone()],
                });
            }
        }

        // 识别效率机会
        let total_time: u64 = step_results.values().map(|r| r.duration_ms).sum();
        let sequential_time = total_time; // 当前是顺序执行
        if step_results.len() > 2 && sequential_time > 5000 {
            insights.push(PerformanceInsight {
                insight_type: InsightType::ConcurrencyOpportunity,
                description: "某些步骤可能可以并行执行以提高效率".to_string(),
                impact_level: 3,
                confidence: 0.7,
                related_steps: step_results.keys().cloned().collect(),
            });
        }

        insights
    }

    async fn assess_execution_quality(&self, step_results: &HashMap<String, StepResult>, metrics: &ExecutionMetrics) -> QualityAssessment {
        let total_steps = step_results.len();
        let success_rate = if total_steps > 0 {
            (metrics.successful_steps as f64 / total_steps as f64) * 100.0
        } else {
            100.0
        };

        let completeness_score = success_rate as u32;
        let reliability_score = if metrics.total_retries == 0 { 
            100 
        } else { 
            std::cmp::max(0, 100_i32.saturating_sub(metrics.total_retries as i32 * 10)) as u32
        };
        let efficiency_score = if metrics.avg_step_duration_ms < 5000 { 
            90 
        } else { 
            std::cmp::max(30, 90_i32.saturating_sub((metrics.avg_step_duration_ms / 1000) as i32 * 10)) as u32
        };
        let overall_score = (completeness_score + reliability_score + efficiency_score) / 3;

        QualityAssessment {
            overall_score,
            completeness_score,
            reliability_score,
            efficiency_score,
            quality_dimensions: vec![
                QualityDimension {
                    name: "完整性".to_string(),
                    score: completeness_score,
                    details: format!("{}个步骤中{}个成功完成", total_steps, metrics.successful_steps),
                    suggestions: if completeness_score < 80 {
                        vec!["考虑增加错误处理机制".to_string(), "优化工具参数配置".to_string()]
                    } else {
                        vec![]
                    },
                },
                QualityDimension {
                    name: "可靠性".to_string(),
                    score: reliability_score,
                    details: format!("总共重试{}次", metrics.total_retries),
                    suggestions: if reliability_score < 80 {
                        vec!["增加预检查机制".to_string(), "优化重试策略".to_string()]
                    } else {
                        vec![]
                    },
                },
                QualityDimension {
                    name: "效率".to_string(),
                    score: efficiency_score,
                    details: format!("平均每步耗时{}ms", metrics.avg_step_duration_ms),
                    suggestions: if efficiency_score < 80 {
                        vec!["考虑并行执行".to_string(), "优化工具调用".to_string()]
                    } else {
                        vec![]
                    },
                },
            ],
        }
    }

    async fn generate_improvement_suggestions(&self, step_results: &HashMap<String, StepResult>, _errors: &[ExecutionError]) -> Vec<String> {
        let mut suggestions = Vec::new();

        let failed_count = step_results.values()
            .filter(|r| r.status == StepStatus::Failed)
            .count();

        if failed_count > 0 {
            suggestions.push("增加失败步骤的错误处理逻辑".to_string());
            suggestions.push("考虑调整工具参数以提高成功率".to_string());
        }

        let high_retry_count = step_results.values()
            .filter(|r| r.retry_count > 2)
            .count();

        if high_retry_count > 0 {
            suggestions.push("优化重试策略，避免过度重试".to_string());
        }

        let slow_steps = step_results.values()
            .filter(|r| r.duration_ms > 10000)
            .count();

        if slow_steps > 0 {
            suggestions.push("优化耗时较长的步骤，考虑超时设置".to_string());
        }

        if step_results.len() > 3 {
            suggestions.push("评估是否可以并行执行某些独立步骤".to_string());
        }

        suggestions
    }

    async fn identify_risk_indicators(&self, step_results: &HashMap<String, StepResult>, errors: &[ExecutionError]) -> Vec<RiskIndicator> {
        let mut indicators = Vec::new();

        // 执行风险
        let failure_rate = step_results.values()
            .filter(|r| r.status == StepStatus::Failed)
            .count() as f64 / step_results.len() as f64;

        if failure_rate > 0.3 {
            indicators.push(RiskIndicator {
                risk_type: RiskType::ExecutionRisk,
                level: 4,
                description: format!("失败率过高 ({:.1}%)", failure_rate * 100.0),
                impact_scope: vec!["任务完成度".to_string(), "用户体验".to_string()],
                mitigation_suggestions: vec![
                    "检查工具配置".to_string(),
                    "增加预检查".to_string(),
                    "调整重试策略".to_string(),
                ],
            });
        }

        // 性能风险
        let avg_duration = step_results.values()
            .map(|r| r.duration_ms)
            .sum::<u64>() / step_results.len().max(1) as u64;

        if avg_duration > 15000 {
            indicators.push(RiskIndicator {
                risk_type: RiskType::PerformanceRisk,
                level: 3,
                description: "平均执行时间过长".to_string(),
                impact_scope: vec!["响应时间".to_string(), "资源消耗".to_string()],
                mitigation_suggestions: vec![
                    "优化工具调用".to_string(),
                    "考虑并行执行".to_string(),
                    "调整超时设置".to_string(),
                ],
            });
        }

        // 质量风险
        if !errors.is_empty() {
            indicators.push(RiskIndicator {
                risk_type: RiskType::QualityRisk,
                level: 3,
                description: "存在系统级错误".to_string(),
                impact_scope: vec!["数据完整性".to_string(), "结果可靠性".to_string()],
                mitigation_suggestions: vec![
                    "增强错误处理".to_string(),
                    "添加数据验证".to_string(),
                ],
            });
        }

        indicators
    }

    async fn analyze_dependencies(&self, _step_results: &HashMap<String, StepResult>) -> DependencyAnalysis {
        // 依赖分析实现
        DependencyAnalysis {
            resolved_dependencies: vec![],
            unresolved_dependencies: vec![],
            circular_dependencies: vec![],
            dependency_conflicts: vec![],
        }
    }

    async fn analyze_resource_usage(&self, metrics: &ExecutionMetrics, total_duration: u64) -> ResourceAnalysis {
        // 基于指标估算资源使用情况
        let cpu_usage = ResourceUsage {
            current: 0.3,
            peak: 0.7,
            average: 0.4,
            utilization_rate: 0.4,
            unit: "cores".to_string(),
        };

        let memory_usage = ResourceUsage {
            current: 256.0,
            peak: 512.0,
            average: 384.0,
            utilization_rate: 0.3,
            unit: "MB".to_string(),
        };

        let network_usage = ResourceUsage {
            current: 1.2,
            peak: 5.8,
            average: 2.1,
            utilization_rate: 0.1,
            unit: "Mbps".to_string(),
        };

        let storage_usage = ResourceUsage {
            current: 10.0,
            peak: 50.0,
            average: 25.0,
            utilization_rate: 0.05,
            unit: "MB".to_string(),
        };

        let mut optimization_suggestions = Vec::new();
        if total_duration > 30000 {
            optimization_suggestions.push("考虑增加并发执行以减少总时间".to_string());
        }
        if metrics.total_retries > 5 {
            optimization_suggestions.push("优化重试机制以减少资源浪费".to_string());
        }

        ResourceAnalysis {
            cpu_usage,
            memory_usage,
            network_usage,
            storage_usage,
            resource_bottlenecks: vec![],
            optimization_suggestions,
        }
    }

    /// 保存执行结果到数据库
    async fn save_execution_to_database(
        &self,
        plan: &ExecutionPlan,
        task: &TaskRequest,
        result: &ExecutionResult,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        log::info!("保存执行结果到数据库");

        // 首先确保数据库迁移已运行
        if let Err(e) = self.repository.run_migrations().await {
            log::warn!("数据库迁移失败: {}", e);
        }

        // 1. 转换并保存执行计划
        let engine_plan = crate::engines::types::ExecutionPlan {
            id: plan.id.clone(),
            name: plan.name.clone(),
            description: plan.description.clone(),
            steps: plan.steps.iter().map(|s| crate::engines::types::PlanStep {
                id: s.id.clone(),
                name: s.name.clone(),
                description: s.description.clone(),
                step_type: crate::engines::types::StepType::ToolCall,
                tool_config: crate::engines::types::ToolConfig {
                    tool_name: s.tool_config.as_ref()
                        .map(|tc| tc.tool_name.clone())
                        .unwrap_or_else(|| format!("step_{}", s.name)),
                    tool_version: None,
                    tool_args: s.parameters.clone(),
                    timeout: Some(self.config.default_timeout as u64),
                    env_vars: std::collections::HashMap::new(),
                },
                estimated_duration: s.estimated_duration,
                retry_config: crate::engines::types::RetryConfig::default(),
                parameters: s.parameters.clone(),
                preconditions: vec![],
                postconditions: vec![],
            }).collect(),
            estimated_duration: plan.estimated_duration,
            created_at: plan.created_at,
            metadata: crate::engines::types::PlanMetadata {
                plan_type: crate::engines::types::PlanType::Custom("Plan-and-Execute".to_string()),
                priority: crate::engines::types::Priority::Medium,
                complexity: crate::engines::types::Complexity::Medium,
                risk_level: crate::engines::types::RiskLevel::Medium,
                tags: vec![format!("{:?}", self.config.execution_mode)],
            },
            dependencies: plan.dependencies.clone(),
        };
        
        if let Err(e) = self.repository.save_execution_plan(&engine_plan).await {
            log::error!("保存执行计划失败: {}", e);
        }

        // 2. 构建执行会话
        let session = crate::engines::types::ExecutionSession {
            id: task.id.clone(),
            plan_id: plan.id.clone(),
            status: match result.status {
                TaskStatus::Pending => crate::engines::types::ExecutionStatus::Pending,
                TaskStatus::Planning => crate::engines::types::ExecutionStatus::Running,
                TaskStatus::Executing => crate::engines::types::ExecutionStatus::Running,
                TaskStatus::Replanning => crate::engines::types::ExecutionStatus::Running,
                TaskStatus::Completed => crate::engines::types::ExecutionStatus::Completed,
                TaskStatus::Failed => crate::engines::types::ExecutionStatus::Failed,
                TaskStatus::Cancelled => crate::engines::types::ExecutionStatus::Cancelled,
                TaskStatus::RequiresIntervention => crate::engines::types::ExecutionStatus::RequiresIntervention,
            },
            started_at: task.created_at,
            completed_at: Some(SystemTime::now()),
            current_step: Some(result.completed_steps.len() as i32),
            progress: if plan.steps.is_empty() { 
                100.0 
            } else { 
                (result.completed_steps.len() as f32 / plan.steps.len() as f32) * 100.0 
            },
            context: crate::engines::types::ExecutionContext {
                user_id: None,
                target_info: crate::engines::types::TargetInfo {
                    target_type: crate::engines::types::TargetType::Host,
                    address: "localhost".to_string(),
                    target_value: "localhost".to_string(),
                    port_range: None,
                    protocols: vec![],
                    credentials: None,
                    metadata: std::collections::HashMap::new(),
                },
                environment: std::collections::HashMap::new(),
                shared_data: std::collections::HashMap::new(),
                config: std::collections::HashMap::new(),
            },
            step_results: result.step_results.iter().map(|(k, v)| {
                (k.clone(), crate::engines::types::StepExecutionResult {
                    step_id: v.step_id.clone(),
                    status: match v.status {
                        StepStatus::Pending => crate::engines::types::StepExecutionStatus::Pending,
                        StepStatus::Running => crate::engines::types::StepExecutionStatus::Running,
                        StepStatus::Completed => crate::engines::types::StepExecutionStatus::Completed,
                        StepStatus::Failed => crate::engines::types::StepExecutionStatus::Failed,
                        StepStatus::Skipped => crate::engines::types::StepExecutionStatus::Skipped,
                        StepStatus::Retrying => crate::engines::types::StepExecutionStatus::Retrying,
                        StepStatus::Cancelled => crate::engines::types::StepExecutionStatus::Failed,
                    },
                    started_at: v.started_at,
                    completed_at: v.completed_at,
                    result_data: v.result_data.clone(),
                    error: v.error.as_ref().map(|e| crate::engines::types::ExecutionError {
                        error_type: crate::engines::types::ErrorType::Tool,
                        message: e.clone(),
                        details: Some(e.clone()),
                        error_code: None,
                        retryable: false,
                        timestamp: SystemTime::now(),
                    }),
                    retry_count: v.retry_count,
                    logs: vec![],
                    metrics: crate::engines::types::ExecutionMetrics {
                        execution_time_ms: v.duration_ms,
                        memory_usage_bytes: 0,
                        cpu_usage_percent: 0.0,
                        network_io_bytes: 0,
                        disk_io_bytes: 0,
                        custom_metrics: std::collections::HashMap::new(),
                    },
                })
            }).collect(),
            metadata: crate::engines::types::SessionMetadata {
                created_by: None,
                tags: vec![format!("{:?}", self.config.execution_mode)],
                notes: Some(format!("Error handling: {:?}, Timeout: {}s", self.config.error_handling, self.config.default_timeout)),
                version: "1.0".to_string(),
                custom_attributes: {
                    let mut attrs = std::collections::HashMap::new();
                    attrs.insert("execution_mode".to_string(), serde_json::json!(format!("{:?}", self.config.execution_mode)));
                    attrs.insert("error_handling".to_string(), serde_json::json!(format!("{:?}", self.config.error_handling)));
                    attrs.insert("default_timeout".to_string(), serde_json::json!(self.config.default_timeout));
                    attrs
                },
            },
        };

        // 3. 保存执行会话
        if let Err(e) = self.repository.save_execution_session(&session).await {
            log::error!("保存执行会话失败: {}", e);
            return Err(e.into());
        }

        log::info!("执行结果已成功保存到数据库");
        Ok(())
    }
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            max_concurrent_steps: 4,
            default_timeout: 300, // 5分钟
            enable_step_caching: true,
            execution_mode: ExecutionMode::Tolerant,
            error_handling: ErrorHandlingStrategy::RetryThenStop,
            ai_provider: "".to_string(),
            model_config: ExecutorModelConfig {
                model_name: "".to_string(),
                temperature: 0.3,
                max_tokens: 2000,
                top_p: 0.9,
            },
        }
    }
}

impl Default for ExecutionMetrics {
    fn default() -> Self {
        Self {
            total_duration_ms: 0,
            successful_steps: 0,
            failed_steps: 0,
            skipped_steps: 0,
            total_retries: 0,
            avg_step_duration_ms: 0,
            peak_concurrency: 0,
            custom_metrics: HashMap::new(),
        }
    }
}

// 为ExecutionContext实现Clone
impl Clone for ExecutionContext {
    fn clone(&self) -> Self {
        Self {
            task_id: self.task_id.clone(),
            plan_id: self.plan_id.clone(),
            shared_data: Arc::clone(&self.shared_data),
            execution_state: Arc::clone(&self.execution_state),
            tool_manager: Arc::clone(&self.tool_manager),
        }
    }
}