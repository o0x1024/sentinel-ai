//! Todos tool for autonomous agent planning and tracking
//! Supports database persistence for session recovery

use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use once_cell::sync::Lazy;
use tauri::{AppHandle, Emitter, Manager};
use sentinel_db::DatabaseService;

/// Todo status
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    /// Todo is waiting to be started
    Pending,
    /// Todo is currently being worked on
    InProgress,
    /// Todo has been successfully completed
    Completed,
    /// Todo has failed
    Failed,
}

impl From<&str> for TodoStatus {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "pending" => TodoStatus::Pending,
            "in_progress" => TodoStatus::InProgress,
            "completed" => TodoStatus::Completed,
            "failed" => TodoStatus::Failed,
            _ => TodoStatus::Pending,
        }
    }
}

impl From<sentinel_db::TodoStatus> for TodoStatus {
    fn from(status: sentinel_db::TodoStatus) -> Self {
        match status {
            sentinel_db::TodoStatus::Pending => TodoStatus::Pending,
            sentinel_db::TodoStatus::InProgress => TodoStatus::InProgress,
            sentinel_db::TodoStatus::Completed => TodoStatus::Completed,
            sentinel_db::TodoStatus::Failed => TodoStatus::Failed,
        }
    }
}

impl From<TodoStatus> for sentinel_db::TodoStatus {
    fn from(status: TodoStatus) -> Self {
        match status {
            TodoStatus::Pending => sentinel_db::TodoStatus::Pending,
            TodoStatus::InProgress => sentinel_db::TodoStatus::InProgress,
            TodoStatus::Completed => sentinel_db::TodoStatus::Completed,
            TodoStatus::Failed => sentinel_db::TodoStatus::Failed,
        }
    }
}

/// A single todo item
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TodoItem {
    /// Description of the todo
    pub description: String,
    /// Current status of the todo
    pub status: TodoStatus,
    /// Optional result or observation from the todo
    pub result: Option<String>,
}

/// The overall todos list
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TodosList {
    /// List of todos
    pub items: Vec<TodoItem>,
    /// Index of the current todo being executed
    pub current_index: Option<usize>,
}

impl TodosList {
    /// Calculate current_index from items
    fn recalculate_current_index(&mut self) {
        self.current_index = self.items.iter().position(|item| item.status == TodoStatus::InProgress);
    }
}

/// In-memory cache for todos (synced with database)
static TODOS_CACHE: Lazy<Arc<RwLock<HashMap<String, TodosList>>>> = Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

/// Global AppHandle for emitting events and database access
static APP_HANDLE: Lazy<RwLock<Option<AppHandle>>> = Lazy::new(|| RwLock::new(None));

/// Set global AppHandle for todos
pub async fn set_todos_app_handle(handle: AppHandle) {
    let mut h = APP_HANDLE.write().await;
    *h = Some(handle);
}

/// Get database service from AppHandle
async fn get_db_service() -> Option<Arc<DatabaseService>> {
    let handle = APP_HANDLE.read().await;
    if let Some(ref h) = *handle {
        h.try_state::<Arc<DatabaseService>>().map(|s| s.inner().clone())
    } else {
        None
    }
}

/// Load todos from database into memory cache
async fn load_todos_from_db(execution_id: &str) -> Option<TodosList> {
    let db = get_db_service().await?;
    match db.get_agent_todos(execution_id).await {
        Ok(db_items) if !db_items.is_empty() => {
            let items: Vec<TodoItem> = db_items.into_iter().map(|item| {
                TodoItem {
                    description: item.description,
                    status: TodoStatus::from(item.status.as_str()),
                    result: item.result,
                }
            }).collect();
            
            let mut list = TodosList { items, current_index: None };
            list.recalculate_current_index();
            
            // Update cache
            let mut cache = TODOS_CACHE.write().await;
            cache.insert(execution_id.to_string(), list.clone());
            
            tracing::info!("Loaded {} todos from database for execution {}", list.items.len(), execution_id);
            Some(list)
        }
        Ok(_) => None,
        Err(e) => {
            tracing::warn!("Failed to load todos from database: {}", e);
            None
        }
    }
}

/// Save todos to database
async fn save_todos_to_db(execution_id: &str, list: &TodosList) {
    if let Some(db) = get_db_service().await {
        let items: Vec<sentinel_db::TodoItemInput> = list.items.iter().map(|item| {
            sentinel_db::TodoItemInput {
                description: item.description.clone(),
                status: item.status.clone().into(),
                result: item.result.clone(),
            }
        }).collect();
        
        if let Err(e) = db.save_agent_todos(execution_id, &items).await {
            tracing::warn!("Failed to save todos to database: {}", e);
        } else {
            tracing::debug!("Saved {} todos to database for execution {}", items.len(), execution_id);
        }
    }
}

/// Delete todos from database
async fn delete_todos_from_db(execution_id: &str) {
    if let Some(db) = get_db_service().await {
        if let Err(e) = db.delete_agent_todos(execution_id).await {
            tracing::warn!("Failed to delete todos from database: {}", e);
        }
    }
}

/// Get or load todos list for an execution
async fn get_or_load_todos(execution_id: &str) -> TodosList {
    // Check cache first
    {
        let cache = TODOS_CACHE.read().await;
        if let Some(list) = cache.get(execution_id) {
            return list.clone();
        }
    }
    
    // Try to load from database
    if let Some(list) = load_todos_from_db(execution_id).await {
        return list;
    }
    
    // Return empty list
    TodosList::default()
}

/// Todos tool arguments
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct TodosArgs {
    /// The execution ID of the current agent run
    pub execution_id: String,
    /// The action to perform: "add_items", "update_status", "get_list", "reset", "replan", "update_item", "delete_item", "insert_item", "cleanup"
    pub action: String,
    /// Items to add (required for "add_items", "replan")
    pub items: Option<Vec<String>>,
    /// Index of the item to update/delete/insert (required for "update_status", "update_item", "delete_item", "insert_item")
    pub item_index: Option<usize>,
    /// New status for the item (required for "update_status")
    pub status: Option<TodoStatus>,
    /// Optional result or observation to record
    pub result: Option<String>,
    /// New item description (required for "update_item", "insert_item")
    pub new_description: Option<String>,
}

/// Todos tool output
#[derive(Debug, Clone, Serialize)]
pub struct TodosOutput {
    pub success: bool,
    pub list: Option<TodosList>,
    pub message: String,
}

/// Todos tool errors
#[derive(Debug, thiserror::Error)]
pub enum TodosError {
    #[error("Missing required parameters for action {0}")]
    MissingParameters(String),
    #[error("Item index {0} out of bounds")]
    IndexOutOfBounds(usize),
    #[error("Internal error: {0}")]
    InternalError(String),
}

/// Todos tool
#[derive(Debug, Clone, Default)]
pub struct TodosTool;

impl TodosTool {
    pub fn new() -> Self {
        Self
    }
    
    pub const NAME: &'static str = "todos";
    pub const DESCRIPTION: &'static str = "Manage and track the agent's execution todos. Actions: add_items (append), update_status (change status), get_list (view existing todos), reset (clear all), replan (replace all items), update_item (modify description), delete_item (remove), insert_item (add at position), cleanup (remove list). Todos persist across sessions - use get_list first to check existing todos before creating new ones.";
}

impl Tool for TodosTool {
    const NAME: &'static str = Self::NAME;
    type Args = TodosArgs;
    type Output = TodosOutput;
    type Error = TodosError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(TodosArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let execution_id = args.execution_id.clone();
        
        // Get or load todos list
        let mut list = get_or_load_todos(&execution_id).await;
        let mut needs_save = false;

        let result = match args.action.as_str() {
            "add_items" => {
                let new_items = args.items.ok_or_else(|| TodosError::MissingParameters("add_items".to_string()))?;
                for desc in new_items {
                    // Avoid duplicate items
                    if !list.items.iter().any(|t| t.description == desc) {
                        list.items.push(TodoItem {
                            description: desc,
                            status: TodoStatus::Pending,
                            result: None,
                        });
                    }
                }
                if list.current_index.is_none() && !list.items.is_empty() {
                    list.current_index = Some(0);
                    list.items[0].status = TodoStatus::InProgress;
                }
                needs_save = true;
                Ok(TodosOutput {
                    success: true,
                    list: Some(list.clone()),
                    message: "Items added to todos".to_string(),
                })
            }
            "update_status" => {
                let idx = args.item_index.ok_or_else(|| TodosError::MissingParameters("update_status".to_string()))?;
                let status = args.status.ok_or_else(|| TodosError::MissingParameters("update_status".to_string()))?;
                
                if idx >= list.items.len() {
                    return Err(TodosError::IndexOutOfBounds(idx));
                }
                
                list.items[idx].status = status.clone();
                if let Some(res) = args.result {
                    list.items[idx].result = Some(res);
                }

                // Auto-advance if completed
                if status == TodoStatus::Completed && Some(idx) == list.current_index {
                    if idx + 1 < list.items.len() {
                        list.current_index = Some(idx + 1);
                        list.items[idx + 1].status = TodoStatus::InProgress;
                    } else {
                        list.current_index = None;
                    }
                }
                needs_save = true;
                Ok(TodosOutput {
                    success: true,
                    list: Some(list.clone()),
                    message: format!("Updated item {} status to {:?}", idx, status),
                })
            }
            "get_list" => {
                // For get_list, we always try to load from database first if cache is empty
                if list.items.is_empty() {
                    if let Some(db_list) = load_todos_from_db(&execution_id).await {
                        list = db_list;
                    }
                }
                Ok(TodosOutput {
                    success: true,
                    list: Some(list.clone()),
                    message: if list.items.is_empty() {
                        "No existing todos found".to_string()
                    } else {
                        format!("Retrieved {} todos", list.items.len())
                    },
                })
            }
            "reset" => {
                list = TodosList::default();
                needs_save = true;
                // Also delete from database
                delete_todos_from_db(&execution_id).await;
                Ok(TodosOutput {
                    success: true,
                    list: Some(list.clone()),
                    message: "Todos list reset successfully".to_string(),
                })
            }
            "replan" => {
                let new_items = args.items.ok_or_else(|| TodosError::MissingParameters("replan".to_string()))?;
                
                // Clear existing items and add new ones (deduplicated)
                list.items.clear();
                let mut seen_descriptions = std::collections::HashSet::new();
                for desc in new_items {
                    if seen_descriptions.insert(desc.clone()) {
                        list.items.push(TodoItem {
                            description: desc,
                            status: TodoStatus::Pending,
                            result: None,
                        });
                    }
                }
                
                // Set first item as in progress
                if !list.items.is_empty() {
                    list.current_index = Some(0);
                    list.items[0].status = TodoStatus::InProgress;
                } else {
                    list.current_index = None;
                }
                needs_save = true;
                Ok(TodosOutput {
                    success: true,
                    list: Some(list.clone()),
                    message: format!("Todos list replaced with {} new items", list.items.len()),
                })
            }
            "update_item" => {
                let idx = args.item_index.ok_or_else(|| TodosError::MissingParameters("update_item".to_string()))?;
                let new_desc = args.new_description.ok_or_else(|| TodosError::MissingParameters("update_item".to_string()))?;
                
                if idx >= list.items.len() {
                    return Err(TodosError::IndexOutOfBounds(idx));
                }
                
                list.items[idx].description = new_desc;
                needs_save = true;
                Ok(TodosOutput {
                    success: true,
                    list: Some(list.clone()),
                    message: format!("Updated item {} description", idx),
                })
            }
            "delete_item" => {
                let idx = args.item_index.ok_or_else(|| TodosError::MissingParameters("delete_item".to_string()))?;
                
                if idx >= list.items.len() {
                    return Err(TodosError::IndexOutOfBounds(idx));
                }
                
                list.items.remove(idx);
                
                // Adjust current_index if necessary
                if let Some(current_idx) = list.current_index {
                    if current_idx == idx {
                        if idx < list.items.len() {
                            list.current_index = Some(idx);
                            list.items[idx].status = TodoStatus::InProgress;
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
                Ok(TodosOutput {
                    success: true,
                    list: Some(list.clone()),
                    message: format!("Deleted item at index {}", idx),
                })
            }
            "insert_item" => {
                let idx = args.item_index.ok_or_else(|| TodosError::MissingParameters("insert_item".to_string()))?;
                let new_desc = args.new_description.ok_or_else(|| TodosError::MissingParameters("insert_item".to_string()))?;
                
                if idx > list.items.len() {
                    return Err(TodosError::IndexOutOfBounds(idx));
                }
                
                list.items.insert(idx, TodoItem {
                    description: new_desc,
                    status: TodoStatus::Pending,
                    result: None,
                });
                
                // Adjust current_index if necessary
                if let Some(current_idx) = list.current_index {
                    if current_idx >= idx {
                        list.current_index = Some(current_idx + 1);
                    }
                }
                needs_save = true;
                Ok(TodosOutput {
                    success: true,
                    list: Some(list.clone()),
                    message: format!("Inserted item at index {}", idx),
                })
            }
            "cleanup" => {
                // Remove from both cache and database
                {
                    let mut cache = TODOS_CACHE.write().await;
                    cache.remove(&execution_id);
                }
                delete_todos_from_db(&execution_id).await;
                
                Ok(TodosOutput {
                    success: true,
                    list: None,
                    message: format!("Cleaned up todos list for execution {}", execution_id),
                })
            }
            _ => Err(TodosError::InternalError(format!("Unknown action: {}", args.action))),
        };

        // Save to database and update cache if needed
        if needs_save {
            // Update cache
            {
                let mut cache = TODOS_CACHE.write().await;
                cache.insert(execution_id.clone(), list.clone());
            }
            // Save to database
            save_todos_to_db(&execution_id, &list).await;
        }

        // Emit event if successful and not just a "get_list" action
        if let Ok(ref output) = result {
            if args.action != "get_list" {
                if let Some(ref list) = output.list {
                    if let Some(handle) = &*APP_HANDLE.read().await {
                        // Emit legacy event for existing UI
                        let _ = handle.emit("agent:plan_updated", serde_json::json!({
                            "execution_id": execution_id,
                            "plan": {
                                "tasks": list.items,
                                "current_task_index": list.current_index
                            }
                        }));

                        // Emit agent-todos-update event for useTodos.ts
                        let todos_json: Vec<serde_json::Value> = list.items.iter().enumerate().map(|(i, item)| {
                            serde_json::json!({
                                "id": format!("{}_{}", execution_id, i),
                                "content": item.description,
                                "status": match item.status {
                                    TodoStatus::Pending => "pending",
                                    TodoStatus::InProgress => "in_progress",
                                    TodoStatus::Completed => "completed",
                                    TodoStatus::Failed => "failed",
                                },
                                "created_at": chrono::Utc::now().timestamp_millis(),
                                "updated_at": chrono::Utc::now().timestamp_millis(),
                                "metadata": {
                                    "step_index": i,
                                    "result": item.result
                                }
                            })
                        }).collect();

                        let _ = handle.emit("agent-todos-update", serde_json::json!({
                            "execution_id": execution_id,
                            "todos": todos_json,
                            "timestamp": chrono::Utc::now().timestamp_millis()
                        }));
                    }
                }
            }
        }

        result
    }
}

/// Helper function to get todos list for an execution
pub async fn get_execution_todos(execution_id: &str) -> Option<TodosList> {
    let list = get_or_load_todos(execution_id).await;
    if list.items.is_empty() {
        None
    } else {
        Some(list)
    }
}

/// Helper function to cleanup todos list for an execution
pub async fn cleanup_execution_todos(execution_id: &str) -> bool {
    let mut cache = TODOS_CACHE.write().await;
    let removed = cache.remove(execution_id).is_some();
    drop(cache);
    
    delete_todos_from_db(execution_id).await;
    removed
}

/// Helper function to cleanup all todos lists (cache only, not database)
pub async fn cleanup_all_todos() {
    let mut cache = TODOS_CACHE.write().await;
    cache.clear();
}
