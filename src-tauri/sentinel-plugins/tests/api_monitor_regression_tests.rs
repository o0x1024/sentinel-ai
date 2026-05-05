use std::fs;

#[cfg(feature = "plugin-ts-transpile")]
use sentinel_plugins::plugin_engine::PluginEngine;
#[cfg(feature = "plugin-ts-transpile")]
use sentinel_plugins::types::{PluginMetadata, Severity};

#[cfg(feature = "plugin-ts-transpile")]
#[tokio::test]
async fn api_monitor_extracts_api_literals_from_large_bundle() {
    let mut engine = PluginEngine::new().expect("Failed to create plugin engine");

    let mut code = String::from(
        r#"
globalThis.fetch = async function(input, init) {
    const resolvedUrl = typeof input === "string"
        ? input
        : (input && typeof input.url === "string" ? input.url : String(input));

    function makeResponse(body, contentType, status = 200, finalUrl = resolvedUrl) {
        return {
            ok: status >= 200 && status < 300,
            status,
            url: finalUrl,
            headers: {
                get(name) {
                    const key = String(name || "").toLowerCase();
                    if (key === "content-type") return contentType;
                    if (key === "content-length") return String(body.length);
                    return null;
                }
            },
            async text() {
                return body;
            }
        };
    }

    if (resolvedUrl === "https://example.test" || resolvedUrl === "https://example.test/") {
        return makeResponse(
            '<!doctype html><html><body><script src="/assets/app.js"></script></body></html>',
            "text/html",
        );
    }

    if (resolvedUrl === "https://example.test/assets/app.js") {
        const filler = "// filler line\n".repeat(50000);
        const bundle = [
            filler,
            "fetch('/api/health');",
            "axios.get('/api/users');",
            "axios({ url: '/api/profile', method: 'GET' });",
        ].join("\n");
        return makeResponse(bundle, "application/javascript");
    }

    if (resolvedUrl.endsWith("/api/health")) {
        return makeResponse('{"ok":true}', "application/json");
    }

    if (resolvedUrl.endsWith("/api/users")) {
        return makeResponse('[{"id":1}]', "application/json");
    }

    if (resolvedUrl.endsWith("/api/profile")) {
        return makeResponse('{"profile":"ok"}', "application/json");
    }

    return makeResponse("Not Found", "text/plain", 404);
};
"#,
    );
    code.push_str(include_str!("../runtime/agent/api_monitor.ts"));

    let metadata = PluginMetadata {
        id: "api_monitor_regression".to_string(),
        name: "API Monitor Regression".to_string(),
        version: "1.0.0".to_string(),
        author: Some("Test".to_string()),
        description: Some("Regression test for large JS bundle AST literal extraction".to_string()),
        monitor_type: Some("api".to_string()),
        target_asset_types: vec![],
        main_category: "agent".to_string(),
        category: "monitor".to_string(),
        tags: vec![],
        default_severity: Severity::Info,
    };

    engine
        .load_plugin_with_metadata(&code, metadata)
        .await
        .expect("Failed to load api_monitor plugin");

    let input = serde_json::json!({
        "targets": ["example.test"],
        "timeout": 500,
        "concurrency": 4,
        "maxJsFiles": 5,
    });

    let (_findings, result) = engine
        .execute_agent(&input)
        .await
        .expect("Failed to execute api_monitor");

    let result = result.expect("api_monitor should return a result");
    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["summary"]["failedChecks"], 0);
    assert_eq!(result["data"]["summary"]["successfulChecks"], 1);

    let target_result = &result["data"]["results"][0];
    assert_eq!(target_result["success"], true);

    let endpoints = target_result["snapshot"]["endpoints"]
        .as_array()
        .expect("snapshot endpoints should be an array");

    assert!(endpoints
        .iter()
        .any(|endpoint| endpoint["path"] == "/api/health"));
    assert!(endpoints
        .iter()
        .any(|endpoint| endpoint["path"] == "/api/users"));
    assert!(endpoints
        .iter()
        .any(|endpoint| endpoint["path"] == "/api/profile"));
}

#[cfg(feature = "plugin-ts-transpile")]
#[tokio::test]
async fn api_monitor_keeps_path_like_literals_and_filters_static_assets() {
    let mut engine = PluginEngine::new().expect("Failed to create plugin engine");

    let mut code = String::from(
        r#"
globalThis.fetch = async function(input, init) {
    const resolvedUrl = typeof input === "string"
        ? input
        : (input && typeof input.url === "string" ? input.url : String(input));

    function makeResponse(body, contentType, status = 200, finalUrl = resolvedUrl) {
        return {
            ok: status >= 200 && status < 300,
            status,
            url: finalUrl,
            headers: {
                get(name) {
                    const key = String(name || "").toLowerCase();
                    if (key === "content-type") return contentType;
                    if (key === "content-length") return String(body.length);
                    return null;
                }
            },
            async text() {
                return body;
            }
        };
    }

    if (resolvedUrl === "https://candidate.test" || resolvedUrl === "https://candidate.test/") {
        return makeResponse(
            '<!doctype html><html><body><script src="/assets/app.js"></script></body></html>',
            "text/html",
        );
    }

    if (resolvedUrl === "https://candidate.test/assets/app.js") {
        const bundle = [
            "const paths = ['/channel/list', '/content/list', '/api/users', '/assets/logo.png', '/_next/static/chunk.js', '/manifest.json', '/robots.txt'];",
            "const absolute = 'https://candidate.test/absolute/path';",
        ].join("\n");
        return makeResponse(bundle, "application/javascript");
    }

    return makeResponse("Not Found", "text/plain", 404);
};
"#,
    );
    code.push_str(include_str!("../runtime/agent/api_monitor.ts"));

    let metadata = PluginMetadata {
        id: "api_monitor_path_candidates".to_string(),
        name: "API Monitor Path Candidates".to_string(),
        version: "1.0.0".to_string(),
        author: Some("Test".to_string()),
        description: Some("Regression test for path candidate extraction and static filtering".to_string()),
        monitor_type: Some("api".to_string()),
        target_asset_types: vec![],
        main_category: "agent".to_string(),
        category: "monitor".to_string(),
        tags: vec![],
        default_severity: Severity::Info,
    };

    engine
        .load_plugin_with_metadata(&code, metadata)
        .await
        .expect("Failed to load api_monitor plugin");

    let input = serde_json::json!({
        "targets": ["candidate.test"],
        "timeout": 500,
        "concurrency": 4,
        "maxJsFiles": 5,
    });

    let (_findings, result) = engine
        .execute_agent(&input)
        .await
        .expect("Failed to execute api_monitor");

    let result = result.expect("api_monitor should return a result");
    assert_eq!(result["success"], true);

    let endpoints = result["data"]["results"][0]["snapshot"]["endpoints"]
        .as_array()
        .expect("snapshot endpoints should be an array");

    assert!(endpoints
        .iter()
        .any(|endpoint| endpoint["path"] == "/channel/list"));
    assert!(endpoints
        .iter()
        .any(|endpoint| endpoint["path"] == "/content/list"));
    assert!(endpoints
        .iter()
        .any(|endpoint| endpoint["path"] == "/api/users"));
    assert!(endpoints
        .iter()
        .any(|endpoint| endpoint["path"] == "/absolute/path"));
    assert!(!endpoints
        .iter()
        .any(|endpoint| endpoint["path"] == "/assets/logo.png"));
    assert!(!endpoints
        .iter()
        .any(|endpoint| endpoint["path"] == "/_next/static/chunk.js"));
    assert!(!endpoints
        .iter()
        .any(|endpoint| endpoint["path"] == "/manifest.json"));
    assert!(!endpoints
        .iter()
        .any(|endpoint| endpoint["path"] == "/robots.txt"));
}

#[cfg(feature = "plugin-ts-transpile")]
#[tokio::test]
async fn api_monitor_ignores_non_literal_concat_expression() {
    let mut engine = PluginEngine::new().expect("Failed to create plugin engine");

    let mut code = String::from(
        r#"
globalThis.fetch = async function(input, init) {
    const resolvedUrl = typeof input === "string"
        ? input
        : (input && typeof input.url === "string" ? input.url : String(input));

    function makeResponse(body, contentType, status = 200, finalUrl = resolvedUrl) {
        return {
            ok: status >= 200 && status < 300,
            status,
            url: finalUrl,
            headers: {
                get(name) {
                    const key = String(name || "").toLowerCase();
                    if (key === "content-type") return contentType;
                    if (key === "content-length") return String(body.length);
                    return null;
                }
            },
            async text() {
                return body;
            }
        };
    }

    if (resolvedUrl === "https://concat.test" || resolvedUrl === "https://concat.test/") {
        return makeResponse(
            '<!doctype html><html><body><script src="/assets/app.js"></script></body></html>',
            "text/html",
        );
    }

    if (resolvedUrl === "https://concat.test/assets/app.js") {
        const hugeTarget = Array.from({ length: 500 }, (_, index) => "'/x" + index + "'").join("+");
        const bundle = [
            "fetch(" + hugeTarget + ");",
            "axios.get('/api/steady');",
        ].join("\n");
        return makeResponse(bundle, "application/javascript");
    }

    if (resolvedUrl.endsWith("/api/steady")) {
        return makeResponse('{"ok":true}', "application/json");
    }

    return makeResponse("Not Found", "text/plain", 404);
};
"#,
    );
    code.push_str(include_str!("../runtime/agent/api_monitor.ts"));

    let metadata = PluginMetadata {
        id: "api_monitor_concat_guard".to_string(),
        name: "API Monitor Concat Guard".to_string(),
        version: "1.0.0".to_string(),
        author: Some("Test".to_string()),
        description: Some("Regression test for non-literal concatenated expressions".to_string()),
        monitor_type: Some("api".to_string()),
        target_asset_types: vec![],
        main_category: "agent".to_string(),
        category: "monitor".to_string(),
        tags: vec![],
        default_severity: Severity::Info,
    };

    engine
        .load_plugin_with_metadata(&code, metadata)
        .await
        .expect("Failed to load api_monitor plugin");

    let input = serde_json::json!({
        "targets": ["concat.test"],
        "timeout": 500,
        "concurrency": 4,
        "maxJsFiles": 5,
    });

    let (_findings, result) = engine
        .execute_agent(&input)
        .await
        .expect("Failed to execute api_monitor");

    let result = result.expect("api_monitor should return a result");
    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["summary"]["failedChecks"], 0);

    let target_result = &result["data"]["results"][0];
    assert_eq!(target_result["success"], true);

    let endpoints = target_result["snapshot"]["endpoints"]
        .as_array()
        .expect("snapshot endpoints should be an array");

    assert!(endpoints
        .iter()
        .any(|endpoint| endpoint["path"] == "/api/steady"));
}

#[cfg(feature = "plugin-ts-transpile")]
#[tokio::test]
async fn api_monitor_allows_non_2xx_html_shell_for_target_page() {
    let mut engine = PluginEngine::new().expect("Failed to create plugin engine");

    let mut code = String::from(
        r#"
globalThis.fetch = async function(input, init) {
    const resolvedUrl = typeof input === "string"
        ? input
        : (input && typeof input.url === "string" ? input.url : String(input));

    function makeResponse(body, contentType, status = 200, finalUrl = resolvedUrl) {
        return {
            ok: status >= 200 && status < 300,
            status,
            url: finalUrl,
            headers: {
                get(name) {
                    const key = String(name || "").toLowerCase();
                    if (key === "content-type") return contentType;
                    if (key === "content-length") return String(body.length);
                    return null;
                }
            },
            async text() {
                return body;
            }
        };
    }

    if (resolvedUrl === "https://status-shell.test" || resolvedUrl === "https://status-shell.test/") {
        return makeResponse(
            '<!doctype html><html><body><script src="/assets/app.js"></script></body></html>',
            "text/html",
            403,
        );
    }

    if (resolvedUrl === "https://status-shell.test/assets/app.js") {
        return makeResponse("axios.get('/api/health');", "application/javascript");
    }

    if (resolvedUrl.endsWith("/api/health")) {
        return makeResponse('{"ok":true}', "application/json");
    }

    return makeResponse("Not Found", "text/plain", 404);
};
"#,
    );
    code.push_str(include_str!("../runtime/agent/api_monitor.ts"));

    let metadata = PluginMetadata {
        id: "api_monitor_status_shell".to_string(),
        name: "API Monitor Status Shell".to_string(),
        version: "1.0.0".to_string(),
        author: Some("Test".to_string()),
        description: Some("Regression test for non-2xx HTML shell handling".to_string()),
        monitor_type: Some("api".to_string()),
        target_asset_types: vec![],
        main_category: "agent".to_string(),
        category: "monitor".to_string(),
        tags: vec![],
        default_severity: Severity::Info,
    };

    engine
        .load_plugin_with_metadata(&code, metadata)
        .await
        .expect("Failed to load api_monitor plugin");

    let input = serde_json::json!({
        "targets": ["status-shell.test"],
        "timeout": 500,
        "concurrency": 4,
        "maxJsFiles": 5,
    });

    let (_findings, result) = engine
        .execute_agent(&input)
        .await
        .expect("Failed to execute api_monitor");

    let result = result.expect("api_monitor should return a result");
    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["summary"]["failedChecks"], 0);
    assert_eq!(result["data"]["summary"]["successfulChecks"], 1);

    let target_result = &result["data"]["results"][0];
    assert_eq!(target_result["success"], true);
    let endpoints = target_result["snapshot"]["endpoints"]
        .as_array()
        .expect("snapshot endpoints should be an array");
    assert!(endpoints
        .iter()
        .any(|endpoint| endpoint["path"] == "/api/health"));
}

#[cfg(feature = "plugin-ts-transpile")]
#[tokio::test]
async fn api_monitor_does_not_fail_plain_404_target_page() {
    let mut engine = PluginEngine::new().expect("Failed to create plugin engine");

    let mut code = String::from(
        r#"
globalThis.fetch = async function(input, init) {
    const resolvedUrl = typeof input === "string"
        ? input
        : (input && typeof input.url === "string" ? input.url : String(input));

    function makeResponse(body, contentType, status = 200, finalUrl = resolvedUrl) {
        return {
            ok: status >= 200 && status < 300,
            status,
            url: finalUrl,
            headers: {
                get(name) {
                    const key = String(name || "").toLowerCase();
                    if (key === "content-type") return contentType;
                    if (key === "content-length") return String(body.length);
                    return null;
                }
            },
            async text() {
                return body;
            }
        };
    }

    if (resolvedUrl === "https://notfound.test" || resolvedUrl === "https://notfound.test/") {
        return makeResponse("<html><body>not found</body></html>", "text/html", 404);
    }

    return makeResponse("Not Found", "text/plain", 404);
};
"#,
    );
    code.push_str(include_str!("../runtime/agent/api_monitor.ts"));

    let metadata = PluginMetadata {
        id: "api_monitor_notfound_target".to_string(),
        name: "API Monitor 404 Target".to_string(),
        version: "1.0.0".to_string(),
        author: Some("Test".to_string()),
        description: Some("Regression test for plain 404 target handling".to_string()),
        monitor_type: Some("api".to_string()),
        target_asset_types: vec![],
        main_category: "agent".to_string(),
        category: "monitor".to_string(),
        tags: vec![],
        default_severity: Severity::Info,
    };

    engine
        .load_plugin_with_metadata(&code, metadata)
        .await
        .expect("Failed to load api_monitor plugin");

    let input = serde_json::json!({
        "targets": ["notfound.test"],
        "timeout": 500,
        "concurrency": 4,
        "maxJsFiles": 5,
    });

    let (_findings, result) = engine
        .execute_agent(&input)
        .await
        .expect("Failed to execute api_monitor");
    let result = result.expect("api_monitor should return a result");

    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["summary"]["failedChecks"], 0);
    assert_eq!(result["data"]["summary"]["successfulChecks"], 1);
    assert_eq!(result["data"]["results"][0]["success"], true);
}

#[cfg(feature = "plugin-ts-transpile")]
#[tokio::test]
async fn api_monitor_filters_common_api_path_fallback_pages() {
    let mut engine = PluginEngine::new().expect("Failed to create plugin engine");

    let mut code = String::from(
        r#"
globalThis.fetch = async function(input, init) {
    const resolvedUrl = typeof input === "string"
        ? input
        : (input && typeof input.url === "string" ? input.url : String(input));

    function makeResponse(body, contentType, status = 200, finalUrl = resolvedUrl) {
        return {
            ok: status >= 200 && status < 300,
            status,
            url: finalUrl,
            headers: {
                get(name) {
                    const key = String(name || "").toLowerCase();
                    if (key === "content-type") return contentType;
                    if (key === "content-length") return String(body.length);
                    return null;
                }
            },
            async text() {
                return body;
            }
        };
    }

    if (resolvedUrl === "https://fallback.test" || resolvedUrl === "https://fallback.test/") {
        return makeResponse("<!doctype html><html><body>home</body></html>", "text/html");
    }

    if (resolvedUrl.startsWith("https://fallback.test/")) {
        return makeResponse("<!doctype html><html><body>same fallback</body></html>", "text/html");
    }

    return makeResponse("Not Found", "text/plain", 404);
};
"#,
    );
    code.push_str(include_str!("../runtime/agent/api_monitor.ts"));

    let metadata = PluginMetadata {
        id: "api_monitor_probe_fallback".to_string(),
        name: "API Monitor Probe Fallback".to_string(),
        version: "1.0.0".to_string(),
        author: Some("Test".to_string()),
        description: Some("Regression test for common API path fallback filtering".to_string()),
        monitor_type: Some("api".to_string()),
        target_asset_types: vec![],
        main_category: "agent".to_string(),
        category: "monitor".to_string(),
        tags: vec![],
        default_severity: Severity::Info,
    };

    engine
        .load_plugin_with_metadata(&code, metadata)
        .await
        .expect("Failed to load api_monitor plugin");

    let input = serde_json::json!({
        "targets": ["fallback.test"],
        "timeout": 500,
        "concurrency": 4,
        "maxJsFiles": 5,
    });

    let (_findings, result) = engine
        .execute_agent(&input)
        .await
        .expect("Failed to execute api_monitor");
    let result = result.expect("api_monitor should return a result");

    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["summary"]["totalEndpoints"], 0);
    assert_eq!(
        result["data"]["results"][0]["snapshot"]["endpoints"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
}

#[test]
fn api_monitor_runtime_contract_markers_stay_in_sync() {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..");
    let source_plugin = repo_root.join("sentinel-plugin/plugins/bounty/api_monitor.ts");
    let runtime_plugin = repo_root.join("sentinel-ai/src-tauri/sentinel-plugins/runtime/agent/api_monitor.ts");

    let source = fs::read_to_string(&source_plugin).expect("failed to read source api_monitor.ts");
    let runtime = fs::read_to_string(&runtime_plugin).expect("failed to read runtime api_monitor.ts");

    assert!(
        !source.contains("includeOpenAPI") && !runtime.contains("includeOpenAPI"),
        "api_monitor contract should not expose includeOpenAPI",
    );
    assert!(
        !source.contains("openApiSpec") && !runtime.contains("openApiSpec"),
        "api_monitor output should not expose openApiSpec",
    );
    assert!(
        !source.contains("includeGraphQL") && !runtime.contains("includeGraphQL"),
        "api_monitor contract should not expose includeGraphQL",
    );
    assert!(
        !source.contains("graphqlEndpoint") && !runtime.contains("graphqlEndpoint"),
        "api_monitor output should not expose graphqlEndpoint",
    );
    assert!(
        source.contains("maxPages?: number;") && runtime.contains("maxPages?: number;"),
        "api_monitor source and runtime should both expose maxPages",
    );
    assert!(
        !source.contains("confidence?:") && !runtime.contains("confidence?:"),
        "api_monitor should not expose endpoint confidence",
    );
    assert!(
        !source.contains("detectionSource?:") && !runtime.contains("detectionSource?:"),
        "api_monitor should not expose endpoint detectionSource",
    );
    assert!(
        source.contains("interface ApiEndpoint {\n    path: string;\n    source: string;\n}")
            && runtime.contains("interface ApiEndpoint {\n    path: string;\n    source: string;\n}"),
        "api_monitor source and runtime should both keep the minimal endpoint shape",
    );
}
