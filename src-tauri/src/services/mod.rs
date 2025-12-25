//! Services module

pub mod ai_manager;
pub mod asset_service;
pub mod database {
    pub use sentinel_db::DatabaseService;
    pub use sentinel_db::Database;
}
pub mod vulnerability;
pub mod prompt_service;
pub mod mcp;

// Re-export from sentinel-services
pub use sentinel_services::message_emitter;
pub use sentinel_services::performance;
pub use sentinel_services::dictionary;

// AI services
pub use ai_manager::{AiServiceManager, AiServiceWrapper};
pub use sentinel_llm::{AiConfig, AiService, SchedulerConfig, SchedulerStage};

// Compatibility module
pub mod ai {
    pub use super::ai_manager::{AiServiceManager, AiServiceWrapper, ModelInfo};
    pub use sentinel_llm::{
        AiConfig, AiToolCall, SchedulerConfig, SchedulerStage,
        StreamError, StreamMessage, TaskProgressMessage, TaskStreamMessage,
        ToolCallResultMessage,
    };
    pub type AiService = super::ai_manager::AiServiceWrapper;
}

// Other services
pub use asset_service::AssetService;
pub use database::DatabaseService;

// Re-export from sentinel-services
pub use sentinel_services::performance::{
    PerformanceConfig, PerformanceMetrics, PerformanceMonitor, PerformanceOptimizer,
};
pub use sentinel_services::message_emitter::TauriMessageEmitter;
pub use sentinel_services::dictionary::DictionaryService;

pub use prompt_service::{
    PromptService, PromptServiceConfig, PromptSession, ExecutionRecord,
    SessionPerformanceStats, PromptBuildRequest, PromptBuildType,
    PromptBuildResponse, OptimizationRequest, ValidationSettings,
    ServiceStats, HealthStatus,
};

pub use vulnerability::VulnerabilityService;

// Database wrapper removed, use sentinel_db::Database trait directly
