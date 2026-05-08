use chrono::Utc;
use sentinel_bounty::services::{
    ChangeMonitorConfig, MonitorPluginConfig, MonitorScheduler, MonitorTask,
};
use sentinel_db::{DatabaseService, MonitorTaskPersistRecord};

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

pub fn infer_monitor_type_for_plugin(
    normalized_name: &str,
    category: &str,
) -> Option<&'static str> {
    match normalized_name {
        "subdomain_enumerator" | "subdomain_brute" => Some("dns"),
        "dns_resolver" => Some("ip"),
        "cert_monitor" | "ssl_scanner" => Some("cert"),
        "content_monitor" => Some("content"),
        "api_monitor" | "js_analyzer" | "js_link_finder" => Some("api"),
        "cidr_mapper" => Some("ip"),
        "port_monitor" => Some("port"),
        "service_monitor" | "service_probe" => Some("service"),
        "http_prober" | "tech_fingerprinter" | "favicon_fingerprinter" => Some("web"),
        "sensitive_file_scanner" | "risk_scanner" => Some("risk"),
        _ => match category.to_lowercase().as_str() {
            "monitor" | "recon" | "reconnaissance" if normalized_name.contains("subdomain") => {
                Some("dns")
            }
            "monitor" | "recon" | "reconnaissance" if normalized_name.contains("resolver") => {
                Some("ip")
            }
            "monitor" if normalized_name.contains("cert") || normalized_name.contains("ssl") => {
                Some("cert")
            }
            "monitor" if normalized_name.contains("content") => Some("content"),
            "monitor" if normalized_name.contains("api") || normalized_name.contains("js") => {
                Some("api")
            }
            "monitor" | "recon"
                if normalized_name.contains("port") || normalized_name.contains("cidr") =>
            {
                Some("port")
            }
            "monitor" | "recon"
                if normalized_name.contains("service")
                    || normalized_name.contains("banner")
                    || normalized_name.contains("fingerprinter") =>
            {
                Some("service")
            }
            "monitor" | "recon"
                if normalized_name.contains("web")
                    || normalized_name.contains("tech")
                    || normalized_name.contains("http") =>
            {
                Some("web")
            }
            "monitor" | "scanner" | "scan"
                if normalized_name.contains("risk")
                    || normalized_name.contains("vuln")
                    || normalized_name.contains("sensitive") =>
            {
                Some("risk")
            }
            _ => None,
        },
    }
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
