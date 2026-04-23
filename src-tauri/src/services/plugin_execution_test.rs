use std::time::Instant;

use sentinel_plugins::{
    HttpTransaction, PluginExecutor, PluginMetadata, RequestContext, ResponseContext, Severity,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginExecutionFinding {
    pub title: String,
    pub description: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginExecutionTestResult {
    pub success: bool,
    pub message: Option<String>,
    pub output: Option<Value>,
    pub findings: Option<Vec<PluginExecutionFinding>>,
    pub execution_time_ms: u128,
    pub error: Option<String>,
}

pub async fn test_plugin_code(
    metadata: PluginMetadata,
    code: String,
    inputs: Option<Value>,
) -> Result<PluginExecutionTestResult, String> {
    let plugin_name = metadata.name.clone();
    let start = Instant::now();

    let result = tokio::task::spawn_blocking(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| format!("Failed to create runtime: {error}"))?;

        let local = tokio::task::LocalSet::new();
        local.block_on(&runtime, async move {
            let executor = PluginExecutor::new(metadata.clone(), code, 1000)
                .map_err(|error| format!("Failed to create plugin executor: {error}"))?;

            match metadata.main_category.as_str() {
                "traffic" => execute_traffic_plugin(executor).await,
                "agent" | "intruder" => {
                    let input = inputs.unwrap_or_else(|| default_execution_input(&metadata));
                    execute_execution_plugin(executor, input).await
                }
                other => Err(format!("Unsupported plugin main_category: {other}")),
            }
        })
    })
    .await
    .map_err(|error| format!("Task failed: {error}"))?;

    let execution_time_ms = start.elapsed().as_millis();

    match result {
        Ok((output, findings)) => Ok(PluginExecutionTestResult {
            success: true,
            message: Some(format!(
                "插件 '{plugin_name}' 执行成功 ({execution_time_ms}ms)"
            )),
            output,
            findings: findings.filter(|items| !items.is_empty()),
            execution_time_ms,
            error: None,
        }),
        Err(error) => Ok(PluginExecutionTestResult {
            success: false,
            message: Some(format!("插件执行失败: {error}")),
            output: None,
            findings: None,
            execution_time_ms,
            error: Some(error),
        }),
    }
}

async fn execute_traffic_plugin(
    executor: PluginExecutor,
) -> Result<(Option<Value>, Option<Vec<PluginExecutionFinding>>), String> {
    let transaction = sample_http_transaction();
    let findings = executor
        .scan_transaction(transaction)
        .await
        .map_err(|error| format!("Plugin execution failed: {error}"))?;

    let mapped = findings
        .into_iter()
        .map(|finding| PluginExecutionFinding {
            title: finding.title,
            description: finding.description,
            severity: finding.severity.to_string(),
        })
        .collect::<Vec<_>>();

    Ok((
        Some(json!({
            "findings": mapped,
            "findings_count": mapped.len(),
        })),
        Some(mapped),
    ))
}

async fn execute_execution_plugin(
    executor: PluginExecutor,
    input: Value,
) -> Result<(Option<Value>, Option<Vec<PluginExecutionFinding>>), String> {
    let (findings, result) = executor
        .execute_agent(&input)
        .await
        .map_err(|error| format!("Plugin execution failed: {error}"))?;

    let mapped = findings
        .into_iter()
        .map(|finding| PluginExecutionFinding {
            title: finding.title,
            description: finding.description,
            severity: format!("{:?}", finding.severity).to_lowercase(),
        })
        .collect::<Vec<_>>();

    let output = if let Some(result) = result {
        Some(result)
    } else if mapped.is_empty() {
        Some(json!({
            "message": "Plugin executed successfully with no output"
        }))
    } else {
        Some(json!({
            "findings": mapped,
            "findings_count": mapped.len(),
        }))
    };

    Ok((output, Some(mapped)))
}

fn sample_http_transaction() -> HttpTransaction {
    let request_id = uuid::Uuid::new_v4().to_string();

    let request = RequestContext {
        id: request_id.clone(),
        method: "GET".to_string(),
        url: "https://example.com/profile?id=1".to_string(),
        http_version: Some("HTTP/1.1".to_string()),
        headers: [
            ("Host".to_string(), "example.com".to_string()),
            (
                "User-Agent".to_string(),
                "Sentinel-Plugin-Test/1.0".to_string(),
            ),
            ("Accept".to_string(), "text/html".to_string()),
        ]
        .into_iter()
        .collect(),
        body: Vec::new(),
        content_type: Some("text/plain".to_string()),
        query_params: [("id".to_string(), "1".to_string())].into_iter().collect(),
        is_https: true,
        timestamp: chrono::Utc::now(),
        was_edited: false,
        edited_method: None,
        edited_url: None,
        edited_headers: None,
        edited_body: None,
    };

    let response = ResponseContext {
        request_id,
        status: 200,
        http_version: Some("HTTP/1.1".to_string()),
        headers: [
            (
                "Content-Type".to_string(),
                "text/html; charset=utf-8".to_string(),
            ),
            ("Server".to_string(), "example".to_string()),
            ("X-Powered-By".to_string(), "sentinel".to_string()),
        ]
        .into_iter()
        .collect(),
        body: br#"<html><body>example response body</body></html>"#.to_vec(),
        content_type: Some("text/html".to_string()),
        timestamp: chrono::Utc::now(),
        was_edited: false,
        edited_status: None,
        edited_headers: None,
        edited_body: None,
    };

    HttpTransaction {
        request,
        response: Some(response),
    }
}

fn default_execution_input(metadata: &PluginMetadata) -> Value {
    if metadata.main_category == "intruder" {
        return default_intruder_input(&metadata.category);
    }

    json!({
        "target": "https://example.com",
        "timeout": 5000,
        "rawRequest": "GET / HTTP/1.1\r\nHost: example.com\r\n\r\n",
    })
}

fn default_intruder_input(category: &str) -> Value {
    match category {
        "payload_generator" => json!({
            "config": {
                "values": ["' OR '1'='1", "<script>alert(1)</script>", "../../etc/passwd"],
                "limit": 3
            },
            "options": {
                "limit": 3
            }
        }),
        "payload_processor" => json!({
            "payload": "admin",
            "baseValue": "admin",
            "positionIndex": 0,
            "config": {
                "prefix": "pre-",
                "suffix": "-post",
                "skipIfContains": "skip-me"
            }
        }),
        "request_processor" => json!({
            "rawRequest": "GET /search?q=test HTTP/1.1\r\nHost: example.com\r\n\r\n",
            "payloadValues": ["test"],
            "payloadSummary": "single payload",
            "requestIndex": 0,
            "config": {
                "headerName": "X-Intruder-Request",
                "headerValue": "preview"
            }
        }),
        _ => json!({}),
    }
}

pub fn build_plugin_metadata(
    id: String,
    name: String,
    main_category: String,
    category: String,
    description: Option<String>,
    monitor_type: Option<String>,
    default_severity: Severity,
) -> PluginMetadata {
    PluginMetadata {
        id,
        name,
        version: "1.0.0".to_string(),
        author: Some("Sentinel Plugin Authoring".to_string()),
        main_category,
        category,
        monitor_type,
        default_severity,
        tags: vec!["ai-authored".to_string()],
        description,
        target_asset_types: Vec::new(),
    }
}

pub fn parse_plugin_severity(value: &str) -> Result<Severity, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "critical" => Ok(Severity::Critical),
        "high" => Ok(Severity::High),
        "medium" => Ok(Severity::Medium),
        "low" => Ok(Severity::Low),
        "info" => Ok(Severity::Info),
        other => Err(format!("Unsupported default_severity: {other}")),
    }
}
