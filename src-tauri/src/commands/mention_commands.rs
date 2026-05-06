use crate::commands::traffic::TrafficAnalysisState;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use tauri::State;
use tokio::io::AsyncReadExt;
use walkdir::{DirEntry, WalkDir};

use sentinel_db::DatabaseService;

const DEFAULT_SEARCH_LIMIT: usize = 8;
const MAX_SEARCH_RESULTS: usize = 20;
const MAX_SCAN_FILES: usize = 5000;
const MAX_PREVIEW_BYTES: u64 = 256 * 1024;
const DEFAULT_PREVIEW_CHARS: usize = 4000;
const MAX_PREVIEW_CHARS: usize = 12000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingDirectoryFileMatch {
    pub relative_path: String,
    pub file_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingDirectoryFilePreview {
    pub id: String,
    pub path: String,
    pub relative_path: String,
    pub preview: String,
    pub truncated: bool,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyRequestMentionMatch {
    pub id: i64,
    pub method: String,
    pub host: String,
    pub url: String,
    pub status_code: i32,
    pub timestamp: String,
}

#[derive(Debug)]
struct RankedMatch {
    relative_path: String,
    file_name: String,
    score: u8,
}

fn normalize_relative_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn skip_directory(entry: &DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return true;
    }
    let name = entry.file_name().to_string_lossy();
    !matches!(
        name.as_ref(),
        ".git" | ".svn" | ".hg" | "node_modules" | "target" | "dist" | "build" | ".next"
    )
}

async fn resolve_effective_working_directory(
    db_service: &DatabaseService,
    conversation_id: Option<&str>,
) -> Result<PathBuf, String> {
    let raw = crate::commands::ai_conversation_binding_support::resolve_effective_conversation_working_directory(
        db_service,
        conversation_id,
    )
    .await?
    .unwrap_or_default();
    if raw.is_empty() {
        return Err("请先在 Agent 设置中配置工作目录".to_string());
    }

    let canonical = std::fs::canonicalize(&raw)
        .map_err(|e| format!("Failed to resolve working directory: {e}"))?;
    if !canonical.is_dir() {
        return Err("工作目录不存在或不是文件夹".to_string());
    }
    Ok(canonical)
}

fn query_tokens(query: &str) -> Vec<String> {
    query
        .split(|ch: char| ch.is_whitespace())
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| part.to_lowercase())
        .collect()
}

fn rank_match(relative_path: &str, query: &str) -> Option<RankedMatch> {
    let normalized_path = relative_path.replace('\\', "/");
    let lower_path = normalized_path.to_lowercase();
    let file_name = Path::new(&normalized_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(relative_path)
        .to_string();
    let lower_name = file_name.to_lowercase();
    let trimmed_query = query.trim().to_lowercase();

    if trimmed_query.is_empty() {
        return Some(RankedMatch {
            relative_path: normalized_path,
            file_name,
            score: 4,
        });
    }

    let tokens = query_tokens(&trimmed_query);
    if !tokens.iter().all(|token| lower_path.contains(token)) {
        return None;
    }

    let score = if lower_name == trimmed_query {
        0
    } else if lower_name.starts_with(&trimmed_query) {
        1
    } else if lower_path.starts_with(&trimmed_query) {
        2
    } else if lower_name.contains(&trimmed_query) {
        3
    } else {
        4
    };

    Some(RankedMatch {
        relative_path: normalized_path,
        file_name,
        score,
    })
}

fn compare_ranked_match(a: &RankedMatch, b: &RankedMatch) -> Ordering {
    a.score
        .cmp(&b.score)
        .then_with(|| a.relative_path.len().cmp(&b.relative_path.len()))
        .then_with(|| a.relative_path.cmp(&b.relative_path))
}

fn ensure_relative_path(path: &str) -> Result<PathBuf, String> {
    let relative = PathBuf::from(path);
    if relative.as_os_str().is_empty() {
        return Err("文件路径不能为空".to_string());
    }
    if relative.is_absolute() {
        return Err("只允许使用工作目录内的相对路径".to_string());
    }
    if relative
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err("文件路径不能包含上级目录跳转".to_string());
    }
    Ok(relative)
}

#[tauri::command]
pub async fn search_recent_proxy_requests(
    query: String,
    limit: Option<usize>,
    traffic_state: State<'_, TrafficAnalysisState>,
) -> Result<Vec<ProxyRequestMentionMatch>, String> {
    let capped_limit = limit
        .unwrap_or(DEFAULT_SEARCH_LIMIT)
        .clamp(1, MAX_SEARCH_RESULTS);

    let filters = sentinel_traffic::HttpRequestFilters {
        scheme: None,
        method: None,
        host: None,
        status_code_min: None,
        status_code_max: None,
        search: if query.trim().is_empty() {
            None
        } else {
            Some(query)
        },
        limit: Some(capped_limit),
        offset: None,
    };

    let requests = traffic_state
        .get_history_cache()
        .list_http_request_summaries(filters)
        .await;

    Ok(requests
        .into_iter()
        .map(|item| ProxyRequestMentionMatch {
            id: item.id,
            method: item.method,
            host: item.host,
            url: item.url,
            status_code: item.status_code,
            timestamp: item.timestamp.to_rfc3339(),
        })
        .collect())
}

#[tauri::command]
pub async fn search_working_directory_files(
    query: String,
    limit: Option<usize>,
    conversation_id: Option<String>,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<WorkingDirectoryFileMatch>, String> {
    let working_directory = resolve_effective_working_directory(
        db_service.inner().as_ref(),
        conversation_id.as_deref(),
    )
    .await?;
    let capped_limit = limit
        .unwrap_or(DEFAULT_SEARCH_LIMIT)
        .clamp(1, MAX_SEARCH_RESULTS);

    let mut ranked: Vec<RankedMatch> = Vec::new();
    let mut scanned_files = 0usize;

    for entry in WalkDir::new(&working_directory)
        .follow_links(false)
        .max_depth(12)
        .into_iter()
        .filter_entry(skip_directory)
        .filter_map(Result::ok)
    {
        if scanned_files >= MAX_SCAN_FILES {
            break;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        scanned_files += 1;

        let Ok(relative) = entry.path().strip_prefix(&working_directory) else {
            continue;
        };
        let relative_path = normalize_relative_path(relative);
        let Some(item) = rank_match(&relative_path, &query) else {
            continue;
        };
        ranked.push(item);
    }

    ranked.sort_by(compare_ranked_match);
    ranked.truncate(capped_limit);

    Ok(ranked
        .into_iter()
        .map(|item| WorkingDirectoryFileMatch {
            relative_path: item.relative_path,
            file_name: item.file_name,
        })
        .collect())
}

#[tauri::command]
pub async fn read_working_directory_file_preview(
    relative_path: String,
    max_chars: Option<usize>,
    conversation_id: Option<String>,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<WorkingDirectoryFilePreview, String> {
    let working_directory = resolve_effective_working_directory(
        db_service.inner().as_ref(),
        conversation_id.as_deref(),
    )
    .await?;
    let safe_relative = ensure_relative_path(&relative_path)?;
    let full_path = working_directory.join(&safe_relative);
    let canonical = std::fs::canonicalize(&full_path)
        .map_err(|e| format!("Failed to resolve file path: {e}"))?;

    if !canonical.starts_with(&working_directory) {
        return Err("文件路径超出工作目录范围".to_string());
    }

    let metadata = tokio::fs::metadata(&canonical)
        .await
        .map_err(|e| format!("Failed to read file metadata: {e}"))?;
    if !metadata.is_file() {
        return Err("目标路径不是文件".to_string());
    }
    if metadata.len() > MAX_PREVIEW_BYTES {
        return Err("文件过大，当前版本暂不支持直接引用该文件".to_string());
    }

    let mut file = tokio::fs::File::open(&canonical)
        .await
        .map_err(|e| format!("Failed to open file: {e}"))?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.read_to_end(&mut bytes)
        .await
        .map_err(|e| format!("Failed to read file: {e}"))?;

    if bytes.iter().take(8192).any(|byte| *byte == 0) {
        return Err("当前只支持引用文本文件".to_string());
    }

    let content = String::from_utf8_lossy(&bytes).to_string();
    let preview_limit = max_chars
        .unwrap_or(DEFAULT_PREVIEW_CHARS)
        .clamp(200, MAX_PREVIEW_CHARS);
    let mut preview = String::new();
    let mut char_count = 0usize;
    let mut truncated = false;

    for ch in content.chars() {
        if char_count >= preview_limit {
            truncated = true;
            break;
        }
        preview.push(ch);
        char_count += 1;
    }

    let relative_path = normalize_relative_path(&safe_relative);

    Ok(WorkingDirectoryFilePreview {
        id: relative_path.clone(),
        path: canonical.to_string_lossy().to_string(),
        relative_path,
        preview,
        truncated,
        size: metadata.len(),
    })
}
