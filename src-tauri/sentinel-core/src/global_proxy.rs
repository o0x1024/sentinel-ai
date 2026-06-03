//! 全局代理配置模块
//!
//! 提供统一的全局代理配置，供所有 sentinel crates 使用
//!
//! 支持的代理协议：
//! - http: 标准HTTP代理
//! - https: HTTPS代理
//! - socks5: SOCKS5代理（本地DNS解析）
//! - socks5h: SOCKS5代理（远程DNS解析，更安全）

use once_cell::sync::Lazy;
use reqwest::Proxy;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// 全局代理配置
///
/// 支持多种代理协议：HTTP、HTTPS、SOCKS5、SOCKS5H
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalProxyConfig {
    /// 是否启用代理
    pub enabled: bool,
    /// 代理协议 (http/https/socks5/socks5h)
    pub scheme: Option<String>,
    /// 代理主机地址
    pub host: Option<String>,
    /// 代理端口
    pub port: Option<u16>,
    /// 用户名（可选）
    pub username: Option<String>,
    /// 密码（可选）
    pub password: Option<String>,
    /// 不使用代理的地址列表（逗号分隔）
    pub no_proxy: Option<String>,
}

impl GlobalProxyConfig {
    /// 构建代理 URL
    pub fn build_proxy_url(&self) -> Option<String> {
        if !self.enabled {
            return None;
        }

        let host = self.host.as_ref()?;
        let port = self.port?;
        let scheme = self.scheme.as_deref().unwrap_or("http");

        if let (Some(username), Some(password)) = (&self.username, &self.password) {
            Some(format!(
                "{}://{}:{}@{}:{}",
                scheme, username, password, host, port
            ))
        } else {
            Some(format!("{}://{}:{}", scheme, host, port))
        }
    }
}

/// 全局代理状态
static GLOBAL_PROXY: Lazy<Arc<RwLock<GlobalProxyConfig>>> =
    Lazy::new(|| Arc::new(RwLock::new(GlobalProxyConfig::default())));

const PROXY_ENV_KEYS: [&str; 6] = [
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "ALL_PROXY",
    "http_proxy",
    "https_proxy",
    "all_proxy",
];

const NO_PROXY_ENV_KEYS: [&str; 2] = ["NO_PROXY", "no_proxy"];
const LOCAL_NO_PROXY_DEFAULTS: [&str; 3] = ["localhost", "127.0.0.1", "::1"];

/// 设置全局代理配置
pub async fn set_global_proxy(config: GlobalProxyConfig) {
    let mut proxy = GLOBAL_PROXY.write().await;
    *proxy = config;
    apply_proxy_env_vars(&proxy);
}

/// 获取全局代理配置
pub async fn get_global_proxy() -> GlobalProxyConfig {
    GLOBAL_PROXY.read().await.clone()
}

/// 清除全局代理配置
pub async fn clear_global_proxy() {
    let mut proxy = GLOBAL_PROXY.write().await;
    *proxy = GlobalProxyConfig::default();
    clear_proxy_env_vars();
}

/// 创建一个应用了全局代理的 HTTP 客户端
pub async fn create_client_with_proxy() -> Result<reqwest::Client, reqwest::Error> {
    let builder = reqwest::Client::builder();
    let builder = apply_proxy_to_client(builder).await;
    builder.build()
}

/// 为 reqwest ClientBuilder 应用全局代理配置
///
/// 支持的代理协议：
/// - http: 标准HTTP代理
/// - https: HTTPS代理  
/// - socks5: SOCKS5代理（本地DNS解析）
/// - socks5h: SOCKS5代理（远程DNS解析，更安全）
///
/// 自动为所有请求添加 X-Sentinel-Internal header，用于在代理中识别本应用流量
pub async fn apply_proxy_to_client(builder: reqwest::ClientBuilder) -> reqwest::ClientBuilder {
    let config = get_global_proxy().await;

    // 添加标识 Header，用于在代理中识别本应用的流量
    let mut default_headers = reqwest::header::HeaderMap::new();
    default_headers.insert(
        reqwest::header::HeaderName::from_static("x-sentinel-internal"),
        reqwest::header::HeaderValue::from_static("true"),
    );

    let builder = builder.default_headers(default_headers);

    if !config.enabled {
        debug!("Global proxy not enabled, returning client builder with sentinel headers");
        return builder;
    }

    if let Some(proxy_url) = config.build_proxy_url() {
        let scheme = config.scheme.as_deref().unwrap_or("http");

        match Proxy::all(&proxy_url) {
            Ok(proxy) => {
                debug!(
                    "Applying {} proxy to reqwest client: {}:{}",
                    scheme,
                    config.host.as_deref().unwrap_or("unknown"),
                    config.port.unwrap_or(0)
                );
                builder.proxy(proxy)
            }
            Err(e) => {
                warn!(
                    "Failed to create {} proxy for reqwest client: {}, using direct connection",
                    scheme, e
                );
                builder
            }
        }
    } else {
        debug!("No valid proxy URL, returning client builder with sentinel headers");
        builder
    }
}

fn apply_proxy_env_vars(config: &GlobalProxyConfig) {
    if !config.enabled {
        clear_proxy_env_vars();
        return;
    }

    let Some(proxy_url) = config.build_proxy_url() else {
        clear_proxy_env_vars();
        return;
    };

    for key in PROXY_ENV_KEYS {
        std::env::set_var(key, &proxy_url);
    }

    let no_proxy = merged_no_proxy(config.no_proxy.as_deref());
    if no_proxy.is_empty() {
        for key in NO_PROXY_ENV_KEYS {
            std::env::remove_var(key);
        }
    } else {
        for key in NO_PROXY_ENV_KEYS {
            std::env::set_var(key, &no_proxy);
        }
    }
}

fn clear_proxy_env_vars() {
    for key in PROXY_ENV_KEYS {
        std::env::remove_var(key);
    }
    for key in NO_PROXY_ENV_KEYS {
        std::env::remove_var(key);
    }
}

fn merged_no_proxy(custom: Option<&str>) -> String {
    let mut entries: Vec<String> = custom
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    for local in LOCAL_NO_PROXY_DEFAULTS {
        if !entries.iter().any(|e| e.eq_ignore_ascii_case(local)) {
            entries.push(local.to_string());
        }
    }

    entries.join(",")
}
