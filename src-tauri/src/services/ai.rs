use crate::commands::ai::{ModelConfig, ModelInfo};
// Removed: use crate::engines::types::{StreamMessageType, UnifiedStreamMessage};
use crate::models::database::{AiConversation, AiMessage};
use crate::services::database::Database;
use crate::services::mcp::McpService;
use crate::utils::ordered_message::ChunkType;
use anyhow::Result;
use chrono::Utc;


use futures::StreamExt;
use rig::client::builder::DynClientBuilder;
use rig::client::ProviderClient;
use rig::streaming::{StreamedAssistantContent, StreamingPrompt};
use rig::agent::{MultiTurnStreamItem, PromptHook, CancelSignal};
use rig::completion::{CompletionModel, CompletionResponse, Message};
use rig::message::{AssistantContent, UserContent};
use rig::providers::gemini::completion::gemini_api_types::{
    AdditionalParameters, GenerationConfig,
};


use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;

use std::sync::Arc;
use tauri::AppHandle;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

// LLM请求响应日志记录Hook
#[derive(Clone)]
struct LLMLoggingHook {
    session_id: String,
    conversation_id: Option<String>,
    provider: String,
    model: String,
}

impl LLMLoggingHook {
    fn new(session_id: String, conversation_id: Option<String>, provider: String, model: String) -> Self {
        Self {
            session_id,
            conversation_id,
            provider,
            model,
        }
    }

    fn write_to_log(&self, log_type: &str, content: &str) {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f UTC");
        let log_entry = format!(
            "[{}] [{}] [Session: {}] [Conversation: {}] [Provider: {}] [Model: {}] {}\n",
            timestamp,
            log_type,
            self.session_id,
            self.conversation_id.as_deref().unwrap_or("N/A"),
            self.provider,
            self.model,
            content
        );

        // 确保日志目录存在
        if let Err(e) = std::fs::create_dir_all("logs") {
            tracing::error!("Failed to create logs directory: {}", e);
            return;
        }

        // 写入专门的LLM请求日志文件
        let log_file_path = format!("logs/llm-http-requests-{}.log", chrono::Utc::now().format("%Y-%m-%d"));
        
        match OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file_path)
        {
            Ok(mut file) => {
                if let Err(e) = file.write_all(log_entry.as_bytes()) {
                    tracing::error!("Failed to write to LLM log file {}: {}", log_file_path, e);
                } else {
                    // 确保数据立即写入磁盘
                    let _ = file.flush();
                }
            }
            Err(e) => {
                tracing::error!("Failed to open LLM log file {}: {}", log_file_path, e);
            }
        }

        // 同时输出到标准日志
        // tracing::info!("LLM_LOG: {}", content);
    }
}

impl<M: CompletionModel> PromptHook<M> for LLMLoggingHook {
    async fn on_tool_call(&self, tool_name: &str, args: &str, _cancel_sig: CancelSignal) {
        let content = format!("TOOL_CALL - Tool: {} | Args: {}", tool_name, args);
        self.write_to_log("TOOL_CALL", &content);
    }

    async fn on_tool_result(
        &self,
        tool_name: &str,
        args: &str,
        result: &str,
        _cancel_sig: CancelSignal,
    ) {
        let content = format!("TOOL_RESULT - Tool: {} | Args: {} | Result: {}", tool_name, args, result);
        self.write_to_log("TOOL_RESULT", &content);
    }

    async fn on_completion_call(
        &self,
        prompt: &Message,
        history: &[Message],
        _cancel_sig: CancelSignal,
    ) {
        let prompt_content = match prompt {
            Message::User { content } => content
                .iter()
                .filter_map(|c| {
                    if let UserContent::Text(text_content) = c {
                        Some(text_content.text.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n"),
            Message::Assistant { content, .. } => content
                .iter()
                .filter_map(|c| if let AssistantContent::Text(text_content) = c {
                    Some(text_content.text.clone())
                } else {
                    None
                })
                .collect::<Vec<_>>()
                .join("\n"),
        };

        let content = format!(
            "REQUEST - History Length: {} | Prompt: {}",
            history.len(),
            prompt_content.chars().take(500).collect::<String>() + if prompt_content.len() > 500 { "..." } else { "" }
        );
        self.write_to_log("REQUEST", &content);
    }

    async fn on_completion_response(
        &self,
        _prompt: &Message,
        response: &CompletionResponse<M::Response>,
        _cancel_sig: CancelSignal,
    ) {
        let response_content = if let Ok(resp_json) = serde_json::to_string(&response.raw_response) {
            format!("Raw Response: {}", resp_json.chars().take(1000).collect::<String>() + if resp_json.len() > 1000 { "..." } else { "" })
        } else {
            "Response: <non-serializable>".to_string()
        };

        let content = format!(
            "RESPONSE - Content: {} | Usage: {:?}",
            response_content,
            response.usage
        );
        self.write_to_log("RESPONSE", &content);
    }
}

// NOTE: rig-core's StreamingPromptHook is private; we only implement PromptHook above.


// 模型配置相关结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    pub enabled: bool,
    pub intent_analysis_model: String,
    pub intent_analysis_provider: String,
    pub planner_model: String,
    pub planner_provider: String,
    pub replanner_model: String,
    pub replanner_provider: String,
    pub executor_model: String,
    pub executor_provider: String,
    pub evaluator_model: String,
    pub evaluator_provider: String,
    pub default_strategy: String,
    pub max_retries: i32,
    pub timeout_seconds: i32,
    pub scenarios: serde_json::Value,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            intent_analysis_model: String::new(),
            intent_analysis_provider: String::new(),
            planner_model: String::new(),
            planner_provider: String::new(),
            replanner_model: String::new(),
            replanner_provider: String::new(),
            executor_model: String::new(),
            executor_provider: String::new(),
            evaluator_model: String::new(),
            evaluator_provider: String::new(),
            default_strategy: "adaptive".to_string(),
            max_retries: 3,
            timeout_seconds: 120,
            scenarios: serde_json::Value::Object(serde_json::Map::new()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SchedulerStage {
    IntentAnalysis,
    Planning,
    Replanning,
    Execution,
    Evaluation,
}

// AI工具调用结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
    pub result: Option<Value>,
    pub error: Option<String>,
}

// AI工具调用结果流消息
#[derive(Debug, Clone, Serialize)]    
pub struct ToolCallResultMessage {
    pub conversation_id: String,
    pub message_id: String,
    pub tool_call_id: String,
    pub result: Value,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AiConfig {
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
    pub api_base: Option<String>,
    pub organization: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamMessage {
    pub conversation_id: String,
    pub message_id: String,
    pub content: String,
    pub is_complete: bool,
    pub token_count: Option<u32>,
    pub total_tokens: Option<u32>,
    pub tool_calls: Option<Vec<AiToolCall>>,
    // Enhanced streaming support
    pub is_incremental: bool, // true for incremental chunks, false for full content
    pub content_delta: Option<String>, // incremental content chunk when is_incremental=true
    pub total_content_length: Option<usize>, // total accumulated content length
    // Intent-specific streaming fields
    pub intent_type: Option<String>,  // "chat", "question", "task"
    pub stream_phase: Option<String>, // "content", "plan", "execution", "results"
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskStreamMessage {
    pub conversation_id: String,
    pub message_id: String,
    pub execution_id: String,
    pub phase: String, // "plan", "execution", "results"
    pub content: String,
    pub execution_plan: Option<serde_json::Value>,
    pub progress: Option<f32>,
    pub current_step: Option<String>,
    pub completed_steps: Option<u32>,
    pub total_steps: Option<u32>,
    pub is_complete: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskProgressMessage {
    pub conversation_id: String,
    pub execution_id: String,
    pub step_name: String,
    pub step_index: u32,
    pub total_steps: u32,
    pub progress: f32,
    pub status: String, // "running", "completed", "error"
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]                                                                                                                                                                                                                                  
pub struct StreamError {
    pub conversation_id: String,
    pub message_id: Option<String>,
    pub execution_id: Option<String>,
    pub error: String,
    pub error_type: String, // "stream", "task_execution", "plan_generation"
}

#[derive(Debug, Clone)]
pub struct AiService {
    config: AiConfig,
    db: Arc<dyn Database + Send + Sync>,
    app_handle: Option<AppHandle>,
    mcp_service: Option<Arc<McpService>>,
    #[allow(unused)]
    max_retries: u32,
}

#[derive(Debug, Clone)]
pub struct AiServiceManager {
    services: Arc<std::sync::RwLock<HashMap<String, AiService>>>,
    db: Arc<dyn Database + Send + Sync>,
    app_handle: Arc<std::sync::RwLock<Option<AppHandle>>>,
    mcp_service: Option<Arc<McpService>>,
}

#[derive(Debug, Deserialize)]
pub struct ProviderConfig {
    #[allow(unused)]
    id: String,
    provider: String,
    name: String,
    api_key: Option<String>,
    api_base: Option<String>,
    organization: Option<String>,
    enabled: bool,
    default_model: String,
    #[allow(unused)]
    models: Vec<ModelDefinition>,
}

#[derive(Debug, Deserialize)]
struct ModelDefinition {
    #[allow(unused)]
    id: String,
    #[allow(unused)]
    name: String,
    #[allow(unused)]
    #[serde(default)]
    config: serde_json::Value,
}

impl AiServiceManager {
    pub fn new(db: Arc<dyn Database + Send + Sync>) -> Self {
        Self {
            services: Arc::new(std::sync::RwLock::new(HashMap::new())),
            db,
            app_handle: Arc::new(std::sync::RwLock::new(None)),
            mcp_service: None,
        }
    }

    pub fn get_db_arc(&self) -> Arc<dyn Database + Send + Sync> {
        self.db.clone()
    }

    /// 从数据库获取AI提供商配置
    pub async fn get_provider_config(&self, provider: &str) -> Result<Option<AiConfig>> {
        // 首先尝试从providers_config中获取
        if let Ok(Some(providers_json)) = self.db.get_config("ai", "providers_config").await {
            if let Ok(providers) =
                serde_json::from_str::<HashMap<String, serde_json::Value>>(&providers_json)
            {
                // 查找匹配的提供商（不区分大小写）
                for (key, provider_data) in providers {
                    if let Some(provider_obj) = provider_data.as_object() {
                        if let Some(provider_name) =
                            provider_obj.get("provider").and_then(|v| v.as_str())
                        {
                            if provider_name.to_lowercase() == provider.to_lowercase()
                                || key.to_lowercase() == provider.to_lowercase()
                            {
                                let api_key = provider_obj
                                    .get("api_key")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());
                                let api_base = provider_obj
                                    .get("api_base")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());
                                let default_model = provider_obj
                                    .get("default_model")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("default")
                                    .to_string();
                                let organization = provider_obj
                                    .get("organization")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());

                                return Ok(Some(AiConfig {
                                    provider: provider_name.to_string(),
                                    model: default_model,
                                    api_key,
                                    api_base,
                                    organization,
                                    temperature: Some(0.7),
                                    max_tokens: Some(4096),
                                }));
                            }
                        }
                    }
                }
            }
        }

        // 如果providers_config中没有找到，尝试从单独的配置项中获取
        let api_key_name = format!("api_key_{}", provider.to_lowercase());
        let api_key = self.db.get_config("ai", &api_key_name).await.ok().flatten();

        if api_key.is_some() {
            let api_base = match provider.to_lowercase().as_str() {
                "openai" => Some("https://api.openai.com/v1".to_string()),
                "anthropic" => Some("https://api.anthropic.com".to_string()),
                "deepseek" => Some("https://api.deepseek.com".to_string()),
                "google" => Some("https://generativelanguage.googleapis.com/v1beta".to_string()),
                "ollama" => Some("http://localhost:11434".to_string()),
                "moonshot" => Some("https://api.moonshot.ai".to_string()),
                "modelscope" => Some("https://api-inference.modelscope.cn/v1".to_string()),
                "openrouter" => Some("https://openrouter.ai/api/v1".to_string()),
                _ => None,
            };

            let default_model = match provider.to_lowercase().as_str() {
                "openai" => "gpt-4o",
                "anthropic" => "claude-3-5-sonnet-20241022",
                "deepseek" => "deepseek-chat",
                "google" => "gemini-pro",
                "ollama" => "llama2",
                "moonshot" => "moonshot-v1",
                "modelscope" => "qwen2.5-coder-32b-instruct",
                "openrouter" => "gpt-4o",
                _ => "default",
            }
            .to_string();

            return Ok(Some(AiConfig {
                provider: provider.to_string(),
                model: default_model,
                api_key,
                api_base,
                organization: None,
                temperature: Some(0.7),
                max_tokens: Some(4096),
            }));
        }

        Ok(None)
    }

    // 设置MCP服务
    pub fn set_mcp_service(&mut self, mcp_service: Arc<McpService>) {
        self.mcp_service = Some(mcp_service.clone());

        // 更新所有已存在的服务
        let mut services = self.services.write().unwrap();
        for service in services.values_mut() {
            service.set_mcp_service(mcp_service.clone());
        }
    }

    // 获取MCP服务
    pub fn get_mcp_service(&self) -> Option<Arc<McpService>> {
        self.mcp_service.clone()
    }

    // 设置Tauri应用句柄，用于事件发送
    pub fn set_app_handle(&self, app_handle: AppHandle) {
        let mut handle_guard = self.app_handle.write().unwrap();
        *handle_guard = Some(app_handle.clone());

        // 更新所有已存在的服务
        let mut services = self.services.write().unwrap();
        for service in services.values_mut() {
            service.set_app_handle(app_handle.clone());
        }
    }

    // 添加AI服务
    pub async fn add_service(&self, name: String, config: AiConfig) -> Result<()> {
        let service = AiService {
            config,
            db: self.db.clone(),
            app_handle: self.app_handle.read().unwrap().clone(),
            mcp_service: self.mcp_service.clone(),
            max_retries: 3,
        };

        let mut services = self.services.write().unwrap();
        services.insert(name, service);

        Ok(())
    }

    // 获取AI服务
    pub fn get_service(&self, name: &str) -> Option<AiService> {
        let services = self.services.read().unwrap();
        services.get(name).cloned()
    }

    // 列出所有服务
    pub fn list_services(&self) -> Vec<String> {
        let services = self.services.read().unwrap();
        services.keys().cloned().collect()
    }

    // 移除AI服务
    pub fn remove_service(&self, name: &str) -> bool {
        let mut services = self.services.write().unwrap();
        services.remove(name).is_some()
    }

    // 重新加载所有服务（清除现有服务并重新初始化）
    pub async fn reload_services(&self) -> anyhow::Result<()> {
        tracing::info!("Reloading AI services...");

        // 清除所有现有服务
        {
            let mut services = self.services.write().unwrap();
            services.clear();
        }

        // 重新初始化
        self.init_default_services().await
    }

    // 初始化默认服务
    pub async fn init_default_services(&self) -> anyhow::Result<()> {
        tracing::debug!("Initializing default AI services...");

        // 从数据库加载并解析providers_config
        if let Ok(Some(config_str)) = self.db.get_config("ai", "providers_config").await {
            match serde_json::from_str::<HashMap<String, ProviderConfig>>(&config_str) {
                Ok(providers) => {
                    tracing::debug!("Successfully parsed 'providers_config' from DB.");
                    for (_id, provider_config
                    ) in providers {
                        if !provider_config.enabled {
                            continue;
                        }

                        tracing::debug!("Initializing enabled provider: {}", provider_config.name);

                        let api_key = provider_config.api_key.as_deref().map(String::from);
                        std::env::set_var(
                            format!("{}_API_KEY", provider_config.name.to_uppercase()),
                            api_key.as_deref().unwrap_or(""),
                        );

                        let default_model = provider_config.default_model.clone();

                        let api_base = provider_config
                            .api_base
                            .filter(|s| !s.is_empty())
                            .map(String::from);

                        let organization = provider_config
                            .organization
                            .filter(|s| !s.is_empty())
                            .map(String::from);

                        let config = AiConfig {
                            provider: provider_config.provider.clone(),
                            model: default_model.clone(),
                            api_key: api_key,
                            api_base: api_base,
                            organization: organization,
                            temperature: Some(0.7),
                            max_tokens: Some(4096), // 确保有默认值，避免响应被截断
                        };

                        if let Err(e) = self.add_service(provider_config.name.clone(), config).await
                        {
                            tracing::error!(
                                "Failed to add service for provider {}: {}",
                                provider_config.name,
                                e
                            );
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to parse 'providers_config' as map: {}. Falling back to env vars. Content: {}", e, config_str);
                }
            }
        } else {
            tracing::info!("'providers_config' not found in database. Trying to initialize from environment variables.");
        }

        // 在成功注册完providers后，尝试读取并应用全局默认Provider
        if let Ok(Some(default_provider)) = self.db.get_config("ai", "default_provider").await {
            let provider_key = default_provider.to_lowercase();
            if self.get_service(&provider_key).is_some() {
                if let Err(e) = self.set_default_alias_to(&provider_key).await {
                    tracing::warn!("Failed to set default alias to '{}': {}", provider_key, e);
                }
            } else {
                tracing::warn!(
                    "Configured default provider '{}' not found among initialized services",
                    provider_key
                );
            }
        }

        // 确保至少有一个default服务
        if !self.services.read().unwrap().contains_key("default") {
            if self.services.read().unwrap().is_empty() {
                tracing::warn!(
                    "No AI services configured. Creating a minimal default service for session management."
                );
                // 创建一个最小化的默认服务用于会话管理
                if let Err(e) = self.create_minimal_default_service().await {
                    tracing::error!("Failed to create minimal default service: {}", e);
                }
            } else {
                tracing::info!("Creating default service alias from existing services.");
                if let Err(e) = self.create_default_alias().await {
                    tracing::error!("Failed to create default alias service: {}", e);
                }
            }
        }

        tracing::debug!(
            "Finished initializing AI services. Total services: {}",
            self.services.read().unwrap().len()
        );

        Ok(())
    }

    // 创建default别名，指向首选的AI服务
    async fn create_default_alias(&self) -> anyhow::Result<()> {
        let services = self.list_services();

        // 如果已经有名为"default"的服务，不需要创建别名
        if services.contains(&"default".to_string()) {
            return Ok(());
        }

        // 寻找首选的提供商（按优先级排序）
        let preferred_providers = vec![
            "deepseek",
            "openai",
            "anthropic",
            "gemini",
            "groq",
            "ollama",
            "moonshot",
            "openrouter",
            "modelscope",
        ];

        for provider in preferred_providers {
            if services.contains(&provider.to_string()) {
                // 获取该提供商的服务配置并复制为default
                if let Some(service) = self.get_service(provider) {
                    let original_config = service.get_config();
                    let config = AiConfig {
                        provider: original_config.provider.clone(),
                        model: original_config.model.clone(),
                        api_key: original_config.api_key.clone(),
                        api_base: original_config.api_base.clone(),
                        organization: original_config.organization.clone(),
                        temperature: original_config.temperature,
                        max_tokens: original_config.max_tokens,
                    };
                    self.add_service("default".to_string(), config).await?;
                    tracing::debug!("Created default service alias, pointing to {}", provider);
                    return Ok(());
                }
            }
        }

        // 如果没有找到首选提供商，使用第一个可用的服务
        if let Some(first_service_name) = services.first() {
            if let Some(service) = self.get_service(first_service_name) {
                let original_config = service.get_config();
                let config = AiConfig {
                    provider: original_config.provider.clone(),
                    model: original_config.model.clone(),
                    api_key: original_config.api_key.clone(),
                    api_base: original_config.api_base.clone(),
                    organization: original_config.organization.clone(),
                    temperature: original_config.temperature,
                    max_tokens: original_config.max_tokens,
                };
                self.add_service("default".to_string(), config).await?;
                tracing::info!(
                    "Created default service alias, pointing to {}",
                    first_service_name
                );
            }
        }

        Ok(())
    }

    /// 将 default 别名指向指定的 provider 对应的服务
    pub async fn set_default_alias_to(&self, provider: &str) -> anyhow::Result<()> {
        let provider_lc = provider.to_lowercase();
        // 找到目标服务：优先通过服务配置中的 provider 字段匹配，其次按服务名（不区分大小写）匹配
        let service = {
            let services = self.services.read().unwrap();
            // 先按配置中的 provider 字段匹配（小写）
            if let Some((_name, svc)) = services
                .iter()
                .find(|(_n, svc)| svc.get_config().provider.to_lowercase() == provider_lc)
            {
                Some(svc.clone())
            } else {
                // 再按服务名匹配（不区分大小写）
                services.iter().find_map(|(n, svc)| {
                    if n.to_lowercase() == provider_lc {
                        Some(svc.clone())
                    } else {
                        None
                    }
                })
            }
        };
        let Some(service) = service else {
            anyhow::bail!("Target provider service '{}' not found", provider);
        };

        // 构造新配置
        let original_config = service.get_config();
        let config = AiConfig {
            provider: original_config.provider.clone(),
            model: original_config.model.clone(),
            api_key: original_config.api_key.clone(),
            api_base: original_config.api_base.clone(),
            organization: original_config.organization.clone(),
            temperature: original_config.temperature,
            max_tokens: original_config.max_tokens,
        };

        // 如已存在 default，移除
        {
            let mut services = self.services.write().unwrap();
            services.remove("default");
        }

        // 新建 default 别名
        self.add_service("default".to_string(), config).await?;
        tracing::info!("Default service alias now points to '{}'", provider_lc);
        Ok(())
    }

    // 创建最小化的默认服务（用于会话管理，无需实际AI功能）
    async fn create_minimal_default_service(&self) -> anyhow::Result<()> {
        tracing::warn!("Creating minimal default service - no AI providers are configured!");
        tracing::warn!("Please configure at least one AI provider (OpenAI, Anthropic, DeepSeek, etc.) in the Settings > AI Configuration to enable AI chat functionality.");

        // 创建一个虚拟的AI配置，仅用于会话管理
        let config = AiConfig {
            provider: "unconfigured".to_string(),
            model: "no-model-configured".to_string(),
            api_key: None,
            api_base: None,
            organization: None,
            temperature: Some(0.7),
            max_tokens: Some(1000),
        };

        self.add_service("default".to_string(), config).await?;
        tracing::warn!("Created minimal default service - AI chat will not work until providers are configured");

        Ok(())
    }

    // 模型配置相关方法
    // 获取模型配置配置
    pub async fn get_scheduler_config(&self) -> anyhow::Result<SchedulerConfig> {
        let mut config = SchedulerConfig::default();

        // 从数据库加载模型配置配置
        if let Ok(Some(intent_model)) = self
            .db
            .get_config("scheduler", "intent_analysis_model")
            .await
        {
            config.intent_analysis_model = intent_model;
        }

        if let Ok(Some(intent_provider)) = self
            .db
            .get_config("scheduler", "intent_analysis_provider")
            .await
        {
            config.intent_analysis_provider = intent_provider;
        }

        if let Ok(Some(planner_model)) = self.db.get_config("scheduler", "planner_model").await {
            config.planner_model = planner_model;
        }

        if let Ok(Some(planner_provider)) =
            self.db.get_config("scheduler", "planner_provider").await
        {
            config.planner_provider = planner_provider;
        }

        if let Ok(Some(replanner_model)) = self.db.get_config("scheduler", "replanner_model").await
        {
            config.replanner_model = replanner_model;
        }

        if let Ok(Some(replanner_provider)) =
            self.db.get_config("scheduler", "replanner_provider").await
        {
            config.replanner_provider = replanner_provider;
        }

        if let Ok(Some(executor_model)) = self.db.get_config("scheduler", "executor_model").await {
            config.executor_model = executor_model;
        }

        if let Ok(Some(executor_provider)) =
            self.db.get_config("scheduler", "executor_provider").await
        {
            config.executor_provider = executor_provider;
        }

        if let Ok(Some(evaluator_model)) = self.db.get_config("scheduler", "evaluator_model").await
        {
            config.evaluator_model = evaluator_model;
        }

        if let Ok(Some(evaluator_provider)) =
            self.db.get_config("scheduler", "evaluator_provider").await
        {
            config.evaluator_provider = evaluator_provider;
        }

        if let Ok(Some(default_strategy)) =
            self.db.get_config("scheduler", "default_strategy").await
        {
            config.default_strategy = default_strategy;
        }

        if let Ok(Some(enabled_str)) = self.db.get_config("scheduler", "enabled").await {
            config.enabled = enabled_str.parse().unwrap_or(true);
        }

        if let Ok(Some(max_retries_str)) = self.db.get_config("scheduler", "max_retries").await {
            config.max_retries = max_retries_str.parse().unwrap_or(3);
        }

        if let Ok(Some(timeout_str)) = self.db.get_config("scheduler", "timeout_seconds").await {
            config.timeout_seconds = timeout_str.parse().unwrap_or(120);
        }

        if let Ok(Some(scenarios_str)) = self.db.get_config("scheduler", "scenarios").await {
            config.scenarios = serde_json::from_str(&scenarios_str)
                .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
        }

        Ok(config)
    }

    /// 根据阶段直接使用配置中的 provider/model 构建一次性服务（使用 Rig）
    pub async fn get_service_for_stage(
        &self,
        stage: SchedulerStage,
    ) -> anyhow::Result<Option<AiService>> {
        let config = self.get_scheduler_config().await?;

        let (model_id, provider_name) = match stage {
            SchedulerStage::IntentAnalysis => (
                config.intent_analysis_model,
                config.intent_analysis_provider,
            ),
            SchedulerStage::Planning => (config.planner_model, config.planner_provider),
            SchedulerStage::Replanning => (config.replanner_model, config.replanner_provider),
            SchedulerStage::Execution => (config.executor_model, config.executor_provider),
            SchedulerStage::Evaluation => (config.evaluator_model, config.evaluator_provider),
        };

        println!("model_id: {}, provider_name: {}", model_id, provider_name);

        if provider_name.trim().is_empty() || model_id.trim().is_empty() {
            return Ok(None);
        }

        let provider_cfg = match self.get_provider_config(&provider_name).await? {
            Some(cfg) => cfg,
            None => return Ok(None),
        };

        let mut dynamic_cfg = provider_cfg;
        dynamic_cfg.model = model_id.clone();

        let app_handle = { self.app_handle.read().unwrap().clone() };
        let service = AiService::new(
            dynamic_cfg,
            self.db.clone(),
            app_handle,
            self.mcp_service.clone(),
        );
        Ok(Some(service))
    }





    /// 获取指定阶段的AI配置，用于框架动态切换模型
    pub async fn get_ai_config_for_stage(
        &self,
        stage: SchedulerStage,
    ) -> anyhow::Result<Option<AiConfig>> {
        let scheduler_config = self.get_scheduler_config().await?;

        let (model_id, provider_name) = match stage {
            SchedulerStage::IntentAnalysis => (
                &scheduler_config.intent_analysis_model,
                &scheduler_config.intent_analysis_provider,
            ),
            SchedulerStage::Planning => (
                &scheduler_config.planner_model,
                &scheduler_config.planner_provider,
            ),
            SchedulerStage::Replanning => (
                &scheduler_config.replanner_model,
                &scheduler_config.replanner_provider,
            ),
            SchedulerStage::Execution => (
                &scheduler_config.executor_model,
                &scheduler_config.executor_provider,
            ),
            SchedulerStage::Evaluation => (
                &scheduler_config.evaluator_model,
                &scheduler_config.evaluator_provider,
            ),
        };

        if model_id.is_empty() {
            return Ok(None);
        }

        // 直接基于配置构建 AI 配置
        if let Some(mut config) = self.get_provider_config(provider_name).await? {
            config.model = model_id.clone();
            Ok(Some(config))
        } else {
            Ok(None)
        }
    }

    pub async fn get_chat_models(&self) -> Result<Vec<ModelInfo>> {
        let mut all_models = Vec::new();

        // 1. 从数据库获取`providers_config`
        let providers_config_str = match self.db.get_config("ai", "providers_config").await? {
            Some(config) => config,
            None => {
                tracing::warn!("'providers_config' not found in database. Attempting to create default configuration.");

                // 检查是否有API密钥配置
                let mut default_providers = HashMap::new();

                // 检查OpenAI API密钥
                if let Ok(Some(openai_key)) = self.db.get_config("ai", "api_key_openai").await {
                    if !openai_key.is_empty() {
                        tracing::info!(
                            "Found OpenAI API key, creating default OpenAI provider config"
                        );
                        let provider_config = serde_json::json!({
                            "id": "OpenAI",
                            "provider": "openai",
                            "name": "OpenAI",
                            "api_key": openai_key,
                            "api_base": "https://api.openai.com/v1",
                            "enabled": true,
                            "default_model": "gpt-4o",
                            "models": [
                                {
                                    "id": "gpt-4o",
                                    "name": "GPT-4o",
                                    "supports_streaming": true,
                                    "supports_tools": true,
                                    "is_available": true
                                },
                                {
                                    "id": "gpt-4o-mini",
                                    "name": "GPT-4o Mini",
                                    "supports_streaming": true,
                                    "supports_tools": true,
                                    "is_available": true
                                }
                            ]
                        });
                        default_providers.insert("OpenAI".to_string(), provider_config);

                        // 添加默认模型
                        all_models.push(ModelInfo {
                            provider: "openai".to_string(),
                            name: "gpt-4o".to_string(),
                            is_chat: true,
                            is_embedding: false,
                        });
                        all_models.push(ModelInfo {
                            provider: "openai".to_string(),
                            name: "gpt-4o-mini".to_string(),
                            is_chat: true,
                            is_embedding: false,
                        });
                    }
                }

                // 检查Anthropic API密钥
                if let Ok(Some(anthropic_key)) = self.db.get_config("ai", "api_key_anthropic").await
                {
                    if !anthropic_key.is_empty() {
                        tracing::info!(
                            "Found Anthropic API key, creating default Anthropic provider config"
                        );
                        let provider_config = serde_json::json!({
                            "id": "Anthropic",
                            "provider": "anthropic",
                            "name": "Anthropic",
                            "api_key": anthropic_key,
                            "api_base": "https://api.anthropic.com",
                            "enabled": true,
                            "default_model": "claude-3-5-sonnet-20241022",
                            "models": [
                                {
                                    "id": "claude-3-5-sonnet-20241022",
                                    "name": "Claude 3.5 Sonnet",
                                    "supports_streaming": true,
                                    "supports_tools": true,
                                    "is_available": true
                                }
                            ]
                        });
                        default_providers.insert("Anthropic".to_string(), provider_config);

                        // 添加默认模型
                        all_models.push(ModelInfo {
                            provider: "anthropic".to_string(),
                            name: "claude-3-5-sonnet-20241022".to_string(),
                            is_chat: true,
                            is_embedding: false,
                        });
                    }
                }

                // 如果找到了API密钥，保存默认配置
                if !default_providers.is_empty() {
                    let config_str = serde_json::to_string(&default_providers).unwrap_or_default();
                    if let Err(e) = self
                        .db
                        .set_config(
                            "ai",
                            "providers_config",
                            &config_str,
                            Some("AI providers configuration"),
                        )
                        .await
                    {
                        tracing::error!("Failed to save default providers config: {}", e);
                    } else {
                        tracing::info!("Successfully saved default providers config");
                        return Ok(all_models);
                    }
                }

                tracing::warn!("Could not find any API keys to create default providers config");
                return Ok(all_models);
            }
        };

        // 解析为服务商名称到服务商配置的映射
        let providers: HashMap<String, serde_json::Value> =
            match serde_json::from_str(&providers_config_str) {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!(
                        "Failed to parse 'providers_config' as map: {}. Content: {}",
                        e,
                        providers_config_str
                    );
                    return Ok(all_models);
                }
            };

        // 2. 遍历所有服务商配置
        for (_provider_key, provider_data) in providers {
            if let Some(provider_obj) = provider_data.as_object() {
                let enabled = provider_obj
                    .get("enabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                // 如果服务商未启用，则跳过
                if !enabled {
                    continue;
                }

                let provider_name = provider_obj
                    .get("provider")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                if provider_name.is_empty() {
                    continue;
                }

                // 从服务商配置中直接解析模型列表
                if let Some(models_val) = provider_obj.get("models") {
                    if let Some(models_arr) = models_val.as_array() {
                        for model_val in models_arr {
                            if let Some(model_obj) = model_val.as_object() {
                                if let Some(model_name) =
                                    model_obj.get("id").and_then(|v| v.as_str())
                                {
                                    all_models.push(ModelInfo {
                                        provider: provider_name.clone(),
                                        name: model_name.to_string(),
                                        is_chat: true,
                                        is_embedding: false,
                                    });
                                }
                            }
                        }
                    }
                } else {
                    // 如果没有找到模型列表，则回退到使用默认模型
                    if let Some(default_model) =
                        provider_obj.get("default_model").and_then(|v| v.as_str())
                    {
                        tracing::warn!("No 'models' array for enabled provider '{}'. Falling back to default model '{}'.", provider_name, default_model);
                        all_models.push(ModelInfo {
                            provider: provider_name.clone(),
                            name: default_model.to_string(),
                            is_chat: true,
                            is_embedding: false,
                        });
                    }
                }
            }
        }

        if all_models.is_empty() {
            tracing::warn!("Could not find any chat models from database configuration. Providing fallback models.");

            // 提供回退模型，即使没有API密钥配置
            all_models = vec![
                ModelInfo {
                    provider: "openai".to_string(),
                    name: "gpt-4o".to_string(),
                    is_chat: true,
                    is_embedding: false,
                },
                ModelInfo {
                    provider: "openai".to_string(),
                    name: "gpt-4o-mini".to_string(),
                    is_chat: true,
                    is_embedding: false,
                },
                ModelInfo {
                    provider: "anthropic".to_string(),
                    name: "claude-3-5-sonnet-20241022".to_string(),
                    is_chat: true,
                    is_embedding: false,
                },
                ModelInfo {
                    provider: "openrouter".to_string(),
                    name: "anthropic/claude-3.5-sonnet".to_string(),
                    is_chat: true,
                    is_embedding: false,
                },
            ];

            tracing::info!("Provided {} fallback models", all_models.len());
        }

        Ok(all_models)
    }

    pub async fn get_embedding_models(&self) -> Result<Vec<ModelInfo>> {
        Ok(vec![])
    }

    pub async fn get_default_model(&self, model_type: &str) -> Result<Option<ModelInfo>> {
        // 从数据库获取默认模型配置
        let config_key = format!("default_{}_model", model_type);

        if let Ok(Some(model_str)) = self.db.get_config("ai", &config_key).await {
            // 解析模型字符串，格式：provider/model_name
            if let Some((provider, model_name)) = model_str.split_once('/') {
                return Ok(Some(ModelInfo {
                    provider: provider.to_string(),
                    name: model_name.to_string(),
                    is_chat: true,
                    is_embedding: false,
                }));
            }
        }

        Ok(None)
    }

    pub async fn set_default_model(
        &self,
        model_type: &str,
        provider: &str,
        model_name: &str,
    ) -> Result<()> {
        // 保存默认模型到数据库，格式：provider/model_name
        let config_key = format!("default_{}_model", model_type);
        let model_value = format!("{}/{}", provider, model_name);

        self.db
            .set_config(
                "ai",
                &config_key,
                &model_value,
                Some(&format!("Default {} model", model_type)),
            )
            .await?;

        tracing::info!("Set default {} model to: {}", model_type, model_value);
        Ok(())
    }

    pub async fn get_default_chat_model(&self) -> Result<Option<(String, String)>> {
        if let Ok(Some(model_str)) = self.db.get_config("ai", "default_chat_model").await {
            // 解析模型字符串，格式：provider/model_name
            if let Some((provider, model_name)) = model_str.split_once('/') {
                return Ok(Some((provider.to_string(), model_name.to_string())));
            }
        }
        Ok(None)
    }

    pub async fn set_default_chat_model(&self, provider: &str, model_name: &str) -> Result<()> {
        self.set_default_model("chat", provider, model_name).await
    }

    pub async fn get_model_config(
        &self,
        _provider: &str,
        _model_name: &str,
    ) -> Result<Option<ModelConfig>> {
        Ok(None)
    }

    pub async fn update_model_config(&self, _config: ModelConfig) -> Result<()> {
        Ok(())
    }
}

impl AiService {
    // 创建新的AI服务实例
    pub fn new(
        config: AiConfig,
        db: Arc<dyn Database + Send + Sync>,
        app_handle: Option<AppHandle>,
        mcp_service: Option<Arc<McpService>>,
    ) -> Self {
        Self {
            config,
            db,
            app_handle,
            mcp_service,
            max_retries: 3,
        }
    }

    // 设置应用句柄
    pub fn set_app_handle(&mut self, app_handle: AppHandle) {
        self.app_handle = Some(app_handle);
    }

    // 设置MCP服务
    pub fn set_mcp_service(&mut self, mcp_service: Arc<McpService>) {
        self.mcp_service = Some(mcp_service);
    }

    // 获取配置信息
    pub fn get_config(&self) -> &AiConfig {
        &self.config
    }

    // 统一的流式发送消息方法
    pub async fn send_message_stream(
        &self,
        user_prompt: Option<&str>,
        system_prompt: Option<&str>,
        conversation_id: Option<String>,
        message_id: Option<String>, // 新增消息ID参数
        stream: bool,
        is_final: bool,
        chunk_type: Option<ChunkType>,
    ) -> Result<String> {
        // 新增参数：skip_save_user_message - 是否跳过保存用户消息到数据库
        // 默认为false（即默认保存）
        self.send_message_stream_with_save_control(
            user_prompt,
            user_prompt, // 默认情况下，保存的内容和发送给LLM的内容一样
            system_prompt,
            conversation_id,
            message_id,
            stream,
            is_final,
            chunk_type,
        ).await
    }

    // 支持分离"发送给LLM的内容"和"保存到数据库的内容"
    pub async fn send_message_stream_with_save_control(
        &self,
        user_prompt_for_llm: Option<&str>, // 发送给LLM的完整prompt
        user_prompt_to_save: Option<&str>, // 保存到数据库的用户消息（可以不同于前者，也可以为None跳过保存）
        system_prompt: Option<&str>,
        conversation_id: Option<String>,
        message_id: Option<String>,
        stream: bool,
        is_final: bool,
        chunk_type: Option<ChunkType>,
    ) -> Result<String> {
        info!("发送流式消息请求 - 模型: {}", self.config.model);

        // 生成或获取execution_id - 确保所有LLM交互都有一个唯一的execution_id
        // 优先使用前端提供的 message_id 作为 execution_id；否则退回到 conversation_id 或新建
        let execution_id = if let Some(ref mid) = message_id {
            mid.clone()
        } else {
            conversation_id
                .clone()
                .unwrap_or_else(|| Uuid::new_v4().to_string())
        };

        // 构建消息列表
        let mut messages = Vec::new();

        // 处理对话历史和系统提示
        match conversation_id {
            Some(ref conv_id) => {

                // 检查对话是否存在，但不自动创建
                let conversation_exists = match self.db.get_ai_conversation(conv_id).await {
                    Ok(Some(_)) => true,
                    Ok(None) => {
                        warn!("会话不存在: {}，不自动创建，需要前端先创建会话", conv_id);
                        false
                    }
                    Err(e) => {
                        warn!("查询AI对话失败: {}", e);
                        false
                    }
                };

                // 获取历史消息
                messages = self
                    .get_conversation_history(conv_id)
                    .await
                    .unwrap_or_else(|e| {
                        warn!("获取对话历史失败: {}, 使用空消息列表", e);
                        Vec::new()
                    });

                // 保存用户消息（如果提供了要保存的内容）
                if let Some(up_to_save) = user_prompt_to_save {
                    if !up_to_save.trim().is_empty() {
                        let user_msg = AiMessage {
                            id: uuid::Uuid::new_v4().to_string(),
                            conversation_id: conv_id.clone(),
                            role: "user".to_string(),
                            content: up_to_save.to_string(), // 保存指定的内容
                            metadata: None,
                            token_count: Some(up_to_save.len() as i32),
                            cost: None,
                            tool_calls: None,
                            attachments: None,
                            timestamp: chrono::Utc::now(),
                        };
                        if conversation_exists {
                            match self.db.create_ai_message(&user_msg).await {
                                Ok(_) => {
                                    debug!("用户消息已保存: {}", up_to_save.chars().take(50).collect::<String>());
                                }
                                Err(e) => {
                                    warn!("用户消息保存失败: {}, 继续执行但不保存到数据库", e)
                                }
                            }
                        } else {
                            debug!("跳过用户消息保存：对话记录不存在");
                        }
                        // 注意：messages列表中使用发送给LLM的完整内容
                        messages.push(AiMessage {
                            id: uuid::Uuid::new_v4().to_string(),
                            conversation_id: conv_id.clone(),
                            role: "user".to_string(),
                            content: user_prompt_for_llm.unwrap_or("").to_string(),
                            metadata: None,
                            token_count: Some(user_prompt_for_llm.unwrap_or("").len() as i32),
                            cost: None,
                            tool_calls: None,
                            attachments: None,
                            timestamp: chrono::Utc::now(),
                        });
                    }
                } else {
                    debug!("跳过用户消息保存（user_prompt_to_save 为 None）");
                    // 即使不保存，也要构建消息用于LLM调用
                    if let Some(up) = user_prompt_for_llm {
                        if !up.trim().is_empty() {
                            messages.push(AiMessage {
                                id: uuid::Uuid::new_v4().to_string(),
                                conversation_id: conv_id.clone(),
                                role: "user".to_string(),
                                content: up.to_string(),
                                metadata: None,
                                token_count: Some(up.len() as i32),
                                cost: None,
                                tool_calls: None,
                                attachments: None,
                                timestamp: chrono::Utc::now(),
                            });
                        }
                    }
                }


                // 内联原 send_chat_stream 逻辑：Rig 动态客户端真流式
                {
                    use futures::StreamExt;
                    use rig::client::builder::DynClientBuilder;
                    use rig::streaming::{StreamedAssistantContent, StreamingPrompt};
                    use rig::agent::MultiTurnStreamItem;

                    // 生成或获取 message_id / execution_id
                    let message_id = message_id
                        .clone()
                        .or_else(|| messages.last().map(|m| m.id.clone()))
                        .unwrap_or_else(|| "unknown".to_string());
                    let exec_id = execution_id.clone();

                    // 校验配置
                    if self.config.provider == "unconfigured" || self.config.provider == "mock" {
                        let error_msg = "AI provider not configured. Please go to Settings > AI Configuration";
                        error!("{}", error_msg);
                        return Err(anyhow::anyhow!(error_msg));
                    }
                    if self.config.api_key.as_ref().map_or(true, |k| k.is_empty()) {
                        let error_msg = format!(
                            "API key not configured for provider '{}'. Please check your AI configuration settings.",
                            self.config.provider
                        );
                        error!("{}", error_msg);
                        return Err(anyhow::anyhow!(error_msg));
                    }

                    // 构造上下文
                    if messages.is_empty() {
                        let error_msg = "No messages provided for chat completion";
                        error!("{}", error_msg);
                        return Err(anyhow::anyhow!(error_msg));
                    }
                    let mut history_buf = String::new();
                    let take = messages.len().min(12);
                    for m in messages.iter().rev().take(take).rev() {
                        history_buf.push_str(&format!("{}\n", m.content));
                    }
                    let user_input = history_buf.trim();
                    if user_input.is_empty() {
                        let error_msg = "Message content is empty after processing";
                        error!("{}", error_msg);
                        return Err(anyhow::anyhow!(error_msg));
                    }

                    // Rig DynClient 真流式
                    let provider = self.config.provider.to_lowercase();
                    let model = self.config.model.clone();
                    info!(
                        "Starting Rig dyn client streaming: provider={}, model={}, input_len={}",
                        provider,
                        model,
                        user_input.len()
                    );
                    
                    // 创建日志记录器
                    let logger = LLMLoggingHook::new(
                        exec_id.clone(),
                        Some(conv_id.clone()),
                        provider.clone(),
                        model.clone(),
                    );
                    
                    // 记录请求开始
                    logger.write_to_log("REQUEST_START", &format!("Input length: {} chars", user_input.len()));
                    logger.write_to_log("REQUEST_START", &format!("\n\n system prompt:\n {}\n\n", system_prompt.unwrap_or("You are a helpful AI assistant.")));
                    logger.write_to_log("REQUEST_START", &format!("\n\n User input:\n {}\n\n", user_input));
                    
                    let agent = {
                        let client = DynClientBuilder::new();
                        let mut agent_builder = client
                            .agent(&provider, &model)?
                            .preamble(system_prompt.unwrap_or("You are a helpful AI assistant."));
                        
                        // 为 Gemini provider 添加必需的 generationConfig
                        if provider == "gemini" {
                            let gen_cfg = GenerationConfig::default();
                            let cfg = AdditionalParameters::default().with_config(gen_cfg);
                            agent_builder = agent_builder.additional_params(serde_json::to_value(cfg).unwrap());
                        }
                        agent_builder.build()
                    };

                    let mut content = String::new();
                    let mut stream_iter = agent.stream_prompt(user_input).await;
                    while let Some(item) = stream_iter.next().await {
                        match item {
                            Ok(MultiTurnStreamItem::StreamItem(StreamedAssistantContent::Text(t))) => {
                                let piece = t.text;
                                if piece.is_empty() { continue; }
                                content.push_str(&piece);
                                if stream {
                                    self.emit_message_chunk(
                                        &exec_id,
                                        &message_id,
                                        Some(conv_id),
                                        chunk_type.clone(),
                                        &piece,
                                        false,
                                        None,
                                    );
                                }
                            }
                            Ok(MultiTurnStreamItem::StreamItem(StreamedAssistantContent::Reasoning(r))) => {
                                let piece = r.reasoning.join("");
                                if !piece.is_empty() && stream {
                                    self.emit_message_chunk(
                                        &exec_id,
                                        &message_id,
                                        Some(conv_id),
                                        Some(ChunkType::Thinking),
                                        &piece,
                                        false,
                                        None,
                                    );
                                }
                            }
                            Ok(MultiTurnStreamItem::StreamItem(StreamedAssistantContent::ToolCall(_))) => {}
                            Ok(MultiTurnStreamItem::FinalResponse(_)) => { break; }
                            Ok(_) => { /* ignore other stream items */ }
                            Err(e) => {
                                error!("Dyn client stream error: {}", e);
                                return Err(anyhow::anyhow!(format!("Stream error: {}", e)));
                            }
                        }
                    }
                    
                    // 记录响应完成
                    logger.write_to_log("RESPONSE_COMPLETE", &format!("Response length: {} chars", content.len()));
                    logger.write_to_log("RESPONSE_COMPLETE", &format!("Response:\n {}\n\n", content));

                    if is_final {
                        self.emit_message_chunk(
                            &exec_id,
                            &message_id,
                            Some(conv_id),
                            Some(ChunkType::Meta),
                            "",
                            true,
                            None,
                        );
                    }

                    return Ok(content);
                }
            }
            None => {
                // 无会话：仅进行内部流式调用，不向前端发块、不写入DB
                let conv_id = &execution_id; // 使用 execution_id 作为临时会话标识
                // 构建系统消息
                if let Some(sp) = system_prompt {
                    if !sp.is_empty() {
                        let sys_msg = AiMessage {
                            id: uuid::Uuid::new_v4().to_string(),
                            conversation_id: conv_id.to_string(),
                            role: "system".to_string(),
                            content: sp.to_string(),
                            metadata: None,
                            token_count: Some(sp.len() as i32),
                            cost: None,
                            tool_calls: None,
                            attachments: None,
                            timestamp: chrono::Utc::now(),
                        };
                        messages.push(sys_msg);
                    }
                }

                // 构建用户消息
                if let Some(up) = user_prompt_for_llm {
                    if !up.is_empty() {
                        let user_msg = AiMessage {
                            id: uuid::Uuid::new_v4().to_string(),
                            conversation_id: conv_id.to_string(),
                            role: "user".to_string(),
                            content: up.to_string(),
                            metadata: None,
                            token_count: Some(up.len() as i32),
                            cost: None,
                            tool_calls: None,
                            attachments: None,
                            timestamp: chrono::Utc::now(),
                        };
                        messages.push(user_msg);
                    }
                }

                // 不保存到DB，不向前端发块（通过 should_save_to_db=false 抑制发块）
                let _model_name_owned = self.config.model.clone();
                // 内联原 send_chat_stream 逻辑（无会话）：Rig 动态客户端真流式
                {


                    let conv_id = conv_id; // same binding name
                    let message_id = message_id
                        .clone()
                        .or_else(|| messages.last().map(|m| m.id.clone()))
                        .unwrap_or_else(|| "unknown".to_string());
                    let exec_id = execution_id.clone();

                    if messages.is_empty() {
                        let error_msg = "No messages provided for chat completion";
                        error!("{}", error_msg);
                        return Err(anyhow::anyhow!(error_msg));
                    }
                    let mut history_buf = String::new();
                    let take = messages.len().min(12);
                    for m in messages.iter().rev().take(take).rev() {
                        history_buf.push_str(&format!("{}\n", m.content));
                    }
                    let user_input = history_buf.trim();
                    if user_input.is_empty() {
                        let error_msg = "Message content is empty after processing";
                        error!("{}", error_msg);
                        return Err(anyhow::anyhow!(error_msg));
                    }

                    let provider = self.config.provider.to_lowercase();
                    let model = self.config.model.clone();
                    
                    // 创建日志记录器 (无会话)
                    let logger = LLMLoggingHook::new(
                        exec_id.clone(),
                        None, // 无会话ID
                        provider.clone(),
                        model.clone(),
                    );
                    
                    // 记录请求开始
                    logger.write_to_log("REQUEST_START", &format!("Input length: {} chars", user_input.len()));
                    logger.write_to_log("REQUEST_START", &format!("system prompt:\n {}\n\n", system_prompt.unwrap_or("You are a helpful AI assistant.")));
                    logger.write_to_log("REQUEST_START", &format!("User input:\n {}\n\n", user_input));
                    
                    let agent = {
                        let client = DynClientBuilder::new();
                        let mut agent_builder = client
                            .agent(&provider, &model)?
                            .preamble(system_prompt.unwrap_or("You are a helpful AI assistant."));
                        
                        // 为 Gemini provider 添加必需的 generationConfig
                        if provider == "gemini" {
                            let gen_cfg = GenerationConfig {
                                ..Default::default()
                            };
                            let cfg = AdditionalParameters::default().with_config(gen_cfg);
                            agent_builder = agent_builder.additional_params(serde_json::to_value(cfg).unwrap());
                        }
                        
                        agent_builder.build()
                    };

                    let mut content = String::new();
                    let mut stream_iter = agent.stream_prompt(user_input).await;
                    while let Some(item) = stream_iter.next().await {
                        match item {
                            Ok(MultiTurnStreamItem::StreamItem(StreamedAssistantContent::Text(t))) => {
                                let piece = t.text;
                                if piece.is_empty() { continue; }
                                content.push_str(&piece);
                                if stream {
                                    self.emit_message_chunk(
                                        &exec_id,
                                        &message_id,
                                        Some(conv_id),
                                        chunk_type.clone(),
                                        &piece,
                                        false,
                                        None,
                                    );
                                }
                            }
                            Ok(MultiTurnStreamItem::StreamItem(StreamedAssistantContent::Reasoning(r))) => {
                                let piece = r.reasoning.join("");
                                if !piece.is_empty() && stream {
                                    self.emit_message_chunk(
                                        &exec_id,
                                        &message_id,
                                        Some(conv_id),
                                        Some(ChunkType::Thinking),
                                        &piece,
                                        false,
                                        None,
                                    );
                                }
                            }
                            Ok(MultiTurnStreamItem::StreamItem(StreamedAssistantContent::ToolCall(_))) => {}
                            Ok(MultiTurnStreamItem::FinalResponse(_)) => { break; }
                            Ok(_) => { /* ignore other stream items */ }
                            Err(e) => return Err(anyhow::anyhow!(format!("Stream error: {}", e))),
                        }
                    }

                    // 记录响应完成
                    logger.write_to_log("RESPONSE_COMPLETE", &format!("Response length: {} chars", content.len()));
                    logger.write_to_log("RESPONSE_COMPLETE", &format!("Response:\n {}\n\n", content));

                    if is_final {
                        self.emit_message_chunk(
                            &exec_id,
                            &message_id,
                            Some(conv_id),
                            Some(ChunkType::Meta),
                            "",
                            true,
                            None,
                        );
                    }

                    return Ok(content);
                }
            }
        }
    }

    // 已移除 send_chat_stream，逻辑合并入 send_message_stream

    // rig_complete 已合并进 send_message_stream，无需保留

    // 创建新对话
    pub async fn create_conversation(&self, title: Option<String>) -> Result<String> {
        let mut conversation = AiConversation::new(
            self.config.model.clone(),
            self.config.provider.clone(), // TODO: This should be the service name, not provider
        );
        conversation.id = Uuid::new_v4().to_string();
        conversation.title = title;

        self.db.create_ai_conversation(&conversation).await?;
        Ok(conversation.id)
    }

    // 获取对话历史
    pub async fn get_conversation_history(&self, conversation_id: &str) -> Result<Vec<AiMessage>> {
        self.db
            .get_ai_messages_by_conversation(conversation_id)
            .await
    }

    // 保存消息到数据库
    #[allow(unused)]
    async fn save_message(&self, conversation_id: &str, role: &str, content: &str) -> Result<()> {
        let message = AiMessage {
            id: Uuid::new_v4().to_string(),
            conversation_id: conversation_id.to_string(),
            role: role.to_string(),
            content: content.to_string(),
            metadata: None,
            token_count: Some(content.len() as i32), // 简单估算
            cost: Some(0.0),
            tool_calls: None,
            attachments: None,
            timestamp: Utc::now(),
        };

        self.db.create_ai_message(&message).await?;
        Ok(())
    }

    // 保存带工具调用的消息到数据库
    #[allow(unused)]
    async fn save_message_with_tool_calls(
        &self,
        conversation_id: &str,
        role: &str,
        content: &str,
        tool_calls: Vec<AiToolCall>,
    ) -> Result<()> {
        let tool_calls_json = serde_json::to_string(&tool_calls)?;

        let message = AiMessage {
            id: Uuid::new_v4().to_string(),
            conversation_id: conversation_id.to_string(),
            role: role.to_string(),
            content: content.to_string(),
            metadata: None,
            token_count: Some(content.len() as i32), // 简单估算
            cost: Some(0.0),
            tool_calls: Some(tool_calls_json),
            attachments: None,
            timestamp: Utc::now(),
        };

        self.db.create_ai_message(&message).await?;
        Ok(())
    }

    // 注意：已删除 create_scan_task_for_tool、update_scan_task_completed 和 update_scan_task_failed 方法
    // 这些方法只被已删除的 execute_tool_call 方法使用，现在不再需要

    // 删除对话
    pub async fn delete_conversation(&self, conversation_id: &str) -> Result<()> {
        self.db.delete_ai_conversation(conversation_id).await
    }

    // 获取所有对话
    pub async fn list_conversations(&self) -> Result<Vec<AiConversation>> {
        self.db.get_ai_conversations().await
    }

    // 更新对话标题
    pub async fn update_conversation_title(
        &self,
        conversation_id: &str,
        title: &str,
    ) -> Result<()> {
        self.db
            .update_ai_conversation_title(conversation_id, title)
            .await
    }

    // 归档对话
    pub async fn archive_conversation(&self, conversation_id: &str) -> Result<()> {
        self.db.archive_ai_conversation(conversation_id).await
    }

    /// 统一发送消息块到前端
    fn emit_message_chunk(
        &self,
        execution_id: &str,
        message_id: &str,
        conversation_id: Option<&str>,
        chunk_type: Option<ChunkType>,
        content: &str,
        is_final: bool,
        stage: Option<&str>,
    ) {
        if let Some(app_handle) = &self.app_handle {
            crate::utils::ordered_message::emit_message_chunk(
                app_handle,
                execution_id,
                message_id,
                conversation_id,
                chunk_type.unwrap_or(ChunkType::Content),
                content,
                is_final,
                stage,
                None
            );
        }
    }
}

