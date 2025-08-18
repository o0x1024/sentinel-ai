use crate::ai_adapter::RawMessage;
use crate::commands::ai::{ModelConfig, ModelInfo};
use crate::models::database::{AiConversation, AiMessage};
use crate::services::database::Database;
use crate::services::mcp::McpService;
use anyhow::Result;
use chrono::Utc;
use crate::ai_adapter::types::ToolCall;
use crate::ai_adapter::raw_message::{RawChatRequest, RawChatOptions};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

// 调度策略相关结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    pub intent_analysis_model: String,
    pub planner_model: String,
    pub replanner_model: String,
    pub executor_model: String,
    pub evaluator_model: String,
    pub default_strategy: String,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            intent_analysis_model: "deepseek-chat".to_string(),
            planner_model: "deepseek-chat".to_string(),
            replanner_model: "deepseek-chat".to_string(),
            executor_model: "deepseek-chat".to_string(),
            evaluator_model: "deepseek-chat".to_string(),
            default_strategy: "adaptive".to_string(),
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
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamError {
    pub conversation_id: String,
    pub error: String,
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
struct ProviderConfig {
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

    /// 从数据库获取AI提供商配置
    pub async fn get_provider_config(&self, provider: &str) -> Result<Option<AiConfig>> {
        // 首先尝试从providers_config中获取
        if let Ok(Some(providers_json)) = self.db.get_config("ai", "providers_config").await {
            if let Ok(providers) = serde_json::from_str::<HashMap<String, serde_json::Value>>(&providers_json) {
                // 查找匹配的提供商（不区分大小写）
                for (key, provider_data) in providers {
                    if let Some(provider_obj) = provider_data.as_object() {
                        if let Some(provider_name) = provider_obj.get("provider").and_then(|v| v.as_str()) {
                            if provider_name.to_lowercase() == provider.to_lowercase() || 
                               key.to_lowercase() == provider.to_lowercase() {
                                let api_key = provider_obj.get("api_key").and_then(|v| v.as_str()).map(|s| s.to_string());
                                let api_base = provider_obj.get("api_base").and_then(|v| v.as_str()).map(|s| s.to_string());
                                let default_model = provider_obj.get("default_model").and_then(|v| v.as_str()).unwrap_or("default").to_string();
                                let organization = provider_obj.get("organization").and_then(|v| v.as_str()).map(|s| s.to_string());
                                
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
                _ => None,
            };
            
            let default_model = match provider.to_lowercase().as_str() {
                "openai" => "gpt-4o",
                "anthropic" => "claude-3-5-sonnet-20241022",
                "deepseek" => "deepseek-chat",
                "google" => "gemini-pro",
                "ollama" => "llama2",
                _ => "default",
            }.to_string();
            
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
        use crate::ai_adapter::core::AiAdapterManager;
        use crate::ai_adapter::types::ProviderConfig;
        
        // 创建提供商配置
        let provider_config = ProviderConfig {
            name: config.provider.clone(),
            api_key: config.api_key.clone().unwrap_or_default(),
            api_base: config.api_base.clone(),
            api_version: None,
            timeout: None,
            max_retries: None,
            extra_headers: None,
        };
        
        // 向全局 AiAdapterManager 注册提供商（使用小写名称确保一致性）
        let provider_name = config.provider.to_lowercase();
        let adapter_manager = AiAdapterManager::global();
        
        // 使用ProviderFactory创建提供商实例
        use crate::ai_adapter::providers::ProviderFactory;
        match ProviderFactory::create(provider_config) {
            Ok(provider) => {
                if let Err(e) = adapter_manager.register_provider(provider) {
                    tracing::warn!("Failed to register provider {}: {}", provider_name, e);
                } else {
                    tracing::info!("Successfully registered provider: {}", provider_name);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to create provider {}: {}", provider_name, e);
            }
        }
        
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
        tracing::info!("Initializing default AI services...");

        // 从数据库加载并解析providers_config
        if let Ok(Some(config_str)) = self.db.get_config("ai", "providers_config").await {
            match serde_json::from_str::<HashMap<String, ProviderConfig>>(&config_str) {
                Ok(providers) => {
                    tracing::info!("Successfully parsed 'providers_config' from DB.");
                    for (_id, provider_config) in providers {
                        if !provider_config.enabled {
                            continue;
                        }

                        tracing::info!("Initializing enabled provider: {}", provider_config.name);

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
                            temperature: None,
                            max_tokens: None,
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
                tracing::error!(
                    "Failed to initialize services from environment variables: {}",
                    e
                );
            }
        }

        // 如果仍然没有任何服务，创建一个默认的别名服务
        if self.services.read().unwrap().is_empty() {
            tracing::warn!(
                "No AI services could be initialized. Creating a default alias service."
            );
            if let Err(e) = self.create_default_alias().await {
                tracing::error!("Failed to create default alias service: {}", e);
            }
        }

        tracing::info!(
            "Finished initializing AI services. Total services: {}",
            self.services.read().unwrap().len()
        );

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
                tracing::info!(
                    "Created default service alias, pointing to {}",
                    first_service_name
                );
            }
        }

        Ok(())
    }

    // 调度策略相关方法
    // 获取调度策略配置
    pub async fn get_scheduler_config(&self) -> anyhow::Result<SchedulerConfig> {
        let mut config = SchedulerConfig::default();
        
        // 从数据库加载调度策略配置
        if let Ok(Some(intent_model)) = self.db.get_config("scheduler", "intent_analysis_model").await {
            config.intent_analysis_model = intent_model;
        }
        
        if let Ok(Some(planner_model)) = self.db.get_config("scheduler", "planner_model").await {
            config.planner_model = planner_model;
        }
        
        if let Ok(Some(replanner_model)) = self.db.get_config("scheduler", "replanner_model").await {
            config.replanner_model = replanner_model;
        }
        
        if let Ok(Some(executor_model)) = self.db.get_config("scheduler", "executor_model").await {
            config.executor_model = executor_model;
        }
        
        if let Ok(Some(evaluator_model)) = self.db.get_config("scheduler", "evaluator_model").await {
            config.evaluator_model = evaluator_model;
        }
        
        if let Ok(Some(default_strategy)) = self.db.get_config("scheduler", "default_strategy").await {
            config.default_strategy = default_strategy;
        }
        
        Ok(config)
    }
    
    // 根据阶段获取对应的AI服务
    pub async fn get_service_for_stage(&self, stage: SchedulerStage) -> anyhow::Result<Option<AiService>> {
        let config = self.get_scheduler_config().await?;
        
        let model_id = match stage {
            SchedulerStage::IntentAnalysis => config.intent_analysis_model,
            SchedulerStage::Planning => config.planner_model,
            SchedulerStage::Replanning => config.replanner_model,
            SchedulerStage::Execution => config.executor_model,
            SchedulerStage::Evaluation => config.evaluator_model,
        };
        
        // 根据模型ID找到对应的服务
        let service = self.find_service_by_model(&model_id).await?;
        Ok(service)
    }
    
    // 根据模型ID查找对应的AI服务
    async fn find_service_by_model(&self, model_id: &str) -> anyhow::Result<Option<AiService>> {
        let services = self.services.read().unwrap();
        
        // 遍历所有服务，找到包含指定模型的服务
        for (_service_name, service) in services.iter() {
            let config = service.get_config();
            
            // 检查服务的默认模型是否匹配
            if config.model == model_id {
                return Ok(Some(service.clone()));
            }
            
            // 检查服务是否支持该模型（通过提供商推断）
            if self.is_model_supported_by_service(model_id, &config.provider) {
                // 创建一个使用指定模型的服务副本
                let mut new_config = config.clone();
                new_config.model = model_id.to_string();
                
                let new_service = AiService {
                    config: new_config,
                    db: self.db.clone(),
                    app_handle: self.app_handle.read().unwrap().clone(),
                    mcp_service: self.mcp_service.clone(),
                    max_retries: 3,
                };
                
                return Ok(Some(new_service));
            }
        }
        
        // 如果没有找到，返回默认服务
        Ok(self.get_service("default"))
    }
    
    // 检查模型是否被指定提供商支持
    fn is_model_supported_by_service(&self, model_id: &str, provider: &str) -> bool {
        match provider {
            "openai" => model_id.starts_with("gpt-") || model_id.starts_with("o1-"),
            "anthropic" => model_id.starts_with("claude-"),
            "gemini" => model_id.starts_with("gemini-"),
            "deepseek" => model_id.starts_with("deepseek-") || model_id == "deepseek-chat" || model_id == "deepseek-coder",
            "groq" => model_id.contains("llama") || model_id.contains("mixtral") || model_id.contains("gemma"),
            "cohere" => model_id.starts_with("command-"),
            "xai" => model_id.starts_with("grok-"),
            "ollama" => true, // Ollama可以支持多种本地模型
            _ => false,
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
            tracing::warn!("Could not find any chat models from database configuration.");
        }

        Ok(all_models)
    }

    pub async fn get_embedding_models(&self) -> Result<Vec<ModelInfo>> {
        Ok(vec![])
    }

    pub async fn get_default_model(&self, _model_type: &str) -> Result<Option<ModelInfo>> {
        Ok(None)
    }

    pub async fn set_default_model(
        &self,
        _model_type: &str,
        _provider: &str,
        _model_name: &str,
    ) -> Result<()> {
        Ok(())
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
    // 解析调度器配置的执行模型，失败则回退到当前配置
    async fn resolve_executor_model(&self) -> String {
        match self.db.get_config("scheduler", "executor_model").await {
            Ok(Some(model)) if !model.trim().is_empty() => model,
            _ => self.config.model.clone(),
        }
    }
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

    // 发送消息
    pub async fn send_message(
        &self,
        content: &str,
        conversation_id: Option<String>,
    ) -> Result<String> {
        let conversation_id = conversation_id.unwrap_or_else(|| Uuid::new_v4().to_string());
        
        // 获取MCP工具信息
        let mut system_prompt = "You are a helpful AI assistant.".to_string();
        if let Some(mcp_service) = &self.mcp_service {
            if let Ok(available_tools) = mcp_service.get_available_tools().await {
                if !available_tools.is_empty() {
                    system_prompt.push_str("\n\nAvailable tools:\n");
                    for tool in &available_tools {
                        system_prompt.push_str(&format!("- {}: {}\n", tool.name, tool.description));
                    }
                }
            }
        }
        
        // 创建或获取对话
        if self.db.get_ai_conversation(&conversation_id).await.is_err() {
            // 创建一个使用指定conversation_id的对话
            let mut conversation = AiConversation::new(
                self.config.model.clone(),
                self.config.provider.clone(),
            );
            conversation.id = conversation_id.clone();
            conversation.title = Some("New Conversation".to_string());
            self.db.create_ai_conversation(&conversation).await?;
        }
        
        // 获取历史消息
        let mut messages = match self.get_conversation_history(&conversation_id).await {
            Ok(msgs) => msgs,
            Err(e) => {
                warn!("获取对话历史失败: {}, 使用空消息列表", e);
                Vec::new()
            }
        };
        
        // 添加系统消息（如果是新对话）
        if messages.is_empty() {
            let system_msg = AiMessage {
                id: Uuid::new_v4().to_string(),
                conversation_id: conversation_id.clone(),
                role: "system".to_string(),
                content: system_prompt,
                metadata: None,
                token_count: None,
                cost: None,
                tool_calls: None,
                attachments: None,
                timestamp: Utc::now(),
            };
            if let Err(e) = self.db.create_ai_message(&system_msg).await {
                debug!("系统消息保存失败: {}, 继续执行但不保存到数据库", e);
            }
            messages.push(system_msg);
        }
        
        // 保存用户消息
        let user_msg = AiMessage {
            id: Uuid::new_v4().to_string(),
            conversation_id: conversation_id.clone(),
            role: "user".to_string(),
            content: content.to_string(),
            metadata: None,
            token_count: Some(content.len() as i32),
            cost: None,
            tool_calls: None,
            attachments: None,
            timestamp: Utc::now(),
        };
        match self.db.create_ai_message(&user_msg).await {
            Ok(_) => messages.push(user_msg),
            Err(e) => warn!("用户消息保存失败: {}, 继续执行但不保存到数据库", e),
        }
        
        // 发送聊天请求
        let model_name_owned = self.resolve_executor_model().await;
        let model_name = &model_name_owned;
        self.send_chat_request(
            model_name,
            messages,
            &conversation_id,
            self.config.temperature,
            self.config.max_tokens,
        ).await
    }

    // 流式发送消息
    pub async fn send_message_stream(
        &self,
        content: &str,
        conversation_id: Option<String>,
    ) -> Result<String> {
        // 简化实现，直接调用普通发送消息
        self.send_message(content, conversation_id).await
    }

    // 带提示的流式发送消息
    pub async fn send_message_stream_with_prompt(
        &self,
        content: &str,
        system_prompt: Option<String>,
        conversation_id: Option<String>,
    ) -> Result<String> {
        let conversation_id = conversation_id.unwrap_or_else(|| Uuid::new_v4().to_string());
        
        // 创建或获取对话
        if self.db.get_ai_conversation(&conversation_id).await.is_err() {
            // 创建一个使用指定conversation_id的对话
            let mut conversation = AiConversation::new(
                self.config.model.clone(),
                self.config.provider.clone(),
            );
            conversation.id = conversation_id.clone();
            conversation.title = Some("New Conversation".to_string());
            self.db.create_ai_conversation(&conversation).await?;
        }
        
        // 获取历史消息
        let mut messages = self.get_conversation_history(&conversation_id).await?;
        
        // 添加系统消息（如果提供了自定义提示）
        if let Some(prompt) = system_prompt {
            let system_msg = AiMessage {
                id: Uuid::new_v4().to_string(),
                conversation_id: conversation_id.clone(),
                role: "system".to_string(),
                content: prompt,
                metadata: None,
                token_count: None,
                cost: None,
                tool_calls: None,
                attachments: None,
                timestamp: Utc::now(),
            };
            self.db.create_ai_message(&system_msg).await?;
            messages.push(system_msg);
        }
        
        // 保存用户消息
        let user_msg = AiMessage {
            id: Uuid::new_v4().to_string(),
            conversation_id: conversation_id.clone(),
            role: "user".to_string(),
            content: content.to_string(),
            metadata: None,
            token_count: Some(content.len() as i32),
            cost: None,
            tool_calls: None,
            attachments: None,
            timestamp: Utc::now(),
        };
        self.db.create_ai_message(&user_msg).await?;
        messages.push(user_msg);
        
        // 发送聊天请求
        let model_name_owned = self.resolve_executor_model().await;
        let model_name = &model_name_owned;
        self.send_chat_request(
            model_name,
            messages,
            &conversation_id,
            self.config.temperature,
            self.config.max_tokens,
        ).await
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

        // 转换消息格式为新的类型系统
        let mut chat_messages = Vec::new();
        for msg in &messages {
            let role = match msg.role.as_str() {
                "system" => crate::ai_adapter::types::MessageRole::System,
                "user" => crate::ai_adapter::types::MessageRole::User,
                "assistant" => crate::ai_adapter::types::MessageRole::Assistant,
                "tool" => crate::ai_adapter::types::MessageRole::Tool,
                _ => crate::ai_adapter::types::MessageRole::User,
            };
            let message = crate::ai_adapter::types::Message {
                role,
                content: msg.content.clone(),
                name: None,
                tool_calls: if let Some(tc_json) = &msg.tool_calls {
                    if let Ok(tool_calls) = serde_json::from_str::<Vec<ToolCall>>(tc_json) {
                        Some(tool_calls)
                    } else {
                        None
                    }
                } else {
                    None
                },
                tool_call_id: if let Some(metadata_str) = &msg.metadata {
                    if let Ok(metadata) = serde_json::from_str::<Value>(metadata_str) {
                        metadata.get("tool_call_id").and_then(|v| v.as_str()).map(|s| s.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                },
            };
            chat_messages.push(message);
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
                    tools_vec.push(crate::ai_adapter::types::Tool {
                        r#type: "function".to_string(),
                        name: t.name,
                        description: t.description,
                        parameters: params_schema,
                    });
                }
            }
        }

        // 设置选项
        let mut options = crate::ai_adapter::types::ChatOptions::default();
        if let Some(temp) = temperature {
            options.temperature = Some(temp);
        }
        if let Some(tokens) = max_tokens {
            options.max_tokens = Some(tokens);
        }

        // 转换为原始消息格式
        let raw_messages: Vec<RawMessage> = chat_messages.into_iter().map(|msg| {
            let role_str = match msg.role {
                crate::ai_adapter::types::MessageRole::System => "system",
                crate::ai_adapter::types::MessageRole::User => "user",
                crate::ai_adapter::types::MessageRole::Assistant => "assistant",
                crate::ai_adapter::types::MessageRole::Tool => "tool",
            };
            let content_str = match msg.content {
                text => text,
            };
            RawMessage {
                role: role_str.to_string(),
                content: content_str,
                name: msg.name,
                tool_calls: msg.tool_calls.map(|tc| serde_json::to_value(tc).unwrap_or(serde_json::Value::Null)),
                tool_call_id: msg.tool_call_id,
            }
        }).collect();
        
        let tools_json = if tools_vec.is_empty() {
            None
        } else {
            Some(serde_json::to_value(&tools_vec).unwrap_or(serde_json::Value::Null))
        };
        
        let _raw_request = RawChatRequest {
            messages: raw_messages.clone(),
            tools: tools_json,
            tool_choice: None,
            model: Some(model_name.to_string()),
            temperature: options.temperature,
            max_tokens: options.max_tokens,
            top_p: options.top_p,
            frequency_penalty: options.frequency_penalty,
            presence_penalty: options.presence_penalty,
            stop: options.stop.clone(),
            stream: options.stream,
        };
        
        let _raw_options = RawChatOptions {
            temperature: options.temperature,
            max_tokens: options.max_tokens,
            top_p: options.top_p,
            frequency_penalty: options.frequency_penalty,
            presence_penalty: options.presence_penalty,
            stop: options.stop.clone(),
            stream: options.stream,
            tools: None,
            tool_choice: None,
            extra_headers: None,
            timeout: None,
        };
        
        // 转换为ChatRequest
        let chat_request = crate::ai_adapter::types::ChatRequest {
            model: model_name.to_string(),
            messages: raw_messages.iter().map(|msg| {
                let role = match msg.role.as_str() {
                    "system" => crate::ai_adapter::types::MessageRole::System,
                    "user" => crate::ai_adapter::types::MessageRole::User,
                    "assistant" => crate::ai_adapter::types::MessageRole::Assistant,
                    "tool" => crate::ai_adapter::types::MessageRole::Tool,
                    _ => crate::ai_adapter::types::MessageRole::User,
                };
                crate::ai_adapter::types::Message {
                    role,
                    content: msg.content.clone(),
                    name: msg.name.clone(),
                    tool_calls: None,
                    tool_call_id: msg.tool_call_id.clone(),
                }
            }).collect(),
            tools: if tools_vec.is_empty() { None } else { Some(tools_vec) },
            tool_choice: None,
            user: None,
            extra_params: None,
            options: Some(options),
        };
        
        // 发送请求
        let client = crate::ai_adapter::global_client();
        let response = client
            .chat(Some(&self.config.provider), chat_request)
            .await
            .map_err(|e| {
                anyhow::anyhow!("Web call failed for model '{}'. Cause: {}", model_name, e)
            })?;

        // 检查是否包含工具调用
        let tool_calls = response.message.tool_calls.clone().unwrap_or_default();
        if !tool_calls.is_empty() {
            info!("Executing tool calls: {:?}", tool_calls);

            // 1. 保存带有工具调用的助手消息
            let answer_text = match &response.message.content {
                text => text.clone(),
            };
            let assistant_msg = AiMessage {
                id: Uuid::new_v4().to_string(),
                conversation_id: conversation_id.to_string(),
                role: "assistant".to_string(),
                content: answer_text.to_string(),
                metadata: None,
                token_count: Some(answer_text.len() as i32),
                cost: Some(0.0),
                tool_calls: Some(
                    serde_json::to_string(&tool_calls).unwrap_or_else(|_| "{}".to_string()),
                ),
                attachments: None,
                timestamp: Utc::now(),
            };

            self.db.create_ai_message(&assistant_msg).await?;
            messages.push(assistant_msg);

            // 2. 执行每个工具调用
            let mut tool_messages = Vec::new();
            for tc in tool_calls {
                let tool_name = &tc.name;
                let args_val: serde_json::Value = serde_json::from_str(&tc.arguments).unwrap_or_default();

                let exec_res = if let Some(mcp) = &self.mcp_service {
                    mcp.execute_tool(tool_name, args_val.clone()).await
                } else {
                    Err(anyhow::anyhow!("MCP service unavailable"))
                };

                // 3. 创建工具响应消息
                let tool_msg = match &exec_res {
                    Ok(result) => {
                        let result_str =
                            serde_json::to_string(result).unwrap_or_else(|_| "{}".to_string());
                        AiMessage {
                            id: Uuid::new_v4().to_string(),
                            conversation_id: conversation_id.to_string(),
                            role: "tool".to_string(),
                            content: result_str.clone(),
                            metadata: Some(format!(
                                "{{\"tool_call_id\":\"{}\",\"tool_name\":\"{}\"}}",
                                tc.id, tool_name
                            )),
                            token_count: Some(result_str.len() as i32),
                            cost: None,
                            tool_calls: None,
                            attachments: None,
                            timestamp: Utc::now(),
                        }
                    }
                    Err(e) => {
                        let error_str = format!("{{\"error\":\"{}\"}}", e);
                        AiMessage {
                            id: Uuid::new_v4().to_string(),
                            conversation_id: conversation_id.to_string(),
                            role: "tool".to_string(),
                            content: error_str.clone(),
                            metadata: Some(format!(
                                "{{\"tool_call_id\":\"{}\",\"tool_name\":\"{}\"}}",
                                tc.id, tool_name
                            )),
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
        let answer = match &response.message.content {
            text => text.clone(),
        };
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

        match self.db.create_ai_message(&assistant_msg).await {
            Ok(_) => {}
            Err(e) => {
                tracing::error!("Failed to create assistant message: {}", e);
            }
        }

        Ok(answer.to_string())
    }


    // 分析查询 - 无状态分析，不保存到数据库
    pub async fn analyze_query(&self, query: &str){
        info!("Analyzing query: {}", query);
        
        // 使用结构化的系统提示来分析查询
        let _system_prompt = format!(r#"
You are a task analysis expert. Please analyze the user query and extract key features.

**Analysis Dimensions:**
1. Task type identification
2. Parallelization potential assessment
3. Complexity level judgment
4. Time sensitivity analysis
5. Resource requirement assessment

**User Query:** {}

**Output Format:**
```json
{{
  "task_type": "security_testing|data_analysis|research|business_process",
  "sub_category": "specific sub-type",
  "parallelization_potential": "high|medium|low",
  "complexity_level": "simple|medium|complex", 
  "time_sensitivity": "high|medium|low",
  "dependency_complexity": "simple|medium|complex",
  "estimated_steps": "number",
  "resource_requirements": "light|medium|heavy",
  "key_indicators": ["keyword1", "keyword2", ...],
  "target_domain": "target domain or IP"
}}
```

Please analyze and output structured results.
        "#, query);
        

    }

    // 发送分析请求 - 无状态，不保存到数据库
    #[allow(unused)]
    async fn send_analysis_request(&self, query: &str, system_prompt: &str) -> Result<String> {
        info!("🔍 发送分析请求到 {} 模型", self.config.model);
        info!("📝 查询内容: {}", query);
        debug!("🎯 系统提示长度: {} 字符", system_prompt.len());
        
        // 构建消息列表（不保存到数据库）
        let mut chat_messages = Vec::new();
        chat_messages.push(crate::ai_adapter::types::Message::system(system_prompt));
        chat_messages.push(crate::ai_adapter::types::Message::user(query));
        
        info!("📨 构建的消息数量: {}", chat_messages.len());
        
        // 构建原始参数
        let mut raw_params = std::collections::HashMap::new();
        if let Some(temp) = self.config.temperature {
            raw_params.insert("temperature".to_string(), serde_json::Value::from(temp as f64));
            debug!("🌡️ 设置温度参数: {}", temp);
        }
        if let Some(tokens) = self.config.max_tokens {
            raw_params.insert("max_tokens".to_string(), serde_json::Value::from(tokens));
            debug!("🔢 设置最大token数: {}", tokens);
        }
        
        // 构建请求（不使用工具）
        let chat_req = crate::ai_adapter::types::ChatRequest {
            model: self.config.model.clone(),
            messages: chat_messages,
            tools: None,
            tool_choice: None,
            user: None,
            extra_params: None,
            options: Some(crate::ai_adapter::types::ChatOptions {
                temperature: self.config.temperature,
                max_tokens: self.config.max_tokens,
                stream: Some(false),
                ..Default::default()
            }),
        };
        
        info!("🚀 开始发送请求到AI客户端...");
        
        // 发送请求
        let client = crate::ai_adapter::global_client();
        let response = client.chat(Some(&self.config.provider), chat_req).await
            .map_err(|e| {
                error!("❌ AI客户端请求失败: {}", e);
                anyhow::anyhow!("Chat request failed: {}", e)
            })?;
        
        info!("✅ 收到AI响应");
        
        // 提取响应内容
        let content = response.message.content;
        let content_str = match &content {
            text => text.clone(),
        };
        info!("📝 响应内容长度: {} 字符", content_str.len());
        debug!("📄 响应内容: {}", content_str);
        
        // 记录使用统计
        if let Some(usage) = &response.usage {
            info!("📊 Token使用统计: prompt={}, completion={}, total={}", 
                usage.prompt_tokens, usage.completion_tokens, usage.total_tokens);
        }
        
        Ok(content_str)
    }
    
    // 辅助函数：发送流式消息到前端
    #[allow(unused)]
    async fn emit_stream_message(
        &self,
        conversation_id: &str,
        message_id: &str,
        content: &str,
        is_complete: bool,
        tool_calls: Option<Vec<AiToolCall>>,
    ) {
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


}
