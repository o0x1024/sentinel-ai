//! 系统代理管理模块
//!
//! 提供系统级代理配置功能：
//! - macOS: 通过 networksetup 命令设置系统代理
//! - pf 防火墙规则管理实现透明代理
//! - Network Extension 实现应用级透明代理

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "macos")]
pub mod pf_firewall;

pub mod network_extension;

// Re-export macOS specific functions
#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(target_os = "macos")]
pub use pf_firewall::{
    PfConfig, 
    TransparentProxyManager, 
    AppProxyFilter,
    is_pf_enabled,
    enable_pf,
    disable_pf,
    load_pf_rules,
    load_anchor_rules,
    flush_anchor_rules,
    get_pf_rules,
    get_anchor_rules,
};

pub use network_extension::{
    NetworkExtensionManager,
    ExtensionStatus,
    VPNStatus,
};

/// 系统代理配置
#[derive(Debug, Clone)]
pub struct SystemProxyConfig {
    pub host: String,
    pub port: u16,
    pub proxy_type: ProxyType,
    pub username: Option<String>,
    pub password: Option<String>,
}

/// 代理类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProxyType {
    Http,
    Https,
    Socks,
}

/// 系统代理状态
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SystemProxyStatus {
    pub http_enabled: bool,
    pub http_host: Option<String>,
    pub http_port: Option<u16>,
    pub https_enabled: bool,
    pub https_host: Option<String>,
    pub https_port: Option<u16>,
    pub socks_enabled: bool,
    pub socks_host: Option<String>,
    pub socks_port: Option<u16>,
    pub network_services: Vec<String>,
}

