//! Integration tests for the sentinel-js plugin runtime.

use sentinel_js::{PluginRuntime, RuntimeConfig, HostBindingsExt};

fn make_runtime() -> PluginRuntime {
    PluginRuntime::new(RuntimeConfig::default()).expect("Failed to create runtime")
}

#[test]
fn test_basic_eval() {
    let rt = make_runtime();
    let result = rt.eval("1 + 2").unwrap();
    assert_eq!(result, serde_json::json!(3));
}

#[test]
fn test_string_eval() {
    let rt = make_runtime();
    let result = rt.eval("'hello' + ' world'").unwrap();
    assert_eq!(result, serde_json::json!("hello world"));
}

#[test]
fn test_console_log() {
    let rt = make_runtime();
    rt.eval_void("console.log('test message')").unwrap();
    rt.eval_void("console.warn('warning')").unwrap();
    rt.eval_void("console.error('error')").unwrap();
}

#[test]
fn test_set_timeout() {
    let rt = make_runtime();
    let result = rt.eval(r#"
        var called = false;
        setTimeout(function() { called = true; }, 0);
        called
    "#).unwrap();
    // After eval, pending jobs should have run
    rt.execute_pending_jobs().unwrap();
    let result2 = rt.eval("called").unwrap();
    assert_eq!(result2, serde_json::json!(true));
}

#[test]
fn test_promise() {
    let rt = make_runtime();
    rt.eval_void(r#"
        var resolved = false;
        Promise.resolve(42).then(function(v) { resolved = v; });
    "#).unwrap();
    rt.execute_pending_jobs().unwrap();
    let result = rt.eval("resolved").unwrap();
    assert_eq!(result, serde_json::json!(42));
}

#[test]
fn test_text_encoder_decoder() {
    let rt = make_runtime();
    let result = rt.eval(r#"
        var enc = new TextEncoder();
        var bytes = enc.encode("hello");
        var dec = new TextDecoder();
        dec.decode(bytes)
    "#).unwrap();
    assert_eq!(result, serde_json::json!("hello"));
}

#[test]
fn test_url_class() {
    let rt = make_runtime();
    let result = rt.eval(r#"
        var u = new URL("https://example.com:8080/path?foo=bar#hash");
        JSON.stringify({
            protocol: u.protocol,
            hostname: u.hostname,
            port: u.port,
            pathname: u.pathname,
            search: u.search,
            hash: u.hash
        })
    "#).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(result.as_str().unwrap()).unwrap();
    assert_eq!(parsed["protocol"], "https:");
    assert_eq!(parsed["hostname"], "example.com");
    assert_eq!(parsed["port"], "8080");
    assert_eq!(parsed["pathname"], "/path");
    assert_eq!(parsed["search"], "?foo=bar");
    assert_eq!(parsed["hash"], "#hash");
}

#[test]
fn test_url_search_params() {
    let rt = make_runtime();
    let result = rt.eval(r#"
        var params = new URLSearchParams("a=1&b=2&c=3");
        params.get("b")
    "#).unwrap();
    assert_eq!(result, serde_json::json!("2"));
}

#[test]
fn test_btoa_atob() {
    let rt = make_runtime();
    let result = rt.eval(r#"
        var encoded = btoa("hello world");
        atob(encoded)
    "#).unwrap();
    assert_eq!(result, serde_json::json!("hello world"));
}

#[test]
fn test_performance_now() {
    let rt = make_runtime();
    let result = rt.eval("typeof performance.now()").unwrap();
    assert_eq!(result, serde_json::json!("number"));
}

#[test]
fn test_crypto_random_uuid() {
    let rt = make_runtime();
    let result = rt.eval("crypto.randomUUID().length").unwrap();
    assert_eq!(result, serde_json::json!(36));
}

#[test]
fn test_set_global() {
    let rt = make_runtime();
    rt.set_global("myVar", &serde_json::json!({"key": "value", "num": 42})).unwrap();
    let result = rt.eval("myVar.key + ':' + myVar.num").unwrap();
    assert_eq!(result, serde_json::json!("value:42"));
}

#[test]
fn test_call_function() {
    let rt = make_runtime();
    rt.eval_void("function add(input) { return input.a + input.b; }").unwrap();
    let result = rt.call_function("add", &serde_json::json!({"a": 3, "b": 4})).unwrap();
    assert_eq!(result, serde_json::json!(7));
}

#[test]
fn test_require_path() {
    let rt = make_runtime();
    let result = rt.eval(r#"
        var path = require("path");
        path.join("/foo", "bar", "baz.txt")
    "#).unwrap();
    assert_eq!(result, serde_json::json!("/foo/bar/baz.txt"));
}

#[test]
fn test_require_path_basename() {
    let rt = make_runtime();
    let result = rt.eval(r#"
        var path = require("path");
        path.basename("/foo/bar/file.txt")
    "#).unwrap();
    assert_eq!(result, serde_json::json!("file.txt"));
}

#[test]
fn test_buffer_from() {
    let rt = make_runtime();
    let result = rt.eval(r#"
        var buf = Buffer.from("hello");
        buf.length
    "#).unwrap();
    assert_eq!(result, serde_json::json!(5));
}

#[test]
fn test_process_platform() {
    let rt = make_runtime();
    let result = rt.eval("process.platform").unwrap();
    assert_eq!(result, serde_json::json!(std::env::consts::OS));
}

#[test]
fn test_host_bindings() {
    let mut bindings = HostBindingsExt::new();
    bindings.register_fn1("__test_double", |json_str| {
        let n: f64 = serde_json::from_str(json_str).unwrap_or(0.0);
        serde_json::to_string(&(n * 2.0)).unwrap()
    });

    let rt = PluginRuntime::with_host_bindings(RuntimeConfig::default(), &bindings).unwrap();
    let result = rt.eval("__test_double(21)").unwrap();
    assert_eq!(result, serde_json::json!(42.0));
}

#[test]
fn test_async_await() {
    let rt = make_runtime();
    rt.eval_void(r#"
        var result = null;
        async function asyncFn() {
            var val = await Promise.resolve(99);
            result = val;
        }
        asyncFn();
    "#).unwrap();
    rt.execute_pending_jobs().unwrap();
    let result = rt.eval("result").unwrap();
    assert_eq!(result, serde_json::json!(99));
}

#[test]
fn test_template_literals() {
    let rt = make_runtime();
    let result = rt.eval(r#"
        var name = "World";
        `Hello ${name}!`
    "#).unwrap();
    assert_eq!(result, serde_json::json!("Hello World!"));
}

#[test]
fn test_error_subclass() {
    let rt = make_runtime();
    let result = rt.eval(r#"
        class CustomError extends Error {
            constructor(msg) {
                super(msg);
                this.name = "CustomError";
            }
        }
        var e = new CustomError("test");
        e.name + ": " + e.message
    "#).unwrap();
    assert_eq!(result, serde_json::json!("CustomError: test"));
}

#[test]
fn test_destructuring() {
    let rt = make_runtime();
    let result = rt.eval(r#"
        var { a, b, ...rest } = { a: 1, b: 2, c: 3, d: 4 };
        a + b + rest.c + rest.d
    "#).unwrap();
    assert_eq!(result, serde_json::json!(10));
}
