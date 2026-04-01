use sentinel_plugins::plugin_engine::PluginEngine;
use sentinel_plugins::types::{PluginMetadata, Severity};

#[cfg(feature = "plugin-ts-transpile")]
#[tokio::test]
async fn api_monitor_handles_large_bundle_with_chunked_fallback() {
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
    code.push_str(include_str!(
        "../../../../sentinel-plugin/plugins/agent/api_monitor.ts"
    ));

    let metadata = PluginMetadata {
        id: "api_monitor_regression".to_string(),
        name: "API Monitor Regression".to_string(),
        version: "1.0.0".to_string(),
        author: Some("Test".to_string()),
        description: Some("Regression test for large JS bundle fallback".to_string()),
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
        "includeGraphQL": false,
        "includeOpenAPI": false,
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
async fn api_monitor_ignores_unresolvable_huge_concat_expression() {
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
    code.push_str(include_str!(
        "../../../../sentinel-plugin/plugins/agent/api_monitor.ts"
    ));

    let metadata = PluginMetadata {
        id: "api_monitor_concat_guard".to_string(),
        name: "API Monitor Concat Guard".to_string(),
        version: "1.0.0".to_string(),
        author: Some("Test".to_string()),
        description: Some("Regression test for huge concatenated expressions".to_string()),
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
        "includeGraphQL": false,
        "includeOpenAPI": false,
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
    code.push_str(include_str!(
        "../../../../sentinel-plugin/plugins/agent/api_monitor.ts"
    ));

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
        "includeGraphQL": false,
        "includeOpenAPI": false,
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
    code.push_str(include_str!(
        "../../../../sentinel-plugin/plugins/agent/api_monitor.ts"
    ));

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
        "includeGraphQL": false,
        "includeOpenAPI": false,
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
    code.push_str(include_str!(
        "../../../../sentinel-plugin/plugins/agent/api_monitor.ts"
    ));

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
        "includeGraphQL": false,
        "includeOpenAPI": false,
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
