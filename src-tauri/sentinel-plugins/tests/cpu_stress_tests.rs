//! CPU 密集型压力测试
//!
//! 测试场景：
//! 1. 复杂正则表达式（回溯爆炸）
//! 2. 大数据排序和过滤
//! 3. 递归算法
//! 4. 密集计算

use sentinel_plugins::{HttpTransaction, PluginExecutor, PluginMetadata, Severity};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::System;

struct CpuMonitor {
    system: System,
    pid: sysinfo::Pid,
    samples: Vec<f64>,
}

impl CpuMonitor {
    fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        let pid = sysinfo::get_current_pid().unwrap();
        
        Self {
            system,
            pid,
            samples: Vec::new(),
        }
    }

    fn sample(&mut self) {
        self.system.refresh_process(self.pid);
        
        if let Some(process) = self.system.process(self.pid) {
            let cpu_percent = process.cpu_usage();
            self.samples.push(cpu_percent as f64);
        }
    }

    fn get_stats(&self) -> (f64, f64, f64) {
        if self.samples.is_empty() {
            return (0.0, 0.0, 0.0);
        }

        let peak = self.samples.iter().cloned().fold(0.0f64, f64::max);
        let avg = self.samples.iter().sum::<f64>() / self.samples.len() as f64;
        let min = self.samples.iter().cloned().fold(f64::INFINITY, f64::min);

        (peak, avg, min)
    }

    fn print_report(&self, test_name: &str, duration: Duration, iterations: usize) {
        let (peak, avg, min) = self.get_stats();
        
        println!("\n{}", "=".repeat(80));
        println!("CPU Stress Test Report: {}", test_name);
        println!("{}", "=".repeat(80));
        println!("Duration: {:?}", duration);
        println!("Iterations: {}", iterations);
        println!("CPU Peak: {:.2}%", peak);
        println!("CPU Avg: {:.2}%", avg);
        println!("CPU Min: {:.2}%", min);
        println!("Throughput: {:.2} ops/sec", iterations as f64 / duration.as_secs_f64());
        println!("{}", "=".repeat(80));
    }
}

fn create_test_transaction(body_size_kb: usize) -> HttpTransaction {
    use chrono::Utc;
    use std::collections::HashMap;

    let body = vec![b'A'; body_size_kb * 1024];
    
    HttpTransaction {
        request: sentinel_plugins::RequestContext {
            id: uuid::Uuid::new_v4().to_string(),
            method: "POST".to_string(),
            url: "https://example.com/api/test".to_string(),
            headers: HashMap::new(),
            body,
            content_type: Some("text/plain".to_string()),
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

/// 测试1: 正则表达式回溯爆炸
#[tokio::test]
#[ignore]
async fn test_regex_backtracking_explosion() {
    let metadata = PluginMetadata {
        id: "cpu-test-regex".to_string(),
        name: "Regex Backtracking Test".to_string(),
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
    const body = new TextDecoder().decode(new Uint8Array(transaction.request.body));
    
    // 危险的正则表达式（可能导致回溯爆炸）
    const dangerousPatterns = [
        /(a+)+b/,
        /(x+x+)+y/,
        /^(a|a)*$/,
        /^(a|ab)*$/,
        /(a*)*b/,
    ];
    
    let matchCount = 0;
    for (const pattern of dangerousPatterns) {
        try {
            // 限制测试字符串长度，避免真正的DoS
            const testStr = body.substring(0, 30);
            if (pattern.test(testStr)) {
                matchCount++;
            }
        } catch (e) {
            // 捕获超时或其他错误
            console.error("Regex error:", e.message);
        }
    }
    
    Sentinel.emitFinding({
        vuln_type: "regex_test",
        title: "Regex Backtracking Test",
        description: "Tested " + dangerousPatterns.length + " patterns, " + matchCount + " matches",
        evidence: "regex_backtracking",
        location: "body",
        severity: "info",
        confidence: "high"
    });
}
"#;

    let executor = PluginExecutor::new(metadata.clone(), code.to_string(), 1000).unwrap();
    

    let mut monitor = CpuMonitor::new();
    let start = Instant::now();
    let iterations = 100;

    for i in 0..iterations {
        if i % 10 == 0 {
            monitor.sample();
        }

        let transaction = create_test_transaction(1);
        let _ = executor.scan_transaction(transaction).await;
    }

    let duration = start.elapsed();
    monitor.print_report("Regex Backtracking", duration, iterations);
}

/// 测试2: 大数据排序和过滤
#[tokio::test]
#[ignore]
async fn test_large_data_sorting() {
    let metadata = PluginMetadata {
        id: "cpu-test-sort".to_string(),
        name: "Large Data Sorting Test".to_string(),
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
    // 生成大量随机数据
    const dataSize = 50000;
    const data = Array.from({ length: dataSize }, (_, i) => ({
        id: i,
        value: Math.random() * 1000000,
        category: Math.floor(Math.random() * 100),
        timestamp: Date.now() + i
    }));
    
    // 多次排序
    const sorted1 = [...data].sort((a, b) => a.value - b.value);
    const sorted2 = [...data].sort((a, b) => b.timestamp - a.timestamp);
    const sorted3 = [...data].sort((a, b) => a.category - b.category);
    
    // 过滤操作
    const filtered1 = data.filter(item => item.value > 500000);
    const filtered2 = data.filter(item => item.category < 50);
    
    // 聚合操作
    const grouped = data.reduce((acc, item) => {
        if (!acc[item.category]) {
            acc[item.category] = [];
        }
        acc[item.category].push(item);
        return acc;
    }, {});
    
    Sentinel.emitFinding({
        vuln_type: "sort_test",
        title: "Large Data Sorting Test",
        description: "Processed " + dataSize + " items, " + Object.keys(grouped).length + " groups",
        evidence: "data_sorting",
        location: "computation",
        severity: "info",
        confidence: "high"
    });
}
"#;

    let executor = PluginExecutor::new(metadata.clone(), code.to_string(), 1000).unwrap();
    

    let mut monitor = CpuMonitor::new();
    let start = Instant::now();
    let iterations = 50;

    for i in 0..iterations {
        if i % 5 == 0 {
            monitor.sample();
        }

        let transaction = create_test_transaction(1);
        let _ = executor.scan_transaction(transaction).await;
    }

    let duration = start.elapsed();
    monitor.print_report("Large Data Sorting", duration, iterations);
}

/// 测试3: 递归算法（斐波那契、阶乘）
#[tokio::test]
#[ignore]
async fn test_recursive_algorithms() {
    let metadata = PluginMetadata {
        id: "cpu-test-recursive".to_string(),
        name: "Recursive Algorithms Test".to_string(),
        version: "1.0.0".to_string(),
        author: None,
        main_category: "passive".to_string(),
        category: "test".to_string(),
        default_severity: Severity::Info,
        tags: vec![],
        description: None,
    };

    let code = r#"
function fibonacci(n) {
    if (n <= 1) return n;
    return fibonacci(n - 1) + fibonacci(n - 2);
}

function factorial(n) {
    if (n <= 1) return 1;
    return n * factorial(n - 1);
}

function ackermann(m, n) {
    if (m === 0) return n + 1;
    if (n === 0) return ackermann(m - 1, 1);
    return ackermann(m - 1, ackermann(m, n - 1));
}

export function scan_transaction(transaction) {
    // 计算多个递归函数
    const fib20 = fibonacci(20);
    const fact10 = factorial(10);
    
    // Ackermann 函数（非常密集）
    let ack = 0;
    try {
        ack = ackermann(3, 4);
    } catch (e) {
        ack = -1;
    }
    
    // 嵌套递归
    const results = [];
    for (let i = 1; i <= 15; i++) {
        results.push(fibonacci(i));
    }
    
    Sentinel.emitFinding({
        vuln_type: "recursive_test",
        title: "Recursive Algorithms Test",
        description: "fib(20)=" + fib20 + ", fact(10)=" + fact10 + ", ack(3,4)=" + ack,
        evidence: "recursive_computation",
        location: "computation",
        severity: "info",
        confidence: "high"
    });
}
"#;

    let executor = PluginExecutor::new(metadata.clone(), code.to_string(), 1000).unwrap();
    

    let mut monitor = CpuMonitor::new();
    let start = Instant::now();
    let iterations = 50;

    for i in 0..iterations {
        if i % 5 == 0 {
            monitor.sample();
        }

        let transaction = create_test_transaction(1);
        let _ = executor.scan_transaction(transaction).await;
    }

    let duration = start.elapsed();
    monitor.print_report("Recursive Algorithms", duration, iterations);
}

/// 测试4: 密集数学计算
#[tokio::test]
#[ignore]
async fn test_intensive_math_computation() {
    let metadata = PluginMetadata {
        id: "cpu-test-math".to_string(),
        name: "Intensive Math Test".to_string(),
        version: "1.0.0".to_string(),
        author: None,
        main_category: "passive".to_string(),
        category: "test".to_string(),
        default_severity: Severity::Info,
        tags: vec![],
        description: None,
    };

    let code = r#"
function isPrime(n) {
    if (n <= 1) return false;
    if (n <= 3) return true;
    if (n % 2 === 0 || n % 3 === 0) return false;
    
    for (let i = 5; i * i <= n; i += 6) {
        if (n % i === 0 || n % (i + 2) === 0) return false;
    }
    return true;
}

function matrixMultiply(a, b) {
    const result = [];
    for (let i = 0; i < a.length; i++) {
        result[i] = [];
        for (let j = 0; j < b[0].length; j++) {
            let sum = 0;
            for (let k = 0; k < a[0].length; k++) {
                sum += a[i][k] * b[k][j];
            }
            result[i][j] = sum;
        }
    }
    return result;
}

export function scan_transaction(transaction) {
    // 质数计算
    const primes = [];
    for (let i = 2; i < 1000; i++) {
        if (isPrime(i)) {
            primes.push(i);
        }
    }
    
    // 矩阵乘法
    const size = 50;
    const matrix1 = Array.from({ length: size }, () => 
        Array.from({ length: size }, () => Math.random())
    );
    const matrix2 = Array.from({ length: size }, () => 
        Array.from({ length: size }, () => Math.random())
    );
    
    const result = matrixMultiply(matrix1, matrix2);
    
    // 三角函数计算
    let trigSum = 0;
    for (let i = 0; i < 10000; i++) {
        trigSum += Math.sin(i) * Math.cos(i) + Math.tan(i / 100);
    }
    
    Sentinel.emitFinding({
        vuln_type: "math_test",
        title: "Intensive Math Test",
        description: "Found " + primes.length + " primes, matrix " + size + "x" + size,
        evidence: "math_computation",
        location: "computation",
        severity: "info",
        confidence: "high"
    });
}
"#;

    let executor = PluginExecutor::new(metadata.clone(), code.to_string(), 1000).unwrap();
    

    let mut monitor = CpuMonitor::new();
    let start = Instant::now();
    let iterations = 50;

    for i in 0..iterations {
        if i % 5 == 0 {
            monitor.sample();
        }

        let transaction = create_test_transaction(1);
        let _ = executor.scan_transaction(transaction).await;
    }

    let duration = start.elapsed();
    monitor.print_report("Intensive Math", duration, iterations);
}

/// 测试5: 字符串处理密集型
#[tokio::test]
#[ignore]
async fn test_string_processing_intensive() {
    let metadata = PluginMetadata {
        id: "cpu-test-string".to_string(),
        name: "String Processing Test".to_string(),
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
    const body = new TextDecoder().decode(new Uint8Array(transaction.request.body));
    
    // 大量字符串操作
    let result = body;
    for (let i = 0; i < 100; i++) {
        result = result.split('').reverse().join('');
        result = result.toUpperCase();
        result = result.toLowerCase();
        result = result.replace(/[aeiou]/gi, 'X');
    }
    
    // 字符串搜索
    const patterns = ['test', 'example', 'data', 'api', 'user'];
    const counts = {};
    for (const pattern of patterns) {
        counts[pattern] = (body.match(new RegExp(pattern, 'gi')) || []).length;
    }
    
    // Base64 编解码
    let encoded = btoa(body.substring(0, 1000));
    for (let i = 0; i < 10; i++) {
        encoded = btoa(encoded);
        const decoded = atob(encoded);
        encoded = btoa(decoded);
    }
    
    // JSON 序列化/反序列化
    const obj = { data: body, counts, timestamp: Date.now() };
    let jsonStr = JSON.stringify(obj);
    for (let i = 0; i < 100; i++) {
        const parsed = JSON.parse(jsonStr);
        jsonStr = JSON.stringify(parsed);
    }
    
    Sentinel.emitFinding({
        vuln_type: "string_test",
        title: "String Processing Test",
        description: "Processed string operations",
        evidence: "string_processing",
        location: "computation",
        severity: "info",
        confidence: "high"
    });
}
"#;

    let executor = PluginExecutor::new(metadata.clone(), code.to_string(), 1000).unwrap();
    

    let mut monitor = CpuMonitor::new();
    let start = Instant::now();
    let iterations = 50;

    for i in 0..iterations {
        if i % 5 == 0 {
            monitor.sample();
        }

        let transaction = create_test_transaction(10);
        let _ = executor.scan_transaction(transaction).await;
    }

    let duration = start.elapsed();
    monitor.print_report("String Processing", duration, iterations);
}

/// 测试6: 并发CPU密集型任务
#[tokio::test]
#[ignore]
async fn test_concurrent_cpu_intensive() {
    let metadata = PluginMetadata {
        id: "cpu-test-concurrent".to_string(),
        name: "Concurrent CPU Test".to_string(),
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
    // CPU密集型任务
    const data = Array.from({ length: 10000 }, (_, i) => ({
        id: i,
        value: Math.random() * 1000
    }));
    
    // 多次排序
    for (let i = 0; i < 10; i++) {
        data.sort((a, b) => a.value - b.value);
        data.sort((a, b) => b.value - a.value);
    }
    
    // 过滤和映射
    const filtered = data
        .filter(item => item.value > 500)
        .map(item => ({ ...item, squared: item.value * item.value }))
        .filter(item => item.squared < 900000);
    
    Sentinel.emitFinding({
        vuln_type: "concurrent_cpu_test",
        title: "Concurrent CPU Test",
        description: "Processed " + data.length + " items",
        evidence: "concurrent_cpu",
        location: "computation",
        severity: "info",
        confidence: "high"
    });
}
"#;

    let mut monitor = CpuMonitor::new();
    let start = Instant::now();
    let iterations = 200;
    let concurrency = 10;

    let success = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    for i in 0..iterations {
        let success = success.clone();
        let code = code.to_string();
        let metadata = metadata.clone();

        let handle = tokio::spawn(async move {
            let executor = match PluginExecutor::new(metadata.clone(), code.to_string(), 1000) {
                Ok(e) => e,
                Err(_) => return,
            };

            let transaction = create_test_transaction(1);
            if executor.scan_transaction(transaction).await.is_ok() {
                success.fetch_add(1, Ordering::Relaxed);
            }
        });

        handles.push(handle);

        if i % 20 == 0 {
            monitor.sample();
        }

        // 限制并发数
        if handles.len() >= concurrency {
            let handle = handles.remove(0);
            let _ = handle.await;
        }
    }

    for handle in handles {
        let _ = handle.await;
    }

    let duration = start.elapsed();
    monitor.print_report("Concurrent CPU Intensive", duration, iterations);

    println!("Success rate: {}/{}", success.load(Ordering::Relaxed), iterations);
}

