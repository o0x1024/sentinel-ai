//! Bug bounty workflow template and workflow output sinking commands.

use super::bounty_commands::ensure_bounty_feature;
use super::bounty_workflow_event_support::run_workflow_template_with_inputs;
use crate::commands::workflow_notification_support::{
    build_workflow_result_summary_event, emit_workflow_result_summary, summarize_workflow_results,
};
use crate::services::{load_plugin_default_inputs, merge_plugin_input_defaults};
use chrono::Utc;
use sentinel_db::{
    BountyEvidenceRow, BountyFindingRow, BountyWorkflowBindingRow, BountyWorkflowTemplateRow,
    Database, DatabaseService,
};
use sentinel_traffic::PluginManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

// ============================================================================
// Workflow Template Commands
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkflowTemplateRequest {
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub workflow_definition_id: Option<String>,
    pub steps: Vec<WorkflowStepDefinition>,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub tags: Option<Vec<String>>,
    pub estimated_duration_mins: Option<i32>,
}

/// Input mapping for workflow step - defines how to get data from upstream steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputMapping {
    /// Target field name in current step's input (e.g., "targets")
    pub target_field: String,
    /// Source step ID (e.g., "step_subdomain_enum"), or "__trigger__" for workflow initial inputs
    pub source_step_id: String,
    /// JSONPath expression to extract data (e.g., "$.data.subdomains")
    pub source_path: String,
    /// Optional transform: "first", "flatten", "map:fieldName"
    #[serde(default)]
    pub transform: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStepDefinition {
    pub id: String,
    pub name: String,
    pub step_type: String, // "tool", "plugin", "condition", "parallel"
    pub tool_name: Option<String>,
    pub plugin_id: Option<String>,
    pub config: serde_json::Value,
    pub depends_on: Vec<String>,
    /// Explicit input mappings from upstream steps
    #[serde(default)]
    pub input_mappings: Vec<InputMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkflowBindingRequest {
    pub program_id: String,
    pub scope_id: Option<String>,
    pub workflow_template_id: String,
    pub is_enabled: Option<bool>,
    pub auto_run_on_change: Option<bool>,
    pub trigger_conditions: Option<serde_json::Value>,
    pub schedule_cron: Option<String>,
}

fn build_builtin_workflow_templates() -> Vec<BountyWorkflowTemplateRow> {
    let now = Utc::now().to_rfc3339();

    let domain_discovery_steps = vec![
        WorkflowStepDefinition {
            id: "step_subdomain_enum".to_string(),
            name: "Subdomain Enumeration".to_string(),
            step_type: "plugin".to_string(),
            tool_name: None,
            plugin_id: Some("subdomain_enumerator".to_string()),
            config: serde_json::json!({
                "domain": "",
                "removeDuplicates": true,
                "concurrency": 5,
            }),
            depends_on: vec![],
            input_mappings: vec![],
        },
        WorkflowStepDefinition {
            id: "step_dns_resolver".to_string(),
            name: "DNS Resolution".to_string(),
            step_type: "plugin".to_string(),
            tool_name: None,
            plugin_id: Some("dns_resolver".to_string()),
            config: serde_json::json!({
                "targets": [],
                "recordTypes": ["A", "AAAA", "CNAME", "MX", "NS"],
                "concurrency": 10,
            }),
            depends_on: vec!["step_subdomain_enum".to_string()],
            input_mappings: vec![],
        },
        WorkflowStepDefinition {
            id: "step_http_probe".to_string(),
            name: "HTTP Probe".to_string(),
            step_type: "plugin".to_string(),
            tool_name: None,
            plugin_id: Some("http_prober".to_string()),
            config: serde_json::json!({
                "targets": [],
                "ports": [80, 443, 8080, 8443],
                "checkHttp": true,
                "checkHttps": true,
            }),
            depends_on: vec!["step_subdomain_enum".to_string()],
            input_mappings: vec![],
        },
        WorkflowStepDefinition {
            id: "step_tech_fp".to_string(),
            name: "Technology Fingerprint".to_string(),
            step_type: "plugin".to_string(),
            tool_name: None,
            plugin_id: Some("tech_fingerprinter".to_string()),
            config: serde_json::json!({
                "url": "",
            }),
            depends_on: vec!["step_http_probe".to_string()],
            input_mappings: vec![],
        },
        WorkflowStepDefinition {
            id: "step_favicon_fp".to_string(),
            name: "Favicon Fingerprint".to_string(),
            step_type: "plugin".to_string(),
            tool_name: None,
            plugin_id: Some("favicon_fingerprinter".to_string()),
            config: serde_json::json!({
                "targets": [],
                "followRedirects": true,
                "concurrency": 10,
            }),
            depends_on: vec!["step_http_probe".to_string()],
            input_mappings: vec![],
        },
        WorkflowStepDefinition {
            id: "step_cert_monitor".to_string(),
            name: "Certificate Discovery".to_string(),
            step_type: "plugin".to_string(),
            tool_name: None,
            plugin_id: Some("cert_monitor".to_string()),
            config: serde_json::json!({
                "targets": [],
                "checkExpiry": true,
                "expiryWarningDays": 30,
            }),
            depends_on: vec!["step_http_probe".to_string()],
            input_mappings: vec![],
        },
    ];

    let network_service_steps = vec![
        WorkflowStepDefinition {
            id: "step_cidr_mapper".to_string(),
            name: "CIDR Expansion".to_string(),
            step_type: "plugin".to_string(),
            tool_name: None,
            plugin_id: Some("cidr_mapper".to_string()),
            config: serde_json::json!({
                "targets": [],
                "maxHosts": 1024,
                "includeNetworkBroadcast": false,
            }),
            depends_on: vec![],
            input_mappings: vec![],
        },
        WorkflowStepDefinition {
            id: "step_port_monitor".to_string(),
            name: "Port Monitoring".to_string(),
            step_type: "plugin".to_string(),
            tool_name: None,
            plugin_id: Some("port_monitor".to_string()),
            config: serde_json::json!({
                "targets": [],
                "detectService": true,
            }),
            depends_on: vec!["step_cidr_mapper".to_string()],
            input_mappings: vec![],
        },
        WorkflowStepDefinition {
            id: "step_service_fp".to_string(),
            name: "Service Probe".to_string(),
            step_type: "plugin".to_string(),
            tool_name: None,
            plugin_id: Some("service_probe".to_string()),
            config: serde_json::json!({
                "targets": [],
                "readBanner": true,
                "followHttpRedirects": true,
                "serviceProbeEngine": "native",
            }),
            depends_on: vec!["step_port_monitor".to_string()],
            input_mappings: vec![],
        },
    ];

    vec![
        BountyWorkflowTemplateRow {
            id: "builtin-asm-domain-discovery".to_string(),
            name: "ASM Domain Discovery".to_string(),
            description: Some(
                "Enumerate subdomains, resolve DNS, probe web services, fingerprint technologies, and discover certificates."
                    .to_string(),
            ),
            category: "recon".to_string(),
            workflow_definition_id: None,
            steps_json: serde_json::to_string(&domain_discovery_steps).unwrap_or_default(),
            input_schema_json: Some(
                serde_json::json!({
                    "type": "object",
                    "required": ["domain"],
                    "properties": {
                        "domain": { "type": "string", "description": "Root domain to enumerate and map" }
                    }
                })
                .to_string(),
            ),
            output_schema_json: Some(
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "surface_bundle": { "type": "object" }
                    }
                })
                .to_string(),
            ),
            tags_json: Some(
                serde_json::json!(["asm", "surface", "dns", "recon", "web", "favicon"]).to_string(),
            ),
            is_built_in: true,
            estimated_duration_mins: Some(25),
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        BountyWorkflowTemplateRow {
            id: "builtin-asm-network-service-mapping".to_string(),
            name: "ASM Network Service Mapping".to_string(),
            description: Some(
                "Scan exposed ports and fingerprint reachable services for network surface mapping."
                    .to_string(),
            ),
            category: "monitoring".to_string(),
            workflow_definition_id: None,
            steps_json: serde_json::to_string(&network_service_steps).unwrap_or_default(),
            input_schema_json: Some(
                serde_json::json!({
                    "type": "object",
                    "required": ["targets"],
                    "properties": {
                        "targets": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Hosts, IPs, or CIDR ranges to expand, scan, and fingerprint"
                        }
                    }
                })
                .to_string(),
            ),
            output_schema_json: Some(
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "surface_bundle": { "type": "object" }
                    }
                })
                .to_string(),
            ),
            tags_json: Some(
                serde_json::json!(["asm", "surface", "ports", "services", "network", "cidr"]).to_string(),
            ),
            is_built_in: true,
            estimated_duration_mins: Some(20),
            created_at: now.clone(),
            updated_at: now,
        },
    ]
}

/// Create a workflow template
#[tauri::command]
pub async fn bounty_create_workflow_template(
    db_service: State<'_, Arc<DatabaseService>>,
    request: CreateWorkflowTemplateRequest,
) -> Result<BountyWorkflowTemplateRow, String> {
    ensure_bounty_feature()?;

    let now = Utc::now().to_rfc3339();

    let template = BountyWorkflowTemplateRow {
        id: Uuid::new_v4().to_string(),
        name: request.name,
        description: request.description,
        category: request.category,
        workflow_definition_id: request.workflow_definition_id,
        steps_json: serde_json::to_string(&request.steps).unwrap_or_default(),
        input_schema_json: request
            .input_schema
            .map(|s| serde_json::to_string(&s).unwrap_or_default()),
        output_schema_json: request
            .output_schema
            .map(|s| serde_json::to_string(&s).unwrap_or_default()),
        tags_json: request
            .tags
            .map(|t| serde_json::to_string(&t).unwrap_or_default()),
        is_built_in: false,
        estimated_duration_mins: request.estimated_duration_mins,
        created_at: now.clone(),
        updated_at: now,
    };

    db_service
        .create_bounty_workflow_template(&template)
        .await
        .map_err(|e| e.to_string())?;
    Ok(template)
}

/// Get a workflow template by ID
#[tauri::command]
pub async fn bounty_get_workflow_template(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<Option<BountyWorkflowTemplateRow>, String> {
    ensure_bounty_feature()?;

    db_service
        .get_bounty_workflow_template(&id)
        .await
        .map_err(|e| e.to_string())
}

/// List workflow templates
#[tauri::command]
pub async fn bounty_list_workflow_templates(
    db_service: State<'_, Arc<DatabaseService>>,
    category: Option<String>,
    is_built_in: Option<bool>,
) -> Result<Vec<BountyWorkflowTemplateRow>, String> {
    ensure_bounty_feature()?;

    db_service
        .list_bounty_workflow_templates(category.as_deref(), is_built_in)
        .await
        .map_err(|e| e.to_string())
}

/// Delete a workflow template
#[tauri::command]
pub async fn bounty_delete_workflow_template(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<bool, String> {
    ensure_bounty_feature()?;

    db_service
        .delete_bounty_workflow_template(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Update a workflow template
#[tauri::command]
pub async fn bounty_update_workflow_template(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
    request: CreateWorkflowTemplateRequest,
) -> Result<BountyWorkflowTemplateRow, String> {
    ensure_bounty_feature()?;

    let existing = db_service
        .get_bounty_workflow_template(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Template not found".to_string())?;

    let now = Utc::now().to_rfc3339();

    let template = BountyWorkflowTemplateRow {
        id: existing.id,
        name: request.name,
        description: request.description,
        category: request.category,
        workflow_definition_id: request.workflow_definition_id,
        steps_json: serde_json::to_string(&request.steps).unwrap_or_default(),
        input_schema_json: request
            .input_schema
            .map(|s| serde_json::to_string(&s).unwrap_or_default()),
        output_schema_json: request
            .output_schema
            .map(|s| serde_json::to_string(&s).unwrap_or_default()),
        tags_json: request
            .tags
            .map(|t| serde_json::to_string(&t).unwrap_or_default()),
        is_built_in: existing.is_built_in,
        estimated_duration_mins: request.estimated_duration_mins,
        created_at: existing.created_at,
        updated_at: now,
    };

    db_service
        .update_bounty_workflow_template(&template)
        .await
        .map_err(|e| e.to_string())?;
    Ok(template)
}

/// Run a workflow template
#[tauri::command]
pub async fn bounty_run_workflow_template(
    app_handle: AppHandle,
    db_service: State<'_, Arc<DatabaseService>>,
    plugin_manager: State<'_, Arc<PluginManager>>,
    template_id: String,
    program_id: Option<String>,
    inputs: serde_json::Value,
) -> Result<String, String> {
    ensure_bounty_feature()?;

    run_workflow_template_with_inputs(
        app_handle,
        db_service.inner().clone(),
        plugin_manager.inner().clone(),
        template_id,
        program_id,
        inputs,
    )
    .await
}

#[tauri::command]
pub async fn cancel_workflow_run(
    db_service: State<'_, Arc<DatabaseService>>,
    app_handle: AppHandle,
    execution_id: String,
) -> Result<bool, String> {
    ensure_bounty_feature()?;

    let cancelled = sentinel_plugins::cancel_plugin_fetch_requests_by_run(
        &execution_id,
        "workflow run cancelled by user",
    );
    db_service
        .update_workflow_run_status(
            &execution_id,
            "cancelled",
            Some(Utc::now()),
            Some("Workflow run cancelled by user"),
        )
        .await
        .map_err(|error| format!("Failed to update workflow run status: {}", error))?;

    let _ = app_handle.emit(
        "workflow:run-cancelled",
        &serde_json::json!({
            "execution_id": execution_id,
            "cancelled_requests": cancelled,
        }),
    );

    Ok(true)
}

/// Execute workflow steps asynchronously
pub(crate) async fn execute_workflow_steps(
    execution_id: String,
    steps: Vec<WorkflowStepDefinition>,
    initial_inputs: serde_json::Value,
    program_id: Option<String>,
    db: Arc<DatabaseService>,
    plugin_manager: Arc<PluginManager>,
    app_handle: AppHandle,
) -> String {
    let total_steps = steps.len();
    let mut completed_steps = 0;
    let mut step_results: HashMap<String, serde_json::Value> = HashMap::new();
    let mut errors: Vec<serde_json::Value> = Vec::new();

    // Build dependency graph
    let step_map: HashMap<String, &WorkflowStepDefinition> =
        steps.iter().map(|s| (s.id.clone(), s)).collect();

    // Topological sort - execute in dependency order
    let execution_order = topological_sort_steps(&steps);

    for step_id in execution_order {
        if workflow_run_is_cancelled(&db, &execution_id).await {
            let _ = app_handle.emit(
                "workflow:run-cancelled",
                &serde_json::json!({
                    "execution_id": execution_id,
                    "reason": "cancelled before next step",
                }),
            );
            return "cancelled".to_string();
        }

        let step = match step_map.get(&step_id) {
            Some(s) => *s,
            None => continue,
        };

        log::info!("Executing step: {} ({})", step.name, step.id);
        let step_started_at = Utc::now();
        let _ = db
            .save_workflow_run_step(&execution_id, &step.id, "running", step_started_at)
            .await;

        // Emit step start event (running status)
        let _ = app_handle.emit(
            "workflow:step-start",
            &serde_json::json!({
                "execution_id": execution_id,
                "step_id": step.id,
                "step_name": step.name,
                "status": "running"
            }),
        );

        // Resolve inputs from dependencies and initial inputs
        let resolved_inputs = resolve_step_inputs(step, &step_results, &initial_inputs);

        // Execute step
        let result = execute_single_step(
            step,
            &resolved_inputs,
            &plugin_manager,
            &db,
            program_id.as_deref(),
            &execution_id,
        )
        .await;

        match result {
            Ok(output) => {
                step_results.insert(step.id.clone(), output.clone());
                completed_steps += 1;
                let _ = db
                    .update_workflow_run_step_status_internal(
                        &execution_id,
                        &step.id,
                        "completed",
                        Utc::now(),
                        Some(output.to_string()),
                        None,
                    )
                    .await;

                // Emit step complete event
                let _ = app_handle.emit(
                    "workflow:step-complete",
                    &serde_json::json!({
                        "execution_id": execution_id,
                        "step_id": step.id,
                        "step_name": step.name,
                        "result": output,
                        "success": true
                    }),
                );
            }
            Err(e) => {
                log::error!("Step {} failed: {}", step.id, e);
                errors.push(serde_json::json!({
                    "step_id": step.id,
                    "step_name": step.name,
                    "error": e
                }));

                // Mark as failed but continue with other steps
                step_results.insert(
                    step.id.clone(),
                    serde_json::json!({
                        "success": false,
                        "error": e
                    }),
                );
                completed_steps += 1;
                let _ = db
                    .update_workflow_run_step_status_internal(
                        &execution_id,
                        &step.id,
                        "failed",
                        Utc::now(),
                        None,
                        Some(&e),
                    )
                    .await;

                let _ = app_handle.emit(
                    "workflow:step-complete",
                    &serde_json::json!({
                        "execution_id": execution_id,
                        "step_id": step.id,
                        "step_name": step.name,
                        "error": e,
                        "success": false
                    }),
                );
            }
        }

        // Emit progress event
        let progress = ((completed_steps as f32 / total_steps as f32) * 100.0) as u32;
        let _ = db
            .update_workflow_run_progress(
                &execution_id,
                progress,
                completed_steps as u32,
                total_steps as u32,
            )
            .await;
        let _ = app_handle.emit(
            "workflow:progress",
            &serde_json::json!({
                "execution_id": execution_id,
                "progress": progress,
                "completed_steps": completed_steps,
                "total_steps": total_steps
            }),
        );
    }

    // Emit completion event
    let status = if workflow_run_is_cancelled(&db, &execution_id).await {
        "cancelled"
    } else if errors.is_empty() {
        "completed"
    } else {
        "completed_with_errors"
    };
    let _ = app_handle.emit(
        "workflow:run-complete",
        &serde_json::json!({
            "execution_id": execution_id,
            "status": status,
            "total_steps": total_steps,
            "completed_steps": completed_steps,
            "errors": errors,
            "results": step_results
        }),
    );

    log::info!(
        "Workflow execution {} completed with status: {}",
        execution_id,
        status
    );

    if let Some((findings_count, asset_count)) =
        summarize_workflow_results(&step_results, errors.len())
    {
        let workflow_name = db
            .get_workflow_run_detail(&execution_id)
            .await
            .ok()
            .flatten()
            .and_then(|detail| {
                detail
                    .get("workflow_name")
                    .and_then(|value| value.as_str())
                    .map(|value| value.to_string())
            })
            .unwrap_or_else(|| format!("Execution {}", &execution_id[..8.min(execution_id.len())]));

        emit_workflow_result_summary(
            &app_handle,
            &build_workflow_result_summary_event(
                &execution_id,
                &workflow_name,
                status,
                findings_count,
                asset_count,
                errors.len(),
            ),
        );
    }

    status.to_string()
}

async fn workflow_run_is_cancelled(db: &Arc<DatabaseService>, execution_id: &str) -> bool {
    db.get_workflow_run_detail(execution_id)
        .await
        .ok()
        .flatten()
        .and_then(|detail| {
            detail
                .get("status")
                .and_then(|value| value.as_str())
                .map(str::to_string)
        })
        .map(|status| status == "cancelled")
        .unwrap_or(false)
}

/// Topological sort for step execution order
fn topological_sort_steps(steps: &[WorkflowStepDefinition]) -> Vec<String> {
    let mut result = Vec::new();
    let mut visited: HashMap<String, bool> = HashMap::new();
    let step_map: HashMap<String, &WorkflowStepDefinition> =
        steps.iter().map(|s| (s.id.clone(), s)).collect();

    fn visit(
        step_id: &str,
        step_map: &HashMap<String, &WorkflowStepDefinition>,
        visited: &mut HashMap<String, bool>,
        result: &mut Vec<String>,
    ) {
        if visited.get(step_id).copied().unwrap_or(false) {
            return;
        }
        visited.insert(step_id.to_string(), true);

        if let Some(step) = step_map.get(step_id) {
            for dep in &step.depends_on {
                visit(dep, step_map, visited, result);
            }
        }
        result.push(step_id.to_string());
    }

    for step in steps {
        visit(&step.id, &step_map, &mut visited, &mut result);
    }

    result
}

/// Resolve step inputs from dependencies and initial inputs using explicit mappings
fn resolve_step_inputs(
    step: &WorkflowStepDefinition,
    step_results: &HashMap<String, serde_json::Value>,
    initial_inputs: &serde_json::Value,
) -> serde_json::Value {
    const TRIGGER_INPUT_SOURCE_ID: &str = "__trigger__";
    let mut resolved = step.config.clone();

    log::info!(
        "resolve_step_inputs for step '{}': input_mappings count = {}",
        step.id,
        step.input_mappings.len()
    );

    // 1. Merge initial inputs (lowest priority)
    if let (Some(config_obj), Some(initial_obj)) =
        (resolved.as_object_mut(), initial_inputs.as_object())
    {
        for (key, value) in initial_obj {
            if !config_obj.contains_key(key) || is_empty_value(config_obj.get(key).unwrap()) {
                config_obj.insert(key.clone(), value.clone());
            }
        }
    }

    // 2. Apply explicit input mappings (highest priority)
    log::info!(
        "Processing {} input mappings for step '{}'",
        step.input_mappings.len(),
        step.id
    );
    for mapping in &step.input_mappings {
        log::info!(
            "Processing mapping: target={}, source_step={}, source_path={}",
            mapping.target_field,
            mapping.source_step_id,
            mapping.source_path
        );

        let source_result = if mapping.source_step_id == TRIGGER_INPUT_SOURCE_ID {
            Some(initial_inputs)
        } else {
            step_results.get(&mapping.source_step_id)
        };

        if let Some(source_result) = source_result {
            log::info!(
                "Found source_result for step '{}', keys: {:?}",
                mapping.source_step_id,
                source_result
                    .as_object()
                    .map(|o| o.keys().collect::<Vec<_>>())
            );

            if let Some(value) = extract_by_jsonpath(source_result, &mapping.source_path) {
                log::info!(
                    "Extracted value type: {:?}, len: {:?}",
                    value.as_array().map(|_| "array"),
                    value.as_array().map(|a| a.len())
                );
                let transformed = apply_transform(value, mapping.transform.as_deref());
                if let Some(obj) = resolved.as_object_mut() {
                    log::info!(
                        "Applied mapping: {}.{} -> {} (transform: {:?})",
                        mapping.source_step_id,
                        mapping.source_path,
                        mapping.target_field,
                        mapping.transform
                    );
                    obj.insert(mapping.target_field.clone(), transformed);
                }
            } else {
                log::warn!(
                    "Failed to extract value from path '{}' in step '{}'",
                    mapping.source_path,
                    mapping.source_step_id
                );
            }
        } else {
            log::warn!(
                "Source step '{}' not found in results. Available steps: {:?}",
                mapping.source_step_id,
                step_results.keys().collect::<Vec<_>>()
            );
            log::warn!(
                "Source step '{}' not found in results for mapping to '{}'",
                mapping.source_step_id,
                mapping.target_field
            );
        }
    }

    // 3. Auto-resolve common fields if no explicit mappings (fallback for backward compatibility)
    if step.input_mappings.is_empty() {
        for dep_id in &step.depends_on {
            if let Some(dep_result) = step_results.get(dep_id) {
                if let Some(config_obj) = resolved.as_object_mut() {
                    let output_data = dep_result
                        .get("output")
                        .or_else(|| dep_result.get("data"))
                        .unwrap_or(dep_result);

                    // Auto-resolve subdomains -> targets
                    if let Some(subdomains) = output_data.get("subdomains") {
                        if !config_obj.contains_key("targets")
                            || is_empty_value(config_obj.get("targets").unwrap())
                        {
                            if let Some(arr) = subdomains.as_array() {
                                let targets: Vec<String> = arr
                                    .iter()
                                    .filter_map(|s| {
                                        s.as_str().map(|s| s.to_string()).or_else(|| {
                                            s.get("subdomain")
                                                .and_then(|v| v.as_str())
                                                .map(|s| s.to_string())
                                        })
                                    })
                                    .collect();
                                if !targets.is_empty() {
                                    log::info!(
                                        "Auto-resolved {} subdomains as targets",
                                        targets.len()
                                    );
                                    config_obj
                                        .insert("targets".to_string(), serde_json::json!(targets));
                                }
                            }
                        }
                    }

                    // Auto-resolve results[*].url -> urls
                    if let Some(results) = output_data.get("results") {
                        if !config_obj.contains_key("urls")
                            || is_empty_value(config_obj.get("urls").unwrap())
                        {
                            if let Some(arr) = results.as_array() {
                                let urls: Vec<String> = arr
                                    .iter()
                                    .filter_map(|r| {
                                        r.get("url").and_then(|v| v.as_str()).map(|s| s.to_string())
                                    })
                                    .collect();
                                if !urls.is_empty() {
                                    log::info!("Auto-resolved {} results as urls", urls.len());
                                    config_obj.insert("urls".to_string(), serde_json::json!(urls));
                                }
                            }
                        }
                    }

                    // Auto-resolve domain/url
                    if let Some(url) = output_data.get("url").and_then(|v| v.as_str()) {
                        if !config_obj.contains_key("url")
                            || is_empty_value(config_obj.get("url").unwrap())
                        {
                            config_obj.insert("url".to_string(), serde_json::json!(url));
                        }
                    }
                    if let Some(domain) = output_data.get("domain").and_then(|v| v.as_str()) {
                        if !config_obj.contains_key("domain")
                            || is_empty_value(config_obj.get("domain").unwrap())
                        {
                            config_obj.insert("domain".to_string(), serde_json::json!(domain));
                        }
                    }
                }
            }
        }
    }

    resolved
}

/// Extract value from JSON data using JSONPath expression
/// Supports: $.field, $.field.subfield, $.field[0], $.field[*].name
/// Note: step_results wraps plugin output in {"output": ...}, so we try both direct path
/// and path prefixed with "output." for compatibility
fn extract_by_jsonpath(data: &serde_json::Value, path: &str) -> Option<serde_json::Value> {
    // Remove leading "$." if present
    let path = path.strip_prefix("$.").unwrap_or(path);
    let path = path.strip_prefix("$").unwrap_or(path);
    let path = path.trim_start_matches('.');

    if path.is_empty() {
        return Some(data.clone());
    }

    // Try direct path first
    let parts: Vec<&str> = split_jsonpath(path);
    if let Some(result) = extract_recursive(data, &parts) {
        return Some(result);
    }

    // If not found and data has "output" field, try extracting from output
    // This handles the step_results wrapper: {"success": true, "output": {...}}
    if let Some(output) = data.get("output") {
        log::debug!(
            "Path '{}' not found at root, trying under 'output' field",
            path
        );
        if let Some(result) = extract_recursive(output, &parts) {
            return Some(result);
        }
    }

    None
}

/// Split JSONPath into parts, handling brackets correctly
fn split_jsonpath(path: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut in_bracket = false;

    for (i, c) in path.char_indices() {
        match c {
            '[' => in_bracket = true,
            ']' => in_bracket = false,
            '.' if !in_bracket => {
                if i > start {
                    parts.push(&path[start..i]);
                }
                start = i + 1;
            }
            _ => {}
        }
    }

    if start < path.len() {
        parts.push(&path[start..]);
    }

    parts
}

/// Recursively extract value from JSON
fn extract_recursive(data: &serde_json::Value, parts: &[&str]) -> Option<serde_json::Value> {
    if parts.is_empty() {
        return Some(data.clone());
    }

    let part = parts[0];
    let remaining = &parts[1..];

    // Handle array wildcard: field[*]
    if part.ends_with("[*]") {
        let field = &part[..part.len() - 3];
        let arr = if field.is_empty() {
            data.as_array()?
        } else {
            data.get(field)?.as_array()?
        };

        if remaining.is_empty() {
            return Some(serde_json::Value::Array(arr.clone()));
        }

        let mapped: Vec<serde_json::Value> = arr
            .iter()
            .filter_map(|item| extract_recursive(item, remaining))
            .collect();

        return Some(serde_json::Value::Array(mapped));
    }

    // Handle array index: field[0]
    if let Some(bracket_pos) = part.find('[') {
        if part.ends_with(']') {
            let field = &part[..bracket_pos];
            let idx_str = &part[bracket_pos + 1..part.len() - 1];

            if let Ok(idx) = idx_str.parse::<usize>() {
                let arr = if field.is_empty() {
                    data.as_array()?
                } else {
                    data.get(field)?.as_array()?
                };
                return extract_recursive(arr.get(idx)?, remaining);
            }
        }
    }

    // Regular field access
    extract_recursive(data.get(part)?, remaining)
}

/// Apply transform to extracted value
fn apply_transform(value: serde_json::Value, transform: Option<&str>) -> serde_json::Value {
    match transform {
        None => value,
        Some("first") => {
            // Get first element of array
            value
                .as_array()
                .and_then(|a| a.first().cloned())
                .unwrap_or(value)
        }
        Some("flatten") => {
            // Flatten nested arrays
            if let Some(arr) = value.as_array() {
                let flat: Vec<serde_json::Value> = arr
                    .iter()
                    .flat_map(|v| v.as_array().cloned().unwrap_or_else(|| vec![v.clone()]))
                    .collect();
                serde_json::Value::Array(flat)
            } else {
                value
            }
        }
        Some(t) if t.starts_with("map:") => {
            // Extract field from each object in array: map:fieldName
            let field = &t[4..];
            if let Some(arr) = value.as_array() {
                let mapped: Vec<serde_json::Value> = arr
                    .iter()
                    .filter_map(|item| item.get(field).cloned())
                    .collect();
                serde_json::Value::Array(mapped)
            } else {
                value
            }
        }
        Some(unknown) => {
            log::warn!("Unknown transform: {}", unknown);
            value
        }
    }
}

fn is_empty_value(val: &serde_json::Value) -> bool {
    match val {
        serde_json::Value::Null => true,
        serde_json::Value::String(s) => s.is_empty(),
        serde_json::Value::Array(arr) => arr.is_empty(),
        serde_json::Value::Object(obj) => obj.is_empty(),
        _ => false,
    }
}

/// Execute a single workflow step
async fn execute_single_step(
    step: &WorkflowStepDefinition,
    inputs: &serde_json::Value,
    plugin_manager: &Arc<PluginManager>,
    db: &Arc<DatabaseService>,
    program_id: Option<&str>,
    execution_id: &str,
) -> Result<serde_json::Value, String> {
    let plugin_id = step
        .plugin_id
        .as_deref()
        .or(step.tool_name.as_deref())
        .ok_or_else(|| "Step has no plugin_id or tool_name".to_string())?;

    log::info!("Executing plugin '{}' with inputs: {:?}", plugin_id, inputs);

    // Ensure plugin is loaded
    if plugin_manager.get_plugin(plugin_id).await.is_none() {
        log::info!(
            "Plugin '{}' not in memory, loading from database...",
            plugin_id
        );

        if let Ok(Some(plugin_data)) = db.get_plugin_from_registry(plugin_id).await {
            let metadata = sentinel_traffic::PluginMetadata {
                id: plugin_id.to_string(),
                name: plugin_data.metadata.name.clone(),
                version: plugin_data.metadata.version.clone(),
                author: plugin_data.metadata.author.clone(),
                main_category: plugin_data.metadata.main_category,
                category: plugin_data.metadata.category.clone(),
                monitor_type: plugin_data.metadata.monitor_type.clone(),
                description: plugin_data.metadata.description.clone(),
                default_severity: sentinel_traffic::types::Severity::Medium,
                tags: plugin_data.metadata.tags.clone(),
                target_asset_types: plugin_data.metadata.target_asset_types.clone(),
                input_mode: None,
                seed_bindings: plugin_data.metadata.seed_bindings.clone(),
            };

            let code = db
                .get_plugin_code(plugin_id)
                .await
                .ok()
                .flatten()
                .unwrap_or_default();
            // Register as enabled for workflow execution
            let _ = plugin_manager
                .register_plugin(plugin_id.to_string(), metadata, true)
                .await;
            let _ = plugin_manager
                .set_plugin_code(plugin_id.to_string(), code)
                .await;
            log::info!("Plugin '{}' loaded from database and enabled", plugin_id);
        } else {
            return Err(format!("Plugin '{}' not found in database", plugin_id));
        }
    } else {
        // Plugin exists in memory, ensure it's enabled for execution
        if let Err(e) = plugin_manager.enable_plugin(plugin_id).await {
            log::warn!("Failed to enable plugin '{}': {}", plugin_id, e);
        }
    }

    let default_inputs = load_plugin_default_inputs(db.as_ref(), plugin_id).await?;
    let resolved_inputs = merge_plugin_input_defaults(&default_inputs, inputs);

    // Execute plugin
    match plugin_manager
        .execute_execution_plugin(
            plugin_id,
            &resolved_inputs,
            "bounty_workflow",
            Some(execution_id.to_string()),
        )
        .await
    {
        Ok((findings, output)) => {
            let result = serde_json::json!({
                "success": true,
                "plugin_id": plugin_id,
                "findings_count": findings.len(),
                "findings": findings,
                "output": output
            });

            // Auto-sink findings to database if program_id is provided
            if let Some(pid) = program_id {
                for finding in &findings {
                    if let Err(e) = auto_sink_finding(db, pid, finding).await {
                        log::warn!("Failed to auto-sink finding: {}", e);
                    }
                }
            }

            Ok(result)
        }
        Err(e) => Err(format!("Plugin execution failed: {}", e)),
    }
}

/// Auto-sink finding to database
async fn auto_sink_finding(
    db: &Arc<DatabaseService>,
    program_id: &str,
    finding: &sentinel_plugins::Finding,
) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();

    // Calculate fingerprint for deduplication
    let fingerprint = finding.calculate_signature();

    let finding_row = BountyFindingRow {
        id: Uuid::new_v4().to_string(),
        program_id: program_id.to_string(),
        scope_id: None,
        asset_id: None,
        title: finding.title.clone(),
        description: finding.description.clone(),
        finding_type: finding.vuln_type.clone(),
        severity: format!("{:?}", finding.severity).to_lowercase(),
        status: "new".to_string(),
        confidence: format!("{:?}", finding.confidence).to_lowercase(),
        cvss_score: None,
        cwe_id: finding.cwe.clone(),
        affected_url: Some(finding.url.clone()),
        affected_parameter: Some(finding.location.clone()),
        reproduction_steps_json: None,
        impact: None,
        remediation: finding.remediation.clone(),
        evidence_ids_json: None,
        tags_json: None,
        metadata_json: None,
        fingerprint,
        duplicate_of: None,
        first_seen_at: now.clone(),
        last_seen_at: now.clone(),
        verified_at: None,
        created_at: now.clone(),
        updated_at: now,
        created_by: "workflow".to_string(),
    };

    db.create_bounty_finding(&finding_row)
        .await
        .map_err(|e| e.to_string())?;
    log::info!("Auto-sinked finding: {}", finding_row.title);

    Ok(())
}

/// Create a workflow binding
#[tauri::command]
pub async fn bounty_create_workflow_binding(
    db_service: State<'_, Arc<DatabaseService>>,
    request: CreateWorkflowBindingRequest,
) -> Result<BountyWorkflowBindingRow, String> {
    ensure_bounty_feature()?;

    let now = Utc::now().to_rfc3339();

    let binding = BountyWorkflowBindingRow {
        id: Uuid::new_v4().to_string(),
        program_id: request.program_id,
        scope_id: request.scope_id,
        workflow_template_id: request.workflow_template_id,
        is_enabled: request.is_enabled.unwrap_or(true),
        auto_run_on_change: request.auto_run_on_change.unwrap_or(false),
        trigger_conditions_json: request
            .trigger_conditions
            .map(|c| serde_json::to_string(&c).unwrap_or_default()),
        schedule_cron: request.schedule_cron,
        last_run_at: None,
        last_run_status: None,
        run_count: 0,
        created_at: now.clone(),
        updated_at: now,
    };

    db_service
        .create_bounty_workflow_binding(&binding)
        .await
        .map_err(|e| e.to_string())?;
    Ok(binding)
}

/// List workflow bindings
#[tauri::command]
pub async fn bounty_list_workflow_bindings(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: Option<String>,
    scope_id: Option<String>,
    is_enabled: Option<bool>,
) -> Result<Vec<BountyWorkflowBindingRow>, String> {
    ensure_bounty_feature()?;

    db_service
        .list_bounty_workflow_bindings(program_id.as_deref(), scope_id.as_deref(), is_enabled)
        .await
        .map_err(|e| e.to_string())
}

/// Delete a workflow binding
#[tauri::command]
pub async fn bounty_delete_workflow_binding(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<bool, String> {
    ensure_bounty_feature()?;

    db_service
        .delete_bounty_workflow_binding(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Initialize built-in workflow templates
#[tauri::command]
pub async fn bounty_init_builtin_templates(
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<BountyWorkflowTemplateRow>, String> {
    ensure_bounty_feature()?;

    let builtins = build_builtin_workflow_templates();
    let existing = db_service
        .list_bounty_workflow_templates(None, None)
        .await
        .map_err(|e| e.to_string())?;

    let existing_by_id: std::collections::HashMap<String, BountyWorkflowTemplateRow> = existing
        .into_iter()
        .map(|template| (template.id.clone(), template))
        .collect();

    let mut created_or_updated = Vec::new();

    for template in builtins {
        if existing_by_id.contains_key(&template.id) {
            db_service
                .update_bounty_workflow_template(&template)
                .await
                .map_err(|e| e.to_string())?;
        } else {
            db_service
                .create_bounty_workflow_template(&template)
                .await
                .map_err(|e| e.to_string())?;
        }
        created_or_updated.push(template);
    }

    Ok(created_or_updated)
}

// ============================================================================
// Workflow Step Output → Finding/Evidence (Step-level Artifact Sinking)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStepOutput {
    pub step_id: String,
    pub step_name: String,
    pub output_type: String, // "finding", "evidence", "asset", "data"
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SinkWorkflowOutputRequest {
    pub program_id: String,
    pub workflow_run_id: String,
    pub binding_id: Option<String>,
    pub outputs: Vec<WorkflowStepOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SinkWorkflowOutputResponse {
    pub findings_created: Vec<String>,
    pub evidence_created: Vec<String>,
    pub assets_updated: i32,
}

/// Sink workflow step outputs to bounty findings/evidence
#[tauri::command]
pub async fn bounty_sink_workflow_outputs(
    db_service: State<'_, Arc<DatabaseService>>,
    request: SinkWorkflowOutputRequest,
) -> Result<SinkWorkflowOutputResponse, String> {
    ensure_bounty_feature()?;

    let now = Utc::now().to_rfc3339();
    let mut findings_created = Vec::new();
    let mut evidence_created = Vec::new();
    let mut assets_updated = 0;

    for output in request.outputs {
        match output.output_type.as_str() {
            "finding" => {
                // Extract finding data from step output
                if let Some(finding_data) = extract_finding_from_output(&output.data) {
                    let finding_id = Uuid::new_v4().to_string();
                    let fingerprint = format!(
                        "{}:{}:{}:{}",
                        request.program_id,
                        finding_data.finding_type,
                        finding_data.affected_url.as_deref().unwrap_or(""),
                        output.step_id
                    );
                    let fingerprint = format!("{:x}", md5::compute(fingerprint.as_bytes()));

                    // Check for duplicate
                    if db_service
                        .get_bounty_finding_by_fingerprint(&fingerprint)
                        .await
                        .map_err(|e| e.to_string())?
                        .is_some()
                    {
                        continue;
                    }

                    let finding = BountyFindingRow {
                        id: finding_id.clone(),
                        program_id: request.program_id.clone(),
                        scope_id: None,
                        asset_id: None,
                        title: finding_data.title,
                        description: finding_data.description,
                        finding_type: finding_data.finding_type,
                        severity: finding_data
                            .severity
                            .unwrap_or_else(|| "medium".to_string()),
                        status: "new".to_string(),
                        confidence: finding_data
                            .confidence
                            .unwrap_or_else(|| "medium".to_string()),
                        cvss_score: None,
                        cwe_id: finding_data.cwe_id,
                        affected_url: finding_data.affected_url,
                        affected_parameter: finding_data.affected_parameter,
                        reproduction_steps_json: finding_data
                            .reproduction_steps
                            .map(|s| serde_json::to_string(&s).unwrap_or_default()),
                        impact: finding_data.impact,
                        remediation: finding_data.remediation,
                        evidence_ids_json: None,
                        tags_json: Some(
                            serde_json::to_string(&vec!["workflow", "automated"])
                                .unwrap_or_default(),
                        ),
                        metadata_json: Some(
                            serde_json::to_string(&serde_json::json!({
                                "source": "workflow",
                                "workflow_run_id": request.workflow_run_id,
                                "step_id": output.step_id,
                                "step_name": output.step_name,
                            }))
                            .unwrap_or_default(),
                        ),
                        fingerprint,
                        duplicate_of: None,
                        first_seen_at: now.clone(),
                        last_seen_at: now.clone(),
                        verified_at: None,
                        created_at: now.clone(),
                        updated_at: now.clone(),
                        created_by: "workflow".to_string(),
                    };

                    db_service
                        .create_bounty_finding(&finding)
                        .await
                        .map_err(|e| e.to_string())?;
                    findings_created.push(finding_id);
                }
            }
            "evidence" => {
                // Extract evidence data from step output
                if let Some(evidence_data) = extract_evidence_from_output(&output.data) {
                    let evidence_id = Uuid::new_v4().to_string();

                    let evidence = BountyEvidenceRow {
                        id: evidence_id.clone(),
                        finding_id: evidence_data.finding_id.unwrap_or_default(),
                        evidence_type: evidence_data.evidence_type,
                        title: format!("{} - {}", output.step_name, evidence_data.title),
                        description: evidence_data.description,
                        file_path: evidence_data.file_path,
                        file_url: evidence_data.file_url,
                        content: evidence_data.content,
                        mime_type: evidence_data.mime_type,
                        file_size: None,
                        http_request_json: evidence_data
                            .http_request
                            .map(|r| serde_json::to_string(&r).unwrap_or_default()),
                        http_response_json: evidence_data
                            .http_response
                            .map(|r| serde_json::to_string(&r).unwrap_or_default()),
                        diff: evidence_data.diff,
                        tags_json: Some(
                            serde_json::to_string(&vec!["workflow", "automated"])
                                .unwrap_or_default(),
                        ),
                        metadata_json: Some(
                            serde_json::to_string(&serde_json::json!({
                                "workflow_run_id": request.workflow_run_id,
                                "step_id": output.step_id,
                            }))
                            .unwrap_or_default(),
                        ),
                        display_order: 0,
                        created_at: now.clone(),
                        updated_at: now.clone(),
                    };

                    db_service
                        .create_bounty_evidence(&evidence)
                        .await
                        .map_err(|e| e.to_string())?;
                    evidence_created.push(evidence_id);
                }
            }
            "asset" | "data" => {
                // For now, just count these
                assets_updated += 1;
            }
            _ => {}
        }
    }

    // Update binding run status if provided
    if let Some(binding_id) = request.binding_id {
        let status = if findings_created.is_empty() {
            "completed"
        } else {
            "findings_generated"
        };
        let _ = db_service
            .update_bounty_workflow_binding_run_status(&binding_id, status)
            .await;
    }

    Ok(SinkWorkflowOutputResponse {
        findings_created,
        evidence_created,
        assets_updated,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExtractedFinding {
    title: String,
    description: String,
    finding_type: String,
    severity: Option<String>,
    confidence: Option<String>,
    affected_url: Option<String>,
    affected_parameter: Option<String>,
    cwe_id: Option<String>,
    impact: Option<String>,
    remediation: Option<String>,
    reproduction_steps: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExtractedEvidence {
    finding_id: Option<String>,
    evidence_type: String,
    title: String,
    description: Option<String>,
    file_path: Option<String>,
    file_url: Option<String>,
    content: Option<String>,
    mime_type: Option<String>,
    http_request: Option<serde_json::Value>,
    http_response: Option<serde_json::Value>,
    diff: Option<String>,
}

fn extract_finding_from_output(data: &serde_json::Value) -> Option<ExtractedFinding> {
    // Try to parse the data as a finding structure
    if let Some(obj) = data.as_object() {
        Some(ExtractedFinding {
            title: obj
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled Finding")
                .to_string(),
            description: obj
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            finding_type: obj
                .get("type")
                .or(obj.get("finding_type"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            severity: obj
                .get("severity")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            confidence: obj
                .get("confidence")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            affected_url: obj
                .get("url")
                .or(obj.get("affected_url"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            affected_parameter: obj
                .get("parameter")
                .or(obj.get("affected_parameter"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            cwe_id: obj
                .get("cwe")
                .or(obj.get("cwe_id"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            impact: obj
                .get("impact")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            remediation: obj
                .get("remediation")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            reproduction_steps: obj
                .get("steps")
                .or(obj.get("reproduction_steps"))
                .and_then(|v| {
                    if let Some(arr) = v.as_array() {
                        Some(
                            arr.iter()
                                .filter_map(|s| s.as_str().map(|s| s.to_string()))
                                .collect(),
                        )
                    } else {
                        None
                    }
                }),
        })
    } else {
        None
    }
}

fn extract_evidence_from_output(data: &serde_json::Value) -> Option<ExtractedEvidence> {
    if let Some(obj) = data.as_object() {
        Some(ExtractedEvidence {
            finding_id: obj
                .get("finding_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            evidence_type: obj
                .get("type")
                .or(obj.get("evidence_type"))
                .and_then(|v| v.as_str())
                .unwrap_or("other")
                .to_string(),
            title: obj
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Evidence")
                .to_string(),
            description: obj
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            file_path: obj
                .get("file_path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            file_url: obj
                .get("file_url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            content: obj
                .get("content")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            mime_type: obj
                .get("mime_type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            http_request: obj.get("request").or(obj.get("http_request")).cloned(),
            http_response: obj.get("response").or(obj.get("http_response")).cloned(),
            diff: obj
                .get("diff")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        })
    } else {
        None
    }
}
