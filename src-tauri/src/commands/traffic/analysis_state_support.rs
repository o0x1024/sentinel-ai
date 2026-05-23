use std::sync::Arc;

use sentinel_db::{Database, DatabaseService};
use sentinel_traffic::{
    CertificateService, InterceptFilterRule as TrafficInterceptFilterRule, InterceptedRequest,
    InterceptedResponse, MatchReplaceRule as TrafficMatchReplaceRule, PendingInterceptRequest,
    PendingInterceptResponse, PendingInterceptWebSocketMessage, PluginManager, PluginMetadata,
    PluginRecord, PluginStatus, ProxyScopeRule, ProxyService, ScanTask,
};
use tauri::AppHandle;
use tokio::sync::{mpsc::UnboundedSender, RwLock};

use crate::services::system_agents::{
    behavior_extension::BrowserBehaviorEvent, BehaviorExtensionEventStore, BrowserShellStore,
    TrafficBehaviorSignalSettings, TrafficContextExtractionSettings,
    TRAFFIC_BEHAVIOR_SIGNAL_SETTINGS_KEY, TRAFFIC_CONTEXT_EXTRACTION_SETTINGS_KEY,
};
use crate::services::{
    load_plugin_default_inputs, merge_plugin_input_defaults, PluginMainCategory,
};
use crate::utils::plugin_registry_cleanup::cleanup_removed_agent_plugins;

pub struct InterceptedRequestInternal {
    pub request: InterceptedRequest,
    pub response_tx: tokio::sync::oneshot::Sender<sentinel_traffic::InterceptAction>,
}

pub struct InterceptedResponseInternal {
    pub response: InterceptedResponse,
    pub response_tx: tokio::sync::oneshot::Sender<sentinel_traffic::InterceptAction>,
}

pub struct InterceptedWebSocketMessageInternal {
    pub id: String,
    pub connection_id: String,
    pub direction: sentinel_traffic::ProxyWebSocketDirection,
    pub message_type: String,
    pub content: Option<String>,
    pub timestamp: i64,
    pub response_tx: tokio::sync::oneshot::Sender<sentinel_traffic::InterceptAction>,
}

pub struct TrafficAnalysisState {
    pub(crate) proxy_service: Arc<RwLock<Option<ProxyService>>>,
    pub(crate) plugin_manager: Arc<PluginManager>,
    pub(crate) certificate_service: Arc<CertificateService>,
    db_service: Arc<DatabaseService>,
    pub(crate) is_running: Arc<RwLock<bool>>,
    pub(crate) scan_tx: Arc<RwLock<Option<UnboundedSender<ScanTask>>>>,
    pub intercept_enabled: Arc<RwLock<bool>>,
    pub request_intercept_enabled: Arc<RwLock<bool>>,
    pub response_intercept_enabled: Arc<RwLock<bool>>,
    pub intercepted_requests:
        Arc<RwLock<std::collections::HashMap<String, InterceptedRequestInternal>>>,
    pub intercepted_responses:
        Arc<RwLock<std::collections::HashMap<String, InterceptedResponseInternal>>>,
    pub app_handle: Arc<RwLock<Option<AppHandle>>>,
    pub intercept_pending_tx: Arc<RwLock<Option<UnboundedSender<PendingInterceptRequest>>>>,
    pub intercept_response_pending_tx:
        Arc<RwLock<Option<UnboundedSender<PendingInterceptResponse>>>>,
    pub websocket_intercept_enabled: Arc<RwLock<bool>>,
    pub intercepted_websocket_messages:
        Arc<RwLock<std::collections::HashMap<String, InterceptedWebSocketMessageInternal>>>,
    pub intercept_websocket_pending_tx:
        Arc<RwLock<Option<UnboundedSender<PendingInterceptWebSocketMessage>>>>,
    pub history_cache: Arc<sentinel_traffic::ProxyHistoryCache>,
    pub request_filter_rules: Arc<RwLock<Vec<TrafficInterceptFilterRule>>>,
    pub response_filter_rules: Arc<RwLock<Vec<TrafficInterceptFilterRule>>>,
    pub match_replace_rules: Arc<RwLock<Vec<TrafficMatchReplaceRule>>>,
    pub dedupe_cache: Arc<RwLock<std::collections::HashSet<String>>>,
    pub exclude_self_traffic: Arc<RwLock<bool>>,
    pub scope_include_rules: Arc<RwLock<Vec<ProxyScopeRule>>>,
    pub scope_exclude_rules: Arc<RwLock<Vec<ProxyScopeRule>>>,
    pub plugin_scanning_enabled: Arc<RwLock<bool>>,
    pub behavior_signal_settings: Arc<RwLock<TrafficBehaviorSignalSettings>>,
    pub behavior_extension_events: Arc<RwLock<BehaviorExtensionEventStore>>,
    pub browser_shell_store: Arc<RwLock<BrowserShellStore>>,
    pub context_extraction_settings: Arc<RwLock<TrafficContextExtractionSettings>>,
}

impl Clone for TrafficAnalysisState {
    fn clone(&self) -> Self {
        Self {
            proxy_service: self.proxy_service.clone(),
            plugin_manager: self.plugin_manager.clone(),
            certificate_service: self.certificate_service.clone(),
            db_service: self.db_service.clone(),
            is_running: self.is_running.clone(),
            scan_tx: self.scan_tx.clone(),
            intercept_enabled: self.intercept_enabled.clone(),
            request_intercept_enabled: self.request_intercept_enabled.clone(),
            response_intercept_enabled: self.response_intercept_enabled.clone(),
            intercepted_requests: self.intercepted_requests.clone(),
            intercepted_responses: self.intercepted_responses.clone(),
            app_handle: self.app_handle.clone(),
            intercept_pending_tx: self.intercept_pending_tx.clone(),
            intercept_response_pending_tx: self.intercept_response_pending_tx.clone(),
            websocket_intercept_enabled: self.websocket_intercept_enabled.clone(),
            intercepted_websocket_messages: self.intercepted_websocket_messages.clone(),
            intercept_websocket_pending_tx: self.intercept_websocket_pending_tx.clone(),
            history_cache: self.history_cache.clone(),
            request_filter_rules: self.request_filter_rules.clone(),
            response_filter_rules: self.response_filter_rules.clone(),
            match_replace_rules: self.match_replace_rules.clone(),
            dedupe_cache: self.dedupe_cache.clone(),
            exclude_self_traffic: self.exclude_self_traffic.clone(),
            scope_include_rules: self.scope_include_rules.clone(),
            scope_exclude_rules: self.scope_exclude_rules.clone(),
            plugin_scanning_enabled: self.plugin_scanning_enabled.clone(),
            behavior_signal_settings: self.behavior_signal_settings.clone(),
            behavior_extension_events: self.behavior_extension_events.clone(),
            browser_shell_store: self.browser_shell_store.clone(),
            context_extraction_settings: self.context_extraction_settings.clone(),
        }
    }
}

impl std::fmt::Debug for TrafficAnalysisState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrafficAnalysisState")
            .field("is_running", &"RwLock<bool>")
            .finish()
    }
}

impl TrafficAnalysisState {
    pub fn new(db_service: Arc<DatabaseService>) -> Self {
        let app_data_dir = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("sentinel-ai");
        let ca_dir = app_data_dir.join("ca");

        Self {
            proxy_service: Arc::new(RwLock::new(None)),
            plugin_manager: Arc::new(PluginManager::new()),
            certificate_service: Arc::new(CertificateService::new(ca_dir)),
            db_service,
            is_running: Arc::new(RwLock::new(false)),
            scan_tx: Arc::new(RwLock::new(None)),
            intercept_enabled: Arc::new(RwLock::new(false)),
            request_intercept_enabled: Arc::new(RwLock::new(true)),
            response_intercept_enabled: Arc::new(RwLock::new(false)),
            intercepted_requests: Arc::new(RwLock::new(std::collections::HashMap::new())),
            intercepted_responses: Arc::new(RwLock::new(std::collections::HashMap::new())),
            app_handle: Arc::new(RwLock::new(None)),
            intercept_pending_tx: Arc::new(RwLock::new(None)),
            intercept_response_pending_tx: Arc::new(RwLock::new(None)),
            websocket_intercept_enabled: Arc::new(RwLock::new(false)),
            intercepted_websocket_messages: Arc::new(RwLock::new(std::collections::HashMap::new())),
            intercept_websocket_pending_tx: Arc::new(RwLock::new(None)),
            history_cache: Arc::new(sentinel_traffic::ProxyHistoryCache::with_defaults()),
            request_filter_rules: Arc::new(RwLock::new(Vec::new())),
            response_filter_rules: Arc::new(RwLock::new(Vec::new())),
            match_replace_rules: Arc::new(RwLock::new(Vec::new())),
            dedupe_cache: Arc::new(RwLock::new(std::collections::HashSet::new())),
            exclude_self_traffic: Arc::new(RwLock::new(true)),
            scope_include_rules: Arc::new(RwLock::new(Vec::new())),
            scope_exclude_rules: Arc::new(RwLock::new(Vec::new())),
            plugin_scanning_enabled: Arc::new(RwLock::new(true)),
            behavior_signal_settings: Arc::new(RwLock::new(
                TrafficBehaviorSignalSettings::default(),
            )),
            behavior_extension_events: Arc::new(
                RwLock::new(BehaviorExtensionEventStore::default()),
            ),
            browser_shell_store: Arc::new(RwLock::new(BrowserShellStore::default())),
            context_extraction_settings: Arc::new(RwLock::new(
                TrafficContextExtractionSettings::default(),
            )),
        }
    }

    pub fn get_db_service(&self) -> Arc<DatabaseService> {
        self.db_service.clone()
    }

    pub fn get_plugin_manager(&self) -> Arc<PluginManager> {
        self.plugin_manager.clone()
    }

    pub fn get_is_running(&self) -> Arc<RwLock<bool>> {
        self.is_running.clone()
    }

    pub fn get_proxy_service(&self) -> Arc<RwLock<Option<ProxyService>>> {
        self.proxy_service.clone()
    }

    pub fn get_history_cache(&self) -> Arc<sentinel_traffic::ProxyHistoryCache> {
        self.history_cache.clone()
    }

    pub fn get_behavior_signal_settings(&self) -> Arc<RwLock<TrafficBehaviorSignalSettings>> {
        self.behavior_signal_settings.clone()
    }

    pub fn get_behavior_extension_events(&self) -> Arc<RwLock<BehaviorExtensionEventStore>> {
        self.behavior_extension_events.clone()
    }

    pub fn get_browser_shell_store(&self) -> Arc<RwLock<BrowserShellStore>> {
        self.browser_shell_store.clone()
    }

    pub fn get_context_extraction_settings(&self) -> Arc<RwLock<TrafficContextExtractionSettings>> {
        self.context_extraction_settings.clone()
    }

    pub async fn hydrate_system_agent_proxy_settings(&self) -> Result<(), String> {
        let behavior_settings = match self
            .db_service
            .load_proxy_config(TRAFFIC_BEHAVIOR_SIGNAL_SETTINGS_KEY)
            .await
        {
            Ok(Some(raw)) => serde_json::from_str::<TrafficBehaviorSignalSettings>(&raw)
                .map(|value| value.sanitized())
                .unwrap_or_default(),
            Ok(None) => TrafficBehaviorSignalSettings::default(),
            Err(error) => {
                return Err(format!(
                    "Failed to load traffic behavior signal settings: {error}"
                ));
            }
        };
        let context_settings = match self
            .db_service
            .load_proxy_config(TRAFFIC_CONTEXT_EXTRACTION_SETTINGS_KEY)
            .await
        {
            Ok(Some(raw)) => serde_json::from_str::<TrafficContextExtractionSettings>(&raw)
                .map(|value| value.sanitized())
                .unwrap_or_default(),
            Ok(None) => TrafficContextExtractionSettings::default(),
            Err(error) => {
                return Err(format!(
                    "Failed to load traffic context extraction settings: {error}"
                ));
            }
        };

        {
            let mut shared = self.behavior_signal_settings.write().await;
            *shared = behavior_settings;
        }
        {
            let mut shared = self.context_extraction_settings.write().await;
            *shared = context_settings;
        }

        Ok(())
    }

    pub async fn record_behavior_extension_event(&self, event: BrowserBehaviorEvent) {
        let mut events = self.behavior_extension_events.write().await;
        events.record_event(event);
    }

    pub fn get_scan_tx(&self) -> Arc<RwLock<Option<UnboundedSender<ScanTask>>>> {
        self.scan_tx.clone()
    }

    pub async fn set_scan_tx(&self, tx: UnboundedSender<ScanTask>) {
        let mut scan_tx_guard = self.scan_tx.write().await;
        *scan_tx_guard = Some(tx);
    }

    pub fn get_intercept_enabled(&self) -> Arc<RwLock<bool>> {
        self.intercept_enabled.clone()
    }

    pub fn get_request_intercept_enabled(&self) -> Arc<RwLock<bool>> {
        self.request_intercept_enabled.clone()
    }

    pub fn get_intercepted_requests(
        &self,
    ) -> Arc<RwLock<std::collections::HashMap<String, InterceptedRequestInternal>>> {
        self.intercepted_requests.clone()
    }

    pub async fn set_app_handle(&self, app: AppHandle) {
        let mut handle = self.app_handle.write().await;
        *handle = Some(app);
    }

    pub fn get_app_handle(&self) -> Arc<RwLock<Option<AppHandle>>> {
        self.app_handle.clone()
    }

    pub async fn get_running_proxy_address(&self) -> Option<String> {
        let is_running = *self.is_running.read().await;
        if !is_running {
            return None;
        }

        let proxy_opt = self.proxy_service.read().await;
        if let Some(proxy) = proxy_opt.as_ref() {
            if let Some(port) = proxy.get_port().await {
                return Some(format!("http://127.0.0.1:{}", port));
            }
        }
        None
    }

    pub async fn list_plugins_internal(&self) -> Result<Vec<PluginRecord>, String> {
        let db = self.get_db_service();
        let _ = cleanup_removed_agent_plugins(db.as_ref()).await?;

        let db_records = db
            .get_plugins_from_registry(Some("default"))
            .await
            .map_err(|e| format!("Failed to query database plugins: {}", e))?;

        let mut records = Vec::new();
        for db_rec in db_records {
            let metadata = PluginMetadata {
                id: db_rec.metadata.id,
                name: db_rec.metadata.name,
                version: db_rec.metadata.version,
                author: db_rec.metadata.author,
                main_category: db_rec.metadata.main_category,
                category: db_rec.metadata.category,
                description: db_rec.metadata.description,
                monitor_type: db_rec.metadata.monitor_type,
                default_severity: match db_rec.metadata.default_severity {
                    sentinel_plugins::Severity::Critical => sentinel_traffic::Severity::Critical,
                    sentinel_plugins::Severity::High => sentinel_traffic::Severity::High,
                    sentinel_plugins::Severity::Medium => sentinel_traffic::Severity::Medium,
                    sentinel_plugins::Severity::Low => sentinel_traffic::Severity::Low,
                    sentinel_plugins::Severity::Info => sentinel_traffic::Severity::Info,
                },
                tags: db_rec.metadata.tags,
                target_asset_types: db_rec.metadata.target_asset_types,
                input_mode: db_rec.metadata.input_mode,
                seed_bindings: db_rec.metadata.seed_bindings,
            };

            let status = match db_rec.status {
                sentinel_plugins::PluginStatus::Enabled => PluginStatus::Enabled,
                sentinel_plugins::PluginStatus::Disabled => PluginStatus::Disabled,
                sentinel_plugins::PluginStatus::Error => PluginStatus::Disabled,
                sentinel_plugins::PluginStatus::Loaded => PluginStatus::Disabled,
            };

            records.push(PluginRecord {
                metadata,
                #[allow(deprecated)]
                path: None,
                status,
                last_error: None,
                is_favorited: db_rec.is_favorited,
            });
        }

        Ok(records)
    }

    pub async fn execute_agent_plugin(
        &self,
        plugin_id: &str,
        inputs: &serde_json::Value,
    ) -> Result<
        (
            Vec<sentinel_traffic::types::Finding>,
            Option<serde_json::Value>,
        ),
        String,
    > {
        let resolved_plugin_id = ensure_execution_plugin_loaded(self, plugin_id).await?;
        let default_inputs =
            load_plugin_default_inputs(self.db_service.as_ref(), &resolved_plugin_id).await?;
        let resolved_inputs = merge_plugin_input_defaults(&default_inputs, inputs);
        let execution_context = self
            .plugin_manager
            .get_plugin(&resolved_plugin_id)
            .await
            .map(|record| match record.metadata.main_category {
                PluginMainCategory::Intruder => "intruder_processor",
                PluginMainCategory::Bounty => "bounty_workflow",
                PluginMainCategory::Agent | PluginMainCategory::Traffic => "agent_tool",
            })
            .unwrap_or("agent_tool");

        self.plugin_manager
            .execute_execution_plugin(
                &resolved_plugin_id,
                &resolved_inputs,
                execution_context,
                None,
            )
            .await
            .map_err(|e| {
                format!(
                    "Failed to execute agent plugin '{}': {}",
                    resolved_plugin_id, e
                )
            })
    }
}

fn normalize_plugin_lookup_id(value: &str) -> String {
    let stripped = value
        .trim()
        .rsplit_once('.')
        .map(|(stem, ext)| {
            if ext.eq_ignore_ascii_case("js") || ext.eq_ignore_ascii_case("ts") {
                stem
            } else {
                value.trim()
            }
        })
        .unwrap_or_else(|| value.trim());

    let mut result = String::with_capacity(stripped.len());
    let mut previous_was_separator = false;

    for ch in stripped.chars() {
        if ch.is_ascii_alphanumeric() {
            result.push(ch.to_ascii_lowercase());
            previous_was_separator = false;
            continue;
        }

        if (ch == '_' || ch == '-' || ch.is_ascii_whitespace()) && !previous_was_separator {
            result.push('_');
            previous_was_separator = true;
        }
    }

    result.trim_matches('_').to_string()
}

pub(crate) async fn resolve_plugin_registry_id(
    db: &DatabaseService,
    requested_plugin_id: &str,
) -> Result<Option<String>, String> {
    let requested_plugin_id = requested_plugin_id.trim();
    if requested_plugin_id.is_empty() {
        return Ok(None);
    }

    if let Some(plugin) = db
        .get_plugin_from_registry(requested_plugin_id)
        .await
        .map_err(|e| format!("Failed to query plugin '{}': {}", requested_plugin_id, e))?
    {
        return Ok(Some(plugin.metadata.id));
    }

    let normalized_plugin_id = normalize_plugin_lookup_id(requested_plugin_id);
    if normalized_plugin_id.is_empty() || normalized_plugin_id == requested_plugin_id {
        return Ok(None);
    }

    let plugin = db
        .get_plugin_from_registry(&normalized_plugin_id)
        .await
        .map_err(|e| format!("Failed to query plugin '{}': {}", normalized_plugin_id, e))?;

    Ok(plugin.map(|record| record.metadata.id))
}

async fn ensure_execution_plugin_loaded(
    state: &TrafficAnalysisState,
    requested_plugin_id: &str,
) -> Result<String, String> {
    let db = state.get_db_service();
    let resolved_plugin_id = resolve_plugin_registry_id(db.as_ref(), requested_plugin_id)
        .await?
        .ok_or_else(|| format!("Plugin not found: {}", requested_plugin_id))?;

    let plugin_record = db
        .get_plugin_from_registry(&resolved_plugin_id)
        .await
        .map_err(|e| format!("Failed to query plugin '{}': {}", resolved_plugin_id, e))?
        .ok_or_else(|| format!("Plugin not found: {}", resolved_plugin_id))?;

    let plugin_manager = state.get_plugin_manager();
    if plugin_manager
        .get_plugin(&resolved_plugin_id)
        .await
        .is_none()
    {
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

        let metadata = PluginMetadata {
            id: plugin_record.metadata.id.clone(),
            name: plugin_record.metadata.name.clone(),
            version: plugin_record.metadata.version.clone(),
            author: plugin_record.metadata.author.clone(),
            main_category: plugin_record.metadata.main_category,
            category: plugin_record.metadata.category.clone(),
            description: plugin_record.metadata.description.clone(),
            monitor_type: plugin_record.metadata.monitor_type.clone(),
            default_severity: match plugin_record.metadata.default_severity {
                sentinel_plugins::Severity::Critical => sentinel_traffic::Severity::Critical,
                sentinel_plugins::Severity::High => sentinel_traffic::Severity::High,
                sentinel_plugins::Severity::Medium => sentinel_traffic::Severity::Medium,
                sentinel_plugins::Severity::Low => sentinel_traffic::Severity::Low,
                sentinel_plugins::Severity::Info => sentinel_traffic::Severity::Info,
            },
            tags: plugin_record.metadata.tags.clone(),
            target_asset_types: plugin_record.metadata.target_asset_types.clone(),
            input_mode: plugin_record.metadata.input_mode.clone(),
            seed_bindings: plugin_record.metadata.seed_bindings.clone(),
        };

        let enabled = plugin_record.status == sentinel_plugins::PluginStatus::Enabled;
        let _ = plugin_manager
            .register_plugin(resolved_plugin_id.clone(), metadata, enabled)
            .await;
        let _ = plugin_manager
            .set_plugin_code(resolved_plugin_id.clone(), code)
            .await;
    }

    if plugin_record.status == sentinel_plugins::PluginStatus::Enabled {
        let _ = plugin_manager.enable_plugin(&resolved_plugin_id).await;
    }

    Ok(resolved_plugin_id)
}
