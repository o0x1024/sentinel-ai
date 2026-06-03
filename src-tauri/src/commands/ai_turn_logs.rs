use sentinel_llm::log::turn_log_jsonl_paths_for_date;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct AiTurnLogQuery {
    pub date: Option<String>,
    pub conversation_id: Option<String>,
    pub session_id: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiTurnLogEntry {
    pub timestamp: String,
    pub session_id: String,
    pub conversation_id: String,
    pub turn: Option<u64>,
    pub provider: String,
    pub model: String,
    pub summary: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiTurnLogSummaryEntry {
    pub timestamp: String,
    pub session_id: String,
    pub conversation_id: String,
    pub turn: Option<u64>,
    pub provider: String,
    pub model: String,
    pub status: String,
    pub duration_ms: Option<i64>,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub tool_call_count: usize,
    pub user_request_preview: String,
    pub assistant_response_preview: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AiTurnLogDetailQuery {
    pub date: Option<String>,
    pub session_id: String,
}

fn resolve_turn_log_date(date: Option<&str>) -> String {
    date.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d").to_string())
}

fn build_turn_log_paths(date: &str) -> Vec<PathBuf> {
    turn_log_jsonl_paths_for_date(date)
}

fn first_existing_turn_log_path(paths: &[PathBuf]) -> Option<PathBuf> {
    paths.iter().find(|path| path.exists()).cloned()
}

fn for_each_jsonl_line_reverse<F>(path: &Path, mut visit: F) -> Result<(), String>
where
    F: FnMut(&str) -> Result<bool, String>,
{
    let mut file = File::open(path)
        .map_err(|e| format!("Failed to open turn log file {}: {}", path.display(), e))?;
    let mut pos = file
        .seek(SeekFrom::End(0))
        .map_err(|e| format!("Failed to seek turn log file {}: {}", path.display(), e))?;
    let mut remainder = Vec::<u8>::new();
    let mut chunk = vec![0u8; 8192];

    while pos > 0 {
        let read_size = usize::try_from(pos.min(chunk.len() as u64)).unwrap_or(chunk.len());
        pos -= read_size as u64;
        file.seek(SeekFrom::Start(pos))
            .map_err(|e| format!("Failed to seek turn log file {}: {}", path.display(), e))?;
        file.read_exact(&mut chunk[..read_size])
            .map_err(|e| format!("Failed to read turn log file {}: {}", path.display(), e))?;

        let mut combined = Vec::with_capacity(read_size + remainder.len());
        combined.extend_from_slice(&chunk[..read_size]);
        combined.extend_from_slice(&remainder);

        let parts = combined.split(|b| *b == b'\n').collect::<Vec<_>>();
        remainder = parts.first().map(|part| part.to_vec()).unwrap_or_default();

        for part in parts.iter().skip(1).rev() {
            let line = String::from_utf8_lossy(part).trim().to_string();
            if line.is_empty() {
                continue;
            }
            if !visit(&line)? {
                return Ok(());
            }
        }
    }

    if !remainder.is_empty() {
        let line = String::from_utf8_lossy(&remainder).trim().to_string();
        if !line.is_empty() && !visit(&line)? {
            return Ok(());
        }
    }

    Ok(())
}

fn preview_from_json_value(value: &serde_json::Value, max_chars: usize) -> String {
    let raw = match value {
        serde_json::Value::String(value) => value.clone(),
        serde_json::Value::Null => String::new(),
        other => serde_json::to_string(other).unwrap_or_default(),
    };
    let normalized = raw
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .trim()
        .to_string();
    if normalized.chars().count() <= max_chars {
        return normalized;
    }
    let mut preview = normalized.chars().take(max_chars).collect::<String>();
    preview.push_str("...");
    preview
}

fn build_turn_log_summary(entry: AiTurnLogEntry) -> AiTurnLogSummaryEntry {
    let status = entry
        .summary
        .get("status")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown")
        .to_string();
    let duration_ms = entry
        .summary
        .get("duration_ms")
        .and_then(|value| value.as_i64());
    let input_tokens = entry
        .summary
        .get("usage")
        .and_then(|value| value.get("input_tokens"))
        .and_then(|value| value.as_u64())
        .and_then(|value| u32::try_from(value).ok());
    let output_tokens = entry
        .summary
        .get("usage")
        .and_then(|value| value.get("output_tokens"))
        .and_then(|value| value.as_u64())
        .and_then(|value| u32::try_from(value).ok());
    let tool_call_count = entry
        .summary
        .get("tool_calls")
        .and_then(|value| value.as_array())
        .map(|value| value.len())
        .unwrap_or(0);
    let user_request_preview = preview_from_json_value(
        entry
            .summary
            .get("user_request")
            .unwrap_or(&serde_json::Value::Null),
        160,
    );
    let assistant_response_preview = preview_from_json_value(
        entry
            .summary
            .get("assistant_response")
            .unwrap_or(&serde_json::Value::Null),
        220,
    );

    AiTurnLogSummaryEntry {
        timestamp: entry.timestamp,
        session_id: entry.session_id,
        conversation_id: entry.conversation_id,
        turn: entry.turn,
        provider: entry.provider,
        model: entry.model,
        status,
        duration_ms,
        input_tokens,
        output_tokens,
        tool_call_count,
        user_request_preview,
        assistant_response_preview,
    }
}

#[tauri::command]
pub async fn get_ai_turn_logs(
    request: AiTurnLogQuery,
) -> Result<Vec<AiTurnLogSummaryEntry>, String> {
    let date = resolve_turn_log_date(request.date.as_deref());
    let paths = build_turn_log_paths(&date);
    let Some(path) = first_existing_turn_log_path(&paths) else {
        return Ok(Vec::new());
    };

    let conversation_filter = request
        .conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string());
    let session_filter = request
        .session_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string());
    let limit = request.limit.unwrap_or(50).min(500);

    let mut rows = Vec::new();
    for_each_jsonl_line_reverse(&path, |line| {
        let entry = match serde_json::from_str::<AiTurnLogEntry>(line) {
            Ok(value) => value,
            Err(e) => {
                tracing::warn!("Skipping malformed turn log row: {}", e);
                return Ok(true);
            }
        };

        if let Some(filter) = conversation_filter.as_deref() {
            if !entry.conversation_id.contains(filter) {
                return Ok(true);
            }
        }
        if let Some(filter) = session_filter.as_deref() {
            if !entry.session_id.contains(filter) {
                return Ok(true);
            }
        }

        rows.push(build_turn_log_summary(entry));
        Ok(rows.len() < limit)
    })?;

    Ok(rows)
}

#[tauri::command]
pub async fn get_ai_turn_log_detail(
    request: AiTurnLogDetailQuery,
) -> Result<Option<AiTurnLogEntry>, String> {
    let session_id = request.session_id.trim();
    if session_id.is_empty() {
        return Err("session_id is required".to_string());
    }

    let date = resolve_turn_log_date(request.date.as_deref());
    let paths = build_turn_log_paths(&date);
    let Some(path) = first_existing_turn_log_path(&paths) else {
        return Ok(None);
    };

    let mut found = None;
    for_each_jsonl_line_reverse(&path, |line| {
        let entry = match serde_json::from_str::<AiTurnLogEntry>(line) {
            Ok(value) => value,
            Err(e) => {
                tracing::warn!(
                    "Skipping malformed turn log row while loading detail: {}",
                    e
                );
                return Ok(true);
            }
        };
        if entry.session_id == session_id {
            found = Some(entry);
            return Ok(false);
        }
        Ok(true)
    })?;

    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::{build_turn_log_paths, first_existing_turn_log_path};
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn build_turn_log_paths_prefers_categorized_path() {
        let paths = build_turn_log_paths("2026-04-08");
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0], PathBuf::from("logs/llm/turns/2026-04-08.jsonl"));
        assert_eq!(paths[1], PathBuf::from("logs/llm-turns-2026-04-08.jsonl"));
    }

    #[test]
    fn first_existing_turn_log_path_falls_back_to_legacy_path() {
        let original_dir = std::env::current_dir().expect("current dir");
        let temp_dir = std::env::temp_dir().join(format!(
            "sentinel-ai-turn-log-path-test-{}",
            std::process::id()
        ));
        let categorized_dir = temp_dir.join("logs/llm/turns");
        let legacy_path = temp_dir.join("logs/llm-turns-2026-04-08.jsonl");

        let run = || -> Result<(), Box<dyn std::error::Error>> {
            fs::create_dir_all(&categorized_dir)?;
            fs::create_dir_all(temp_dir.join("logs"))?;
            fs::write(&legacy_path, b"{}\n")?;
            std::env::set_current_dir(&temp_dir)?;

            let selected = first_existing_turn_log_path(&build_turn_log_paths("2026-04-08"))
                .expect("a log path should exist");
            assert_eq!(selected, PathBuf::from("logs/llm-turns-2026-04-08.jsonl"));
            Ok(())
        };

        let result = run();
        let _ = std::env::set_current_dir(&original_dir);
        let _ = fs::remove_dir_all(&temp_dir);
        result.expect("legacy path fallback should work");
    }
}
