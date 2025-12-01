pub mod traits;
pub mod types;
pub mod llm_client;

pub mod memory;

// ReWOO 架构模块
pub mod rewoo;
// LLMCompiler 架构模块
pub mod llm_compiler;

pub mod plan_and_execute;

// ReAct 架构模块
pub mod react;

// Orchestrator 架构模块已删除,使用Travel替代

// Travel 架构模块 (OODA循环)
pub mod travel;

// 智能调度器与工作流引擎
pub mod intelligent_dispatcher;

// 重新导出核心类型和trait
pub use types::*;

// 导出公共 LLM 客户端
pub use llm_client::{
    LlmConfig, LlmClient, StreamingLlmClient, StreamContent,
    create_llm_config, create_client, create_streaming_client,
    // 向后兼容
    SimpleLlmClient, create_simple_client
};

pub use types::{
    ErrorType, ExecutionContext, ExecutionError, ExecutionMetrics, ExecutionPlan, ExecutionSession,
    StepExecutionResult,
};

// 导出 ReAct 核心类型
pub use react::{ReactConfig, ReactEngine, ReactStatus, ReactTrace};

// 导出 Travel 核心类型
pub use travel::{TravelConfig, TravelEngine, OodaCycle, OodaPhase, TaskComplexity};
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
