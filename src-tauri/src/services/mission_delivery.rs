use std::sync::Arc;

use sentinel_core::models::mission::{Mission, MissionRun};
use sentinel_db::Database;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::services::database::DatabaseService;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionDeliveryPolicy {
    #[serde(default)]
    pub on_success: String,
    #[serde(default)]
    pub on_change: String,
    #[serde(default)]
    pub on_failure: String,
    #[serde(default)]
    pub primary: Option<MissionDeliveryTarget>,
    #[serde(default)]
    pub quiet_hours: Option<QuietHours>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionDeliveryTarget {
    pub kind: String,
    #[serde(default)]
    pub ref_data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuietHours {
    pub start_hour: u32,
    pub end_hour: u32,
    #[serde(default)]
    pub timezone: Option<String>,
}

/// Route and deliver mission run results based on delivery policy.
pub async fn deliver_mission_result(
    app_handle: &AppHandle,
    db: &Arc<DatabaseService>,
    mission: &Mission,
    run: &MissionRun,
    has_changes: bool,
) -> Result<(), String> {
    let policy_json = match &mission.delivery_policy_json {
        Some(json) if !json.is_empty() => json.clone(),
        _ => return Ok(()),
    };

    let policy: MissionDeliveryPolicy = serde_json::from_str(&policy_json)
        .map_err(|e| format!("Failed to parse delivery policy: {e}"))?;

    let target = match &policy.primary {
        Some(t) => t,
        None => return Ok(()),
    };

    // Check quiet hours
    if let Some(ref qh) = policy.quiet_hours {
        if is_quiet_hours(qh) {
            tracing::info!(
                "Mission {} run {}: delivery deferred (quiet hours)",
                mission.id,
                run.id
            );
            return Ok(());
        }
    }

    let should_deliver = match run.status.as_str() {
        "succeeded" => !policy.on_success.is_empty() && policy.on_success != "none",
        "partial" => !policy.on_success.is_empty() && policy.on_success != "none",
        "failed" | "timed_out" => !policy.on_failure.is_empty() && policy.on_failure != "none",
        _ => false,
    };

    let change_delivery = has_changes && !policy.on_change.is_empty() && policy.on_change != "none";

    if !should_deliver && !change_delivery {
        return Ok(());
    }

    let payload = build_delivery_payload(mission, run);
    let delivery_id = Uuid::new_v4().to_string();

    let target_json =
        serde_json::to_string(target).map_err(|e| format!("Failed to serialize target: {e}"))?;
    let payload_json =
        serde_json::to_string(&payload).map_err(|e| format!("Failed to serialize payload: {e}"))?;

    db.save_mission_delivery(
        &delivery_id,
        &mission.id,
        &run.id,
        &target_json,
        "pending",
        Some(&payload_json),
    )
    .await
    .map_err(|e| format!("Failed to save delivery: {e}"))?;

    let result = match target.kind.as_str() {
        "app_notification" => deliver_to_app_notification(app_handle, mission, run, &payload).await,
        "webhook" => deliver_to_webhook(target, &payload_json).await,
        "bot" => deliver_to_bot(db, target, &payload, run).await,
        "assistant_conversation" => {
            tracing::info!(
                "Mission {} delivery target=assistant_conversation (stored as observation)",
                mission.id
            );
            Ok(())
        }
        other => {
            tracing::warn!("Unknown delivery target kind: {other}");
            Err(format!("Unknown delivery target: {other}"))
        }
    };

    match result {
        Ok(()) => {
            let _ = db
                .update_mission_delivery_status(&delivery_id, "sent", None, None)
                .await;
        }
        Err(ref e) => {
            let _ = db
                .update_mission_delivery_status(&delivery_id, "failed", Some(e), None)
                .await;
        }
    }

    result
}

async fn deliver_to_bot(
    db: &Arc<DatabaseService>,
    target: &MissionDeliveryTarget,
    payload: &serde_json::Value,
    run: &MissionRun,
) -> Result<(), String> {
    let transport = target
        .ref_data
        .get("transport")
        .and_then(|v| v.as_str())
        .ok_or("Bot target missing transport")?;
    if transport != "weixin" {
        return Err(format!("Unsupported bot transport: {transport}"));
    }

    let account_id = target
        .ref_data
        .get("account_id")
        .and_then(|v| v.as_str())
        .ok_or("Bot target missing account_id")?;
    let peer_type = target
        .ref_data
        .get("peer_type")
        .and_then(|v| v.as_str())
        .ok_or("Bot target missing peer_type")?;
    let peer_id = target
        .ref_data
        .get("peer_id")
        .and_then(|v| v.as_str())
        .ok_or("Bot target missing peer_id")?;

    let config = load_weixin_config(db).await?;
    if config.account_id != account_id {
        return Err(format!(
            "Weixin delivery account mismatch: mission target={} configured={}",
            account_id, config.account_id
        ));
    }

    let text = format_bot_delivery_text(payload);
    crate::services::weixin_gateway::runtime::deliver_weixin_text(
        db,
        &config.account_id,
        &config.base_url,
        &config.token,
        peer_type,
        peer_id,
        &text,
        run.bot_execution_run_id
            .as_deref()
            .or(run.agent_execution_id.as_deref()),
    )
    .await
}

async fn load_weixin_config(
    db: &Arc<DatabaseService>,
) -> Result<crate::services::weixin_gateway::WeixinGatewayConfig, String> {
    let raw = db
        .get_config("network", "weixin_gateway_config")
        .await
        .map_err(|e| format!("Failed to load Weixin gateway config: {e}"))?
        .ok_or("Weixin gateway config not found")?;
    let mut config: crate::services::weixin_gateway::WeixinGatewayConfig =
        serde_json::from_str(&raw).map_err(|e| format!("Invalid Weixin gateway config: {e}"))?;
    config.normalize();
    config.validate_for_runtime()?;
    Ok(config)
}

fn format_bot_delivery_text(payload: &serde_json::Value) -> String {
    let title = payload
        .get("mission_title")
        .and_then(|v| v.as_str())
        .unwrap_or("Mission");
    let status = payload
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let summary = payload
        .get("result_summary")
        .and_then(|v| v.as_str())
        .or_else(|| payload.get("error_message").and_then(|v| v.as_str()))
        .unwrap_or("");

    if summary.trim().is_empty() {
        format!("任务「{}」执行完成，状态：{}", title, status)
    } else {
        format!("任务「{}」执行完成，状态：{}\n\n{}", title, status, summary)
    }
}

fn build_delivery_payload(mission: &Mission, run: &MissionRun) -> serde_json::Value {
    serde_json::json!({
        "mission_id": mission.id,
        "mission_title": mission.title,
        "run_id": run.id,
        "run_index": run.run_index,
        "status": run.status,
        "result_summary": run.result_summary,
        "error_message": run.error_message,
        "started_at": run.started_at,
        "completed_at": run.completed_at,
    })
}

async fn deliver_to_app_notification(
    app_handle: &AppHandle,
    mission: &Mission,
    run: &MissionRun,
    _payload: &serde_json::Value,
) -> Result<(), String> {
    let event_data = serde_json::json!({
        "type": "mission_run_completed",
        "mission_id": mission.id,
        "mission_title": mission.title,
        "run_id": run.id,
        "run_status": run.status,
        "result_summary": run.result_summary,
    });

    app_handle
        .emit("mission-notification", &event_data)
        .map_err(|e| format!("Failed to emit app notification: {e}"))?;

    Ok(())
}

async fn deliver_to_webhook(
    target: &MissionDeliveryTarget,
    payload_json: &str,
) -> Result<(), String> {
    let url = target
        .ref_data
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or("Webhook target missing 'url'")?;

    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .header("Content-Type", "application/json")
        .body(payload_json.to_string())
        .send()
        .await
        .map_err(|e| format!("Webhook POST failed: {e}"))?;

    if resp.status().is_success() {
        Ok(())
    } else {
        Err(format!("Webhook returned status: {}", resp.status()))
    }
}

fn is_quiet_hours(qh: &QuietHours) -> bool {
    let now = chrono::Local::now();
    let hour = now.format("%H").to_string().parse::<u32>().unwrap_or(0);

    if qh.start_hour <= qh.end_hour {
        hour >= qh.start_hour && hour < qh.end_hour
    } else {
        hour >= qh.start_hour || hour < qh.end_hour
    }
}
