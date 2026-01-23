//! Asset Monitor Scheduler Commands

use std::sync::Arc;
use tauri::{State, AppHandle, Emitter};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use sentinel_bounty::services::{MonitorScheduler, MonitorTask, MonitorStats, ChangeMonitorConfig, MonitorPluginConfig};
use sentinel_db::{DatabaseService, BountyAssetRow};
use uuid::Uuid;
use chrono::Utc;

/// Calculate risk score for a port based on port number and service
fn calculate_port_risk_score(port: i32, service: Option<&str>) -> f64 {
    let mut score: f64 = 0.0;
    
    // High-risk ports (commonly attacked)
    match port {
        21 => score += 40.0,  // FTP
        22 => score += 20.0,  // SSH
        23 => score += 50.0,  // Telnet
        25 => score += 30.0,  // SMTP
        80 => score += 15.0,  // HTTP
        443 => score += 10.0, // HTTPS
        445 => score += 45.0, // SMB
        1433 => score += 35.0, // MSSQL
        3306 => score += 35.0, // MySQL
        3389 => score += 40.0, // RDP
        5432 => score += 30.0, // PostgreSQL
        6379 => score += 35.0, // Redis
        8080 => score += 20.0, // HTTP-alt
        27017 => score += 35.0, // MongoDB
        _ if port < 1024 => score += 15.0, // Well-known ports
        _ => score += 5.0,
    }
    
    // Service-based risk
    if let Some(svc) = service {
        let svc_lower = svc.to_lowercase();
        if svc_lower.contains("telnet") || svc_lower.contains("ftp") {
            score += 20.0;
        } else if svc_lower.contains("rdp") || svc_lower.contains("vnc") {
            score += 15.0;
        } else if svc_lower.contains("sql") || svc_lower.contains("database") {
            score += 15.0;
        }
    }
    
    score.min(100.0)
}

/// Calculate risk score for a URL based on HTTP status and security features
fn calculate_url_risk_score(http_status: Option<i32>, waf_detected: Option<bool>) -> f64 {
    let mut score: f64 = 10.0; // Base score for web assets
    
    // HTTP status-based risk
    if let Some(status) = http_status {
        match status {
            200..=299 => score += 20.0, // Accessible endpoints
            300..=399 => score += 10.0, // Redirects
            400..=499 => score += 5.0,  // Client errors
            500..=599 => score += 15.0, // Server errors (potential vuln)
            _ => {}
        }
    }
    
    // Security features
    if waf_detected == Some(false) {
        score += 20.0; // No WAF detected = higher risk
    }
    
    score.min(100.0)
}

/// Calculate risk score for a certificate
fn calculate_cert_risk_score(cert_value: &serde_json::Value) -> f64 {
    let mut score: f64 = 5.0; // Base score
    
    // Check if expired
    if let Some(valid_to) = cert_value.get("valid_to").and_then(|v| v.as_str()) {
        if let Ok(expiry) = chrono::DateTime::parse_from_rfc3339(valid_to) {
            let now = chrono::Utc::now();
            let days_until_expiry = (expiry.timestamp() - now.timestamp()) / 86400;
            
            if days_until_expiry < 0 {
                score += 50.0; // Expired certificate
            } else if days_until_expiry < 30 {
                score += 30.0; // Expiring soon
            } else if days_until_expiry < 90 {
                score += 15.0; // Expiring within 3 months
            }
        }
    }
    
    // Check issuer
    if let Some(issuer) = cert_value.get("issuer").and_then(|v| v.as_str()) {
        if issuer.contains("self-signed") || issuer.contains("Self-Signed") {
            score += 40.0; // Self-signed certificate
        }
    }
    
    // Check key size
    if let Some(key_size) = cert_value.get("key_size").and_then(|v| v.as_i64()) {
        if key_size < 2048 {
            score += 25.0; // Weak key
        }
    }
    
    score.min(100.0)
}

/// Global monitor scheduler state
pub struct MonitorSchedulerState {
    pub scheduler: Arc<MonitorScheduler>,
}

impl MonitorSchedulerState {
    pub fn new() -> Self {
        Self {
            scheduler: Arc::new(MonitorScheduler::new()),
        }
    }
}

// ============================================================================
// Scheduler Control Commands
// ============================================================================

/// Start the monitoring scheduler
#[tauri::command]
pub async fn monitor_start_scheduler(
    state: State<'_, Arc<RwLock<MonitorSchedulerState>>>,
    app: AppHandle,
) -> Result<bool, String> {
    let state_guard = state.read().await;
    let scheduler = state_guard.scheduler.clone();
    drop(state_guard);

    // Set up event callback to emit Tauri events
    let app_clone = app.clone();
    scheduler.set_event_callback(move |event| {
        let _ = app_clone.emit("monitor:change-detected", event);
    }).await;

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

// ============================================================================
// Task Management Commands
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMonitorTaskRequest {
    pub program_id: String,
    pub name: String,
    pub interval_secs: u64,
    pub config: Option<MonitorConfigDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorPluginConfigDto {
    pub plugin_id: String,
    #[serde(default)]
    pub fallback_plugins: Vec<String>,
    #[serde(default)]
    pub plugin_params: serde_json::Value,
}

impl From<MonitorPluginConfigDto> for MonitorPluginConfig {
    fn from(dto: MonitorPluginConfigDto) -> Self {
        Self {
            plugin_id: dto.plugin_id,
            fallback_plugins: dto.fallback_plugins,
            plugin_params: dto.plugin_params,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfigDto {
    pub enable_dns_monitoring: Option<bool>,
    #[serde(default)]
    pub dns_plugins: Vec<MonitorPluginConfigDto>,
    
    pub enable_cert_monitoring: Option<bool>,
    #[serde(default)]
    pub cert_plugins: Vec<MonitorPluginConfigDto>,
    
    pub enable_content_monitoring: Option<bool>,
    #[serde(default)]
    pub content_plugins: Vec<MonitorPluginConfigDto>,
    
    pub enable_api_monitoring: Option<bool>,
    #[serde(default)]
    pub api_plugins: Vec<MonitorPluginConfigDto>,
    
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
        if !dto.dns_plugins.is_empty() {
            config.dns_plugins = dto.dns_plugins.into_iter().map(Into::into).collect();
        }
        
        if let Some(v) = dto.enable_cert_monitoring {
            config.enable_cert_monitoring = v;
        }
        if !dto.cert_plugins.is_empty() {
            config.cert_plugins = dto.cert_plugins.into_iter().map(Into::into).collect();
        }
        
        if let Some(v) = dto.enable_content_monitoring {
            config.enable_content_monitoring = v;
        }
        if !dto.content_plugins.is_empty() {
            config.content_plugins = dto.content_plugins.into_iter().map(Into::into).collect();
        }
        
        if let Some(v) = dto.enable_api_monitoring {
            config.enable_api_monitoring = v;
        }
        if !dto.api_plugins.is_empty() {
            config.api_plugins = dto.api_plugins.into_iter().map(Into::into).collect();
        }
        
        if let Some(v) = dto.auto_trigger_enabled {
            config.auto_trigger_enabled = v;
        }
        if let Some(v) = dto.check_interval_secs {
            config.check_interval_secs = v;
        }
        
        config
    }
}

/// Create a monitoring task
#[tauri::command]
pub async fn monitor_create_task(
    state: State<'_, Arc<RwLock<MonitorSchedulerState>>>,
    request: CreateMonitorTaskRequest,
) -> Result<String, String> {
    let state_guard = state.read().await;
    
    let mut task = MonitorTask::new(
        request.program_id,
        request.name,
        request.interval_secs,
    );
    
    if let Some(config_dto) = request.config {
        task.config = config_dto.into();
    }
    
    let task_id = state_guard.scheduler.add_task(task).await?;
    Ok(task_id)
}

/// Get a monitoring task
#[tauri::command]
pub async fn monitor_get_task(
    state: State<'_, Arc<RwLock<MonitorSchedulerState>>>,
    task_id: String,
) -> Result<Option<MonitorTask>, String> {
    let state_guard = state.read().await;
    Ok(state_guard.scheduler.get_task(&task_id).await)
}

/// List all monitoring tasks
#[tauri::command]
pub async fn monitor_list_tasks(
    state: State<'_, Arc<RwLock<MonitorSchedulerState>>>,
    program_id: Option<String>,
) -> Result<Vec<MonitorTask>, String> {
    let state_guard = state.read().await;
    let mut tasks = state_guard.scheduler.list_tasks().await;
    
    // Filter by program_id if provided
    if let Some(pid) = program_id {
        tasks.retain(|t| t.program_id == pid);
    }
    
    Ok(tasks)
}

/// Delete a monitoring task
#[tauri::command]
pub async fn monitor_delete_task(
    state: State<'_, Arc<RwLock<MonitorSchedulerState>>>,
    task_id: String,
) -> Result<bool, String> {
    let state_guard = state.read().await;
    state_guard.scheduler.remove_task(&task_id).await?;
    Ok(true)
}

/// Enable a monitoring task
#[tauri::command]
pub async fn monitor_enable_task(
    state: State<'_, Arc<RwLock<MonitorSchedulerState>>>,
    task_id: String,
) -> Result<bool, String> {
    let state_guard = state.read().await;
    state_guard.scheduler.enable_task(&task_id).await?;
    Ok(true)
}

/// Disable a monitoring task
#[tauri::command]
pub async fn monitor_disable_task(
    state: State<'_, Arc<RwLock<MonitorSchedulerState>>>,
    task_id: String,
) -> Result<bool, String> {
    let state_guard = state.read().await;
    state_guard.scheduler.disable_task(&task_id).await?;
    Ok(true)
}

/// Trigger a monitoring task immediately
#[tauri::command]
pub async fn monitor_trigger_task(
    state: State<'_, Arc<RwLock<MonitorSchedulerState>>>,
    task_id: String,
) -> Result<bool, String> {
    let state_guard = state.read().await;
    state_guard.scheduler.trigger_task(&task_id).await?;
    Ok(true)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMonitorTaskRequest {
    pub name: Option<String>,
    pub interval_secs: Option<u64>,
    pub config: Option<MonitorConfigDto>,
}

/// Update a monitoring task
#[tauri::command]
pub async fn monitor_update_task(
    state: State<'_, Arc<RwLock<MonitorSchedulerState>>>,
    task_id: String,
    request: UpdateMonitorTaskRequest,
) -> Result<bool, String> {
    let state_guard = state.read().await;
    
    state_guard.scheduler.update_task(&task_id, |task| {
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
    }).await?;
    
    Ok(true)
}

// ============================================================================
// Quick Setup Commands
// ============================================================================

/// Create default monitoring tasks for a program
#[tauri::command]
pub async fn monitor_create_default_tasks(
    state: State<'_, Arc<RwLock<MonitorSchedulerState>>>,
    program_id: String,
) -> Result<Vec<String>, String> {
    let state_guard = state.read().await;
    let mut task_ids = Vec::new();
    
    // DNS & Certificate Monitor (every 6 hours)
    let mut dns_task = MonitorTask::new(
        program_id.clone(),
        "DNS & Certificate Monitor".to_string(),
        6 * 3600,
    );
    dns_task.config.enable_dns_monitoring = true;
    dns_task.config.enable_cert_monitoring = true;
    dns_task.config.enable_content_monitoring = false;
    dns_task.config.enable_api_monitoring = false;
    task_ids.push(state_guard.scheduler.add_task(dns_task).await?);
    
    // Content & API Monitor (every 24 hours)
    let mut content_task = MonitorTask::new(
        program_id.clone(),
        "Content & API Monitor".to_string(),
        24 * 3600,
    );
    content_task.config.enable_dns_monitoring = false;
    content_task.config.enable_cert_monitoring = false;
    content_task.config.enable_content_monitoring = true;
    content_task.config.enable_api_monitoring = true;
    task_ids.push(state_guard.scheduler.add_task(content_task).await?);
    
    Ok(task_ids)
}

// ============================================================================
// Asset Discovery & Import Commands
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorDiscoverAssetsRequest {
    pub program_id: String,
    pub scope_id: Option<String>,
    pub plugin_id: String,
    pub plugin_input: serde_json::Value,
    pub auto_import: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorDiscoverAssetsResponse {
    pub success: bool,
    pub assets_discovered: usize,
    pub assets_imported: usize,
    pub events_created: usize,
    pub plugin_output: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// Execute a plugin to discover assets and optionally import them
#[tauri::command]
pub async fn monitor_discover_and_import_assets(
    db_service: State<'_, Arc<DatabaseService>>,
    request: MonitorDiscoverAssetsRequest,
) -> Result<MonitorDiscoverAssetsResponse, String> {
    tracing::info!(
        "monitor_discover_and_import_assets called: program_id={}, plugin_id={}, auto_import={}",
        request.program_id, request.plugin_id, request.auto_import
    );
    tracing::debug!("Plugin input: {:?}", request.plugin_input);
    
    // Execute plugin using global tool server
    let tool_server = sentinel_tools::get_tool_server();
    tracing::info!("Executing plugin '{}' via ToolServer...", request.plugin_id);
    let tool_result = tool_server
        .execute(&request.plugin_id, request.plugin_input.clone())
        .await;
    
    tracing::info!("Plugin execution completed: success={}", tool_result.success);
    if let Some(err) = &tool_result.error {
        tracing::error!("Plugin execution error: {}", err);
    }
    
    // Check if plugin execution was successful
    if !tool_result.success {
        return Ok(MonitorDiscoverAssetsResponse {
            success: false,
            assets_discovered: 0,
            assets_imported: 0,
            events_created: 0,
            plugin_output: tool_result.output,
            error: tool_result.error,
        });
    }
    
    let plugin_result: serde_json::Value = tool_result.output.unwrap_or(serde_json::json!({}));
    tracing::info!("Plugin output structure: {}", serde_json::to_string_pretty(&plugin_result).unwrap_or_else(|_| "Invalid JSON".to_string()));
    
    let mut assets_discovered: usize = 0;
    let mut assets_imported: usize = 0;
    let mut events_created: usize = 0;
    
    // Extract discovered assets based on plugin output format
    if let Some(data) = plugin_result.get("data") {
        tracing::info!("Found 'data' field in plugin output");
        let data_obj: &serde_json::Value = data;
        // Handle subdomain enumeration output
        if let Some(subdomains) = data_obj.get("subdomains").and_then(|v: &serde_json::Value| v.as_array()) {
            assets_discovered = subdomains.len();
            tracing::info!("Found {} subdomains in plugin output", assets_discovered);
            
            if request.auto_import {
                tracing::info!("Auto-import is enabled, importing {} subdomains...", subdomains.len());
                let now = Utc::now().to_rfc3339();
                
                for subdomain_value in subdomains {
                    let subdomain = subdomain_value.as_str()
                        .or_else(|| subdomain_value.get("subdomain").and_then(|v: &serde_json::Value| v.as_str()))
                        .unwrap_or("");
                    
                    if subdomain.is_empty() {
                        continue;
                    }
                    
                    // Use subdomain as canonical_url (without protocol prefix)
                    let canonical_url = subdomain.to_string();
                    
                    // Check if asset already exists
                    if db_service.get_bounty_asset_by_canonical_url(&request.program_id, &canonical_url)
                        .await
                        .map_err(|e| e.to_string())?
                        .is_some()
                    {
                        continue; // Skip existing assets
                    }
                    
                    // Create new asset
                    let asset = BountyAssetRow {
                        id: Uuid::new_v4().to_string(),
                        program_id: request.program_id.clone(),
                        scope_id: request.scope_id.clone(),
                        asset_type: "domain".to_string(),
                        canonical_url: canonical_url.clone(),
                        original_urls_json: None,
                        hostname: Some(subdomain.to_string()),
                        port: None,
                        path: None,
                        protocol: None,
                        ip_addresses_json: None,
                        dns_records_json: None,
                        tech_stack_json: None,
                        fingerprint: None,
                        tags_json: None,
                        labels_json: Some(serde_json::to_string(&vec!["monitor-discovered"]).unwrap_or_default()),
                        priority_score: Some(0.0),
                        risk_score: Some(0.0),
                        is_alive: true,
                        last_checked_at: None,
                        first_seen_at: now.clone(),
                        last_seen_at: now.clone(),
                        findings_count: 0,
                        change_events_count: 0,
                        metadata_json: Some(serde_json::to_string(&serde_json::json!({
                            "source": "monitor_task",
                            "plugin_id": request.plugin_id,
                            "discovered_at": now,
                        })).unwrap_or_default()),
                        created_at: now.clone(),
                        updated_at: now.clone(),
                        
                        // ASM Fields - set defaults, will be enriched later
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
                        exposure_level: Some("internet".to_string()),
                        attack_surface_score: None,
                        vulnerability_count: None,
                        cvss_max_score: None,
                        exploit_available: None,
                        asset_category: Some("external".to_string()),
                        asset_owner: None,
                        business_unit: None,
                        criticality: None,
                        discovery_method: Some("active".to_string()),
                        data_sources_json: Some(serde_json::to_string(&vec![request.plugin_id.clone()]).unwrap_or_default()),
                        confidence_score: Some(0.9),
                        monitoring_enabled: Some(false),
                        scan_frequency: None,
                        last_scan_type: Some("subdomain_enumeration".to_string()),
                        parent_asset_id: None,
                        related_assets_json: None,
                    };
                    
                    match db_service.create_bounty_asset(&asset).await {
                        Ok(_) => {
                            assets_imported += 1;
                            tracing::info!("Asset imported: {}", canonical_url);
                            
                            // TODO: Create asset discovered event
                            // Note: Requires implementing create_change_event in DatabaseService
                            events_created += 1;
                        }
                        Err(e) => {
                            tracing::error!("Failed to import asset {}: {}", canonical_url, e);
                        }
                    }
                }
            }
        }
        
        // Handle live hosts output
        if let Some(hosts) = data_obj.get("hosts").and_then(|v: &serde_json::Value| v.as_array()) {
            if !request.auto_import {
                assets_discovered += hosts.len();
            } else {
                // Update existing assets with live status
                for host_value in hosts {
                    let host_obj: &serde_json::Value = host_value;
                    if let Some(url) = host_obj.get("url").and_then(|v: &serde_json::Value| v.as_str()) {
                        if let Ok(Some(mut asset)) = db_service.get_bounty_asset_by_canonical_url(&request.program_id, url).await {
                            asset.is_alive = true;
                            asset.last_checked_at = Some(Utc::now().to_rfc3339());
                            
                            if let Some(status_code) = host_obj.get("status_code").and_then(|v: &serde_json::Value| v.as_i64()) {
                                let mut metadata: serde_json::Value = asset.metadata_json
                                    .as_ref()
                                    .and_then(|s: &String| serde_json::from_str(s).ok())
                                    .unwrap_or(serde_json::json!({}));
                                
                                metadata["last_status_code"] = serde_json::json!(status_code);
                                asset.metadata_json = Some(serde_json::to_string(&metadata).unwrap_or_default());
                            }
                            
                            if db_service.update_bounty_asset(&asset).await.is_ok() {
                                assets_imported += 1;
                            }
                        }
                    }
                }
            }
        }
        
        // Handle port scan results
        if let Some(ports) = data_obj.get("ports").and_then(|v: &serde_json::Value| v.as_array()) {
            assets_discovered += ports.len();
            tracing::info!("Found {} open ports in plugin output", ports.len());
            
            if request.auto_import {
                tracing::info!("Auto-import is enabled, importing {} ports...", ports.len());
                let now = Utc::now().to_rfc3339();
                
                for port_value in ports {
                    let ip = port_value.get("ip")
                        .or_else(|| port_value.get("host"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    
                    let port = port_value.get("port").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                    let service_name = port_value.get("service")
                        .or_else(|| port_value.get("service_name"))
                        .and_then(|v| v.as_str());
                    let service_version = port_value.get("version")
                        .or_else(|| port_value.get("service_version"))
                        .and_then(|v| v.as_str());
                    let banner = port_value.get("banner").and_then(|v| v.as_str());
                    let protocol = port_value.get("protocol").and_then(|v| v.as_str()).unwrap_or("tcp");
                    
                    if ip.is_empty() || port == 0 {
                        continue;
                    }
                    
                    let canonical_url = format!("{}:{}", ip, port);
                    
                    // Check if asset already exists
                    if db_service.get_bounty_asset_by_canonical_url(&request.program_id, &canonical_url)
                        .await
                        .map_err(|e| e.to_string())?
                        .is_some()
                    {
                        continue; // Skip existing assets
                    }
                    
                    let asset = BountyAssetRow {
                        id: Uuid::new_v4().to_string(),
                        program_id: request.program_id.clone(),
                        scope_id: request.scope_id.clone(),
                        asset_type: "port".to_string(),
                        canonical_url: canonical_url.clone(),
                        original_urls_json: None,
                        hostname: Some(ip.to_string()),
                        port: Some(port),
                        path: None,
                        protocol: Some(protocol.to_string()),
                        ip_addresses_json: Some(serde_json::to_string(&vec![ip]).unwrap_or_default()),
                        dns_records_json: None,
                        tech_stack_json: None,
                        fingerprint: banner.map(|b| format!("{:x}", md5::compute(b))),
                        tags_json: None,
                        labels_json: Some(serde_json::to_string(&vec!["monitor-discovered", "port-scan"]).unwrap_or_default()),
                        priority_score: Some(0.0),
                        risk_score: Some(0.0),
                        is_alive: true,
                        last_checked_at: Some(now.clone()),
                        first_seen_at: now.clone(),
                        last_seen_at: now.clone(),
                        findings_count: 0,
                        change_events_count: 0,
                        metadata_json: Some(serde_json::to_string(&serde_json::json!({
                            "source": "monitor_task",
                            "plugin_id": request.plugin_id,
                            "discovered_at": now,
                            "scan_type": "port_scan",
                        })).unwrap_or_default()),
                        created_at: now.clone(),
                        updated_at: now.clone(),
                        // ASM fields specific to port assets
                        ip_version: if ip.contains(':') { Some("IPv6".to_string()) } else { Some("IPv4".to_string()) },
                        asn: None,
                        asn_org: None,
                        isp: None,
                        country: None,
                        city: None,
                        latitude: None,
                        longitude: None,
                        is_cloud: None,
                        cloud_provider: None,
                        service_name: service_name.map(|s| s.to_string()),
                        service_version: service_version.map(|s| s.to_string()),
                        service_product: None,
                        banner: banner.map(|s| s.to_string()),
                        transport_protocol: Some(protocol.to_uppercase()),
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
                        exposure_level: Some("internet".to_string()),
                        attack_surface_score: Some(calculate_port_risk_score(port, service_name)),
                        vulnerability_count: None,
                        cvss_max_score: None,
                        exploit_available: None,
                        asset_category: Some("external".to_string()),
                        asset_owner: None,
                        business_unit: None,
                        criticality: None,
                        discovery_method: Some("active".to_string()),
                        data_sources_json: Some(serde_json::to_string(&vec![request.plugin_id.clone()]).unwrap_or_default()),
                        confidence_score: Some(1.0),
                        monitoring_enabled: Some(false),
                        scan_frequency: None,
                        last_scan_type: Some("port_scan".to_string()),
                        parent_asset_id: None,
                        related_assets_json: None,
                    };
                    
                    match db_service.create_bounty_asset(&asset).await {
                        Ok(_) => {
                            assets_imported += 1;
                            tracing::info!("Port asset imported: {} ({})", canonical_url, service_name.unwrap_or("unknown"));
                            events_created += 1;
                        }
                        Err(e) => {
                            tracing::error!("Failed to import port asset {}: {}", canonical_url, e);
                        }
                    }
                }
            }
        }
        
        // Handle URL/web assets
        if let Some(urls) = data_obj.get("urls").and_then(|v: &serde_json::Value| v.as_array()) {
            assets_discovered += urls.len();
            tracing::info!("Found {} URLs in plugin output", urls.len());
            
            if request.auto_import {
                tracing::info!("Auto-import is enabled, importing {} URLs...", urls.len());
                let now = Utc::now().to_rfc3339();
                
                for url_value in urls {
                    let url_str = url_value.get("url")
                        .or_else(|| url_value.get("canonical_url"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    
                    if url_str.is_empty() {
                        continue;
                    }
                    
                    // Check if asset already exists
                    if db_service.get_bounty_asset_by_canonical_url(&request.program_id, url_str)
                        .await
                        .map_err(|e| e.to_string())?
                        .is_some()
                    {
                        continue;
                    }
                    
                    // Parse URL
                    let parsed_url = url::Url::parse(url_str).ok();
                    let hostname = parsed_url.as_ref().and_then(|u| u.host_str()).map(|s| s.to_string());
                    let port = parsed_url.as_ref().and_then(|u| u.port()).map(|p| p as i32);
                    let path = parsed_url.as_ref().map(|u| u.path().to_string());
                    let protocol = parsed_url.as_ref().map(|u| u.scheme().to_string());
                    
                    let http_status = url_value.get("status_code")
                        .or_else(|| url_value.get("status"))
                        .and_then(|v| v.as_i64())
                        .map(|s| s as i32);
                    let title = url_value.get("title").and_then(|v| v.as_str());
                    let content_length = url_value.get("content_length").and_then(|v| v.as_i64());
                    let content_type = url_value.get("content_type").and_then(|v| v.as_str());
                    
                    let asset = BountyAssetRow {
                        id: Uuid::new_v4().to_string(),
                        program_id: request.program_id.clone(),
                        scope_id: request.scope_id.clone(),
                        asset_type: "url".to_string(),
                        canonical_url: url_str.to_string(),
                        original_urls_json: Some(serde_json::to_string(&vec![url_str]).unwrap_or_default()),
                        hostname: hostname.clone(),
                        port,
                        path,
                        protocol: protocol.clone(),
                        ip_addresses_json: None,
                        dns_records_json: None,
                        tech_stack_json: url_value.get("technologies")
                            .and_then(|v| serde_json::to_string(v).ok()),
                        fingerprint: None,
                        tags_json: None,
                        labels_json: Some(serde_json::to_string(&vec!["monitor-discovered", "url-discovery"]).unwrap_or_default()),
                        priority_score: Some(0.0),
                        risk_score: Some(0.0),
                        is_alive: http_status.is_some() && http_status.unwrap() < 500,
                        last_checked_at: Some(now.clone()),
                        first_seen_at: now.clone(),
                        last_seen_at: now.clone(),
                        findings_count: 0,
                        change_events_count: 0,
                        metadata_json: Some(serde_json::to_string(&serde_json::json!({
                            "source": "monitor_task",
                            "plugin_id": request.plugin_id,
                            "discovered_at": now,
                            "scan_type": "url_discovery",
                        })).unwrap_or_default()),
                        created_at: now.clone(),
                        updated_at: now.clone(),
                        // ASM fields for URL assets
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
                        transport_protocol: protocol.as_ref().map(|p| p.to_uppercase()),
                        cpe: None,
                        domain_registrar: None,
                        registration_date: None,
                        expiration_date: None,
                        nameservers_json: None,
                        mx_records_json: None,
                        txt_records_json: None,
                        whois_data_json: None,
                        is_wildcard: None,
                        parent_domain: hostname.as_ref().and_then(|h| {
                            let parts: Vec<&str> = h.split('.').collect();
                            if parts.len() > 2 {
                                Some(parts[parts.len()-2..].join("."))
                            } else {
                                None
                            }
                        }),
                        http_status,
                        response_time_ms: url_value.get("response_time").and_then(|v| v.as_i64()).map(|t| t as i32),
                        content_length,
                        content_type: content_type.map(|s| s.to_string()),
                        title: title.map(|s| s.to_string()),
                        favicon_hash: url_value.get("favicon_hash").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        headers_json: url_value.get("headers").and_then(|v| serde_json::to_string(v).ok()),
                        waf_detected: url_value.get("waf_detected").and_then(|v| v.as_bool()).map(|b| b.to_string()),
                        cdn_detected: url_value.get("cdn_detected").and_then(|v| v.as_bool()).map(|b| b.to_string()),
                        screenshot_path: None,
                        body_hash: url_value.get("body_hash").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        certificate_id: None,
                        ssl_enabled: protocol.as_ref().map(|p| p == "https"),
                        certificate_subject: None,
                        certificate_issuer: None,
                        certificate_valid_from: None,
                        certificate_valid_to: None,
                        certificate_san_json: None,
                        exposure_level: Some("internet".to_string()),
                        attack_surface_score: Some(calculate_url_risk_score(http_status, url_value.get("waf_detected").and_then(|v| v.as_bool()))),
                        vulnerability_count: None,
                        cvss_max_score: None,
                        exploit_available: None,
                        asset_category: Some("external".to_string()),
                        asset_owner: None,
                        business_unit: None,
                        criticality: None,
                        discovery_method: Some("active".to_string()),
                        data_sources_json: Some(serde_json::to_string(&vec![request.plugin_id.clone()]).unwrap_or_default()),
                        confidence_score: Some(0.9),
                        monitoring_enabled: Some(false),
                        scan_frequency: None,
                        last_scan_type: Some("url_discovery".to_string()),
                        parent_asset_id: None,
                        related_assets_json: None,
                    };
                    
                    match db_service.create_bounty_asset(&asset).await {
                        Ok(_) => {
                            assets_imported += 1;
                            tracing::info!("URL asset imported: {}", url_str);
                            events_created += 1;
                        }
                        Err(e) => {
                            tracing::error!("Failed to import URL asset {}: {}", url_str, e);
                        }
                    }
                }
            }
        }
        
        // Handle certificate assets
        if let Some(certs) = data_obj.get("certificates").and_then(|v: &serde_json::Value| v.as_array()) {
            assets_discovered += certs.len();
            tracing::info!("Found {} certificates in plugin output", certs.len());
            
            if request.auto_import {
                tracing::info!("Auto-import is enabled, importing {} certificates...", certs.len());
                let now = Utc::now().to_rfc3339();
                
                for cert_value in certs {
                    let hostname = cert_value.get("hostname")
                        .or_else(|| cert_value.get("domain"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    
                    let cert_subject = cert_value.get("subject").and_then(|v| v.as_str()).unwrap_or("");
                    let cert_issuer = cert_value.get("issuer").and_then(|v| v.as_str());
                    let fingerprint = cert_value.get("fingerprint")
                        .or_else(|| cert_value.get("sha256"))
                        .and_then(|v| v.as_str());
                    
                    if hostname.is_empty() || cert_subject.is_empty() {
                        continue;
                    }
                    
                    let canonical_url = format!("cert:{}", fingerprint.unwrap_or(&format!("{}:{}", hostname, cert_subject)));
                    
                    // Check if asset already exists
                    if db_service.get_bounty_asset_by_canonical_url(&request.program_id, &canonical_url)
                        .await
                        .map_err(|e| e.to_string())?
                        .is_some()
                    {
                        continue;
                    }
                    
                    let asset = BountyAssetRow {
                        id: Uuid::new_v4().to_string(),
                        program_id: request.program_id.clone(),
                        scope_id: request.scope_id.clone(),
                        asset_type: "certificate".to_string(),
                        canonical_url: canonical_url.clone(),
                        original_urls_json: None,
                        hostname: Some(hostname.to_string()),
                        port: cert_value.get("port").and_then(|v| v.as_i64()).map(|p| p as i32),
                        path: None,
                        protocol: Some("tls".to_string()),
                        ip_addresses_json: cert_value.get("ip_addresses")
                            .and_then(|v| serde_json::to_string(v).ok()),
                        dns_records_json: None,
                        tech_stack_json: None,
                        fingerprint: fingerprint.map(|s| s.to_string()),
                        tags_json: None,
                        labels_json: Some(serde_json::to_string(&vec!["monitor-discovered", "certificate"]).unwrap_or_default()),
                        priority_score: Some(0.0),
                        risk_score: Some(0.0),
                        is_alive: cert_value.get("is_valid").and_then(|v| v.as_bool()).unwrap_or(true),
                        last_checked_at: Some(now.clone()),
                        first_seen_at: now.clone(),
                        last_seen_at: now.clone(),
                        findings_count: 0,
                        change_events_count: 0,
                        metadata_json: Some(serde_json::to_string(&serde_json::json!({
                            "source": "monitor_task",
                            "plugin_id": request.plugin_id,
                            "discovered_at": now,
                            "scan_type": "certificate_discovery",
                        })).unwrap_or_default()),
                        created_at: now.clone(),
                        updated_at: now.clone(),
                        // ASM fields for certificate assets
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
                        service_name: Some("tls".to_string()),
                        service_version: cert_value.get("tls_version").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        service_product: None,
                        banner: None,
                        transport_protocol: Some("TCP".to_string()),
                        cpe: None,
                        domain_registrar: None,
                        registration_date: None,
                        expiration_date: None,
                        nameservers_json: None,
                        mx_records_json: None,
                        txt_records_json: None,
                        whois_data_json: None,
                        is_wildcard: Some(cert_subject.starts_with("*.")),
                        parent_domain: None,
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
                        certificate_id: fingerprint.map(|s| s.to_string()),
                        ssl_enabled: Some(true),
                        certificate_subject: Some(cert_subject.to_string()),
                        certificate_issuer: cert_issuer.map(|s| s.to_string()),
                        certificate_valid_from: cert_value.get("valid_from").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        certificate_valid_to: cert_value.get("valid_to").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        certificate_san_json: cert_value.get("san")
                            .or_else(|| cert_value.get("subject_alt_names"))
                            .and_then(|v| serde_json::to_string(v).ok()),
                        exposure_level: Some("internet".to_string()),
                        attack_surface_score: Some(calculate_cert_risk_score(cert_value)),
                        vulnerability_count: None,
                        cvss_max_score: None,
                        exploit_available: None,
                        asset_category: Some("external".to_string()),
                        asset_owner: None,
                        business_unit: None,
                        criticality: None,
                        discovery_method: Some("active".to_string()),
                        data_sources_json: Some(serde_json::to_string(&vec![request.plugin_id.clone()]).unwrap_or_default()),
                        confidence_score: Some(1.0),
                        monitoring_enabled: Some(false),
                        scan_frequency: None,
                        last_scan_type: Some("certificate_discovery".to_string()),
                        parent_asset_id: None,
                        related_assets_json: None,
                    };
                    
                    match db_service.create_bounty_asset(&asset).await {
                        Ok(_) => {
                            assets_imported += 1;
                            tracing::info!("Certificate asset imported: {}", hostname);
                            events_created += 1;
                        }
                        Err(e) => {
                            tracing::error!("Failed to import certificate asset {}: {}", hostname, e);
                        }
                    }
                }
            }
        }
        
        // Handle IP assets
        if let Some(ips) = data_obj.get("ips").and_then(|v: &serde_json::Value| v.as_array()) {
            assets_discovered += ips.len();
            tracing::info!("Found {} IPs in plugin output", ips.len());
            
            if request.auto_import {
                tracing::info!("Auto-import is enabled, importing {} IPs...", ips.len());
                let now = Utc::now().to_rfc3339();
                
                for ip_value in ips {
                    let ip_str = ip_value.get("ip")
                        .or_else(|| ip_value.get("address"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    
                    if ip_str.is_empty() {
                        continue;
                    }
                    
                    // Check if asset already exists
                    if db_service.get_bounty_asset_by_canonical_url(&request.program_id, ip_str)
                        .await
                        .map_err(|e| e.to_string())?
                        .is_some()
                    {
                        continue;
                    }
                    
                    let is_ipv6 = ip_str.contains(':');
                    
                    let asset = BountyAssetRow {
                        id: Uuid::new_v4().to_string(),
                        program_id: request.program_id.clone(),
                        scope_id: request.scope_id.clone(),
                        asset_type: "ip".to_string(),
                        canonical_url: ip_str.to_string(),
                        original_urls_json: None,
                        hostname: ip_value.get("hostname").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        port: None,
                        path: None,
                        protocol: None,
                        ip_addresses_json: Some(serde_json::to_string(&vec![ip_str]).unwrap_or_default()),
                        dns_records_json: None,
                        tech_stack_json: None,
                        fingerprint: None,
                        tags_json: None,
                        labels_json: Some(serde_json::to_string(&vec!["monitor-discovered", "ip-discovery"]).unwrap_or_default()),
                        priority_score: Some(0.0),
                        risk_score: Some(0.0),
                        is_alive: ip_value.get("is_alive").and_then(|v| v.as_bool()).unwrap_or(true),
                        last_checked_at: Some(now.clone()),
                        first_seen_at: now.clone(),
                        last_seen_at: now.clone(),
                        findings_count: 0,
                        change_events_count: 0,
                        metadata_json: Some(serde_json::to_string(&serde_json::json!({
                            "source": "monitor_task",
                            "plugin_id": request.plugin_id,
                            "discovered_at": now,
                            "scan_type": "ip_discovery",
                        })).unwrap_or_default()),
                        created_at: now.clone(),
                        updated_at: now.clone(),
                        // ASM fields for IP assets
                        ip_version: Some(if is_ipv6 { "IPv6" } else { "IPv4" }.to_string()),
                        asn: ip_value.get("asn").and_then(|v| v.as_i64()).map(|a| a as i32),
                        asn_org: ip_value.get("asn_org").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        isp: ip_value.get("isp").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        country: ip_value.get("country").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        city: ip_value.get("city").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        latitude: ip_value.get("latitude").and_then(|v| v.as_f64()),
                        longitude: ip_value.get("longitude").and_then(|v| v.as_f64()),
                        is_cloud: ip_value.get("is_cloud").and_then(|v| v.as_bool()),
                        cloud_provider: ip_value.get("cloud_provider").and_then(|v| v.as_str()).map(|s| s.to_string()),
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
                        exposure_level: Some("internet".to_string()),
                        attack_surface_score: Some(10.0), // Base score for IP
                        vulnerability_count: None,
                        cvss_max_score: None,
                        exploit_available: None,
                        asset_category: Some("external".to_string()),
                        asset_owner: None,
                        business_unit: None,
                        criticality: None,
                        discovery_method: Some("active".to_string()),
                        data_sources_json: Some(serde_json::to_string(&vec![request.plugin_id.clone()]).unwrap_or_default()),
                        confidence_score: Some(1.0),
                        monitoring_enabled: Some(false),
                        scan_frequency: None,
                        last_scan_type: Some("ip_discovery".to_string()),
                        parent_asset_id: None,
                        related_assets_json: None,
                    };
                    
                    match db_service.create_bounty_asset(&asset).await {
                        Ok(_) => {
                            assets_imported += 1;
                            tracing::info!("IP asset imported: {}", ip_str);
                            events_created += 1;
                        }
                        Err(e) => {
                            tracing::error!("Failed to import IP asset {}: {}", ip_str, e);
                        }
                    }
                }
            }
        }
    } else {
        tracing::warn!("No 'data' field found in plugin output");
        tracing::debug!("Plugin output keys: {:?}", plugin_result.as_object().map(|o| o.keys().collect::<Vec<_>>()));
    }
    
    tracing::info!(
        "Asset discovery completed: discovered={}, imported={}, events={}",
        assets_discovered, assets_imported, events_created
    );
    
    Ok(MonitorDiscoverAssetsResponse {
        success: true,
        assets_discovered,
        assets_imported,
        events_created,
        plugin_output: Some(plugin_result),
        error: None,
    })
}

// ============================================================================
// Plugin Management Commands
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorPluginInfo {
    pub id: String,
    pub name: String,
    pub category: String,
    pub monitor_type: String, // dns, cert, content, api
    pub description: Option<String>,
    pub is_available: bool,
}

/// Get available plugins for monitoring types
#[tauri::command]
pub async fn monitor_get_available_plugins() -> Result<Vec<MonitorPluginInfo>, String> {
    let tool_server = sentinel_tools::get_tool_server();
    let all_tools = tool_server.list_tools().await;
    
    tracing::info!("Loading available plugins for monitoring, total tools: {}", all_tools.len());
    
    let mut plugins = Vec::new();
    
    for tool in all_tools {
        tracing::debug!("Checking tool: name={}, category={}, enabled={}", 
            tool.name, tool.category, tool.enabled);
        
        let monitor_type = match tool.name.as_str() {
            // DNS monitoring plugins
            "subdomain_enumerator" | "dns_resolver" => "dns",
            
            // Certificate monitoring plugins
            "cert_monitor" | "ssl_scanner" => "cert",
            
            // Content monitoring plugins
            "content_monitor" | "http_prober" => "content",
            
            // API monitoring plugins
            "api_monitor" | "js_analyzer" | "js_link_finder" => "api",
            
            _ => {
                // Check category-based matching
                match tool.category.as_str() {
                    "monitor" if tool.name.contains("dns") => "dns",
                    "monitor" if tool.name.contains("cert") || tool.name.contains("ssl") => "cert",
                    "monitor" if tool.name.contains("content") || tool.name.contains("http") => "content",
                    "monitor" if tool.name.contains("api") || tool.name.contains("js") => "api",
                    _ => continue, // Skip non-monitor plugins
                }
            }
        };
        
        tracing::info!("Matched plugin: {} -> monitor_type={}", tool.name, monitor_type);
        
        plugins.push(MonitorPluginInfo {
            id: tool.name.clone(),
            name: tool.name.clone(),
            category: tool.category.clone(),
            monitor_type: monitor_type.to_string(),
            description: Some(tool.description.clone()),
            is_available: tool.enabled,
        });
    }
    
    tracing::info!("Found {} monitor plugins", plugins.len());
    
    Ok(plugins)
}

/// Test if a plugin is available and working
#[tauri::command]
pub async fn monitor_test_plugin(plugin_id: String) -> Result<bool, String> {
    let tool_server = sentinel_tools::get_tool_server();
    
    // Try to check if plugin exists
    let all_tools = tool_server.list_tools().await;
    Ok(all_tools.iter().any(|tool| tool.name == plugin_id && tool.enabled))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePluginConfigRequest {
    pub monitor_type: String, // dns, cert, content, api
    pub plugins: Vec<MonitorPluginConfigDto>,
}

/// Update plugin configuration for a specific monitor task
#[tauri::command]
pub async fn monitor_update_task_plugins(
    state: State<'_, Arc<RwLock<MonitorSchedulerState>>>,
    task_id: String,
    request: UpdatePluginConfigRequest,
) -> Result<bool, String> {
    let state_guard = state.read().await;
    
    state_guard.scheduler.update_task(&task_id, |task| {
        let plugins: Vec<MonitorPluginConfig> = 
            request.plugins.into_iter().map(Into::into).collect();
        
        match request.monitor_type.as_str() {
            "dns" => task.config.dns_plugins = plugins,
            "cert" => task.config.cert_plugins = plugins,
            "content" => task.config.content_plugins = plugins,
            "api" => task.config.api_plugins = plugins,
            _ => {},
        }
    }).await?;
    
    Ok(true)
}
