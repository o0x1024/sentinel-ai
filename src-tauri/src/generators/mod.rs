//! Advanced plugin generators for Plan B

pub mod advanced_generator;
pub mod agent_plugin_contract;
pub mod auto_approval;
pub mod few_shot_examples;
pub mod prompt_templates;
pub mod quality_model;
pub mod validator;

pub use advanced_generator::{
    AdvancedPluginGenerator, GeneratedPlugin, PluginGenerationRequest, PluginStatus,
    QualityBreakdown,
};
pub use agent_plugin_contract::{
    agent_contract_generation_instructions, parse_agent_plugin_definition,
    render_agent_plugin_definition, validate_agent_plugin_source_contract, validate_schema_object,
    AgentPluginDefinition, AgentPluginRenderContext, AGENT_TOOL_CONTRACT,
    AGENT_TOOL_CONTRACT_VERSION,
};
pub use auto_approval::{
    ApprovalDecision, ApprovalStats, PluginAutoApprovalConfig, PluginAutoApprovalEngine,
};
pub use few_shot_examples::{FewShotExample, FewShotRepository};
pub use prompt_templates::PromptTemplateBuilder;
pub use quality_model::{CodeFeatures, QualityModel, TrainingReport, TrainingSample};
pub use validator::{ExecutionTestResult, PluginValidator, ValidationResult};
