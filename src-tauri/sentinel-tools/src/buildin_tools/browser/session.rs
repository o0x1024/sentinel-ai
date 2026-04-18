use super::driver::{command_payload, BrowserDriverHandle, SharedBrowserDriver};
use super::{BrowserAction, BrowserToolArgs};
use anyhow::Result;
use once_cell::sync::Lazy;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

static SESSION_REGISTRY: Lazy<Mutex<HashMap<String, SharedBrowserDriver>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub async fn close_browser_session(session_id: &str) -> Result<bool> {
    if let Some(driver) = get_existing_driver(session_id).await {
        let payload = command_payload("close", true, Map::new());
        let _ = driver.send(payload).await;
        remove_session(session_id).await;
        driver.terminate().await;
        return Ok(true);
    }

    Ok(false)
}

pub async fn execute_browser_action(args: &BrowserToolArgs) -> Result<(String, Value)> {
    let session_id = args
        .session_id
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "default".to_string());
    let headless = args.headless.unwrap_or(true);

    let action_name = action_name(&args.action);
    if matches!(args.action, BrowserAction::Close) {
        if let Some(driver) = get_existing_driver(&session_id).await {
            let payload = command_payload(action_name, headless, build_fields(args));
            let result = driver.send(payload).await;
            remove_session(&session_id).await;
            driver.terminate().await;
            return result.map(|value| (session_id, value));
        }

        return Ok((
            session_id,
            serde_json::json!({ "closed": true, "existed": false }),
        ));
    }

    let driver = get_or_create_driver(&session_id, headless).await?;
    let payload = command_payload(action_name, headless, build_fields(args));
    driver.send(payload).await.map(|value| (session_id, value))
}

async fn get_or_create_driver(
    session_id: &str,
    headless: bool,
) -> Result<Arc<BrowserDriverHandle>> {
    let mut registry = SESSION_REGISTRY.lock().await;
    if let Some(existing) = registry.get(session_id) {
        return Ok(existing.clone());
    }

    let driver = Arc::new(BrowserDriverHandle::spawn(headless).await?);
    registry.insert(session_id.to_string(), driver.clone());
    Ok(driver)
}

async fn get_existing_driver(session_id: &str) -> Option<Arc<BrowserDriverHandle>> {
    let registry = SESSION_REGISTRY.lock().await;
    registry.get(session_id).cloned()
}

async fn remove_session(session_id: &str) {
    let mut registry = SESSION_REGISTRY.lock().await;
    registry.remove(session_id);
}

fn build_fields(args: &BrowserToolArgs) -> Map<String, Value> {
    let mut fields = Map::new();
    if let Some(url) = &args.url {
        fields.insert("url".to_string(), Value::String(url.clone()));
    }
    if let Some(selector) = &args.selector {
        fields.insert("selector".to_string(), Value::String(selector.clone()));
    }
    if let Some(value) = &args.value {
        fields.insert("value".to_string(), Value::String(value.clone()));
    }
    if let Some(script) = &args.script {
        fields.insert("script".to_string(), Value::String(script.clone()));
    }
    if let Some(timeout_secs) = args.timeout_secs {
        fields.insert(
            "timeout_secs".to_string(),
            Value::Number(timeout_secs.into()),
        );
    }
    if let Some(wait_until) = &args.wait_until {
        fields.insert("wait_until".to_string(), Value::String(wait_until.clone()));
    }
    if let Some(limit) = args.network_limit {
        fields.insert("network_limit".to_string(), Value::Number(limit.into()));
    }
    fields
}

fn action_name(action: &BrowserAction) -> &'static str {
    match action {
        BrowserAction::Launch => "launch",
        BrowserAction::Goto => "goto",
        BrowserAction::Snapshot => "snapshot",
        BrowserAction::Click => "click",
        BrowserAction::Fill => "fill",
        BrowserAction::Eval => "eval",
        BrowserAction::Cookies => "cookies",
        BrowserAction::NetworkLog => "network_log",
        BrowserAction::Close => "close",
    }
}
