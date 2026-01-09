//! 并发压力测试
//!
//! 测试场景：
//! 1. 最大并发线程数
//! 2. 死锁检测
//! 3. 竞态条件
//! 4. 线程池耗尽

use sentinel_plugins::{HttpTransaction, PluginExecutor, PluginManager, PluginMetadata, Severity};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Semaphore, RwLock};

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
        id: "concurrency-test".to_string(),
        name: "Concurrency Test Plugin".to_string(),
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
        title: "Concurrency Test",
        description: "Test",
        evidence: url,
        location: "url",
        severity: "info",
        confidence: "high"
    }];
}
"#.to_string();

    (metadata, code)
}

/// 测试1: 逐步增加并发数，找到系统极限
#[tokio::test]
#[ignore]
async fn test_find_max_concurrency() {
    println!("\n{}", "=".repeat(80));
    println!("Finding Maximum Concurrency Level");
    println!("{}", "=".repeat(80));

    let concurrency_levels = vec![10, 25, 50, 100, 200, 500, 1000, 2000];
    
    for concurrency in concurrency_levels {
        println!("\nTesting concurrency: {}", concurrency);
        
        let start = Instant::now();
        let success = Arc::new(AtomicUsize::new(0));
        let errors = Arc::new(AtomicUsize::new(0));
        let semaphore = Arc::new(Semaphore::new(concurrency));
        
        let mut handles = vec![];
        
        for _ in 0..concurrency {
            let sem = semaphore.clone();
            let success = success.clone();
            let errors = errors.clone();
            
            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                
                let (metadata, code) = create_simple_plugin();
                let executor = match PluginExecutor::new(metadata.clone(), code.to_string(), 1000) {
                    Ok(e) => e,
                    Err(_) => {
                        errors.fetch_add(1, Ordering::Relaxed);
                        return;
                    }
                };
                
                let transaction = create_test_transaction();
                match executor.scan_transaction(transaction).await {
                    Ok(_) => success.fetch_add(1, Ordering::Relaxed),
                    Err(_) => errors.fetch_add(1, Ordering::Relaxed),
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
        let error_rate = error_count as f64 / (success_count + error_count) as f64;
        
        println!("  Duration: {:?}", duration);
        println!("  Success: {} | Errors: {}", success_count, error_count);
        println!("  Error Rate: {:.2}%", error_rate * 100.0);
        println!("  Throughput: {:.2} ops/sec", concurrency as f64 / duration.as_secs_f64());
        
        // 如果错误率超过20%，停止测试
        if error_rate > 0.2 {
            println!("\n⚠️  ERROR RATE TOO HIGH!");
            println!("Maximum safe concurrency: ~{}", concurrency / 2);
            break;
        }
    }
}

/// 测试2: 持续高并发压力测试
#[tokio::test]
#[ignore]
async fn test_sustained_high_concurrency() {
    println!("\n{}", "=".repeat(80));
    println!("Sustained High Concurrency Test");
    println!("{}", "=".repeat(80));

    let concurrency = 100;
    let duration = Duration::from_secs(60);
    let start = Instant::now();
    
    let success = Arc::new(AtomicUsize::new(0));
    let errors = Arc::new(AtomicUsize::new(0));
    let active_tasks = Arc::new(AtomicUsize::new(0));
    let should_stop = Arc::new(AtomicBool::new(false));
    
    let mut handles = vec![];
    
    // 启动监控任务
    let monitor_success = success.clone();
    let monitor_errors = errors.clone();
    let monitor_active = active_tasks.clone();
    let monitor_stop = should_stop.clone();
    
    let monitor_handle = tokio::spawn(async move {
        let mut last_success = 0;
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        
        while !monitor_stop.load(Ordering::Relaxed) {
            interval.tick().await;
            
            let current_success = monitor_success.load(Ordering::Relaxed);
            let current_errors = monitor_errors.load(Ordering::Relaxed);
            let current_active = monitor_active.load(Ordering::Relaxed);
            let throughput = (current_success - last_success) as f64 / 5.0;
            
            println!(
                "[{:?}] Active: {} | Success: {} | Errors: {} | Throughput: {:.2} ops/sec",
                start.elapsed(),
                current_active,
                current_success,
                current_errors,
                throughput
            );
            
            last_success = current_success;
        }
    });
    
    // 持续生成任务
    while start.elapsed() < duration {
        while active_tasks.load(Ordering::Relaxed) < concurrency {
            let success = success.clone();
            let errors = errors.clone();
            let active = active_tasks.clone();
            
            active.fetch_add(1, Ordering::Relaxed);
            
            let handle = tokio::spawn(async move {
                let (metadata, code) = create_simple_plugin();
                let executor = match PluginExecutor::new(metadata.clone(), code.to_string(), 1000) {
                    Ok(e) => e,
                    Err(_) => {
                        errors.fetch_add(1, Ordering::Relaxed);
                        active.fetch_sub(1, Ordering::Relaxed);
                        return;
                    }
                };
                
                let transaction = create_test_transaction();
                match executor.scan_transaction(transaction).await {
                    Ok(_) => success.fetch_add(1, Ordering::Relaxed),
                    Err(_) => errors.fetch_add(1, Ordering::Relaxed),
                };
                
                active.fetch_sub(1, Ordering::Relaxed);
            });
            
            handles.push(handle);
        }
        
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    should_stop.store(true, Ordering::Relaxed);
    
    // 等待所有任务完成
    for handle in handles {
        let _ = handle.await;
    }
    
    let _ = monitor_handle.await;
    
    let total_duration = start.elapsed();
    let total_success = success.load(Ordering::Relaxed);
    let total_errors = errors.load(Ordering::Relaxed);
    
    println!("\n{}", "=".repeat(80));
    println!("Final Results:");
    println!("  Duration: {:?}", total_duration);
    println!("  Total Success: {}", total_success);
    println!("  Total Errors: {}", total_errors);
    println!("  Average Throughput: {:.2} ops/sec", total_success as f64 / total_duration.as_secs_f64());
    println!("  Error Rate: {:.2}%", total_errors as f64 / (total_success + total_errors) as f64 * 100.0);
    println!("{}", "=".repeat(80));
}

/// 测试3: PluginExecutor 并发测试
#[tokio::test]
#[ignore]
async fn test_plugin_executor_concurrency() {
    println!("\n{}", "=".repeat(80));
    println!("PluginExecutor Concurrency Test");
    println!("{}", "=".repeat(80));

    let (metadata, code) = create_simple_plugin();
    let executor = Arc::new(PluginExecutor::new(metadata, code, 1000).unwrap());
    
    let concurrency = 100;
    let iterations = 1000;
    let start = Instant::now();
    
    let success = Arc::new(AtomicUsize::new(0));
    let errors = Arc::new(AtomicUsize::new(0));
    let semaphore = Arc::new(Semaphore::new(concurrency));
    
    let mut handles = vec![];
    
    for _ in 0..iterations {
        let sem = semaphore.clone();
        let success = success.clone();
        let errors = errors.clone();
        let executor = executor.clone();
        
        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            
            let transaction = create_test_transaction();
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
    
    println!("Duration: {:?}", duration);
    println!("Success: {} | Errors: {}", success_count, error_count);
    println!("Throughput: {:.2} ops/sec", iterations as f64 / duration.as_secs_f64());
    println!("Error Rate: {:.2}%", error_count as f64 / iterations as f64 * 100.0);
}

/// 测试4: PluginManager 多插件并发测试
#[tokio::test]
#[ignore]
async fn test_plugin_manager_multi_plugin_concurrency() {
    println!("\n{}", "=".repeat(80));
    println!("PluginManager Multi-Plugin Concurrency Test");
    println!("{}", "=".repeat(80));

    let manager = PluginManager::new();
    let num_plugins = 10;
    
    // 注册多个插件
    for i in 0..num_plugins {
        let (mut metadata, code) = create_simple_plugin();
        metadata.id = format!("concurrency-test-{}", i);
        
        manager.register_plugin(metadata.id.clone(), metadata, true).await.unwrap();
        manager.set_plugin_code(format!("concurrency-test-{}", i), code).await.unwrap();
    }
    
    let concurrency = 50;
    let iterations = 500;
    let start = Instant::now();
    
    let success = Arc::new(AtomicUsize::new(0));
    let errors = Arc::new(AtomicUsize::new(0));
    let semaphore = Arc::new(Semaphore::new(concurrency));
    
    let mut handles = vec![];
    
    for i in 0..iterations {
        let sem = semaphore.clone();
        let success = success.clone();
        let errors = errors.clone();
        let manager = manager.clone();
        
        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            
            let plugin_id = format!("concurrency-test-{}", i % num_plugins);
            let transaction = create_test_transaction();
            
            match manager.scan_transaction(&plugin_id, &transaction).await {
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
    
    println!("Plugins: {}", num_plugins);
    println!("Duration: {:?}", duration);
    println!("Success: {} | Errors: {}", success_count, error_count);
    println!("Throughput: {:.2} ops/sec", iterations as f64 / duration.as_secs_f64());
    println!("Error Rate: {:.2}%", error_count as f64 / iterations as f64 * 100.0);
}

/// 测试5: 竞态条件测试（共享状态）
#[tokio::test]
#[ignore]
async fn test_race_condition() {
    println!("\n{}", "=".repeat(80));
    println!("Race Condition Test");
    println!("{}", "=".repeat(80));

    let counter = Arc::new(AtomicUsize::new(0));
    let shared_data = Arc::new(RwLock::new(Vec::new()));
    
    let concurrency = 100;
    let iterations = 1000;
    let start = Instant::now();
    
    let mut handles = vec![];
    
    for i in 0..iterations {
        let counter = counter.clone();
        let shared_data = shared_data.clone();
        
        let handle = tokio::spawn(async move {
            // 原子操作
            counter.fetch_add(1, Ordering::SeqCst);
            
            // 共享数据写入
            {
                let mut data = shared_data.write().await;
                data.push(i);
            }
            
            // 共享数据读取
            {
                let data = shared_data.read().await;
                let _ = data.len();
            }
            
            // 创建引擎并执行
            let (metadata, code) = create_simple_plugin();
            let executor = PluginExecutor::new(metadata.clone(), code.to_string(), 1000).unwrap();
            
            let transaction = create_test_transaction();
            let _ = executor.scan_transaction(transaction).await;
        });
        
        handles.push(handle);
        
        // 限制并发数
        if handles.len() >= concurrency {
            let handle = handles.remove(0);
            handle.await.unwrap();
        }
    }
    
    for handle in handles {
        handle.await.unwrap();
    }
    
    let duration = start.elapsed();
    let final_counter = counter.load(Ordering::SeqCst);
    let data_len = shared_data.read().await.len();
    
    println!("Duration: {:?}", duration);
    println!("Counter: {} (expected: {})", final_counter, iterations);
    println!("Shared Data Length: {} (expected: {})", data_len, iterations);
    println!("Counter Match: {}", if final_counter == iterations { "✓" } else { "✗" });
    println!("Data Match: {}", if data_len == iterations { "✓" } else { "✗" });
    
    assert_eq!(final_counter, iterations, "Race condition detected in counter");
    assert_eq!(data_len, iterations, "Race condition detected in shared data");
}

/// 测试6: 死锁检测测试
#[tokio::test]
#[ignore]
async fn test_deadlock_detection() {
    println!("\n{}", "=".repeat(80));
    println!("Deadlock Detection Test");
    println!("{}", "=".repeat(80));

    let lock1 = Arc::new(RwLock::new(0));
    let lock2 = Arc::new(RwLock::new(0));
    
    let timeout = Duration::from_secs(10);
    let start = Instant::now();
    
    let lock1_clone = lock1.clone();
    let lock2_clone = lock2.clone();
    
    let handle1 = tokio::spawn(async move {
        for i in 0..100 {
            let _g1 = lock1_clone.write().await;
            tokio::time::sleep(Duration::from_millis(1)).await;
            let _g2 = lock2_clone.write().await;
            println!("Task 1 iteration {}", i);
        }
    });
    
    let lock1_clone = lock1.clone();
    let lock2_clone = lock2.clone();
    
    let handle2 = tokio::spawn(async move {
        for i in 0..100 {
            let _g2 = lock2_clone.write().await;
            tokio::time::sleep(Duration::from_millis(1)).await;
            let _g1 = lock1_clone.write().await;
            println!("Task 2 iteration {}", i);
        }
    });
    
    // 使用 timeout 检测死锁
    let result = tokio::time::timeout(timeout, async {
        handle1.await.unwrap();
        handle2.await.unwrap();
    }).await;
    
    let duration = start.elapsed();
    
    match result {
        Ok(_) => {
            println!("Duration: {:?}", duration);
            println!("Status: ✓ No deadlock detected");
        }
        Err(_) => {
            println!("Duration: {:?} (timeout)", duration);
            println!("Status: ✗ Potential deadlock detected!");
            panic!("Deadlock detected");
        }
    }
}

/// 测试7: 线程池耗尽测试
#[tokio::test]
#[ignore]
async fn test_thread_pool_exhaustion() {
    println!("\n{}", "=".repeat(80));
    println!("Thread Pool Exhaustion Test");
    println!("{}", "=".repeat(80));

    let iterations = 1000;
    let start = Instant::now();
    
    let success = Arc::new(AtomicUsize::new(0));
    let errors = Arc::new(AtomicUsize::new(0));
    
    let mut handles = vec![];
    
    // 不限制并发，尝试耗尽线程池
    for _ in 0..iterations {
        let success = success.clone();
        let errors = errors.clone();
        
        let handle = tokio::spawn(async move {
            let (metadata, code) = create_simple_plugin();
            let executor = match PluginExecutor::new(metadata.clone(), code.to_string(), 1000) {
                Ok(e) => e,
                Err(_) => {
                    errors.fetch_add(1, Ordering::Relaxed);
                    return;
                }
            };
            
            let transaction = create_test_transaction();
            match executor.scan_transaction(transaction).await {
                Ok(_) => success.fetch_add(1, Ordering::Relaxed),
                Err(_) => errors.fetch_add(1, Ordering::Relaxed),
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
    
    println!("Duration: {:?}", duration);
    println!("Success: {} | Errors: {}", success_count, error_count);
    println!("Throughput: {:.2} ops/sec", iterations as f64 / duration.as_secs_f64());
    println!("Error Rate: {:.2}%", error_count as f64 / iterations as f64 * 100.0);
    
    if error_count > 0 {
        println!("\n⚠️  Thread pool may be exhausted or system resources limited");
    }
}

