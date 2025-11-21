//! 统一的有序消息流模块
//! 
//! 用于替代复杂的UnifiedStreamMessage，提供简化的时序消息处理

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use tauri::{AppHandle, Emitter};

/// 架构类型标识
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ArchitectureType {
    ReAct,
    ReWOO,
    LLMCompiler,
    PlanAndExecute,
    Travel,
    Unknown,
}

/// 消息块类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChunkType {
    /// 主要内容
    Content,
    /// AI思考过程
    Thinking,
    /// 工具执行结果
    ToolResult,
    /// 计划信息
    PlanInfo,
    /// 错误信息
    Error,
    /// 元数据信息
    Meta,
    /// 流完成信号
    StreamComplete,
}

/// 有序消息块
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderedMessageChunk {
    /// 执行ID
    pub execution_id: String,
    /// 消息ID
    pub message_id: String,
    /// 会话ID（可选）
    pub conversation_id: Option<String>,
    /// 严格递增的序号
    pub sequence: u64,
    /// 消息块类型
    pub chunk_type: ChunkType,
    /// 内容
    pub content: String,
    /// 时间戳
    pub timestamp: SystemTime,
    /// 是否为最后一个块
    pub is_final: bool,
    /// 阶段标识（可选）
    pub stage: Option<String>,
    /// 工具名称
    pub tool_name: Option<String>,
    /// 架构类型标识
    pub architecture: Option<ArchitectureType>,
    /// 架构特定的结构化数据
    pub structured_data: Option<serde_json::Value>,
}

/// 每个执行的序号分配器
static SEQUENCE_COUNTERS: std::sync::LazyLock<Mutex<HashMap<String, u64>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// 获取下一个序号
fn next_sequence_for(execution_id: &str) -> u64 {
    if let Ok(mut counters) = SEQUENCE_COUNTERS.lock() {
        let entry = counters.entry(execution_id.to_string()).or_insert(0);
        *entry += 1;
        *entry
    } else {
        log::error!("Failed to acquire sequence counter lock for execution_id: {}", execution_id);
        1
    }
}

/// 清理执行ID的序号计数器（执行完成后调用）
pub fn cleanup_sequence_counter(execution_id: &str) {
    if let Ok(mut counters) = SEQUENCE_COUNTERS.lock() {
        counters.remove(execution_id);
        log::debug!("Cleaned up sequence counter for execution_id: {}", execution_id);
    }
}

/// 统一的消息块发送函数
pub fn emit_message_chunk(
    app_handle: &AppHandle,
    execution_id: &str,
    message_id: &str,
    conversation_id: Option<&str>,
    chunk_type: ChunkType,
    content: &str,
    is_final: bool,
    stage: Option<&str>,
    tool_name: Option<&str>,
) {
    emit_message_chunk_with_arch(
        app_handle,
        execution_id,
        message_id,
        conversation_id,
        chunk_type,
        content,
        is_final,
        stage,
        tool_name,
        None,
        None,
    );
}

/// 带架构信息的消息块发送函数
pub fn emit_message_chunk_with_arch(
    app_handle: &AppHandle,
    execution_id: &str,
    message_id: &str,
    conversation_id: Option<&str>,
    chunk_type: ChunkType,
    content: &str,
    is_final: bool,
    stage: Option<&str>,
    tool_name: Option<&str>,
    architecture: Option<ArchitectureType>,
    structured_data: Option<serde_json::Value>,
) {
    // 使用 message_id 作为序号计数的键，确保同一条前端消息的所有来源（LLM流、工具结果、Meta）
    // 共享一个严格递增的序列，从根本上消除跨 execution_id 的交错问题
    let sequence_key = format!("msg:{}", message_id);
    let sequence = next_sequence_for(&sequence_key);
    
    let chunk = OrderedMessageChunk {
        execution_id: execution_id.to_string(),
        message_id: message_id.to_string(),
        conversation_id: conversation_id.map(|s| s.to_string()),
        sequence,
        chunk_type,
        content: content.to_string(),
        timestamp: SystemTime::now(),
        is_final,
        stage: stage.map(|s| s.to_string()),
        tool_name: tool_name.map(|s| s.to_string()),
        architecture,
        structured_data,
    };

    log::debug!(
        "Emitting message chunk: execution_id={}, message_id={}, sequence={}, type={:?}, content_len={}, is_final={}, arch={:?}",
        execution_id, message_id, sequence, chunk.chunk_type, content.len(), is_final, chunk.architecture
    );

    if let Err(e) = app_handle.emit("message_chunk", &chunk) {
        log::error!("Failed to emit message chunk: {}", e);
    }
}

/// Arc包装版本，用于多线程环境
pub fn emit_message_chunk_arc(
    app_handle: &Arc<AppHandle>,
    execution_id: &str,
    message_id: &str,
    conversation_id: Option<&str>,
    chunk_type: ChunkType,
    content: &str,
    is_final: bool,
    stage: Option<&str>,
    tool_name: Option<&str>,
    architecture: Option<ArchitectureType>,
    structured_data: Option<serde_json::Value>,
) {
    emit_message_chunk_with_arch(
        app_handle.as_ref(),
        execution_id,
        message_id,
        conversation_id,
        chunk_type,
        content,
        is_final,
        stage,
        tool_name,
        architecture,
        structured_data,
    );
}

/// 便捷函数：发送内容块
pub fn emit_content_chunk(
    app_handle: &AppHandle,
    execution_id: &str,
    message_id: &str,
    conversation_id: Option<&str>,
    content: &str,
    is_final: bool,
) {
    emit_message_chunk(
        app_handle,
        execution_id,
        message_id,
        conversation_id,
        ChunkType::Content,
        content,
        is_final,
        None,
        None,
    );
}

/// 便捷函数：发送思考块
pub fn emit_thinking_chunk(
    app_handle: &AppHandle,
    execution_id: &str,
    message_id: &str,
    conversation_id: Option<&str>,
    thinking: &str,
    stage: Option<&str>,
) {
    emit_message_chunk(
        app_handle,
        execution_id,
        message_id,
        conversation_id,
        ChunkType::Thinking,
        thinking,
        false,
        stage,
        None,
    );
}

/// Emit thinking chunk with tool name
pub fn emit_thinking_chunk_with_tool(
    app_handle: &AppHandle,
    execution_id: &str,
    message_id: &str,
    conversation_id: Option<&str>,
    thinking: &str,
    stage: Option<&str>,
    tool_name: Option<&str>,
) {
    emit_message_chunk(
        app_handle,
        execution_id,
        message_id,
        conversation_id,
        ChunkType::Thinking,
        thinking,
        false,
        stage,
        tool_name,
    );
}

/// 便捷函数：发送工具结果块
pub fn emit_tool_result_chunk(
    app_handle: &AppHandle,
    execution_id: &str,
    message_id: &str,
    conversation_id: Option<&str>,
    tool_result: &str,
    stage: Option<&str>,
    tool_name: Option<&str>,
) {
    emit_message_chunk(
        app_handle,
        execution_id,
        message_id,
        conversation_id,
        ChunkType::ToolResult,
        tool_result,
        false,
        stage,
        tool_name,

    );
}

/// 便捷函数：发送计划信息块
pub fn emit_plan_info_chunk(
    app_handle: &AppHandle,
    execution_id: &str,
    message_id: &str,
    conversation_id: Option<&str>,
    plan_info: &str,
    stage: Option<&str>,
    tool_name: Option<&str>,
) {
    emit_message_chunk(
        app_handle,
        execution_id,
        message_id,
        conversation_id,
        ChunkType::PlanInfo,
        plan_info,
        false,
        stage,
        tool_name,
    );
}

/// 便捷函数：发送错误块
pub fn emit_error_chunk(
    app_handle: &AppHandle,
    execution_id: &str,
    message_id: &str,
    conversation_id: Option<&str>,
    error: &str,
    stage: Option<&str>,
    tool_name: Option<&str>,
) {
    emit_message_chunk(
        app_handle,
        execution_id,
        message_id,
        conversation_id,
        ChunkType::Error,
        error,
        true, // 错误通常是最终的
        stage,
        tool_name,
    );
}

/// 便捷函数：发送元数据块
pub fn emit_meta_chunk(
    app_handle: &AppHandle,
    execution_id: &str,
    message_id: &str,
    conversation_id: Option<&str>,
    meta_info: &str,
    tool_name: Option<&str>,
) {
    emit_message_chunk(
        app_handle,
        execution_id,
        message_id,
        conversation_id,
        ChunkType::Meta,
        meta_info,
        false,
        None,
        tool_name,
    );
}

impl ChunkType {
    /// 获取块类型的显示标签
    pub fn display_label(&self) -> &'static str {
        match self {
            ChunkType::Content => "",
            ChunkType::Thinking => "🤔 **思考过程**",
            ChunkType::ToolResult => "🔧 **工具执行**",
            ChunkType::PlanInfo => "📋 **执行计划**",
            ChunkType::Error => "❌ **错误**",
            ChunkType::Meta => "ℹ️ **元数据**",
            ChunkType::StreamComplete => "✅ **完成**",
        }
    }

    /// 检查是否需要在内容前添加标签
    pub fn needs_label(&self) -> bool {
        !matches!(self, ChunkType::Content)
    }
}

impl OrderedMessageChunk {
    /// 格式化为markdown内容
    pub fn to_markdown(&self) -> String {
        if self.chunk_type.needs_label() {
            format!("{}\n{}", self.chunk_type.display_label(), self.content)
        } else {
            self.content.clone()
        }
    }
    
    /// 检查是否为错误块
    pub fn is_error(&self) -> bool {
        matches!(self.chunk_type, ChunkType::Error)
    }
    
    /// 检查是否为内容块
    pub fn is_content(&self) -> bool {
        matches!(self.chunk_type, ChunkType::Content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sequence_generation() {
        let exec_id = "test_exec_1";
        
        let seq1 = next_sequence_for(exec_id);
        let seq2 = next_sequence_for(exec_id);
        let seq3 = next_sequence_for(exec_id);
        
        assert_eq!(seq1, 1);
        assert_eq!(seq2, 2);
        assert_eq!(seq3, 3);
        
        cleanup_sequence_counter(exec_id);
        
        let seq4 = next_sequence_for(exec_id);
        assert_eq!(seq4, 1);
    }

    #[test]
    fn test_chunk_type_labels() {
        assert_eq!(ChunkType::Content.display_label(), "");
        assert_eq!(ChunkType::Thinking.display_label(), "🤔 **思考过程**");
        assert_eq!(ChunkType::ToolResult.display_label(), "🔧 **工具执行**");
        assert_eq!(ChunkType::Error.display_label(), "❌ **错误**");
        
        assert!(!ChunkType::Content.needs_label());
        assert!(ChunkType::Thinking.needs_label());
        assert!(ChunkType::Error.needs_label());
    }

    #[test]
    fn test_markdown_formatting() {
        let content_chunk = OrderedMessageChunk {
            execution_id: "test".to_string(),
            message_id: "msg1".to_string(),
            conversation_id: None,
            sequence: 1,
            chunk_type: ChunkType::Content,
            content: "Hello world".to_string(),
            timestamp: SystemTime::now(),
            is_final: false,
            stage: None,
            tool_name: None,
            architecture: todo!(),
            structured_data: todo!(),
        };
        
        let thinking_chunk = OrderedMessageChunk {
            execution_id: "test".to_string(),
            message_id: "msg1".to_string(),
            conversation_id: None,
            sequence: 2,
            chunk_type: ChunkType::Thinking,
            content: "Let me think...".to_string(),
            timestamp: SystemTime::now(),
            is_final: false,
            stage: None,
            tool_name: None,
            architecture: todo!(),
            structured_data: todo!(),
        };
        
        assert_eq!(content_chunk.to_markdown(), "Hello world");
        assert_eq!(thinking_chunk.to_markdown(), "🤔 **思考过程**\nLet me think...");
    }
}
