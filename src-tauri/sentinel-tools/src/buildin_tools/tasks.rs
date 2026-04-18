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
    fn is_terminal(&self) -> bool {
        matches!(self, TaskStatus::Completed | TaskStatus::Failed)
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
    /// The action to perform: "add_items", "update_status", "get_list", "reset", "replan", "update_item", "delete_item", "insert_item", "cleanup"
    pub action: String,
    /// Items to add (required for "add_items", "replan")
    pub items: Option<Vec<String>>,
    /// Index of the item to update/delete/insert (required for "update_status", "update_item", "delete_item", "insert_item")
    pub item_index: Option<usize>,
    /// New status for the item (required for "update_status")
    pub status: Option<TaskStatus>,
    /// Optional result or observation to record
    pub result: Option<String>,
    /// New item description (required for "update_item", "insert_item")
    pub new_description: Option<String>,
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
    #[error("Missing required parameters for action {0}")]
    MissingParameters(String),
    #[error("Item index {0} out of bounds")]
    IndexOutOfBounds(usize),
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
        "Persistent task tracker for the current agent execution. ",
        "Use it to create or revise a multi-step plan, mark progress, record step results, ",
        "and recover existing tasks across sessions. Call action='get_list' before creating new items ",
        "to avoid duplicates. Actions: add_items, update_status, get_list, reset, replan, update_item, ",
        "delete_item, insert_item, cleanup. Use this for execution tracking, not as a general note store."
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

        // Get or load tasks list
        let mut list = get_or_load_tasks(&execution_id).await;
        let mut needs_save = false;

        let result = match args.action.as_str() {
            "add_items" => {
                let new_items = args
                    .items
                    .ok_or_else(|| TasksError::MissingParameters("add_items".to_string()))?;
                for desc in new_items {
                    // Avoid duplicate items
                    if !list.items.iter().any(|t| t.description == desc) {
                        list.items.push(TaskItem {
                            description: desc,
                            status: TaskStatus::Pending,
                            result: None,
                        });
                    }
                }
                if list.current_index.is_none() && !list.items.is_empty() {
                    list.current_index = Some(0);
                    list.items[0].status = TaskStatus::InProgress;
                }
                needs_save = true;
                Ok(TasksOutput {
                    success: true,
                    list: Some(list.clone()),
                    message: "Items added to tasks".to_string(),
                })
            }
            "update_status" => {
                let idx = args
                    .item_index
                    .ok_or_else(|| TasksError::MissingParameters("update_status".to_string()))?;
                let status = args
                    .status
                    .ok_or_else(|| TasksError::MissingParameters("update_status".to_string()))?;

                if idx >= list.items.len() {
                    return Err(TasksError::IndexOutOfBounds(idx));
                }

                list.items[idx].status = status.clone();
                if let Some(res) = args.result {
                    list.items[idx].result = Some(res);
                }

                // Advance the pointer whenever the current item reaches a terminal state.
                if status.is_terminal() && Some(idx) == list.current_index {
                    if idx + 1 < list.items.len() {
                        list.current_index = Some(idx + 1);
                        list.items[idx + 1].status = TaskStatus::InProgress;
                    } else {
                        list.current_index = None;
                    }
                }
                needs_save = true;
                Ok(TasksOutput {
                    success: true,
                    list: Some(list.clone()),
                    message: format!("Updated item {} status to {:?}", idx, status),
                })
            }
            "get_list" => {
                // For get_list, we always try to load from database first if cache is empty
                if list.items.is_empty() {
                    if let Some(db_list) = load_tasks_from_db(&execution_id).await {
                        list = db_list;
                    }
                }
                Ok(TasksOutput {
                    success: true,
                    list: Some(list.clone()),
                    message: if list.items.is_empty() {
                        "No existing tasks found".to_string()
                    } else {
                        format!("Retrieved {} tasks", list.items.len())
                    },
                })
            }
            "reset" => {
                list = TasksList::default();
                needs_save = true;
                // Also delete from database
                delete_tasks_from_db(&execution_id).await;
                Ok(TasksOutput {
                    success: true,
                    list: Some(list.clone()),
                    message: "Tasks list reset successfully".to_string(),
                })
            }
            "replan" => {
                let new_items = args
                    .items
                    .ok_or_else(|| TasksError::MissingParameters("replan".to_string()))?;

                // Clear existing items and add new ones (deduplicated)
                list.items.clear();
                let mut seen_descriptions = std::collections::HashSet::new();
                for desc in new_items {
                    if seen_descriptions.insert(desc.clone()) {
                        list.items.push(TaskItem {
                            description: desc,
                            status: TaskStatus::Pending,
                            result: None,
                        });
                    }
                }

                // Set first item as in progress
                if !list.items.is_empty() {
                    list.current_index = Some(0);
                    list.items[0].status = TaskStatus::InProgress;
                } else {
                    list.current_index = None;
                }
                needs_save = true;
                Ok(TasksOutput {
                    success: true,
                    list: Some(list.clone()),
                    message: format!("Tasks list replaced with {} new items", list.items.len()),
                })
            }
            "update_item" => {
                let idx = args
                    .item_index
                    .ok_or_else(|| TasksError::MissingParameters("update_item".to_string()))?;
                let new_desc = args
                    .new_description
                    .ok_or_else(|| TasksError::MissingParameters("update_item".to_string()))?;

                if idx >= list.items.len() {
                    return Err(TasksError::IndexOutOfBounds(idx));
                }

                list.items[idx].description = new_desc;
                needs_save = true;
                Ok(TasksOutput {
                    success: true,
                    list: Some(list.clone()),
                    message: format!("Updated item {} description", idx),
                })
            }
            "delete_item" => {
                let idx = args
                    .item_index
                    .ok_or_else(|| TasksError::MissingParameters("delete_item".to_string()))?;

                if idx >= list.items.len() {
                    return Err(TasksError::IndexOutOfBounds(idx));
                }

                list.items.remove(idx);

                // Adjust current_index if necessary
                if let Some(current_idx) = list.current_index {
                    if current_idx == idx {
                        if idx < list.items.len() {
                            list.current_index = Some(idx);
                            list.items[idx].status = TaskStatus::InProgress;
                        } else if idx > 0 {
                            list.current_index = Some(idx - 1);
                        } else {
                            list.current_index = None;
                        }
                    } else if current_idx > idx {
                        list.current_index = Some(current_idx - 1);
                    }
                }
                needs_save = true;
                Ok(TasksOutput {
                    success: true,
                    list: Some(list.clone()),
                    message: format!("Deleted item at index {}", idx),
                })
            }
            "insert_item" => {
                let idx = args
                    .item_index
                    .ok_or_else(|| TasksError::MissingParameters("insert_item".to_string()))?;
                let new_desc = args
                    .new_description
                    .ok_or_else(|| TasksError::MissingParameters("insert_item".to_string()))?;

                if idx > list.items.len() {
                    return Err(TasksError::IndexOutOfBounds(idx));
                }

                list.items.insert(
                    idx,
                    TaskItem {
                        description: new_desc,
                        status: TaskStatus::Pending,
                        result: None,
                    },
                );

                // Adjust current_index if necessary
                if let Some(current_idx) = list.current_index {
                    if current_idx >= idx {
                        list.current_index = Some(current_idx + 1);
                    }
                }
                needs_save = true;
                Ok(TasksOutput {
                    success: true,
                    list: Some(list.clone()),
                    message: format!("Inserted item at index {}", idx),
                })
            }
            "cleanup" => {
                // Remove from both cache and database
                {
                    let mut cache = TASKS_CACHE.write().await;
                    cache.remove(&execution_id);
                }
                delete_tasks_from_db(&execution_id).await;

                Ok(TasksOutput {
                    success: true,
                    list: None,
                    message: format!("Cleaned up tasks list for execution {}", execution_id),
                })
            }
            _ => Err(TasksError::InternalError(format!(
                "Unknown action: {}",
                args.action
            ))),
        };

        // Save to database and update cache if needed
        if needs_save {
            // Update cache
            {
                let mut cache = TASKS_CACHE.write().await;
                cache.insert(execution_id.clone(), list.clone());
            }
            // Save to database
            save_tasks_to_db(&execution_id, &list).await;
        }

        // Emit events for UI synchronization.
        if let Ok(ref output) = result {
            if let Some(ref list) = output.list {
                if let Some(handle) = &*APP_HANDLE.read().await {
                    if args.action != "get_list" {
                        // Emit plan event for UI synchronization.
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
                    }

                    // Emit agent-tasks-update for all actions with a concrete list, including get_list.
                    let tasks_json: Vec<serde_json::Value> = list
                        .items
                        .iter()
                        .enumerate()
                        .map(|(i, item)| {
                            serde_json::json!({
                                "id": format!("{}_{}", execution_id, i),
                                "content": item.description,
                                "status": match item.status {
                                    TaskStatus::Pending => "pending",
                                    TaskStatus::InProgress => "in_progress",
                                    TaskStatus::Completed => "completed",
                                    TaskStatus::Failed => "failed",
                                },
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

/// Helper function to auto-complete all unfinished tasks for an execution.
/// Used as a server-side fallback when the agent's response clearly indicates
/// the task is done but it forgot to call tasks(update_status, completed).
pub async fn auto_complete_all_tasks(execution_id: &str, reason: &str) -> bool {
    let mut list = get_or_load_tasks(execution_id).await;
    if list.items.is_empty() {
        return false;
    }

    let mut changed = false;
    for item in list.items.iter_mut() {
        if item.status != TaskStatus::Completed {
            item.status = TaskStatus::Completed;
            if item.result.is_none() {
                item.result = Some(format!("[auto-completed] {}", reason));
            }
            changed = true;
        }
    }
    if !changed {
        return false;
    }

    list.current_index = None;

    // Update cache
    {
        let mut cache = TASKS_CACHE.write().await;
        cache.insert(execution_id.to_string(), list.clone());
    }
    // Persist to database
    save_tasks_to_db(execution_id, &list).await;

    // Emit UI event
    if let Some(handle) = &*APP_HANDLE.read().await {
        let _ = handle.emit(
            "agent:plan_updated",
            serde_json::json!({
                "execution_id": execution_id,
                "plan": {
                    "tasks": list.items,
                    "current_task_index": serde_json::Value::Null
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
                    "status": "completed",
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

    tracing::info!(
        "Auto-completed all tasks for execution {} (reason: {})",
        execution_id,
        reason
    );
    true
}

/// Helper function to cleanup all task lists (cache only, not database)
pub async fn cleanup_all_tasks() {
    let mut cache = TASKS_CACHE.write().await;
    cache.clear();
}
