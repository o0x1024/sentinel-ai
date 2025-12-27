//! 插件系统压力测试
//!
//! 测试目标：
//! 1. 内存泄漏检测（长时间运行、大量插件实例）
//! 2. CPU 密集型操作（复杂正则、大数据处理）
//! 3. 并发线程压力（最大并发数、死锁检测）
//! 4. V8 隔离器限制（堆内存、栈溢出）
//! 5. 资源耗尽场景（文件句柄、网络连接）

use sentinel_plugins::{
    HttpTransaction, PluginEngine, PluginExecutor, PluginManager, PluginMetadata,
    RequestContext, ResponseContext, Severity,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::System;
use tokio::sync::Semaphore;

/// 测试结果统计
#[derive(Debug, Clone)]
struct StressTestResult {
    test_name: String,
    duration: Duration,
    iterations: usize,
    success_count: usize,
    error_count: usize,
    peak_memory_mb: f64,
    avg_memory_mb: f64,
    peak_cpu_percent: f64,
    avg_cpu_percent: f64,
    throughput_per_sec: f64,
}

impl StressTestResult {
    fn print_report(&self) {
        println!("\n{}", "=".repeat(80));
        println!("Stress Test Report: {}", self.test_name);
        println!("{}", "=".repeat(80));
        println!("Duration: {:?}", self.duration);
        println!("Iterations: {}", self.iterations);
        println!("Success: {} | Errors: {}", self.success_count, self.error_count);
        println!("Peak Memory: {:.2} MB | Avg Memory: {:.2} MB", self.peak_memory_mb, self.avg_memory_mb);
        println!("Peak CPU: {:.2}% | Avg CPU: {:.2}%", self.peak_cpu_percent, self.avg_cpu_percent);
        println!("Throughput: {:.2} ops/sec", self.throughput_per_sec);
        println!("{}", "=".repeat(80));
    }
}

/// 系统资源监控器
struct ResourceMonitor {
    system: System,
    pid: sysinfo::Pid,
    memory_samples: Vec<f64>,
    cpu_samples: Vec<f64>,
}

impl ResourceMonitor {
    fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        let pid = sysinfo::get_current_pid().unwrap();
        
        Self {
            system,
            pid,
            memory_samples: Vec::new(),
            cpu_samples: Vec::new(),
        }
    }

    fn sample(&mut self) {
        self.system.refresh_process(self.pid);
        
        if let Some(process) = self.system.process(self.pid) {
            let memory_mb = process.memory() as f64 / 1024.0 / 1024.0;
            let cpu_percent = process.cpu_usage();
            
            self.memory_samples.push(memory_mb);
            self.cpu_samples.push(cpu_percent as f64);
        }
    }

    fn get_stats(&self) -> (f64, f64, f64, f64) {
        let peak_mem = self.memory_samples.iter().cloned().fold(0.0f64, f64::max);
        let avg_mem = self.memory_samples.iter().sum::<f64>() / self.memory_samples.len() as f64;
        let peak_cpu = self.cpu_samples.iter().cloned().fold(0.0f64, f64::max);
        let avg_cpu = self.cpu_samples.iter().sum::<f64>() / self.cpu_samples.len() as f64;
        
        (peak_mem, avg_mem, peak_cpu, avg_cpu)
    }
}

/// 创建测试用的 HTTP 事务
fn create_test_transaction(size_kb: usize) -> HttpTransaction {
    use chrono::Utc;
    use std::collections::HashMap;

    let body = vec![b'A'; size_kb * 1024];
    
    HttpTransaction {
        request: RequestContext {
            id: uuid::Uuid::new_v4().to_string(),
            method: "POST".to_string(),
            url: "https://example.com/api/test".to_string(),
            headers: HashMap::from([
                ("Content-Type".to_string(), "application/json".to_string()),
                ("User-Agent".to_string(), "StressTest/1.0".to_string()),
            ]),
            body: body.clone(),
            content_type: Some("application/json".to_string()),
            query_params: HashMap::new(),
            is_https: true,
            timestamp: Utc::now(),
            was_edited: false,
            edited_method: None,
            edited_url: None,
            edited_headers: None,
            edited_body: None,
        },
        response: Some(ResponseContext {
            request_id: uuid::Uuid::new_v4().to_string(),
            status: 200,
            headers: HashMap::from([
                ("Content-Type".to_string(), "application/json".to_string()),
            ]),
            body,
            content_type: Some("application/json".to_string()),
            timestamp: Utc::now(),
            was_edited: false,
            edited_status: None,
            edited_headers: None,
            edited_body: None,
        }),
    }
}

/// 创建简单测试插件
fn create_simple_plugin() -> (PluginMetadata, String) {
    let metadata = PluginMetadata {
        id: "stress-test-simple".to_string(),
        name: "Simple Stress Test Plugin".to_string(),
        version: "1.0.0".to_string(),
        author: Some("Stress Test".to_string()),
        main_category: "traffic".to_string(),
        category: "test".to_string(),
        default_severity: Severity::Info,
        tags: vec!["test".to_string()],
        description: Some("Simple plugin for stress testing".to_string()),
    };

    let code = r#"
export function scan_transaction(transaction) {
    const url = transaction.request.url;
    if (url.includes("test")) {
        Sentinel.emitFinding({
            vuln_type: "test",
            title: "Test Finding",
            description: "Test description",
            evidence: url,
            location: "url",
            severity: "info",
            confidence: "high"
        });
    }
}
"#.to_string();

    (metadata, code)
}

/// 创建CPU密集型插件（复杂正则）
fn create_cpu_intensive_plugin() -> (PluginMetadata, String) {
    let metadata = PluginMetadata {
        id: "stress-test-cpu".to_string(),
        name: "CPU Intensive Plugin".to_string(),
        version: "1.0.0".to_string(),
        author: Some("Stress Test".to_string()),
        main_category: "traffic".to_string(),
        category: "test".to_string(),
        default_severity: Severity::Info,
        tags: vec!["test".to_string()],
        description: Some("CPU intensive plugin for stress testing".to_string()),
    };

    let code = r#"
export function scan_transaction(transaction) {
    const body = new TextDecoder().decode(new Uint8Array(transaction.request.body));
    
    // CPU密集型：复杂正则回溯
    const patterns = [
        /(a+)+b/,
        /(x+x+)+y/,
        /^(a|a)*$/,
        /(\w+\s?)*\d+/,
    ];
    
    for (let i = 0; i < 100; i++) {
        for (const pattern of patterns) {
            try {
                pattern.test(body + "a".repeat(20));
            } catch (e) {
                // Ignore timeouts
            }
        }
    }
    
    // 大数据处理
    const data = [];
    for (let i = 0; i < 10000; i++) {
        data.push({ id: i, value: Math.random() });
    }
    
    data.sort((a, b) => b.value - a.value);
    
    Sentinel.emitFinding({
        vuln_type: "cpu_test",
        title: "CPU Test",
        description: "Processed " + data.length + " items",
        evidence: "cpu_intensive",
        location: "body",
        severity: "info",
        confidence: "high"
    });
}
"#.to_string();

    (metadata, code)
}

/// 创建内存密集型插件
fn create_memory_intensive_plugin() -> (PluginMetadata, String) {
    let metadata = PluginMetadata {
        id: "stress-test-memory".to_string(),
        name: "Memory Intensive Plugin".to_string(),
        version: "1.0.0".to_string(),
        author: Some("Stress Test".to_string()),
        main_category: "traffic".to_string(),
        category: "test".to_string(),
        default_severity: Severity::Info,
        tags: vec!["test".to_string()],
        description: Some("Memory intensive plugin for stress testing".to_string()),
    };

    let code = r#"
export function scan_transaction(transaction) {
    // 内存密集型：创建大量对象
    const largeArrays = [];
    for (let i = 0; i < 100; i++) {
        largeArrays.push(new Array(10000).fill({ data: "x".repeat(100) }));
    }
    
    // 字符串拼接
    let bigString = "";
    for (let i = 0; i < 1000; i++) {
        bigString += transaction.request.url + i;
    }
    
    Sentinel.emitFinding({
        vuln_type: "memory_test",
        title: "Memory Test",
        description: "Allocated " + largeArrays.length + " arrays",
        evidence: bigString.substring(0, 100),
        location: "memory",
        severity: "info",
        confidence: "high"
    });
}
"#.to_string();

    (metadata, code)
}

// ============================================================
// 测试用例
// ============================================================

/// 测试1: 单引擎长时间运行（内存泄漏检测）
#[tokio::test]
#[ignore] // 运行时间较长，使用 --ignored 运行
async fn test_single_engine_long_running() {
    let (metadata, code) = create_simple_plugin();
    let mut engine = PluginEngine::new().unwrap();
    engine.load_plugin_with_metadata(&code, metadata).await.unwrap();

    let mut monitor = ResourceMonitor::new();
    let start = Instant::now();
    let iterations = 10000;
    let mut success = 0;
    let mut errors = 0;

    for i in 0..iterations {
        if i % 100 == 0 {
            monitor.sample();
        }

        let transaction = create_test_transaction(1);
        match engine.scan_transaction(&transaction).await {
            Ok(_) => success += 1,
            Err(_) => errors += 1,
        }
    }

    let duration = start.elapsed();
    let (peak_mem, avg_mem, peak_cpu, avg_cpu) = monitor.get_stats();

    let result = StressTestResult {
        test_name: "Single Engine Long Running".to_string(),
        duration,
        iterations,
        success_count: success,
        error_count: errors,
        peak_memory_mb: peak_mem,
        avg_memory_mb: avg_mem,
        peak_cpu_percent: peak_cpu,
        avg_cpu_percent: avg_cpu,
        throughput_per_sec: iterations as f64 / duration.as_secs_f64(),
    };

    result.print_report();

    // 内存泄漏检测：平均内存不应超过峰值的80%
    assert!(avg_mem < peak_mem * 0.8, "Potential memory leak detected");
}

/// 测试2: 并发引擎创建和销毁
#[tokio::test]
#[ignore]
async fn test_concurrent_engine_creation() {
    let mut monitor = ResourceMonitor::new();
    let start = Instant::now();
    let iterations = 100;
    let concurrency = 10;

    let success = Arc::new(AtomicUsize::new(0));
    let errors = Arc::new(AtomicUsize::new(0));
    let semaphore = Arc::new(Semaphore::new(concurrency));

    let mut handles = vec![];

    for i in 0..iterations {
        let sem = semaphore.clone();
        let success = success.clone();
        let errors = errors.clone();

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();

            let (metadata, code) = create_simple_plugin();
            let transaction = create_test_transaction(1);
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

        if i % 10 == 0 {
            monitor.sample();
        }
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let duration = start.elapsed();
    let (peak_mem, avg_mem, peak_cpu, avg_cpu) = monitor.get_stats();

    let result = StressTestResult {
        test_name: "Concurrent Engine Creation".to_string(),
        duration,
        iterations,
        success_count: success.load(Ordering::Relaxed),
        error_count: errors.load(Ordering::Relaxed),
        peak_memory_mb: peak_mem,
        avg_memory_mb: avg_mem,
        peak_cpu_percent: peak_cpu,
        avg_cpu_percent: avg_cpu,
        throughput_per_sec: iterations as f64 / duration.as_secs_f64(),
    };

    result.print_report();
}

/// 测试3: CPU密集型插件压力测试
#[tokio::test]
#[ignore]
async fn test_cpu_intensive_plugin() {
    let (metadata, code) = create_cpu_intensive_plugin();
    let mut engine = PluginEngine::new().unwrap();
    engine.load_plugin_with_metadata(&code, metadata).await.unwrap();

    let mut monitor = ResourceMonitor::new();
    let start = Instant::now();
    let iterations = 50;
    let mut success = 0;
    let mut errors = 0;

    for _i in 0..iterations {
        monitor.sample();

        let transaction = create_test_transaction(10);
        match engine.scan_transaction(&transaction).await {
            Ok(_) => success += 1,
            Err(_) => errors += 1,
        }
    }

    let duration = start.elapsed();
    let (peak_mem, avg_mem, peak_cpu, avg_cpu) = monitor.get_stats();

    let result = StressTestResult {
        test_name: "CPU Intensive Plugin".to_string(),
        duration,
        iterations,
        success_count: success,
        error_count: errors,
        peak_memory_mb: peak_mem,
        avg_memory_mb: avg_mem,
        peak_cpu_percent: peak_cpu,
        avg_cpu_percent: avg_cpu,
        throughput_per_sec: iterations as f64 / duration.as_secs_f64(),
    };

    result.print_report();

    // CPU使用率应该较高
    println!("CPU usage check: avg={:.2}%, peak={:.2}%", avg_cpu, peak_cpu);
}

/// 测试4: 内存密集型插件压力测试
#[tokio::test]
#[ignore]
async fn test_memory_intensive_plugin() {
    let (metadata, code) = create_memory_intensive_plugin();
    let mut engine = PluginEngine::new().unwrap();
    engine.load_plugin_with_metadata(&code, metadata).await.unwrap();

    let mut monitor = ResourceMonitor::new();
    let start = Instant::now();
    let iterations = 100;
    let mut success = 0;
    let mut errors = 0;

    for i in 0..iterations {
        if i % 10 == 0 {
            monitor.sample();
        }

        let transaction = create_test_transaction(10);
        match engine.scan_transaction(&transaction).await {
            Ok(_) => success += 1,
            Err(_) => errors += 1,
        }
    }

    let duration = start.elapsed();
    let (peak_mem, avg_mem, peak_cpu, avg_cpu) = monitor.get_stats();

    let result = StressTestResult {
        test_name: "Memory Intensive Plugin".to_string(),
        duration,
        iterations,
        success_count: success,
        error_count: errors,
        peak_memory_mb: peak_mem,
        avg_memory_mb: avg_mem,
        peak_cpu_percent: peak_cpu,
        avg_cpu_percent: avg_cpu,
        throughput_per_sec: iterations as f64 / duration.as_secs_f64(),
    };

    result.print_report();

    // 内存使用应该可控
    println!("Memory usage check: avg={:.2}MB, peak={:.2}MB", avg_mem, peak_mem);
}

/// 测试5: 最大并发线程数测试
#[tokio::test]
#[ignore]
async fn test_max_concurrent_threads() {
    let mut monitor = ResourceMonitor::new();
    let start = Instant::now();

    let success = Arc::new(AtomicUsize::new(0));
    let errors = Arc::new(AtomicUsize::new(0));

    // 逐步增加并发数，找到系统极限
    let concurrency_levels = vec![10, 50, 100, 200, 500, 1000];
    
    for concurrency in concurrency_levels {
        println!("\nTesting concurrency level: {}", concurrency);
        
        let semaphore = Arc::new(Semaphore::new(concurrency));
        let mut handles = vec![];

        for _ in 0..concurrency {
            let sem = semaphore.clone();
            let success = success.clone();
            let errors = errors.clone();

            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();

                let (metadata, code) = create_simple_plugin();
                let transaction = create_test_transaction(1);
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
            let _ = handle.await;
        }

        monitor.sample();
        
        let current_success = success.load(Ordering::Relaxed);
        let current_errors = errors.load(Ordering::Relaxed);
        let (peak_mem, _avg_mem, peak_cpu, _avg_cpu) = monitor.get_stats();
        
        println!("  Success: {} | Errors: {}", current_success, current_errors);
        println!("  Memory: {:.2}MB | CPU: {:.2}%", peak_mem, peak_cpu);
        
        // 如果错误率超过20%，停止测试
        if current_errors as f64 / (current_success + current_errors) as f64 > 0.2 {
            println!("  ERROR RATE TOO HIGH - Max concurrency: {}", concurrency / 2);
            break;
        }
    }

    let duration = start.elapsed();
    let (peak_mem, avg_mem, peak_cpu, avg_cpu) = monitor.get_stats();

    let result = StressTestResult {
        test_name: "Max Concurrent Threads".to_string(),
        duration,
        iterations: success.load(Ordering::Relaxed) + errors.load(Ordering::Relaxed),
        success_count: success.load(Ordering::Relaxed),
        error_count: errors.load(Ordering::Relaxed),
        peak_memory_mb: peak_mem,
        avg_memory_mb: avg_mem,
        peak_cpu_percent: peak_cpu,
        avg_cpu_percent: avg_cpu,
        throughput_per_sec: 0.0,
    };

    result.print_report();
}

/// 测试6: PluginExecutor 长期运行测试
#[tokio::test]
#[ignore]
async fn test_plugin_executor_long_running() {
    let (metadata, code) = create_simple_plugin();
    let executor = PluginExecutor::new(metadata, code, 1000).unwrap();

    let mut monitor = ResourceMonitor::new();
    let start = Instant::now();
    let iterations = 5000;
    let mut success = 0;
    let mut errors = 0;

    for i in 0..iterations {
        if i % 100 == 0 {
            monitor.sample();
        }

        let transaction = create_test_transaction(1);
        match executor.scan_transaction(transaction).await {
            Ok(_) => success += 1,
            Err(_) => errors += 1,
        }
    }

    let duration = start.elapsed();
    let (peak_mem, avg_mem, peak_cpu, avg_cpu) = monitor.get_stats();

    let result = StressTestResult {
        test_name: "PluginExecutor Long Running".to_string(),
        duration,
        iterations,
        success_count: success,
        error_count: errors,
        peak_memory_mb: peak_mem,
        avg_memory_mb: avg_mem,
        peak_cpu_percent: peak_cpu,
        avg_cpu_percent: avg_cpu,
        throughput_per_sec: iterations as f64 / duration.as_secs_f64(),
    };

    result.print_report();
}

/// 测试7: 大数据量事务处理
#[tokio::test]
#[ignore]
async fn test_large_transaction_processing() {
    let (metadata, code) = create_simple_plugin();
    let mut engine = PluginEngine::new().unwrap();
    engine.load_plugin_with_metadata(&code, metadata).await.unwrap();

    let mut monitor = ResourceMonitor::new();
    let start = Instant::now();
    
    // 测试不同大小的事务
    let sizes_kb = vec![1, 10, 100, 1000, 5000];
    let mut success = 0;
    let mut errors = 0;
    let mut total_iterations = 0;

    for size in sizes_kb {
        println!("\nTesting transaction size: {} KB", size);
        
        for _ in 0..10 {
            monitor.sample();
            
            let transaction = create_test_transaction(size);
            match engine.scan_transaction(&transaction).await {
                Ok(_) => success += 1,
                Err(_) => errors += 1,
            }
            total_iterations += 1;
        }
        
        let (peak_mem, _, _, _) = monitor.get_stats();
        println!("  Memory after {} KB: {:.2} MB", size, peak_mem);
    }

    let duration = start.elapsed();
    let (peak_mem, avg_mem, peak_cpu, avg_cpu) = monitor.get_stats();

    let result = StressTestResult {
        test_name: "Large Transaction Processing".to_string(),
        duration,
        iterations: total_iterations,
        success_count: success,
        error_count: errors,
        peak_memory_mb: peak_mem,
        avg_memory_mb: avg_mem,
        peak_cpu_percent: peak_cpu,
        avg_cpu_percent: avg_cpu,
        throughput_per_sec: total_iterations as f64 / duration.as_secs_f64(),
    };

    result.print_report();
}

/// 测试8: PluginManager 并发扫描压力测试
#[tokio::test]
#[ignore]
async fn test_plugin_manager_concurrent_scan() {
    let manager = PluginManager::new();
    
    // 注册多个插件
    for i in 0..5 {
        let (mut metadata, code) = create_simple_plugin();
        metadata.id = format!("stress-test-{}", i);
        
        manager.register_plugin(metadata.id.clone(), metadata, true).await.unwrap();
        manager.set_plugin_code(format!("stress-test-{}", i), code).await.unwrap();
    }

    let mut monitor = ResourceMonitor::new();
    let start = Instant::now();
    let iterations = 1000;
    
    let success = Arc::new(AtomicUsize::new(0));
    let errors = Arc::new(AtomicUsize::new(0));
    let semaphore = Arc::new(Semaphore::new(50));

    let mut handles = vec![];

    for i in 0..iterations {
        let sem = semaphore.clone();
        let success = success.clone();
        let errors = errors.clone();
        let manager = manager.clone();

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();

            let plugin_id = format!("stress-test-{}", i % 5);
            let transaction = create_test_transaction(1);
            
            match manager.scan_transaction(&plugin_id, &transaction).await {
                Ok(_) => success.fetch_add(1, Ordering::Relaxed),
                Err(_) => errors.fetch_add(1, Ordering::Relaxed),
            };
        });

        handles.push(handle);

        if i % 100 == 0 {
            monitor.sample();
        }
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let duration = start.elapsed();
    let (peak_mem, avg_mem, peak_cpu, avg_cpu) = monitor.get_stats();

    let result = StressTestResult {
        test_name: "PluginManager Concurrent Scan".to_string(),
        duration,
        iterations,
        success_count: success.load(Ordering::Relaxed),
        error_count: errors.load(Ordering::Relaxed),
        peak_memory_mb: peak_mem,
        avg_memory_mb: avg_mem,
        peak_cpu_percent: peak_cpu,
        avg_cpu_percent: avg_cpu,
        throughput_per_sec: iterations as f64 / duration.as_secs_f64(),
    };

    result.print_report();
}

