//! 全局代理配置（用于 sentinel-tools crate）
//! 
//! 由于 sentinel-tools 是独立的 crate，需要自己的全局代理配置存储

use once_cell::sync::Lazy;
use reqwest::Proxy;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// 全局代理配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalProxyConfig {
    pub enabled: bool,
    pub scheme: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
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
            Some(format!("{}://{}:{}@{}:{}", scheme, username, password, host, port))
        } else {
            Some(format!("{}://{}:{}", scheme, host, port))
        }
    }
}

/// 全局代理状态
static GLOBAL_PROXY: Lazy<Arc<RwLock<GlobalProxyConfig>>> = 
    Lazy::new(|| Arc::new(RwLock::new(GlobalProxyConfig::default())));

/// 设置全局代理配置
pub async fn set_global_proxy(config: GlobalProxyConfig) {
    let mut proxy = GLOBAL_PROXY.write().await;
    *proxy = config;
}

/// 获取全局代理配置
pub async fn get_global_proxy() -> GlobalProxyConfig {
    GLOBAL_PROXY.read().await.clone()
}

/// 为 reqwest ClientBuilder 应用全局代理配置
pub async fn apply_proxy_to_client(builder: reqwest::ClientBuilder) -> reqwest::ClientBuilder {
    let config = get_global_proxy().await;
    
    if !config.enabled {
        debug!("Global proxy not enabled, returning unmodified client builder");
        return builder;
    }

    if let Some(proxy_url) = config.build_proxy_url() {
        match Proxy::all(&proxy_url) {
            Ok(proxy) => {
                debug!("Applying global proxy to reqwest client: {}://{}:{}", 
                    config.scheme.as_deref().unwrap_or("http"),
                    config.host.as_deref().unwrap_or("unknown"),
                    config.port.unwrap_or(0));
                builder.proxy(proxy)
            }
            Err(e) => {
                warn!("Failed to create proxy for reqwest client: {}, using direct connection", e);
                builder
            }
        }
    } else {
        debug!("No valid proxy URL, returning unmodified client builder");
        builder
    }
}

