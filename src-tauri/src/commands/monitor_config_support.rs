use crate::services::PluginMainCategory;
use chrono::Utc;
use sentinel_bounty::services::{
    ChangeMonitorConfig, MonitorPluginConfig, MonitorScheduler, MonitorTask,
};
use sentinel_db::{DatabaseService, MonitorTaskPersistRecord};
use sentinel_plugins::MonitorSeedBinding;

pub fn normalize_loaded_monitor_task(mut task: MonitorTask) -> MonitorTask {
    task.config.migrate_legacy_port_service_plugins();
    task
}

fn monitor_task_to_record(task: &MonitorTask) -> Result<MonitorTaskPersistRecord, String> {
    Ok(MonitorTaskPersistRecord {
        id: task.id.clone(),
        program_id: task.program_id.clone(),
        name: task.name.clone(),
        interval_secs: task.interval_secs as i64,
        enabled: task.enabled,
        config_json: serde_json::to_string(&task.config).map_err(|e| e.to_string())?,
        task_json: serde_json::to_string(task).map_err(|e| e.to_string())?,
        last_run_at: task.last_run_at.map(|value| value.to_rfc3339()),
        next_run_at: task.next_run_at.map(|value| value.to_rfc3339()),
        run_count: task.run_count as i64,
        events_detected: task.events_detected as i64,
        created_at: task.created_at.to_rfc3339(),
        updated_at: Utc::now().to_rfc3339(),
    })
}

pub async fn save_tasks_to_db(
    scheduler: &MonitorScheduler,
    db: &DatabaseService,
) -> Result<(), String> {
    let tasks = scheduler.list_tasks().await;
    let records = tasks
        .iter()
        .map(monitor_task_to_record)
        .collect::<Result<Vec<_>, _>>()?;

    db.replace_monitor_tasks(&records)
        .await
        .map_err(|e| e.to_string())
}

pub async fn load_tasks_from_db(
    scheduler: &MonitorScheduler,
    db: &DatabaseService,
) -> Result<(), String> {
    let task_jsons = db
        .list_monitor_task_jsons(None)
        .await
        .map_err(|e| e.to_string())?;

    if !task_jsons.is_empty() {
        for json in task_jsons {
            let task: MonitorTask = serde_json::from_str(&json).map_err(|e| e.to_string())?;
            let task = normalize_loaded_monitor_task(task);
            if scheduler.get_task(&task.id).await.is_none() {
                scheduler.add_task(task).await?;
            }
        }
        return Ok(());
    }

    let legacy_json = db
        .get_config_internal("monitor_scheduler", "tasks")
        .await
        .map_err(|e| e.to_string())?;
    let Some(json) = legacy_json.filter(|value| !value.trim().is_empty()) else {
        return Ok(());
    };

    let tasks: Vec<MonitorTask> = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    let tasks = tasks
        .into_iter()
        .map(normalize_loaded_monitor_task)
        .collect::<Vec<_>>();

    let records = tasks
        .iter()
        .map(monitor_task_to_record)
        .collect::<Result<Vec<_>, _>>()?;
    db.replace_monitor_tasks(&records)
        .await
        .map_err(|e| e.to_string())?;
    db.delete_config_internal("monitor_scheduler", "tasks")
        .await
        .map_err(|e| e.to_string())?;

    for task in tasks {
        if scheduler.get_task(&task.id).await.is_none() {
            scheduler.add_task(task).await?;
        }
    }

    Ok(())
}

pub fn normalize_monitor_type(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "dns" => Some("dns"),
        "ip" => Some("ip"),
        "cert" => Some("cert"),
        "content" => Some("content"),
        "api" => Some("api"),
        "port" => Some("port"),
        "service" => Some("service"),
        "web" => Some("web"),
        "risk" => Some("risk"),
        _ => None,
    }
}

pub fn normalize_optional_monitor_type(value: Option<String>) -> Result<Option<String>, String> {
    match value {
        Some(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }

            normalize_monitor_type(trimmed)
                .map(|normalized| Some(normalized.to_string()))
                .ok_or_else(|| format!("Unsupported monitor_type: {}", value))
        }
        None => Ok(None),
    }
}

pub fn validate_plugin_monitor_type(
    main_category: PluginMainCategory,
    monitor_type: Option<String>,
) -> Result<Option<String>, String> {
    let monitor_type = normalize_optional_monitor_type(monitor_type)?;

    if matches!(
        main_category,
        PluginMainCategory::Agent | PluginMainCategory::Bounty
    ) && monitor_type.is_none()
    {
        return Err(format!(
            "monitor_type is required for {} plugins",
            main_category
        ));
    }

    Ok(monitor_type)
}

pub fn normalize_input_mode(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "asset" => Some("asset"),
        "seed" => Some("seed"),
        "hybrid" => Some("hybrid"),
        _ => None,
    }
}

pub fn normalize_optional_input_mode(value: Option<String>) -> Result<Option<String>, String> {
    match value {
        Some(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }

            normalize_input_mode(trimmed)
                .map(|normalized| Some(normalized.to_string()))
                .ok_or_else(|| format!("Unsupported input_mode: {}", value))
        }
        None => Ok(None),
    }
}

pub fn validate_plugin_input_mode(
    main_category: PluginMainCategory,
    input_mode: Option<String>,
    seed_bindings: &[MonitorSeedBinding],
) -> Result<Option<String>, String> {
    let input_mode = normalize_optional_input_mode(input_mode)?;

    if !seed_bindings.is_empty() {
        if !matches!(
            main_category,
            PluginMainCategory::Agent | PluginMainCategory::Bounty
        ) {
            return Err("seed_bindings are only supported for agent/bounty plugins".to_string());
        }

        if matches!(input_mode.as_deref(), None | Some("asset")) {
            return Err(
                "input_mode must be seed or hybrid when seed_bindings are configured".to_string(),
            );
        }
    }

    Ok(input_mode)
}

pub fn apply_plugins_to_monitor_type(
    config: &mut ChangeMonitorConfig,
    monitor_type: &str,
    plugins: Vec<MonitorPluginConfig>,
) {
    match monitor_type {
        "dns" => config.dns_plugins = plugins,
        "ip" => config.ip_plugins = plugins,
        "cert" => config.cert_plugins = plugins,
        "content" => config.content_plugins = plugins,
        "api" => config.api_plugins = plugins,
        "port" => config.port_plugins = plugins,
        "service" => config.service_plugins = plugins,
        "web" => config.web_plugins = plugins,
        "risk" => config.risk_plugins = plugins,
        _ => {}
    }
}
