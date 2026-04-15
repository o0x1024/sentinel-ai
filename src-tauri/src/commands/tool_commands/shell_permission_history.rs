use sentinel_llm::log::tool_calls_log_jsonl_paths_for_date;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct ShellPermissionHistoryQuery {
    pub date: Option<String>,
    pub days: Option<usize>,
    pub execution_id: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellPermissionRulePreview {
    pub rule: String,
    pub reason_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellPermissionHistoryEntry {
    pub timestamp: String,
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_id: Option<String>,
    pub command: String,
    pub decision: String,
    pub allowed: bool,
    pub semantic_kind: String,
    pub semantic_code: String,
    pub semantic_summary_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_reason_key: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suggested_allow_rules: Vec<ShellPermissionRulePreview>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub persisted_allow_rules: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct StructuredToolEvent {
    timestamp: String,
    event_type: String,
    session_id: String,
    conversation_id: String,
    payload: StructuredShellPermissionPayload,
}

#[derive(Debug, Deserialize)]
struct StructuredShellPermissionPayload {
    command: String,
    execution_id: Option<String>,
    decision: String,
    allowed: bool,
    semantic_kind: String,
    semantic_code: String,
    semantic_summary_key: String,
    semantic_reason_key: Option<String>,
    #[serde(default)]
    suggested_allow_rules: Vec<ShellPermissionRulePreview>,
    #[serde(default)]
    persisted_allow_rules: Vec<String>,
}

fn resolve_history_dates(request: &ShellPermissionHistoryQuery) -> Vec<String> {
    let explicit_date = request
        .date
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string());
    if let Some(date) = explicit_date {
        return vec![date];
    }

    let days = request.days.unwrap_or(7).clamp(1, 30);
    let today = chrono::Utc::now().date_naive();

    (0..days)
        .map(|offset| {
            (today - chrono::Days::new(offset as u64))
                .format("%Y-%m-%d")
                .to_string()
        })
        .collect()
}

fn build_history_paths(dates: &[String]) -> Vec<PathBuf> {
    dates
        .iter()
        .flat_map(|date| tool_calls_log_jsonl_paths_for_date(date))
        .collect()
}

fn for_each_jsonl_line_reverse<F>(path: &Path, mut visit: F) -> Result<(), String>
where
    F: FnMut(&str) -> Result<bool, String>,
{
    let mut file = File::open(path)
        .map_err(|e| format!("Failed to open shell permission log {}: {}", path.display(), e))?;
    let mut pos = file
        .seek(SeekFrom::End(0))
        .map_err(|e| format!("Failed to seek shell permission log {}: {}", path.display(), e))?;
    let mut remainder = Vec::<u8>::new();
    let mut chunk = vec![0u8; 8192];

    while pos > 0 {
        let read_size = usize::try_from(pos.min(chunk.len() as u64)).unwrap_or(chunk.len());
        pos -= read_size as u64;
        file.seek(SeekFrom::Start(pos))
            .map_err(|e| format!("Failed to seek shell permission log {}: {}", path.display(), e))?;
        file.read_exact(&mut chunk[..read_size])
            .map_err(|e| format!("Failed to read shell permission log {}: {}", path.display(), e))?;

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

pub async fn get_shell_permission_history(
    request: ShellPermissionHistoryQuery,
) -> Result<Vec<ShellPermissionHistoryEntry>, String> {
    let execution_filter = request
        .execution_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string());
    let limit = request.limit.unwrap_or(20).clamp(1, 200);
    let mut rows = Vec::new();

    for path in build_history_paths(&resolve_history_dates(&request)) {
        if !path.exists() {
            continue;
        }

        for_each_jsonl_line_reverse(&path, |line| {
            let event = match serde_json::from_str::<StructuredToolEvent>(line) {
                Ok(value) => value,
                Err(_) => return Ok(true),
            };
            if event.event_type != "shell_permission" {
                return Ok(true);
            }

            let execution_id = event
                .payload
                .execution_id
                .clone()
                .or_else(|| match event.conversation_id.as_str() {
                    "" | "N/A" => None,
                    other => Some(other.to_string()),
                });

            if execution_filter
                .as_ref()
                .is_some_and(|filter| execution_id.as_deref() != Some(filter.as_str()))
            {
                return Ok(true);
            }

            rows.push(ShellPermissionHistoryEntry {
                timestamp: event.timestamp,
                session_id: event.session_id,
                execution_id,
                command: event.payload.command,
                decision: event.payload.decision,
                allowed: event.payload.allowed,
                semantic_kind: event.payload.semantic_kind,
                semantic_code: event.payload.semantic_code,
                semantic_summary_key: event.payload.semantic_summary_key,
                semantic_reason_key: event.payload.semantic_reason_key,
                suggested_allow_rules: event.payload.suggested_allow_rules,
                persisted_allow_rules: event.payload.persisted_allow_rules,
            });

            Ok(rows.len() < limit)
        })?;

        if rows.len() >= limit {
            break;
        }
    }

    Ok(rows)
}
