use super::config::{WeixinGatewayConfig, WeixinGatewayStatus};
use super::ilink_client::WeixinIlinkClient;
use super::schedule::{
    calculate_next_run_at, create_weixin_schedule_run, delete_weixin_schedule,
    finalize_weixin_schedule_run, format_schedule_timestamp, get_weixin_schedule,
    list_weixin_schedules_for_account, list_weixin_schedules_for_peer,
    migrate_legacy_weixin_schedule_registry, new_weixin_schedule, new_weixin_schedule_run,
    normalize_bot_schedule, upsert_weixin_schedule,
};
use super::schedule_parser::{looks_like_schedule_text, parse_schedule_intent};
use crate::agents::{AgentExecuteParams, ContextEngineMode, ToolConfig, ToolSelectionStrategy};
use crate::commands::ai::cancel_conversation_stream;
use crate::commands::assistant_profile_commands::{
    load_assistant_profile_by_id_or_default, AssistantProfilePayload,
};
use crate::models::database::{AiConversation, AiMessage, BotMessage, BotSchedule};
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
    migrate_legacy_weixin_schedule_registry(&db).await?;

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
    let running_schedule_ids = Arc::new(RwLock::new(HashSet::<String>::new()));
    let mut schedule_tick = tokio::time::interval(Duration::from_secs(20));
    schedule_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = &mut shutdown_rx => break,
            _ = schedule_tick.tick() => {
                if let Err(error) = dispatch_due_schedules(
                    &config,
                    &client,
                    &db,
                    &ai_manager,
                    &app_handle,
                    &running_schedule_ids,
                )
                .await
                {
                    set_runtime_error(error).await;
                }
            }
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
        "/schedules" => {
            let text = list_peer_schedules(config, db, &inbound).await?;
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
        text if text.starts_with("/schedule pause ") => {
            let schedule_id = text.trim_start_matches("/schedule pause ").trim();
            let text = set_peer_schedule_enabled(config, db, &inbound, schedule_id, false).await?;
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
        text if text.starts_with("/schedule resume ") => {
            let schedule_id = text.trim_start_matches("/schedule resume ").trim();
            let text = set_peer_schedule_enabled(config, db, &inbound, schedule_id, true).await?;
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
        text if text.starts_with("/schedule delete ") => {
            let schedule_id = text.trim_start_matches("/schedule delete ").trim();
            let text = delete_peer_schedule(config, db, &inbound, schedule_id).await?;
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
                "Unsupported command. Available: /status, /stop, /permissions, /approve <id>, /deny <id>, /schedules, /schedule pause <id>, /schedule resume <id>, /schedule delete <id>.",
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
async fn maybe_create_schedule_from_message(
    config: &WeixinGatewayConfig,
    db: &Arc<DatabaseService>,
    ai_manager: &Arc<AiServiceManager>,
    inbound: &InboundWeixinMessage,
) -> Result<Option<String>, String> {
    if !looks_like_schedule_text(&inbound.text) {
        return Ok(None);
    }

    let intent = parse_schedule_intent(ai_manager, inbound.text.trim()).await?;
    if intent.action != "create" {
        return Ok(Some(
            "当前只支持周期性定时任务。请明确描述类似“每天早上 8 点给我发送新闻摘要”的需求。"
                .to_string(),
        ));
    }

    let cron = intent
        .cron
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "schedule parser did not return cron".to_string())?;
    let task = intent
        .task
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "schedule parser did not return task".to_string())?;

    let assistant_profile = load_assistant_profile_by_id_or_default(
        db.as_ref(),
        config.assistant_profile_id.as_deref(),
    )
    .await?;
    ensure_assistant_mode_profile(assistant_profile.as_ref())?;
    let assistant_profile_id = assistant_profile.as_ref().map(|profile| profile.id.clone());

    let schedule = new_weixin_schedule(
        config.account_id.clone(),
        inbound.peer_type.clone(),
        inbound.peer_id.clone(),
        inbound.sender_id.clone(),
        assistant_profile_id,
        inbound.text.clone(),
        task.to_string(),
        cron.to_string(),
    )?;
    upsert_weixin_schedule(db, &schedule).await?;

    Ok(Some(format!(
        "已创建定时任务\nID: {}\nCron: {}\n下次执行: {}\n任务: {}\n查看任务: /schedules",
        schedule.id,
        schedule.cron_expr,
        format_schedule_timestamp(schedule.next_run_at),
        schedule.task_text
    )))
}

async fn list_peer_schedules(
    config: &WeixinGatewayConfig,
    db: &Arc<DatabaseService>,
    inbound: &InboundWeixinMessage,
) -> Result<String, String> {
    let items = list_weixin_schedules_for_peer(
        db,
        &config.account_id,
        &inbound.peer_type,
        &inbound.peer_id,
    )
    .await?;

    if items.is_empty() {
        return Ok("当前没有定时任务。".to_string());
    }

    let mut lines = vec!["当前定时任务:".to_string()];
    for schedule in items {
        lines.push(format!(
            "{} [{}] cron={} next={} task={}",
            schedule.id,
            if schedule.enabled {
                "enabled"
            } else {
                "paused"
            },
            schedule.cron_expr,
            format_schedule_timestamp(schedule.next_run_at),
            schedule.task_text
        ));
    }
    Ok(lines.join("\n"))
}

async fn set_peer_schedule_enabled(
    config: &WeixinGatewayConfig,
    db: &Arc<DatabaseService>,
    inbound: &InboundWeixinMessage,
    schedule_id: &str,
    enabled: bool,
) -> Result<String, String> {
    let now = Utc::now();
    let Some(mut schedule) = get_weixin_schedule(db, schedule_id).await? else {
        return Err(format!("schedule not found: {schedule_id}"));
    };
    if schedule.transport != "weixin"
        || schedule.account_id != config.account_id
        || schedule.peer_type != inbound.peer_type
        || schedule.peer_id != inbound.peer_id
    {
        return Err(format!("schedule not found: {schedule_id}"));
    }

    schedule.enabled = enabled;
    schedule.updated_at = now;
    schedule.last_error = None;
    schedule.next_run_at = if enabled {
        Some(calculate_next_run_at(&schedule.cron_expr, now)?)
    } else {
        None
    };
    normalize_bot_schedule(&mut schedule)?;
    upsert_weixin_schedule(db, &schedule).await?;
    let schedule_id = schedule.id.clone();
    let action_text = if enabled { "恢复" } else { "暂停" }.to_string();

    Ok(format!("定时任务 {} 已{}。", schedule_id, action_text))
}

async fn delete_peer_schedule(
    config: &WeixinGatewayConfig,
    db: &Arc<DatabaseService>,
    inbound: &InboundWeixinMessage,
    schedule_id: &str,
) -> Result<String, String> {
    let Some(schedule) = get_weixin_schedule(db, schedule_id).await? else {
        return Err(format!("schedule not found: {schedule_id}"));
    };
    if schedule.transport != "weixin"
        || schedule.account_id != config.account_id
        || schedule.peer_type != inbound.peer_type
        || schedule.peer_id != inbound.peer_id
    {
        return Err(format!("schedule not found: {schedule_id}"));
    }
    delete_weixin_schedule(db, schedule_id).await?;
    Ok(format!("定时任务 {} 已删除。", schedule_id))
}

async fn dispatch_due_schedules(
    config: &WeixinGatewayConfig,
    client: &WeixinIlinkClient,
    db: &Arc<DatabaseService>,
    ai_manager: &Arc<AiServiceManager>,
    app_handle: &AppHandle,
    running_schedule_ids: &Arc<RwLock<HashSet<String>>>,
) -> Result<(), String> {
    let mut schedules = list_weixin_schedules_for_account(db, &config.account_id).await?;
    let now = Utc::now();
    let active_runs = running_schedule_ids.read().await.clone();
    let mut due_schedules = Vec::new();
    let mut changed_schedules = Vec::new();

    for schedule in &mut schedules {
        if !schedule.enabled {
            continue;
        }
        if active_runs.contains(&schedule.id) {
            continue;
        }

        if schedule.next_run_at.is_none() {
            match calculate_next_run_at(&schedule.cron_expr, now) {
                Ok(next_run_at) => {
                    schedule.next_run_at = Some(next_run_at);
                    schedule.last_error = None;
                }
                Err(error) => {
                    schedule.last_error = Some(error);
                }
            }
            schedule.updated_at = now;
            changed_schedules.push(schedule.clone());
        }

        let Some(next_run_at) = schedule.next_run_at else {
            continue;
        };
        if next_run_at > now {
            continue;
        }

        match calculate_next_run_at(&schedule.cron_expr, now) {
            Ok(next_due) => {
                schedule.next_run_at = Some(next_due);
                schedule.last_error = None;
                schedule.updated_at = now;
                changed_schedules.push(schedule.clone());
                due_schedules.push(schedule.clone());
            }
            Err(error) => {
                schedule.last_error = Some(error);
                schedule.next_run_at = None;
                schedule.updated_at = now;
                changed_schedules.push(schedule.clone());
            }
        }
    }

    for schedule in changed_schedules {
        upsert_weixin_schedule(db, &schedule).await?;
    }

    for schedule in due_schedules {
        {
            let mut active_runs = running_schedule_ids.write().await;
            active_runs.insert(schedule.id.clone());
        }
        let config = config.clone();
        let client = client.clone();
        let db = db.clone();
        let ai_manager = ai_manager.clone();
        let app_handle = app_handle.clone();
        let running_schedule_ids = running_schedule_ids.clone();
        tokio::spawn(async move {
            run_scheduled_schedule(
                config,
                client,
                db,
                ai_manager,
                app_handle,
                schedule,
                running_schedule_ids,
            )
            .await;
        });
    }

    Ok(())
}

async fn run_scheduled_schedule(
    config: WeixinGatewayConfig,
    client: WeixinIlinkClient,
    db: Arc<DatabaseService>,
    ai_manager: Arc<AiServiceManager>,
    app_handle: AppHandle,
    schedule: BotSchedule,
    running_schedule_ids: Arc<RwLock<HashSet<String>>>,
) {
    let schedule_run = new_weixin_schedule_run(&schedule);
    if let Err(error) = create_weixin_schedule_run(&db, &schedule_run).await {
        let _ = update_schedule_execution_result(
            &db,
            &schedule,
            Some(format!("failed to create schedule run: {error}")),
        )
        .await;
        running_schedule_ids.write().await.remove(&schedule.id);
        return;
    }

    let execution_result = execute_weixin_task(
        &config,
        &db,
        &ai_manager,
        &app_handle,
        &schedule.peer_type,
        &schedule.peer_id,
        &schedule.sender_id,
        &schedule.task_text,
        &schedule_conversation_id_for_peer(
            &config,
            &schedule.peer_type,
            &schedule.peer_id,
            &schedule.id,
        ),
        &session_id_for_peer(&config, &schedule.peer_type, &schedule.peer_id),
        schedule.assistant_profile_id.as_deref(),
        "schedule",
        None,
        None,
    )
    .await;

    let send_result: Result<(Option<String>, String), (Option<String>, String)> =
        match execution_result {
            Ok(outcome) => match outcome.result {
                Ok(response) => {
                    let response_text = if response.trim().is_empty() {
                        "Task completed.".to_string()
                    } else {
                        response
                    };
                    let execution_run_id = outcome.run_id;
                    send_text_chunks(
                        &db,
                        &client,
                        &config,
                        &schedule.peer_type,
                        &schedule.peer_id,
                        &response_text,
                        None,
                        Some(execution_run_id.as_str()),
                    )
                    .await
                    .map(|_| (Some(execution_run_id.clone()), response_text))
                    .map_err(|error| (Some(execution_run_id), error))
                }
                Err(error) => {
                    let execution_run_id = outcome.run_id;
                    let text = format!("Scheduled task failed: {error}");
                    match send_text_chunks(
                        &db,
                        &client,
                        &config,
                        &schedule.peer_type,
                        &schedule.peer_id,
                        &text,
                        None,
                        Some(execution_run_id.as_str()),
                    )
                    .await
                    {
                        Ok(_) => Err((Some(execution_run_id), error)),
                        Err(send_error) => Err((
                            Some(execution_run_id),
                            format!("{error}; send failed: {send_error}"),
                        )),
                    }
                }
            },
            Err(error) => {
                let text = format!("Scheduled task failed: {error}");
                match send_text_chunks(
                    &db,
                    &client,
                    &config,
                    &schedule.peer_type,
                    &schedule.peer_id,
                    &text,
                    None,
                    None,
                )
                .await
                {
                    Ok(_) => Err((None, error)),
                    Err(send_error) => Err((None, format!("{error}; send failed: {send_error}"))),
                }
            }
        };

    match send_result {
        Ok((execution_run_id, response)) => {
            let _ = finalize_weixin_schedule_run(
                &db,
                &schedule_run.id,
                execution_run_id.as_deref(),
                "completed",
                Some(response.as_str()),
                None,
            )
            .await;
            let _ = update_schedule_execution_result(&db, &schedule, None).await;
        }
        Err((execution_run_id, error)) => {
            let _ = finalize_weixin_schedule_run(
                &db,
                &schedule_run.id,
                execution_run_id.as_deref(),
                "failed",
                None,
                Some(error.as_str()),
            )
            .await;
            let _ = update_schedule_execution_result(&db, &schedule, Some(error)).await;
        }
    }
    running_schedule_ids.write().await.remove(&schedule.id);
}

async fn update_schedule_execution_result(
    db: &Arc<DatabaseService>,
    schedule: &BotSchedule,
    error: Option<String>,
) -> Result<(), String> {
    let mut updated = schedule.clone();
    updated.last_run_at = Some(Utc::now());
    updated.updated_at = Utc::now();
    updated.last_error = error;
    normalize_bot_schedule(&mut updated)?;
    upsert_weixin_schedule(db, &updated).await?;
    Ok(())
}

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
or manage a Mission. Ask for missing critical information first. Do not say a scheduled task has
been created unless mission_scheduler returns success. Pass the current execution_id to
mission_scheduler so the Mission is bound to this Weixin chat.
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
        None,
        linked_execution_run_id,
    )
    .await
}

#[derive(Debug)]
struct InboundWeixinMessage {
    message_id: String,
    peer_type: String,
    peer_id: String,
    sender_id: String,
    text: String,
    context_token: Option<String>,
}

fn parse_inbound_message(
    config: &WeixinGatewayConfig,
    message: &Value,
    message_id: String,
) -> Option<InboundWeixinMessage> {
    let text = extract_text(message.get("item_list").and_then(Value::as_array)?)?;
    if text.trim().is_empty() {
        return None;
    }
    let room_id = string_field(message, "room_id")
        .or_else(|| string_field(message, "chat_room_id"))
        .unwrap_or_default();
    let from_user_id = string_field(message, "from_user_id")?;
    let to_user_id = string_field(message, "to_user_id").unwrap_or_default();
    let is_group = !room_id.is_empty()
        || (!to_user_id.is_empty()
            && to_user_id != config.account_id
            && message.get("msg_type").and_then(Value::as_i64) == Some(1));
    let peer_id = if is_group {
        if room_id.is_empty() {
            to_user_id
        } else {
            room_id
        }
    } else {
        from_user_id.clone()
    };

    Some(InboundWeixinMessage {
        message_id,
        peer_type: if is_group { "group" } else { "dm" }.to_string(),
        peer_id,
        sender_id: from_user_id,
        text,
        context_token: string_field(message, "context_token"),
    })
}

fn extract_text(items: &[Value]) -> Option<String> {
    for item in items {
        if item.get("type").and_then(Value::as_i64) == Some(1) {
            return item
                .get("text_item")
                .and_then(|value| value.get("text"))
                .and_then(Value::as_str)
                .map(str::to_string);
        }
    }
    for item in items {
        if item.get("type").and_then(Value::as_i64) == Some(3) {
            if let Some(text) = item
                .get("voice_item")
                .and_then(|value| value.get("text"))
                .and_then(Value::as_str)
            {
                return Some(text.to_string());
            }
        }
    }
    None
}

fn is_self_message(config: &WeixinGatewayConfig, message: &Value) -> bool {
    string_field(message, "from_user_id").as_deref() == Some(config.account_id.as_str())
}

fn is_authorized(config: &WeixinGatewayConfig, message: &InboundWeixinMessage) -> bool {
    if message.peer_type == "group" {
        if config.group_policy == "open" {
            return true;
        }
        return config.group_policy == "allowlist"
            && config
                .group_allowed_users
                .iter()
                .any(|value| value == &message.peer_id || value == &message.sender_id);
    }
    if config.dm_policy == "open" {
        return true;
    }
    config.dm_policy == "allowlist"
        && config
            .allowed_users
            .iter()
            .any(|value| value == &message.sender_id || value == &message.peer_id)
}

fn session_id(config: &WeixinGatewayConfig, inbound: &InboundWeixinMessage) -> String {
    session_id_for_peer(config, &inbound.peer_type, &inbound.peer_id)
}

fn execution_lock_key_for_inbound(
    config: &WeixinGatewayConfig,
    inbound: &InboundWeixinMessage,
) -> String {
    session_id_for_peer(config, &inbound.peer_type, &inbound.peer_id)
}

fn session_id_for_peer(config: &WeixinGatewayConfig, peer_type: &str, peer_id: &str) -> String {
    format!("weixin:{}:{}:{}", config.account_id, peer_type, peer_id)
}

fn schedule_conversation_id_for_peer(
    config: &WeixinGatewayConfig,
    peer_type: &str,
    peer_id: &str,
    schedule_id: &str,
) -> String {
    format!(
        "weixin:{}:{}:{}:schedule:{}",
        config.account_id, peer_type, peer_id, schedule_id
    )
}

fn message_identity(message: &Value) -> String {
    string_field(message, "msg_id")
        .or_else(|| string_field(message, "message_id"))
        .unwrap_or_else(|| {
            format!(
                "{}:{}:{}",
                string_field(message, "from_user_id").unwrap_or_default(),
                message
                    .get("create_time")
                    .and_then(Value::as_i64)
                    .unwrap_or(0),
                string_field(message, "client_id").unwrap_or_default()
            )
        })
}

fn string_field(message: &Value, key: &str) -> Option<String> {
    message
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn trim_seen_messages(seen_messages: &mut HashSet<String>) {
    if seen_messages.len() <= 512 {
        return;
    }
    seen_messages.clear();
}

fn split_message(text: &str, max_len: usize) -> Vec<String> {
    if text.len() <= max_len {
        return vec![text.to_string()];
    }
    let mut chunks = Vec::new();
    let mut current = String::new();
    for line in text.lines() {
        if current.len() + line.len() + 1 > max_len && !current.is_empty() {
            chunks.push(current.trim().to_string());
            current.clear();
        }
        if line.len() > max_len {
            for part in line.as_bytes().chunks(max_len) {
                chunks.push(String::from_utf8_lossy(part).to_string());
            }
            continue;
        }
        current.push_str(line);
        current.push('\n');
    }
    if !current.trim().is_empty() {
        chunks.push(current.trim().to_string());
    }
    chunks
}

async fn mark_message_seen() {
    let mut state = WEIXIN_RUNTIME.write().await;
    state.last_message_at = Some(Utc::now());
}

async fn set_runtime_error(error: String) {
    let mut state = WEIXIN_RUNTIME.write().await;
    state.last_error = Some(error);
}
