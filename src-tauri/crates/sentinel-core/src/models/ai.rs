use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// AI服务提供商
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum AiProvider {
    OpenAI,
    Anthropic,
    Gemini,
    XAI, // XAI/Grok
    Ollama,
    Groq,
    DeepSeek,
    Cohere,
    ModelScope,
    OpenRouter,
    Custom, // 自定义提供商
}

impl std::fmt::Display for AiProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiProvider::OpenAI => write!(f, "openai"),
            AiProvider::Anthropic => write!(f, "anthropic"),
            AiProvider::Gemini => write!(f, "gemini"),
            AiProvider::XAI => write!(f, "xai"),
            AiProvider::Ollama => write!(f, "ollama"),
            AiProvider::Groq => write!(f, "groq"),
            AiProvider::DeepSeek => write!(f, "deepseek"),
            AiProvider::Cohere => write!(f, "cohere"),
            AiProvider::ModelScope => write!(f, "modelscope"),
            AiProvider::OpenRouter => write!(f, "openrouter"),
            AiProvider::Custom => write!(f, "custom"),
        }
    }
}

/// AI模型信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiModel {
    pub id: String,
    pub name: String,
    pub provider: AiProvider,
    pub description: String,
    pub context_length: u32,
    pub supports_streaming: bool,
    pub supports_tools: bool,
    pub supports_vision: bool,
    pub cost_per_1k_input: Option<f64>,
    pub cost_per_1k_output: Option<f64>,
    pub is_available: bool,
}

/// AI提供商配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiProviderConfig {
    pub provider: AiProvider,
    pub name: String,
    pub api_key: Option<String>,
    pub api_base: Option<String>,
    pub organization: Option<String>,
    pub project: Option<String>,
    pub models: Vec<AiModel>,
    pub enabled: bool,
    pub default_model: Option<String>,
    pub rate_limit: Option<RateLimit>,
    pub timeout: Option<f64>,
    pub extra_headers: Option<HashMap<String, String>>,
}

/// 速率限制配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub requests_per_minute: u32,
    pub tokens_per_minute: u32,
}

/// AI对话消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiMessage {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub model: Option<String>,
    pub provider: Option<AiProvider>,
    pub tokens_used: Option<TokenUsage>,
    pub tools_used: Option<Vec<String>>,
    pub metadata: Option<serde_json::Value>,
}

/// 消息角色
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// Token使用情况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
    pub cost: Option<f64>,
}

/// AI对话会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConversation {
    pub id: String,
    pub title: String,
    pub provider: AiProvider,
    pub model: String,
    pub messages: Vec<AiMessage>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub total_tokens: u32,
    pub total_cost: f64,
    pub system_prompt: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub metadata: Option<serde_json::Value>,
}

/// AI工具调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiToolCall {
    pub id: String,
    pub tool_name: String,
    pub parameters: serde_json::Value,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub execution_time: Option<f64>,
}

/// AI 角色
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiRole {
    pub id: String,
    pub title: String,
    pub description: String,
    pub prompt: String,
    pub is_system: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// AI配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub providers: HashMap<AiProvider, AiProviderConfig>,
    pub default_provider: AiProvider,
    pub default_model: String,
    pub system_prompt: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub stream_response: bool,
    pub enable_tools: bool,
    pub auto_save_conversations: bool,
    pub conversation_history_limit: u32,
}

/// AI服务状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiServiceStatus {
    pub provider: AiProvider,
    pub is_available: bool,
    pub last_check: DateTime<Utc>,
    pub error: Option<String>,
    pub models_loaded: u32,
    pub active_conversations: u32,
}

/// 流式响应块
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub id: String,
    pub content: String,
    pub is_complete: bool,
    pub tokens_used: Option<u32>,
    pub tool_calls: Option<Vec<AiToolCall>>,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            providers: HashMap::new(),
            default_provider: AiProvider::OpenAI,
            default_model: "gpt-4".to_string(),
            system_prompt: "You are a professional security and vulnerability detection assistant. Please provide accurate and practical advice and help based on user needs.".to_string(),
            temperature: 0.7,
            max_tokens: 4096,
            stream_response: true,
            enable_tools: true,
            auto_save_conversations: true,
            conversation_history_limit: 100,
        }
    }
}

impl AiModel {
    pub fn new(id: String, name: String, provider: AiProvider) -> Self {
        Self {
            id,
            name,
            provider,
            description: String::new(),
            context_length: 4096,
            supports_streaming: true,
            supports_tools: false,
            supports_vision: false,
            cost_per_1k_input: None,
            cost_per_1k_output: None,
            is_available: true,
        }
    }
}

impl AiConversation {
    pub fn new(provider: AiProvider, model: String) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: "New conversation".to_string(),
            provider,
            model,
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
            total_tokens: 0,
            total_cost: 0.0,
            system_prompt: None,
            temperature: None,
            max_tokens: None,
            metadata: None,
        }
    }

    pub fn add_message(&mut self, message: AiMessage) {
        self.messages.push(message);
        self.updated_at = Utc::now();
    }

    pub fn calculate_cost(&self) -> f64 {
        self.messages
            .iter()
            .filter_map(|msg| msg.tokens_used.as_ref())
            .filter_map(|usage| usage.cost)
            .sum()
    }
}

/// 获取默认模型列表
pub fn get_default_models() -> HashMap<AiProvider, Vec<AiModel>> {
    let models = HashMap::new();

    models
}


