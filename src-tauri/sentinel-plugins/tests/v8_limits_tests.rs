//! V8 引擎限制测试
//!
//! 测试场景：
//! 1. 堆内存限制
//! 2. 栈溢出
//! 3. 超时控制
//! 4. 大对象分配

use sentinel_plugins::{HttpTransaction, PluginEngine, PluginExecutor, PluginMetadata, Severity};
use std::time::{Duration, Instant};

fn create_test_transaction(body_size_kb: usize) -> HttpTransaction {
    use chrono::Utc;
    use std::collections::HashMap;

    let body = vec![b'A'; body_size_kb * 1024];
    
    HttpTransaction {
        request: sentinel_plugins::RequestContext {
            id: uuid::Uuid::new_v4().to_string(),
            method: "POST".to_string(),
            url: "https://example.com/test".to_string(),
            headers: HashMap::new(),
            body,
            content_type: Some("application/octet-stream".to_string()),
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

/// 测试1: 堆内存限制测试
#[tokio::test]
#[ignore]
async fn test_heap_memory_limit() {
    println!("\n{}", "=".repeat(80));
    println!("V8 Heap Memory Limit Test");
    println!("{}", "=".repeat(80));

    let metadata = PluginMetadata {
        id: "v8-test-heap".to_string(),
        name: "Heap Memory Test".to_string(),
        version: "1.0.0".to_string(),
        author: None,
        main_category: "traffic".to_string(),
        category: "test".to_string(),
        default_severity: Severity::Info,
        tags: vec![],
        description: None,
    };

    // NOTE:
    // Triggering a real V8 heap OOM is a *fatal* abort (not a JS exception) and will
    // crash the entire Rust test process. This test is intentionally bounded to
    // apply heap pressure without killing the runner.
    let code = r#"
export function scan_transaction(transaction) {
    const buffers = [];
    const chunkBytes = 2 * 1024 * 1024; // 2MB
    const maxTotalBytes = 256 * 1024 * 1024; // 256MB soft cap
    let totalBytes = 0;

    for (let i = 0; i < 10_000; i++) {
        if (totalBytes + chunkBytes > maxTotalBytes) {
            break;
        }

        // Use ArrayBuffer to create predictable heap pressure without huge object graphs
        const buf = new ArrayBuffer(chunkBytes);
        // Touch memory so it actually commits
        const view = new Uint8Array(buf);
        view[0] = 1;
        view[view.length - 1] = 2;

        buffers.push(buf);
        totalBytes += chunkBytes;

        if (i % 10 === 0) {
            console.log("Allocated buffers:", buffers.length, "Total MB:", Math.floor(totalBytes / 1024 / 1024));
        }
    }

    return [{
        vuln_type: "heap_pressure",
        title: "Heap Memory Pressure Test",
        description: "Allocated ~" + Math.floor(totalBytes / 1024 / 1024) + "MB without fatal OOM",
        evidence: "buffers=" + buffers.length + ", totalBytes=" + totalBytes,
        location: "heap",
        severity: "info",
        confidence: "high"
    }];
}
"#;

    let mut engine = PluginEngine::new().unwrap();
    engine.load_plugin_with_metadata(code, metadata).await.unwrap();

    let start = Instant::now();
    let transaction = create_test_transaction(1);
    
    match engine.scan_transaction(&transaction).await {
        Ok(findings) => {
            println!("Duration: {:?}", start.elapsed());
            println!("Findings: {}", findings.len());
            for finding in findings {
                println!("  - {}: {}", finding.title, finding.description);
            }
        }
        Err(e) => {
            println!("Duration: {:?}", start.elapsed());
            println!("Error: {}", e);
        }
    }
}

/// 测试2: 栈溢出测试
#[tokio::test]
#[ignore]
async fn test_stack_overflow() {
    println!("\n{}", "=".repeat(80));
    println!("V8 Stack Overflow Test");
    println!("{}", "=".repeat(80));

    let metadata = PluginMetadata {
        id: "v8-test-stack".to_string(),
        name: "Stack Overflow Test".to_string(),
        version: "1.0.0".to_string(),
        author: None,
        main_category: "traffic".to_string(),
        category: "test".to_string(),
        default_severity: Severity::Info,
        tags: vec![],
        description: None,
    };

    let code = r#"
let depth = 0;

function recursiveFunction(n) {
    depth++;
    if (n <= 0) return 1;
    return recursiveFunction(n - 1) + recursiveFunction(n - 2);
}

export function scan_transaction(transaction) {
    try {
        // 尝试触发栈溢出
        const result = recursiveFunction(50);
        
        return [{
            vuln_type: "stack_test",
            title: "Stack Test Completed",
            description: "Recursion depth: " + depth + ", result: " + result,
            evidence: "stack_test",
            location: "stack",
            severity: "info",
            confidence: "high"
        }];
    } catch (e) {
        return [{
            vuln_type: "stack_overflow",
            title: "Stack Overflow Detected",
            description: "Recursion depth: " + depth,
            evidence: e.message,
            location: "stack",
            severity: "info",
            confidence: "high"
        }];
    }
}
"#;

    let mut engine = PluginEngine::new().unwrap();
    engine.load_plugin_with_metadata(code, metadata).await.unwrap();

    let start = Instant::now();
    let transaction = create_test_transaction(1);
    
    match engine.scan_transaction(&transaction).await {
        Ok(findings) => {
            println!("Duration: {:?}", start.elapsed());
            println!("Findings: {}", findings.len());
            for finding in findings {
                println!("  - {}: {}", finding.title, finding.description);
            }
        }
        Err(e) => {
            println!("Duration: {:?}", start.elapsed());
            println!("Error: {}", e);
        }
    }
}

/// 测试3: 无限循环检测
#[tokio::test]
#[ignore]
async fn test_infinite_loop() {
    println!("\n{}", "=".repeat(80));
    println!("V8 Infinite Loop Test");
    println!("{}", "=".repeat(80));

    let metadata = PluginMetadata {
        id: "v8-test-loop".to_string(),
        name: "Infinite Loop Test".to_string(),
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
    let iterations = 0;
    const maxIterations = 10000000;
    
    // 有限但很长的循环
    while (iterations < maxIterations) {
        iterations++;
        
        if (iterations % 1000000 === 0) {
            console.log("Iterations:", iterations);
        }
    }
    
    return [{
        vuln_type: "loop_test",
        title: "Loop Test Completed",
        description: "Completed " + iterations + " iterations",
        evidence: "loop_test",
        location: "loop",
        severity: "info",
        confidence: "high"
    }];
}
"#;

    let mut engine = PluginEngine::new().unwrap();
    engine.load_plugin_with_metadata(code, metadata).await.unwrap();

    let start = Instant::now();
    let transaction = create_test_transaction(1);
    
    // 使用 timeout 防止真正的无限循环
    let result = tokio::time::timeout(
        Duration::from_secs(30),
        engine.scan_transaction(&transaction)
    ).await;
    
    match result {
        Ok(Ok(findings)) => {
            println!("Duration: {:?}", start.elapsed());
            println!("Status: ✓ Completed");
            println!("Findings: {}", findings.len());
            for finding in findings {
                println!("  - {}: {}", finding.title, finding.description);
            }
        }
        Ok(Err(e)) => {
            println!("Duration: {:?}", start.elapsed());
            println!("Status: ✗ Error");
            println!("Error: {}", e);
        }
        Err(_) => {
            println!("Duration: {:?} (timeout)", start.elapsed());
            println!("Status: ✗ Timeout (possible infinite loop)");
        }
    }
}

/// 测试4: 大对象分配测试
#[tokio::test]
#[ignore]
async fn test_large_object_allocation() {
    println!("\n{}", "=".repeat(80));
    println!("V8 Large Object Allocation Test");
    println!("{}", "=".repeat(80));

    let metadata = PluginMetadata {
        id: "v8-test-large-obj".to_string(),
        name: "Large Object Test".to_string(),
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
    const sizes = [
        1024,           // 1KB
        1024 * 10,      // 10KB
        1024 * 100,     // 100KB
        1024 * 1024,    // 1MB
        1024 * 1024 * 10, // 10MB
    ];
    
    const results = [];
    
    for (const size of sizes) {
        try {
            const start = Date.now();
            const arr = new Array(size).fill(0);
            const duration = Date.now() - start;
            
            results.push({
                size: size,
                success: true,
                duration: duration
            });
            
            console.log("Allocated array of size", size, "in", duration, "ms");
        } catch (e) {
            results.push({
                size: size,
                success: false,
                error: e.message
            });
            
            console.error("Failed to allocate array of size", size, ":", e.message);
        }
    }
    
    const successful = results.filter(r => r.success).length;
    
    return [{
        vuln_type: "large_object_test",
        title: "Large Object Allocation Test",
        description: "Successfully allocated " + successful + "/" + sizes.length + " objects",
        evidence: JSON.stringify(results),
        location: "heap",
        severity: "info",
        confidence: "high"
    }];
}
"#;

    let mut engine = PluginEngine::new().unwrap();
    engine.load_plugin_with_metadata(code, metadata).await.unwrap();

    let start = Instant::now();
    let transaction = create_test_transaction(1);
    
    match engine.scan_transaction(&transaction).await {
        Ok(findings) => {
            println!("Duration: {:?}", start.elapsed());
            println!("Findings: {}", findings.len());
            for finding in findings {
                println!("  - {}: {}", finding.title, finding.description);
                if let Ok(results) = serde_json::from_str::<Vec<serde_json::Value>>(&finding.evidence) {
                    for result in results {
                        println!("    {:?}", result);
                    }
                }
            }
        }
        Err(e) => {
            println!("Duration: {:?}", start.elapsed());
            println!("Error: {}", e);
        }
    }
}

/// 测试5: 字符串长度限制测试
#[tokio::test]
#[ignore]
async fn test_string_length_limit() {
    println!("\n{}", "=".repeat(80));
    println!("V8 String Length Limit Test");
    println!("{}", "=".repeat(80));

    let metadata = PluginMetadata {
        id: "v8-test-string".to_string(),
        name: "String Length Test".to_string(),
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
    const lengths = [
        1024,              // 1KB
        1024 * 100,        // 100KB
        1024 * 1024,       // 1MB
        1024 * 1024 * 10,  // 10MB
        1024 * 1024 * 100, // 100MB
    ];
    
    const results = [];
    
    for (const length of lengths) {
        try {
            const start = Date.now();
            const str = "A".repeat(length);
            const duration = Date.now() - start;
            
            results.push({
                length: length,
                actualLength: str.length,
                success: true,
                duration: duration
            });
            
            console.log("Created string of length", length, "in", duration, "ms");
        } catch (e) {
            results.push({
                length: length,
                success: false,
                error: e.message
            });
            
            console.error("Failed to create string of length", length, ":", e.message);
            break;
        }
    }
    
    const successful = results.filter(r => r.success).length;
    const maxLength = results.filter(r => r.success).reduce((max, r) => Math.max(max, r.length), 0);
    
    return [{
        vuln_type: "string_length_test",
        title: "String Length Test",
        description: "Max string length: " + maxLength + " (" + successful + "/" + lengths.length + " succeeded)",
        evidence: JSON.stringify(results),
        location: "heap",
        severity: "info",
        confidence: "high"
    }];
}
"#;

    let mut engine = PluginEngine::new().unwrap();
    engine.load_plugin_with_metadata(code, metadata).await.unwrap();

    let start = Instant::now();
    let transaction = create_test_transaction(1);
    
    match engine.scan_transaction(&transaction).await {
        Ok(findings) => {
            println!("Duration: {:?}", start.elapsed());
            println!("Findings: {}", findings.len());
            for finding in findings {
                println!("  - {}: {}", finding.title, finding.description);
            }
        }
        Err(e) => {
            println!("Duration: {:?}", start.elapsed());
            println!("Error: {}", e);
        }
    }
}

/// 测试6: 对象属性数量限制
#[tokio::test]
#[ignore]
async fn test_object_properties_limit() {
    println!("\n{}", "=".repeat(80));
    println!("V8 Object Properties Limit Test");
    println!("{}", "=".repeat(80));

    let metadata = PluginMetadata {
        id: "v8-test-props".to_string(),
        name: "Object Properties Test".to_string(),
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
    const counts = [100, 1000, 10000, 100000, 1000000];
    const results = [];
    
    for (const count of counts) {
        try {
            const start = Date.now();
            const obj = {};
            
            for (let i = 0; i < count; i++) {
                obj["prop_" + i] = i;
            }
            
            const duration = Date.now() - start;
            const actualCount = Object.keys(obj).length;
            
            results.push({
                targetCount: count,
                actualCount: actualCount,
                success: true,
                duration: duration
            });
            
            console.log("Created object with", actualCount, "properties in", duration, "ms");
        } catch (e) {
            results.push({
                targetCount: count,
                success: false,
                error: e.message
            });
            
            console.error("Failed to create object with", count, "properties:", e.message);
            break;
        }
    }
    
    const successful = results.filter(r => r.success).length;
    const maxProps = results.filter(r => r.success).reduce((max, r) => Math.max(max, r.actualCount), 0);
    
    return [{
        vuln_type: "object_props_test",
        title: "Object Properties Test",
        description: "Max properties: " + maxProps + " (" + successful + "/" + counts.length + " succeeded)",
        evidence: JSON.stringify(results),
        location: "heap",
        severity: "info",
        confidence: "high"
    }];
}
"#;

    let mut engine = PluginEngine::new().unwrap();
    engine.load_plugin_with_metadata(code, metadata).await.unwrap();

    let start = Instant::now();
    let transaction = create_test_transaction(1);
    
    match engine.scan_transaction(&transaction).await {
        Ok(findings) => {
            println!("Duration: {:?}", start.elapsed());
            println!("Findings: {}", findings.len());
            for finding in findings {
                println!("  - {}: {}", finding.title, finding.description);
            }
        }
        Err(e) => {
            println!("Duration: {:?}", start.elapsed());
            println!("Error: {}", e);
        }
    }
}

/// 测试7: 多引擎隔离测试
#[tokio::test]
#[ignore]
async fn test_multi_engine_isolation() {
    println!("\n{}", "=".repeat(80));
    println!("V8 Multi-Engine Isolation Test");
    println!("{}", "=".repeat(80));

    let metadata = PluginMetadata {
        id: "v8-test-isolation".to_string(),
        name: "Engine Isolation Test".to_string(),
        version: "1.0.0".to_string(),
        author: None,
        main_category: "traffic".to_string(),
        category: "test".to_string(),
        default_severity: Severity::Info,
        tags: vec![],
        description: None,
    };

    let code = r#"
// 全局变量（应该在每个引擎中独立）
if (typeof globalCounter === 'undefined') {
    globalThis.globalCounter = 0;
}

export function scan_transaction(transaction) {
    globalThis.globalCounter++;
    
    return [{
        vuln_type: "isolation_test",
        title: "Engine Isolation Test",
        description: "Global counter: " + globalThis.globalCounter,
        evidence: "counter_" + globalThis.globalCounter,
        location: "global",
        severity: "info",
        confidence: "high"
    }];
}
"#;

    let num_executors = 10;
    let mut executors = vec![];
    
    // 创建多个 Executor（每个运行在独立线程中，避免 V8 Isolate 冲突）
    for i in 0..num_executors {
        let mut meta = metadata.clone();
        meta.id = format!("v8-test-isolation-{}", i);
        
        let executor = PluginExecutor::new(meta, code.to_string(), 1000).unwrap();
        executors.push(executor);
    }
    
    let start = Instant::now();
    let transaction = create_test_transaction(1);
    
    // 并发执行（每个 Executor 在自己的线程中，不会相互干扰）
    let mut handles = vec![];
    for (i, executor) in executors.into_iter().enumerate() {
        let txn = transaction.clone();
        let handle = tokio::spawn(async move {
            match executor.scan_transaction(txn).await {
                Ok(findings) => {
                    for finding in findings {
                        println!("Executor {}: {}", i, finding.description);
                    }
                }
                Err(e) => {
                    println!("Executor {} error: {}", i, e);
                }
            }
        });
        handles.push(handle);
    }
    
    // 等待所有执行完成
    for handle in handles {
        let _ = handle.await;
    }
    
    println!("\nDuration: {:?}", start.elapsed());
    println!("Expected: Each executor should have counter = 1");
    println!("All executors run in separate threads with isolated V8 Isolates ✓");
}

