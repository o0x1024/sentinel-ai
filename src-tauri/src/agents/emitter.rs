//! Agent 消息发送器
//!
//! 统一的消息发送接口，支持流式输出到前端

use super::types::{AgentMessage, MessageType, StepResult};
use super::todo_manager::Todo;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// 消息发送器
pub struct AgentMessageEmitter {
    app_handle: Option<AppHandle>,
    execution_id: String,
    message_id: Option<String>,
}

impl AgentMessageEmitter {
    pub fn new(app_handle: Option<AppHandle>, execution_id: impl Into<String>) -> Self {
        Self {
            app_handle,
            execution_id: execution_id.into(),
            message_id: None,
        }
    }

    pub fn with_message_id(mut self, message_id: impl Into<String>) -> Self {
        self.message_id = Some(message_id.into());
        self
    }

    /// 发送开始事件
    pub fn emit_start(&self, task: &str) {
        self.emit_event("agent-start", AgentStartPayload {
            execution_id: self.execution_id.clone(),
            task: task.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
        });
    }

    /// 发送消息事件
    pub fn emit_message(&self, message: &AgentMessage) {
        self.emit_event("agent-message", AgentMessagePayload {
            execution_id: self.execution_id.clone(),
            message: message.clone(),
        });
    }

    /// 发送思考内容
    pub fn emit_thinking(&self, content: &str) {
        let msg = AgentMessage::thinking(content);
        self.emit_message(&msg);
    }

    /// 发送规划内容
    pub fn emit_plan(&self, content: &str) {
        let msg = AgentMessage::planning(content);
        self.emit_message(&msg);
    }

    /// 发送工具调用
    pub fn emit_tool_call(&self, tool_name: &str, args: &serde_json::Value) {
        let msg = AgentMessage::tool_call(tool_name, args);
        self.emit_message(&msg);
    }

    /// 发送工具结果
    pub fn emit_tool_result(&self, tool_name: &str, result: &str, success: bool, duration_ms: u64) {
        let msg = AgentMessage::tool_result(tool_name, result, success, duration_ms);
        self.emit_message(&msg);
    }

    /// 发送进度更新
    pub fn emit_progress(&self, step_index: usize, total_steps: usize, content: &str) {
        let msg = AgentMessage::progress(step_index, total_steps, content);
        self.emit_message(&msg);
    }

    /// 发送最终答案
    pub fn emit_final(&self, answer: &str) {
        let msg = AgentMessage::final_answer(answer);
        self.emit_message(&msg);
    }

    /// 发送错误
    pub fn emit_error(&self, error: &str) {
        let msg = AgentMessage::error(error);
        self.emit_message(&msg);
    }

    /// 发送完成事件
    pub fn emit_complete(&self, success: bool, data: Option<serde_json::Value>) {
        self.emit_event("agent-complete", AgentCompletePayload {
            execution_id: self.execution_id.clone(),
            success,
            data,
            timestamp: chrono::Utc::now().timestamp_millis(),
        });
    }

    /// 发送 Todos 更新
    pub fn emit_todos_update(&self, todos: &[Todo]) {
        self.emit_event("agent-todos-update", TodosUpdatePayload {
            execution_id: self.execution_id.clone(),
            todos: todos.to_vec(),
            timestamp: chrono::Utc::now().timestamp_millis(),
        });
    }

    /// 发送流式内容块
    pub fn emit_content_chunk(&self, chunk: &str, is_complete: bool) {
        self.emit_event("agent-content", ContentChunkPayload {
            execution_id: self.execution_id.clone(),
            message_id: self.message_id.clone(),
            chunk: chunk.to_string(),
            is_complete,
            timestamp: chrono::Utc::now().timestamp_millis(),
        });
    }

    /// 通用事件发送
    fn emit_event<T: Serialize + Clone>(&self, event_name: &str, payload: T) {
        if let Some(app) = &self.app_handle {
            if let Err(e) = app.emit(event_name, &payload) {
                tracing::warn!("Failed to emit event {}: {}", event_name, e);
            }
        }
    }
}

// ============ 事件 Payload 定义 ============

#[derive(Debug, Clone, Serialize)]
pub struct AgentStartPayload {
    pub execution_id: String,
    pub task: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentMessagePayload {
    pub execution_id: String,
    pub message: AgentMessage,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentCompletePayload {
    pub execution_id: String,
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TodosUpdatePayload {
    pub execution_id: String,
    pub todos: Vec<Todo>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContentChunkPayload {
    pub execution_id: String,
    pub message_id: Option<String>,
    pub chunk: String,
    pub is_complete: bool,
    pub timestamp: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emitter_creation() {
        let emitter = AgentMessageEmitter::new(None, "exec-1");
        assert_eq!(emitter.execution_id, "exec-1");
    }

    #[test]
    fn test_emitter_with_message_id() {
        let emitter = AgentMessageEmitter::new(None, "exec-1")
            .with_message_id("msg-1");
        assert_eq!(emitter.message_id, Some("msg-1".to_string()));
    }
}

