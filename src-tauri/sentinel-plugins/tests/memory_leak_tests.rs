//! 内存泄漏专项测试
//!
//! 针对性测试：
//! 1. V8 隔离器内存泄漏
//! 2. Rust-JS 边界内存泄漏
//! 3. 循环引用检测
//! 4. 大对象分配和释放

use sentinel_plugins::{HttpTransaction, PluginEngine, PluginMetadata, Severity};
use std::time::{Duration, Instant};
use sysinfo::System;

/// 内存泄漏检测器
struct MemoryLeakDetector {
    system: System,
    pid: sysinfo::Pid,
    baseline_mb: f64,
    samples: Vec<(Duration, f64)>,
}

impl MemoryLeakDetector {
    fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        let pid = sysinfo::get_current_pid().unwrap();
        
        system.refresh_process(pid);
        let baseline_mb = system
            .process(pid)
            .map(|p| p.memory() as f64 / 1024.0 / 1024.0)
            .unwrap_or(0.0);
        
        Self {
            system,
            pid,
            baseline_mb,
            samples: Vec::new(),
        }
    }

    fn sample(&mut self, elapsed: Duration) {
        self.system.refresh_process(self.pid);
        
        if let Some(process) = self.system.process(self.pid) {
            let memory_mb = process.memory() as f64 / 1024.0 / 1024.0;
            self.samples.push((elapsed, memory_mb));
        }
    }

    /// 检测是否存在内存泄漏
    /// 使用线性回归检测内存增长趋势
    fn detect_leak(&self) -> (bool, f64, f64) {
        if self.samples.len() < 10 {
            return (false, 0.0, 0.0);
        }

        let n = self.samples.len() as f64;
        let sum_x: f64 = self.samples.iter().map(|(d, _)| d.as_secs_f64()).sum();
        let sum_y: f64 = self.samples.iter().map(|(_, m)| *m).sum();
        let sum_xy: f64 = self.samples.iter().map(|(d, m)| d.as_secs_f64() * m).sum();
        let sum_x2: f64 = self.samples.iter().map(|(d, _)| d.as_secs_f64().powi(2)).sum();

        // 线性回归: y = mx + b
        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x.powi(2));
        let intercept = (sum_y - slope * sum_x) / n;

        // 如果斜率 > 0.1 MB/s，认为存在内存泄漏
        let is_leaking = slope > 0.1;

        (is_leaking, slope, intercept)
    }

    fn print_report(&self) {
        let (is_leaking, slope, _) = self.detect_leak();
        
        println!("\n{}", "=".repeat(80));
        println!("Memory Leak Detection Report");
        println!("{}", "=".repeat(80));
        println!("Baseline Memory: {:.2} MB", self.baseline_mb);
        println!("Samples Collected: {}", self.samples.len());
        
        if let Some((_, first_mem)) = self.samples.first() {
            if let Some((_, last_mem)) = self.samples.last() {
                println!("First Sample: {:.2} MB", first_mem);
                println!("Last Sample: {:.2} MB", last_mem);
                println!("Memory Growth: {:.2} MB", last_mem - first_mem);
            }
        }
        
        println!("Growth Rate: {:.4} MB/s", slope);
        println!("Leak Detected: {}", if is_leaking { "YES ⚠️" } else { "NO ✓" });
        println!("{}", "=".repeat(80));
    }
}

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

/// 测试1: 简单插件长时间运行内存泄漏检测
#[tokio::test]
#[ignore]
async fn test_simple_plugin_memory_leak() {
    let metadata = PluginMetadata {
        id: "leak-test-simple".to_string(),
        name: "Simple Leak Test".to_string(),
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
    // 简单操作，不应该泄漏
    const url = transaction.request.url;
    if (url.includes("test")) {
        Sentinel.emitFinding({
            vuln_type: "test",
            title: "Test",
            description: "Test",
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

    let mut detector = MemoryLeakDetector::new();
    let start = Instant::now();
    let test_duration = Duration::from_secs(30);

    let mut iteration = 0;
    while start.elapsed() < test_duration {
        let transaction = create_test_transaction();
        let _ = engine.scan_transaction(&transaction).await;
        
        iteration += 1;
        if iteration % 100 == 0 {
            detector.sample(start.elapsed());
        }
    }

    detector.print_report();
    
    let (is_leaking, slope, _) = detector.detect_leak();
    assert!(!is_leaking, "Memory leak detected: {:.4} MB/s", slope);
}

/// 测试2: 大对象分配内存泄漏检测
#[tokio::test]
#[ignore]
async fn test_large_object_memory_leak() {
    let metadata = PluginMetadata {
        id: "leak-test-large".to_string(),
        name: "Large Object Leak Test".to_string(),
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
    // 分配大对象
    const largeArray = new Array(100000).fill(0).map((_, i) => ({
        id: i,
        data: "x".repeat(100)
    }));
    
    // 处理数据
    const filtered = largeArray.filter(item => item.id % 2 === 0);
    
    Sentinel.emitFinding({
        vuln_type: "test",
        title: "Large Object Test",
        description: "Processed " + filtered.length + " items",
        evidence: "large_object",
        location: "memory",
        severity: "info",
        confidence: "high"
    });
}
"#;

    let mut engine = PluginEngine::new().unwrap();
    engine.load_plugin_with_metadata(code, metadata).await.unwrap();

    let mut detector = MemoryLeakDetector::new();
    let start = Instant::now();
    let test_duration = Duration::from_secs(30);

    let mut iteration = 0;
    while start.elapsed() < test_duration {
        let transaction = create_test_transaction();
        let _ = engine.scan_transaction(&transaction).await;
        
        iteration += 1;
        if iteration % 10 == 0 {
            detector.sample(start.elapsed());
        }
    }

    detector.print_report();
    
    let (is_leaking, slope, _) = detector.detect_leak();
    assert!(!is_leaking, "Memory leak detected: {:.4} MB/s", slope);
}

/// 测试3: 闭包和循环引用内存泄漏检测
#[tokio::test]
#[ignore]
async fn test_closure_memory_leak() {
    let metadata = PluginMetadata {
        id: "leak-test-closure".to_string(),
        name: "Closure Leak Test".to_string(),
        version: "1.0.0".to_string(),
        author: None,
        main_category: "traffic".to_string(),
        category: "test".to_string(),
        default_severity: Severity::Info,
        tags: vec![],
        description: None,
    };

    let code = r#"
// 全局缓存（可能导致内存泄漏）
const cache = new Map();

export function scan_transaction(transaction) {
    const url = transaction.request.url;
    
    // 创建闭包
    const processor = (data) => {
        return data.map(item => ({
            ...item,
            url: url,
            timestamp: Date.now()
        }));
    };
    
    // 存储到缓存（潜在泄漏点）
    cache.set(url, processor);
    
    // 清理旧缓存（防止无限增长）
    if (cache.size > 100) {
        const firstKey = cache.keys().next().value;
        cache.delete(firstKey);
    }
    
    const data = Array.from({ length: 1000 }, (_, i) => ({ id: i }));
    const processed = processor(data);
    
    Sentinel.emitFinding({
        vuln_type: "test",
        title: "Closure Test",
        description: "Processed " + processed.length + " items",
        evidence: "closure",
        location: "memory",
        severity: "info",
        confidence: "high"
    });
}
"#;

    let mut engine = PluginEngine::new().unwrap();
    engine.load_plugin_with_metadata(code, metadata).await.unwrap();

    let mut detector = MemoryLeakDetector::new();
    let start = Instant::now();
    let test_duration = Duration::from_secs(30);

    let mut iteration = 0;
    while start.elapsed() < test_duration {
        let transaction = create_test_transaction();
        let _ = engine.scan_transaction(&transaction).await;
        
        iteration += 1;
        if iteration % 50 == 0 {
            detector.sample(start.elapsed());
        }
    }

    detector.print_report();
    
    let (is_leaking, slope, _) = detector.detect_leak();
    assert!(!is_leaking, "Memory leak detected: {:.4} MB/s", slope);
}

/// 测试4: 字符串拼接内存泄漏检测
#[tokio::test]
#[ignore]
async fn test_string_concatenation_memory_leak() {
    let metadata = PluginMetadata {
        id: "leak-test-string".to_string(),
        name: "String Leak Test".to_string(),
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
    // 大量字符串拼接
    let result = "";
    for (let i = 0; i < 1000; i++) {
        result += transaction.request.url + i;
    }
    
    // 正则匹配
    const patterns = [/test/, /example/, /\d+/];
    for (const pattern of patterns) {
        pattern.test(result);
    }
    
    Sentinel.emitFinding({
        vuln_type: "test",
        title: "String Test",
        description: "Processed string of length " + result.length,
        evidence: result.substring(0, 100),
        location: "string",
        severity: "info",
        confidence: "high"
    });
}
"#;

    let mut engine = PluginEngine::new().unwrap();
    engine.load_plugin_with_metadata(code, metadata).await.unwrap();

    let mut detector = MemoryLeakDetector::new();
    let start = Instant::now();
    let test_duration = Duration::from_secs(30);

    let mut iteration = 0;
    while start.elapsed() < test_duration {
        let transaction = create_test_transaction();
        let _ = engine.scan_transaction(&transaction).await;
        
        iteration += 1;
        if iteration % 50 == 0 {
            detector.sample(start.elapsed());
        }
    }

    detector.print_report();
    
    let (is_leaking, slope, _) = detector.detect_leak();
    assert!(!is_leaking, "Memory leak detected: {:.4} MB/s", slope);
}

/// 测试5: 异步操作内存泄漏检测
#[tokio::test]
#[ignore]
async fn test_async_operations_memory_leak() {
    let metadata = PluginMetadata {
        id: "leak-test-async".to_string(),
        name: "Async Leak Test".to_string(),
        version: "1.0.0".to_string(),
        author: None,
        main_category: "traffic".to_string(),
        category: "test".to_string(),
        default_severity: Severity::Info,
        tags: vec![],
        description: None,
    };

    let code = r#"
export async function scan_transaction(transaction) {
    // 异步操作
    const promises = [];
    for (let i = 0; i < 10; i++) {
        promises.push(
            new Promise(resolve => {
                setTimeout(() => {
                    resolve({ id: i, url: transaction.request.url });
                }, 1);
            })
        );
    }
    
    const results = await Promise.all(promises);
    
    Sentinel.emitFinding({
        vuln_type: "test",
        title: "Async Test",
        description: "Processed " + results.length + " async operations",
        evidence: "async",
        location: "async",
        severity: "info",
        confidence: "high"
    });
}
"#;

    let mut engine = PluginEngine::new().unwrap();
    engine.load_plugin_with_metadata(code, metadata).await.unwrap();

    let mut detector = MemoryLeakDetector::new();
    let start = Instant::now();
    let test_duration = Duration::from_secs(30);

    let mut iteration = 0;
    while start.elapsed() < test_duration {
        let transaction = create_test_transaction();
        let _ = engine.scan_transaction(&transaction).await;
        
        iteration += 1;
        if iteration % 20 == 0 {
            detector.sample(start.elapsed());
        }
    }

    detector.print_report();
    
    let (is_leaking, slope, _) = detector.detect_leak();
    assert!(!is_leaking, "Memory leak detected: {:.4} MB/s", slope);
}

/// 测试6: 多引擎实例内存隔离测试
#[tokio::test]
#[ignore]
async fn test_multi_engine_memory_isolation() {
    let metadata = PluginMetadata {
        id: "leak-test-multi".to_string(),
        name: "Multi Engine Test".to_string(),
        version: "1.0.0".to_string(),
        author: None,
        main_category: "traffic".to_string(),
        category: "test".to_string(),
        default_severity: Severity::Info,
        tags: vec![],
        description: None,
    };

    let code = r#"
const globalData = [];

export function scan_transaction(transaction) {
    // 向全局数组添加数据
    globalData.push({
        url: transaction.request.url,
        timestamp: Date.now()
    });
    
    Sentinel.emitFinding({
        vuln_type: "test",
        title: "Multi Engine Test",
        description: "Global data size: " + globalData.length,
        evidence: "multi_engine",
        location: "global",
        severity: "info",
        confidence: "high"
    });
}
"#;

    let mut detector = MemoryLeakDetector::new();
    let start = Instant::now();
    let test_duration = Duration::from_secs(30);

    let mut iteration = 0;
    while start.elapsed() < test_duration {
        // 每次创建新引擎（测试隔离性）
        let mut engine = PluginEngine::new().unwrap();
        engine.load_plugin_with_metadata(code, metadata.clone()).await.unwrap();
        
        let transaction = create_test_transaction();
        let _ = engine.scan_transaction(&transaction).await;
        
        // 显式销毁引擎
        drop(engine);
        
        iteration += 1;
        if iteration % 10 == 0 {
            detector.sample(start.elapsed());
        }
    }

    detector.print_report();
    
    let (is_leaking, slope, _) = detector.detect_leak();
    assert!(!is_leaking, "Memory leak detected: {:.4} MB/s", slope);
}

