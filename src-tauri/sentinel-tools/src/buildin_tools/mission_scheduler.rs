use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use once_cell::sync::Lazy;
use rig::tool::Tool;
use schemars::JsonSchema;
use sentinel_core::models::mission::{
    CreateMissionRequest, ListMissionsFilter, Mission, UpdateMissionFieldsRequest,
};
use sentinel_db::DatabaseService;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tokio::sync::RwLock;

static APP_HANDLE: Lazy<RwLock<Option<AppHandle>>> = Lazy::new(|| RwLock::new(None));

pub async fn set_mission_scheduler_app_handle(handle: AppHandle) {
    let mut h = APP_HANDLE.write().await;
    *h = Some(handle);
}

async fn get_db_service() -> Option<Arc<DatabaseService>> {
    let handle = APP_HANDLE.read().await;
    h_try_db(handle.as_ref())
}

fn h_try_db(handle: Option<&AppHandle>) -> Option<Arc<DatabaseService>> {
    handle.and_then(|h| {
        h.try_state::<Arc<DatabaseService>>()
            .map(|state| state.inner().clone())
    })
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MissionSchedulerAction {
    CreateMission,
    UpdateMission,
    ListMissions,
    PauseMission,
    ResumeMission,
    DeleteMission,
    RunMissionNow,
    ValidateCron,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct MissionTriggerInput {
    pub kind: String,
    pub cron_expr: Option<String>,
    pub interval_seconds: Option<i64>,
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct MissionDeliveryInput {
    pub kind: String,
    pub transport: Option<String>,
    pub account_id: Option<String>,
    pub peer_type: Option<String>,
    pub peer_id: Option<String>,
    pub on_success: Option<String>,
    pub on_change: Option<String>,
    pub on_failure: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct MissionBudgetInput {
    pub max_runs_per_day: Option<i64>,
    pub timeout_seconds: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct MissionSchedulerArgs {
    pub action: MissionSchedulerAction,
    pub execution_id: Option<String>,
    pub mission_id: Option<String>,
    pub title: Option<String>,
    pub objective: Option<String>,
    pub trigger: Option<MissionTriggerInput>,
    pub delivery: Option<MissionDeliveryInput>,
    pub context_strategy: Option<String>,
    pub assistant_profile_id: Option<String>,
    pub budget: Option<MissionBudgetInput>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub cron_expr: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MissionSchedulerOutput {
    pub success: bool,
    pub message: String,
    pub mission: Option<Mission>,
    pub missions: Option<Vec<Mission>>,
    pub run_id: Option<String>,
    pub next_run_at: Option<DateTime<Utc>>,
}

#[derive(Debug, thiserror::Error)]
pub enum MissionSchedulerError {
    #[error("Mission scheduler tool is not initialized")]
    MissingAppHandle,
    #[error("{0}")]
    InvalidInput(String),
    #[error("Database operation failed: {0}")]
    Database(String),
}

#[derive(Default)]
pub struct MissionSchedulerTool;

impl MissionSchedulerTool {
    pub const NAME: &'static str = "mission_scheduler";
    pub const DESCRIPTION: &'static str = concat!(
        "Create and manage durable scheduled Missions. Use this when the user asks for recurring, ",
        "periodic, daily, weekly, monthly, long-running, monitoring, subscription, or scheduled delivery work. ",
        "For bot conversations, pass execution_id so the tool can bind the Mission owner and delivery target ",
        "to the current bot chat. For corrections or changes to an existing scheduled task, list missions first ",
        "and call update_mission with the existing mission_id instead of creating another Mission. Do not claim ",
        "a scheduled task was created or changed unless this tool succeeds."
    );
}

impl Tool for MissionSchedulerTool {
    const NAME: &'static str = Self::NAME;
    type Args = MissionSchedulerArgs;
    type Output = MissionSchedulerOutput;
    type Error = MissionSchedulerError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(MissionSchedulerArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let db = get_db_service()
            .await
            .ok_or(MissionSchedulerError::MissingAppHandle)?;

        match args.action {
            MissionSchedulerAction::CreateMission => create_mission(db, args).await,
            MissionSchedulerAction::UpdateMission => update_mission(db, args).await,
            MissionSchedulerAction::ListMissions => list_missions(db, args).await,
            MissionSchedulerAction::PauseMission => update_status(db, args, "paused").await,
            MissionSchedulerAction::ResumeMission => update_status(db, args, "active").await,
            MissionSchedulerAction::DeleteMission => delete_mission(db, args).await,
            MissionSchedulerAction::RunMissionNow => run_mission_now(db, args).await,
            MissionSchedulerAction::ValidateCron => validate_cron(args),
        }
    }
}

async fn update_mission(
    db: Arc<DatabaseService>,
    args: MissionSchedulerArgs,
) -> Result<MissionSchedulerOutput, MissionSchedulerError> {
    let id = required_text(args.mission_id.as_deref(), "mission_id")?.to_string();

    let bot_context = match args.execution_id.as_deref() {
        Some(execution_id) => resolve_bot_context(&db, execution_id).await?,
        None => None,
    };

    let (trigger_json, next_run_at) = if let Some(trigger) = args.trigger.as_ref() {
        let trigger_json = build_trigger_json(trigger)?;
        let next_run_at = calculate_next_run(&trigger_json)?;
        (Some(trigger_json), next_run_at)
    } else {
        (None, None)
    };

    let delivery_policy_json = if args.delivery.is_some() {
        Some(build_delivery_policy_json(
            args.delivery.as_ref(),
            bot_context.as_ref(),
        )?)
    } else {
        None
    };

    let context_strategy_json = args
        .context_strategy
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|mode| serde_json::json!({ "mode": mode }).to_string());

    let budget_json = args.budget.as_ref().map(|budget| {
        serde_json::json!({
            "max_runs_per_day": budget.max_runs_per_day.unwrap_or(1),
            "timeout_seconds": budget.timeout_seconds.unwrap_or(300),
        })
        .to_string()
    });

    let update = UpdateMissionFieldsRequest {
        id: id.clone(),
        title: clean_optional_text(args.title),
        objective: clean_optional_text(args.objective),
        trigger_json,
        delivery_policy_json,
        assistant_profile_id: clean_optional_text(args.assistant_profile_id),
        step_plan_json: None,
        success_criteria_json: None,
        context_strategy_json,
        budget_json,
        failure_policy_json: None,
        missed_run_policy: None,
        next_run_at,
    };

    let mut mission = db
        .update_mission_fields(update)
        .await
        .map_err(|e| MissionSchedulerError::Database(e.to_string()))?;

    if let Some(status) = clean_optional_text(args.status) {
        mission = db
            .update_mission_status(&id, &status)
            .await
            .map_err(|e| MissionSchedulerError::Database(e.to_string()))?;
    } else if mission.status == "draft" && mission.next_run_at.is_some() {
        mission = db
            .update_mission_status(&id, "active")
            .await
            .map_err(|e| MissionSchedulerError::Database(e.to_string()))?;
    }

    Ok(MissionSchedulerOutput {
        success: true,
        message: format!(
            "Mission updated: {} (status={}, next_run_at={})",
            mission.id,
            mission.status,
            mission
                .next_run_at
                .map(|t| t.to_rfc3339())
                .unwrap_or_else(|| "manual".to_string())
        ),
        mission: Some(mission.clone()),
        missions: None,
        run_id: None,
        next_run_at: mission.next_run_at,
    })
}

async fn create_mission(
    db: Arc<DatabaseService>,
    args: MissionSchedulerArgs,
) -> Result<MissionSchedulerOutput, MissionSchedulerError> {
    let title = required_text(args.title.as_deref(), "title")?;
    let objective = required_text(args.objective.as_deref(), "objective")?;
    let trigger = args
        .trigger
        .ok_or_else(|| MissionSchedulerError::InvalidInput("trigger is required".to_string()))?;
    let trigger_json = build_trigger_json(&trigger)?;
    let next_run_at = calculate_next_run(&trigger_json)?;

    let bot_context = match args.execution_id.as_deref() {
        Some(id) => resolve_bot_context(&db, id).await?,
        None => None,
    };

    let (owner_kind, owner_ref) = if let Some(ctx) = bot_context.as_ref() {
        (
            "bot_peer".to_string(),
            format!(
                "{}:{}:{}:{}",
                ctx.transport, ctx.account_id, ctx.peer_type, ctx.peer_id
            ),
        )
    } else {
        ("user".to_string(), "default".to_string())
    };

    let delivery_policy_json =
        build_delivery_policy_json(args.delivery.as_ref(), bot_context.as_ref())?;
    let context_strategy = args
        .context_strategy
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("stateless");
    let budget = args.budget.unwrap_or(MissionBudgetInput {
        max_runs_per_day: Some(1),
        timeout_seconds: Some(300),
    });

    let request = CreateMissionRequest {
        title: title.to_string(),
        objective: objective.to_string(),
        owner_kind,
        owner_ref,
        source_json: Some(
            serde_json::json!({
                "source": "agent_tool",
                "tool": MissionSchedulerTool::NAME,
                "execution_id": args.execution_id,
            })
            .to_string(),
        ),
        delivery_policy_json: Some(delivery_policy_json),
        assistant_profile_id: args.assistant_profile_id.or_else(|| {
            bot_context
                .as_ref()
                .and_then(|ctx| ctx.assistant_profile_id.clone())
        }),
        trigger_json: Some(trigger_json),
        step_plan_json: None,
        success_criteria_json: None,
        context_strategy_json: Some(serde_json::json!({ "mode": context_strategy }).to_string()),
        budget_json: Some(
            serde_json::json!({
                "max_runs_per_day": budget.max_runs_per_day.unwrap_or(1),
                "timeout_seconds": budget.timeout_seconds.unwrap_or(300),
            })
            .to_string(),
        ),
        failure_policy_json: None,
        missed_run_policy: "skip".to_string(),
        next_run_at,
    };

    let mission = db
        .create_mission(request)
        .await
        .map_err(|e| MissionSchedulerError::Database(e.to_string()))?;

    let mission = if next_run_at.is_some() {
        db.update_mission_status(&mission.id, "active")
            .await
            .map_err(|e| MissionSchedulerError::Database(e.to_string()))?
    } else {
        mission
    };

    Ok(MissionSchedulerOutput {
        success: true,
        message: format!(
            "Mission created: {} (status={}, next_run_at={})",
            mission.id,
            mission.status,
            mission
                .next_run_at
                .map(|t| t.to_rfc3339())
                .unwrap_or_else(|| "manual".to_string())
        ),
        mission: Some(mission),
        missions: None,
        run_id: None,
        next_run_at,
    })
}

async fn list_missions(
    db: Arc<DatabaseService>,
    args: MissionSchedulerArgs,
) -> Result<MissionSchedulerOutput, MissionSchedulerError> {
    let bot_context = match args.execution_id.as_deref() {
        Some(id) => resolve_bot_context(&db, id).await?,
        None => None,
    };
    let filter = if let Some(ctx) = bot_context {
        ListMissionsFilter {
            owner_kind: Some("bot_peer".to_string()),
            owner_ref: Some(format!(
                "{}:{}:{}:{}",
                ctx.transport, ctx.account_id, ctx.peer_type, ctx.peer_id
            )),
            status: args.status,
            limit: args.limit.unwrap_or(50),
            offset: 0,
        }
    } else {
        ListMissionsFilter {
            owner_kind: None,
            owner_ref: None,
            status: args.status,
            limit: args.limit.unwrap_or(50),
            offset: 0,
        }
    };

    let missions = db
        .list_missions(&filter)
        .await
        .map_err(|e| MissionSchedulerError::Database(e.to_string()))?;
    Ok(MissionSchedulerOutput {
        success: true,
        message: format!("Loaded {} missions", missions.len()),
        mission: None,
        missions: Some(missions),
        run_id: None,
        next_run_at: None,
    })
}

async fn update_status(
    db: Arc<DatabaseService>,
    args: MissionSchedulerArgs,
    status: &str,
) -> Result<MissionSchedulerOutput, MissionSchedulerError> {
    let id = required_text(args.mission_id.as_deref(), "mission_id")?;
    let mission = db
        .update_mission_status(id, status)
        .await
        .map_err(|e| MissionSchedulerError::Database(e.to_string()))?;
    Ok(MissionSchedulerOutput {
        success: true,
        message: format!(
            "Mission {} status updated to {}",
            mission.id, mission.status
        ),
        mission: Some(mission),
        missions: None,
        run_id: None,
        next_run_at: None,
    })
}

async fn delete_mission(
    db: Arc<DatabaseService>,
    args: MissionSchedulerArgs,
) -> Result<MissionSchedulerOutput, MissionSchedulerError> {
    let id = required_text(args.mission_id.as_deref(), "mission_id")?;
    db.delete_mission(id)
        .await
        .map_err(|e| MissionSchedulerError::Database(e.to_string()))?;
    Ok(MissionSchedulerOutput {
        success: true,
        message: format!("Mission {id} deleted"),
        mission: None,
        missions: None,
        run_id: None,
        next_run_at: None,
    })
}

async fn run_mission_now(
    db: Arc<DatabaseService>,
    args: MissionSchedulerArgs,
) -> Result<MissionSchedulerOutput, MissionSchedulerError> {
    let id = required_text(args.mission_id.as_deref(), "mission_id")?;
    let run = db
        .create_mission_run(id, "manual")
        .await
        .map_err(|e| MissionSchedulerError::Database(e.to_string()))?;
    Ok(MissionSchedulerOutput {
        success: true,
        message: format!("Mission run queued: {}", run.id),
        mission: None,
        missions: None,
        run_id: Some(run.id),
        next_run_at: None,
    })
}

fn validate_cron(
    args: MissionSchedulerArgs,
) -> Result<MissionSchedulerOutput, MissionSchedulerError> {
    let cron_expr = required_text(args.cron_expr.as_deref(), "cron_expr")?;
    let timezone = parse_timezone(args.trigger.as_ref().and_then(|trigger| {
        trigger
            .timezone
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }))?;
    let schedule = cron_expr.parse::<cron::Schedule>().map_err(|e| {
        MissionSchedulerError::InvalidInput(format!("Invalid cron expression: {e}"))
    })?;
    let next = schedule
        .upcoming(timezone)
        .next()
        .ok_or_else(|| MissionSchedulerError::InvalidInput("No upcoming cron time".to_string()))?
        .with_timezone(&Utc);
    Ok(MissionSchedulerOutput {
        success: true,
        message: format!("Cron is valid. Next run: {}", next.to_rfc3339()),
        mission: None,
        missions: None,
        run_id: None,
        next_run_at: Some(next),
    })
}

fn clean_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn required_text<'a>(
    value: Option<&'a str>,
    field: &str,
) -> Result<&'a str, MissionSchedulerError> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| MissionSchedulerError::InvalidInput(format!("{field} is required")))
}

fn build_trigger_json(trigger: &MissionTriggerInput) -> Result<String, MissionSchedulerError> {
    match trigger.kind.trim() {
        "cron" => {
            let cron_expr = required_text(trigger.cron_expr.as_deref(), "trigger.cron_expr")?;
            let timezone = trigger
                .timezone
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("Asia/Shanghai");
            parse_timezone(Some(timezone))?;
            cron_expr.parse::<cron::Schedule>().map_err(|e| {
                MissionSchedulerError::InvalidInput(format!("Invalid cron expression: {e}"))
            })?;
            Ok(serde_json::json!({
                "kind": "cron",
                "cron_expr": cron_expr,
                "timezone": timezone,
            })
            .to_string())
        }
        "interval" => {
            let seconds = trigger.interval_seconds.unwrap_or(0);
            if seconds <= 0 {
                return Err(MissionSchedulerError::InvalidInput(
                    "trigger.interval_seconds must be positive".to_string(),
                ));
            }
            if seconds >= 86_400 {
                return Err(MissionSchedulerError::InvalidInput(
                    "Use a cron trigger for daily-or-longer fixed-time recurring tasks".to_string(),
                ));
            }
            Ok(serde_json::json!({
                "kind": "interval",
                "interval_seconds": seconds,
            })
            .to_string())
        }
        "manual" => Ok(serde_json::json!({ "kind": "manual" }).to_string()),
        other => Err(MissionSchedulerError::InvalidInput(format!(
            "Unsupported trigger kind: {other}"
        ))),
    }
}

fn calculate_next_run(trigger_json: &str) -> Result<Option<DateTime<Utc>>, MissionSchedulerError> {
    let trigger: serde_json::Value = serde_json::from_str(trigger_json)
        .map_err(|e| MissionSchedulerError::InvalidInput(format!("Invalid trigger JSON: {e}")))?;
    match trigger
        .get("kind")
        .and_then(|v| v.as_str())
        .unwrap_or("manual")
    {
        "cron" => {
            let cron_expr = required_text(
                trigger.get("cron_expr").and_then(|v| v.as_str()),
                "trigger.cron_expr",
            )?;
            let timezone = parse_timezone(trigger.get("timezone").and_then(|v| v.as_str()))?;
            let schedule = cron_expr.parse::<cron::Schedule>().map_err(|e| {
                MissionSchedulerError::InvalidInput(format!("Invalid cron expression: {e}"))
            })?;
            Ok(schedule
                .upcoming(timezone)
                .next()
                .map(|next| next.with_timezone(&Utc)))
        }
        "interval" => {
            let seconds = trigger
                .get("interval_seconds")
                .and_then(|v| v.as_i64())
                .unwrap_or(3600);
            Ok(Some(Utc::now() + chrono::Duration::seconds(seconds)))
        }
        _ => Ok(None),
    }
}

fn parse_timezone(timezone: Option<&str>) -> Result<Tz, MissionSchedulerError> {
    timezone
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("Asia/Shanghai")
        .parse::<Tz>()
        .map_err(|e| MissionSchedulerError::InvalidInput(format!("Invalid timezone: {e}")))
}

#[derive(Debug, Clone)]
struct BotExecutionContext {
    transport: String,
    account_id: String,
    peer_type: String,
    peer_id: String,
    assistant_profile_id: Option<String>,
}

async fn resolve_bot_context(
    db: &Arc<DatabaseService>,
    execution_id: &str,
) -> Result<Option<BotExecutionContext>, MissionSchedulerError> {
    let run = db
        .get_bot_execution_run(execution_id)
        .await
        .map_err(|e| MissionSchedulerError::Database(e.to_string()))?;
    Ok(run.map(|run| BotExecutionContext {
        transport: run.transport,
        account_id: run.account_id,
        peer_type: run.peer_type,
        peer_id: run.peer_id,
        assistant_profile_id: run.assistant_profile_id,
    }))
}

fn build_delivery_policy_json(
    delivery: Option<&MissionDeliveryInput>,
    bot_context: Option<&BotExecutionContext>,
) -> Result<String, MissionSchedulerError> {
    let on_success = delivery
        .and_then(|d| d.on_success.as_deref())
        .unwrap_or("summary");
    let on_change = delivery
        .and_then(|d| d.on_change.as_deref())
        .unwrap_or("immediate");
    let on_failure = delivery
        .and_then(|d| d.on_failure.as_deref())
        .unwrap_or("immediate");
    let kind = delivery.map(|d| d.kind.as_str()).unwrap_or("bot");

    let primary = match kind {
        "bot" => {
            let transport = delivery
                .and_then(|d| d.transport.as_deref())
                .or_else(|| bot_context.map(|ctx| ctx.transport.as_str()))
                .ok_or_else(|| {
                    MissionSchedulerError::InvalidInput(
                        "bot delivery requires transport or execution_id".to_string(),
                    )
                })?;
            let account_id = delivery
                .and_then(|d| d.account_id.as_deref())
                .or_else(|| bot_context.map(|ctx| ctx.account_id.as_str()))
                .ok_or_else(|| {
                    MissionSchedulerError::InvalidInput(
                        "bot delivery requires account_id or execution_id".to_string(),
                    )
                })?;
            let peer_type = delivery
                .and_then(|d| d.peer_type.as_deref())
                .or_else(|| bot_context.map(|ctx| ctx.peer_type.as_str()))
                .ok_or_else(|| {
                    MissionSchedulerError::InvalidInput(
                        "bot delivery requires peer_type or execution_id".to_string(),
                    )
                })?;
            let peer_id = delivery
                .and_then(|d| d.peer_id.as_deref())
                .or_else(|| bot_context.map(|ctx| ctx.peer_id.as_str()))
                .ok_or_else(|| {
                    MissionSchedulerError::InvalidInput(
                        "bot delivery requires peer_id or execution_id".to_string(),
                    )
                })?;
            serde_json::json!({
                "kind": "bot",
                "ref_data": {
                    "transport": transport,
                    "account_id": account_id,
                    "peer_type": peer_type,
                    "peer_id": peer_id,
                }
            })
        }
        "app_notification" => serde_json::json!({
            "kind": "app_notification",
            "ref_data": {},
        }),
        other => {
            return Err(MissionSchedulerError::InvalidInput(format!(
                "Unsupported delivery kind: {other}"
            )))
        }
    };

    Ok(serde_json::json!({
        "on_success": on_success,
        "on_change": on_change,
        "on_failure": on_failure,
        "primary": primary,
    })
    .to_string())
}
