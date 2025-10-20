//! AI适配器模块
//! 
//! 提供统一的AI服务接口，支持多种AI提供商

pub mod types;
pub mod error;
pub mod http;
pub mod core;
pub mod providers;
pub mod provider_adapter;
pub mod request_logger;

// 重新导出核心类型
pub use types::*;
pub use error::{AiAdapterError, Result, ErrorDetails};
pub use core::{AiClient, BaseProvider};
pub use http::HttpClient;

// 重新导出工具模块
pub mod utils;
pub use utils::*;

// 重新导出日志记录器
pub use request_logger::{init_global_logger, set_global_logger_enabled, log_http_request, cleanup_old_logs, is_global_logger_enabled};

// 重新导出提供商
pub use providers::*;

/// 初始化AI适配器
pub fn init() -> Result<()> {
    tracing::info!("Initializing AI adapter");
    Ok(())
}

/// 获取全局AI客户端实例
pub fn global_client() -> &'static AiClient {
    AiClient::global()
}

/// 创建新的AI客户端实例
pub fn new_client() -> AiClient {
    AiClient::new()
}

