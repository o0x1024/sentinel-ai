use super::config::{WeixinGatewayConfig, WeixinGatewayStatus};
use super::ilink_client::WeixinIlinkClient;
use crate::agents::{AgentExecuteParams, ContextEngineMode, ToolConfig, ToolSelectionStrategy};
use crate::commands::ai::cancel_conversation_stream;
use crate::commands::assistant_profile_commands::{
    load_assistant_profile_by_id_or_default, AssistantProfilePayload,
};
use crate::models::database::{AiConversation, AiMessage, BotMessage};
use crate::services::ai::AiServiceManager;
use crate::services::bot_execution::{execute_bot_execution, BotExecutionRequest};
use crate::services::database::DatabaseService;
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use sentinel_db::Database;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tauri::AppHandle;
use tokio::sync::{oneshot, RwLock};
use tokio::task::JoinHandle;
use uuid::Uuid;

struct WeixinRuntimeState {
    runtime: Option<WeixinGatewayRuntime>,
    last_error: Option<String>,
    last_message_at: Option<DateTime<Utc>>,
}

impl Default for WeixinRuntimeState {
    fn default() -> Self {
        Self {
            runtime: None,
            last_error: None,
            last_message_at: None,
        }
    }
}

struct WeixinGatewayRuntime {
    account_id: String,
    started_at: DateTime<Utc>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    task: JoinHandle<()>,
}

struct WeixinTaskExecutionOutcome {
    run_id: String,
    result: Result<String, String>,
}

static WEIXIN_RUNTIME: Lazy<Arc<RwLock<WeixinRuntimeState>>> =
    Lazy::new(|| Arc::new(RwLock::new(WeixinRuntimeState::default())));
static ACTIVE_WEIXIN_EXECUTIONS: Lazy<Arc<RwLock<HashMap<String, String>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

pub async fn start_weixin_gateway_runtime(
    mut config: WeixinGatewayConfig,
    db: Arc<DatabaseService>,
    ai_manager: Arc<AiServiceManager>,
    app_handle: AppHandle,
) -> Result<(), String> {
    config.normalize();
    config.validate_for_runtime()?;

    let mut state = WEIXIN_RUNTIME.write().await;
    if state
        .runtime
        .as_ref()
        .map(|runtime| runtime.task.is_finished())
        .unwrap_or(false)
    {
        state.runtime = None;
    }
    if state.runtime.is_some() {
        return Err("Weixin gateway is already running".to_string());
    }

    let client = WeixinIlinkClient::new()?;
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let account_id = config.account_id.clone();
    let task = tokio::spawn(run_loop(
        config,
        client,
        db,
        ai_manager,
        app_handle,
        shutdown_rx,
    ));

    state.runtime = Some(WeixinGatewayRuntime {
        account_id,
        started_at: Utc::now(),
        shutdown_tx: Some(shutdown_tx),
        task,
    });
    state.last_error = None;
    Ok(())
}

pub async fn stop_weixin_gateway_runtime() -> Result<(), String> {
    let runtime = {
        let mut state = WEIXIN_RUNTIME.write().await;
        state.runtime.take()
    };
    if let Some(mut runtime) = runtime {
        if let Some(tx) = runtime.shutdown_tx.take() {
            let _ = tx.send(());
        }
        let _ = tokio::time::timeout(Duration::from_secs(3), runtime.task).await;
    }
    Ok(())
}

pub async fn weixin_gateway_status() -> WeixinGatewayStatus {
    let state = WEIXIN_RUNTIME.read().await;
    if let Some(runtime) = state.runtime.as_ref() {
        return WeixinGatewayStatus {
            running: !runtime.task.is_finished(),
            account_id: Some(runtime.account_id.clone()),
            started_at: Some(runtime.started_at),
            last_error: state.last_error.clone(),
            last_message_at: state.last_message_at,
        };
    }
    WeixinGatewayStatus {
        running: false,
        account_id: None,
        started_at: None,
        last_error: state.last_error.clone(),
        last_message_at: state.last_message_at,
    }
}

async fn register_active_execution(lock_key: &str, execution_id: &str) -> Result<(), String> {
    let mut active = ACTIVE_WEIXIN_EXECUTIONS.write().await;
    if let Some(existing) = active.get(lock_key) {
        return Err(format!(
            "Another Weixin execution is already running for this chat: {}",
            existing
        ));
    }
    active.insert(lock_key.to_string(), execution_id.to_string());
    Ok(())
}

async fn clear_active_execution(lock_key: &str, execution_id: &str) {
    let mut active = ACTIVE_WEIXIN_EXECUTIONS.write().await;
    if active.get(lock_key).map(String::as_str) == Some(execution_id) {
        active.remove(lock_key);
    }
}

async fn active_execution_id(lock_key: &str) -> Option<String> {
    ACTIVE_WEIXIN_EXECUTIONS.read().await.get(lock_key).cloned()
}

async fn run_loop(
    config: WeixinGatewayConfig,
    client: WeixinIlinkClient,
    db: Arc<DatabaseService>,
    ai_manager: Arc<AiServiceManager>,
    app_handle: AppHandle,
    mut shutdown_rx: oneshot::Receiver<()>,
) {
    let mut sync_buf = String::new();
    let mut seen_messages = HashSet::new();
    let mut context_tokens: HashMap<String, String> = HashMap::new();

    loop {
        tokio::select! {
            _ = &mut shutdown_rx => break,
            result = client.get_updates(&config.base_url, &config.token, &sync_buf) => {
                match result {
                    Ok(payload) => {
                        if let Some(next_buf) = payload.get("get_updates_buf").and_then(Value::as_str) {
                            sync_buf = next_buf.to_string();
                        }
                        if let Err(error) = process_updates(
                            &config,
                            &client,
                            &db,
                            &ai_manager,
                            &app_handle,
                            &mut seen_messages,
                            &mut context_tokens,
                            &payload,
                        )
                        .await
                        {
                            set_runtime_error(error).await;
                            tokio::time::sleep(Duration::from_secs(2)).await;
                        }
                    }
                    Err(error) => {
                        set_runtime_error(error).await;
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        }
    }
}

async fn process_updates(
    config: &WeixinGatewayConfig,
    client: &WeixinIlinkClient,
    db: &Arc<DatabaseService>,
    ai_manager: &Arc<AiServiceManager>,
    app_handle: &AppHandle,
    seen_messages: &mut HashSet<String>,
    context_tokens: &mut HashMap<String, String>,
    payload: &Value,
) -> Result<(), String> {
    let ret = payload.get("ret").and_then(Value::as_i64).unwrap_or(0);
    let errcode = payload.get("errcode").and_then(Value::as_i64).unwrap_or(0);
    if ret != 0 || errcode != 0 {
        return Err(format!(
            "Weixin getupdates returned ret={ret} errcode={errcode}"
        ));
    }

    let Some(messages) = payload.get("msgs").and_then(Value::as_array) else {
        return Ok(());
    };

    for message in messages {
        if is_self_message(config, message) {
            continue;
        }
        let message_id = message_identity(message);
        if !seen_messages.insert(message_id.clone()) {
            continue;
        }
        trim_seen_messages(seen_messages);

        let inbound = match parse_inbound_message(config, message, message_id.clone()) {
            Some(value) => value,
            None => continue,
        };
        if !is_authorized(config, &inbound) {
            continue;
        }
        if let Some(token) = inbound.context_token.as_deref() {
            context_tokens.insert(inbound.peer_id.clone(), token.to_string());
        }
        mark_message_seen().await;

        handle_inbound_message(
            config,
            client,
            db,
            ai_manager,
            app_handle,
            context_tokens,
            inbound,
        )
        .await?;
    }
    Ok(())
}

async fn handle_inbound_message(
    config: &WeixinGatewayConfig,
    client: &WeixinIlinkClient,
    db: &Arc<DatabaseService>,
    ai_manager: &Arc<AiServiceManager>,
    app_handle: &AppHandle,
    context_tokens: &HashMap<String, String>,
    inbound: InboundWeixinMessage,
) -> Result<(), String> {
    let context_token = context_tokens.get(&inbound.peer_id).map(String::as_str);
    let inbound_bot_message_id = persist_inbound_bot_message(config, db, &inbound).await?;
    match inbound.text.trim() {
        "/status" => {
            send_text_chunks(
                db,
                client,
                config,
                &inbound.peer_type,
                &inbound.peer_id,
                "Sentinel Weixin remote control is running.",
                context_token,
                None,
            )
            .await
        }
        "/stop" => {
            let session_id = session_id(config, &inbound);
            let Some(execution_id) = active_execution_id(&session_id).await else {
                return send_text_chunks(
                    db,
                    client,
                    config,
                    &inbound.peer_type,
                    &inbound.peer_id,
                    "No active execution.",
                    context_token,
                    None,
                )
                .await;
            };
            cancel_conversation_stream(&execution_id);
            let _ = crate::managers::cancellation_manager::cancel_execution(&execution_id).await;
            let _ = sentinel_tools::buildin_tools::shell::cancel_shell_execution(&execution_id).await;
            if let Err(error) = sentinel_tools::buildin_tools::shell_background::stop_background_shell_tasks_for_execution(&execution_id).await {
                tracing::warn!(
                    "Failed to stop background shell tasks for {}: {}",
                    execution_id,
                    error
                );
            }
            send_text_chunks(
                db,
                client,
                config,
                &inbound.peer_type,
                &inbound.peer_id,
                "Stop signal sent.",
                context_token,
                None,
            )
            .await
        }
        "/permissions" => {
            let session_id = session_id(config, &inbound);
            let Some(execution_id) = active_execution_id(&session_id).await else {
                return send_text_chunks(
                    db,
                    client,
                    config,
                    &inbound.peer_type,
                    &inbound.peer_id,
                    "No active execution.",
                    context_token,
                    None,
                )
                .await;
            };
            let pending = crate::commands::tool_commands::get_pending_shell_permissions()
                .await
                .unwrap_or_default()
                .into_iter()
                .filter(|item| item.execution_id.as_deref() == Some(execution_id.as_str()))
                .map(|item| format!("{} {}", item.id, item.command))
                .collect::<Vec<_>>();
            let text = if pending.is_empty() {
                "No pending shell permissions.".to_string()
            } else {
                format!("Pending shell permissions:\n{}", pending.join("\n"))
            };
            send_text_chunks(
                db,
                client,
                config,
                &inbound.peer_type,
                &inbound.peer_id,
                &text,
                context_token,
                None,
            )
            .await
        }
        text if text.starts_with("/approve ") => {
            let id = text.trim_start_matches("/approve ").trim();
            let result = crate::commands::tool_commands::respond_shell_permission(
                id.to_string(),
                true,
            )
            .await;
            let text = match result {
                Ok(_) => format!("Approved permission {id}."),
                Err(error) => format!("Approve failed: {error}"),
            };
            send_text_chunks(
                db,
                client,
                config,
                &inbound.peer_type,
                &inbound.peer_id,
                &text,
                context_token,
                None,
            )
            .await
        }
        text if text.starts_with("/deny ") => {
            let id = text.trim_start_matches("/deny ").trim();
            let result = crate::commands::tool_commands::respond_shell_permission(
                id.to_string(),
                false,
            )
            .await;
            let text = match result {
                Ok(_) => format!("Denied permission {id}."),
                Err(error) => format!("Deny failed: {error}"),
            };
            send_text_chunks(
                db,
                client,
                config,
                &inbound.peer_type,
                &inbound.peer_id,
                &text,
                context_token,
                None,
            )
            .await
        }
        text if text.starts_with('/') => {
            send_text_chunks(
                db,
                client,
                config,
                &inbound.peer_type,
                &inbound.peer_id,
                "Unsupported command. Available: /status, /stop, /permissions, /approve <id>, /deny <id>.",
                context_token,
                None,
            )
            .await
        }
        _ => {
            let peer_lock_key = execution_lock_key_for_inbound(config, &inbound);
            let conversation_id = session_id(config, &inbound);
            if active_execution_id(&peer_lock_key).await.is_some() {
                return send_text_chunks(
                    db,
                    client,
                    config,
                    &inbound.peer_type,
                    &inbound.peer_id,
                    "Task failed: another execution is already running for this chat.",
                    context_token,
                    None,
                )
                .await;
            }
            ensure_conversation_exists(config, db, "default", "default", &conversation_id).await?;
            let trigger_ai_message_id =
                persist_inbound_user_message(db, config, &inbound, &conversation_id).await?;
            let outcome = match execute_remote_task(
                config,
                db,
                ai_manager,
                app_handle,
                &inbound,
                &inbound_bot_message_id,
                &trigger_ai_message_id,
            )
            .await
            {
                Ok(outcome) => outcome,
                Err(error) => {
                    return send_text_chunks(
                        db,
                        client,
                        config,
                        &inbound.peer_type,
                        &inbound.peer_id,
                        &format!("Task failed: {error}"),
                        context_token,
                        None,
                    )
                    .await;
                }
            };
            match outcome.result {
                Ok(response) => {
                    let text = response.trim();
                    send_text_chunks(
                        db,
                        client,
                        config,
                        &inbound.peer_type,
                        &inbound.peer_id,
                        if text.is_empty() {
                            "Task completed."
                        } else {
                            text
                        },
                        context_token,
                        Some(outcome.run_id.as_str()),
                    )
                    .await
                }
                Err(error) => {
                    send_text_chunks(
                        db,
                        client,
                        config,
                        &inbound.peer_type,
                        &inbound.peer_id,
                        &format!("Task failed: {error}"),
                        context_token,
                        Some(outcome.run_id.as_str()),
                    )
                    .await
                }
            }
        }
    }
}

async fn execute_remote_task(
    config: &WeixinGatewayConfig,
    db: &Arc<DatabaseService>,
    ai_manager: &Arc<AiServiceManager>,
    app_handle: &AppHandle,
    inbound: &InboundWeixinMessage,
    trigger_bot_message_id: &str,
    trigger_ai_message_id: &str,
) -> Result<WeixinTaskExecutionOutcome, String> {
    execute_weixin_task(
        config,
        db,
        ai_manager,
        app_handle,
        &inbound.peer_type,
        &inbound.peer_id,
        &inbound.sender_id,
        &inbound.text,
        &session_id(config, inbound),
        &execution_lock_key_for_inbound(config, inbound),
        config.assistant_profile_id.as_deref(),
        "message",
        Some(trigger_bot_message_id),
        Some(trigger_ai_message_id),
    )
    .await
}

async fn execute_weixin_task(
    config: &WeixinGatewayConfig,
    db: &Arc<DatabaseService>,
    ai_manager: &Arc<AiServiceManager>,
    app_handle: &AppHandle,
    peer_type: &str,
    peer_id: &str,
    sender_id: &str,
    task_text: &str,
    conversation_id: &str,
    execution_lock_key: &str,
    assistant_profile_id: Option<&str>,
    trigger_kind: &str,
    trigger_bot_message_id: Option<&str>,
    trigger_ai_message_id: Option<&str>,
) -> Result<WeixinTaskExecutionOutcome, String> {
    let run_id = Uuid::new_v4().to_string();

    register_active_execution(execution_lock_key, &run_id).await?;

    let execution_result = async {
        let assistant_profile =
            load_assistant_profile_by_id_or_default(db.as_ref(), assistant_profile_id).await?;
        ensure_assistant_mode_profile(assistant_profile.as_ref())?;

        let (default_provider, default_model_name) = ai_manager
            .get_default_llm_model()
            .await
            .map_err(|e| format!("Failed to get default model: {e}"))?
            .ok_or_else(|| "Default chat model is not configured".to_string())?;
        let (provider, model_name) = resolve_profile_model(
            assistant_profile.as_ref(),
            &default_provider,
            &default_model_name,
        );
        let provider_config = ai_manager
            .get_provider_config(&provider)
            .await
            .map_err(|e| format!("Failed to load provider config '{provider}': {e}"))?
            .ok_or_else(|| format!("Provider '{provider}' configuration not found"))?;
        let rig_provider = provider_config
            .rig_provider
            .clone()
            .unwrap_or(provider_config.provider.clone());
        ensure_conversation_exists(config, db, &provider, &model_name, conversation_id).await?;
        let system_prompt = resolve_weixin_system_prompt(db).await?;
        let tool_config = resolve_profile_tool_config(assistant_profile.as_ref(), db).await;
        let context_engine_mode = assistant_profile
            .as_ref()
            .and_then(|profile| ContextEngineMode::from_str(&profile.context_mode))
            .unwrap_or(ContextEngineMode::ClaudeLike);

        let params = AgentExecuteParams {
            execution_id: run_id.clone(),
            conversation_id: Some(conversation_id.to_string()),
            cancellation_generation: None,
            model: model_name,
            system_prompt,
            task: task_text.to_string(),
            active_browser_shell_direct_write_enabled: false,
            active_browser_shell_session_id: None,
            active_terminal_session_fingerprint: None,
            active_terminal_session_id: None,
            working_directory: None,
            provider_config_key: provider.clone(),
            rig_provider,
            api_key: provider_config.api_key.clone(),
            api_base: provider_config.api_base.clone(),
            max_iterations: config.max_iterations,
            timeout_secs: config.timeout_secs,
            tool_config,
            enable_tenth_man_rule: false,
            tenth_man_config: None,
            document_attachments: None,
            image_attachments: None,
            referenced_traffic: None,
            persist_messages: true,
            subagent_run_id: None,
            harness_run_id: None,
            context_policy: None,
            context_engine_mode: Some(context_engine_mode),
            recursion_depth: 0,
        };

        execute_bot_execution(
            app_handle,
            db,
            BotExecutionRequest {
                run_id: run_id.clone(),
                transport: "weixin".to_string(),
                account_id: config.account_id.clone(),
                peer_type: peer_type.to_string(),
                peer_id: peer_id.to_string(),
                sender_id: sender_id.to_string(),
                conversation_id: conversation_id.to_string(),
                assistant_profile_id: assistant_profile_id.map(str::to_string),
                trigger_kind: trigger_kind.to_string(),
                trigger_bot_message_id: trigger_bot_message_id.map(str::to_string),
                trigger_ai_message_id: trigger_ai_message_id.map(str::to_string),
                task_text: task_text.to_string(),
                agent_params: params,
            },
        )
        .await
    }
    .await;

    clear_active_execution(execution_lock_key, &run_id).await;
    execution_result.map(|outcome| WeixinTaskExecutionOutcome {
        run_id: outcome.run_id,
        result: outcome.result,
    })
}

async fn persist_inbound_bot_message(
    config: &WeixinGatewayConfig,
    db: &Arc<DatabaseService>,
    inbound: &InboundWeixinMessage,
) -> Result<String, String> {
    let now = Utc::now();
    db.upsert_bot_account_presence("weixin", &config.account_id, None, Some("running"), now)
        .await
        .map_err(|e| format!("Failed to upsert bot account: {e}"))?;
    db.record_bot_peer_activity(
        "weixin",
        &config.account_id,
        &inbound.peer_type,
        &inbound.peer_id,
        Some(&inbound.sender_id),
        "inbound",
        now,
    )
    .await
    .map_err(|e| format!("Failed to update bot peer activity: {e}"))?;

    let message = BotMessage {
        id: Uuid::new_v4().to_string(),
        transport: "weixin".to_string(),
        account_id: config.account_id.clone(),
        peer_type: inbound.peer_type.clone(),
        peer_id: inbound.peer_id.clone(),
        sender_id: inbound.sender_id.clone(),
        direction: "inbound".to_string(),
        content: inbound.text.clone(),
        transport_message_id: Some(inbound.message_id.clone()),
        context_token: inbound.context_token.clone(),
        conversation_id: Some(session_id(config, inbound)),
        ai_message_id: None,
        linked_execution_run_id: None,
        metadata_json: Some(json!({ "source": "weixin_gateway" }).to_string()),
        created_at: now,
    };
    db.create_bot_message(&message)
        .await
        .map_err(|e| format!("Failed to persist inbound bot message: {e}"))?;
    Ok(message.id)
}

async fn persist_inbound_user_message(
    db: &Arc<DatabaseService>,
    config: &WeixinGatewayConfig,
    inbound: &InboundWeixinMessage,
    conversation_id: &str,
) -> Result<String, String> {
    let message = AiMessage {
        id: Uuid::new_v4().to_string(),
        conversation_id: conversation_id.to_string(),
        role: "user".to_string(),
        content: inbound.text.clone(),
        metadata: Some(
            json!({
                "transport": "weixin",
                "account_id": config.account_id.as_str(),
                "peer_type": inbound.peer_type.as_str(),
                "peer_id": inbound.peer_id.as_str(),
                "sender_id": inbound.sender_id.as_str(),
                "transport_message_id": inbound.message_id.as_str(),
                "context_token": inbound.context_token.as_deref(),
            })
            .to_string(),
        ),
        token_count: Some(inbound.text.len() as i32),
        cost: None,
        tool_calls: None,
        attachments: None,
        reasoning_content: None,
        timestamp: Utc::now(),
        architecture_type: None,
        architecture_meta: None,
        structured_data: None,
    };
    db.create_ai_message(&message)
        .await
        .map_err(|e| format!("Failed to persist inbound AI message: {e}"))?;
    Ok(message.id)
}

async fn persist_outbound_bot_message(
    config: &WeixinGatewayConfig,
    db: &Arc<DatabaseService>,
    peer_type: &str,
    peer_id: &str,
    transport_message_id: &str,
    chunk: &str,
    context_token: Option<&str>,
    linked_execution_run_id: Option<&str>,
    chunk_index: usize,
    chunk_count: usize,
) -> Result<(), String> {
    let now = Utc::now();
    db.upsert_bot_account_presence("weixin", &config.account_id, None, Some("running"), now)
        .await
        .map_err(|e| format!("Failed to upsert bot account: {e}"))?;
    db.record_bot_peer_activity(
        "weixin",
        &config.account_id,
        peer_type,
        peer_id,
        Some(&config.account_id),
        "outbound",
        now,
    )
    .await
    .map_err(|e| format!("Failed to update bot peer activity: {e}"))?;

    let message = BotMessage {
        id: Uuid::new_v4().to_string(),
        transport: "weixin".to_string(),
        account_id: config.account_id.clone(),
        peer_type: peer_type.to_string(),
        peer_id: peer_id.to_string(),
        sender_id: config.account_id.clone(),
        direction: "outbound".to_string(),
        content: chunk.to_string(),
        transport_message_id: Some(transport_message_id.to_string()),
        context_token: context_token.map(str::to_string),
        conversation_id: Some(session_id_for_peer(config, peer_type, peer_id)),
        ai_message_id: None,
        linked_execution_run_id: linked_execution_run_id.map(str::to_string),
        metadata_json: Some(
            json!({
                "source": "weixin_gateway",
                "chunk_index": chunk_index,
                "chunk_count": chunk_count,
            })
            .to_string(),
        ),
        created_at: now,
    };
    db.create_bot_message(&message)
        .await
        .map_err(|e| format!("Failed to persist outbound bot message: {e}"))?;
    Ok(())
}

#[allow(dead_code)]
fn ensure_assistant_mode_profile(profile: Option<&AssistantProfilePayload>) -> Result<(), String> {
    if let Some(profile) = profile {
        if profile.run_mode == "team" {
            return Err(format!(
                "Weixin bot only supports assistant-mode Agent Profiles; '{}' is team mode",
                profile.label
            ));
        }
    }
    Ok(())
}

fn resolve_profile_model(
    profile: Option<&AssistantProfilePayload>,
    default_provider: &str,
    default_model_name: &str,
) -> (String, String) {
    let Some(default_model) = profile
        .and_then(|profile| profile.default_model.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return (default_provider.to_string(), default_model_name.to_string());
    };

    let Some((provider, model)) = default_model.split_once('/') else {
        return (default_provider.to_string(), default_model.to_string());
    };
    (provider.to_string(), model.to_string())
}

async fn resolve_profile_tool_config(
    profile: Option<&AssistantProfilePayload>,
    db: &Arc<DatabaseService>,
) -> Option<ToolConfig> {
    let baseline = db
        .get_config("agent", "tool_config")
        .await
        .ok()
        .flatten()
        .and_then(|raw| ToolConfig::from_json_str(&raw).ok())
        .unwrap_or_default();

    let Some(profile) = profile else {
        return Some(baseline);
    };

    Some(ToolConfig {
        enabled: profile.default_tools_enabled,
        selection_strategy: profile_selection_strategy(profile),
        max_tools: (profile.default_max_tools as usize).max(1),
        preselected_tools: normalize_tool_ids(&profile.default_preselected_tools),
        disabled_tools: normalize_tool_ids(&profile.default_disabled_tools),
        allowed_tools: Vec::new(),
    })
}

fn profile_selection_strategy(profile: &AssistantProfilePayload) -> ToolSelectionStrategy {
    match profile.default_tool_selection_strategy.as_str() {
        "All" => ToolSelectionStrategy::All,
        "LLM" => ToolSelectionStrategy::LLM,
        "Hybrid" => ToolSelectionStrategy::Hybrid,
        "Manual" => {
            ToolSelectionStrategy::Manual(normalize_tool_ids(&profile.default_manual_tools))
        }
        "Deferred" => ToolSelectionStrategy::Deferred,
        "Keyword" => ToolSelectionStrategy::Keyword,
        _ => ToolSelectionStrategy::Keyword,
    }
}

fn normalize_tool_ids(items: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for item in items {
        let normalized = item.trim().replace("::", "__");
        if !normalized.is_empty() && !out.iter().any(|value| value == &normalized) {
            out.push(normalized);
        }
    }
    out
}

async fn resolve_weixin_system_prompt(db: &Arc<DatabaseService>) -> Result<String, String> {
    let base = db
        .get_current_ai_role()
        .await
        .map_err(|e| format!("Failed to load current AI role: {e}"))?
        .map(|role| role.prompt)
        .unwrap_or_default();
    let mission_rule = r#"

[Weixin Bot Mission Rule]
When the user asks for recurring, periodic, scheduled, daily, weekly, monthly, long-running,
monitoring, subscription, or scheduled delivery work, use the mission_scheduler tool to create
or manage a Mission. For stateful or multi-day work, pass a mission_spec with generic state_schema,
action_schema, completion_policy, and report_policy so later runs can continue from persisted state.
Ask for missing critical information first. Do not say a scheduled task has been created unless
mission_scheduler returns success. Pass the current execution_id to mission_scheduler so the Mission
is bound to this Weixin chat.
When the user corrects, changes, or updates an existing scheduled task, first list Missions for
the current execution_id, then call update_mission with the existing mission_id. Do not create a
replacement Mission for a correction unless the user explicitly asks for a new separate task.
"#;
    Ok(format!("{}{}", base.trim_end(), mission_rule))
}

async fn ensure_conversation_exists(
    config: &WeixinGatewayConfig,
    db: &Arc<DatabaseService>,
    provider: &str,
    model_name: &str,
    session_id: &str,
) -> Result<(), String> {
    if let Some(mut conversation) = db
        .get_ai_conversation(session_id)
        .await
        .map_err(|e| format!("Failed to query conversation: {e}"))?
    {
        let mut changed = false;
        if conversation.model_name != model_name {
            conversation.model_name = model_name.to_string();
            changed = true;
        }
        if conversation.model_provider.as_deref() != Some(provider) {
            conversation.model_provider = Some(provider.to_string());
            changed = true;
        }
        if conversation.service_name != config.default_service_name {
            conversation.service_name = config.default_service_name.clone();
            changed = true;
        }
        if conversation.title.as_deref() != Some("Weixin Remote Control") {
            conversation.title = Some("Weixin Remote Control".to_string());
            changed = true;
        }
        if conversation.tags.as_deref() != Some(r#"["weixin","remote-control"]"#) {
            conversation.tags = Some(r#"["weixin","remote-control"]"#.to_string());
            changed = true;
        }
        if changed {
            conversation.updated_at = Utc::now();
            db.update_ai_conversation(&conversation)
                .await
                .map_err(|e| format!("Failed to update conversation: {e}"))?;
        }
        return Ok(());
    }

    let mut conversation =
        AiConversation::new(model_name.to_string(), config.default_service_name.clone());
    conversation.id = session_id.to_string();
    conversation.model_provider = Some(provider.to_string());
    conversation.title = Some("Weixin Remote Control".to_string());
    conversation.tags = Some(r#"["weixin","remote-control"]"#.to_string());
    db.create_ai_conversation(&conversation)
        .await
        .map_err(|e| format!("Failed to create conversation: {e}"))
}

async fn send_text_chunks(
    db: &Arc<DatabaseService>,
    client: &WeixinIlinkClient,
    config: &WeixinGatewayConfig,
    peer_type: &str,
    peer_id: &str,
    text: &str,
    context_token: Option<&str>,
    linked_execution_run_id: Option<&str>,
) -> Result<(), String> {
    let chunks = split_message(text, 1800);
    let chunk_count = chunks.len();
    for (index, chunk) in chunks.into_iter().enumerate() {
        let transport_message_id = client
            .send_text(
                &config.base_url,
                &config.token,
                peer_id,
                &chunk,
                context_token,
            )
            .await?;
        persist_outbound_bot_message(
            config,
            db,
            peer_type,
            peer_id,
            &transport_message_id,
            &chunk,
            context_token,
            linked_execution_run_id,
            index,
            chunk_count,
        )
        .await?;
    }
    Ok(())
}

pub async fn deliver_weixin_text(
    db: &Arc<DatabaseService>,
    account_id: &str,
    base_url: &str,
    token: &str,
    peer_type: &str,
    peer_id: &str,
    text: &str,
    context_token: Option<&str>,
    linked_execution_run_id: Option<&str>,
) -> Result<(), String> {
    let config = WeixinGatewayConfig {
        enabled: true,
        account_id: account_id.to_string(),
        token: token.to_string(),
        base_url: base_url.to_string(),
        assistant_profile_id: None,
        dm_policy: "open".to_string(),
        allowed_users: Vec::new(),
        group_policy: "open".to_string(),
        group_allowed_users: Vec::new(),
        default_service_name: "default".to_string(),
        max_iterations: 1,
        timeout_secs: 30,
    };
    let client = WeixinIlinkClient::new()?;
    send_text_chunks(
        db,
        &client,
        &config,
        peer_type,
        peer_id,
        text,
        context_token,
        linked_execution_run_id,
    )
    .await
}

mod runtime_message;
use runtime_message::{
    execution_lock_key_for_inbound, is_authorized, is_self_message, message_identity,
    parse_inbound_message, session_id, session_id_for_peer, split_message, trim_seen_messages,
    InboundWeixinMessage,
};

async fn mark_message_seen() {
    let mut state = WEIXIN_RUNTIME.write().await;
    state.last_message_at = Some(Utc::now());
}

async fn set_runtime_error(error: String) {
    let mut state = WEIXIN_RUNTIME.write().await;
    state.last_error = Some(error);
}
