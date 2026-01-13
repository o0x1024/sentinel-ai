//! Tool Output Storage Module
//! 
//! Stores large tool outputs to files in Docker container for on-demand retrieval
//! All tools (HTTP, Shell, etc.) use unified container storage: /workspace/context/

use serde::{Serialize, Deserialize};
use crate::docker_sandbox::DockerSandbox;

/// Default storage threshold (10KB)
const DEFAULT_STORAGE_THRESHOLD: usize = 10_000;

use std::sync::RwLock;
use once_cell::sync::Lazy;

/// Global storage threshold configuration
static STORAGE_THRESHOLD: Lazy<RwLock<usize>> = Lazy::new(|| RwLock::new(DEFAULT_STORAGE_THRESHOLD));

/// Set storage threshold
pub fn set_storage_threshold(threshold: usize) {
    if let Ok(mut t) = STORAGE_THRESHOLD.write() {
        *t = threshold;
    }
}

/// Get storage threshold
pub fn get_storage_threshold() -> usize {
    STORAGE_THRESHOLD.read().map(|t| *t).unwrap_or(DEFAULT_STORAGE_THRESHOLD)
}

/// Container context directory (unified for all tools)
pub const CONTAINER_CONTEXT_DIR: &str = "/workspace/context";

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

/// Generate platform-specific file access commands
fn generate_file_access_commands(file_path: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        format!(
            r#"   • Get-Content "{}" | Select-String "pattern"  (search for pattern)
   • Get-Content "{}" -Tail 50                   (view last 50 lines)
   • Get-Content "{}" -Head 50                   (view first 50 lines)
   • Get-Content "{}"                            (view full content)
   • (Get-Content "{}").Count                    (count lines)
   • type "{}"                                   (view full content - cmd)
   • find /c /v "" "{}"                          (count lines - cmd)"#,
            file_path, file_path, file_path, file_path, file_path, file_path, file_path
        )
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        format!(
            r#"   • grep -i "pattern" "{}"     (search for pattern)
   • tail -n 50 "{}"             (view last 50 lines)  
   • head -n 50 "{}"             (view first 50 lines)
   • cat "{}"                    (view full content)
   • wc -l "{}"                  (count lines)"#,
            file_path, file_path, file_path, file_path, file_path
        )
    }
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
}

/// Store large output in Docker container
/// Returns StorageResult with container file path
pub async fn store_output_in_container(
    sandbox: &DockerSandbox,
    tool_name: &str,
    output: &str,
    call_id: Option<&str>,
) -> anyhow::Result<StorageResult> {
    // Check size against configured threshold
    let threshold = get_storage_threshold();
    if output.len() <= threshold {
        return Ok(StorageResult::Direct(output.to_string()));
    }

    // Generate filename
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let call_suffix = call_id.map(|id| format!("_{}", &id[..8.min(id.len())])).unwrap_or_default();
    let filename = format!("{}_{}{}.txt", tool_name, timestamp, call_suffix);
    let container_path = format!("{}/{}", CONTAINER_CONTEXT_DIR, filename);

    // Create context directory in container if not exists
    let mkdir_cmd = format!("mkdir -p {}", CONTAINER_CONTEXT_DIR);
    let _ = sandbox.execute(&mkdir_cmd, 5).await;

    // Write output to file in container using echo with base64 encoding to avoid quote issues
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(output);
    let write_cmd = format!(
        "echo '{}' | base64 -d > {}",
        encoded,
        container_path
    );
    
    sandbox.execute(&write_cmd, 30).await
        .map_err(|e| anyhow::anyhow!("Failed to write output to container: {}", e))?;

    let lines = output.lines().count();
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

To access the full content in container, use shell tool with:
   • grep -i "pattern" {}     (search for pattern)
   • tail -n 50 {}             (view last 50 lines)  
   • head -n 50 {}             (view first 50 lines)
   • cat {}                    (view full content)
   • wc -l {}                  (count lines)

All context files are in: {}
"#,
        container_path,
        size,
        size as f64 / 1024.0,
        lines,
        preview,
        preview_end,
        container_path,
        container_path,
        container_path,
        container_path,
        container_path,
        CONTAINER_CONTEXT_DIR
    );

    Ok(StorageResult::Stored {
        container_path,
        summary,
        size,
        lines,
    })
}

/// Store conversation history in container
pub async fn store_history_in_container(
    sandbox: &DockerSandbox,
    history_content: &str,
) -> anyhow::Result<String> {
    let container_path = format!("{}/history.txt", CONTAINER_CONTEXT_DIR);

    // Create context directory
    let mkdir_cmd = format!("mkdir -p {}", CONTAINER_CONTEXT_DIR);
    let _ = sandbox.execute(&mkdir_cmd, 5).await;

    // Write history using base64 encoding
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(history_content);
    let write_cmd = format!(
        "echo '{}' | base64 -d > {}",
        encoded,
        container_path
    );
    
    sandbox.execute(&write_cmd, 30).await
        .map_err(|e| anyhow::anyhow!("Failed to write history to container: {}", e))?;

    Ok(container_path)
}

/// Initialize context directory in container
pub async fn init_container_context(sandbox: &DockerSandbox) -> anyhow::Result<()> {
    let mkdir_cmd = format!("mkdir -p {}", CONTAINER_CONTEXT_DIR);
    sandbox.execute(&mkdir_cmd, 5).await
        .map_err(|e| anyhow::anyhow!("Failed to create context directory: {}", e))?;
    Ok(())
}

/// Clean up context files in container
/// Should be called when execution completes
/// 
/// Removes:
/// - Tool output files: http_response_*.txt, shell_stdout_*.txt, shell_stderr_*.txt
/// - Temporary files in /workspace root: *.py, *.js, *.php, *.txt, etc.
/// - Temporary directories in /workspace (except context/)
/// 
/// Preserves:
/// - /workspace/context/history.txt (conversation history for search)
pub async fn cleanup_container_context(sandbox: &DockerSandbox) -> anyhow::Result<()> {
    tracing::info!("Cleaning up container workspace and context directories");
    
    // 1. Remove all tool output files in /workspace/context except history.txt
    // This includes: http_response_*.txt, shell_stdout_*.txt, shell_stderr_*.txt
    let cleanup_context_cmd = format!(
        "find {} -type f ! -name 'history.txt' -delete 2>/dev/null || true",
        CONTAINER_CONTEXT_DIR
    );
    
    // 2. Clean up temporary files in /workspace root (preserve essential directories)
    // Remove files but keep directories like /workspace/context, /workspace/src (if any)
    let cleanup_workspace_cmd = r#"
        cd /workspace 2>/dev/null && \
        find . -maxdepth 1 -type f -delete 2>/dev/null || true && \
        find . -maxdepth 1 -type d ! -name '.' ! -name 'context' -exec rm -rf {} + 2>/dev/null || true
    "#;
    
    // Execute cleanup commands
    sandbox.execute(&cleanup_context_cmd, 10).await
        .map_err(|e| anyhow::anyhow!("Failed to cleanup context directory: {}", e))?;
    
    sandbox.execute(cleanup_workspace_cmd, 15).await
        .map_err(|e| anyhow::anyhow!("Failed to cleanup workspace directory: {}", e))?;
    
    tracing::info!("Container workspace cleanup completed");
    Ok(())
}

/// Clean up all workspace files including context and tool outputs
/// Use with caution - removes everything in /workspace except context/history.txt
/// 
/// Alternative cleanup option that does the same as cleanup_container_context
/// but uses a single find command for efficiency
pub async fn cleanup_container_workspace_full(sandbox: &DockerSandbox) -> anyhow::Result<()> {
    tracing::info!("Full cleanup of container workspace (alternative method)");
    
    // Remove everything except context/history.txt
    // This includes all tool output files and temporary files
    let cleanup_cmd = r#"
        cd /workspace 2>/dev/null && \
        find . ! -path './context/history.txt' ! -path './context' ! -path '.' -delete 2>/dev/null || true
    "#;
    
    sandbox.execute(cleanup_cmd, 15).await
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
) -> anyhow::Result<StorageResult> {
    // Check size against configured threshold
    let threshold = get_storage_threshold();
    if output.len() <= threshold {
        return Ok(StorageResult::Direct(output.to_string()));
    }

    // Create context directory on host
    let context_dir = get_host_context_dir();
    std::fs::create_dir_all(&context_dir)
        .map_err(|e| anyhow::anyhow!("Failed to create context directory: {}", e))?;

    // Generate filename
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let call_suffix = call_id.map(|id| format!("_{}", &id[..8.min(id.len())])).unwrap_or_default();
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
    
    // Generate platform-specific command suggestions
    let access_commands = generate_file_access_commands(&host_path_str);
    
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

To access the full content on host, use shell tool with:
{}

All context files are in: {}
"#,
        host_path_str,
        size,
        size as f64 / 1024.0,
        lines,
        preview,
        preview_end,
        access_commands,
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
) -> anyhow::Result<StorageResult> {
    // Check size against configured threshold
    let threshold = get_storage_threshold();
    if output.len() <= threshold {
        return Ok(StorageResult::Direct(output.to_string()));
    }

    // Check if Docker is available
    if !DockerSandbox::is_docker_available().await {
        tracing::debug!("Docker not available, using host filesystem storage");
        return store_output_on_host(tool_name, output, call_id).await;
    }

    // Try to use container storage
    use crate::shell::get_shell_config;
    let shell_config = get_shell_config().await;
    let docker_config = shell_config.docker_config.unwrap_or_default();
    let sandbox = DockerSandbox::new(docker_config);

    // Try container storage, fallback to host if it fails
    match store_output_in_container(&sandbox, tool_name, output, call_id).await {
        Ok(result) => Ok(result),
        Err(e) => {
            tracing::warn!("Container storage failed ({}), falling back to host storage", e);
            store_output_on_host(tool_name, output, call_id).await
        }
    }
}
