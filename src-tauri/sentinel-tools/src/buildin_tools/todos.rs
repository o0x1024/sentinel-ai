//! Todos tool for autonomous agent planning and tracking

use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use once_cell::sync::Lazy;
use tauri::{AppHandle, Emitter};

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

/// Global todos storage, keyed by execution_id
static TODOS: Lazy<Arc<RwLock<HashMap<String, TodosList>>>> = Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

/// Global AppHandle for emitting events
static APP_HANDLE: Lazy<RwLock<Option<AppHandle>>> = Lazy::new(|| RwLock::new(None));

/// Set global AppHandle for todos
pub async fn set_todos_app_handle(handle: AppHandle) {
    let mut h = APP_HANDLE.write().await;
    *h = Some(handle);
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
    pub const DESCRIPTION: &'static str = "Manage and track the agent's execution todos. Actions: add_items (append), update_status (change status), get_list (view), reset (clear all), replan (replace all items), update_item (modify description), delete_item (remove), insert_item (add at position), cleanup (remove list from memory). Mandatory for complex multi-step security tasks.";
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
        let mut todos = TODOS.write().await;
        let list = todos.entry(args.execution_id.clone()).or_insert_with(TodosList::default);

        let result = match args.action.as_str() {
            "add_items" => {
                let new_items = args.items.ok_or_else(|| TodosError::MissingParameters("add_items".to_string()))?;
                for desc in new_items {
                    // Avoid duplicate items by checking if description already exists
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

                Ok(TodosOutput {
                    success: true,
                    list: Some(list.clone()),
                    message: format!("Updated item {} status to {:?}", idx, status),
                })
            }
            "get_list" => {
                Ok(TodosOutput {
                    success: true,
                    list: Some(list.clone()),
                    message: "Retrieved current todos list".to_string(),
                })
            }
            "reset" => {
                *list = TodosList::default();
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
                        // If we deleted the current item, move to next or set to None
                        if idx < list.items.len() {
                            list.current_index = Some(idx);
                            list.items[idx].status = TodoStatus::InProgress;
                        } else if idx > 0 {
                            list.current_index = Some(idx - 1);
                        } else {
                            list.current_index = None;
                        }
                    } else if current_idx > idx {
                        // If current item is after deleted item, decrement index
                        list.current_index = Some(current_idx - 1);
                    }
                }
                
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
                
                Ok(TodosOutput {
                    success: true,
                    list: Some(list.clone()),
                    message: format!("Inserted item at index {}", idx),
                })
            }
            "cleanup" => {
                // Remove list from global storage
                let execution_id = args.execution_id.clone();
                todos.remove(&execution_id);
                
                Ok(TodosOutput {
                    success: true,
                    list: None,
                    message: format!("Cleaned up todos list for execution {}", execution_id),
                })
            }
            _ => Err(TodosError::InternalError(format!("Unknown action: {}", args.action))),
        };

        // Emit event if successful and not just a "get_list" action
        if let Ok(ref output) = result {
            if args.action != "get_list" {
                if let Some(ref list) = output.list {
                    if let Some(handle) = &*APP_HANDLE.read().await {
                        // Emit legacy event for existing UI
                        let _ = handle.emit("agent:plan_updated", serde_json::json!({
                            "execution_id": args.execution_id,
                            "plan": {
                                "tasks": list.items,
                                "current_task_index": list.current_index
                            }
                        }));

                        // Emit agent-todos-update event for useTodos.ts
                        let todos_json: Vec<serde_json::Value> = list.items.iter().enumerate().map(|(i, item)| {
                            serde_json::json!({
                                "id": format!("{}_{}", args.execution_id, i),
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
                            "execution_id": args.execution_id,
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
    let todos = TODOS.read().await;
    todos.get(execution_id).cloned()
}

/// Helper function to cleanup todos list for an execution
pub async fn cleanup_execution_todos(execution_id: &str) -> bool {
    let mut todos = TODOS.write().await;
    todos.remove(execution_id).is_some()
}

/// Helper function to cleanup all todos lists
pub async fn cleanup_all_todos() {
    let mut todos = TODOS.write().await;
    todos.clear();
}
