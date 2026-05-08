//! Sentinel AI 流量分析插件系统
//!
//! 此 crate 提供：
//! - **插件引擎**: 基于 Deno Core 的 JS/TS 插件运行时
//! - **插件管理器**: 插件加载、启用/禁用、注册表管理
//! - **内置插件**: SQL 注入、XSS、敏感信息检测等
//! - **类型定义**: TypeScript 类型定义和插件模板
//!
//! ## 模块结构
//!
//! - `plugin_engine`: Deno Core 插件引擎
//! - `plugin`: 插件管理器（PluginManager）
//! - `types`: 核心类型（Finding, RequestContext, ResponseContext 等）
//! - `error`: 错误类型
//!
//! ## 内置插件
//!
//! 所有插件源码位于 `plugins/` 目录：
//! - `plugins/builtin/` - 内置插件（SQL 注入、XSS、敏感信息）
//! - `plugins/template.ts` - 插件模板
//! - `plugins/plugin-types.d.ts` - TypeScript 类型定义
//! - `plugins/README.md` - 开发指南

pub mod active_probe_scheduler;
pub mod dictionary_runtime;
pub mod error;
pub mod executor;
mod monitor_progress;
mod network_scan;
pub mod plugin;
pub mod plugin_context;
pub mod plugin_engine;
mod plugin_fetch_context;
mod plugin_fetch_types;
mod plugin_finding_sanitizer;
pub mod plugin_ops;
pub mod request_scheduler;
pub mod runtime_config;
mod runtime_events;
mod service_probe;
mod service_probe_engine;
mod service_probe_native;
mod service_probe_runtime;
pub mod types;

pub use active_probe_scheduler::{
    cancel_active_probe, complete_active_probe, enqueue_active_probe, fail_active_probe,
    get_active_probe_queue_snapshot, mark_active_probe_running, ActiveProbeDispatchGrant,
    ActiveProbeQueueEntry, ActiveProbeQueuePhase, ActiveProbeQueueSnapshot, ActiveProbeRequest,
};
pub use dictionary_runtime::init_dictionary_pool;
pub use error::{PluginError, Result};
pub use executor::{ExecutorStats, PluginExecutor};
pub use monitor_progress::{
    emit_plugin_monitor_progress, MonitorProgressContext, PluginMonitorProgressUpdate,
};
pub use plugin::{
    get_input_schema_from_code, get_output_schema_from_code, PluginManager, PluginRecord,
    PluginStatus,
};
pub use plugin_context::PluginContext;
pub use plugin_engine::PluginEngine;
pub use plugin_ops::{cancel_plugin_fetch_requests_by_run, sentinel_plugin_ext};
pub use request_scheduler::{
    cancel_plugin_request, cancel_plugin_requests_by_run, complete_plugin_request,
    enqueue_plugin_request, fail_plugin_request, get_plugin_request_queue_snapshot,
    get_plugin_request_queue_stats, mark_plugin_request_running, PluginFetchPolicy,
    PluginFetchPolicyKind, PluginRequestDispatchGrant, PluginRequestPhase, PluginRequestQueueEntry,
    PluginRequestQueueSnapshot, PluginRequestQueueStats, PluginRequestScheduleRequest,
};
pub use runtime_config::{
    get_plugin_runtime_settings, set_plugin_runtime_settings, ActiveProbeRuntimeSettings,
    PluginRuntimeSettings,
};
pub use runtime_events::register_app_handle;
pub use service_probe_runtime::{
    op_probe_services, probe_services, ServiceProbeRequest, ServiceProbeResponse,
    ServiceProbeResult, ServiceProbeRule, ServiceProbeTarget,
};
pub use types::*;

/// 获取内置插件目录路径
pub fn get_builtin_plugins_dir() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/plugins/builtin")
}

/// 获取插件模板路径
pub fn get_plugin_template_path() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/plugins/template.ts")
}

/// 获取类型定义路径
pub fn get_types_definition_path() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/plugins/plugin-types.d.ts")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paths() {
        let builtin_dir = get_builtin_plugins_dir();
        let template = get_plugin_template_path();
        let types = get_types_definition_path();

        assert!(builtin_dir.contains("sentinel-plugins/plugins/builtin"));
        assert!(template.contains("sentinel-plugins/plugins/template.ts"));
        assert!(types.contains("sentinel-plugins/plugins/plugin-types.d.ts"));
    }
}
