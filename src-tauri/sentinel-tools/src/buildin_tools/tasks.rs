//! Tasks tool for autonomous agent planning and tracking
//! Supports database persistence for session recovery

use once_cell::sync::Lazy;
use rig::tool::Tool;
use schemars::JsonSchema;
use sentinel_db::DatabaseService;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::RwLock;

/// Task status
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Todo is waiting to be started
    Pending,
    /// Todo is currently being worked on
    InProgress,
    /// Todo has been successfully completed
    Completed,
    /// Todo has failed
    Failed,
}

impl TaskStatus {
    fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::InProgress => "in_progress",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
        }
    }
}

impl From<&str> for TaskStatus {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "pending" => TaskStatus::Pending,
            "in_progress" => TaskStatus::InProgress,
            "completed" => TaskStatus::Completed,
            "failed" => TaskStatus::Failed,
            _ => TaskStatus::Pending,
        }
    }
}

impl From<sentinel_db::ExecutionTaskStatus> for TaskStatus {
    fn from(status: sentinel_db::ExecutionTaskStatus) -> Self {
        match status {
            sentinel_db::ExecutionTaskStatus::Pending => TaskStatus::Pending,
            sentinel_db::ExecutionTaskStatus::InProgress => TaskStatus::InProgress,
            sentinel_db::ExecutionTaskStatus::Completed => TaskStatus::Completed,
            sentinel_db::ExecutionTaskStatus::Failed => TaskStatus::Failed,
        }
    }
}

impl From<TaskStatus> for sentinel_db::ExecutionTaskStatus {
    fn from(status: TaskStatus) -> Self {
        match status {
            TaskStatus::Pending => sentinel_db::ExecutionTaskStatus::Pending,
            TaskStatus::InProgress => sentinel_db::ExecutionTaskStatus::InProgress,
            TaskStatus::Completed => sentinel_db::ExecutionTaskStatus::Completed,
            TaskStatus::Failed => sentinel_db::ExecutionTaskStatus::Failed,
        }
    }
}

/// A single task item
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TaskItem {
    /// Description of the task
    pub description: String,
    /// Current status of the task
    pub status: TaskStatus,
    /// Optional result or observation from the task
    pub result: Option<String>,
}

/// The overall tasks list
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TasksList {
    /// List of tasks
    pub items: Vec<TaskItem>,
    /// Index of the current task being executed
    pub current_index: Option<usize>,
}

impl TasksList {
    /// Calculate current_index from items
    fn recalculate_current_index(&mut self) {
        self.current_index = self
            .items
            .iter()
            .position(|item| item.status == TaskStatus::InProgress);
    }
}

/// In-memory cache for tasks (synced with database)
static TASKS_CACHE: Lazy<Arc<RwLock<HashMap<String, TasksList>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

/// Global AppHandle for emitting events and database access
static APP_HANDLE: Lazy<RwLock<Option<AppHandle>>> = Lazy::new(|| RwLock::new(None));

/// Set global AppHandle for tasks
pub async fn set_tasks_app_handle(handle: AppHandle) {
    let mut h = APP_HANDLE.write().await;
    *h = Some(handle);
}

/// Get database service from AppHandle
async fn get_db_service() -> Option<Arc<DatabaseService>> {
    let handle = APP_HANDLE.read().await;
    if let Some(ref h) = *handle {
        h.try_state::<Arc<DatabaseService>>()
            .map(|s| s.inner().clone())
    } else {
        None
    }
}

/// Load tasks from database into memory cache
async fn load_tasks_from_db(execution_id: &str) -> Option<TasksList> {
    let db = get_db_service().await?;
    match db.get_execution_tasks(execution_id).await {
        Ok(db_items) if !db_items.is_empty() => {
            let items: Vec<TaskItem> = db_items
                .into_iter()
                .map(|item| TaskItem {
                    description: item.description,
                    status: TaskStatus::from(item.status.as_str()),
                    result: item.result,
                })
                .collect();

            let mut list = TasksList {
                items,
                current_index: None,
            };
            list.recalculate_current_index();

            // Update cache
            let mut cache = TASKS_CACHE.write().await;
            cache.insert(execution_id.to_string(), list.clone());

            tracing::info!(
                "Loaded {} tasks from database for execution {}",
                list.items.len(),
                execution_id
            );
            Some(list)
        }
        Ok(_) => None,
        Err(e) => {
            tracing::warn!("Failed to load tasks from database: {}", e);
            None
        }
    }
}

/// Save tasks to database
async fn save_tasks_to_db(execution_id: &str, list: &TasksList) {
    if let Some(db) = get_db_service().await {
        let items: Vec<sentinel_db::ExecutionTaskInput> = list
            .items
            .iter()
            .map(|item| sentinel_db::ExecutionTaskInput {
                description: item.description.clone(),
                status: item.status.clone().into(),
                result: item.result.clone(),
            })
            .collect();

        if let Err(e) = db.save_execution_tasks(execution_id, &items).await {
            tracing::warn!("Failed to save tasks to database: {}", e);
        } else {
            tracing::debug!(
                "Saved {} tasks to database for execution {}",
                items.len(),
                execution_id
            );
        }
    }
}

/// Delete tasks from database
async fn delete_tasks_from_db(execution_id: &str) {
    if let Some(db) = get_db_service().await {
        if let Err(e) = db.delete_execution_tasks(execution_id).await {
            tracing::warn!("Failed to delete tasks from database: {}", e);
        }
    }
}

/// Get or load tasks list for an execution
async fn get_or_load_tasks(execution_id: &str) -> TasksList {
    // Check cache first
    {
        let cache = TASKS_CACHE.read().await;
        if let Some(list) = cache.get(execution_id) {
            return list.clone();
        }
    }

    // Try to load from database
    if let Some(list) = load_tasks_from_db(execution_id).await {
        return list;
    }

    // Return empty list
    TasksList::default()
}

/// Tasks tool arguments
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct TasksArgs {
    /// The execution ID of the current agent run
    pub execution_id: String,
    /// Optional explanation for the plan update
    pub explanation: Option<String>,
    /// Complete plan for the current execution
    pub plan: Vec<TaskItem>,
}

/// Tasks tool output
#[derive(Debug, Clone, Serialize)]
pub struct TasksOutput {
    pub success: bool,
    pub list: Option<TasksList>,
    pub message: String,
}

/// Tasks tool errors
#[derive(Debug, thiserror::Error)]
pub enum TasksError {
    #[error("Invalid plan: {0}")]
    InvalidPlan(String),
    #[error("Internal error: {0}")]
    InternalError(String),
}

/// Tasks tool
#[derive(Debug, Clone, Default)]
pub struct TasksTool;

impl TasksTool {
    pub fn new() -> Self {
        Self
    }

    pub const NAME: &'static str = "tasks";
    pub const DESCRIPTION: &'static str = concat!(
        "Updates the current execution plan shown in the UI. ",
        "Submit the complete desired plan each time using the plan array; omitted old steps are removed from the displayed plan. ",
        "Use pending for steps not started, in_progress for the one active step, and completed for finished steps. ",
        "At most one step may be in_progress. ",
        "This tool is a plan display/event stream, not a completion gate; normal agent execution success, cancellation, or errors determine the run outcome."
    );
}

impl Tool for TasksTool {
    const NAME: &'static str = Self::NAME;
    type Args = TasksArgs;
    type Output = TasksOutput;
    type Error = TasksError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(TasksArgs)).unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let execution_id = args.execution_id.clone();

        let in_progress_count = args
            .plan
            .iter()
            .filter(|item| item.status == TaskStatus::InProgress)
            .count();
        if in_progress_count > 1 {
            return Err(TasksError::InvalidPlan(
                "at most one item may be in_progress".to_string(),
            ));
        }

        let mut list = TasksList {
            items: args.plan,
            current_index: None,
        };
        list.recalculate_current_index();

        {
            let mut cache = TASKS_CACHE.write().await;
            cache.insert(execution_id.clone(), list.clone());
        }
        save_tasks_to_db(&execution_id, &list).await;

        let result = Ok(TasksOutput {
            success: true,
            list: Some(list.clone()),
            message: args
                .explanation
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| format!("Plan updated with {} item(s)", list.items.len())),
        });

        // Emit events for UI synchronization.
        if let Ok(ref output) = result {
            if let Some(ref list) = output.list {
                if let Some(handle) = &*APP_HANDLE.read().await {
                    let _ = handle.emit(
                        "agent:plan_updated",
                        serde_json::json!({
                            "execution_id": execution_id,
                            "plan": {
                                "tasks": list.items,
                                "current_task_index": list.current_index
                            }
                        }),
                    );

                    let tasks_json: Vec<serde_json::Value> = list
                        .items
                        .iter()
                        .enumerate()
                        .map(|(i, item)| {
                            serde_json::json!({
                                "id": format!("{}_{}", execution_id, i),
                                "content": item.description,
                                "status": item.status.as_str(),
                                "created_at": chrono::Utc::now().timestamp_millis(),
                                "updated_at": chrono::Utc::now().timestamp_millis(),
                                "metadata": {
                                    "step_index": i,
                                    "result": item.result
                                }
                            })
                        })
                        .collect();

                    let _ = handle.emit(
                        "agent-tasks-update",
                        serde_json::json!({
                            "execution_id": execution_id,
                            "tasks": tasks_json,
                            "timestamp": chrono::Utc::now().timestamp_millis()
                        }),
                    );
                }
            }
        }

        result
    }
}

/// Helper function to get tasks list for an execution
pub async fn get_execution_tasks(execution_id: &str) -> Option<TasksList> {
    let list = get_or_load_tasks(execution_id).await;
    if list.items.is_empty() {
        None
    } else {
        Some(list)
    }
}

/// Helper function to cleanup tasks list for an execution
pub async fn cleanup_execution_tasks(execution_id: &str) -> bool {
    let mut cache = TASKS_CACHE.write().await;
    let removed = cache.remove(execution_id).is_some();
    drop(cache);

    delete_tasks_from_db(execution_id).await;
    removed
}

/// Helper function to drop a cached task list without deleting persisted history.
pub async fn invalidate_execution_tasks(execution_id: &str) {
    let mut cache = TASKS_CACHE.write().await;
    cache.remove(execution_id);
}

/// Helper function to cleanup all task lists (cache only, not database)
pub async fn cleanup_all_tasks() {
    let mut cache = TASKS_CACHE.write().await;
    cache.clear();
}

#[cfg(test)]
mod tests {
    use super::{TaskItem, TaskStatus, TasksArgs, TasksError, TasksTool};
    use rig::tool::Tool;

    #[tokio::test]
    async fn tasks_tool_replaces_display_plan_with_complete_plan() {
        let output = TasksTool::new()
            .call(TasksArgs {
                execution_id: "test-plan-replace".to_string(),
                explanation: Some("testing plan update".to_string()),
                plan: vec![
                    TaskItem {
                        description: "inspect".to_string(),
                        status: TaskStatus::Completed,
                        result: Some("done".to_string()),
                    },
                    TaskItem {
                        description: "patch".to_string(),
                        status: TaskStatus::InProgress,
                        result: None,
                    },
                ],
            })
            .await
            .expect("plan update should succeed");

        let list = output.list.expect("list");
        assert_eq!(list.items.len(), 2);
        assert_eq!(list.current_index, Some(1));
        assert_eq!(output.message, "testing plan update");
    }

    #[tokio::test]
    async fn tasks_tool_rejects_multiple_in_progress_items() {
        let error = TasksTool::new()
            .call(TasksArgs {
                execution_id: "test-plan-invalid".to_string(),
                explanation: None,
                plan: vec![
                    TaskItem {
                        description: "one".to_string(),
                        status: TaskStatus::InProgress,
                        result: None,
                    },
                    TaskItem {
                        description: "two".to_string(),
                        status: TaskStatus::InProgress,
                        result: None,
                    },
                ],
            })
            .await
            .expect_err("invalid plan should fail");

        assert!(matches!(error, TasksError::InvalidPlan(_)));
    }
}
