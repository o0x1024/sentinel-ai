use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use sentinel_db::Database;

const MODEL_VISION_CAPABILITY_CACHE_CATEGORY: &str = "ai";
const MODEL_VISION_CAPABILITY_CACHE_KEY: &str = "model_vision_capability_cache";
const RUNTIME_PROBE_TTL_MS: i64 = 7 * 24 * 60 * 60 * 1000;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelVisionCapabilityStatus {
    Supported,
    Unsupported,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelVisionCapabilitySource {
    ProviderMetadata,
    LocalRegistry,
    RuntimeProbe,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelVisionCapabilityRecord {
    pub status: ModelVisionCapabilityStatus,
    pub source: ModelVisionCapabilitySource,
    pub checked_at_ms: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolvedModelVisionCapability {
    pub status: ModelVisionCapabilityStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<ModelVisionCapabilitySource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
}

impl ResolvedModelVisionCapability {
    pub fn unknown() -> Self {
        Self {
            status: ModelVisionCapabilityStatus::Unknown,
            source: None,
            evidence: None,
        }
    }
}

fn normalize_cache_part(value: Option<&str>) -> String {
    value.unwrap_or_default().trim().to_ascii_lowercase()
}

fn build_cache_key(
    provider: &str,
    model_name: &str,
    api_base: Option<&str>,
    rig_provider: Option<&str>,
) -> String {
    format!(
        "{}|{}|{}|{}",
        provider.trim().to_ascii_lowercase(),
        model_name.trim().to_ascii_lowercase(),
        normalize_cache_part(api_base),
        normalize_cache_part(rig_provider),
    )
}

fn cache_key_matches_scope(
    key: &str,
    provider: &str,
    api_base: Option<&str>,
    rig_provider: Option<&str>,
) -> bool {
    let mut parts = key.splitn(4, '|');
    let key_provider = parts.next().unwrap_or_default();
    let _key_model = parts.next().unwrap_or_default();
    let key_api_base = parts.next().unwrap_or_default();
    let key_rig_provider = parts.next().unwrap_or_default();

    key_provider == provider.trim().to_ascii_lowercase()
        && key_api_base == normalize_cache_part(api_base)
        && key_rig_provider == normalize_cache_part(rig_provider)
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_millis() as i64)
        .unwrap_or_default()
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

async fn load_capability_cache(
    db: &sentinel_db::DatabaseService,
) -> HashMap<String, ModelVisionCapabilityRecord> {
    match db
        .get_config(
            MODEL_VISION_CAPABILITY_CACHE_CATEGORY,
            MODEL_VISION_CAPABILITY_CACHE_KEY,
        )
        .await
    {
        Ok(Some(raw)) => serde_json::from_str::<HashMap<String, ModelVisionCapabilityRecord>>(&raw)
            .unwrap_or_default(),
        _ => HashMap::new(),
    }
}

pub async fn load_model_vision_capability_cache_snapshot(
    db: &sentinel_db::DatabaseService,
) -> HashMap<String, ModelVisionCapabilityRecord> {
    load_capability_cache(db).await
}

async fn save_capability_cache(
    db: &sentinel_db::DatabaseService,
    cache: &HashMap<String, ModelVisionCapabilityRecord>,
) -> Result<(), String> {
    let serialized = serde_json::to_string(cache)
        .map_err(|e| format!("Failed to serialize model vision capability cache: {}", e))?;
    db.set_config(
        MODEL_VISION_CAPABILITY_CACHE_CATEGORY,
        MODEL_VISION_CAPABILITY_CACHE_KEY,
        &serialized,
        Some("Persistent model vision capability cache"),
    )
    .await
    .map_err(|e| format!("Failed to save model vision capability cache: {}", e))
}

fn runtime_probe_record_is_fresh(record: &ModelVisionCapabilityRecord) -> bool {
    now_ms().saturating_sub(record.checked_at_ms) <= RUNTIME_PROBE_TTL_MS
}

pub fn get_cached_model_vision_capability_from_snapshot(
    cache: &HashMap<String, ModelVisionCapabilityRecord>,
    provider: &str,
    model_name: &str,
    api_base: Option<&str>,
    rig_provider: Option<&str>,
) -> Option<ModelVisionCapabilityRecord> {
    let key = build_cache_key(provider, model_name, api_base, rig_provider);
    let record = cache.get(&key)?.clone();
    if record.source == ModelVisionCapabilitySource::RuntimeProbe
        && !runtime_probe_record_is_fresh(&record)
    {
        return None;
    }
    Some(record)
}

pub async fn load_cached_model_vision_capability(
    db: &sentinel_db::DatabaseService,
    provider: &str,
    model_name: &str,
    api_base: Option<&str>,
    rig_provider: Option<&str>,
) -> Option<ModelVisionCapabilityRecord> {
    let cache = load_capability_cache(db).await;
    get_cached_model_vision_capability_from_snapshot(
        &cache,
        provider,
        model_name,
        api_base,
        rig_provider,
    )
}

pub async fn save_cached_model_vision_capability(
    db: &sentinel_db::DatabaseService,
    provider: &str,
    model_name: &str,
    api_base: Option<&str>,
    rig_provider: Option<&str>,
    status: ModelVisionCapabilityStatus,
    evidence: Option<String>,
) -> Result<(), String> {
    let mut cache = load_capability_cache(db).await;
    let key = build_cache_key(provider, model_name, api_base, rig_provider);
    cache.insert(
        key,
        ModelVisionCapabilityRecord {
            status,
            source: ModelVisionCapabilitySource::RuntimeProbe,
            checked_at_ms: now_ms(),
            evidence,
        },
    );
    save_capability_cache(db, &cache).await
}

pub async fn clear_cached_model_vision_capabilities(
    db: &sentinel_db::DatabaseService,
    provider: &str,
    api_base: Option<&str>,
    rig_provider: Option<&str>,
) -> Result<usize, String> {
    let mut cache = load_capability_cache(db).await;
    let original_len = cache.len();
    cache.retain(|key, _| !cache_key_matches_scope(key, provider, api_base, rig_provider));
    let removed = original_len.saturating_sub(cache.len());
    if removed == 0 {
        return Ok(0);
    }
    save_capability_cache(db, &cache).await?;
    Ok(removed)
}

pub async fn resolve_model_vision_capability(
    db: &sentinel_db::DatabaseService,
    provider: &str,
    model_name: &str,
    api_base: Option<&str>,
    rig_provider: Option<&str>,
) -> ResolvedModelVisionCapability {
    let provider_lower = provider.trim().to_ascii_lowercase();
    let model_lower = model_name.trim().to_ascii_lowercase();
    if provider_lower.is_empty() || model_lower.is_empty() {
        return ResolvedModelVisionCapability::unknown();
    }

    let providers_json = match db.get_config("ai", "providers_config").await {
        Ok(Some(value)) => value,
        _ => String::new(),
    };

    if !providers_json.trim().is_empty() {
        match serde_json::from_str::<HashMap<String, serde_json::Value>>(&providers_json) {
            Ok(providers) => {
                for (provider_key, provider_value) in providers {
                    let Some(provider_obj) = provider_value.as_object() else {
                        continue;
                    };

                    let provider_matches = provider_key.eq_ignore_ascii_case(&provider_lower)
                        || provider_obj
                            .get("provider")
                            .and_then(|v| v.as_str())
                            .map(|v| v.eq_ignore_ascii_case(&provider_lower))
                            .unwrap_or(false)
                        || provider_obj
                            .get("rig_provider")
                            .and_then(|v| v.as_str())
                            .map(|v| v.eq_ignore_ascii_case(&provider_lower))
                            .unwrap_or(false);
                    if !provider_matches {
                        continue;
                    }

                    let Some(models) = provider_obj.get("models").and_then(|v| v.as_array()) else {
                        break;
                    };

                    for model in models {
                        let model_id_matches = model
                            .get("id")
                            .and_then(|v| v.as_str())
                            .map(|v| v.eq_ignore_ascii_case(&model_lower))
                            .unwrap_or(false);
                        let model_name_matches = model
                            .get("name")
                            .and_then(|v| v.as_str())
                            .map(|v| v.eq_ignore_ascii_case(&model_lower))
                            .unwrap_or(false);
                        if !model_id_matches && !model_name_matches {
                            continue;
                        }

                        let explicit =
                            read_json_bool_field(model, "supports_vision").or_else(|| {
                                model
                                    .get("config")
                                    .and_then(|cfg| read_json_bool_field(cfg, "supports_vision"))
                            });

                        if let Some(supports_vision) = explicit {
                            return ResolvedModelVisionCapability {
                                status: if supports_vision {
                                    ModelVisionCapabilityStatus::Supported
                                } else {
                                    ModelVisionCapabilityStatus::Unsupported
                                },
                                source: Some(ModelVisionCapabilitySource::ProviderMetadata),
                                evidence: Some("providers_config.supports_vision".to_string()),
                            };
                        }
                    }

                    break;
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to parse providers_config while resolving model vision capability: {}",
                    e
                );
            }
        }
    }

    if let Some(record) =
        load_cached_model_vision_capability(db, provider, model_name, api_base, rig_provider).await
    {
        return ResolvedModelVisionCapability {
            status: record.status,
            source: Some(record.source),
            evidence: record.evidence,
        };
    }

    ResolvedModelVisionCapability::unknown()
}

pub fn classify_model_vision_capability_error(error: &str) -> Option<ModelVisionCapabilityStatus> {
    let lower = error.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return None;
    }

    let definitive_unsupported_patterns = [
        "does not support image",
        "doesn't support image",
        "does not support vision",
        "doesn't support vision",
        "does not support multimodal",
        "unsupported image input",
        "image input is not supported",
        "image inputs are not supported",
        "vision is not supported",
        "multimodal is not supported",
        "image_url is only supported by certain models",
        "invalid content type. image",
        "unknown type: input_image",
    ];

    if definitive_unsupported_patterns
        .iter()
        .any(|pattern| lower.contains(pattern))
    {
        return Some(ModelVisionCapabilityStatus::Unsupported);
    }

    let negative_words = ["unsupported", "not support", "does not support", "invalid"];
    let capability_words = ["vision", "image", "multimodal", "image_url", "input_image"];
    if negative_words
        .iter()
        .any(|negative| lower.contains(negative))
        && capability_words
            .iter()
            .any(|capability| lower.contains(capability))
    {
        return Some(ModelVisionCapabilityStatus::Unsupported);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{
        build_cache_key, cache_key_matches_scope, classify_model_vision_capability_error,
        get_cached_model_vision_capability_from_snapshot, ModelVisionCapabilityRecord,
        ModelVisionCapabilitySource, ModelVisionCapabilityStatus,
    };
    use std::collections::HashMap;

    #[test]
    fn build_cache_key_includes_endpoint_dimensions() {
        let key = build_cache_key(
            "OpenAI",
            "gpt-4o",
            Some("https://api.openai.com/v1"),
            Some("openai"),
        );
        assert_eq!(key, "openai|gpt-4o|https://api.openai.com/v1|openai");
    }

    #[test]
    fn cache_key_matches_scope_checks_provider_and_endpoint_dimensions() {
        let key = build_cache_key(
            "OpenAI",
            "gpt-4o",
            Some("https://api.openai.com/v1"),
            Some("openai"),
        );
        assert!(cache_key_matches_scope(
            &key,
            "openai",
            Some("https://api.openai.com/v1"),
            Some("openai"),
        ));
        assert!(!cache_key_matches_scope(
            &key,
            "openai",
            Some("https://example.com/v1"),
            Some("openai"),
        ));
        assert!(!cache_key_matches_scope(
            &key,
            "anthropic",
            Some("https://api.openai.com/v1"),
            Some("openai"),
        ));
    }

    #[test]
    fn classify_model_vision_capability_error_detects_unsupported_errors() {
        assert_eq!(
            classify_model_vision_capability_error("This model does not support image input."),
            Some(ModelVisionCapabilityStatus::Unsupported)
        );
        assert_eq!(
            classify_model_vision_capability_error("image_url is only supported by certain models"),
            Some(ModelVisionCapabilityStatus::Unsupported)
        );
    }

    #[test]
    fn classify_model_vision_capability_error_ignores_transport_failures() {
        assert_eq!(
            classify_model_vision_capability_error("request timed out after 30s"),
            None
        );
        assert_eq!(
            classify_model_vision_capability_error("401 unauthorized"),
            None
        );
    }

    #[test]
    fn snapshot_lookup_returns_matching_fresh_record() {
        let mut cache = HashMap::new();
        cache.insert(
            build_cache_key(
                "OpenAI",
                "gpt-4o",
                Some("https://api.openai.com/v1"),
                Some("openai"),
            ),
            ModelVisionCapabilityRecord {
                status: ModelVisionCapabilityStatus::Supported,
                source: ModelVisionCapabilitySource::RuntimeProbe,
                checked_at_ms: i64::MAX,
                evidence: Some("probe succeeded".to_string()),
            },
        );

        let record = get_cached_model_vision_capability_from_snapshot(
            &cache,
            "openai",
            "gpt-4o",
            Some("https://api.openai.com/v1"),
            Some("openai"),
        )
        .expect("record should be found");
        assert_eq!(record.status, ModelVisionCapabilityStatus::Supported);
    }
}
