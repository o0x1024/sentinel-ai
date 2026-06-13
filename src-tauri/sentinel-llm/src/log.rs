//! LLM 请求/响应日志记录模块

use std::fs::{self, File, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Once,
};

const TOOL_LOG_MAX_CHARS: usize = 8000;
const LLM_REQUEST_LOG_MAX_CHARS: usize = 12000;
const LLM_RESPONSE_LOG_MAX_CHARS: usize = 12000;
const LLM_ERROR_LOG_MAX_CHARS: usize = 6000;
const LLM_JSONL_PREVIEW_MAX_CHARS: usize = 800;
const STREAM_EVENT_LOG_MAX_CHARS: usize = 8000;
const TURN_LOG_MAX_CHARS: usize = 12000;
const LLM_LOG_ROOT_DIR: &str = "logs/llm";
static LLM_TURN_COUNTER: AtomicU64 = AtomicU64::new(1);
static LLM_STREAM_EVENT_COUNTER: AtomicU64 = AtomicU64::new(1);
static LLM_LOG_STORAGE_INIT: Once = Once::new();

#[derive(Clone, Copy)]
struct LlmLogCategory {
    directory: &'static str,
    legacy_prefix: &'static str,
}

const HTTP_REQUESTS_LOG_CATEGORY: LlmLogCategory = LlmLogCategory {
    directory: "http-requests",
    legacy_prefix: "llm-http-requests-",
};

const STREAM_EVENTS_LOG_CATEGORY: LlmLogCategory = LlmLogCategory {
    directory: "stream-events",
    legacy_prefix: "llm-stream-events-",
};

const TURNS_LOG_CATEGORY: LlmLogCategory = LlmLogCategory {
    directory: "turns",
    legacy_prefix: "llm-turns-",
};

const TOOL_CALLS_LOG_CATEGORY: LlmLogCategory = LlmLogCategory {
    directory: "tool-calls",
    legacy_prefix: "llm-tool-calls-",
};

const MIGRATED_LOG_CATEGORIES: [LlmLogCategory; 4] = [
    HTTP_REQUESTS_LOG_CATEGORY,
    STREAM_EVENTS_LOG_CATEGORY,
    TURNS_LOG_CATEGORY,
    TOOL_CALLS_LOG_CATEGORY,
];

fn ensure_directory(path: &Path) -> io::Result<()> {
    fs::create_dir_all(path)
}

fn logs_root_dir() -> PathBuf {
    PathBuf::from("logs")
}

fn llm_log_category_dir(category: LlmLogCategory) -> PathBuf {
    PathBuf::from(LLM_LOG_ROOT_DIR).join(category.directory)
}

fn current_utc_log_date() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

fn legacy_llm_log_path(category: LlmLogCategory, date: &str, extension: &str) -> PathBuf {
    logs_root_dir().join(format!("{}{}.{}", category.legacy_prefix, date, extension))
}

fn initialize_llm_log_storage() {
    LLM_LOG_STORAGE_INIT.call_once(|| {
        if let Err(e) = ensure_directory(&logs_root_dir()) {
            tracing::error!("Failed to create logs directory: {}", e);
            return;
        }

        for category in MIGRATED_LOG_CATEGORIES {
            let category_dir = llm_log_category_dir(category);
            if let Err(e) = ensure_directory(&category_dir) {
                tracing::error!(
                    "Failed to create categorized LLM log directory {}: {}",
                    category_dir.display(),
                    e
                );
            }
        }

        migrate_legacy_llm_logs();
    });
}

fn categorized_llm_log_path(category: LlmLogCategory, extension: &str) -> PathBuf {
    llm_log_category_dir(category).join(format!("{}.{}", current_utc_log_date(), extension))
}

fn categorized_llm_log_path_for_date(
    category: LlmLogCategory,
    date: &str,
    extension: &str,
) -> PathBuf {
    llm_log_category_dir(category).join(format!("{}.{}", date, extension))
}

fn open_categorized_llm_log_file(
    category: LlmLogCategory,
    extension: &str,
) -> io::Result<(PathBuf, File)> {
    initialize_llm_log_storage();

    let category_dir = llm_log_category_dir(category);
    ensure_directory(&category_dir)?;

    let path = categorized_llm_log_path(category, extension);
    let file = OpenOptions::new().create(true).append(true).open(&path)?;
    Ok((path, file))
}

pub fn turn_log_jsonl_paths_for_date(date: &str) -> Vec<PathBuf> {
    vec![
        categorized_llm_log_path_for_date(TURNS_LOG_CATEGORY, date, "jsonl"),
        legacy_llm_log_path(TURNS_LOG_CATEGORY, date, "jsonl"),
    ]
}

pub fn tool_calls_log_jsonl_paths_for_date(date: &str) -> Vec<PathBuf> {
    vec![
        categorized_llm_log_path_for_date(TOOL_CALLS_LOG_CATEGORY, date, "jsonl"),
        legacy_llm_log_path(TOOL_CALLS_LOG_CATEGORY, date, "jsonl"),
    ]
}

fn categorize_legacy_llm_log_file(file_name: &str) -> Option<(LlmLogCategory, &str)> {
    MIGRATED_LOG_CATEGORIES.iter().find_map(|category| {
        file_name
            .strip_prefix(category.legacy_prefix)
            .map(|suffix| (*category, suffix))
    })
}

fn move_legacy_log_file(source: &Path, destination: &Path) -> io::Result<()> {
    if source == destination || !source.exists() {
        return Ok(());
    }

    if let Some(parent) = destination.parent() {
        ensure_directory(parent)?;
    }

    if !destination.exists() {
        return match fs::rename(source, destination) {
            Ok(()) => Ok(()),
            Err(_) => {
                fs::copy(source, destination)?;
                fs::remove_file(source)
            }
        };
    }

    let mut source_file = File::open(source)?;
    let mut destination_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(destination)?;

    io::copy(&mut source_file, &mut destination_file)?;
    destination_file.flush()?;
    fs::remove_file(source)
}

fn migrate_legacy_llm_logs() {
    let read_dir = match fs::read_dir(logs_root_dir()) {
        Ok(entries) => entries,
        Err(e) => {
            tracing::error!("Failed to scan logs directory for legacy LLM logs: {}", e);
            return;
        }
    };

    for entry in read_dir {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                tracing::warn!("Failed to inspect a logs directory entry: {}", e);
                continue;
            }
        };

        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let file_name = match entry.file_name().into_string() {
            Ok(file_name) => file_name,
            Err(_) => continue,
        };

        let Some((category, suffix)) = categorize_legacy_llm_log_file(&file_name) else {
            continue;
        };

        let destination = llm_log_category_dir(category).join(suffix);
        if let Err(e) = move_legacy_log_file(&path, &destination) {
            tracing::warn!(
                "Failed to migrate legacy LLM log {} to {}: {}",
                path.display(),
                destination.display(),
                e
            );
        }
    }
}

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

    match open_categorized_llm_log_file(HTTP_REQUESTS_LOG_CATEGORY, "jsonl") {
        Ok((jsonl_file_path, mut file)) => {
            if let Err(e) = writeln!(file, "{}", event) {
                tracing::error!(
                    "Failed to write to LLM JSONL log file {}: {}",
                    jsonl_file_path.display(),
                    e
                );
            } else {
                let _ = file.flush();
            }
        }
        Err(e) => {
            tracing::error!("Failed to open LLM JSONL log file: {}", e);
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

    match open_categorized_llm_log_file(HTTP_REQUESTS_LOG_CATEGORY, "log") {
        Ok((log_file_path, mut file)) => {
            if let Err(e) = file.write_all(log_entry.as_bytes()) {
                tracing::error!(
                    "Failed to write to LLM log file {}: {}",
                    log_file_path.display(),
                    e
                );
            } else {
                let _ = file.flush();
            }
        }
        Err(e) => {
            tracing::error!("Failed to open LLM log file: {}", e);
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

    match open_categorized_llm_log_file(STREAM_EVENTS_LOG_CATEGORY, "log") {
        Ok((log_file_path, mut file)) => {
            if let Err(e) = file.write_all(log_entry.as_bytes()) {
                tracing::error!(
                    "Failed to write to stream log file {}: {}",
                    log_file_path.display(),
                    e
                );
            } else {
                let _ = file.flush();
            }
        }
        Err(e) => {
            tracing::error!("Failed to open stream log file: {}", e);
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

    match open_categorized_llm_log_file(STREAM_EVENTS_LOG_CATEGORY, "jsonl") {
        Ok((jsonl_file_path, mut file)) => {
            if let Err(e) = writeln!(file, "{}", event) {
                tracing::error!(
                    "Failed to write to stream JSONL log file {}: {}",
                    jsonl_file_path.display(),
                    e
                );
            } else {
                let _ = file.flush();
            }
        }
        Err(e) => {
            tracing::error!("Failed to open stream JSONL log file: {}", e);
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

    match open_categorized_llm_log_file(TURNS_LOG_CATEGORY, "jsonl") {
        Ok((jsonl_file_path, mut file)) => {
            if let Err(e) = writeln!(file, "{}", event) {
                tracing::error!(
                    "Failed to write to turn JSONL log file {}: {}",
                    jsonl_file_path.display(),
                    e
                );
            } else {
                let _ = file.flush();
            }
        }
        Err(e) => {
            tracing::error!("Failed to open turn JSONL log file: {}", e);
        }
    }

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
    match open_categorized_llm_log_file(TURNS_LOG_CATEGORY, "log") {
        Ok((log_file_path, mut file)) => {
            if let Err(e) = file.write_all(log_entry.as_bytes()) {
                tracing::error!(
                    "Failed to write to turn log file {}: {}",
                    log_file_path.display(),
                    e
                );
            } else {
                let _ = file.flush();
            }
        }
        Err(e) => {
            tracing::error!("Failed to open turn log file: {}", e);
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

    match open_categorized_llm_log_file(TOOL_CALLS_LOG_CATEGORY, "log") {
        Ok((log_file_path, mut file)) => {
            if let Err(e) = file.write_all(log_entry.as_bytes()) {
                tracing::error!(
                    "Failed to write to tool log file {}: {}",
                    log_file_path.display(),
                    e
                );
            } else {
                let _ = file.flush();
            }
        }
        Err(e) => {
            tracing::error!("Failed to open tool log file: {}", e);
        }
    }
}

pub fn log_structured_tool_event(
    session_id: &str,
    conversation_id: Option<&str>,
    provider: &str,
    model: &str,
    event_type: &str,
    payload: &serde_json::Value,
) {
    let timestamp = chrono::Utc::now();
    let sanitized_payload = truncate_json_value_strings(payload, TOOL_LOG_MAX_CHARS);
    let event = serde_json::json!({
        "timestamp": timestamp.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        "event_type": event_type,
        "session_id": session_id,
        "conversation_id": conversation_id.unwrap_or("N/A"),
        "provider": provider,
        "model": model,
        "payload": sanitized_payload,
    });

    match open_categorized_llm_log_file(TOOL_CALLS_LOG_CATEGORY, "jsonl") {
        Ok((jsonl_file_path, mut file)) => {
            if let Err(e) = writeln!(file, "{}", event) {
                tracing::error!(
                    "Failed to write to tool JSONL log file {}: {}",
                    jsonl_file_path.display(),
                    e
                );
            } else {
                let _ = file.flush();
            }
        }
        Err(e) => {
            tracing::error!("Failed to open tool JSONL log file: {}", e);
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

/// 记录 LLM 上下文消息（orchestrator context / history）
pub fn log_context_messages(
    session_id: &str,
    conversation_id: Option<&str>,
    provider: &str,
    model: &str,
    messages: &[(&str, &str)],
) {
    if messages.is_empty() {
        return;
    }
    let mut body = String::new();
    let mut total_chars = 0;
    for (i, (role, content)) in messages.iter().enumerate() {
        let header = format!("\n--- Message {} [{}] ---\n", i + 1, role);
        let content_preview = truncate_with_marker(content, 3000);
        let entry = format!("{}{}\n", header, content_preview);
        total_chars += entry.len();
        if total_chars > LLM_REQUEST_LOG_MAX_CHARS {
            body.push_str(&format!(
                "\n...[{} more messages truncated]",
                messages.len() - i
            ));
            break;
        }
        body.push_str(&entry);
    }

    write_llm_log(
        session_id,
        conversation_id,
        provider,
        model,
        &format!("CONTEXT MESSAGES ({} total)", messages.len()),
        &body,
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
    use super::{
        categorize_legacy_llm_log_file, truncate_utf8_at_boundary, turn_log_jsonl_paths_for_date,
        write_tool_log, HTTP_REQUESTS_LOG_CATEGORY, STREAM_EVENTS_LOG_CATEGORY,
        TOOL_CALLS_LOG_CATEGORY, TURNS_LOG_CATEGORY,
    };
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn truncate_utf8_never_panics_on_multibyte_boundary() {
        let input = "a中😀b";
        let out = truncate_utf8_at_boundary(input, 4);
        assert_eq!(out, "a中");
    }

    #[test]
    fn categorize_legacy_llm_log_file_maps_known_prefixes() {
        let http = categorize_legacy_llm_log_file("llm-http-requests-2026-04-08.log");
        assert_eq!(
            http.map(|(category, _)| category.directory),
            Some(HTTP_REQUESTS_LOG_CATEGORY.directory)
        );
        assert_eq!(http.map(|(_, suffix)| suffix), Some("2026-04-08.log"));

        let stream = categorize_legacy_llm_log_file("llm-stream-events-2026-04-08.jsonl");
        assert_eq!(
            stream.map(|(category, _)| category.directory),
            Some(STREAM_EVENTS_LOG_CATEGORY.directory)
        );
        assert_eq!(stream.map(|(_, suffix)| suffix), Some("2026-04-08.jsonl"));

        let tool = categorize_legacy_llm_log_file("llm-tool-calls-2026-04-08.log");
        assert_eq!(
            tool.map(|(category, _)| category.directory),
            Some(TOOL_CALLS_LOG_CATEGORY.directory)
        );
        assert_eq!(tool.map(|(_, suffix)| suffix), Some("2026-04-08.log"));

        let turn = categorize_legacy_llm_log_file("llm-turns-2026-04-08.jsonl");
        assert_eq!(
            turn.map(|(category, _)| category.directory),
            Some(TURNS_LOG_CATEGORY.directory)
        );
        assert_eq!(turn.map(|(_, suffix)| suffix), Some("2026-04-08.jsonl"));
    }

    #[test]
    fn write_tool_log_uses_categorized_directory_and_migrates_legacy_files() {
        let original_dir = env::current_dir().expect("current dir");
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("duration")
            .as_nanos();
        let temp_dir = env::temp_dir().join(format!("sentinel-llm-log-test-{}", unique));
        let legacy_dir = temp_dir.join("logs");
        let legacy_path = legacy_dir.join("llm-tool-calls-2026-04-08.log");
        let run = || -> Result<(), Box<dyn std::error::Error>> {
            fs::create_dir_all(&legacy_dir)?;
            fs::write(&legacy_path, "legacy-entry\n")?;

            env::set_current_dir(&temp_dir)?;
            write_tool_log(
                "session-1",
                Some("conversation-1"),
                "provider-x",
                "model-y",
                "TOOL CALL",
                "tool payload",
            );

            let migrated_path = temp_dir.join("logs/llm/tool-calls/2026-04-08.log");
            let today_path = temp_dir.join(format!(
                "logs/llm/tool-calls/{}.log",
                chrono::Utc::now().format("%Y-%m-%d")
            ));

            assert!(migrated_path.exists(), "legacy file should be migrated");
            assert!(
                !legacy_path.exists(),
                "legacy flat file should be removed from logs root"
            );
            assert!(
                today_path.exists(),
                "new tool log should use categorized directory"
            );
            Ok(())
        };

        let result = run();
        let _ = env::set_current_dir(&original_dir);
        let _ = fs::remove_dir_all(&temp_dir);
        result.expect("categorized tool log write should succeed");
    }

    #[test]
    fn turn_log_jsonl_paths_prioritize_categorized_path() {
        let paths = turn_log_jsonl_paths_for_date("2026-04-08");
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0], PathBuf::from("logs/llm/turns/2026-04-08.jsonl"));
        assert_eq!(paths[1], PathBuf::from("logs/llm-turns-2026-04-08.jsonl"));
    }
}
