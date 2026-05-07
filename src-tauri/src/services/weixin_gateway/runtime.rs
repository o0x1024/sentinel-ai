use super::config::{WeixinGatewayConfig, WeixinGatewayStatus};
use super::ilink_client::WeixinIlinkClient;
use crate::agents::{AgentExecuteParams, ContextEngineMode, ToolConfig, ToolSelectionStrategy};
use crate::commands::assistant_profile_commands::{
    load_assistant_profile_by_id_or_default, AssistantProfilePayload,
};
use crate::models::database::AiConversation;
use crate::services::ai::AiServiceManager;
use crate::services::database::DatabaseService;
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use sentinel_db::Database;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tauri::AppHandle;
use tokio::sync::{oneshot, RwLock};
use tokio::task::JoinHandle;

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

static WEIXIN_RUNTIME: Lazy<Arc<RwLock<WeixinRuntimeState>>> =
    Lazy::new(|| Arc::new(RwLock::new(WeixinRuntimeState::default())));

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
        if !seen_messages.insert(message_id) {
            continue;
        }
        trim_seen_messages(seen_messages);

        let inbound = match parse_inbound_message(config, message) {
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
    match inbound.text.trim() {
        "/status" => {
            send_text_chunks(
                client,
                config,
                &inbound.peer_id,
                "Sentinel Weixin remote control is running.",
                context_token,
            )
            .await
        }
        "/stop" => {
            let session_id = session_id(config, &inbound);
            let _ = crate::managers::cancellation_manager::cancel_execution(&session_id).await;
            send_text_chunks(
                client,
                config,
                &inbound.peer_id,
                "Stop signal sent.",
                context_token,
            )
            .await
        }
        "/permissions" => {
            let session_id = session_id(config, &inbound);
            let pending = crate::commands::tool_commands::get_pending_shell_permissions()
                .await
                .unwrap_or_default()
                .into_iter()
                .filter(|item| item.execution_id.as_deref() == Some(session_id.as_str()))
                .map(|item| format!("{} {}", item.id, item.command))
                .collect::<Vec<_>>();
            let text = if pending.is_empty() {
                "No pending shell permissions.".to_string()
            } else {
                format!("Pending shell permissions:\n{}", pending.join("\n"))
            };
            send_text_chunks(client, config, &inbound.peer_id, &text, context_token).await
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
            send_text_chunks(client, config, &inbound.peer_id, &text, context_token).await
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
            send_text_chunks(client, config, &inbound.peer_id, &text, context_token).await
        }
        text if text.starts_with('/') => {
            send_text_chunks(
                client,
                config,
                &inbound.peer_id,
                "Unsupported command. Available: /status, /stop, /permissions, /approve <id>, /deny <id>.",
                context_token,
            )
            .await
        }
        _ => {
            let result = execute_remote_task(config, db, ai_manager, app_handle, &inbound).await;
            match result {
                Ok(response) => {
                    let text = response.trim();
                    send_text_chunks(
                        client,
                        config,
                        &inbound.peer_id,
                        if text.is_empty() {
                            "Task completed."
                        } else {
                            text
                        },
                        context_token,
                    )
                    .await
                }
                Err(error) => {
                    send_text_chunks(
                        client,
                        config,
                        &inbound.peer_id,
                        &format!("Task failed: {error}"),
                        context_token,
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
) -> Result<String, String> {
    let session_id = session_id(config, inbound);
    let assistant_profile = load_assistant_profile_by_id_or_default(
        db.as_ref(),
        config.assistant_profile_id.as_deref(),
    )
    .await?;
    if let Some(profile) = assistant_profile.as_ref() {
        if profile.run_mode == "team" {
            return Err(format!(
                "Weixin bot only supports assistant-mode Agent Profiles; '{}' is team mode",
                profile.label
            ));
        }
    }

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
    ensure_conversation_exists(config, db, &provider, &model_name, &session_id).await?;
    let system_prompt = resolve_weixin_system_prompt(db).await?;
    let tool_config = resolve_profile_tool_config(assistant_profile.as_ref(), db).await;
    let context_engine_mode = assistant_profile
        .as_ref()
        .and_then(|profile| ContextEngineMode::from_str(&profile.context_mode))
        .unwrap_or(ContextEngineMode::ClaudeLike);

    let params = AgentExecuteParams {
        execution_id: session_id,
        cancellation_generation: None,
        model: model_name,
        system_prompt,
        task: inbound.text.clone(),
        active_browser_shell_direct_write_enabled: false,
        active_browser_shell_session_id: None,
        active_terminal_session_fingerprint: None,
        active_terminal_session_id: None,
        working_directory: None,
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
        context_policy: None,
        context_engine_mode: Some(context_engine_mode),
        recursion_depth: 0,
    };

    crate::agents::execute_agent(app_handle, params)
        .await
        .map_err(|e| format!("Agent execution failed: {e}"))
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
    Ok(db
        .get_current_ai_role()
        .await
        .map_err(|e| format!("Failed to load current AI role: {e}"))?
        .map(|role| role.prompt)
        .unwrap_or_default())
}

async fn ensure_conversation_exists(
    config: &WeixinGatewayConfig,
    db: &Arc<DatabaseService>,
    provider: &str,
    model_name: &str,
    session_id: &str,
) -> Result<(), String> {
    if db
        .get_ai_conversation(session_id)
        .await
        .map_err(|e| format!("Failed to query conversation: {e}"))?
        .is_some()
    {
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
    client: &WeixinIlinkClient,
    config: &WeixinGatewayConfig,
    peer_id: &str,
    text: &str,
    context_token: Option<&str>,
) -> Result<(), String> {
    for chunk in split_message(text, 1800) {
        client
            .send_text(
                &config.base_url,
                &config.token,
                peer_id,
                &chunk,
                context_token,
            )
            .await?;
    }
    Ok(())
}

#[derive(Debug)]
struct InboundWeixinMessage {
    peer_type: String,
    peer_id: String,
    sender_id: String,
    text: String,
    context_token: Option<String>,
}

fn parse_inbound_message(
    config: &WeixinGatewayConfig,
    message: &Value,
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
    format!(
        "weixin:{}:{}:{}",
        config.account_id, inbound.peer_type, inbound.peer_id
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
