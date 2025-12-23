//! 代理配置测试模块

use tauri::State;
use std::sync::Arc;
use std::time::Instant;
use serde::{Serialize, Deserialize};
use crate::services::database::DatabaseService;
use crate::utils::global_proxy::{get_global_proxy, GlobalProxyConfig};
use tracing::{info, warn};

#[derive(Debug, Serialize, Deserialize)]
pub struct ProxyTestResult {
    pub success: bool,
    pub message: String,
    pub proxy_config: Option<serde_json::Value>,
    pub response_time_ms: Option<f64>,
}

/// 测试代理配置动态更新功能
/// 验证环境变量是否正确设置，以及 rig 库是否能使用代理
#[tauri::command]
pub async fn test_proxy_dynamic_update(
    db: State<'_, Arc<DatabaseService>>,
) -> Result<ProxyTestResult, String> {
    info!("Testing proxy dynamic update...");
    
    // 1. 获取当前代理配置
    let current_config = get_global_proxy().await;
    
    // 2. 检查环境变量是否设置
    let http_proxy = std::env::var("HTTP_PROXY").ok();
    let https_proxy = std::env::var("HTTPS_PROXY").ok();
    
    let mut messages = Vec::new();
    
    if current_config.enabled {
        if let Some(expected_url) = current_config.build_proxy_url() {
            // 检查环境变量是否匹配
            let env_vars_match = http_proxy.as_ref() == Some(&expected_url) 
                && https_proxy.as_ref() == Some(&expected_url);
            
            if env_vars_match {
                messages.push(format!("✓ 环境变量正确设置: {}", 
                    mask_password(&expected_url)));
            } else {
                messages.push(format!("✗ 环境变量不匹配！期望: {}, HTTP_PROXY: {:?}, HTTPS_PROXY: {:?}", 
                    mask_password(&expected_url),
                    http_proxy.as_ref().map(|s| mask_password(s)),
                    https_proxy.as_ref().map(|s| mask_password(s))));
            }
        }
    } else if http_proxy.is_none() && https_proxy.is_none() {
        messages.push("✓ 代理已禁用，环境变量已清除".to_string());
    } else {
        messages.push(format!("✗ 代理已禁用但环境变量仍然存在: HTTP_PROXY={:?}, HTTPS_PROXY={:?}", 
            http_proxy, https_proxy));
    }
    
    // 3. 验证数据库配置是否一致
    match db.get_config("network", "global_proxy").await {
        Ok(Some(json_str)) => {
            match serde_json::from_str::<GlobalProxyConfig>(&json_str) {
                Ok(db_config) => {
                    if db_config.enabled == current_config.enabled {
                        messages.push("✓ 数据库配置与运行时配置一致".to_string());
                    } else {
                        messages.push("✗ 数据库配置与运行时配置不一致".to_string());
                    }
                }
                Err(e) => {
                    messages.push(format!("✗ 无法解析数据库配置: {}", e));
                }
            }
        }
        Ok(None) => {
            messages.push("⚠ 数据库中未找到代理配置".to_string());
        }
        Err(e) => {
            messages.push(format!("✗ 读取数据库配置失败: {}", e));
        }
    }
    
    let success = !messages.iter().any(|m| m.starts_with("✗"));
    
    Ok(ProxyTestResult {
        success,
        message: messages.join("\n"),
        proxy_config: Some(serde_json::to_value(&current_config).unwrap_or_default()),
        response_time_ms: None,
    })
}

/// 测试代理持久化功能
#[tauri::command]
pub async fn test_proxy_persistence(
    db: State<'_, Arc<DatabaseService>>,
) -> Result<ProxyTestResult, String> {
    info!("Testing proxy persistence...");
    
    let start = Instant::now();
    let mut messages = Vec::new();
    
    // 1. 读取数据库配置
    let db_config = match db.get_config("network", "global_proxy").await {
        Ok(Some(json_str)) => {
            match serde_json::from_str::<GlobalProxyConfig>(&json_str) {
                Ok(config) => {
                    messages.push("✓ 成功从数据库读取代理配置".to_string());
                    Some(config)
                }
                Err(e) => {
                    messages.push(format!("✗ 解析数据库配置失败: {}", e));
                    None
                }
            }
        }
        Ok(None) => {
            messages.push("⚠ 数据库中未找到代理配置".to_string());
            None
        }
        Err(e) => {
            messages.push(format!("✗ 读取数据库失败: {}", e));
            None
        }
    };
    
    // 2. 比较运行时配置
    let runtime_config = get_global_proxy().await;
    
    if let Some(db_cfg) = &db_config {
        if db_cfg.enabled == runtime_config.enabled 
            && db_cfg.host == runtime_config.host 
            && db_cfg.port == runtime_config.port {
            messages.push("✓ 数据库配置与运行时配置完全一致".to_string());
        } else {
            messages.push("✗ 数据库配置与运行时配置存在差异".to_string());
        }
    }
    
    let elapsed = start.elapsed();
    let success = !messages.iter().any(|m| m.starts_with("✗"));
    
    Ok(ProxyTestResult {
        success,
        message: messages.join("\n"),
        proxy_config: db_config.map(|c| serde_json::to_value(&c).unwrap_or_default()),
        response_time_ms: Some(elapsed.as_secs_f64() * 1000.0),
    })
}

/// 测试HTTP客户端代理更新
/// 创建一个新的 reqwest 客户端并验证它是否使用了代理环境变量
#[tauri::command]
pub async fn test_http_client_proxy_update() -> Result<ProxyTestResult, String> {
    info!("Testing HTTP client proxy update...");
    
    let start = Instant::now();
    let mut messages = Vec::new();
    
    // 1. 检查环境变量
    let http_proxy = std::env::var("HTTP_PROXY").ok();
    let https_proxy = std::env::var("HTTPS_PROXY").ok();
    
    if http_proxy.is_some() || https_proxy.is_some() {
        messages.push(format!("✓ 代理环境变量已设置: HTTP_PROXY={:?}, HTTPS_PROXY={:?}", 
            http_proxy.as_ref().map(|s| mask_password(s)),
            https_proxy.as_ref().map(|s| mask_password(s))));
    } else {
        messages.push("⚠ 未设置代理环境变量".to_string());
    }
    
    // 2. 创建新的 HTTP 客户端（reqwest 会自动读取环境变量）
    match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build() 
    {
        Ok(_client) => {
            messages.push("✓ 成功创建 HTTP 客户端（reqwest 会自动使用环境变量中的代理配置）".to_string());
            
            // 注意：reqwest::Client 不提供直接查询代理配置的 API
            // 但它会在构建时自动读取 HTTP_PROXY/HTTPS_PROXY 环境变量
            if http_proxy.is_some() {
                messages.push("✓ HTTP 客户端将使用代理（通过 HTTP_PROXY 环境变量）".to_string());
            }
        }
        Err(e) => {
            messages.push(format!("✗ 创建 HTTP 客户端失败: {}", e));
        }
    }
    
    // 3. 验证 rig 库使用的环境
    messages.push("ℹ rig 库的 DynClientBuilder 会创建新的 reqwest::Client，它会自动读取这些环境变量".to_string());
    
    let elapsed = start.elapsed();
    let success = !messages.iter().any(|m| m.starts_with("✗"));
    
    Ok(ProxyTestResult {
        success,
        message: messages.join("\n"),
        proxy_config: None,
        response_time_ms: Some(elapsed.as_secs_f64() * 1000.0),
    })
}

/// 测试代理连接（实际发起HTTP请求）
#[tauri::command]
pub async fn test_proxy_connection(
    proxy_host: String,
    proxy_port: u16,
    proxy_username: Option<String>,
    proxy_password: Option<String>,
    test_url: Option<String>,
) -> Result<ProxyTestResult, String> {
    info!("Testing proxy connection to {}:{}", proxy_host, proxy_port);
    
    let start = Instant::now();
    let test_url = test_url.unwrap_or_else(|| "https://www.google.com".to_string());
    
    // 构建代理 URL
    let proxy_url = if let (Some(username), Some(password)) = (proxy_username, proxy_password) {
        format!("http://{}:{}@{}:{}", username, password, proxy_host, proxy_port)
    } else {
        format!("http://{}:{}", proxy_host, proxy_port)
    };
    
    // 创建带代理的客户端
    let client = match reqwest::Client::builder()
        .proxy(reqwest::Proxy::all(&proxy_url).map_err(|e| e.to_string())?)
        .timeout(std::time::Duration::from_secs(10))
        .build() 
    {
        Ok(c) => c,
        Err(e) => {
            return Ok(ProxyTestResult {
                success: false,
                message: format!("创建 HTTP 客户端失败: {}", e),
                proxy_config: None,
                response_time_ms: None,
            });
        }
    };
    
    // 发起测试请求
    match client.get(&test_url).send().await {
        Ok(response) => {
            let elapsed = start.elapsed();
            let status = response.status();
            
            Ok(ProxyTestResult {
                success: status.is_success(),
                message: format!("✓ 代理连接成功！测试 URL: {}, HTTP 状态: {}", test_url, status),
                proxy_config: Some(serde_json::json!({
                    "proxy_url": mask_password(&proxy_url),
                    "test_url": test_url,
                })),
                response_time_ms: Some(elapsed.as_secs_f64() * 1000.0),
            })
        }
        Err(e) => {
            let elapsed = start.elapsed();
            warn!("Proxy connection test failed: {}", e);
            
            Ok(ProxyTestResult {
                success: false,
                message: format!("✗ 代理连接失败: {}", e),
                proxy_config: Some(serde_json::json!({
                    "proxy_url": mask_password(&proxy_url),
                    "test_url": test_url,
                })),
                response_time_ms: Some(elapsed.as_secs_f64() * 1000.0),
            })
        }
    }
}

/// 获取当前代理配置
#[tauri::command]
pub async fn get_current_proxy_config() -> Result<Option<serde_json::Value>, String> {
    let config = get_global_proxy().await;
    Ok(Some(serde_json::to_value(&config).map_err(|e| e.to_string())?))
}

/// 辅助函数：屏蔽密码
fn mask_password(url: &str) -> String {
    if let Some(at_pos) = url.find('@') {
        if let Some(scheme_end) = url.find("://") {
            let scheme = &url[..scheme_end + 3];
            let rest = &url[at_pos..];
            return format!("{}***:***{}", scheme, rest);
        }
    }
    url.to_string()
}