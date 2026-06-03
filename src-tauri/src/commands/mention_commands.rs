use crate::commands::traffic::TrafficAnalysisState;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};
use tokio::io::AsyncReadExt;
use walkdir::{DirEntry, WalkDir};

use sentinel_db::DatabaseService;

const DEFAULT_SEARCH_LIMIT: usize = 8;
const MAX_SEARCH_RESULTS: usize = 20;
const MAX_SCAN_FILES: usize = 5000;
const MAX_PREVIEW_BYTES: u64 = 256 * 1024;
const DEFAULT_PREVIEW_CHARS: usize = 4000;
const MAX_PREVIEW_CHARS: usize = 12000;
const MAX_DIRECTORY_ENTRIES: usize = 500;
const MAX_EDIT_BYTES: usize = 256 * 1024;
const WORKSPACE_PATH_CHANGED_EVENT: &str = "agent:workspace-path-changed";

static WORKSPACE_WATCHERS: Lazy<Mutex<HashMap<String, RecommendedWatcher>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static NEXT_WORKSPACE_WATCHER_ID: AtomicU64 = AtomicU64::new(1);

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
    pub sha256: String,
    pub modified_ms: Option<i64>,
}

pub type WorkingDirectoryFileWriteResponse = WorkingDirectoryFilePreview;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingDirectoryEntry {
    pub name: String,
    pub relative_path: String,
    pub is_directory: bool,
    pub size: Option<u64>,
    pub modified_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingDirectoryEntriesResponse {
    pub root: String,
    pub relative_path: String,
    pub parent_relative_path: Option<String>,
    pub entries: Vec<WorkingDirectoryEntry>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingDirectoryWatchResponse {
    pub watcher_id: String,
    pub root: String,
    pub relative_path: String,
    pub is_directory: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingDirectoryOpenTarget {
    pub path: String,
    pub relative_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingDirectoryMutationResponse {
    pub relative_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingDirectoryChangeEvent {
    pub watcher_id: String,
    pub root: String,
    pub relative_path: String,
    pub is_directory: bool,
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

fn is_hidden_heavy_directory_name(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".svn"
            | ".hg"
            | "node_modules"
            | "target"
            | "dist"
            | "build"
            | ".next"
            | ".nuxt"
            | "coverage"
    )
}

fn resolve_working_directory_path(raw: &str) -> Result<PathBuf, String> {
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
    resolve_working_directory_path(raw.trim())
}

async fn resolve_requested_working_directory(
    db_service: &DatabaseService,
    conversation_id: Option<&str>,
    working_directory: Option<&str>,
) -> Result<PathBuf, String> {
    let requested = working_directory.unwrap_or("").trim();
    if !requested.is_empty() {
        return resolve_working_directory_path(requested);
    }
    resolve_effective_working_directory(db_service, conversation_id).await
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

fn ensure_relative_directory_path(path: Option<&str>) -> Result<PathBuf, String> {
    let raw = path.unwrap_or("").trim();
    if raw.is_empty() {
        return Ok(PathBuf::new());
    }
    ensure_relative_path(raw)
}

fn ensure_entry_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("名称不能为空".to_string());
    }
    let path = Path::new(trimmed);
    if path.is_absolute() || path.components().count() != 1 {
        return Err("名称不能包含路径分隔符".to_string());
    }
    if matches!(trimmed, "." | "..") {
        return Err("名称不能使用特殊路径段".to_string());
    }
    Ok(trimmed.to_string())
}

fn modified_ms(metadata: &std::fs::Metadata) -> Option<i64> {
    metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as i64)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

async fn resolve_working_directory_file(
    db_service: &DatabaseService,
    conversation_id: Option<&str>,
    working_directory: Option<&str>,
    relative_path: &str,
) -> Result<(PathBuf, PathBuf), String> {
    let working_directory =
        resolve_requested_working_directory(db_service, conversation_id, working_directory).await?;
    let safe_relative = ensure_relative_path(relative_path)?;
    let full_path = working_directory.join(&safe_relative);
    let canonical = std::fs::canonicalize(&full_path)
        .map_err(|e| format!("Failed to resolve file path: {e}"))?;

    if !canonical.starts_with(&working_directory) {
        return Err("文件路径超出工作目录范围".to_string());
    }

    Ok((working_directory, canonical))
}

async fn resolve_working_directory_dir(
    db_service: &DatabaseService,
    conversation_id: Option<&str>,
    working_directory: Option<&str>,
    relative_path: Option<&str>,
) -> Result<(PathBuf, PathBuf), String> {
    let working_directory =
        resolve_requested_working_directory(db_service, conversation_id, working_directory).await?;
    let safe_relative = ensure_relative_directory_path(relative_path)?;
    let full_path = working_directory.join(&safe_relative);
    let canonical = std::fs::canonicalize(&full_path)
        .map_err(|e| format!("Failed to resolve directory path: {e}"))?;

    if !canonical.starts_with(&working_directory) {
        return Err("目录路径超出工作目录范围".to_string());
    }
    if !canonical.is_dir() {
        return Err("目标路径不是文件夹".to_string());
    }

    Ok((working_directory, canonical))
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
pub async fn list_working_directory_entries(
    relative_path: Option<String>,
    conversation_id: Option<String>,
    working_directory: Option<String>,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<WorkingDirectoryEntriesResponse, String> {
    let working_directory = resolve_requested_working_directory(
        db_service.inner().as_ref(),
        conversation_id.as_deref(),
        working_directory.as_deref(),
    )
    .await?;
    let safe_relative = ensure_relative_directory_path(relative_path.as_deref())?;
    let full_path = working_directory.join(&safe_relative);
    let canonical = std::fs::canonicalize(&full_path)
        .map_err(|e| format!("Failed to resolve directory path: {e}"))?;

    if !canonical.starts_with(&working_directory) {
        return Err("目录路径超出工作目录范围".to_string());
    }

    let metadata = tokio::fs::metadata(&canonical)
        .await
        .map_err(|e| format!("Failed to read directory metadata: {e}"))?;
    if !metadata.is_dir() {
        return Err("目标路径不是文件夹".to_string());
    }

    let mut entries = Vec::new();
    let mut dir = tokio::fs::read_dir(&canonical)
        .await
        .map_err(|e| format!("Failed to read directory: {e}"))?;
    let mut truncated = false;

    while let Some(entry) = dir
        .next_entry()
        .await
        .map_err(|e| format!("Failed to read directory entry: {e}"))?
    {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.is_empty() || name == "." || name == ".." {
            continue;
        }

        let file_type = entry
            .file_type()
            .await
            .map_err(|e| format!("Failed to read file type: {e}"))?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() && is_hidden_heavy_directory_name(&name) {
            continue;
        }

        let metadata = entry
            .metadata()
            .await
            .map_err(|e| format!("Failed to read entry metadata: {e}"))?;
        let entry_relative = safe_relative.join(&name);
        entries.push(WorkingDirectoryEntry {
            name,
            relative_path: normalize_relative_path(&entry_relative),
            is_directory: metadata.is_dir(),
            size: if metadata.is_file() {
                Some(metadata.len())
            } else {
                None
            },
            modified_ms: modified_ms(&metadata),
        });

        if entries.len() >= MAX_DIRECTORY_ENTRIES {
            truncated = true;
            break;
        }
    }

    entries.sort_by(|a, b| {
        b.is_directory
            .cmp(&a.is_directory)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.name.cmp(&b.name))
    });

    let parent_relative_path = safe_relative.parent().and_then(|parent| {
        if parent.as_os_str().is_empty() {
            None
        } else {
            Some(normalize_relative_path(parent))
        }
    });

    Ok(WorkingDirectoryEntriesResponse {
        root: working_directory.to_string_lossy().to_string(),
        relative_path: normalize_relative_path(&safe_relative),
        parent_relative_path,
        entries,
        truncated,
    })
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
    working_directory: Option<String>,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<WorkingDirectoryFilePreview, String> {
    let (working_directory, canonical) = resolve_working_directory_file(
        db_service.inner().as_ref(),
        conversation_id.as_deref(),
        working_directory.as_deref(),
        &relative_path,
    )
    .await?;

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

    let content = String::from_utf8(bytes).map_err(|_| "当前只支持 UTF-8 文本文件".to_string())?;
    let content_hash = sha256_hex(content.as_bytes());
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

    let safe_relative = canonical
        .strip_prefix(&working_directory)
        .map_err(|_| "文件路径超出工作目录范围".to_string())?;
    let relative_path = normalize_relative_path(&safe_relative);

    Ok(WorkingDirectoryFilePreview {
        id: relative_path.clone(),
        path: canonical.to_string_lossy().to_string(),
        relative_path,
        preview,
        truncated,
        size: metadata.len(),
        sha256: content_hash,
        modified_ms: modified_ms(&metadata),
    })
}

#[tauri::command]
pub async fn write_working_directory_file(
    relative_path: String,
    content: String,
    expected_sha256: String,
    conversation_id: Option<String>,
    working_directory: Option<String>,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<WorkingDirectoryFileWriteResponse, String> {
    if content.len() > MAX_EDIT_BYTES {
        return Err("文件内容过大，当前版本暂不支持在面板内保存".to_string());
    }

    let (working_directory, canonical) = resolve_working_directory_file(
        db_service.inner().as_ref(),
        conversation_id.as_deref(),
        working_directory.as_deref(),
        &relative_path,
    )
    .await?;

    let metadata = tokio::fs::metadata(&canonical)
        .await
        .map_err(|e| format!("Failed to read file metadata: {e}"))?;
    if !metadata.is_file() {
        return Err("目标路径不是文件".to_string());
    }
    if metadata.len() > MAX_PREVIEW_BYTES {
        return Err("文件过大，当前版本暂不支持在面板内保存".to_string());
    }

    let mut file = tokio::fs::File::open(&canonical)
        .await
        .map_err(|e| format!("Failed to open file: {e}"))?;
    let mut current_bytes = Vec::with_capacity(metadata.len() as usize);
    file.read_to_end(&mut current_bytes)
        .await
        .map_err(|e| format!("Failed to read file: {e}"))?;

    if current_bytes.iter().take(8192).any(|byte| *byte == 0) {
        return Err("当前只支持编辑文本文件".to_string());
    }
    String::from_utf8(current_bytes.clone())
        .map_err(|_| "当前只支持 UTF-8 文本文件".to_string())?;

    let current_sha256 = sha256_hex(&current_bytes);
    if current_sha256 != expected_sha256.trim() {
        return Err("文件已被外部修改，请刷新后再保存。".to_string());
    }

    tokio::fs::write(&canonical, content.as_bytes())
        .await
        .map_err(|e| format!("Failed to write file: {e}"))?;

    read_working_directory_file_preview(
        canonical
            .strip_prefix(&working_directory)
            .map_err(|_| "文件路径超出工作目录范围".to_string())?
            .to_string_lossy()
            .to_string(),
        Some(MAX_PREVIEW_CHARS),
        conversation_id,
        Some(working_directory.to_string_lossy().to_string()),
        db_service,
    )
    .await
}

#[tauri::command]
pub async fn resolve_working_directory_file_open_target(
    relative_path: String,
    conversation_id: Option<String>,
    working_directory: Option<String>,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<WorkingDirectoryOpenTarget, String> {
    let (working_directory, canonical) = resolve_working_directory_file(
        db_service.inner().as_ref(),
        conversation_id.as_deref(),
        working_directory.as_deref(),
        &relative_path,
    )
    .await?;

    let metadata = tokio::fs::metadata(&canonical)
        .await
        .map_err(|e| format!("Failed to read file metadata: {e}"))?;
    if !metadata.is_file() {
        return Err("目标路径不是文件".to_string());
    }

    let safe_relative = canonical
        .strip_prefix(&working_directory)
        .map_err(|_| "文件路径超出工作目录范围".to_string())?;

    Ok(WorkingDirectoryOpenTarget {
        path: canonical.to_string_lossy().to_string(),
        relative_path: normalize_relative_path(safe_relative),
    })
}

#[tauri::command]
pub async fn create_working_directory_entry(
    parent_relative_path: Option<String>,
    name: String,
    is_directory: bool,
    conversation_id: Option<String>,
    working_directory: Option<String>,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<WorkingDirectoryMutationResponse, String> {
    let (working_directory, parent_dir) = resolve_working_directory_dir(
        db_service.inner().as_ref(),
        conversation_id.as_deref(),
        working_directory.as_deref(),
        parent_relative_path.as_deref(),
    )
    .await?;
    let safe_name = ensure_entry_name(&name)?;
    let target = parent_dir.join(safe_name);
    if target.exists() {
        return Err("目标已存在".to_string());
    }

    if is_directory {
        tokio::fs::create_dir(&target)
            .await
            .map_err(|e| format!("Failed to create directory: {e}"))?;
    } else {
        tokio::fs::write(&target, "")
            .await
            .map_err(|e| format!("Failed to create file: {e}"))?;
    }

    let canonical = std::fs::canonicalize(&target)
        .map_err(|e| format!("Failed to resolve created path: {e}"))?;
    if !canonical.starts_with(&working_directory) {
        return Err("创建路径超出工作目录范围".to_string());
    }
    let relative = canonical
        .strip_prefix(&working_directory)
        .map_err(|_| "创建路径超出工作目录范围".to_string())?;

    Ok(WorkingDirectoryMutationResponse {
        relative_path: normalize_relative_path(relative),
    })
}

#[tauri::command]
pub async fn rename_working_directory_entry(
    relative_path: String,
    new_name: String,
    conversation_id: Option<String>,
    working_directory: Option<String>,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<WorkingDirectoryMutationResponse, String> {
    let working_directory = resolve_requested_working_directory(
        db_service.inner().as_ref(),
        conversation_id.as_deref(),
        working_directory.as_deref(),
    )
    .await?;
    let safe_relative = ensure_relative_path(&relative_path)?;
    let source = working_directory.join(&safe_relative);
    if std::fs::symlink_metadata(&source)
        .map_err(|e| format!("Failed to read source metadata: {e}"))?
        .file_type()
        .is_symlink()
    {
        return Err("不支持重命名符号链接".to_string());
    }
    let canonical_source = std::fs::canonicalize(&source)
        .map_err(|e| format!("Failed to resolve source path: {e}"))?;
    if !canonical_source.starts_with(&working_directory) {
        return Err("源路径超出工作目录范围".to_string());
    }

    let safe_name = ensure_entry_name(&new_name)?;
    let parent = canonical_source
        .parent()
        .ok_or_else(|| "源路径没有父目录".to_string())?;
    let target = parent.join(safe_name);
    if target.exists() {
        return Err("目标已存在".to_string());
    }

    tokio::fs::rename(&canonical_source, &target)
        .await
        .map_err(|e| format!("Failed to rename entry: {e}"))?;
    let canonical_target = std::fs::canonicalize(&target)
        .map_err(|e| format!("Failed to resolve renamed path: {e}"))?;
    if !canonical_target.starts_with(&working_directory) {
        return Err("目标路径超出工作目录范围".to_string());
    }
    let relative = canonical_target
        .strip_prefix(&working_directory)
        .map_err(|_| "目标路径超出工作目录范围".to_string())?;

    Ok(WorkingDirectoryMutationResponse {
        relative_path: normalize_relative_path(relative),
    })
}

#[tauri::command]
pub async fn delete_working_directory_entry(
    relative_path: String,
    conversation_id: Option<String>,
    working_directory: Option<String>,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<WorkingDirectoryMutationResponse, String> {
    let working_directory = resolve_requested_working_directory(
        db_service.inner().as_ref(),
        conversation_id.as_deref(),
        working_directory.as_deref(),
    )
    .await?;
    let safe_relative = ensure_relative_path(&relative_path)?;
    let target = working_directory.join(&safe_relative);
    if std::fs::symlink_metadata(&target)
        .map_err(|e| format!("Failed to read target metadata: {e}"))?
        .file_type()
        .is_symlink()
    {
        return Err("不支持删除符号链接".to_string());
    }
    let canonical = std::fs::canonicalize(&target)
        .map_err(|e| format!("Failed to resolve delete path: {e}"))?;
    if !canonical.starts_with(&working_directory) {
        return Err("删除路径超出工作目录范围".to_string());
    }

    let metadata = tokio::fs::metadata(&canonical)
        .await
        .map_err(|e| format!("Failed to read target metadata: {e}"))?;
    let deleted_relative = normalize_relative_path(&safe_relative);
    if metadata.is_dir() {
        tokio::fs::remove_dir(&canonical)
            .await
            .map_err(|e| format!("Failed to delete directory: {e}"))?;
    } else if metadata.is_file() {
        tokio::fs::remove_file(&canonical)
            .await
            .map_err(|e| format!("Failed to delete file: {e}"))?;
    } else {
        return Err("不支持删除该类型路径".to_string());
    }

    Ok(WorkingDirectoryMutationResponse {
        relative_path: deleted_relative,
    })
}

#[tauri::command]
pub async fn watch_working_directory_path(
    relative_path: Option<String>,
    is_directory: bool,
    conversation_id: Option<String>,
    working_directory: Option<String>,
    app_handle: AppHandle,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<WorkingDirectoryWatchResponse, String> {
    let working_directory = resolve_requested_working_directory(
        db_service.inner().as_ref(),
        conversation_id.as_deref(),
        working_directory.as_deref(),
    )
    .await?;
    let safe_relative = ensure_relative_directory_path(relative_path.as_deref())?;
    let full_path = working_directory.join(&safe_relative);
    let canonical = std::fs::canonicalize(&full_path)
        .map_err(|e| format!("Failed to resolve watch path: {e}"))?;

    if !canonical.starts_with(&working_directory) {
        return Err("监听路径超出工作目录范围".to_string());
    }

    let metadata = tokio::fs::metadata(&canonical)
        .await
        .map_err(|e| format!("Failed to read watch path metadata: {e}"))?;
    if is_directory && !metadata.is_dir() {
        return Err("监听目标不是文件夹".to_string());
    }
    if !is_directory && !metadata.is_file() {
        return Err("监听目标不是文件".to_string());
    }

    let watcher_id = format!(
        "workspace-watch-{}",
        NEXT_WORKSPACE_WATCHER_ID.fetch_add(1, AtomicOrdering::Relaxed)
    );
    let relative = canonical
        .strip_prefix(&working_directory)
        .map_err(|_| "监听路径超出工作目录范围".to_string())?;
    let relative_path = normalize_relative_path(relative);
    let root = working_directory.to_string_lossy().to_string();
    let event_payload = WorkingDirectoryChangeEvent {
        watcher_id: watcher_id.clone(),
        root: root.clone(),
        relative_path: relative_path.clone(),
        is_directory,
    };
    let event_app = app_handle.clone();
    let mut watcher = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
        if result.is_ok() {
            let _ = event_app.emit(WORKSPACE_PATH_CHANGED_EVENT, &event_payload);
        }
    })
    .map_err(|e| format!("Failed to create workspace watcher: {e}"))?;

    watcher
        .watch(&canonical, RecursiveMode::NonRecursive)
        .map_err(|e| format!("Failed to watch workspace path: {e}"))?;

    let mut watchers = WORKSPACE_WATCHERS
        .lock()
        .map_err(|_| "工作目录监听器状态不可用".to_string())?;
    watchers.insert(watcher_id.clone(), watcher);

    Ok(WorkingDirectoryWatchResponse {
        watcher_id,
        root,
        relative_path,
        is_directory,
    })
}

#[tauri::command]
pub fn stop_working_directory_watch(watcher_id: String) -> Result<(), String> {
    if watcher_id.trim().is_empty() {
        return Ok(());
    }
    let mut watchers = WORKSPACE_WATCHERS
        .lock()
        .map_err(|_| "工作目录监听器状态不可用".to_string())?;
    watchers.remove(watcher_id.trim());
    Ok(())
}
