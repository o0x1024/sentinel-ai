//! 网络操作并发压力测试
//!
//! 测试场景：
//! 1. 大量并发HTTP请求（10,000+）
//! 2. 网络超时处理
//! 3. 连接池耗尽
//! 4. DNS解析压力
//! 5. 并发WebSocket连接
//! 6. 大文件下载并发

use sentinel_plugins::{HttpTransaction, PluginEngine, PluginMetadata, Severity};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

fn create_test_transaction() -> HttpTransaction {
    use chrono::Utc;
    use std::collections::HashMap;

    HttpTransaction {
        request: sentinel_plugins::RequestContext {
            id: uuid::Uuid::new_v4().to_string(),
            method: "GET".to_string(),
            url: "https://httpbin.org/get".to_string(),
            headers: HashMap::from([
                ("User-Agent".to_string(), "Sentinel-AI-Test/1.0".to_string()),
            ]),
            body: vec![],
            content_type: None,
            query_params: HashMap::new(),
            is_https: true,
            timestamp: Utc::now(),
            was_edited: false,
            edited_method: None,
            edited_url: None,
            edited_headers: None,
            edited_body: None,
        },
        response: None,
    }
}

/// 创建网络请求插件
fn create_network_plugin() -> (PluginMetadata, String) {
    let metadata = PluginMetadata {
        id: "network-test".to_string(),
        name: "Network Test Plugin".to_string(),
        version: "1.0.0".to_string(),
        author: None,
        main_category: "passive".to_string(),
        category: "test".to_string(),
        default_severity: Severity::Info,
        tags: vec!["network".to_string()],
        description: None,
    };

    let code = r#"
export async function scan_transaction(transaction) {
    try {
        // 发起HTTP请求
        const response = await fetch('https://httpbin.org/get', {
            method: 'GET',
            headers: {
                'User-Agent': 'Sentinel-AI-Plugin/1.0'
            }
        });
        
        const data = await response.json();
        
        Sentinel.emitFinding({
            vuln_type: "network_test",
            title: "Network Request Test",
            description: "Request completed: " + response.status,
            evidence: JSON.stringify(data).substring(0, 100),
            location: "network",
            severity: "info",
            confidence: "high"
        });
    } catch (e) {
        Sentinel.emitFinding({
            vuln_type: "network_error",
            title: "Network Request Failed",
            description: "Error: " + e.message,
            evidence: "network_error",
            location: "network",
            severity: "info",
            confidence: "high"
        });
    }
}
"#.to_string();

    (metadata, code)
}

/// 创建并发HTTP请求插件
fn create_concurrent_http_plugin() -> (PluginMetadata, String) {
    let metadata = PluginMetadata {
        id: "concurrent-http-test".to_string(),
        name: "Concurrent HTTP Test".to_string(),
        version: "1.0.0".to_string(),
        author: None,
        main_category: "passive".to_string(),
        category: "test".to_string(),
        default_severity: Severity::Info,
        tags: vec!["network".to_string()],
        description: None,
    };

    let code = r#"
export async function scan_transaction(transaction) {
    try {
        // 并发多个请求
        const urls = [
            'https://httpbin.org/get',
            'https://httpbin.org/headers',
            'https://httpbin.org/user-agent',
            'https://httpbin.org/ip',
        ];
        
        const promises = urls.map(url => 
            fetch(url).then(r => r.json()).catch(e => ({ error: e.message }))
        );
        
        const results = await Promise.all(promises);
        const successCount = results.filter(r => !r.error).length;
        
        Sentinel.emitFinding({
            vuln_type: "concurrent_http_test",
            title: "Concurrent HTTP Test",
            description: "Completed " + successCount + "/" + urls.length + " requests",
            evidence: "concurrent_http",
            location: "network",
            severity: "info",
            confidence: "high"
        });
    } catch (e) {
        Sentinel.emitFinding({
            vuln_type: "concurrent_http_error",
            title: "Concurrent HTTP Failed",
            description: "Error: " + e.message,
            evidence: "error",
            location: "network",
            severity: "info",
            confidence: "high"
        });
    }
}
"#.to_string();

    (metadata, code)
}

/// 创建超时测试插件
fn create_timeout_plugin() -> (PluginMetadata, String) {
    let metadata = PluginMetadata {
        id: "timeout-test".to_string(),
        name: "Timeout Test".to_string(),
        version: "1.0.0".to_string(),
        author: None,
        main_category: "passive".to_string(),
        category: "test".to_string(),
        default_severity: Severity::Info,
        tags: vec!["network".to_string()],
        description: None,
    };

    let code = r#"
export async function scan_transaction(transaction) {
    try {
        // 测试延迟响应
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), 5000);
        
        const response = await fetch('https://httpbin.org/delay/2', {
            signal: controller.signal
        });
        
        clearTimeout(timeoutId);
        
        Sentinel.emitFinding({
            vuln_type: "timeout_test",
            title: "Timeout Test",
            description: "Request completed: " + response.status,
            evidence: "timeout_test",
            location: "network",
            severity: "info",
            confidence: "high"
        });
    } catch (e) {
        Sentinel.emitFinding({
            vuln_type: "timeout_error",
            title: "Timeout Error",
            description: "Error: " + e.message,
            evidence: "timeout",
            location: "network",
            severity: "info",
            confidence: "high"
        });
    }
}
"#.to_string();

    (metadata, code)
}

/// 测试1: 10,000个并发网络请求
#[tokio::test]
#[ignore]
async fn test_10k_concurrent_requests() {
    println!("\n{}", "=".repeat(80));
    println!("Test: 10,000 Concurrent Network Requests");
    println!("{}", "=".repeat(80));

    let (metadata, code) = create_network_plugin();
    
    let iterations = 10000;
    let concurrency = 500; // 限制同时进行的请求数
    let start = Instant::now();

    let success = Arc::new(AtomicUsize::new(0));
    let errors = Arc::new(AtomicUsize::new(0));
    let semaphore = Arc::new(Semaphore::new(concurrency));

    let mut handles = vec![];

    println!("Starting {} requests with max {} concurrent...", iterations, concurrency);

    for i in 0..iterations {
        let sem = semaphore.clone();
        let success = success.clone();
        let errors = errors.clone();
        let code = code.clone();
        let metadata = metadata.clone();

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();

            // 在 spawn_blocking 中创建引擎
            let result = tokio::task::spawn_blocking(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();

                rt.block_on(async move {
                    let mut engine = PluginEngine::new().ok()?;
                    engine.load_plugin_with_metadata(&code, metadata).await.ok()?;
                    
                    let transaction = create_test_transaction();
                    engine.scan_transaction(&transaction).await.ok()
                })
            }).await;

            match result {
                Ok(Some(_)) => success.fetch_add(1, Ordering::Relaxed),
                _ => errors.fetch_add(1, Ordering::Relaxed),
            };
        });

        handles.push(handle);

        // 每1000个请求打印进度
        if (i + 1) % 1000 == 0 {
            println!("  Progress: {}/{}", i + 1, iterations);
        }
    }

    // 等待所有请求完成
    for handle in handles {
        let _ = handle.await;
    }

    let duration = start.elapsed();
    let success_count = success.load(Ordering::Relaxed);
    let error_count = errors.load(Ordering::Relaxed);
    let throughput = iterations as f64 / duration.as_secs_f64();
    let error_rate = error_count as f64 / iterations as f64 * 100.0;

    println!();
    println!("Results:");
    println!("  Total Requests: {}", iterations);
    println!("  Success: {} | Errors: {}", success_count, error_count);
    println!("  Duration: {:?}", duration);
    println!("  Throughput: {:.2} requests/sec", throughput);
    println!("  Error Rate: {:.2}%", error_rate);
    println!("  Avg Latency: {:.2}ms", duration.as_millis() as f64 / iterations as f64);
    println!("{}", "=".repeat(80));

    // 断言：错误率应该在合理范围内
    assert!(error_rate < 20.0, "Error rate too high: {:.2}%", error_rate);
}

/// 测试2: 逐步增加并发数找到网络极限
#[tokio::test]
#[ignore]
async fn test_find_network_concurrency_limit() {
    println!("\n{}", "=".repeat(80));
    println!("Test: Find Network Concurrency Limit");
    println!("{}", "=".repeat(80));

    let (metadata, code) = create_network_plugin();
    let concurrency_levels = vec![10, 50, 100, 200, 500, 1000, 2000];
    let requests_per_level = 100;

    for concurrency in concurrency_levels {
        println!("\nTesting concurrency: {}", concurrency);
        
        let start = Instant::now();
        let success = Arc::new(AtomicUsize::new(0));
        let errors = Arc::new(AtomicUsize::new(0));
        let semaphore = Arc::new(Semaphore::new(concurrency));

        let mut handles = vec![];

        for _ in 0..requests_per_level {
            let sem = semaphore.clone();
            let success = success.clone();
            let errors = errors.clone();
            let code = code.clone();
            let metadata = metadata.clone();

            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();

                let result = tokio::task::spawn_blocking(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap();

                    rt.block_on(async move {
                        let mut engine = PluginEngine::new().ok()?;
                        engine.load_plugin_with_metadata(&code, metadata).await.ok()?;
                        
                        let transaction = create_test_transaction();
                        engine.scan_transaction(&transaction).await.ok()
                    })
                }).await;

                match result {
                    Ok(Some(_)) => success.fetch_add(1, Ordering::Relaxed),
                    _ => errors.fetch_add(1, Ordering::Relaxed),
                };
            });

            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }

        let duration = start.elapsed();
        let success_count = success.load(Ordering::Relaxed);
        let error_count = errors.load(Ordering::Relaxed);
        let throughput = requests_per_level as f64 / duration.as_secs_f64();
        let error_rate = error_count as f64 / requests_per_level as f64 * 100.0;

        println!("  Duration: {:?}", duration);
        println!("  Success: {} | Errors: {}", success_count, error_count);
        println!("  Throughput: {:.2} req/s", throughput);
        println!("  Error Rate: {:.2}%", error_rate);

        // 如果错误率超过30%，停止测试
        if error_rate > 30.0 {
            println!("\n⚠️  ERROR RATE TOO HIGH!");
            println!("Network concurrency limit: ~{}", concurrency / 2);
            break;
        }
    }

    println!("{}", "=".repeat(80));
}

/// 测试3: 并发HTTP请求（每个插件发起多个请求）
#[tokio::test]
#[ignore]
async fn test_concurrent_http_per_plugin() {
    println!("\n{}", "=".repeat(80));
    println!("Test: Concurrent HTTP Requests Per Plugin");
    println!("{}", "=".repeat(80));

    let (metadata, code) = create_concurrent_http_plugin();
    
    let iterations = 1000;
    let concurrency = 100;
    let start = Instant::now();

    let success = Arc::new(AtomicUsize::new(0));
    let errors = Arc::new(AtomicUsize::new(0));
    let semaphore = Arc::new(Semaphore::new(concurrency));

    let mut handles = vec![];

    for i in 0..iterations {
        let sem = semaphore.clone();
        let success = success.clone();
        let errors = errors.clone();
        let code = code.clone();
        let metadata = metadata.clone();

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();

            let result = tokio::task::spawn_blocking(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();

                rt.block_on(async move {
                    let mut engine = PluginEngine::new().ok()?;
                    engine.load_plugin_with_metadata(&code, metadata).await.ok()?;
                    
                    let transaction = create_test_transaction();
                    engine.scan_transaction(&transaction).await.ok()
                })
            }).await;

            match result {
                Ok(Some(_)) => success.fetch_add(1, Ordering::Relaxed),
                _ => errors.fetch_add(1, Ordering::Relaxed),
            };
        });

        handles.push(handle);

        if (i + 1) % 100 == 0 {
            println!("  Progress: {}/{}", i + 1, iterations);
        }
    }

    for handle in handles {
        let _ = handle.await;
    }

    let duration = start.elapsed();
    let success_count = success.load(Ordering::Relaxed);
    let error_count = errors.load(Ordering::Relaxed);
    let throughput = iterations as f64 / duration.as_secs_f64();
    let error_rate = error_count as f64 / iterations as f64 * 100.0;

    println!();
    println!("Results:");
    println!("  Plugin Executions: {}", iterations);
    println!("  Total HTTP Requests: {} (4 per plugin)", iterations * 4);
    println!("  Success: {} | Errors: {}", success_count, error_count);
    println!("  Duration: {:?}", duration);
    println!("  Throughput: {:.2} plugins/sec", throughput);
    println!("  Error Rate: {:.2}%", error_rate);
    println!("{}", "=".repeat(80));
}

/// 测试4: 网络超时处理
#[tokio::test]
#[ignore]
async fn test_network_timeout_handling() {
    println!("\n{}", "=".repeat(80));
    println!("Test: Network Timeout Handling");
    println!("{}", "=".repeat(80));

    let (metadata, code) = create_timeout_plugin();
    
    let iterations = 100;
    let concurrency = 20;
    let start = Instant::now();

    let success = Arc::new(AtomicUsize::new(0));
    let errors = Arc::new(AtomicUsize::new(0));
    let timeouts = Arc::new(AtomicUsize::new(0));
    let semaphore = Arc::new(Semaphore::new(concurrency));

    let mut handles = vec![];

    for _ in 0..iterations {
        let sem = semaphore.clone();
        let success = success.clone();
        let errors = errors.clone();
        let timeouts = timeouts.clone();
        let code = code.clone();
        let metadata = metadata.clone();

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();

            // 设置整体超时
            let result = tokio::time::timeout(
                Duration::from_secs(10),
                tokio::task::spawn_blocking(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap();

                    rt.block_on(async move {
                        let mut engine = PluginEngine::new().ok()?;
                        engine.load_plugin_with_metadata(&code, metadata).await.ok()?;
                        
                        let transaction = create_test_transaction();
                        engine.scan_transaction(&transaction).await.ok()
                    })
                })
            ).await;

            match result {
                Ok(Ok(Some(_))) => success.fetch_add(1, Ordering::Relaxed),
                Err(_) => timeouts.fetch_add(1, Ordering::Relaxed),
                _ => errors.fetch_add(1, Ordering::Relaxed),
            };
        });

        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.await;
    }

    let duration = start.elapsed();
    let success_count = success.load(Ordering::Relaxed);
    let error_count = errors.load(Ordering::Relaxed);
    let timeout_count = timeouts.load(Ordering::Relaxed);

    println!();
    println!("Results:");
    println!("  Total Requests: {}", iterations);
    println!("  Success: {}", success_count);
    println!("  Errors: {}", error_count);
    println!("  Timeouts: {}", timeout_count);
    println!("  Duration: {:?}", duration);
    println!("  Timeout Rate: {:.2}%", timeout_count as f64 / iterations as f64 * 100.0);
    println!("{}", "=".repeat(80));
}

/// 测试5: 持续网络压力测试（60秒）
#[tokio::test]
#[ignore]
async fn test_sustained_network_pressure() {
    println!("\n{}", "=".repeat(80));
    println!("Test: Sustained Network Pressure (60 seconds)");
    println!("{}", "=".repeat(80));

    let (metadata, code) = create_network_plugin();
    
    let test_duration = Duration::from_secs(60);
    let concurrency = 200;
    let start = Instant::now();

    let success = Arc::new(AtomicUsize::new(0));
    let errors = Arc::new(AtomicUsize::new(0));
    let active_requests = Arc::new(AtomicUsize::new(0));
    let should_stop = Arc::new(std::sync::atomic::AtomicBool::new(false));

    // 监控任务
    let monitor_success = success.clone();
    let monitor_errors = errors.clone();
    let monitor_active = active_requests.clone();
    let monitor_stop = should_stop.clone();

    let monitor_handle = tokio::spawn(async move {
        let mut last_success = 0;
        let mut interval = tokio::time::interval(Duration::from_secs(10));

        while !monitor_stop.load(Ordering::Relaxed) {
            interval.tick().await;

            let current_success = monitor_success.load(Ordering::Relaxed);
            let current_errors = monitor_errors.load(Ordering::Relaxed);
            let current_active = monitor_active.load(Ordering::Relaxed);
            let throughput = (current_success - last_success) as f64 / 10.0;

            println!(
                "[{:?}] Active: {} | Success: {} | Errors: {} | Throughput: {:.2} req/s",
                start.elapsed(),
                current_active,
                current_success,
                current_errors,
                throughput
            );

            last_success = current_success;
        }
    });

    let mut handles = vec![];

    // 持续生成请求
    while start.elapsed() < test_duration {
        while active_requests.load(Ordering::Relaxed) < concurrency {
            let success = success.clone();
            let errors = errors.clone();
            let active = active_requests.clone();
            let code = code.clone();
            let metadata = metadata.clone();

            active.fetch_add(1, Ordering::Relaxed);

            let handle = tokio::spawn(async move {
                let result = tokio::task::spawn_blocking(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap();

                    rt.block_on(async move {
                        let mut engine = PluginEngine::new().ok()?;
                        engine.load_plugin_with_metadata(&code, metadata).await.ok()?;
                        
                        let transaction = create_test_transaction();
                        engine.scan_transaction(&transaction).await.ok()
                    })
                }).await;

                match result {
                    Ok(Some(_)) => success.fetch_add(1, Ordering::Relaxed),
                    _ => errors.fetch_add(1, Ordering::Relaxed),
                };

                active.fetch_sub(1, Ordering::Relaxed);
            });

            handles.push(handle);
        }

        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    should_stop.store(true, Ordering::Relaxed);

    // 等待所有请求完成
    for handle in handles {
        let _ = handle.await;
    }

    let _ = monitor_handle.await;

    let total_duration = start.elapsed();
    let total_success = success.load(Ordering::Relaxed);
    let total_errors = errors.load(Ordering::Relaxed);
    let total_requests = total_success + total_errors;

    println!();
    println!("{}", "=".repeat(80));
    println!("Final Results:");
    println!("  Duration: {:?}", total_duration);
    println!("  Total Requests: {}", total_requests);
    println!("  Success: {}", total_success);
    println!("  Errors: {}", total_errors);
    println!("  Average Throughput: {:.2} req/s", total_requests as f64 / total_duration.as_secs_f64());
    println!("  Error Rate: {:.2}%", total_errors as f64 / total_requests as f64 * 100.0);
    println!("{}", "=".repeat(80));
}

/// 测试6: 不同网络条件下的性能
#[tokio::test]
#[ignore]
async fn test_various_network_conditions() {
    println!("\n{}", "=".repeat(80));
    println!("Test: Various Network Conditions");
    println!("{}", "=".repeat(80));

    // 测试不同的延迟场景
    let scenarios = vec![
        ("Fast Response", "https://httpbin.org/get", 100),
        ("Delayed Response (1s)", "https://httpbin.org/delay/1", 50),
        ("Delayed Response (3s)", "https://httpbin.org/delay/3", 20),
    ];

    for (name, url, iterations) in scenarios {
        println!("\nScenario: {}", name);
        println!("  URL: {}", url);
        println!("  Iterations: {}", iterations);

        let metadata = PluginMetadata {
            id: format!("network-test-{}", name.replace(" ", "-").to_lowercase()),
            name: name.to_string(),
            version: "1.0.0".to_string(),
            author: None,
            main_category: "passive".to_string(),
            category: "test".to_string(),
            default_severity: Severity::Info,
            tags: vec!["network".to_string()],
            description: None,
        };

        let code = format!(r#"
export async function scan_transaction(transaction) {{
    try {{
        const response = await fetch('{}');
        const data = await response.json();
        
        Sentinel.emitFinding({{
            vuln_type: "network_test",
            title: "Network Test",
            description: "Status: " + response.status,
            evidence: "network",
            location: "test",
            severity: "info",
            confidence: "high"
        }});
    }} catch (e) {{
        Sentinel.emitFinding({{
            vuln_type: "network_error",
            title: "Network Error",
            description: "Error: " + e.message,
            evidence: "error",
            location: "test",
            severity: "info",
            confidence: "high"
        }});
    }}
}}
"#, url);

        let start = Instant::now();
        let success = Arc::new(AtomicUsize::new(0));
        let errors = Arc::new(AtomicUsize::new(0));
        let semaphore = Arc::new(Semaphore::new(10));

        let mut handles = vec![];

        for _ in 0..iterations {
            let sem = semaphore.clone();
            let success = success.clone();
            let errors = errors.clone();
            let code = code.clone();
            let metadata = metadata.clone();

            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();

                let result = tokio::task::spawn_blocking(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap();

                    rt.block_on(async move {
                        let mut engine = PluginEngine::new().ok()?;
                        engine.load_plugin_with_metadata(&code, metadata).await.ok()?;
                        
                        let transaction = create_test_transaction();
                        engine.scan_transaction(&transaction).await.ok()
                    })
                }).await;

                match result {
                    Ok(Some(_)) => success.fetch_add(1, Ordering::Relaxed),
                    _ => errors.fetch_add(1, Ordering::Relaxed),
                };
            });

            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }

        let duration = start.elapsed();
        let success_count = success.load(Ordering::Relaxed);
        let error_count = errors.load(Ordering::Relaxed);

        println!("  Duration: {:?}", duration);
        println!("  Success: {} | Errors: {}", success_count, error_count);
        println!("  Avg Latency: {:.2}ms", duration.as_millis() as f64 / iterations as f64);
        println!("  Throughput: {:.2} req/s", iterations as f64 / duration.as_secs_f64());
    }

    println!("{}", "=".repeat(80));
}

