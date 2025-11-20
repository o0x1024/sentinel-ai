//! 全局代理配置管理
//!
//! 提供线程安全的全局代理配置，支持：
//! - 启用/禁用代理
//! - 配置代理服务器地址和端口
//! - 支持认证
//! - 应用到 reqwest 客户端

use once_cell::sync::Lazy;
use reqwest::{Client, Proxy};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// 全局代理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalProxyConfig {
    /// 是否启用代理
    pub enabled: bool,
    /// 代理协议 (http/https)
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

impl Default for GlobalProxyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            scheme: None,
            host: None,
            port: None,
            username: None,
            password: None,
            no_proxy: None,
        }
    }
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
    *proxy = config.clone();
    
    if config.enabled {
        if let Some(url) = config.build_proxy_url() {
            // 不记录密码
            let safe_url = if config.username.is_some() {
                format!("{}://***:***@{}:{}", 
                    config.scheme.as_deref().unwrap_or("http"),
                    config.host.as_deref().unwrap_or("unknown"),
                    config.port.unwrap_or(0))
            } else {
                url.clone()
            };
            info!("Global proxy enabled: {}", safe_url);
        }
    } else {
        info!("Global proxy disabled");
    }
}

/// 获取全局代理配置
pub async fn get_global_proxy() -> GlobalProxyConfig {
    GLOBAL_PROXY.read().await.clone()
}

/// 清除全局代理配置
pub async fn clear_global_proxy() {
    let mut proxy = GLOBAL_PROXY.write().await;
    *proxy = GlobalProxyConfig::default();
    info!("Global proxy cleared");
}

/// 为 reqwest Client 应用全局代理配置
pub async fn apply_proxy_to_client(builder: reqwest::ClientBuilder) -> reqwest::ClientBuilder {
    let config = get_global_proxy().await;
    
    if !config.enabled {
        debug!("Global proxy not enabled, returning unmodified client builder");
        return builder;
    }

    if let Some(proxy_url) = config.build_proxy_url() {
        match Proxy::all(&proxy_url) {
            Ok(proxy) => {
                debug!("Applying proxy to reqwest client: {}://{}:{}", 
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

/// 创建带有全局代理的 reqwest Client
pub async fn create_client_with_proxy() -> Result<Client, reqwest::Error> {
    let builder = Client::builder();
    let builder = apply_proxy_to_client(builder).await;
    builder.build()
}

/// 创建带有全局代理和自定义配置的 reqwest Client
pub async fn create_client_with_proxy_and_config(
    builder: reqwest::ClientBuilder,
) -> Result<Client, reqwest::Error> {
    let builder = apply_proxy_to_client(builder).await;
    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_proxy_url_generation() {
        let config = GlobalProxyConfig {
            enabled: true,
            scheme: Some("http".to_string()),
            host: Some("127.0.0.1".to_string()),
            port: Some(8080),
            username: None,
            password: None,
            no_proxy: None,
        };

        assert_eq!(
            config.build_proxy_url(),
            Some("http://127.0.0.1:8080".to_string())
        );
    }

    #[tokio::test]
    async fn test_proxy_url_with_auth() {
        let config = GlobalProxyConfig {
            enabled: true,
            scheme: Some("http".to_string()),
            host: Some("proxy.example.com".to_string()),
            port: Some(3128),
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
            no_proxy: None,
        };

        assert_eq!(
            config.build_proxy_url(),
            Some("http://user:pass@proxy.example.com:3128".to_string())
        );
    }

    #[tokio::test]
    async fn test_disabled_proxy() {
        let config = GlobalProxyConfig {
            enabled: false,
            scheme: Some("http".to_string()),
            host: Some("127.0.0.1".to_string()),
            port: Some(8080),
            username: None,
            password: None,
            no_proxy: None,
        };

        assert_eq!(config.build_proxy_url(), None);
    }
}

