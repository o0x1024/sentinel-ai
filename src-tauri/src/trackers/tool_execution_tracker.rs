use anyhow::Result;
use sentinel_db::DatabaseService;
use sentinel_db::models::task_tool::*;
use std::sync::{Arc, OnceLock};
use tauri::{AppHandle, Emitter};
use tracing::{debug, error, info};

/// Tool execution tracker for monitoring and recording tool usage
pub struct ToolExecutionTracker {
    db: Arc<DatabaseService>,
    app_handle: AppHandle,
}

impl ToolExecutionTracker {
    pub fn new(db: Arc<DatabaseService>, app_handle: AppHandle) -> Self {
        Self { db, app_handle }
    }

    /// Start tracking a tool execution
    pub async fn track_start(
        &self,
        task_id: String,
        tool_id: String,
        tool_name: String,
        tool_type: ToolType,
        input_params: Option<serde_json::Value>,
    ) -> Result<String> {
        info!(
            "Tracking tool execution start - task: {}, tool: {} ({})",
            task_id, tool_name, tool_type.to_string()
        );

        // Record in database
        let log_id = self
            .db
            .record_tool_execution_start(
                task_id.clone(),
                tool_id.clone(),
                tool_name.clone(),
                tool_type.clone(),
                input_params,
            )
            .await?;

        // Emit event to frontend
        let _ = self.app_handle.emit(
            "task:tool:started",
            &serde_json::json!({
                "task_id": task_id,
                "tool_id": tool_id,
                "tool_name": tool_name,
                "tool_type": tool_type.to_string(),
                "log_id": log_id,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
        );

        Ok(log_id)
    }

    /// Complete tracking a tool execution
    pub async fn track_complete(
        &self,
        log_id: String,
        task_id: String,
        tool_id: String,
        success: bool,
        output_result: Option<serde_json::Value>,
        error_message: Option<String>,
    ) -> Result<()> {
        debug!(
            "Tracking tool execution complete - log_id: {}, success: {}",
            log_id, success
        );

        // Record in database
        self.db
            .record_tool_execution_complete(
                log_id.clone(),
                success,
                output_result.clone(),
                error_message.clone(),
            )
            .await?;

        // Get updated statistics
        let statistics = self.db.get_task_tool_statistics(task_id.clone()).await?;
        let active_tools = self.db.get_task_active_tools(task_id.clone()).await?;

        // Emit completion event
        let _ = self.app_handle.emit(
            "task:tool:completed",
            &serde_json::json!({
                "task_id": task_id,
                "tool_id": tool_id,
                "log_id": log_id,
                "success": success,
                "error_message": error_message,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
        );

        // Emit status changed event with updated statistics
        let _ = self.app_handle.emit(
            "task:tool:status_changed",
            &serde_json::json!({
                "task_id": task_id,
                "active_tools": active_tools,
                "statistics": statistics,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
        );

        Ok(())
    }

    /// Track a failed tool execution
    pub async fn track_error(
        &self,
        log_id: String,
        task_id: String,
        tool_id: String,
        error_message: String,
    ) -> Result<()> {
        error!(
            "Tracking tool execution error - log_id: {}, error: {}",
            log_id, error_message
        );

        self.track_complete(
            log_id.clone(),
            task_id.clone(),
            tool_id.clone(),
            false,
            None,
            Some(error_message.clone()),
        )
        .await?;

        // Emit error event
        let _ = self.app_handle.emit(
            "task:tool:failed",
            &serde_json::json!({
                "task_id": task_id,
                "tool_id": tool_id,
                "log_id": log_id,
                "error_message": error_message,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
        );

        Ok(())
    }

    /// Get current task tool statistics
    pub async fn get_statistics(&self, task_id: String) -> Result<ToolStatistics> {
        self.db.get_task_tool_statistics(task_id).await
    }

    /// Get active tools for a task
    pub async fn get_active_tools(&self, task_id: String) -> Result<Vec<ActiveToolInfo>> {
        self.db.get_task_active_tools(task_id).await
    }

    /// Get execution history
    pub async fn get_execution_history(
        &self,
        task_id: String,
        tool_id: Option<String>,
        limit: Option<i64>,
    ) -> Result<Vec<ExecutionRecord>> {
        self.db
            .get_tool_execution_history(task_id, tool_id, limit)
            .await
    }
}

/// Global tool execution tracker instance
static TRACKER_INSTANCE: OnceLock<Arc<ToolExecutionTracker>> = OnceLock::new();

/// Initialize the global tracker
pub fn init_tracker(db: Arc<DatabaseService>, app_handle: AppHandle) {
    let tracker = Arc::new(ToolExecutionTracker::new(db, app_handle));
    let _ = TRACKER_INSTANCE.set(tracker);
    info!("Tool execution tracker initialized");
}

/// Get the global tracker instance
pub fn get_tracker() -> Option<Arc<ToolExecutionTracker>> {
    TRACKER_INSTANCE.get().cloned()
}

/// Helper macro for tracking tool execution
#[macro_export]
macro_rules! track_tool_execution {
    ($task_id:expr, $tool_id:expr, $tool_name:expr, $tool_type:expr, $input:expr, $body:expr) => {{
        use $crate::trackers::tool_execution_tracker::get_tracker;
        
        let tracker = get_tracker();
        let log_id = if let Some(ref t) = tracker {
            match t.track_start(
                $task_id.clone(),
                $tool_id.clone(),
                $tool_name.clone(),
                $tool_type,
                $input,
            ).await {
                Ok(id) => Some(id),
                Err(e) => {
                    tracing::error!("Failed to track tool start: {}", e);
                    None
                }
            }
        } else {
            None
        };

        let result = $body.await;

        if let (Some(ref t), Some(ref lid)) = (&tracker, &log_id) {
            match &result {
                Ok(output) => {
                    let _ = t.track_complete(
                        lid.clone(),
                        $task_id.clone(),
                        $tool_id.clone(),
                        true,
                        Some(serde_json::json!(output)),
                        None,
                    ).await;
                }
                Err(e) => {
                    let _ = t.track_error(
                        lid.clone(),
                        $task_id.clone(),
                        $tool_id.clone(),
                        e.to_string(),
                    ).await;
                }
            }
        }

        result
    }};
}
