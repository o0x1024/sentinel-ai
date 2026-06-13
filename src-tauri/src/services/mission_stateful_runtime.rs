use std::sync::Arc;

use chrono::Utc;
use sentinel_core::models::mission::Mission;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::services::database::DatabaseService;

#[derive(Debug, Clone)]
pub struct MissionRuntimeContext {
    pub mission_spec_json: String,
    pub previous_state_json: String,
    pub execution_context_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionRuntimeOutput {
    #[serde(default)]
    pub observations: Vec<MissionRuntimeObservation>,
    #[serde(default)]
    pub actions: Vec<MissionRuntimeAction>,
    #[serde(default = "empty_object")]
    pub state_patch: Value,
    pub report: MissionRuntimeReport,
    pub completion: MissionRuntimeCompletion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionRuntimeObservation {
    #[serde(rename = "type")]
    pub observation_type: String,
    #[serde(default = "default_severity")]
    pub severity: String,
    pub title: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub data: Option<Value>,
    #[serde(default)]
    pub artifact_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionRuntimeAction {
    #[serde(rename = "type")]
    pub action_type: String,
    #[serde(default = "default_action_status")]
    pub status: String,
    #[serde(default)]
    pub input: Option<Value>,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionRuntimeReport {
    pub title: String,
    pub summary: String,
    #[serde(default)]
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionRuntimeCompletion {
    pub status: String,
    pub reason: String,
}

fn empty_object() -> Value {
    Value::Object(Map::new())
}

fn default_severity() -> String {
    "info".to_string()
}

fn default_action_status() -> String {
    "recorded".to_string()
}

pub async fn build_runtime_context(
    db: &Arc<DatabaseService>,
    mission: &Mission,
    run_id: &str,
) -> Result<MissionRuntimeContext, String> {
    let mission_spec_json = match mission.mission_spec_json.as_deref() {
        Some(spec) if !spec.trim().is_empty() => {
            validate_json_object(spec, "mission_spec_json")?;
            spec.trim().to_string()
        }
        _ => default_mission_spec_json(mission)?,
    };

    let previous_state_json = match db
        .get_latest_mission_state_snapshot(&mission.id)
        .await
        .map_err(|e| format!("Failed to load mission state: {e}"))?
    {
        Some(snapshot) => {
            validate_json_object(&snapshot.state_json, "previous mission state")?;
            snapshot.state_json
        }
        None => "{}".to_string(),
    };

    let recent_observations = db
        .list_mission_observations(&mission.id, None, None)
        .await
        .map_err(|e| format!("Failed to load mission observations: {e}"))?;

    let observations: Vec<Value> = recent_observations
        .iter()
        .rev()
        .take(20)
        .map(|item| {
            serde_json::json!({
                "type": item.observation_type,
                "severity": item.severity,
                "title": item.title,
                "summary": item.summary,
                "data": parse_optional_json(item.data_json.as_deref()),
                "created_at": item.created_at.to_rfc3339(),
            })
        })
        .collect();

    let execution_context = serde_json::json!({
        "mission": {
            "id": mission.id,
            "title": mission.title,
            "objective": mission.objective,
            "run_id": run_id,
            "run_count": mission.run_count,
            "current_due_at": mission.next_run_at.map(|value| value.to_rfc3339()),
        },
        "trigger": parse_optional_json(mission.trigger_json.as_deref()),
        "mission_spec": serde_json::from_str::<Value>(&mission_spec_json)
            .map_err(|e| format!("Invalid mission_spec_json: {e}"))?,
        "previous_state": serde_json::from_str::<Value>(&previous_state_json)
            .map_err(|e| format!("Invalid previous mission state: {e}"))?,
        "recent_observations": observations,
    });

    let execution_context_json = serde_json::to_string_pretty(&execution_context)
        .map_err(|e| format!("Failed to serialize mission context: {e}"))?;

    Ok(MissionRuntimeContext {
        mission_spec_json,
        previous_state_json,
        execution_context_json,
    })
}

pub fn build_system_prompt(mission: &Mission) -> String {
    if crate::services::observer_data_collector::is_observer_mission(mission) {
        return build_observer_system_prompt();
    }

    format!(
        "You are executing a stateful Mission.\n\
         Mission objective: {}\n\n\
         CRITICAL OUTPUT RULE — VIOLATION WILL CAUSE MISSION FAILURE:\n\
         Your ENTIRE final response must be exactly one JSON object. No character may appear\n\
         before the opening {{ or after the closing }}. No markdown, no code fences, no\n\
         natural-language introduction, no commentary, no thinking-out-loud. The first byte\n\
         of your response after whitespace trimming MUST be {{.\n\n\
         The database, not your prose, owns durable mission state. Do not describe what you\n\
         are about to do. Do not narrate tool use. Put every note, plan, failure, and result\n\
         inside the JSON fields below.\n\n\
         Required JSON shape:\n\
         {{\n\
           \"observations\": [{{\"type\":\"...\",\"severity\":\"info|warning|error\",\"title\":\"...\",\"summary\":\"...\",\"data\":{{}},\"artifact_ids\":[]}}],\n\
           \"actions\": [{{\"type\":\"...\",\"status\":\"recorded|succeeded|failed|skipped\",\"input\":{{}},\"result\":{{}},\"error\":null}}],\n\
           \"state_patch\": {{}},\n\
           \"report\": {{\"title\":\"...\",\"summary\":\"...\",\"details\":\"...\"}},\n\
           \"completion\": {{\"status\":\"continue|completed|blocked|failed\",\"reason\":\"...\"}}\n\
         }}\n\n\
         state_patch is a JSON merge patch over previous_state. Do not put durable facts only in\n\
         report text; put them in observations, actions, or state_patch.\n\n\
         If data is unavailable, a tool fails, the task is blocked, or you cannot complete the\n\
         requested work, still return this exact JSON object. Record the failure in observations,\n\
         actions, report, and completion.status.",
        mission.objective
    )
}

fn build_observer_system_prompt() -> String {
    "You are a Bot operations observer (Observer).\n\
     Your responsibilities:\n\
     1. Analyze overall Bot execution health from the provided observer_data snapshot.\n\
     2. Detect abnormal patterns (failure spikes, slow runs, offline accounts, stuck missions).\n\
     3. Produce concise, actionable Chinese reports for operators.\n\
     4. Recommend follow-up actions when serious issues are found.\n\n\
     The execution context includes observer_data with:\n\
     - execution_summary\n\
     - account_health\n\
     - mission_progress\n\
     - failure_details\n\
     - token_usage\n\n\
     CRITICAL OUTPUT RULE — VIOLATION WILL CAUSE MISSION FAILURE:\n\
     Your ENTIRE final response must be exactly one JSON object. No character may appear\n\
     before the opening { or after the closing }. No markdown, no code fences, no\n\
     natural-language introduction, no commentary, no thinking-out-loud. The first byte\n\
     of your response after whitespace trimming MUST be {.\n\n\
     Required JSON shape:\n\
     {\n\
       \"observations\": [{\"type\":\"...\",\"severity\":\"info|warning|error\",\"title\":\"...\",\"summary\":\"...\",\"data\":{},\"artifact_ids\":[]}],\n\
       \"actions\": [{\"type\":\"...\",\"status\":\"recorded|succeeded|failed|skipped\",\"input\":{},\"result\":{},\"error\":null}],\n\
       \"state_patch\": {\"last_observation_at\":\"<RFC3339>\",\"previous_failure_rate\":0.0},\n\
       \"report\": {\"title\":\"...\",\"summary\":\"...\",\"details\":\"...\"},\n\
       \"completion\": {\"status\":\"continue\",\"reason\":\"Observer keeps running\"}\n\
     }\n\n\
     Always set completion.status to continue. Put durable facts in observations, actions, or state_patch.\n\
     Write report text in Chinese. Keep it concise and highlight changes versus previous baselines when possible."
        .to_string()
}

pub fn build_task(mission: &Mission, context: &MissionRuntimeContext) -> String {
    format!(
        "Mission: {}\n\nObjective: {}\n\nExecution context:\n{}\n\nAdvance this mission by one tick.\n\n\
         Final response contract: output only the required JSON object. The first non-whitespace \
         character of your final response must be {{. Do not include prose before or after the JSON.",
        mission.title, mission.objective, context.execution_context_json
    )
}

pub fn parse_runtime_output(final_response: &str) -> Result<MissionRuntimeOutput, String> {
    // Attempt 1: strict parse (model obeyed the JSON-only contract perfectly)
    if let Ok(output) = serde_json::from_str::<MissionRuntimeOutput>(final_response.trim()) {
        validate_runtime_output(&output)?;
        return Ok(output);
    }

    // Attempt 2: the model prefixed natural language before the JSON object.
    // Find the first '{', then extract the JSON object by brace counting.
    if let Some(extracted_json) = extract_first_json_object(final_response) {
        match serde_json::from_str::<MissionRuntimeOutput>(&extracted_json) {
            Ok(output) => {
                validate_runtime_output(&output)?;
                tracing::info!(
                    "Mission runtime output: extracted JSON from mixed response ({} chars prefix stripped)",
                    final_response.trim().len().saturating_sub(extracted_json.len())
                );
                return Ok(output);
            }
            Err(json_err) => {
                return Err(format!(
                    "Mission run JSON extracted but invalid: {}; response_preview={}",
                    json_err,
                    runtime_response_preview(final_response)
                ));
            }
        }
    }

    // All attempts failed
    Err(format_runtime_parse_error(
        final_response,
        "no valid JSON object found",
    ))
}

/// Extract the first top-level JSON object from a string that may contain
/// leading natural language text. Returns the substring from the first '{'
/// through its matching '}' (tracking nesting depth).
fn extract_first_json_object(text: &str) -> Option<String> {
    let start = text.find('{')?;
    let bytes = text.as_bytes();
    let mut depth = 0u32;
    let mut in_string = false;
    let mut escaped = false;

    for (i, &ch) in bytes.iter().enumerate().skip(start) {
        if escaped {
            escaped = false;
            continue;
        }
        if in_string {
            match ch {
                b'\\' => escaped = true,
                b'"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match ch {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let json_str = std::str::from_utf8(&bytes[start..=i]).ok()?;
                    return Some(json_str.to_string());
                }
            }
            _ => {}
        }
    }
    None
}

pub fn runtime_response_preview(final_response: &str) -> String {
    const MAX_PREVIEW_CHARS: usize = 800;
    let normalized = final_response.trim();
    let preview: String = normalized.chars().take(MAX_PREVIEW_CHARS).collect();
    if normalized.chars().count() > MAX_PREVIEW_CHARS {
        format!("{preview}...")
    } else if preview.is_empty() {
        "(empty response)".to_string()
    } else {
        preview
    }
}

fn format_runtime_parse_error(final_response: &str, parse_error: &str) -> String {
    let trimmed = final_response.trim_start();
    let first_char = trimmed
        .chars()
        .next()
        .map(|ch| ch.to_string())
        .unwrap_or_else(|| "(none)".to_string());
    format!(
        "Mission run must return strict JSON: {parse_error}; first_char={first_char}; response_preview={}",
        runtime_response_preview(final_response)
    )
}

pub async fn persist_runtime_output(
    db: &Arc<DatabaseService>,
    mission: &Mission,
    run_id: &str,
    context: &MissionRuntimeContext,
    output: &MissionRuntimeOutput,
) -> Result<Value, String> {
    let mut state: Value = serde_json::from_str(&context.previous_state_json)
        .map_err(|e| format!("Invalid previous mission state: {e}"))?;
    apply_state_patch(&mut state, &output.state_patch)?;
    let next_state_json = serde_json::to_string(&state)
        .map_err(|e| format!("Failed to serialize next mission state: {e}"))?;

    db.save_mission_state_snapshot(&mission.id, Some(run_id), &next_state_json)
        .await
        .map_err(|e| format!("Failed to save mission state snapshot: {e}"))?;

    for observation in &output.observations {
        let data_json = observation
            .data
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| format!("Failed to serialize observation data: {e}"))?;
        let artifact_ids_json = serde_json::to_string(&observation.artifact_ids)
            .map_err(|e| format!("Failed to serialize observation artifact ids: {e}"))?;
        db.save_mission_observation_record(
            &uuid::Uuid::new_v4().to_string(),
            &mission.id,
            run_id,
            None,
            &observation.observation_type,
            &observation.severity,
            &observation.title,
            observation.summary.as_deref(),
            data_json.as_deref(),
            Some(&artifact_ids_json),
        )
        .await
        .map_err(|e| format!("Failed to save mission observation: {e}"))?;
    }

    for action in &output.actions {
        let input_json = action
            .input
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| format!("Failed to serialize action input: {e}"))?;
        let result_json = action
            .result
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| format!("Failed to serialize action result: {e}"))?;
        let saved_action = db
            .save_mission_action(
                &mission.id,
                run_id,
                &action.action_type,
                &action.status,
                input_json.as_deref(),
                result_json.as_deref(),
                action.error.as_deref(),
            )
            .await
            .map_err(|e| format!("Failed to save mission action: {e}"))?;
        db.save_mission_action_result(
            &saved_action.id,
            &mission.id,
            run_id,
            &action.status,
            result_json.as_deref(),
            action.error.as_deref(),
        )
        .await
        .map_err(|e| format!("Failed to save mission action result: {e}"))?;
    }

    let event_payload = serde_json::json!({
        "report": &output.report,
        "completion": &output.completion,
        "state_patch": &output.state_patch,
        "persisted_at": Utc::now().to_rfc3339(),
    });
    let payload_json = serde_json::to_string(&event_payload)
        .map_err(|e| format!("Failed to serialize mission event: {e}"))?;
    db.save_mission_event(
        &mission.id,
        Some(run_id),
        "mission_tick_completed",
        &output.report.title,
        Some(&payload_json),
    )
    .await
    .map_err(|e| format!("Failed to save mission event: {e}"))?;

    Ok(state)
}

pub fn summarize_output(output: &MissionRuntimeOutput) -> String {
    let mut summary = output.report.summary.clone();
    if let Some(details) = output
        .report
        .details
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        summary = format!("{summary}\n\n{details}");
    }
    if summary.chars().count() > 500 {
        format!("{}...", summary.chars().take(500).collect::<String>())
    } else {
        summary
    }
}

pub fn run_status_from_completion(status: &str) -> Result<&'static str, String> {
    match status {
        "continue" | "completed" => Ok("succeeded"),
        "blocked" => Ok("partial"),
        "failed" => Ok("failed"),
        other => Err(format!("Invalid completion.status: {other}")),
    }
}

fn validate_runtime_output(output: &MissionRuntimeOutput) -> Result<(), String> {
    if !output.state_patch.is_object() {
        return Err("state_patch must be a JSON object".to_string());
    }
    if output.report.title.trim().is_empty() {
        return Err("report.title is required".to_string());
    }
    if output.report.summary.trim().is_empty() {
        return Err("report.summary is required".to_string());
    }
    match output.completion.status.as_str() {
        "continue" | "completed" | "blocked" | "failed" => {}
        other => return Err(format!("completion.status is invalid: {other}")),
    }
    if output.completion.reason.trim().is_empty() {
        return Err("completion.reason is required".to_string());
    }
    for observation in &output.observations {
        if observation.observation_type.trim().is_empty() {
            return Err("observation.type is required".to_string());
        }
        if observation.title.trim().is_empty() {
            return Err("observation.title is required".to_string());
        }
    }
    for action in &output.actions {
        if action.action_type.trim().is_empty() {
            return Err("action.type is required".to_string());
        }
    }
    Ok(())
}

fn apply_state_patch(state: &mut Value, patch: &Value) -> Result<(), String> {
    let patch_object = patch
        .as_object()
        .ok_or_else(|| "state_patch must be a JSON object".to_string())?;
    if !state.is_object() {
        *state = Value::Object(Map::new());
    }
    let state_object = state
        .as_object_mut()
        .ok_or_else(|| "mission state must be a JSON object".to_string())?;
    merge_object_patch(state_object, patch_object);
    Ok(())
}

fn merge_object_patch(target: &mut Map<String, Value>, patch: &Map<String, Value>) {
    for (key, value) in patch {
        if value.is_null() {
            target.remove(key);
            continue;
        }
        match (target.get_mut(key), value) {
            (Some(Value::Object(target_obj)), Value::Object(patch_obj)) => {
                merge_object_patch(target_obj, patch_obj);
            }
            _ => {
                target.insert(key.clone(), value.clone());
            }
        }
    }
}

fn validate_json_object(raw: &str, label: &str) -> Result<(), String> {
    let parsed: Value =
        serde_json::from_str(raw).map_err(|e| format!("{label} must be valid JSON: {e}"))?;
    if parsed.is_object() {
        Ok(())
    } else {
        Err(format!("{label} must be a JSON object"))
    }
}

fn parse_optional_json(raw: Option<&str>) -> Option<Value> {
    raw.and_then(|value| serde_json::from_str::<Value>(value).ok())
}

fn default_mission_spec_json(mission: &Mission) -> Result<String, String> {
    serde_json::to_string(&serde_json::json!({
        "kind": "generic_stateful_mission",
        "objective": mission.objective,
        "state_schema": { "type": "object" },
        "action_schema": { "type": "object" },
        "completion_policy": { "kind": "manual_or_agent_completion" },
        "report_policy": { "kind": "each_run_summary" }
    }))
    .map_err(|e| format!("Failed to build default mission spec: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_strict_runtime_output() {
        let output = parse_runtime_output(
            r#"{
                "observations":[{"type":"daily_status","title":"State advanced"}],
                "actions":[{"type":"record_decision","status":"succeeded","input":{"a":1}}],
                "state_patch":{"day":1},
                "report":{"title":"Report","summary":"Done"},
                "completion":{"status":"continue","reason":"More runs remain"}
            }"#,
        )
        .expect("runtime output should parse");
        assert_eq!(output.observations[0].observation_type, "daily_status");
        assert_eq!(output.actions[0].action_type, "record_decision");
    }

    #[test]
    fn rejects_non_json_output() {
        // The extraction logic will find {} inside the markdown fence,
        // but the empty object will fail validation (missing required fields).
        let err = parse_runtime_output("```json\n{}\n```").expect_err("empty object must fail");
        assert!(
            err.contains("JSON extracted but invalid")
                || err.contains("no valid JSON object found"),
            "expected extraction-failure message, got: {err}"
        );
    }

    #[test]
    fn extracts_json_from_mixed_response() {
        // Simulates a model that outputs natural language before the JSON object.
        let output = parse_runtime_output(
            "First, let's verify market data.\nNow executing trades.\n{\n  \
             \"observations\":[{\"type\":\"market_check\",\"title\":\"Market verified\"}],\n  \
             \"actions\":[{\"type\":\"execute_trade\",\"status\":\"succeeded\"}],\n  \
             \"state_patch\":{\"tick\":5},\n  \
             \"report\":{\"title\":\"Tick 5\",\"summary\":\"Completed\",\"details\":\"All trades executed\"},\n  \
             \"completion\":{\"status\":\"continue\",\"reason\":\"More ticks remain\"}\n}",
        )
        .expect("mixed response should parse");
        assert_eq!(output.observations[0].observation_type, "market_check");
        assert_eq!(output.report.title, "Tick 5");
        assert_eq!(output.completion.status, "continue");
    }

    #[test]
    fn extract_first_json_object_handles_nested_braces() {
        let mixed =
            "Prefix text {\"outer\": {\"inner\": [{\"a\":1}, {\"b\":2}]}, \"x\": 3} trailing";
        let json = extract_first_json_object(mixed).expect("should extract");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("should be valid JSON");
        assert_eq!(parsed["x"], 3);
        assert_eq!(parsed["outer"]["inner"][1]["b"], 2);
    }

    #[test]
    fn previews_empty_runtime_response() {
        assert_eq!(runtime_response_preview(" \n\t"), "(empty response)");
    }

    #[test]
    fn task_repeats_strict_json_contract() {
        let mission = Mission {
            id: "mission-1".to_string(),
            title: "Strict mission".to_string(),
            objective: "Return structured state".to_string(),
            status: "active".to_string(),
            owner_kind: "user".to_string(),
            owner_ref: "default".to_string(),
            source_json: None,
            delivery_policy_json: None,
            assistant_profile_id: None,
            trigger_json: None,
            mission_spec_json: None,
            step_plan_json: None,
            success_criteria_json: None,
            context_strategy_json: None,
            budget_json: None,
            failure_policy_json: None,
            missed_run_policy: "skip".to_string(),
            next_run_at: None,
            last_run_at: None,
            last_error: None,
            run_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let context = MissionRuntimeContext {
            mission_spec_json: "{}".to_string(),
            previous_state_json: "{}".to_string(),
            execution_context_json: "{}".to_string(),
        };

        let task = build_task(&mission, &context);
        assert!(task.contains("Final response contract"));
        assert!(task.contains("must be {"));
        assert!(task.contains("Do not include prose before or after the JSON"));
    }

    #[test]
    fn observer_mission_uses_observer_system_prompt() {
        let mission = Mission {
            id: "mission-observer".to_string(),
            title: "Observer".to_string(),
            objective: "Observe bot health".to_string(),
            status: "active".to_string(),
            owner_kind: "observer".to_string(),
            owner_ref: "global".to_string(),
            source_json: None,
            delivery_policy_json: None,
            assistant_profile_id: None,
            trigger_json: None,
            mission_spec_json: Some(r#"{"kind":"observer_mission"}"#.to_string()),
            step_plan_json: None,
            success_criteria_json: None,
            context_strategy_json: None,
            budget_json: None,
            failure_policy_json: None,
            missed_run_policy: "skip".to_string(),
            next_run_at: None,
            last_run_at: None,
            last_error: None,
            run_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let prompt = build_system_prompt(&mission);
        assert!(prompt.contains("Bot operations observer"));
        assert!(prompt.contains("observer_data"));
    }

    #[test]
    fn applies_object_patch_recursively() {
        let mut state = serde_json::json!({"a":{"b":1,"c":2},"x":1});
        let patch = serde_json::json!({"a":{"b":3},"x":null,"y":4});
        apply_state_patch(&mut state, &patch).expect("patch should apply");
        assert_eq!(state, serde_json::json!({"a":{"b":3,"c":2},"y":4}));
    }
}
