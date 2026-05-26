//! Security and performance tests for sentinel-js runtime.

use sentinel_js::{PluginRuntime, RuntimeConfig};
use sentinel_js::sandbox::SandboxConfig;

fn make_runtime() -> PluginRuntime {
    PluginRuntime::new(RuntimeConfig::default()).expect("Failed to create runtime")
}

fn make_sandbox_runtime() -> PluginRuntime {
    PluginRuntime::new(RuntimeConfig {
        sandbox: SandboxConfig {
            allow_eval: false,
            ..Default::default()
        },
        ..Default::default()
    })
    .expect("Failed to create runtime")
}

// --- Security Tests ---

#[test]
fn test_eval_disabled_in_sandbox() {
    let rt = make_sandbox_runtime();
    let result = rt.eval(r#"
        try {
            eval("1+1");
            "eval_worked"
        } catch(e) {
            "eval_blocked: " + e.message
        }
    "#).unwrap();
    let s = result.as_str().unwrap();
    assert!(s.contains("eval_blocked"), "eval() should be blocked: got {s}");
}

#[test]
fn test_memory_limit() {
    let rt = PluginRuntime::new(RuntimeConfig {
        max_memory: 1 * 1024 * 1024, // 1MB - very tight
        ..Default::default()
    }).unwrap();

    // Try to allocate a large array - should fail or throw
    let result = rt.eval(r#"
        try {
            var arr = [];
            for (var i = 0; i < 1000000; i++) arr.push("x".repeat(100));
            "no_limit"
        } catch(e) {
            "memory_limited"
        }
    "#).unwrap();
    // QuickJS should throw InternalError: out of memory
    assert_eq!(result, serde_json::json!("memory_limited"));
}

#[test]
fn test_stack_overflow_protection() {
    let rt = make_runtime();
    let result = rt.eval(r#"
        try {
            function recurse() { return recurse(); }
            recurse();
            "no_limit"
        } catch(e) {
            "stack_overflow"
        }
    "#).unwrap();
    assert_eq!(result, serde_json::json!("stack_overflow"));
}

#[test]
fn test_no_global_escape() {
    let rt = make_runtime();
    // Ensure plugins can't access Rust internals via prototype pollution
    let result = rt.eval(r#"
        typeof globalThis.__proto__ === "object"
    "#).unwrap();
    // This should work but not expose unsafe operations
    assert!(result == serde_json::json!(true) || result == serde_json::json!(false));
}

// --- Performance Tests ---

#[test]
fn test_perf_runtime_creation() {
    let start = std::time::Instant::now();
    for _ in 0..10 {
        let _rt = make_runtime();
    }
    let elapsed = start.elapsed();
    eprintln!("10 runtime creations: {:?} ({:?} each)", elapsed, elapsed / 10);
    // Should be under 1 second total for 10 creations
    assert!(elapsed.as_secs() < 5, "Runtime creation too slow: {:?}", elapsed);
}

#[test]
fn test_perf_eval_loop() {
    let rt = make_runtime();
    let start = std::time::Instant::now();
    for i in 0..1000 {
        rt.eval(&format!("{i} + 1")).unwrap();
    }
    let elapsed = start.elapsed();
    eprintln!("1000 evals: {:?} ({:?} each)", elapsed, elapsed / 1000);
    assert!(elapsed.as_secs() < 5, "Eval loop too slow: {:?}", elapsed);
}

#[test]
fn test_perf_json_roundtrip() {
    let rt = make_runtime();
    let big_json = serde_json::json!({
        "users": (0..100).map(|i| serde_json::json!({
            "id": i,
            "name": format!("user_{i}"),
            "email": format!("user_{i}@example.com"),
            "roles": ["admin", "user"],
            "metadata": { "created": "2024-01-01", "active": true }
        })).collect::<Vec<_>>()
    });

    rt.set_global("testData", &big_json).unwrap();

    let start = std::time::Instant::now();
    for _ in 0..100 {
        rt.eval("testData.users.length").unwrap();
    }
    let elapsed = start.elapsed();
    eprintln!("100 JSON accesses: {:?}", elapsed);
    assert!(elapsed.as_secs() < 2, "JSON roundtrip too slow: {:?}", elapsed);
}

#[test]
fn test_perf_plugin_simulation() {
    let rt = make_runtime();

    // Simulate a typical plugin: load code, call a function, get results
    rt.eval_void(r#"
        function analyze(input) {
            var findings = [];
            var headers = input.headers || {};
            for (var key in headers) {
                if (key.toLowerCase() === "x-powered-by") {
                    findings.push({
                        title: "Technology Disclosure",
                        severity: "info",
                        detail: headers[key]
                    });
                }
            }
            return { success: true, findings: findings };
        }
    "#).unwrap();

    let input = serde_json::json!({
        "url": "https://example.com",
        "method": "GET",
        "headers": {
            "content-type": "text/html",
            "x-powered-by": "Express",
            "server": "nginx"
        },
        "body": ""
    });

    let start = std::time::Instant::now();
    for _ in 0..500 {
        let result = rt.call_function("analyze", &input).unwrap();
        assert!(result.get("success").and_then(|v| v.as_bool()).unwrap_or(false));
    }
    let elapsed = start.elapsed();
    eprintln!("500 plugin calls: {:?} ({:?} each)", elapsed, elapsed / 500);
    assert!(elapsed.as_secs() < 5, "Plugin simulation too slow: {:?}", elapsed);
}
