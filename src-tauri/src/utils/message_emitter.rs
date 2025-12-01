//! 标准化消息发送器
//! 
//! 为各个架构提供统一的消息发送接口，确保消息格式一致性

use serde_json::Value;
use std::sync::Arc;
use tauri::AppHandle;

use super::ordered_message::{
    emit_message_chunk_with_arch, ArchitectureType, ChunkType,
};

/// 标准消息发送器
pub struct StandardMessageEmitter {
    app_handle: Arc<AppHandle>,
    execution_id: String,
    message_id: String,
    conversation_id: Option<String>,
    architecture: ArchitectureType,
}

impl StandardMessageEmitter {
    /// 创建新的消息发送器
    pub fn new(
        app_handle: Arc<AppHandle>,
        execution_id: String,
        message_id: String,
        conversation_id: Option<String>,
        architecture: ArchitectureType,
    ) -> Self {
        Self {
            app_handle,
            execution_id,
            message_id,
            conversation_id,
            architecture,
        }
    }

    /// 发送架构开始信号
    pub fn emit_start(&self, plan_summary: Option<Value>) {
        let meta_data = serde_json::json!({
            "type": "architecture_start",
            "architecture": format!("{:?}", self.architecture),
            "plan_summary": plan_summary,
        });

        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::Meta,
            &meta_data.to_string(),
            false,
            Some("start"),
            None,
            Some(self.architecture.clone()),
            Some(meta_data),
        );

        log::info!(
            "Architecture {:?} started: execution_id={}, message_id={}",
            self.architecture,
            self.execution_id,
            self.message_id
        );
    }

    /// 发送思考内容
    pub fn emit_thinking(&self, content: &str) {
        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::Thinking,
            content,
            false,
            Some(&format!("{:?}", self.architecture).to_lowercase()),
            None,
            Some(self.architecture.clone()),
            None,
        );
    }

    /// 发送内容块
    pub fn emit_content(&self, content: &str, is_final: bool) {
        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::Content,
            content,
            is_final,
            Some(&format!("{:?}", self.architecture).to_lowercase()),
            None,
            Some(self.architecture.clone()),
            None,
        );
    }

    /// 发送工具结果（强制要求tool_name）
    pub fn emit_tool_result(&self, tool_name: &str, result: &Value) {
        let result_str = result.to_string();
        
        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::ToolResult,
            &result_str,
            false,
            Some(&format!("{:?}", self.architecture).to_lowercase()),
            Some(tool_name),
            Some(self.architecture.clone()),
            Some(result.clone()),
        );

        log::debug!(
            "Tool result emitted: tool={}, execution_id={}, message_id={}",
            tool_name,
            self.execution_id,
            self.message_id
        );
    }

    /// 发送计划信息
    pub fn emit_plan(&self, plan_info: &str) {
        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::PlanInfo,
            plan_info,
            false,
            Some(&format!("{:?}", self.architecture).to_lowercase()),
            None,
            Some(self.architecture.clone()),
            None,
        );
    }

    /// 发送步骤更新
    pub fn emit_step_update(&self, step_index: usize, step_name: &str, status: &str) {
        let meta_data = serde_json::json!({
            "type": "step_update",
            "step_index": step_index,
            "step_name": step_name,
            "status": status,
        });

        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::Meta,
            &meta_data.to_string(),
            false,
            Some(&format!("{:?}", self.architecture).to_lowercase()),
            None,
            Some(self.architecture.clone()),
            Some(meta_data),
        );
    }

    /// 发送错误信息
    pub fn emit_error(&self, error: &str) {
        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::Error,
            error,
            true,
            Some(&format!("{:?}", self.architecture).to_lowercase()),
            None,
            Some(self.architecture.clone()),
            None,
        );
    }

    /// 发送元数据
    pub fn emit_meta(&self, meta_content: &str) {
        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::Meta,
            meta_content,
            false,
            Some(&format!("{:?}", self.architecture).to_lowercase()),
            None,
            Some(self.architecture.clone()),
            None,
        );
    }

    /// 发送完成信号（必须调用）
    pub fn emit_complete(&self, summary: Option<Value>) {
        let complete_data = serde_json::json!({
            "type": "stream_complete",
            "architecture": format!("{:?}", self.architecture),
            "summary": summary,
        });

        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::StreamComplete,
            &complete_data.to_string(),
            true,
            Some("complete"),
            None,
            Some(self.architecture.clone()),
            Some(complete_data),
        );

        log::info!(
            "Architecture {:?} completed: execution_id={}, message_id={}",
            self.architecture,
            self.execution_id,
            self.message_id
        );
    }

    /// 获取执行ID
    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }

    /// 获取消息ID
    pub fn message_id(&self) -> &str {
        &self.message_id
    }

    /// 获取会话ID
    pub fn conversation_id(&self) -> Option<&str> {
        self.conversation_id.as_deref()
    }

    /// 获取架构类型
    pub fn architecture(&self) -> &ArchitectureType {
        &self.architecture
    }
}

// ========== Travel 专用消息类型 ==========

/// Travel OODA 阶段步骤消息（与前端 TravelMessageProcessor 对齐）
#[derive(Debug, Clone, serde::Serialize)]
pub struct TravelPhaseStep {
    /// 循环编号
    pub cycle: u32,
    /// 阶段名称: Observe, Orient, Decide, Act
    pub phase: String,
    /// 阶段状态: running, completed, failed
    pub status: String,
    /// 思考内容
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thought: Option<String>,
    /// 工具调用
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<TravelAction>,
    /// 阶段输出
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,
    /// 错误信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Travel 工具调用
#[derive(Debug, Clone, serde::Serialize)]
pub struct TravelAction {
    pub tool: String,
    pub args: Value,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
}

impl StandardMessageEmitter {
    /// 发送 Travel OODA 阶段步骤（流式更新）
    pub fn emit_travel_step(&self, step: &TravelPhaseStep) {
        let meta_data = serde_json::json!({
            "type": "ooda_step",
            "step": step,
        });

        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::Meta,
            &meta_data.to_string(),
            false,
            Some(&step.phase),
            None,
            Some(ArchitectureType::Travel),
            Some(meta_data),
        );

        log::debug!(
            "Travel step emitted: cycle={}, phase={}, status={}",
            step.cycle, step.phase, step.status
        );
    }

    /// 发送 Travel 工具调用开始
    pub fn emit_travel_tool_start(&self, cycle: u32, phase: &str, tool: &str, args: &Value) {
        let step = TravelPhaseStep {
            cycle,
            phase: phase.to_string(),
            status: "running".to_string(),
            thought: None,
            action: Some(TravelAction {
                tool: tool.to_string(),
                args: args.clone(),
                status: "running".to_string(),
                result: None,
            }),
            output: None,
            error: None,
        };
        self.emit_travel_step(&step);
    }

    /// 发送 Travel 工具调用完成
    pub fn emit_travel_tool_complete(&self, cycle: u32, phase: &str, tool: &str, args: &Value, result: &Value, success: bool) {
        let step = TravelPhaseStep {
            cycle,
            phase: phase.to_string(),
            status: "running".to_string(),
            thought: None,
            action: Some(TravelAction {
                tool: tool.to_string(),
                args: args.clone(),
                status: if success { "completed".to_string() } else { "failed".to_string() },
                result: Some(result.clone()),
            }),
            output: None,
            error: None,
        };
        self.emit_travel_step(&step);
    }

    /// 发送 Travel 阶段思考
    pub fn emit_travel_thought(&self, cycle: u32, phase: &str, thought: &str) {
        let step = TravelPhaseStep {
            cycle,
            phase: phase.to_string(),
            status: "running".to_string(),
            thought: Some(thought.to_string()),
            action: None,
            output: None,
            error: None,
        };
        self.emit_travel_step(&step);
    }

    /// 发送 Travel 阶段完成
    pub fn emit_travel_phase_complete(&self, cycle: u32, phase: &str, output: Option<Value>) {
        let step = TravelPhaseStep {
            cycle,
            phase: phase.to_string(),
            status: "completed".to_string(),
            thought: None,
            action: None,
            output,
            error: None,
        };
        self.emit_travel_step(&step);
    }

    /// 发送 Travel 阶段错误
    pub fn emit_travel_phase_error(&self, cycle: u32, phase: &str, error: &str) {
        let step = TravelPhaseStep {
            cycle,
            phase: phase.to_string(),
            status: "failed".to_string(),
            thought: None,
            action: None,
            output: None,
            error: Some(error.to_string()),
        };
        self.emit_travel_step(&step);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emitter_creation() {
        // 基本创建测试（不需要实际的AppHandle）
        // 实际使用时需要真实的Tauri AppHandle
    }
}

