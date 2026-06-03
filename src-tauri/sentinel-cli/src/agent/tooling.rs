use crate::runtime::RuntimeStateStore;
use anyhow::Result;
use chrono::Utc;
use rig::tool::Tool;
use sentinel_tools::buildin_tools::http_request::{HttpRequestArgs, HttpRequestTool};
use sentinel_tools::buildin_tools::route_discovery::{RouteDiscoveryArgs, RouteDiscoveryTool};
use sentinel_tools::buildin_tools::search_exploit::{SearchExploitArgs, SearchExploitTool};
use sentinel_tools::buildin_tools::shell::{ShellArgs, ShellTool};
use sentinel_tools::buildin_tools::tenth_man_tool::{TenthManTool, TenthManToolArgs};
use sentinel_tools::buildin_tools::web_search::{WebSearchArgs, WebSearchTool};
use sentinel_tools::dynamic_tool::{
    DynamicTool, DynamicToolBuilder, ToolCategory, ToolExecutionPolicy, ToolSource,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContestToolDigest {
    pub status: String,
    pub tool_name: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContestToolWindowStats {
    pub total: usize,
    pub shell: usize,
    pub http_request: usize,
    pub route_discovery: usize,
    pub search_exploit: usize,
    pub tenth_man_review: usize,
    pub web_search: usize,
    pub trailing_shell: usize,
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

    pub async fn recent_tool_digests(&self, limit: usize) -> Vec<ContestToolDigest> {
        let records = self.records.lock().await;
        records
            .iter()
            .rev()
            .take(limit)
            .map(build_tool_digest)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    pub async fn recent_tool_window_stats(&self, limit: usize) -> ContestToolWindowStats {
        let records = self.records.lock().await;
        let slice = records.iter().rev().take(limit).collect::<Vec<_>>();
        let mut counts = BTreeMap::<&str, usize>::new();
        let mut trailing_shell = 0_usize;
        for (idx, record) in slice.iter().enumerate() {
            *counts.entry(record.name.as_str()).or_default() += 1;
            if idx == trailing_shell && record.name == "shell" {
                trailing_shell += 1;
            }
        }
        ContestToolWindowStats {
            total: slice.len(),
            shell: counts.get("shell").copied().unwrap_or(0),
            http_request: counts.get("http_request").copied().unwrap_or(0),
            route_discovery: counts.get("route_discovery").copied().unwrap_or(0),
            search_exploit: counts.get("search_exploit").copied().unwrap_or(0),
            tenth_man_review: counts.get("tenth_man_review").copied().unwrap_or(0),
            web_search: counts.get("web_search").copied().unwrap_or(0),
            trailing_shell,
        }
    }
}

pub async fn build_contest_dynamic_tools(
    recorder: ContestTraceRecorder,
) -> Result<Vec<DynamicTool>> {
    let sequence = Arc::new(AtomicU32::new(0));
    Ok(vec![
        build_http_tool(recorder.clone(), sequence.clone()).await?,
        build_route_discovery_tool(recorder.clone(), sequence.clone()).await?,
        build_shell_tool(recorder.clone(), sequence.clone()).await?,
        build_web_search_tool(recorder.clone(), sequence.clone()).await?,
        build_search_exploit_tool(recorder.clone(), sequence.clone()).await?,
        build_tenth_man_tool(recorder, sequence).await?,
    ])
}

fn build_signature(record: &ContestToolCallRecord) -> String {
    match record.name.as_str() {
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
        "route_discovery" => {
            let base_url = record
                .arguments
                .get("base_url")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            let path_count = record
                .arguments
                .get("paths")
                .and_then(|value| value.as_array())
                .map(|values| values.len())
                .unwrap_or_default();
            format!("route_discovery:{}:{}", base_url, path_count)
        }
        "web_search" => {
            let query = record
                .arguments
                .get("query")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            format!("web_search:{}", query)
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
        "route_discovery" => {
            let findings = result
                .get("findings")
                .and_then(|value| value.as_array())
                .map(|values| values.len())
                .unwrap_or_default();
            let filtered = result
                .get("filtered_as_wildcard")
                .and_then(|value| value.as_u64())
                .unwrap_or_default();
            Some(format!("route_discovery:{}:{}", findings, filtered))
        }
        "web_search" => {
            let total = result
                .get("total_results")
                .and_then(|value| value.as_u64())
                .unwrap_or_default();
            let source = result
                .get("source")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            Some(format!("web_search:{}:{}", source, total))
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

fn build_tool_digest(record: &ContestToolCallRecord) -> ContestToolDigest {
    let status = if record.success { "ok" } else { "error" }.to_string();
    let summary = build_result_signature(record).unwrap_or_else(|| build_signature(record));
    ContestToolDigest {
        status,
        tool_name: record.name.clone(),
        summary: normalize_preview(&summary.chars().take(200).collect::<String>()),
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
        .category(ToolCategory::Network)
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

async fn build_route_discovery_tool(
    recorder: ContestTraceRecorder,
    sequence: Arc<AtomicU32>,
) -> Result<DynamicTool> {
    let tool = RouteDiscoveryTool;
    let definition = tool.definition(String::new()).await;

    let def = DynamicToolBuilder::new(RouteDiscoveryTool::NAME.to_string())
        .description(definition.description)
        .input_schema(definition.parameters)
        .source(ToolSource::Builtin)
        .category(ToolCategory::Network)
        .execution_policy(ToolExecutionPolicy {
            read_only: true,
            mutating: false,
            concurrency_safe: true,
            requires_permission: false,
            supports_background: false,
        })
        .executor(move |args| {
            let tool = RouteDiscoveryTool;
            let recorder = recorder.clone();
            let sequence = sequence.clone();
            async move {
                let started_at = Utc::now().timestamp_millis();
                let started = Instant::now();
                let current_sequence = sequence.fetch_add(1, Ordering::Relaxed);
                let call_id = format!("route-discovery-{}-{}", started_at, current_sequence);
                let parsed_args: RouteDiscoveryArgs = serde_json::from_value(args.clone())
                    .map_err(|error| format!("invalid route_discovery args: {}", error))?;

                let call_result = tool.call(parsed_args).await;
                let completed_at = Utc::now().timestamp_millis();
                let duration_ms = started.elapsed().as_millis() as i64;
                let (success, result_value, response) = match call_result {
                    Ok(output) => {
                        let value = serde_json::to_value(&output).map_err(|error| {
                            format!("failed to serialize route_discovery output: {}", error)
                        })?;
                        (true, Some(value.clone()), Ok(value))
                    }
                    Err(error) => (
                        false,
                        None,
                        Err(format!("route_discovery failed: {}", error)),
                    ),
                };

                recorder
                    .append(&ContestToolCallRecord {
                        id: call_id,
                        name: RouteDiscoveryTool::NAME.to_string(),
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
        .category(ToolCategory::System)
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

async fn build_web_search_tool(
    recorder: ContestTraceRecorder,
    sequence: Arc<AtomicU32>,
) -> Result<DynamicTool> {
    let tool = WebSearchTool::default();
    let definition = tool.definition(String::new()).await;

    let def = DynamicToolBuilder::new(WebSearchTool::NAME.to_string())
        .description(definition.description)
        .input_schema(definition.parameters)
        .source(ToolSource::Builtin)
        .category(ToolCategory::AI)
        .execution_policy(ToolExecutionPolicy {
            read_only: true,
            mutating: false,
            concurrency_safe: true,
            requires_permission: false,
            supports_background: false,
        })
        .executor(move |args| {
            let tool = WebSearchTool::default();
            let recorder = recorder.clone();
            let sequence = sequence.clone();
            async move {
                let started_at = Utc::now().timestamp_millis();
                let started = Instant::now();
                let current_sequence = sequence.fetch_add(1, Ordering::Relaxed);
                let call_id = format!("web-search-{}-{}", started_at, current_sequence);
                let parsed_args: WebSearchArgs = serde_json::from_value(args.clone())
                    .map_err(|error| format!("invalid web_search args: {}", error))?;

                let call_result = tool.call(parsed_args).await;
                let completed_at = Utc::now().timestamp_millis();
                let duration_ms = started.elapsed().as_millis() as i64;
                let (success, result_value, response) = match call_result {
                    Ok(output) => {
                        let value = serde_json::to_value(&output).map_err(|error| {
                            format!("failed to serialize web_search output: {}", error)
                        })?;
                        (true, Some(value.clone()), Ok(value))
                    }
                    Err(error) => (false, None, Err(format!("web_search failed: {}", error))),
                };

                recorder
                    .append(&ContestToolCallRecord {
                        id: call_id,
                        name: WebSearchTool::NAME.to_string(),
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
        .category(ToolCategory::AI)
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
        .category(ToolCategory::Exploitation)
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
