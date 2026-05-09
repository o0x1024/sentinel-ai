use crate::commands::monitor_commands::MonitorSchedulerState;
use crate::commands::monitor_config_support::{
    apply_plugins_to_monitor_type, load_tasks_from_db, save_tasks_to_db,
};
use sentinel_bounty::services::{
    ChangeMonitorConfig, MonitorPluginConfig, MonitorPluginSeedBindingConfig,
    MonitorPluginSeedConfig, MonitorTask,
};
use sentinel_db::DatabaseService;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

async fn ensure_monitor_tasks_loaded(
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMonitorTaskRequest {
    pub program_id: String,
    pub name: String,
    pub interval_secs: u64,
    pub config: Option<MonitorConfigDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorPluginSeedBindingConfigDto {
    pub seed_type: String,
    pub input_key: String,
    #[serde(default)]
    pub use_project_seeds: bool,
    #[serde(default)]
    pub selected_project_values: Vec<String>,
    #[serde(default)]
    pub manual_values: Vec<String>,
}

impl From<MonitorPluginSeedBindingConfigDto> for MonitorPluginSeedBindingConfig {
    fn from(dto: MonitorPluginSeedBindingConfigDto) -> Self {
        Self {
            seed_type: dto.seed_type,
            input_key: dto.input_key,
            use_project_seeds: dto.use_project_seeds,
            selected_project_values: dto.selected_project_values,
            manual_values: dto.manual_values,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MonitorPluginSeedConfigDto {
    #[serde(default)]
    pub bindings: Vec<MonitorPluginSeedBindingConfigDto>,
}

impl From<MonitorPluginSeedConfigDto> for MonitorPluginSeedConfig {
    fn from(dto: MonitorPluginSeedConfigDto) -> Self {
        Self {
            bindings: dto.bindings.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorPluginConfigDto {
    pub plugin_id: String,
    #[serde(default)]
    pub fallback_plugins: Vec<MonitorPluginConfigDto>,
    #[serde(default)]
    pub plugin_params: serde_json::Value,
    #[serde(default)]
    pub target_asset_types: Vec<String>,
    #[serde(default)]
    pub seed_config: MonitorPluginSeedConfigDto,
}

impl From<MonitorPluginConfigDto> for MonitorPluginConfig {
    fn from(dto: MonitorPluginConfigDto) -> Self {
        Self {
            plugin_id: dto.plugin_id,
            fallback_plugins: dto.fallback_plugins.into_iter().map(Into::into).collect(),
            plugin_params: dto.plugin_params,
            target_asset_types: dto.target_asset_types,
            seed_config: dto.seed_config.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfigDto {
    pub enable_dns_monitoring: Option<bool>,
    #[serde(default)]
    pub dns_plugins: Vec<MonitorPluginConfigDto>,

    pub enable_ip_monitoring: Option<bool>,
    #[serde(default)]
    pub ip_plugins: Vec<MonitorPluginConfigDto>,

    pub enable_cert_monitoring: Option<bool>,
    #[serde(default)]
    pub cert_plugins: Vec<MonitorPluginConfigDto>,

    pub enable_content_monitoring: Option<bool>,
    #[serde(default)]
    pub content_plugins: Vec<MonitorPluginConfigDto>,

    pub enable_api_monitoring: Option<bool>,
    #[serde(default)]
    pub api_plugins: Vec<MonitorPluginConfigDto>,

    pub enable_port_monitoring: Option<bool>,
    #[serde(default)]
    pub port_plugins: Vec<MonitorPluginConfigDto>,

    pub enable_service_monitoring: Option<bool>,
    #[serde(default)]
    pub service_plugins: Vec<MonitorPluginConfigDto>,

    pub enable_web_monitoring: Option<bool>,
    #[serde(default)]
    pub web_plugins: Vec<MonitorPluginConfigDto>,

    #[serde(alias = "enable_vuln_monitoring")]
    pub enable_risk_monitoring: Option<bool>,
    #[serde(default)]
    #[serde(alias = "vuln_plugins")]
    pub risk_plugins: Vec<MonitorPluginConfigDto>,

    pub auto_trigger_enabled: Option<bool>,
    pub auto_trigger_min_severity: Option<String>,
    pub check_interval_secs: Option<u64>,
}

impl From<MonitorConfigDto> for ChangeMonitorConfig {
    fn from(dto: MonitorConfigDto) -> Self {
        let mut config = ChangeMonitorConfig::default();

        if let Some(v) = dto.enable_dns_monitoring {
            config.enable_dns_monitoring = v;
        }
        config.dns_plugins = dto.dns_plugins.into_iter().map(Into::into).collect();

        if let Some(v) = dto.enable_ip_monitoring {
            config.enable_ip_monitoring = v;
        }
        config.ip_plugins = dto.ip_plugins.into_iter().map(Into::into).collect();

        if let Some(v) = dto.enable_cert_monitoring {
            config.enable_cert_monitoring = v;
        }
        config.cert_plugins = dto.cert_plugins.into_iter().map(Into::into).collect();

        if let Some(v) = dto.enable_content_monitoring {
            config.enable_content_monitoring = v;
        }
        config.content_plugins = dto.content_plugins.into_iter().map(Into::into).collect();

        if let Some(v) = dto.enable_api_monitoring {
            config.enable_api_monitoring = v;
        }
        config.api_plugins = dto.api_plugins.into_iter().map(Into::into).collect();

        if let Some(v) = dto.enable_port_monitoring {
            config.enable_port_monitoring = v;
        }
        config.port_plugins = dto.port_plugins.into_iter().map(Into::into).collect();

        if let Some(v) = dto.enable_service_monitoring {
            config.enable_service_monitoring = v;
        }
        config.service_plugins = dto.service_plugins.into_iter().map(Into::into).collect();

        if let Some(v) = dto.enable_web_monitoring {
            config.enable_web_monitoring = v;
        }
        config.web_plugins = dto.web_plugins.into_iter().map(Into::into).collect();

        if let Some(v) = dto.enable_risk_monitoring {
            config.enable_risk_monitoring = v;
        }
        config.risk_plugins = dto.risk_plugins.into_iter().map(Into::into).collect();

        if let Some(v) = dto.auto_trigger_enabled {
            config.auto_trigger_enabled = v;
        }
        if let Some(v) = dto.check_interval_secs {
            config.check_interval_secs = v;
        }

        config.migrate_legacy_port_service_plugins();
        config
    }
}

#[tauri::command]
pub async fn monitor_create_task(
    state: State<'_, Arc<RwLock<MonitorSchedulerState>>>,
    db_service: State<'_, Arc<DatabaseService>>,
    request: CreateMonitorTaskRequest,
) -> Result<String, String> {
    ensure_monitor_tasks_loaded(state.inner(), db_service.inner()).await?;
    let state_guard = state.read().await;

    let mut task = MonitorTask::new(request.program_id, request.name, request.interval_secs);
    if let Some(config_dto) = request.config {
        task.config = config_dto.into();
    }

    let task_id = state_guard.scheduler.add_task(task).await?;
    save_tasks_to_db(&state_guard.scheduler, &db_service).await?;

    Ok(task_id)
}

#[tauri::command]
pub async fn monitor_get_task(
    state: State<'_, Arc<RwLock<MonitorSchedulerState>>>,
    db_service: State<'_, Arc<DatabaseService>>,
    task_id: String,
) -> Result<Option<MonitorTask>, String> {
    ensure_monitor_tasks_loaded(state.inner(), db_service.inner()).await?;
    let state_guard = state.read().await;
    Ok(state_guard.scheduler.get_task(&task_id).await)
}

#[tauri::command]
pub async fn monitor_list_tasks(
    state: State<'_, Arc<RwLock<MonitorSchedulerState>>>,
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: Option<String>,
) -> Result<Vec<MonitorTask>, String> {
    ensure_monitor_tasks_loaded(state.inner(), db_service.inner()).await?;
    let state_guard = state.read().await;
    let mut tasks = state_guard.scheduler.list_tasks().await;

    if let Some(pid) = program_id {
        tasks.retain(|task| task.program_id == pid);
    }

    Ok(tasks)
}

#[tauri::command]
pub async fn monitor_get_running_tasks(
    state: State<'_, Arc<RwLock<MonitorSchedulerState>>>,
) -> Result<Vec<String>, String> {
    let state_guard = state.read().await;
    let running = state_guard.running_task_ids.read().await;
    Ok(running.iter().cloned().collect())
}

#[tauri::command]
pub async fn monitor_stop_task(
    state: State<'_, Arc<RwLock<MonitorSchedulerState>>>,
    task_id: String,
) -> Result<bool, String> {
    let state_guard = state.read().await;
    let is_running = state_guard.running_task_ids.read().await.contains(&task_id);
    if !is_running {
        return Err(format!("Task is not currently running: {}", task_id));
    }

    {
        let mut cancel = state_guard.cancel_requested_task_ids.write().await;
        cancel.insert(task_id.clone());
    }
    let active_run_id = state_guard
        .active_task_run_ids
        .read()
        .await
        .get(&task_id)
        .cloned();
    if let Some(run_id) = active_run_id {
        let cancelled = sentinel_plugins::cancel_plugin_fetch_requests_by_run(
            &run_id,
            "monitor task stopped by user",
        );
        tracing::info!(
            "Cancelled {} queued/running plugin fetch requests for monitor task {} ({})",
            cancelled,
            task_id,
            run_id
        );
    }

    tracing::info!("Stop requested for monitor task: {}", task_id);
    Ok(true)
}

#[tauri::command]
pub async fn monitor_delete_task(
    state: State<'_, Arc<RwLock<MonitorSchedulerState>>>,
    db_service: State<'_, Arc<DatabaseService>>,
    task_id: String,
) -> Result<bool, String> {
    ensure_monitor_tasks_loaded(state.inner(), db_service.inner()).await?;
    let state_guard = state.read().await;
    state_guard.scheduler.remove_task(&task_id).await?;
    save_tasks_to_db(&state_guard.scheduler, &db_service).await?;
    Ok(true)
}

#[tauri::command]
pub async fn monitor_enable_task(
    state: State<'_, Arc<RwLock<MonitorSchedulerState>>>,
    db_service: State<'_, Arc<DatabaseService>>,
    task_id: String,
) -> Result<bool, String> {
    ensure_monitor_tasks_loaded(state.inner(), db_service.inner()).await?;
    let state_guard = state.read().await;
    state_guard.scheduler.enable_task(&task_id).await?;
    save_tasks_to_db(&state_guard.scheduler, &db_service).await?;
    Ok(true)
}

#[tauri::command]
pub async fn monitor_disable_task(
    state: State<'_, Arc<RwLock<MonitorSchedulerState>>>,
    db_service: State<'_, Arc<DatabaseService>>,
    task_id: String,
) -> Result<bool, String> {
    ensure_monitor_tasks_loaded(state.inner(), db_service.inner()).await?;
    let state_guard = state.read().await;
    state_guard.scheduler.disable_task(&task_id).await?;
    save_tasks_to_db(&state_guard.scheduler, &db_service).await?;
    Ok(true)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMonitorTaskRequest {
    pub name: Option<String>,
    pub interval_secs: Option<u64>,
    pub config: Option<MonitorConfigDto>,
}

#[tauri::command]
pub async fn monitor_update_task(
    state: State<'_, Arc<RwLock<MonitorSchedulerState>>>,
    db_service: State<'_, Arc<DatabaseService>>,
    task_id: String,
    request: UpdateMonitorTaskRequest,
) -> Result<bool, String> {
    ensure_monitor_tasks_loaded(state.inner(), db_service.inner()).await?;
    let state_guard = state.read().await;

    state_guard
        .scheduler
        .update_task(&task_id, |task| {
            if let Some(name) = request.name {
                task.name = name;
            }
            if let Some(interval) = request.interval_secs {
                task.interval_secs = interval;
                task.calculate_next_run();
            }
            if let Some(config_dto) = request.config {
                task.config = config_dto.into();
            }
        })
        .await?;

    save_tasks_to_db(&state_guard.scheduler, &db_service).await?;

    Ok(true)
}

#[tauri::command]
pub async fn monitor_create_default_tasks(
    state: State<'_, Arc<RwLock<MonitorSchedulerState>>>,
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: String,
) -> Result<Vec<String>, String> {
    ensure_monitor_tasks_loaded(state.inner(), db_service.inner()).await?;
    let state_guard = state.read().await;
    let mut task_ids = Vec::new();

    let mut dns_task = MonitorTask::new(
        program_id.clone(),
        "DNS, IP & Certificate Monitor".to_string(),
        6 * 3600,
    );
    dns_task.config.enable_dns_monitoring = true;
    dns_task.config.dns_plugins = vec![
        MonitorPluginConfig::new("subdomain_enumerator".to_string()),
        MonitorPluginConfig::new("subdomain_brute".to_string()),
    ];
    dns_task.config.enable_ip_monitoring = true;
    dns_task.config.ip_plugins = vec![MonitorPluginConfig::new("dns_resolver".to_string())];
    dns_task.config.enable_cert_monitoring = true;
    dns_task.config.enable_port_monitoring = false;
    dns_task.config.enable_service_monitoring = false;
    dns_task.config.enable_web_monitoring = false;
    dns_task.config.enable_content_monitoring = false;
    dns_task.config.enable_api_monitoring = false;
    dns_task.config.enable_risk_monitoring = false;
    task_ids.push(state_guard.scheduler.add_task(dns_task).await?);

    let mut content_task = MonitorTask::new(
        program_id.clone(),
        "Content & API Monitor".to_string(),
        24 * 3600,
    );
    content_task.config.enable_dns_monitoring = false;
    content_task.config.enable_ip_monitoring = false;
    content_task.config.enable_cert_monitoring = false;
    content_task.config.enable_content_monitoring = true;
    content_task.config.enable_api_monitoring = true;
    content_task.config.enable_port_monitoring = false;
    content_task.config.enable_service_monitoring = false;
    content_task.config.enable_web_monitoring = false;
    content_task.config.enable_risk_monitoring = false;
    task_ids.push(state_guard.scheduler.add_task(content_task).await?);

    let mut network_task = MonitorTask::new(
        program_id.clone(),
        "Network Surface Monitor".to_string(),
        12 * 3600,
    );
    network_task.config.enable_dns_monitoring = false;
    network_task.config.enable_ip_monitoring = false;
    network_task.config.enable_cert_monitoring = false;
    network_task.config.enable_content_monitoring = false;
    network_task.config.enable_api_monitoring = false;
    network_task.config.enable_port_monitoring = true;
    network_task.config.port_plugins = vec![MonitorPluginConfig::new("port_monitor".to_string())];
    network_task.config.enable_service_monitoring = true;
    network_task.config.service_plugins = vec![MonitorPluginConfig::with_fallbacks(
        "service_monitor".to_string(),
        vec![MonitorPluginConfig::new("service_probe".to_string())],
    )];
    network_task.config.enable_web_monitoring = true;
    network_task.config.enable_risk_monitoring = false;
    task_ids.push(state_guard.scheduler.add_task(network_task).await?);

    let mut risk_task = MonitorTask::new(program_id.clone(), "Risk Monitor".to_string(), 24 * 3600);
    risk_task.config.enable_dns_monitoring = false;
    risk_task.config.enable_ip_monitoring = false;
    risk_task.config.enable_cert_monitoring = false;
    risk_task.config.enable_content_monitoring = false;
    risk_task.config.enable_api_monitoring = false;
    risk_task.config.enable_port_monitoring = false;
    risk_task.config.enable_service_monitoring = false;
    risk_task.config.enable_web_monitoring = false;
    risk_task.config.enable_risk_monitoring = true;
    risk_task.config.risk_plugins = vec![
        MonitorPluginConfig::new("sensitive_file_scanner".to_string()),
        MonitorPluginConfig::new("risk_scanner".to_string()),
    ];
    task_ids.push(state_guard.scheduler.add_task(risk_task).await?);

    save_tasks_to_db(&state_guard.scheduler, &db_service).await?;

    Ok(task_ids)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePluginConfigRequest {
    pub monitor_type: String,
    pub plugins: Vec<MonitorPluginConfigDto>,
}

#[tauri::command]
pub async fn monitor_update_task_plugins(
    state: State<'_, Arc<RwLock<MonitorSchedulerState>>>,
    db_service: State<'_, Arc<DatabaseService>>,
    task_id: String,
    request: UpdatePluginConfigRequest,
) -> Result<bool, String> {
    ensure_monitor_tasks_loaded(state.inner(), db_service.inner()).await?;
    let state_guard = state.read().await;

    state_guard
        .scheduler
        .update_task(&task_id, |task| {
            let plugins: Vec<MonitorPluginConfig> =
                request.plugins.into_iter().map(Into::into).collect();
            apply_plugins_to_monitor_type(&mut task.config, &request.monitor_type, plugins);
        })
        .await?;

    save_tasks_to_db(&state_guard.scheduler, &db_service).await?;

    Ok(true)
}
