//! Sentinel Traffic - 流量分析代理与插件引擎\n//!\n//! 本 crate 提供：\n//! - 基于 Hudsucker 的 HTTP/HTTPS 拦截代理（MITM）\n//! - 流量分析流水线（请求/响应上下文、插件分发、Finding 去重）
//! - 证书管理（Root CA 生成、macOS Keychain 集成）
//! - HTML 报告导出（Tera 模板）
//!
//! ## 插件系统
//!
//! 插件系统已移至独立 crate `sentinel-plugins`，提供：
//! - 基于 Deno Core 的插件引擎（全权限，热重载）
//! - 插件管理器（加载、启用/禁用、注册表）
//! - 内置插件（SQL 注入、XSS、敏感信息检测）

pub mod certificate;
pub mod certificate_authority;
pub mod error;
pub mod finding;
pub mod history_cache;
pub mod packet_capture;
pub mod proxy;
pub mod scanner;
pub mod system_proxy;
pub mod types;

pub use certificate::CertificateService;
pub use certificate_authority::ChainedCertificateAuthority;
pub use error::{TrafficError, Result};

// Re-export traffic database types from sentinel-db
pub use sentinel_db::{
    TrafficEvidenceRecord as EvidenceRecord,
    TrafficVulnerabilityFilters as VulnerabilityFilters,
    TrafficVulnerabilityRecord as VulnerabilityRecord,
    TrafficVulnerabilityWithEvidence as VulnerabilityWithEvidence,
    ProxyRequestFilters,
    ProxyRequestRecord,
};
pub use history_cache::{
    HistoryCacheConfig, HistoryCacheStats, HttpRequestFilters, HttpRequestRecord,
    ProxyHistoryCache, ProxyHistoryFilters, ProxyHistoryItem, WebSocketConnectionRecord,
    WebSocketConnectionStatus, WebSocketDirection, WebSocketFilters, WebSocketMessageRecord,
    WebSocketMessageType,
};
pub use packet_capture::{
    CapturedPacket, ExtractedFile, FileExtractor, InterfaceInfo, PacketCaptureService, PcapFileOps,
    ProtocolLayer,
};
pub use proxy::{
    FailedConnection, InterceptAction, InterceptFilterRule, InterceptState, PendingInterceptRequest,
    PendingInterceptResponse, PendingInterceptWebSocketMessage, ProxyConfig, ProxyService,
    ScanSender, ScanTask, UpstreamProxyConfig, WebSocketConnectionContext,
    WebSocketDirection as ProxyWebSocketDirection, WebSocketMessageContext,
};
pub use scanner::{FindingDeduplicator, FindingReceiver, FindingSender, ScanPipeline};
pub use types::*;

// 重导出插件系统（来自 sentinel-plugins）
pub use sentinel_plugins::{
    PluginEngine, PluginError, PluginManager, PluginMetadata, PluginRecord, PluginStatus,
};

/// 流量分析系统版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
