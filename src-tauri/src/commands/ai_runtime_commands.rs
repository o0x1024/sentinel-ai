use crate::commands::ai::{
    create_cancellation_token, emit_agent_execution_finished, is_conversation_cancelled,
    perform_rag_enhancement, stream_chat_with_llm, AgentExecutionOutcome, CancellationGuard,
    MAX_SAFE_OUTPUT_STORAGE_THRESHOLD, USER_FORCED_RULES_CONFIG_CATEGORY,
    USER_FORCED_RULES_CONFIG_KEY,
};
use crate::commands::ai_task_support::{
    build_virtual_tool_context, complete_external_profile_run_failure,
    complete_external_profile_run_success, load_external_profile_context,
    merge_external_profile_prompt, run_external_text_task, start_external_profile_run,
};
use crate::commands::traffic::TrafficAnalysisState;
use crate::models::attachment::{load_image_from_path, MessageAttachment};
use crate::models::database::{SubagentMessage, SubagentRun};
use crate::services::ai::AiServiceManager;
use crate::services::database::DatabaseService;
use crate::services::SystemAgentRuntime;
use chrono::Utc;
use sentinel_db::Database;
use sentinel_workflow::WorkflowGraph;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HandleTaskExecutionStreamRequest {
    pub user_input: String,
    pub conversation_id: String,
    pub message_id: String,
    pub execution_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentExecuteConfig {
    pub conversation_id: Option<String>,
    pub message_id: Option<String>,
    pub enable_rag: Option<bool>,
    pub attachments: Option<serde_json::Value>,
    #[serde(default)]
    pub document_attachments:
        Option<Vec<crate::commands::document_commands::ProcessedDocumentResult>>,
    #[serde(default)]
    pub tool_config: Option<crate::agents::ToolConfig>,
    #[serde(default)]
    pub traffic_context: Option<String>,
    #[serde(default)]
    pub display_content: Option<String>,
    #[serde(default)]
    pub referenced_files: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub referenced_messages: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub referenced_assets: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub referenced_traffic: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub model_override: Option<String>,
    #[serde(default)]
    pub context_mode: Option<String>,
    #[serde(default)]
    pub max_iterations: Option<usize>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub force_todos: Option<bool>,
    #[serde(default)]
    pub enable_tenth_man_rule: Option<bool>,
    #[serde(default)]
    pub tenth_man_config: Option<crate::agents::tenth_man::TenthManConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentExecuteRequest {
    pub task: String,
    pub config: Option<AgentExecuteConfig>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EffectiveImageAttachmentMode {
    LocalOcr,
    ModelVision,
}

pub async fn get_subagent_runs(
    parent_execution_id: String,
    db_service: Arc<DatabaseService>,
) -> Result<Vec<SubagentRun>, String> {
    db_service
        .get_subagent_runs_by_parent_internal(&parent_execution_id)
        .await
        .map_err(|e| {
            format!(
                "Failed to get subagent runs for {}: {}",
                parent_execution_id, e
            )
        })
}

pub async fn get_subagent_messages(
    subagent_run_id: String,
    db_service: Arc<DatabaseService>,
) -> Result<Vec<SubagentMessage>, String> {
    db_service
        .get_subagent_messages_by_run_internal(&subagent_run_id)
        .await
        .map_err(|e| {
            format!(
                "Failed to get subagent messages for {}: {}",
                subagent_run_id, e
            )
        })
}

pub async fn delete_subagent_runs_after(
    parent_execution_id: String,
    after_timestamp_ms: i64,
    db_service: Arc<DatabaseService>,
) -> Result<u64, String> {
    use chrono::TimeZone;

    let timestamp = Utc
        .timestamp_millis_opt(after_timestamp_ms)
        .single()
        .ok_or_else(|| "Invalid timestamp".to_string())?;

    db_service
        .delete_subagent_runs_after_internal(&parent_execution_id, timestamp)
        .await
        .map_err(|e| format!("Failed to delete subagent runs: {}", e))
}

pub async fn clear_conversation_messages(
    conversation_id: String,
    db_service: Arc<DatabaseService>,
) -> Result<(), String> {
    db_service
        .delete_ai_messages_by_conversation(&conversation_id)
        .await
        .map_err(|e: anyhow::Error| {
            format!(
                "Failed to clear messages for conversation {}: {}",
                conversation_id, e
            )
        })?;

    if let Err(e) = db_service
        .delete_sliding_window_summaries(&conversation_id)
        .await
    {
        tracing::warn!(
            "Failed to clear sliding window summaries for {}: {}",
            conversation_id,
            e
        );
    }

    if let Err(e) = db_service.delete_agent_run_state(&conversation_id).await {
        tracing::warn!(
            "Failed to clear agent run_state for {}: {}",
            conversation_id,
            e
        );
    }

    Ok(())
}

pub async fn save_tool_config(
    tool_config: crate::agents::ToolConfig,
    app_handle: AppHandle,
) -> Result<(), String> {
    save_tool_config_to_db(&app_handle, &tool_config).await
}

pub async fn get_tool_config(
    app_handle: AppHandle,
) -> Result<Option<crate::agents::ToolConfig>, String> {
    Ok(load_tool_config_from_db(&app_handle).await)
}

pub async fn generate_workflow_from_nl(
    description: String,
    ai_manager: Arc<AiServiceManager>,
    traffic_state: &TrafficAnalysisState,
    system_agent_runtime: Arc<SystemAgentRuntime>,
) -> Result<WorkflowGraph, String> {
    let desc = description.trim();
    if desc.is_empty() {
        return Err("description is empty".to_string());
    }

    let agent_context = load_external_profile_context(
        &system_agent_runtime,
        "workflow_designer_agent",
        &["tool_catalog_reader", "workflow_catalog_reader"],
    )
    .await
    .map_err(|e| e.to_string())?;

    let virtual_tool_sections = build_virtual_tool_context(&agent_context, Some(traffic_state))
        .await
        .map_err(|e| e.to_string())?;
    let virtual_tool_context = if virtual_tool_sections.is_empty() {
        String::new()
    } else {
        format!("\n{}\n", virtual_tool_sections.join("\n\n"))
    };

    let tracked_run = start_external_profile_run(
        &system_agent_runtime,
        "workflow_designer_agent",
        serde_json::json!({
            "description": desc,
            "declaredTools": agent_context
                .tool_policy
                .as_ref()
                .map(|policy| policy.declared_tools())
                .unwrap_or_default(),
        }),
    )
    .await;

    let system_prompt = format!(
        r#"You are a workflow design assistant for Sentinel AI.
Based on the user's natural language description, output a WorkflowGraph that strictly conforms to the following JSON Schema.
Only output JSON, do not explain, do not include Markdown.

{}

Schema:
{{
  "id": "string",
  "name": "string",
  "version": "string",
  "nodes": [
    {{
      "id": "string",
      "node_type": "string",
      "node_name": "string",
      "x": number,
      "y": number,
      "params": {{...actual parameters for this node type...}},
      "input_ports": [{{"id":"in","name":"输入","port_type":"String","required":false}}],
      "output_ports": [{{"id":"out","name":"输出","port_type":"String","required":false}}]
    }}
  ],
  "edges": [
    {{
      "id":"string",
      "from_node":"string",
      "from_port":"out",
      "to_node":"string",
      "to_port":"in",
      "source_scope":"output",
      "source_path":"response",
      "target_path":"prompt",
      "merge_mode":"replace"
    }}
  ],
  "variables": [],
  "credentials": []
}}

CRITICAL RULES:
1) node_type selection:
   - Use "trigger_schedule" for scheduled/timed triggers
   - Use "tool::browser" for opening URLs and web scraping
   - Use "tool::http_request" for HTTP API calls
   - Use "ai_chat" for AI text generation/summarization
   - Use "notify" for sending notifications/emails
   - Use "raw" for static JSON/text input data
   - Use "start" for manual trigger entry point

2) params MUST contain actual values extracted from user description:
   - For "trigger_schedule": {{"trigger_type":"daily","hour":8,"minute":0,"second":0,"weekdays":"1,2,3,4,5"}}
   - For "tool::browser": {{"url":"https://example.com","action":"navigate","wait_until":"networkidle"}}
   - For "tool::http_request": {{"url":"https://api.example.com","method":"GET"}}
   - For "ai_chat": {{"prompt":"Summarize the following content: {{{{input}}}}","system_prompt":"You are a helpful assistant"}}
   - For "notify": {{"title":"Notification","content":"{{{{input}}}}","use_input_as_content":true}}
   - For "raw": {{"raw_type":"json","value":"{{\"query\":\"漏洞情报\"}}"}}

3) Extract specific values from user description:
   - Times like "8点" -> hour:8, minute:0
   - URLs mentioned -> put in url parameter
   - Email/notification requirements -> use notify node

4) Layout: x increases left-to-right (0, 250, 500...), y for parallel branches

5) Keep variables and credentials as empty arrays []

6) Every node MUST have meaningful params filled based on its purpose in the workflow
7) Every edge MUST include a non-empty target_path that maps data into the downstream node params
8) source_scope must be "output" or "input"; merge_mode must be "replace", "deep_merge", or "append"
"#,
        virtual_tool_context
    );
    let system_prompt =
        merge_external_profile_prompt(Some(system_prompt), &agent_context).unwrap_or_default();

    let user_prompt = format!("用户描述：{}\n请生成 WorkflowGraph JSON。", desc);
    let run_id = tracked_run
        .as_ref()
        .map(|run| run.run_id.as_str())
        .unwrap_or("workflow-designer-ad-hoc");
    let raw = match run_external_text_task(
        &system_agent_runtime.app_handle(),
        &ai_manager,
        "workflow_designer_agent",
        run_id,
        &agent_context,
        Some(system_prompt.clone()),
        user_prompt,
    )
    .await
    {
        Ok(raw) => raw,
        Err(e) => {
            complete_external_profile_run_failure(
                &system_agent_runtime,
                &tracked_run,
                e.to_string(),
            )
            .await;
            return Err(e.to_string());
        }
    };

    let json_str = raw.trim();
    let parsed_value: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(e0) => {
            if let (Some(s), Some(e)) = (json_str.find('{'), json_str.rfind('}')) {
                serde_json::from_str(&json_str[s..=e])
                    .map_err(|e| format!("Failed to parse extracted JSON: {}", e))?
            } else {
                let error = format!("Failed to parse LLM output as JSON: {}", e0);
                complete_external_profile_run_failure(&system_agent_runtime, &tracked_run, &error)
                    .await;
                return Err(error);
            }
        }
    };

    let mut graph: WorkflowGraph = match serde_json::from_value(parsed_value) {
        Ok(graph) => graph,
        Err(e) => {
            let error = format!("Failed to parse workflow graph: {}", e);
            complete_external_profile_run_failure(&system_agent_runtime, &tracked_run, &error)
                .await;
            return Err(error);
        }
    };

    if graph.id.trim().is_empty() {
        graph.id = format!("wf_{}", Utc::now().timestamp_millis());
    }
    if graph.name.trim().is_empty() {
        graph.name = "AI生成工作流".to_string();
    }
    if graph.version.trim().is_empty() {
        graph.version = "0.1.0".to_string();
    }
    if graph.variables.is_empty() {
        graph.variables = vec![];
    }
    if graph.credentials.is_empty() {
        graph.credentials = vec![];
    }

    complete_external_profile_run_success(
        &system_agent_runtime,
        &tracked_run,
        serde_json::json!({ "workflowGraph": &graph }),
    )
    .await;

    Ok(graph)
}

pub async fn save_scheduler_config(
    config: crate::services::ai::SchedulerConfig,
    db: Arc<DatabaseService>,
) -> Result<(), String> {
    tracing::info!("Saving scheduler configuration");

    db.set_config(
        "scheduler",
        "intent_analysis_model",
        &config.intent_analysis_model,
        Some("Intent analysis model for scheduler"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "intent_analysis_provider",
        &config.intent_analysis_provider,
        Some("Intent analysis provider for scheduler"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "planner_model",
        &config.planner_model,
        Some("Planner model for scheduler"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "planner_provider",
        &config.planner_provider,
        Some("Planner provider for scheduler"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "replanner_model",
        &config.replanner_model,
        Some("Replanner model for scheduler"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "replanner_provider",
        &config.replanner_provider,
        Some("Replanner provider for scheduler"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "executor_model",
        &config.executor_model,
        Some("Executor model for scheduler"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "executor_provider",
        &config.executor_provider,
        Some("Executor provider for scheduler"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "evaluator_model",
        &config.evaluator_model,
        Some("Evaluator model for scheduler"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "evaluator_provider",
        &config.evaluator_provider,
        Some("Evaluator provider for scheduler"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "default_strategy",
        &config.default_strategy,
        Some("Default replanning strategy"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "enabled",
        &config.enabled.to_string(),
        Some("Scheduler enabled status"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "max_retries",
        &config.max_retries.to_string(),
        Some("Maximum retry attempts"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "timeout_seconds",
        &config.timeout_seconds.to_string(),
        Some("Timeout in seconds"),
    )
    .await
    .map_err(|e| e.to_string())?;

    let scenarios_str = serde_json::to_string(&config.scenarios)
        .map_err(|e| format!("Failed to serialize scenarios: {}", e))?;
    db.set_config(
        "scheduler",
        "scenarios",
        &scenarios_str,
        Some("Scenario configurations"),
    )
    .await
    .map_err(|e| e.to_string())?;

    tracing::info!("Successfully saved scheduler configuration");
    Ok(())
}

pub async fn refresh_lm_studio_models(
    _api_base: Option<String>,
    _api_key: Option<String>,
) -> Result<Vec<String>, String> {
    Err("LM Studio model refresh disabled - ai_adapter removed, use Rig instead".to_string())
}

pub async fn get_lm_studio_status(
    _api_base: Option<String>,
    _api_key: Option<String>,
) -> Result<serde_json::Value, String> {
    Err("LM Studio status check disabled - ai_adapter removed, use Rig instead".to_string())
}

pub async fn test_lm_studio_provider_connection(
    _api_base: Option<String>,
    _api_key: Option<String>,
) -> Result<crate::commands::aisettings::TestConnectionResponse, String> {
    Err("LM Studio provider test disabled - ai_adapter removed, use Rig instead".to_string())
}

pub async fn upload_image_attachment(file_path: String) -> Result<serde_json::Value, String> {
    tracing::info!("上传图片附件: {}", file_path);

    match load_image_from_path(&file_path).await {
        Ok(image_attachment) => {
            let attachment = MessageAttachment::Image(image_attachment);
            serde_json::to_value(&attachment).map_err(|e| format!("序列化图片附件失败: {}", e))
        }
        Err(e) => {
            tracing::error!("加载图片失败: {}", e);
            Err(format!("加载图片失败: {}", e))
        }
    }
}

pub async fn upload_multiple_images(
    file_paths: Vec<String>,
) -> Result<Vec<serde_json::Value>, String> {
    tracing::info!("批量上传 {} 个图片", file_paths.len());

    let mut attachments = Vec::new();
    let mut errors = Vec::new();

    for file_path in file_paths {
        match load_image_from_path(&file_path).await {
            Ok(image_attachment) => {
                let attachment = MessageAttachment::Image(image_attachment);
                if let Ok(value) = serde_json::to_value(&attachment) {
                    attachments.push(value);
                } else {
                    errors.push(format!("序列化失败: {}", file_path));
                }
            }
            Err(e) => errors.push(format!("{}: {}", file_path, e)),
        }
    }

    if !errors.is_empty() {
        tracing::warn!("部分图片上传失败: {:?}", errors);
    }

    if attachments.is_empty() {
        Err(format!("所有图片上传失败: {:?}", errors))
    } else {
        Ok(attachments)
    }
}

pub async fn agent_execute(
    task: String,
    config: Option<AgentExecuteConfig>,
    app_handle: AppHandle,
    ai_manager: Arc<AiServiceManager>,
) -> Result<String, String> {
    #[cfg(not(debug_assertions))]
    if !sentinel_license::is_licensed() {
        return Err("License required for this feature".to_string());
    }

    let config = config.unwrap_or(AgentExecuteConfig {
        conversation_id: None,
        message_id: None,
        enable_rag: Some(false),
        attachments: None,
        document_attachments: None,
        tool_config: None,
        traffic_context: None,
        display_content: None,
        referenced_files: None,
        referenced_messages: None,
        referenced_assets: None,
        referenced_traffic: None,
        model_override: None,
        context_mode: None,
        max_iterations: None,
        timeout_secs: None,
        force_todos: None,
        enable_tenth_man_rule: None,
        tenth_man_config: None,
    });

    let conversation_id = config
        .conversation_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let message_id = config
        .message_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let enable_rag = config.enable_rag.unwrap_or(false);
    let raw_attachments = config.attachments.clone();
    let attachments_for_save = raw_attachments.as_ref().map(sanitize_image_attachments);
    let document_attachments_for_save = config.document_attachments.clone();
    let referenced_files_for_save = config.referenced_files.clone();
    let referenced_messages_for_save = config.referenced_messages.clone();
    let referenced_assets_for_save = config.referenced_assets.clone();
    let referenced_traffic_for_save = config.referenced_traffic.clone();

    let effective_tool_config = if config.tool_config.is_some() {
        tracing::info!("Using tool config from frontend request");
        config.tool_config.clone()
    } else {
        load_tool_config_from_db(&app_handle).await
    };

    tracing::info!(
        "Agent execute: conv={}, msg={}, rag={}, tools={}",
        conversation_id,
        message_id,
        enable_rag,
        effective_tool_config
            .as_ref()
            .map(|c| c.enabled)
            .unwrap_or(false)
    );

    if let Err(e) =
        crate::commands::ai_execution_state_support::clear_persisted_agent_execution_state(
            &app_handle,
            &conversation_id,
        )
        .await
    {
        tracing::warn!(
            "Failed to clear persisted execution state for conversation {}: {}",
            conversation_id,
            e
        );
    }

    let requested_override = config
        .model_override
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| {
            v.split_once('/')
                .map(|(p, m)| (p.trim().to_string(), m.trim().to_string()))
        });

    let (provider, model_name) = match requested_override {
        Some(Some((provider, model))) if !provider.is_empty() && !model.is_empty() => {
            tracing::info!("Using assistant model override: {}/{}", provider, model);
            (provider, model)
        }
        Some(_) => {
            return Err("Invalid model_override format, expected 'provider/model_name'".to_string())
        }
        None => match ai_manager.get_default_llm_model().await {
            Ok(Some((p, m))) => {
                tracing::info!("Using default chat model: {}/{}", p, m);
                (p, m)
            }
            Ok(None) => return Err("Default chat model is not configured".to_string()),
            Err(e) => return Err(format!("Failed to read default chat model: {}", e)),
        },
    };

    let provider_config = ai_manager
        .get_provider_config(&provider)
        .await
        .map_err(|e| format!("Failed to load provider config '{}': {}", provider, e))?
        .ok_or_else(|| format!("Provider '{}' configuration not found", provider))?;

    let mut dynamic_config = provider_config.clone();
    dynamic_config.model = model_name.clone();

    let db_service = app_handle.state::<Arc<DatabaseService>>();
    let (image_mode, allow_image_upload_to_model) =
        load_image_attachment_settings(db_service.inner()).await;
    let mut service = crate::services::ai::AiService::new(
        dynamic_config,
        db_service.inner().clone(),
        Some(app_handle.clone()),
    );
    service.set_app_handle(app_handle.clone());

    if let Ok(threshold_str_opt) = db_service
        .get_config_internal("ai", "output_storage_threshold")
        .await
    {
        if let Some(threshold_str) = threshold_str_opt {
            if let Ok(threshold) = threshold_str.parse::<usize>() {
                let effective_threshold = threshold.min(MAX_SAFE_OUTPUT_STORAGE_THRESHOLD);
                if effective_threshold != threshold {
                    tracing::warn!(
                        "Configured output storage threshold {} is too high, clamped to {} bytes for stream stability",
                        threshold,
                        effective_threshold
                    );
                } else {
                    tracing::info!(
                        "Setting output storage threshold to {} bytes (Dynamic Context Discovery)",
                        effective_threshold
                    );
                }
                sentinel_tools::set_storage_threshold(effective_threshold);
            }
        }
    }

    let (_cancellation_token, cancel_gen) = create_cancellation_token(&conversation_id);

    let service_clone = service.clone();
    let conv_id = conversation_id.clone();
    let msg_id = message_id.clone();
    let task_clone = task.clone();
    let display_content_clone = config.display_content.clone();
    let mut base_system_prompt: Option<String> = None;

    let provider_for_closure = if provider_config.rig_provider.is_some() {
        provider_config.rig_provider.clone().unwrap()
    } else {
        provider_config.provider.clone()
    };

    let model_name_for_closure = model_name.clone();
    let provider_config_for_closure = provider_config.clone();

    tokio::spawn(async move {
        let _guard = CancellationGuard(conv_id.clone(), cancel_gen);
        if let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() {
            let conversation_exists = db
                .get_ai_conversation(&conv_id)
                .await
                .map(|c| c.is_some())
                .unwrap_or(false);

            if !conversation_exists {
                use sentinel_core::models::database as core_db;
                let new_conv = core_db::AiConversation {
                    id: conv_id.clone(),
                    title: Some(task_clone.chars().take(50).collect::<String>()),
                    service_name: "default".to_string(),
                    model_name: "default".to_string(),
                    model_provider: None,
                    context_type: None,
                    project_id: None,
                    vulnerability_id: None,
                    scan_task_id: None,
                    conversation_data: None,
                    summary: None,
                    total_messages: 0,
                    total_tokens: 0,
                    cost: 0.0,
                    tags: None,
                    tool_config: None,
                    is_archived: false,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                };
                if let Err(e) = db.create_ai_conversation(&new_conv).await {
                    tracing::warn!("Failed to create conversation: {}", e);
                }
            }

            use sentinel_core::models::database as core_db;
            let user_msg_id = Uuid::new_v4().to_string();
            let display_text = display_content_clone.as_ref().unwrap_or(&task_clone);

            let structured_data = {
                let mut data = serde_json::json!({});
                if let Some(ref content) = display_content_clone {
                    data["display_content"] = serde_json::json!(content);
                }
                if let Some(ref doc_atts) = document_attachments_for_save {
                    if !doc_atts.is_empty() {
                        data["document_attachments"] =
                            serde_json::to_value(doc_atts).unwrap_or_default();
                    }
                }
                if let Some(ref files) = referenced_files_for_save {
                    if !files.is_empty() {
                        data["referenced_files"] = serde_json::json!(files);
                    }
                }
                if let Some(ref messages) = referenced_messages_for_save {
                    if !messages.is_empty() {
                        data["referenced_messages"] = serde_json::json!(messages);
                    }
                }
                if let Some(ref assets) = referenced_assets_for_save {
                    if !assets.is_empty() {
                        data["referenced_assets"] = serde_json::json!(assets);
                    }
                }
                if let Some(ref traffic) = referenced_traffic_for_save {
                    if !traffic.is_empty() {
                        data["referenced_traffic"] = serde_json::json!(traffic);
                    }
                }
                if data.as_object().map(|o| o.is_empty()).unwrap_or(true) {
                    None
                } else {
                    Some(data.to_string())
                }
            };

            let metadata = {
                let mut meta = serde_json::json!({});
                if let Some(ref atts) = attachments_for_save {
                    meta["image_attachments"] = atts.clone();
                }
                if let Some(ref doc_atts) = document_attachments_for_save {
                    if !doc_atts.is_empty() {
                        meta["document_attachments"] =
                            serde_json::to_value(doc_atts).unwrap_or_default();
                    }
                }
                if let Some(ref files) = referenced_files_for_save {
                    if !files.is_empty() {
                        meta["referenced_files"] = serde_json::json!(files);
                    }
                }
                if let Some(ref messages) = referenced_messages_for_save {
                    if !messages.is_empty() {
                        meta["referenced_messages"] = serde_json::json!(messages);
                    }
                }
                if let Some(ref assets) = referenced_assets_for_save {
                    if !assets.is_empty() {
                        meta["referenced_assets"] = serde_json::json!(assets);
                    }
                }
                if let Some(ref traffic) = referenced_traffic_for_save {
                    if !traffic.is_empty() {
                        meta["referenced_traffic"] = serde_json::json!(traffic);
                    }
                }
                if meta.as_object().map(|o| o.is_empty()).unwrap_or(true) {
                    None
                } else {
                    Some(meta.to_string())
                }
            };

            let user_msg = core_db::AiMessage {
                id: user_msg_id.clone(),
                conversation_id: conv_id.clone(),
                role: "user".to_string(),
                content: task_clone.clone(),
                metadata,
                token_count: Some(task_clone.len() as i32),
                cost: None,
                tool_calls: None,
                attachments: attachments_for_save
                    .as_ref()
                    .and_then(|v| serde_json::to_string(v).ok()),
                reasoning_content: None,
                timestamp: chrono::Utc::now(),
                architecture_type: None,
                architecture_meta: None,
                structured_data,
            };
            if let Err(e) = db.create_ai_message(&user_msg).await {
                tracing::warn!("Failed to save user message: {}", e);
            } else {
                let _ = app_handle.emit(
                    "agent:user_message",
                    &serde_json::json!({
                        "execution_id": conv_id,
                        "message_id": user_msg_id,
                        "content": display_text,
                        "timestamp": user_msg.timestamp.timestamp_millis(),
                        "document_attachments": document_attachments_for_save,
                        "image_attachments": attachments_for_save,
                        "referenced_files": referenced_files_for_save,
                        "referenced_messages": referenced_messages_for_save,
                        "referenced_assets": referenced_assets_for_save,
                        "referenced_traffic": referenced_traffic_for_save,
                    }),
                );
            }
        }

        let mut role_prompt = String::new();
        if let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() {
            if let Ok(Some(current_role)) = db.get_current_ai_role().await {
                if !current_role.prompt.trim().is_empty() {
                    role_prompt = current_role.prompt;
                    tracing::info!("Using role prompt: {}", current_role.title);
                }
            }
        }

        if !role_prompt.is_empty() {
            base_system_prompt = match base_system_prompt {
                Some(existing) if !existing.trim().is_empty() => {
                    Some(format!("{}\n\n{}", role_prompt, existing))
                }
                _ => Some(role_prompt),
            };
        }

        if let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() {
            if let Ok(Some(raw_rules)) = db
                .get_config(
                    USER_FORCED_RULES_CONFIG_CATEGORY,
                    USER_FORCED_RULES_CONFIG_KEY,
                )
                .await
            {
                let forced_rules = raw_rules.trim();
                if !forced_rules.is_empty() {
                    let rules_block = format!(
                        "[User Forced Rules]\n{}\n\n[Rule Priority]\n- The above rules are user-defined mandatory instructions. Follow them unless they conflict with higher-priority safety/system constraints.",
                        forced_rules
                    );
                    base_system_prompt = match base_system_prompt {
                        Some(existing) if !existing.trim().is_empty() => {
                            Some(format!("{}\n\n{}", existing, rules_block))
                        }
                        _ => Some(rules_block),
                    };
                    tracing::info!("Injected user forced rules into system prompt");
                }
            }
        }

        let mut augmented_task = task_clone.clone();

        if let Some(ref raw) = raw_attachments {
            let shell_cfg = sentinel_tools::buildin_tools::shell::get_shell_config().await;
            if shell_cfg.default_execution_mode
                == sentinel_tools::buildin_tools::shell::ShellExecutionMode::Docker
            {
                match crate::utils::image_ocr::stage_images_to_docker_context(raw).await {
                    Ok(paths) => {
                        if !paths.is_empty() {
                            let mut lines = Vec::new();
                            for p in paths {
                                let name = p
                                    .filename
                                    .as_deref()
                                    .filter(|s| !s.trim().is_empty())
                                    .unwrap_or("image");
                                lines.push(format!("- {}: {}", name, p.container_path));
                            }
                            augmented_task = format!(
                                "[Image Files in Docker]\n{}\n\n{}",
                                lines.join("\n"),
                                augmented_task
                            );
                        }
                    }
                    Err(e) => tracing::warn!("Failed to stage images to docker context: {}", e),
                }
            }
        }

        let effective_mode = if image_mode == EffectiveImageAttachmentMode::ModelVision
            && allow_image_upload_to_model
        {
            EffectiveImageAttachmentMode::ModelVision
        } else {
            EffectiveImageAttachmentMode::LocalOcr
        };

        let image_attachments_for_execution: Option<serde_json::Value> = match effective_mode {
            EffectiveImageAttachmentMode::LocalOcr => {
                if let Some(ref raw) = raw_attachments {
                    match crate::utils::image_ocr::ocr_images_from_attachments(raw).await {
                        Ok(results) => {
                            let ctx = crate::utils::image_ocr::format_ocr_context(&results, 8000);
                            if !ctx.trim().is_empty() {
                                augmented_task =
                                    format!("[Image OCR]\n{}\n\n{}", ctx, augmented_task);
                            }
                        }
                        Err(e) => tracing::warn!("Image OCR failed: {}", e),
                    }
                }
                None
            }
            EffectiveImageAttachmentMode::ModelVision => attachments_for_save.clone(),
        };

        if enable_rag {
            if let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() {
                let history_messages = match db.get_ai_messages_by_conversation(&conv_id).await {
                    Ok(msgs) => msgs,
                    Err(_) => Vec::new(),
                };

                let rag_query = display_content_clone.as_ref().unwrap_or(&task_clone);
                match perform_rag_enhancement(
                    &app_handle,
                    &conv_id,
                    rag_query,
                    &history_messages,
                    None,
                )
                .await
                {
                    Ok((context, citations)) => {
                        if !context.trim().is_empty() {
                            let base = base_system_prompt.unwrap_or_default();
                            let policy = "you must strictly answer the question based on the evidence. When citing evidence in your response, use the [SOURCE n] format. If the evidence is insufficient, please answer directly and avoid fabricating. ";
                            let augmented = if base.trim().is_empty() {
                                format!(
                                    "[rule of knowledge]\n{}\n\n[Source Evidence Block]\n{}",
                                    policy, context
                                )
                            } else {
                                format!(
                                    "{}\n\n[rule of knowledge]\n{}\n\n[Source Evidence Block]\n{}",
                                    base, policy, context
                                )
                            };
                            base_system_prompt = Some(augmented);

                            let _ = app_handle.emit(
                                "ai_meta_info",
                                &serde_json::json!({
                                    "conversation_id": conv_id,
                                    "message_id": msg_id,
                                    "rag_applied": true,
                                    "rag_sources_used": !citations.is_empty(),
                                    "source_count": citations.len(),
                                    "citations": citations
                                }),
                            );
                        }
                    }
                    Err(e) => tracing::warn!("RAG enhancement failed in agent_execute: {}", e),
                }
            }
        }

        if let Err(e) = app_handle.emit(
            "ai_stream_start",
            &serde_json::json!({
                "conversation_id": conv_id,
                "message_id": msg_id
            }),
        ) {
            tracing::error!("Failed to emit stream start event: {}", e);
        }

        if let Some(ref tool_cfg) = effective_tool_config {
            if tool_cfg.enabled {
                tracing::info!(
                    "Using tool-enabled agent executor for conversation: {}",
                    conv_id
                );

                let doc_attachments = if let Some(docs) = config.document_attachments.as_ref() {
                    let mut resolved = Vec::with_capacity(docs.len());
                    for d in docs {
                        let runtime_path = match crate::commands::document_commands::resolve_uploaded_file_for_execution_by_id(
                            &app_handle,
                            &d.file_id,
                        )
                        .await {
                            Ok(path) => path,
                            Err(e) => {
                                emit_agent_execution_finished(
                                    &app_handle,
                                    &conv_id,
                                    AgentExecutionOutcome::Failed,
                                    Some(format!(
                                        "Failed to resolve uploaded file {}: {}",
                                        d.file_id, e
                                    )),
                                    None,
                                    None,
                                );
                                return;
                            }
                        };
                        resolved.push(crate::agents::DocumentAttachmentInfo {
                            id: d.file_id.clone(),
                            original_filename: d.original_filename.clone(),
                            file_size: d.file_size,
                            mime_type: d.mime_type.clone(),
                            file_path: Some(runtime_path),
                        });
                    }
                    Some(resolved)
                } else {
                    None
                };

                let executor_params = crate::agents::executor::AgentExecuteParams {
                    execution_id: conv_id.clone(),
                    model: model_name_for_closure.clone(),
                    system_prompt: base_system_prompt.unwrap_or_default(),
                    task: augmented_task.clone(),
                    rig_provider: provider_for_closure.clone(),
                    api_key: provider_config_for_closure.api_key.clone(),
                    api_base: provider_config_for_closure.api_base.clone(),
                    max_iterations: config
                        .max_iterations
                        .unwrap_or(provider_config.max_turns.unwrap_or(50))
                        .max(1),
                    timeout_secs: config.timeout_secs.unwrap_or(300),
                    tool_config: effective_tool_config.clone(),
                    enable_tenth_man_rule: config.enable_tenth_man_rule.unwrap_or(false),
                    tenth_man_config: config.tenth_man_config.clone(),
                    document_attachments: doc_attachments,
                    image_attachments: image_attachments_for_execution.clone(),
                    persist_messages: true,
                    subagent_run_id: None,
                    context_policy: None,
                    context_engine_mode: config
                        .context_mode
                        .as_deref()
                        .and_then(crate::agents::ContextEngineMode::from_str),
                    recursion_depth: 0,
                };

                match crate::agents::executor::execute_agent(&app_handle, executor_params).await {
                    Ok(response) => {
                        if is_conversation_cancelled(&conv_id) {
                            tracing::info!(
                                "Agent execution ended after cancellation for conversation: {}",
                                conv_id
                            );
                            return;
                        }
                        tracing::info!("Agent with tools completed for conversation: {}", conv_id);
                        emit_agent_execution_finished(
                            &app_handle,
                            &conv_id,
                            AgentExecutionOutcome::Succeeded,
                            None,
                            Some(response),
                            None,
                        );
                    }
                    Err(e) => {
                        if is_conversation_cancelled(&conv_id) {
                            tracing::info!(
                                "Agent execution failed after cancellation for conversation: {}",
                                conv_id
                            );
                            return;
                        }
                        tracing::error!("Agent with tools execution failed: {}", e);
                        emit_agent_execution_finished(
                            &app_handle,
                            &conv_id,
                            AgentExecutionOutcome::Failed,
                            Some(e.to_string()),
                            None,
                            None,
                        );
                    }
                }

                return;
            }
        }

        match stream_chat_with_llm(
            &service_clone,
            &app_handle,
            &conv_id,
            &msg_id,
            &augmented_task,
            base_system_prompt.as_deref(),
            image_attachments_for_execution,
            true,
        )
        .await
        {
            Ok(_) => tracing::info!("Stream chat completed for conversation: {}", conv_id),
            Err(e) => tracing::error!("Stream chat failed: {}", e),
        }
    });

    Ok(message_id)
}

fn sanitize_image_attachments(attachments: &serde_json::Value) -> serde_json::Value {
    fn sanitize_one(v: &mut serde_json::Value) {
        let img = if v.get("type").and_then(|t| t.as_str()) == Some("image") {
            Some(v)
        } else {
            v.get_mut("image")
        };
        let Some(img) = img else { return };
        if let Some(obj) = img.as_object_mut() {
            obj.remove("source_path");
        }
    }

    let mut cloned = attachments.clone();
    if let Some(arr) = cloned.as_array_mut() {
        for item in arr.iter_mut() {
            sanitize_one(item);
        }
    } else if cloned.is_object() {
        sanitize_one(&mut cloned);
    }
    cloned
}

async fn load_image_attachment_settings(
    db: &DatabaseService,
) -> (EffectiveImageAttachmentMode, bool) {
    let mode_str = db
        .get_config("agent", "image_attachment_mode")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "local_ocr".to_string());
    let mode = if mode_str == "model_vision" {
        EffectiveImageAttachmentMode::ModelVision
    } else {
        EffectiveImageAttachmentMode::LocalOcr
    };

    let allow_upload = db
        .get_config("agent", "allow_image_upload_to_model")
        .await
        .ok()
        .flatten()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    (mode, allow_upload)
}

async fn load_tool_config_from_db(app_handle: &AppHandle) -> Option<crate::agents::ToolConfig> {
    if let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() {
        if let Ok(Some(config_str)) = db.get_config("agent", "tool_config").await {
            if let Ok(config) = serde_json::from_str::<crate::agents::ToolConfig>(&config_str) {
                tracing::info!("Loaded global tool config from database");
                return Some(config);
            }
        }
    }

    tracing::info!("No global tool config found, using default");
    None
}

async fn save_tool_config_to_db(
    app_handle: &AppHandle,
    config: &crate::agents::ToolConfig,
) -> Result<(), String> {
    if let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() {
        let config_str = serde_json::to_string(config)
            .map_err(|e| format!("Failed to serialize tool config: {}", e))?;

        db.set_config(
            "agent",
            "tool_config",
            &config_str,
            Some("Global tool configuration"),
        )
        .await
        .map_err(|e| format!("Failed to save tool config: {}", e))?;

        tracing::info!("Saved global tool config to database");
        Ok(())
    } else {
        Err("Database service not available".to_string())
    }
}
