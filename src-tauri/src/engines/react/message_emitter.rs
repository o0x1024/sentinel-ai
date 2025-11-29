//! ReAct 消息发送器
//!
//! 发送结构化步骤数据到前端，字段命名与前端 ReActStepDisplay 对齐

use crate::utils::ordered_message::{emit_message_chunk_with_arch, ArchitectureType, ChunkType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::AppHandle;

/// ReAct 消息发送器
pub struct ReactMessageEmitter {
    app_handle: Arc<AppHandle>,
    execution_id: String,
    message_id: String,
    conversation_id: Option<String>,
}

/// 发送到前端的步骤数据（字段与前端 ReActStepDisplay 对齐）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactMessageStep {
    /// 步骤索引（从 0 开始，与前端 index 一致）
    pub index: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thought: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<ReactMessageAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observation: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_answer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 工具调用信息（字段与前端 action 对齐）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactMessageAction {
    pub tool: String,
    pub args: serde_json::Value,
    pub status: String,
}

/// 工具结果信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactMessageObservation {
    pub success: bool,
    pub output: serde_json::Value,
    pub duration_ms: u64,
}

/// 执行统计（字段与前端 ReActArchitectureMeta.statistics 对齐）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactExecutionStats {
    pub total_iterations: u32,
    pub tool_calls_count: u32,
    pub successful_tool_calls: u32,
    pub failed_tool_calls: u32,
    pub total_duration_ms: u64,
    pub status: String,
}

// iteration（从1开始）转换为 index（从0开始）
fn iteration_to_index(iteration: u32) -> u32 {
    iteration.saturating_sub(1)
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
        }
    }

    /// 发送执行开始信号
    pub fn emit_start(&self, config: Option<serde_json::Value>) {
        self.emit_meta("start", serde_json::json!({
            "type": "start",
            "config": config
        }));
    }

    /// 发送思考步骤（包含 thought 内容，供前端结构化显示）
    pub fn emit_thought(&self, iteration: u32, thought: &str, _has_rag: bool) {
        let cleaned = Self::extract_thought(thought);
        if cleaned.is_empty() {
            return;
        }
        
        let step = ReactMessageStep {
            index: iteration_to_index(iteration),
            thought: Some(cleaned),
            action: None,
            observation: None,
            final_answer: None,
            error: None,
        };
        self.emit_step(&step, "thought");
    }

    /// 发送工具调用开始
    pub fn emit_action_start(&self, iteration: u32, tool: &str, args: &serde_json::Value) {
        let step = ReactMessageStep {
            index: iteration_to_index(iteration),
            thought: None,
            action: Some(ReactMessageAction {
                tool: tool.to_string(),
                args: args.clone(),
                status: "running".to_string(),
            }),
            observation: None,
            final_answer: None,
            error: None,
        };
        self.emit_step(&step, "action");
    }

    /// 发送工具执行结果
    pub fn emit_observation(
        &self,
        iteration: u32,
        tool: &str,
        result: &serde_json::Value,
        success: bool,
        _duration_ms: u64,
    ) {
        let step = ReactMessageStep {
            index: iteration_to_index(iteration),
            thought: None,
            action: Some(ReactMessageAction {
                tool: tool.to_string(),
                args: serde_json::Value::Null,
                status: if success { "completed" } else { "failed" }.to_string(),
            }),
            observation: Some(result.clone()),
            final_answer: None,
            error: None,
        };
        self.emit_step(&step, "observation");
    }

    /// 发送最终答案（包含完整的 answer 内容）
    pub fn emit_final_answer(&self, iteration: u32, answer: &str, _citations: &[String]) {
        // 提取 Final Answer 后面的实际内容
        let cleaned_answer = Self::extract_final_answer(answer);
        
        let step = ReactMessageStep {
            index: iteration_to_index(iteration),
            thought: None,
            action: None,
            observation: None,
            final_answer: Some(cleaned_answer),
            error: None,
        };
        self.emit_step(&step, "final");
    }

    /// 发送错误
    pub fn emit_error(&self, iteration: u32, error: &str) {
        let step = ReactMessageStep {
            index: iteration_to_index(iteration),
            thought: None,
            action: None,
            observation: None,
            final_answer: None,
            error: Some(error.to_string()),
        };
        self.emit_step(&step, "error");
    }

    /// 发送执行完成信号
    pub fn emit_complete(&self, stats: ReactExecutionStats) {
        self.emit_meta("complete", serde_json::json!({
            "type": "complete",
            "statistics": stats
        }));
    }

    // === 流式内容发送（用于 LLM 输出） ===

    /// 发送流式内容 chunk（LLM 输出的每个 token）
    pub fn emit_content(&self, content: &str, is_final: bool) {
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

    // === 内部方法 ===

    fn emit_step(&self, step: &ReactMessageStep, sub_stage: &str) {
        self.emit_meta(&format!("step:{}", sub_stage), serde_json::json!({
            "type": "step",
            "step": step
        }));
    }

    fn emit_meta(&self, stage: &str, data: serde_json::Value) {
        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::Meta,
            "",
            stage == "complete",
            Some(stage),
            None,
            Some(ArchitectureType::ReAct),
            Some(data),
        );
    }

    /// 从 LLM 输出中提取 thought 内容
    /// 移除 "Thought:" 前缀，截取到 "Action:" 或 "Final Answer:" 之前
    fn extract_thought(raw: &str) -> String {
        // 移除 Thought: 前缀
        let text = raw
            .trim_start_matches("Thought:")
            .trim_start_matches("thought:")
            .trim_start();

        // 找到行首的 Action: 或 Final Answer:
        let mut end_pos = text.len();
        for (i, line) in text.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("Action:") || trimmed.starts_with("action:") ||
               trimmed.starts_with("Final Answer:") || trimmed.starts_with("final answer:") {
                // 计算这一行在原文中的起始位置
                let pos: usize = text.lines().take(i).map(|l| l.len() + 1).sum();
                end_pos = pos.saturating_sub(1); // 减去换行符
                break;
            }
        }

        text[..end_pos.min(text.len())].trim().to_string()
    }

    /// 从 LLM 输出中提取 final answer 内容
    /// 移除 "Final Answer:" 前缀
    fn extract_final_answer(raw: &str) -> String {
        // 移除 Final Answer: 前缀
        let text = raw
            .trim_start_matches("Final Answer:")
            .trim_start_matches("final answer:")
            .trim_start();

        text.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iteration_to_index() {
        assert_eq!(iteration_to_index(1), 0);
        assert_eq!(iteration_to_index(2), 1);
        assert_eq!(iteration_to_index(0), 0); // saturating_sub 防止溢出
    }

    #[test]
    fn test_extract_thought_with_action() {
        let input = "Thought: 我需要搜索一下\nAction: search\nAction Input: {\"query\": \"test\"}";
        let extracted = ReactMessageEmitter::extract_thought(input);
        assert_eq!(extracted, "我需要搜索一下");
    }

    #[test]
    fn test_extract_thought_with_final_answer() {
        let input = "Thought: 我已经得到了答案\nFinal Answer: 这是最终答案";
        let extracted = ReactMessageEmitter::extract_thought(input);
        assert_eq!(extracted, "我已经得到了答案");
    }

    #[test]
    fn test_extract_thought_with_action_in_content() {
        // 内容中包含 "action" 但不是行首的 "Action:"
        let input = "Thought: The user is asking about what action to take\nAction: search";
        let extracted = ReactMessageEmitter::extract_thought(input);
        assert_eq!(extracted, "The user is asking about what action to take");
    }

    #[test]
    fn test_extract_thought_pure() {
        let input = "这只是一个普通的思考内容";
        let extracted = ReactMessageEmitter::extract_thought(input);
        assert_eq!(extracted, "这只是一个普通的思考内容");
    }

    #[test]
    fn test_extract_final_answer() {
        let input = "Final Answer: 这是最终答案，包含多行内容\n第二行\n第三行";
        let extracted = ReactMessageEmitter::extract_final_answer(input);
        assert_eq!(extracted, "这是最终答案，包含多行内容\n第二行\n第三行");
    }

    #[test]
    fn test_extract_final_answer_without_prefix() {
        let input = "这是没有前缀的答案";
        let extracted = ReactMessageEmitter::extract_final_answer(input);
        assert_eq!(extracted, "这是没有前缀的答案");
    }
}
