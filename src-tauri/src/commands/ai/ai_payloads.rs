use crate::commands::ai_conversation_binding_support::AssistantConversationBinding;
use crate::services::ai::{AiConfig, AiToolCall};
use sentinel_llm::ChatMessage as LlmChatMessage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateConversationRequest {
    pub title: Option<String>,
    pub service_name: String,
    #[serde(default)]
    pub conversation_binding: Option<AssistantConversationBinding>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub conversation_id: String,
    pub message: String,
    pub service_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendStreamMessageRequest {
    pub conversation_id: String,
    pub message: String,
    pub service_name: Option<String>,
    pub system_prompt: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveMessageRequest {
    pub id: Option<String>,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
    pub architecture_type: Option<String>,
    pub architecture_meta: Option<String>,
    pub structured_data: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecuteToolCallRequest {
    pub conversation_id: String,
    pub service_name: String,
    pub tool_call: AiToolCall,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AiServiceInfo {
    pub name: String,
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AiModelInfo {
    pub provider: String,
    pub models: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AiServiceStatusResponse {
    pub provider: String,
    pub is_available: bool,
    pub models_count: usize,
    pub active_conversations: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelInfo {
    pub name: String,
    pub provider: String,
    pub is_chat: bool,
    pub is_embedding: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelConfig {
    pub name: String,
    pub provider: String,
    pub config: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StopStreamRequest {
    pub conversation_id: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentExecutionOutcome {
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecutionFinishedEvent {
    pub execution_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation: Option<u64>,
    pub outcome: AgentExecutionOutcome,
    pub success: bool,
    pub error: Option<String>,
    pub response: Option<String>,
    pub message: Option<String>,
}

// 轻量级流式生成请求（不保存消息到数据库）
#[derive(Debug, Clone, Deserialize)]
pub struct GenerateStreamRequest {
    pub stream_id: String,
    pub message: String,
    pub system_prompt: Option<String>,
    pub service_name: Option<String>,
    pub history: Option<Vec<LlmChatMessage>>,
}

// AI 助手对话请求（专门用于编辑器内的 AI 助手面板）
#[derive(Debug, Clone, Deserialize)]
pub struct PluginAssistantRequest {
    pub stream_id: String,
    pub message: String,
    pub system_prompt: Option<String>,
    pub service_name: Option<String>,
    pub history: Option<Vec<LlmChatMessage>>,
    pub current_code: Option<String>, // 当前编辑的代码
    pub code_context: Option<String>, // 代码上下文（选中的代码片段）
    pub assistant_profile_id: Option<String>,
    pub task_kind: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProviderUsageStats {
    pub input_tokens: f64,
    pub output_tokens: f64,
    pub total_tokens: f64,
    pub cost: f64,
}

// DTO for Tauri command argument to avoid CommandArg bound issues
#[derive(Debug, Clone, Deserialize)]
pub struct CommandAiConfig {
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
    pub api_base: Option<String>,
    pub organization: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub extra_headers: Option<HashMap<String, String>>,
    #[serde(default)]
    pub extra_body: Option<serde_json::Value>,
}

impl From<CommandAiConfig> for AiConfig {
    fn from(c: CommandAiConfig) -> Self {
        AiConfig {
            provider: c.provider,
            model: c.model,
            api_key: c.api_key,
            api_base: c.api_base,
            organization: c.organization,
            temperature: c.temperature,
            max_tokens: c.max_tokens,
            rig_provider: None,
            max_turns: None,
            extra_headers: c.extra_headers,
            extra_body: c.extra_body,
        }
    }
}
