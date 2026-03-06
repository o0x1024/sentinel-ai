//! LLM 请求/响应日志记录模块

use std::fs::OpenOptions;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};

const TOOL_LOG_MAX_CHARS: usize = 8000;
const LLM_REQUEST_LOG_MAX_CHARS: usize = 12000;
const LLM_RESPONSE_LOG_MAX_CHARS: usize = 12000;
const LLM_ERROR_LOG_MAX_CHARS: usize = 6000;
const LLM_JSONL_PREVIEW_MAX_CHARS: usize = 800;
const STREAM_EVENT_LOG_MAX_CHARS: usize = 8000;
const TURN_LOG_MAX_CHARS: usize = 12000;
static LLM_TURN_COUNTER: AtomicU64 = AtomicU64::new(1);
static LLM_STREAM_EVENT_COUNTER: AtomicU64 = AtomicU64::new(1);

fn truncate_utf8_at_boundary(input: &str, max_bytes: usize) -> String {
    if input.len() <= max_bytes {
        return input.to_string();
    }

    let mut safe_len = max_bytes;
    while safe_len > 0 && !input.is_char_boundary(safe_len) {
        safe_len -= 1;
    }

    input[..safe_len].to_string()
}

fn truncate_with_marker(input: &str, max_bytes: usize) -> String {
    let mut trimmed = truncate_utf8_at_boundary(input, max_bytes);
    if trimmed.len() < input.len() {
        trimmed.push_str("\n...[truncated]");
    }
    trimmed
}

fn content_hash_u64(input: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}

fn truncate_json_value_strings(value: &serde_json::Value, max_bytes: usize) -> serde_json::Value {
    match value {
        serde_json::Value::String(s) => {
            serde_json::Value::String(truncate_with_marker(s, max_bytes))
        }
        serde_json::Value::Array(items) => serde_json::Value::Array(
            items
                .iter()
                .map(|item| truncate_json_value_strings(item, max_bytes))
                .collect(),
        ),
        serde_json::Value::Object(map) => serde_json::Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), truncate_json_value_strings(v, max_bytes)))
                .collect(),
        ),
        other => other.clone(),
    }
}

fn write_llm_jsonl_log(
    session_id: &str,
    conversation_id: Option<&str>,
    provider: &str,
    model: &str,
    log_type: &str,
    content: &str,
) {
    let timestamp = chrono::Utc::now();
    let normalized_content = content.trim();
    let preview = truncate_with_marker(normalized_content, LLM_JSONL_PREVIEW_MAX_CHARS);
    let event = serde_json::json!({
        "timestamp": timestamp.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        "log_type": log_type,
        "session_id": session_id,
        "conversation_id": conversation_id.unwrap_or("N/A"),
        "provider": provider,
        "model": model,
        "content_length": normalized_content.len(),
        "content_hash_u64": content_hash_u64(normalized_content),
        "content_preview": preview,
        "truncated": normalized_content.len() > LLM_JSONL_PREVIEW_MAX_CHARS || normalized_content.contains("[truncated]"),
    });

    let jsonl_file_path = format!(
        "logs/llm-http-requests-{}.jsonl",
        chrono::Utc::now().format("%Y-%m-%d")
    );
    match OpenOptions::new()
        .create(true)
        .append(true)
        .open(&jsonl_file_path)
    {
        Ok(mut file) => {
            if let Err(e) = writeln!(file, "{}", event) {
                tracing::error!(
                    "Failed to write to LLM JSONL log file {}: {}",
                    jsonl_file_path,
                    e
                );
            } else {
                let _ = file.flush();
            }
        }
        Err(e) => {
            tracing::error!(
                "Failed to open LLM JSONL log file {}: {}",
                jsonl_file_path,
                e
            );
        }
    }
}

/// Build stable session id using conversation id + turn id.
/// This helps correlate retries and turns in a single execution.
pub fn build_log_session_id(conversation_id: Option<&str>) -> String {
    let turn_id = LLM_TURN_COUNTER.fetch_add(1, Ordering::Relaxed);
    let conv = conversation_id
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("session");
    format!("{}::turn-{}", conv, turn_id)
}

/// 写入 LLM 日志
pub fn write_llm_log(
    session_id: &str,
    conversation_id: Option<&str>,
    provider: &str,
    model: &str,
    log_type: &str,
    content: &str,
) {
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f UTC");
    let log_entry = format!(
        "[{}] [{}] [Session: {}] [Conversation: {}] [Provider: {}] [Model: {}] {}\n",
        timestamp,
        log_type,
        session_id,
        conversation_id.unwrap_or("N/A"),
        provider,
        model,
        content
    );

    // 确保日志目录存在
    if let Err(e) = std::fs::create_dir_all("logs") {
        tracing::error!("Failed to create logs directory: {}", e);
        return;
    }

    // 写入专门的 LLM 请求日志文件
    let log_file_path = format!(
        "logs/llm-http-requests-{}.log",
        chrono::Utc::now().format("%Y-%m-%d")
    );

    match OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)
    {
        Ok(mut file) => {
            if let Err(e) = file.write_all(log_entry.as_bytes()) {
                tracing::error!("Failed to write to LLM log file {}: {}", log_file_path, e);
            } else {
                let _ = file.flush();
            }
        }
        Err(e) => {
            tracing::error!("Failed to open LLM log file {}: {}", log_file_path, e);
        }
    }

    write_llm_jsonl_log(
        session_id,
        conversation_id,
        provider,
        model,
        log_type,
        content,
    );
}

fn write_stream_log(
    session_id: &str,
    conversation_id: Option<&str>,
    provider: &str,
    model: &str,
    sequence: u64,
    event_type: &str,
    payload: &serde_json::Value,
) {
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f UTC");
    let payload_text = truncate_with_marker(&payload.to_string(), STREAM_EVENT_LOG_MAX_CHARS);
    let log_entry = format!(
        "[{}] [STREAM {}] [Session: {}] [Conversation: {}] [Provider: {}] [Model: {}] [Event: {}] {}\n",
        timestamp,
        sequence,
        session_id,
        conversation_id.unwrap_or("N/A"),
        provider,
        model,
        event_type,
        payload_text
    );

    if let Err(e) = std::fs::create_dir_all("logs") {
        tracing::error!("Failed to create logs directory: {}", e);
        return;
    }

    let log_file_path = format!(
        "logs/llm-stream-events-{}.log",
        chrono::Utc::now().format("%Y-%m-%d")
    );

    match OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)
    {
        Ok(mut file) => {
            if let Err(e) = file.write_all(log_entry.as_bytes()) {
                tracing::error!(
                    "Failed to write to stream log file {}: {}",
                    log_file_path,
                    e
                );
            } else {
                let _ = file.flush();
            }
        }
        Err(e) => {
            tracing::error!("Failed to open stream log file {}: {}", log_file_path, e);
        }
    }
}

pub fn log_stream_event(
    session_id: &str,
    conversation_id: Option<&str>,
    provider: &str,
    model: &str,
    event_type: &str,
    payload: &serde_json::Value,
) {
    let sequence = LLM_STREAM_EVENT_COUNTER.fetch_add(1, Ordering::Relaxed);
    let timestamp = chrono::Utc::now();
    let sanitized_payload = truncate_json_value_strings(payload, STREAM_EVENT_LOG_MAX_CHARS);
    let event = serde_json::json!({
        "timestamp": timestamp.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        "sequence": sequence,
        "event_type": event_type,
        "session_id": session_id,
        "conversation_id": conversation_id.unwrap_or("N/A"),
        "provider": provider,
        "model": model,
        "payload": sanitized_payload,
    });

    if let Err(e) = std::fs::create_dir_all("logs") {
        tracing::error!("Failed to create logs directory: {}", e);
        return;
    }

    let jsonl_file_path = format!(
        "logs/llm-stream-events-{}.jsonl",
        chrono::Utc::now().format("%Y-%m-%d")
    );
    match OpenOptions::new()
        .create(true)
        .append(true)
        .open(&jsonl_file_path)
    {
        Ok(mut file) => {
            if let Err(e) = writeln!(file, "{}", event) {
                tracing::error!(
                    "Failed to write to stream JSONL log file {}: {}",
                    jsonl_file_path,
                    e
                );
            } else {
                let _ = file.flush();
            }
        }
        Err(e) => {
            tracing::error!(
                "Failed to open stream JSONL log file {}: {}",
                jsonl_file_path,
                e
            );
        }
    }

    write_stream_log(
        session_id,
        conversation_id,
        provider,
        model,
        sequence,
        event_type,
        &event["payload"],
    );
}

fn extract_turn_number(session_id: &str) -> Option<u64> {
    session_id
        .rsplit("::turn-")
        .next()
        .and_then(|raw| raw.parse::<u64>().ok())
}

pub fn log_turn_summary(
    session_id: &str,
    conversation_id: Option<&str>,
    provider: &str,
    model: &str,
    payload: &serde_json::Value,
) {
    let sanitized_payload = truncate_json_value_strings(payload, TURN_LOG_MAX_CHARS);
    let turn_number = extract_turn_number(session_id);
    let event = serde_json::json!({
        "timestamp": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        "session_id": session_id,
        "conversation_id": conversation_id.unwrap_or("N/A"),
        "turn": turn_number,
        "provider": provider,
        "model": model,
        "summary": sanitized_payload,
    });

    if let Err(e) = std::fs::create_dir_all("logs") {
        tracing::error!("Failed to create logs directory: {}", e);
        return;
    }

    let jsonl_file_path = format!(
        "logs/llm-turns-{}.jsonl",
        chrono::Utc::now().format("%Y-%m-%d")
    );
    match OpenOptions::new()
        .create(true)
        .append(true)
        .open(&jsonl_file_path)
    {
        Ok(mut file) => {
            if let Err(e) = writeln!(file, "{}", event) {
                tracing::error!(
                    "Failed to write to turn JSONL log file {}: {}",
                    jsonl_file_path,
                    e
                );
            } else {
                let _ = file.flush();
            }
        }
        Err(e) => {
            tracing::error!(
                "Failed to open turn JSONL log file {}: {}",
                jsonl_file_path,
                e
            );
        }
    }

    let log_file_path = format!(
        "logs/llm-turns-{}.log",
        chrono::Utc::now().format("%Y-%m-%d")
    );
    let payload_text = truncate_with_marker(&event["summary"].to_string(), TURN_LOG_MAX_CHARS);
    let log_entry = format!(
        "[{}] [TURN {}] [Session: {}] [Conversation: {}] [Provider: {}] [Model: {}] {}\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f UTC"),
        turn_number
            .map(|v| v.to_string())
            .unwrap_or_else(|| "N/A".to_string()),
        session_id,
        conversation_id.unwrap_or("N/A"),
        provider,
        model,
        payload_text
    );
    match OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)
    {
        Ok(mut file) => {
            if let Err(e) = file.write_all(log_entry.as_bytes()) {
                tracing::error!("Failed to write to turn log file {}: {}", log_file_path, e);
            } else {
                let _ = file.flush();
            }
        }
        Err(e) => {
            tracing::error!("Failed to open turn log file {}: {}", log_file_path, e);
        }
    }
}

/// 写入工具调用日志
pub fn write_tool_log(
    session_id: &str,
    conversation_id: Option<&str>,
    provider: &str,
    model: &str,
    log_type: &str,
    content: &str,
) {
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f UTC");
    let log_entry = format!(
        "[{}] [{}] [Session: {}] [Conversation: {}] [Provider: {}] [Model: {}] {}\n",
        timestamp,
        log_type,
        session_id,
        conversation_id.unwrap_or("N/A"),
        provider,
        model,
        content
    );

    if let Err(e) = std::fs::create_dir_all("logs") {
        tracing::error!("Failed to create logs directory: {}", e);
        return;
    }

    let log_file_path = format!(
        "logs/llm-tool-calls-{}.log",
        chrono::Utc::now().format("%Y-%m-%d")
    );

    match OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)
    {
        Ok(mut file) => {
            if let Err(e) = file.write_all(log_entry.as_bytes()) {
                tracing::error!("Failed to write to tool log file {}: {}", log_file_path, e);
            } else {
                let _ = file.flush();
            }
        }
        Err(e) => {
            tracing::error!("Failed to open tool log file {}: {}", log_file_path, e);
        }
    }
}

pub fn log_tool_call(
    session_id: &str,
    conversation_id: Option<&str>,
    provider: &str,
    model: &str,
    tool_name: &str,
    tool_call_id: &str,
    arguments: &str,
) {
    write_tool_log(
        session_id,
        conversation_id,
        provider,
        model,
        "TOOL CALL",
        &format!(
            "\nTool: {}\nCall ID: {}\nArguments: {}\n",
            tool_name, tool_call_id, arguments
        ),
    );
}

pub fn log_tool_result(
    session_id: &str,
    conversation_id: Option<&str>,
    provider: &str,
    model: &str,
    tool_name: &str,
    tool_call_id: &str,
    duration_ms: Option<i64>,
    success: bool,
    result: &str,
) {
    let mut result_trimmed = truncate_utf8_at_boundary(result, TOOL_LOG_MAX_CHARS);
    if result_trimmed.len() < result.len() {
        result_trimmed.push_str("\n...[truncated]");
    }

    let duration_str = duration_ms
        .map(|v| v.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    write_tool_log(
        session_id,
        conversation_id,
        provider,
        model,
        "TOOL RESULT",
        &format!(
            "\nTool: {}\nCall ID: {}\nDuration: {} ms\nSuccess: {}\nResult:\n{}\n",
            tool_name, tool_call_id, duration_str, success, result_trimmed
        ),
    );
}

/// 记录 LLM 请求
pub fn log_request(
    session_id: &str,
    conversation_id: Option<&str>,
    provider: &str,
    model: &str,
    system_prompt: Option<&str>,
    user_prompt: &str,
) {
    log_request_with_image(
        session_id,
        conversation_id,
        provider,
        model,
        system_prompt,
        user_prompt,
        false,
    );
}

/// 记录 LLM 请求（含图片标记）
pub fn log_request_with_image(
    session_id: &str,
    conversation_id: Option<&str>,
    provider: &str,
    model: &str,
    system_prompt: Option<&str>,
    user_prompt: &str,
    has_image: bool,
) {
    // 记录 system prompt（如果存在）
    if let Some(system_prompt) = system_prompt {
        let system_trimmed = truncate_with_marker(system_prompt, LLM_REQUEST_LOG_MAX_CHARS);
        write_llm_log(
            session_id,
            conversation_id,
            provider,
            model,
            "SYSTEM REQUEST",
            &format!("\n{}\n", system_trimmed),
        );
    }
    // 记录 user prompt（含图片标记）
    let image_tag = if has_image { " [WITH IMAGE]" } else { "" };
    let user_trimmed = truncate_with_marker(user_prompt, LLM_REQUEST_LOG_MAX_CHARS);
    write_llm_log(
        session_id,
        conversation_id,
        provider,
        model,
        &format!("USER REQUEST{}", image_tag),
        &format!("\n{}\n", user_trimmed),
    );
}

/// 记录 LLM 响应
pub fn log_response(
    session_id: &str,
    conversation_id: Option<&str>,
    provider: &str,
    model: &str,
    response: &str,
) {
    let response_trimmed = truncate_with_marker(response, LLM_RESPONSE_LOG_MAX_CHARS);
    write_llm_log(
        session_id,
        conversation_id,
        provider,
        model,
        "OUTPUT RESPONSE",
        &format!("\n{}\n", response_trimmed),
    );
}

/// 记录 LLM 错误响应
pub fn log_error_response(
    session_id: &str,
    conversation_id: Option<&str>,
    provider: &str,
    model: &str,
    error_type: &str,
    error_message: &str,
) {
    let error_trimmed = truncate_with_marker(error_message, LLM_ERROR_LOG_MAX_CHARS);
    write_llm_log(
        session_id,
        conversation_id,
        provider,
        model,
        "ERROR RESPONSE",
        &format!("\nType: {}\nMessage: {}\n", error_type, error_trimmed),
    );
}

#[cfg(test)]
mod tests {
    use super::truncate_utf8_at_boundary;

    #[test]
    fn truncate_utf8_never_panics_on_multibyte_boundary() {
        let input = "a中😀b";
        let out = truncate_utf8_at_boundary(input, 4);
        assert_eq!(out, "a中");
    }
}
