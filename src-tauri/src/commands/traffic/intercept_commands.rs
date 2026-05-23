use tauri::{AppHandle, State};

use super::{InterceptedRequest, InterceptedResponse, TrafficAnalysisState};
use crate::commands::command_response_support::CommandResponse;
use sentinel_traffic::{
    InterceptAction, InterceptRuleConfig, InterceptRuleConfigSet, RuntimeInterceptRule,
    INTERCEPT_FILTER_RULES_CONFIG_KEY,
};

#[tauri::command]
pub async fn set_intercept_enabled(
    app: AppHandle,
    state: State<'_, TrafficAnalysisState>,
    enabled: bool,
) -> Result<CommandResponse<bool>, String> {
    tracing::info!("Setting intercept enabled: {}", enabled);

    state.set_app_handle(app).await;

    let mut intercept = state.intercept_enabled.write().await;
    *intercept = enabled;

    tracing::info!(
        "Intercept mode {}",
        if enabled { "enabled" } else { "disabled" }
    );
    Ok(CommandResponse::ok(enabled))
}

#[tauri::command]
pub async fn get_intercept_enabled(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<bool>, String> {
    let enabled = *state.intercept_enabled.read().await;
    Ok(CommandResponse::ok(enabled))
}

#[tauri::command]
pub async fn set_request_intercept_enabled(
    app: AppHandle,
    state: State<'_, TrafficAnalysisState>,
    enabled: bool,
) -> Result<CommandResponse<bool>, String> {
    tracing::info!("Setting request intercept enabled: {}", enabled);

    state.set_app_handle(app).await;

    let mut intercept = state.request_intercept_enabled.write().await;
    *intercept = enabled;

    if !enabled {
        let mut requests = state.intercepted_requests.write().await;
        let pending_requests: Vec<_> = requests.drain().collect();
        drop(requests);

        for (request_id, req_internal) in pending_requests {
            let _ = req_internal
                .response_tx
                .send(InterceptAction::Forward(None));
            tracing::info!(
                "Auto-forwarded intercepted request after disabling request intercept: {}",
                request_id
            );
        }
    }

    tracing::info!(
        "Request intercept mode {}",
        if enabled { "enabled" } else { "disabled" }
    );
    Ok(CommandResponse::ok(enabled))
}

#[tauri::command]
pub async fn get_request_intercept_enabled(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<bool>, String> {
    let enabled = *state.request_intercept_enabled.read().await;
    Ok(CommandResponse::ok(enabled))
}

#[tauri::command]
pub async fn get_intercepted_requests(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<Vec<InterceptedRequest>>, String> {
    let requests = state.intercepted_requests.read().await;
    let list: Vec<InterceptedRequest> = requests.values().map(|r| r.request.clone()).collect();
    Ok(CommandResponse::ok(list))
}

#[tauri::command]
pub async fn forward_intercepted_request(
    state: State<'_, TrafficAnalysisState>,
    request_id: String,
    modified_content: Option<String>,
) -> Result<CommandResponse<()>, String> {
    tracing::info!("Forwarding intercepted request: {}", request_id);

    let mut requests = state.intercepted_requests.write().await;
    if let Some(req_internal) = requests.remove(&request_id) {
        let _ = req_internal
            .response_tx
            .send(InterceptAction::Forward(modified_content));
        tracing::info!("Request forwarded: {}", request_id);
        Ok(CommandResponse::ok(()))
    } else {
        tracing::warn!("Request not found: {}", request_id);
        Ok(CommandResponse::err(format!(
            "Request not found: {}",
            request_id
        )))
    }
}

#[tauri::command]
pub async fn drop_intercepted_request(
    state: State<'_, TrafficAnalysisState>,
    request_id: String,
) -> Result<CommandResponse<()>, String> {
    tracing::info!("Dropping intercepted request: {}", request_id);

    let mut requests = state.intercepted_requests.write().await;
    if let Some(req_internal) = requests.remove(&request_id) {
        let _ = req_internal.response_tx.send(InterceptAction::Drop);
        tracing::info!("Request dropped: {}", request_id);
        Ok(CommandResponse::ok(()))
    } else {
        tracing::warn!("Request not found: {}", request_id);
        Ok(CommandResponse::err(format!(
            "Request not found: {}",
            request_id
        )))
    }
}

#[tauri::command]
pub async fn set_response_intercept_enabled(
    app: AppHandle,
    state: State<'_, TrafficAnalysisState>,
    enabled: bool,
) -> Result<CommandResponse<bool>, String> {
    tracing::info!("Setting response intercept enabled: {}", enabled);

    state.set_app_handle(app).await;

    let mut intercept = state.response_intercept_enabled.write().await;
    *intercept = enabled;

    if !enabled {
        let mut responses = state.intercepted_responses.write().await;
        let pending_responses: Vec<_> = responses.drain().collect();
        drop(responses);

        for (response_id, resp_internal) in pending_responses {
            let _ = resp_internal
                .response_tx
                .send(InterceptAction::Forward(None));
            tracing::info!(
                "Auto-forwarded intercepted response after disabling response intercept: {}",
                response_id
            );
        }
    }

    tracing::info!(
        "Response intercept mode {}",
        if enabled { "enabled" } else { "disabled" }
    );
    Ok(CommandResponse::ok(enabled))
}

#[tauri::command]
pub async fn get_response_intercept_enabled(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<bool>, String> {
    let enabled = *state.response_intercept_enabled.read().await;
    Ok(CommandResponse::ok(enabled))
}

#[tauri::command]
pub async fn get_intercepted_responses(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<Vec<InterceptedResponse>>, String> {
    let responses = state.intercepted_responses.read().await;
    let list: Vec<InterceptedResponse> = responses.values().map(|r| r.response.clone()).collect();
    Ok(CommandResponse::ok(list))
}

#[tauri::command]
pub async fn forward_intercepted_response(
    state: State<'_, TrafficAnalysisState>,
    response_id: String,
    modified_content: Option<String>,
) -> Result<CommandResponse<()>, String> {
    tracing::info!("Forwarding intercepted response: {}", response_id);

    let mut responses = state.intercepted_responses.write().await;
    if let Some(resp_internal) = responses.remove(&response_id) {
        let _ = resp_internal
            .response_tx
            .send(InterceptAction::Forward(modified_content));
        tracing::info!("Response forwarded: {}", response_id);
        Ok(CommandResponse::ok(()))
    } else {
        tracing::warn!("Response not found: {}", response_id);
        Ok(CommandResponse::err(format!(
            "Response not found: {}",
            response_id
        )))
    }
}

#[tauri::command]
pub async fn drop_intercepted_response(
    state: State<'_, TrafficAnalysisState>,
    response_id: String,
) -> Result<CommandResponse<()>, String> {
    tracing::info!("Dropping intercepted response: {}", response_id);

    let mut responses = state.intercepted_responses.write().await;
    if let Some(resp_internal) = responses.remove(&response_id) {
        let _ = resp_internal.response_tx.send(InterceptAction::Drop);
        tracing::info!("Response dropped: {}", response_id);
        Ok(CommandResponse::ok(()))
    } else {
        tracing::warn!("Response not found: {}", response_id);
        Ok(CommandResponse::err(format!(
            "Response not found: {}",
            response_id
        )))
    }
}

#[tauri::command]
pub async fn get_websocket_intercept_enabled(
    state: tauri::State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<bool>, String> {
    let enabled = *state.websocket_intercept_enabled.read().await;
    Ok(CommandResponse::ok(enabled))
}

#[tauri::command]
pub async fn set_websocket_intercept_enabled(
    state: tauri::State<'_, TrafficAnalysisState>,
    enabled: bool,
) -> Result<CommandResponse<()>, String> {
    let mut guard = state.websocket_intercept_enabled.write().await;
    *guard = enabled;
    tracing::info!("WebSocket intercept enabled set to: {}", enabled);
    Ok(CommandResponse::ok(()))
}

#[tauri::command]
pub async fn forward_intercepted_websocket(
    state: tauri::State<'_, TrafficAnalysisState>,
    id: String,
    content: Option<String>,
) -> Result<CommandResponse<()>, String> {
    let mut messages = state.intercepted_websocket_messages.write().await;
    if let Some(msg) = messages.remove(&id) {
        if msg
            .response_tx
            .send(InterceptAction::Forward(content))
            .is_err()
        {
            return Err("Failed to send forward action: receiver dropped".to_string());
        }
        tracing::info!("Forwarded intercepted WebSocket message: {}", id);
        Ok(CommandResponse::ok(()))
    } else {
        Err(format!("Intercepted message not found: {}", id))
    }
}

#[tauri::command]
pub async fn drop_intercepted_websocket(
    state: tauri::State<'_, TrafficAnalysisState>,
    id: String,
) -> Result<CommandResponse<()>, String> {
    let mut messages = state.intercepted_websocket_messages.write().await;
    if let Some(msg) = messages.remove(&id) {
        if msg.response_tx.send(InterceptAction::Drop).is_err() {
            return Err("Failed to send drop action: receiver dropped".to_string());
        }
        tracing::info!("Dropped intercepted WebSocket message: {}", id);
        Ok(CommandResponse::ok(()))
    } else {
        Err(format!("Intercepted message not found: {}", id))
    }
}

#[tauri::command]
pub async fn add_intercept_filter_rule(
    state: State<'_, TrafficAnalysisState>,
    rule: InterceptRuleConfig,
) -> Result<CommandResponse<InterceptRuleConfig>, String> {
    tracing::info!("Adding intercept filter rule: {:?}", rule);

    let db = state.get_db_service();
    let mut rules = match db
        .load_proxy_config(INTERCEPT_FILTER_RULES_CONFIG_KEY)
        .await
    {
        Ok(Some(json)) => serde_json::from_str::<InterceptRuleConfigSet>(&json).unwrap_or_default(),
        _ => InterceptRuleConfigSet::default(),
    };

    let new_rule = rule.with_new_id();

    rules.rules.push(new_rule.clone());

    let json = serde_json::to_string(&rules).map_err(|e| format!("Serialization error: {}", e))?;
    db.save_proxy_config(INTERCEPT_FILTER_RULES_CONFIG_KEY, &json)
        .await
        .map_err(|e| format!("Failed to save rules: {}", e))?;

    tracing::info!("Intercept filter rule added: {}", new_rule.id);
    Ok(CommandResponse::ok(new_rule))
}

#[tauri::command]
pub async fn get_intercept_filter_rules(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<Vec<InterceptRuleConfig>>, String> {
    let db = state.get_db_service();

    let rules = match db
        .load_proxy_config(INTERCEPT_FILTER_RULES_CONFIG_KEY)
        .await
    {
        Ok(Some(json)) => {
            serde_json::from_str::<InterceptRuleConfigSet>(&json)
                .unwrap_or_default()
                .rules
        }
        _ => Vec::new(),
    };

    Ok(CommandResponse::ok(rules))
}

#[tauri::command]
pub async fn remove_intercept_filter_rule(
    state: State<'_, TrafficAnalysisState>,
    rule_id: String,
) -> Result<CommandResponse<()>, String> {
    tracing::info!("Removing intercept filter rule: {}", rule_id);

    let db = state.get_db_service();
    let mut rules = match db
        .load_proxy_config(INTERCEPT_FILTER_RULES_CONFIG_KEY)
        .await
    {
        Ok(Some(json)) => serde_json::from_str::<InterceptRuleConfigSet>(&json).unwrap_or_default(),
        _ => InterceptRuleConfigSet::default(),
    };

    rules.rules.retain(|r| r.id != rule_id);

    let json = serde_json::to_string(&rules).map_err(|e| format!("Serialization error: {}", e))?;
    db.save_proxy_config(INTERCEPT_FILTER_RULES_CONFIG_KEY, &json)
        .await
        .map_err(|e| format!("Failed to save rules: {}", e))?;

    tracing::info!("Intercept filter rule removed: {}", rule_id);
    Ok(CommandResponse::ok(()))
}

#[tauri::command]
pub async fn update_intercept_filter_rule(
    state: State<'_, TrafficAnalysisState>,
    rule: InterceptRuleConfig,
) -> Result<CommandResponse<()>, String> {
    tracing::info!("Updating intercept filter rule: {:?}", rule);

    let db = state.get_db_service();
    let mut rules = match db
        .load_proxy_config(INTERCEPT_FILTER_RULES_CONFIG_KEY)
        .await
    {
        Ok(Some(json)) => serde_json::from_str::<InterceptRuleConfigSet>(&json).unwrap_or_default(),
        _ => InterceptRuleConfigSet::default(),
    };

    if let Some(existing) = rules.rules.iter_mut().find(|r| r.id == rule.id) {
        *existing = rule;
    } else {
        return Ok(CommandResponse::err(format!("Rule not found: {}", rule.id)));
    }

    let json = serde_json::to_string(&rules).map_err(|e| format!("Serialization error: {}", e))?;
    db.save_proxy_config(INTERCEPT_FILTER_RULES_CONFIG_KEY, &json)
        .await
        .map_err(|e| format!("Failed to save rules: {}", e))?;

    tracing::info!("Intercept filter rule updated");
    Ok(CommandResponse::ok(()))
}

#[tauri::command]
pub async fn update_runtime_filter_rules(
    state: State<'_, TrafficAnalysisState>,
    rule_type: String,
    rules: Vec<RuntimeInterceptRule>,
) -> Result<CommandResponse<()>, String> {
    tracing::info!(
        "Updating runtime {} filter rules: {} rules",
        rule_type,
        rules.len()
    );

    let traffic_rules: Vec<sentinel_traffic::InterceptFilterRule> =
        rules.iter().map(Into::into).collect();

    if rule_type == "request" {
        let mut guard = state.request_filter_rules.write().await;
        *guard = traffic_rules;
        tracing::info!("Request filter rules updated: {} rules", guard.len());
    } else if rule_type == "response" {
        let mut guard = state.response_filter_rules.write().await;
        *guard = traffic_rules;
        tracing::info!("Response filter rules updated: {} rules", guard.len());
    } else {
        return Ok(CommandResponse::err(format!(
            "Unknown rule type: {}",
            rule_type
        )));
    }

    let db = state.get_db_service();
    let mut all_rules = match db
        .load_proxy_config(INTERCEPT_FILTER_RULES_CONFIG_KEY)
        .await
    {
        Ok(Some(json)) => serde_json::from_str::<InterceptRuleConfigSet>(&json).unwrap_or_default(),
        _ => InterceptRuleConfigSet::default(),
    };

    all_rules.replace_runtime_rules(&rule_type, &rules);

    let json = serde_json::to_string(&all_rules)
        .map_err(|e| format!("Failed to serialize rules: {}", e))?;
    db.save_proxy_config(INTERCEPT_FILTER_RULES_CONFIG_KEY, &json)
        .await
        .map_err(|e| format!("Failed to persist rules: {}", e))?;

    tracing::info!(
        "Filter rules persisted to database: {} {} rules",
        rules.len(),
        rule_type
    );
    Ok(CommandResponse::ok(()))
}
