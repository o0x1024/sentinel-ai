use crate::runtime::RuntimeStateStore;
use anyhow::Result;
use chrono::Utc;
use rig::tool::Tool;
use sentinel_tools::buildin_tools::browser::{BrowserTool, BrowserToolArgs};
use sentinel_tools::buildin_tools::http_request::{HttpRequestArgs, HttpRequestTool};
use sentinel_tools::buildin_tools::search_exploit::{SearchExploitArgs, SearchExploitTool};
use sentinel_tools::buildin_tools::shell::{ShellArgs, ShellTool};
use sentinel_tools::buildin_tools::tenth_man_tool::{TenthManTool, TenthManToolArgs};
use sentinel_tools::dynamic_tool::{
    DynamicTool, DynamicToolBuilder, ToolExecutionPolicy, ToolSource,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContestToolCallRecord {
    pub id: String,
    pub name: String,
    pub arguments: Value,
    pub result: Option<Value>,
    pub success: bool,
    pub sequence: u32,
    pub started_at_ms: i64,
    pub completed_at_ms: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContestLatestCallSummary {
    pub tool_signature: String,
    pub result_signature: Option<String>,
    pub success: bool,
}

#[derive(Clone)]
pub struct ContestTraceRecorder {
    store: RuntimeStateStore,
    code: String,
    attempt_id: String,
    records: Arc<Mutex<Vec<ContestToolCallRecord>>>,
}

impl ContestTraceRecorder {
    pub fn new(
        store: RuntimeStateStore,
        code: impl Into<String>,
        attempt_id: impl Into<String>,
    ) -> Self {
        Self {
            store,
            code: code.into(),
            attempt_id: attempt_id.into(),
            records: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn append(&self, record: &ContestToolCallRecord) {
        self.records.lock().await.push(record.clone());
        let _ = self
            .store
            .append_tool_trace(&self.code, &self.attempt_id, record)
            .await;
    }

    pub async fn latest_call_summary(&self) -> Option<ContestLatestCallSummary> {
        let records = self.records.lock().await;
        records.last().map(|record| ContestLatestCallSummary {
            tool_signature: build_signature(record),
            result_signature: build_result_signature(record),
            success: record.success,
        })
    }

    pub fn default_browser_session_id(&self) -> String {
        format!("browser-{}-{}", self.code, self.attempt_id)
    }
}

pub async fn build_contest_dynamic_tools(
    recorder: ContestTraceRecorder,
) -> Result<Vec<DynamicTool>> {
    let sequence = Arc::new(AtomicU32::new(0));
    Ok(vec![
        build_browser_tool(recorder.clone(), sequence.clone()).await?,
        build_http_tool(recorder.clone(), sequence.clone()).await?,
        build_shell_tool(recorder.clone(), sequence.clone()).await?,
        build_search_exploit_tool(recorder.clone(), sequence.clone()).await?,
        build_tenth_man_tool(recorder, sequence).await?,
    ])
}

fn build_signature(record: &ContestToolCallRecord) -> String {
    match record.name.as_str() {
        "browser" => {
            let action = record
                .arguments
                .get("action")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown");
            let session_id = record
                .arguments
                .get("session_id")
                .and_then(|value| value.as_str())
                .unwrap_or("default");
            let url = record
                .arguments
                .get("url")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            let selector = record
                .arguments
                .get("selector")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            format!("browser:{}:{}:{}:{}", action, session_id, url, selector)
        }
        "http_request" => {
            let method = record
                .arguments
                .get("method")
                .and_then(|value| value.as_str())
                .unwrap_or("GET");
            let url = record
                .arguments
                .get("url")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            format!("http:{}:{}", method.to_ascii_uppercase(), url)
        }
        "shell" => {
            let command = record
                .arguments
                .get("command")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            format!("shell:{}", command)
        }
        "search_exploit" => {
            let action = record
                .arguments
                .get("action")
                .and_then(|value| value.as_str())
                .unwrap_or("search");
            match action {
                "get" => {
                    let edb_id = record
                        .arguments
                        .get("edb_id")
                        .and_then(|value| value.as_u64())
                        .unwrap_or_default();
                    format!("search_exploit:get:{}", edb_id)
                }
                _ => {
                    let query = record
                        .arguments
                        .get("query")
                        .and_then(|value| value.as_str())
                        .unwrap_or_default();
                    let product = record
                        .arguments
                        .get("product")
                        .and_then(|value| value.as_str())
                        .unwrap_or_default();
                    let cves = record
                        .arguments
                        .get("cves")
                        .and_then(|value| value.as_array())
                        .map(|values| {
                            values
                                .iter()
                                .filter_map(|value| value.as_str())
                                .collect::<Vec<_>>()
                                .join(",")
                        })
                        .unwrap_or_default();
                    format!("search_exploit:search:{}:{}:{}", query, product, cves)
                }
            }
        }
        _ => record.name.clone(),
    }
}

fn build_result_signature(record: &ContestToolCallRecord) -> Option<String> {
    let result = record.result.as_ref()?;
    match record.name.as_str() {
        "browser" => {
            let action = result
                .get("action")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown");
            let session_id = result
                .get("session_id")
                .and_then(|value| value.as_str())
                .unwrap_or("default");
            let data_preview = result
                .get("data")
                .and_then(|value| serde_json::to_string(value).ok())
                .unwrap_or_default();
            Some(format!(
                "browser:{}:{}:{}",
                action,
                session_id,
                normalize_preview(&data_preview.chars().take(200).collect::<String>())
            ))
        }
        "http_request" => {
            let status = result.get("status_code").and_then(|value| value.as_u64())?;
            let url = result
                .get("url")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            let body = result
                .get("body")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            let body_preview = body.chars().take(160).collect::<String>();
            Some(format!(
                "http:{}:{}:{}",
                status,
                url,
                normalize_preview(&body_preview)
            ))
        }
        "shell" => {
            let exit_code = result
                .get("exit_code")
                .and_then(|value| value.as_i64())
                .unwrap_or(-999);
            let stdout = result
                .get("stdout")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            let stderr = result
                .get("stderr")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            let preview = format!(
                "{}|{}",
                stdout.chars().take(120).collect::<String>(),
                stderr.chars().take(120).collect::<String>()
            );
            Some(format!(
                "shell:{}:{}",
                exit_code,
                normalize_preview(&preview)
            ))
        }
        "search_exploit" => {
            let action = result
                .get("action")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown");
            let repo_ready = result
                .get("search")
                .and_then(|value| value.get("repo_ready"))
                .or_else(|| {
                    result
                        .get("detail")
                        .and_then(|value| value.get("repo_ready"))
                })
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            let result_count = result
                .get("search")
                .and_then(|value| value.get("results"))
                .and_then(|value| value.as_array())
                .map(|values| values.len())
                .unwrap_or_else(|| {
                    if result
                        .get("detail")
                        .and_then(|value| value.get("exploit"))
                        .is_some()
                    {
                        1
                    } else {
                        0
                    }
                });
            Some(format!(
                "search_exploit:{}:{}:{}",
                action, repo_ready, result_count
            ))
        }
        _ => {
            let raw = serde_json::to_string(result).ok()?;
            Some(normalize_preview(
                &raw.chars().take(200).collect::<String>(),
            ))
        }
    }
}

fn normalize_preview(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(180)
        .collect()
}

async fn build_browser_tool(
    recorder: ContestTraceRecorder,
    sequence: Arc<AtomicU32>,
) -> Result<DynamicTool> {
    let tool = BrowserTool::default();
    let definition = tool.definition(String::new()).await;

    let def = DynamicToolBuilder::new(BrowserTool::NAME.to_string())
        .description(definition.description)
        .input_schema(definition.parameters)
        .source(ToolSource::Builtin)
        .category("browser")
        .execution_policy(ToolExecutionPolicy {
            read_only: false,
            mutating: true,
            concurrency_safe: false,
            requires_permission: false,
            supports_background: false,
        })
        .executor(move |args| {
            let tool = BrowserTool::default();
            let recorder = recorder.clone();
            let sequence = sequence.clone();
            async move {
                let started_at = Utc::now().timestamp_millis();
                let started = Instant::now();
                let current_sequence = sequence.fetch_add(1, Ordering::Relaxed);
                let call_id = format!("browser-{}-{}", started_at, current_sequence);
                let mut parsed_args: BrowserToolArgs = serde_json::from_value(args.clone())
                    .map_err(|error| format!("invalid browser args: {}", error))?;
                if parsed_args
                    .session_id
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .is_empty()
                {
                    parsed_args.session_id = Some(recorder.default_browser_session_id());
                }

                let call_result = tool.call(parsed_args).await;
                let completed_at = Utc::now().timestamp_millis();
                let duration_ms = started.elapsed().as_millis() as i64;
                let (success, result_value, response) = match call_result {
                    Ok(output) => {
                        let value = serde_json::to_value(&output).map_err(|error| {
                            format!("failed to serialize browser output: {}", error)
                        })?;
                        (true, Some(value.clone()), Ok(value))
                    }
                    Err(error) => (false, None, Err(format!("browser failed: {}", error))),
                };

                recorder
                    .append(&ContestToolCallRecord {
                        id: call_id,
                        name: BrowserTool::NAME.to_string(),
                        arguments: args,
                        result: result_value,
                        success,
                        sequence: current_sequence,
                        started_at_ms: started_at,
                        completed_at_ms: completed_at,
                        duration_ms,
                    })
                    .await;

                response
            }
        })
        .build()
        .map_err(anyhow::Error::msg)?;

    Ok(DynamicTool::new(def))
}

async fn build_http_tool(
    recorder: ContestTraceRecorder,
    sequence: Arc<AtomicU32>,
) -> Result<DynamicTool> {
    let tool = HttpRequestTool::default();
    let definition = tool.definition(String::new()).await;

    let def = DynamicToolBuilder::new(HttpRequestTool::NAME.to_string())
        .description(definition.description)
        .input_schema(definition.parameters)
        .source(ToolSource::Builtin)
        .category("network")
        .execution_policy(ToolExecutionPolicy {
            read_only: true,
            mutating: false,
            concurrency_safe: true,
            requires_permission: false,
            supports_background: false,
        })
        .executor(move |args| {
            let tool = HttpRequestTool::default();
            let recorder = recorder.clone();
            let sequence = sequence.clone();
            async move {
                let started_at = Utc::now().timestamp_millis();
                let started = Instant::now();
                let current_sequence = sequence.fetch_add(1, Ordering::Relaxed);
                let call_id = format!("http-{}-{}", started_at, current_sequence);
                let parsed_args: HttpRequestArgs = serde_json::from_value(args.clone())
                    .map_err(|error| format!("invalid http_request args: {}", error))?;

                let call_result = tool.call(parsed_args).await;
                let completed_at = Utc::now().timestamp_millis();
                let duration_ms = started.elapsed().as_millis() as i64;
                let (success, result_value, response) = match call_result {
                    Ok(output) => {
                        let value = serde_json::to_value(&output).map_err(|error| {
                            format!("failed to serialize http output: {}", error)
                        })?;
                        (true, Some(value.clone()), Ok(value))
                    }
                    Err(error) => (false, None, Err(format!("http_request failed: {}", error))),
                };

                recorder
                    .append(&ContestToolCallRecord {
                        id: call_id,
                        name: HttpRequestTool::NAME.to_string(),
                        arguments: args,
                        result: result_value,
                        success,
                        sequence: current_sequence,
                        started_at_ms: started_at,
                        completed_at_ms: completed_at,
                        duration_ms,
                    })
                    .await;

                response
            }
        })
        .build()
        .map_err(anyhow::Error::msg)?;

    Ok(DynamicTool::new(def))
}

async fn build_shell_tool(
    recorder: ContestTraceRecorder,
    sequence: Arc<AtomicU32>,
) -> Result<DynamicTool> {
    let tool = ShellTool::new();
    let definition = tool.definition(String::new()).await;

    let def = DynamicToolBuilder::new(ShellTool::NAME.to_string())
        .description(definition.description)
        .input_schema(definition.parameters)
        .source(ToolSource::Builtin)
        .category("system")
        .execution_policy(ToolExecutionPolicy {
            read_only: false,
            mutating: true,
            concurrency_safe: false,
            requires_permission: false,
            supports_background: true,
        })
        .executor(move |args| {
            let tool = ShellTool::new();
            let recorder = recorder.clone();
            let sequence = sequence.clone();
            async move {
                let started_at = Utc::now().timestamp_millis();
                let started = Instant::now();
                let current_sequence = sequence.fetch_add(1, Ordering::Relaxed);
                let call_id = format!("shell-{}-{}", started_at, current_sequence);
                let parsed_args: ShellArgs = serde_json::from_value(args.clone())
                    .map_err(|error| format!("invalid shell args: {}", error))?;

                let call_result = tool.call(parsed_args).await;
                let completed_at = Utc::now().timestamp_millis();
                let duration_ms = started.elapsed().as_millis() as i64;
                let (success, result_value, response) = match call_result {
                    Ok(output) => {
                        let value = serde_json::to_value(&output).map_err(|error| {
                            format!("failed to serialize shell output: {}", error)
                        })?;
                        (true, Some(value.clone()), Ok(value))
                    }
                    Err(error) => (false, None, Err(format!("shell failed: {}", error))),
                };

                recorder
                    .append(&ContestToolCallRecord {
                        id: call_id,
                        name: ShellTool::NAME.to_string(),
                        arguments: args,
                        result: result_value,
                        success,
                        sequence: current_sequence,
                        started_at_ms: started_at,
                        completed_at_ms: completed_at,
                        duration_ms,
                    })
                    .await;

                response
            }
        })
        .build()
        .map_err(anyhow::Error::msg)?;

    Ok(DynamicTool::new(def))
}

async fn build_tenth_man_tool(
    recorder: ContestTraceRecorder,
    sequence: Arc<AtomicU32>,
) -> Result<DynamicTool> {
    let tool = TenthManTool::new();
    let definition = tool.definition(String::new()).await;

    let def = DynamicToolBuilder::new(TenthManTool::NAME.to_string())
        .description(definition.description)
        .input_schema(definition.parameters)
        .source(ToolSource::Builtin)
        .category("reasoning")
        .execution_policy(ToolExecutionPolicy {
            read_only: true,
            mutating: false,
            concurrency_safe: false,
            requires_permission: false,
            supports_background: false,
        })
        .executor(move |args| {
            let tool = TenthManTool::new();
            let recorder = recorder.clone();
            let sequence = sequence.clone();
            async move {
                let started_at = Utc::now().timestamp_millis();
                let started = Instant::now();
                let current_sequence = sequence.fetch_add(1, Ordering::Relaxed);
                let call_id = format!("tenth-man-{}-{}", started_at, current_sequence);
                let parsed_args: TenthManToolArgs = serde_json::from_value(args.clone())
                    .map_err(|error| format!("invalid tenth_man_review args: {}", error))?;

                let call_result = tool.call(parsed_args).await;
                let completed_at = Utc::now().timestamp_millis();
                let duration_ms = started.elapsed().as_millis() as i64;
                let (success, result_value, response) = match call_result {
                    Ok(output) => {
                        let value = serde_json::to_value(&output).map_err(|error| {
                            format!("failed to serialize tenth_man_review output: {}", error)
                        })?;
                        (true, Some(value.clone()), Ok(value))
                    }
                    Err(error) => (
                        false,
                        None,
                        Err(format!("tenth_man_review failed: {}", error)),
                    ),
                };

                recorder
                    .append(&ContestToolCallRecord {
                        id: call_id,
                        name: TenthManTool::NAME.to_string(),
                        arguments: args,
                        result: result_value,
                        success,
                        sequence: current_sequence,
                        started_at_ms: started_at,
                        completed_at_ms: completed_at,
                        duration_ms,
                    })
                    .await;

                response
            }
        })
        .build()
        .map_err(anyhow::Error::msg)?;

    Ok(DynamicTool::new(def))
}

async fn build_search_exploit_tool(
    recorder: ContestTraceRecorder,
    sequence: Arc<AtomicU32>,
) -> Result<DynamicTool> {
    let tool = SearchExploitTool;
    let definition = tool.definition(String::new()).await;

    let def = DynamicToolBuilder::new(SearchExploitTool::NAME.to_string())
        .description(definition.description)
        .input_schema(definition.parameters)
        .source(ToolSource::Builtin)
        .category("exploitation")
        .execution_policy(ToolExecutionPolicy {
            read_only: false,
            mutating: true,
            concurrency_safe: false,
            requires_permission: false,
            supports_background: false,
        })
        .executor(move |args| {
            let tool = SearchExploitTool;
            let recorder = recorder.clone();
            let sequence = sequence.clone();
            async move {
                let started_at = Utc::now().timestamp_millis();
                let started = Instant::now();
                let current_sequence = sequence.fetch_add(1, Ordering::Relaxed);
                let call_id = format!("search-exploit-{}-{}", started_at, current_sequence);
                let parsed_args: SearchExploitArgs = serde_json::from_value(args.clone())
                    .map_err(|error| format!("invalid search_exploit args: {}", error))?;

                crate::exploitdb::ensure_ready_for_tool_use()
                    .await
                    .map_err(|error| format!("search_exploit bootstrap failed: {}", error))?;

                let call_result = tool.call(parsed_args).await;
                let completed_at = Utc::now().timestamp_millis();
                let duration_ms = started.elapsed().as_millis() as i64;
                let (success, result_value, response) = match call_result {
                    Ok(output) => {
                        let value = serde_json::to_value(&output).map_err(|error| {
                            format!("failed to serialize search_exploit output: {}", error)
                        })?;
                        (true, Some(value.clone()), Ok(value))
                    }
                    Err(error) => (
                        false,
                        None,
                        Err(format!("search_exploit failed: {}", error)),
                    ),
                };

                recorder
                    .append(&ContestToolCallRecord {
                        id: call_id,
                        name: SearchExploitTool::NAME.to_string(),
                        arguments: args,
                        result: result_value,
                        success,
                        sequence: current_sequence,
                        started_at_ms: started_at,
                        completed_at_ms: completed_at,
                        duration_ms,
                    })
                    .await;

                response
            }
        })
        .build()
        .map_err(anyhow::Error::msg)?;

    Ok(DynamicTool::new(def))
}
