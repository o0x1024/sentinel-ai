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
#[derive(Default)]
pub struct GlobalProxyConfig {
    /// 是否启用代理
    pub enabled: bool,
    /// 代理协议 (http/https/socks5/socks5h)
    /// - http: HTTP代理
    /// - https: HTTPS代理
    /// - socks5: SOCKS5代理
    /// - socks5h: SOCKS5代理（远程DNS解析）
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
/// 
/// 重要说明：
/// - reqwest 默认不会自动读取环境变量代理，即使设置了 HTTP_PROXY 等变量
/// - 但某些第三方库（如 rig）可能会创建自己的 reqwest 客户端
/// - 为了让这些客户端也能使用代理，我们需要：
///   1. 设置环境变量（用于某些可能读取它的库）
///   2. 在应用启动时就配置好代理，避免后续创建的客户端无法感知
/// - 最佳实践：应用启动后立即调用此函数，确保所有 HTTP 客户端都能使用代理
pub async fn set_global_proxy(config: GlobalProxyConfig) {
    let mut proxy = GLOBAL_PROXY.write().await;
    *proxy = config.clone();
    
    if config.enabled {
        if let Some(url) = config.build_proxy_url() {
            let scheme = config.scheme.as_deref().unwrap_or("http");
            
            // 设置环境变量
            // 注意：这些环境变量主要用于：
            // 1. 兼容某些可能读取它们的第三方库
            // 2. 为手动创建的 reqwest 客户端提供配置参考
            // 3. 但 reqwest 本身默认不会自动读取这些变量！
            match scheme {
                "socks5" | "socks5h" => {
                    // SOCKS5 代理需要设置专门的环境变量
                    std::env::set_var("ALL_PROXY", &url);
                    std::env::set_var("all_proxy", &url);
                    // 同时设置 HTTP/HTTPS 代理变量以提高兼容性
                    std::env::set_var("HTTP_PROXY", &url);
                    std::env::set_var("HTTPS_PROXY", &url);
                    std::env::set_var("http_proxy", &url);
                    std::env::set_var("https_proxy", &url);
                }
                _ => {
                    // HTTP/HTTPS 代理
                    std::env::set_var("HTTP_PROXY", &url);
                    std::env::set_var("HTTPS_PROXY", &url);
                    std::env::set_var("http_proxy", &url);
                    std::env::set_var("https_proxy", &url);
                }
            }
            
            // 设置 no_proxy 环境变量
            if let Some(no_proxy) = &config.no_proxy {
                if !no_proxy.trim().is_empty() {
                    std::env::set_var("NO_PROXY", no_proxy);
                    std::env::set_var("no_proxy", no_proxy);
                }
            }
            
            // 不记录密码
            let safe_url = if config.username.is_some() {
                format!("{}://***:***@{}:{}", 
                    scheme,
                    config.host.as_deref().unwrap_or("unknown"),
                    config.port.unwrap_or(0))
            } else {
                url.clone()
            };
            info!("Global proxy enabled ({}): {}", scheme, safe_url);
            debug!("Environment variables set: HTTP_PROXY={}, HTTPS_PROXY={}", 
                std::env::var("HTTP_PROXY").unwrap_or_default(),
                std::env::var("HTTPS_PROXY").unwrap_or_default());
        }
    } else {
        // 清除所有代理环境变量
        std::env::remove_var("HTTP_PROXY");
        std::env::remove_var("HTTPS_PROXY");
        std::env::remove_var("http_proxy");
        std::env::remove_var("https_proxy");
        std::env::remove_var("ALL_PROXY");
        std::env::remove_var("all_proxy");
        std::env::remove_var("NO_PROXY");
        std::env::remove_var("no_proxy");
        info!("Global proxy disabled and all environment variables cleared");
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
    
    // 清除所有代理环境变量（包括大小写版本）
    std::env::remove_var("HTTP_PROXY");
    std::env::remove_var("HTTPS_PROXY");
    std::env::remove_var("http_proxy");
    std::env::remove_var("https_proxy");
    std::env::remove_var("ALL_PROXY");
    std::env::remove_var("all_proxy");
    std::env::remove_var("NO_PROXY");
    std::env::remove_var("no_proxy");
    
    info!("Global proxy cleared and all environment variables removed");
}

/// 为 reqwest Client 应用全局代理配置
/// 
/// 支持的代理协议：
/// - http: 标准HTTP代理
/// - https: HTTPS代理
/// - socks5: SOCKS5代理（本地DNS解析）
/// - socks5h: SOCKS5代理（远程DNS解析，更安全）
pub async fn apply_proxy_to_client(builder: reqwest::ClientBuilder) -> reqwest::ClientBuilder {
    let config = get_global_proxy().await;
    
    if !config.enabled {
        debug!("Global proxy not enabled, returning unmodified client builder");
        return builder;
    }

    if let Some(proxy_url) = config.build_proxy_url() {
        let scheme = config.scheme.as_deref().unwrap_or("http");
        
        match Proxy::all(&proxy_url) {
            Ok(proxy) => {
                debug!("Applying {} proxy to reqwest client: {}:{}", 
                    scheme,
                    config.host.as_deref().unwrap_or("unknown"),
                    config.port.unwrap_or(0));
                builder.proxy(proxy)
            }
            Err(e) => {
                warn!("Failed to create {} proxy for reqwest client: {}, using direct connection", scheme, e);
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

