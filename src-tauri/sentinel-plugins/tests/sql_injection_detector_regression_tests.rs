#![cfg(feature = "plugin-ts-transpile")]

use chrono::Utc;
use sentinel_plugins::{
    Confidence, HttpTransaction, PluginEngine, PluginMetadata, RequestContext, ResponseContext,
    Severity,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

const SQLI_PLUGIN_CODE: &str =
    include_str!("../../../../sentinel-plugin/plugins/traffic/sql_injection_detector.ts");

#[derive(Clone, Copy)]
enum Scenario {
    ConfirmedSqlError,
    PossibleBehavior,
    ValidationError,
    WafBlock,
    JsonArraySecondElementOnly,
    JsonSameFieldDifferentPaths,
}

struct TestServer {
    base_url: String,
    shutdown: Option<oneshot::Sender<()>>,
    handle: JoinHandle<()>,
}

impl TestServer {
    async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        let _ = self.handle.await;
    }
}

#[derive(Debug)]
struct ParsedRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: String,
}

#[derive(Clone)]
struct ResponseTemplate {
    status: u16,
    content_type: &'static str,
    body: String,
}

fn plugin_metadata() -> PluginMetadata {
    PluginMetadata {
        id: "sql-injection-detector-regression".to_string(),
        name: "SQL Injection Detector Regression".to_string(),
        version: "1.0.0".to_string(),
        author: None,
        main_category: "traffic".to_string(),
        category: "sqli".to_string(),
        default_severity: Severity::High,
        tags: vec!["sqli".to_string()],
        description: Some("Regression coverage for sql_injection_detector".to_string()),
        monitor_type: None,
        target_asset_types: vec![],
    }
}

fn create_engine() -> PluginEngine {
    PluginEngine::new().expect("engine")
}

fn create_transaction(
    method: &str,
    url: String,
    query_params: HashMap<String, String>,
    body: Vec<u8>,
    content_type: Option<&str>,
    response_status: u16,
    response_body: &str,
    response_content_type: Option<&str>,
) -> HttpTransaction {
    let request_id = uuid::Uuid::new_v4().to_string();
    HttpTransaction {
        request: RequestContext {
            id: request_id.clone(),
            method: method.to_string(),
            url,
            http_version: Some("HTTP/1.1".to_string()),
            headers: HashMap::new(),
            body,
            content_type: content_type.map(str::to_string),
            query_params,
            is_https: false,
            timestamp: Utc::now(),
            was_edited: false,
            edited_method: None,
            edited_url: None,
            edited_headers: None,
            edited_body: None,
        },
        response: Some(ResponseContext {
            request_id,
            status: response_status,
            http_version: Some("HTTP/1.1".to_string()),
            headers: HashMap::new(),
            body: response_body.as_bytes().to_vec(),
            content_type: response_content_type.map(str::to_string),
            timestamp: Utc::now(),
            was_edited: false,
            edited_status: None,
            edited_headers: None,
            edited_body: None,
        }),
    }
}

async fn read_http_request(stream: &mut TcpStream) -> Option<ParsedRequest> {
    let mut buffer = Vec::new();
    let mut temp = [0u8; 4096];
    let mut header_end = None;
    let mut content_length = 0usize;

    loop {
        let read = stream.read(&mut temp).await.ok()?;
        if read == 0 {
            return None;
        }
        buffer.extend_from_slice(&temp[..read]);

        if header_end.is_none() {
            header_end = buffer.windows(4).position(|window| window == b"\r\n\r\n");
            if let Some(index) = header_end {
                let headers_text = String::from_utf8_lossy(&buffer[..index + 4]);
                for line in headers_text.lines().skip(1) {
                    if let Some((name, value)) = line.split_once(':') {
                        if name.eq_ignore_ascii_case("content-length") {
                            content_length = value.trim().parse::<usize>().unwrap_or(0);
                        }
                    }
                }
            }
        }

        if let Some(index) = header_end {
            let body_start = index + 4;
            if buffer.len() >= body_start + content_length {
                let header_text = String::from_utf8_lossy(&buffer[..index]);
                let mut lines = header_text.lines();
                let request_line = lines.next()?;
                let mut request_line_parts = request_line.split_whitespace();
                let method = request_line_parts.next()?.to_string();
                let path = request_line_parts.next()?.to_string();
                let mut headers = HashMap::new();
                for line in lines {
                    if let Some((name, value)) = line.split_once(':') {
                        headers.insert(name.trim().to_lowercase(), value.trim().to_string());
                    }
                }
                let body =
                    String::from_utf8_lossy(&buffer[body_start..body_start + content_length])
                        .to_string();
                return Some(ParsedRequest {
                    method,
                    path,
                    headers,
                    body,
                });
            }
        }
    }
}

fn response_for_request(scenario: Scenario, request: &ParsedRequest) -> ResponseTemplate {
    let probe_header = request
        .headers
        .get("x-sentinel-active-probe")
        .map(String::as_str)
        .unwrap_or_default();

    match scenario {
        Scenario::ConfirmedSqlError => {
            if probe_header.starts_with("sql_injection_detector") && request.path.contains("%27") {
                return ResponseTemplate {
                    status: 500,
                    content_type: "text/html",
                    body: "<html><title>Error</title><body>You have an error in your SQL syntax; check the manual that corresponds to your MySQL server version</body></html>".to_string(),
                };
            }
            ResponseTemplate {
                status: 200,
                content_type: "text/html",
                body: "<html><title>Items</title><body>Item 1</body></html>".to_string(),
            }
        }
        Scenario::PossibleBehavior => {
            if probe_header.starts_with("sql_injection_detector") && request.path.contains("%27") {
                return ResponseTemplate {
                    status: 500,
                    content_type: "text/html",
                    body: "<html><title>Application Error</title><body>Unexpected application failure while loading record</body></html>".to_string(),
                };
            }
            ResponseTemplate {
                status: 200,
                content_type: "text/html",
                body: "<html><title>Catalog</title><body>Normal listing</body></html>".to_string(),
            }
        }
        Scenario::ValidationError => {
            if probe_header.starts_with("sql_injection_detector")
                && request.method == "POST"
                && request.body.contains("alice'")
            {
                return ResponseTemplate {
                    status: 422,
                    content_type: "application/json",
                    body: r#"{"error":"validation failed: invalid parameter"}"#.to_string(),
                };
            }
            ResponseTemplate {
                status: 200,
                content_type: "application/json",
                body: r#"{"ok":true,"name":"alice"}"#.to_string(),
            }
        }
        Scenario::WafBlock => {
            if probe_header.starts_with("sql_injection_detector") && request.path.contains("%27") {
                return ResponseTemplate {
                    status: 403,
                    content_type: "text/html",
                    body: "<html><title>Forbidden</title><body>Access denied by web application firewall</body></html>".to_string(),
                };
            }
            ResponseTemplate {
                status: 200,
                content_type: "text/html",
                body: "<html><title>Search</title><body>Result ok</body></html>".to_string(),
            }
        }
        Scenario::JsonArraySecondElementOnly => {
            if probe_header.starts_with("sql_injection_detector")
                && request.method == "POST"
                && request.body.contains(r#""second'"#)
            {
                return ResponseTemplate {
                    status: 500,
                    content_type: "application/json",
                    body: r#"{"error":"You have an error in your SQL syntax"}"#.to_string(),
                };
            }
            ResponseTemplate {
                status: 200,
                content_type: "application/json",
                body: r#"{"ok":true}"#.to_string(),
            }
        }
        Scenario::JsonSameFieldDifferentPaths => {
            if probe_header.starts_with("sql_injection_detector") && request.method == "POST" {
                let first_path_hit = request
                    .body
                    .contains(r#""primary":{"property_name":"utm_source'""#);
                let second_path_hit = request
                    .body
                    .contains(r#""secondary":{"property_name":"utm_source'""#);

                if first_path_hit || second_path_hit {
                    return ResponseTemplate {
                        status: 500,
                        content_type: "application/json",
                        body: r#"{"error":"DB::Exception"}"#.to_string(),
                    };
                }
            }
            ResponseTemplate {
                status: 200,
                content_type: "application/json",
                body: r#"{"ok":true}"#.to_string(),
            }
        }
    }
}

async fn handle_connection(mut stream: TcpStream, scenario: Scenario) {
    if let Some(request) = read_http_request(&mut stream).await {
        let response = response_for_request(scenario, &request);
        let response_text = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response.status,
            reason_phrase(response.status),
            response.content_type,
            response.body.len(),
            response.body
        );
        let _ = stream.write_all(response_text.as_bytes()).await;
    }
    let _ = stream.shutdown().await;
}

fn reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        403 => "Forbidden",
        422 => "Unprocessable Entity",
        500 => "Internal Server Error",
        _ => "OK",
    }
}

async fn spawn_test_server(scenario: Scenario) -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local addr");
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
    let listener = Arc::new(listener);
    let handle = tokio::spawn({
        let listener = listener.clone();
        async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => break,
                    accepted = listener.accept() => {
                        let Ok((stream, _)) = accepted else { break };
                        tokio::spawn(handle_connection(stream, scenario));
                    }
                }
            }
        }
    });

    TestServer {
        base_url: format!("http://{}", addr),
        shutdown: Some(shutdown_tx),
        handle,
    }
}

#[tokio::test]
async fn sql_injection_detector_confirms_explicit_sql_errors() {
    let server = spawn_test_server(Scenario::ConfirmedSqlError).await;
    let mut engine = create_engine();
    engine
        .load_plugin_with_metadata(SQLI_PLUGIN_CODE, plugin_metadata())
        .await
        .expect("load plugin");

    let mut query_params = HashMap::new();
    query_params.insert("id".to_string(), "1".to_string());
    let transaction = create_transaction(
        "GET",
        format!("{}/items?id=1", server.base_url),
        query_params,
        vec![],
        None,
        200,
        "<html><title>Items</title><body>Item 1</body></html>",
        Some("text/html"),
    );

    let findings = engine
        .scan_transaction(&transaction)
        .await
        .expect("scan ok");
    assert_eq!(findings.len(), 1);
    let finding = &findings[0];
    assert_eq!(finding.title, "Confirmed SQL Error Disclosure");
    assert_eq!(finding.severity, Severity::High);
    assert_eq!(finding.confidence, Confidence::High);
    assert!(finding.evidence.contains("sql_error="));
    assert!(finding.url.contains("%27"));
    assert_eq!(finding.method, "GET");
    assert!(finding
        .request_headers
        .as_deref()
        .unwrap_or_default()
        .contains("\"x-sentinel-active-probe\""));
    assert_eq!(finding.response_status, Some(500));
    assert!(finding
        .response_headers
        .as_deref()
        .unwrap_or_default()
        .contains("\"content-type\""));
    assert!(finding
        .response_body
        .as_deref()
        .unwrap_or_default()
        .contains("You have an error in your SQL syntax"));

    server.shutdown().await;
}

#[tokio::test]
async fn sql_injection_detector_discards_validation_failures() {
    let server = spawn_test_server(Scenario::ValidationError).await;
    let mut engine = create_engine();
    engine
        .load_plugin_with_metadata(SQLI_PLUGIN_CODE, plugin_metadata())
        .await
        .expect("load plugin");

    let body = br#"{"name":"alice"}"#.to_vec();
    let transaction = create_transaction(
        "POST",
        format!("{}/profile", server.base_url),
        HashMap::new(),
        body,
        Some("application/json"),
        200,
        r#"{"ok":true,"name":"alice"}"#,
        Some("application/json"),
    );

    let findings = engine
        .scan_transaction(&transaction)
        .await
        .expect("scan ok");
    assert!(findings.is_empty());

    server.shutdown().await;
}

#[tokio::test]
async fn sql_injection_detector_reports_possible_behavior_changes() {
    let server = spawn_test_server(Scenario::PossibleBehavior).await;
    let mut engine = create_engine();
    engine
        .load_plugin_with_metadata(SQLI_PLUGIN_CODE, plugin_metadata())
        .await
        .expect("load plugin");

    let mut query_params = HashMap::new();
    query_params.insert("id".to_string(), "7".to_string());
    let transaction = create_transaction(
        "GET",
        format!("{}/catalog?id=7", server.base_url),
        query_params,
        vec![],
        None,
        200,
        "<html><title>Catalog</title><body>Normal listing</body></html>",
        Some("text/html"),
    );

    let findings = engine
        .scan_transaction(&transaction)
        .await
        .expect("scan ok");
    assert_eq!(findings.len(), 1);
    let finding = &findings[0];
    assert_eq!(finding.title, "Possible SQL Injection Behavior");
    assert_eq!(finding.severity, Severity::Medium);
    assert_eq!(finding.confidence, Confidence::Medium);
    assert!(finding.evidence.contains("reference_status=200"));
    assert!(finding.evidence.contains("probe_status=500"));
    assert_eq!(finding.response_status, Some(500));
    assert!(finding
        .response_body
        .as_deref()
        .unwrap_or_default()
        .contains("Unexpected application failure"));

    server.shutdown().await;
}

#[tokio::test]
async fn sql_injection_detector_discards_waf_block_pages() {
    let server = spawn_test_server(Scenario::WafBlock).await;
    let mut engine = create_engine();
    engine
        .load_plugin_with_metadata(SQLI_PLUGIN_CODE, plugin_metadata())
        .await
        .expect("load plugin");

    let mut query_params = HashMap::new();
    query_params.insert("q".to_string(), "book".to_string());
    let transaction = create_transaction(
        "GET",
        format!("{}/search?q=book", server.base_url),
        query_params,
        vec![],
        None,
        200,
        "<html><title>Search</title><body>Result ok</body></html>",
        Some("text/html"),
    );

    let findings = engine
        .scan_transaction(&transaction)
        .await
        .expect("scan ok");
    assert!(findings.is_empty());

    server.shutdown().await;
}

#[tokio::test]
async fn sql_injection_detector_ignores_non_string_json_fields() {
    let server = spawn_test_server(Scenario::ConfirmedSqlError).await;
    let mut engine = create_engine();
    engine
        .load_plugin_with_metadata(SQLI_PLUGIN_CODE, plugin_metadata())
        .await
        .expect("load plugin");

    let body = br#"{"age":42,"enabled":true}"#.to_vec();
    let transaction = create_transaction(
        "POST",
        format!("{}/profile", server.base_url),
        HashMap::new(),
        body,
        Some("application/json"),
        200,
        r#"{"ok":true}"#,
        Some("application/json"),
    );

    let findings = engine
        .scan_transaction(&transaction)
        .await
        .expect("scan ok");
    assert!(findings.is_empty());

    server.shutdown().await;
}

#[tokio::test]
async fn sql_injection_detector_only_scans_first_json_array_element() {
    let server = spawn_test_server(Scenario::JsonArraySecondElementOnly).await;
    let mut engine = create_engine();
    engine
        .load_plugin_with_metadata(SQLI_PLUGIN_CODE, plugin_metadata())
        .await
        .expect("load plugin");

    let body = br#"{"items":["first","second"]}"#.to_vec();
    let transaction = create_transaction(
        "POST",
        format!("{}/profile", server.base_url),
        HashMap::new(),
        body,
        Some("application/json"),
        200,
        r#"{"ok":true}"#,
        Some("application/json"),
    );

    let findings = engine
        .scan_transaction(&transaction)
        .await
        .expect("scan ok");
    assert!(findings.is_empty());

    server.shutdown().await;
}

#[tokio::test]
async fn sql_injection_detector_distinguishes_same_field_name_across_json_paths() {
    let server = spawn_test_server(Scenario::JsonSameFieldDifferentPaths).await;
    let mut engine = create_engine();
    engine
        .load_plugin_with_metadata(SQLI_PLUGIN_CODE, plugin_metadata())
        .await
        .expect("load plugin");

    let body = br#"{
        "primary":{"property_name":"utm_source"},
        "secondary":{"property_name":"utm_source"}
    }"#
    .to_vec();
    let transaction = create_transaction(
        "POST",
        format!("{}/profile", server.base_url),
        HashMap::new(),
        body,
        Some("application/json"),
        200,
        r#"{"ok":true}"#,
        Some("application/json"),
    );

    let findings = engine
        .scan_transaction(&transaction)
        .await
        .expect("scan ok");

    assert_eq!(
        findings.len(),
        2,
        "expected two distinct findings for two JSON paths"
    );

    let mut locations = findings
        .iter()
        .map(|finding| finding.location.clone())
        .collect::<Vec<_>>();
    locations.sort();

    assert_eq!(
        locations,
        vec![
            "param:body:primary.property_name".to_string(),
            "param:body:secondary.property_name".to_string(),
        ]
    );

    server.shutdown().await;
}
