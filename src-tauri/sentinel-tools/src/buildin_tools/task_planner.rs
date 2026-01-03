//! Task Planner tool for autonomous agent planning and tracking

use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use once_cell::sync::Lazy;
use tauri::{AppHandle, Emitter};

/// Task status
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task is waiting to be started
    Pending,
    /// Task is currently being worked on
    InProgress,
    /// Task has been successfully completed
    Completed,
    /// Task has failed
    Failed,
}

/// A single task in the plan
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Task {
    /// Description of the task
    pub description: String,
    /// Current status of the task
    pub status: TaskStatus,
    /// Optional result or observation from the task
    pub result: Option<String>,
}

/// The overall execution plan
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Plan {
    /// List of tasks
    pub tasks: Vec<Task>,
    /// Index of the current task being executed
    pub current_task_index: Option<usize>,
}

/// Global plan storage, keyed by execution_id
static PLANS: Lazy<Arc<RwLock<HashMap<String, Plan>>>> = Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

/// Global AppHandle for emitting events
static APP_HANDLE: Lazy<RwLock<Option<AppHandle>>> = Lazy::new(|| RwLock::new(None));

/// Set global AppHandle for task planner
pub async fn set_planner_app_handle(handle: AppHandle) {
    let mut h = APP_HANDLE.write().await;
    *h = Some(handle);
}

/// Task planner arguments
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct TaskPlannerArgs {
    /// The execution ID of the current agent run
    pub execution_id: String,
    /// The action to perform: "add_tasks", "update_status", "get_plan", "reset", "replan", "update_task", "delete_task", "insert_task"
    pub action: String,
    /// Tasks to add (required for "add_tasks", "replan")
    pub tasks: Option<Vec<String>>,
    /// Index of the task to update/delete/insert (required for "update_status", "update_task", "delete_task", "insert_task")
    pub task_index: Option<usize>,
    /// New status for the task (required for "update_status")
    pub status: Option<TaskStatus>,
    /// Optional result or observation to record
    pub result: Option<String>,
    /// New task description (required for "update_task", "insert_task")
    pub new_description: Option<String>,
}

/// Task planner output
#[derive(Debug, Clone, Serialize)]
pub struct TaskPlannerOutput {
    pub success: bool,
    pub plan: Option<Plan>,
    pub message: String,
}

/// Task planner errors
#[derive(Debug, thiserror::Error)]
pub enum TaskPlannerError {
    #[error("Missing required parameters for action {0}")]
    MissingParameters(String),
    #[error("Task index {0} out of bounds")]
    IndexOutOfBounds(usize),
    #[error("Internal error: {0}")]
    InternalError(String),
}

/// Task planner tool
#[derive(Debug, Clone, Default)]
pub struct TaskPlannerTool;

impl TaskPlannerTool {
    pub fn new() -> Self {
        Self
    }
    
    pub const NAME: &'static str = "task_planner";
    pub const DESCRIPTION: &'static str = "Manage and track the agent's execution plan. Actions: add_tasks (append), update_status (change status), get_plan (view), reset (clear all), replan (replace all tasks), update_task (modify description), delete_task (remove), insert_task (add at position). Mandatory for complex multi-step security tasks.";
}

impl Tool for TaskPlannerTool {
    const NAME: &'static str = Self::NAME;
    type Args = TaskPlannerArgs;
    type Output = TaskPlannerOutput;
    type Error = TaskPlannerError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(TaskPlannerArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let mut plans = PLANS.write().await;
        let plan = plans.entry(args.execution_id.clone()).or_insert_with(Plan::default);

        let result = match args.action.as_str() {
            "add_tasks" => {
                let new_tasks = args.tasks.ok_or_else(|| TaskPlannerError::MissingParameters("add_tasks".to_string()))?;
                for desc in new_tasks {
                    plan.tasks.push(Task {
                        description: desc,
                        status: TaskStatus::Pending,
                        result: None,
                    });
                }
                if plan.current_task_index.is_none() && !plan.tasks.is_empty() {
                    plan.current_task_index = Some(0);
                    plan.tasks[0].status = TaskStatus::InProgress;
                }
                Ok(TaskPlannerOutput {
                    success: true,
                    plan: Some(plan.clone()),
                    message: "Tasks added to plan".to_string(),
                })
            }
            "update_status" => {
                let idx = args.task_index.ok_or_else(|| TaskPlannerError::MissingParameters("update_status".to_string()))?;
                let status = args.status.ok_or_else(|| TaskPlannerError::MissingParameters("update_status".to_string()))?;
                
                if idx >= plan.tasks.len() {
                    return Err(TaskPlannerError::IndexOutOfBounds(idx));
                }
                
                plan.tasks[idx].status = status.clone();
                if let Some(res) = args.result {
                    plan.tasks[idx].result = Some(res);
                }

                // Auto-advance if completed
                if status == TaskStatus::Completed && Some(idx) == plan.current_task_index {
                    if idx + 1 < plan.tasks.len() {
                        plan.current_task_index = Some(idx + 1);
                        plan.tasks[idx + 1].status = TaskStatus::InProgress;
                    } else {
                        plan.current_task_index = None;
                    }
                }

                Ok(TaskPlannerOutput {
                    success: true,
                    plan: Some(plan.clone()),
                    message: format!("Updated task {} status to {:?}", idx, status),
                })
            }
            "get_plan" => {
                Ok(TaskPlannerOutput {
                    success: true,
                    plan: Some(plan.clone()),
                    message: "Retrieved current plan".to_string(),
                })
            }
            "reset" => {
                *plan = Plan::default();
                Ok(TaskPlannerOutput {
                    success: true,
                    plan: Some(plan.clone()),
                    message: "Plan reset successfully".to_string(),
                })
            }
            "replan" => {
                let new_tasks = args.tasks.ok_or_else(|| TaskPlannerError::MissingParameters("replan".to_string()))?;
                
                // Clear existing tasks and add new ones
                plan.tasks.clear();
                for desc in new_tasks {
                    plan.tasks.push(Task {
                        description: desc,
                        status: TaskStatus::Pending,
                        result: None,
                    });
                }
                
                // Set first task as in progress
                if !plan.tasks.is_empty() {
                    plan.current_task_index = Some(0);
                    plan.tasks[0].status = TaskStatus::InProgress;
                } else {
                    plan.current_task_index = None;
                }
                
                Ok(TaskPlannerOutput {
                    success: true,
                    plan: Some(plan.clone()),
                    message: format!("Plan replaced with {} new tasks", plan.tasks.len()),
                })
            }
            "update_task" => {
                let idx = args.task_index.ok_or_else(|| TaskPlannerError::MissingParameters("update_task".to_string()))?;
                let new_desc = args.new_description.ok_or_else(|| TaskPlannerError::MissingParameters("update_task".to_string()))?;
                
                if idx >= plan.tasks.len() {
                    return Err(TaskPlannerError::IndexOutOfBounds(idx));
                }
                
                plan.tasks[idx].description = new_desc;
                
                Ok(TaskPlannerOutput {
                    success: true,
                    plan: Some(plan.clone()),
                    message: format!("Updated task {} description", idx),
                })
            }
            "delete_task" => {
                let idx = args.task_index.ok_or_else(|| TaskPlannerError::MissingParameters("delete_task".to_string()))?;
                
                if idx >= plan.tasks.len() {
                    return Err(TaskPlannerError::IndexOutOfBounds(idx));
                }
                
                plan.tasks.remove(idx);
                
                // Adjust current_task_index if necessary
                if let Some(current_idx) = plan.current_task_index {
                    if current_idx == idx {
                        // If we deleted the current task, move to next or set to None
                        if idx < plan.tasks.len() {
                            plan.current_task_index = Some(idx);
                            plan.tasks[idx].status = TaskStatus::InProgress;
                        } else if idx > 0 {
                            plan.current_task_index = Some(idx - 1);
                        } else {
                            plan.current_task_index = None;
                        }
                    } else if current_idx > idx {
                        // If current task is after deleted task, decrement index
                        plan.current_task_index = Some(current_idx - 1);
                    }
                }
                
                Ok(TaskPlannerOutput {
                    success: true,
                    plan: Some(plan.clone()),
                    message: format!("Deleted task at index {}", idx),
                })
            }
            "insert_task" => {
                let idx = args.task_index.ok_or_else(|| TaskPlannerError::MissingParameters("insert_task".to_string()))?;
                let new_desc = args.new_description.ok_or_else(|| TaskPlannerError::MissingParameters("insert_task".to_string()))?;
                
                if idx > plan.tasks.len() {
                    return Err(TaskPlannerError::IndexOutOfBounds(idx));
                }
                
                plan.tasks.insert(idx, Task {
                    description: new_desc,
                    status: TaskStatus::Pending,
                    result: None,
                });
                
                // Adjust current_task_index if necessary
                if let Some(current_idx) = plan.current_task_index {
                    if current_idx >= idx {
                        plan.current_task_index = Some(current_idx + 1);
                    }
                }
                
                Ok(TaskPlannerOutput {
                    success: true,
                    plan: Some(plan.clone()),
                    message: format!("Inserted task at index {}", idx),
                })
            }
            _ => Err(TaskPlannerError::InternalError(format!("Unknown action: {}", args.action))),
        };

        // Emit event if successful and not just a "get_plan" action
        if let Ok(ref output) = result {
            if args.action != "get_plan" {
                if let Some(ref plan) = output.plan {
                    if let Some(handle) = &*APP_HANDLE.read().await {
                        // Emit legacy event for existing UI
                        let _ = handle.emit("agent:plan_updated", serde_json::json!({
                            "execution_id": args.execution_id,
                            "plan": plan
                        }));

                        // Emit agent-todos-update event for useTodos.ts
                        let todos: Vec<serde_json::Value> = plan.tasks.iter().enumerate().map(|(i, t)| {
                            serde_json::json!({
                                "id": format!("{}_{}", args.execution_id, i),
                                "content": t.description,
                                "status": match t.status {
                                    TaskStatus::Pending => "pending",
                                    TaskStatus::InProgress => "in_progress",
                                    TaskStatus::Completed => "completed",
                                    TaskStatus::Failed => "failed",
                                },
                                "created_at": chrono::Utc::now().timestamp_millis(),
                                "updated_at": chrono::Utc::now().timestamp_millis(),
                                "metadata": {
                                    "step_index": i,
                                    "result": t.result
                                }
                            })
                        }).collect();

                        let _ = handle.emit("agent-todos-update", serde_json::json!({
                            "execution_id": args.execution_id,
                            "todos": todos,
                            "timestamp": chrono::Utc::now().timestamp_millis()
                        }));
                    }
                }
            }
        }

        result
    }
}

/// Helper function to get plan for an execution
pub async fn get_execution_plan(execution_id: &str) -> Option<Plan> {
    let plans = PLANS.read().await;
    plans.get(execution_id).cloned()
}

