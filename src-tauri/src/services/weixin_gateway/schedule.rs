use chrono::{DateTime, Local, Utc};
use cron::Schedule;
use sentinel_db::Database;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::database::{BotSchedule, BotScheduleRun};
use crate::services::database::DatabaseService;

const LEGACY_WEIXIN_SCHEDULE_CATEGORY: &str = "network";
const LEGACY_WEIXIN_SCHEDULE_KEY: &str = "weixin_schedule_registry";
const DEFAULT_TIMEZONE: &str = "Asia/Shanghai";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacyWeixinScheduleRecord {
    id: String,
    gateway_account_id: String,
    peer_type: String,
    peer_id: String,
    sender_id: String,
    assistant_profile_id: Option<String>,
    source_text: String,
    task: String,
    cron: String,
    timezone: String,
    enabled: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    last_run_at: Option<DateTime<Utc>>,
    next_run_at: Option<DateTime<Utc>>,
    last_error: Option<String>,
}

impl LegacyWeixinScheduleRecord {
    fn normalize(&mut self) -> Result<(), String> {
        self.id = self.id.trim().to_string();
        self.gateway_account_id = self.gateway_account_id.trim().to_string();
        self.peer_type = self.peer_type.trim().to_string();
        self.peer_id = self.peer_id.trim().to_string();
        self.sender_id = self.sender_id.trim().to_string();
        self.assistant_profile_id = normalize_optional_string(self.assistant_profile_id.clone());
        self.source_text = self.source_text.trim().to_string();
        self.task = self.task.trim().to_string();
        self.cron = normalize_cron_expression(&self.cron)?;
        self.timezone = normalize_timezone(&self.timezone);
        if self.enabled && self.next_run_at.is_none() {
            self.next_run_at = Some(calculate_next_run_at(&self.cron, Utc::now())?);
        }
        Ok(())
    }

    fn into_bot_schedule(self) -> Result<BotSchedule, String> {
        let mut schedule = BotSchedule {
            id: self.id,
            transport: "weixin".to_string(),
            account_id: self.gateway_account_id,
            peer_type: self.peer_type,
            peer_id: self.peer_id,
            sender_id: self.sender_id,
            assistant_profile_id: normalize_optional_string(self.assistant_profile_id),
            source_text: self.source_text,
            task_text: self.task,
            cron_expr: normalize_cron_expression(&self.cron)?,
            timezone: normalize_timezone(&self.timezone),
            enabled: self.enabled,
            last_run_at: self.last_run_at,
            next_run_at: self.next_run_at,
            last_error: normalize_optional_string(self.last_error),
            created_at: self.created_at,
            updated_at: self.updated_at,
        };
        normalize_bot_schedule(&mut schedule)?;
        Ok(schedule)
    }
}

pub fn new_weixin_schedule(
    account_id: String,
    peer_type: String,
    peer_id: String,
    sender_id: String,
    assistant_profile_id: Option<String>,
    source_text: String,
    task_text: String,
    cron_expr: String,
) -> Result<BotSchedule, String> {
    let now = Utc::now();
    let normalized_cron = normalize_cron_expression(&cron_expr)?;
    Ok(BotSchedule {
        id: format!("bts_{}", Uuid::new_v4().simple()),
        transport: "weixin".to_string(),
        account_id: account_id.trim().to_string(),
        peer_type: peer_type.trim().to_string(),
        peer_id: peer_id.trim().to_string(),
        sender_id: sender_id.trim().to_string(),
        assistant_profile_id: normalize_optional_string(assistant_profile_id),
        source_text: source_text.trim().to_string(),
        task_text: task_text.trim().to_string(),
        cron_expr: normalized_cron.clone(),
        timezone: DEFAULT_TIMEZONE.to_string(),
        enabled: true,
        last_run_at: None,
        next_run_at: Some(calculate_next_run_at(&normalized_cron, now)?),
        last_error: None,
        created_at: now,
        updated_at: now,
    })
}

pub fn normalize_bot_schedule(schedule: &mut BotSchedule) -> Result<(), String> {
    schedule.id = schedule.id.trim().to_string();
    schedule.transport = schedule.transport.trim().to_string();
    schedule.account_id = schedule.account_id.trim().to_string();
    schedule.peer_type = schedule.peer_type.trim().to_string();
    schedule.peer_id = schedule.peer_id.trim().to_string();
    schedule.sender_id = schedule.sender_id.trim().to_string();
    schedule.assistant_profile_id =
        normalize_optional_string(schedule.assistant_profile_id.clone());
    schedule.source_text = schedule.source_text.trim().to_string();
    schedule.task_text = schedule.task_text.trim().to_string();
    schedule.cron_expr = normalize_cron_expression(&schedule.cron_expr)?;
    schedule.timezone = normalize_timezone(&schedule.timezone);
    schedule.last_error = normalize_optional_string(schedule.last_error.clone());
    if schedule.enabled && schedule.next_run_at.is_none() {
        schedule.next_run_at = Some(calculate_next_run_at(&schedule.cron_expr, Utc::now())?);
    }
    if !schedule.enabled {
        schedule.next_run_at = None;
    }
    Ok(())
}

pub fn new_weixin_schedule_run(schedule: &BotSchedule) -> BotScheduleRun {
    let now = Utc::now();
    BotScheduleRun {
        id: format!("bsr_{}", Uuid::new_v4().simple()),
        schedule_id: schedule.id.clone(),
        transport: schedule.transport.clone(),
        account_id: schedule.account_id.clone(),
        peer_type: schedule.peer_type.clone(),
        peer_id: schedule.peer_id.clone(),
        sender_id: schedule.sender_id.clone(),
        execution_run_id: None,
        status: "running".to_string(),
        result_text: None,
        error_message: None,
        triggered_at: now,
        completed_at: None,
        created_at: now,
        updated_at: now,
    }
}

pub async fn list_weixin_schedules_for_account(
    db: &Arc<DatabaseService>,
    account_id: &str,
) -> Result<Vec<BotSchedule>, String> {
    db.list_bot_schedules_by_transport_account("weixin", account_id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn list_weixin_schedules_for_peer(
    db: &Arc<DatabaseService>,
    account_id: &str,
    peer_type: &str,
    peer_id: &str,
) -> Result<Vec<BotSchedule>, String> {
    db.list_bot_schedules_for_peer("weixin", account_id, peer_type, peer_id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn get_weixin_schedule(
    db: &Arc<DatabaseService>,
    schedule_id: &str,
) -> Result<Option<BotSchedule>, String> {
    db.get_bot_schedule(schedule_id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn upsert_weixin_schedule(
    db: &Arc<DatabaseService>,
    schedule: &BotSchedule,
) -> Result<(), String> {
    db.upsert_bot_schedule(schedule)
        .await
        .map_err(|e| e.to_string())
}

pub async fn delete_weixin_schedule(
    db: &Arc<DatabaseService>,
    schedule_id: &str,
) -> Result<(), String> {
    db.delete_bot_schedule(schedule_id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn create_weixin_schedule_run(
    db: &Arc<DatabaseService>,
    schedule_run: &BotScheduleRun,
) -> Result<(), String> {
    db.create_bot_schedule_run(schedule_run)
        .await
        .map_err(|e| e.to_string())
}

pub async fn finalize_weixin_schedule_run(
    db: &Arc<DatabaseService>,
    schedule_run_id: &str,
    execution_run_id: Option<&str>,
    status: &str,
    result_text: Option<&str>,
    error_message: Option<&str>,
) -> Result<(), String> {
    db.update_bot_schedule_run_result(
        schedule_run_id,
        execution_run_id,
        status,
        result_text,
        error_message,
        Utc::now(),
    )
    .await
    .map_err(|e| e.to_string())
}

pub async fn migrate_legacy_weixin_schedule_registry(
    db: &Arc<DatabaseService>,
) -> Result<(), String> {
    let Some(raw) = db
        .get_config(LEGACY_WEIXIN_SCHEDULE_CATEGORY, LEGACY_WEIXIN_SCHEDULE_KEY)
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(());
    };

    let mut legacy: Vec<LegacyWeixinScheduleRecord> =
        serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    for item in &mut legacy {
        item.normalize()?;
        let schedule = item.clone().into_bot_schedule()?;
        db.upsert_bot_schedule(&schedule)
            .await
            .map_err(|e| e.to_string())?;
    }

    db.execute_query(&format!(
        "DELETE FROM configurations WHERE category = '{}' AND key = '{}'",
        LEGACY_WEIXIN_SCHEDULE_CATEGORY, LEGACY_WEIXIN_SCHEDULE_KEY
    ))
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn normalize_cron_expression(raw: &str) -> Result<String, String> {
    let mut parts = raw
        .split_whitespace()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if parts.len() == 5 {
        parts.insert(0, "0".to_string());
    }
    if parts.len() != 6 {
        return Err("cron expression must contain 5 or 6 fields".to_string());
    }
    if parts[3] == "?" {
        parts[3] = "*".to_string();
    }
    if parts[5] == "?" {
        parts[5] = "*".to_string();
    }
    let normalized = parts.join(" ");
    Schedule::from_str(&normalized).map_err(|e| format!("invalid cron expression: {e}"))?;
    Ok(normalized)
}

pub fn calculate_next_run_at(
    cron_expr: &str,
    after: DateTime<Utc>,
) -> Result<DateTime<Utc>, String> {
    let normalized = normalize_cron_expression(cron_expr)?;
    let schedule =
        Schedule::from_str(&normalized).map_err(|e| format!("invalid cron expression: {e}"))?;
    let after_local = after.with_timezone(&Local) + chrono::Duration::seconds(1);
    schedule
        .after(&after_local)
        .next()
        .map(|value| value.with_timezone(&Utc))
        .ok_or_else(|| "cron expression did not produce a next run time".to_string())
}

pub fn format_schedule_timestamp(value: Option<DateTime<Utc>>) -> String {
    value
        .map(|timestamp| {
            timestamp
                .with_timezone(&Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        })
        .unwrap_or_else(|| "-".to_string())
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
}

fn normalize_timezone(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        DEFAULT_TIMEZONE.to_string()
    } else {
        trimmed.to_string()
    }
}
