use std::sync::Arc;

use sentinel_core::models::mission::{Mission, MissionRun};
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
        "bot" => {
            tracing::info!(
                "Mission {} delivery target=bot (not yet wired to bot gateway)",
                mission.id
            );
            Ok(())
        }
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
