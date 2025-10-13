use crate::ai_adapter::core::AiAdapterManager;
use crate::commands::ai::{ModelConfig, ModelInfo};
// Removed: use crate::engines::types::{StreamMessageType, UnifiedStreamMessage};
use crate::ai_adapter::types::ToolCall;
use crate::models::database::{AiConversation, AiMessage};
use crate::services::database::Database;
use crate::services::mcp::McpService;
use crate::utils::ordered_message::ChunkType;
use anyhow::Result;
use chrono::Utc;

use crate::ai_adapter::types::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use std::sync::Arc;
use tauri::AppHandle;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

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

        // 向全局 AI 客户端注册提供商（使用小写名称确保一致性）
        let provider_name = config.provider.to_lowercase();

        // 使用ProviderFactory创建提供商实例
        use crate::ai_adapter::providers::ProviderFactory;
        match ProviderFactory::create(provider_config) {
            Ok(provider) => {
                // 注册到全局AI适配器管理器
                let adapter_manager = AiAdapterManager::global();
                if let Err(e) = adapter_manager.register_provider(provider.clone()) {
                    tracing::warn!(
                        "Failed to register provider {} to AiAdapterManager: {}",
                        provider_name,
                        e
                    );
                } else {
                    tracing::debug!(
                        "Successfully registered provider {} to AiAdapterManager: {}",
                        provider_name,
                        provider.name()
                    );
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
        tracing::debug!("Initializing default AI services...");

        // 从数据库加载并解析providers_config
        if let Ok(Some(config_str)) = self.db.get_config("ai", "providers_config").await {
            match serde_json::from_str::<HashMap<String, ProviderConfig>>(&config_str) {
                Ok(providers) => {
                    tracing::debug!("Successfully parsed 'providers_config' from DB.");
                    for (_id, provider_config) in providers {
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
                // 同步设置到全局AI适配器管理器
                if let Ok(adapter_manager) =
                    crate::ai_adapter::core::AiAdapterManager::global().get_client()
                {
                    if let Ok(mut client) = adapter_manager.write() {
                        if let Err(e) = client.set_default_provider(&provider_key) {
                            tracing::warn!(
                                "Failed to set global default provider to '{}': {}",
                                provider_key,
                                e
                            );
                        } else {
                            tracing::info!("Global default provider set to '{}'", provider_key);
                        }
                    }
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

    /// 根据模型配置阶段获取对应的AI模型服务
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

        // 根据提供商和模型ID找到对应的服务
        let service = self
            .find_service_by_provider_and_model(&provider_name, &model_id)
            .await?;
        Ok(service)
    }

    // 根据提供商和模型ID查找对应的AI服务
    pub async fn find_service_by_provider_and_model(
        &self,
        provider_name: &str,
        model_id: &str,
    ) -> anyhow::Result<Option<AiService>> {
        if model_id.is_empty() {
            return Ok(None);
        }

        // 如果指定了提供商，优先根据提供商查找
        if !provider_name.is_empty() {
            // 先收集匹配的服务，然后释放锁
            let matching_services = {
                let services = self.services.read().unwrap();
                services
                    .iter()
                    .filter_map(|(_service_name, service)| {
                        let config = service.get_config();
                        if config.provider.to_lowercase() == provider_name.to_lowercase() {
                            Some((service.clone(), config.provider.clone()))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            };

            // 现在可以安全地进行async操作
            for (service, provider) in matching_services {
                if self
                    .is_model_supported_by_service(model_id, &provider)
                    .await
                {
                    tracing::info!(
                        "Found service for provider '{}' and model '{}': {}",
                        provider_name,
                        model_id,
                        provider
                    );
                    return Ok(Some(service));
                }
            }

            tracing::warn!(
                "No service found for provider '{}' with model '{}'",
                provider_name,
                model_id
            );
        }

        // 如果没有指定提供商或没找到匹配的服务，回退到按模型查找
        self.find_service_by_model(model_id).await
    }

    // 根据模型ID查找对应的AI服务
    pub async fn find_service_by_model(&self, model_id: &str) -> anyhow::Result<Option<AiService>> {
        // 首先检查是否有服务直接支持该模型
        let direct_match = {
            let services = self.services.read().unwrap();

            for (_service_name, service) in services.iter() {
                let config = service.get_config();

                // 检查服务的默认模型是否匹配
                if config.model == model_id {
                    return Ok(Some(service.clone()));
                }
            }
            None::<AiService>
        };

        if direct_match.is_some() {
            return Ok(direct_match);
        }

        // 检查服务是否支持该模型（通过提供商推断）
        let supported_service = {
            let services = self.services.read().unwrap();
            let mut result: Option<AiService> = None;

            for (_service_name, service) in services.iter() {
                let config = service.get_config();

                // 需要在这里检查支持，但不能使用async方法
                // 我们只检查硬编码规则
                let hardcoded_support = match config.provider.to_lowercase().as_str() {
                    "openai" => model_id.starts_with("gpt-") || model_id.starts_with("o1-"),
                    "anthropic" => model_id.starts_with("claude-"),
                    "deepseek" => model_id.starts_with("deepseek-"),
                    "groq" => {
                        model_id.starts_with("llama")
                            || model_id.starts_with("mixtral")
                            || model_id.starts_with("gemma")
                    }
                    "ollama" => true, // Ollama支持各种模型
                    "gemini" => model_id.starts_with("gemini-"),
                    "zhipu" => model_id.starts_with("glm-"),
                    "cohere" => model_id.starts_with("command-") || model_id.starts_with("c4ai-"),
                    "xai" => model_id.starts_with("grok-"),
                    "moonshot" => {
                        model_id.starts_with("moonshot-") || model_id.starts_with("kimi-")
                    }
                    "modelscope" => {
                        model_id.starts_with("qwen")
                            || model_id.starts_with("baichuan")
                            || model_id.starts_with("chatglm")
                            || model_id.starts_with("internlm")
                            || model_id.starts_with("yi-")
                            || model_id.starts_with("deepseek-")
                    }
                    _ => false,
                };

                if hardcoded_support {
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

                    info!("为模型 {} 找到匹配的提供商 {}", model_id, config.provider);
                    result = Some(new_service);
                    break;
                }
            }
            result
        };

        if supported_service.is_some() {
            return Ok(supported_service);
        }

        // 检查数据库中的providers_config是否支持该模型
        if self.is_model_supported_by_any_provider(model_id).await {
            // 如果数据库支持，尝试推断提供商
            if let Some(provider_name) = self.infer_provider_from_model(model_id).await {
                // 查找该提供商的服务
                let services = self.services.read().unwrap();
                for (_service_name, service) in services.iter() {
                    let config = service.get_config();
                    if config.provider.to_lowercase() == provider_name.to_lowercase() {
                        // 创建使用指定模型的服务副本
                        let mut new_config = config.clone();
                        new_config.model = model_id.to_string();

                        let new_service = AiService {
                            config: new_config,
                            db: self.db.clone(),
                            app_handle: self.app_handle.read().unwrap().clone(),
                            mcp_service: self.mcp_service.clone(),
                            max_retries: 3,
                        };

                        info!(
                            "通过数据库配置为模型 {} 找到提供商 {}",
                            model_id, provider_name
                        );
                        return Ok(Some(new_service));
                    }
                }
            }
        }

        // 如果找不到支持该模型的服务，尝试根据模型名推断提供商
        let inferred_provider = self.infer_provider_from_model(model_id).await;
        if let Some(provider_name) = inferred_provider {
            // 查找该提供商的服务
            let services = self.services.read().unwrap();
            for (_service_name, service) in services.iter() {
                let config = service.get_config();
                if config.provider.to_lowercase() == provider_name.to_lowercase() {
                    // 创建使用指定模型的服务副本
                    let mut new_config = config.clone();
                    new_config.model = model_id.to_string();

                    let new_service = AiService {
                        config: new_config,
                        db: self.db.clone(),
                        app_handle: self.app_handle.read().unwrap().clone(),
                        mcp_service: self.mcp_service.clone(),
                        max_retries: 3,
                    };

                    info!("通过推断为模型 {} 找到提供商 {}", model_id, provider_name);
                    return Ok(Some(new_service));
                }
            }
        }

        warn!("找不到支持模型 {} 的服务", model_id);
        Ok(None)
    }

    /// 根据模型名推断提供商
    async fn infer_provider_from_model(&self, model_id: &str) -> Option<String> {
        let model_lower = model_id.to_lowercase();
        tracing::debug!("开始为模型 {} 推断提供商", model_id);

        // 首先检查硬编码的模型前缀
        if model_lower.starts_with("gpt-") || model_lower.starts_with("o1-") {
            tracing::debug!("通过硬编码前缀为模型 {} 推断提供商: openai", model_id);
            return Some("openai".to_string());
        } else if model_lower.starts_with("claude-") {
            tracing::debug!("通过硬编码前缀为模型 {} 推断提供商: anthropic", model_id);
            return Some("anthropic".to_string());
        } else if model_lower.starts_with("deepseek-") {
            tracing::debug!("通过硬编码前缀为模型 {} 推断提供商: deepseek", model_id);
            return Some("deepseek".to_string());
        } else if model_lower.starts_with("gemini-") {
            tracing::debug!("通过硬编码前缀为模型 {} 推断提供商: google", model_id);
            return Some("google".to_string());
        } else if model_lower.starts_with("glm-") {
            tracing::debug!("通过硬编码前缀为模型 {} 推断提供商: zhipu", model_id);
            return Some("zhipu".to_string());
        } else if model_lower.starts_with("moonshot-") || model_lower.starts_with("kimi-") {
            tracing::debug!("通过硬编码前缀为模型 {} 推断提供商: moonshot", model_id);
            return Some("moonshot".to_string());
        } else if model_lower.starts_with("command-") || model_lower.starts_with("c4ai-") {
            tracing::debug!("通过硬编码前缀为模型 {} 推断提供商: cohere", model_id);
            return Some("cohere".to_string());
        } else if model_lower.starts_with("grok-") {
            tracing::debug!("通过硬编码前缀为模型 {} 推断提供商: xai", model_id);
            return Some("xai".to_string());
        } else if model_lower.starts_with("llama")
            || model_lower.starts_with("mixtral")
            || model_lower.starts_with("gemma")
        {
            tracing::debug!("通过硬编码前缀为模型 {} 推断提供商: groq", model_id);
            return Some("groq".to_string());
        }

        tracing::debug!(
            "硬编码前缀未匹配，尝试从数据库providers_config查找模型 {}",
            model_id
        );

        // 如果硬编码前缀没找到，尝试从数据库中的providers_config查找
        if let Ok(Some(providers_json)) = self.db.get_config("ai", "providers_config").await {
            if let Ok(providers) = serde_json::from_str::<
                std::collections::HashMap<String, serde_json::Value>,
            >(&providers_json)
            {
                for (_key, provider_data) in providers {
                    if let Some(provider_obj) = provider_data.as_object() {
                        // 检查提供商是否启用
                        if !provider_obj
                            .get("enabled")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false)
                        {
                            continue;
                        }

                        let provider_name = provider_obj.get("provider").and_then(|v| v.as_str());

                        // 检查该提供商的模型列表中是否包含该模型
                        if let Some(models_val) = provider_obj.get("models") {
                            if let Some(models_arr) = models_val.as_array() {
                                for model_val in models_arr {
                                    if let Some(model_obj) = model_val.as_object() {
                                        if let Some(model_name) =
                                            model_obj.get("id").and_then(|v| v.as_str())
                                        {
                                            // 支持精确匹配和部分匹配
                                            if model_name == model_id
                                                || model_name.to_lowercase() == model_lower
                                                || model_id.contains(model_name)
                                                || model_name.contains(model_id)
                                            {
                                                if let Some(provider) = provider_name {
                                                    tracing::info!("通过数据库providers_config为模型 {} 找到提供商 {}", model_id, provider);
                                                    return Some(provider.to_string());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // 如果模型列表为空，检查默认模型
                        if let Some(default_model) =
                            provider_obj.get("default_model").and_then(|v| v.as_str())
                        {
                            if default_model == model_id
                                || default_model.to_lowercase() == model_lower
                            {
                                if let Some(provider) = provider_name {
                                    tracing::info!("通过数据库providers_config的默认模型为模型 {} 找到提供商 {}", model_id, provider);
                                    return Some(provider.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        tracing::debug!("无法为模型 {} 推断出提供商", model_id);
        None
    }

    /// 检查模型是否被指定提供商支持
    async fn is_model_supported_by_service(&self, model_id: &str, provider: &str) -> bool {
        // 首先检查硬编码的支持规则
        let hardcoded_support = match provider.to_lowercase().as_str() {
            "openai" => model_id.starts_with("gpt-") || model_id.starts_with("o1-"),
            "anthropic" => model_id.starts_with("claude-"),
            "deepseek" => model_id.starts_with("deepseek-"),
            "groq" => {
                model_id.starts_with("llama")
                    || model_id.starts_with("mixtral")
                    || model_id.starts_with("gemma")
            }
            "ollama" => true, // Ollama支持各种模型
            "gemini" => model_id.starts_with("gemini-"),
            "zhipu" => model_id.starts_with("glm-"),
            "cohere" => model_id.starts_with("command-") || model_id.starts_with("c4ai-"),
            "xai" => model_id.starts_with("grok-"),
            "moonshot" => model_id.starts_with("moonshot-") || model_id.starts_with("kimi-"),
            "modelscope" => {
                // ModelScope 只支持特定的模型前缀，不支持所有模型
                model_id.starts_with("qwen")
                    || model_id.starts_with("baichuan")
                    || model_id.starts_with("chatglm")
                    || model_id.starts_with("internlm")
                    || model_id.starts_with("yi-")
                    || model_id.starts_with("deepseek-") // ModelScope 也提供 DeepSeek 模型
            }
            _ => false,
        };

        if hardcoded_support {
            return true;
        }

        // 如果硬编码规则不支持，检查数据库中的providers_config
        if let Ok(Some(providers_json)) = self.db.get_config("ai", "providers_config").await {
            if let Ok(providers) = serde_json::from_str::<
                std::collections::HashMap<String, serde_json::Value>,
            >(&providers_json)
            {
                for (_key, provider_data) in providers {
                    if let Some(provider_obj) = provider_data.as_object() {
                        let provider_name = provider_obj.get("provider").and_then(|v| v.as_str());

                        // 检查是否为目标提供商
                        if let Some(p_name) = provider_name {
                            if p_name.to_lowercase() == provider.to_lowercase() {
                                // 检查该提供商的模型列表
                                if let Some(models_val) = provider_obj.get("models") {
                                    if let Some(models_arr) = models_val.as_array() {
                                        for model_val in models_arr {
                                            if let Some(model_obj) = model_val.as_object() {
                                                if let Some(model_name) =
                                                    model_obj.get("id").and_then(|v| v.as_str())
                                                {
                                                    if model_name == model_id
                                                        || model_name.to_lowercase()
                                                            == model_id.to_lowercase()
                                                        || model_id.contains(model_name)
                                                        || model_name.contains(model_id)
                                                    {
                                                        return true;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // 检查默认模型
                                if let Some(default_model) =
                                    provider_obj.get("default_model").and_then(|v| v.as_str())
                                {
                                    if default_model == model_id
                                        || default_model.to_lowercase() == model_id.to_lowercase()
                                    {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        false
    }

    /// 检查数据库中是否有任何提供商支持该模型
    async fn is_model_supported_by_any_provider(&self, model_id: &str) -> bool {
        if let Ok(Some(providers_json)) = self.db.get_config("ai", "providers_config").await {
            if let Ok(providers) = serde_json::from_str::<
                std::collections::HashMap<String, serde_json::Value>,
            >(&providers_json)
            {
                for (_key, provider_data) in providers {
                    if let Some(provider_obj) = provider_data.as_object() {
                        // 检查该提供商的模型列表
                        if let Some(models_val) = provider_obj.get("models") {
                            if let Some(models_arr) = models_val.as_array() {
                                for model_val in models_arr {
                                    if let Some(model_obj) = model_val.as_object() {
                                        if let Some(model_name) =
                                            model_obj.get("id").and_then(|v| v.as_str())
                                        {
                                            if model_name == model_id
                                                || model_name.to_lowercase()
                                                    == model_id.to_lowercase()
                                                || model_id.contains(model_name)
                                                || model_name.contains(model_id)
                                            {
                                                return true;
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // 检查默认模型
                        if let Some(default_model) =
                            provider_obj.get("default_model").and_then(|v| v.as_str())
                        {
                            if default_model == model_id
                                || default_model.to_lowercase() == model_id.to_lowercase()
                            {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
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

        // 查找支持该模型的提供商配置
        if let Some(service) = self
            .find_service_by_provider_and_model(provider_name, model_id)
            .await?
        {
            let config = service.get_config();
            let ai_config = AiConfig {
                provider: config.provider.clone(),
                model: model_id.clone(),
                api_key: config.api_key.clone(),
                api_base: config.api_base.clone(),
                organization: config.organization.clone(),
                temperature: config.temperature,
                max_tokens: config.max_tokens,
            };
            Ok(Some(ai_config))
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
        info!("发送流式消息请求 - 模型: {}", self.config.model);
        println!("stream:{}, chunk_type: {:?}", stream, chunk_type);
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
                // 有状态请求：需要处理对话历史

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

                // 无论是否有历史，都在本次请求前注入 system prompt（仅用于LLM调用，不入库）
                if let Some(sys_prompt) = system_prompt {
                    if !sys_prompt.is_empty() {
                        // 移除已有的临时 system 消息，避免重复或冲突
                        messages.retain(|m| m.role != "system");
                        let system_msg = AiMessage {
                            id: uuid::Uuid::new_v4().to_string(),
                            conversation_id: conv_id.clone(),
                            role: "system".to_string(),
                            content: sys_prompt.to_string(),
                            metadata: None,
                            token_count: None,
                            cost: None,
                            tool_calls: None,
                            attachments: None,
                            timestamp: chrono::Utc::now(),
                        };
                        messages.insert(0, system_msg);
                        debug!("系统提示已插入到消息列表首位（仅本次调用，未保存到数据库）");
                    }
                }

                // 保存用户消息（如果提供了非空的用户输入）
                if let Some(up) = user_prompt {
                    if !up.trim().is_empty() {
                        let user_msg = AiMessage {
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
                        };
                        if conversation_exists {
                            match self.db.create_ai_message(&user_msg).await {
                                Ok(_) => {}
                                Err(e) => {
                                    warn!("用户消息保存失败: {}, 继续执行但不保存到数据库", e)
                                }
                            }
                        } else {
                            debug!("跳过用户消息保存：对话记录不存在");
                        }
                        messages.push(user_msg);
                    }
                }

                // 调用底层的聊天流式方法
                let model_name_owned = self.config.model.clone();

                self.send_chat_stream(
                    &model_name_owned,
                    messages,
                    conv_id,
                    self.config.temperature,
                    self.config.max_tokens,
                    message_id.clone(),         // 传递消息ID
                    Some(execution_id.clone()), // 明确传递execution_id
                    stream,
                    is_final,
                    chunk_type
                )
                .await
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
                if let Some(up) = user_prompt {
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
                let model_name_owned = self.config.model.clone();
                self.send_chat_stream(
                    &model_name_owned,
                    messages,
                    conv_id,
                    self.config.temperature,
                    self.config.max_tokens,
                    message_id.clone(),        // 保持 message_id 透传
                    Some(execution_id.clone()),
                    stream,
                    is_final,
                    chunk_type
                    
                )
                .await
            }
        }
    }

    // 发送聊天请求（处理工具调用） - 流式版本
    async fn send_chat_stream(
        &self,
        model_name: &str,
        messages: Vec<AiMessage>,
        conversation_id: &str,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
        assistant_message_id: Option<String>, // 新增助手消息ID参数
        execution_id: Option<String>,         // 新增执行ID参数
        stream_flg: bool,
        is_final: bool,
        chunk_type: Option<ChunkType>,
    ) -> Result<String> {
        info!(
            "Sending chat request to {} model with provider {}",
            model_name, self.config.provider
        );

        // Get or generate message ID
        let message_id = assistant_message_id
            .or_else(|| messages.last().map(|m| m.id.clone()))
            .unwrap_or_else(|| "unknown".to_string());

        // Get or generate execution ID - 优先使用传入的execution_id，其次使用conversation_id
        let execution_id = execution_id.unwrap_or_else(|| conversation_id.to_string());

        // Validate provider configuration
        if self.config.provider == "unconfigured" || self.config.provider == "mock" {
            let error_msg = "AI provider not configured. Please go to Settings > AI Configuration to set up an AI provider (OpenAI, Anthropic, DeepSeek, etc.)";
            error!("{}", error_msg);
            return Err(anyhow::anyhow!(error_msg));
        }

        // Validate API key
        if self.config.api_key.is_none()
            || self.config.api_key.as_ref().map_or(true, |k| k.is_empty())
        {
            let error_msg = format!("API key not configured for provider '{}'. Please check your AI configuration settings.", self.config.provider);
            error!("{}", error_msg);
            return Err(anyhow::anyhow!(error_msg));
        }

        // 转换消息格式为新的类型系统
        let mut chat_messages = Vec::new();
        for msg in &messages {
            // 过滤掉内容为空的消息，避免发送到AI提供商
            if msg.content.trim().is_empty() {
                debug!(
                    "Skipping message with empty content: id={}, role={}",
                    msg.id, msg.role
                );
                continue;
            }

            let role = match msg.role.as_str() {
                "system" => MessageRole::System,
                "user" => MessageRole::User,
                "assistant" => MessageRole::Assistant,
                "tool" => MessageRole::Tool,
                _ => MessageRole::User,
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
                        metadata
                            .get("tool_call_id")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                },
            };
            chat_messages.push(message);
        }

        // 准备工具（占位，如未来接入工具调用可填充）
        let tools_vec: Vec<crate::ai_adapter::types::Tool> = Vec::new();

        // 设置选项
        let mut options = crate::ai_adapter::types::ChatOptions::default();
        if let Some(temp) = temperature {
            options.temperature = Some(temp);
        }
        if let Some(tokens) = max_tokens {
            options.max_tokens = Some(tokens);
        }

        // 构建请求消息，并在必要时限制总大小，避免提供商的消息大小上限错误
        let mut request_messages: Vec<Message> = chat_messages
            .iter()
            .map(|msg| Message {
                role: msg.role.clone(),
                content: msg.content.clone(),
                name: msg.name.clone(),
                tool_calls: None,
                tool_call_id: msg.tool_call_id.clone(),
            })
            .collect();

        // 根据 providers_config 中当前模型的 context_length 动态裁剪上下文
        // 若未配置，则根据已知提供商采取安全回退（如 moonshot ~4MB）
        if let Ok(Some(providers_json)) = self.db.get_config("ai", "providers_config").await {
            if let Ok(root) = serde_json::from_str::<serde_json::Value>(&providers_json) {
                let provider_lc = self.config.provider.to_lowercase();
                // providers_config 是 map: { id: { provider, name, models: [...] } }
                if let Some(obj) = root.as_object() {
                    for (_key, prov_val) in obj {
                        if let Some(pobj) = prov_val.as_object() {
                            let prov_name = pobj
                                .get("provider")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_lowercase();
                            let svc_name = pobj
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_lowercase();
                            if prov_name == provider_lc || svc_name == provider_lc {
                                if let Some(models) = pobj.get("models").and_then(|v| v.as_array())
                                {
                                    // 匹配当前模型
                                    let mut found_tokens: Option<usize> = None;
                                    for m in models {
                                        if let Some(mo) = m.as_object() {
                                            let mid =
                                                mo.get("id").and_then(|v| v.as_str()).unwrap_or("");
                                            let mname = mo
                                                .get("name")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("");
                                            if mid == model_name || mname == model_name {
                                                if let Some(cfg) =
                                                    mo.get("config").and_then(|v| v.as_object())
                                                {
                                                    if let Some(cl) = cfg
                                                        .get("context_length")
                                                        .and_then(|v| v.as_u64())
                                                    {
                                                        found_tokens = Some(cl as usize);
                                                    }
                                                }
                                                break;
                                            }
                                        }
                                    }
                                    if let Some(tokens) = found_tokens
                                        .or_else(|| self.config.max_tokens.map(|t| t as usize))
                                    {
                                        // 估算最大字节：保守按 4 字节/Token，并预留 10% 裁剪空间
                                        let mut max_bytes = tokens.saturating_mul(4);
                                        max_bytes = max_bytes.saturating_mul(9) / 10; // 预留 10%
                                        if max_bytes > 0 {
                                            limit_total_message_bytes(
                                                &mut request_messages,
                                                max_bytes,
                                            );
                                        }
                                    }
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }

        // 转换为ChatRequest
        let chat_request = ChatRequest {
            model: model_name.to_string(),
            messages: request_messages,
            tools: if tools_vec.is_empty() {
                None
            } else {
                Some(tools_vec)
            },
            tool_choice: None,
            user: None,
            extra_params: None,
            options: Some(options),
        };

        // 发送请求 - 使用AiAdapterManager来获取提供商
        let adapter_manager = AiAdapterManager::global();
        let provider = adapter_manager
            .get_provider_or_default(&self.config.provider)
            .map_err(|e| {
                error!("Failed to get provider '{}': {}", self.config.provider, e);
                anyhow::anyhow!("Web call failed for model '{}'. Cause: {}", model_name, e)
            })?;

        // Use streaming response and emit events in real-time
        use futures::StreamExt;
        let mut stream = provider
            .send_chat_stream(&chat_request)
            .await
            .map_err(|e| {
                error!(
                    "Stream request failed for model '{}' on provider '{}': {}",
                    model_name,
                    provider.name(),
                    e
                );
                anyhow::anyhow!("Web call failed for model '{}'. Cause: {}", model_name, e)
            })?;

        // Process streaming response
        let mut content = String::new();
        let mut response_id = String::new();
        let mut response_model = String::new();
        let mut finish_reason: Option<String> = None;
        let mut usage = None;
        let mut chunk_count = 0;
        let stream_start_time = std::time::Instant::now();
        let mut last_chunk_time = stream_start_time;

        while let Some(chunk_result) = stream.stream.next().await {
            chunk_count += 1;
            let chunk_receive_time = std::time::Instant::now();
            let _chunk_interval = chunk_receive_time
                .duration_since(last_chunk_time)
                .as_millis();
            last_chunk_time = chunk_receive_time;

            match chunk_result {
                Ok(raw_chunk) => {
                    let mut parsed_delta: Option<String> = None;
                    let mut parsed_finish_reason: Option<String> = None;

                    if raw_chunk.content.trim_start().starts_with('{') {
                        if let Ok(json) =
                            serde_json::from_str::<serde_json::Value>(&raw_chunk.content)
                        {
                            if let Some(choice) = json
                                .get("choices")
                                .and_then(|c| c.as_array())
                                .and_then(|arr| arr.first())
                            {
                                // content delta
                                if let Some(delta_obj) = choice.get("delta") {
                                    if let Some(delta_str) =
                                        delta_obj.get("content").and_then(|c| c.as_str())
                                    {
                                        if !delta_str.is_empty() {
                                            parsed_delta = Some(delta_str.to_string());
                                        }
                                    }
                                    // optional reasoning stream
                                    if let Some(reasoning_str) =
                                        delta_obj.get("reasoning").and_then(|c| c.as_str())
                                    {
                                        if !reasoning_str.is_empty() {
                                            if stream_flg {
                                                self.emit_message_chunk(
                                                    &execution_id,
                                                    &message_id,
                                                    Some(conversation_id),
                                                    Some(ChunkType::Thinking),
                                                    reasoning_str,
                                                    is_final,
                                                    None,
                                                );
                                            }
                                        }
                                    }
                                }

                                // finish reason if present
                                parsed_finish_reason = choice
                                    .get("finish_reason")
                                    .and_then(|f| f.as_str())
                                    .map(|s| s.to_string());
                            }

                            // Fallback: some providers may send a cumulative message object
                            // Convert cumulative content into an incremental delta by diffing with the already accumulated `content`
                            if parsed_delta.is_none() {
                                if let Some(message_obj) = json.get("message") {
                                    if let Some(content_str) =
                                        message_obj.get("content").and_then(|c| c.as_str())
                                    {
                                        if !content_str.is_empty() {
                                            let full = content_str;
                                            let current = content.as_str();
                                            let incremental = if full.starts_with(current) {
                                                &full[current.len()..]
                                            } else {
                                                // Compute longest common prefix by chars to be UTF-8 safe
                                                let mut lcp_len = 0usize;
                                                for (a, b) in current.chars().zip(full.chars()) {
                                                    if a == b { lcp_len += a.len_utf8(); } else { break; }
                                                }
                                                &full[lcp_len..]
                                            };
                                            if !incremental.is_empty() {
                                                parsed_delta = Some(incremental.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Use parsed delta when available; otherwise treat content as plain text
                    if let Some(delta) = parsed_delta {
                        content.push_str(&delta);
                        if stream_flg {
                            self.emit_message_chunk(
                                &execution_id,
                                &message_id,
                                Some(conversation_id),
                                chunk_type.clone(),
                                &delta,
                                false,
                                None,
                            );
                        }
                    } else if !raw_chunk.content.is_empty() {
                        // Only forward non-JSON plain text directly
                        if !raw_chunk.content.trim_start().starts_with('{') {
                            content.push_str(&raw_chunk.content);
                            if stream_flg {
                                self.emit_message_chunk(
                                    &execution_id,
                                    &message_id,
                                    Some(conversation_id),
                                    chunk_type.clone(),
                                    &raw_chunk.content,
                                    false,
                                    None,
                                );
                            }
                        } else {
                            // JSON with no delta/content: ignore (e.g., role notifications)
                        }
                    }

                    if let Some(reason) = parsed_finish_reason.or(raw_chunk.finish_reason) {
                        finish_reason = Some(reason);
                    }
                    if !raw_chunk.id.is_empty() {
                        response_id = raw_chunk.id;
                    }
                    if !raw_chunk.model.is_empty() {
                        response_model = raw_chunk.model
                    }
                    if let Some(chunk_usage) = raw_chunk.usage {
                        usage = Some(chunk_usage);
                    }
                }
                Err(e) => {
                    error!("Stream chunk error after {} chunks: {}", chunk_count, e);

                    self.emit_message_chunk(
                        &execution_id,
                        &message_id,
                        Some(conversation_id),
                        Some(ChunkType::Error),
                        &e.to_string(),
                        true, // 标记为最终块，防止前端卡在流式状态
                        None,
                    );

                    return Err(anyhow::anyhow!("Stream error: {}", e));
                }
            }
        }

        info!("chunk_type: {:?} content: {:?}", chunk_type, content);
        if !stream_flg {
            self.emit_message_chunk(
                &execution_id,
                &message_id,
                Some(conversation_id),
                chunk_type,
                &content,
                false,
                None,
            );
        }

        // 对于流式调用，流结束后统一发送一次终结块
        if is_final {
            self.emit_message_chunk(
                &execution_id,
                &message_id,
                Some(conversation_id),
                Some(ChunkType::Meta),
                "",
                true,
                None,
            );
        }

        // Handle empty content with valid finish reason
        if content.is_empty() && finish_reason.is_some() {
            info!(
                "Stream completed with {} chunks and empty content but valid finish_reason: {:?}",
                chunk_count, finish_reason
            );
        } else {
            info!(
                "Stream completed successfully after {} chunks, total content length: {}",
                chunk_count,
                content.len()
            );
        }

        // 创建响应对象
        let response = crate::ai_adapter::types::ChatResponse {
            id: response_id,
            model: response_model,
            message: crate::ai_adapter::types::Message::assistant(&content),
            choices: vec![crate::ai_adapter::types::Choice {
                index: 0,
                message: crate::ai_adapter::types::Message::assistant(&content),
                finish_reason: finish_reason.clone(),
            }],
            usage,
            finish_reason,
            created_at: std::time::SystemTime::now(),
        };

        // 如果没有工具调用，保存助手响应并返回
        let mut answer = match &response.message.content {
            text => text.clone(),
        };

        // 如果助手响应为空，提供有用的错误信息
        if answer.trim().is_empty() {
            warn!(
                "Assistant response is empty for model '{}' on provider '{}'",
                model_name,
                provider.name()
            );
            answer = format!("抱歉，AI模型 {} 在 {} 提供商上没有返回任何响应。这可能是由于：\n\n1. API配置问题（请检查API密钥和基础URL）\n2. 模型暂时不可用\n3. 请求被限流\n\n请尝试重新发送消息或切换到其他模型。", model_name, provider.name());
        }
        let _assistant_msg = AiMessage {
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

        Ok(answer.to_string())
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

/// Cap total UTF-8 bytes of messages to avoid provider request size limits.
fn limit_total_message_bytes(
    messages: &mut Vec<crate::ai_adapter::types::Message>,
    max_bytes: usize,
) {
    let mut total: usize = messages.iter().map(|m| m.content.as_bytes().len()).sum();
    if total <= max_bytes {
        return;
    }

    // Prefer dropping oldest non-system/non-tool messages first
    let mut i = 0;
    while total > max_bytes && i < messages.len() {
        let role = &messages[i].role;
        let is_system_or_tool = matches!(
            role,
            crate::models::ai::MessageRole::System | crate::models::ai::MessageRole::Tool
        );
        if !is_system_or_tool {
            if let Some(msg) = messages.get(i) {
                total = total.saturating_sub(msg.content.as_bytes().len());
            }
            messages.remove(i);
            continue;
        }
        i += 1;
    }

    // If still too large, remove from start regardless of role
    while total > max_bytes && !messages.is_empty() {
        if let Some(msg) = messages.first() {
            total = total.saturating_sub(msg.content.as_bytes().len());
        }
        messages.remove(0);
    }
}
