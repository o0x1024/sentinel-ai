use super::bounty_workflow_template_commands::{execute_workflow_steps, WorkflowStepDefinition};
use crate::services::ensure_bug_bounty_access;
use chrono::Utc;
use sentinel_db::{
    BountyChangeEventRow, BountyChangeEventWorkflowRunRow, Database, DatabaseService,
};
use sentinel_traffic::PluginManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, State};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerCondition {
    pub event_types: Option<Vec<String>>,
    pub min_severity: Option<String>,
    pub asset_tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTriggerResult {
    pub binding_id: String,
    pub template_id: String,
    pub template_name: String,
    pub triggered: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
struct EventWorkflowContext {
    event_id: String,
    binding_id: Option<String>,
    trigger_mode: String,
}

pub(crate) async fn build_workflow_trigger_inputs_from_event(
    db_service: &Arc<DatabaseService>,
    event: &BountyChangeEventRow,
) -> Result<serde_json::Value, String> {
    let mut inputs = serde_json::json!({
        "asset_id": event.asset_id,
        "event_id": event.id,
    });

    let Some(asset) = db_service
        .get_bounty_asset(&event.asset_id)
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(inputs);
    };

    let asset_type = asset.asset_type.trim().to_lowercase();
    let canonical_url = asset.canonical_url.trim().to_string();
    let hostname = asset.hostname.unwrap_or_default().trim().to_string();
    let protocol = asset.protocol.unwrap_or_default().trim().to_string();
    let port = asset.port;

    if let Some(obj) = inputs.as_object_mut() {
        if !asset_type.is_empty() {
            obj.insert("asset_type".to_string(), serde_json::json!(asset_type));
        }

        if !canonical_url.is_empty() {
            obj.insert(
                "canonical_url".to_string(),
                serde_json::json!(canonical_url.clone()),
            );
        }

        if !hostname.is_empty() {
            obj.insert("hostname".to_string(), serde_json::json!(hostname.clone()));
            obj.insert("host".to_string(), serde_json::json!(hostname.clone()));
        }

        if let Some(port) = port.filter(|p| *p > 0) {
            obj.insert("port".to_string(), serde_json::json!(port));
        }

        if !protocol.is_empty() {
            obj.insert("protocol".to_string(), serde_json::json!(protocol));
        }

        if asset_type == "port" && !hostname.is_empty() {
            obj.insert("targets".to_string(), serde_json::json!([hostname.clone()]));
            if let Some(port) = port.filter(|p| *p > 0) {
                obj.insert(
                    "host_port".to_string(),
                    serde_json::json!(format!("{}:{}", hostname, port)),
                );
            }
        }

        if canonical_url.is_empty() {
            return Ok(inputs);
        }

        let has_scheme =
            canonical_url.starts_with("http://") || canonical_url.starts_with("https://");
        if has_scheme {
            obj.insert("url".to_string(), serde_json::json!(canonical_url.clone()));
            if let Ok(parsed) = url::Url::parse(&canonical_url) {
                obj.insert("domain".to_string(), serde_json::json!(parsed.host_str()));
            }
            return Ok(inputs);
        }

        if asset_type == "url" || asset_type == "web" {
            obj.insert(
                "url".to_string(),
                serde_json::json!(format!("https://{}", canonical_url)),
            );
        }

        if matches!(asset_type.as_str(), "domain" | "subdomain" | "url" | "web") {
            let domain = canonical_url.split('/').next().unwrap_or_default();
            if !domain.is_empty() {
                obj.insert("domain".to_string(), serde_json::json!(domain));
            }
        }
    }

    Ok(inputs)
}

async fn spawn_template_execution(
    app_handle: AppHandle,
    db_service: Arc<DatabaseService>,
    plugin_manager: Arc<PluginManager>,
    template_id: String,
    program_id: Option<String>,
    inputs: serde_json::Value,
    event_context: Option<EventWorkflowContext>,
) -> Result<String, String> {
    let template = db_service
        .get_bounty_workflow_template(&template_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Template not found".to_string())?;

    let steps: Vec<WorkflowStepDefinition> =
        serde_json::from_str(&template.steps_json).map_err(|e| format!("Invalid steps: {}", e))?;

    if steps.is_empty() {
        return Err("Template has no steps".to_string());
    }

    let execution_id = Uuid::new_v4().to_string();
    let started_at = Utc::now();

    db_service
        .create_workflow_run(
            &execution_id,
            &template.id,
            &template.name,
            &template.updated_at,
            "running",
            started_at,
        )
        .await
        .map_err(|e| e.to_string())?;

    if let Some(context) = &event_context {
        let now = started_at.to_rfc3339();
        let row = BountyChangeEventWorkflowRunRow {
            id: Uuid::new_v4().to_string(),
            event_id: context.event_id.clone(),
            execution_id: execution_id.clone(),
            binding_id: context.binding_id.clone(),
            workflow_template_id: template.id.clone(),
            workflow_template_name: Some(template.name.clone()),
            trigger_mode: context.trigger_mode.clone(),
            status: "running".to_string(),
            error_message: None,
            created_at: now.clone(),
            updated_at: now,
        };
        db_service
            .create_bounty_change_event_workflow_run(&row)
            .await
            .map_err(|e| e.to_string())?;
    }

    log::info!(
        "Starting workflow execution {} for template {}",
        execution_id,
        template_id
    );
    log::info!("Inputs: {:?}", inputs);
    log::info!("Steps: {}", steps.len());

    let exec_id = execution_id.clone();
    let db = db_service.clone();
    let pm = plugin_manager.clone();
    let app = app_handle.clone();
    let initial_inputs = inputs.clone();
    let event_context = event_context.clone();

    tokio::spawn(async move {
        let final_status = execute_workflow_steps(
            exec_id.clone(),
            steps,
            initial_inputs,
            program_id,
            db.clone(),
            pm,
            app,
        )
        .await;
        let error_message = if final_status == "completed_with_errors" {
            Some("One or more workflow steps failed")
        } else {
            None
        };

        let _ = db
            .update_workflow_run_status(&exec_id, &final_status, Some(Utc::now()), error_message)
            .await;

        if event_context.is_some() {
            let _ = db
                .update_bounty_change_event_workflow_run_status(
                    &exec_id,
                    &final_status,
                    error_message,
                )
                .await;
        }
    });

    Ok(execution_id)
}

pub(crate) async fn run_workflow_template_with_inputs(
    app_handle: AppHandle,
    db_service: Arc<DatabaseService>,
    plugin_manager: Arc<PluginManager>,
    template_id: String,
    program_id: Option<String>,
    inputs: serde_json::Value,
) -> Result<String, String> {
    spawn_template_execution(
        app_handle,
        db_service,
        plugin_manager,
        template_id,
        program_id,
        inputs,
        None,
    )
    .await
}

#[tauri::command]
pub async fn bounty_run_workflow_template_for_event(
    app_handle: AppHandle,
    db_service: State<'_, Arc<DatabaseService>>,
    plugin_manager: State<'_, Arc<PluginManager>>,
    template_id: String,
    event_id: String,
) -> Result<String, String> {
    ensure_bug_bounty_access()?;

    let event = db_service
        .get_bounty_change_event(&event_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Change event not found".to_string())?;

    let inputs = build_workflow_trigger_inputs_from_event(db_service.inner(), &event).await?;

    let execution_id = spawn_template_execution(
        app_handle,
        db_service.inner().clone(),
        plugin_manager.inner().clone(),
        template_id.clone(),
        event.program_id.clone(),
        inputs,
        Some(EventWorkflowContext {
            event_id: event_id.clone(),
            binding_id: None,
            trigger_mode: "manual".to_string(),
        }),
    )
    .await?;

    let _ = db_service
        .update_bounty_change_event_status(&event_id, "workflow_triggered", None)
        .await;

    Ok(execution_id)
}

#[tauri::command]
pub async fn bounty_list_change_event_workflow_runs(
    db_service: State<'_, Arc<DatabaseService>>,
    event_id: String,
) -> Result<Vec<BountyChangeEventWorkflowRunRow>, String> {
    ensure_bug_bounty_access()?;

    db_service
        .list_bounty_change_event_workflow_runs(&event_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn bounty_retry_change_event_workflow_run(
    app_handle: AppHandle,
    db_service: State<'_, Arc<DatabaseService>>,
    plugin_manager: State<'_, Arc<PluginManager>>,
    execution_id: String,
) -> Result<String, String> {
    ensure_bug_bounty_access()?;

    let run = db_service
        .get_bounty_change_event_workflow_run_by_execution_id(&execution_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Change event workflow run not found".to_string())?;

    if run.status == "running" {
        return Err("Workflow run is still running".to_string());
    }

    let event = db_service
        .get_bounty_change_event(&run.event_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Change event not found".to_string())?;

    let inputs = build_workflow_trigger_inputs_from_event(db_service.inner(), &event).await?;

    let retried_execution_id = spawn_template_execution(
        app_handle,
        db_service.inner().clone(),
        plugin_manager.inner().clone(),
        run.workflow_template_id.clone(),
        event.program_id.clone(),
        inputs,
        Some(EventWorkflowContext {
            event_id: run.event_id.clone(),
            binding_id: run.binding_id.clone(),
            trigger_mode: "retry".to_string(),
        }),
    )
    .await?;

    if let Some(binding_id) = &run.binding_id {
        let _ = db_service
            .update_bounty_workflow_binding_run_status(binding_id, "triggered")
            .await;
    }

    let _ = db_service
        .update_bounty_change_event_status(&run.event_id, "workflow_triggered", None)
        .await;

    Ok(retried_execution_id)
}

#[tauri::command]
pub async fn bounty_get_triggered_workflows(
    db_service: State<'_, Arc<DatabaseService>>,
    event_id: String,
) -> Result<Vec<WorkflowTriggerResult>, String> {
    ensure_bug_bounty_access()?;

    bounty_get_triggered_workflows_internal(db_service.inner(), &event_id).await
}

#[tauri::command]
pub async fn bounty_trigger_workflows_for_event(
    app_handle: AppHandle,
    db_service: State<'_, Arc<DatabaseService>>,
    plugin_manager: State<'_, Arc<PluginManager>>,
    event_id: String,
) -> Result<Vec<String>, String> {
    ensure_bug_bounty_access()?;

    bounty_trigger_workflows_for_event_internal(
        app_handle,
        db_service.inner().clone(),
        plugin_manager.inner().clone(),
        event_id,
    )
    .await
}

pub async fn bounty_trigger_workflows_for_event_internal(
    app_handle: AppHandle,
    db_service: Arc<DatabaseService>,
    plugin_manager: Arc<PluginManager>,
    event_id: String,
) -> Result<Vec<String>, String> {
    let triggered_results = bounty_get_triggered_workflows_internal(&db_service, &event_id).await?;

    let event = db_service
        .get_bounty_change_event(&event_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Change event not found".to_string())?;

    let workflow_inputs = build_workflow_trigger_inputs_from_event(&db_service, &event).await?;
    let mut triggered_binding_ids = Vec::new();

    for result in triggered_results {
        if !result.triggered {
            continue;
        }

        if let Some(existing) = db_service
            .get_bounty_change_event_workflow_run_for_binding(&event_id, &result.binding_id)
            .await
            .map_err(|e| e.to_string())?
        {
            log::info!(
                "Skipping duplicate workflow trigger for event {} binding {} (existing execution {})",
                event_id,
                result.binding_id,
                existing.execution_id
            );
            continue;
        }

        let execution_id = spawn_template_execution(
            app_handle.clone(),
            db_service.clone(),
            plugin_manager.clone(),
            result.template_id.clone(),
            event.program_id.clone(),
            workflow_inputs.clone(),
            Some(EventWorkflowContext {
                event_id: event_id.clone(),
                binding_id: Some(result.binding_id.clone()),
                trigger_mode: "auto".to_string(),
            }),
        )
        .await?;

        let _ = db_service
            .update_bounty_workflow_binding_run_status(&result.binding_id, "triggered")
            .await;

        log::info!(
            "Auto-triggered workflow execution {} for binding {} on event {}",
            execution_id,
            result.binding_id,
            event_id
        );

        triggered_binding_ids.push(result.binding_id);
    }

    if !triggered_binding_ids.is_empty() {
        let _ = db_service
            .update_bounty_change_event_status(&event_id, "workflow_triggered", None)
            .await;
    }

    Ok(triggered_binding_ids)
}

async fn bounty_get_triggered_workflows_internal(
    db_service: &Arc<DatabaseService>,
    event_id: &str,
) -> Result<Vec<WorkflowTriggerResult>, String> {
    let event = db_service
        .get_bounty_change_event(event_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Change event not found".to_string())?;

    let program_id = event
        .program_id
        .as_ref()
        .ok_or_else(|| "Event has no program_id".to_string())?;

    let bindings = db_service
        .get_auto_trigger_workflow_bindings(program_id)
        .await
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();

    for binding in bindings {
        let template = db_service
            .get_bounty_workflow_template(&binding.workflow_template_id)
            .await
            .map_err(|e| e.to_string())?;

        let template_name = template
            .as_ref()
            .map(|t| t.name.clone())
            .unwrap_or_default();

        let (should_trigger, reason) =
            if let Some(conditions_json) = &binding.trigger_conditions_json {
                if let Ok(conditions) = serde_json::from_str::<TriggerCondition>(conditions_json) {
                    check_trigger_conditions(&event, &conditions)
                } else {
                    (
                        false,
                        Some("Invalid trigger conditions configuration".to_string()),
                    )
                }
            } else {
                (true, None)
            };

        results.push(WorkflowTriggerResult {
            binding_id: binding.id,
            template_id: binding.workflow_template_id,
            template_name,
            triggered: should_trigger,
            reason,
        });
    }

    Ok(results)
}

fn check_trigger_conditions(
    event: &BountyChangeEventRow,
    conditions: &TriggerCondition,
) -> (bool, Option<String>) {
    if let Some(ref allowed_types) = conditions.event_types {
        if !allowed_types.contains(&event.event_type) {
            return (
                false,
                Some(format!(
                    "Event type '{}' not in allowed types",
                    event.event_type
                )),
            );
        }
    }

    if let Some(ref min_severity) = conditions.min_severity {
        let event_severity_rank = severity_rank(&event.severity);
        let min_severity_rank = severity_rank(min_severity);
        if event_severity_rank < min_severity_rank {
            return (
                false,
                Some(format!(
                    "Severity '{}' below minimum '{}'",
                    event.severity, min_severity
                )),
            );
        }
    }

    if let Some(ref required_tags) = conditions.asset_tags {
        if let Some(ref tags_json) = event.tags_json {
            if let Ok(event_tags) = serde_json::from_str::<Vec<String>>(tags_json) {
                let has_required_tag = required_tags.iter().any(|tag| event_tags.contains(tag));
                if !has_required_tag {
                    return (false, Some("Event does not have required tags".to_string()));
                }
            }
        }
    }

    (true, None)
}

fn severity_rank(severity: &str) -> i32 {
    match severity {
        "critical" => 4,
        "high" => 3,
        "medium" => 2,
        "low" => 1,
        _ => 0,
    }
}
