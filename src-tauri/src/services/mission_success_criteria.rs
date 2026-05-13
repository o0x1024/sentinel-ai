use std::sync::Arc;

use sentinel_core::models::mission::Mission;
use serde::{Deserialize, Serialize};

use crate::services::database::DatabaseService;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionSuccessCriteria {
    pub rules: Vec<SuccessRule>,
    #[serde(default = "default_aggregation")]
    pub aggregation: String,
}

fn default_aggregation() -> String {
    "all".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum SuccessRule {
    #[serde(rename = "artifact_produced")]
    ArtifactProduced {
        artifact_type: String,
        #[serde(default = "default_min_count")]
        min_count: i64,
    },
    #[serde(rename = "observation_present")]
    ObservationPresent { observation_type: String },
    #[serde(rename = "observation_absent")]
    ObservationAbsent { observation_type: String },
    #[serde(rename = "content_changed")]
    ContentChanged { artifact_type: String },
    #[serde(rename = "content_unchanged")]
    ContentUnchanged { artifact_type: String },
    #[serde(rename = "task_ledger_complete")]
    TaskLedgerComplete,
}

fn default_min_count() -> i64 {
    1
}

#[derive(Debug, Clone, Serialize)]
pub struct RuleEvaluationResult {
    pub rule_index: usize,
    pub passed: bool,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CriteriaEvaluationResult {
    pub passed: bool,
    pub aggregation: String,
    pub rule_results: Vec<RuleEvaluationResult>,
}

/// Evaluate the success criteria for a given mission run.
/// Returns None if no criteria are defined (treated as auto-pass).
pub async fn evaluate_success_criteria(
    db: &Arc<DatabaseService>,
    mission: &Mission,
    run_id: &str,
) -> Result<Option<CriteriaEvaluationResult>, String> {
    let criteria_json = match &mission.success_criteria_json {
        Some(json) if !json.is_empty() => json,
        _ => return Ok(None),
    };

    let criteria: MissionSuccessCriteria = serde_json::from_str(criteria_json)
        .map_err(|e| format!("Failed to parse success criteria: {e}"))?;

    if criteria.rules.is_empty() {
        return Ok(None);
    }

    let mut rule_results = Vec::new();

    for (i, rule) in criteria.rules.iter().enumerate() {
        let result = evaluate_rule(db, &mission.id, run_id, rule).await?;
        rule_results.push(RuleEvaluationResult {
            rule_index: i,
            passed: result.0,
            description: result.1,
        });
    }

    let passed = match criteria.aggregation.as_str() {
        "any" => rule_results.iter().any(|r| r.passed),
        _ => rule_results.iter().all(|r| r.passed), // "all" is default
    };

    Ok(Some(CriteriaEvaluationResult {
        passed,
        aggregation: criteria.aggregation,
        rule_results,
    }))
}

async fn evaluate_rule(
    db: &Arc<DatabaseService>,
    mission_id: &str,
    run_id: &str,
    rule: &SuccessRule,
) -> Result<(bool, String), String> {
    match rule {
        SuccessRule::ArtifactProduced {
            artifact_type,
            min_count,
        } => {
            let artifacts = db
                .list_mission_artifacts(mission_id, Some(run_id))
                .await
                .map_err(|e| e.to_string())?;
            let count = artifacts
                .iter()
                .filter(|a| a.artifact_type == *artifact_type)
                .count() as i64;
            let passed = count >= *min_count;
            Ok((
                passed,
                format!(
                    "artifact_produced({}): found {count}, need {min_count}",
                    artifact_type
                ),
            ))
        }
        SuccessRule::ObservationPresent { observation_type } => {
            let obs = db
                .list_mission_observations(mission_id, Some(run_id), Some(observation_type))
                .await
                .map_err(|e| e.to_string())?;
            let passed = !obs.is_empty();
            Ok((
                passed,
                format!(
                    "observation_present({}): {}",
                    observation_type,
                    if passed { "found" } else { "not found" }
                ),
            ))
        }
        SuccessRule::ObservationAbsent { observation_type } => {
            let obs = db
                .list_mission_observations(mission_id, Some(run_id), Some(observation_type))
                .await
                .map_err(|e| e.to_string())?;
            let passed = obs.is_empty();
            Ok((
                passed,
                format!(
                    "observation_absent({}): {}",
                    observation_type,
                    if passed { "absent" } else { "present" }
                ),
            ))
        }
        SuccessRule::ContentChanged { artifact_type } => {
            let obs = db
                .list_mission_observations(mission_id, Some(run_id), Some("content_changed"))
                .await
                .map_err(|e| e.to_string())?;
            let passed = obs.iter().any(|o| o.title.contains(artifact_type));
            Ok((
                passed,
                format!(
                    "content_changed({}): {}",
                    artifact_type,
                    if passed { "changed" } else { "unchanged" }
                ),
            ))
        }
        SuccessRule::ContentUnchanged { artifact_type } => {
            let obs = db
                .list_mission_observations(mission_id, Some(run_id), Some("content_changed"))
                .await
                .map_err(|e| e.to_string())?;
            let passed = !obs.iter().any(|o| o.title.contains(artifact_type));
            Ok((
                passed,
                format!(
                    "content_unchanged({}): {}",
                    artifact_type,
                    if passed { "unchanged" } else { "changed" }
                ),
            ))
        }
        SuccessRule::TaskLedgerComplete => {
            // Check via run result - if execution completed without error, ledger is complete
            let run = db
                .get_mission_run(run_id)
                .await
                .map_err(|e| e.to_string())?;
            let passed = run
                .map(|r| r.error_message.is_none() && r.status == "succeeded")
                .unwrap_or(false);
            Ok((
                passed,
                format!(
                    "task_ledger_complete: {}",
                    if passed { "complete" } else { "incomplete" }
                ),
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// Cross-Run Context Injection
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionContextStrategy {
    pub mode: String,
    #[serde(default)]
    pub context_window: Option<ContextWindow>,
    #[serde(default)]
    pub completion_condition: Option<CompletionCondition>,
    #[serde(default)]
    pub synthesis_on_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextWindow {
    #[serde(default = "default_max_runs")]
    pub max_runs: i64,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: i64,
    #[serde(default)]
    pub summarize_older: bool,
}

fn default_max_runs() -> i64 {
    5
}
fn default_max_tokens() -> i64 {
    4000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionCondition {
    pub kind: String,
    #[serde(default)]
    pub value: serde_json::Value,
}

/// Build cross-run context injection text based on the context strategy.
pub async fn build_cross_run_context(
    db: &Arc<DatabaseService>,
    mission: &Mission,
) -> Result<Option<String>, String> {
    let strategy_json = match &mission.context_strategy_json {
        Some(json) if !json.is_empty() => json,
        _ => return Ok(None),
    };

    let strategy: MissionContextStrategy = serde_json::from_str(strategy_json)
        .map_err(|e| format!("Failed to parse context strategy: {e}"))?;

    match strategy.mode.as_str() {
        "stateless" => Ok(None),
        "incremental" => build_incremental_context(db, &mission.id).await,
        "cumulative" => {
            let window = strategy.context_window.unwrap_or(ContextWindow {
                max_runs: default_max_runs(),
                max_tokens: default_max_tokens(),
                summarize_older: false,
            });
            build_cumulative_context(db, &mission.id, &window).await
        }
        _ => Ok(None),
    }
}

async fn build_incremental_context(
    db: &Arc<DatabaseService>,
    mission_id: &str,
) -> Result<Option<String>, String> {
    let runs = db
        .list_mission_runs(mission_id, 1, 0)
        .await
        .map_err(|e| e.to_string())?;

    let last_run = match runs.first() {
        Some(r) => r,
        None => return Ok(None),
    };

    let observations = db
        .list_mission_observations(mission_id, Some(&last_run.id), None)
        .await
        .map_err(|e| e.to_string())?;

    if observations.is_empty() {
        return Ok(None);
    }

    let mut context = String::from("## Previous Run Observations\n\n");
    for obs in &observations {
        context.push_str(&format!(
            "- [{}] {}: {}\n",
            obs.severity,
            obs.title,
            obs.summary.as_deref().unwrap_or("")
        ));
    }

    if let Some(ref summary) = last_run.result_summary {
        context.push_str(&format!("\n## Previous Run Summary\n\n{summary}\n"));
    }

    Ok(Some(context))
}

async fn build_cumulative_context(
    db: &Arc<DatabaseService>,
    mission_id: &str,
    window: &ContextWindow,
) -> Result<Option<String>, String> {
    let runs = db
        .list_mission_runs(mission_id, window.max_runs, 0)
        .await
        .map_err(|e| e.to_string())?;

    if runs.is_empty() {
        return Ok(None);
    }

    let mut context = String::from("## Historical Run Observations\n\n");
    let mut total_chars: i64 = 0;
    let char_limit = window.max_tokens * 4; // rough token-to-char estimate

    for run in runs.iter().rev() {
        if total_chars >= char_limit {
            break;
        }

        let observations = db
            .list_mission_observations(mission_id, Some(&run.id), None)
            .await
            .map_err(|e| e.to_string())?;

        if observations.is_empty() {
            continue;
        }

        let run_header = format!("### Run {} ({})\n", run.run_index, run.status);
        context.push_str(&run_header);
        total_chars += run_header.len() as i64;

        for obs in &observations {
            let line = format!(
                "- [{}] {}: {}\n",
                obs.severity,
                obs.title,
                obs.summary.as_deref().unwrap_or("")
            );
            total_chars += line.len() as i64;
            if total_chars >= char_limit {
                context.push_str("...(truncated)\n");
                break;
            }
            context.push_str(&line);
        }
        context.push('\n');
    }

    Ok(Some(context))
}

/// Check if a mission should be completed based on its completion condition.
pub async fn check_completion_condition(
    db: &Arc<DatabaseService>,
    mission: &Mission,
) -> Result<bool, String> {
    let strategy_json = match &mission.context_strategy_json {
        Some(json) if !json.is_empty() => json,
        _ => return Ok(false),
    };

    let strategy: MissionContextStrategy = serde_json::from_str(strategy_json)
        .map_err(|e| format!("Failed to parse context strategy: {e}"))?;

    let condition = match strategy.completion_condition {
        Some(c) => c,
        None => return Ok(false),
    };

    match condition.kind.as_str() {
        "run_count" => {
            let target = condition.value.as_i64().unwrap_or(0);
            Ok(mission.run_count >= target)
        }
        "date" => {
            let target_str = condition.value.as_str().unwrap_or("");
            if let Ok(target_date) = chrono::DateTime::parse_from_rfc3339(target_str) {
                Ok(chrono::Utc::now() >= target_date)
            } else {
                Ok(false)
            }
        }
        "observation_match" => {
            let obs_type = condition
                .value
                .get("observation_type")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if obs_type.is_empty() {
                return Ok(false);
            }
            let observations = db
                .list_mission_observations(&mission.id, None, Some(obs_type))
                .await
                .map_err(|e| e.to_string())?;
            Ok(!observations.is_empty())
        }
        "manual" => Ok(false),
        _ => Ok(false),
    }
}
