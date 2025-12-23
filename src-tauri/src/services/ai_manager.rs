//! AI 服务管理器
//!
//! 管理多个 AI 提供商配置，从数据库加载配置并创建 AiService 实例。

use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::AppHandle;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::models::database::{AiConversation, AiMessage};
use crate::services::database::Database;
use crate::utils::ordered_message::ChunkType;
use sentinel_llm::{AiConfig, AiService, SchedulerConfig, SchedulerStage};

/// AI 服务管理器
#[derive(Debug, Clone)]
pub struct AiServiceManager {
    services: Arc<std::sync::RwLock<HashMap<String, AiServiceWrapper>>>,
    db: Arc<dyn Database + Send + Sync>,
    app_handle: Arc<std::sync::RwLock<Option<AppHandle>>>,
}

/// 包装 AiService 并添加应用特定功能
#[derive(Clone)]
pub struct AiServiceWrapper {
    pub service: AiService,
    pub config: AiConfig,
    pub db: Arc<dyn Database + Send + Sync>,
    pub app_handle: Option<AppHandle>,
}

impl std::fmt::Debug for AiServiceWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AiServiceWrapper")
            .field("config", &self.config)
            .field("app_handle", &self.app_handle.is_some())
            .finish()
    }
}

impl AiServiceWrapper {
    pub fn new(
        config: AiConfig,
        db: Arc<dyn Database + Send + Sync>,
        app_handle: Option<AppHandle>,
    ) -> Self {
        Self {
            service: AiService::new(config.clone()),
            config,
            db,
            app_handle,
        }
    }

    pub fn get_config(&self) -> &AiConfig {
        &self.config
    }

    pub fn set_app_handle(&mut self, app_handle: AppHandle) {
        self.app_handle = Some(app_handle);
    }

    // 对话管理方法
    pub async fn create_conversation(&self, title: Option<String>) -> Result<String> {
        let mut conversation = AiConversation::new(
            self.config.model.clone(),
            self.config.provider.clone(),
        );
        conversation.id = Uuid::new_v4().to_string();
        conversation.title = title;
        self.db.create_ai_conversation(&conversation).await?;
        Ok(conversation.id)
    }

    pub async fn get_conversation_history(&self, conversation_id: &str) -> Result<Vec<AiMessage>> {
        self.db.get_ai_messages_by_conversation(conversation_id).await
    }

    pub async fn delete_conversation(&self, conversation_id: &str) -> Result<()> {
        self.db.delete_ai_conversation(conversation_id).await
    }

    pub async fn list_conversations(&self) -> Result<Vec<AiConversation>> {
        self.db.get_ai_conversations().await
    }

    pub async fn update_conversation_title(&self, conversation_id: &str, title: &str) -> Result<()> {
        self.db.update_ai_conversation_title(conversation_id, title).await
    }

    pub async fn archive_conversation(&self, _conversation_id: &str) -> Result<()> {
        warn!("archive_ai_conversation feature not fully implemented");
        Ok(())
    }

    /// 发送消息块到前端
    pub fn emit_message_chunk(
        &self,
        execution_id: &str,
        message_id: &str,
        conversation_id: Option<&str>,
        chunk_type: Option<ChunkType>,
        content: &str,
        is_final: bool,
        stage: Option<&str>,
        architecture: Option<crate::utils::ordered_message::ArchitectureType>,
    ) {
        if let Some(app_handle) = &self.app_handle {
            crate::utils::ordered_message::emit_message_chunk_with_arch(
                app_handle,
                execution_id,
                message_id,
                conversation_id,
                chunk_type.unwrap_or(ChunkType::Content),
                content,
                is_final,
                stage,
                None,
                architecture,
                None,
            );
        }
    }

}

#[derive(Debug, Deserialize)]
pub struct ProviderConfig {
    #[allow(unused)]
    id: String,
    provider: String,
    name: String,
    #[serde(default)]
    rig_provider: Option<String>,
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
        }
    }

    pub fn get_db_arc(&self) -> Arc<dyn Database + Send + Sync> {
        self.db.clone()
    }

    /// 从数据库获取 AI 提供商配置
    pub async fn get_provider_config(&self, provider: &str) -> Result<Option<AiConfig>> {
        if let Ok(Some(providers_json)) = self.db.get_config("ai", "providers_config").await {
            if let Ok(providers) =
                serde_json::from_str::<HashMap<String, serde_json::Value>>(&providers_json)
            {
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
                                let rig_provider = provider_obj
                                    .get("rig_provider")
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
                                    rig_provider,
                                }));
                            }
                        }
                    }
                }
            }
        }

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

            // 根据 provider 推断 rig_provider
            let rig_provider = match provider.to_lowercase().as_str() {
                "openai" => Some("openai".to_string()),
                "anthropic" => Some("anthropic".to_string()),
                "deepseek" => Some("deepseek".to_string()),
                "google" | "gemini" => Some("gemini".to_string()),
                "ollama" => Some("ollama".to_string()),
                "moonshot" => Some("moonshot".to_string()),
                "modelscope" => Some("openai".to_string()), // OpenAI 兼容
                "openrouter" => Some("openrouter".to_string()),
                "groq" => Some("groq".to_string()),
                "xai" => Some("xai".to_string()),
                "cohere" => Some("cohere".to_string()),
                "perplexity" => Some("perplexity".to_string()),
                "togetherai" => Some("togetherai".to_string()),
                "hyperbolic" => Some("hyperbolic".to_string()),
                _ => Some("openai".to_string()), // 默认 OpenAI 兼容
            };

            return Ok(Some(AiConfig {
                provider: provider.to_string(),
                model: default_model,
                api_key,
                api_base,
                organization: None,
                temperature: Some(0.7),
                max_tokens: Some(4096),
                rig_provider,
            }));
        }

        Ok(None)
    }

    pub fn set_app_handle(&self, app_handle: AppHandle) {
        let mut handle_guard = self.app_handle.write().unwrap();
        *handle_guard = Some(app_handle.clone());
        let mut services = self.services.write().unwrap();
        for service in services.values_mut() {
            service.set_app_handle(app_handle.clone());
        }
    }

    pub async fn add_service(&self, name: String, config: AiConfig) -> Result<()> {
        let wrapper = AiServiceWrapper::new(
            config,
            self.db.clone(),
            self.app_handle.read().unwrap().clone(),
        );
        let mut services = self.services.write().unwrap();
        services.insert(name, wrapper);
        Ok(())
    }

    pub fn get_service(&self, name: &str) -> Option<AiServiceWrapper> {
        let services = self.services.read().unwrap();
        services.get(name).cloned()
    }

    pub fn list_services(&self) -> Vec<String> {
        let services = self.services.read().unwrap();
        services.keys().cloned().collect()
    }

    pub fn remove_service(&self, name: &str) -> bool {
        let mut services = self.services.write().unwrap();
        services.remove(name).is_some()
    }

    pub async fn reload_services(&self) -> anyhow::Result<()> {
        info!("Reloading AI services...");
        {
            let mut services = self.services.write().unwrap();
            services.clear();
        }
        self.init_default_services().await
    }

    pub async fn init_default_services(&self) -> anyhow::Result<()> {
        debug!("Initializing default AI services...");

        if let Ok(Some(config_str)) = self.db.get_config("ai", "providers_config").await {
            match serde_json::from_str::<HashMap<String, ProviderConfig>>(&config_str) {
                Ok(providers) => {
                    debug!("Successfully parsed 'providers_config' from DB.");
                    for (_id, provider_config) in providers {
                        if !provider_config.enabled {
                            continue;
                        }

                        debug!("Initializing enabled provider: {}", provider_config.name);

                        let api_key = provider_config.api_key.as_deref().map(String::from);
                        std::env::set_var(
                            format!("{}_API_KEY", provider_config.name.to_uppercase()),
                            api_key.as_deref().unwrap_or(""),
                        );

                        let default_model = provider_config.default_model.clone();
                        let api_base = provider_config
                            .api_base
                            .filter(|s| !s.is_empty());
                        let organization = provider_config
                            .organization
                            .filter(|s| !s.is_empty());

                        let rig_provider = provider_config
                            .rig_provider
                            .filter(|s| !s.is_empty())
                            .unwrap_or_else(|| provider_config.provider.clone());

                        debug!(
                            "Provider {} using rig_provider: {}",
                            provider_config.name, rig_provider
                        );

                        let config = AiConfig {
                            provider: rig_provider.clone(),
                            model: default_model,
                            api_key,
                            api_base,
                            organization,
                            temperature: Some(0.7),
                            max_tokens: Some(4096),
                            rig_provider: Some(rig_provider),
                        };

                        if let Err(e) = self.add_service(provider_config.name.clone(), config).await
                        {
                            error!(
                                "Failed to add service for provider {}: {}",
                                provider_config.name, e
                            );
                        }
                    }
                }
                Err(e) => {
                    error!(
                        "Failed to parse 'providers_config': {}. Content: {}",
                        e, config_str
                    );
                }
            }
        } else {
            info!("'providers_config' not found in database.");
        }

        // Try new config key first, fallback to legacy key and migrate
        let default_llm_provider = match self.db.get_config("ai", "default_llm_provider").await {
            Ok(Some(v)) => Some(v),
            _ => {
                // Migrate from legacy key if exists
                if let Ok(Some(legacy)) = self.db.get_config("ai", "default_provider").await {
                    info!("Migrating default_provider to default_llm_provider: {}", legacy);
                    let _ = self.db.set_config("ai", "default_llm_provider", &legacy, 
                        Some("Global default LLM provider")).await;
                    Some(legacy)
                } else {
                    None
                }
            }
        };
        
        if let Some(provider) = default_llm_provider {
            let provider_key = provider.to_lowercase();
            if self.get_service(&provider_key).is_some() {
                if let Err(e) = self.set_default_alias_to(&provider_key).await {
                    warn!("Failed to set default alias to '{}': {}", provider_key, e);
                }
            }
        }

        if !self.services.read().unwrap().contains_key("default") {
            if self.services.read().unwrap().is_empty() {
                warn!("No AI services configured. Creating minimal default service.");
                if let Err(e) = self.create_minimal_default_service().await {
                    error!("Failed to create minimal default service: {}", e);
                }
            } else if let Err(e) = self.create_default_alias().await {
                error!("Failed to create default alias: {}", e);
            }
        }

        debug!(
            "Finished initializing AI services. Total: {}",
            self.services.read().unwrap().len()
        );

        Ok(())
    }

    async fn create_default_alias(&self) -> anyhow::Result<()> {
        let services = self.list_services();
        if services.contains(&"default".to_string()) {
            return Ok(());
        }

        let preferred = vec![
            "deepseek", "openai", "anthropic", "gemini", "groq", "ollama", "moonshot",
            "openrouter", "modelscope",
        ];

        for provider in preferred {
            if services.contains(&provider.to_string()) {
                if let Some(wrapper) = self.get_service(provider) {
                    let config = wrapper.get_config().clone();
                    self.add_service("default".to_string(), config).await?;
                    debug!("Created default alias pointing to {}", provider);
                    return Ok(());
                }
            }
        }

        if let Some(first) = services.first() {
            if let Some(wrapper) = self.get_service(first) {
                let config = wrapper.get_config().clone();
                self.add_service("default".to_string(), config).await?;
                info!("Created default alias pointing to {}", first);
            }
        }

        Ok(())
    }

    pub async fn set_default_alias_to(&self, provider: &str) -> anyhow::Result<()> {
        let provider_lc = provider.to_lowercase();
        let service = {
            let services = self.services.read().unwrap();
            if let Some((_name, svc)) = services
                .iter()
                .find(|(_n, svc)| svc.get_config().provider.to_lowercase() == provider_lc)
            {
                Some(svc.clone())
            } else {
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

        let config = service.get_config().clone();
        {
            let mut services = self.services.write().unwrap();
            services.remove("default");
        }
        self.add_service("default".to_string(), config).await?;
        info!("Default service alias now points to '{}'", provider_lc);
        Ok(())
    }

    async fn create_minimal_default_service(&self) -> anyhow::Result<()> {
        warn!("Creating minimal default service - no AI providers configured!");
        let config = AiConfig {
            provider: "unconfigured".to_string(),
            model: "no-model-configured".to_string(),
            api_key: None,
            api_base: None,
            organization: None,
            temperature: Some(0.7),
            max_tokens: Some(1000),
            rig_provider: None,
        };
        self.add_service("default".to_string(), config).await?;
        Ok(())
    }

    pub async fn get_scheduler_config(&self) -> anyhow::Result<SchedulerConfig> {
        let mut config = SchedulerConfig::default();

        if let Ok(Some(v)) = self.db.get_config("scheduler", "intent_analysis_model").await {
            config.intent_analysis_model = v;
        }
        if let Ok(Some(v)) = self.db.get_config("scheduler", "intent_analysis_provider").await {
            config.intent_analysis_provider = v;
        }
        if let Ok(Some(v)) = self.db.get_config("scheduler", "planner_model").await {
            config.planner_model = v;
        }
        if let Ok(Some(v)) = self.db.get_config("scheduler", "planner_provider").await {
            config.planner_provider = v;
        }
        if let Ok(Some(v)) = self.db.get_config("scheduler", "replanner_model").await {
            config.replanner_model = v;
        }
        if let Ok(Some(v)) = self.db.get_config("scheduler", "replanner_provider").await {
            config.replanner_provider = v;
        }
        if let Ok(Some(v)) = self.db.get_config("scheduler", "executor_model").await {
            config.executor_model = v;
        }
        if let Ok(Some(v)) = self.db.get_config("scheduler", "executor_provider").await {
            config.executor_provider = v;
        }
        if let Ok(Some(v)) = self.db.get_config("scheduler", "evaluator_model").await {
            config.evaluator_model = v;
        }
        if let Ok(Some(v)) = self.db.get_config("scheduler", "evaluator_provider").await {
            config.evaluator_provider = v;
        }
        if let Ok(Some(v)) = self.db.get_config("scheduler", "default_strategy").await {
            config.default_strategy = v;
        }
        if let Ok(Some(v)) = self.db.get_config("scheduler", "enabled").await {
            config.enabled = v.parse().unwrap_or(true);
        }
        if let Ok(Some(v)) = self.db.get_config("scheduler", "max_retries").await {
            config.max_retries = v.parse().unwrap_or(3);
        }
        if let Ok(Some(v)) = self.db.get_config("scheduler", "timeout_seconds").await {
            config.timeout_seconds = v.parse().unwrap_or(120);
        }
        if let Ok(Some(v)) = self.db.get_config("scheduler", "scenarios").await {
            config.scenarios = serde_json::from_str(&v)
                .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
        }

        Ok(config)
    }

    pub async fn get_service_for_stage(
        &self,
        stage: SchedulerStage,
    ) -> anyhow::Result<Option<AiServiceWrapper>> {
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

        if provider_name.trim().is_empty() || model_id.trim().is_empty() {
            return Ok(None);
        }

        let provider_cfg = match self.get_provider_config(&provider_name).await? {
            Some(cfg) => cfg,
            None => return Ok(None),
        };

        let mut dynamic_cfg = provider_cfg;
        dynamic_cfg.model = model_id;

        let app_handle = self.app_handle.read().unwrap().clone();
        let wrapper = AiServiceWrapper::new(
            dynamic_cfg,
            self.db.clone(),
            app_handle,
        );
        Ok(Some(wrapper))
    }

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

        if let Some(mut config) = self.get_provider_config(provider_name).await? {
            config.model = model_id.clone();
            Ok(Some(config))
        } else {
            Ok(None)
        }
    }
}

// 模型相关方法
impl AiServiceManager {
    pub async fn get_chat_models(&self) -> Result<Vec<ModelInfo>> {
        let mut all_models = Vec::new();

        if let Ok(Some(providers_config_str)) = self.db.get_config("ai", "providers_config").await {
            if let Ok(providers) =
                serde_json::from_str::<HashMap<String, serde_json::Value>>(&providers_config_str)
            {
                for (_provider_key, provider_data) in providers {
                    if let Some(provider_obj) = provider_data.as_object() {
                        let enabled = provider_obj
                            .get("enabled")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);

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

                        if let Some(models_arr) =
                            provider_obj.get("models").and_then(|v| v.as_array())
                        {
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
                        } else if let Some(default_model) =
                            provider_obj.get("default_model").and_then(|v| v.as_str())
                        {
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
        }

        if all_models.is_empty() {
            all_models = vec![
                ModelInfo {
                    provider: "openai".to_string(),
                    name: "gpt-4o".to_string(),
                    is_chat: true,
                    is_embedding: false,
                },
                ModelInfo {
                    provider: "anthropic".to_string(),
                    name: "claude-3-5-sonnet-20241022".to_string(),
                    is_chat: true,
                    is_embedding: false,
                },
            ];
        }

        Ok(all_models)
    }

    pub async fn get_embedding_models(&self) -> Result<Vec<ModelInfo>> {
        Ok(vec![])
    }

    pub async fn get_default_llm_model(&self) -> Result<Option<(String, String)>> {
        if let Ok(Some(model_str)) = self.db.get_config("ai", "default_llm_model").await {
            if let Some((provider, model_name)) = model_str.split_once('/') {
                return Ok(Some((provider.to_string(), model_name.to_string())));
            }
        }
        Ok(None)
    }

    pub async fn set_default_llm_model(&self, provider: &str, model_name: &str) -> Result<()> {
        let model_value = format!("{}/{}", provider, model_name);
        self.db
            .set_config(
                "ai",
                "default_llm_model",
                &model_value,
                Some("Default LLM model"),
            )
            .await?;
        info!("Set default chat model to: {}", model_value);
        Ok(())
    }

    pub async fn set_default_model(
        &self,
        model_type: &str,
        provider: &str,
        model_name: &str,
    ) -> Result<()> {
        let config_key = format!("default_{}_model", model_type);
        let model_value = format!("{}/{}", provider, model_name);
        self.db
            .set_config("ai", &config_key, &model_value, Some(&format!("Default {} model", model_type)))
            .await?;
        info!("Set default {} model to: {}", model_type, model_value);
        Ok(())
    }

    pub async fn get_default_model(&self, model_type: &str) -> Result<Option<ModelInfo>> {
        let config_key = format!("default_{}_model", model_type);
        if let Ok(Some(model_str)) = self.db.get_config("ai", &config_key).await {
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
}

/// 模型信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelInfo {
    pub provider: String,
    pub name: String,
    pub is_chat: bool,
    pub is_embedding: bool,
}

// 为兼容性提供类型别名
pub type LegacyAiService = AiServiceWrapper;

