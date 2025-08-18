//! 统一工具管理系统
//! 
//! 这是一个精简的工具管理系统，提供：
//! - 统一的工具接口和管理
//! - 内置工具和外部工具的统一调度
//! - 动态工具发现和执行
//! - 批量执行和并发控制
//! - 执行历史和统计信息

// 核心模块
pub mod builtin;
pub mod mapping;

pub mod protocol;
pub mod server; // 重新启用
pub mod mcp_provider; // MCP工具提供者

// 增强型客户端模块
pub mod client;
pub mod client_impl;
pub mod oauth_manager;
pub mod batch_progress_manager;

// 统一工具系统
pub mod unified_types;
pub mod unified_manager;

// 框架适配器系统
pub mod framework_adapters;
pub mod adapter_factory;



// 重新导出核心组件
pub use builtin::BuiltinToolProvider;
pub use mcp_provider::{McpToolProvider, create_mcp_tool_provider};

// 重新导出增强型客户端组件
pub use client::{
    McpClientConfig, McpSession, McpSessionImpl, 
    McpClientManager, TransportType, OAuthConfig,
    create_child_process_config, create_sse_client_config
};
pub use oauth_manager::{
    OAuth21Manager, OAuth21Config, AccessTokenInfo, AuthStatus, PkceParams,
    create_default_oauth21_config
};
pub use batch_progress_manager::{
    BatchProgressManager, BatchRequest, BatchResponse, ProgressNotification,
    BatchRequestBuilder, create_default_batch_manager
};

// 重新导出服务器组件（暂时注释掉）
// pub use server::{McpServerManager, SentinelMcpServer};

// 重新导出统一工具系统
pub use unified_types::*;
pub use unified_manager::*;

// 重新导出框架适配器系统
pub use framework_adapters::{
    BaseFrameworkAdapter, PlanAndExecuteAdapter, ReWOOAdapter, LLMCompilerAdapter
};
pub use adapter_factory::{
    AdapterFactory, AdapterRegistry, GlobalAdapterManager,
    initialize_global_adapter_manager, get_global_adapter_manager,
    get_framework_adapter, get_engine_adapter, get_global_engine_adapter,
    is_global_adapter_manager_initialized,
    AdapterConfigBuilder, create_plan_execute_config, create_rewoo_config, create_llm_compiler_config
};



// 导出服务器组件
pub use server::{McpServerManager, SentinelMcpServer};



// 模块别名
pub mod mcp {
    pub use super::*;
}