//! 快速示例：展示如何运行和分析压力测试
//!
//! 这个文件包含简化的示例测试，用于快速验证系统性能

use sentinel_plugins::{HttpTransaction, PluginEngine, PluginExecutor, PluginMetadata, Severity};
use std::time::Instant;

fn create_simple_transaction() -> HttpTransaction {
    use chrono::Utc;
    use std::collections::HashMap;

    HttpTransaction {
        request: sentinel_plugins::RequestContext {
            id: uuid::Uuid::new_v4().to_string(),
            method: "GET".to_string(),
            url: "https://example.com/test".to_string(),
            headers: HashMap::new(),
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

/// 示例1: 基本性能测试
/// 
/// 测试单个插件引擎的基本性能
#[tokio::test]
#[ignore]
async fn example_basic_performance() {
    println!("\n{}", "=".repeat(80));
    println!("Example 1: Basic Performance Test");
    println!("{}", "=".repeat(80));

    let metadata = PluginMetadata {
        id: "example-plugin".to_string(),
        name: "Example Plugin".to_string(),
        version: "1.0.0".to_string(),
        author: None,
        main_category: "passive".to_string(),
        category: "test".to_string(),
        default_severity: Severity::Info,
        tags: vec![],
        description: None,
    };

    let code = r#"
export function scan_transaction(transaction) {
    const url = transaction.request.url;
    if (url.includes("test")) {
        Sentinel.emitFinding({
            vuln_type: "test",
            title: "Test Finding",
            description: "Found test URL",
            evidence: url,
            location: "url",
            severity: "info",
            confidence: "high"
        });
    }
}
"#;

    let mut engine = PluginEngine::new().unwrap();
    engine.load_plugin_with_metadata(code, metadata).await.unwrap();

    let iterations = 100;
    let start = Instant::now();
    let mut success = 0;
    let mut errors = 0;

    for _ in 0..iterations {
        let transaction = create_simple_transaction();
        match engine.scan_transaction(&transaction).await {
            Ok(_) => success += 1,
            Err(_) => errors += 1,
        }
    }

    let duration = start.elapsed();
    let throughput = iterations as f64 / duration.as_secs_f64();

    println!("Results:");
    println!("  Iterations: {}", iterations);
    println!("  Success: {} | Errors: {}", success, errors);
    println!("  Duration: {:?}", duration);
    println!("  Throughput: {:.2} ops/sec", throughput);
    println!();

    // 性能断言
    assert!(throughput > 10.0, "Throughput too low: {:.2} ops/sec", throughput);
    assert_eq!(errors, 0, "Should have no errors");
}

/// 示例2: 并发性能测试
/// 
/// 测试并发执行的性能和稳定性
#[tokio::test]
#[ignore]
async fn example_concurrent_performance() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tokio::sync::Semaphore;

    println!("\n{}", "=".repeat(80));
    println!("Example 2: Concurrent Performance Test");
    println!("{}", "=".repeat(80));

    let metadata = PluginMetadata {
        id: "example-concurrent".to_string(),
        name: "Concurrent Example".to_string(),
        version: "1.0.0".to_string(),
        author: None,
        main_category: "passive".to_string(),
        category: "test".to_string(),
        default_severity: Severity::Info,
        tags: vec![],
        description: None,
    };

    let code = r#"
export function scan_transaction(transaction) {
    // 模拟一些处理
    const data = Array.from({ length: 1000 }, (_, i) => i);
    const sum = data.reduce((a, b) => a + b, 0);
    
    Sentinel.emitFinding({
        vuln_type: "test",
        title: "Concurrent Test",
        description: "Sum: " + sum,
        evidence: "concurrent",
        location: "test",
        severity: "info",
        confidence: "high"
    });
}
"#;

    let concurrency = 20;
    let iterations = 100;
    let start = Instant::now();

    let success = Arc::new(AtomicUsize::new(0));
    let errors = Arc::new(AtomicUsize::new(0));
    let semaphore = Arc::new(Semaphore::new(concurrency));

    let mut handles = vec![];

    for _ in 0..iterations {
        let sem = semaphore.clone();
        let success = success.clone();
        let errors = errors.clone();
        let code = code.to_string();
        let metadata = metadata.clone();

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();

            let transaction = create_simple_transaction();
            let executor = match PluginExecutor::new(metadata, code, 1000) {
                Ok(e) => e,
                Err(_) => {
                    errors.fetch_add(1, Ordering::Relaxed);
                    return;
                }
            };

            match executor.scan_transaction(transaction).await {
                Ok(_) => success.fetch_add(1, Ordering::Relaxed),
                Err(_) => errors.fetch_add(1, Ordering::Relaxed),
            };
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let duration = start.elapsed();
    let success_count = success.load(Ordering::Relaxed);
    let error_count = errors.load(Ordering::Relaxed);
    let throughput = iterations as f64 / duration.as_secs_f64();
    let error_rate = error_count as f64 / iterations as f64 * 100.0;

    println!("Results:");
    println!("  Concurrency: {}", concurrency);
    println!("  Iterations: {}", iterations);
    println!("  Success: {} | Errors: {}", success_count, error_count);
    println!("  Duration: {:?}", duration);
    println!("  Throughput: {:.2} ops/sec", throughput);
    println!("  Error Rate: {:.2}%", error_rate);
    println!();

    // 性能断言
    assert!(error_rate < 5.0, "Error rate too high: {:.2}%", error_rate);
    assert!(throughput > 5.0, "Throughput too low: {:.2} ops/sec", throughput);
}

/// 示例3: 内存使用监控
/// 
/// 展示如何监控内存使用情况
#[tokio::test]
#[ignore]
async fn example_memory_monitoring() {
    use sysinfo::System;

    println!("\n{}", "=".repeat(80));
    println!("Example 3: Memory Monitoring");
    println!("{}", "=".repeat(80));

    let mut system = System::new_all();
    let pid = sysinfo::get_current_pid().unwrap();

    let metadata = PluginMetadata {
        id: "example-memory".to_string(),
        name: "Memory Example".to_string(),
        version: "1.0.0".to_string(),
        author: None,
        main_category: "passive".to_string(),
        category: "test".to_string(),
        default_severity: Severity::Info,
        tags: vec![],
        description: None,
    };

    let code = r#"
export function scan_transaction(transaction) {
    // 分配一些内存
    const data = new Array(10000).fill({ value: "test".repeat(10) });
    
    Sentinel.emitFinding({
        vuln_type: "test",
        title: "Memory Test",
        description: "Allocated " + data.length + " items",
        evidence: "memory",
        location: "test",
        severity: "info",
        confidence: "high"
    });
}
"#;

    let mut engine = PluginEngine::new().unwrap();
    engine.load_plugin_with_metadata(code, metadata).await.unwrap();

    // 获取初始内存
    system.refresh_process(pid);
    let initial_memory = system.process(pid).unwrap().memory() as f64 / 1024.0 / 1024.0;
    println!("Initial Memory: {:.2} MB", initial_memory);

    let iterations = 50;
    let mut memory_samples = vec![];

    for i in 0..iterations {
        let transaction = create_simple_transaction();
        let _ = engine.scan_transaction(&transaction).await;

        if i % 10 == 0 {
            system.refresh_process(pid);
            let current_memory = system.process(pid).unwrap().memory() as f64 / 1024.0 / 1024.0;
            memory_samples.push(current_memory);
            println!("  Iteration {}: {:.2} MB", i, current_memory);
        }
    }

    // 获取最终内存
    system.refresh_process(pid);
    let final_memory = system.process(pid).unwrap().memory() as f64 / 1024.0 / 1024.0;

    let peak_memory = memory_samples.iter().cloned().fold(0.0f64, f64::max);
    let avg_memory = memory_samples.iter().sum::<f64>() / memory_samples.len() as f64;
    let memory_growth = final_memory - initial_memory;

    println!();
    println!("Memory Statistics:");
    println!("  Initial: {:.2} MB", initial_memory);
    println!("  Final: {:.2} MB", final_memory);
    println!("  Peak: {:.2} MB", peak_memory);
    println!("  Average: {:.2} MB", avg_memory);
    println!("  Growth: {:.2} MB", memory_growth);
    println!();

    // 内存断言（允许一定增长）
    assert!(memory_growth < 100.0, "Memory growth too high: {:.2} MB", memory_growth);
}

/// 示例4: 找到最佳并发数
/// 
/// 通过测试不同并发级别找到最佳配置
#[tokio::test]
#[ignore]
async fn example_find_optimal_concurrency() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tokio::sync::Semaphore;

    println!("\n{}", "=".repeat(80));
    println!("Example 4: Find Optimal Concurrency");
    println!("{}", "=".repeat(80));

    let metadata = PluginMetadata {
        id: "example-optimal".to_string(),
        name: "Optimal Concurrency Example".to_string(),
        version: "1.0.0".to_string(),
        author: None,
        main_category: "passive".to_string(),
        category: "test".to_string(),
        default_severity: Severity::Info,
        tags: vec![],
        description: None,
    };

    let code = r#"
export function scan_transaction(transaction) {
    const data = Array.from({ length: 1000 }, (_, i) => i);
    const sum = data.reduce((a, b) => a + b, 0);
    
    Sentinel.emitFinding({
        vuln_type: "test",
        title: "Optimal Test",
        description: "Sum: " + sum,
        evidence: "optimal",
        location: "test",
        severity: "info",
        confidence: "high"
    });
}
"#;

    let concurrency_levels = vec![5, 10, 20, 50];
    let iterations_per_level = 50;
    let mut best_throughput = 0.0;
    let mut best_concurrency = 0;

    println!("Testing different concurrency levels...\n");

    for concurrency in concurrency_levels {
        let start = Instant::now();
        let success = Arc::new(AtomicUsize::new(0));
        let errors = Arc::new(AtomicUsize::new(0));
        let semaphore = Arc::new(Semaphore::new(concurrency));

        let mut handles = vec![];

        for _ in 0..iterations_per_level {
            let sem = semaphore.clone();
            let success = success.clone();
            let errors = errors.clone();
            let code = code.to_string();
            let metadata = metadata.clone();

            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();

                let transaction = create_simple_transaction();
                let executor = match PluginExecutor::new(metadata, code, 1000) {
                    Ok(e) => e,
                    Err(_) => {
                        errors.fetch_add(1, Ordering::Relaxed);
                        return;
                    }
                };

                match executor.scan_transaction(transaction).await {
                    Ok(_) => success.fetch_add(1, Ordering::Relaxed),
                    Err(_) => errors.fetch_add(1, Ordering::Relaxed),
                };
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        let duration = start.elapsed();
        let success_count = success.load(Ordering::Relaxed);
        let error_count = errors.load(Ordering::Relaxed);
        let throughput = iterations_per_level as f64 / duration.as_secs_f64();
        let error_rate = error_count as f64 / iterations_per_level as f64 * 100.0;

        println!("Concurrency: {}", concurrency);
        println!("  Duration: {:?}", duration);
        println!("  Throughput: {:.2} ops/sec", throughput);
        println!("  Error Rate: {:.2}%", error_rate);

        if error_rate < 5.0 && throughput > best_throughput {
            best_throughput = throughput;
            best_concurrency = concurrency;
        }

        println!();
    }

    println!("{}", "=".repeat(80));
    println!("Optimal Configuration:");
    println!("  Concurrency: {}", best_concurrency);
    println!("  Throughput: {:.2} ops/sec", best_throughput);
    println!("{}", "=".repeat(80));
}

