use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::AppHandle;

use crate::commands::ai_task_support::{
    build_virtual_tool_context, complete_external_profile_run_failure,
    complete_external_profile_run_success, load_external_profile_context,
    merge_external_profile_prompt, start_external_profile_run, ExternalProfileRun,
};
use crate::commands::plugin_generation_commands::get_combined_plugin_prompt_api;
use crate::commands::plugin_review_commands::{
    RuntimeSchemaValidationMetadata, RuntimeSchemaValidationResult,
};
use crate::commands::traffic::plugin_commands::{
    refresh_active_agent_plugin_tools, resolved_store_plugin_monitor_type,
};
use crate::commands::traffic::TrafficAnalysisState;
use crate::events::{emit_plugin_changed, PluginChangedEvent};
use crate::generators::{PluginValidator, ValidationResult};
use crate::services::plugin_execution_test::{
    build_plugin_metadata, parse_plugin_severity, test_plugin_code, PluginExecutionTestResult,
};
use crate::services::{AiServiceManager, SystemAgentRuntime};
use sentinel_db::{Database, TrafficPluginMetadata};

const PLUGIN_AUTHORING_PROFILE_ID: &str = "traffic_plugin_generator_agent";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginAuthoringAction {
    Generate,
    Improve,
    Validate,
    Test,
    SaveDraft,
    Enable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginAuthoringRequest {
    pub action: PluginAuthoringAction,
    pub main_category: Option<String>,
    pub category: Option<String>,
    pub requirements: Option<String>,
    pub existing_plugin_id: Option<String>,
    pub plugin_id: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub default_severity: Option<String>,
    pub monitor_type: Option<String>,
    pub traffic_samples: Option<Value>,
    pub code: Option<String>,
    pub enable_after_test: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginAuthoringResult {
    pub action: PluginAuthoringAction,
    pub plugin_id: Option<String>,
    pub model: Option<String>,
    pub saved_as_draft: bool,
    pub enabled: bool,
    pub message: String,
    pub code: Option<String>,
    pub validation: Option<ValidationResult>,
    pub runtime_schema: Option<RuntimeSchemaValidationResult>,
    pub execution_test: Option<PluginExecutionTestResult>,
}

#[derive(Debug, Clone)]
struct PluginAuthoringContext {
    plugin_id: String,
    name: String,
    description: String,
    author: Option<String>,
    main_category: String,
    category: String,
    monitor_type: Option<String>,
    default_severity: String,
}

pub async fn execute_plugin_authoring(
    app_handle: &AppHandle,
    traffic_state: &TrafficAnalysisState,
    ai_manager: &Arc<AiServiceManager>,
    runtime: &Arc<SystemAgentRuntime>,
    request: PluginAuthoringRequest,
) -> Result<PluginAuthoringResult> {
    match request.action {
        PluginAuthoringAction::Generate | PluginAuthoringAction::Improve => {
            execute_generation_flow(app_handle, traffic_state, ai_manager, runtime, request).await
        }
        PluginAuthoringAction::Validate => execute_validate_flow(traffic_state, request).await,
        PluginAuthoringAction::Test => execute_test_flow(traffic_state, request).await,
        PluginAuthoringAction::SaveDraft => {
            execute_save_draft_flow(app_handle, traffic_state, request).await
        }
        PluginAuthoringAction::Enable => {
            execute_enable_flow(app_handle, traffic_state, request).await
        }
    }
}

async fn execute_generation_flow(
    app_handle: &AppHandle,
    traffic_state: &TrafficAnalysisState,
    ai_manager: &Arc<AiServiceManager>,
    runtime: &Arc<SystemAgentRuntime>,
    request: PluginAuthoringRequest,
) -> Result<PluginAuthoringResult> {
    let request = hydrate_existing_plugin_code(traffic_state, request).await?;
    required_text(&request.requirements, "requirements")?;
    let context = resolve_authoring_context(traffic_state, &request).await?;

    let generation_run = start_generation_run(runtime, &request, &context).await;

    let generated = generate_plugin_code(app_handle, ai_manager, runtime, &request, &context).await;

    let (code, model) = match generated {
        Ok(output) => output,
        Err(error) => {
            complete_external_profile_run_failure(runtime, &generation_run, error.to_string())
                .await;
            return Err(error);
        }
    };

    let validation = validate_category_code(&context, &code).await?;
    let execution_test = test_category_code(&context, &code).await?;

    if !validation_allows_save(&validation, Some(&execution_test)) {
        let result = PluginAuthoringResult {
            action: request.action,
            plugin_id: Some(context.plugin_id.clone()),
            model: Some(model),
            saved_as_draft: false,
            enabled: false,
            message: "生成完成，但校验或测试未通过，未保存草稿".to_string(),
            code: Some(code),
            validation: validation.validation,
            runtime_schema: validation.runtime_schema,
            execution_test: Some(execution_test),
        };
        complete_external_profile_run_success(
            runtime,
            &generation_run,
            serde_json::to_value(&result)?,
        )
        .await;
        return Ok(result);
    }

    save_plugin_draft(app_handle, traffic_state, &context, &code).await?;

    let mut enabled = false;
    if request.enable_after_test.unwrap_or(false) {
        enable_plugin_by_id(app_handle, traffic_state, &context.plugin_id).await?;
        enabled = true;
    }

    let result = PluginAuthoringResult {
        action: request.action,
        plugin_id: Some(context.plugin_id.clone()),
        model: Some(model),
        saved_as_draft: true,
        enabled,
        message: if enabled {
            format!(
                "插件 '{}' 已生成、测试通过、保存为草稿并启用",
                context.plugin_id
            )
        } else {
            format!(
                "插件 '{}' 已生成、测试通过并保存为禁用草稿",
                context.plugin_id
            )
        },
        code: Some(code),
        validation: validation.validation,
        runtime_schema: validation.runtime_schema,
        execution_test: Some(execution_test),
    };

    complete_external_profile_run_success(runtime, &generation_run, serde_json::to_value(&result)?)
        .await;
    Ok(result)
}

async fn execute_validate_flow(
    traffic_state: &TrafficAnalysisState,
    request: PluginAuthoringRequest,
) -> Result<PluginAuthoringResult> {
    let (context, code) = resolve_context_and_code(traffic_state, &request).await?;
    let validation = validate_category_code(&context, &code).await?;

    Ok(PluginAuthoringResult {
        action: PluginAuthoringAction::Validate,
        plugin_id: Some(context.plugin_id),
        model: None,
        saved_as_draft: false,
        enabled: false,
        message: "插件代码校验已完成".to_string(),
        code: Some(code),
        validation: validation.validation,
        runtime_schema: validation.runtime_schema,
        execution_test: None,
    })
}

async fn execute_test_flow(
    traffic_state: &TrafficAnalysisState,
    request: PluginAuthoringRequest,
) -> Result<PluginAuthoringResult> {
    let (context, code) = resolve_context_and_code(traffic_state, &request).await?;
    let execution_test = test_category_code(&context, &code).await?;

    Ok(PluginAuthoringResult {
        action: PluginAuthoringAction::Test,
        plugin_id: Some(context.plugin_id),
        model: None,
        saved_as_draft: false,
        enabled: false,
        message: "插件测试已完成".to_string(),
        code: Some(code),
        validation: None,
        runtime_schema: None,
        execution_test: Some(execution_test),
    })
}

async fn execute_save_draft_flow(
    app_handle: &AppHandle,
    traffic_state: &TrafficAnalysisState,
    request: PluginAuthoringRequest,
) -> Result<PluginAuthoringResult> {
    let context = resolve_authoring_context(traffic_state, &request).await?;
    let code = required_text(&request.code, "code")?.to_string();
    save_plugin_draft(app_handle, traffic_state, &context, &code).await?;

    Ok(PluginAuthoringResult {
        action: PluginAuthoringAction::SaveDraft,
        plugin_id: Some(context.plugin_id.clone()),
        model: None,
        saved_as_draft: true,
        enabled: false,
        message: format!("插件 '{}' 已保存为禁用草稿", context.plugin_id),
        code: Some(code),
        validation: None,
        runtime_schema: None,
        execution_test: None,
    })
}

async fn execute_enable_flow(
    app_handle: &AppHandle,
    traffic_state: &TrafficAnalysisState,
    request: PluginAuthoringRequest,
) -> Result<PluginAuthoringResult> {
    let plugin_id = request
        .existing_plugin_id
        .or(request.plugin_id)
        .ok_or_else(|| anyhow!("plugin_id is required for enable"))?;

    enable_plugin_by_id(app_handle, traffic_state, &plugin_id).await?;

    Ok(PluginAuthoringResult {
        action: PluginAuthoringAction::Enable,
        plugin_id: Some(plugin_id.clone()),
        model: None,
        saved_as_draft: false,
        enabled: true,
        message: format!("插件 '{}' 已启用", plugin_id),
        code: None,
        validation: None,
        runtime_schema: None,
        execution_test: None,
    })
}

async fn generate_plugin_code(
    app_handle: &AppHandle,
    ai_manager: &Arc<AiServiceManager>,
    runtime: &Arc<SystemAgentRuntime>,
    request: &PluginAuthoringRequest,
    context: &PluginAuthoringContext,
) -> Result<(String, String)> {
    let agent_context = load_external_profile_context(runtime, PLUGIN_AUTHORING_PROFILE_ID, &[])
        .await
        .context("Failed to load plugin authoring profile")?;

    let system_prompt = get_combined_plugin_prompt_api(
        context.main_category.clone(),
        context.category.clone(),
        context.default_severity.clone(),
    )
    .map_err(|error| anyhow!(error))?;

    let virtual_tool_sections = build_virtual_tool_context(&agent_context, None).await?;
    let system_prompt = merge_external_profile_prompt(
        Some(if virtual_tool_sections.is_empty() {
            system_prompt
        } else {
            format!(
                "{}\n\nVirtual tool context:\n{}",
                system_prompt,
                virtual_tool_sections.join("\n\n")
            )
        }),
        &agent_context,
    );

    let user_prompt = build_generation_prompt(request, context).await?;
    let run_id = format!("plugin-authoring-{}", uuid::Uuid::new_v4());
    let response = crate::commands::ai_task_support::run_external_text_task(
        app_handle,
        ai_manager,
        PLUGIN_AUTHORING_PROFILE_ID,
        &run_id,
        &agent_context,
        system_prompt,
        user_prompt,
    )
    .await
    .context("Failed to generate plugin code")?;

    let (_, model) = crate::commands::ai_task_support::resolve_external_llm_config(
        ai_manager,
        Some(&agent_context),
        None,
    )
    .await?;

    let code = extract_and_clean_code(&response);
    if code.trim().is_empty() {
        return Err(anyhow!("Generated plugin code is empty"));
    }

    Ok((code, model))
}

async fn build_generation_prompt(
    request: &PluginAuthoringRequest,
    context: &PluginAuthoringContext,
) -> Result<String> {
    let requirements = required_text(&request.requirements, "requirements")?;
    let action_label = match request.action {
        PluginAuthoringAction::Generate => "generate",
        PluginAuthoringAction::Improve => "improve",
        _ => return Err(anyhow!("Unsupported action for generation prompt")),
    };

    let mut sections = vec![
        format!("Task: {action_label} a Sentinel plugin."),
        format!("main_category: {}", context.main_category),
        format!("category: {}", context.category),
        format!("plugin_id: {}", context.plugin_id),
        format!("name: {}", context.name),
        format!("default_severity: {}", context.default_severity),
        format!("description: {}", context.description),
        format!("requirements:\n{}", requirements),
    ];

    if let Some(existing_plugin_id) = request.existing_plugin_id.as_deref() {
        sections.push(format!("existing_plugin_id: {existing_plugin_id}"));
    }

    if let Some(samples) = &request.traffic_samples {
        sections.push(format!(
            "traffic_samples:\n{}",
            serde_json::to_string_pretty(samples)?
        ));
    }

    if let Some(existing_code) = &request.code {
        sections.push(format!(
            "existing_code:\n```typescript\n{}\n```",
            existing_code
        ));
    }

    sections.push(
        "Output only executable TypeScript code. Do not include explanations outside the code block."
            .to_string(),
    );

    Ok(sections.join("\n\n"))
}

async fn resolve_context_and_code(
    traffic_state: &TrafficAnalysisState,
    request: &PluginAuthoringRequest,
) -> Result<(PluginAuthoringContext, String)> {
    if let Some(code) = &request.code {
        let context = resolve_authoring_context(traffic_state, request).await?;
        return Ok((context, code.clone()));
    }

    let plugin_id = request
        .existing_plugin_id
        .as_deref()
        .or(request.plugin_id.as_deref())
        .ok_or_else(|| anyhow!("plugin_id is required when code is not provided"))?;

    let db = traffic_state.get_db_service();
    let plugin_record = db
        .get_plugin_from_registry(plugin_id)
        .await?
        .ok_or_else(|| anyhow!("Plugin not found: {plugin_id}"))?;
    let code = db
        .get_plugin_code(plugin_id)
        .await?
        .ok_or_else(|| anyhow!("Plugin code not found: {plugin_id}"))?;

    let default_severity = format!("{:?}", plugin_record.metadata.default_severity).to_lowercase();
    let context = PluginAuthoringContext {
        plugin_id: plugin_record.metadata.id.clone(),
        name: plugin_record.metadata.name.clone(),
        description: plugin_record
            .metadata
            .description
            .clone()
            .unwrap_or_else(|| plugin_record.metadata.name.clone()),
        author: plugin_record.metadata.author.clone(),
        main_category: plugin_record.metadata.main_category.clone(),
        category: plugin_record.metadata.category.clone(),
        monitor_type: plugin_record.metadata.monitor_type.clone(),
        default_severity,
    };

    Ok((context, code))
}

async fn resolve_authoring_context(
    traffic_state: &TrafficAnalysisState,
    request: &PluginAuthoringRequest,
) -> Result<PluginAuthoringContext> {
    let db = traffic_state.get_db_service();
    let existing = match request.existing_plugin_id.as_deref() {
        Some(plugin_id) => db.get_plugin_from_registry(plugin_id).await?,
        None => None,
    };

    let main_category = request
        .main_category
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            existing
                .as_ref()
                .map(|plugin| plugin.metadata.main_category.clone())
        })
        .ok_or_else(|| anyhow!("main_category is required"))?;

    let category = request
        .category
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            existing
                .as_ref()
                .map(|plugin| plugin.metadata.category.clone())
        })
        .ok_or_else(|| anyhow!("category is required"))?;

    let requirements = request
        .requirements
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let description = request
        .description
        .clone()
        .or(requirements.clone())
        .or_else(|| {
            existing
                .as_ref()
                .and_then(|plugin| plugin.metadata.description.clone())
        })
        .ok_or_else(|| anyhow!("description or requirements is required"))?;

    let default_severity = request
        .default_severity
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            existing
                .as_ref()
                .map(|plugin| format!("{:?}", plugin.metadata.default_severity).to_lowercase())
        })
        .unwrap_or_else(|| "medium".to_string());

    let requested_plugin_id = request
        .plugin_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    let plugin_id = match (
        &request.action,
        requested_plugin_id,
        request.existing_plugin_id.as_deref(),
    ) {
        (_, Some(plugin_id), _) => plugin_id,
        (PluginAuthoringAction::Improve, None, Some(existing_plugin_id)) => {
            format!("{}_draft", normalize_plugin_id(existing_plugin_id))
        }
        _ => derive_plugin_id(&description),
    };

    let name = request
        .name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| existing.as_ref().map(|plugin| plugin.metadata.name.clone()))
        .unwrap_or_else(|| derive_plugin_name(&description));

    let author = request
        .author
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            existing
                .as_ref()
                .and_then(|plugin| plugin.metadata.author.clone())
        });

    let monitor_type = request.monitor_type.clone().or_else(|| {
        resolved_store_plugin_monitor_type(existing.as_ref(), &plugin_id, &main_category, &category)
    });

    Ok(PluginAuthoringContext {
        plugin_id,
        name,
        description,
        author,
        main_category,
        category,
        monitor_type,
        default_severity,
    })
}

async fn hydrate_existing_plugin_code(
    traffic_state: &TrafficAnalysisState,
    mut request: PluginAuthoringRequest,
) -> Result<PluginAuthoringRequest> {
    if !matches!(request.action, PluginAuthoringAction::Improve) || request.code.is_some() {
        return Ok(request);
    }

    let Some(existing_plugin_id) = request.existing_plugin_id.as_deref() else {
        return Err(anyhow!("existing_plugin_id is required for improve"));
    };

    let db = traffic_state.get_db_service();
    request.code = Some(
        db.get_plugin_code(existing_plugin_id)
            .await?
            .ok_or_else(|| anyhow!("Plugin code not found: {existing_plugin_id}"))?,
    );

    Ok(request)
}

async fn validate_category_code(
    context: &PluginAuthoringContext,
    code: &str,
) -> Result<CategoryValidationResult> {
    match context.main_category.as_str() {
        "traffic" => {
            let validator = PluginValidator::new();
            let validation = validator.validate(code).await?;
            Ok(CategoryValidationResult {
                validation: Some(validation),
                runtime_schema: None,
            })
        }
        "agent" | "intruder" => {
            let metadata = RuntimeSchemaValidationMetadata {
                id: context.plugin_id.clone(),
                name: context.name.clone(),
                main_category: context.main_category.clone(),
                category: context.category.clone(),
                author: Some("Sentinel Plugin Authoring".to_string()),
                description: Some(context.description.clone()),
                default_severity: Some(context.default_severity.clone()),
                monitor_type: context.monitor_type.clone(),
            };

            let runtime_schema =
                crate::commands::plugin_review_commands::validate_plugin_runtime_schema(
                    code.to_string(),
                    metadata,
                )
                .await
                .map_err(|error| anyhow!(error))?;

            Ok(CategoryValidationResult {
                validation: None,
                runtime_schema: Some(runtime_schema),
            })
        }
        other => Err(anyhow!("Unsupported main_category: {other}")),
    }
}

async fn test_category_code(
    context: &PluginAuthoringContext,
    code: &str,
) -> Result<PluginExecutionTestResult> {
    let severity = parse_plugin_severity(&context.default_severity).map_err(anyhow::Error::msg)?;
    let metadata = build_plugin_metadata(
        context.plugin_id.clone(),
        context.name.clone(),
        context.main_category.clone(),
        context.category.clone(),
        Some(context.description.clone()),
        context.monitor_type.clone(),
        severity,
    );

    test_plugin_code(metadata, code.to_string(), None)
        .await
        .map_err(anyhow::Error::msg)
}

async fn save_plugin_draft(
    app_handle: &AppHandle,
    traffic_state: &TrafficAnalysisState,
    context: &PluginAuthoringContext,
    code: &str,
) -> Result<()> {
    let db = traffic_state.get_db_service();
    let existing = db.get_plugin_from_registry(&context.plugin_id).await?;

    if matches!(
        existing.as_ref().map(|plugin| plugin.status),
        Some(sentinel_plugins::PluginStatus::Enabled)
    ) {
        return Err(anyhow!(
            "Plugin '{}' is enabled. Save draft to a new plugin_id instead of mutating the active plugin.",
            context.plugin_id
        ));
    }

    let severity = parse_plugin_severity(&context.default_severity).map_err(anyhow::Error::msg)?;
    let metadata = sentinel_traffic::PluginMetadata {
        id: context.plugin_id.clone(),
        name: context.name.clone(),
        version: "1.0.0".to_string(),
        author: Some(
            context
                .author
                .clone()
                .or_else(|| {
                    existing
                        .as_ref()
                        .and_then(|plugin| plugin.metadata.author.clone())
                })
                .unwrap_or_else(|| "Sentinel Plugin Authoring".to_string()),
        ),
        main_category: context.main_category.clone(),
        category: context.category.clone(),
        description: Some(context.description.clone()),
        monitor_type: context.monitor_type.clone(),
        default_severity: match severity {
            sentinel_plugins::Severity::Critical => sentinel_traffic::Severity::Critical,
            sentinel_plugins::Severity::High => sentinel_traffic::Severity::High,
            sentinel_plugins::Severity::Medium => sentinel_traffic::Severity::Medium,
            sentinel_plugins::Severity::Low => sentinel_traffic::Severity::Low,
            sentinel_plugins::Severity::Info => sentinel_traffic::Severity::Info,
        },
        tags: vec!["ai-authored".to_string(), "draft".to_string()],
        target_asset_types: existing
            .as_ref()
            .map(|plugin| plugin.metadata.target_asset_types.clone())
            .unwrap_or_default(),
    };

    let traffic_plugin = TrafficPluginMetadata {
        id: metadata.id.clone(),
        name: metadata.name.clone(),
        version: metadata.version.clone(),
        author: metadata.author.clone(),
        main_category: metadata.main_category.clone(),
        category: metadata.category.clone(),
        description: metadata.description.clone(),
        default_severity: context.default_severity.clone(),
        tags: metadata.tags.clone(),
    };

    if existing.is_some() {
        db.update_traffic_plugin(&traffic_plugin, code).await?;
    } else {
        db.register_traffic_plugin_with_code(&traffic_plugin, code)
            .await?;
    }

    let metadata_json = serde_json::to_value(&metadata)?;
    db.update_plugin(&metadata_json, code).await?;
    db.update_plugin_enabled(&context.plugin_id, false).await?;

    emit_plugin_changed(
        app_handle,
        PluginChangedEvent {
            plugin_id: context.plugin_id.clone(),
            enabled: false,
            name: context.name.clone(),
        },
    );

    Ok(())
}

async fn enable_plugin_by_id(
    app_handle: &AppHandle,
    traffic_state: &TrafficAnalysisState,
    plugin_id: &str,
) -> Result<()> {
    let db = traffic_state.get_db_service();
    let (main_category, _) = db
        .get_plugin_summary(plugin_id)
        .await?
        .ok_or_else(|| anyhow!("Plugin not found: {plugin_id}"))?;

    db.update_plugin_enabled(plugin_id, true).await?;

    if main_category == "agent" {
        refresh_active_agent_plugin_tools(db.as_ref())
            .await
            .map_err(anyhow::Error::msg)?;
    }

    if main_category == "traffic" && *traffic_state.is_running.read().await {
        if let Some(scan_tx) = traffic_state.scan_tx.read().await.as_ref() {
            let _ = scan_tx.send(sentinel_traffic::ScanTask::ReloadPlugin(
                plugin_id.to_string(),
            ));
        }
    }

    let plugin_name = db
        .get_plugin_name(plugin_id)
        .await?
        .unwrap_or_else(|| plugin_id.to_string());

    emit_plugin_changed(
        app_handle,
        PluginChangedEvent {
            plugin_id: plugin_id.to_string(),
            enabled: true,
            name: plugin_name,
        },
    );

    Ok(())
}

async fn start_generation_run(
    runtime: &Arc<SystemAgentRuntime>,
    request: &PluginAuthoringRequest,
    context: &PluginAuthoringContext,
) -> Option<ExternalProfileRun> {
    start_external_profile_run(
        runtime,
        PLUGIN_AUTHORING_PROFILE_ID,
        serde_json::json!({
            "action": request.action,
            "pluginId": context.plugin_id,
            "mainCategory": context.main_category,
            "category": context.category,
            "enableAfterTest": request.enable_after_test.unwrap_or(false),
        }),
    )
    .await
}

fn required_text<'a>(value: &'a Option<String>, field: &str) -> Result<&'a str> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| anyhow!("{field} is required"))
}

fn derive_plugin_id(source: &str) -> String {
    let normalized = normalize_plugin_id(source);
    if normalized.is_empty() {
        format!("plugin_{}", uuid::Uuid::new_v4().simple())
    } else {
        normalized
    }
}

fn normalize_plugin_id(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut last_was_separator = false;

    for character in source.chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_lowercase());
            last_was_separator = false;
            continue;
        }

        if !last_was_separator {
            output.push('_');
            last_was_separator = true;
        }
    }

    output.trim_matches('_').to_string()
}

fn derive_plugin_name(source: &str) -> String {
    source
        .split_whitespace()
        .take(8)
        .collect::<Vec<_>>()
        .join(" ")
}

fn validation_allows_save(
    validation: &CategoryValidationResult,
    execution_test: Option<&PluginExecutionTestResult>,
) -> bool {
    let validation_ok = validation
        .validation
        .as_ref()
        .map(|result| result.is_valid)
        .unwrap_or(true)
        && validation
            .runtime_schema
            .as_ref()
            .map(|result| result.success)
            .unwrap_or(true);

    let execution_ok = execution_test.map(|result| result.success).unwrap_or(true);
    validation_ok && execution_ok
}

fn extract_and_clean_code(response: &str) -> String {
    extract_from_markdown(response)
        .or_else(|| extract_from_json(response))
        .unwrap_or_else(|| response.trim().to_string())
}

fn extract_from_markdown(text: &str) -> Option<String> {
    let patterns = ["```typescript\n", "```ts\n", "```\n"];

    for pattern in patterns {
        if let Some(start_position) = text.find(pattern) {
            let code_start = start_position + pattern.len();
            if let Some(end_position) = text[code_start..].find("\n```") {
                return Some(
                    text[code_start..code_start + end_position]
                        .trim()
                        .to_string(),
                );
            }
        }
    }

    None
}

fn extract_from_json(text: &str) -> Option<String> {
    let json = serde_json::from_str::<Value>(text).ok()?;
    for field in ["code", "plugin_code", "typescript", "content"] {
        if let Some(code) = json.get(field).and_then(Value::as_str) {
            return Some(code.to_string());
        }
    }
    None
}

struct CategoryValidationResult {
    validation: Option<ValidationResult>,
    runtime_schema: Option<RuntimeSchemaValidationResult>,
}
