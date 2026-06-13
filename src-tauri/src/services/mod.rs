//! Services module

pub mod ai_manager;
pub mod ai_task_plugin_fix;
pub mod ai_tasks;
pub mod asset_service;
pub mod bot_execution;
pub mod builtin_bounty_plugins;
pub mod feature_entitlements;
pub mod finding_ai_review;
pub mod plugin_authoring;
pub mod plugin_categories;
pub mod plugin_default_inputs;
pub mod plugin_execution_test;
pub mod database {
    pub use sentinel_db::Database;
    pub use sentinel_db::DatabaseService;
}
pub mod http_gateway;
pub mod mcp;
pub mod mission_artifacts;
pub mod mission_delivery;
pub mod mission_planner;
pub mod mission_run_worker;
pub mod mission_runner;
pub mod mission_scheduler;
pub mod mission_stateful_runtime;
pub mod mission_success_criteria;
pub mod observer_data_collector;
pub mod observer_event_evaluator;
pub mod model_capabilities;
pub mod system_agents;
pub mod traffic_codec;
pub mod traffic_oast;
pub mod vulnerability;
pub mod weixin_gateway;

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
pub use feature_entitlements::{
    build_app_entitlements, ensure_bot_console_access, ensure_bug_bounty_access,
    ensure_plugin_allowed_for_current_tier, ensure_plugin_catalog_write_access,
    ensure_plugin_delete_access, filter_plugins_for_current_tier, AppEntitlements,
};
pub use plugin_authoring::{
    execute_plugin_authoring, PluginAuthoringAction, PluginAuthoringRequest, PluginAuthoringResult,
};
pub use plugin_categories::{IntruderPluginCategory, PluginCategory, PluginMainCategory};
pub use plugin_default_inputs::{
    load_plugin_default_inputs, merge_plugin_input_defaults, save_plugin_default_inputs,
};
pub use plugin_execution_test::{
    build_plugin_metadata, parse_plugin_severity, test_plugin_code, PluginExecutionTestResult,
};
pub use system_agents::SystemAgentRuntime;
pub use traffic_oast::{
    delete_traffic_oast_token, generate_traffic_oast_token, lookup_traffic_oast_token,
    test_traffic_oast_config, OastGenerateResponse, TrafficOastConfig, TrafficOastEvent,
    TrafficOastEventKey, TrafficOastRecord, TrafficOastTestResult,
};

// Re-export from sentinel-services
pub use sentinel_services::dictionary::DictionaryService;
pub use sentinel_services::message_emitter::TauriMessageEmitter;
pub use sentinel_services::performance::{
    PerformanceConfig, PerformanceMetrics, PerformanceMonitor, PerformanceOptimizer,
};

pub use vulnerability::VulnerabilityService;

// Database wrapper removed, use sentinel_db::Database trait directly
