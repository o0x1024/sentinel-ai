//! Unified file upload and attachment commands.

use crate::models::attachment::DocumentAttachment;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::AppHandle;
use tauri::Manager;
use uuid::Uuid;

use sentinel_db::Database;
use sentinel_tools::shell::{get_shell_config, ShellExecutionMode};
use sentinel_tools::DockerSandbox;

const INDEX_FILE: &str = "index.json";
const DEFAULT_MAX_FILE_MB: u64 = 20;
const DEFAULT_MAX_TOTAL_MB: u64 = 1024;
const DEFAULT_MAX_FILES_PER_CONVERSATION: usize = 100;
const DEFAULT_RETENTION_DAYS: i64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerAnalysisStatus {
    pub docker_available: bool,
    pub image_exists: bool,
    pub container_ready: bool,
    pub ready_for_file_analysis: bool,
    pub supported_file_types: Vec<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedDocumentResult {
    pub id: String,
    pub file_id: String,
    pub original_filename: String,
    pub file_size: u64,
    pub mime_type: String,
    pub status: String,
    pub file_path: Option<String>,
    pub sha256: String,
    pub created_at: i64,
    pub conversation_id: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStatResult {
    pub size: u64,
    pub is_file: bool,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadedFileEntry {
    pub file_id: String,
    pub date: String,
    pub filename: String,
    pub path: String,
    pub size: u64,
    pub mime_type: String,
    pub sha256: String,
    pub created_at: i64,
    pub conversation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadedFileSettings {
    pub auto_cleanup_enabled: bool,
    pub retention_days: i64,
    pub max_file_mb: u64,
    pub max_total_mb: u64,
    pub max_files_per_conversation: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UploadIndex {
    files: Vec<UploadedFileEntry>,
}

impl Default for UploadIndex {
    fn default() -> Self {
        Self { files: vec![] }
    }
}

fn get_upload_root_dir() -> Result<PathBuf, String> {
    let base = dirs::data_dir().ok_or_else(|| "Failed to resolve data directory".to_string())?;
    Ok(base.join("sentinel-ai").join("uploads"))
}

fn get_index_path(root: &Path) -> PathBuf {
    root.join(INDEX_FILE)
}

fn sanitize_file_stem(stem: &str) -> String {
    let sanitized = stem
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();
    if sanitized.trim().is_empty() {
        "file".to_string()
    } else {
        sanitized
    }
}

fn is_valid_date_segment(date: &str) -> bool {
    date.len() == 10
        && date.chars().enumerate().all(|(idx, ch)| {
            if idx == 4 || idx == 7 {
                ch == '-'
            } else {
                ch.is_ascii_digit()
            }
        })
}

async fn ensure_upload_root() -> Result<PathBuf, String> {
    let root = get_upload_root_dir()?;
    tokio::fs::create_dir_all(&root)
        .await
        .map_err(|e| format!("Failed to create upload root: {}", e))?;
    Ok(root)
}

async fn read_upload_index(root: &Path) -> Result<UploadIndex, String> {
    let index_path = get_index_path(root);
    if !index_path.exists() {
        return Ok(UploadIndex::default());
    }
    let raw = tokio::fs::read_to_string(&index_path)
        .await
        .map_err(|e| format!("Failed to read upload index: {}", e))?;
    serde_json::from_str::<UploadIndex>(&raw)
        .map_err(|e| format!("Failed to parse upload index: {}", e))
}

async fn write_upload_index(root: &Path, index: &UploadIndex) -> Result<(), String> {
    let index_path = get_index_path(root);
    let raw = serde_json::to_string_pretty(index)
        .map_err(|e| format!("Failed to serialize upload index: {}", e))?;
    tokio::fs::write(index_path, raw)
        .await
        .map_err(|e| format!("Failed to write upload index: {}", e))
}

fn detect_mime_type(path: &Path, bytes: &[u8]) -> String {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if bytes.starts_with(b"%PDF-") {
        return "application/pdf".to_string();
    }
    if bytes.starts_with(&[0x50, 0x4B, 0x03, 0x04]) {
        return "application/zip".to_string();
    }
    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        return "image/png".to_string();
    }

    DocumentAttachment::mime_type_from_extension(&ext).to_string()
}

fn compute_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

async fn load_uploaded_file_settings(app_handle: &AppHandle) -> UploadedFileSettings {
    let mut settings = UploadedFileSettings {
        auto_cleanup_enabled: false,
        retention_days: DEFAULT_RETENTION_DAYS,
        max_file_mb: DEFAULT_MAX_FILE_MB,
        max_total_mb: DEFAULT_MAX_TOTAL_MB,
        max_files_per_conversation: DEFAULT_MAX_FILES_PER_CONVERSATION,
    };

    let db_state = app_handle.try_state::<Arc<crate::services::database::DatabaseService>>();
    let Some(db) = db_state else {
        return settings;
    };

    if let Ok(Some(v)) = db.get_config("agent", "upload_auto_cleanup_enabled").await {
        settings.auto_cleanup_enabled = v == "1" || v.eq_ignore_ascii_case("true");
    }
    if let Ok(Some(v)) = db.get_config("agent", "upload_retention_days").await {
        if let Ok(n) = v.parse::<i64>() {
            settings.retention_days = n.max(1);
        }
    }
    if let Ok(Some(v)) = db.get_config("agent", "upload_max_file_mb").await {
        if let Ok(n) = v.parse::<u64>() {
            settings.max_file_mb = n.max(1);
        }
    }
    if let Ok(Some(v)) = db.get_config("agent", "upload_max_total_mb").await {
        if let Ok(n) = v.parse::<u64>() {
            settings.max_total_mb = n.max(1);
        }
    }
    if let Ok(Some(v)) = db.get_config("agent", "upload_max_files_per_conversation").await {
        if let Ok(n) = v.parse::<usize>() {
            settings.max_files_per_conversation = n.max(1);
        }
    }

    settings
}

#[tauri::command]
pub async fn get_uploaded_file_settings(app_handle: AppHandle) -> Result<UploadedFileSettings, String> {
    Ok(load_uploaded_file_settings(&app_handle).await)
}

#[tauri::command]
pub async fn save_uploaded_file_settings(
    app_handle: AppHandle,
    settings: UploadedFileSettings,
) -> Result<(), String> {
    let db = app_handle
        .try_state::<Arc<crate::services::database::DatabaseService>>()
        .ok_or_else(|| "Database service not available".to_string())?;

    db.set_config("agent", "upload_auto_cleanup_enabled", if settings.auto_cleanup_enabled { "true" } else { "false" }, None)
        .await
        .map_err(|e| format!("Failed to save config: {}", e))?;
    db.set_config("agent", "upload_retention_days", &settings.retention_days.to_string(), None)
        .await
        .map_err(|e| format!("Failed to save config: {}", e))?;
    db.set_config("agent", "upload_max_file_mb", &settings.max_file_mb.to_string(), None)
        .await
        .map_err(|e| format!("Failed to save config: {}", e))?;
    db.set_config("agent", "upload_max_total_mb", &settings.max_total_mb.to_string(), None)
        .await
        .map_err(|e| format!("Failed to save config: {}", e))?;
    db.set_config(
        "agent",
        "upload_max_files_per_conversation",
        &settings.max_files_per_conversation.to_string(),
        None,
    )
    .await
    .map_err(|e| format!("Failed to save config: {}", e))?;

    Ok(())
}

async fn maybe_auto_cleanup(app_handle: &AppHandle, index: &mut UploadIndex) -> Result<(), String> {
    let settings = load_uploaded_file_settings(app_handle).await;
    if !settings.auto_cleanup_enabled {
        return Ok(());
    }

    let cutoff = chrono::Utc::now().timestamp() - settings.retention_days * 24 * 3600;
    let mut removed = 0usize;
    index.files.retain(|entry| {
        if entry.created_at < cutoff {
            let p = PathBuf::from(&entry.path);
            let _ = std::fs::remove_file(p);
            removed += 1;
            false
        } else {
            true
        }
    });

    if removed > 0 {
        tracing::info!("[upload-audit] auto-cleanup removed {} files", removed);
    }
    Ok(())
}

async fn resolve_runtime_path_for_host_file(host_path: &str) -> Result<(String, Option<String>), String> {
    let shell_config = get_shell_config().await;
    let docker_requested = shell_config.default_execution_mode == ShellExecutionMode::Docker
        && shell_config.docker_config.is_some();
    let docker_ready = DockerSandbox::is_docker_available().await
        && DockerSandbox::image_exists("sentinel-sandbox:latest").await;

    if !(docker_requested && docker_ready) {
        return Ok((host_path.to_string(), None));
    }

    let docker_config = shell_config
        .docker_config
        .ok_or_else(|| "Docker config not available".to_string())?;
    let sandbox = DockerSandbox::new(docker_config);
    let host_path_obj = Path::new(host_path);

    let date_dir = host_path_obj
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("misc");
    let filename = host_path_obj
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file.bin");

    let container_dir = format!("/workspace/uploads/{}", date_dir);
    let container_path = format!("{}/{}", container_dir, filename);

    let _ = sandbox.execute(&format!("mkdir -p '{}'", container_dir), 10).await;
    sandbox
        .copy_file_to_container(host_path, &container_path)
        .await
        .map_err(|e| format!("Failed to copy file to docker: {}", e))?;

    Ok((container_path.clone(), Some(container_path)))
}

pub async fn resolve_uploaded_file_for_execution_by_id(
    app_handle: &AppHandle,
    file_id: &str,
) -> Result<String, String> {
    let root = ensure_upload_root().await?;
    let mut index = read_upload_index(&root).await?;
    maybe_auto_cleanup(app_handle, &mut index).await?;

    let file = index
        .files
        .iter()
        .find(|f| f.file_id == file_id)
        .ok_or_else(|| format!("Uploaded file not found: {}", file_id))?;

    if !Path::new(&file.path).exists() {
        return Err(format!("Uploaded file missing from disk: {}", file.path));
    }

    let (runtime_path, _) = resolve_runtime_path_for_host_file(&file.path).await?;
    Ok(runtime_path)
}

#[tauri::command]
pub async fn run_file_security_analysis(file_id: String) -> Result<String, String> {
    let docker_available = DockerSandbox::is_docker_available().await;
    let image_exists = DockerSandbox::image_exists("sentinel-sandbox:latest").await;
    let container_ready = DockerSandbox::get_persistent_shell_container_info()
        .await
        .map(|info| info.is_some())
        .unwrap_or(false);
    if !(docker_available && image_exists && container_ready) {
        return Err("Security analysis requires a ready Docker sandbox".to_string());
    }
    Ok(format!("Security analysis accepted for file_id={}.", file_id))
}

#[tauri::command]
pub async fn get_file_stat(path: String) -> Result<FileStatResult, String> {
    let metadata = std::fs::metadata(&path)
        .map_err(|e| format!("Failed to get file metadata: {}", e))?;

    Ok(FileStatResult {
        size: metadata.len(),
        is_file: metadata.is_file(),
        is_dir: metadata.is_dir(),
    })
}

#[tauri::command]
pub async fn check_docker_for_file_analysis() -> Result<DockerAnalysisStatus, String> {
    let docker_available = DockerSandbox::is_docker_available().await;

    if !docker_available {
        return Ok(DockerAnalysisStatus {
            docker_available: false,
            image_exists: false,
            container_ready: false,
            ready_for_file_analysis: false,
            supported_file_types: DocumentAttachment::SUPPORTED_EXTENSIONS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            error_message: Some("Docker is not available".to_string()),
        });
    }

    let image_exists = DockerSandbox::image_exists("sentinel-sandbox:latest").await;

    if !image_exists {
        return Ok(DockerAnalysisStatus {
            docker_available: true,
            image_exists: false,
            container_ready: false,
            ready_for_file_analysis: false,
            supported_file_types: DocumentAttachment::SUPPORTED_EXTENSIONS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            error_message: Some("Sandbox image not found. Please build it first.".to_string()),
        });
    }

    let container_ready = DockerSandbox::get_persistent_shell_container_info()
        .await
        .map(|info| info.is_some())
        .unwrap_or(false);

    Ok(DockerAnalysisStatus {
        docker_available: true,
        image_exists: true,
        container_ready,
        ready_for_file_analysis: container_ready,
        supported_file_types: DocumentAttachment::SUPPORTED_EXTENSIONS
            .iter()
            .map(|s| s.to_string())
            .collect(),
        error_message: None,
    })
}

#[tauri::command]
pub async fn upload_document_attachment(
    app_handle: AppHandle,
    file_path: String,
    client_id: Option<String>,
    conversation_id: Option<String>,
) -> Result<ProcessedDocumentResult, String> {
    let source = Path::new(&file_path);
    if !source.exists() {
        return Err(format!("File not found: {}", file_path));
    }
    if !source.is_file() {
        return Err(format!("Not a regular file: {}", file_path));
    }

    let settings = load_uploaded_file_settings(&app_handle).await;

    let bytes = tokio::fs::read(source)
        .await
        .map_err(|e| format!("Failed to read source file: {}", e))?;
    let file_size = bytes.len() as u64;
    let max_file_bytes = settings.max_file_mb * 1024 * 1024;
    if file_size > max_file_bytes {
        return Err(format!(
            "File too large: {} bytes (limit {} MB)",
            file_size, settings.max_file_mb
        ));
    }

    let root = ensure_upload_root().await?;
    let mut index = read_upload_index(&root).await?;
    maybe_auto_cleanup(&app_handle, &mut index).await?;

    if let Some(conv_id) = &conversation_id {
        let conv_count = index
            .files
            .iter()
            .filter(|f| f.conversation_id.as_ref() == Some(conv_id))
            .count();
        if conv_count >= settings.max_files_per_conversation {
            return Err(format!(
                "Too many files in conversation (limit {})",
                settings.max_files_per_conversation
            ));
        }
    }

    let total_size: u64 = index
        .files
        .iter()
        .filter_map(|f| std::fs::metadata(&f.path).ok().map(|m| m.len()))
        .sum();
    let max_total_bytes = settings.max_total_mb * 1024 * 1024;
    if total_size + file_size > max_total_bytes {
        return Err(format!(
            "Total upload quota exceeded (limit {} MB)",
            settings.max_total_mb
        ));
    }

    let sha256 = compute_sha256(&bytes);

    if let Some(existing) = index
        .files
        .iter()
        .find(|f| f.sha256 == sha256 && Path::new(&f.path).exists())
        .cloned()
    {
        let virtual_path = format!("file://{}", existing.file_id);
        tracing::info!(
            "[upload-audit] dedup hit file_id={} source={} conv={}",
            existing.file_id,
            file_path,
            conversation_id.clone().unwrap_or_else(|| "-".to_string())
        );
        return Ok(ProcessedDocumentResult {
            id: client_id.clone().unwrap_or_else(|| existing.file_id.clone()),
            file_id: existing.file_id.clone(),
            original_filename: existing.filename,
            file_size: existing.size,
            mime_type: existing.mime_type,
            status: "ready".to_string(),
            file_path: Some(virtual_path),
            sha256: existing.sha256,
            created_at: existing.created_at,
            conversation_id: existing.conversation_id,
            error_message: None,
        });
    }

    let date_dir = chrono::Local::now().format("%Y-%m-%d").to_string();
    let target_dir = root.join(&date_dir);
    tokio::fs::create_dir_all(&target_dir)
        .await
        .map_err(|e| format!("Failed to create upload directory: {}", e))?;

    let file_id = Uuid::new_v4().to_string();
    let extension = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let stem = source
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("file");
    let safe_stem = sanitize_file_stem(stem);
    let target_name = if extension.is_empty() {
        format!("{}_{}", file_id, safe_stem)
    } else {
        format!("{}_{}.{}", file_id, safe_stem, extension)
    };
    let stored_host_path = target_dir.join(target_name);
    tokio::fs::write(&stored_host_path, &bytes)
        .await
        .map_err(|e| format!("Failed to store uploaded file: {}", e))?;

    let stored_host_path_str = stored_host_path.to_string_lossy().to_string();
    let mime_type = detect_mime_type(source, &bytes);
    let original_filename = source
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    let created_at = chrono::Utc::now().timestamp();

    let entry = UploadedFileEntry {
        file_id: file_id.clone(),
        date: date_dir,
        filename: original_filename.clone(),
        path: stored_host_path_str.clone(),
        size: file_size,
        mime_type: mime_type.clone(),
        sha256: sha256.clone(),
        created_at,
        conversation_id: conversation_id.clone(),
    };
    index.files.push(entry);
    write_upload_index(&root, &index).await?;

    tracing::info!(
        "[upload-audit] stored file_id={} path={} size={} conv={}",
        file_id,
        stored_host_path_str,
        file_size,
        conversation_id.clone().unwrap_or_else(|| "-".to_string())
    );

    Ok(ProcessedDocumentResult {
        id: client_id.unwrap_or_else(|| file_id.clone()),
        file_id: file_id.clone(),
        original_filename,
        file_size,
        mime_type,
        status: "ready".to_string(),
        file_path: Some(format!("file://{}", file_id)),
        sha256,
        created_at,
        conversation_id,
        error_message: None,
    })
}

#[tauri::command]
pub async fn list_uploaded_files(
    app_handle: AppHandle,
    conversation_id: Option<String>,
    date: Option<String>,
) -> Result<Vec<UploadedFileEntry>, String> {
    let root = ensure_upload_root().await?;
    let mut index = read_upload_index(&root).await?;
    maybe_auto_cleanup(&app_handle, &mut index).await?;

    index.files.retain(|f| Path::new(&f.path).exists());
    write_upload_index(&root, &index).await?;

    let mut items: Vec<UploadedFileEntry> = index
        .files
        .into_iter()
        .filter(|f| {
            let conv_ok = conversation_id
                .as_ref()
                .map(|cid| f.conversation_id.as_ref() == Some(cid))
                .unwrap_or(true);
            let date_ok = date
                .as_ref()
                .map(|d| &f.date == d)
                .unwrap_or(true);
            conv_ok && date_ok
        })
        .collect();

    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(items)
}

fn cleanup_empty_date_dirs(root: &Path) {
    if let Ok(read_dir) = std::fs::read_dir(root) {
        for entry in read_dir.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let is_empty = std::fs::read_dir(&p)
                    .map(|mut it| it.next().is_none())
                    .unwrap_or(false);
                if is_empty {
                    let _ = std::fs::remove_dir(&p);
                }
            }
        }
    }
}

fn clear_host_context_cache() -> Result<(), String> {
    let context_dir = sentinel_tools::output_storage::get_host_context_dir();
    if !context_dir.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(&context_dir).map_err(|e| format!("Failed to read context cache: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to access context cache entry: {}", e))?;
        let path = entry.path();
        if path.is_dir() {
            std::fs::remove_dir_all(&path)
                .map_err(|e| format!("Failed to remove context cache dir {}: {}", path.display(), e))?;
        } else {
            std::fs::remove_file(&path)
                .map_err(|e| format!("Failed to remove context cache file {}: {}", path.display(), e))?;
        }
    }
    Ok(())
}

async fn clear_container_workspace_cache() -> Result<(), String> {
    let shell_config = get_shell_config().await;
    let docker_requested = shell_config.default_execution_mode == ShellExecutionMode::Docker
        && shell_config.docker_config.is_some();
    if !docker_requested || !DockerSandbox::is_docker_available().await {
        return Ok(());
    }

    let docker_config = shell_config
        .docker_config
        .ok_or_else(|| "Docker config not available".to_string())?;
    let sandbox = DockerSandbox::new(docker_config);
    let cleanup_cmd = r#"
        mkdir -p /workspace/context /workspace/uploads && \
        find /workspace/context -mindepth 1 -maxdepth 1 -exec rm -rf {} + 2>/dev/null || true && \
        find /workspace/uploads -mindepth 1 -maxdepth 1 -exec rm -rf {} + 2>/dev/null || true
    "#;
    sandbox
        .execute(cleanup_cmd, 20)
        .await
        .map_err(|e| format!("Failed to clear container workspace cache: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn clear_uploaded_files(
    _app_handle: AppHandle,
    date: Option<String>,
    conversation_id: Option<String>,
) -> Result<u64, String> {
    if let Some(d) = &date {
        if !is_valid_date_segment(d) {
            return Err("Invalid date format, expected YYYY-MM-DD".to_string());
        }
    }

    let root = ensure_upload_root().await?;
    let full_clear = date.is_none() && conversation_id.is_none();
    let mut index = read_upload_index(&root).await?;

    if full_clear {
        let removed_count = index.files.len() as u64;
        if root.exists() {
            tokio::fs::remove_dir_all(&root)
                .await
                .map_err(|e| format!("Failed to clear upload root: {}", e))?;
        }
        tokio::fs::create_dir_all(&root)
            .await
            .map_err(|e| format!("Failed to recreate upload root: {}", e))?;
        write_upload_index(&root, &UploadIndex::default()).await?;

        clear_host_context_cache()?;
        if let Err(e) = clear_container_workspace_cache().await {
            tracing::warn!("[upload-audit] failed to clear container workspace cache: {}", e);
        }

        tracing::info!("[upload-audit] full clear completed, removed files count={}", removed_count);
        return Ok(removed_count);
    }

    let mut removed_count = 0u64;
    let mut keep: Vec<UploadedFileEntry> = Vec::with_capacity(index.files.len());

    for item in index.files {
        let date_match = date.as_ref().map(|d| &item.date == d).unwrap_or(true);
        let conv_match = conversation_id
            .as_ref()
            .map(|cid| item.conversation_id.as_ref() == Some(cid))
            .unwrap_or(true);

        if date_match && conv_match {
            let _ = std::fs::remove_file(&item.path);
            removed_count += 1;
        } else {
            keep.push(item);
        }
    }

    index.files = keep;
    write_upload_index(&root, &index).await?;
    cleanup_empty_date_dirs(&root);

    tracing::info!(
        "[upload-audit] cleared files count={} date={:?} conv={:?}",
        removed_count,
        date,
        conversation_id
    );

    Ok(removed_count)
}
