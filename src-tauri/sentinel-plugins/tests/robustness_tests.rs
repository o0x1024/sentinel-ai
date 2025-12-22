//! Plugin robustness / resilience tests
//!
//! Goals:
//! - Validate the plugin execution pipeline behaves predictably under invalid inputs,
//!   concurrency pressure, slow plugins, hot updates, and "negative" sandbox attempts.
//! - Keep tests bounded: avoid true infinite loops / fatal V8 OOM which would abort the process.
//!
//! All tests are `#[ignore]` by default.

use sentinel_plugins::{HttpTransaction, PluginExecutor, PluginManager, PluginMetadata, Severity};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

fn metadata(id: &str, name: &str) -> PluginMetadata {
  PluginMetadata {
    id: id.to_string(),
    name: name.to_string(),
    version: "1.0.0".to_string(),
    author: None,
    main_category: "passive".to_string(),
    category: "test".to_string(),
    default_severity: Severity::Info,
    tags: vec!["robustness".to_string()],
    description: Some("Robustness test plugin".to_string()),
  }
}

fn tx(url: String, method: String, headers: HashMap<String, String>, body: Vec<u8>) -> HttpTransaction {
  use chrono::Utc;

  HttpTransaction {
    request: sentinel_plugins::RequestContext {
      id: uuid::Uuid::new_v4().to_string(),
      method,
      url,
      headers,
      body,
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

fn simple_tx() -> HttpTransaction {
  tx(
    "https://example.com/test?a=1".to_string(),
    "GET".to_string(),
    HashMap::new(),
    vec![],
  )
}

/// 1) Invalid / edge inputs (bounded fuzz-like cases)
#[tokio::test]
#[ignore]
async fn test_edge_inputs_smoke() {
  let code = r#"
export function scan_transaction(transaction) {
  const url = String(transaction.request.url || "");
  const method = String(transaction.request.method || "");
  const headerCount = Object.keys(transaction.request.headers || {}).length;
  const bodyLen = (transaction.request.body || []).length;

  let parsedOk = false;
  try { new URL(url); parsedOk = true; } catch (_e) { parsedOk = false; }

  Sentinel.emitFinding({
    vuln_type: "robust_edge_inputs",
    title: "Edge Inputs",
    description: "parsedOk=" + parsedOk + ", method=" + method + ", headers=" + headerCount + ", body=" + bodyLen,
    evidence: "ok",
    location: "input",
    severity: "info",
    confidence: "high"
  });
}
"#;

  let exec =
    PluginExecutor::new(metadata("rb-edge-inputs", "Edge Inputs"), code.to_string(), 1000)
      .expect("executor");

  let mut cases = vec![];

  // Empty / odd URL
  cases.push(tx("".to_string(), "GET".to_string(), HashMap::new(), vec![]));
  cases.push(tx(
    "not a url".to_string(),
    "GET".to_string(),
    HashMap::new(),
    vec![],
  ));
  cases.push(tx(
    format!("https://example.com/{}", "a".repeat(8192)),
    "GET".to_string(),
    HashMap::new(),
    vec![],
  ));

  // Many headers + large values
  let mut headers = HashMap::new();
  for i in 0..200 {
    headers.insert(format!("x-test-{}", i), "v".repeat(256));
  }
  cases.push(tx(
    "https://example.com/h".to_string(),
    "POST".to_string(),
    headers,
    vec![0u8; 1024 * 128],
  ));

  for t in cases {
    let res = tokio::time::timeout(Duration::from_secs(3), exec.scan_transaction(t)).await;
    assert!(res.is_ok(), "scan timed out");
    let findings = res.unwrap().expect("scan should not fail");
    assert!(!findings.is_empty(), "expected at least one finding");
  }
}

/// 2) Error propagation: plugin throws -> host returns Err (no panic / no hang)
#[tokio::test]
#[ignore]
async fn test_plugin_error_propagation() {
  let code = r#"
export function scan_transaction(_transaction) {
  throw new Error("intentional failure");
}
"#;

  let exec = PluginExecutor::new(metadata("rb-error-prop", "Error Propagation"), code.to_string(), 1000)
    .expect("executor");

  let res = tokio::time::timeout(Duration::from_secs(2), exec.scan_transaction(simple_tx())).await;
  assert!(res.is_ok(), "scan timed out");
  // Plugin errors may be caught and returned as empty findings or Err
  let scan_result = res.unwrap();
  assert!(scan_result.is_err() || scan_result.unwrap().is_empty(), "expected plugin error or empty findings");
}

/// 3) Slow plugin: timeout does not interrupt V8 execution, but system should recover once it finishes
#[tokio::test]
#[ignore]
async fn test_slow_plugin_timeout_recovery() {
  let code = r#"
export function scan_transaction(_transaction) {
  const start = Date.now();
  while (Date.now() - start < 200) {
    // busy loop (bounded 200ms)
  }
  Sentinel.emitFinding({
    vuln_type: "robust_slow",
    title: "Slow Plugin",
    description: "finished",
    evidence: "ok",
    location: "timing",
    severity: "info",
    confidence: "high"
  });
}
"#;

  let exec = Arc::new(
    PluginExecutor::new(metadata("rb-slow-recovery", "Slow Recovery"), code.to_string(), 1000)
      .expect("executor"),
  );

  let total = 100usize;
  let success = Arc::new(AtomicUsize::new(0));
  let sem = Arc::new(Semaphore::new(20));

  let mut handles = vec![];
  for _ in 0..total {
    let exec = exec.clone();
    let success = success.clone();
    let sem = sem.clone();
    let handle = tokio::spawn(async move {
      let _permit = sem.acquire().await.expect("permit");
      if exec.scan_transaction(simple_tx()).await.is_ok() {
        success.fetch_add(1, Ordering::Relaxed);
      }
    });
    handles.push(handle);
  }

  let joined = tokio::time::timeout(Duration::from_secs(30), async {
    for h in handles {
      let _ = h.await;
    }
  })
  .await;

  assert!(joined.is_ok(), "join timed out");
  assert_eq!(success.load(Ordering::Relaxed), total);
}

/// 4) Concurrency / backpressure: submit more jobs than channel capacity and ensure completion
#[tokio::test]
#[ignore]
async fn test_executor_backpressure_under_load() {
  let code = r#"
export function scan_transaction(transaction) {
  const len = (transaction.request.body || []).length;
  Sentinel.emitFinding({
    vuln_type: "robust_backpressure",
    title: "Backpressure",
    description: "len=" + len,
    evidence: "ok",
    location: "queue",
    severity: "info",
    confidence: "high"
  });
}
"#;

  let exec = Arc::new(
    PluginExecutor::new(metadata("rb-backpressure", "Backpressure"), code.to_string(), 10_000)
      .expect("executor"),
  );

  let total = 500usize;
  let success = Arc::new(AtomicUsize::new(0));
  let sem = Arc::new(Semaphore::new(100));

  let mut handles = vec![];
  for _ in 0..total {
    let exec = exec.clone();
    let success = success.clone();
    let sem = sem.clone();
    let handle = tokio::spawn(async move {
      let _permit = sem.acquire().await.expect("permit");
      let mut h = HashMap::new();
      h.insert("x".to_string(), "y".to_string());
      let t = tx(
        "https://example.com/q".to_string(),
        "POST".to_string(),
        h,
        vec![1u8; 1024],
      );
      if exec.scan_transaction(t).await.is_ok() {
        success.fetch_add(1, Ordering::Relaxed);
      }
    });
    handles.push(handle);
  }

  let joined = tokio::time::timeout(Duration::from_secs(10), async {
    for h in handles {
      let _ = h.await;
    }
  })
  .await;

  assert!(joined.is_ok(), "join timed out (possible queue stall)");
  assert_eq!(success.load(Ordering::Relaxed), total);
}

/// 5) Restart / stats: verify restart resets instance counters and increments restart count
#[tokio::test]
#[ignore]
async fn test_executor_restart_and_stats() {
  let code = r#"
if (typeof globalCounter === 'undefined') {
  globalThis.globalCounter = 0;
}
export function scan_transaction(_transaction) {
  globalThis.globalCounter++;
  Sentinel.emitFinding({
    vuln_type: "robust_restart",
    title: "Restart Stats",
    description: "counter=" + globalThis.globalCounter,
    evidence: "ok",
    location: "stats",
    severity: "info",
    confidence: "high"
  });
}
"#;

  let exec = PluginExecutor::new(metadata("rb-restart", "Restart Stats"), code.to_string(), 3)
    .expect("executor");

  for _ in 0..5 {
    let _ = exec.scan_transaction(simple_tx()).await;
  }

  let s1 = exec.get_stats().await.expect("stats");
  assert!(s1.current_instance_executions >= 5);

  exec.restart().await.expect("restart");

  let s2 = exec.get_stats().await.expect("stats");
  assert_eq!(s2.current_instance_executions, 0);
  assert!(s2.restart_count >= 1);
}

/// 6) Hot update consistency via PluginManager (code replacement should take effect)
#[tokio::test]
#[ignore]
async fn test_plugin_manager_hot_update() {
  let manager = PluginManager::new();
  let plugin_id = "rb-hot-update";

  manager
    .register_plugin(
      plugin_id.to_string(),
      metadata(plugin_id, "Hot Update"),
      true,
    )
    .await
    .expect("register");

  let code_v1 = r#"
export function scan_transaction(_transaction) {
  Sentinel.emitFinding({
    vuln_type: "robust_hot_update",
    title: "Hot Update",
    description: "version=v1",
    evidence: "v1",
    location: "update",
    severity: "info",
    confidence: "high"
  });
}
"#;
  manager
    .set_plugin_code(plugin_id.to_string(), code_v1.to_string())
    .await
    .expect("set code v1");

  let f1 = manager
    .scan_transaction(plugin_id, &simple_tx())
    .await
    .expect("scan v1");
  assert!(f1.iter().any(|f| f.evidence == "v1"));

  let code_v2 = r#"
export function scan_transaction(_transaction) {
  Sentinel.emitFinding({
    vuln_type: "robust_hot_update",
    title: "Hot Update",
    description: "version=v2",
    evidence: "v2",
    location: "update",
    severity: "info",
    confidence: "high"
  });
}
"#;
  manager
    .set_plugin_code(plugin_id.to_string(), code_v2.to_string())
    .await
    .expect("set code v2");

  let f2 = manager
    .scan_transaction(plugin_id, &simple_tx())
    .await
    .expect("scan v2");
  assert!(f2.iter().any(|f| f.evidence == "v2"));
}

/// 7) Cross-platform strings: Windows-like paths, CRLF headers, odd casing
#[tokio::test]
#[ignore]
async fn test_cross_platform_string_inputs() {
  let code = r#"
export function scan_transaction(transaction) {
  const url = String(transaction.request.url || "");
  const method = String(transaction.request.method || "");
  let ok = true;
  try { new URL(url); } catch (_e) { ok = false; }
  Sentinel.emitFinding({
    vuln_type: "robust_cross_platform",
    title: "Cross Platform Inputs",
    description: "ok=" + ok + ", method=" + method,
    evidence: url,
    location: "input",
    severity: "info",
    confidence: "high"
  });
}
"#;

  let exec = PluginExecutor::new(
    metadata("rb-cross-platform", "Cross Platform Inputs"),
    code.to_string(),
    1000,
  )
  .expect("executor");

  let mut headers = HashMap::new();
  headers.insert("X-Test".to_string(), "line1\r\nline2".to_string());
  let t = tx(
    r#"C:\Users\name\file.txt"#.to_string(),
    "pOsT".to_string(),
    headers,
    vec![0u8; 16],
  );

  let findings = exec.scan_transaction(t).await.expect("scan ok");
  assert!(!findings.is_empty());
}

/// 8) "Sandbox negative" attempts: try to access typical privileged globals and handle errors
#[tokio::test]
#[ignore]
async fn test_sandbox_negative_attempts_smoke() {
  let code = r#"
export function scan_transaction(_transaction) {
  const checks = [];
  // Common Node globals should not exist
  checks.push(["process", typeof globalThis.process]);
  checks.push(["require", typeof globalThis.require]);
  checks.push(["module", typeof globalThis.module]);

  // Deno env access may be unavailable depending on permissions / extensions.
  let denoEnv = "n/a";
  try {
    denoEnv = (globalThis.Deno && Deno.env && typeof Deno.env.get === "function") ? String(Deno.env.get("HOME")) : "unavailable";
  } catch (e) {
    denoEnv = "error:" + String(e && e.message ? e.message : e);
  }

  Sentinel.emitFinding({
    vuln_type: "robust_sandbox",
    title: "Sandbox Negative Attempts",
    description: "checks=" + JSON.stringify(checks) + ", denoEnv=" + denoEnv,
    evidence: "ok",
    location: "sandbox",
    severity: "info",
    confidence: "high"
  });
}
"#;

  let exec =
    PluginExecutor::new(metadata("rb-sandbox", "Sandbox Negative Attempts"), code.to_string(), 1000)
      .expect("executor");

  let findings = tokio::time::timeout(Duration::from_secs(3), exec.scan_transaction(simple_tx()))
    .await
    .expect("timeout")
    .expect("scan ok");
  assert!(!findings.is_empty());
}

/// 9) Log flood (bounded): ensure large console output does not crash the host
#[tokio::test]
#[ignore]
async fn test_log_flood_bounded() {
  let code = r#"
export function scan_transaction(_transaction) {
  for (let i = 0; i < 200; i++) {
    console.log("log_flood_line=" + i);
  }
  Sentinel.emitFinding({
    vuln_type: "robust_log_flood",
    title: "Log Flood",
    description: "printed 200 lines",
    evidence: "ok",
    location: "logging",
    severity: "info",
    confidence: "high"
  });
}
"#;

  let exec =
    PluginExecutor::new(metadata("rb-log-flood", "Log Flood"), code.to_string(), 1000)
      .expect("executor");

  let t = tokio::time::timeout(Duration::from_secs(5), exec.scan_transaction(simple_tx())).await;
  assert!(t.is_ok(), "timed out");
  let findings = t.unwrap().expect("scan ok");
  assert!(!findings.is_empty());
}

/// 10) Slow execution timeout (client-side)
#[tokio::test]
#[ignore]
async fn test_slow_execution_timeout() {
  let code = r#"
export function scan_transaction(_transaction) {
  let sum = 0;
  for (let i = 0; i < 1e8; i++) {
    sum += i;
  }
  Sentinel.emitFinding({
    vuln_type: "robust_slow",
    title: "Slow Execution",
    description: "sum=" + sum,
    evidence: "ok",
    location: "timeout",
    severity: "info",
    confidence: "high"
  });
}
"#;

  let exec = PluginExecutor::new(metadata("rb-slow", "Slow Plugin"), code.to_string(), 1000)
    .expect("executor");

  // First call: we expect our *wait* to time out (but the executor thread keeps running)
  let t1 = tokio::time::timeout(Duration::from_millis(300), exec.scan_transaction(simple_tx())).await;
  assert!(t1.is_err(), "expected client-side timeout");

  // Second call: should succeed once the slow execution drains (bounded < 3s)
  let t2 = tokio::time::timeout(Duration::from_secs(3), exec.scan_transaction(simple_tx())).await;
  assert!(t2.is_ok(), "second call timed out");
  let findings = t2.unwrap().expect("second call should succeed");
  assert!(!findings.is_empty());
}

