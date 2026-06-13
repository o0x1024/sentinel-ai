//! Tool Output Storage Module
//!
//! Stores large tool outputs to files in Docker container for on-demand retrieval
//! All tools (HTTP, Shell, etc.) use unified container storage: /workspace/context/

use crate::docker_sandbox::DockerSandbox;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Default storage threshold (50KB, aligned with Claude Code tool result cap)
const DEFAULT_STORAGE_THRESHOLD: usize = 50_000;
const DEFAULT_READBACK_CHUNK_LINES: usize = 200;

use once_cell::sync::Lazy;
use std::sync::RwLock;

/// Global storage threshold configuration
static STORAGE_THRESHOLD: Lazy<RwLock<usize>> =
    Lazy::new(|| RwLock::new(DEFAULT_STORAGE_THRESHOLD));

/// Maps execution_id → conversation_id so that all executions within the
/// same conversation share a single session directory.
static SESSION_SCOPE_MAP: Lazy<RwLock<std::collections::HashMap<String, String>>> =
    Lazy::new(|| RwLock::new(std::collections::HashMap::new()));

/// Register that `execution_id` belongs to `conversation_id`.
/// Subsequent calls to `get_execution_context_dir` with this execution_id
/// will resolve to the conversation-scoped directory.
pub fn register_session_scope(execution_id: &str, conversation_id: &str) {
    let eid = execution_id.trim();
    let cid = conversation_id.trim();
    if eid.is_empty() || cid.is_empty() || eid == cid {
        return;
    }
    if let Ok(mut map) = SESSION_SCOPE_MAP.write() {
        map.insert(eid.to_string(), cid.to_string());
    }
}

fn resolve_session_scope(execution_id: &str) -> String {
    match SESSION_SCOPE_MAP.read() {
        Ok(map) => map
            .get(execution_id.trim())
            .cloned()
            .unwrap_or_else(|| execution_id.to_string()),
        Err(_) => execution_id.to_string(),
    }
}

/// Set storage threshold
pub fn set_storage_threshold(threshold: usize) {
    if let Ok(mut t) = STORAGE_THRESHOLD.write() {
        *t = threshold;
    }
}

/// Get storage threshold
pub fn get_storage_threshold() -> usize {
    STORAGE_THRESHOLD
        .read()
        .map(|t| *t)
        .unwrap_or(DEFAULT_STORAGE_THRESHOLD)
}

/// Container context directory (unified for all tools)
pub const CONTAINER_CONTEXT_DIR: &str = "/workspace/context";
const EXECUTION_DIR_PREFIX: &str = "session_";
const DEFAULT_HISTORY_FILENAME: &str = "history.txt";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredOutputArtifact {
    pub slot: String,
    pub path: String,
    pub storage_backend: String,
    pub size: usize,
    pub lines: usize,
}

/// Host context directory (for non-Docker execution)
/// Uses system-specific user data directory
pub fn get_host_context_dir() -> std::path::PathBuf {
    // Use user data directory for better cross-platform support
    if let Some(data_dir) = dirs::data_dir() {
        data_dir.join("sentinel-ai").join("context")
    } else {
        // Fallback to current directory if data_dir is not available
        std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join(".sentinel-context")
    }
}

fn normalize_execution_scope_id(execution_id: Option<&str>) -> Option<String> {
    let raw = execution_id?.trim();
    if raw.is_empty() {
        return None;
    }

    let sanitized = raw
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
        .take(12)
        .collect::<String>();

    if sanitized.is_empty() {
        None
    } else {
        Some(sanitized)
    }
}

fn execution_dir_name(execution_id: Option<&str>) -> Option<String> {
    normalize_execution_scope_id(execution_id)
        .map(|short_id| format!("{}{}", EXECUTION_DIR_PREFIX, short_id))
}

pub fn get_execution_context_dir(context_dir: &str, execution_id: Option<&str>) -> String {
    let scope_id = execution_id.map(|id| resolve_session_scope(id));
    match execution_dir_name(scope_id.as_deref()) {
        Some(dir_name) => format!("{}/{}", context_dir, dir_name),
        None => context_dir.to_string(),
    }
}

pub fn get_host_execution_context_dir(execution_id: Option<&str>) -> std::path::PathBuf {
    let base_dir = get_host_context_dir();
    let scope_id = execution_id.map(|id| resolve_session_scope(id));
    match execution_dir_name(scope_id.as_deref()) {
        Some(dir_name) => base_dir.join(dir_name),
        None => base_dir,
    }
}

fn history_filename() -> &'static str {
    DEFAULT_HISTORY_FILENAME
}

/// Generate platform-specific file access commands
fn generate_file_access_commands(file_path: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        format!(
            r#"PowerShell:
   • Get-Content "{}" | Select-String "pattern"  (search for pattern)
   • Get-Content "{}" -Tail 50                   (view last 50 lines)
   • Get-Content "{}" -Head 50                   (view first 50 lines)
   • Get-Content "{}"                            (view full content)
   • (Get-Content "{}").Count                    (count lines)

CMD:
   • findstr /I "pattern" "{}"                   (search for pattern)
   • powershell -Command "Get-Content '{}' -Tail 50"  (view last 50 lines)
   • powershell -Command "Get-Content '{}' -Head 50"  (view first 50 lines)
   • type "{}"                                   (view full content)"#,
            file_path,
            file_path,
            file_path,
            file_path,
            file_path,
            file_path,
            file_path,
            file_path,
            file_path
        )
    }

    #[cfg(not(target_os = "windows"))]
    {
        format!(
            r#"   • grep -i "pattern" "{}"     (search for pattern)
   • tail -n 50 "{}"             (view last 50 lines)  
   • head -n 50 "{}"             (view first 50 lines)
   • wc -l "{}"                  (count lines)"#,
            file_path, file_path, file_path, file_path
        )
    }
}

fn build_container_readback_protocol(file_path: &str, total_lines: usize) -> String {
    let chunk_lines = recommended_chunk_lines(total_lines);
    let second_chunk_end = chunk_lines * 2;

    format!(
        r#"Readback protocol for full-file analysis:
1. Treat the preview as a hint only. Do not summarize or make whole-file claims from the preview.
2. Read the file sequentially in bounded chunks until you have covered all {} lines.
3. Use shell line-range reads, for example:
   • sed -n '1,{}p' {}                (first chunk)
   • sed -n '{},{}p' {}            (next chunk)
4. Continue increasing the line range until the final line is reached.
5. If you only inspected part of the file, state clearly that the analysis is partial and name the line ranges you read.

Quick access commands:
   • grep -i "pattern" "{}"     (search for pattern)
   • tail -n 50 "{}"             (view last 50 lines)
   • head -n 50 "{}"             (view first 50 lines)
   • cat "{}"                    (view full content, avoid on very large files)
   • wc -l "{}"                  (count lines)"#,
        total_lines,
        chunk_lines,
        file_path,
        chunk_lines + 1,
        second_chunk_end,
        file_path,
        file_path,
        file_path,
        file_path,
        file_path,
        file_path
    )
}

fn build_container_single_line_json_readback_protocol(
    file_path: &str,
    formatted_lines: usize,
) -> String {
    let chunk_lines = recommended_chunk_lines(formatted_lines);
    let second_chunk_start = chunk_lines + 1;
    let second_chunk_end = chunk_lines * 2;

    format!(
        r#"Readback protocol for full-file analysis:
1. Treat the preview as a hint only. Do not summarize or make whole-file claims from the preview.
2. This artifact is minified single-line JSON. Raw line counts are not useful for chunking.
3. Format the JSON and read the formatted output sequentially in bounded chunks, for example:
   • python3 -m json.tool {} | sed -n '1,{}p'
   • python3 -m json.tool {} | sed -n '{},{}p'
4. Continue increasing the formatted line range until no more output is returned.
5. If you only inspected part of the formatted output, state clearly that the analysis is partial and name the formatted line ranges you read.

Quick access commands:
   • python3 -m json.tool {} | head -n {}   (view first formatted chunk)
   • python3 -m json.tool {} | tail -n 50   (view last formatted lines)
   • grep -i "pattern" "{}"                 (search raw file)
   • wc -l "{}"                             (raw line count; expected to stay 1)"#,
        file_path,
        chunk_lines,
        file_path,
        second_chunk_start,
        second_chunk_end,
        file_path,
        chunk_lines,
        file_path,
        file_path,
        file_path
    )
}

fn build_host_readback_protocol(file_path: &str, total_lines: usize) -> String {
    let chunk_lines = recommended_chunk_lines(total_lines);
    let next_offset = chunk_lines + 1;

    format!(
        r#"Readback protocol for full-file analysis:
1. Treat the preview as a hint only. Do not summarize or make whole-file claims from the preview.
2. Prefer `file_read` for exact sequential reads on host files.
3. Read the file in bounded chunks until you have covered all {} lines, for example:
   • file_read {{ "file_path": "{}", "offset": 1, "limit": {} }}
   • file_read {{ "file_path": "{}", "offset": {}, "limit": {} }}
4. Continue increasing `offset` until `end_line == total_lines`.
5. If you only inspected part of the file, state clearly that the analysis is partial and name the line ranges you read.

Shell fallback commands:
{}"#,
        total_lines,
        file_path,
        chunk_lines,
        file_path,
        next_offset,
        chunk_lines,
        generate_file_access_commands(file_path)
    )
}

fn recommended_chunk_lines(total_lines: usize) -> usize {
    total_lines.max(1).min(DEFAULT_READBACK_CHUNK_LINES)
}

#[derive(Debug, Clone)]
struct ContainerReadbackPlan {
    reported_lines: usize,
    lines_summary: String,
    protocol: String,
}

fn build_container_readback_plan(file_path: &str, output: &str) -> ContainerReadbackPlan {
    if let Some(formatted_lines) = detect_single_line_json_formatted_lines(output) {
        return ContainerReadbackPlan {
            reported_lines: formatted_lines,
            lines_summary: format!(
                "{} formatted JSON lines (raw file: 1 line)",
                formatted_lines
            ),
            protocol: build_container_single_line_json_readback_protocol(
                file_path,
                formatted_lines,
            ),
        };
    }

    let raw_lines = output.lines().count();
    ContainerReadbackPlan {
        reported_lines: raw_lines,
        lines_summary: raw_lines.to_string(),
        protocol: build_container_readback_protocol(file_path, raw_lines),
    }
}

fn detect_single_line_json_formatted_lines(output: &str) -> Option<usize> {
    if output.lines().count() != 1 {
        return None;
    }
    let trimmed = output.trim();
    if !(trimmed.starts_with('{') || trimmed.starts_with('[')) {
        return None;
    }
    let value: Value = serde_json::from_str(trimmed).ok()?;
    let pretty = serde_json::to_string_pretty(&value).ok()?;
    Some(pretty.lines().count().max(1))
}

/// Storage result enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageResult {
    /// Output is small, returned directly
    Direct(String),
    /// Output stored to file in container with summary
    Stored {
        container_path: String,
        summary: String,
        size: usize,
        lines: usize,
    },
    /// Output stored to file on host filesystem
    StoredHost {
        host_path: String,
        summary: String,
        size: usize,
        lines: usize,
    },
}

impl StorageResult {
    /// Get the content to return to Agent
    pub fn get_agent_content(&self) -> String {
        match self {
            StorageResult::Direct(content) => content.clone(),
            StorageResult::Stored { summary, .. } => summary.clone(),
            StorageResult::StoredHost { summary, .. } => summary.clone(),
        }
    }

    pub fn to_stored_artifact(&self, slot: impl Into<String>) -> Option<StoredOutputArtifact> {
        let slot = slot.into();
        match self {
            StorageResult::Direct(_) => None,
            StorageResult::Stored {
                container_path,
                size,
                lines,
                ..
            } => Some(StoredOutputArtifact {
                slot,
                path: container_path.clone(),
                storage_backend: "container".to_string(),
                size: *size,
                lines: *lines,
            }),
            StorageResult::StoredHost {
                host_path,
                size,
                lines,
                ..
            } => Some(StoredOutputArtifact {
                slot,
                path: host_path.clone(),
                storage_backend: "host".to_string(),
                size: *size,
                lines: *lines,
            }),
        }
    }
}

/// Store large output in Docker container
/// Returns StorageResult with container file path
pub async fn store_output_in_container(
    sandbox: &DockerSandbox,
    tool_name: &str,
    output: &str,
    call_id: Option<&str>,
    execution_id: Option<&str>,
) -> anyhow::Result<StorageResult> {
    // Check size against configured threshold
    let threshold = get_storage_threshold();
    if output.len() <= threshold {
        return Ok(StorageResult::Direct(output.to_string()));
    }

    // Generate filename
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let call_suffix = call_id
        .map(|id| format!("_{}", &id[..8.min(id.len())]))
        .unwrap_or_default();
    let filename = format!("{}_{}{}.txt", tool_name, timestamp, call_suffix);
    let session_context_dir = get_execution_context_dir(CONTAINER_CONTEXT_DIR, execution_id);
    let container_path = format!("{}/{}", session_context_dir, filename);

    // Create context directory in container if not exists
    let mkdir_cmd = format!("mkdir -p {}", session_context_dir);
    let _ = sandbox.execute(&mkdir_cmd, 5).await;

    // Write output to file in container using echo with base64 encoding to avoid quote issues
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(output);
    let write_cmd = format!("echo '{}' | base64 -d > {}", encoded, container_path);

    sandbox
        .execute(&write_cmd, 30)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to write output to container: {}", e))?;

    let readback_plan = build_container_readback_plan(&container_path, output);
    let lines = readback_plan.reported_lines;
    let size = output.len();

    // Generate preview (first 500 chars)
    let preview = output.chars().take(500).collect::<String>();
    let preview_end = if output.len() > 500 { "\n..." } else { "" };

    // Create summary with instructions for container-based file access
    let summary = format!(
        r#"[Large Output Stored to Container File]
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 Container Path: {}
 Size: {} bytes ({:.2} KB)
 Lines: {}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Preview (first 500 chars):
{}{}

{}

All context files are in: {}
"#,
        container_path,
        size,
        size as f64 / 1024.0,
        readback_plan.lines_summary,
        preview,
        preview_end,
        readback_plan.protocol,
        session_context_dir
    );

    Ok(StorageResult::Stored {
        container_path,
        summary,
        size,
        lines,
    })
}

/// Store conversation history in container (isolated by execution_id)
pub async fn store_history_in_container(
    sandbox: &DockerSandbox,
    history_content: &str,
) -> anyhow::Result<String> {
    store_history_in_container_with_id(sandbox, history_content, None).await
}

/// Store conversation history in container with execution_id isolation
/// Each execution gets its own history file to prevent cross-session data leakage
pub async fn store_history_in_container_with_id(
    sandbox: &DockerSandbox,
    history_content: &str,
    execution_id: Option<&str>,
) -> anyhow::Result<String> {
    let session_context_dir = get_execution_context_dir(CONTAINER_CONTEXT_DIR, execution_id);
    let container_path = format!("{}/{}", session_context_dir, history_filename());

    // Create context directory
    let mkdir_cmd = format!("mkdir -p {}", session_context_dir);
    let _ = sandbox.execute(&mkdir_cmd, 5).await;

    // Write history using base64 encoding
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(history_content);
    let write_cmd = format!("echo '{}' | base64 -d > {}", encoded, container_path);

    sandbox
        .execute(&write_cmd, 30)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to write history to container: {}", e))?;

    Ok(container_path)
}

/// Store conversation history on host filesystem (isolated by execution_id).
/// When `custom_base_dir` is provided, uses it instead of the default app data directory.
pub async fn store_history_on_host(
    history_content: &str,
    execution_id: Option<&str>,
    custom_base_dir: Option<&str>,
) -> anyhow::Result<String> {
    let context_dir = match custom_base_dir.filter(|d| !d.trim().is_empty()) {
        Some(base) => {
            let base_path = std::path::PathBuf::from(base);
            let scope_id = execution_id.map(|id| resolve_session_scope(id));
            match execution_dir_name(scope_id.as_deref()) {
                Some(dir_name) => base_path.join(dir_name),
                None => base_path,
            }
        }
        None => get_host_execution_context_dir(execution_id),
    };
    std::fs::create_dir_all(&context_dir)
        .map_err(|e| anyhow::anyhow!("Failed to create context directory: {}", e))?;

    let host_path = context_dir.join(history_filename());

    std::fs::write(&host_path, history_content)
        .map_err(|e| anyhow::anyhow!("Failed to write history to host file: {}", e))?;

    Ok(host_path.display().to_string())
}

/// Get history file path for a given execution_id
pub fn get_history_path(context_dir: &str, execution_id: Option<&str>) -> String {
    let session_context_dir = get_execution_context_dir(context_dir, execution_id);
    format!("{}/{}", session_context_dir, history_filename())
}

/// Initialize context directory in container
pub async fn init_container_context(sandbox: &DockerSandbox) -> anyhow::Result<()> {
    let mkdir_cmd = format!("mkdir -p {}", CONTAINER_CONTEXT_DIR);
    sandbox
        .execute(&mkdir_cmd, 5)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create context directory: {}", e))?;
    Ok(())
}

/// Clean up context files in container for a specific execution
/// Should be called when execution completes
///
/// Removes:
/// - Temporary files in /workspace root: *.py, *.js, *.php, *.txt, etc.
/// - Temporary directories in /workspace (except context/)
/// - The specific execution's history file
///
/// Preserves:
/// - Per-execution output artifacts under /workspace/context/session_*
/// - Other executions' history files
pub async fn cleanup_container_context(sandbox: &DockerSandbox) -> anyhow::Result<()> {
    cleanup_container_context_with_id(sandbox, None).await
}

/// Clean up context files in container for a specific execution_id
pub async fn cleanup_container_context_with_id(
    sandbox: &DockerSandbox,
    execution_id: Option<&str>,
) -> anyhow::Result<()> {
    tracing::info!("Cleaning up container workspace and context directories");

    // 1. Preserve context files generated during execution.
    // Do not bulk-delete /workspace/context, because users may need those artifacts
    // (e.g. extracted files like /workspace/context/pdf_content.txt) after completion.

    // 2. If execution_id provided, also remove that specific history file
    if let Some(id) = execution_id {
        if normalize_execution_scope_id(Some(id)).is_some() {
            let history_file = get_history_path(CONTAINER_CONTEXT_DIR, Some(id));
            let rm_history_cmd = format!("rm -f {} 2>/dev/null || true", history_file);
            let _ = sandbox.execute(&rm_history_cmd, 5).await;
            tracing::info!("Removed execution-specific history file: {}", history_file);
        }
    }

    // 2. Clean up temporary files in /workspace root (preserve essential directories)
    // Keep both context/ and uploads/ so uploaded files and generated context artifacts survive.
    let cleanup_workspace_cmd = r#"
        cd /workspace 2>/dev/null && \
        find . -maxdepth 1 -type f -delete 2>/dev/null || true && \
        find . -maxdepth 1 -type d ! -name '.' ! -name 'context' ! -name 'uploads' -exec rm -rf {} + 2>/dev/null || true
    "#;

    // Execute cleanup command
    sandbox
        .execute(cleanup_workspace_cmd, 15)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to cleanup workspace directory: {}", e))?;

    tracing::info!("Container workspace cleanup completed");
    Ok(())
}

/// Clean up host context files for a specific execution
pub fn cleanup_host_context_with_id(execution_id: Option<&str>) -> anyhow::Result<()> {
    let context_dir = get_host_context_dir();

    if !context_dir.exists() {
        return Ok(());
    }

    match execution_id {
        Some(id) if normalize_execution_scope_id(Some(id)).is_some() => {
            let context_dir_str = context_dir.display().to_string();
            let history_path =
                std::path::PathBuf::from(get_history_path(&context_dir_str, Some(id)));
            if history_path.exists() {
                let _ = std::fs::remove_file(&history_path);
                tracing::info!(
                    "Removed execution-specific history file: {}",
                    history_path.display()
                );
            }
        }
        _ => {
            let legacy_history_path = context_dir.join(history_filename());
            if legacy_history_path.exists() {
                let _ = std::fs::remove_file(&legacy_history_path);
                tracing::info!(
                    "Removed legacy history file: {}",
                    legacy_history_path.display()
                );
            }
        }
    }

    tracing::info!("Host context cleanup completed");
    Ok(())
}

/// Clean up workspace scratch files while preserving stored context artifacts
///
/// Alternative cleanup option that does the same as cleanup_container_context
/// but uses a single find command for efficiency
pub async fn cleanup_container_workspace_full(sandbox: &DockerSandbox) -> anyhow::Result<()> {
    tracing::info!("Full cleanup of container workspace (alternative method)");

    // Preserve the full context/ and uploads/ trees so per-session artifacts remain available.
    let cleanup_cmd = r#"
        cd /workspace 2>/dev/null && \
        find . -mindepth 1 \
            ! -path './context' ! -path './context/*' \
            ! -path './uploads' ! -path './uploads/*' \
            -delete 2>/dev/null || true
    "#;

    sandbox
        .execute(cleanup_cmd, 15)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to full cleanup workspace: {}", e))?;

    tracing::info!("Full container workspace cleanup completed");
    Ok(())
}

/// Store large output to host filesystem
/// Returns StorageResult with host file path
pub async fn store_output_on_host(
    tool_name: &str,
    output: &str,
    call_id: Option<&str>,
    execution_id: Option<&str>,
) -> anyhow::Result<StorageResult> {
    // Check size against configured threshold
    let threshold = get_storage_threshold();
    if output.len() <= threshold {
        return Ok(StorageResult::Direct(output.to_string()));
    }

    // Create context directory on host
    let context_dir = get_host_execution_context_dir(execution_id);
    std::fs::create_dir_all(&context_dir)
        .map_err(|e| anyhow::anyhow!("Failed to create context directory: {}", e))?;

    // Generate filename
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let call_suffix = call_id
        .map(|id| format!("_{}", &id[..8.min(id.len())]))
        .unwrap_or_default();
    let filename = format!("{}_{}{}.txt", tool_name, timestamp, call_suffix);
    let host_path = context_dir.join(&filename);

    // Write output to file
    std::fs::write(&host_path, output)
        .map_err(|e| anyhow::anyhow!("Failed to write output to host file: {}", e))?;

    let lines = output.lines().count();
    let size = output.len();

    // Generate preview (first 500 chars)
    let preview = output.chars().take(500).collect::<String>();
    let preview_end = if output.len() > 500 { "\n..." } else { "" };

    let host_path_str = host_path.display().to_string();

    let readback_protocol = build_host_readback_protocol(&host_path_str, lines);

    // Create summary with instructions for host-based file access
    let summary = format!(
        r#"[Large Output Stored to Host File]
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
| Host Path: {}
| Size: {} bytes ({:.2} KB)
| Lines: {}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Preview (first 500 chars):
{}{}

{}

All context files are in: {}
"#,
        host_path_str,
        size,
        size as f64 / 1024.0,
        lines,
        preview,
        preview_end,
        readback_protocol,
        context_dir.display()
    );

    Ok(StorageResult::StoredHost {
        host_path: host_path_str,
        summary,
        size,
        lines,
    })
}

/// Store large output to container (unified for all tools)
/// Automatically creates a Docker sandbox if needed, falls back to host storage if Docker unavailable
/// Returns StorageResult with container file path or host file path
pub async fn store_output_unified(
    tool_name: &str,
    output: &str,
    call_id: Option<&str>,
    execution_id: Option<&str>,
) -> anyhow::Result<StorageResult> {
    // Check size against configured threshold
    let threshold = get_storage_threshold();
    if output.len() <= threshold {
        return Ok(StorageResult::Direct(output.to_string()));
    }

    // Respect the user's configured execution mode: only use container
    // storage when the execution mode is explicitly set to Docker AND
    // Docker is actually available.
    use crate::shell::get_shell_config;
    let shell_config = get_shell_config().await;
    let docker_mode = shell_config.default_execution_mode
        == crate::shell::ShellExecutionMode::Docker
        && shell_config.docker_config.is_some()
        && DockerSandbox::is_docker_available().await;

    if !docker_mode {
        tracing::debug!(
            "Execution mode is host or Docker unavailable, using host filesystem storage"
        );
        return store_output_on_host(tool_name, output, call_id, execution_id).await;
    }

    let docker_config = shell_config.docker_config.unwrap_or_default();
    let sandbox = DockerSandbox::new(docker_config);

    // Try container storage, fallback to host if it fails
    match store_output_in_container(&sandbox, tool_name, output, call_id, execution_id).await {
        Ok(result) => Ok(result),
        Err(e) => {
            tracing::warn!(
                "Container storage failed ({}), falling back to host storage",
                e
            );
            store_output_on_host(tool_name, output, call_id, execution_id).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn container_protocol_requires_sequential_readback() {
        let protocol = build_container_readback_protocol("/workspace/context/output.txt", 480);

        assert!(protocol.contains("Read the file sequentially in bounded chunks"));
        assert!(protocol.contains("sed -n '1,200p' /workspace/context/output.txt"));
        assert!(protocol.contains("only inspected part of the file"));
    }

    #[test]
    fn host_protocol_prefers_file_read() {
        let protocol = build_host_readback_protocol("/tmp/output.txt", 380);

        assert!(protocol.contains("Prefer `file_read`"));
        assert!(protocol.contains("\"offset\": 1, \"limit\": 200"));
        assert!(protocol.contains("end_line == total_lines"));
    }

    #[test]
    fn container_protocol_formats_single_line_json_before_chunking() {
        let plan = build_container_readback_plan(
            "/workspace/context/output.txt",
            r#"{"openapi":"3.1.0","info":{"title":"demo"},"paths":{"/health":{"get":{"responses":{"200":{"description":"ok"}}}}}}"#,
        );

        assert!(plan.reported_lines > 1);
        assert!(plan.lines_summary.contains("formatted JSON lines"));
        assert!(plan.protocol.contains("minified single-line JSON"));
        assert!(plan
            .protocol
            .contains("python3 -m json.tool /workspace/context/output.txt | sed -n '1,"));
    }

    #[test]
    fn stored_result_exposes_structured_artifact_metadata() {
        let artifact = StorageResult::Stored {
            container_path: "/workspace/context/output.txt".to_string(),
            summary: "stored".to_string(),
            size: 4096,
            lines: 320,
        }
        .to_stored_artifact("stdout")
        .expect("artifact metadata");

        assert_eq!(artifact.slot, "stdout");
        assert_eq!(artifact.path, "/workspace/context/output.txt");
        assert_eq!(artifact.storage_backend, "container");
        assert_eq!(artifact.size, 4096);
        assert_eq!(artifact.lines, 320);
    }

    #[test]
    fn execution_context_dir_is_scoped_by_execution_id() {
        assert_eq!(
            get_execution_context_dir("/workspace/context", Some("exec-1234567890abcdef")),
            "/workspace/context/session_exec-1234567"
        );
    }

    #[test]
    fn history_path_uses_session_directory() {
        assert_eq!(
            get_history_path("/workspace/context", Some("exec-1234567890abcdef")),
            "/workspace/context/session_exec-1234567/history.txt"
        );
    }
}
