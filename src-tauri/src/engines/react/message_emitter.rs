//! ReAct 消息发送器
//!
//! 简化版：直接发送流式内容到前端，并收集完整内容用于保存

use crate::utils::ordered_message::{emit_message_chunk_with_arch, ArchitectureType, ChunkType};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::AppHandle;

/// ReAct 消息发送器
pub struct ReactMessageEmitter {
    app_handle: Arc<AppHandle>,
    execution_id: String,
    message_id: String,
    conversation_id: Option<String>,
    /// 收集所有发送的内容，用于保存到数据库
    content_collector: Arc<Mutex<String>>,
}

/// 执行统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactExecutionStats {
    pub total_iterations: u32,
    pub tool_calls_count: u32,
    pub successful_tool_calls: u32,
    pub failed_tool_calls: u32,
    pub total_duration_ms: u64,
    pub status: String,
}

impl ReactMessageEmitter {
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
            content_collector: Arc::new(Mutex::new(String::new())),
        }
    }

    /// 获取收集的完整内容（用于保存到数据库）
    pub fn get_full_content(&self) -> String {
        self.content_collector.lock().unwrap().clone()
    }

    /// 发送执行开始信号
    pub fn emit_start(&self, config: Option<serde_json::Value>) {
        self.emit_meta("start", serde_json::json!({
            "type": "start",
            "config": config
        }));
    }

    /// 发送执行完成信号
    pub fn emit_complete(&self, stats: ReactExecutionStats) {
        // 发送完成信号（is_final = true）
        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::Meta,
            "",
            true, // is_final
            Some("complete"),
            None,
            Some(ArchitectureType::ReAct),
            Some(serde_json::json!({
                "type": "complete",
                "statistics": stats
            })),
        );
    }

    /// 发送流式内容 chunk（LLM 输出的每个 token）
    pub fn emit_content(&self, content: &str, is_final: bool) {
        // 收集内容用于保存到数据库
        if let Ok(mut collector) = self.content_collector.lock() {
            collector.push_str(content);
        }
        
        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::Content,
            content,
            is_final,
            None,
            None,
            Some(ArchitectureType::ReAct),
            None,
        );
    }

    /// 发送思考内容 chunk（用于显示 LLM 的 reasoning 过程）
    pub fn emit_thinking(&self, content: &str) {
        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::Thinking,
            content,
            false,
            None,
            None,
            Some(ArchitectureType::ReAct),
            None,
        );
    }

    /// 发送工具调用信息（内联 markdown 格式 + 结构化数据）
    pub fn emit_tool_call(&self, iteration: u32, tool_name: &str, args: &serde_json::Value) {
        let args_str = serde_json::to_string_pretty(args).unwrap_or_default();
        let content = format!(
            "\n\n---\n🔧 **调用工具: `{}`**\n<details>\n<summary>📥 参数</summary>\n\n```json\n{}\n```\n</details>\n",
            tool_name, args_str
        );
        self.emit_content(&content, false);

        // 同时发送结构化数据（用于状态追踪）
        self.emit_step("action", serde_json::json!({
            "type": "step",
            "step": {
                "index": iteration.saturating_sub(1),
                "action": {
                    "tool": tool_name,
                    "args": args,
                    "status": "running"
                }
            }
        }));
    }

    /// 发送工具执行结果（内联 markdown 格式 + 结构化数据）
    pub fn emit_tool_result(&self, iteration: u32, tool_name: &str, args: &serde_json::Value, result: &serde_json::Value, success: bool, duration_ms: u64) {
        let status_icon = if success { "✅" } else { "❌" };
        let result_str = serde_json::to_string_pretty(result).unwrap_or_default();
        let content = format!(
            "<details>\n<summary>{} 结果 ({}ms)</summary>\n\n```json\n{}\n```\n</details>\n---\n\n",
            status_icon, duration_ms, result_str
        );
        self.emit_content(&content, false);

        // 同时发送结构化数据（用于状态追踪）
        let status = if success { "completed" } else { "failed" };
        self.emit_step("observation", serde_json::json!({
            "type": "step",
            "step": {
                "index": iteration.saturating_sub(1),
                "action": {
                    "tool": tool_name,
                    "args": args,
                    "status": status
                },
                "observation": result
            }
        }));
    }

    /// 发送步骤数据
    fn emit_step(&self, stage: &str, data: serde_json::Value) {
        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::Meta,
            "",
            false,
            Some(stage),
            None,
            Some(ArchitectureType::ReAct),
            Some(data),
        );
    }

    // === 内部方法 ===

    fn emit_meta(&self, stage: &str, data: serde_json::Value) {
        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::Meta,
            "",
            false,
            Some(stage),
            None,
            Some(ArchitectureType::ReAct),
            Some(data),
        );
    }
}
