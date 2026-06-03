use std::sync::Arc;

use sentinel_core::models::mission::CreateMissionRequest;
use tauri::State;

use crate::services::database::DatabaseService;

/// T-11.1: Create a daily AI news digest mission for validation.
#[tauri::command]
pub async fn mission_scenario_daily_news(
    owner_kind: String,
    owner_ref: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<serde_json::Value, String> {
    let trigger = serde_json::json!({
        "kind": "cron",
        "cron_expr": "0 9 * * *",
        "timezone": "UTC"
    });

    let delivery = serde_json::json!({
        "onSuccess": "summary",
        "onChange": "none",
        "onFailure": "immediate",
        "primary": {
            "kind": "app_notification",
            "refData": {}
        }
    });

    let budget = serde_json::json!({
        "max_runs_per_day": 1,
        "max_tool_calls_per_run": 20,
        "timeout_seconds": 300
    });

    let request = CreateMissionRequest {
        title: "Daily AI News Digest".to_string(),
        objective: "Search for the latest AI and machine learning news, summarize the top 5 stories with key insights, and deliver as a morning briefing.".to_string(),
        owner_kind,
        owner_ref,
        source_json: Some(serde_json::json!({"scenario": "daily_news"}).to_string()),
        delivery_policy_json: Some(delivery.to_string()),
        assistant_profile_id: None,
        trigger_json: Some(trigger.to_string()),
        mission_spec_json: Some(serde_json::json!({
            "kind": "daily_digest",
            "state_schema": { "type": "object" },
            "action_schema": { "type": "object" },
            "completion_policy": { "kind": "manual_or_agent_completion" },
            "report_policy": { "kind": "each_run_summary" }
        }).to_string()),
        step_plan_json: Some(serde_json::json!([
            {"description": "Search for latest AI news using web search"},
            {"description": "Fetch and read the top results"},
            {"description": "Summarize key stories with insights"},
            {"description": "Format as a morning briefing"}
        ]).to_string()),
        success_criteria_json: Some(serde_json::json!({
            "rules": [{"kind": "artifact_produced", "artifact_type": "news_digest", "min_count": 1}],
            "aggregation": "all"
        }).to_string()),
        context_strategy_json: Some(serde_json::json!({
            "mode": "stateless"
        }).to_string()),
        budget_json: Some(budget.to_string()),
        failure_policy_json: None,
        missed_run_policy: "skip".to_string(),
        next_run_at: None,
    };

    let mission = db_service
        .create_mission(request)
        .await
        .map_err(|e| format!("Failed to create scenario mission: {e}"))?;

    Ok(serde_json::json!({
        "mission_id": mission.id,
        "title": mission.title,
        "status": mission.status,
        "scenario": "daily_ai_news_digest"
    }))
}

/// T-11.2: Create a website change monitoring mission for validation.
#[tauri::command]
pub async fn mission_scenario_website_monitor(
    owner_kind: String,
    owner_ref: String,
    target_url: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<serde_json::Value, String> {
    let trigger = serde_json::json!({
        "kind": "cron",
        "cron_expr": "0 * * * *",
        "timezone": "UTC"
    });

    let delivery = serde_json::json!({
        "onSuccess": "none",
        "onChange": "immediate",
        "onFailure": "immediate",
        "primary": {
            "kind": "app_notification",
            "refData": {}
        }
    });

    let request = CreateMissionRequest {
        title: format!("Monitor: {}", target_url),
        objective: format!(
            "Fetch the webpage at {} and detect any content changes since the last check. Report changes with a diff summary.",
            target_url
        ),
        owner_kind,
        owner_ref,
        source_json: Some(serde_json::json!({"scenario": "website_monitor", "target_url": target_url}).to_string()),
        delivery_policy_json: Some(delivery.to_string()),
        assistant_profile_id: None,
        trigger_json: Some(trigger.to_string()),
        mission_spec_json: Some(serde_json::json!({
            "kind": "change_monitor",
            "state_schema": { "type": "object" },
            "action_schema": { "type": "object" },
            "completion_policy": { "kind": "manual_or_agent_completion" },
            "report_policy": { "kind": "on_change_summary" }
        }).to_string()),
        step_plan_json: Some(serde_json::json!([
            {"description": format!("Fetch the webpage at {}", target_url)},
            {"description": "Store the page content as a snapshot artifact"},
            {"description": "Compare content hash with previous snapshot"},
            {"description": "If changed, generate a diff summary observation"}
        ]).to_string()),
        success_criteria_json: Some(serde_json::json!({
            "rules": [{"kind": "artifact_produced", "artifact_type": "page_snapshot", "min_count": 1}],
            "aggregation": "all"
        }).to_string()),
        context_strategy_json: Some(serde_json::json!({
            "mode": "incremental"
        }).to_string()),
        budget_json: Some(serde_json::json!({
            "max_runs_per_day": 24,
            "max_tool_calls_per_run": 10,
            "timeout_seconds": 120
        }).to_string()),
        failure_policy_json: None,
        missed_run_policy: "skip".to_string(),
        next_run_at: None,
    };

    let mission = db_service
        .create_mission(request)
        .await
        .map_err(|e| format!("Failed to create scenario mission: {e}"))?;

    Ok(serde_json::json!({
        "mission_id": mission.id,
        "title": mission.title,
        "status": mission.status,
        "scenario": "website_change_monitor"
    }))
}

/// T-11.3: Create a daily project test run mission for validation.
#[tauri::command]
pub async fn mission_scenario_daily_test(
    owner_kind: String,
    owner_ref: String,
    project_path: String,
    test_command: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<serde_json::Value, String> {
    let trigger = serde_json::json!({
        "kind": "cron",
        "cron_expr": "0 8 * * *",
        "timezone": "UTC"
    });

    let delivery = serde_json::json!({
        "onSuccess": "none",
        "onChange": "none",
        "onFailure": "immediate",
        "primary": {
            "kind": "app_notification",
            "refData": {}
        }
    });

    let request = CreateMissionRequest {
        title: format!("Daily Test: {}", project_path),
        objective: format!(
            "Run the test suite for the project at {} using the command '{}'. Report pass/fail status and any failures.",
            project_path, test_command
        ),
        owner_kind,
        owner_ref,
        source_json: Some(serde_json::json!({
            "scenario": "daily_test",
            "project_path": project_path,
            "test_command": test_command
        }).to_string()),
        delivery_policy_json: Some(delivery.to_string()),
        assistant_profile_id: None,
        trigger_json: Some(trigger.to_string()),
        mission_spec_json: Some(serde_json::json!({
            "kind": "scheduled_test_run",
            "state_schema": { "type": "object" },
            "action_schema": { "type": "object" },
            "completion_policy": { "kind": "manual_or_agent_completion" },
            "report_policy": { "kind": "failure_summary" }
        }).to_string()),
        step_plan_json: Some(serde_json::json!([
            {"description": format!("Navigate to project directory: {}", project_path)},
            {"description": format!("Run test command: {}", test_command)},
            {"description": "Capture test output as artifact"},
            {"description": "Analyze results and generate pass/fail observation"}
        ]).to_string()),
        success_criteria_json: Some(serde_json::json!({
            "rules": [
                {"kind": "artifact_produced", "artifact_type": "test_log", "min_count": 1},
                {"kind": "observation_absent", "observation_type": "test_failure"}
            ],
            "aggregation": "all"
        }).to_string()),
        context_strategy_json: Some(serde_json::json!({
            "mode": "stateless"
        }).to_string()),
        budget_json: Some(serde_json::json!({
            "max_runs_per_day": 1,
            "max_tool_calls_per_run": 15,
            "timeout_seconds": 600
        }).to_string()),
        failure_policy_json: None,
        missed_run_policy: "run_once_on_startup".to_string(),
        next_run_at: None,
    };

    let mission = db_service
        .create_mission(request)
        .await
        .map_err(|e| format!("Failed to create scenario mission: {e}"))?;

    Ok(serde_json::json!({
        "mission_id": mission.id,
        "title": mission.title,
        "status": mission.status,
        "scenario": "daily_project_test"
    }))
}
