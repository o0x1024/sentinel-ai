//! LLM Compiler Message Emitter
//!
//! Unified message emission for the LLM Compiler engine.
//! Provides consistent message formatting and sending to the frontend.

use crate::utils::ordered_message::{
    emit_error_chunk, emit_message_chunk_with_arch, emit_meta_chunk, emit_plan_info_chunk,
    ArchitectureType, ChunkType,
};
use std::sync::Arc;
use tauri::AppHandle;
use tracing::debug;

/// LLM Compiler Message Emitter
/// Provides unified message emission interface for all LLM Compiler components
#[derive(Clone)]
pub struct LlmCompilerMessageEmitter {
    app_handle: Arc<AppHandle>,
    execution_id: String,
    message_id: String,
    conversation_id: Option<String>,
}

impl LlmCompilerMessageEmitter {
    /// Create a new message emitter
    pub fn new(
        app_handle: Arc<AppHandle>,
        execution_id: String,
        message_id: String,
        conversation_id: Option<String>,
    ) -> Self {
        Self {
            app_handle,
            execution_id,
            message_id,
            conversation_id,
        }
    }

    /// Get the architecture type
    fn arch_type() -> ArchitectureType {
        ArchitectureType::LLMCompiler
    }

    /// Emit a thinking/reasoning message
    pub fn emit_thinking(&self, content: &str, stage: &str) {
        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::Thinking,
            content,
            false,
            Some(stage),
            None,
            Some(Self::arch_type()),
            None,
        );
        debug!("LLMCompiler emitted thinking: stage={}", stage);
    }

    /// Emit a planning stage message
    pub fn emit_planning(&self, content: &str) {
        self.emit_thinking(content, "llm_compiler_planning");
    }

    /// Emit a joiner/evaluation stage message
    pub fn emit_joiner(&self, content: &str) {
        self.emit_thinking(content, "llm_compiler_joiner");
    }

    /// Emit a plan information message
    pub fn emit_plan_info(&self, content: &str) {
        emit_plan_info_chunk(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            content,
            Some("llm_compiler_planning"),
            None,
        );
        debug!("LLMCompiler emitted plan info");
    }

    /// Emit a tool call message (using Meta chunk type with tool_call indicator)
    pub fn emit_tool_call(&self, tool_name: &str, task_id: &str) {
        let content = serde_json::json!({
            "type": "tool_call",
            "tool_name": tool_name,
            "task_id": task_id,
            "status": "calling"
        })
        .to_string();

        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::Meta,
            &content,
            false,
            Some("llm_compiler_tool_call"),
            Some(tool_name),
            Some(Self::arch_type()),
            None,
        );
        debug!(
            "LLMCompiler emitted tool call: tool={}, task={}",
            tool_name, task_id
        );
    }

    /// Emit a tool result message
    pub fn emit_tool_result(&self, tool_name: &str, task_id: &str, result: &str, success: bool) {
        let content = if success {
            result.to_string()
        } else {
            serde_json::json!({
                "error": result,
                "success": false,
                "task_id": task_id
            })
            .to_string()
        };

        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::ToolResult,
            &content,
            false,
            Some("llm_compiler"),
            Some(tool_name),
            Some(Self::arch_type()),
            None,
        );
        debug!(
            "LLMCompiler emitted tool result: tool={}, task={}, success={}",
            tool_name, task_id, success
        );
    }

    /// Emit a content message (final response streaming)
    pub fn emit_content(&self, content: &str, is_complete: bool) {
        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::Content,
            content,
            is_complete,
            None,
            None,
            Some(Self::arch_type()),
            None,
        );
    }

    /// Emit an error message
    pub fn emit_error(&self, error_msg: &str, stage: Option<&str>) {
        emit_error_chunk(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            error_msg,
            stage,
            None,
        );
        debug!("LLMCompiler emitted error: stage={:?}", stage);
    }

    /// Emit a meta/decision message
    pub fn emit_meta(&self, content: &str) {
        emit_meta_chunk(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            content,
            None,
        );
        debug!("LLMCompiler emitted meta");
    }

    /// Emit stream complete signal
    pub fn emit_stream_complete(&self) {
        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::StreamComplete,
            "",
            true,
            None,
            None,
            Some(Self::arch_type()),
            Some(serde_json::json!({"stream_complete": true})),
        );
        debug!("LLMCompiler emitted stream complete");
    }

    /// Emit execution progress
    pub fn emit_progress(&self, completed: usize, total: usize, current_task: Option<&str>) {
        let content = serde_json::json!({
            "completed": completed,
            "total": total,
            "current_task": current_task,
            "progress_percentage": if total > 0 { (completed as f64 / total as f64) * 100.0 } else { 0.0 }
        })
        .to_string();

        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::Meta,
            &content,
            false,
            Some("llm_compiler_progress"),
            None,
            Some(Self::arch_type()),
            None,
        );
    }

    /// Emit replanning notification
    pub fn emit_replanning(&self, round: usize, reason: &str) {
        let content = format!("重规划 (轮次 {}): {}", round, reason);
        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::Thinking,
            &content,
            false,
            Some("llm_compiler_replanning"),
            None,
            Some(Self::arch_type()),
            None,
        );
        debug!("LLMCompiler emitted replanning: round={}", round);
    }

    /// Emit DAG visualization data
    pub fn emit_dag_visualization(&self, nodes: &[serde_json::Value], edges: &[serde_json::Value]) {
        let content = serde_json::json!({
            "type": "dag_visualization",
            "nodes": nodes,
            "edges": edges
        })
        .to_string();

        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::Meta,
            &content,
            false,
            Some("llm_compiler_dag"),
            None,
            Some(Self::arch_type()),
            None,
        );
        debug!("LLMCompiler emitted DAG visualization");
    }

    /// Get execution ID
    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }

    /// Get message ID
    pub fn message_id(&self) -> &str {
        &self.message_id
    }

    /// Get conversation ID
    pub fn conversation_id(&self) -> Option<&str> {
        self.conversation_id.as_deref()
    }
}

/// Builder for LlmCompilerMessageEmitter
pub struct LlmCompilerMessageEmitterBuilder {
    app_handle: Option<Arc<AppHandle>>,
    execution_id: Option<String>,
    message_id: Option<String>,
    conversation_id: Option<String>,
}

impl LlmCompilerMessageEmitterBuilder {
    pub fn new() -> Self {
        Self {
            app_handle: None,
            execution_id: None,
            message_id: None,
            conversation_id: None,
        }
    }

    pub fn app_handle(mut self, handle: Arc<AppHandle>) -> Self {
        self.app_handle = Some(handle);
        self
    }

    pub fn execution_id(mut self, id: String) -> Self {
        self.execution_id = Some(id);
        self
    }

    pub fn message_id(mut self, id: String) -> Self {
        self.message_id = Some(id);
        self
    }

    pub fn conversation_id(mut self, id: Option<String>) -> Self {
        self.conversation_id = id;
        self
    }

    pub fn build(self) -> Option<LlmCompilerMessageEmitter> {
        let app_handle = self.app_handle?;
        let execution_id = self.execution_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let message_id = self.message_id.unwrap_or_else(|| execution_id.clone());

        Some(LlmCompilerMessageEmitter::new(
            app_handle,
            execution_id,
            message_id,
            self.conversation_id,
        ))
    }
}

impl Default for LlmCompilerMessageEmitterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

