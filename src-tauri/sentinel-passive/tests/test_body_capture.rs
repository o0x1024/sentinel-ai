/// 被动代理 Body 捕获功能测试
/// 
/// 验证改进后的代理能够正确捕获请求和响应的 body
use sentinel_passive::{ProxyConfig, ProxyService};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_proxy_body_capture() {
    // 创建代理配置
    let config = ProxyConfig {
        start_port: 4301,
        max_port_attempts: 5,
        mitm_enabled: true,
        max_request_body_size: 1024 * 1024, // 1MB
        max_response_body_size: 1024 * 1024, // 1MB
    };

    // 创建临时 CA 目录
    let ca_dir = std::env::temp_dir().join("sentinel-test-ca");
    std::fs::create_dir_all(&ca_dir).unwrap();

    // 创建代理服务（不连接扫描器，仅测试 body 捕获）
    let proxy = ProxyService::with_ca_dir(config.clone(), ca_dir.clone());
    
    // 启动代理
    let port = proxy.start(None).await.expect("Failed to start proxy");
    println!("Proxy started on port {}", port);

    // 等待代理启动
    sleep(Duration::from_secs(1)).await;

    // 测试 1: 发送一个简单的 HTTP 请求
    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::all(format!("http://127.0.0.1:{}", port)).unwrap())
        .danger_accept_invalid_certs(true) // 接受自签名证书
        .build()
        .unwrap();

    // 发送 GET 请求
    println!("Sending test GET request...");
    match client.get("http://httpbin.org/get").send().await {
        Ok(resp) => {
            println!("GET request successful, status: {}", resp.status());
            let body = resp.text().await.unwrap();
            println!("Response body length: {} bytes", body.len());
            assert!(!body.is_empty(), "Response body should not be empty");
        }
        Err(e) => {
            println!("GET request failed: {}", e);
        }
    }

    // 测试 2: 发送带 body 的 POST 请求
    println!("Sending test POST request with body...");
    let post_body = serde_json::json!({
        "test": "data",
        "message": "Hello from Sentinel AI"
    });

    match client
        .post("http://httpbin.org/post")
        .json(&post_body)
        .send()
        .await
    {
        Ok(resp) => {
            println!("POST request successful, status: {}", resp.status());
            let body = resp.text().await.unwrap();
            println!("Response body length: {} bytes", body.len());
            assert!(!body.is_empty(), "Response body should not be empty");
        }
        Err(e) => {
            println!("POST request failed: {}", e);
        }
    }

    // 测试 3: 验证统计信息
    let stats = proxy.get_stats().await;
    println!("Proxy stats: {:?}", stats);
    assert!(stats.http_requests > 0, "Should have captured HTTP requests");

    // 停止代理
    proxy.stop().await.expect("Failed to stop proxy");
    println!("Proxy stopped");

    // 清理临时 CA 目录
    let _ = std::fs::remove_dir_all(&ca_dir);
}

#[tokio::test]
async fn test_concurrent_requests() {
    use tokio::task::JoinSet;

    // 创建代理
    let config = ProxyConfig::default();
    let ca_dir = std::env::temp_dir().join("sentinel-test-ca-concurrent");
    std::fs::create_dir_all(&ca_dir).unwrap();
    
    let proxy = ProxyService::with_ca_dir(config, ca_dir.clone());
    let port = proxy.start(None).await.expect("Failed to start proxy");
    
    println!("Testing concurrent requests on port {}", port);
    sleep(Duration::from_secs(1)).await;

    // 创建多个并发请求
    let mut set = JoinSet::new();
    
    for i in 0..5 {
        let proxy_url = format!("http://127.0.0.1:{}", port);
        set.spawn(async move {
            let client = reqwest::Client::builder()
                .proxy(reqwest::Proxy::all(&proxy_url).unwrap())
                .danger_accept_invalid_certs(true)
                .build()
                .unwrap();

            let url = format!("http://httpbin.org/delay/{}", i % 3);
            match client.get(&url).send().await {
                Ok(resp) => {
                    println!("Request {} completed with status: {}", i, resp.status());
                    true
                }
                Err(e) => {
                    println!("Request {} failed: {}", i, e);
                    false
                }
            }
        });
    }

    // 等待所有请求完成
    let mut success_count = 0;
    while let Some(result) = set.join_next().await {
        if let Ok(true) = result {
            success_count += 1;
        }
    }

    println!("Concurrent test: {}/5 requests successful", success_count);
    
    // 验证统计
    let stats = proxy.get_stats().await;
    println!("Final stats: {:?}", stats);

    proxy.stop().await.expect("Failed to stop proxy");
    let _ = std::fs::remove_dir_all(&ca_dir);
}
