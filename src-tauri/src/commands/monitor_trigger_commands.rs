use crate::commands::monitor_commands::{
    inject_monitor_execution_context, inject_monitor_plugin_targets,
    monitor_plugin_has_invocable_input, monitor_plugin_runtime_label, MonitorSchedulerState,
};
use crate::commands::monitor_config_support::save_tasks_to_db;
use crate::commands::monitor_execution_heartbeat_support::start_monitor_execution_heartbeat;
use crate::commands::monitor_finding_support::import_monitor_findings_from_output;
use crate::commands::monitor_notification_support::{
    build_monitor_plugin_failure_event, build_monitor_task_summary_event,
    emit_monitor_plugin_failure, emit_monitor_task_summary, normalize_monitor_error_message,
};
use crate::commands::monitor_plugin_execution_support::execute_monitor_plugin;
use crate::commands::monitor_plugin_output_support::extract_plugin_failure;
use crate::commands::monitor_progress_support::{
    build_monitor_task_log_event, build_monitor_task_progress_event, collect_monitor_plugins,
    emit_monitor_task_log, emit_monitor_task_progress, with_monitor_target_breakdown,
};
use crate::commands::monitor_snapshot_support::{
    persist_api_monitor_inventory_output, persist_monitor_plugin_snapshots_to_task,
};
use crate::commands::monitor_surface_support::{
    collect_monitor_target_payload_for_plugin, format_monitor_target_breakdown,
    ingest_surface_plugin_output,
};
use crate::services::ensure_bug_bounty_access;
use chrono::Utc;
use sentinel_bounty::services::MonitorPluginConfig;
use sentinel_db::{BountyAssetRow, BountyChangeEventRow, DatabaseService};
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, State};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Trigger a monitoring task immediately (executes now, not waiting for scheduler)
#[tauri::command]
pub async fn monitor_trigger_task(
    state: State<'_, Arc<RwLock<MonitorSchedulerState>>>,
    db_service: State<'_, Arc<DatabaseService>>,
    plugin_manager: State<'_, Arc<sentinel_traffic::PluginManager>>,
    app: AppHandle,
    task_id: String,
) -> Result<bool, String> {
    ensure_bug_bounty_access()?;

    tracing::info!(
        "Manually triggering task for immediate execution: task_id={}",
        task_id
    );

    let state_guard = state.read().await;

    if state_guard.running_task_ids.read().await.contains(&task_id) {
        return Err(format!("Task is already running: {}", task_id));
    }

    // Get task to execute
    let task = match state_guard.scheduler.get_task(&task_id).await {
        Some(t) => {
            tracing::info!(
                "Found task to execute: name='{}', enabled={}, program_id={}",
                t.name,
                t.enabled,
                t.program_id
            );
            t
        }
        None => {
            tracing::warn!("Task not found: {}", task_id);
            return Err(format!("Task not found: {}", task_id));
        }
    };

    // Execute the task immediately using the same logic as the scheduler
    tracing::info!("Executing task '{}' immediately...", task.name);

    let plugins_to_run = collect_monitor_plugins(&task);

    tracing::info!(
        "Task '{}' will execute {} plugins",
        task.name,
        plugins_to_run.len()
    );

    // 3. Execute Plugins (spawn async to not block the UI)
    let db_clone = db_service.inner().clone();
    let plugin_manager_clone = plugin_manager.inner().clone();
    let app_clone = app.clone();
    let task_clone = task.clone();
    let task_name = task.name.clone();
    let task_id_clone = task_id.clone();
    let scheduler = state_guard.scheduler.clone();
    let running_task_ids = state_guard.running_task_ids.clone();
    let cancel_requested_task_ids = state_guard.cancel_requested_task_ids.clone();
    let active_task_run_ids = state_guard.active_task_run_ids.clone();

    tokio::spawn(async move {
        let execution_started_at = Utc::now().to_rfc3339();
        {
            let mut running = running_task_ids.write().await;
            running.insert(task_id_clone.clone());
        }
        {
            let mut cancel = cancel_requested_task_ids.write().await;
            cancel.remove(&task_id_clone);
        }
        {
            let run_id = format!("monitor:{task_id_clone}:{execution_started_at}");
            let mut active_runs = active_task_run_ids.write().await;
            active_runs.insert(task_id_clone.clone(), run_id);
        }

        tracing::info!(
            "Background execution started for task '{}'",
            task_clone.name
        );
        let total_steps = plugins_to_run.len();
        let mut total_imported = 0;
        let mut total_findings_created = 0usize;
        let mut total_findings_updated = 0usize;
        let mut completed_steps = 0usize;
        let mut stopped = false;

        emit_monitor_task_progress(
            &app_clone,
            &build_monitor_task_progress_event(
                &task_clone,
                "manual_trigger",
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
                &app_clone,
                &build_monitor_task_progress_event(
                    &task_clone,
                    "manual_trigger",
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
        }

        for (index, plugin) in plugins_to_run.iter().enumerate() {
            let imported_before_plugin = total_imported;
            let configured_plugin_label = monitor_plugin_runtime_label(plugin, None);
            let mut effective_plugin_label = configured_plugin_label.clone();
            if cancel_requested_task_ids
                .read()
                .await
                .contains(&task_id_clone)
            {
                tracing::info!(
                    "Manual execution of task '{}' stopped by user request before plugin '{}'",
                    task_clone.name,
                    configured_plugin_label
                );
                stopped = true;
                emit_monitor_task_progress(
                    &app_clone,
                    &build_monitor_task_progress_event(
                        &task_clone,
                        "manual_trigger",
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
                    &app_clone,
                    &build_monitor_task_log_event(
                        &task_clone,
                        configured_plugin_label.as_str(),
                        "manual_trigger",
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
                task_clone.name
            );
            emit_monitor_task_progress(
                &app_clone,
                &build_monitor_task_progress_event(
                    &task_clone,
                    "manual_trigger",
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
                &app_clone,
                &build_monitor_task_log_event(
                    &task_clone,
                    configured_plugin_label.as_str(),
                    "manual_trigger",
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
                    &db_clone,
                    &task_clone,
                    &candidate,
                )
                .await
                {
                    Ok(value) => value,
                    Err(error) => {
                        last_failure_reason = error.clone();
                        tracing::error!(
                            "Failed to resolve targets for plugin {} in task {}: {}",
                            candidate.plugin_id,
                            task_clone.name,
                            error
                        );
                        emit_monitor_task_log(
                            &app_clone,
                            &build_monitor_task_log_event(
                                &task_clone,
                                attempt_label.as_str(),
                                "manual_trigger",
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
                let target_breakdown_label = format_monitor_target_breakdown(&resolved_targets);

                if cancel_requested_task_ids
                    .read()
                    .await
                    .contains(&task_id_clone)
                {
                    tracing::info!(
                        "Manual execution of task '{}' stopped by user request before plugin '{}'",
                        task_clone.name,
                        attempt_label
                    );
                    stopped = true;
                    emit_monitor_task_progress(
                        &app_clone,
                        &build_monitor_task_progress_event(
                            &task_clone,
                            "manual_trigger",
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
                        &app_clone,
                        &build_monitor_task_log_event(
                            &task_clone,
                            attempt_label.as_str(),
                            "manual_trigger",
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

                if !monitor_plugin_has_invocable_input(&resolved_targets) {
                    last_failure_reason = format!(
                        "Plugin {} skipped because no target assets or discovery seed inputs were configured",
                        attempt_label
                    );
                    emit_monitor_task_log(
                        &app_clone,
                        &build_monitor_task_log_event(
                            &task_clone,
                            attempt_label.as_str(),
                            "manual_trigger",
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

                emit_monitor_task_progress(
                    &app_clone,
                    &with_monitor_target_breakdown(
                        build_monitor_task_progress_event(
                            &task_clone,
                            "manual_trigger",
                            "running",
                            completed_steps,
                            total_steps,
                            Some(attempt_label.as_str()),
                            Some(index + 1),
                            plugin_targets.len(),
                            total_imported,
                            Some(format!("Running plugin {}", attempt_label)),
                            &execution_started_at,
                        ),
                        target_breakdown_label.clone(),
                    ),
                );

                let plugin_started_at = Instant::now();
                let mut input = candidate.plugin_params.clone();
                if !input.is_object() {
                    input = serde_json::json!({});
                }
                inject_monitor_plugin_targets(&mut input, &resolved_targets);
                inject_monitor_execution_context(
                    &mut input,
                    &task_clone,
                    "manual_trigger",
                    &execution_started_at,
                    attempt_label.as_str(),
                    index + 1,
                    completed_steps,
                    total_steps,
                    total_imported,
                );

                let heartbeat = start_monitor_execution_heartbeat(
                    app_clone.clone(),
                    task_clone.clone(),
                    "manual_trigger",
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
                    &db_clone,
                    &plugin_manager_clone,
                    &candidate.plugin_id,
                    input,
                )
                .await;
                heartbeat.stop().await;

                if cancel_requested_task_ids
                    .read()
                    .await
                    .contains(&task_id_clone)
                {
                    tracing::info!(
                        "Manual execution of task '{}' stopped by user request during plugin '{}'",
                        task_clone.name,
                        attempt_label
                    );
                    stopped = true;
                    emit_monitor_task_progress(
                        &app_clone,
                        &build_monitor_task_progress_event(
                            &task_clone,
                            "manual_trigger",
                            "stopped",
                            completed_steps,
                            total_steps,
                            Some(attempt_label.as_str()),
                            Some(index + 1),
                            plugin_targets.len(),
                            total_imported,
                            Some(format!("Stopped during plugin {}", attempt_label)),
                            &execution_started_at,
                        ),
                    );
                    emit_monitor_task_log(
                        &app_clone,
                        &build_monitor_task_log_event(
                            &task_clone,
                            attempt_label.as_str(),
                            "manual_trigger",
                            "stopped",
                            index + 1,
                            total_steps,
                            Some(plugin_started_at.elapsed().as_millis() as u64),
                            0,
                            total_imported,
                            format!("Stopped during plugin {}", attempt_label),
                        ),
                    );
                    break;
                }

                if !result.success {
                    last_failure_reason = normalize_monitor_error_message(
                        result
                            .error
                            .as_deref()
                            .unwrap_or("monitor plugin execution failed"),
                    );
                    tracing::error!("Plugin {} failed: {:?}", attempt_label, result.error);
                    emit_monitor_task_log(
                        &app_clone,
                        &build_monitor_task_log_event(
                            &task_clone,
                            attempt_label.as_str(),
                            "manual_trigger",
                            "failed",
                            index + 1,
                            total_steps,
                            Some(plugin_started_at.elapsed().as_millis() as u64),
                            0,
                            total_imported,
                            format!("Plugin {} failed: {}", attempt_label, last_failure_reason),
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
                            task_clone.name,
                            plugin_error
                        );
                        emit_monitor_task_log(
                            &app_clone,
                            &build_monitor_task_log_event(
                                &task_clone,
                                runtime_plugin_label.as_str(),
                                "manual_trigger",
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
                    &app_clone,
                    &build_monitor_plugin_failure_event(
                        &task_clone.id,
                        &task_clone.name,
                        &task_clone.program_id,
                        "manual_trigger",
                        &last_attempt_plugin_id,
                        last_attempt_label.as_str(),
                        last_failure_reason.as_str(),
                        &execution_started_at,
                    ),
                );
                emit_monitor_task_progress(
                    &app_clone,
                    &build_monitor_task_progress_event(
                        &task_clone,
                        "manual_trigger",
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
                    &app_clone,
                    &build_monitor_task_log_event(
                        &task_clone,
                        last_attempt_label.as_str(),
                        "manual_trigger",
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

            let plugin = selected_plugin
                .as_ref()
                .expect("selected plugin must exist");
            let result = selected_result
                .as_ref()
                .expect("selected result must exist");
            let resolved_targets = selected_resolved_targets
                .as_ref()
                .expect("selected targets must exist");
            let plugin_targets = &resolved_targets.targets;
            let plugin_started_at =
                selected_plugin_started_at.expect("selected plugin start time must exist");

            let mut plugin_subdomains_count: usize = 0;
            let mut plugin_urls_count: usize = 0;
            let mut plugin_ips_count: usize = 0;
            let mut plugin_assets_count: usize = 0;
            let mut plugin_results_count: usize = 0;
            let mut plugin_findings_count: usize = 0;
            let mut plugin_targets_scanned_count: usize = 0;
            let mut plugin_rules_loaded_count: usize = 0;
            let mut plugin_existing_count: usize = 0;
            let mut plugin_inserted_count: usize = 0;
            let mut inserted_from_subdomains: usize = 0;
            let mut inserted_from_urls: usize = 0;
            let mut inserted_from_ips: usize = 0;
            let mut inserted_from_assets: usize = 0;
            let mut inserted_from_results: usize = 0;
            let mut plugin_summary_details: Option<String> = None;

            // Process output (simplified asset import logic)
            if let Some(output) = &result.output {
                let runtime_plugin_label = monitor_plugin_runtime_label(plugin, Some(output));
                effective_plugin_label = runtime_plugin_label.clone();
                if let Some(plugin_error) = extract_plugin_failure(output) {
                    let failure_reason = normalize_monitor_error_message(plugin_error.as_str());
                    tracing::error!(
                        "Plugin {} returned failure output for task {}: {}",
                        runtime_plugin_label,
                        task_clone.name,
                        plugin_error
                    );
                    completed_steps = index + 1;
                    emit_monitor_plugin_failure(
                        &app_clone,
                        &build_monitor_plugin_failure_event(
                            &task_clone.id,
                            &task_clone.name,
                            &task_clone.program_id,
                            "manual_trigger",
                            &plugin.plugin_id,
                            runtime_plugin_label.as_str(),
                            failure_reason.as_str(),
                            &execution_started_at,
                        ),
                    );
                    emit_monitor_task_progress(
                        &app_clone,
                        &build_monitor_task_progress_event(
                            &task_clone,
                            "manual_trigger",
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
                        &app_clone,
                        &build_monitor_task_log_event(
                            &task_clone,
                            runtime_plugin_label.as_str(),
                            "manual_trigger",
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
                    &db_clone,
                    &task_clone,
                    plugin,
                    output,
                    "manual_trigger",
                    Some(&task_clone.id),
                )
                .await
                {
                    tracing::warn!(
                        "Failed to persist API monitor inventory for task {}: {}",
                        task_clone.id,
                        error
                    );
                }
                let mut task_state_updated = false;
                if let Err(error) = scheduler
                    .update_task(&task_clone.id, |scheduled_task| {
                        if persist_monitor_plugin_snapshots_to_task(scheduled_task, plugin, output)
                        {
                            task_state_updated = true;
                        }
                    })
                    .await
                {
                    tracing::warn!(
                        "Failed to update persisted monitor task state for {}: {}",
                        task_clone.id,
                        error
                    );
                } else if task_state_updated {
                    if let Err(error) = save_tasks_to_db(&scheduler, &db_clone).await {
                        tracing::warn!(
                            "Failed to save monitor tasks after plugin {}: {}",
                            runtime_plugin_label,
                            error
                        );
                    }
                }
                match ingest_surface_plugin_output(
                    &db_clone,
                    &task_clone.program_id,
                    &plugin.plugin_id,
                    "monitor_trigger_task",
                    Some(&task_clone.id),
                    output,
                    Some(serde_json::json!({
                        "task_id": task_clone.id,
                        "task_name": task_clone.name,
                        "execution_mode": "manual_trigger",
                    })),
                )
                .await
                {
                    Ok(Some(stats)) => {
                        total_imported += stats.created_assets + stats.enriched_assets;
                        tracing::info!(
                            "Manual task '{}' materialized {} surface assets and enriched {} existing assets for plugin '{}'",
                            task_clone.name,
                            stats.created_assets,
                            stats.enriched_assets,
                            runtime_plugin_label
                        );
                        completed_steps = index + 1;
                        emit_monitor_task_progress(
                            &app_clone,
                            &build_monitor_task_progress_event(
                                &task_clone,
                                "manual_trigger",
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
                            &app_clone,
                            &build_monitor_task_log_event(
                                &task_clone,
                                runtime_plugin_label.as_str(),
                                "manual_trigger",
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
                        "Failed to ingest surface output for manual task '{}' plugin '{}': {}",
                        task_clone.name,
                        runtime_plugin_label,
                        e
                    ),
                }

                let data = output.get("data").unwrap_or(output);

                plugin_subdomains_count = data
                    .get("subdomains")
                    .and_then(|v| v.as_array())
                    .map(|v| v.len())
                    .unwrap_or(0);
                plugin_urls_count = data
                    .get("urls")
                    .and_then(|v| v.as_array())
                    .map(|v| v.len())
                    .unwrap_or(0);
                plugin_ips_count = data
                    .get("ips")
                    .and_then(|v| v.as_array())
                    .map(|v| v.len())
                    .unwrap_or(0);
                plugin_assets_count = data
                    .get("assets")
                    .and_then(|v| v.as_array())
                    .map(|v| v.len())
                    .unwrap_or(0);
                plugin_results_count = data
                    .get("results")
                    .and_then(|v| v.as_array())
                    .map(|v| v.len())
                    .unwrap_or(0);
                plugin_findings_count = data
                    .get("findings_count")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize)
                    .or_else(|| {
                        data.get("findings")
                            .and_then(|v| v.as_array())
                            .map(|v| v.len())
                    })
                    .unwrap_or(0);
                plugin_targets_scanned_count = data
                    .get("summary")
                    .and_then(|v| v.get("scannedTargets"))
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize)
                    .or_else(|| {
                        data.get("summary")
                            .and_then(|v| v.get("totalTargets"))
                            .and_then(|v| v.as_u64())
                            .map(|v| v as usize)
                    })
                    .unwrap_or(0);
                plugin_rules_loaded_count = data
                    .get("summary")
                    .and_then(|v| v.get("scannedRules"))
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize)
                    .unwrap_or(0);
                plugin_summary_details = data.get("summary").and_then(|summary| {
                    if summary.is_object() {
                        serde_json::to_string(summary).ok()
                    } else {
                        None
                    }
                });

                match import_monitor_findings_from_output(
                    &db_clone,
                    &task_clone,
                    &plugin.plugin_id,
                    output,
                    "manual_trigger",
                )
                .await
                {
                    Ok(stats) if stats.created > 0 || stats.updated > 0 => {
                        total_findings_created += stats.created;
                        total_findings_updated += stats.updated;
                        tracing::info!(
                            "Manual task '{}' imported {} findings and refreshed {} existing findings for plugin '{}'",
                            task_clone.name,
                            stats.created,
                            stats.updated,
                            plugin.plugin_id
                        );
                    }
                    Ok(_) => {}
                    Err(error) => tracing::warn!(
                        "Failed to import monitor findings for manual task '{}' plugin '{}': {}",
                        task_clone.name,
                        plugin.plugin_id,
                        error
                    ),
                }

                // Import discovered subdomains
                if let Some(subdomains) = data.get("subdomains").and_then(|v| v.as_array()) {
                    for sub in subdomains {
                        let sub_str = sub
                            .as_str()
                            .or_else(|| sub.get("domain").and_then(|s| s.as_str()))
                            .unwrap_or("");
                        if sub_str.is_empty() {
                            continue;
                        }

                        let clean_domain = sub_str
                            .trim_start_matches("http://")
                            .trim_start_matches("https://")
                            .trim_matches('/');

                        // Check existence
                        let exists = db_clone
                            .get_bounty_asset_by_canonical_url(&task_clone.program_id, clean_domain)
                            .await
                            .map(|opt| opt.is_some())
                            .unwrap_or(false);

                        if exists {
                            plugin_existing_count += 1;
                        }

                        if !exists {
                            let now = Utc::now().to_rfc3339();
                            let asset = BountyAssetRow {
                                id: Uuid::new_v4().to_string(),
                                program_id: task_clone.program_id.clone(),
                                scope_id: None,
                                asset_type: "domain".to_string(),
                                canonical_url: clean_domain.to_string(),
                                hostname: Some(clean_domain.to_string()),
                                is_alive: true,
                                created_at: now.clone(),
                                updated_at: now.clone(),
                                first_seen_at: now.clone(),
                                last_seen_at: now.clone(),
                                labels_json: Some("[\"manual-trigger\"]".to_string()),
                                discovery_method: Some("monitor-manual".to_string()),
                                monitoring_enabled: Some(true),
                                // ... other fields with defaults
                                original_urls_json: None,
                                port: None,
                                path: None,
                                protocol: None,
                                priority_score: Some(0.0),
                                risk_score: Some(0.0),
                                findings_count: 0,
                                change_events_count: 0,
                                ip_addresses_json: None,
                                dns_records_json: None,
                                tech_stack_json: None,
                                fingerprint: None,
                                tags_json: None,
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
                                data_sources_json: None,
                                confidence_score: None,
                                scan_frequency: None,
                                last_scan_type: None,
                                last_checked_at: None,
                                parent_asset_id: None,
                                related_assets_json: None,
                            };

                            if let Err(e) = db_clone.create_bounty_asset(&asset).await {
                                tracing::error!("Failed to save asset: {}", e);
                            } else {
                                total_imported += 1;
                                plugin_inserted_count += 1;
                                inserted_from_subdomains += 1;

                                let change_event = BountyChangeEventRow {
                                    id: Uuid::new_v4().to_string(),
                                    program_id: Some(task_clone.program_id.clone()),
                                    asset_id: asset.id.clone(),
                                    event_type: "asset_discovered".to_string(),
                                    severity: "medium".to_string(),
                                    status: "new".to_string(),
                                    title: format!(
                                        "New asset discovered by manual monitor run: {}",
                                        clean_domain
                                    ),
                                    description: format!(
                                        "Task '{}' imported new asset '{}' via plugin '{}'.",
                                        task_clone.name, clean_domain, plugin.plugin_id
                                    ),
                                    old_value: None,
                                    new_value: Some(clean_domain.to_string()),
                                    diff: None,
                                    affected_scope: None,
                                    detection_method: format!(
                                        "manual-trigger:{}",
                                        plugin.plugin_id
                                    ),
                                    generated_findings_json: None,
                                    tags_json: Some(
                                        serde_json::to_string(&vec![
                                            "manual-trigger",
                                            "asset-discovered",
                                        ])
                                        .unwrap_or_default(),
                                    ),
                                    metadata_json: None,
                                    risk_score: 45.0,
                                    auto_trigger_enabled: task_clone.config.auto_trigger_enabled,
                                    created_at: now.clone(),
                                    updated_at: now.clone(),
                                    resolved_at: None,
                                };

                                if let Err(e) =
                                    db_clone.create_bounty_change_event(&change_event).await
                                {
                                    tracing::error!("Failed to create manual change event: {}", e);
                                } else if task_clone.config.auto_trigger_enabled {
                                    match crate::commands::bounty_workflow_event_support::bounty_trigger_workflows_for_event_internal(
                                        app_clone.clone(),
                                        db_clone.clone(),
                                        plugin_manager_clone.clone(),
                                        change_event.id.clone(),
                                    )
                                    .await {
                                        Ok(triggered_ids) => {
                                            tracing::info!(
                                                "manual-trigger workflow fired: event_id={}, triggered_count={}, triggered_bindings={:?}",
                                                change_event.id,
                                                triggered_ids.len(),
                                                triggered_ids
                                            );
                                        }
                                        Err(e) => {
                                            tracing::error!(
                                                "manual-trigger workflow failed: event_id={}, error={}",
                                                change_event.id,
                                                e
                                            );
                                        }
                                    }
                                } else {
                                    tracing::info!(
                                        "manual-trigger workflow skipped: event_id={}, auto_trigger_enabled=false",
                                        change_event.id
                                    );
                                }
                            }
                        }
                    }
                }

                if let Some(urls) = data.get("urls").and_then(|v| v.as_array()) {
                    for url_item in urls {
                        let url_str = url_item
                            .as_str()
                            .or_else(|| url_item.get("url").and_then(|v| v.as_str()))
                            .or_else(|| url_item.get("value").and_then(|v| v.as_str()))
                            .unwrap_or("")
                            .trim();

                        if url_str.is_empty() {
                            continue;
                        }

                        let canonical_url = url_str.to_string();
                        let exists = db_clone
                            .get_bounty_asset_by_canonical_url(
                                &task_clone.program_id,
                                &canonical_url,
                            )
                            .await
                            .map(|opt| opt.is_some())
                            .unwrap_or(false);

                        if exists {
                            plugin_existing_count += 1;
                            continue;
                        }

                        let hostname = canonical_url
                            .trim_start_matches("http://")
                            .trim_start_matches("https://")
                            .split('/')
                            .next()
                            .map(|s| s.to_string());

                        let now = Utc::now().to_rfc3339();
                        let asset = BountyAssetRow {
                            id: Uuid::new_v4().to_string(),
                            program_id: task_clone.program_id.clone(),
                            scope_id: None,
                            asset_type: "url".to_string(),
                            canonical_url: canonical_url.clone(),
                            hostname,
                            is_alive: true,
                            created_at: now.clone(),
                            updated_at: now.clone(),
                            first_seen_at: now.clone(),
                            last_seen_at: now.clone(),
                            labels_json: Some("[\"manual-trigger\"]".to_string()),
                            discovery_method: Some("monitor-manual".to_string()),
                            monitoring_enabled: Some(true),
                            original_urls_json: None,
                            port: None,
                            path: None,
                            protocol: None,
                            priority_score: Some(0.0),
                            risk_score: Some(0.0),
                            findings_count: 0,
                            change_events_count: 0,
                            ip_addresses_json: None,
                            dns_records_json: None,
                            tech_stack_json: None,
                            fingerprint: None,
                            tags_json: None,
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
                            data_sources_json: None,
                            confidence_score: None,
                            scan_frequency: None,
                            last_scan_type: None,
                            last_checked_at: None,
                            parent_asset_id: None,
                            related_assets_json: None,
                        };

                        if let Err(e) = db_clone.create_bounty_asset(&asset).await {
                            tracing::error!("Failed to save URL asset: {}", e);
                        } else {
                            total_imported += 1;
                            plugin_inserted_count += 1;
                            inserted_from_urls += 1;
                        }
                    }
                }

                if let Some(ips) = data.get("ips").and_then(|v| v.as_array()) {
                    for ip_item in ips {
                        let ip_str = ip_item
                            .as_str()
                            .or_else(|| ip_item.get("ip").and_then(|v| v.as_str()))
                            .or_else(|| ip_item.get("address").and_then(|v| v.as_str()))
                            .unwrap_or("")
                            .trim();

                        if ip_str.is_empty() {
                            continue;
                        }

                        let canonical_url = ip_str.to_string();
                        let exists = db_clone
                            .get_bounty_asset_by_canonical_url(
                                &task_clone.program_id,
                                &canonical_url,
                            )
                            .await
                            .map(|opt| opt.is_some())
                            .unwrap_or(false);

                        if exists {
                            plugin_existing_count += 1;
                            continue;
                        }

                        let now = Utc::now().to_rfc3339();
                        let asset = BountyAssetRow {
                            id: Uuid::new_v4().to_string(),
                            program_id: task_clone.program_id.clone(),
                            scope_id: None,
                            asset_type: "ip".to_string(),
                            canonical_url: canonical_url.clone(),
                            hostname: None,
                            is_alive: true,
                            created_at: now.clone(),
                            updated_at: now.clone(),
                            first_seen_at: now.clone(),
                            last_seen_at: now.clone(),
                            labels_json: Some("[\"manual-trigger\"]".to_string()),
                            discovery_method: Some("monitor-manual".to_string()),
                            monitoring_enabled: Some(true),
                            original_urls_json: None,
                            port: None,
                            path: None,
                            protocol: None,
                            priority_score: Some(0.0),
                            risk_score: Some(0.0),
                            findings_count: 0,
                            change_events_count: 0,
                            ip_addresses_json: None,
                            dns_records_json: None,
                            tech_stack_json: None,
                            fingerprint: None,
                            tags_json: None,
                            metadata_json: None,
                            ip_version: if canonical_url.contains(':') {
                                Some("IPv6".to_string())
                            } else {
                                Some("IPv4".to_string())
                            },
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
                            data_sources_json: None,
                            confidence_score: None,
                            scan_frequency: None,
                            last_scan_type: None,
                            last_checked_at: None,
                            parent_asset_id: None,
                            related_assets_json: None,
                        };

                        if let Err(e) = db_clone.create_bounty_asset(&asset).await {
                            tracing::error!("Failed to save IP asset: {}", e);
                        } else {
                            total_imported += 1;
                            plugin_inserted_count += 1;
                            inserted_from_ips += 1;
                        }
                    }
                }

                if let Some(assets) = data.get("assets").and_then(|v| v.as_array()) {
                    for asset_item in assets {
                        let canonical_url = asset_item
                            .get("value")
                            .or_else(|| asset_item.get("url"))
                            .or_else(|| asset_item.get("domain"))
                            .or_else(|| asset_item.get("ip"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .trim()
                            .to_string();

                        if canonical_url.is_empty() {
                            continue;
                        }

                        let exists = db_clone
                            .get_bounty_asset_by_canonical_url(
                                &task_clone.program_id,
                                &canonical_url,
                            )
                            .await
                            .map(|opt| opt.is_some())
                            .unwrap_or(false);

                        if exists {
                            plugin_existing_count += 1;
                            continue;
                        }

                        let asset_type = asset_item
                            .get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("domain")
                            .to_string();
                        let hostname = asset_item
                            .get("hostname")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                            .or_else(|| {
                                if asset_type == "domain" || asset_type == "url" {
                                    Some(
                                        canonical_url
                                            .trim_start_matches("http://")
                                            .trim_start_matches("https://")
                                            .split('/')
                                            .next()
                                            .unwrap_or("")
                                            .to_string(),
                                    )
                                } else {
                                    None
                                }
                            });

                        let now = Utc::now().to_rfc3339();
                        let asset = BountyAssetRow {
                            id: Uuid::new_v4().to_string(),
                            program_id: task_clone.program_id.clone(),
                            scope_id: None,
                            asset_type,
                            canonical_url: canonical_url.clone(),
                            hostname,
                            is_alive: true,
                            created_at: now.clone(),
                            updated_at: now.clone(),
                            first_seen_at: now.clone(),
                            last_seen_at: now.clone(),
                            labels_json: Some("[\"manual-trigger\"]".to_string()),
                            discovery_method: Some("monitor-manual".to_string()),
                            monitoring_enabled: Some(true),
                            original_urls_json: None,
                            port: None,
                            path: None,
                            protocol: None,
                            priority_score: Some(0.0),
                            risk_score: Some(0.0),
                            findings_count: 0,
                            change_events_count: 0,
                            ip_addresses_json: None,
                            dns_records_json: None,
                            tech_stack_json: None,
                            fingerprint: None,
                            tags_json: None,
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
                            data_sources_json: None,
                            confidence_score: None,
                            scan_frequency: None,
                            last_scan_type: None,
                            last_checked_at: None,
                            parent_asset_id: None,
                            related_assets_json: None,
                        };

                        if let Err(e) = db_clone.create_bounty_asset(&asset).await {
                            tracing::error!("Failed to save generic asset: {}", e);
                        } else {
                            total_imported += 1;
                            plugin_inserted_count += 1;
                            inserted_from_assets += 1;
                        }
                    }
                }

                if let Some(results) = data.get("results").and_then(|v| v.as_array()) {
                    for result_item in results {
                        let is_alive = result_item
                            .get("alive")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(true);
                        if !is_alive {
                            continue;
                        }

                        let canonical_url = result_item
                            .get("url")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .trim()
                            .to_string();

                        if canonical_url.is_empty() {
                            continue;
                        }

                        let exists = db_clone
                            .get_bounty_asset_by_canonical_url(
                                &task_clone.program_id,
                                &canonical_url,
                            )
                            .await
                            .map(|opt| opt.is_some())
                            .unwrap_or(false);

                        if exists {
                            plugin_existing_count += 1;
                            continue;
                        }

                        let hostname = canonical_url
                            .trim_start_matches("http://")
                            .trim_start_matches("https://")
                            .split('/')
                            .next()
                            .map(|s| s.to_string());

                        let protocol = if canonical_url.starts_with("https://") {
                            Some("https".to_string())
                        } else if canonical_url.starts_with("http://") {
                            Some("http".to_string())
                        } else {
                            None
                        };

                        let path = canonical_url.split_once("//").and_then(|(_, rest)| {
                            rest.split_once('/').map(|(_, p)| format!("/{}", p))
                        });

                        let metadata_json = serde_json::to_string(result_item).ok();

                        let now = Utc::now().to_rfc3339();
                        let asset = BountyAssetRow {
                            id: Uuid::new_v4().to_string(),
                            program_id: task_clone.program_id.clone(),
                            scope_id: None,
                            asset_type: "website".to_string(),
                            canonical_url: canonical_url.clone(),
                            hostname,
                            is_alive,
                            created_at: now.clone(),
                            updated_at: now.clone(),
                            first_seen_at: now.clone(),
                            last_seen_at: now.clone(),
                            labels_json: Some("[\"manual-trigger\"]".to_string()),
                            discovery_method: Some("monitor-manual".to_string()),
                            monitoring_enabled: Some(true),
                            original_urls_json: None,
                            port: None,
                            path,
                            protocol,
                            priority_score: Some(0.0),
                            risk_score: Some(0.0),
                            findings_count: 0,
                            change_events_count: 0,
                            ip_addresses_json: None,
                            dns_records_json: None,
                            tech_stack_json: None,
                            fingerprint: None,
                            tags_json: None,
                            metadata_json,
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
                            http_status: result_item
                                .get("statusCode")
                                .and_then(|v| v.as_i64())
                                .map(|v| v as i32),
                            response_time_ms: result_item
                                .get("responseTime")
                                .and_then(|v| v.as_i64())
                                .map(|v| v as i32),
                            content_length: result_item
                                .get("contentLength")
                                .and_then(|v| v.as_i64()),
                            content_type: result_item
                                .get("contentType")
                                .and_then(|v| v.as_str())
                                .map(|v| v.to_string()),
                            title: result_item
                                .get("title")
                                .and_then(|v| v.as_str())
                                .map(|v| v.to_string()),
                            favicon_hash: None,
                            headers_json: result_item
                                .get("headers")
                                .and_then(|v| serde_json::to_string(v).ok()),
                            waf_detected: None,
                            cdn_detected: None,
                            screenshot_path: None,
                            body_hash: None,
                            certificate_id: None,
                            ssl_enabled: Some(canonical_url.starts_with("https://")),
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
                            data_sources_json: None,
                            confidence_score: None,
                            scan_frequency: None,
                            last_scan_type: None,
                            last_checked_at: None,
                            parent_asset_id: None,
                            related_assets_json: None,
                        };

                        if let Err(e) = db_clone.create_bounty_asset(&asset).await {
                            tracing::error!("Failed to save website asset from results: {}", e);
                        } else {
                            total_imported += 1;
                            plugin_inserted_count += 1;
                            inserted_from_results += 1;
                        }
                    }
                }
            } else {
                tracing::info!(
                    "Plugin {} produced no output payload for task {}",
                    effective_plugin_label,
                    task_clone.name
                );
            }

            tracing::info!(
                "Plugin {} output summary for task {}: targets_scanned={}, rules_loaded={}, findings={}, subdomains={}, urls={}, ips={}, assets={}, results={}, existing_total={}, inserted_total={}, inserted_from_subdomains={}, inserted_from_urls={}, inserted_from_ips={}, inserted_from_assets={}, inserted_from_results={}",
                effective_plugin_label,
                task_clone.name,
                plugin_targets_scanned_count,
                plugin_rules_loaded_count,
                plugin_findings_count,
                plugin_subdomains_count,
                plugin_urls_count,
                plugin_ips_count,
                plugin_assets_count,
                plugin_results_count,
                plugin_existing_count,
                plugin_inserted_count,
                inserted_from_subdomains,
                inserted_from_urls,
                inserted_from_ips,
                inserted_from_assets,
                inserted_from_results
            );
            if let Some(summary) = &plugin_summary_details {
                tracing::info!(
                    "Plugin {} detailed summary for task {}: {}",
                    effective_plugin_label,
                    task_clone.name,
                    summary
                );
            }

            tracing::info!(
                "Plugin {} completed for task {}: status=success, duration_ms={}",
                effective_plugin_label,
                task_clone.name,
                plugin_started_at.elapsed().as_millis()
            );
            completed_steps = index + 1;
            emit_monitor_task_progress(
                &app_clone,
                &build_monitor_task_progress_event(
                    &task_clone,
                    "manual_trigger",
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
                &app_clone,
                &build_monitor_task_log_event(
                    &task_clone,
                    effective_plugin_label.as_str(),
                    "manual_trigger",
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
                &app_clone,
                &build_monitor_task_progress_event(
                    &task_clone,
                    "manual_trigger",
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
                &app_clone,
                &build_monitor_task_summary_event(
                    &task_clone.id,
                    &task_clone.name,
                    &task_clone.program_id,
                    "manual_trigger",
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
                &app_clone,
                &build_monitor_task_summary_event(
                    &task_clone.id,
                    &task_clone.name,
                    &task_clone.program_id,
                    "manual_trigger",
                    "stopped",
                    total_imported,
                    total_findings_created,
                    total_findings_updated,
                    &execution_started_at,
                    Some("Monitor task stopped".to_string()),
                ),
            );
        }

        // Update task statistics
        let _ = scheduler
            .update_task(&task_id_clone, |t| {
                t.last_run_at = Some(Utc::now());
                t.run_count += 1;
                t.calculate_next_run();
            })
            .await;

        tracing::info!(
            "Manual execution of task '{}' completed: {} assets imported",
            task_clone.name,
            total_imported
        );

        {
            let mut running = running_task_ids.write().await;
            running.remove(&task_id_clone);
        }
        {
            let mut cancel = cancel_requested_task_ids.write().await;
            cancel.remove(&task_id_clone);
        }
        {
            let mut active_runs = active_task_run_ids.write().await;
            active_runs.remove(&task_id_clone);
        }
    });

    tracing::info!("Task '{}' execution started in background", task_name);
    Ok(true)
}
