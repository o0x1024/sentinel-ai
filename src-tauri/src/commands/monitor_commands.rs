//! Asset Monitor Scheduler Commands

use crate::commands::monitor_config_support::{load_tasks_from_db, save_tasks_to_db};
use crate::commands::monitor_execution_heartbeat_support::start_monitor_execution_heartbeat;
use crate::commands::monitor_finding_support::import_monitor_findings_from_output;
use crate::commands::monitor_notification_support::{
    build_monitor_plugin_failure_event, build_monitor_task_summary_event,
    emit_monitor_plugin_failure, emit_monitor_task_summary, normalize_monitor_error_message,
};
use crate::commands::monitor_plugin_execution_support::execute_monitor_plugin;
use crate::commands::monitor_plugin_output_support::{
    extract_plugin_failure, extract_service_probe_engine_used,
};
use crate::commands::monitor_progress_support::{
    build_monitor_task_log_event, build_monitor_task_progress_event, collect_monitor_plugins,
    emit_monitor_task_log, emit_monitor_task_progress,
};
use crate::commands::monitor_snapshot_support::{
    persist_api_monitor_inventory_output, persist_monitor_plugin_snapshots_to_task,
};
use crate::commands::monitor_surface_support::{
    collect_monitor_target_payload_for_plugin, ingest_surface_plugin_output, MonitorResolvedTargets,
};
use chrono::Utc;
use sentinel_bounty::services::{MonitorPluginConfig, MonitorScheduler, MonitorStats, MonitorTask};
use sentinel_db::{BountyAssetRow, DatabaseService};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::RwLock;
use uuid::Uuid;

pub(crate) fn inject_monitor_plugin_targets(
    input: &mut serde_json::Value,
    resolved_targets: &MonitorResolvedTargets,
) {
    let plugin_targets = &resolved_targets.targets;
    let plugin_target_domains = plugin_targets.join(",");

    input["domains"] = serde_json::Value::String(plugin_target_domains.clone());
    input["urls"] = serde_json::Value::String(plugin_target_domains.clone());
    input["targets"] = serde_json::to_value(plugin_targets).unwrap_or(serde_json::json!([]));
    input["target_objects"] = serde_json::Value::Array(resolved_targets.target_objects.clone());

    if let Some(first) = plugin_targets.first() {
        input["domain"] = serde_json::Value::String(first.clone());
        let url = if first.starts_with("http") {
            first.clone()
        } else {
            format!("https://{}", first)
        };
        input["url"] = serde_json::Value::String(url);
    } else {
        input["domain"] = serde_json::Value::Null;
        input["url"] = serde_json::Value::Null;
    }

    let service_targets: Vec<serde_json::Value> = resolved_targets
        .target_objects
        .iter()
        .filter(|target| target.get("type").and_then(|value| value.as_str()) == Some("service"))
        .cloned()
        .collect();
    input["service_targets"] = serde_json::Value::Array(service_targets);

    for (key, value) in &resolved_targets.extra_input {
        input[key] = value.clone();
    }
}

pub(crate) fn inject_monitor_execution_context(
    input: &mut serde_json::Value,
    task: &MonitorTask,
    execution_mode: &str,
    started_at: &str,
    current_plugin: &str,
    current_plugin_index: usize,
    completed_steps: usize,
    total_steps: usize,
    imported_assets: usize,
) {
    input["__monitorExecution"] = serde_json::json!({
        "task_id": task.id,
        "task_name": task.name,
        "program_id": task.program_id,
        "execution_mode": execution_mode,
        "started_at": started_at,
        "current_plugin": current_plugin,
        "current_plugin_index": current_plugin_index,
        "completed_steps": completed_steps,
        "total_steps": total_steps,
        "imported_assets": imported_assets,
    });
}

fn configured_service_probe_engine(plugin: &MonitorPluginConfig) -> Option<String> {
    let normalized_plugin_id = plugin
        .plugin_id
        .strip_prefix("plugin__")
        .unwrap_or(&plugin.plugin_id);
    if !matches!(normalized_plugin_id, "service_monitor" | "service_probe") {
        return None;
    }

    plugin
        .plugin_params
        .get("serviceProbeEngine")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_lowercase())
}

pub(crate) fn monitor_plugin_runtime_label(
    plugin: &MonitorPluginConfig,
    output: Option<&serde_json::Value>,
) -> String {
    let engine = output
        .and_then(extract_service_probe_engine_used)
        .or_else(|| configured_service_probe_engine(plugin));

    match engine {
        Some(engine) => format!("{} [{}]", plugin.plugin_id, engine),
        None => plugin.plugin_id.clone(),
    }
}

/// Global monitor scheduler state
pub struct MonitorSchedulerState {
    pub scheduler: Arc<MonitorScheduler>,
    pub initialized: bool,
    pub running_task_ids: Arc<RwLock<HashSet<String>>>,
    pub cancel_requested_task_ids: Arc<RwLock<HashSet<String>>>,
    pub active_task_run_ids: Arc<RwLock<HashMap<String, String>>>,
}

impl MonitorSchedulerState {
    pub fn new() -> Self {
        Self {
            scheduler: Arc::new(MonitorScheduler::new()),
            initialized: false,
            running_task_ids: Arc::new(RwLock::new(HashSet::new())),
            cancel_requested_task_ids: Arc::new(RwLock::new(HashSet::new())),
            active_task_run_ids: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

async fn ensure_monitor_scheduler_initialized(
    state: &Arc<RwLock<MonitorSchedulerState>>,
    db_service: &Arc<DatabaseService>,
) -> Result<(), String> {
    {
        let state_read = state.read().await;
        if state_read.initialized {
            return Ok(());
        }
    }

    let mut state_write = state.write().await;
    if !state_write.initialized {
        load_tasks_from_db(&state_write.scheduler, db_service).await?;
        state_write.initialized = true;
    }

    Ok(())
}

// ============================================================================
// Scheduler Control Commands
// ============================================================================

/// Start the monitoring scheduler
#[tauri::command]
pub async fn monitor_start_scheduler(
    state: State<'_, Arc<RwLock<MonitorSchedulerState>>>,
    db_service: State<'_, Arc<DatabaseService>>,
    plugin_manager: State<'_, Arc<sentinel_traffic::PluginManager>>,
    app: AppHandle,
) -> Result<bool, String> {
    ensure_monitor_scheduler_initialized(state.inner(), db_service.inner()).await?;

    let state_guard = state.read().await;
    let scheduler = state_guard.scheduler.clone();
    let running_task_ids = state_guard.running_task_ids.clone();
    let cancel_requested_task_ids = state_guard.cancel_requested_task_ids.clone();
    let active_task_run_ids = state_guard.active_task_run_ids.clone();
    let app_for_executor = app.clone();
    let scheduler_for_executor = scheduler.clone();
    drop(state_guard);

    // Set up event callback to emit Tauri events and save to DB
    let app_clone = app.clone();
    let db_for_event = (*db_service).clone();
    let pm_for_event = (*plugin_manager).clone();

    scheduler.set_event_callback(move |event| {
        let _ = app_clone.emit("monitor:change-detected", &event);

        let db = db_for_event.clone();
        let pm = pm_for_event.clone();
        let app_handle = app_clone.clone();

        tokio::spawn(async move {
            let row = sentinel_db::BountyChangeEventRow {
                id: event.id.clone(),
                program_id: event.program_id.clone(),
                asset_id: event.asset_id.clone(),
                event_type: serde_json::to_string(&event.event_type).unwrap_or_default().trim_matches('"').to_string(),
                severity: serde_json::to_string(&event.severity).unwrap_or_default().trim_matches('"').to_string(),
                status: "new".to_string(),
                title: event.title.clone(),
                description: event.description.clone(),
                old_value: event.old_value.clone(),
                new_value: event.new_value.clone(),
                diff: event.diff.clone(),
                affected_scope: event.affected_scope.clone(),
                detection_method: event.detection_method.clone(),
                generated_findings_json: if !event.generated_findings.is_empty() { Some(serde_json::to_string(&event.generated_findings).unwrap_or_default()) } else { None },
                tags_json: if !event.tags.is_empty() { Some(serde_json::to_string(&event.tags).unwrap_or_default()) } else { None },
                metadata_json: if !event.metadata.is_empty() { Some(serde_json::to_string(&event.metadata).unwrap_or_default()) } else { None },
                risk_score: event.risk_score,
                auto_trigger_enabled: event.auto_trigger_enabled,
                created_at: event.created_at.to_rfc3339(),
                updated_at: event.updated_at.to_rfc3339(),
                resolved_at: event.resolved_at.map(|d| d.to_rfc3339()),
            };

            if let Err(e) = db.create_bounty_change_event(&row).await {
                tracing::error!("Failed to save change event to DB: {}", e);
                return;
            }

            if event.auto_trigger_enabled {
                let _ = crate::commands::bounty_workflow_event_support::bounty_trigger_workflows_for_event_internal(
                    app_handle,
                    db.clone(),
                    pm.clone(),
                    event.id.clone()
                ).await;
            }
        });
    }).await;

    // Register Task Executor
    let db_service_clone = db_service.inner().clone();
    let plugin_manager_for_executor = plugin_manager.inner().clone();
    let running_task_ids_for_executor = running_task_ids.clone();
    let cancel_requested_task_ids_for_executor = cancel_requested_task_ids.clone();
    let active_task_run_ids_for_executor = active_task_run_ids.clone();
    let app_for_executor_clone = app_for_executor.clone();
    scheduler
        .set_task_executor(move |task| {
            let db = db_service_clone.clone();
            let plugin_manager = plugin_manager_for_executor.clone();
            let scheduler = scheduler_for_executor.clone();
            let running_task_ids = running_task_ids_for_executor.clone();
            let cancel_requested_task_ids = cancel_requested_task_ids_for_executor.clone();
            let active_task_run_ids = active_task_run_ids_for_executor.clone();
            let app_handle = app_for_executor_clone.clone();
            Box::pin(async move {
                let task_id = task.id.clone();
                let execution_started_at = Utc::now().to_rfc3339();
                {
                    let mut running = running_task_ids.write().await;
                    running.insert(task_id.clone());
                }
                {
                    let mut cancel = cancel_requested_task_ids.write().await;
                    cancel.remove(&task_id);
                }
                {
                    let run_id = format!("monitor:{task_id}:{execution_started_at}");
                    let mut active_runs = active_task_run_ids.write().await;
                    active_runs.insert(task_id.clone(), run_id);
                }

                let result: Result<Vec<sentinel_bounty::models::ChangeEvent>, String> = async {
                    tracing::info!("Executor running for task: {}", task.name);
                    let all_events = Vec::new();
                    let mut total_imported = 0usize;
                    let mut total_findings_created = 0usize;
                    let mut total_findings_updated = 0usize;
                    let mut completed_steps = 0usize;
                    let mut stopped = false;

                    let plugins_to_run = collect_monitor_plugins(&task);
                    let total_steps = plugins_to_run.len();

                    emit_monitor_task_progress(
                        &app_handle,
                        &build_monitor_task_progress_event(
                            &task,
                            "scheduler",
                            "running",
                            0,
                            total_steps,
                            None,
                            None,
                            0,
                            0,
                            Some("Preparing monitor task".to_string()),
                            &execution_started_at,
                        ),
                    );

                    if total_steps == 0 {
                        emit_monitor_task_progress(
                            &app_handle,
                            &build_monitor_task_progress_event(
                                &task,
                                "scheduler",
                                "completed",
                                0,
                                0,
                                None,
                                None,
                                0,
                                0,
                                Some("No monitor plugins configured".to_string()),
                                &execution_started_at,
                            ),
                        );
                        emit_monitor_task_summary(
                            &app_handle,
                            &build_monitor_task_summary_event(
                                &task.id,
                                &task.name,
                                &task.program_id,
                                "scheduler",
                                "completed",
                                0,
                                0,
                                0,
                                &execution_started_at,
                                Some("No monitor plugins configured".to_string()),
                            ),
                        );
                        return Ok(vec![]);
                    }

                    for (index, plugin) in plugins_to_run.iter().enumerate() {
                        let imported_before_plugin = total_imported;
                        let configured_plugin_label = monitor_plugin_runtime_label(plugin, None);
                        let mut effective_plugin_label = configured_plugin_label.clone();
                        if cancel_requested_task_ids.read().await.contains(&task_id) {
                            tracing::info!(
                                "Task '{}' stopped by user request before running plugin '{}'",
                                task.name,
                                configured_plugin_label
                            );
                            stopped = true;
                            emit_monitor_task_progress(
                                &app_handle,
                                &build_monitor_task_progress_event(
                                    &task,
                                    "scheduler",
                                    "stopped",
                                    completed_steps,
                                    total_steps,
                                    Some(configured_plugin_label.as_str()),
                                    Some(index + 1),
                                    0,
                                    total_imported,
                                    Some(format!("Stopped before plugin {}", configured_plugin_label)),
                                    &execution_started_at,
                                ),
                            );
                            emit_monitor_task_log(
                                &app_handle,
                                &build_monitor_task_log_event(
                                    &task,
                                    configured_plugin_label.as_str(),
                                    "scheduler",
                                    "stopped",
                                    index + 1,
                                    total_steps,
                                    None,
                                    0,
                                    total_imported,
                                    format!("Stopped before plugin {}", configured_plugin_label),
                                ),
                            );
                            break;
                        }

                        tracing::info!(
                            "Running plugin chain: {} for task {}",
                            configured_plugin_label,
                            task.name
                        );
                        emit_monitor_task_progress(
                            &app_handle,
                            &build_monitor_task_progress_event(
                                &task,
                                "scheduler",
                                "running",
                                completed_steps,
                                total_steps,
                                Some(configured_plugin_label.as_str()),
                                Some(index + 1),
                                0,
                                total_imported,
                                Some(format!("Running plugin {}", configured_plugin_label)),
                                &execution_started_at,
                            ),
                        );
                        emit_monitor_task_log(
                            &app_handle,
                            &build_monitor_task_log_event(
                                &task,
                                configured_plugin_label.as_str(),
                                "scheduler",
                                "running",
                                index + 1,
                                total_steps,
                                None,
                                0,
                                total_imported,
                                format!("Running plugin {}", configured_plugin_label),
                            ),
                        );

                        let mut selected_plugin: Option<MonitorPluginConfig> = None;
                        let mut selected_result = None;
                        let mut selected_resolved_targets = None;
                        let mut selected_plugin_started_at = None;
                        let mut last_failure_reason = String::from("monitor plugin execution failed");
                        let mut last_attempt_label = configured_plugin_label.clone();
                        let mut last_attempt_plugin_id = plugin.plugin_id.clone();

                        for candidate in plugin.execution_chain() {
                            let attempt_label = monitor_plugin_runtime_label(&candidate, None);
                            last_attempt_label = attempt_label.clone();
                            last_attempt_plugin_id = candidate.plugin_id.clone();

                            let resolved_targets = match collect_monitor_target_payload_for_plugin(
                                &db, &task, &candidate,
                            )
                            .await
                            {
                                Ok(value) => value,
                                Err(error) => {
                                    last_failure_reason = error.clone();
                                    emit_monitor_task_log(
                                        &app_handle,
                                        &build_monitor_task_log_event(
                                            &task,
                                            attempt_label.as_str(),
                                            "scheduler",
                                            "failed",
                                            index + 1,
                                            total_steps,
                                            None,
                                            0,
                                            total_imported,
                                            format!(
                                                "Plugin {} target resolution failed: {}",
                                                attempt_label, error
                                            ),
                                        ),
                                    );
                                    continue;
                                }
                            };
                            let plugin_targets = &resolved_targets.targets;

                            if cancel_requested_task_ids.read().await.contains(&task_id) {
                                tracing::info!(
                                    "Task '{}' stopped by user request before running plugin '{}'",
                                    task.name,
                                    attempt_label
                                );
                                stopped = true;
                                emit_monitor_task_progress(
                                    &app_handle,
                                    &build_monitor_task_progress_event(
                                        &task,
                                        "scheduler",
                                        "stopped",
                                        completed_steps,
                                        total_steps,
                                        Some(attempt_label.as_str()),
                                        Some(index + 1),
                                        plugin_targets.len(),
                                        total_imported,
                                        Some(format!("Stopped before plugin {}", attempt_label)),
                                        &execution_started_at,
                                    ),
                                );
                                emit_monitor_task_log(
                                    &app_handle,
                                    &build_monitor_task_log_event(
                                        &task,
                                        attempt_label.as_str(),
                                        "scheduler",
                                        "stopped",
                                        index + 1,
                                        total_steps,
                                        None,
                                        0,
                                        total_imported,
                                        format!("Stopped before plugin {}", attempt_label),
                                    ),
                                );
                                break;
                            }

                            if plugin_targets.is_empty() {
                                last_failure_reason = format!(
                                    "Plugin {} skipped because no targets matched its declared asset types",
                                    attempt_label
                                );
                                emit_monitor_task_log(
                                    &app_handle,
                                    &build_monitor_task_log_event(
                                        &task,
                                        attempt_label.as_str(),
                                        "scheduler",
                                        "stopped",
                                        index + 1,
                                        total_steps,
                                        None,
                                        0,
                                        total_imported,
                                        last_failure_reason.clone(),
                                    ),
                                );
                                continue;
                            }

                            let plugin_started_at = Instant::now();
                            let mut input = candidate.plugin_params.clone();
                            if !input.is_object() {
                                input = serde_json::json!({});
                            }
                            inject_monitor_plugin_targets(&mut input, &resolved_targets);
                            inject_monitor_execution_context(
                                &mut input,
                                &task,
                                "scheduler",
                                &execution_started_at,
                                attempt_label.as_str(),
                                index + 1,
                                completed_steps,
                                total_steps,
                                total_imported,
                            );

                            let heartbeat = start_monitor_execution_heartbeat(
                                app_handle.clone(),
                                task.clone(),
                                "scheduler",
                                completed_steps,
                                total_steps,
                                attempt_label.clone(),
                                index + 1,
                                plugin_targets.len(),
                                total_imported,
                                execution_started_at.clone(),
                                plugin_started_at,
                            );
                            let result = execute_monitor_plugin(
                                &db,
                                &plugin_manager,
                                &candidate.plugin_id,
                                input,
                            )
                            .await;
                            heartbeat.stop().await;

                            if !result.success {
                                last_failure_reason = normalize_monitor_error_message(
                                    result
                                        .error
                                        .as_deref()
                                        .unwrap_or("monitor plugin execution failed"),
                                );
                                tracing::error!("Plugin {} failed: {:?}", attempt_label, result.error);
                                emit_monitor_task_log(
                                    &app_handle,
                                    &build_monitor_task_log_event(
                                        &task,
                                        attempt_label.as_str(),
                                        "scheduler",
                                        "failed",
                                        index + 1,
                                        total_steps,
                                        Some(plugin_started_at.elapsed().as_millis() as u64),
                                        0,
                                        total_imported,
                                        format!(
                                            "Plugin {} failed: {}",
                                            attempt_label, last_failure_reason
                                        ),
                                    ),
                                );
                                continue;
                            }

                            if let Some(output) = &result.output {
                                let runtime_plugin_label =
                                    monitor_plugin_runtime_label(&candidate, Some(output));
                                if let Some(plugin_error) = extract_plugin_failure(output) {
                                    last_failure_reason =
                                        normalize_monitor_error_message(plugin_error.as_str());
                                    tracing::error!(
                                        "Plugin {} returned failure output for task {}: {}",
                                        runtime_plugin_label,
                                        task.name,
                                        plugin_error
                                    );
                                    emit_monitor_task_log(
                                        &app_handle,
                                        &build_monitor_task_log_event(
                                            &task,
                                            runtime_plugin_label.as_str(),
                                            "scheduler",
                                            "failed",
                                            index + 1,
                                            total_steps,
                                            Some(plugin_started_at.elapsed().as_millis() as u64),
                                            0,
                                            total_imported,
                                            format!(
                                                "Plugin {} returned failure: {}",
                                                runtime_plugin_label, last_failure_reason
                                            ),
                                        ),
                                    );
                                    continue;
                                }

                                effective_plugin_label = runtime_plugin_label;
                            } else {
                                effective_plugin_label = attempt_label.clone();
                            }

                            selected_plugin = Some(candidate);
                            selected_result = Some(result);
                            selected_resolved_targets = Some(resolved_targets);
                            selected_plugin_started_at = Some(plugin_started_at);
                            break;
                        }

                        if stopped {
                            break;
                        }

                        if selected_plugin.is_none() {
                            completed_steps = index + 1;
                            emit_monitor_plugin_failure(
                                &app_handle,
                                &build_monitor_plugin_failure_event(
                                    &task.id,
                                    &task.name,
                                    &task.program_id,
                                    "scheduler",
                                    &last_attempt_plugin_id,
                                    last_attempt_label.as_str(),
                                    last_failure_reason.as_str(),
                                    &execution_started_at,
                                ),
                            );
                            emit_monitor_task_progress(
                                &app_handle,
                                &build_monitor_task_progress_event(
                                    &task,
                                    "scheduler",
                                    "running",
                                    completed_steps,
                                    total_steps,
                                    Some(last_attempt_label.as_str()),
                                    Some(index + 1),
                                    0,
                                    total_imported,
                                    Some(format!(
                                        "Plugin {} failed: {}",
                                        last_attempt_label, last_failure_reason
                                    )),
                                    &execution_started_at,
                                ),
                            );
                            emit_monitor_task_log(
                                &app_handle,
                                &build_monitor_task_log_event(
                                    &task,
                                    last_attempt_label.as_str(),
                                    "scheduler",
                                    "failed",
                                    index + 1,
                                    total_steps,
                                    None,
                                    0,
                                    total_imported,
                                    format!(
                                        "Plugin {} failed after exhausting fallback chain: {}",
                                        last_attempt_label, last_failure_reason
                                    ),
                                ),
                            );
                            continue;
                        }

                        let plugin = selected_plugin.as_ref().expect("selected plugin must exist");
                        let result = selected_result.as_ref().expect("selected result must exist");
                        let resolved_targets = selected_resolved_targets
                            .as_ref()
                            .expect("selected targets must exist");
                        let plugin_targets = &resolved_targets.targets;
                        let plugin_started_at =
                            selected_plugin_started_at.expect("selected plugin start time must exist");

                        // Process Output
                        if let Some(output) = &result.output {
                            let runtime_plugin_label =
                                monitor_plugin_runtime_label(plugin, Some(output));
                            effective_plugin_label = runtime_plugin_label.clone();
                            if let Some(plugin_error) = extract_plugin_failure(output) {
                                let failure_reason =
                                    normalize_monitor_error_message(plugin_error.as_str());
                                tracing::error!(
                                    "Plugin {} returned failure output for task {}: {}",
                                    runtime_plugin_label,
                                    task.name,
                                    plugin_error
                                );
                                completed_steps = index + 1;
                                emit_monitor_plugin_failure(
                                    &app_handle,
                                    &build_monitor_plugin_failure_event(
                                        &task.id,
                                        &task.name,
                                        &task.program_id,
                                        "scheduler",
                                        &plugin.plugin_id,
                                        runtime_plugin_label.as_str(),
                                        failure_reason.as_str(),
                                        &execution_started_at,
                                    ),
                                );
                                emit_monitor_task_progress(
                                    &app_handle,
                                    &build_monitor_task_progress_event(
                                        &task,
                                        "scheduler",
                                        "running",
                                        completed_steps,
                                        total_steps,
                                        Some(runtime_plugin_label.as_str()),
                                        Some(index + 1),
                                        plugin_targets.len(),
                                        total_imported,
                                        Some(format!(
                                            "Plugin {} returned failure: {}",
                                            runtime_plugin_label, failure_reason
                                        )),
                                        &execution_started_at,
                                    ),
                                );
                                emit_monitor_task_log(
                                    &app_handle,
                                    &build_monitor_task_log_event(
                                        &task,
                                        runtime_plugin_label.as_str(),
                                        "scheduler",
                                        "failed",
                                        index + 1,
                                        total_steps,
                                        Some(plugin_started_at.elapsed().as_millis() as u64),
                                        0,
                                        total_imported,
                                        format!(
                                            "Plugin {} returned failure: {}",
                                            runtime_plugin_label, failure_reason
                                        ),
                                    ),
                                );
                                continue;
                            }

                            if let Err(error) = persist_api_monitor_inventory_output(
                                &db,
                                &task,
                                plugin,
                                output,
                                "scheduler",
                                Some(&task.id),
                            )
                            .await
                            {
                                tracing::warn!(
                                    "Failed to persist API monitor inventory for task {}: {}",
                                    task.id,
                                    error
                                );
                            }
                            let mut task_state_updated = false;
                            if let Err(error) = scheduler
                                .update_task(&task.id, |scheduled_task| {
                                    if persist_monitor_plugin_snapshots_to_task(
                                        scheduled_task,
                                        plugin,
                                        output,
                                    ) {
                                        task_state_updated = true;
                                    }
                                })
                                .await
                            {
                                tracing::warn!(
                                    "Failed to update persisted monitor task state for {}: {}",
                                    task.id,
                                    error
                                );
                            } else if task_state_updated {
                                if let Err(error) = save_tasks_to_db(&scheduler, &db).await {
                                    tracing::warn!(
                                        "Failed to save monitor tasks after plugin {}: {}",
                                        runtime_plugin_label,
                                        error
                                    );
                                }
                            }

                            match ingest_surface_plugin_output(
                                &db,
                                &task.program_id,
                                &plugin.plugin_id,
                                "monitor_scheduler",
                                Some(&task.id),
                                output,
                                Some(serde_json::json!({
                                    "task_id": task.id,
                                    "task_name": task.name,
                                    "execution_mode": "scheduler",
                                })),
                            )
                            .await
                            {
                                Ok(Some(stats)) => {
                                    total_imported += stats.created_assets;
                                    tracing::info!(
                                        "Scheduler task '{}' materialized {} surface assets and enriched {} existing assets for plugin '{}'",
                                        task.name,
                                        stats.created_assets,
                                        stats.enriched_assets,
                                        runtime_plugin_label
                                    );
                                    completed_steps = index + 1;
                                    emit_monitor_task_progress(
                                        &app_handle,
                                        &build_monitor_task_progress_event(
                                            &task,
                                            "scheduler",
                                            "running",
                                            completed_steps,
                                            total_steps,
                                            Some(runtime_plugin_label.as_str()),
                                            Some(index + 1),
                                            plugin_targets.len(),
                                            total_imported,
                                            Some(format!("Plugin {} completed", runtime_plugin_label)),
                                            &execution_started_at,
                                        ),
                                    );
                                    emit_monitor_task_log(
                                        &app_handle,
                                        &build_monitor_task_log_event(
                                            &task,
                                            runtime_plugin_label.as_str(),
                                            "scheduler",
                                            "completed",
                                            index + 1,
                                            total_steps,
                                            Some(plugin_started_at.elapsed().as_millis() as u64),
                                            total_imported.saturating_sub(imported_before_plugin),
                                            total_imported,
                                            format!("Plugin {} completed", runtime_plugin_label),
                                        ),
                                    );
                                    continue;
                                }
                                Ok(None) => {}
                                Err(e) => tracing::warn!(
                                    "Failed to ingest surface output for scheduled task '{}' plugin '{}': {}",
                                    task.name,
                                    runtime_plugin_label,
                                    e
                                ),
                            }

                            tracing::info!("Plugin {} output: {}", runtime_plugin_label, output);

                            // Normalize output: wrapping in 'data' or using direct keys
                            let data = output.get("data").unwrap_or(output);

                            match import_monitor_findings_from_output(
                                &db,
                                &task,
                                &plugin.plugin_id,
                                output,
                                "scheduler",
                            )
                            .await
                            {
                                Ok(stats) if stats.created > 0 || stats.updated > 0 => {
                                    total_findings_created += stats.created;
                                    total_findings_updated += stats.updated;
                                    tracing::info!(
                                        "Scheduler task '{}' imported {} findings and refreshed {} existing findings for plugin '{}'",
                                        task.name,
                                        stats.created,
                                        stats.updated,
                                        runtime_plugin_label
                                    );
                                }
                                Ok(_) => {}
                                Err(error) => tracing::warn!(
                                    "Failed to import monitor findings for scheduled task '{}' plugin '{}': {}",
                                    task.name,
                                    runtime_plugin_label,
                                    error
                                ),
                            }

                            let mut discovered_assets = Vec::new();

                            // 1. Check for explicit 'subdomains' list
                            if let Some(subdomains) =
                                data.get("subdomains").and_then(|v| v.as_array())
                            {
                                for sub in subdomains {
                                    let sub_str = sub
                                        .as_str()
                                        .or_else(|| sub.get("domain").and_then(|s| s.as_str()))
                                        .unwrap_or("");
                                    if !sub_str.is_empty() {
                                        discovered_assets.push(sub_str.to_string());
                                    }
                                }
                            }

                            // 1.1 Check for 'urls' list (Standard Recon)
                            if let Some(urls) = data.get("urls").and_then(|v| v.as_array()) {
                                for url in urls {
                                    if let Some(url_str) = url.as_str() {
                                        if !url_str.is_empty() {
                                            // Extract domain from URL or keep as is?
                                            // For now, let's treat it as an asset source
                                            discovered_assets.push(url_str.to_string());
                                        }
                                    }
                                }
                            }

                            // 1.2 Check for 'ips' list (Standard Recon)
                            if let Some(ips) = data.get("ips").and_then(|v| v.as_array()) {
                                for ip in ips {
                                    if let Some(ip_str) = ip.as_str() {
                                        if !ip_str.is_empty() {
                                            discovered_assets.push(ip_str.to_string());
                                        }
                                    }
                                }
                            }

                            // 2. Check for standard 'findings' list
                            if let Some(findings) = data.get("findings").and_then(|v| v.as_array())
                            {
                                for finding in findings {
                                    // Extract potential asset from finding
                                    // Strategy: check 'url', then 'evidence' if it looks like a domain/url
                                    if let Some(url) = finding.get("url").and_then(|s| s.as_str()) {
                                        if !url.is_empty() {
                                            discovered_assets.push(url.to_string());
                                        }
                                    } else if let Some(evidence) =
                                        finding.get("evidence").and_then(|s| s.as_str())
                                    {
                                        // Simple check if evidence looks like a domain/url
                                        if !evidence.contains('\n')
                                            && (evidence.contains('.') || evidence.contains("http"))
                                        {
                                            discovered_assets.push(evidence.to_string());
                                        }
                                    }
                                }
                            }

                            for sub_str in discovered_assets {
                                if sub_str.is_empty() {
                                    continue;
                                }

                                // Clean/Normalize domain (remove protocol)
                                let clean_domain = sub_str
                                    .trim_start_matches("http://")
                                    .trim_start_matches("https://")
                                    .trim_matches('/');

                                // Check existence
                                let exists = db
                                    .get_bounty_asset_by_canonical_url(
                                        &task.program_id,
                                        clean_domain,
                                    )
                                    .await
                                    .map(|opt| opt.is_some())
                                    .unwrap_or(false);

                                if !exists {
                                    // Create new asset
                                    let now = Utc::now().to_rfc3339();
                                    let asset_id = Uuid::new_v4().to_string();
                                    let asset = BountyAssetRow {
                                        id: asset_id.clone(),
                                        program_id: task.program_id.clone(),
                                        scope_id: None,
                                        asset_type: "domain".to_string(),
                                        canonical_url: clean_domain.to_string(),
                                        original_urls_json: None,
                                        hostname: Some(clean_domain.to_string()),
                                        port: None,
                                        path: None,
                                        protocol: None,
                                        is_alive: true,
                                        last_checked_at: None,
                                        created_at: now.clone(),
                                        updated_at: now.clone(),
                                        first_seen_at: now.clone(),
                                        last_seen_at: now.clone(),
                                        // ... default other fields
                                        priority_score: Some(0.0),
                                        risk_score: Some(0.0),
                                        findings_count: 0,
                                        change_events_count: 0,
                                        ip_addresses_json: None,
                                        dns_records_json: None,
                                        tech_stack_json: None,
                                        fingerprint: None,
                                        tags_json: None,
                                        labels_json: Some("[\"monitor-task\"]".to_string()),
                                        metadata_json: None,
                                        ip_version: None,
                                        asn: None,
                                        asn_org: None,
                                        isp: None,
                                        country: None,
                                        city: None,
                                        latitude: None,
                                        longitude: None,
                                        is_cloud: None,
                                        cloud_provider: None,
                                        service_name: None,
                                        service_version: None,
                                        service_product: None,
                                        banner: None,
                                        transport_protocol: None,
                                        cpe: None,
                                        domain_registrar: None,
                                        registration_date: None,
                                        expiration_date: None,
                                        nameservers_json: None,
                                        mx_records_json: None,
                                        txt_records_json: None,
                                        whois_data_json: None,
                                        is_wildcard: None,
                                        parent_domain: None,
                                        root_domain: None,
                                        subdomain_level: None,
                                        http_status: None,
                                        response_time_ms: None,
                                        content_length: None,
                                        content_type: None,
                                        title: None,
                                        favicon_hash: None,
                                        headers_json: None,
                                        waf_detected: None,
                                        cdn_detected: None,
                                        screenshot_path: None,
                                        body_hash: None,
                                        certificate_id: None,
                                        ssl_enabled: None,
                                        certificate_subject: None,
                                        certificate_issuer: None,
                                        certificate_valid_from: None,
                                        certificate_valid_to: None,
                                        certificate_san_json: None,
                                        exposure_level: None,
                                        attack_surface_score: None,
                                        vulnerability_count: None,
                                        cvss_max_score: None,
                                        exploit_available: None,
                                        asset_category: None,
                                        asset_owner: None,
                                        business_unit: None,
                                        criticality: None,
                                        discovery_method: Some("monitor".to_string()),
                                        data_sources_json: None,
                                        confidence_score: None,
                                        monitoring_enabled: Some(true),
                                        scan_frequency: None,
                                        last_scan_type: None,
                                        parent_asset_id: None,
                                        related_assets_json: None,
                                    };

                                        if let Err(e) = db.create_bounty_asset(&asset).await {
                                            tracing::error!("Failed to save asset: {}", e);
                                        } else {
                                            total_imported += 1;
                                            tracing::info!(
                                                "Monitor imported new asset: {}",
                                                clean_domain
                                        );
                                    }
                                }
                            }

                            // 3. Check for 'assets' list (Complex Asset Objects)
                            if let Some(assets) = data.get("assets").and_then(|v| v.as_array()) {
                                for asset_obj in assets {
                                    // Parse fields
                                    let asset_type = asset_obj
                                        .get("type")
                                        .and_then(|s| s.as_str())
                                        .unwrap_or("unknown");
                                    let canonical_url = asset_obj
                                        .get("value")
                                        .and_then(|s| s.as_str())
                                        .unwrap_or("");

                                    if canonical_url.is_empty() {
                                        continue;
                                    }

                                    // Check existence
                                    let exists = db
                                        .get_bounty_asset_by_canonical_url(
                                            &task.program_id,
                                            canonical_url,
                                        )
                                        .await
                                        .map(|opt| opt.is_some())
                                        .unwrap_or(false);

                                    if !exists {
                                        let now = Utc::now().to_rfc3339();
                                        let asset_id = Uuid::new_v4().to_string();

                                        let attrs = asset_obj.get("attributes");
                                        let tags = asset_obj.get("tags").map(|v| v.to_string());
                                        let metadata =
                                            asset_obj.get("metadata").map(|v| v.to_string());

                                        let hostname = asset_obj
                                            .get("hostname")
                                            .and_then(|s| s.as_str())
                                            .map(|s| s.to_string())
                                            .or_else(|| {
                                                if asset_type == "domain" {
                                                    Some(canonical_url.to_string())
                                                } else {
                                                    None
                                                }
                                            });
                                        let port = asset_obj
                                            .get("port")
                                            .and_then(|v| v.as_i64())
                                            .map(|i| i as i32);

                                        let asset = BountyAssetRow {
                                            id: asset_id.clone(),
                                            program_id: task.program_id.clone(),
                                            scope_id: None,
                                            asset_type: asset_type.to_string(),
                                            canonical_url: canonical_url.to_string(),
                                            original_urls_json: None,
                                            hostname,
                                            port,
                                            path: None,
                                            protocol: None,
                                            is_alive: asset_obj
                                                .get("is_alive")
                                                .and_then(|v| v.as_bool())
                                                .unwrap_or(true),
                                            last_checked_at: None,
                                            created_at: now.clone(),
                                            updated_at: now.clone(),
                                            first_seen_at: now.clone(),
                                            last_seen_at: now.clone(),
                                            priority_score: Some(0.0),
                                            risk_score: Some(0.0),
                                            findings_count: 0,
                                            change_events_count: 0,

                                            // Parse attributes from "attributes" object or flat fields
                                            ip_addresses_json: None,
                                            dns_records_json: None,
                                            tech_stack_json: attrs
                                                .and_then(|a| a.get("tech_stack"))
                                                .map(|v| v.to_string()),
                                            fingerprint: None,
                                            tags_json: tags,
                                            labels_json: Some("[\"monitor-task\"]".to_string()),
                                            metadata_json: metadata,

                                            // IP Attributes
                                            ip_version: attrs
                                                .and_then(|a| a.get("ip_version"))
                                                .and_then(|s| s.as_str())
                                                .map(|s| s.to_string()),
                                            asn: attrs
                                                .and_then(|a| a.get("asn"))
                                                .and_then(|v| v.as_i64())
                                                .map(|i| i as i32),
                                            asn_org: attrs
                                                .and_then(|a| a.get("asn_org"))
                                                .and_then(|s| s.as_str())
                                                .map(|s| s.to_string()),
                                            isp: attrs
                                                .and_then(|a| a.get("isp"))
                                                .and_then(|s| s.as_str())
                                                .map(|s| s.to_string()),
                                            country: attrs
                                                .and_then(|a| a.get("country"))
                                                .and_then(|s| s.as_str())
                                                .map(|s| s.to_string()),
                                            city: attrs
                                                .and_then(|a| a.get("city"))
                                                .and_then(|s| s.as_str())
                                                .map(|s| s.to_string()),
                                            latitude: attrs
                                                .and_then(|a| a.get("latitude"))
                                                .and_then(|v| v.as_f64()),
                                            longitude: attrs
                                                .and_then(|a| a.get("longitude"))
                                                .and_then(|v| v.as_f64()),
                                            is_cloud: attrs
                                                .and_then(|a| a.get("is_cloud"))
                                                .and_then(|v| v.as_bool()),
                                            cloud_provider: attrs
                                                .and_then(|a| a.get("cloud_provider"))
                                                .and_then(|s| s.as_str())
                                                .map(|s| s.to_string()),

                                            // Port/Service
                                            service_name: attrs
                                                .and_then(|a| a.get("service_name"))
                                                .and_then(|s| s.as_str())
                                                .map(|s| s.to_string()),
                                            service_version: attrs
                                                .and_then(|a| a.get("service_version"))
                                                .and_then(|s| s.as_str())
                                                .map(|s| s.to_string()),
                                            service_product: attrs
                                                .and_then(|a| a.get("service_product"))
                                                .and_then(|s| s.as_str())
                                                .map(|s| s.to_string()),
                                            banner: attrs
                                                .and_then(|a| a.get("banner"))
                                                .and_then(|s| s.as_str())
                                                .map(|s| s.to_string()),
                                            transport_protocol: attrs
                                                .and_then(|a| a.get("transport_protocol"))
                                                .and_then(|s| s.as_str())
                                                .map(|s| s.to_string()),
                                            cpe: attrs
                                                .and_then(|a| a.get("cpe"))
                                                .and_then(|s| s.as_str())
                                                .map(|s| s.to_string()),

                                            // Domain
                                            domain_registrar: attrs
                                                .and_then(|a| a.get("domain_registrar"))
                                                .and_then(|s| s.as_str())
                                                .map(|s| s.to_string()),
                                            registration_date: attrs
                                                .and_then(|a| a.get("registration_date"))
                                                .and_then(|s| s.as_str())
                                                .map(|s| s.to_string()),
                                            expiration_date: attrs
                                                .and_then(|a| a.get("expiration_date"))
                                                .and_then(|s| s.as_str())
                                                .map(|s| s.to_string()),
                                            nameservers_json: attrs
                                                .and_then(|a| a.get("nameservers"))
                                                .map(|v| v.to_string()),
                                            mx_records_json: None,
                                            txt_records_json: None,
                                            whois_data_json: None,
                                            is_wildcard: attrs
                                                .and_then(|a| a.get("is_wildcard"))
                                                .and_then(|v| v.as_bool()),
                                            parent_domain: attrs
                                                .and_then(|a| a.get("parent_domain"))
                                                .and_then(|s| s.as_str())
                                                .map(|s| s.to_string()),
                                            root_domain: attrs
                                                .and_then(|a| a.get("root_domain"))
                                                .and_then(|s| s.as_str())
                                                .map(|s| s.to_string()),
                                            subdomain_level: attrs
                                                .and_then(|a| a.get("subdomain_level"))
                                                .and_then(|v| v.as_i64())
                                                .map(|value| value as i32),

                                            // Web
                                            http_status: attrs
                                                .and_then(|a| a.get("http_status"))
                                                .and_then(|v| v.as_i64())
                                                .map(|i| i as i32),
                                            response_time_ms: attrs
                                                .and_then(|a| a.get("response_time_ms"))
                                                .and_then(|v| v.as_i64())
                                                .map(|i| i as i32),
                                            content_length: attrs
                                                .and_then(|a| a.get("content_length"))
                                                .and_then(|v| v.as_i64()),
                                            content_type: attrs
                                                .and_then(|a| a.get("content_type"))
                                                .and_then(|s| s.as_str())
                                                .map(|s| s.to_string()),
                                            title: attrs
                                                .and_then(|a| a.get("title"))
                                                .and_then(|s| s.as_str())
                                                .map(|s| s.to_string()),
                                            favicon_hash: attrs
                                                .and_then(|a| a.get("favicon_hash"))
                                                .and_then(|s| s.as_str())
                                                .map(|s| s.to_string()),
                                            headers_json: attrs
                                                .and_then(|a| a.get("headers"))
                                                .map(|v| v.to_string()),
                                            waf_detected: attrs
                                                .and_then(|a| a.get("waf_detected"))
                                                .and_then(|s| s.as_str())
                                                .map(|s| s.to_string()),
                                            cdn_detected: attrs
                                                .and_then(|a| a.get("cdn_detected"))
                                                .and_then(|s| s.as_str())
                                                .map(|s| s.to_string()),
                                            screenshot_path: None,
                                            body_hash: None,

                                            // Certificate
                                            certificate_id: None,
                                            ssl_enabled: attrs
                                                .and_then(|a| a.get("ssl_enabled"))
                                                .and_then(|v| v.as_bool()),
                                            certificate_subject: attrs
                                                .and_then(|a| a.get("certificate_subject"))
                                                .and_then(|s| s.as_str())
                                                .map(|s| s.to_string()),
                                            certificate_issuer: attrs
                                                .and_then(|a| a.get("certificate_issuer"))
                                                .and_then(|s| s.as_str())
                                                .map(|s| s.to_string()),
                                            certificate_valid_from: None,
                                            certificate_valid_to: attrs
                                                .and_then(|a| a.get("certificate_valid_to"))
                                                .and_then(|s| s.as_str())
                                                .map(|s| s.to_string()),
                                            certificate_san_json: None,

                                            exposure_level: None,
                                            attack_surface_score: None,
                                            vulnerability_count: None,
                                            cvss_max_score: None,
                                            exploit_available: None,
                                            asset_category: None,
                                            asset_owner: None,
                                            business_unit: None,
                                            criticality: None,
                                            discovery_method: Some("monitor-plugin".to_string()),
                                            data_sources_json: None,
                                            confidence_score: Some(1.0),
                                            monitoring_enabled: Some(true),
                                            scan_frequency: None,
                                            last_scan_type: None,
                                            parent_asset_id: None,
                                            related_assets_json: None,
                                        };

                                        if let Err(e) = db.create_bounty_asset(&asset).await {
                                            tracing::error!("Failed to save asset: {}", e);
                                        } else {
                                            total_imported += 1;
                                            tracing::info!(
                                                "Monitor imported new detailed asset: {}",
                                                canonical_url
                                            );
                                        }
                                    }
                                }
                            }
                        }

                        tracing::info!(
                            "Plugin {} completed for task {}: status=success, duration_ms={}",
                            effective_plugin_label,
                            task.name,
                            plugin_started_at.elapsed().as_millis()
                        );
                        completed_steps = index + 1;
                        emit_monitor_task_progress(
                            &app_handle,
                            &build_monitor_task_progress_event(
                                &task,
                                "scheduler",
                                "running",
                                completed_steps,
                                total_steps,
                                Some(effective_plugin_label.as_str()),
                                Some(index + 1),
                                plugin_targets.len(),
                                total_imported,
                                Some(format!("Plugin {} completed", effective_plugin_label)),
                                &execution_started_at,
                            ),
                        );
                        emit_monitor_task_log(
                            &app_handle,
                            &build_monitor_task_log_event(
                                &task,
                                effective_plugin_label.as_str(),
                                "scheduler",
                                "completed",
                                index + 1,
                                total_steps,
                                Some(plugin_started_at.elapsed().as_millis() as u64),
                                total_imported.saturating_sub(imported_before_plugin),
                                total_imported,
                                format!("Plugin {} completed", effective_plugin_label),
                            ),
                        );
                    }

                    if !stopped {
                        emit_monitor_task_progress(
                            &app_handle,
                            &build_monitor_task_progress_event(
                                &task,
                                "scheduler",
                                "completed",
                                completed_steps.max(total_steps),
                                total_steps,
                                None,
                                None,
                                0,
                                total_imported,
                                Some("Monitor task completed".to_string()),
                                &execution_started_at,
                            ),
                        );
                        emit_monitor_task_summary(
                            &app_handle,
                            &build_monitor_task_summary_event(
                                &task.id,
                                &task.name,
                                &task.program_id,
                                "scheduler",
                                "completed",
                                total_imported,
                                total_findings_created,
                                total_findings_updated,
                                &execution_started_at,
                                Some("Monitor task completed".to_string()),
                            ),
                        );
                    } else {
                        emit_monitor_task_summary(
                            &app_handle,
                            &build_monitor_task_summary_event(
                                &task.id,
                                &task.name,
                                &task.program_id,
                                "scheduler",
                                "stopped",
                                total_imported,
                                total_findings_created,
                                total_findings_updated,
                                &execution_started_at,
                                Some("Monitor task stopped".to_string()),
                            ),
                        );
                    }

                    Ok(all_events)
                }
                .await;

                if let Err(error) = &result {
                    emit_monitor_task_summary(
                        &app_handle,
                        &build_monitor_task_summary_event(
                            &task_id,
                            &task.name,
                            &task.program_id,
                            "scheduler",
                            "failed",
                            0,
                            0,
                            0,
                            &execution_started_at,
                            Some(error.clone()),
                        ),
                    );
                }

                {
                    let mut running = running_task_ids.write().await;
                    running.remove(&task_id);
                }
                {
                    let mut cancel = cancel_requested_task_ids.write().await;
                    cancel.remove(&task_id);
                }
                {
                    let mut active_runs = active_task_run_ids.write().await;
                    active_runs.remove(&task_id);
                }

                result
            })
        })
        .await;

    if scheduler.is_running().await {
        return Ok(true);
    }

    scheduler.start().await?;

    // Emit scheduler started event
    let _ = app.emit("monitor:scheduler-started", ());

    Ok(true)
}

/// Stop the monitoring scheduler
#[tauri::command]
pub async fn monitor_stop_scheduler(
    state: State<'_, Arc<RwLock<MonitorSchedulerState>>>,
    app: AppHandle,
) -> Result<bool, String> {
    let state_guard = state.read().await;
    state_guard.scheduler.stop().await?;
    let active_runs = state_guard
        .active_task_run_ids
        .read()
        .await
        .values()
        .cloned()
        .collect::<Vec<_>>();
    for run_id in active_runs {
        let cancelled = sentinel_plugins::cancel_plugin_fetch_requests_by_run(
            &run_id,
            "monitor scheduler stopped by user",
        );
        tracing::info!(
            "Cancelled {} queued/running plugin fetch requests for monitor run {}",
            cancelled,
            run_id
        );
    }

    // Emit scheduler stopped event
    let _ = app.emit("monitor:scheduler-stopped", ());

    Ok(true)
}

/// Check if scheduler is running
#[tauri::command]
pub async fn monitor_is_running(
    state: State<'_, Arc<RwLock<MonitorSchedulerState>>>,
) -> Result<bool, String> {
    let state_guard = state.read().await;
    Ok(state_guard.scheduler.is_running().await)
}

/// Get scheduler statistics
#[tauri::command]
pub async fn monitor_get_stats(
    state: State<'_, Arc<RwLock<MonitorSchedulerState>>>,
) -> Result<MonitorStats, String> {
    let state_guard = state.read().await;
    Ok(state_guard.scheduler.get_stats().await)
}
