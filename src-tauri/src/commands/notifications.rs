use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tauri::State;
use sentinel_core::models::database::NotificationRule;
use crate::services::database::DatabaseService;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NotificationMessage {
    pub title: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SendNotificationPayload {
    pub channel: String,
    pub config: Value,
    pub message: NotificationMessage,
}

#[tauri::command]
pub async fn send_notification(payload: SendNotificationPayload) -> Result<bool, String> {
    match sentinel_notify::send(&payload.channel, payload.config.clone(),
        sentinel_notify::NotificationMessage { title: payload.message.title.clone(), content: payload.message.content.clone() }
    ).await {
        Ok(_) => Ok(true),
        Err(e) => Err(e.to_string()),
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TestNotificationRuleRequest {
    pub id: Option<String>,
    pub channel: Option<String>,
    pub config: Option<Value>,
}

#[tauri::command]
pub async fn test_notification_rule_connection(
    db_service: State<'_, Arc<DatabaseService>>, 
    request: TestNotificationRuleRequest
) -> Result<bool, String> {
    let mut channel = String::new();
    let mut config = serde_json::json!({});
    let mut title = String::from("Sentinel AI 通知测试");
    let mut content = String::from("这是一条测试消息，用于验证连接状态");

    if let Some(id) = request.id.as_ref() {
        let rule_opt = db_service.get_notification_rule(id).await.map_err(|e| e.to_string())?;
        if let Some(rule) = rule_opt {
            channel = rule.channel.clone();
            if let Some(cfg_str) = rule.config.clone() {
                config = serde_json::from_str(&cfg_str).unwrap_or_else(|_| serde_json::json!({}));
            }
            title = format!("通知规则测试: {}", rule.name);
            content = format!("通道: {}", rule.channel);
        } else {
            return Err(String::from("notification_rule_not_found"));
        }
    } else {
        if let Some(ch) = request.channel.as_ref() { channel = ch.clone(); } else { return Err(String::from("missing_channel")); }
        if let Some(cfg) = request.config.as_ref() { config = cfg.clone(); } else { config = serde_json::json!({}); }
    }

    match sentinel_notify::send(&channel, config, sentinel_notify::NotificationMessage { title, content }).await {
        Ok(_) => Ok(true),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn create_notification_rule(
    db_service: State<'_, Arc<DatabaseService>>, 
    rule: NotificationRule
) -> Result<bool, String> {
    db_service.create_notification_rule(&rule).await.map(|_| true).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_notification_rule(
    db_service: State<'_, Arc<DatabaseService>>, 
    rule: NotificationRule
) -> Result<bool, String> {
    db_service.update_notification_rule(&rule).await.map(|_| true).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_notification_rule(
    db_service: State<'_, Arc<DatabaseService>>, 
    id: String
) -> Result<bool, String> {
    db_service.delete_notification_rule(&id).await.map(|_| true).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_notification_rules(
    db_service: State<'_, Arc<DatabaseService>>
) -> Result<Vec<NotificationRule>, String> {
    db_service.get_notification_rules().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_notification_rule(
    db_service: State<'_, Arc<DatabaseService>>, 
    id: String
) -> Result<Option<NotificationRule>, String> {
    db_service.get_notification_rule(&id).await.map_err(|e| e.to_string())
}
