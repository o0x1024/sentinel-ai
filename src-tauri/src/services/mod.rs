//! Services module

pub mod ai_manager;
pub mod ai_task_plugin_fix;
pub mod ai_tasks;
pub mod asset_service;
pub mod builtin_bounty_plugins;
pub mod database {
    pub use sentinel_db::Database;
    pub use sentinel_db::DatabaseService;
}
pub mod http_gateway;
pub mod mcp;
pub mod model_capabilities;
pub mod system_agents;
pub mod vulnerability;

// Re-export from sentinel-services
pub use sentinel_services::dictionary;
pub use sentinel_services::message_emitter;
pub use sentinel_services::performance;

// AI services
pub use ai_manager::{AiServiceManager, AiServiceWrapper};
pub use model_capabilities::{
    classify_model_vision_capability_error, clear_cached_model_vision_capabilities,
    get_cached_model_vision_capability_from_snapshot, load_model_vision_capability_cache_snapshot,
    resolve_model_vision_capability, save_cached_model_vision_capability,
    ModelVisionCapabilitySource, ModelVisionCapabilityStatus, ResolvedModelVisionCapability,
};
pub use sentinel_llm::{AiConfig, AiService, SchedulerConfig, SchedulerStage};

// Compatibility module
pub mod ai {
    pub use super::ai_manager::{AiServiceManager, AiServiceWrapper, ModelInfo};
    pub use sentinel_llm::{
        AiConfig, AiToolCall, SchedulerConfig, SchedulerStage, StreamError, StreamMessage,
        TaskProgressMessage, TaskStreamMessage, ToolCallResultMessage,
    };
    pub type AiService = super::ai_manager::AiServiceWrapper;
}

// Other services
pub use ai_task_plugin_fix::{run_plugin_fix_task, PluginFixTaskRequest, PluginFixTaskResult};
pub use asset_service::AssetService;
pub use database::DatabaseService;
pub use system_agents::SystemAgentRuntime;

// Re-export from sentinel-services
pub use sentinel_services::dictionary::DictionaryService;
pub use sentinel_services::message_emitter::TauriMessageEmitter;
pub use sentinel_services::performance::{
    PerformanceConfig, PerformanceMetrics, PerformanceMonitor, PerformanceOptimizer,
};

pub use vulnerability::VulnerabilityService;

// Database wrapper removed, use sentinel_db::Database trait directly
