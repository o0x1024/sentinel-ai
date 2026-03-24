use super::bounty_commands::{execute_workflow_steps, WorkflowStepDefinition};
use sentinel_db::{BountyChangeEventRow, DatabaseService};
use sentinel_traffic::PluginManager;
use std::sync::Arc;
use tauri::{AppHandle, State};
use uuid::Uuid;

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

        let has_scheme = canonical_url.starts_with("http://") || canonical_url.starts_with("https://");
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

pub(crate) async fn run_workflow_template_with_inputs(
    app_handle: AppHandle,
    db_service: Arc<DatabaseService>,
    plugin_manager: Arc<PluginManager>,
    template_id: String,
    program_id: Option<String>,
    inputs: serde_json::Value,
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

    log::info!(
        "Starting workflow execution {} for template {}",
        execution_id,
        template_id
    );
    log::info!("Inputs: {:?}", inputs);
    log::info!("Steps: {}", steps.len());

    for step in &steps {
        if !step.input_mappings.is_empty() {
            log::info!(
                "Step '{}' has {} input mappings: {:?}",
                step.id,
                step.input_mappings.len(),
                step.input_mappings
            );
        }
    }

    let exec_id = execution_id.clone();
    let db = db_service.clone();
    let pm = plugin_manager.clone();
    let app = app_handle.clone();
    let initial_inputs = inputs.clone();

    tokio::spawn(async move {
        execute_workflow_steps(exec_id, steps, initial_inputs, program_id, db, pm, app).await;
    });

    Ok(execution_id)
}

#[tauri::command]
pub async fn bounty_run_workflow_template_for_event(
    app_handle: AppHandle,
    db_service: State<'_, Arc<DatabaseService>>,
    plugin_manager: State<'_, Arc<PluginManager>>,
    template_id: String,
    event_id: String,
) -> Result<String, String> {
    let event = db_service
        .get_bounty_change_event(&event_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Change event not found".to_string())?;

    let inputs = build_workflow_trigger_inputs_from_event(db_service.inner(), &event).await?;

    run_workflow_template_with_inputs(
        app_handle,
        db_service.inner().clone(),
        plugin_manager.inner().clone(),
        template_id,
        event.program_id,
        inputs,
    )
    .await
}
