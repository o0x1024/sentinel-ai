//! Tool result digest utilities.

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::LazyLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDigest {
    pub tool_name: String,
    pub status: String,
    pub summary: String,
    #[serde(default)]
    pub artifact_id: Option<String>,
    #[serde(default)]
    pub artifact_kind: Option<String>,
    #[serde(default)]
    pub preview_snippets: Vec<String>,
    #[serde(default)]
    pub metadata: Option<Value>,
    pub created_at_ms: i64,
}

pub fn build_tool_digest(tool_name: &str, args: &Value, result: &str) -> ToolDigest {
    let created_at_ms = chrono::Utc::now().timestamp_millis();
    let status = match serde_json::from_str::<Value>(result) {
        Ok(Value::Object(map)) => {
            if tool_name == "ask_user_question" {
                return ToolDigest {
                    tool_name: tool_name.to_string(),
                    status: match map
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("resolved")
                    {
                        "timeout_with_default" | "timeout_without_default" => "timeout".to_string(),
                        _ => "ok".to_string(),
                    },
                    summary: build_ask_user_question_summary(&map),
                    artifact_id: None,
                    artifact_kind: None,
                    preview_snippets: Vec::new(),
                    metadata: Some(json!({
                        "status": map.get("status").and_then(|v| v.as_str()).unwrap_or("resolved"),
                        "source": map.get("source").and_then(|v| v.as_str()).unwrap_or("user"),
                        "answer_count": map.get("answers").and_then(|v| v.as_object()).map(|answers| answers.len()).unwrap_or(0),
                        "question_count": map.get("questions").and_then(|v| v.as_array()).map(|questions| questions.len()).unwrap_or(0),
                    })),
                    created_at_ms,
                };
            }
            if let Some(success) = map.get("success").and_then(|v| v.as_bool()) {
                if success {
                    "ok".to_string()
                } else {
                    "error".to_string()
                }
            } else if map
                .get("backgrounded")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                "ok".to_string()
            } else if let Some(exit_code) = map.get("exit_code").and_then(|v| v.as_i64()) {
                if exit_code == 0 {
                    "ok".to_string()
                } else {
                    "error".to_string()
                }
            } else if let Some(status_code) = map.get("status_code").and_then(|v| v.as_i64()) {
                if (200..400).contains(&status_code) {
                    "ok".to_string()
                } else {
                    "error".to_string()
                }
            } else if result.to_lowercase().contains("error") {
                "error".to_string()
            } else {
                "ok".to_string()
            }
        }
        _ => {
            if result.to_lowercase().contains("error") {
                "error".to_string()
            } else {
                "ok".to_string()
            }
        }
    };

    let parsed_result = serde_json::from_str::<Value>(result).ok();

    let summary = match parsed_result.as_ref() {
        Some(Value::Object(map)) => {
            if tool_name.contains("http") {
                let url = map.get("url").and_then(|v| v.as_str()).unwrap_or("unknown");
                let status_code = map.get("status_code").and_then(|v| v.as_i64()).unwrap_or(0);
                let status_text = map
                    .get("status_text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let body_len = map.get("body_length").and_then(|v| v.as_i64()).unwrap_or(0);
                let truncated = map
                    .get("truncated")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                format!(
                    "HTTP {} {} {} ({} bytes, truncated: {})",
                    status_code, status_text, url, body_len, truncated
                )
            } else if tool_name.contains("shell") || tool_name.contains("interactive_shell") {
                let command = map
                    .get("command")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                if map
                    .get("backgrounded")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    let task_id = map
                        .get("background_task_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let session_id = map
                        .get("background_session_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let status = map
                        .get("background_status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("running");
                    format!(
                        "Shell `{}` -> background {} | task {} | session {}",
                        command, status, task_id, session_id
                    )
                } else {
                    let exit_code = map.get("exit_code").and_then(|v| v.as_i64()).unwrap_or(-1);
                    let stdout = map.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
                    let stderr = map.get("stderr").and_then(|v| v.as_str()).unwrap_or("");
                    let output = if !stdout.trim().is_empty() {
                        stdout
                    } else {
                        stderr
                    };
                    format!(
                        "Shell `{}` -> exit {} | {}",
                        command,
                        exit_code,
                        condense_text(output, 160)
                    )
                }
            } else if tool_name == "glob" {
                let pattern = map
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let num_files = map.get("num_files").and_then(|v| v.as_u64()).unwrap_or(0);
                let truncated = map
                    .get("truncated")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                format!(
                    "Glob `{}` -> {} files (truncated: {})",
                    pattern, num_files, truncated
                )
            } else if tool_name == "grep" {
                let pattern = map
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let num_matches = map.get("num_matches").and_then(|v| v.as_u64()).unwrap_or(0);
                let output_mode = map
                    .get("output_mode")
                    .and_then(|v| v.as_str())
                    .unwrap_or("content");
                format!(
                    "Grep `{}` -> {} matches ({})",
                    pattern, num_matches, output_mode
                )
            } else if tool_name == "file_read" {
                let file_path = map
                    .get("file_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let start_line = map.get("start_line").and_then(|v| v.as_u64()).unwrap_or(0);
                let end_line = map.get("end_line").and_then(|v| v.as_u64()).unwrap_or(0);
                let truncated = map
                    .get("truncated")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                format!(
                    "Read {}:{}-{} (truncated: {})",
                    file_path, start_line, end_line, truncated
                )
            } else if tool_name == "file_edit" {
                let file_path = map
                    .get("file_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let replacements = map
                    .get("replacements")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let after_preview = map
                    .get("after_preview")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let first_changed_line = map
                    .get("change_summary")
                    .and_then(|v| v.get("first_changed_line"))
                    .and_then(|v| v.as_u64());
                let changed_line_count = map
                    .get("change_summary")
                    .and_then(|v| v.get("changed_line_count"))
                    .and_then(|v| v.as_u64());
                let change_suffix = match (first_changed_line, changed_line_count) {
                    (Some(line), Some(count)) => format!(" | line {} ({} lines)", line, count),
                    (Some(line), None) => format!(" | line {}", line),
                    _ => String::new(),
                };
                format!(
                    "Edit {} -> {} replacements{}{}",
                    file_path,
                    replacements,
                    change_suffix,
                    if after_preview.is_empty() {
                        String::new()
                    } else {
                        format!(" | {}", condense_text(after_preview, 80))
                    }
                )
            } else if tool_name == "file_write" {
                let file_path = map
                    .get("file_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let operation = map
                    .get("operation")
                    .and_then(|v| v.as_str())
                    .unwrap_or("write");
                let content_preview = map
                    .get("content_preview")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let added_line_count = map
                    .get("change_summary")
                    .and_then(|v| v.get("added_line_count"))
                    .and_then(|v| v.as_u64());
                let removed_line_count = map
                    .get("change_summary")
                    .and_then(|v| v.get("removed_line_count"))
                    .and_then(|v| v.as_u64());
                format!(
                    "Write {} -> {}{}{}",
                    file_path,
                    operation,
                    match (added_line_count, removed_line_count) {
                        (Some(added), Some(removed)) => {
                            format!(" | +{} / -{} lines", added, removed)
                        }
                        _ => String::new(),
                    },
                    if content_preview.is_empty() {
                        String::new()
                    } else {
                        format!(" | {}", condense_text(content_preview, 80))
                    }
                )
            } else if tool_name == "lsp" {
                let action = map
                    .get("action")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let symbol_count = map
                    .get("symbols")
                    .and_then(|v| v.as_array())
                    .map(|items| items.len())
                    .unwrap_or(0);
                let reference_count = map
                    .get("references")
                    .and_then(|v| v.as_array())
                    .map(|items| items.len())
                    .unwrap_or(0);
                format!(
                    "LSP {} -> {} symbols, {} references",
                    action, symbol_count, reference_count
                )
            } else if tool_name == "tool_search" {
                let action = map
                    .get("action")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let match_count = map
                    .get("matches")
                    .and_then(|v| v.as_array())
                    .map(|items| items.len())
                    .unwrap_or(0);
                let activated_count = map
                    .get("activated_tool_ids")
                    .and_then(|v| v.as_array())
                    .map(|items| items.len())
                    .unwrap_or(0);
                let runtime_hint = map
                    .get("runtime_hint")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                format!(
                    "Tool search {} -> {} matches, {} activated{}",
                    action,
                    match_count,
                    activated_count,
                    if runtime_hint.is_empty() {
                        String::new()
                    } else {
                        format!(" | {}", condense_text(runtime_hint, 80))
                    }
                )
            } else if tool_name.contains("tasks") {
                let item_count = map
                    .get("list")
                    .and_then(|v| v.get("items"))
                    .and_then(|v| v.as_array())
                    .map(|items| items.len())
                    .or_else(|| {
                        args.get("plan")
                            .and_then(|v| v.as_array())
                            .map(|items| items.len())
                    })
                    .unwrap_or(0);
                let preview = map
                    .get("list")
                    .and_then(|v| v.get("items"))
                    .and_then(|v| v.as_array())
                    .and_then(|items| {
                        items.iter().find_map(|item| {
                            item.get("description")
                                .and_then(|v| v.as_str())
                                .map(|text| condense_text(text, 48))
                        })
                    })
                    .unwrap_or_default();

                format!(
                    "Plan updated: {}{}",
                    item_count,
                    if preview.is_empty() {
                        String::new()
                    } else {
                        format!(" | {}", preview)
                    }
                )
            } else {
                condense_text(result, 240)
            }
        }
        _ => condense_text(result, 240),
    };

    let (artifact_id, artifact_kind) = extract_artifact_reference(result);
    let mut preview_snippets = Vec::new();
    if let Some(preview) = extract_preview_snippet(result) {
        preview_snippets.push(preview);
    }
    let metadata = match parsed_result.as_ref() {
        Some(Value::Object(map)) => build_tool_metadata(tool_name, map),
        _ => None,
    };

    ToolDigest {
        tool_name: tool_name.to_string(),
        status,
        summary,
        artifact_id,
        artifact_kind,
        preview_snippets,
        metadata,
        created_at_ms,
    }
}

fn build_ask_user_question_summary(map: &serde_json::Map<String, Value>) -> String {
    let answer_count = map
        .get("answers")
        .and_then(|v| v.as_object())
        .map(|answers| answers.len())
        .unwrap_or(0);
    let question_count = map
        .get("questions")
        .and_then(|v| v.as_array())
        .map(|questions| questions.len())
        .unwrap_or(0);
    let response_status = map
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("resolved");
    let source = map.get("source").and_then(|v| v.as_str()).unwrap_or("user");
    match response_status {
        "timeout_with_default" => format!(
            "AskUserQuestion timed out and used defaults for {} / {} answers ({})",
            answer_count, question_count, source
        ),
        "timeout_without_default" => "AskUserQuestion timed out without answers".to_string(),
        _ => format!(
            "AskUserQuestion collected {} / {} answers ({})",
            answer_count, question_count, source
        ),
    }
}

pub fn condense_text(text: &str, max_len: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_len {
        return trimmed.to_string();
    }
    let head_len = max_len.saturating_sub(40).max(20);
    let tail_len = max_len.saturating_sub(head_len).min(20);
    let head: String = trimmed.chars().take(head_len).collect();
    let tail: String = trimmed
        .chars()
        .rev()
        .take(tail_len)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{}...<truncated>...{}", head, tail)
}

fn extract_artifact_reference(result: &str) -> (Option<String>, Option<String>) {
    let Ok(value) = serde_json::from_str::<Value>(result) else {
        return (None, None);
    };
    let Some(obj) = value.as_object() else {
        return (None, None);
    };

    if let Some(path) = obj
        .get("container_path")
        .and_then(|v| v.as_str())
        .or_else(|| obj.get("host_path").and_then(|v| v.as_str()))
        .or_else(|| obj.get("file_path").and_then(|v| v.as_str()))
        .or_else(|| obj.get("stored_path").and_then(|v| v.as_str()))
    {
        let kind = if path.starts_with('/') {
            "file".to_string()
        } else {
            "artifact".to_string()
        };
        return (Some(path.to_string()), Some(kind));
    }

    if let Some(path) = obj
        .get("stored_artifacts")
        .and_then(|value| value.as_array())
        .and_then(|items| items.first())
        .and_then(|item| item.get("path"))
        .and_then(|value| value.as_str())
    {
        return (Some(path.to_string()), Some("file".to_string()));
    }

    if let Some(stored) = obj.get("output_stored").and_then(|v| v.as_bool()) {
        if stored {
            let command = obj
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("tool");
            return (
                Some(format!("inline://{}:{}", command, created_suffix(result))),
                Some("inline".to_string()),
            );
        }
    }

    (None, None)
}

fn extract_preview_snippet(result: &str) -> Option<String> {
    let value = serde_json::from_str::<Value>(result).ok()?;
    let obj = value.as_object()?;
    let preview = obj
        .get("change_summary")
        .and_then(|v| v.get("after_preview"))
        .and_then(|v| v.as_str())
        .or_else(|| {
            obj.get("change_summary")
                .and_then(|v| v.get("before_preview"))
                .and_then(|v| v.as_str())
        })
        .or_else(|| obj.get("after_preview").and_then(|v| v.as_str()))
        .or_else(|| obj.get("content_preview").and_then(|v| v.as_str()))
        .or_else(|| obj.get("before_preview").and_then(|v| v.as_str()))
        .or_else(|| obj.get("previous_preview").and_then(|v| v.as_str()))
        .or_else(|| obj.get("summary").and_then(|v| v.as_str()))
        .or_else(|| obj.get("stdout").and_then(|v| v.as_str()))
        .or_else(|| obj.get("stderr").and_then(|v| v.as_str()))
        .or_else(|| obj.get("output").and_then(|v| v.as_str()))?;
    Some(condense_text(preview, 120))
}

fn build_tool_metadata(tool_name: &str, map: &serde_json::Map<String, Value>) -> Option<Value> {
    match tool_name {
        "file_edit" | "file_write" => build_file_change_metadata(map),
        "file_read" => build_file_read_metadata(map),
        "tool_search" => {
            let runtime_hint = map.get("runtime_hint").and_then(|value| value.as_str());
            let recommendation_reason = map
                .get("recommendation_reason")
                .and_then(|value| value.as_str());
            if runtime_hint.is_none() && recommendation_reason.is_none() {
                return None;
            }
            Some(json!({
                "runtime_hint": runtime_hint,
                "recommendation_reason": recommendation_reason,
            }))
        }
        _ if tool_name.contains("http") => build_http_metadata(map),
        _ if tool_name.contains("shell") || tool_name.contains("interactive_shell") => {
            build_shell_metadata(map)
        }
        _ => None,
    }
}

fn build_file_change_metadata(map: &serde_json::Map<String, Value>) -> Option<Value> {
    let change_summary = map.get("change_summary")?;
    let mut metadata = serde_json::Map::new();
    metadata.insert("change_summary".to_string(), change_summary.clone());

    if let Some(stored_artifacts) = map.get("stored_artifacts").cloned().filter(|value| {
        value
            .as_array()
            .map(|items| !items.is_empty())
            .unwrap_or(false)
    }) {
        metadata.insert("stored_artifacts".to_string(), stored_artifacts);
    }

    if let Some(content_hash) = map.get("content_hash").and_then(|value| value.as_str()) {
        metadata.insert("content_hash".to_string(), json!(content_hash));
    }

    Some(Value::Object(metadata))
}

fn build_file_read_metadata(map: &serde_json::Map<String, Value>) -> Option<Value> {
    let file_path = map.get("file_path").and_then(|value| value.as_str())?;
    let start_line = map
        .get("start_line")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let end_line = map
        .get("end_line")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let total_lines = map.get("total_lines").and_then(|value| value.as_u64());
    let truncated = map
        .get("truncated")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    if start_line == 0 || end_line == 0 {
        return None;
    }
    let mut metadata = serde_json::Map::new();
    metadata.insert(
        "artifact_read".to_string(),
        json!({
            "artifact_id": file_path,
            "start_line": start_line,
            "end_line": end_line,
            "total_lines": total_lines,
            "truncated": truncated,
            "reader": "file_read",
        }),
    );
    if let Some(content_hash) = map.get("content_hash").and_then(|value| value.as_str()) {
        metadata.insert("content_hash".to_string(), json!(content_hash));
    }
    Some(Value::Object(metadata))
}

fn build_http_metadata(map: &serde_json::Map<String, Value>) -> Option<Value> {
    let stored_artifacts = map.get("stored_artifacts").cloned().filter(|value| {
        value
            .as_array()
            .map(|items| !items.is_empty())
            .unwrap_or(false)
    });
    stored_artifacts.map(|stored_artifacts| {
        json!({
            "stored_artifacts": stored_artifacts,
        })
    })
}

fn build_shell_metadata(map: &serde_json::Map<String, Value>) -> Option<Value> {
    let mut metadata = serde_json::Map::new();
    if let Some(stored_artifacts) = map.get("stored_artifacts").cloned().filter(|value| {
        value
            .as_array()
            .map(|items| !items.is_empty())
            .unwrap_or(false)
    }) {
        metadata.insert("stored_artifacts".to_string(), stored_artifacts);
    }
    if let Some(artifact_read) = build_shell_artifact_read_metadata(map) {
        metadata.insert("artifact_read".to_string(), artifact_read);
    }
    if metadata.is_empty() {
        None
    } else {
        Some(Value::Object(metadata))
    }
}

fn build_shell_artifact_read_metadata(map: &serde_json::Map<String, Value>) -> Option<Value> {
    let command = map.get("command").and_then(|value| value.as_str())?;
    if map
        .get("output_stored")
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
        || shell_output_was_truncated(map)
    {
        return None;
    }

    let (start_line, end_line, path) = if let Some(captures) = SED_RANGE_READ_RE.captures(command) {
        (
            captures
                .name("start")
                .and_then(|value| value.as_str().parse::<u64>().ok())?,
            captures
                .name("end")
                .and_then(|value| value.as_str().parse::<u64>().ok())?,
            normalize_shell_path(captures.name("path")?.as_str()),
        )
    } else if let Some(captures) = JSON_TOOL_SED_RANGE_READ_RE.captures(command) {
        (
            captures
                .name("start")
                .and_then(|value| value.as_str().parse::<u64>().ok())?,
            captures
                .name("end")
                .and_then(|value| value.as_str().parse::<u64>().ok())?,
            normalize_shell_path(captures.name("path")?.as_str()),
        )
    } else if let Some(captures) = HEAD_RANGE_READ_RE.captures(command) {
        (
            1,
            captures
                .name("count")
                .and_then(|value| value.as_str().parse::<u64>().ok())?,
            normalize_shell_path(captures.name("path")?.as_str()),
        )
    } else if let Some(captures) = TAIL_HEAD_RANGE_READ_RE.captures(command) {
        let start_line = captures
            .name("start")
            .and_then(|value| value.as_str().parse::<u64>().ok())?;
        let line_count = captures
            .name("count")
            .and_then(|value| value.as_str().parse::<u64>().ok())?;
        let end_line = start_line
            .checked_add(line_count.saturating_sub(1))
            .filter(|end| *end >= start_line)?;
        (
            start_line,
            end_line,
            normalize_shell_path(captures.name("path")?.as_str()),
        )
    } else {
        return None;
    };

    Some(json!({
        "artifact_id": path,
        "start_line": start_line,
        "end_line": end_line,
        "reader": "shell_range",
    }))
}

fn shell_output_was_truncated(map: &serde_json::Map<String, Value>) -> bool {
    [map.get("stdout"), map.get("stderr")]
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str())
        .any(|text| text.contains("[Truncated:"))
}

fn normalize_shell_path(raw: &str) -> String {
    raw.trim_matches(|ch| ch == '"' || ch == '\'').to_string()
}

static SED_RANGE_READ_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?x)
        (?:^|\s)sed\s+-n\s+['"]?
        (?P<start>\d+)\s*,\s*(?P<end>\d+)p
        ['"]?\s+
        (?P<path>"[^"]+"|'[^']+'|\S+)
    "#,
    )
    .expect("valid sed range regex")
});

static HEAD_RANGE_READ_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?x)
        (?:^|\s)head\s+-n\s+
        (?P<count>\d+)\s+
        (?P<path>"[^"]+"|'[^']+'|\S+)
    "#,
    )
    .expect("valid head range regex")
});

static JSON_TOOL_SED_RANGE_READ_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?x)
        (?:^|\s)python(?:3)?\s+-m\s+json\.tool\s+
        (?P<path>"[^"]+"|'[^']+'|\S+)
        \s*\|\s*
        sed\s+-n\s+['"]?
        (?P<start>\d+)\s*,\s*(?P<end>\d+)p
        ['"]?
        (?:\s|$)
    "#,
    )
    .expect("valid json.tool sed range regex")
});

static TAIL_HEAD_RANGE_READ_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?x)
        (?:^|\s)tail\s+-n\s+\+
        (?P<start>\d+)\s+
        (?P<path>"[^"]+"|'[^']+'|\S+)
        \s*\|\s*
        head\s+-n\s+
        (?P<count>\d+)
        (?:\s|$)
    "#,
    )
    .expect("valid tail/head range regex")
});

fn created_suffix(result: &str) -> String {
    let hash = result.bytes().fold(0u64, |acc, b| {
        acc.wrapping_mul(16777619).wrapping_add(b as u64)
    });
    format!("{:016x}", hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_edit_digest_uses_change_summary_line_metadata() {
        let digest = build_tool_digest(
            "file_edit",
            &json!({}),
            r#"{
                "file_path":"src/main.rs",
                "replacements":1,
                "after_preview":"fn updated() {}",
                "content_hash":"abc123",
                "stored_artifacts":[{"slot":"file","path":"src/main.rs","storage_backend":"host","size":120,"lines":12}],
                "change_summary":{"first_changed_line":42,"changed_line_count":3,"after_preview":"fn updated() {}"}
            }"#,
        );

        assert!(digest.summary.contains("line 42"));
        assert!(digest.summary.contains("3 lines"));
        assert_eq!(digest.preview_snippets, vec!["fn updated() {}"]);
        assert_eq!(
            digest
                .metadata
                .as_ref()
                .and_then(|value| value.get("change_summary"))
                .and_then(|value| value.get("first_changed_line"))
                .and_then(|value| value.as_u64()),
            Some(42)
        );
        assert_eq!(
            digest
                .metadata
                .as_ref()
                .and_then(|value| value.get("content_hash"))
                .and_then(|value| value.as_str()),
            Some("abc123")
        );
        assert_eq!(
            digest
                .metadata
                .as_ref()
                .and_then(|value| value.get("stored_artifacts"))
                .and_then(|value| value.as_array())
                .map(|items| items.len()),
            Some(1)
        );
    }

    #[test]
    fn file_write_digest_prefers_change_summary_preview() {
        let digest = build_tool_digest(
            "file_write",
            &json!({}),
            r#"{
                "file_path":"src/new.rs",
                "operation":"create",
                "content_preview":"stale preview",
                "content_hash":"def456",
                "stored_artifacts":[{"slot":"file","path":"src/new.rs","storage_backend":"host","size":32,"lines":2}],
                "change_summary":{"added_line_count":2,"removed_line_count":0,"after_preview":"line one\nline two"}
            }"#,
        );

        assert!(digest.summary.contains("+2 / -0 lines"));
        assert_eq!(digest.preview_snippets, vec!["line one\nline two"]);
        assert_eq!(
            digest
                .metadata
                .as_ref()
                .and_then(|value| value.get("change_summary"))
                .and_then(|value| value.get("added_line_count"))
                .and_then(|value| value.as_u64()),
            Some(2)
        );
        assert_eq!(
            digest
                .metadata
                .as_ref()
                .and_then(|value| value.get("content_hash"))
                .and_then(|value| value.as_str()),
            Some("def456")
        );
        assert_eq!(digest.artifact_id.as_deref(), Some("src/new.rs"));
    }

    #[test]
    fn tool_search_digest_surfaces_runtime_hint_and_metadata() {
        let digest = build_tool_digest(
            "tool_search",
            &json!({}),
            r#"{
                "action":"search",
                "matches":[{"tool_id":"file_read"},{"tool_id":"file_write"}],
                "activated_tool_ids":[],
                "recommended_tool_ids":["file_read"],
                "recommendation_reason":"recommended bundle prioritizes reading back the result",
                "runtime_hint":"recent file changes triggered readback bias",
                "requires_reload":false,
                "message":"Found matching tools"
            }"#,
        );

        assert!(digest
            .summary
            .contains("Tool search search -> 2 matches, 0 activated"));
        assert!(digest
            .summary
            .contains("recent file changes triggered readback bias"));
        assert_eq!(
            digest
                .metadata
                .as_ref()
                .and_then(|value| value.get("runtime_hint"))
                .and_then(|value| value.as_str()),
            Some("recent file changes triggered readback bias")
        );
        assert_eq!(
            digest
                .metadata
                .as_ref()
                .and_then(|value| value.get("recommendation_reason"))
                .and_then(|value| value.as_str()),
            Some("recommended bundle prioritizes reading back the result")
        );
    }

    #[test]
    fn http_digest_captures_structured_stored_artifacts() {
        let digest = build_tool_digest(
            "http_request",
            &json!({}),
            r#"{
                "url":"https://example.com",
                "status_code":200,
                "status_text":"200 OK",
                "body":"[Large Output Stored to Host File]",
                "body_length":120,
                "truncated":true,
                "original_size":4096,
                "stored_artifacts":[{"slot":"body","path":"/tmp/http_response.txt","storage_backend":"host","size":4096,"lines":380}]
            }"#,
        );

        assert_eq!(
            digest.artifact_id.as_deref(),
            Some("/tmp/http_response.txt")
        );
        assert_eq!(
            digest
                .metadata
                .as_ref()
                .and_then(|value| value.get("stored_artifacts"))
                .and_then(|value| value.as_array())
                .map(|items| items.len()),
            Some(1)
        );
    }

    #[test]
    fn file_read_digest_exposes_artifact_read_metadata() {
        let digest = build_tool_digest(
            "file_read",
            &json!({}),
            r#"{
                "file_path":"/tmp/http_response.txt",
                "content":"a\nb\nc",
                "start_line":1,
                "end_line":200,
                "total_lines":380,
                "truncated":true,
                "content_hash":"read789"
            }"#,
        );

        assert_eq!(
            digest
                .metadata
                .as_ref()
                .and_then(|value| value.get("artifact_read"))
                .and_then(|value| value.get("artifact_id"))
                .and_then(|value| value.as_str()),
            Some("/tmp/http_response.txt")
        );
        assert_eq!(
            digest
                .metadata
                .as_ref()
                .and_then(|value| value.get("artifact_read"))
                .and_then(|value| value.get("end_line"))
                .and_then(|value| value.as_u64()),
            Some(200)
        );
        assert_eq!(
            digest
                .metadata
                .as_ref()
                .and_then(|value| value.get("content_hash"))
                .and_then(|value| value.as_str()),
            Some("read789")
        );
    }

    #[test]
    fn shell_digest_detects_sequential_range_reads() {
        let digest = build_tool_digest(
            "shell",
            &json!({"command":"sed -n '201,400p' /workspace/context/http_response.txt"}),
            r#"{
                "command":"sed -n '201,400p' /workspace/context/http_response.txt",
                "stdout":"line 201\nline 202",
                "stderr":"",
                "exit_code":0,
                "completed":true,
                "output_stored":false,
                "execution_mode":"docker"
            }"#,
        );

        assert_eq!(
            digest
                .metadata
                .as_ref()
                .and_then(|value| value.get("artifact_read"))
                .and_then(|value| value.get("artifact_id"))
                .and_then(|value| value.as_str()),
            Some("/workspace/context/http_response.txt")
        );
        assert_eq!(
            digest
                .metadata
                .as_ref()
                .and_then(|value| value.get("artifact_read"))
                .and_then(|value| value.get("start_line"))
                .and_then(|value| value.as_u64()),
            Some(201)
        );
    }

    #[test]
    fn shell_digest_detects_tail_head_chunk_reads() {
        let digest = build_tool_digest(
            "shell",
            &json!({"command":"tail -n +201 /workspace/context/http_response.txt | head -n 150"}),
            r#"{
                "command":"tail -n +201 /workspace/context/http_response.txt | head -n 150",
                "stdout":"line 201\nline 202",
                "stderr":"",
                "exit_code":0,
                "completed":true,
                "output_stored":false,
                "execution_mode":"docker"
            }"#,
        );

        assert_eq!(
            digest
                .metadata
                .as_ref()
                .and_then(|value| value.get("artifact_read"))
                .and_then(|value| value.get("artifact_id"))
                .and_then(|value| value.as_str()),
            Some("/workspace/context/http_response.txt")
        );
        assert_eq!(
            digest
                .metadata
                .as_ref()
                .and_then(|value| value.get("artifact_read"))
                .and_then(|value| value.get("start_line"))
                .and_then(|value| value.as_u64()),
            Some(201)
        );
        assert_eq!(
            digest
                .metadata
                .as_ref()
                .and_then(|value| value.get("artifact_read"))
                .and_then(|value| value.get("end_line"))
                .and_then(|value| value.as_u64()),
            Some(350)
        );
    }

    #[test]
    fn shell_digest_detects_json_tool_formatted_range_reads() {
        let digest = build_tool_digest(
            "shell",
            &json!({"command":"python3 -m json.tool /workspace/context/http_response.txt | sed -n '201,400p'"}),
            r#"{
                "command":"python3 -m json.tool /workspace/context/http_response.txt | sed -n '201,400p'",
                "stdout":"  \"openapi\": \"3.1.0\"",
                "stderr":"",
                "exit_code":0,
                "completed":true,
                "output_stored":false,
                "execution_mode":"docker"
            }"#,
        );

        assert_eq!(
            digest
                .metadata
                .as_ref()
                .and_then(|value| value.get("artifact_read"))
                .and_then(|value| value.get("artifact_id"))
                .and_then(|value| value.as_str()),
            Some("/workspace/context/http_response.txt")
        );
        assert_eq!(
            digest
                .metadata
                .as_ref()
                .and_then(|value| value.get("artifact_read"))
                .and_then(|value| value.get("start_line"))
                .and_then(|value| value.as_u64()),
            Some(201)
        );
        assert_eq!(
            digest
                .metadata
                .as_ref()
                .and_then(|value| value.get("artifact_read"))
                .and_then(|value| value.get("end_line"))
                .and_then(|value| value.as_u64()),
            Some(400)
        );
    }
}
