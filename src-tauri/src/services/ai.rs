use std::collections::HashMap;
use std::sync::Arc;
use std::env;
use genai::{
    chat::{ChatMessage, ChatRequest, ChatOptions, Tool, ToolResponse},
    Client as GenaiClient,
};
use serde::{Deserialize, Serialize};
use crate::models::database::{AiConversation, AiMessage};
use crate::services::database::Database;
use anyhow::Result;
use uuid::Uuid;
use chrono::Utc;
use tauri::{AppHandle, Emitter};
use tracing::{info, debug, error};
use crate::services::mcp::McpService;
use serde_json::Value;
use regex;
use crate::commands::ai::{ModelInfo, ModelConfig};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamError {
    pub conversation_id: String,
    pub error: String,
}

#[derive(Debug, Clone)]
pub struct AiService {
    client: Arc<GenaiClient>,
    config: AiConfig,
    db: Arc<dyn Database + Send + Sync>,
    app_handle: Option<AppHandle>,
    mcp_service: Option<Arc<McpService>>,
}

#[derive(Debug, Clone)]
pub struct AiServiceManager {
    services: Arc<std::sync::RwLock<HashMap<String, AiService>>>,
    db: Arc<dyn Database + Send + Sync>,
    app_handle: Arc<std::sync::RwLock<Option<AppHandle>>>,
    mcp_service: Option<Arc<McpService>>,
}

#[derive(Debug, Deserialize)]
struct ProviderConfig {
    id: String,
    provider: String,
    name: String,
    api_key: Option<String>,
    api_base: Option<String>,
    organization: Option<String>,
    enabled: bool,
    default_model: String,
    models: Vec<ModelDefinition>,
}

#[derive(Debug, Deserialize)]
struct ModelDefinition {
    id: String,
    name: String,
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

    // 设置MCP服务
    pub fn set_mcp_service(&mut self, mcp_service: Arc<McpService>) {
        self.mcp_service = Some(mcp_service.clone());
        
        // 更新所有已存在的服务
        let mut services = self.services.write().unwrap();
        for service in services.values_mut() {
            service.set_mcp_service(mcp_service.clone());
        }
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
        let client = GenaiClient::default();
        
        let service = AiService {
            client: Arc::new(client),
            config,
            db: self.db.clone(),
            app_handle: self.app_handle.read().unwrap().clone(),
            mcp_service: self.mcp_service.clone(),
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
        tracing::info!("Initializing default AI services...");
        
        // 创建genai客户端
        let client = GenaiClient::default();
        tracing::info!("Successfully created GenAI client");
        
        // 从数据库加载并解析providers_config
        if let Ok(Some(config_str)) = self.db.get_config("ai", "providers_config").await {
            match serde_json::from_str::<HashMap<String, ProviderConfig>>(&config_str) {
                Ok(providers) => {
                    tracing::info!("Successfully parsed 'providers_config' from DB.");
                    for (id, provider_config) in providers {
                        if !provider_config.enabled {
                            continue;
                        }

                        tracing::info!("Initializing enabled provider: {}", provider_config.name);

                        let api_key = provider_config.api_key.as_deref().map(String::from);
                        std::env::set_var(format!("{}_API_KEY", provider_config.name.to_uppercase()), api_key.as_deref().unwrap_or(""));

                        let default_model = provider_config.default_model.clone();

                        let api_base = provider_config.api_base.filter(|s| !s.is_empty()).map(String::from);

                        let organization = provider_config.organization.filter(|s| !s.is_empty()).map(String::from);
                        
                        let config = AiConfig {
                            provider: provider_config.provider.clone(),
                            model: default_model.clone(),
                            api_key: api_key,
                            api_base: api_base,
                            organization: organization,
                            temperature: None,
                            max_tokens: None,
                        };
                        
                        if let Err(e) = self.add_service(provider_config.name.clone(), config).await {
                            tracing::error!("Failed to add service for provider {}: {}", provider_config.name, e);
                        }
                    }
                },
                Err(e) => {
                    tracing::error!("Failed to parse 'providers_config' as map: {}. Falling back to env vars. Content: {}", e, config_str);
                    return self.init_services_from_env().await;
                }
            }
        } else {
            tracing::info!("'providers_config' not found in database. Trying to initialize from environment variables.");
            return self.init_services_from_env().await;
        }
        
        // 如果没有从数据库配置中成功添加任何服务，则尝试从环境变量初始化
        if self.services.read().unwrap().is_empty() {
            tracing::info!("No services initialized from database, attempting to initialize from environment variables.");
            if let Err(e) = self.init_services_from_env().await {
                 tracing::error!("Failed to initialize services from environment variables: {}", e);
            }
        }

        // 如果仍然没有任何服务，创建一个默认的别名服务
        if self.services.read().unwrap().is_empty() {
            tracing::warn!("No AI services could be initialized. Creating a default alias service.");
            if let Err(e) = self.create_default_alias().await {
                tracing::error!("Failed to create default alias service: {}", e);
            }
        }

        tracing::info!("Finished initializing AI services. Total services: {}", self.services.read().unwrap().len());
        
        Ok(())
    }
    
    // 从环境变量初始化服务
    async fn init_services_from_env(&self) -> anyhow::Result<()> {
        // 检查各种API密钥环境变量，为每个可用的提供商添加服务
        
        // OpenAI
        if std::env::var("OPENAI_API_KEY").is_ok() {
            let config = AiConfig {
                provider: "openai".to_string(),
                model: "gpt-4o".to_string(),
                api_key: std::env::var("OPENAI_API_KEY").ok(),
                api_base: std::env::var("OPENAI_API_BASE").ok(),
                organization: std::env::var("OPENAI_ORGANIZATION").ok(),
                temperature: Some(0.7),
                max_tokens: Some(1000),
            };
            self.add_service("openai".to_string(), config).await?;
            tracing::info!("Added OpenAI service");
        }
        
        // Anthropic
        if std::env::var("ANTHROPIC_API_KEY").is_ok() {
            let config = AiConfig {
                provider: "anthropic".to_string(),
                model: "claude-3-sonnet-20240229".to_string(),
                api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
                api_base: std::env::var("ANTHROPIC_API_BASE").ok(),
                organization: None,
                temperature: Some(0.7),
                max_tokens: Some(1000),
            };
            self.add_service("anthropic".to_string(), config).await?;
            tracing::info!("Added Anthropic service");
        }
        
        // Gemini
        if std::env::var("GEMINI_API_KEY").is_ok() {
            let config = AiConfig {
                provider: "gemini".to_string(),
                model: "gemini-1.5-pro".to_string(),
                api_key: std::env::var("GEMINI_API_KEY").ok(),
                api_base: None,
                organization: None,
                temperature: Some(0.7),
                max_tokens: Some(1000),
            };
            self.add_service("gemini".to_string(), config).await?;
            tracing::info!("Added Gemini service");
        }
        
        // DeepSeek
        if std::env::var("DEEPSEEK_API_KEY").is_ok() {
            let config = AiConfig {
                provider: "deepseek".to_string(),
                model: "deepseek-chat".to_string(),
                api_key: std::env::var("DEEPSEEK_API_KEY").ok(),
                api_base: std::env::var("DEEPSEEK_API_BASE").ok(),
                organization: None,
                temperature: Some(0.7),
                max_tokens: Some(1000),
            };
            self.add_service("deepseek".to_string(), config).await?;
            tracing::info!("Added DeepSeek service");
        }
        
        // Groq
        if std::env::var("GROQ_API_KEY").is_ok() {
            let config = AiConfig {
                provider: "groq".to_string(),
                model: "llama3-70b-8192".to_string(),
                api_key: std::env::var("GROQ_API_KEY").ok(),
                api_base: None,
                organization: None,
                temperature: Some(0.7),
                max_tokens: Some(1000),
            };
            self.add_service("groq".to_string(), config).await?;
            tracing::info!("Added Groq service");
        }
        
        // Cohere
        if std::env::var("COHERE_API_KEY").is_ok() {
            let config = AiConfig {
                provider: "cohere".to_string(),
                model: "command-r".to_string(),
                api_key: std::env::var("COHERE_API_KEY").ok(),
                api_base: None,
                organization: None,
                temperature: Some(0.7),
                max_tokens: Some(1000),
            };
            self.add_service("cohere".to_string(), config).await?;
            tracing::info!("Added Cohere service");
        }
        
        // xAI/Grok
        if std::env::var("XAI_API_KEY").is_ok() {
            let config = AiConfig {
                provider: "xai".to_string(),
                model: "grok-1".to_string(),
                api_key: std::env::var("XAI_API_KEY").ok(),
                api_base: None,
                organization: None,
                temperature: Some(0.7),
                max_tokens: Some(1000),
            };
            self.add_service("xai".to_string(), config).await?;
            tracing::info!("Added xAI/Grok service");
        }
        
        // Ollama (本地)
        let config = AiConfig {
            provider: "ollama".to_string(),
            model: "llama3:8b".to_string(),
            api_key: None,
            api_base: Some("http://localhost:11434".to_string()),
            organization: None,
            temperature: Some(0.7),
            max_tokens: Some(1000),
        };
        match self.add_service("ollama".to_string(), config).await {
            Ok(_) => tracing::info!("Added Ollama service"),
            Err(e) => tracing::warn!("Failed to add Ollama service: {}", e),
        }
        
        Ok(())
    }
    
    // 创建默认服务（当没有任何配置时）
    async fn create_default_service(&self) -> anyhow::Result<()> {
        tracing::info!("Creating default AI service (using mock configuration)");
        
        // 创建一个默认的配置，但不包含真实的API密钥
        let default_config = AiConfig {
            provider: "default".to_string(),
            model: "default-model".to_string(),
            api_key: None,
            api_base: None,
            organization: None,
            temperature: Some(0.7),
            max_tokens: Some(1000),
        };
        
        self.add_service("default".to_string(), default_config).await?;
        tracing::info!("Created default AI service");
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
        let preferred_providers = vec!["deepseek", "openai", "anthropic", "gemini", "groq", "ollama"];
        
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
                    tracing::info!("Created default service alias, pointing to {}", provider);
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
                tracing::info!("Created default service alias, pointing to {}", first_service_name);
            }
        }
        
        Ok(())
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
                        tracing::info!("Found OpenAI API key, creating default OpenAI provider config");
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
                if let Ok(Some(anthropic_key)) = self.db.get_config("ai", "api_key_anthropic").await {
                    if !anthropic_key.is_empty() {
                        tracing::info!("Found Anthropic API key, creating default Anthropic provider config");
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
                    if let Err(e) = self.db.set_config("ai", "providers_config", &config_str, Some("AI providers configuration")).await {
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
        let providers: HashMap<String, serde_json::Value> = match serde_json::from_str(&providers_config_str) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("Failed to parse 'providers_config' as map: {}. Content: {}", e, providers_config_str);
                return Ok(all_models);
            }
        };
        
        // 2. 遍历所有服务商配置
        for (_provider_key, provider_data) in providers {
            if let Some(provider_obj) = provider_data.as_object() {
                let enabled = provider_obj.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);

                // 如果服务商未启用，则跳过
                if !enabled {
                    continue;
                }

                let provider_name = provider_obj.get("provider")
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
                                if let Some(model_name) = model_obj.get("id").and_then(|v| v.as_str()) {
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
                     if let Some(default_model) = provider_obj.get("default_model").and_then(|v| v.as_str()) {
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
            tracing::warn!("Could not find any chat models from database configuration.");
        }

        Ok(all_models)
    }

    pub async fn get_embedding_models(&self) -> Result<Vec<ModelInfo>> {
        // Placeholder implementation
        Ok(vec![])
    }

    pub async fn get_default_model(&self, model_type: &str) -> Result<Option<ModelInfo>> {
        // Placeholder implementation
        Ok(None)
    }

    pub async fn set_default_model(&self, model_type: &str, provider: &str, model_name: &str) -> Result<()> {
        // Placeholder implementation
        Ok(())
    }

    pub async fn get_model_config(&self, provider: &str, model_name: &str) -> Result<Option<ModelConfig>> {
        // Placeholder implementation
        Ok(None)
    }

    pub async fn update_model_config(&self, config: ModelConfig) -> Result<()> {
        // Placeholder implementation
        Ok(())
    }
}

impl AiService {
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

    // 发送消息（非流式）
    pub async fn send_message(&self, message: &str, conversation_id: Option<String>) -> Result<String> {
        // 获取可用的MCP工具信息
        let mut tool_info = String::new();
        if let Some(mcp_service) = &self.mcp_service {
            match mcp_service.get_available_tools().await {
                Ok(tools) => {
                    if !tools.is_empty() {
                        tool_info.push_str("\n\nAvailable security tools:\n");
                        for tool in tools.iter().take(10) { // 限制显示的工具数量
                            tool_info.push_str(&format!(
                                "- {}: {} (Category: {:?})\n",
                                tool.name, 
                                tool.description,
                                tool.category
                            ));
                        }
                        tool_info.push_str("\nWhen users ask about related functionality, you can recommend appropriate tools.");
                    }
                }
                Err(e) => {
                    debug!("Failed to get MCP tools: {}", e);
                }
            }
        }
        
        let system_prompt = format!(
            "You are a professional cybersecurity expert specializing in vulnerability discovery and security analysis. Provide professional, accurate advice to help users with security-related questions.{}",
            tool_info
        );
        
        let chat_request = ChatRequest::new(vec![
            ChatMessage::system(system_prompt),
            ChatMessage::user(message.to_string()),
        ]);

        let response = self.client.exec_chat(&self.config.model, chat_request, None).await?;
        
        let response_text = response.first_text().unwrap_or("No response").to_string();

        // 保存到数据库
        if let Some(conv_id) = conversation_id {
            self.save_message(&conv_id, "user", message).await?;
            self.save_message(&conv_id, "assistant", &response_text).await?;
        }

        Ok(response_text)
    }

    // 发送消息（流式）- 核心功能
    pub async fn send_message_stream(
        &self,
        message: &str,
        conversation_id: String,
    ) -> Result<String> {
        self.send_message_stream_with_prompt(message, conversation_id, None).await
    }

    // 发送带自定义系统提示的流式消息
    pub async fn send_message_stream_with_prompt(
        &self,
        message: &str,
        conversation_id: String,
        system_prompt: Option<String>,
    ) -> Result<String> {
        let conversation_id = conversation_id.clone();
        let message = message.to_string();
        let service = self.clone();
        
        let message_id = Uuid::new_v4().to_string();
        let message_id_clone = message_id.clone();
        
        tokio::spawn(async move {
            let conv_id = conversation_id;

            // 如果没有提供system_prompt，尝试从数据库加载
            let mut messages = match service.get_conversation_history(&conv_id).await {
                Ok(history) => history,
                Err(e) => {
                    error!("Failed to get conversation history for {}: {}", conv_id, e);
                    vec![]
                }
            };
            
            // 添加系统提示（如果提供）
            if let Some(prompt) = system_prompt {
                messages.insert(0, AiMessage {
                    id: Uuid::new_v4().to_string(),
                    conversation_id: conv_id.clone(),
                    role: "system".to_string(),
                    content: prompt,
                    metadata: None,
                    token_count: None,
                    cost: None,
                    tool_calls: None,
                    attachments: None,
                    timestamp: Utc::now(),
                });
            }

            // 添加当前用户消息
            if let Err(e) = service.save_message(&conv_id, "user", &message).await {
                error!("Failed to save user message for {}: {}", conv_id, e);
            }
            messages.push(AiMessage {
                id: Uuid::new_v4().to_string(),
                conversation_id: conv_id.clone(),
                role: "user".to_string(),
                content: message,
                metadata: None,
                token_count: None,
                cost: None,
                tool_calls: None,
                attachments: None,
                timestamp: Utc::now(),
            });

            match service.send_chat_request(
                &service.config.model,
                messages,
                &conv_id,
                service.config.temperature,
                service.config.max_tokens
            ).await {
                Ok(response_content) => {
                    if let Err(e) = service.save_message(&conv_id, "assistant", &response_content).await {
                        error!("Failed to save assistant message for {}: {}", conv_id, e);
                    }
                },
                Err(e) => {
                    error!("Error sending chat request for conversation {}: {}", conv_id, e);
                    service.emit_stream_message(&conv_id, &message_id_clone, &format!("Error: {}", e), true, None).await;
                }
            }
        });

        Ok(message_id)
    }

    // 发送聊天请求（处理工具调用）
    async fn send_chat_request(
        &self,
        model_name: &str,
        mut messages: Vec<AiMessage>,
        conversation_id: &str,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Result<String> {
        info!("Sending chat request to {} model", model_name);
        
        // 转换消息格式
        let mut chat_messages = Vec::new();
        for msg in &messages {
            match msg.role.as_str() {
                "user" => chat_messages.push(ChatMessage::user(&msg.content)),
                "assistant" => {
                    if let Some(tc_json) = &msg.tool_calls {
                        if let Ok(tool_calls) = serde_json::from_str::<Vec<genai::chat::ToolCall>>(tc_json) {
                            chat_messages.push(ChatMessage::from(tool_calls));
                        } else {
                            chat_messages.push(ChatMessage::assistant(&msg.content));
                        }
                    } else {
                        chat_messages.push(ChatMessage::assistant(&msg.content));
                    }
                },
                "system" => chat_messages.push(ChatMessage::system(&msg.content)),
                "tool" => {
                    if let Some(metadata_str) = &msg.metadata {
                        if let Ok(metadata) = serde_json::from_str::<Value>(metadata_str) {
                            if let Some(tool_call_id) = metadata.get("tool_call_id").and_then(|v| v.as_str()) {
                                let tool_response = ToolResponse::new(tool_call_id.to_string(), msg.content.clone());
                                chat_messages.push(ChatMessage::from(tool_response));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        
        // 准备工具
        let mut tools_vec = Vec::new();
        if let Some(mcp_service) = &self.mcp_service {
            if let Ok(available_tools) = mcp_service.get_available_tools().await {
                for t in available_tools {
                    let params_schema = if t.parameters.schema.is_null() {
                        serde_json::json!({"type":"object","properties":{}})
                    } else {
                        t.parameters.schema.clone()
                    };
                    tools_vec.push(Tool {
                        name: t.name,
                        description: Some(t.description),
                        schema: Some(params_schema),
                        config: None,
                    });
                }
            }
        }
        
        // 设置选项
        let mut options = ChatOptions::default();
        if let Some(temp) = temperature {
            options.temperature = Some(temp as f64);
        }
        if let Some(tokens) = max_tokens {
            options.max_tokens = Some(tokens as u32);
        }
        
        // 构建请求
        let mut chat_req = ChatRequest::new(chat_messages);
        if !tools_vec.is_empty() {
            chat_req = chat_req.with_tools(tools_vec.clone());
        }
        
        // 发送请求
        let response = self.client
            .exec_chat(model_name, chat_req, Some(&options))
            .await
            .map_err(|e| anyhow::anyhow!("Web call failed for model '{}'. Cause: {}", model_name, e))?;
        
        // 检查是否包含工具调用
        let tool_calls = response.tool_calls();
        if !tool_calls.is_empty() {
            info!("Executing tool calls: {:?}", tool_calls);
            
            // 1. 保存带有工具调用的助手消息
            let answer_text = response.first_text().unwrap_or_default();
            let assistant_msg = AiMessage {
                id: Uuid::new_v4().to_string(),
                conversation_id: conversation_id.to_string(),
                role: "assistant".to_string(),
                content: answer_text.to_string(),
                metadata: None,
                token_count: Some(answer_text.len() as i32),
                cost: Some(0.0),
                tool_calls: Some(serde_json::to_string(&tool_calls).unwrap_or_else(|_| "{}".to_string())),
                attachments: None,
                timestamp: Utc::now(),
            };
            
            self.db.create_ai_message(&assistant_msg).await?;
            messages.push(assistant_msg);
            
            // 2. 执行每个工具调用
            let mut tool_messages = Vec::new();
            for tc in tool_calls {
                let tool_name = &tc.fn_name;
                let args_val = tc.fn_arguments.clone();
                
                let exec_res = if let Some(mcp) = &self.mcp_service {
                    mcp.execute_tool(tool_name, args_val.clone()).await
                } else {
                    Err(anyhow::anyhow!("MCP service unavailable"))
                };
                
                // 3. 创建工具响应消息
                let tool_msg = match &exec_res {
                    Ok(result) => {
                        let result_str = serde_json::to_string(result).unwrap_or_else(|_| "{}".to_string());
                        AiMessage {
                            id: Uuid::new_v4().to_string(),
                            conversation_id: conversation_id.to_string(),
                            role: "tool".to_string(),
                            content: result_str.clone(),
                            metadata: Some(format!("{{\"tool_call_id\":\"{}\",\"tool_name\":\"{}\"}}", tc.call_id, tool_name)),
                            token_count: Some(result_str.len() as i32),
                            cost: None,
                            tool_calls: None,
                            attachments: None,
                            timestamp: Utc::now(),
                        }
                    },
                    Err(e) => {
                        let error_str = format!("{{\"error\":\"{}\"}}", e);
                        AiMessage {
                            id: Uuid::new_v4().to_string(),
                            conversation_id: conversation_id.to_string(),
                            role: "tool".to_string(),
                            content: error_str.clone(),
                            metadata: Some(format!("{{\"tool_call_id\":\"{}\",\"tool_name\":\"{}\"}}", tc.call_id, tool_name)),
                            token_count: Some(error_str.len() as i32),
                            cost: None,
                            tool_calls: None,
                            attachments: None,
                            timestamp: Utc::now(),
                        }
                    }
                };
                
                self.db.create_ai_message(&tool_msg).await?;
                tool_messages.push(tool_msg);
            }
            
            messages.extend(tool_messages);
            
            // 4. 再次调用API，附带工具结果，并使用 Box::pin 解决异步递归问题
            info!("Resending request with tool results...");
            let recursive_call = self.send_chat_request(
                    model_name,
                    messages,
                    conversation_id,
                    temperature,
                    max_tokens,
                );
            return Box::pin(recursive_call).await;
        }
        
        // 如果没有工具调用，保存助手响应并返回
        let answer = response.first_text().unwrap_or_default();
        let assistant_msg = AiMessage {
            id: Uuid::new_v4().to_string(),
            conversation_id: conversation_id.to_string(),
            role: "assistant".to_string(),
            content: answer.to_string(),
            metadata: None,
            token_count: Some(answer.len() as i32),
            cost: Some(0.0),
            tool_calls: None,
            attachments: None,
            timestamp: Utc::now(),
        };
        
        self.db.create_ai_message(&assistant_msg).await?;
        
        Ok(answer.to_string())
    }

    // 执行工具调用
    pub async fn execute_tool_call(&self, conversation_id: &str, tool_name: &str, args: Value) -> Result<Value> {
        info!("Executing tool call '{}' for conversation {}", tool_name, conversation_id);
        
        // 这里可以根据tool_name执行不同的逻辑
        // 例如，如果tool_name是 "search_web"，则调用网络搜索API
        
        // 示例：使用MCP服务执行工具
        if let Some(mcp_service) = &self.mcp_service {
            match mcp_service.execute_tool(tool_name, args.clone()).await {
                Ok(result) => {
                    // 保存工具调用结果
                    self.save_tool_call(conversation_id, tool_name, &args, Ok(&result)).await?;
                    
                    // 将结果作为新的消息发送回AI
                    // self.send_message_with_tool_response(conversation_id, tool_name, result.clone()).await?;

                    Ok(result)
                }
                Err(e) => {
                    error!("MCP tool execution failed: {}", e);
                    self.save_tool_call(conversation_id, tool_name, &args, Err(&e)).await?;
                    Err(e)
                }
            }
        } else {
            let err_msg = "MCP service not available for tool execution".to_string();
            error!("{}", err_msg);
            Err(anyhow::anyhow!(err_msg))
        }
    }
    
    // 辅助函数：发送流式消息到前端
    async fn emit_stream_message(&self, conversation_id: &str, message_id: &str, content: &str, is_complete: bool, tool_calls: Option<Vec<AiToolCall>>) {
        if let Some(app_handle) = &self.app_handle {
            let message = StreamMessage {
                conversation_id: conversation_id.to_string(),
                message_id: message_id.to_string(),
                content: content.to_string(),
                is_complete,
                token_count: None, // TODO: token计数
                total_tokens: None,
                tool_calls,
            };
            if let Err(e) = app_handle.emit("ai_stream_message", &message) {
                error!("Failed to emit stream message: {}", e);
            }
        }
    }

    // 辅助函数：保存工具调用记录到数据库
    async fn save_tool_call(&self, conversation_id: &str, tool_name: &str, args: &Value, result: Result<&Value, &anyhow::Error>) -> Result<()> {
        // Find the last assistant message in the conversation to append the tool call result
        let mut messages = self.get_conversation_history(conversation_id).await?;
        if let Some(last_message) = messages.iter_mut().rev().find(|m| m.role == "assistant") {
            let mut tool_calls: Vec<AiToolCall> = last_message.tool_calls.as_ref()
                .and_then(|json_str| serde_json::from_str(json_str).ok())
                .unwrap_or_default();

            if let Some(tool_call) = tool_calls.iter_mut().find(|tc| tc.name == tool_name && tc.arguments == *args) {
                match result {
                    Ok(res_val) => tool_call.result = Some(res_val.clone()),
                    Err(err) => tool_call.error = Some(err.to_string()),
                }
            }
            
            last_message.tool_calls = Some(serde_json::to_string(&tool_calls)?);
            // Here we need to update the message in the database.
            // This assumes we have a method to update a message, which might need to be added.
            // For now, let's assume we update the whole conversation, which is less efficient.
            let mut conversation = self.db.get_ai_conversation(conversation_id).await?.ok_or_else(|| anyhow::anyhow!("Conversation not found"))?;
            // This is tricky. We need a way to update just the messages.
            // Let's just log it for now.
             info!("Would update message with tool call result: {:?}", last_message);

        }
        Ok(())
    }

    // 创建新对话
    pub async fn create_conversation(&self, title: Option<String>) -> Result<String> {
        let mut conversation = AiConversation::new(
            self.config.model.clone(),
            self.config.provider.clone() // TODO: This should be the service name, not provider
        );
        conversation.id = Uuid::new_v4().to_string();
        conversation.title = title;

        self.db.create_ai_conversation(&conversation).await?;
        Ok(conversation.id)
    }

    // 获取对话历史
    pub async fn get_conversation_history(&self, conversation_id: &str) -> Result<Vec<AiMessage>> {
        self.db.get_ai_messages_by_conversation(conversation_id).await
    }

    // 保存消息到数据库
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
    async fn save_message_with_tool_calls(&self, conversation_id: &str, role: &str, content: &str, tool_calls: Vec<AiToolCall>) -> Result<()> {
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

    // 删除对话
    pub async fn delete_conversation(&self, conversation_id: &str) -> Result<()> {
        self.db.delete_ai_conversation(conversation_id).await
    }

    // 获取所有对话
    pub async fn list_conversations(&self) -> Result<Vec<AiConversation>> {
        self.db.get_ai_conversations().await
    }

    // 更新对话标题
    pub async fn update_conversation_title(&self, conversation_id: &str, title: &str) -> Result<()> {
        self.db.update_ai_conversation_title(conversation_id, title).await
    }

    // 归档对话
    pub async fn archive_conversation(&self, conversation_id: &str) -> Result<()> {
        self.db.archive_ai_conversation(conversation_id).await
    }
}

