//! Advanced plugin generators for Plan B

pub mod advanced_generator;
pub mod auto_approval;
pub mod prompt_templates;
pub mod validator;
pub mod few_shot_examples;
pub mod quality_model;

pub use advanced_generator::{
    AdvancedPluginGenerator, GeneratedPlugin, PluginGenerationRequest,
    PluginStatus, QualityBreakdown,
};
pub use auto_approval::{
    PluginAutoApprovalEngine, PluginAutoApprovalConfig, ApprovalDecision, ApprovalStats,
};
pub use prompt_templates::PromptTemplateBuilder;
pub use validator::{PluginValidator, ValidationResult, ExecutionTestResult};
pub use few_shot_examples::{FewShotExample, FewShotRepository};
pub use quality_model::{QualityModel, TrainingSample, CodeFeatures, TrainingReport};

