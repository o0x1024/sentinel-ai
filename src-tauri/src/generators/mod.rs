//! Advanced plugin generators for Plan B

pub mod advanced_generator;
pub mod auto_approval;
pub mod few_shot_examples;
pub mod prompt_templates;
pub mod quality_model;
pub mod validator;

pub use advanced_generator::{
    AdvancedPluginGenerator, GeneratedPlugin, PluginGenerationRequest, PluginStatus,
    QualityBreakdown,
};
pub use auto_approval::{
    ApprovalDecision, ApprovalStats, PluginAutoApprovalConfig, PluginAutoApprovalEngine,
};
pub use few_shot_examples::{FewShotExample, FewShotRepository};
pub use prompt_templates::PromptTemplateBuilder;
pub use quality_model::{CodeFeatures, QualityModel, TrainingReport, TrainingSample};
pub use validator::{ExecutionTestResult, PluginValidator, ValidationResult};
