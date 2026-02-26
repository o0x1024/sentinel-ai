//! 代理配置测试模块

use sentinel_core::global_proxy::get_global_proxy;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tracing::{info, warn};

#[derive(Debug, Serialize, Deserialize)]
pub struct ProxyTestResult {
    pub success: bool,
    pub message: String,
    pub proxy_config: Option<serde_json::Value>,
    pub response_time_ms: Option<f64>,
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
        format!(
            "http://{}:{}@{}:{}",
            username, password, proxy_host, proxy_port
        )
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
                message: format!(
                    "✓ 代理连接成功！测试 URL: {}, HTTP 状态: {}",
                    test_url, status
                ),
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
    Ok(Some(
        serde_json::to_value(&config).map_err(|e| e.to_string())?,
    ))
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
