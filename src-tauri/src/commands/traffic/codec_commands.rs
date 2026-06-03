use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use tauri::State;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::commands::command_response_support::CommandResponse;
use crate::services::traffic_codec::crypto::CodecDirection;
use crate::services::traffic_codec::{
    CodecExportData, CodecPipeline, CodecRequestMeta, CodecResult, CodecRuleExport,
    CodecStep, KeyPlaceholderInfo, TrafficCodecEngine, TrafficCodecRule,
    TrafficCodecStore,
};

const SENSITIVE_CONFIG_KEYS: &[&str] = &["key", "iv", "private_key", "public_key"];

pub struct TrafficCodecState {
    pub engine: Arc<RwLock<TrafficCodecEngine>>,
    pub store: Arc<TrafficCodecStore>,
}

async fn reload_engine_from_store(state: &TrafficCodecState) -> Result<(), String> {
    let rules = state
        .store
        .list_rules()
        .await
        .map_err(|error| error.to_string())?;
    state.engine.write().await.reload_rules(rules);
    Ok(())
}

fn placeholder_for_key(key: &str) -> String {
    format!("{{{{{key}}}}}")
}

fn sanitize_pipeline_for_export(
    pipeline: &CodecPipeline,
    key_placeholders: &mut HashMap<String, KeyPlaceholderInfo>,
) -> CodecPipeline {
    CodecPipeline {
        steps: pipeline
            .steps
            .iter()
            .map(|step| {
                let mut config = step.config.clone();
                for key in SENSITIVE_CONFIG_KEYS {
                    if let Some(value) = config.get(*key) {
                        if value.is_empty() {
                            continue;
                        }
                        let placeholder = placeholder_for_key(key);
                        let format_key = format!("{key}_format");
                        let format = config
                            .get(&format_key)
                            .cloned()
                            .unwrap_or_else(|| "utf8".to_string());
                        key_placeholders.entry(placeholder.clone()).or_insert_with(|| {
                            KeyPlaceholderInfo {
                                description: format!("Sensitive value for `{key}`"),
                                format,
                                length: Some(value.len() as u32),
                            }
                        });
                        config.insert(key.to_string(), placeholder);
                    }
                }
                CodecStep {
                    id: step.id.clone(),
                    step_type: step.step_type.clone(),
                    codec: step.codec.clone(),
                    plugin_id: step.plugin_id.clone(),
                    config,
                    enabled: step.enabled,
                }
            })
            .collect(),
    }
}

fn restore_pipeline_from_import(
    pipeline: &CodecPipeline,
    key_values: &HashMap<String, String>,
) -> Result<CodecPipeline, String> {
    Ok(CodecPipeline {
        steps: pipeline
            .steps
            .iter()
            .map(|step| {
                let mut config = step.config.clone();
                for key in SENSITIVE_CONFIG_KEYS {
                    if let Some(value) = config.get(*key) {
                        if value.starts_with("{{") && value.ends_with("}}") {
                            let restored = key_values.get(value).ok_or_else(|| {
                                format!("missing key value for placeholder: {value}")
                            })?;
                            config.insert(key.to_string(), restored.clone());
                        }
                    }
                }
                Ok(CodecStep {
                    id: Uuid::new_v4().to_string(),
                    step_type: step.step_type.clone(),
                    codec: step.codec.clone(),
                    plugin_id: step.plugin_id.clone(),
                    config,
                    enabled: step.enabled,
                })
            })
            .collect::<Result<Vec<_>, String>>()?,
    })
}

fn parse_direction(direction: &str) -> Result<CodecDirection, String> {
    match direction.to_lowercase().as_str() {
        "encode" => Ok(CodecDirection::Encode),
        "decode" => Ok(CodecDirection::Decode),
        other => Err(format!("invalid direction: {other}")),
    }
}

#[tauri::command]
pub async fn codec_list_rules(
    state: State<'_, TrafficCodecState>,
) -> Result<CommandResponse<Vec<TrafficCodecRule>>, String> {
    match state.store.list_rules().await {
        Ok(rules) => Ok(CommandResponse::ok(rules)),
        Err(error) => Ok(CommandResponse::err(error.to_string())),
    }
}

#[tauri::command]
pub async fn codec_save_rule(
    state: State<'_, TrafficCodecState>,
    mut rule: TrafficCodecRule,
) -> Result<CommandResponse<()>, String> {
    rule.updated_at = Utc::now().to_rfc3339();
    if rule.created_at.is_empty() {
        rule.created_at = rule.updated_at.clone();
    }

    match state.store.save_rule(&rule).await {
        Ok(()) => {
            if let Err(error) = reload_engine_from_store(&state).await {
                return Ok(CommandResponse::err(error));
            }
            Ok(CommandResponse::ok(()))
        }
        Err(error) => Ok(CommandResponse::err(error.to_string())),
    }
}

#[tauri::command]
pub async fn codec_delete_rule(
    state: State<'_, TrafficCodecState>,
    id: String,
) -> Result<CommandResponse<()>, String> {
    match state.store.delete_rule(&id).await {
        Ok(()) => {
            if let Err(error) = reload_engine_from_store(&state).await {
                return Ok(CommandResponse::err(error));
            }
            Ok(CommandResponse::ok(()))
        }
        Err(error) => Ok(CommandResponse::err(error.to_string())),
    }
}

#[tauri::command]
pub async fn codec_reorder_rules(
    state: State<'_, TrafficCodecState>,
    ids: Vec<String>,
) -> Result<CommandResponse<()>, String> {
    match state.store.reorder_rules(&ids).await {
        Ok(()) => {
            if let Err(error) = reload_engine_from_store(&state).await {
                return Ok(CommandResponse::err(error));
            }
            Ok(CommandResponse::ok(()))
        }
        Err(error) => Ok(CommandResponse::err(error.to_string())),
    }
}

#[tauri::command]
pub async fn codec_decode(
    state: State<'_, TrafficCodecState>,
    content: String,
    meta: CodecRequestMeta,
) -> Result<CommandResponse<CodecResult>, String> {
    let engine = state.engine.read().await;
    Ok(CommandResponse::ok(engine.decode(&content, &meta)))
}

#[tauri::command]
pub async fn codec_encode(
    state: State<'_, TrafficCodecState>,
    content: String,
    meta: CodecRequestMeta,
) -> Result<CommandResponse<CodecResult>, String> {
    let engine = state.engine.read().await;
    Ok(CommandResponse::ok(engine.encode(&content, &meta)))
}

#[tauri::command]
pub async fn codec_batch_encode(
    state: State<'_, TrafficCodecState>,
    contents: Vec<String>,
    meta: CodecRequestMeta,
) -> Result<CommandResponse<Vec<CodecResult>>, String> {
    let engine = state.engine.read().await;
    Ok(CommandResponse::ok(engine.batch_encode(&contents, &meta)))
}

#[tauri::command]
pub async fn codec_test_pipeline(
    state: State<'_, TrafficCodecState>,
    content: String,
    steps: Vec<CodecStep>,
    direction: String,
) -> Result<CommandResponse<CodecResult>, String> {
    let codec_direction = match parse_direction(&direction) {
        Ok(value) => value,
        Err(error) => return Ok(CommandResponse::err(error)),
    };
    let engine = state.engine.read().await;
    Ok(CommandResponse::ok(
        engine.test_pipeline(&content, &steps, codec_direction),
    ))
}

#[tauri::command]
pub async fn codec_export_rules(
    state: State<'_, TrafficCodecState>,
    rule_ids: Vec<String>,
) -> Result<CommandResponse<CodecExportData>, String> {
    let rules = match state.store.list_rules().await {
        Ok(rules) => rules,
        Err(error) => return Ok(CommandResponse::err(error.to_string())),
    };

    let selected: Vec<TrafficCodecRule> = if rule_ids.is_empty() {
        rules
    } else {
        rules
            .into_iter()
            .filter(|rule| rule_ids.contains(&rule.id))
            .collect()
    };

    let mut key_placeholders = HashMap::new();
    let exported_rules = selected
        .into_iter()
        .map(|rule| CodecRuleExport {
            name: rule.name,
            match_rule: rule.match_rule,
            scope: rule.scope,
            pipeline: sanitize_pipeline_for_export(&rule.pipeline, &mut key_placeholders),
            reversible: rule.reversible,
        })
        .collect();

    Ok(CommandResponse::ok(CodecExportData {
        version: "1".to_string(),
        format: "sentinel-codec-rules".to_string(),
        exported_at: Utc::now().to_rfc3339(),
        rules: exported_rules,
        key_placeholders,
    }))
}

#[tauri::command]
pub async fn codec_import_rules(
    state: State<'_, TrafficCodecState>,
    data: CodecExportData,
    key_values: HashMap<String, String>,
) -> Result<CommandResponse<Vec<String>>, String> {
    let mut imported_ids = Vec::new();
    let now = Utc::now().to_rfc3339();

    for (index, exported) in data.rules.into_iter().enumerate() {
        let pipeline = match restore_pipeline_from_import(&exported.pipeline, &key_values) {
            Ok(pipeline) => pipeline,
            Err(error) => return Ok(CommandResponse::err(error)),
        };

        let rule = TrafficCodecRule {
            id: Uuid::new_v4().to_string(),
            name: exported.name,
            enabled: true,
            order: index as i32,
            match_rule: exported.match_rule,
            scope: exported.scope,
            pipeline,
            reversible: exported.reversible,
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        if let Err(error) = state.store.save_rule(&rule).await {
            return Ok(CommandResponse::err(error.to_string()));
        }
        imported_ids.push(rule.id);
    }

    if let Err(error) = reload_engine_from_store(&state).await {
        return Ok(CommandResponse::err(error));
    }

    Ok(CommandResponse::ok(imported_ids))
}
