use crate::commands::monitor_commands::MonitorSchedulerState;
use crate::commands::monitor_config_support::{
    apply_plugins_to_monitor_type, load_tasks_from_db, repair_legacy_monitor_task_groups,
    save_tasks_to_db,
};
use crate::commands::monitor_progress_support::{
    build_monitor_task_progress_event, emit_monitor_task_progress,
};
use crate::commands::traffic::analysis_state_support::resolve_plugin_registry_id;
use crate::services::ensure_bug_bounty_access;
use chrono::Utc;
use sentinel_bounty::services::{
    ChangeMonitorConfig, MonitorPluginConfig, MonitorPluginSeedBindingConfig,
    MonitorPluginSeedConfig, MonitorTask,
};
use sentinel_db::{BountyProgramRow, Database, DatabaseService, ProgramQueryFilter};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tauri::{AppHandle, State};
use tokio::sync::RwLock;

const MIN_MONITOR_INTERVAL_SECS: u64 = 60;

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

fn validate_monitor_interval_secs(interval_secs: u64) -> Result<u64, String> {
    if interval_secs < MIN_MONITOR_INTERVAL_SECS {
        return Err("检查间隔不能小于 1 分钟".to_string());
    }

    Ok(interval_secs)
}

fn monitor_run_started_at(task_id: &str, run_id: Option<&str>) -> String {
    run_id
        .and_then(|value| value.strip_prefix(&format!("monitor:{task_id}:")))
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| Utc::now().to_rfc3339())
}

fn sanitize_monitor_plugin_params_value(value: &Value) -> Value {
    let Some(object) = value.as_object() else {
        return Value::Object(Map::new());
    };

    let injected_keys = [
        "targets",
        "target_objects",
        "service_targets",
        "urls",
        "url",
        "domains",
        "domain",
        "__monitorExecution",
    ];

    let mut sanitized = Map::new();
    for (key, item) in object {
        if key.starts_with("__monitor") || injected_keys.contains(&key.as_str()) {
            continue;
        }
        sanitized.insert(key.clone(), item.clone());
    }

    Value::Object(sanitized)
}

fn sanitize_value_by_schema(value: &Value, schema: &Value) -> Option<Value> {
    if schema
        .get("enum")
        .and_then(Value::as_array)
        .map(|items| !items.is_empty())
        .unwrap_or(false)
    {
        return Some(value.clone());
    }

    match schema.get("type").and_then(Value::as_str) {
        Some("object") => {
            let Some(value_object) = value.as_object() else {
                return Some(value.clone());
            };

            let Some(properties) = schema.get("properties").and_then(Value::as_object) else {
                return Some(value.clone());
            };

            let mut sanitized = Map::new();
            for (key, property_schema) in properties {
                let Some(child_value) = value_object.get(key) else {
                    continue;
                };

                if let Some(child_sanitized) =
                    sanitize_value_by_schema(child_value, property_schema)
                {
                    sanitized.insert(key.clone(), child_sanitized);
                }
            }

            Some(Value::Object(sanitized))
        }
        Some("array") => {
            let Some(items_schema) = schema.get("items") else {
                return Some(value.clone());
            };
            let Some(array) = value.as_array() else {
                return Some(value.clone());
            };

            Some(Value::Array(
                array
                    .iter()
                    .filter_map(|item| sanitize_value_by_schema(item, items_schema))
                    .collect(),
            ))
        }
        _ => Some(value.clone()),
    }
}

async fn load_monitor_plugin_input_schema(
    db: &DatabaseService,
    requested_plugin_id: &str,
) -> Result<Value, String> {
    let resolved_plugin_id = resolve_plugin_registry_id(db, requested_plugin_id)
        .await?
        .ok_or_else(|| format!("Plugin not found: {}", requested_plugin_id))?;

    let plugin_record = db
        .get_plugin_from_registry(&resolved_plugin_id)
        .await
        .map_err(|e| format!("Failed to query plugin '{}': {}", resolved_plugin_id, e))?
        .ok_or_else(|| format!("Plugin not found: {}", resolved_plugin_id))?;

    let code = db
        .get_plugin_code(&resolved_plugin_id)
        .await
        .map_err(|e| {
            format!(
                "Failed to load plugin code for {}: {}",
                resolved_plugin_id, e
            )
        })?
        .ok_or_else(|| format!("Plugin code not found: {}", resolved_plugin_id))?;

    Ok(
        sentinel_tools::plugin_adapter::PluginToolAdapter::get_input_schema_runtime(
            &code,
            plugin_record.metadata,
        )
        .await,
    )
}

fn collect_monitor_plugin_ids(plugins: &[MonitorPluginConfigDto], ids: &mut HashSet<String>) {
    for plugin in plugins {
        let plugin_id = plugin.plugin_id.trim();
        if !plugin_id.is_empty() {
            ids.insert(plugin_id.to_string());
        }
        collect_monitor_plugin_ids(&plugin.fallback_plugins, ids);
    }
}

fn sanitize_monitor_plugin_configs_by_schema(
    plugins: &mut [MonitorPluginConfigDto],
    schema_map: &HashMap<String, Value>,
) {
    for plugin in plugins {
        let schema = schema_map
            .get(plugin.plugin_id.trim())
            .cloned()
            .unwrap_or_else(|| Value::Object(Map::new()));
        let sanitized_params = sanitize_monitor_plugin_params_value(&plugin.plugin_params);
        plugin.plugin_params = sanitize_value_by_schema(&sanitized_params, &schema)
            .unwrap_or(Value::Object(Map::new()));
        sanitize_monitor_plugin_configs_by_schema(&mut plugin.fallback_plugins, schema_map);
    }
}

async fn sanitize_monitor_config_dto(
    db: &DatabaseService,
    mut config: MonitorConfigDto,
) -> Result<MonitorConfigDto, String> {
    let mut plugin_ids = HashSet::new();
    collect_monitor_plugin_ids(&config.dns_plugins, &mut plugin_ids);
    collect_monitor_plugin_ids(&config.ip_plugins, &mut plugin_ids);
    collect_monitor_plugin_ids(&config.cert_plugins, &mut plugin_ids);
    collect_monitor_plugin_ids(&config.content_plugins, &mut plugin_ids);
    collect_monitor_plugin_ids(&config.api_plugins, &mut plugin_ids);
    collect_monitor_plugin_ids(&config.port_plugins, &mut plugin_ids);
    collect_monitor_plugin_ids(&config.service_plugins, &mut plugin_ids);
    collect_monitor_plugin_ids(&config.web_plugins, &mut plugin_ids);
    collect_monitor_plugin_ids(&config.risk_plugins, &mut plugin_ids);

    let mut schema_map = HashMap::new();
    for plugin_id in plugin_ids {
        let schema = load_monitor_plugin_input_schema(db, &plugin_id).await?;
        schema_map.insert(plugin_id, schema);
    }

    sanitize_monitor_plugin_configs_by_schema(&mut config.dns_plugins, &schema_map);
    sanitize_monitor_plugin_configs_by_schema(&mut config.ip_plugins, &schema_map);
    sanitize_monitor_plugin_configs_by_schema(&mut config.cert_plugins, &schema_map);
    sanitize_monitor_plugin_configs_by_schema(&mut config.content_plugins, &schema_map);
    sanitize_monitor_plugin_configs_by_schema(&mut config.api_plugins, &schema_map);
    sanitize_monitor_plugin_configs_by_schema(&mut config.port_plugins, &schema_map);
    sanitize_monitor_plugin_configs_by_schema(&mut config.service_plugins, &schema_map);
    sanitize_monitor_plugin_configs_by_schema(&mut config.web_plugins, &schema_map);
    sanitize_monitor_plugin_configs_by_schema(&mut config.risk_plugins, &schema_map);

    Ok(config)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMonitorTaskRequest {
    pub program_id: String,
    pub name: String,
    pub interval_secs: u64,
    pub config: Option<MonitorConfigDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMonitorTasksForProgramsRequest {
    #[serde(default)]
    pub program_ids: Vec<String>,
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

async fn list_all_programs(
    db_service: &Arc<DatabaseService>,
) -> Result<Vec<BountyProgramRow>, String> {
    db_service
        .list_bounty_programs_filtered(ProgramQueryFilter {
            platforms: None,
            statuses: None,
            program_types: None,
            tags: None,
            search: None,
            min_priority: None,
            limit: None,
            offset: None,
        })
        .await
        .map_err(|e| e.to_string())
}

async fn resolve_monitor_target_programs(
    db_service: &Arc<DatabaseService>,
    program_ids: &[String],
) -> Result<Vec<BountyProgramRow>, String> {
    let mut programs = list_all_programs(db_service).await?;
    let requested_ids: Vec<String> = program_ids
        .iter()
        .map(|id| id.trim())
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .collect();
    if requested_ids.is_empty() {
        return Err("请选择至少一个项目".to_string());
    }

    programs.retain(|program| requested_ids.iter().any(|id| id == &program.id));
    if programs.len() != requested_ids.len() {
        return Err("选择的项目不存在或已被删除".to_string());
    }
    if programs.is_empty() {
        return Err("没有可创建监控任务的项目".to_string());
    }

    programs.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(programs)
}

fn scoped_task_name(
    base_name: &str,
    program: &BountyProgramRow,
    append_program_name: bool,
) -> String {
    let trimmed = base_name.trim();
    if append_program_name {
        format!("{} - {}", trimmed, program.name)
    } else {
        trimmed.to_string()
    }
}

#[tauri::command]
pub async fn monitor_create_task(
    state: State<'_, Arc<RwLock<MonitorSchedulerState>>>,
    db_service: State<'_, Arc<DatabaseService>>,
    request: CreateMonitorTaskRequest,
) -> Result<String, String> {
    ensure_bug_bounty_access()?;

    ensure_monitor_tasks_loaded(state.inner(), db_service.inner()).await?;
    let CreateMonitorTaskRequest {
        program_id,
        name,
        interval_secs,
        config,
    } = request;
    let interval_secs = validate_monitor_interval_secs(interval_secs)?;
    let sanitized_config = match config {
        Some(config_dto) => {
            Some(sanitize_monitor_config_dto(db_service.inner().as_ref(), config_dto).await?)
        }
        None => None,
    };
    let state_guard = state.read().await;

    let mut task = MonitorTask::new(program_id, name, interval_secs);
    if let Some(config_dto) = sanitized_config {
        task.config = config_dto.into();
    }

    let task_id = state_guard.scheduler.add_task(task).await?;
    save_tasks_to_db(&state_guard.scheduler, &db_service).await?;

    Ok(task_id)
}

#[tauri::command]
pub async fn monitor_create_tasks_for_programs(
    state: State<'_, Arc<RwLock<MonitorSchedulerState>>>,
    db_service: State<'_, Arc<DatabaseService>>,
    request: CreateMonitorTasksForProgramsRequest,
) -> Result<Vec<String>, String> {
    ensure_bug_bounty_access()?;

    ensure_monitor_tasks_loaded(state.inner(), db_service.inner()).await?;
    let CreateMonitorTasksForProgramsRequest {
        program_ids,
        name,
        interval_secs,
        config,
    } = request;
    let interval_secs = validate_monitor_interval_secs(interval_secs)?;

    let name = name.trim();
    if name.is_empty() {
        return Err("任务名称不能为空".to_string());
    }

    let sanitized_config = match config {
        Some(config_dto) => {
            Some(sanitize_monitor_config_dto(db_service.inner().as_ref(), config_dto).await?)
        }
        None => None,
    };

    let programs = resolve_monitor_target_programs(db_service.inner(), &program_ids).await?;
    let append_program_name = programs.len() > 1;
    let group_id = append_program_name.then(|| uuid::Uuid::new_v4().to_string());
    let state_guard = state.read().await;
    let mut task_ids = Vec::with_capacity(programs.len());

    for program in programs {
        let mut task = MonitorTask::new(
            program.id.clone(),
            scoped_task_name(name, &program, append_program_name),
            interval_secs,
        );
        if let Some(group_id) = group_id.as_ref() {
            task.group_id = Some(group_id.clone());
            task.group_name = Some(name.to_string());
        }
        if let Some(config_dto) = sanitized_config.clone() {
            task.config = config_dto.into();
        }
        task_ids.push(state_guard.scheduler.add_task(task).await?);
    }

    save_tasks_to_db(&state_guard.scheduler, &db_service).await?;

    Ok(task_ids)
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
    repair_legacy_monitor_task_groups(&state_guard.scheduler, db_service.inner()).await?;
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
    let cancel_requested = state_guard.cancel_requested_task_ids.read().await;
    Ok(running
        .iter()
        .filter(|task_id| !cancel_requested.contains(*task_id))
        .cloned()
        .collect())
}

#[tauri::command]
pub async fn monitor_stop_task(
    state: State<'_, Arc<RwLock<MonitorSchedulerState>>>,
    app: AppHandle,
    task_id: String,
) -> Result<bool, String> {
    ensure_bug_bounty_access()?;

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
    let task = state_guard.scheduler.get_task(&task_id).await;
    if let Some(run_id) = active_run_id.as_deref() {
        let cancelled = sentinel_plugins::cancel_plugin_fetch_requests_by_run(
            run_id,
            "monitor task stopped by user",
        );
        tracing::info!(
            "Cancelled {} queued/running plugin fetch requests for monitor task {} ({})",
            cancelled,
            task_id,
            run_id
        );
    }
    if let Some(task) = task {
        let started_at = monitor_run_started_at(&task_id, active_run_id.as_deref());
        emit_monitor_task_progress(
            &app,
            &build_monitor_task_progress_event(
                &task,
                "stop_request",
                "stopped",
                0,
                0,
                None,
                None,
                0,
                0,
                Some("Monitor task stop requested".to_string()),
                &started_at,
            ),
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
    ensure_bug_bounty_access()?;

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
    ensure_bug_bounty_access()?;

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
    ensure_bug_bounty_access()?;

    ensure_monitor_tasks_loaded(state.inner(), db_service.inner()).await?;
    let state_guard = state.read().await;
    state_guard.scheduler.disable_task(&task_id).await?;
    save_tasks_to_db(&state_guard.scheduler, &db_service).await?;
    Ok(true)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMonitorTaskRequest {
    pub name: Option<String>,
    pub group_name: Option<String>,
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
    ensure_bug_bounty_access()?;

    ensure_monitor_tasks_loaded(state.inner(), db_service.inner()).await?;
    let UpdateMonitorTaskRequest {
        name,
        group_name,
        interval_secs,
        config,
    } = request;
    let interval_secs = match interval_secs {
        Some(interval_secs) => Some(validate_monitor_interval_secs(interval_secs)?),
        None => None,
    };
    let sanitized_config = match config {
        Some(config_dto) => {
            Some(sanitize_monitor_config_dto(db_service.inner().as_ref(), config_dto).await?)
        }
        None => None,
    };
    let state_guard = state.read().await;

    state_guard
        .scheduler
        .update_task(&task_id, |task| {
            if let Some(name) = name {
                task.name = name;
            }
            if let Some(group_name) = group_name {
                if task.group_id.is_some() {
                    task.group_name = Some(group_name);
                }
            }
            if let Some(interval) = interval_secs {
                task.interval_secs = interval;
                task.calculate_next_run();
            }
            if let Some(config_dto) = sanitized_config {
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
    ensure_bug_bounty_access()?;

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
    ensure_bug_bounty_access()?;

    ensure_monitor_tasks_loaded(state.inner(), db_service.inner()).await?;
    let UpdatePluginConfigRequest {
        monitor_type,
        mut plugins,
    } = request;
    let mut config = MonitorConfigDto {
        enable_dns_monitoring: None,
        dns_plugins: Vec::new(),
        enable_ip_monitoring: None,
        ip_plugins: Vec::new(),
        enable_cert_monitoring: None,
        cert_plugins: Vec::new(),
        enable_content_monitoring: None,
        content_plugins: Vec::new(),
        enable_api_monitoring: None,
        api_plugins: Vec::new(),
        enable_port_monitoring: None,
        port_plugins: Vec::new(),
        enable_service_monitoring: None,
        service_plugins: Vec::new(),
        enable_web_monitoring: None,
        web_plugins: Vec::new(),
        enable_risk_monitoring: None,
        risk_plugins: Vec::new(),
        auto_trigger_enabled: None,
        auto_trigger_min_severity: None,
        check_interval_secs: None,
    };
    match monitor_type.as_str() {
        "dns" => config.dns_plugins = std::mem::take(&mut plugins),
        "ip" => config.ip_plugins = std::mem::take(&mut plugins),
        "cert" => config.cert_plugins = std::mem::take(&mut plugins),
        "content" => config.content_plugins = std::mem::take(&mut plugins),
        "api" => config.api_plugins = std::mem::take(&mut plugins),
        "port" => config.port_plugins = std::mem::take(&mut plugins),
        "service" => config.service_plugins = std::mem::take(&mut plugins),
        "web" => config.web_plugins = std::mem::take(&mut plugins),
        "risk" => config.risk_plugins = std::mem::take(&mut plugins),
        _ => return Err(format!("Unsupported monitor type: {}", monitor_type)),
    }
    let sanitized_config = sanitize_monitor_config_dto(db_service.inner().as_ref(), config).await?;
    let plugins = match monitor_type.as_str() {
        "dns" => sanitized_config.dns_plugins,
        "ip" => sanitized_config.ip_plugins,
        "cert" => sanitized_config.cert_plugins,
        "content" => sanitized_config.content_plugins,
        "api" => sanitized_config.api_plugins,
        "port" => sanitized_config.port_plugins,
        "service" => sanitized_config.service_plugins,
        "web" => sanitized_config.web_plugins,
        "risk" => sanitized_config.risk_plugins,
        _ => Vec::new(),
    };
    let state_guard = state.read().await;

    state_guard
        .scheduler
        .update_task(&task_id, |task| {
            let plugins: Vec<MonitorPluginConfig> = plugins.into_iter().map(Into::into).collect();
            apply_plugins_to_monitor_type(&mut task.config, &monitor_type, plugins);
        })
        .await?;

    save_tasks_to_db(&state_guard.scheduler, &db_service).await?;

    Ok(true)
}
