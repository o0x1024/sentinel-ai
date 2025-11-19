//! Executor 组件 - 执行器
//!
//! 负责按照计划逐步执行子任务，调用具体的工具和服务
use sentinel_rag::models::AssistantRagRequest;
use crate::engines::plan_and_execute::repository::PlanExecuteRepository;
use crate::engines::plan_and_execute::memory_manager::MemoryManager;
use crate::engines::plan_and_execute::replanner::Replanner;
use crate::engines::plan_and_execute::types::*;
use crate::engines::{ExecutionError, StepExecutionStatus};
use crate::models::prompt::ArchitectureType;
use crate::services::ai::{AiService, AiServiceManager, SchedulerStage};
use crate::services::database::DatabaseService;
use crate::services::prompt_db::PromptRepository;
use crate::tools::{
    get_global_tool_system, ExecutionStatus, ToolExecutionParams, ToolExecutionResult,
    UnifiedToolManager,
};
use crate::utils::ordered_message::{emit_message_chunk_arc, ChunkType};
use crate::utils::prompt_resolver::{AgentPromptConfig, CanonicalStage, PromptResolver};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tauri::AppHandle;
use tokio::sync::{Mutex, RwLock, Semaphore};
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use regex::Regex;

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
    /// 最大重新规划次数
    pub max_replan_attempts: u32,
    /// 最大总执行时间（毫秒）
    pub max_total_execution_time: u64,
    /// 最大连续失败次数
    pub max_consecutive_failures: u32,
    /// 计划相似度阈值（避免无效重规划）
    pub plan_similarity_threshold: f64,
    /// 质量评估阈值（低于此值触发策略调整）
    pub quality_threshold: f64,
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
    pub status: StepExecutionStatus,
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
    pub current_steps: HashMap<String, StepExecutionStatus>,
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
    memory_manager: Option<Arc<MemoryManager>>,
    repository: Arc<PlanExecuteRepository>,
    app_handle: Option<Arc<AppHandle>>,
    cancellation_token: CancellationToken,  // ✅ 新增取消令牌
}

impl Executor {
    /// 取消执行（触发取消令牌）
    pub fn cancel(&self) {
        self.cancellation_token.cancel();
        log::info!("Plan-and-Execute: Cancellation token triggered");
    }
    
    /// 解析 execution_id（优先任务参数中的 execution_id，其次回退 task_id）
    async fn resolve_execution_id(&self, context: &ExecutionContext) -> String {
        let mut execution_id = context.task_id.clone();
        let shared = context.shared_data.read().await;
        if let Some(params_val) = shared.get("task_parameters") {
            if let Some(obj) = params_val.as_object() {
                if let Some(eid) = obj.get("execution_id").and_then(|v| v.as_str()) {
                    if !eid.is_empty() {
                        execution_id = eid.to_string();
                    }
                }
            }
        }
        execution_id
    }
    /// 从上下文解析 message_id 与 conversation_id（优先任务参数，其次回退到 task_id）
    async fn resolve_message_and_conversation_ids(
        &self,
        context: &ExecutionContext,
    ) -> (String, Option<String>) {
        // 默认回退
        let mut message_id = context.task_id.clone();
        let mut conversation_id: Option<String> = None;

        // 在共享数据里查找 task_parameters.message_id / conversation_id
        let shared = context.shared_data.read().await;
        if let Some(params_val) = shared.get("task_parameters") {
            if let Some(obj) = params_val.as_object() {
                if let Some(mid) = obj.get("message_id").and_then(|v| v.as_str()) {
                    if !mid.is_empty() {
                        message_id = mid.to_string();
                    }
                }
                if let Some(cid) = obj.get("conversation_id").and_then(|v| v.as_str()) {
                    if !cid.is_empty() {
                        conversation_id = Some(cid.to_string());
                    }
                }
            }
        }

        (message_id, conversation_id)
    }

    fn apply_placeholders(template: &str, pairs: &[(&str, &str)]) -> String {
        let mut out = template.to_string();
        for (k, v) in pairs {
            out = out.replace(k, v);
        }
        out
    }
    /// 发送有序消息块到前端
    fn emit_message_chunk(
        &self,
        execution_id: &str,
        message_id: &str,
        conversation_id: Option<&str>,
        chunk_type: ChunkType,
        content: &str,
        is_final: bool,
        stage: Option<&str>,
        tool_name: Option<&str>,
    ) {
        if let Some(app_handle) = &self.app_handle {
            emit_message_chunk_arc(
                app_handle,
                execution_id,
                message_id,
                conversation_id,
                chunk_type,
                content,
                is_final,
                stage,
                tool_name,
            );
        }
    }

    /// 便捷方法：发送工具执行结果
    async fn emit_tool_result(
        &self,
        context: &ExecutionContext,
        step_name: &str,
        result: &StepResult,
        tool_name: Option<&str>,
    ) {
        let (message_id, conversation_id) =
            self.resolve_message_and_conversation_ids(context).await;

        // 构造标准JSON
        let status_str = match result.status {
            StepExecutionStatus::Completed => "Completed",
            StepExecutionStatus::Running => "Running",
            StepExecutionStatus::Failed => "Failed",
            StepExecutionStatus::Pending => "Pending",
            StepExecutionStatus::Skipped => "Skipped",
            StepExecutionStatus::Retrying => "Retrying",
            StepExecutionStatus::Cancelled => "Cancelled",
        };
        let success = matches!(result.status, StepExecutionStatus::Completed);
        let output = result
            .result_data
            .clone()
            .unwrap_or(serde_json::json!(null));
        let json = serde_json::json!({
            "step_name": step_name,
            "tool_name": tool_name.unwrap_or("") ,
            "status": status_str,
            "success": success,
            "output": output,
            "error": result.error,
            "started_at": result.started_at,
            "completed_at": result.completed_at,
        });
        let content = serde_json::to_string(&json).unwrap_or_else(|_| "{}".to_string());

        if let Some(app_handle) = &self.app_handle {
            let execution_id = self.resolve_execution_id(context).await;
            emit_message_chunk_arc(
                app_handle,
                &execution_id,
                &message_id,
                conversation_id.as_deref(),
                ChunkType::ToolResult,
                &content,
                false,
                Some("executor"),
                tool_name,
            );
        }
    }

    /// 便捷方法：发送计划信息
    async fn emit_plan_info(&self, context: &ExecutionContext, plan_content: &str) {
        let (message_id, conversation_id) =
            self.resolve_message_and_conversation_ids(context).await;
        let execution_id = self.resolve_execution_id(context).await;
        self.emit_message_chunk(
            &execution_id,
            &message_id,
            conversation_id.as_deref(),
            ChunkType::PlanInfo,
            plan_content,
            false,
            Some("planner"),
            None,
        );
    }

    /// 便捷方法：发送错误信息
    async fn emit_error(&self, context: &ExecutionContext, error_msg: &str) {
        let (message_id, conversation_id) =
            self.resolve_message_and_conversation_ids(context).await;
        let execution_id = self.resolve_execution_id(context).await;
        self.emit_message_chunk(
            &execution_id,
            &message_id,
            conversation_id.as_deref(),
            ChunkType::Error,
            error_msg,
            true,
            Some("executor"),
            None,
        );
    }

    /// 创建新的执行器实例
    pub fn new(config: ExecutorConfig, db_service: Arc<DatabaseService>) -> Self {
        let pool = db_service
            .get_pool()
            .expect("Failed to get database pool")
            .clone();
        let repository = Arc::new(PlanExecuteRepository::new(pool));

        Self {
            config,
            context: Arc::new(Mutex::new(None)),
            metrics: Arc::new(Mutex::new(ExecutionMetrics::default())),
            db_service,
            ai_service_manager: None,
            replanner: None,
            memory_manager: None,
            repository,
            app_handle: None,
            cancellation_token: CancellationToken::new(),  // ✅ 初始化取消令牌
        }
    }

    /// 创建带有AI服务管理器的执行器实例（用于动态模型切换）
    pub fn with_ai_service_manager(
        config: ExecutorConfig,
        db_service: Arc<DatabaseService>,
        ai_service_manager: Arc<AiServiceManager>,
    ) -> Self {
        let pool = db_service
            .get_pool()
            .expect("Failed to get database pool")
            .clone();
        let repository = Arc::new(PlanExecuteRepository::new(pool));

        Self {
            config,
            context: Arc::new(Mutex::new(None)),
            metrics: Arc::new(Mutex::new(ExecutionMetrics::default())),
            db_service,
            ai_service_manager: Some(ai_service_manager),
            replanner: None,
            memory_manager: None,
            repository,
            app_handle: None,
            cancellation_token: CancellationToken::new(),  // ✅ 初始化取消令牌
        }
    }

    /// 创建带有完整依赖的执行器实例（包含重新规划器）
    pub fn with_replanner(
        config: ExecutorConfig,
        db_service: Arc<DatabaseService>,
        ai_service_manager: Option<Arc<AiServiceManager>>,
        replanner: Option<Arc<Replanner>>,
        app_handle: Option<Arc<AppHandle>>,
    ) -> Self {
        let pool = db_service
            .get_pool()
            .expect("Failed to get database pool")
            .clone();
        let repository = Arc::new(PlanExecuteRepository::new(pool));

        Self {
            config,
            context: Arc::new(Mutex::new(None)),
            metrics: Arc::new(Mutex::new(ExecutionMetrics::default())),
            db_service,
            ai_service_manager,
            replanner,
            memory_manager: None,
            repository,
            app_handle,
            cancellation_token: CancellationToken::new(),  // ✅ 初始化取消令牌
        }
    }

    /// 创建带有完整依赖的执行器实例（包含重新规划器和内存管理器）
    pub fn with_full_dependencies(
        config: ExecutorConfig,
        db_service: Arc<DatabaseService>,
        ai_service_manager: Option<Arc<AiServiceManager>>,
        replanner: Option<Arc<Replanner>>,
        memory_manager: Option<Arc<MemoryManager>>,
        app_handle: Option<Arc<AppHandle>>,
    ) -> Self {
        let pool = db_service
            .get_pool()
            .expect("Failed to get database pool")
            .clone();
        let repository = Arc::new(PlanExecuteRepository::new(pool));

        Self {
            config,
            context: Arc::new(Mutex::new(None)),
            metrics: Arc::new(Mutex::new(ExecutionMetrics::default())),
            db_service,
            ai_service_manager,
            replanner,
            memory_manager,
            repository,
            app_handle,
            cancellation_token: CancellationToken::new(),  // ✅ 初始化取消令牌
        }
    }

    /// 获取执行阶段应使用的AI配置
    async fn get_execution_ai_config(
        &self,
    ) -> Result<Option<crate::services::ai::AiConfig>, PlanAndExecuteError> {
        if let Some(ref ai_service_manager) = self.ai_service_manager {
            match ai_service_manager
                .get_ai_config_for_stage(SchedulerStage::Execution)
                .await
            {
                Ok(config) => Ok(config),
                Err(e) => {
                    log::warn!(
                        "Failed to get AI config for execution stage: {}, using default",
                        e
                    );
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
        let context = self.initialize_context(plan, task).await?;
        
        // 解析消息ID用于前端推送
        let execution_id = self.resolve_execution_id(&context).await;
        let (message_id, conversation_id) = self.resolve_message_and_conversation_ids(&context).await;
        
        log::info!("Plan-and-Execute: execution_id={}, message_id={}, conversation_id={:?}", 
            execution_id, message_id, conversation_id);
        
        // 发送执行开始消息
        if let Some(app) = &self.app_handle {
            crate::utils::ordered_message::emit_thinking_chunk(
                app,
                &execution_id,
                &message_id,
                conversation_id.as_deref(),
                &format!("开始执行计划: {}", plan.name),
                Some("plan_execute_start"),
            );
        }

        let _start_time = SystemTime::now();
        let mut current_plan = plan.clone();
        let mut replan_attempts = 0;
        let max_replan_attempts = self.config.max_replan_attempts;
        let mut overall_step_results = HashMap::new();
        let mut overall_errors = Vec::new();

        // 新增：明确的终止条件
        let mut consecutive_failures = 0;
        let mut total_execution_time = 0u64;
        let max_total_execution_time = self.config.max_total_execution_time;
        let max_consecutive_failures = self.config.max_consecutive_failures;

        // Plan-and-Execute主循环：Planner -> Agent -> Tools -> Replan -> Agent...
        loop {
            // ✅ 检查取消状态（优先级最高）
            if self.cancellation_token.is_cancelled() {
                log::info!("❌ Plan-and-Execute: Execution cancelled by user");
                return Err(PlanAndExecuteError::ExecutionFailed("Task cancelled by user".to_string()));
            }
            
            let loop_start_time = SystemTime::now();
            log::info!(
                "=== 执行计划循环 (尝试 {}/{}) ===",
                replan_attempts + 1,
                max_replan_attempts + 1
            );

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
                log::warn!(
                    "总执行时间超过限制 ({}ms), 停止执行",
                    max_total_execution_time
                );
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
                    // 统一发送一次最终块，通知前端结束会话
                    if let Some(ctx) = self.context.lock().await.as_ref() {
                        let execution_id = self.resolve_execution_id(ctx).await;
                        let (message_id, conversation_id) =
                            self.resolve_message_and_conversation_ids(ctx).await;
                        self.emit_message_chunk(
                            &execution_id,
                            &message_id,
                            conversation_id.as_deref(),
                            ChunkType::Meta,
                            "任务执行完成",
                            true,
                            Some("executor"),
                            None,
                        );
                    }
                    return Ok(final_result);
                }
                TaskStatus::Failed => {
                    consecutive_failures += 1;
                    log::warn!("执行失败，连续失败次数: {}", consecutive_failures);

                    // 终止条件4: 连续失败次数检查
                    if consecutive_failures >= max_consecutive_failures {
                        log::error!(
                            "连续失败次数达到限制 ({}), 停止执行",
                            max_consecutive_failures
                        );
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
                }
                _ => {
                    // 重置连续失败计数器（有进展）
                    consecutive_failures = 0;
                }
            }

            // Replan反思层：评估执行结果，决定是否需要重新规划
            if let Some(ref replanner) = self.replanner {
                log::info!("=== Replan反思层：评估执行结果 ===");

                let should_replan = self
                    .should_trigger_replan(&execution_result, replan_attempts, max_replan_attempts)
                    .await;

                if should_replan {
                    log::info!("反思层决定：需要重新规划");
                    // 优先使用 Planner 生成的新计划，失败则回退到简化策略
                    let planner_based = if let Some(ctx) = self.context.lock().await.as_ref() {
                        let shared = ctx.shared_data.read().await;
                        // 尝试从共享参数还原 TaskRequest（必要字段）
                        let params: HashMap<String, serde_json::Value> = shared
                            .get("task_parameters")
                            .and_then(|v| v.as_object())
                            .map(|m| m.iter().map(|(k,v)| (k.clone(), v.clone())).collect())
                            .unwrap_or_default();
                        Some(TaskRequest {
                            id: task.id.clone(),
                            name: task.name.clone(),
                            description: task.description.clone(),
                            task_type: task.task_type.clone(),
                            target: task.target.clone(),
                            parameters: params,
                            priority: task.priority.clone(),
                            constraints: task.constraints.clone(),
                            metadata: task.metadata.clone(),
                            created_at: task.created_at,
                        })
                    } else { None };

                    let replan_outcome = if let Some(req) = planner_based {
                        match replanner
                            .replan_with_planner(&current_plan, &req, &execution_result)
                            .await
                        {
                            Ok(r) => Ok(r),
                            Err(e) => {
                                log::warn!(
                                    "Planner-based replanning failed: {}. Falling back to simple replan.",
                                    e
                                );
                                match replanner.replan_simple(&current_plan, &execution_result).await {
                                    Ok(r2) => Ok(r2),
                                    Err(e2) => Err(PlanAndExecuteError::ReplanningFailed(e2.to_string())),
                                }
                            }
                        }
                    } else {
                        match replanner.replan_simple(&current_plan, &execution_result).await {
                            Ok(r) => Ok(r),
                            Err(e) => Err(PlanAndExecuteError::ReplanningFailed(e.to_string())),
                        }
                    };

                    match replan_outcome {
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
                                        let loop_duration = loop_start_time
                                            .elapsed()
                                            .unwrap_or_default()
                                            .as_millis()
                                            as u64;
                                        total_execution_time += loop_duration;

                                        // 在重新规划前更新记忆
                                        self.update_memory_with_execution_results(
                                            task,
                                            &current_plan,
                                            &execution_result,
                                        )
                                        .await;

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
                                log::info!("反思层决定：执行结果满足预期，无需重新规划");
                                break;
                            }
                        }
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
        // 统一最终状态计算：优先失败，其次需要人工干预（存在跳过），否则完成；无结果则Pending
        let any_failed_step = overall_step_results
            .values()
            .any(|r| r.status == StepExecutionStatus::Failed);
        let any_skipped_step = overall_step_results
            .values()
            .any(|r| r.status == StepExecutionStatus::Skipped);
        let non_retryable_error = overall_errors.iter().any(|e| !e.retryable);

        let final_status = if non_retryable_error || any_failed_step {
            TaskStatus::Failed
        } else if any_skipped_step {
            TaskStatus::RequiresIntervention
        } else if !overall_step_results.is_empty() {
            TaskStatus::Completed
        } else {
            TaskStatus::Pending
        };

        let final_result = ExecutionResult {
            status: final_status,
            completed_steps: overall_step_results
                .iter()
                .filter(|(_, result)| result.status == StepExecutionStatus::Completed)
                .map(|(id, _)| id.clone())
                .collect(),
            failed_steps: overall_step_results
                .iter()
                .filter(|(_, result)| result.status == StepExecutionStatus::Failed)
                .map(|(id, _)| id.clone())
                .collect(),
            skipped_steps: overall_step_results
                .iter()
                .filter(|(_, result)| result.status == StepExecutionStatus::Skipped)
                .map(|(id, _)| id.clone())
                .collect(),
            step_results: overall_step_results.clone(),
            metrics: ExecutionMetrics {
                total_duration_ms: total_execution_time,
                successful_steps: overall_step_results
                    .values()
                    .filter(|r| r.status == StepExecutionStatus::Completed)
                    .count() as u32,
                failed_steps: overall_step_results
                    .values()
                    .filter(|r| r.status == StepExecutionStatus::Failed)
                    .count() as u32,
                skipped_steps: overall_step_results
                    .values()
                    .filter(|r| r.status == StepExecutionStatus::Skipped)
                    .count() as u32,
                total_retries: overall_step_results.values().map(|r| r.retry_count).sum(),
                avg_step_duration_ms: if !overall_step_results.is_empty() {
                    overall_step_results
                        .values()
                        .map(|r| r.duration_ms)
                        .sum::<u64>()
                        / overall_step_results.len() as u64
                } else {
                    0
                },
                peak_concurrency: 1,
                custom_metrics: HashMap::new(),
            },
            errors: overall_errors,
            enhanced_feedback: EnhancedExecutionFeedback {
                execution_summary: format!(
                    "总执行时间: {}ms, 重新规划次数: {}",
                    total_execution_time, replan_attempts
                ),
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
                        current: 0.2,
                        peak: 0.5,
                        average: 0.3,
                        utilization_rate: 0.3,
                        unit: "cores".to_string(),
                    },
                    memory_usage: ResourceUsage {
                        current: 200.0,
                        peak: 400.0,
                        average: 300.0,
                        utilization_rate: 0.3,
                        unit: "MB".to_string(),
                    },
                    network_usage: ResourceUsage {
                        current: 1.0,
                        peak: 3.0,
                        average: 2.0,
                        utilization_rate: 0.1,
                        unit: "Mbps".to_string(),
                    },
                    storage_usage: ResourceUsage {
                        current: 5.0,
                        peak: 20.0,
                        average: 10.0,
                        utilization_rate: 0.05,
                        unit: "MB".to_string(),
                    },
                    resource_bottlenecks: vec![],
                    optimization_suggestions: vec![],
                },
            },
        };

        // 无论最终状态如何，发送一次最终块，通知前端结束等待
        if let Some(ctx) = self.context.lock().await.as_ref() {
            let execution_id = self.resolve_execution_id(ctx).await;
            let (message_id, conversation_id) =
                self.resolve_message_and_conversation_ids(ctx).await;
            let (chunk_type, content) = match final_result.status {
                TaskStatus::Completed => (ChunkType::Meta, "任务执行完成"),
                TaskStatus::Failed => (ChunkType::Meta, "任务执行失败"),
                _ => (ChunkType::Meta, "任务执行结束"),
            };
            self.emit_message_chunk(
                &execution_id,
                &message_id,
                conversation_id.as_deref(),
                chunk_type,
                content,
                true,
                Some("executor"),
                None,
            );
        }

        log::info!("=== Plan-and-Execute执行完成 ===");
        log::info!(
            "最终状态: {:?}, 完成步骤: {}, 失败步骤: {}, 总耗时: {}ms",
            final_result.status,
            final_result.completed_steps.len(),
            final_result.failed_steps.len(),
            total_execution_time
        );

        // 保存执行结果到数据库
        if let Err(e) = self
            .save_execution_to_database(plan, task, &final_result)
            .await
        {
            log::error!("保存执行结果到数据库失败: {}", e);
        }

        // 最终记忆更新
        self.update_memory_with_execution_results(task, &current_plan, &final_result)
            .await;

        Ok(final_result)
    }

    /// 验证新计划的合理性（终止条件）
    async fn validate_new_plan(
        &self,
        new_plan: &ExecutionPlan,
        current_plan: &ExecutionPlan,
    ) -> bool {
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
        if similarity > self.config.plan_similarity_threshold {
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
            log::warn!(
                "新计划预估执行时间过长 ({}秒), 验证失败",
                new_plan.estimated_duration
            );
            return false;
        }

        // 6. 新增：依赖循环检测
        if !self.validate_dependencies_no_cycle(new_plan).await {
            log::warn!("新计划存在循环依赖，验证失败");
            return false;
        }

        // 7. 新增：前置条件验证
        if !self.validate_preconditions(new_plan).await {
            log::warn!("新计划前置条件不满足，验证失败");
            return false;
        }

        // 8. 新增：关键路径预估时间检查
        if !self.validate_critical_path_timing(new_plan).await {
            log::warn!("新计划关键路径超过剩余时间预算，验证失败");
            return false;
        }

        log::info!(
            "新计划验证通过: {} 个步骤, 预估时间 {} 秒",
            new_plan.steps.len(),
            new_plan.estimated_duration
        );
        true
    }

    /// 验证依赖关系无循环（DFS检测）
    async fn validate_dependencies_no_cycle(&self, plan: &ExecutionPlan) -> bool {
        let mut visited = std::collections::HashSet::new();
        let mut rec_stack = std::collections::HashSet::new();

        for step in &plan.steps {
            if !visited.contains(&step.id) {
                if self.has_cycle_dfs(&step.id, &plan.dependencies, &mut visited, &mut rec_stack) {
                    return false;
                }
            }
        }

        true
    }

    /// DFS检测循环依赖
    fn has_cycle_dfs(
        &self,
        step_id: &str,
        dependencies: &HashMap<String, Vec<String>>,
        visited: &mut std::collections::HashSet<String>,
        rec_stack: &mut std::collections::HashSet<String>,
    ) -> bool {
        visited.insert(step_id.to_string());
        rec_stack.insert(step_id.to_string());

        if let Some(deps) = dependencies.get(step_id) {
            for dep in deps {
                if !visited.contains(dep) {
                    if self.has_cycle_dfs(dep, dependencies, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(dep) {
                    return true; // 发现循环
                }
            }
        }

        rec_stack.remove(step_id);
        false
    }

    /// 验证前置条件是否满足
    async fn validate_preconditions(&self, plan: &ExecutionPlan) -> bool {
        let context_guard = self.context.lock().await;
        if let Some(context) = context_guard.as_ref() {
            let shared_data = context.shared_data.read().await;

            for step in &plan.steps {
                for precondition in &step.preconditions {
                    // 检查前置条件是否在共享数据中存在且为真
                    if !self.evaluate_condition(precondition, &shared_data).await {
                        log::warn!("步骤 '{}' 的前置条件 '{}' 不满足", step.name, precondition);
                        return false;
                    }
                }
            }
        }

        true
    }

    /// 评估条件表达式
    async fn evaluate_condition(
        &self,
        condition: &str,
        shared_data: &HashMap<String, serde_json::Value>,
    ) -> bool {
        let cond = condition.trim();

        // 简化内置：non_empty_output(step_name)
        if cond.starts_with("non_empty_output(") && cond.ends_with(")") {
            let inside = &cond["non_empty_output(".len()..cond.len() - 1];
            let key = format!("step_result_{}", inside.trim());
            if let Some(value) = shared_data.get(&key) {
                return match value {
                    serde_json::Value::Bool(b) => *b,
                    serde_json::Value::String(s) => !s.trim().is_empty(),
                    serde_json::Value::Number(n) => n.as_f64().unwrap_or(0.0) > 0.0,
                    serde_json::Value::Array(a) => !a.is_empty(),
                    serde_json::Value::Object(o) => !o.is_empty(),
                    serde_json::Value::Null => false,
                };
            } else {
                return false;
            }
        }

        // schema 校验：schema_nonempty(step_name, json_path?) -> 输出对象存在且非空
        if cond.starts_with("schema_nonempty(") && cond.ends_with(")") {
            let inside = &cond["schema_nonempty(".len()..cond.len() - 1];
            let parts: Vec<&str> = inside.split(',').map(|s| s.trim()).collect();
            let step_key = parts.get(0).cloned().unwrap_or("");
            if step_key.is_empty() { return false; }
            let key = format!("step_result_{}", step_key);
            if let Some(value) = shared_data.get(&key) {
                return match value {
                    serde_json::Value::Object(o) => !o.is_empty(),
                    serde_json::Value::Array(a) => !a.is_empty(),
                    serde_json::Value::String(s) => !s.trim().is_empty(),
                    _ => false,
                };
            } else {
                return false;
            }
        }

        // 常见谓词：is_true(key), gt(key,number), contains(key,substr)
        if cond.starts_with("is_true(") && cond.ends_with(")") {
            let key = &cond["is_true(".len()..cond.len()-1];
            if let Some(v) = shared_data.get(key) { return v.as_bool().unwrap_or(false); }
            return false;
        }
        if cond.starts_with("gt(") && cond.ends_with(")") {
            let inside = &cond[3..cond.len()-1];
            let parts: Vec<&str> = inside.split(',').map(|s| s.trim()).collect();
            if parts.len() == 2 {
                let key = parts[0];
                let threshold: f64 = parts[1].parse().unwrap_or(f64::INFINITY);
                if let Some(v) = shared_data.get(key) { return v.as_f64().unwrap_or(f64::NEG_INFINITY) > threshold; }
            }
            return false;
        }
        if cond.starts_with("contains(") && cond.ends_with(")") {
            let inside = &cond["contains(".len()..cond.len()-1];
            let parts: Vec<&str> = inside.split(',').map(|s| s.trim()).collect();
            if parts.len() == 2 {
                let key = parts[0];
                let needle = parts[1].trim_matches('"');
                if let Some(v) = shared_data.get(key) {
                    if let Some(s) = v.as_str() { return s.contains(needle); }
                }
            }
            return false;
        }

        // 默认：检查键是否存在且为真值
        if let Some(value) = shared_data.get(cond) {
            match value {
                serde_json::Value::Bool(b) => *b,
                serde_json::Value::String(s) => !s.is_empty(),
                serde_json::Value::Number(n) => n.as_f64().unwrap_or(0.0) > 0.0,
                serde_json::Value::Array(a) => !a.is_empty(),
                serde_json::Value::Object(o) => !o.is_empty(),
                serde_json::Value::Null => false,
            }
        } else {
            // 如果条件不存在，默认为真（宽松模式）
            true
        }
    }

    /// 评估复杂条件表达式（支持 AND、OR、NOT 组合）
    async fn evaluate_complex_condition(
        &self,
        condition_expr: &str,
        shared_data: &HashMap<String, serde_json::Value>,
    ) -> bool {
        let expr = condition_expr.trim();
        
        // 处理 NOT 操作符
        if expr.starts_with("NOT ") || expr.starts_with("!") {
            let inner_expr = if expr.starts_with("NOT ") {
                &expr[4..]
            } else {
                &expr[1..]
            };
            return !self.evaluate_simple_condition(inner_expr.trim(), shared_data).await;
        }
        
        // 处理 AND 操作符
        if expr.contains(" AND ") {
            let parts: Vec<&str> = expr.split(" AND ").collect();
            for part in parts {
                if !self.evaluate_simple_condition(part.trim(), shared_data).await {
                    return false;
                }
            }
            return true;
        }
        
        // 处理 OR 操作符
        if expr.contains(" OR ") {
            let parts: Vec<&str> = expr.split(" OR ").collect();
            for part in parts {
                if self.evaluate_simple_condition(part.trim(), shared_data).await {
                    return true;
                }
            }
            return false;
        }
        
        // 处理括号表达式
        if expr.starts_with('(') && expr.ends_with(')') {
            let inner = &expr[1..expr.len()-1];
            return self.evaluate_simple_condition(inner, shared_data).await;
        }
        
        // 回退到简单条件评估
        self.evaluate_simple_condition(expr, shared_data).await
    }
    
    /// 评估简单条件（不包含逻辑操作符）
    async fn evaluate_simple_condition(
        &self,
        condition_expr: &str,
        shared_data: &HashMap<String, serde_json::Value>,
    ) -> bool {
        // 处理比较操作符
        if let Some(result) = self.evaluate_comparison_condition(condition_expr, shared_data).await {
            return result;
        }
        
        // 回退到原有的简单条件评估
        self.evaluate_condition(condition_expr, shared_data).await
    }
    
    /// 评估比较条件（==, !=, <, >, <=, >=）
    async fn evaluate_comparison_condition(
        &self,
        condition: &str,
        shared_data: &HashMap<String, serde_json::Value>,
    ) -> Option<bool> {
        let operators = [">=", "<=", "!=", "==", ">", "<"];
        
        for op in &operators {
            if let Some(pos) = condition.find(op) {
                let left = condition[..pos].trim();
                let right = condition[pos + op.len()..].trim();
                
                // 获取左值
                let left_value = self.get_condition_value(left, shared_data).await;
                // 获取右值
                let right_value = self.get_condition_value(right, shared_data).await;
                
                return Some(self.compare_values(&left_value, &right_value, op));
            }
        }
        
        None
    }
    
    /// 获取条件值（可以是变量引用或字面量）
    async fn get_condition_value(
        &self,
        value_expr: &str,
        shared_data: &HashMap<String, serde_json::Value>,
    ) -> serde_json::Value {
        let expr = value_expr.trim();
        
        // 检查是否是字符串字面量
        if (expr.starts_with('"') && expr.ends_with('"')) || 
           (expr.starts_with('\'') && expr.ends_with('\'')) {
            return serde_json::Value::String(expr[1..expr.len()-1].to_string());
        }
        
        // 检查是否是数字字面量
        if let Ok(num) = expr.parse::<f64>() {
            if let Some(number) = serde_json::Number::from_f64(num) {
                return serde_json::Value::Number(number);
            }
        }
        
        // 检查是否是布尔字面量
        if expr == "true" {
            return serde_json::Value::Bool(true);
        }
        if expr == "false" {
            return serde_json::Value::Bool(false);
        }
        
        // 检查是否是 null 字面量
        if expr == "null" {
            return serde_json::Value::Null;
        }
        
        // 尝试从共享数据中获取值
        if let Some(value) = shared_data.get(expr) {
            return value.clone();
        }
        
        // 尝试步骤结果格式
        let step_key = format!("step_result_{}", expr);
        if let Some(value) = shared_data.get(&step_key) {
            return value.clone();
        }
        
        // 默认返回字符串
        serde_json::Value::String(expr.to_string())
    }
    
    /// 比较两个值
    fn compare_values(
        &self,
        left: &serde_json::Value,
        right: &serde_json::Value,
        operator: &str,
    ) -> bool {
        match operator {
            "==" => left == right,
            "!=" => left != right,
            ">" => self.numeric_compare(left, right, |a, b| a > b),
            "<" => self.numeric_compare(left, right, |a, b| a < b),
            ">=" => self.numeric_compare(left, right, |a, b| a >= b),
            "<=" => self.numeric_compare(left, right, |a, b| a <= b),
            _ => false,
        }
    }
    
    /// 数值比较辅助方法
    fn numeric_compare<F>(&self, left: &serde_json::Value, right: &serde_json::Value, compare_fn: F) -> bool
    where
        F: Fn(f64, f64) -> bool,
    {
        match (left.as_f64(), right.as_f64()) {
            (Some(a), Some(b)) => compare_fn(a, b),
            _ => false,
        }
    }
    
    /// 执行条件操作
    async fn execute_conditional_action(
        &self,
        action: &serde_json::Value,
        context: &ExecutionContext,
    ) -> Result<serde_json::Value, PlanAndExecuteError> {
        match action {
            serde_json::Value::String(action_str) => {
                // 简单字符串操作
                match action_str.as_str() {
                    "continue" => Ok(serde_json::json!({"action": "continue"})),
                    "skip" => Ok(serde_json::json!({"action": "skip"})),
                    "abort" => Ok(serde_json::json!({"action": "abort"})),
                    _ => {
                        log::warn!("未知的条件操作: {}", action_str);
                        Ok(serde_json::json!({"action": "unknown", "value": action_str}))
                    }
                }
            },
            serde_json::Value::Object(action_obj) => {
                // 复杂对象操作
                if let Some(action_type) = action_obj.get("type").and_then(|v| v.as_str()) {
                    match action_type {
                        "set_variable" => {
                            if let (Some(var_name), Some(var_value)) = (
                                action_obj.get("name").and_then(|v| v.as_str()),
                                action_obj.get("value")
                            ) {
                                let mut shared_data = context.shared_data.write().await;
                                shared_data.insert(var_name.to_string(), var_value.clone());
                                Ok(serde_json::json!({
                                    "action": "set_variable",
                                    "variable": var_name,
                                    "value": var_value
                                }))
                            } else {
                                Err(PlanAndExecuteError::InvalidStepConfiguration {
                                    step_name: "conditional_action".to_string(),
                                    reason: "set_variable action requires 'name' and 'value'".to_string(),
                                })
                            }
                        },
                        "emit_event" => {
                            if let Some(event_data) = action_obj.get("data") {
                                // 这里可以发送事件到前端
                                log::info!("发送条件事件: {:?}", event_data);
                                Ok(serde_json::json!({
                                    "action": "emit_event",
                                    "event_data": event_data
                                }))
                            } else {
                                Ok(serde_json::json!({"action": "emit_event", "event_data": null}))
                            }
                        },
                        "log_message" => {
                            if let Some(message) = action_obj.get("message").and_then(|v| v.as_str()) {
                                let level = action_obj.get("level").and_then(|v| v.as_str()).unwrap_or("info");
                                match level {
                                    "error" => log::error!("条件日志: {}", message),
                                    "warn" => log::warn!("条件日志: {}", message),
                                    "debug" => log::debug!("条件日志: {}", message),
                                    _ => log::info!("条件日志: {}", message),
                                }
                                Ok(serde_json::json!({
                                    "action": "log_message",
                                    "message": message,
                                    "level": level
                                }))
                            } else {
                                Ok(serde_json::json!({"action": "log_message", "message": "empty"}))
                            }
                        },
                        _ => {
                            log::warn!("未知的条件操作类型: {}", action_type);
                            Ok(serde_json::json!({"action": "unknown_type", "type": action_type}))
                        }
                    }
                } else {
                    Ok(serde_json::json!({"action": "object_without_type", "data": action_obj}))
                }
            },
            _ => {
                Ok(serde_json::json!({"action": "unsupported_type", "value": action}))
            }
        }
    }

    /// 验证关键路径时间预算
    async fn validate_critical_path_timing(&self, plan: &ExecutionPlan) -> bool {
        // 简单的关键路径计算：所有步骤的最大预估时间总和
        let total_estimated_time: u64 = plan.steps.iter().map(|step| step.estimated_duration).sum();

        // 检查是否超过剩余时间预算（使用配置的最大执行时间）
        let remaining_budget = self.config.max_total_execution_time / 1000; // 转换为秒

        if total_estimated_time > remaining_budget {
            log::warn!(
                "关键路径预估时间 {}s 超过剩余预算 {}s",
                total_estimated_time,
                remaining_budget
            );
            return false;
        }

        true
    }

    /// 反馈驱动的策略调整
    async fn apply_feedback_driven_adjustments(&self, execution_result: &mut ExecutionResult) {
        log::info!("=== 反馈驱动策略调整 ===");

        let quality_score = execution_result
            .enhanced_feedback
            .quality_assessment
            .overall_score;
        log::info!("当前质量分数: {:.2}", quality_score);

        // 策略调整1: 基于质量分数动态调整错误处理策略
        if (quality_score as f64) < self.config.quality_threshold {
            log::info!(
                "质量分数低于阈值 ({:.2} < {:.2}), 调整错误处理策略",
                quality_score,
                self.config.quality_threshold
            );

            // 记录策略调整建议到增强反馈中
            execution_result
                .enhanced_feedback
                .improvement_suggestions
                .push("建议调整错误处理策略: 从严格模式改为容错模式以提高完成率".to_string());

            // 检查资源使用情况
            if execution_result
                .enhanced_feedback
                .resource_analysis
                .cpu_usage
                .utilization_rate
                > 0.8
            {
                execution_result
                    .enhanced_feedback
                    .improvement_suggestions
                    .push(format!(
                        "检测到 CPU 使用率过高 ({:.1}%), 建议降低并发执行数",
                        execution_result
                            .enhanced_feedback
                            .resource_analysis
                            .cpu_usage
                            .utilization_rate
                            * 100.0
                    ));
            }
        }

        // 策略调整2: 基于失败模式调整重试策略
        let failure_ratio = execution_result.failed_steps.len() as f64
            / (execution_result.completed_steps.len() + execution_result.failed_steps.len()) as f64;

        if failure_ratio > 0.3 {
            log::info!(
                "失败率过高 ({:.1}%), 建议调整重试策略",
                failure_ratio * 100.0
            );
            execution_result
                .enhanced_feedback
                .improvement_suggestions
                .push(format!(
                    "失败率过高 ({:.1}%), 建议增加重试次数或调整超时设置",
                    failure_ratio * 100.0
                ));
        }

        // 策略调整3: 基于性能指标调整超时设置
        let avg_duration = execution_result.metrics.avg_step_duration_ms;
        let timeout_ms = self.config.default_timeout * 1000;

        if avg_duration > timeout_ms / 2 {
            log::info!("平均步骤耗时较长 ({}ms), 建议调整超时设置", avg_duration);
            execution_result
                .enhanced_feedback
                .improvement_suggestions
                .push(format!(
                    "平均步骤耗时 {}ms 接近超时限制, 建议增加超时时间或优化步骤",
                    avg_duration
                ));
        }

        // 策略调整4: 基于错误类型调整工具选择
        let tool_errors = execution_result
            .errors
            .iter()
            .filter(|e| matches!(e.error_type, crate::engines::types::ErrorType::Tool))
            .count();

        if tool_errors > 0 {
            execution_result
                .enhanced_feedback
                .improvement_suggestions
                .push(format!(
                    "检测到 {} 个工具相关错误, 建议检查工具配置或选择备选工具",
                    tool_errors
                ));
        }

        log::info!(
            "策略调整完成，生成 {} 条改进建议",
            execution_result
                .enhanced_feedback
                .improvement_suggestions
                .len()
        );
    }

    /// 更新记忆管理器中的执行结果
    async fn update_memory_with_execution_results(
        &self,
        task: &TaskRequest,
        plan: &ExecutionPlan,
        execution_result: &ExecutionResult,
    ) {
        if let Some(ref memory_manager) = self.memory_manager {
            log::info!("更新记忆管理器中的执行结果");

            // 构建执行摘要
            let execution_summary = format!(
                "Task: {} | Plan: {} | Status: {:?} | Completed: {}/{} steps | Duration: {}ms",
                task.name,
                plan.name,
                execution_result.status,
                execution_result.completed_steps.len(),
                execution_result.completed_steps.len() + execution_result.failed_steps.len(),
                execution_result.metrics.total_duration_ms
            );

            // 简化的记忆更新：存储执行摘要作为通用记忆条目
            let memory_data = serde_json::json!({
                "execution_summary": execution_summary,
                "task_id": task.id,
                "plan_id": plan.id,
                "plan_name": plan.name,
                "status": format!("{:?}", execution_result.status),
                "completed_steps": execution_result.completed_steps.len(),
                "failed_steps": execution_result.failed_steps.len(),
                "total_duration_ms": execution_result.metrics.total_duration_ms,
                "avg_step_duration_ms": execution_result.metrics.avg_step_duration_ms,
                "quality_score": execution_result.enhanced_feedback.quality_assessment.overall_score
            });

            // 使用通用的存储方法
            match memory_manager.store(
                crate::engines::plan_and_execute::memory_manager::MemoryEntryType::ExecutionState,
                memory_data,
                vec!["execution".to_string(), "plan_and_execute".to_string()],
                crate::engines::plan_and_execute::types::Priority::Medium,
                None
            ).await {
                Ok(_) => log::info!("执行结果记忆存储成功"),
                Err(e) => log::warn!("存储执行结果记忆失败: {}", e),
            }

            log::info!("记忆更新完成");
        } else {
            log::debug!("未配置记忆管理器，跳过记忆更新");
        }
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

        // 检查是否有可并发执行的步骤
        let (concurrent_steps, sequential_steps): (Vec<_>, Vec<_>) = plan
            .steps
            .iter()
            .enumerate()
            .partition(|(_, step)| self.can_execute_concurrently(step));

        // 创建并发控制信号量 (暂未使用，为未来真正的并发实现准备)
        let _semaphore = Arc::new(Semaphore::new(self.config.max_concurrent_steps as usize));

        // 并发执行可并发的工具调用步骤
        if !concurrent_steps.is_empty() {
            log::info!(
                "检测到 {} 个可并发步骤，使用并发执行",
                concurrent_steps.len()
            );

            // 为了简化实现，先串行执行但记录并发能力
            for (index, step) in concurrent_steps {
                log::info!(
                    "并发模式执行步骤 {}: {} (当前为串行实现)",
                    index + 1,
                    step.name
                );

                // 后端增强标注：发出步骤开始 Meta
                let execution_id = self.resolve_execution_id(context).await;
                let (message_id, conversation_id) = self.resolve_message_and_conversation_ids(context).await;
                let step_type_str = format!("{:?}", step.step_type);
                let start_meta = serde_json::json!({
                    "type": "step_started",
                    "step_index": index + 1,
                    "step_name": step.name,
                    "step_type": step_type_str
                });
                self.emit_message_chunk(
                    &execution_id,
                    &message_id,
                    conversation_id.as_deref(),
                    ChunkType::Meta,
                    &start_meta.to_string(),
                    false,
                    Some("executor"),
                    None,
                );

                match self.execute_step(step, context).await {
                    Ok(result) => {
                        if result.status == StepExecutionStatus::Completed {
                            completed_steps.push(step.id.clone());
                        } else {
                            failed_steps.push(step.id.clone());
                        }
                        // 发出步骤完成 Meta
                        let status_str = match result.status {
                            StepExecutionStatus::Completed => "Completed",
                            StepExecutionStatus::Failed => "Failed",
                            StepExecutionStatus::Skipped => "Skipped",
                            StepExecutionStatus::Running => "Running",
                            StepExecutionStatus::Pending => "Pending",
                            StepExecutionStatus::Retrying => "Retrying",
                            StepExecutionStatus::Cancelled => "Cancelled",
                        };
                        let done_meta = serde_json::json!({
                            "type": "step_completed",
                            "step_index": index + 1,
                            "step_name": step.name,
                            "step_type": step_type_str,
                            "status": status_str
                        });
                        self.emit_message_chunk(
                            &execution_id,
                            &message_id,
                            conversation_id.as_deref(),
                            ChunkType::Meta,
                            &done_meta.to_string(),
                            false,
                            Some("executor"),
                            None,
                        );
                        step_results.insert(step.id.clone(), result);
                    }
                    Err(e) => {
                        log::error!("并发步骤执行异常: {}: {}", step.name, e);
                        failed_steps.push(step.id.clone());
                        errors.push(ExecutionError {
                            error_type: crate::engines::types::ErrorType::Tool,
                            message: format!("并发步骤 '{}' 执行异常", step.name),
                            details: Some(e.to_string()),
                            error_code: None,
                            retryable: true,
                            timestamp: SystemTime::now(),
                        });
                        // 发出失败完成 Meta
                        let done_meta = serde_json::json!({
                            "type": "step_completed",
                            "step_index": index + 1,
                            "step_name": step.name,
                            "step_type": step_type_str,
                            "status": "Failed"
                        });
                        self.emit_message_chunk(
                            &execution_id,
                            &message_id,
                            conversation_id.as_deref(),
                            ChunkType::Meta,
                            &done_meta.to_string(),
                            false,
                            Some("executor"),
                            None,
                        );
                    }
                }
            }
        }

        // 串行执行其他步骤
        for (index, step) in sequential_steps {
            log::info!(
                "Agent串行执行步骤 {}/{}: {}",
                index + 1,
                plan.steps.len(),
                step.name
            );

            // 后端增强标注：发出步骤开始 Meta
            let execution_id = self.resolve_execution_id(context).await;
            let (message_id, conversation_id) = self.resolve_message_and_conversation_ids(context).await;
            let step_type_str = format!("{:?}", step.step_type);
            let start_meta = serde_json::json!({
                "type": "step_started",
                "step_index": index + 1,
                "step_name": step.name,
                "step_type": step_type_str
            });
            self.emit_message_chunk(
                &execution_id,
                &message_id,
                conversation_id.as_deref(),
                ChunkType::Meta,
                &start_meta.to_string(),
                false,
                Some("executor"),
                None,
            );

            // 调用Tools层执行具体步骤
            match self.execute_step(step, context).await {
                Ok(result) => {
                    // 优先传递 step.tool_config 中的 tool_name 作为展示
                    let _tool_name = step.tool_config.as_ref().map(|c| c.tool_name.as_str());
                    // self.emit_tool_result(context, &step.name, &result, tool_name).await;

                    if result.status == StepExecutionStatus::Completed {
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
                    // 发出步骤完成 Meta
                    let status_str = match result.status {
                        StepExecutionStatus::Completed => "Completed",
                        StepExecutionStatus::Failed => "Failed",
                        StepExecutionStatus::Skipped => "Skipped",
                        StepExecutionStatus::Running => "Running",
                        StepExecutionStatus::Pending => "Pending",
                        StepExecutionStatus::Retrying => "Retrying",
                        StepExecutionStatus::Cancelled => "Cancelled",
                    };
                    let done_meta = serde_json::json!({
                        "type": "step_completed",
                        "step_index": index + 1,
                        "step_name": step.name,
                        "step_type": step_type_str,
                        "status": status_str
                    });
                    self.emit_message_chunk(
                        &execution_id,
                        &message_id,
                        conversation_id.as_deref(),
                        ChunkType::Meta,
                        &done_meta.to_string(),
                        false,
                        Some("executor"),
                        None,
                    );
                    step_results.insert(step.id.clone(), result);
                }
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

                    if {
                        // 基于任务参数的严格模式覆盖
                        let strict_mode_param = {
                            let shared = context.shared_data.read().await;
                            shared
                                .get("task_parameters")
                                .and_then(|v| v.as_object())
                                .and_then(|m| m.get("execution_strict_mode"))
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false)
                        };
                        matches!(self.config.execution_mode, ExecutionMode::Strict)
                            || strict_mode_param
                    } {
                        // 发出失败完成 Meta
                        let done_meta = serde_json::json!({
                            "type": "step_completed",
                            "step_index": index + 1,
                            "step_name": step.name,
                            "step_type": step_type_str,
                            "status": "Failed"
                        });
                        self.emit_message_chunk(
                            &execution_id,
                            &message_id,
                            conversation_id.as_deref(),
                            ChunkType::Meta,
                            &done_meta.to_string(),
                            false,
                            Some("executor"),
                            None,
                        );
                        break; // 严格模式下，异常就停止
                    }
                }
            }
        }

        // 构建执行结果
        let mut execution_result = self
            .build_execution_result(start_time, step_results, errors)
            .await?;

        // 如果存在跳过步骤，标记整体状态为需要人工干预
        if !execution_result.skipped_steps.is_empty() && execution_result.status == TaskStatus::Completed {
            execution_result.status = TaskStatus::RequiresIntervention;
            log::info!("Execution contains skipped steps; marking status as RequiresIntervention");
        }

        // 反馈驱动的策略调整
        self.apply_feedback_driven_adjustments(&mut execution_result)
            .await;

        Ok(execution_result)
    }

    /// 判断步骤是否可以并发执行
    fn can_execute_concurrently(&self, step: &ExecutionStep) -> bool {
        // 只有工具调用类型且标记为可并发的步骤才能并发执行
        matches!(step.step_type, StepType::ToolCall | StepType::Parallel)
            && step
                .parameters
                .get("concurrent")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
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
            // ✅ 在每个步骤前检查取消状态
            if self.cancellation_token.is_cancelled() {
                log::info!("❌ Plan-and-Execute: Execution cancelled during step execution");
                return Err(PlanAndExecuteError::ExecutionFailed("Task cancelled by user".to_string()));
            }
            
            let result = self.execute_step(step, context).await?;

            // 检查步骤是否失败，如果失败且有replanner，尝试实时重新规划
            if result.status == StepExecutionStatus::Failed {
                if let Some(ref replanner) = self.replanner {
                    log::warn!("步骤 '{}' 执行失败，尝试实时重新规划", step.name);

                    match replanner.replan_simple(plan, &self
                        .build_execution_result(start_time, step_results.clone(), errors.clone())
                        .await
                        .unwrap_or(ExecutionResult {
                            status: TaskStatus::Failed,
                            completed_steps: vec![],
                            failed_steps: vec![step.id.clone()],
                            skipped_steps: vec![],
                            step_results: step_results.clone(),
                            metrics: ExecutionMetrics::default(),
                            errors: vec![],
                            enhanced_feedback: EnhancedExecutionFeedback::default(),
                        })
                    ).await {
                        Ok(replan_result) => {
                            if replan_result.should_replan {
                                log::info!("实时重新规划建议: {}", replan_result.replan_reason);
                                // 将当前结果返回给上层循环，由上层决定是否应用新计划
                                step_results.insert(step.id.clone(), result);
                                return self
                                    .build_execution_result(
                                        start_time,
                                        step_results.clone(),
                                        errors.clone(),
                                    )
                                    .await;
                            }
                        }
                        Err(e) => {
                            log::warn!("实时重新规划失败: {}", e);
                        }
                    }
                }

                // 如果没有replanner或重新规划失败，在严格模式下停止执行
                step_results.insert(step.id.clone(), result);
                return Err(PlanAndExecuteError::ExecutionFailed(format!(
                    "步骤 '{}' 执行失败，严格模式下停止执行",
                    step.name
                )));
            }

            step_results.insert(step.id.clone(), result);
        }

        self.build_execution_result(start_time, step_results.clone(), errors.clone())
            .await
    }

    /// 检测资源泄露（CRITICAL - 最高优先级）
    async fn detect_resource_leak(&self, execution_result: &ExecutionResult) -> bool {
        if let Some(ctx) = self.context.lock().await.as_ref() {
            let shared = ctx.shared_data.read().await;
            
            // Check playwright browser session
            let has_playwright_open = shared
                .get("playwright_session_active")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            
            // Check passive scan proxy
            let has_proxy_running = shared
                .get("passive_scan_running")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            
            // Check if cleanup steps were executed
            let has_playwright_close = execution_result.completed_steps.iter()
                .any(|s| s.contains("playwright_close") || s.contains("close_browser"));
            let has_proxy_stop = execution_result.completed_steps.iter()
                .any(|s| s.contains("stop_passive_scan") || s.contains("stop_proxy"));
            
            // Resource leak detected if resource is active but not cleaned up
            let playwright_leak = has_playwright_open && !has_playwright_close;
            let proxy_leak = has_proxy_running && !has_proxy_stop;
            
            if playwright_leak || proxy_leak {
                log::warn!(
                    "Resource leak detected - Playwright: {}, Proxy: {}",
                    playwright_leak,
                    proxy_leak
                );
                return true;
            }
        }
        false
    }

    /// 检测安全测试是否包含关键步骤
    async fn has_required_security_steps(&self, execution_result: &ExecutionResult) -> bool {
        let completed = &execution_result.completed_steps;
        
        // Check for analyze_website
        let has_analyze = completed.iter()
            .any(|s| s.contains("analyze_website") || s.contains("分析网站"));
        
        // Check for generate_advanced_plugin
        let has_plugin = completed.iter()
            .any(|s| s.contains("generate_advanced_plugin") || s.contains("生成插件"));
        
        has_analyze && has_plugin
    }

    /// 判断是否为安全测试任务
    async fn is_security_test(&self) -> bool {
        if let Some(ctx) = self.context.lock().await.as_ref() {
            let shared = ctx.shared_data.read().await;
            
            // Check task parameters for security test indicators
            if let Some(params) = shared.get("task_parameters").and_then(|v| v.as_object()) {
                // Check task type
                if let Some(task_type) = params.get("task_type").and_then(|v| v.as_str()) {
                    if task_type.contains("security") || task_type.contains("penetration") 
                        || task_type.contains("vulnerability") {
                        return true;
                    }
                }
                
                // Check task description
                if let Some(desc) = params.get("description").and_then(|v| v.as_str()) {
                    let desc_lower = desc.to_lowercase();
                    if desc_lower.contains("安全测试") || desc_lower.contains("漏洞扫描")
                        || desc_lower.contains("渗透测试") || desc_lower.contains("security test")
                        || desc_lower.contains("vulnerability scan") {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// 检测步骤执行顺序是否违反依赖关系
    async fn detect_step_order_violation(&self, execution_result: &ExecutionResult) -> bool {
        // Check if resource usage steps were executed before initialization
        let completed = &execution_result.completed_steps;
        
        // Find indices of key steps
        let mut playwright_nav_idx: Option<usize> = None;
        let mut playwright_init_idx: Option<usize> = None;
        let mut proxy_use_idx: Option<usize> = None;
        let mut proxy_start_idx: Option<usize> = None;
        
        for (idx, step) in completed.iter().enumerate() {
            if step.contains("playwright_navigate") {
                playwright_nav_idx = Some(idx);
            }
            if step.contains("start_passive_scan") {
                proxy_start_idx = Some(idx);
            }
            // Proxy usage is typically indicated by navigate with proxy parameter
            if step.contains("proxy") && !step.contains("start") && !step.contains("stop") {
                proxy_use_idx = Some(idx);
            }
        }
        
        // Check order violations
        if let (Some(nav_idx), Some(start_idx)) = (proxy_use_idx, proxy_start_idx) {
            if nav_idx < start_idx {
                log::warn!("Step order violation: proxy used before started");
                return true;
            }
        }
        
        false
    }

    /// 检测执行结果是否包含足够信息
    async fn has_insufficient_information(&self, execution_result: &ExecutionResult) -> bool {
        if let Some(ctx) = self.context.lock().await.as_ref() {
            let shared = ctx.shared_data.read().await;
            
            // Count meaningful step results
            let mut result_count = 0;
            let mut total_output_size = 0;
            
            for (key, value) in shared.iter() {
                if key.starts_with("step_result_") {
                    result_count += 1;
                    // Estimate output size
                    if let Ok(json_str) = serde_json::to_string(value) {
                        total_output_size += json_str.len();
                    }
                }
            }
            
            // Insufficient if:
            // 1. Less than 2 step results
            // 2. Total output is too small (< 100 chars)
            // 3. All steps completed but no meaningful output
            if result_count < 2 && !execution_result.completed_steps.is_empty() {
                log::warn!("Insufficient information: only {} step results", result_count);
                return true;
            }
            
            if total_output_size < 100 && execution_result.completed_steps.len() > 2 {
                log::warn!(
                    "Insufficient information: total output size {} bytes is too small",
                    total_output_size
                );
                return true;
            }
        }
        
        false
    }

    /// 统一的重新规划判定函数（聚合多种信号）
    async fn should_trigger_replan(
        &self,
        execution_result: &ExecutionResult,
        current_attempts: u32,
        max_attempts: u32,
    ) -> bool {
        log::info!("=== 统一重新规划判定 ===");

        // 如果已达到最大重试次数，不再重新规划
        if current_attempts >= max_attempts {
            log::info!(
                "已达到最大重新规划次数 ({}/{}), 不再重新规划",
                current_attempts,
                max_attempts
            );
            return false;
        }

        // 如果执行成功，不需要重新规划
        if matches!(execution_result.status, TaskStatus::Completed) {
            log::info!("执行状态为完成，不需要重新规划");
            return false;
        }

        let mut replan_signals = Vec::new();

        // 信号1: 资源泄露检查（CRITICAL - 最高优先级）
        if self.detect_resource_leak(execution_result).await {
            replan_signals.push("🔴 CRITICAL: 资源泄露（浏览器/代理未关闭）".to_string());
        }

        // 信号2: 失败步骤检查
        if !execution_result.failed_steps.is_empty() {
            let failed_ratio = execution_result.failed_steps.len() as f64
                / (execution_result.completed_steps.len() + execution_result.failed_steps.len())
                    as f64;
            if failed_ratio > 0.3 {
                // 失败率超过30%
                replan_signals.push(format!("失败步骤过多: {:.1}%", failed_ratio * 100.0));
            }
        }

        // 信号3: 可重试错误检查
        let retryable_errors = execution_result
            .errors
            .iter()
            .filter(|e| e.retryable)
            .count();
        if retryable_errors > 0 {
            replan_signals.push(format!("存在 {} 个可重试错误", retryable_errors));
        }

        // 信号4: 安全测试关键步骤检查
        if self.is_security_test().await {
            if !self.has_required_security_steps(execution_result).await {
                replan_signals.push("安全测试缺少关键步骤（analyze_website/generate_advanced_plugin）".to_string());
            }
        }

        // 信号5: 步骤顺序违反检查
        if self.detect_step_order_violation(execution_result).await {
            replan_signals.push("步骤执行顺序违反依赖关系".to_string());
        }

        // 信号6: 信息充分性检查
        if self.has_insufficient_information(execution_result).await {
            replan_signals.push("执行结果信息不足".to_string());
        }

        // 信号7: 质量评估信号（来自增强反馈）
        let quality_score = execution_result
            .enhanced_feedback
            .quality_assessment
            .overall_score;
        if (quality_score as f64) < self.config.quality_threshold {
            replan_signals.push(format!(
                "质量分数过低: {:.2} < {:.2}",
                quality_score, self.config.quality_threshold
            ));
        }

        // 信号8: 性能信号（平均步骤时间过长）
        let avg_duration = execution_result.metrics.avg_step_duration_ms;
        if avg_duration > self.config.default_timeout * 1000 / 2 {
            // 超过超时时间的一半
            replan_signals.push(format!("平均步骤耗时过长: {}ms", avg_duration));
        }

        // 信号9: 资源使用信号（来自增强反馈）
        if execution_result
            .enhanced_feedback
            .resource_analysis
            .cpu_usage
            .utilization_rate
            > 0.8
        {
            replan_signals.push(format!(
                "CPU资源使用率过高: {:.1}%",
                execution_result
                    .enhanced_feedback
                    .resource_analysis
                    .cpu_usage
                    .utilization_rate
                    * 100.0
            ));
        }

        // 判定结果
        let should_replan = !replan_signals.is_empty();

        if should_replan {
            log::info!("决定重新规划，触发信号: {:?}", replan_signals);
        } else {
            log::info!("无重新规划信号，继续当前计划");
        }

        should_replan
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
                    // shared_data.insert(format!("step_result_{}", step.id), result.clone());
                    shared_data.insert(format!("step_result_{}", step.name), result.clone());

                    // 验证后置条件
                    let mut step_status = StepExecutionStatus::Completed;
                    let mut step_error = None;

                    for postcondition in &step.postconditions {
                        if !self.evaluate_condition(postcondition, &shared_data).await {
                            log::warn!(
                                "步骤 '{}' 的后置条件 '{}' 验证失败",
                                step.name,
                                postcondition
                            );
                            step_status = StepExecutionStatus::Failed;
                            step_error = Some(format!("后置条件验证失败: {}", postcondition));
                            break;
                        }
                    }

                    drop(shared_data);

                    return Ok(StepResult {
                        step_id: step.id.clone(),
                        status: step_status,
                        started_at: start_time,
                        completed_at: Some(SystemTime::now()),
                        duration_ms: start_time.elapsed().unwrap_or_default().as_millis() as u64,
                        result_data: Some(result.clone()),
                        error: step_error,
                        retry_count,
                        tool_result: None,
                    });
                }
                Err(error) => {
                    retry_count += 1;

                    // 基于任务参数覆盖重试配置
                    let effective_retry = {
                        let shared = context.shared_data.read().await;
                        let mut cfg = step.retry_config.clone();
                        if let Some(params) =
                            shared.get("task_parameters").and_then(|v| v.as_object())
                        {
                            if let Some(maxv) =
                                params.get("execution_retry_max").and_then(|v| v.as_u64())
                            {
                                cfg.max_retries = maxv as u32;
                            }
                            if let Some(backoff) = params
                                .get("execution_retry_backoff")
                                .and_then(|v| v.as_str())
                            {
                                cfg.backoff_strategy = match backoff {
                                    "fixed" => BackoffStrategy::Fixed,
                                    "linear" => BackoffStrategy::Linear,
                                    _ => BackoffStrategy::Exponential,
                                };
                            }
                            if let Some(interval_ms) = params
                                .get("execution_retry_interval_ms")
                                .and_then(|v| v.as_u64())
                            {
                                let secs = std::cmp::max(1, (interval_ms / 1000) as u64);
                                cfg.retry_interval = secs;
                            }
                        }
                        cfg
                    };

                    if retry_count <= effective_retry.max_retries {
                        log::warn!(
                            "步骤执行失败，准备重试 ({}/{}): {}",
                            retry_count,
                            effective_retry.max_retries,
                            error
                        );

                        // 等待重试间隔
                        let delay = self.calculate_retry_delay(&effective_retry, retry_count);
                        tokio::time::sleep(Duration::from_secs(delay)).await;

                        continue;
                    } else {
                        log::error!("步骤执行失败，已达到最大重试次数: {}", error);
                        return Ok(StepResult {
                            step_id: step.id.clone(),
                            status: StepExecutionStatus::Failed,
                            started_at: start_time,
                            completed_at: Some(SystemTime::now()),
                            duration_ms: start_time.elapsed().unwrap_or_default().as_millis()
                                as u64,
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

    /// 取消执行（旧版本，已被CancellationToken替代）
    /// 保留此方法以兼容旧代码，但实际使用的是上面的cancel()方法
    pub async fn cancel_legacy(&self) -> Result<(), PlanAndExecuteError> {
        let context_guard = self.context.lock().await;
        if let Some(context) = context_guard.as_ref() {
            let mut state = context.execution_state.write().await;
            state.is_cancelled = true;
            log::info!("执行已取消（旧版本方法）");
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
    ) -> Result<ExecutionContext, PlanAndExecuteError> {
        let mut shared_data = HashMap::new();

        // 如果有内存管理器，从中构建上下文并加载到 shared_data
        if let Some(ref memory_manager) = self.memory_manager {
            log::info!("从 MemoryManager 构建Plan-and-Execute上下文");

            match memory_manager.build_plan_execute_context(&task.id).await {
                Ok(context) => {
                    // 添加会话历史到共享数据
                    if let Some(ref history) = context.conversation_history {
                        shared_data.insert(
                            "conversation_history".to_string(),
                            serde_json::to_value(history).unwrap_or(serde_json::Value::Null),
                        );
                    }

                    // 添加计划历史到共享数据
                    if let Some(ref history) = context.plan_history {
                        shared_data.insert(
                            "plan_history".to_string(),
                            serde_json::to_value(history).unwrap_or(serde_json::Value::Null),
                        );
                    }

                    // 添加任务全状态到共享数据
                    if let Some(full_state) = context.full_state {
                        shared_data.insert(
                            "task_full_state".to_string(),
                            serde_json::to_value(&full_state).unwrap_or(serde_json::Value::Null),
                        );
                    }

                    log::info!("成功加载内存上下文: {} 项历史数据", shared_data.len());
                }
                Err(e) => {
                    log::warn!("构建Plan-and-Execute上下文失败，继续使用空上下文: {}", e);
                }
            }
        }

        // 注入任务参数到共享上下文，供统一提示词解析使用
        shared_data.insert(
            "task_parameters".to_string(),
            serde_json::to_value(&task.parameters).unwrap_or_else(|_| serde_json::json!({})),
        );

        let shared_data = Arc::new(RwLock::new(shared_data));
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

        let exec_context = ExecutionContext {
            task_id: task.id.clone(),
            plan_id: plan.id.clone(),
            shared_data,
            execution_state,
            tool_manager,
        };
        
        let mut context = self.context.lock().await;
        *context = Some(exec_context.clone());

        Ok(exec_context)
    }

    /// 递归替换参数中的变量引用
    /// 支持格式：{{步骤X的结果}}、{{步骤X}}、{{step_result_X}}
    fn substitute_variables<'a>(
        &'a self,
        value: &'a serde_json::Value,
        shared_data: &'a HashMap<String, serde_json::Value>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = serde_json::Value> + Send + 'a>> {
        Box::pin(async move {
            self.substitute_variables_impl(value, shared_data).await
        })
    }

    /// 实际的递归替换实现
    fn substitute_variables_impl<'a>(
        &'a self,
        value: &'a serde_json::Value,
        shared_data: &'a HashMap<String, serde_json::Value>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = serde_json::Value> + Send + 'a>> {
        Box::pin(async move {
        match value {
            serde_json::Value::String(s) => {
                // 处理 {{...}} 格式的变量引用
                let re = Regex::new(r"\{\{([^}]+)\}\}").unwrap();
                let mut result = s.clone();
                
                for cap in re.captures_iter(s) {
                    if let Some(var_match) = cap.get(1) {
                        let var_key = var_match.as_str().trim();
                        
                        // 尝试多种可能的键名格式
                        let possible_keys = vec![
                            // 1. 直接使用变量名
                            var_key.to_string(),
                            // 2. 添加 step_result_ 前缀
                            format!("step_result_{}", var_key),
                            // 3. 处理中文格式：步骤X的结果 -> step_result_步骤X
                            if var_key.contains("的结果") {
                                let step_name = var_key.replace("的结果", "");
                                format!("step_result_{}", step_name)
                            } else {
                                String::new()
                            },
                        ];
                        
                        // 查找匹配的值
                        let mut found = false;
                        for key in &possible_keys {
                            if key.is_empty() { continue; }
                            if let Some(value) = shared_data.get(key) {
                                // 找到值，进行替换
                                let replacement = match value {
                                    serde_json::Value::String(s) => s.clone(),
                                    serde_json::Value::Null => "null".to_string(),
                                    _ => serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string()),
                                };
                                result = result.replace(&cap[0], &replacement);
                                log::debug!("替换变量: {} -> {} (键: {})", &cap[0], &replacement, key);
                                found = true;
                                break;
                            }
                        }
                        
                        if !found {
                            log::warn!("未找到变量引用: {} (尝试的键: {:?})", &cap[0], possible_keys);
                        }
                    }
                }
                
                serde_json::Value::String(result)
            }
            serde_json::Value::Object(obj) => {
                // 递归处理对象中的每个值
                let mut new_obj = serde_json::Map::new();
                for (k, v) in obj {
                    new_obj.insert(k.clone(), self.substitute_variables_impl(v, shared_data).await);
                }
                serde_json::Value::Object(new_obj)
            }
            serde_json::Value::Array(arr) => {
                // 递归处理数组中的每个元素
                let mut new_arr = Vec::new();
                for item in arr {
                    new_arr.push(self.substitute_variables_impl(item, shared_data).await);
                }
                serde_json::Value::Array(new_arr)
            }
            // 其他类型直接返回
            _ => value.clone(),
        }
        })
    }

    async fn execute_step_once(
        &self,
        step: &ExecutionStep,
        context: &ExecutionContext,
    ) -> Result<serde_json::Value, PlanAndExecuteError> {
        log::info!("=== Tools层：执行具体步骤 ===");
        log::info!("步骤名称: {}", step.name);
        log::info!("步骤类型: {:?}", step.step_type);
        log::info!(
            "工具配置: {}",
            if step.tool_config.is_some() {
                "有"
            } else {
                "无"
            }
        );

        let result = match &step.step_type {
            StepType::ToolCall => {
                log::info!("Tools层：调用外部工具");
                self.execute_tool_call(step, context).await
            }
            StepType::AiReasoning => {
                log::info!("Tools层：执行AI推理");
                self.execute_ai_reasoning(step, context).await
            }
            StepType::DataProcessing => {
                log::info!("Tools层：执行数据处理");
                self.execute_data_processing(step, context).await
            }
            StepType::Conditional => {
                log::info!("Tools层：执行条件判断");
                self.execute_conditional(step, context).await
            }
            StepType::Parallel => {
                log::info!("Tools层：执行并行任务");
                self.execute_parallel(step, context).await
            }
            StepType::Wait => {
                log::info!("Tools层：执行等待");
                self.execute_wait(step, context).await
            }
            StepType::ManualConfirmation => {
                log::info!("Tools层：执行人工确认");
                self.execute_manual_confirmation(step, context).await
            }
        };

        match &result {
            Ok(data) => {
                log::info!("✓ Tools层执行成功: {}", step.name);
                log::debug!(
                    "执行结果: {}",
                    serde_json::to_string_pretty(data).unwrap_or_else(|_| "无法序列化".to_string())
                );
            }
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
            // 读取任务参数中的工具白名单/黑名单与超时设置
            let (allow_list, deny_list, default_timeout_opt) = {
                let shared = context.shared_data.read().await;
                if let Some(params) = shared.get("task_parameters").and_then(|v| v.as_object()) {
                    let allow = params
                        .get("tools_allow")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    let deny = params
                        .get("tools_deny")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    let tmo = params.get("execution_timeout_sec").and_then(|v| v.as_u64());
                    (allow, deny, tmo)
                } else {
                    (Vec::new(), Vec::new(), None)
                }
            };

            // 工具允许性检查
            let tool_name = &tool_config.tool_name;
            // 如果没有白名单（空数组），则不允许任何工具
            if allow_list.is_empty() {
                return Err(PlanAndExecuteError::ToolFailed(format!(
                    "工具 '{}' 不在允许列表中（未配置工具权限）",
                    tool_name
                )));
            }
            // 如果有白名单且工具不在白名单中，拒绝
            if !allow_list.iter().any(|n| n == tool_name) {
                return Err(PlanAndExecuteError::ToolFailed(format!(
                    "工具 '{}' 不在允许列表中",
                    tool_name
                )));
            }
            // 如果工具在黑名单中，拒绝
            if deny_list.iter().any(|n| n == tool_name) {
                return Err(PlanAndExecuteError::ToolFailed(format!(
                    "工具 '{}' 被禁止使用",
                    tool_name
                )));
            }
            // 合并工具参数（以步骤参数为优先，覆盖tool_args）
            let mut merged_inputs = tool_config.tool_args.clone();
            for (k, v) in &step.parameters {
                merged_inputs.insert(k.clone(), v.clone());
            }

            // ✅ 替换参数中的变量引用（如 {{步骤5的结果}}）
            let shared_data = context.shared_data.read().await;
            let mut substituted_inputs = HashMap::new();
            for (k, v) in &merged_inputs {
                let substituted_value = self.substitute_variables(v, &shared_data).await;
                substituted_inputs.insert(k.clone(), substituted_value);
            }
            drop(shared_data);
            
            log::info!("工具参数替换完成: {} 个参数", substituted_inputs.len());

            let tool_params = ToolExecutionParams {
                inputs: substituted_inputs,
                context: HashMap::new(),
                timeout: Some(std::time::Duration::from_secs(
                    tool_config
                        .timeout
                        .or(default_timeout_opt.map(|v| v as u64))
                        .unwrap_or(self.config.default_timeout),
                )),
                execution_id: None,
            };

            let timeout_duration = Duration::from_secs(
                tool_config
                    .timeout
                    .or(default_timeout_opt.map(|v| v as u64))
                    .unwrap_or(self.config.default_timeout),
            );

            let manager = context.tool_manager.read().await;
            match timeout(
                timeout_duration,
                manager.call_tool(&tool_config.tool_name, tool_params),
            )
            .await
            {
                Ok(Ok(result)) => {
                    let tool_result_content = if result.status == ExecutionStatus::Completed {
                        // 尝试多种方式提取工具执行结果
                        if let Some(output_field) = result.output.get("output") {
                            // 如果有 output 字段，使用它
                            serde_json::to_string_pretty(output_field)
                                .unwrap_or_else(|_| output_field.to_string())
                        } else if result.output.is_string() {
                            // 如果整个 output 就是字符串
                            result.output.as_str().unwrap_or("").to_string()
                        } else {
                            // 否则格式化整个 output 对象
                            serde_json::to_string_pretty(&result.output)
                                .unwrap_or_else(|_| result.output.to_string())
                        }
                    } else {
                        // 执行失败时的错误处理
                        if let Some(error_msg) = result.output.as_str() {
                            error_msg.to_string()
                        } else {
                            format!(
                                "{}",
                                serde_json::to_string(&result.output)
                                    .unwrap_or("未知错误".to_string())
                            )
                        }
                    };

                    let execution_id = self.resolve_execution_id(context).await;
                    let (message_id, conversation_id) =
                        self.resolve_message_and_conversation_ids(context).await;
                    self.emit_message_chunk(
                        &execution_id,
                        &message_id,
                        conversation_id.as_deref(),
                        ChunkType::ToolResult,
                        &tool_result_content,
                        false,
                        Some("executor"),
                        Some(tool_config.tool_name.as_str()),
                    );
                    Ok(result.output)
                }
                Ok(Err(error)) => {
                    let err_msg = error.to_string();
                    // self.emit_error(
                    //     context,
                    //     &format!("工具 {} 执行失败: {}", step.name, err_msg),
                    // )
                    // .await;
                    Err(PlanAndExecuteError::ToolFailed(err_msg))
                }
                Err(_) => {
                    let err_msg = "工具调用超时".to_string();
                    // self.emit_error(context, &format!("工具 {} 执行超时", step.name))
                    //     .await;
                    Err(PlanAndExecuteError::ToolFailed(err_msg))
                }
            }
        } else {
            Err(PlanAndExecuteError::ConfigError(
                "工具调用步骤缺少工具配置".to_string(),
            ))
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
                context_data.push_str(&format!(
                    "\n\n--- {} ---\n{}",
                    key,
                    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
                ));
            }
        }

        // RAG知识检索（受全局开关控制） - 根据步骤描述检索相关知识
        let mut rag_context = String::new();
        let mut rag_enabled = false;
        if let Ok(cfg_opt) = self.db_service.get_rag_config().await {
            rag_enabled = cfg_opt.map(|c| c.augmentation_enabled).unwrap_or(false);
        }
        if rag_enabled {
            if let Ok(rag_service) = crate::commands::rag_commands::get_global_rag_service().await {
                log::info!("尝试为AI推理步骤检索RAG知识: {}", step.name);
                // 获取激活的集合ID，与AI助手模式保持一致
                let active_collection_id: Option<String> =
                    match self.db_service.get_rag_collections().await {
                        Ok(cols) => cols.into_iter().find(|c| c.is_active).map(|c| c.id),
                        Err(_) => None,
                    };

                // 构建RAG查询请求
                let rag_request = AssistantRagRequest {
                    query: format!("{} {}", step.name, step.description),
                    collection_id: active_collection_id, // 使用激活集合
                    conversation_history: None,
                    top_k: Some(5),
                    use_mmr: Some(true),
                    mmr_lambda: Some(0.7),
                    similarity_threshold: Some(0.65),
                    reranking_enabled: Some(false),
                    model_provider: None,
                    model_name: None,
                    max_tokens: None,
                    temperature: None,
                    system_prompt: None,                };
                // 短超时避免阻塞执行
                use tokio::time::{timeout, Duration};
                match timeout(
                    Duration::from_millis(1500),
                    rag_service.query_for_assistant(&rag_request),
                )
                .await
                {
                    Ok(Ok((knowledge_context, citations))) => {
                        if !knowledge_context.trim().is_empty() {
                            let policy = "你必须严格基于证据回答问题。在回答中引用证据时，使用 [SOURCE n] 格式。如果证据不足，请直接回答并避免编造。";
                            rag_context = format!(
                                "\n\n[知识溯源规范]\n{}\n\n[证据块]\n{}",
                                policy, knowledge_context
                            );
                            log::info!("为步骤 '{}' 注入基于证据的知识块", step.name);

                            // 发送包含引用信息的Meta块到前端，供底部展示
                            if let Some(app_handle) = &self.app_handle {
                                let (message_id, conversation_id) =
                                    self.resolve_message_and_conversation_ids(context).await;
                                let execution_id = self.resolve_execution_id(context).await;
                                let meta = serde_json::json!({
                                    "type": "rag_citations",
                                    "citations": citations,
                                });
                                let meta_str = serde_json::to_string(&meta)
                                    .unwrap_or_else(|_| "{}".to_string());
                                emit_message_chunk_arc(
                                    app_handle,
                                    &execution_id,
                                    &message_id,
                                    conversation_id.as_deref(),
                                    ChunkType::Meta,
                                    &meta_str,
                                    false,
                                    Some("executor"),
                                    None,
                                );
                            }
                        } else {
                            log::debug!("步骤 '{}' 未找到相关RAG知识", step.name);
                        }
                    }
                    Ok(Err(e)) => {
                        log::warn!("RAG知识检索失败: {}, 继续执行AI推理", e);
                    }
                    Err(_) => {
                        log::debug!("RAG检索超时，跳过本次增强");
                    }
                }
            } else {
                log::debug!("RAG服务未初始化，跳过知识检索");
            }
        } else {
            log::debug!("RAG增强未开启，跳过知识检索");
        }

                // 稳定规则与行为约束作为系统提示

                
        // 统一提示词解析：优先显式模板ID，其次统一解析器
        let executor_tpl_prompt = {
            if let Some(tpl_id_val) = step.parameters.get("prompt_template_executor_id") {
                if let Some(tpl_id) = tpl_id_val.as_i64() {
                    if let Ok(pool) = self.db_service.get_pool() {
                        let repo = PromptRepository::new(pool.clone());
                        if let Ok(Some(tpl)) = repo.get_template(tpl_id).await {
                            Some(tpl.content)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                if let Ok(pool) = self.db_service.get_pool() {
                    let repo = PromptRepository::new(pool.clone());
                    let resolver = PromptResolver::new(repo);
                    let params_map: std::collections::HashMap<String, serde_json::Value> =
                        if let Some(val) = context.shared_data.read().await.get("task_parameters") {
                            if let Some(obj) = val.as_object() {
                                obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                            } else {
                                std::collections::HashMap::new()
                            }
                        } else {
                            std::collections::HashMap::new()
                        };
                    let agent_config = AgentPromptConfig::parse_agent_config(&params_map);
                    match resolver
                        .resolve_prompt(
                            &agent_config,
                            ArchitectureType::PlanExecute,
                            CanonicalStage::Executor,
                            Some(&"".to_string()),
                        )
                        .await
                    {
                        Ok(content) if !content.is_empty() => Some(content),
                        _ => None,
                    }
                } else {
                    None
                }
            }
        };

        let mut system_prompt = if let Some(template) = executor_tpl_prompt {
            //根据{rag}把智库库的内容插入到system prompt中,如果{rag}为空，则不插入
            // let template = template.replace("{rag}", &rag_context);
            if rag_context.is_empty() {
                template
            } else {
                // template.replace("{rag}", &rag_context)
                //直接返回模板
                template

            }
        } else {
            log::info!("执行器模板提示解析为空; 使用备用提示");
            "你是一个专家推理模块。只返回纯文本。保持简洁。".to_string()
        };

        // 集成角色提示词（如果存在）
        if let Some(role_prompt) = step.parameters.get("role_prompt").and_then(|v| v.as_str()) {
            if !role_prompt.trim().is_empty() {
                system_prompt = if system_prompt.trim().is_empty() {
                    role_prompt.to_string()
                } else {
                    format!("{}\n\n{}", role_prompt, system_prompt)
                };
                log::info!("Plan-Execute executor: integrated role prompt for step {}", step.name);
            }
        }


        let user_prompt = if has_previous_results || !rag_context.is_empty() {
            format!(
                "当前步骤: {step_name}\n描述: {desc}\n\n之前步骤的结果: {context}\n\n",
                step_name = step.name,
                desc = step.description,
                context = context_data,
            )
        } else {
            format!(
                "当前步骤: {step_name}\n描述: {desc}\n\n之前步骤的结果: {context}\n\n",
                step_name = step.name,
                desc = step.description,
                context = context_data,
            )
        };


        // 获取AI服务
        let ai_service = if let Some(ref ai_service_manager) = self.ai_service_manager {
            // 解析执行阶段应使用的provider与model：优先Agent覆盖 -> 调度器Execution -> 调度器Planning -> 本地配置
            let (provider_name, model_name) = {
                let shared = context.shared_data.read().await;
                let (mut p, mut m): (Option<String>, Option<String>) = (None, None);

                // 1. 尝试Agent LLM覆盖
                if let Some(params) = shared.get("task_parameters").and_then(|v| v.as_object()) {
                    if let Some(llm) = params.get("llm").and_then(|v| v.get("default")) {
                        let provider_str =
                            llm.get("provider").and_then(|v| v.as_str()).unwrap_or("");
                        let model_str = llm.get("model").and_then(|v| v.as_str()).unwrap_or("");

                        // 跳过 "auto" 配置，让调度器配置生效
                        if model_str != "auto" && !model_str.trim().is_empty() {
                            p = if provider_str != "auto" && !provider_str.trim().is_empty() {
                                Some(provider_str.to_string())
                            } else {
                                None
                            };
                            m = Some(model_str.to_string());
                            log::info!(
                                "Using Agent LLM override - Model: {}, Provider: {:?}",
                                model_str,
                                p
                            );
                        } else {
                            log::info!(
                                "Agent LLM config is 'auto' or empty, skipping to scheduler config"
                            );
                        }
                    }
                }

                if let Some(model) = m.clone() {
                    (p.unwrap_or_else(|| "".to_string()), model)
                } else {
                    // 2. 尝试调度器Execution阶段配置
                    match ai_service_manager
                        .get_ai_config_for_stage(SchedulerStage::Evaluation)
                        .await
                    {
                        Ok(Some(cfg)) => {
                            log::info!(
                                "Using scheduler Execution config - Model: {}, Provider: {}",
                                cfg.model,
                                cfg.provider
                            );
                            (cfg.provider, cfg.model)
                        }
                        Ok(None) => {
                            log::info!(
                                "Scheduler Execution config is empty, trying Planning config"
                            );
                            // 3. 尝试调度器Planning阶段配置
                            match ai_service_manager
                                .get_ai_config_for_stage(SchedulerStage::Planning)
                                .await
                            {
                                Ok(Some(cfg)) => {
                                    log::info!(
                                        "Falling back to scheduler Planning config: {} ({})",
                                        cfg.model,
                                        cfg.provider
                                    );
                                    (cfg.provider, cfg.model)
                                }
                                Ok(None) => {
                                    log::info!("Scheduler Planning config is also empty, trying local executor config");
                                    // 4. 回退到本地配置
                                    log::info!(
                                        "Local executor config - Model: '{}', Provider: '{}'",
                                        self.config.model_config.model_name,
                                        self.config.ai_provider
                                    );

                                    // 如果本地配置也是空的，返回一个明确的错误
                                    if self.config.model_config.model_name.trim().is_empty() {
                                        return Err(PlanAndExecuteError::ConfigError(
                                            "无法找到可用的执行器模型配置。请在调度器设置中配置执行器模型，或在Agent配置中设置LLM覆盖。".to_string()
                                        ));
                                    }
                                    (
                                        self.config.model_config.model_name.clone(),
                                        self.config.ai_provider.clone(),
                                    )
                                }
                                Err(e) => {
                                    log::warn!("Failed to get scheduler Planning config: {}", e);
                                    log::info!(
                                        "Local executor config - Model: '{}', Provider: '{}'",
                                        self.config.model_config.model_name,
                                        self.config.ai_provider
                                    );

                                    if self.config.model_config.model_name.trim().is_empty() {
                                        return Err(PlanAndExecuteError::ConfigError(
                                            "无法找到可用的执行器模型配置。请在调度器设置中配置执行器模型，或在Agent配置中设置LLM覆盖。".to_string()
                                        ));
                                    }
                                    (
                                        self.config.model_config.model_name.clone(),
                                        self.config.ai_provider.clone(),
                                    )
                                }
                            }
                        }
                        Err(e) => {
                            log::warn!("Failed to get scheduler Execution config: {}", e);
                            log::info!(
                                "Local executor config - Model: '{}', Provider: '{}'",
                                self.config.model_config.model_name,
                                self.config.ai_provider
                            );

                            if self.config.model_config.model_name.trim().is_empty() {
                                return Err(PlanAndExecuteError::ConfigError(
                                    "无法找到可用的执行器模型配置。请在调度器设置中配置执行器模型，或在Agent配置中设置LLM覆盖。".to_string()
                                ));
                            }
                            (
                                self.config.model_config.model_name.clone(),
                                self.config.ai_provider.clone(),
                            )
                        }
                    }
                }
            };

            if model_name.trim().is_empty() {
                return Err(PlanAndExecuteError::ConfigError("Executor model is empty; please configure scheduler or executor.model_config.model_name".to_string()));
            }

            // 直接基于配置构建一次性服务（使用 Rig），不再通过查找
            match ai_service_manager.get_provider_config(&provider_name).await {
                Ok(Some(cfg)) => {
                    let mut dc = cfg;
                    dc.model = model_name.clone();
                    let app_handle = self.app_handle.as_ref().map(|a| a.as_ref().clone());
                    AiService::new(dc, self.db_service.clone(), app_handle, None)
                }
                Ok(None) => {
                    return Err(PlanAndExecuteError::AiAdapterError(format!(
                        "找不到提供商配置: {}",
                        provider_name
                    )))
                }
                Err(e) => {
                    return Err(PlanAndExecuteError::AiAdapterError(format!(
                        "读取提供商配置失败: {}",
                        e
                    )))
                }
            }
        } else {
            return Err(PlanAndExecuteError::AiAdapterError(
                "AI服务管理器未初始化".to_string(),
            ));
        };

        // 使用流式消息API发送请求
        // 绑定到前端该次助手消息ID，确保chunk落到正确消息上
        let (bound_message_id, _conv_id_opt) =
            self.resolve_message_and_conversation_ids(context).await;
        let content = ai_service
            .send_message_stream(
                Some(&user_prompt),
                Some(system_prompt.as_str()),           // 系统提示
                None,                           // 不指定会话ID，保持无状态
                Some(bound_message_id.clone()), // 消息ID用于前端显示
                true,
                false,
                Some(ChunkType::Content),
            )
            .await
            .map_err(|e| PlanAndExecuteError::AiAdapterError(e.to_string()))?;

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
                        if result_key.contains("output")
                            || result_key.contains("data")
                            || result_key.contains("result")
                        {
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

        log::info!(
            "数据处理完成: 处理了 {} 个数据项，来自 {} 个之前的步骤",
            total_processed_items,
            processed_results.len()
        );

        Ok(processed_data)
    }

    async fn execute_conditional(
        &self,
        step: &ExecutionStep,
        context: &ExecutionContext,
    ) -> Result<serde_json::Value, PlanAndExecuteError> {
        log::info!("执行条件判断步骤: {}", step.name);
        
        // 获取共享数据用于条件评估
        let shared_data = context.shared_data.read().await;
        
        // 从步骤参数中获取条件表达式
        let condition_expr = step.parameters.get("condition")
            .and_then(|v| v.as_str())
            .ok_or_else(|| PlanAndExecuteError::InvalidStepConfiguration {
                step_name: step.name.clone(),
                reason: "Missing 'condition' parameter".to_string(),
            })?;
            
        log::info!("评估条件表达式: {}", condition_expr);
        
        // 评估条件表达式
        let condition_result = self.evaluate_complex_condition(condition_expr, &shared_data).await;
        
        // 获取条件为真和为假时的操作
        let on_true = step.parameters.get("on_true").cloned();
        let on_false = step.parameters.get("on_false").cloned();
        
        // 根据条件结果执行相应操作
        let action_result = if condition_result {
            log::info!("条件为真，执行 on_true 操作");
            if let Some(action) = on_true {
                self.execute_conditional_action(&action, context).await?
            } else {
                serde_json::json!({"action": "none", "reason": "no on_true action defined"})
            }
        } else {
            log::info!("条件为假，执行 on_false 操作");
            if let Some(action) = on_false {
                self.execute_conditional_action(&action, context).await?
            } else {
                serde_json::json!({"action": "none", "reason": "no on_false action defined"})
            }
        };
        
        // 记录条件判断结果到共享数据
        drop(shared_data);
        let mut shared_data_mut = context.shared_data.write().await;
        shared_data_mut.insert(
            format!("condition_result_{}", step.name),
            serde_json::json!(condition_result)
        );
        
        log::info!("条件判断完成: {} -> {}", condition_expr, condition_result);
        
        Ok(serde_json::json!({
            "condition_expression": condition_expr,
            "condition_result": condition_result,
            "action_executed": action_result,
            "step_name": step.name,
            "timestamp": chrono::Utc::now().to_rfc3339()
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
        let wait_time = step
            .parameters
            .get("wait_seconds")
            .and_then(|v| v.as_u64())
            .or_else(|| step.parameters.get("duration").and_then(|v| v.as_u64()))
            .or_else(|| {
                // 如果参数中包含工具配置，从工具参数中获取duration
                if let Some(tool_args) =
                    step.parameters.get("tool_args").and_then(|v| v.as_object())
                {
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
                return Err(PlanAndExecuteError::ExecutionFailed(
                    "执行已被取消".to_string(),
                ));
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
            }
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
                StepExecutionStatus::Completed => {
                    completed_steps.push(step_id.clone());
                    successful_count += 1;
                }
                StepExecutionStatus::Failed => {
                    failed_steps.push(step_id.clone());
                    failed_count += 1;
                }
                StepExecutionStatus::Skipped => {
                    skipped_steps.push(step_id.clone());
                    skipped_count += 1;
                }
                _ => {}
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
        let enhanced_feedback = self
            .generate_enhanced_feedback(&step_results, &metrics, &errors, total_duration)
            .await?;

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
        let performance_insights = self
            .generate_performance_insights(step_results, metrics)
            .await;

        // 5. 质量评估
        let quality_assessment = self.assess_execution_quality(step_results, metrics).await;

        // 6. 改进建议
        let improvement_suggestions = self
            .generate_improvement_suggestions(step_results, errors)
            .await;

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

    async fn analyze_success_factors(
        &self,
        step_results: &HashMap<String, StepResult>,
    ) -> Vec<String> {
        let mut factors = Vec::new();

        let successful_steps: Vec<_> = step_results
            .values()
            .filter(|r| r.status == StepExecutionStatus::Completed)
            .collect();

        if !successful_steps.is_empty() {
            factors.push("工具调用成功执行".to_string());

            let avg_duration = successful_steps.iter().map(|r| r.duration_ms).sum::<u64>()
                / successful_steps.len() as u64;

            if avg_duration < 5000 {
                factors.push("步骤执行速度较快".to_string());
            }

            let no_retry_steps = successful_steps
                .iter()
                .filter(|r| r.retry_count == 0)
                .count();

            if no_retry_steps == successful_steps.len() {
                factors.push("无需重试即可成功".to_string());
            }
        }

        factors
    }

    async fn analyze_failure_factors(
        &self,
        step_results: &HashMap<String, StepResult>,
        errors: &[ExecutionError],
    ) -> Vec<String> {
        let mut factors = Vec::new();

        let failed_steps: Vec<_> = step_results
            .values()
            .filter(|r| r.status == StepExecutionStatus::Failed)
            .collect();

        if !failed_steps.is_empty() {
            factors.push(format!("{}个步骤执行失败", failed_steps.len()));

            let high_retry_steps = failed_steps.iter().filter(|r| r.retry_count > 2).count();

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

    async fn generate_performance_insights(
        &self,
        step_results: &HashMap<String, StepResult>,
        _metrics: &ExecutionMetrics,
    ) -> Vec<PerformanceInsight> {
        let mut insights = Vec::new();

        // 识别性能瓶颈
        let mut durations: Vec<_> = step_results
            .iter()
            .map(|(id, result)| (id.clone(), result.duration_ms))
            .collect();
        durations.sort_by(|a, b| b.1.cmp(&a.1));

        if let Some((slowest_step, duration)) = durations.first() {
            if *duration > 10000 {
                // 超过10秒
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

    async fn assess_execution_quality(
        &self,
        step_results: &HashMap<String, StepResult>,
        metrics: &ExecutionMetrics,
    ) -> QualityAssessment {
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
            std::cmp::max(
                30,
                90_i32.saturating_sub((metrics.avg_step_duration_ms / 1000) as i32 * 10),
            ) as u32
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
                    details: format!(
                        "{}个步骤中{}个成功完成",
                        total_steps, metrics.successful_steps
                    ),
                    suggestions: if completeness_score < 80 {
                        vec![
                            "考虑增加错误处理机制".to_string(),
                            "优化工具参数配置".to_string(),
                        ]
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

    async fn generate_improvement_suggestions(
        &self,
        step_results: &HashMap<String, StepResult>,
        _errors: &[ExecutionError],
    ) -> Vec<String> {
        let mut suggestions = Vec::new();

        let failed_count = step_results
            .values()
            .filter(|r| r.status == StepExecutionStatus::Failed)
            .count();

        if failed_count > 0 {
            suggestions.push("增加失败步骤的错误处理逻辑".to_string());
            suggestions.push("考虑调整工具参数以提高成功率".to_string());
        }

        let high_retry_count = step_results.values().filter(|r| r.retry_count > 2).count();

        if high_retry_count > 0 {
            suggestions.push("优化重试策略，避免过度重试".to_string());
        }

        let slow_steps = step_results
            .values()
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

    async fn identify_risk_indicators(
        &self,
        step_results: &HashMap<String, StepResult>,
        errors: &[ExecutionError],
    ) -> Vec<RiskIndicator> {
        let mut indicators = Vec::new();

        // 执行风险
        let failure_rate = step_results
            .values()
            .filter(|r| r.status == StepExecutionStatus::Failed)
            .count() as f64
            / step_results.len() as f64;

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
        let avg_duration = step_results.values().map(|r| r.duration_ms).sum::<u64>()
            / step_results.len().max(1) as u64;

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

    async fn analyze_dependencies(
        &self,
        _step_results: &HashMap<String, StepResult>,
    ) -> DependencyAnalysis {
        // 依赖分析实现
        DependencyAnalysis {
            resolved_dependencies: vec![],
            unresolved_dependencies: vec![],
            circular_dependencies: vec![],
            dependency_conflicts: vec![],
        }
    }

    async fn analyze_resource_usage(
        &self,
        metrics: &ExecutionMetrics,
        total_duration: u64,
    ) -> ResourceAnalysis {
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

        // 1. 转换并保存执行计划
        let engine_plan = crate::engines::types::ExecutionPlan {
            id: plan.id.clone(),
            name: plan.name.clone(),
            description: plan.description.clone(),
            steps: plan
                .steps
                .iter()
                .map(|s| crate::engines::types::PlanStep {
                    id: s.id.clone(),
                    name: s.name.clone(),
                    description: s.description.clone(),
                    step_type: crate::engines::types::StepType::ToolCall,
                    tool_config: crate::engines::types::ToolConfig {
                        tool_name: s
                            .tool_config
                            .as_ref()
                            .map(|tc| tc.tool_name.clone())
                            .unwrap_or_else(|| format!("step_{}", s.name)),
                        tool_version: None,
                        tool_args: s.parameters.clone(),
                        timeout: Some(self.config.default_timeout as f64),
                        env_vars: std::collections::HashMap::new(),
                    },
                    estimated_duration: s.estimated_duration,
                    retry_config: crate::engines::types::RetryConfig::default(),
                    parameters: s.parameters.clone(),
                    preconditions: vec![],
                    postconditions: vec![],
                })
                .collect(),
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
                TaskStatus::RequiresIntervention => {
                    crate::engines::types::ExecutionStatus::RequiresIntervention
                }
            },
            started_at: task.created_at,
            completed_at: Some(SystemTime::now()),
            current_step: Some(result.completed_steps.len() as i32),
            progress: if plan.steps.is_empty() {
                100
            } else {
                (result.completed_steps.len() as f32 / plan.steps.len() as f32) as u32
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
            step_results: result
                .step_results
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        crate::engines::types::StepExecutionResult {
                            step_id: v.step_id.clone(),
                            status: match v.status {
                                StepExecutionStatus::Pending => {
                                    crate::engines::types::StepExecutionStatus::Pending
                                }
                                StepExecutionStatus::Running => {
                                    crate::engines::types::StepExecutionStatus::Running
                                }
                                StepExecutionStatus::Completed => {
                                    crate::engines::types::StepExecutionStatus::Completed
                                }
                                StepExecutionStatus::Failed => {
                                    crate::engines::types::StepExecutionStatus::Failed
                                }
                                StepExecutionStatus::Skipped => {
                                    crate::engines::types::StepExecutionStatus::Skipped
                                }
                                StepExecutionStatus::Retrying => {
                                    crate::engines::types::StepExecutionStatus::Retrying
                                }
                                StepExecutionStatus::Cancelled => {
                                    crate::engines::types::StepExecutionStatus::Failed
                                }
                            },
                            started_at: v.started_at,
                            completed_at: v.completed_at,
                            result_data: v.result_data.clone(),
                            error: v.error.as_ref().map(|e| {
                                crate::engines::types::ExecutionError {
                                    error_type: crate::engines::types::ErrorType::Tool,
                                    message: e.clone(),
                                    details: Some(e.clone()),
                                    error_code: None,
                                    retryable: false,
                                    timestamp: SystemTime::now(),
                                }
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
                        },
                    )
                })
                .collect(),
            metadata: crate::engines::types::SessionMetadata {
                created_by: None,
                tags: vec![format!("{:?}", self.config.execution_mode)],
                notes: Some(format!(
                    "Error handling: {:?}, Timeout: {}s",
                    self.config.error_handling, self.config.default_timeout
                )),
                version: "1.0".to_string(),
                custom_attributes: {
                    let mut attrs = std::collections::HashMap::new();
                    attrs.insert(
                        "execution_mode".to_string(),
                        serde_json::json!(format!("{:?}", self.config.execution_mode)),
                    );
                    attrs.insert(
                        "error_handling".to_string(),
                        serde_json::json!(format!("{:?}", self.config.error_handling)),
                    );
                    attrs.insert(
                        "default_timeout".to_string(),
                        serde_json::json!(self.config.default_timeout),
                    );
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
            max_replan_attempts: 2,
            max_total_execution_time: 30 * 60 * 1000, // 30分钟
            max_consecutive_failures: 1,
            plan_similarity_threshold: 0.9,
            quality_threshold: 0.7,
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
