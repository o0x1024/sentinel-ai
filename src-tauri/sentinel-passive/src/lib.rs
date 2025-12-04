//! Sentinel Passive - 被动扫描代理与插件引擎
//!
//! 本 crate 提供：
//! - 基于 Hudsucker 的 HTTP/HTTPS 拦截代理（MITM）
//! - 被动扫描流水线（请求/响应上下文、插件分发、Finding 去重）
//! - 证书管理（Root CA 生成、macOS Keychain 集成）
//! - HTML 报告导出（Tera 模板）
//!
//! ## 插件系统
//!
//! 插件系统已移至独立 crate `sentinel-plugins`，提供：
//! - 基于 Deno Core 的插件引擎（全权限，热重载）
//! - 插件管理器（加载、启用/禁用、注册表）
//! - 内置插件（SQL 注入、XSS、敏感信息检测）

pub mod proxy;
pub mod certificate;
pub mod certificate_authority;
pub mod scanner;
pub mod finding;
pub mod database;
pub mod error;
pub mod types;
pub mod system_proxy;

pub use error::{PassiveError, Result};
pub use types::*;
pub use proxy::{ProxyConfig, ProxyService, ScanTask, ScanSender, InterceptState, InterceptAction, PendingInterceptRequest, PendingInterceptResponse, FailedConnection};
pub use scanner::{ScanPipeline, FindingDeduplicator, FindingSender, FindingReceiver};
pub use database::{
    PassiveDatabaseService, 
    VulnerabilityFilters, 
    VulnerabilityRecord, 
    EvidenceRecord,
    VulnerabilityWithEvidence,
    ProxyRequestRecord,
    ProxyRequestFilters,
};
pub use certificate::CertificateService;
pub use certificate_authority::ChainedCertificateAuthority;

// 重导出插件系统（来自 sentinel-plugins）
pub use sentinel_plugins::{
    PluginManager, 
    PluginEngine, 
    PluginStatus, 
    PluginRecord,
    PluginMetadata,
    PluginError,
};

/// 被动扫描系统版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
