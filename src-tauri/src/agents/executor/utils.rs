//! Executor utility helpers.
use std::sync::Arc;
use sentinel_db::Database;
use tauri::{AppHandle, Manager};

/// Truncate text for compact memory summaries.
pub fn truncate_for_memory(text: &str, max_len: usize) -> String {
    if text.chars().count() <= max_len {
        return text.to_string();
    }
    let head: String = text.chars().take(max_len).collect();
    format!("{}... [truncated]", head)
}

/// Cleanup container workspace files asynchronously (non-blocking).
/// Removes temporary files created during task execution in /workspace.
/// Preserves conversation history at /workspace/context/history.txt.
pub async fn cleanup_container_context_async(app_handle: &AppHandle, execution_id: &str) {
    let auto_cleanup_enabled = match app_handle.try_state::<Arc<crate::services::database::DatabaseService>>() {
        Some(db) => match db.get_config("agent", "workspace_auto_cleanup_enabled").await {
            Ok(Some(v)) => v == "1" || v.eq_ignore_ascii_case("true"),
            _ => false,
        },
        None => false,
    };

    if !auto_cleanup_enabled {
        tracing::info!(
            "Skipping container workspace cleanup for execution {} because upload auto-cleanup is disabled",
            execution_id
        );
        return;
    }

    tracing::info!("Starting container workspace cleanup for execution: {}", execution_id);

    let execution_id = execution_id.to_string();
    tokio::spawn(async move {
        use sentinel_tools::shell::get_shell_config;

        match get_shell_config().await.docker_config {
            Some(docker_config) => {
                let sandbox = sentinel_tools::DockerSandbox::new(docker_config);

                match sentinel_tools::cleanup_container_context(&sandbox).await {
                    Ok(_) => {
                        tracing::info!(
                            "Container workspace cleanup completed for execution: {}",
                            execution_id
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to cleanup container workspace for execution {}: {}",
                            execution_id,
                            e
                        );
                    }
                }
            }
            None => {
                tracing::debug!("No Docker config, skipping container workspace cleanup");
            }
        }
    });
}
