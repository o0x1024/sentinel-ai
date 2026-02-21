//! Tool result digest utilities.

use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    pub created_at_ms: i64,
}

pub fn build_tool_digest(tool_name: &str, args: &Value, result: &str) -> ToolDigest {
    let created_at_ms = chrono::Utc::now().timestamp_millis();
    let status = match serde_json::from_str::<Value>(result) {
        Ok(Value::Object(map)) => {
            if let Some(success) = map.get("success").and_then(|v| v.as_bool()) {
                if success {
                    "ok".to_string()
                } else {
                    "error".to_string()
                }
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

    let summary = match serde_json::from_str::<Value>(result) {
        Ok(Value::Object(map)) => {
            if tool_name.contains("http") {
                let url = map.get("url").and_then(|v| v.as_str()).unwrap_or("unknown");
                let status_code = map.get("status_code").and_then(|v| v.as_i64()).unwrap_or(0);
                let status_text = map.get("status_text").and_then(|v| v.as_str()).unwrap_or("");
                let body_len = map.get("body_length").and_then(|v| v.as_i64()).unwrap_or(0);
                let truncated = map.get("truncated").and_then(|v| v.as_bool()).unwrap_or(false);
                format!(
                    "HTTP {} {} {} ({} bytes, truncated: {})",
                    status_code, status_text, url, body_len, truncated
                )
            } else if tool_name.contains("shell") || tool_name.contains("interactive_shell") {
                let command = map.get("command").and_then(|v| v.as_str()).unwrap_or("unknown");
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
            } else if tool_name.contains("todos") {
                let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("unknown");
                format!("Todos action: {}", action)
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

    ToolDigest {
        tool_name: tool_name.to_string(),
        status,
        summary,
        artifact_id,
        artifact_kind,
        preview_snippets,
        created_at_ms,
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
    let tail: String = trimmed.chars().rev().take(tail_len).collect::<Vec<_>>().into_iter().rev().collect();
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

    if let Some(stored) = obj.get("output_stored").and_then(|v| v.as_bool()) {
        if stored {
            let command = obj.get("command").and_then(|v| v.as_str()).unwrap_or("tool");
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
        .get("summary")
        .and_then(|v| v.as_str())
        .or_else(|| obj.get("stdout").and_then(|v| v.as_str()))
        .or_else(|| obj.get("stderr").and_then(|v| v.as_str()))
        .or_else(|| obj.get("output").and_then(|v| v.as_str()))?;
    Some(condense_text(preview, 120))
}

fn created_suffix(result: &str) -> String {
    let hash = result
        .bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(16777619).wrapping_add(b as u64));
    format!("{:016x}", hash)
}
