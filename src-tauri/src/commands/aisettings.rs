use crate::services::ai::AiServiceManager;
use crate::services::database::DatabaseService;
use crate::services::{
    clear_cached_model_vision_capabilities, get_cached_model_vision_capability_from_snapshot,
    load_model_vision_capability_cache_snapshot, ModelVisionCapabilitySource,
    ModelVisionCapabilityStatus,
};
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
pub struct DeleteProviderRequest {
    pub provider: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClearModelVisionCapabilityCacheRequest {
    pub provider: String,
    pub api_base: Option<String>,
    pub rig_provider: Option<String>,
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

const DERIVED_MODEL_CONFIG_KEYS: [&str; 3] = [
    "vision_capability_status",
    "vision_capability_source",
    "vision_capability_evidence",
];

pub async fn cleanup_legacy_ai_config_keys(db: &DatabaseService) -> Result<usize, String> {
    let mut removed = 0usize;
    for key in [
        "default_vlm_provider",
        "default_vlm_model",
        "enable_multimodal",
    ] {
        match db.get_config_internal("ai", key).await {
            Ok(Some(_)) => {
                db.delete_config_internal("ai", key).await.map_err(|e| {
                    format!("Failed to clear legacy AI config key '{}': {}", key, e)
                })?;
                removed += 1;
            }
            Ok(None) => {}
            Err(e) => {
                return Err(format!(
                    "Failed to inspect legacy AI config key '{}': {}",
                    key, e
                ));
            }
        }
    }
    Ok(removed)
}

fn read_json_bool_field(value: &serde_json::Value, key: &str) -> Option<bool> {
    match value.get(key) {
        Some(serde_json::Value::Bool(v)) => Some(*v),
        Some(serde_json::Value::String(v)) => match v.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => Some(true),
            "false" | "0" | "no" | "off" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

fn read_json_bool_value(value: Option<&serde_json::Value>) -> Option<bool> {
    match value {
        Some(serde_json::Value::Bool(v)) => Some(*v),
        Some(serde_json::Value::String(v)) => match v.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => Some(true),
            "false" | "0" | "no" | "off" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

fn annotate_model_vision_capabilities(
    providers_config: &mut serde_json::Value,
    capability_cache: &HashMap<
        String,
        crate::services::model_capabilities::ModelVisionCapabilityRecord,
    >,
) {
    let Some(providers) = providers_config.as_object_mut() else {
        return;
    };

    for (provider_key, provider_value) in providers.iter_mut() {
        let Some(provider_obj) = provider_value.as_object_mut() else {
            continue;
        };

        let provider_name = provider_obj
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or(provider_key.as_str())
            .to_string();
        let api_base = provider_obj
            .get("api_base")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string());
        let rig_provider = provider_obj
            .get("rig_provider")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string());

        let Some(models) = provider_obj
            .get_mut("models")
            .and_then(|v| v.as_array_mut())
        else {
            continue;
        };

        for model in models.iter_mut() {
            let Some(model_obj) = model.as_object_mut() else {
                continue;
            };

            let model_name = model_obj
                .get("id")
                .and_then(|v| v.as_str())
                .or_else(|| model_obj.get("name").and_then(|v| v.as_str()))
                .unwrap_or_default()
                .trim()
                .to_string();

            let explicit = read_json_bool_value(model_obj.get("supports_vision")).or_else(|| {
                model_obj
                    .get("config")
                    .and_then(|cfg| read_json_bool_field(cfg, "supports_vision"))
            });

            let (status, source, evidence) = if let Some(supports_vision) = explicit {
                (
                    if supports_vision {
                        ModelVisionCapabilityStatus::Supported
                    } else {
                        ModelVisionCapabilityStatus::Unsupported
                    },
                    Some(ModelVisionCapabilitySource::ProviderMetadata),
                    Some("providers_config.supports_vision".to_string()),
                )
            } else if !model_name.is_empty() {
                match get_cached_model_vision_capability_from_snapshot(
                    capability_cache,
                    &provider_name,
                    &model_name,
                    api_base.as_deref(),
                    rig_provider.as_deref(),
                ) {
                    Some(record) => (record.status, Some(record.source), record.evidence),
                    None => (ModelVisionCapabilityStatus::Unknown, None, None),
                }
            } else {
                (ModelVisionCapabilityStatus::Unknown, None, None)
            };

            model_obj.insert(
                "vision_capability_status".to_string(),
                serde_json::to_value(status).unwrap_or(serde_json::Value::String("unknown".into())),
            );
            match source {
                Some(source_value) => {
                    model_obj.insert(
                        "vision_capability_source".to_string(),
                        serde_json::to_value(source_value)
                            .unwrap_or(serde_json::Value::String("runtime_probe".into())),
                    );
                }
                None => {
                    model_obj.remove("vision_capability_source");
                }
            }
            match evidence {
                Some(evidence_value) if !evidence_value.trim().is_empty() => {
                    model_obj.insert(
                        "vision_capability_evidence".to_string(),
                        serde_json::Value::String(evidence_value),
                    );
                }
                _ => {
                    model_obj.remove("vision_capability_evidence");
                }
            }
        }
    }
}

fn strip_derived_model_config_fields(models: &mut [serde_json::Value]) {
    for model in models.iter_mut() {
        let Some(model_obj) = model.as_object_mut() else {
            continue;
        };
        for key in DERIVED_MODEL_CONFIG_KEYS {
            model_obj.remove(key);
        }
    }
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
    let mut config = config;

    for provider in config.providers.values_mut() {
        strip_derived_model_config_fields(&mut provider.models);
    }

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
        if let Ok(Some(default_llm_provider)) = db
            .inner()
            .get_config_internal("ai", "default_llm_provider")
            .await
        {
            if let Err(e) = ai_manager.set_default_alias_to(&default_llm_provider).await {
                tracing::warn!(
                    "Failed to set default alias to '{}': {}",
                    default_llm_provider,
                    e
                );
            } else {
                tracing::info!(
                    "Default LLM provider alias updated to '{}'",
                    default_llm_provider
                );
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

#[cfg(test)]
mod tests {
    use super::strip_derived_model_config_fields;

    #[test]
    fn strip_derived_model_config_fields_removes_runtime_vision_keys() {
        let mut models = vec![serde_json::json!({
            "id": "gpt-4o",
            "supports_vision": true,
            "vision_capability_status": "supported",
            "vision_capability_source": "runtime_probe",
            "vision_capability_evidence": "probe ok"
        })];

        strip_derived_model_config_fields(&mut models);

        assert_eq!(
            models[0],
            serde_json::json!({
                "id": "gpt-4o",
                "supports_vision": true
            })
        );
    }
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
    let mut providers: HashMap<String, AiProviderConfig> =
        match db.get_config_internal("ai", "providers_config").await {
            Ok(Some(config_str)) => {
                serde_json::from_str(&config_str).unwrap_or_else(|_| HashMap::new())
            }
            _ => HashMap::new(),
        };

    let provider_id = request.name.clone();

    // Check if already exists
    if providers.contains_key(&provider_id) {
        return Err(format!(
            "Provider '{}' already exists",
            request.display_name
        ));
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

    tracing::info!(
        "Custom provider '{}' added successfully",
        request.display_name
    );

    // Reload AI services
    if let Some(ai_manager) = app.try_state::<Arc<AiServiceManager>>() {
        if let Err(e) = ai_manager.reload_services().await {
            tracing::error!(
                "Failed to reload AI services after adding custom provider: {}",
                e
            );
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

/// Delete custom provider
#[tauri::command]
pub async fn delete_ai_provider(
    request: DeleteProviderRequest,
    db: State<'_, Arc<DatabaseService>>,
    app: AppHandle,
) -> Result<(), String> {
    tracing::info!("Deleting AI provider: {}", request.provider);

    let mut providers: HashMap<String, AiProviderConfig> =
        match db.get_config_internal("ai", "providers_config").await {
            Ok(Some(config_str)) => {
                serde_json::from_str(&config_str).unwrap_or_else(|_| HashMap::new())
            }
            _ => HashMap::new(),
        };

    let matched_key = providers
        .keys()
        .find(|key| key.eq_ignore_ascii_case(&request.provider))
        .cloned()
        .ok_or_else(|| format!("Provider '{}' not found", request.provider))?;

    let removed_provider = providers
        .remove(&matched_key)
        .ok_or_else(|| format!("Provider '{}' not found", matched_key))?;

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

    let api_key_name = format!("api_key_{}", removed_provider.provider.to_lowercase());
    db.delete_config_internal("ai", &api_key_name)
        .await
        .map_err(|e| format!("Failed to remove provider API key: {}", e))?;

    let deleted_provider_lower = removed_provider.provider.to_lowercase();
    let fallback_provider = providers
        .values()
        .find(|provider| provider.provider.eq_ignore_ascii_case("openai"))
        .map(|provider| provider.provider.to_lowercase())
        .or_else(|| {
            providers
                .values()
                .next()
                .map(|provider| provider.provider.to_lowercase())
        });

    if let Ok(Some(default_llm_provider)) =
        db.get_config_internal("ai", "default_llm_provider").await
    {
        if default_llm_provider.eq_ignore_ascii_case(&deleted_provider_lower) {
            if let Some(fallback_provider) = &fallback_provider {
                db.set_config_internal(
                    "ai",
                    "default_llm_provider",
                    fallback_provider,
                    Some("Global default LLM provider"),
                )
                .await
                .map_err(|e| format!("Failed to update default LLM provider: {}", e))?;

                if let Err(e) = app.emit("ai_default_llm_provider_updated", fallback_provider) {
                    tracing::warn!(
                        "Failed to emit ai_default_llm_provider_updated event: {}",
                        e
                    );
                }
            } else {
                db.delete_config_internal("ai", "default_llm_provider")
                    .await
                    .map_err(|e| format!("Failed to clear default LLM provider: {}", e))?;
            }
        }
    }

    if let Ok(Some(default_llm_model)) = db.get_config_internal("ai", "default_llm_model").await {
        if provider_model_belongs_to(&default_llm_model, &deleted_provider_lower) {
            db.delete_config_internal("ai", "default_llm_model")
                .await
                .map_err(|e| format!("Failed to clear default LLM model: {}", e))?;

            if let Err(e) = app.emit("ai_default_llm_model_updated", "") {
                tracing::warn!("Failed to emit ai_default_llm_model_updated event: {}", e);
            }
        }
    }

    if let Err(e) = cleanup_legacy_ai_config_keys(db.inner()).await {
        tracing::warn!(
            "Failed to clear legacy AI config keys after provider delete: {}",
            e
        );
    }

    if let Some(ai_manager) = app.try_state::<Arc<AiServiceManager>>() {
        if let Err(e) = ai_manager.reload_services().await {
            tracing::error!(
                "Failed to reload AI services after deleting provider: {}",
                e
            );
        } else if let Some(fallback_provider) = &fallback_provider {
            if let Err(e) = ai_manager.set_default_alias_to(fallback_provider).await {
                tracing::warn!(
                    "Failed to set fallback default alias '{}' after deleting provider: {}",
                    fallback_provider,
                    e
                );
            }
        }
    }

    if let Err(e) = app.emit("ai_config_updated", ()) {
        tracing::warn!("Failed to emit ai_config_updated event: {}", e);
    }

    Ok(())
}

/// Restore built-in provider
#[tauri::command]
pub async fn restore_builtin_ai_provider(
    request: DeleteProviderRequest,
    db: State<'_, Arc<DatabaseService>>,
    app: AppHandle,
) -> Result<(), String> {
    tracing::info!("Restoring built-in AI provider: {}", request.provider);

    let defaults = default_providers_config();
    let default_map = defaults
        .as_object()
        .ok_or_else(|| "Built-in AI provider catalog is invalid".to_string())?;

    let matched_entry = default_map
        .iter()
        .find(|(key, value)| {
            key.eq_ignore_ascii_case(&request.provider)
                || value
                    .get("provider")
                    .and_then(|provider| provider.as_str())
                    .map(|provider| provider.eq_ignore_ascii_case(&request.provider))
                    .unwrap_or(false)
        })
        .ok_or_else(|| format!("Built-in provider '{}' not found", request.provider))?;

    let matched_key = matched_entry.0.clone();
    let provider_value = matched_entry.1.clone();

    let mut providers = match db.get_config_internal("ai", "providers_config").await {
        Ok(Some(config_str)) => {
            serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&config_str)
                .unwrap_or_default()
        }
        _ => serde_json::Map::new(),
    };

    if providers
        .keys()
        .any(|key| key.eq_ignore_ascii_case(&matched_key))
    {
        return Err(format!("Provider '{}' already exists", matched_key));
    }

    providers.insert(matched_key.clone(), provider_value);

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

    if let Some(ai_manager) = app.try_state::<Arc<AiServiceManager>>() {
        if let Err(e) = ai_manager.reload_services().await {
            tracing::error!(
                "Failed to reload AI services after restoring provider: {}",
                e
            );
        }
    }

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

    if let Err(e) = cleanup_legacy_ai_config_keys(db.inner()).await {
        tracing::warn!(
            "Failed to clear legacy AI config keys after default provider update: {}",
            e
        );
    }

    // Apply to runtime
    if let Some(ai_manager) = app.try_state::<Arc<AiServiceManager>>() {
        if let Err(e) = ai_manager.set_default_alias_to(&provider).await {
            tracing::warn!("Failed to update default alias: {}", e);
        }
    }

    // Notify frontend
    if let Err(e) = app.emit("ai_default_llm_provider_updated", &provider) {
        tracing::warn!(
            "Failed to emit ai_default_llm_provider_updated event: {}",
            e
        );
    }

    Ok(())
}

/// Set default LLM model
#[tauri::command]
pub async fn set_default_llm_model(
    model: String,
    ai_manager: State<'_, Arc<AiServiceManager>>,
    db: State<'_, Arc<DatabaseService>>,
    app: AppHandle,
) -> Result<(), String> {
    // Save to database
    db.set_config_internal("ai", "default_llm_model", &model, Some("Default LLM model"))
        .await
        .map_err(|e| e.to_string())?;

    if let Err(e) = cleanup_legacy_ai_config_keys(db.inner()).await {
        tracing::warn!(
            "Failed to clear legacy AI config keys after default model update: {}",
            e
        );
    }

    // Update AI manager if format is provider/model_name
    if let Some((provider, model_name)) = model.split_once('/') {
        if let Err(e) = ai_manager.set_default_llm_model(provider, model_name).await {
            tracing::warn!("Failed to update AI manager default chat model: {}", e);
        }
    }

    tracing::info!("Set default chat model to: {}", model);

    if let Err(e) = app.emit("ai_default_llm_model_updated", &model) {
        tracing::warn!("Failed to emit ai_default_llm_model_updated event: {}", e);
    }

    Ok(())
}

#[tauri::command]
pub async fn clear_model_vision_capability_cache(
    request: ClearModelVisionCapabilityCacheRequest,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<usize, String> {
    if request.provider.trim().is_empty() {
        return Err("Provider is required".to_string());
    }

    clear_cached_model_vision_capabilities(
        db.inner(),
        &request.provider,
        request.api_base.as_deref(),
        request.rig_provider.as_deref(),
    )
    .await
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

    // Get providers config from database. On first install (or invalid data), initialize defaults.
    let mut should_persist_default_providers = false;
    let providers_config = match db.get_config_internal("ai", "providers_config").await {
        Ok(Some(providers_json)) => {
            match serde_json::from_str::<serde_json::Value>(&providers_json) {
                Ok(providers) => providers,
                Err(e) => {
                    tracing::warn!(
                        "Failed to parse AI providers configuration, using Rig defaults: {}",
                        e
                    );
                    should_persist_default_providers = true;
                    default_providers_config()
                }
            }
        }
        Ok(None) => {
            tracing::info!("No AI providers configuration found, initializing Rig defaults");
            should_persist_default_providers = true;
            default_providers_config()
        }
        Err(e) => {
            tracing::warn!("Failed to load AI providers configuration: {}", e);
            should_persist_default_providers = true;
            default_providers_config()
        }
    };
    let mut providers_config_with_capabilities = providers_config.clone();
    let capability_cache = load_model_vision_capability_cache_snapshot(db.inner()).await;
    annotate_model_vision_capabilities(&mut providers_config_with_capabilities, &capability_cache);
    ai_config["providers"] = providers_config_with_capabilities;

    if should_persist_default_providers {
        match serde_json::to_string(&providers_config) {
            Ok(config_str) => {
                if let Err(e) = db
                    .set_config_internal(
                        "ai",
                        "providers_config",
                        &config_str,
                        Some("AI providers configuration"),
                    )
                    .await
                {
                    tracing::warn!(
                        "Failed to persist default AI providers configuration: {}",
                        e
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to serialize default AI providers configuration: {}",
                    e
                );
            }
        }
    }

    // Get other AI config items
    if let Ok(Some(default_llm_provider)) =
        db.get_config_internal("ai", "default_llm_provider").await
    {
        ai_config["default_llm_provider"] = serde_json::Value::String(default_llm_provider);
    }

    if let Ok(Some(default_model)) = db.get_config_internal("ai", "default_model").await {
        ai_config["default_model"] = serde_json::Value::String(default_model);
    }

    if let Ok(Some(default_llm_model)) = db.get_config_internal("ai", "default_llm_model").await {
        ai_config["default_llm_model"] = serde_json::Value::String(default_llm_model);
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

    if let Ok(Some(max_turns_str)) = db.get_config_internal("ai", "max_turns").await {
        if let Ok(max_turns) = max_turns_str.parse::<u32>() {
            ai_config["max_turns"] = serde_json::Value::Number(serde_json::Number::from(max_turns));
        }
    }

    if let Ok(Some(output_storage_threshold_str)) = db
        .get_config_internal("ai", "output_storage_threshold")
        .await
    {
        if let Ok(output_storage_threshold) = output_storage_threshold_str.parse::<u64>() {
            ai_config["output_storage_threshold"] =
                serde_json::Value::Number(serde_json::Number::from(output_storage_threshold));
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
                org.parse()
                    .map_err(|e| format!("Invalid organization ID: {}", e))?,
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
                org.parse()
                    .map_err(|e| format!("Invalid organization ID: {}", e))?,
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
    let api_base = request
        .api_base
        .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

    // Check if this is a local service (LM Studio, Ollama, etc.) that doesn't require API key
    let is_local_service = api_base.starts_with("http://localhost")
        || api_base.starts_with("http://127.0.0.1")
        || api_base.starts_with("http://0.0.0.0");

    // For non-local services, API key is required
    if !is_local_service && request.api_key.is_none() {
        return Ok(TestConnectionResponse {
            success: false,
            message: "OpenAI API key cannot be empty".to_string(),
            models: None,
        });
    }

    let client = create_client_with_proxy()
        .await
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let mut headers = reqwest::header::HeaderMap::new();

    // Use provided API key, or use a dummy key for local services
    let api_key = request.api_key.unwrap_or_else(|| "lm-studio".to_string());
    headers.insert(
        "Authorization",
        format!("Bearer {}", api_key)
            .parse()
            .map_err(|e| format!("Invalid API key: {}", e))?,
    );

    if let Some(org) = &request.organization {
        if !org.is_empty() {
            headers.insert(
                "OpenAI-Organization",
                org.parse()
                    .map_err(|e| format!("Invalid organization ID: {}", e))?,
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
        let models = vec![];

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

    let base_provider = |id: &str, name: &str, rig_provider: &str, api_base: Option<&str>| {
        json!({
            "id": id,
            "provider": id,
            "rig_provider": rig_provider,
            "name": name,
            "enabled": false,
            "api_key": null,
            "api_base": api_base,
            "organization": null,
            "default_model": "",
            "models": [],
            "max_context_length": null
        })
    };

    let providers: Vec<(&'static str, serde_json::Value)> = vec![
        (
            "Anthropic",
            base_provider(
                "anthropic",
                "Anthropic",
                "anthropic",
                Some("https://api.anthropic.com"),
            ),
        ),
        (
            "OpenAI",
            base_provider(
                "openai",
                "OpenAI",
                "openai",
                Some("https://api.openai.com/v1"),
            ),
        ),
        (
            "Azure OpenAI",
            base_provider("azure", "Azure OpenAI", "azure", None),
        ),
        (
            "Cohere",
            base_provider("cohere", "Cohere", "cohere", Some("https://api.cohere.ai")),
        ),
        (
            "DeepSeek",
            base_provider(
                "deepseek",
                "DeepSeek",
                "deepseek",
                Some("https://api.deepseek.com/v1"),
            ),
        ),
        (
            "EternalAI",
            base_provider("eternalai", "EternalAI", "eternalai", None),
        ),
        (
            "Google Gemini",
            base_provider("gemini", "Google Gemini", "gemini", None),
        ),
        (
            "Galadriel",
            base_provider("galadriel", "Galadriel", "galadriel", None),
        ),
        (
            "Groq",
            base_provider(
                "groq",
                "Groq",
                "groq",
                Some("https://api.groq.com/openai/v1"),
            ),
        ),
        (
            "Hyperbolic",
            base_provider(
                "hyperbolic",
                "Hyperbolic",
                "hyperbolic",
                Some("https://api.hyperbolic.xyz/v1"),
            ),
        ),
        ("Mira", base_provider("mira", "Mira", "mira", None)),
        (
            "Moonshot",
            base_provider(
                "moonshot",
                "Moonshot",
                "moonshot",
                Some("https://api.moonshot.cn/v1"),
            ),
        ),
        (
            "Ollama",
            base_provider("ollama", "Ollama", "ollama", Some("http://localhost:11434")),
        ),
        (
            "Perplexity",
            base_provider(
                "perplexity",
                "Perplexity",
                "perplexity",
                Some("https://api.perplexity.ai"),
            ),
        ),
        (
            "TogetherAI",
            base_provider(
                "togetherai",
                "TogetherAI",
                "togetherai",
                Some("https://api.together.xyz/v1"),
            ),
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
                "max_context_length": null,
                "http_referer": null,
                "x_title": null
            }),
        ),
        (
            "xAI",
            base_provider("xai", "xAI", "xai", Some("https://api.x.ai/v1")),
        ),
    ];

    let mut map = serde_json::Map::new();
    for (key, value) in providers {
        map.insert(key.to_string(), value);
    }
    serde_json::Value::Object(map)
}

fn provider_model_belongs_to(model: &str, provider: &str) -> bool {
    model
        .split_once('/')
        .map(|(model_provider, _)| model_provider.eq_ignore_ascii_case(provider))
        .unwrap_or(false)
}
