pub mod types;
pub mod traits;

pub mod memory;

// ReWOO 架构模块
pub mod rewoo;
// LLMCompiler 架构模块
pub mod llm_compiler;

pub mod plan_and_execute;

// ReAct 架构模块
pub mod react;


// 重新导出核心类型和trait
pub use types::*;

pub use types::{ExecutionPlan, ExecutionContext, ExecutionMetrics, ExecutionSession, StepExecutionResult, ExecutionError, ErrorType};

// 导出 ReAct 核心类型
pub use react::{ReactEngine, ReactConfig, ReactTrace, ReactStatus};
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
// pub use intelligent_dispatcher::{
//     IntelligentDispatcher, QueryFeatures, ArchitectureSelection, ArchitectureConfig, 
//     ResourceConfig, DynamicPrompts, ExecutionRecord, ExecutionStatus, ExecutionHistoryResult,
//     ExecutionHistoryRecord, DispatcherStats
// };
