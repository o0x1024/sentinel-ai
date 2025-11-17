//! 统一工具管理系统
//!
//! 这是一个精简的工具管理系统，提供：
//! - 统一的工具接口和管理
//! - 内置工具和外部工具的统一调度
//! - 动态工具发现和执行
//! - 批量执行和并发控制
//! - 执行历史和统计信息

// 核心模块
pub mod analyzer_tools; // 网站分析工具（Plan B）
pub mod builtin;
pub mod generator_tools; // AI插件生成工具（Plan B）
pub mod mapping {
    pub use sentinel_tools::mapping::*;
}

pub mod mcp_provider;
pub mod passive_provider; // 被动扫描工具提供者
pub mod agent_plugin_provider; // Agent插件工具提供者
pub mod plugin_parser; // 插件代码解析器
pub mod passive_integration; // 被动扫描工具集成
// plugin_generator (方案A) 已删除，使用 generator_tools (方案B)
pub mod protocol;
pub mod server; // 重新启用 // MCP工具提供者

// 测试模块
#[cfg(test)]
mod test_agent_plugin;

// 增强型客户端模块
pub mod client;
pub mod client_impl;
pub mod batch_progress_manager {
    pub use sentinel_tools::batch_progress_manager::*;
}

// 统一工具系统（迁移至 sentinel-tools）
pub use sentinel_tools::unified_types::*;
pub mod unified_types {
    pub use sentinel_tools::unified_types::*;
}
pub use sentinel_tools::UnifiedToolManager;
pub mod error_classifier {
    pub use sentinel_tools::error_classifier::*;
}
pub mod error_config_loader {
    pub use sentinel_tools::error_config_loader::*;
}
pub mod unified_manager;

// 框架适配器系统
pub mod adapter_factory;
pub mod framework_adapters;

// 重新导出核心组件
pub use builtin::BuiltinToolProvider;
pub use mcp_provider::{create_mcp_tool_provider, McpToolProvider};
pub use passive_provider::PassiveToolProvider;
pub use agent_plugin_provider::AgentPluginProvider;
pub use passive_integration::register_passive_tools;

// 重新导出增强型客户端组件
pub use batch_progress_manager::{
    create_default_batch_manager, BatchProgressManager, BatchRequest, BatchRequestBuilder,
    BatchResponse, ProgressNotification,
};
pub use client::{
    create_child_process_config, create_sse_client_config, McpClientConfig, McpClientManager,
    McpSession, McpSessionImpl, OAuthConfig, TransportType,
};

// 重新导出服务器组件（暂时注释掉）
// pub use server::{McpServerManager, SentinelMcpServer};

// 重新导出统一工具系统
pub use error_classifier::*;
pub use error_config_loader::*;
pub use unified_manager::*;

// 重新导出框架适配器系统
pub use adapter_factory::{
    create_llm_compiler_config, create_plan_execute_config, create_rewoo_config,
    get_engine_adapter, get_framework_adapter, get_global_adapter_manager,
    get_global_engine_adapter, initialize_global_adapter_manager,
    is_global_adapter_manager_initialized, AdapterConfigBuilder, AdapterFactory, AdapterRegistry,
    GlobalAdapterManager,
};
pub use framework_adapters::{
    BaseFrameworkAdapter, LLMCompilerAdapter, PlanAndExecuteAdapter, ReWOOAdapter,
};

// 导出服务器组件
pub use server::{McpServerManager, SentinelMcpServer};

// 模块别名
pub mod mcp {
    pub use super::*;
}
