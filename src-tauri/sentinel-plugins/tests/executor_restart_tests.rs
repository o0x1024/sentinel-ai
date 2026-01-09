//! 执行器重启机制测试
//!
//! 验证 PluginExecutor 的重启功能能有效防止内存泄漏

use sentinel_plugins::{
    HttpTransaction, PluginExecutor, PluginMetadata, Severity,
};
use std::time::{Duration, Instant};
use sysinfo::System;

fn create_test_transaction() -> HttpTransaction {
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

fn create_simple_plugin() -> (PluginMetadata, String) {
    let metadata = PluginMetadata {
        id: "restart-test".to_string(),
        name: "Restart Test Plugin".to_string(),
        version: "1.0.0".to_string(),
        author: None,
        main_category: "traffic".to_string(),
        category: "test".to_string(),
        default_severity: Severity::Info,
        tags: vec![],
        description: None,
    };

    let code = r#"
export function scan_transaction(transaction) {
    const url = transaction.request.url;
    return [{
        vuln_type: "test",
        title: "Test Finding",
        description: "Test",
        evidence: url,
        location: "url",
        severity: "info",
        confidence: "high"
    }];
}
"#
    .to_string();

    (metadata, code)
}

/// 测试1: 验证自动重启功能
#[tokio::test]
#[ignore]
async fn test_auto_restart_functionality() {
    println!("\n{}", "=".repeat(80));
    println!("Test: Auto Restart Functionality");
    println!("{}", "=".repeat(80));

    let (metadata, code) = create_simple_plugin();
    let restart_threshold = 100;
    let executor =
        PluginExecutor::new(metadata, code, restart_threshold).unwrap();

    println!("Restart threshold: {} executions", restart_threshold);

    // 执行足够多次以触发多次重启
    let iterations = 350;
    for i in 0..iterations {
        let transaction = create_test_transaction();
        let _ = executor.scan_transaction(transaction).await;

        if (i + 1) % 50 == 0 {
            let stats = executor.get_stats().await.unwrap();
            println!(
                "  Progress: {}/{} | Restarts: {} | Current instance: {}",
                i + 1,
                iterations,
                stats.restart_count,
                stats.current_instance_executions
            );
        }
    }

    let stats = executor.get_stats().await.unwrap();
    println!();
    println!("Final Statistics:");
    println!("  Total executions: {}", stats.total_executions);
    println!("  Restart count: {}", stats.restart_count);
    println!("  Current instance executions: {}", stats.current_instance_executions);

    // 验证重启次数
    let expected_restarts = iterations / restart_threshold;
    assert!(
        stats.restart_count >= expected_restarts,
        "Expected at least {} restarts, got {}",
        expected_restarts,
        stats.restart_count
    );

    println!("{}", "=".repeat(80));
}

/// 测试2: 对比有无重启的内存使用
#[tokio::test]
#[ignore]
async fn test_restart_memory_comparison() {
    println!("\n{}", "=".repeat(80));
    println!("Test: Memory Usage With/Without Restart");
    println!("{}", "=".repeat(80));

    let mut system = System::new_all();
    let pid = sysinfo::get_current_pid().unwrap();

    // 测试场景1: 不重启（模拟旧的 PluginExecutor）
    println!("\nScenario 1: Without Restart (1000 executions)");
    {
        let (metadata, code) = create_simple_plugin();
        let executor =
            PluginExecutor::new(metadata, code, 999999).unwrap(); // 永不重启

        system.refresh_process(pid);
        let start_mem = system.process(pid).unwrap().memory() as f64 / 1024.0 / 1024.0;
        println!("  Start memory: {:.2} MB", start_mem);

        let start = Instant::now();
        for _ in 0..1000 {
            let transaction = create_test_transaction();
            let _ = executor.scan_transaction(transaction).await;
        }
        let duration = start.elapsed();

        system.refresh_process(pid);
        let end_mem = system.process(pid).unwrap().memory() as f64 / 1024.0 / 1024.0;
        let growth = end_mem - start_mem;

        println!("  End memory: {:.2} MB", end_mem);
        println!("  Growth: {:.2} MB", growth);
        println!("  Duration: {:?}", duration);
    }

    // 等待一下，让系统稳定
    tokio::time::sleep(Duration::from_secs(2)).await;

    // 测试场景2: 定期重启
    println!("\nScenario 2: With Restart Every 100 Executions");
    {
        let (metadata, code) = create_simple_plugin();
        let executor = PluginExecutor::new(metadata, code, 100).unwrap();

        system.refresh_process(pid);
        let start_mem = system.process(pid).unwrap().memory() as f64 / 1024.0 / 1024.0;
        println!("  Start memory: {:.2} MB", start_mem);

        let start = Instant::now();
        for _ in 0..1000 {
            let transaction = create_test_transaction();
            let _ = executor.scan_transaction(transaction).await;
        }
        let duration = start.elapsed();

        system.refresh_process(pid);
        let end_mem = system.process(pid).unwrap().memory() as f64 / 1024.0 / 1024.0;
        let growth = end_mem - start_mem;

        let stats = executor.get_stats().await.unwrap();
        println!("  End memory: {:.2} MB", end_mem);
        println!("  Growth: {:.2} MB", growth);
        println!("  Duration: {:?}", duration);
        println!("  Restarts: {}", stats.restart_count);
    }

    println!("{}", "=".repeat(80));
}

/// 测试3: 长时间运行测试（有重启）
#[tokio::test]
#[ignore]
async fn test_long_running_with_restart() {
    println!("\n{}", "=".repeat(80));
    println!("Test: Long Running With Restart (30 seconds)");
    println!("{}", "=".repeat(80));

    let (metadata, code) = create_simple_plugin();
    let executor = PluginExecutor::new(metadata, code, 100).unwrap();

    let mut system = System::new_all();
    let pid = sysinfo::get_current_pid().unwrap();

    system.refresh_process(pid);
    let start_mem = system.process(pid).unwrap().memory() as f64 / 1024.0 / 1024.0;

    let start = Instant::now();
    let test_duration = Duration::from_secs(30);
    let mut iteration = 0;
    let mut memory_samples = Vec::new();

    println!("Starting 30-second test with restart every 100 executions...");

    while start.elapsed() < test_duration {
        let transaction = create_test_transaction();
        let _ = executor.scan_transaction(transaction).await;

        iteration += 1;

        // 每100次采样一次
        if iteration % 100 == 0 {
            system.refresh_process(pid);
            let current_mem = system.process(pid).unwrap().memory() as f64 / 1024.0 / 1024.0;
            memory_samples.push(current_mem);

            let stats = executor.get_stats().await.unwrap();
            println!(
                "  [{:?}] Iterations: {} | Memory: {:.2} MB | Restarts: {}",
                start.elapsed(),
                iteration,
                current_mem,
                stats.restart_count
            );
        }
    }

    let duration = start.elapsed();
    system.refresh_process(pid);
    let end_mem = system.process(pid).unwrap().memory() as f64 / 1024.0 / 1024.0;

    let stats = executor.get_stats().await.unwrap();

    println!();
    println!("Results:");
    println!("  Duration: {:?}", duration);
    println!("  Total iterations: {}", iteration);
    println!("  Total restarts: {}", stats.restart_count);
    println!("  Start memory: {:.2} MB", start_mem);
    println!("  End memory: {:.2} MB", end_mem);
    println!("  Memory growth: {:.2} MB", end_mem - start_mem);

    // 计算内存增长率
    if memory_samples.len() >= 2 {
        let first = memory_samples[0];
        let last = *memory_samples.last().unwrap();
        let growth_rate = (last - first) / duration.as_secs_f64();
        println!("  Growth rate: {:.4} MB/s", growth_rate);

        // 使用更宽松的阈值，因为重启有成本
        if growth_rate < 10.0 {
            println!("  ✓ Memory growth is acceptable with restart mechanism");
        } else {
            println!("  ⚠️  Memory growth is still high even with restart");
        }
    }

    println!("{}", "=".repeat(80));
}

/// 测试4: 手动重启功能
#[tokio::test]
#[ignore]
async fn test_manual_restart() {
    println!("\n{}", "=".repeat(80));
    println!("Test: Manual Restart");
    println!("{}", "=".repeat(80));

    let (metadata, code) = create_simple_plugin();
    let executor = PluginExecutor::new(metadata, code, 1000).unwrap();

    // 执行一些任务
    println!("Executing 50 tasks...");
    for _ in 0..50 {
        let transaction = create_test_transaction();
        let _ = executor.scan_transaction(transaction).await;
    }

    let stats_before = executor.get_stats().await.unwrap();
    println!("Before restart:");
    println!("  Total executions: {}", stats_before.total_executions);
    println!("  Current instance: {}", stats_before.current_instance_executions);
    println!("  Restarts: {}", stats_before.restart_count);

    // 手动重启
    println!("\nTriggering manual restart...");
    executor.restart().await.unwrap();

    // 再执行一些任务
    println!("Executing 50 more tasks...");
    for _ in 0..50 {
        let transaction = create_test_transaction();
        let _ = executor.scan_transaction(transaction).await;
    }

    let stats_after = executor.get_stats().await.unwrap();
    println!("\nAfter restart:");
    println!("  Total executions: {}", stats_after.total_executions);
    println!("  Current instance: {}", stats_after.current_instance_executions);
    println!("  Restarts: {}", stats_after.restart_count);

    // 验证
    assert_eq!(stats_after.total_executions, 100);
    assert_eq!(stats_after.restart_count, 1);
    assert_eq!(stats_after.current_instance_executions, 50);

    println!("\n✓ Manual restart works correctly");
    println!("{}", "=".repeat(80));
}

/// 测试5: 不同重启阈值的对比
#[tokio::test]
#[ignore]
async fn test_different_restart_thresholds() {
    println!("\n{}", "=".repeat(80));
    println!("Test: Different Restart Thresholds");
    println!("{}", "=".repeat(80));

    let thresholds = vec![50, 100, 200, 500];
    let iterations = 1000;

    for threshold in thresholds {
        println!("\nTesting threshold: {}", threshold);

        let (metadata, code) = create_simple_plugin();
        let executor =
            PluginExecutor::new(metadata, code, threshold).unwrap();

        let start = Instant::now();
        for _ in 0..iterations {
            let transaction = create_test_transaction();
            let _ = executor.scan_transaction(transaction).await;
        }
        let duration = start.elapsed();

        let stats = executor.get_stats().await.unwrap();
        let throughput = iterations as f64 / duration.as_secs_f64();

        println!("  Duration: {:?}", duration);
        println!("  Restarts: {}", stats.restart_count);
        println!("  Throughput: {:.2} ops/sec", throughput);
        println!(
            "  Avg time per restart: {:.2}s",
            duration.as_secs_f64() / stats.restart_count as f64
        );
    }

    println!("\n{}", "=".repeat(80));
}

