use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::services::ai::AiServiceManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissionDraft {
    pub title: Option<String>,
    pub objective: Option<String>,
    pub trigger: Option<MissionDraftTrigger>,
    pub delivery: Option<MissionDraftDelivery>,
    pub assistant_profile_id: Option<String>,
    pub context_strategy: Option<String>,
    pub budget: Option<MissionDraftBudget>,
    #[serde(default)]
    pub missing_fields: Vec<String>,
    #[serde(default)]
    pub followup_question: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissionDraftTrigger {
    pub kind: String,
    pub cron_expr: Option<String>,
    pub interval_seconds: Option<i64>,
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissionDraftDelivery {
    pub target_kind: String,
    pub on_success: Option<String>,
    pub on_change: Option<String>,
    pub on_failure: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissionDraftBudget {
    pub max_runs_per_day: Option<i64>,
    pub max_tool_calls_per_run: Option<i64>,
    pub timeout_seconds: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannerResult {
    pub draft: Option<MissionDraft>,
    pub is_complete: bool,
    pub validation_errors: Vec<String>,
    pub followup_question: Option<String>,
}

const PLANNER_SYSTEM_PROMPT: &str = r#"You are a Mission Planner. Your job is to convert a user's natural language request into a structured MissionDraft JSON object.

A MissionDraft has these fields:
- title: short descriptive title for the mission
- objective: clear description of what the mission should accomplish
- trigger: { kind: "cron"|"interval"|"manual", cronExpr?: "cron expression", intervalSeconds?: number, timezone?: "UTC" }
- delivery: { targetKind: "app_notification"|"bot"|"webhook", onSuccess?: "summary"|"full"|"none", onChange?: "immediate"|"summary"|"none", onFailure?: "immediate"|"none" }
- assistantProfileId: null (use default)
- contextStrategy: "stateless"|"incremental"|"cumulative"
- budget: { maxRunsPerDay?: number, maxToolCallsPerRun?: number, timeoutSeconds?: number }

Rules:
1. If the user's request is missing critical information (like a URL for monitoring, or when to run), set the missing fields in "missingFields" and provide a "followupQuestion" asking for that information.
2. Always output valid JSON only, no markdown or explanation.
3. For daily tasks, use cron expressions (e.g., "0 9 * * *" for 9 AM daily).
4. For monitoring tasks, default to incremental context strategy.
5. For news/digest tasks, default to stateless context strategy.
6. For trend analysis tasks, default to cumulative context strategy.

Output format:
{
  "title": "...",
  "objective": "...",
  "trigger": { "kind": "cron", "cronExpr": "...", "timezone": "UTC" },
  "delivery": { "targetKind": "app_notification", "onSuccess": "summary", "onFailure": "immediate" },
  "assistantProfileId": null,
  "contextStrategy": "stateless",
  "budget": { "maxRunsPerDay": 1, "timeoutSeconds": 300 },
  "missingFields": [],
  "followupQuestion": null
}"#;

/// Parse a natural language mission request into a structured MissionDraft.
pub async fn plan_mission_from_text(
    ai_manager: &Arc<AiServiceManager>,
    user_input: &str,
) -> Result<PlannerResult, String> {
    let service = ai_manager
        .resolve_generation_service(None)
        .await
        .map_err(|e| format!("No AI service available: {e}"))?;

    let user_prompt = format!(
        "Convert the following user request into a MissionDraft JSON:\n\n{}",
        user_input
    );

    let response_text = service
        .service
        .completion(Some(PLANNER_SYSTEM_PROMPT), &user_prompt)
        .await
        .map_err(|e| format!("LLM call failed: {e}"))?;

    parse_planner_response(&response_text)
}

fn parse_planner_response(response: &str) -> Result<PlannerResult, String> {
    let cleaned = response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let draft: MissionDraft = serde_json::from_str(cleaned)
        .map_err(|e| format!("Failed to parse planner response as JSON: {e}"))?;

    let mut validation_errors = Vec::new();

    if draft.title.as_ref().map_or(true, |t| t.is_empty()) {
        validation_errors.push("Missing title".to_string());
    }
    if draft.objective.as_ref().map_or(true, |o| o.is_empty()) {
        validation_errors.push("Missing objective".to_string());
    }

    let has_followup = draft.followup_question.is_some() || !draft.missing_fields.is_empty();

    let is_complete = validation_errors.is_empty() && !has_followup;

    Ok(PlannerResult {
        followup_question: draft.followup_question.clone(),
        draft: Some(draft),
        is_complete,
        validation_errors,
    })
}

/// Validate a MissionDraft before creating the mission.
pub fn validate_mission_draft(draft: &MissionDraft) -> Vec<String> {
    let mut errors = Vec::new();

    if draft.title.as_ref().map_or(true, |t| t.trim().is_empty()) {
        errors.push("Title is required".to_string());
    }
    if draft
        .objective
        .as_ref()
        .map_or(true, |o| o.trim().is_empty())
    {
        errors.push("Objective is required".to_string());
    }

    if let Some(ref trigger) = draft.trigger {
        match trigger.kind.as_str() {
            "cron" => {
                if let Some(ref expr) = trigger.cron_expr {
                    if expr.parse::<cron::Schedule>().is_err() {
                        errors.push(format!("Invalid cron expression: {expr}"));
                    }
                } else {
                    errors.push("Cron trigger requires cron_expr".to_string());
                }
            }
            "interval" => {
                if trigger.interval_seconds.unwrap_or(0) <= 0 {
                    errors.push("Interval trigger requires positive interval_seconds".to_string());
                }
            }
            "manual" | "event" => {}
            other => errors.push(format!("Unknown trigger kind: {other}")),
        }
    }

    errors
}
