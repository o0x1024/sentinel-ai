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
    pub timeout: Option<u64>,
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
    pub execution_time: Option<u64>,
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
    let mut models = HashMap::new();

    // OpenAI 模型
    models.insert(
        AiProvider::OpenAI,
        vec![
            AiModel {
                id: "gpt-4o".to_string(),
                name: "GPT-4o".to_string(),
                provider: AiProvider::OpenAI,
                description: "The latest GPT-4 Omni model, supporting text, images and audio"
                    .to_string(),
                context_length: 128000,
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
                cost_per_1k_input: Some(5.0),
                cost_per_1k_output: Some(15.0),
                is_available: true,
            },
            AiModel {
                id: "gpt-4o-mini".to_string(),
                name: "GPT-4o Mini".to_string(),
                provider: AiProvider::OpenAI,
                description: "A lightweight GPT-4o model with lower cost".to_string(),
                context_length: 128000,
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
                cost_per_1k_input: Some(0.15),
                cost_per_1k_output: Some(0.6),
                is_available: true,
            },
            AiModel {
                id: "gpt-4-turbo".to_string(),
                name: "GPT-4 Turbo".to_string(),
                provider: AiProvider::OpenAI,
                description: "A high-performance GPT-4 model".to_string(),
                context_length: 128000,
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
                cost_per_1k_input: Some(10.0),
                cost_per_1k_output: Some(30.0),
                is_available: true,
            },
        ],
    );

    // Anthropic 模型
    models.insert(
        AiProvider::Anthropic,
        vec![
            AiModel {
                id: "claude-3-5-sonnet-20241022".to_string(),
                name: "Claude 3.5 Sonnet".to_string(),
                provider: AiProvider::Anthropic,
                description: "The most powerful model from Anthropic".to_string(),
                context_length: 200000,
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
                cost_per_1k_input: Some(3.0),
                cost_per_1k_output: Some(15.0),
                is_available: true,
            },
            AiModel {
                id: "claude-3-haiku-20240307".to_string(),
                name: "Claude 3 Haiku".to_string(),
                provider: AiProvider::Anthropic,
                description: "A fast and economical Claude model".to_string(),
                context_length: 200000,
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
                cost_per_1k_input: Some(0.25),
                cost_per_1k_output: Some(1.25),
                is_available: true,
            },
        ],
    );

    // Google Gemini 模型
    models.insert(
        AiProvider::Gemini,
        vec![
            AiModel {
                id: "gemini-1.5-pro".to_string(),
                name: "Gemini 1.5 Pro".to_string(),
                provider: AiProvider::Gemini,
                description: "The most advanced multimodal model from Google".to_string(),
                context_length: 2000000,
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
                cost_per_1k_input: Some(3.5),
                cost_per_1k_output: Some(10.5),
                is_available: true,
            },
            AiModel {
                id: "gemini-1.5-flash".to_string(),
                name: "Gemini 1.5 Flash".to_string(),
                provider: AiProvider::Gemini,
                description: "A fast Gemini model".to_string(),
                context_length: 1000000,
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
                cost_per_1k_input: Some(0.075),
                cost_per_1k_output: Some(0.3),
                is_available: true,
            },
        ],
    );

    // XAI/Grok 模型
    models.insert(
        AiProvider::XAI,
        vec![AiModel {
            id: "grok-beta".to_string(),
            name: "Grok Beta".to_string(),
            provider: AiProvider::XAI,
            description: "The Grok model from xAI".to_string(),
            context_length: 131072,
            supports_streaming: true,
            supports_tools: false,
            supports_vision: false,
            cost_per_1k_input: Some(5.0),
            cost_per_1k_output: Some(15.0),
            is_available: true,
        }],
    );

    // Groq 模型
    models.insert(
        AiProvider::Groq,
        vec![
            AiModel {
                id: "llama-3.1-70b-versatile".to_string(),
                name: "Llama 3.1 70B".to_string(),
                provider: AiProvider::Groq,
                description: "The Llama 3.1 70B model from Meta, running on Groq".to_string(),
                context_length: 131072,
                supports_streaming: true,
                supports_tools: true,
                supports_vision: false,
                cost_per_1k_input: Some(0.59),
                cost_per_1k_output: Some(0.79),
                is_available: true,
            },
            AiModel {
                id: "mixtral-8x7b-32768".to_string(),
                name: "Mixtral 8x7B".to_string(),
                provider: AiProvider::Groq,
                description: "The Mixtral 8x7B model from Mistral".to_string(),
                context_length: 32768,
                supports_streaming: true,
                supports_tools: true,
                supports_vision: false,
                cost_per_1k_input: Some(0.27),
                cost_per_1k_output: Some(0.27),
                is_available: true,
            },
        ],
    );

    // DeepSeek 模型
    models.insert(
        AiProvider::DeepSeek,
        vec![AiModel {
            id: "deepseek-chat".to_string(),
            name: "DeepSeek Chat".to_string(),
            provider: AiProvider::DeepSeek,
            description: "The DeepSeek chat model".to_string(),
            context_length: 32768,
            supports_streaming: true,
            supports_tools: true,
            supports_vision: false,
            cost_per_1k_input: Some(0.14),
            cost_per_1k_output: Some(0.28),
            is_available: true,
        }],
    );

    // Cohere 模型
    models.insert(
        AiProvider::Cohere,
        vec![AiModel {
            id: "command-r-plus".to_string(),
            name: "Command R+".to_string(),
            provider: AiProvider::Cohere,
            description: "The latest Command model from Cohere".to_string(),
            context_length: 128000,
            supports_streaming: true,
            supports_tools: true,
            supports_vision: false,
            cost_per_1k_input: Some(3.0),
            cost_per_1k_output: Some(15.0),
            is_available: true,
        }],
    );

    // Ollama 模型（本地运行）
    models.insert(
        AiProvider::Ollama,
        vec![
            AiModel {
                id: "llama3.1".to_string(),
                name: "Llama 3.1".to_string(),
                provider: AiProvider::Ollama,
                description: "The Llama 3.1 model running locally".to_string(),
                context_length: 128000,
                supports_streaming: true,
                supports_tools: false,
                supports_vision: false,
                cost_per_1k_input: Some(0.0), // 本地运行无成本
                cost_per_1k_output: Some(0.0),
                is_available: false, // 需要本地安装
            },
            AiModel {
                id: "mistral".to_string(),
                name: "Mistral".to_string(),
                provider: AiProvider::Ollama,
                description: "The Mistral model running locally".to_string(),
                context_length: 32768,
                supports_streaming: true,
                supports_tools: false,
                supports_vision: false,
                cost_per_1k_input: Some(0.0),
                cost_per_1k_output: Some(0.0),
                is_available: false,
            },
        ],
    );

    models
}
