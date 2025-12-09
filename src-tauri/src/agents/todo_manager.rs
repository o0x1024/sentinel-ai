//! Todo 任务管理器
//! 
//! 参考 Claude Code 的 TodoWrite 工具机制，实现可视化的任务进度追踪。
//! 
//! ## 核心功能
//! - 任务创建、更新、删除
//! - 状态管理（pending/in_progress/completed）
//! - 实时事件通知到前端
//! - 支持任务层级（父子任务）

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tauri::AppHandle;
use tauri::Emitter;

/// Todo 状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
}

impl Default for TodoStatus {
    fn default() -> Self {
        Self::Pending
    }
}

impl std::fmt::Display for TodoStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TodoStatus::Pending => write!(f, "pending"),
            TodoStatus::InProgress => write!(f, "in_progress"),
            TodoStatus::Completed => write!(f, "completed"),
        }
    }
}

/// Todo 元数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TodoMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 单个 Todo 项
/// 
/// 与 Claude Code 的 TodoWrite 保持一致的字段:
/// - content: 祈使句描述任务（如 "Run tests"）
/// - active_form: 进行时态描述（如 "Running tests"）
/// - status: 任务状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: String,
    /// 任务描述（祈使句形式）
    pub content: String,
    /// 任务进行时描述（用于显示当前执行状态）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_form: Option<String>,
    pub status: TodoStatus,
    pub created_at: u64,
    pub updated_at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<TodoMetadata>,
}

impl Todo {
    /// 创建新 Todo
    /// 
    /// # Arguments
    /// * `id` - 唯一标识符
    /// * `content` - 任务描述（祈使句，如 "Run tests"）
    pub fn new(id: impl Into<String>, content: impl Into<String>) -> Self {
        let now = chrono::Utc::now().timestamp_millis() as u64;
        Self {
            id: id.into(),
            content: truncate_content(&content.into(), 70),
            active_form: None,
            status: TodoStatus::Pending,
            created_at: now,
            updated_at: now,
            metadata: None,
        }
    }

    /// 创建带有进行时描述的 Todo（推荐方式）
    /// 
    /// # Arguments
    /// * `id` - 唯一标识符
    /// * `content` - 任务描述（祈使句，如 "Run tests"）
    /// * `active_form` - 进行时描述（如 "Running tests"）
    pub fn with_forms(
        id: impl Into<String>,
        content: impl Into<String>,
        active_form: impl Into<String>,
    ) -> Self {
        let now = chrono::Utc::now().timestamp_millis() as u64;
        Self {
            id: id.into(),
            content: truncate_content(&content.into(), 70),
            active_form: Some(truncate_content(&active_form.into(), 70)),
            status: TodoStatus::Pending,
            created_at: now,
            updated_at: now,
            metadata: None,
        }
    }

    /// 设置状态
    pub fn with_status(mut self, status: TodoStatus) -> Self {
        self.status = status;
        self.updated_at = chrono::Utc::now().timestamp_millis() as u64;
        self
    }

    /// 设置进行时描述
    pub fn with_active_form(mut self, active_form: impl Into<String>) -> Self {
        self.active_form = Some(truncate_content(&active_form.into(), 70));
        self
    }

    /// 设置元数据
    pub fn with_metadata(mut self, metadata: TodoMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// 设置父任务 ID（子任务）
    pub fn with_parent(mut self, parent_id: impl Into<String>) -> Self {
        let metadata = self.metadata.get_or_insert_with(Default::default);
        metadata.parent_id = Some(parent_id.into());
        self
    }

    /// 设置关联工具
    pub fn with_tool(mut self, tool_name: impl Into<String>) -> Self {
        let metadata = self.metadata.get_or_insert_with(Default::default);
        metadata.tool_name = Some(tool_name.into());
        self
    }

    /// 设置步骤索引
    pub fn with_step_index(mut self, index: usize) -> Self {
        let metadata = self.metadata.get_or_insert_with(Default::default);
        metadata.step_index = Some(index);
        self
    }

    /// 设置错误信息
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        let metadata = self.metadata.get_or_insert_with(Default::default);
        metadata.error = Some(error.into());
        self
    }

    /// 获取当前显示文本（如果正在执行，返回 active_form）
    pub fn display_text(&self) -> &str {
        if self.status == TodoStatus::InProgress {
            self.active_form.as_deref().unwrap_or(&self.content)
        } else {
            &self.content
        }
    }

    /// 检查是否已完成
    pub fn is_completed(&self) -> bool {
        self.status == TodoStatus::Completed
    }

    /// 检查是否正在执行
    pub fn is_in_progress(&self) -> bool {
        self.status == TodoStatus::InProgress
    }
}

/// Todos 更新事件负载
#[derive(Debug, Clone, Serialize)]
pub struct TodosUpdatePayload {
    pub execution_id: String,
    pub todos: Vec<Todo>,
    pub timestamp: i64,
}

/// Todo 管理器
pub struct TodoManager {
    /// 按 execution_id 存储 todos
    todos: Arc<RwLock<HashMap<String, Vec<Todo>>>>,
    /// Tauri AppHandle 用于发送事件
    app_handle: Option<AppHandle>,
}

impl TodoManager {
    /// 创建新的 TodoManager
    pub fn new(app_handle: Option<AppHandle>) -> Self {
        Self {
            todos: Arc::new(RwLock::new(HashMap::new())),
            app_handle,
        }
    }

    /// 写入/更新 todos
    pub async fn write_todos(
        &self,
        execution_id: &str,
        todos: Vec<Todo>,
        merge: bool,
    ) -> anyhow::Result<()> {
        let mut store = self.todos.write().await;
        
        if merge {
            // 合并模式：根据 id 更新现有 todos
            let existing = store.entry(execution_id.to_string())
                .or_insert_with(Vec::new);
            
            for new_todo in todos {
                if let Some(pos) = existing.iter().position(|t| t.id == new_todo.id) {
                    existing[pos] = new_todo;
                } else {
                    existing.push(new_todo);
                }
            }
        } else {
            // 替换模式：直接替换所有 todos
            store.insert(execution_id.to_string(), todos);
        }
        
        // 发送更新事件到前端
        let current = store.get(execution_id).cloned().unwrap_or_default();
        drop(store); // 释放锁后再发送事件
        
        self.emit_todos_update(execution_id, &current);
        
        Ok(())
    }

    /// 更新单个 todo 状态
    pub async fn update_status(
        &self,
        execution_id: &str,
        todo_id: &str,
        status: TodoStatus,
    ) -> anyhow::Result<()> {
        let mut store = self.todos.write().await;
        
        if let Some(todos) = store.get_mut(execution_id) {
            if let Some(todo) = todos.iter_mut().find(|t| t.id == todo_id) {
                todo.status = status;
                todo.updated_at = chrono::Utc::now().timestamp_millis() as u64;
            }
        }
        
        let current = store.get(execution_id).cloned().unwrap_or_default();
        drop(store);
        
        self.emit_todos_update(execution_id, &current);
        
        Ok(())
    }

    /// 批量更新 todo 状态
    pub async fn update_statuses(
        &self,
        execution_id: &str,
        updates: Vec<(String, TodoStatus)>,
    ) -> anyhow::Result<()> {
        let mut store = self.todos.write().await;
        let now = chrono::Utc::now().timestamp_millis() as u64;
        
        if let Some(todos) = store.get_mut(execution_id) {
            for (todo_id, status) in updates {
                if let Some(todo) = todos.iter_mut().find(|t| t.id == todo_id) {
                    todo.status = status;
                    todo.updated_at = now;
                }
            }
        }
        
        let current = store.get(execution_id).cloned().unwrap_or_default();
        drop(store);
        
        self.emit_todos_update(execution_id, &current);
        
        Ok(())
    }

    /// 获取当前 todos
    pub async fn get_todos(&self, execution_id: &str) -> Vec<Todo> {
        let store = self.todos.read().await;
        store.get(execution_id).cloned().unwrap_or_default()
    }

    /// 获取进行中的 todo
    pub async fn get_in_progress(&self, execution_id: &str) -> Option<Todo> {
        let store = self.todos.read().await;
        store.get(execution_id)
            .and_then(|todos| todos.iter().find(|t| t.status == TodoStatus::InProgress).cloned())
    }

    /// 获取统计信息
    pub async fn get_stats(&self, execution_id: &str) -> TodoStats {
        let store = self.todos.read().await;
        let todos = store.get(execution_id).cloned().unwrap_or_default();
        
        TodoStats {
            total: todos.len(),
            pending: todos.iter().filter(|t| t.status == TodoStatus::Pending).count(),
            in_progress: todos.iter().filter(|t| t.status == TodoStatus::InProgress).count(),
            completed: todos.iter().filter(|t| t.status == TodoStatus::Completed).count(),
        }
    }

    /// 从 TodoWriteInput 写入 todos（与 Claude Code TodoWrite 兼容）
    pub async fn write_from_input(
        &self,
        execution_id: &str,
        input: TodoWriteInput,
    ) -> anyhow::Result<()> {
        let todos: Vec<Todo> = input.todos.into_iter().map(Todo::from).collect();
        self.write_todos(execution_id, todos, false).await
    }

    /// 标记下一个待处理任务为进行中
    pub async fn start_next(&self, execution_id: &str) -> Option<Todo> {
        let mut store = self.todos.write().await;
        let now = chrono::Utc::now().timestamp_millis() as u64;
        
        if let Some(todos) = store.get_mut(execution_id) {
            // 找到第一个 pending 的任务
            if let Some(todo) = todos.iter_mut().find(|t| t.status == TodoStatus::Pending) {
                todo.status = TodoStatus::InProgress;
                todo.updated_at = now;
                let result = todo.clone();
                
                let current = todos.clone();
                drop(store);
                self.emit_todos_update(execution_id, &current);
                
                return Some(result);
            }
        }
        None
    }

    /// 完成当前进行中的任务并开始下一个
    pub async fn complete_current_and_start_next(&self, execution_id: &str) -> Option<Todo> {
        let mut store = self.todos.write().await;
        let now = chrono::Utc::now().timestamp_millis() as u64;
        
        if let Some(todos) = store.get_mut(execution_id) {
            // 将当前 in_progress 标记为 completed
            for todo in todos.iter_mut() {
                if todo.status == TodoStatus::InProgress {
                    todo.status = TodoStatus::Completed;
                    todo.updated_at = now;
                    break;
                }
            }
            
            // 找到下一个 pending 并标记为 in_progress
            let mut next = None;
            for todo in todos.iter_mut() {
                if todo.status == TodoStatus::Pending {
                    todo.status = TodoStatus::InProgress;
                    todo.updated_at = now;
                    next = Some(todo.clone());
                    break;
                }
            }
            
            let current = todos.clone();
            drop(store);
            self.emit_todos_update(execution_id, &current);
            
            return next;
        }
        None
    }

    /// 清除 todos
    pub async fn clear(&self, execution_id: &str) {
        let mut store = self.todos.write().await;
        store.remove(execution_id);
    }

    /// 清除所有 todos
    pub async fn clear_all(&self) {
        let mut store = self.todos.write().await;
        store.clear();
    }

    /// 发送 todos 更新事件到前端
    fn emit_todos_update(&self, execution_id: &str, todos: &[Todo]) {
        if let Some(app) = &self.app_handle {
            let payload = TodosUpdatePayload {
                execution_id: execution_id.to_string(),
                todos: todos.to_vec(),
                timestamp: chrono::Utc::now().timestamp_millis(),
            };
            
            if let Err(e) = app.emit("agent-todos-update", &payload) {
                tracing::warn!("Failed to emit todos update: {}", e);
            }
        }
    }
}

/// Todo 统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoStats {
    pub total: usize,
    pub pending: usize,
    pub in_progress: usize,
    pub completed: usize,
}

impl TodoStats {
    /// 计算完成百分比
    pub fn progress_percent(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            (self.completed as f32 / self.total as f32) * 100.0
        }
    }

    /// 检查是否全部完成
    pub fn is_all_completed(&self) -> bool {
        self.total > 0 && self.completed == self.total
    }
}

/// TodoWrite 工具的输入参数（与 Claude Code 一致）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoWriteInput {
    pub todos: Vec<TodoItem>,
}

/// 单个 Todo 项的输入（与 Claude Code 一致）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub content: String,
    pub status: TodoStatus,
    #[serde(rename = "activeForm")]
    pub active_form: String,
}

impl From<TodoItem> for Todo {
    fn from(item: TodoItem) -> Self {
        Todo::with_forms(
            uuid::Uuid::new_v4().to_string(),
            item.content,
            item.active_form,
        ).with_status(item.status)
    }
}

/// 截断内容到指定长度
fn truncate_content(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        s.chars().take(max_len - 3).collect::<String>() + "..."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_todo_crud() {
        let manager = TodoManager::new(None);
        let execution_id = "test-exec-1";

        // 创建 todos（使用新的 with_forms 方法）
        let todos = vec![
            Todo::with_forms("1", "Run tests", "Running tests")
                .with_status(TodoStatus::InProgress),
            Todo::with_forms("2", "Build project", "Building project"),
            Todo::with_forms("3", "Fix subtask", "Fixing subtask").with_parent("1"),
        ];

        manager.write_todos(execution_id, todos, false).await.unwrap();

        // 验证
        let result = manager.get_todos(execution_id).await;
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].status, TodoStatus::InProgress);
        assert_eq!(result[0].active_form, Some("Running tests".to_string()));

        // 更新状态
        manager.update_status(execution_id, "1", TodoStatus::Completed).await.unwrap();
        
        let result = manager.get_todos(execution_id).await;
        assert_eq!(result[0].status, TodoStatus::Completed);

        // 统计
        let stats = manager.get_stats(execution_id).await;
        assert_eq!(stats.total, 3);
        assert_eq!(stats.completed, 1);
        assert_eq!(stats.pending, 2);
    }

    #[tokio::test]
    async fn test_merge_mode() {
        let manager = TodoManager::new(None);
        let execution_id = "test-exec-2";

        // 初始 todos
        let todos = vec![
            Todo::with_forms("1", "Task 1", "Doing task 1"),
            Todo::with_forms("2", "Task 2", "Doing task 2"),
        ];
        manager.write_todos(execution_id, todos, false).await.unwrap();

        // 合并新 todos
        let new_todos = vec![
            Todo::with_forms("2", "Task 2 Updated", "Updating task 2")
                .with_status(TodoStatus::Completed),
            Todo::with_forms("3", "Task 3", "Doing task 3"),
        ];
        manager.write_todos(execution_id, new_todos, true).await.unwrap();

        let result = manager.get_todos(execution_id).await;
        assert_eq!(result.len(), 3);
        assert_eq!(result.iter().find(|t| t.id == "2").unwrap().status, TodoStatus::Completed);
    }

    #[tokio::test]
    async fn test_start_next_and_complete() {
        let manager = TodoManager::new(None);
        let execution_id = "test-exec-3";

        // 创建 todos
        let todos = vec![
            Todo::with_forms("1", "First task", "Doing first task"),
            Todo::with_forms("2", "Second task", "Doing second task"),
            Todo::with_forms("3", "Third task", "Doing third task"),
        ];
        manager.write_todos(execution_id, todos, false).await.unwrap();

        // 开始第一个任务
        let started = manager.start_next(execution_id).await;
        assert!(started.is_some());
        assert_eq!(started.unwrap().id, "1");

        // 完成当前并开始下一个
        let next = manager.complete_current_and_start_next(execution_id).await;
        assert!(next.is_some());
        assert_eq!(next.unwrap().id, "2");

        // 检查状态
        let stats = manager.get_stats(execution_id).await;
        assert_eq!(stats.completed, 1);
        assert_eq!(stats.in_progress, 1);
        assert_eq!(stats.pending, 1);
    }

    #[tokio::test]
    async fn test_todo_write_input() {
        let manager = TodoManager::new(None);
        let execution_id = "test-exec-4";

        // 模拟 Claude Code 的 TodoWrite 输入
        let input = TodoWriteInput {
            todos: vec![
                TodoItem {
                    content: "Run the build".to_string(),
                    status: TodoStatus::InProgress,
                    active_form: "Running the build".to_string(),
                },
                TodoItem {
                    content: "Fix type errors".to_string(),
                    status: TodoStatus::Pending,
                    active_form: "Fixing type errors".to_string(),
                },
            ],
        };

        manager.write_from_input(execution_id, input).await.unwrap();

        let result = manager.get_todos(execution_id).await;
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].status, TodoStatus::InProgress);
        assert_eq!(result[0].active_form, Some("Running the build".to_string()));
    }

    #[test]
    fn test_truncate_content() {
        assert_eq!(truncate_content("short", 70), "short");
        
        let long = "a".repeat(100);
        let truncated = truncate_content(&long, 70);
        assert_eq!(truncated.chars().count(), 70);
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_todo_display_text() {
        let todo = Todo::with_forms("1", "Run tests", "Running tests")
            .with_status(TodoStatus::InProgress);
        assert_eq!(todo.display_text(), "Running tests");

        let todo_pending = Todo::with_forms("2", "Build project", "Building project");
        assert_eq!(todo_pending.display_text(), "Build project");
    }

    #[test]
    fn test_stats_progress() {
        let stats = TodoStats {
            total: 10,
            pending: 3,
            in_progress: 1,
            completed: 6,
        };
        assert_eq!(stats.progress_percent(), 60.0);
        assert!(!stats.is_all_completed());

        let complete_stats = TodoStats {
            total: 5,
            pending: 0,
            in_progress: 0,
            completed: 5,
        };
        assert!(complete_stats.is_all_completed());
    }
}

