pub mod traits;
pub mod types;

pub mod memory;

// ReAct 架构模块（泛化版本，统一引擎）
// 包含所有任务类型支持，任务特性通过 Prompt 配置
pub mod react;

// 视觉探索引擎 (VLM驱动的网站全流量发现)
pub mod vision_explorer;

// 智能调度器与工作流引擎
pub mod intelligent_dispatcher;

// 重新导出核心类型和trait
pub use types::*;

// 重新导出 sentinel_llm 的 LLM 客户端
pub use sentinel_llm::{
    LlmConfig, LlmClient, StreamingLlmClient, StreamContent,
    ChatMessage, ImageAttachment, Message,
    create_llm_config as create_llm_config_raw, create_client as create_client_raw,
    create_streaming_client as create_streaming_client_raw,
};

use crate::services::ai::AiService;
use std::sync::Arc;

/// 从 AiService 创建 LlmConfig
pub fn create_llm_config(ai_service: &AiService) -> LlmConfig {
    let config = ai_service.get_config();
    LlmConfig::new(&config.provider, &config.model)
        .with_api_key(config.api_key.clone().unwrap_or_default())
        .with_base_url(config.api_base.clone().unwrap_or_default())
        .with_timeout(120)
}

/// 从 Arc<AiService> 创建 LlmConfig
pub fn create_llm_config_from_arc(ai_service: &Arc<AiService>) -> LlmConfig {
    create_llm_config(ai_service.as_ref())
}

/// 从 AiService 创建 LlmClient
pub fn create_client(ai_service: &AiService) -> LlmClient {
    LlmClient::new(create_llm_config(ai_service))
}

/// 从 AiService 创建 StreamingLlmClient
pub fn create_streaming_client(ai_service: &AiService) -> StreamingLlmClient {
    StreamingLlmClient::new(create_llm_config(ai_service))
}

pub use types::{
    ErrorType, ExecutionContext, ExecutionError, ExecutionMetrics, ExecutionPlan, ExecutionSession,
    StepExecutionResult,
};

// 导出 ReAct 核心类型（统一引擎）
pub use react::{ReactConfig, ReactEngine, ReactStatus, ReactTrace};

// 导出 VisionExplorer 核心类型
pub use vision_explorer::{
    VisionExplorer, VisionExplorerConfig, ExplorationState, ExplorationStatus,
    ExplorationSummary, BrowserAction, ApiEndpoint,
};
// // 导出prompt相关模块
// pub use prompt_config::{
//     PromptConfigManager, PromptConfig, AgentProfile, DomainTemplate,
//     CoreTemplates, CustomTemplate, ABTestConfig, OptimalConfig
// };
// pub use prompt_builder::{
//     PromptBuilder, CompiledTemplate, VariableResolver, PromptBuildContext,
//     TargetInfo, AuthInfo, ErrorInfo, HistoryItem,
//     ToolInfo, ResourceRequirements, PromptBuildResult, BuildStats,
//     ValidationResult as PromptValidationResult
// };
// pub use prompt_template_manager::{
//     PromptTemplateManager, CachedTemplate, TemplateVersionManager,
//     TemplateVersion, TemplateManagerConfig, ValidationRules,
//     TemplateSearchResult, TemplateStats
// };
// pub use prompt_ab_test_manager::{
//     PromptABTestManager, ABTest, TestVariant, TrafficAllocation,
//     EvaluationMetric, TestConditions, TestExecution, TestAnalysis,
//     TestResultsStorage, StatisticalAnalyzer, CreateTestRequest,
//     AllocationStrategy, MetricType, CalculationMethod
// };
// pub use prompt_optimizer::{
//     PromptOptimizer, OptimizationStrategy, OptimizerConfig, OptimizationTarget,
//     TargetType, OptimizationDirection, PerformanceRecord, UserFeedback,
//     SystemMetrics, TokenUsage, OptimizationSuggestion, SuggestionType,
//     Complexity, RiskAssessment, RiskLevel, PromptChange, ChangeType,
//     OptimizationContext, ResourceConstraints, OptimizationResult,
//     ValidationResults, PerformanceComparison
// };

// 导出智能调度器模块
pub use intelligent_dispatcher::workflow_engine::{
    WorkflowEngine, WorkflowDefinition, WorkflowMetadata, WorkflowStep, ExecutionStatus,
};
