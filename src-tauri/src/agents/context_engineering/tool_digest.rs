//! Tool result digest utilities.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDigest {
    pub tool_name: String,
    pub status: String,
    pub summary: String,
    pub created_at_ms: i64,
}

pub fn build_tool_digest(tool_name: &str, args: &Value, result: &str) -> ToolDigest {
    let created_at_ms = chrono::Utc::now().timestamp_millis();
    let status = if result.to_lowercase().contains("error") {
        "error".to_string()
    } else {
        "ok".to_string()
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

    ToolDigest {
        tool_name: tool_name.to_string(),
        status,
        summary,
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

