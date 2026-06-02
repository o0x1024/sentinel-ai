use crate::commands::monitor_progress_support::{
    build_monitor_task_log_event, build_monitor_task_progress_event, emit_monitor_task_log,
    emit_monitor_task_progress,
};
use sentinel_bounty::services::MonitorTask;
use std::time::{Duration, Instant};
use tauri::AppHandle;
use tokio::sync::oneshot;

pub(crate) struct MonitorExecutionHeartbeat {
    stop_tx: Option<oneshot::Sender<()>>,
    join_handle: tokio::task::JoinHandle<()>,
}

impl MonitorExecutionHeartbeat {
    pub(crate) async fn stop(mut self) {
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(());
        }
        let _ = self.join_handle.await;
    }
}

pub(crate) fn start_monitor_execution_heartbeat(
    app: AppHandle,
    task: MonitorTask,
    execution_mode: &'static str,
    completed_steps: usize,
    total_steps: usize,
    plugin_id: String,
    plugin_index: usize,
    target_count: usize,
    imported_assets: usize,
    execution_started_at: String,
    plugin_started_at: Instant,
) -> MonitorExecutionHeartbeat {
    let (stop_tx, mut stop_rx) = oneshot::channel();
    let join_handle = tokio::spawn(async move {
        let mut heartbeat_count: u32 = 0;
        loop {
            tokio::select! {
                _ = &mut stop_rx => break,
                _ = tokio::time::sleep(Duration::from_secs(5)) => {
                    heartbeat_count += 1;
                    let elapsed = plugin_started_at.elapsed();
                    let elapsed_secs = elapsed.as_secs();
                    let mut payload = build_monitor_task_progress_event(
                        &task,
                        execution_mode,
                        "running",
                        completed_steps,
                        total_steps,
                        Some(plugin_id.as_str()),
                        Some(plugin_index),
                        target_count,
                        imported_assets,
                        Some(build_heartbeat_message(&plugin_id, target_count, elapsed_secs)),
                        &execution_started_at,
                    );
                    if let Some(latest) = sentinel_plugins::get_latest_plugin_progress(&task.id) {
                        payload.plugin_completed_units = latest.plugin_completed_units;
                        payload.plugin_total_units = latest.plugin_total_units;
                        payload.plugin_phase = latest.plugin_phase;
                        payload.plugin_phase_label = latest.plugin_phase_label;
                        payload.current_target = latest.current_target;

                        let fraction = match (payload.plugin_completed_units, payload.plugin_total_units) {
                            (Some(current), Some(total)) if total > 0 => {
                                current.min(total) as f64 / total as f64
                            }
                            _ => 0.0,
                        };
                        payload.indeterminate = false;
                        payload.progress = if total_steps == 0 {
                            (fraction * 100.0).round().min(99.0) as u32
                        } else {
                            (((completed_steps as f64 + fraction) / total_steps as f64) * 100.0)
                                .round()
                                .min(99.0) as u32
                        };
                    } else {
                        payload.indeterminate = true;
                        payload.progress = if total_steps == 0 {
                            0
                        } else {
                            ((completed_steps as f64 / total_steps as f64) * 100.0).round() as u32
                        };
                    }
                    emit_monitor_task_progress(&app, &payload);

                    if heartbeat_count % 6 == 0 {
                        emit_monitor_task_log(
                            &app,
                            &build_monitor_task_log_event(
                                &task,
                                plugin_id.as_str(),
                                execution_mode,
                                "running",
                                plugin_index,
                                total_steps,
                                Some(elapsed.as_millis() as u64),
                                0,
                                imported_assets,
                                build_heartbeat_message(&plugin_id, target_count, elapsed_secs),
                            ),
                        );
                    }
                }
            }
        }
    });

    MonitorExecutionHeartbeat {
        stop_tx: Some(stop_tx),
        join_handle,
    }
}

fn build_heartbeat_message(plugin_id: &str, target_count: usize, elapsed_secs: u64) -> String {
    let elapsed_label = if elapsed_secs >= 60 {
        format!("{}m {}s", elapsed_secs / 60, elapsed_secs % 60)
    } else {
        format!("{}s", elapsed_secs)
    };

    if plugin_id.contains("sensitive_file_scanner") {
        format!(
            "Sensitive file scan is still running. Targets: {}, elapsed: {}",
            target_count, elapsed_label
        )
    } else {
        format!(
            "Plugin {} is still running. Elapsed: {}",
            plugin_id, elapsed_label
        )
    }
}
