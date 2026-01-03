use crate::services::ai::AiServiceManager;
use crate::services::database::DatabaseService;
use sentinel_core::global_proxy::create_client_with_proxy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};

// ============== Data Structures ==============

#[derive(Debug, Serialize, Deserialize)]
pub struct TestConnectionRequest {
    pub provider: String,
    pub api_key: Option<String>,
    pub api_base: Option<String>,
    pub organization: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestConnectionResponse {
    pub success: bool,
    pub message: String,
    pub models: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveAiConfigRequest {
    pub providers: HashMap<String, AiProviderConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetDefaultProviderRequest {
    pub provider: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddCustomProviderRequest {
    pub name: String,
    pub display_name: String,
    pub api_key: Option<String>,
    pub api_base: String,
    pub model_id: String,
    pub compat_mode: String,
    pub extra_headers: Option<HashMap<String, String>>,
    pub timeout: Option<u64>,
    pub max_retries: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AiProviderConfig {
    pub id: String,
    pub provider: String,
    pub name: String,
    pub api_key: Option<String>,
    pub api_base: Option<String>,
    pub organization: Option<String>,
    pub enabled: bool,
    pub default_model: String,
    pub models: Vec<serde_json::Value>,
    pub rig_provider: Option<String>,
    pub max_context_length: Option<u32>,
}

// ============== Tauri Commands ==============

/// Get AI provider models
#[tauri::command]
pub async fn get_provider_models(
    provider: String,
    api_key: Option<String>,
    api_base: Option<String>,
    organization: Option<String>,
) -> Result<Vec<String>, String> {
    let request = TestConnectionRequest {
        provider: provider.clone(),
        api_key,
        api_base,
        organization,
        model: None,
    };

    let response = match provider.to_lowercase().as_str() {
        "openai" => test_openai_connection(request).await?,
        "anthropic" => test_anthropic_connection(request).await?,
        "gemini" => test_gemini_connection(request).await?,
        "deepseek" => test_deepseek_connection(request).await?,
        "moonshot" => test_moonshot_connection(request).await?,
        "ollama" => test_ollama_connection(request).await?,
        "openrouter" => test_openrouter_connection(request).await?,
        "modelscope" => test_modelscope_connection(request).await?,
        "lm studio" | "lmstudio" | "lm_studio" => test_lm_studio_connection(request).await?,
        _ => return Err(format!("Unsupported AI provider: {}", provider)),
    };

    if response.success {
        Ok(response.models.unwrap_or_default())
    } else {
        Err(response.message)
    }
}

/// Test AI provider connection
#[tauri::command]
pub async fn test_ai_connection(
    request: TestConnectionRequest,
) -> Result<TestConnectionResponse, String> {
    match request.provider.to_lowercase().as_str() {
        "openai" => test_openai_connection(request).await,
        "anthropic" => test_anthropic_connection(request).await,
        "gemini" => test_gemini_connection(request).await,
        "deepseek" => test_deepseek_connection(request).await,
        "moonshot" => test_moonshot_connection(request).await,
        "ollama" => test_ollama_connection(request).await,
        "openrouter" => test_openrouter_connection(request).await,
        "modelscope" => test_modelscope_connection(request).await,
        "lm studio" | "lmstudio" | "lm_studio" => test_lm_studio_connection(request).await,
        _ => Ok(TestConnectionResponse {
            success: false,
            message: format!("Unsupported AI provider: {}", request.provider),
            models: None,
        }),
    }
}

/// Save AI configuration
#[tauri::command]
pub async fn save_ai_config(
    config: SaveAiConfigRequest,
    db: State<'_, Arc<DatabaseService>>,
    _ai_manager_state: State<'_, Arc<AiServiceManager>>,
    app: AppHandle,
) -> Result<(), String> {
    tracing::info!("Starting to save AI configuration...");

    let db_service = db.inner().clone();

    // Save providers config as JSON
    let config_str = serde_json::to_string(&config.providers)
        .map_err(|e| format!("Failed to serialize providers config: {}", e))?;

    db_service
        .set_config_internal(
            "ai",
            "providers_config",
            &config_str,
            Some("AI providers configuration"),
        )
        .await
        .map_err(|e| format!("Failed to save providers config to DB: {}", e))?;

    // Save API keys for each provider
    for provider in config.providers.values() {
        if provider.enabled {
            if let Some(api_key) = &provider.api_key {
                if !api_key.is_empty() {
                    let key_name = format!("api_key_{}", provider.provider.to_lowercase());
                    let description = format!("{} API Key", provider.provider);
                    if let Err(e) = db_service
                        .set_config_internal("ai", &key_name, api_key, Some(&description))
                        .await
                    {
                        tracing::error!("Failed to save API key for {}: {}", provider.provider, e);
                    } else {
                        tracing::info!("Saved API key for {}", provider.provider);
                    }
                }
            }
        }
    }

    tracing::info!("AI configuration saved successfully");

    // Reload AI services
    if let Some(ai_manager) = app.try_state::<Arc<AiServiceManager>>() {
        if let Err(e) = ai_manager.reload_services().await {
            tracing::error!("Failed to reload AI services after saving config: {}", e);
        } else {
            tracing::info!("AI services reloaded after saving config");
        }
        
        // Apply default LLM provider alias
        if let Ok(Some(default_llm_provider)) = db.inner().get_config_internal("ai", "default_llm_provider").await {
            if let Err(e) = ai_manager.set_default_alias_to(&default_llm_provider).await {
                tracing::warn!(
                    "Failed to set default alias to '{}': {}",
                    default_llm_provider,
                    e
                );
            } else {
                tracing::info!("Default LLM provider alias updated to '{}'", default_llm_provider);
            }
        }
    }

    // Emit config update event
    if let Err(e) = app.emit("ai_config_updated", ()) {
        tracing::warn!("Failed to emit ai_config_updated event: {}", e);
    } else {
        tracing::info!("Emitted ai_config_updated event to frontend");
    }

    Ok(())
}

/// Add custom provider
#[tauri::command]
pub async fn add_custom_provider(
    request: AddCustomProviderRequest,
    db: State<'_, Arc<DatabaseService>>,
    app: AppHandle,
) -> Result<(), String> {
    tracing::info!("Adding custom provider: {}", request.name);

    // Load existing config
    let mut providers: HashMap<String, AiProviderConfig> = match db.get_config_internal("ai", "providers_config").await {
        Ok(Some(config_str)) => {
            serde_json::from_str(&config_str)
                .unwrap_or_else(|_| HashMap::new())
        }
        _ => HashMap::new(),
    };

    let provider_id = request.name.clone();

    // Check if already exists
    if providers.contains_key(&provider_id) {
        return Err(format!("Provider '{}' already exists", request.display_name));
    }

    // Build model config
    let model_config = serde_json::json!({
        "id": request.model_id,
        "name": request.model_id,
        "context_window": 128000,
        "max_output_tokens": 4096,
        "input_cost_per_token": 0.0,
        "output_cost_per_token": 0.0,
        "supports_vision": false,
        "supports_function_calling": true,
    });

    // Create new provider config
    let new_provider = AiProviderConfig {
        id: provider_id.clone(),
        provider: provider_id.clone(),
        name: request.display_name.clone(),
        api_key: request.api_key.clone(),
        api_base: Some(request.api_base.clone()),
        organization: None,
        enabled: true,
        default_model: request.model_id.clone(),
        models: vec![model_config],
        rig_provider: Some(request.compat_mode.clone()),
        max_context_length: Some(128000), // Default context length
    };

    providers.insert(provider_id.clone(), new_provider);

    // Save to database
    let config_str = serde_json::to_string(&providers)
        .map_err(|e| format!("Failed to serialize providers config: {}", e))?;

    db.set_config_internal(
        "ai",
        "providers_config",
        &config_str,
        Some("AI providers configuration"),
    )
    .await
    .map_err(|e| format!("Failed to save providers config to DB: {}", e))?;

    // Save API key if provided
    if let Some(api_key) = &request.api_key {
        if !api_key.is_empty() {
            let key_name = format!("api_key_{}", provider_id);
            let description = format!("{} API Key", request.display_name);
            db.set_config_internal("ai", &key_name, api_key, Some(&description))
                .await
                .map_err(|e| format!("Failed to save API key: {}", e))?;
        }
    }

    tracing::info!("Custom provider '{}' added successfully", request.display_name);

    // Reload AI services
    if let Some(ai_manager) = app.try_state::<Arc<AiServiceManager>>() {
        if let Err(e) = ai_manager.reload_services().await {
            tracing::error!("Failed to reload AI services after adding custom provider: {}", e);
        } else {
            tracing::info!("AI services reloaded after adding custom provider");
        }
    }

    // Emit config update event
    if let Err(e) = app.emit("ai_config_updated", ()) {
        tracing::warn!("Failed to emit ai_config_updated event: {}", e);
    }

    Ok(())
}

/// Set default LLM provider
#[tauri::command]
pub async fn set_default_llm_provider(
    request: SetDefaultProviderRequest,
    db: State<'_, Arc<DatabaseService>>,
    app: AppHandle,
) -> Result<(), String> {
    let provider = request.provider.to_lowercase();

    // Save to database
    db.set_config_internal(
        "ai",
        "default_llm_provider",
        &provider,
        Some("Global default LLM provider"),
    )
    .await
    .map_err(|e| e.to_string())?;

    // Apply to runtime
    if let Some(ai_manager) = app.try_state::<Arc<AiServiceManager>>() {
        if let Err(e) = ai_manager.set_default_alias_to(&provider).await {
            tracing::warn!("Failed to update default alias: {}", e);
        }
    }

    // Notify frontend
    if let Err(e) = app.emit("ai_default_llm_provider_updated", &provider) {
        tracing::warn!("Failed to emit ai_default_llm_provider_updated event: {}", e);
    }

    Ok(())
}

/// Set default LLM model
#[tauri::command]
pub async fn set_default_llm_model(
    model: String,
    ai_manager: State<'_, Arc<AiServiceManager>>,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    // Save to database
    db.set_config_internal(
        "ai",
        "default_llm_model",
        &model,
        Some("Default LLM model"),
    )
    .await
    .map_err(|e| e.to_string())?;

    // Update AI manager if format is provider/model_name
    if let Some((provider, model_name)) = model.split_once('/') {
        if let Err(e) = ai_manager
            .set_default_llm_model(provider, model_name)
            .await
        {
            tracing::warn!("Failed to update AI manager default chat model: {}", e);
        }
    }

    tracing::info!("Set default chat model to: {}", model);
    Ok(())
}

/// Set default VLM model
#[tauri::command]
pub async fn set_default_vlm_model(
    model: String,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    db.set_config_internal(
        "ai",
        "default_vlm_model",
        &model,
        Some("Default VLM model"),
    )
    .await
    .map_err(|e| e.to_string())?;

    tracing::info!("Set default vision model to: {}", model);
    Ok(())
}

/// Get AI configuration
#[tauri::command]
pub async fn get_ai_config(
    db: State<'_, Arc<DatabaseService>>,
) -> Result<serde_json::Value, String> {
    tracing::info!("Getting AI configuration");

    let mut ai_config = serde_json::json!({
        "providers": {}
    });

    // Get providers config from database
    match db.get_config_internal("ai", "providers_config").await {
        Ok(Some(providers_json)) => {
            if let Ok(providers) = serde_json::from_str::<serde_json::Value>(&providers_json) {
                ai_config["providers"] = providers;
            }
        }
        Ok(None) => {
            tracing::info!("No AI providers configuration found, using Rig defaults");
            ai_config["providers"] = default_providers_config();
        }
        Err(e) => {
            tracing::warn!("Failed to load AI providers configuration: {}", e);
            ai_config["providers"] = default_providers_config();
        }
    }

    // Get other AI config items
    if let Ok(Some(default_llm_provider)) = db.get_config_internal("ai", "default_llm_provider").await {
        ai_config["default_llm_provider"] = serde_json::Value::String(default_llm_provider);
    }

    if let Ok(Some(default_model)) = db.get_config_internal("ai", "default_model").await {
        ai_config["default_model"] = serde_json::Value::String(default_model);
    }

    if let Ok(Some(default_llm_model)) = db.get_config_internal("ai", "default_llm_model").await {
        ai_config["default_llm_model"] = serde_json::Value::String(default_llm_model);
    }

    if let Ok(Some(default_vlm_provider)) = db.get_config_internal("ai", "default_vlm_provider").await {
        ai_config["default_vlm_provider"] = serde_json::Value::String(default_vlm_provider);
    }

    if let Ok(Some(default_vlm_model)) = db.get_config_internal("ai", "default_vlm_model").await {
        ai_config["default_vlm_model"] = serde_json::Value::String(default_vlm_model);
    }

    if let Ok(Some(temperature_str)) = db.get_config_internal("ai", "temperature").await {
        if let Ok(temperature) = temperature_str.parse::<f64>() {
            ai_config["temperature"] = serde_json::Value::Number(
                serde_json::Number::from_f64(temperature)
                    .unwrap_or(serde_json::Number::from_f64(0.7).unwrap()),
            );
        }
    }

    if let Ok(Some(max_tokens_str)) = db.get_config_internal("ai", "max_tokens").await {
        if let Ok(max_tokens) = max_tokens_str.parse::<u32>() {
            ai_config["max_tokens"] =
                serde_json::Value::Number(serde_json::Number::from(max_tokens));
        }
    }

    if let Ok(Some(stream_response_str)) = db.get_config_internal("ai", "stream_response").await {
        if let Ok(stream_response) = stream_response_str.parse::<bool>() {
            ai_config["stream_response"] = serde_json::Value::Bool(stream_response);
        }
    }

    if let Ok(Some(enable_multimodal_str)) = db.get_config_internal("ai", "enable_multimodal").await {
        if let Ok(enable_multimodal) = enable_multimodal_str.parse::<bool>() {
            ai_config["enable_multimodal"] = serde_json::Value::Bool(enable_multimodal);
        }
    }

    if let Ok(Some(max_turns_str)) = db.get_config_internal("ai", "max_turns").await {
        if let Ok(max_turns) = max_turns_str.parse::<u32>() {
            ai_config["max_turns"] =
                serde_json::Value::Number(serde_json::Number::from(max_turns));
        }
    }

    tracing::info!("Successfully retrieved AI configuration");
    Ok(ai_config)
}

// ============== Provider Connection Tests ==============

async fn test_modelscope_connection(
    request: TestConnectionRequest,
) -> Result<TestConnectionResponse, String> {
    if request.api_key.is_none() {
        return Ok(TestConnectionResponse {
            success: false,
            message: "ModelScope API key cannot be empty".to_string(),
            models: None,
        });
    }

    let client = create_client_with_proxy()
        .await
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    let api_base = request
        .api_base
        .unwrap_or_else(|| "https://api-inference.modelscope.cn/v1/models".to_string());

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "Authorization",
        format!("Bearer {}", request.api_key.unwrap())
            .parse()
            .map_err(|e| format!("Invalid API key: {}", e))?,
    );

    if let Some(org) = &request.organization {
        if !org.is_empty() {
            headers.insert(
                "x-title",
                org.parse().map_err(|e| format!("Invalid organization ID: {}", e))?,
            );
        }
    }

    let response = client
        .get(format!("{}/models", api_base))
        .headers(headers)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to ModelScope: {}", e))?;

    if response.status().is_success() {
        let models_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if let Some(models_array) = models_response.get("data").and_then(|d| d.as_array()) {
            let models: Vec<String> = models_array
                .iter()
                .filter_map(|m| m.get("id").and_then(|id| id.as_str()).map(String::from))
                .collect();

            Ok(TestConnectionResponse {
                success: true,
                message: format!(
                    "Successfully connected to ModelScope, found {} models",
                    models.len()
                ),
                models: Some(models),
            })
        } else {
            Ok(TestConnectionResponse {
                success: true,
                message: "Successfully connected to ModelScope, but failed to get model list"
                    .to_string(),
                models: None,
            })
        }
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Ok(TestConnectionResponse {
            success: false,
            message: format!("Failed to connect to ModelScope: {}", error_text),
            models: None,
        })
    }
}

pub async fn test_openrouter_connection(
    request: TestConnectionRequest,
) -> Result<TestConnectionResponse, String> {
    if request.api_key.is_none() {
        return Ok(TestConnectionResponse {
            success: false,
            message: "OpenRouter API key cannot be empty".to_string(),
            models: None,
        });
    }

    let client = create_client_with_proxy()
        .await
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    let api_base = request
        .api_base
        .unwrap_or_else(|| "https://openrouter.ai/api/v1".to_string());

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "Authorization",
        format!("Bearer {}", request.api_key.unwrap())
            .parse()
            .map_err(|e| format!("Invalid API key: {}", e))?,
    );

    if let Some(org) = &request.organization {
        if !org.is_empty() {
            headers.insert(
                "x-title",
                org.parse().map_err(|e| format!("Invalid organization ID: {}", e))?,
            );
        }
    }

    let response = client
        .get(format!("{}/models", api_base))
        .headers(headers)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to OpenRouter: {}", e))?;

    if response.status().is_success() {
        let models_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if let Some(models_array) = models_response.get("data").and_then(|d| d.as_array()) {
            let models: Vec<String> = models_array
                .iter()
                .filter_map(|m| m.get("id").and_then(|id| id.as_str()).map(String::from))
                .collect();

            Ok(TestConnectionResponse {
                success: true,
                message: format!(
                    "Successfully connected to OpenRouter, found {} models",
                    models.len()
                ),
                models: Some(models),
            })
        } else {
            Ok(TestConnectionResponse {
                success: true,
                message: "Successfully connected to OpenRouter, but failed to get model list"
                    .to_string(),
                models: None,
            })
        }
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Ok(TestConnectionResponse {
            success: false,
            message: format!("Failed to connect to OpenRouter: {}", error_text),
            models: None,
        })
    }
}

async fn test_openai_connection(
    request: TestConnectionRequest,
) -> Result<TestConnectionResponse, String> {
    if request.api_key.is_none() {
        return Ok(TestConnectionResponse {
            success: false,
            message: "OpenAI API key cannot be empty".to_string(),
            models: None,
        });
    }

    let client = create_client_with_proxy()
        .await
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    let api_base = request
        .api_base
        .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "Authorization",
        format!("Bearer {}", request.api_key.unwrap())
            .parse()
            .map_err(|e| format!("Invalid API key: {}", e))?,
    );

    if let Some(org) = &request.organization {
        if !org.is_empty() {
            headers.insert(
                "OpenAI-Organization",
                org.parse().map_err(|e| format!("Invalid organization ID: {}", e))?,
            );
        }
    }

    let response = client
        .get(format!("{}/models", api_base))
        .headers(headers)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to OpenAI: {}", e))?;

    if response.status().is_success() {
        let models_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if let Some(models_array) = models_response.get("data").and_then(|d| d.as_array()) {
            let models: Vec<String> = models_array
                .iter()
                .filter_map(|m| m.get("id").and_then(|id| id.as_str()).map(String::from))
                .collect();

            Ok(TestConnectionResponse {
                success: true,
                message: format!(
                    "Successfully connected to OpenAI, found {} models",
                    models.len()
                ),
                models: Some(models),
            })
        } else {
            Ok(TestConnectionResponse {
                success: true,
                message: "Successfully connected to OpenAI, but failed to get model list"
                    .to_string(),
                models: None,
            })
        }
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Ok(TestConnectionResponse {
            success: false,
            message: format!("Failed to connect to OpenAI: {}", error_text),
            models: None,
        })
    }
}

async fn test_anthropic_connection(
    request: TestConnectionRequest,
) -> Result<TestConnectionResponse, String> {
    if request.api_key.is_none() {
        return Ok(TestConnectionResponse {
            success: false,
            message: "Anthropic API key cannot be empty".to_string(),
            models: None,
        });
    }

    let client = create_client_with_proxy()
        .await
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    let api_base = request
        .api_base
        .unwrap_or_else(|| "https://api.anthropic.com".to_string());

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "x-api-key",
        request
            .api_key
            .unwrap()
            .parse()
            .map_err(|e| format!("Invalid API key: {}", e))?,
    );
    headers.insert("anthropic-version", "2023-06-01".parse().unwrap());

    let test_payload = serde_json::json!({
        "model": "claude-3-haiku-20240307",
        "max_tokens": 1,
        "messages": [
            {
                "role": "user",
                "content": "Hello"
            }
        ]
    });

    let response = client
        .post(format!("{}/v1/messages", api_base))
        .headers(headers)
        .json(&test_payload)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to Anthropic: {}", e))?;

    if response.status().is_success() {
        let models = vec![
            "claude-3-opus-20240229".to_string(),
            "claude-3-sonnet-20240229".to_string(),
            "claude-3-haiku-20240307".to_string(),
            "claude-2.1".to_string(),
            "claude-2.0".to_string(),
            "claude-instant-1.2".to_string(),
        ];

        Ok(TestConnectionResponse {
            success: true,
            message: "Successfully connected to Anthropic Claude API".to_string(),
            models: Some(models),
        })
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Ok(TestConnectionResponse {
            success: false,
            message: format!("Failed to connect to Anthropic: {}", error_text),
            models: None,
        })
    }
}

async fn test_gemini_connection(
    request: TestConnectionRequest,
) -> Result<TestConnectionResponse, String> {
    if request.api_key.is_none() {
        return Ok(TestConnectionResponse {
            success: false,
            message: "Gemini API key cannot be empty".to_string(),
            models: None,
        });
    }

    let client = create_client_with_proxy()
        .await
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    let api_key = request.api_key.unwrap();

    let response = client
        .get(format!(
            "https://generativelanguage.googleapis.com/v1/models?key={}",
            api_key
        ))
        .send()
        .await
        .map_err(|e| format!("Failed to connect to Gemini: {}", e))?;

    if response.status().is_success() {
        let models_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if let Some(models_array) = models_response.get("models").and_then(|m| m.as_array()) {
            let models: Vec<String> = models_array
                .iter()
                .filter_map(|m| {
                    m.get("name")
                        .and_then(|n| n.as_str())
                        .map(|s| s.replace("models/", ""))
                })
                .collect();

            Ok(TestConnectionResponse {
                success: true,
                message: format!(
                    "Successfully connected to Gemini, found {} models",
                    models.len()
                ),
                models: Some(models),
            })
        } else {
            Ok(TestConnectionResponse {
                success: true,
                message: "Successfully connected to Gemini, but failed to get model list"
                    .to_string(),
                models: None,
            })
        }
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Ok(TestConnectionResponse {
            success: false,
            message: format!("Failed to connect to Gemini: {}", error_text),
            models: None,
        })
    }
}

async fn test_deepseek_connection(
    request: TestConnectionRequest,
) -> Result<TestConnectionResponse, String> {
    if request.api_key.is_none() {
        return Ok(TestConnectionResponse {
            success: false,
            message: "DeepSeek API key cannot be empty".to_string(),
            models: None,
        });
    }

    let client = create_client_with_proxy()
        .await
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    let api_base = request
        .api_base
        .unwrap_or_else(|| "https://api.deepseek.com/v1".to_string());

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "Authorization",
        format!("Bearer {}", request.api_key.unwrap())
            .parse()
            .map_err(|e| format!("Invalid API key: {}", e))?,
    );

    let response = client
        .get(format!("{}/models", api_base))
        .headers(headers)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to DeepSeek: {}", e))?;

    if response.status().is_success() {
        let models_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if let Some(models_array) = models_response.get("data").and_then(|d| d.as_array()) {
            let models: Vec<String> = models_array
                .iter()
                .filter_map(|m| m.get("id").and_then(|id| id.as_str()).map(String::from))
                .collect();

            Ok(TestConnectionResponse {
                success: true,
                message: format!(
                    "Successfully connected to DeepSeek, found {} models",
                    models.len()
                ),
                models: Some(models),
            })
        } else {
            Ok(TestConnectionResponse {
                success: true,
                message: "Successfully connected to DeepSeek, using default model list".to_string(),
                models: Some(vec![
                    "deepseek-reasoner".to_string(),
                    "deepseek-chat".to_string(),
                ]),
            })
        }
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Ok(TestConnectionResponse {
            success: false,
            message: format!("Failed to connect to DeepSeek: {}", error_text),
            models: None,
        })
    }
}

async fn test_lm_studio_connection(
    request: TestConnectionRequest,
) -> Result<TestConnectionResponse, String> {
    let client = create_client_with_proxy()
        .await
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    let api_base = request
        .api_base
        .unwrap_or_else(|| "http://localhost:1234".to_string());

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Content-Type", "application/json".parse().unwrap());
    headers.insert("Accept", "application/json".parse().unwrap());

    if let Some(api_key) = &request.api_key {
        if !api_key.is_empty() && api_key != "lm-studio" {
            headers.insert(
                "Authorization",
                format!("Bearer {}", api_key).parse().unwrap(),
            );
        }
    }

    let response = client
        .get(format!("{}/v1/models", api_base))
        .headers(headers)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to LM Studio: {}", e))?;

    if response.status().is_success() {
        let models_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if let Some(models_array) = models_response.get("data").and_then(|m| m.as_array()) {
            let models: Vec<String> = models_array
                .iter()
                .filter_map(|m| m.get("id").and_then(|n| n.as_str()).map(String::from))
                .collect();

            Ok(TestConnectionResponse {
                success: true,
                message: format!(
                    "Successfully connected to LM Studio, found {} local models",
                    models.len()
                ),
                models: Some(models),
            })
        } else {
            Ok(TestConnectionResponse {
                success: true,
                message: "Successfully connected to LM Studio, but no models found".to_string(),
                models: Some(vec![]),
            })
        }
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Ok(TestConnectionResponse {
            success: false,
            message: format!("Failed to connect to LM Studio: {}", error_text),
            models: None,
        })
    }
}

async fn test_ollama_connection(
    request: TestConnectionRequest,
) -> Result<TestConnectionResponse, String> {
    let api_base = request
        .api_base
        .unwrap_or_else(|| "http://localhost:11434".to_string());

    let client = create_client_with_proxy()
        .await
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    let response = client
        .get(format!("{}/api/tags", api_base))
        .send()
        .await
        .map_err(|e| format!("Failed to connect to Ollama: {}", e))?;

    if response.status().is_success() {
        let models_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if let Some(models_array) = models_response.get("models").and_then(|m| m.as_array()) {
            let models: Vec<String> = models_array
                .iter()
                .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(String::from))
                .collect();

            let mut rig_test_result = None;
            if !models.is_empty() {
                let test_model = &models[0];
                match test_ollama_with_rig(test_model).await {
                    Ok(rig_msg) => {
                        rig_test_result = Some(format!(" (Rig test: {})", rig_msg));
                    }
                    Err(e) => {
                        tracing::warn!("Rig connection test failed: {}", e);
                        rig_test_result = Some(format!(" (Rig test failed: {})", e));
                    }
                }
            }

            Ok(TestConnectionResponse {
                success: true,
                message: format!(
                    "Successfully connected to Ollama, found {} local models{}",
                    models.len(),
                    rig_test_result.unwrap_or_default()
                ),
                models: Some(models),
            })
        } else {
            Ok(TestConnectionResponse {
                success: true,
                message: "Successfully connected to Ollama, but failed to get model list"
                    .to_string(),
                models: None,
            })
        }
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Ok(TestConnectionResponse {
            success: false,
            message: format!("Failed to connect to Ollama: {}", error_text),
            models: None,
        })
    }
}

async fn test_ollama_with_rig(model: &str) -> Result<String, String> {
    use rig::client::{CompletionClient, ProviderClient};
    use rig::completion::Prompt;
    use rig::providers::ollama;

    let client = ollama::Client::from_env();
    let agent = client.agent(model).build();

    match agent.prompt("Hello").await {
        Ok(response) => {
            let response_text = response.trim();
            if response_text.is_empty() {
                Ok("Connected but got empty response".to_string())
            } else {
                Ok(format!(
                    "Connected and got response ({} chars)",
                    response_text.len()
                ))
            }
        }
        Err(e) => Err(format!("Rig connection failed: {}", e)),
    }
}

async fn test_moonshot_connection(
    request: TestConnectionRequest,
) -> Result<TestConnectionResponse, String> {
    if request.api_key.is_none() {
        return Ok(TestConnectionResponse {
            success: false,
            message: "Moonshot API key cannot be empty".to_string(),
            models: None,
        });
    }

    let client = create_client_with_proxy()
        .await
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    let api_base = request
        .api_base
        .unwrap_or_else(|| "https://api.moonshot.cn/v1".to_string());

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "Authorization",
        format!("Bearer {}", request.api_key.unwrap())
            .parse()
            .map_err(|e| format!("Invalid API key: {}", e))?,
    );

    let response = client
        .get(format!("{}/models", api_base))
        .headers(headers)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to Moonshot: {}", e))?;

    if response.status().is_success() {
        let models_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if let Some(models_array) = models_response.get("data").and_then(|d| d.as_array()) {
            let models: Vec<String> = models_array
                .iter()
                .filter_map(|m| m.get("id").and_then(|id| id.as_str()).map(String::from))
                .collect();

            Ok(TestConnectionResponse {
                success: true,
                message: format!(
                    "Successfully connected to Moonshot, found {} models",
                    models.len()
                ),
                models: Some(models),
            })
        } else {
            Ok(TestConnectionResponse {
                success: true,
                message: "Successfully connected to Moonshot, using default model list".to_string(),
                models: Some(vec![
                    "moonshot-v1-8k".to_string(),
                    "moonshot-v1-32k".to_string(),
                    "moonshot-v1-128k".to_string(),
                ]),
            })
        }
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Ok(TestConnectionResponse {
            success: false,
            message: format!("Failed to connect to Moonshot: {}", error_text),
            models: None,
        })
    }
}

// ============== Helper Functions ==============

fn default_providers_config() -> serde_json::Value {
    use serde_json::json;
    
    let providers: Vec<(&'static str, serde_json::Value)> = vec![
        (
            "OpenAI",
            json!({
                "id": "openai",
                "provider": "openai",
                "rig_provider": "openai",
                "name": "OpenAI",
                "enabled": false,
                "api_key": null,
                "api_base": "https://api.openai.com/v1",
                "organization": null,
                "default_model": "",
                "models": []
            }),
        ),
        (
            "Anthropic",
            json!({
                "id": "anthropic",
                "provider": "anthropic",
                "rig_provider": "anthropic",
                "name": "Anthropic",
                "enabled": false,
                "api_key": null,
                "api_base": "https://api.anthropic.com",
                "organization": null,
                "default_model": "",
                "models": []
            }),
        ),
        (
            "Gemini",
            json!({
                "id": "gemini",
                "provider": "gemini",
                "rig_provider": "gemini",
                "name": "Gemini",
                "enabled": false,
                "api_key": null,
                "api_base": null,
                "organization": null,
                "default_model": "",
                "models": []
            }),
        ),
        (
            "DeepSeek",
            json!({
                "id": "deepseek",
                "provider": "deepseek",
                "rig_provider": "deepseek",
                "name": "DeepSeek",
                "enabled": false,
                "api_key": null,
                "api_base": "https://api.deepseek.com/v1",
                "organization": null,
                "default_model": "",
                "models": []
            }),
        ),
        (
            "Ollama",
            json!({
                "id": "ollama",
                "provider": "ollama",
                "rig_provider": "ollama",
                "name": "Ollama",
                "enabled": false,
                "api_key": null,
                "api_base": "http://localhost:11434",
                "organization": null,
                "default_model": "",
                "models": []
            }),
        ),
        (
            "Groq",
            json!({
                "id": "groq",
                "provider": "groq",
                "rig_provider": "groq",
                "name": "Groq",
                "enabled": false,
                "api_key": null,
                "api_base": "https://api.groq.com/openai/v1",
                "organization": null,
                "default_model": "",
                "models": []
            }),
        ),
        (
            "Cohere",
            json!({
                "id": "cohere",
                "provider": "cohere",
                "rig_provider": "cohere",
                "name": "Cohere",
                "enabled": false,
                "api_key": null,
                "api_base": "https://api.cohere.ai",
                "organization": null,
                "default_model": "",
                "models": []
            }),
        ),
        (
            "Moonshot",
            json!({
                "id": "moonshot",
                "provider": "moonshot",
                "rig_provider": "moonshot",
                "name": "Moonshot",
                "enabled": false,
                "api_key": null,
                "api_base": "https://api.moonshot.cn/v1",
                "organization": null,
                "default_model": "",
                "models": []
            }),
        ),
        (
            "xAI",
            json!({
                "id": "xai",
                "provider": "xai",
                "rig_provider": "xai",
                "name": "xAI",
                "enabled": false,
                "api_key": null,
                "api_base": "https://api.x.ai/v1",
                "organization": null,
                "default_model": "",
                "models": []
            }),
        ),
        (
            "Perplexity",
            json!({
                "id": "perplexity",
                "provider": "perplexity",
                "rig_provider": "perplexity",
                "name": "Perplexity",
                "enabled": false,
                "api_key": null,
                "api_base": "https://api.perplexity.ai",
                "organization": null,
                "default_model": "",
                "models": []
            }),
        ),
        (
            "TogetherAI",
            json!({
                "id": "togetherai",
                "provider": "togetherai",
                "rig_provider": "togetherai",
                "name": "TogetherAI",
                "enabled": false,
                "api_key": null,
                "api_base": "https://api.together.xyz/v1",
                "organization": null,
                "default_model": "",
                "models": []
            }),
        ),
        (
            "OpenRouter",
            json!({
                "id": "openrouter",
                "provider": "openrouter",
                "rig_provider": "openrouter",
                "name": "OpenRouter",
                "enabled": false,
                "api_key": null,
                "api_base": "https://openrouter.ai/api/v1",
                "organization": null,
                "default_model": "",
                "models": [],
                "http_referer": null,
                "x_title": null
            }),
        ),
        (
            "ModelScope",
            json!({
                "id": "modelscope",
                "provider": "modelscope",
                "rig_provider": "openai",
                "name": "ModelScope",
                "enabled": false,
                "api_key": null,
                "api_base": "https://api-inference.modelscope.cn/v1",
                "organization": null,
                "default_model": "",
                "models": []
            }),
        ),
        (
            "LM Studio",
            json!({
                "id": "lm studio",
                "provider": "lm studio",
                "rig_provider": "openai",
                "name": "LM Studio",
                "enabled": false,
                "api_key": null,
                "api_base": "http://172.28.38.178:1234",
                "organization": null,
                "default_model": "",
                "models": []
            }),
        ),
        (
            "Hyperbolic",
            json!({
                "id": "hyperbolic",
                "provider": "hyperbolic",
                "rig_provider": "hyperbolic",
                "name": "Hyperbolic",
                "enabled": false,
                "api_key": null,
                "api_base": "https://api.hyperbolic.xyz/v1",
                "organization": null,
                "default_model": "",
                "models": []
            }),
        ),
    ];

    let mut map = serde_json::Map::new();
    for (key, value) in providers {
        map.insert(key.to_string(), value);
    }
    serde_json::Value::Object(map)
}

