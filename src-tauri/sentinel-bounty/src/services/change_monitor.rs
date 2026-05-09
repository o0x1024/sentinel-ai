//! Change monitoring service for ASM (Attack Surface Management)
//!
//! Monitors assets for changes and triggers workflows when changes are detected.

use crate::models::{ChangeEvent, ChangeEventType, ChangeSeverity, CreateChangeEventRequest};
use chrono::Utc;
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

fn normalize_monitor_plugin_id(value: &str) -> String {
    value
        .trim()
        .replace("plugin__service_fingerprinter", "plugin__service_probe")
        .replace("service_fingerprinter", "service_probe")
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
enum MonitorPluginConfigOrId {
    Config(MonitorPluginConfig),
    PluginId(String),
}

fn deserialize_monitor_fallback_plugins<'de, D>(
    deserializer: D,
) -> Result<Vec<MonitorPluginConfig>, D::Error>
where
    D: Deserializer<'de>,
{
    let values = Vec::<MonitorPluginConfigOrId>::deserialize(deserializer)?;

    Ok(values
        .into_iter()
        .map(|value| match value {
            MonitorPluginConfigOrId::Config(config) => config,
            MonitorPluginConfigOrId::PluginId(plugin_id) => MonitorPluginConfig::new(plugin_id),
        })
        .collect())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MonitorPluginSeedBindingConfig {
    pub seed_type: String,
    pub input_key: String,
    #[serde(default)]
    pub use_project_seeds: bool,
    #[serde(default)]
    pub selected_project_values: Vec<String>,
    #[serde(default)]
    pub manual_values: Vec<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MonitorPluginSeedConfig {
    #[serde(default)]
    pub bindings: Vec<MonitorPluginSeedBindingConfig>,
}

/// Plugin configuration for a monitor type
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MonitorPluginConfig {
    /// Primary plugin ID to use
    pub plugin_id: String,
    /// Fallback plugin IDs if primary fails
    #[serde(default, deserialize_with = "deserialize_monitor_fallback_plugins")]
    pub fallback_plugins: Vec<MonitorPluginConfig>,
    /// Custom plugin parameters
    #[serde(default)]
    pub plugin_params: serde_json::Value,
    /// Asset types the plugin expects as targets (web/domain/host/ip/service)
    #[serde(default)]
    pub target_asset_types: Vec<String>,
    /// Explicit discovery seed configuration
    #[serde(default)]
    pub seed_config: MonitorPluginSeedConfig,
}

impl MonitorPluginConfig {
    pub fn new(plugin_id: String) -> Self {
        let normalized_plugin_id = normalize_monitor_plugin_id(&plugin_id);
        let target_asset_types = default_target_asset_types_for_plugin(&plugin_id)
            .into_iter()
            .map(str::to_string)
            .collect();
        Self {
            plugin_id: normalized_plugin_id,
            fallback_plugins: Vec::new(),
            plugin_params: serde_json::Value::Null,
            target_asset_types,
            seed_config: MonitorPluginSeedConfig::default(),
        }
    }

    pub fn with_fallbacks(plugin_id: String, fallbacks: Vec<MonitorPluginConfig>) -> Self {
        let mut plugin = Self::new(plugin_id);
        plugin.fallback_plugins = fallbacks;
        plugin
    }

    /// Get all plugin IDs in order (primary + fallbacks)
    pub fn all_plugins(&self) -> Vec<String> {
        let mut plugins = vec![self.plugin_id.clone()];
        for fallback in &self.fallback_plugins {
            plugins.extend(fallback.all_plugins());
        }
        plugins
    }

    pub fn execution_chain(&self) -> Vec<MonitorPluginConfig> {
        let mut chain = vec![MonitorPluginConfig {
            plugin_id: self.plugin_id.clone(),
            fallback_plugins: Vec::new(),
            plugin_params: self.plugin_params.clone(),
            target_asset_types: self.target_asset_types.clone(),
            seed_config: self.seed_config.clone(),
        }];
        for fallback in &self.fallback_plugins {
            chain.extend(fallback.execution_chain());
        }
        chain
    }

    pub fn normalized_seed_config(&self) -> MonitorPluginSeedConfig {
        let mut bindings = Vec::new();

        for binding in &self.seed_config.bindings {
            let seed_type = binding.seed_type.trim().to_ascii_lowercase();
            let input_key = binding.input_key.trim().to_string();
            if seed_type.is_empty() || input_key.is_empty() {
                continue;
            }

            let mut seen_manual = std::collections::HashSet::new();
            let manual_values = binding
                .manual_values
                .iter()
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .filter(|value| seen_manual.insert(value.to_string()))
                .map(str::to_string)
                .collect();

            bindings.push(MonitorPluginSeedBindingConfig {
                seed_type,
                input_key,
                use_project_seeds: binding.use_project_seeds,
                selected_project_values: binding
                    .selected_project_values
                    .iter()
                    .map(|value| value.trim())
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
                    .collect(),
                manual_values,
            });
        }

        MonitorPluginSeedConfig { bindings }
    }

    pub fn resolved_target_asset_types(
        &self,
        plugin_metadata_target_asset_types: &[String],
    ) -> Vec<String> {
        if !self.target_asset_types.is_empty() {
            return self
                .target_asset_types
                .iter()
                .map(|value| value.trim().to_lowercase())
                .filter(|value| !value.is_empty())
                .collect();
        }

        if !plugin_metadata_target_asset_types.is_empty() {
            return plugin_metadata_target_asset_types
                .iter()
                .map(|value| value.trim().to_lowercase())
                .filter(|value| !value.is_empty())
                .collect();
        }

        default_target_asset_types_for_plugin(&self.plugin_id)
            .into_iter()
            .map(str::to_string)
            .collect()
    }
}

fn ensure_service_probe_engine_default(plugin: &mut MonitorPluginConfig) {
    plugin.plugin_id = normalize_monitor_plugin_id(&plugin.plugin_id);
    for fallback in &mut plugin.fallback_plugins {
        ensure_service_probe_engine_default(fallback);
    }

    let normalized_plugin_id = plugin
        .plugin_id
        .strip_prefix("plugin__")
        .unwrap_or(&plugin.plugin_id);
    if !matches!(normalized_plugin_id, "service_monitor" | "service_probe") {
        return;
    }

    let current = plugin
        .plugin_params
        .get("serviceProbeEngine")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .map(str::to_lowercase);

    if matches!(current.as_deref(), Some("native")) {
        return;
    }

    let mut params = plugin
        .plugin_params
        .as_object()
        .cloned()
        .unwrap_or_default();
    params.insert(
        "serviceProbeEngine".to_string(),
        serde_json::Value::String("native".to_string()),
    );
    plugin.plugin_params = serde_json::Value::Object(params);
}

fn default_target_asset_types_for_plugin(plugin_id: &str) -> Vec<&'static str> {
    let normalized_plugin_id = normalize_monitor_plugin_id(plugin_id);
    let normalized = normalized_plugin_id
        .strip_prefix("plugin__")
        .unwrap_or(&normalized_plugin_id);
    match normalized {
        "http_prober" => vec!["web", "domain", "service"],
        "sensitive_file_scanner"
        | "tech_fingerprinter"
        | "favicon_fingerprinter"
        | "content_monitor"
        | "api_monitor"
        | "js_analyzer"
        | "js_link_finder"
        | "risk_scanner" => vec!["web"],
        "fofa_asset_monitor" => vec!["web", "domain"],
        "subdomain_enumerator" | "dns_resolver" | "subdomain_brute" => vec!["domain"],
        "cert_monitor" | "ssl_scanner" => vec!["domain", "service"],
        "port_monitor" => vec!["ip"],
        "service_monitor" | "service_probe" => vec!["service"],
        "cidr_mapper" => vec!["ip"],
        _ => vec![],
    }
}

fn normalize_dns_monitor_target_asset_types(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.trim().to_lowercase())
        .filter(|value| {
            matches!(
                value.as_str(),
                "domain"
                    | "domain_root"
                    | "root_domain"
                    | "domain_level_1"
                    | "subdomain_level_1"
                    | "first_level_subdomain"
                    | "domain_level_2"
                    | "subdomain_level_2"
                    | "second_level_subdomain"
                    | "domain_level_3_plus"
                    | "subdomain_level_3_plus"
                    | "third_level_subdomain"
            )
        })
        .map(|value| match value.as_str() {
            "root_domain" => "domain_root".to_string(),
            "subdomain_level_1" | "first_level_subdomain" => "domain_level_1".to_string(),
            "subdomain_level_2" | "second_level_subdomain" => "domain_level_2".to_string(),
            "subdomain_level_3_plus" | "third_level_subdomain" => "domain_level_3_plus".to_string(),
            _ => value,
        })
        .collect()
}

/// Change monitor configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChangeMonitorConfig {
    /// Enable DNS change monitoring
    pub enable_dns_monitoring: bool,
    /// Plugin configuration for DNS monitoring
    #[serde(default)]
    pub dns_plugins: Vec<MonitorPluginConfig>,

    /// Enable domain-driven IP resolution monitoring
    #[serde(default)]
    pub enable_ip_monitoring: bool,
    /// Plugin configuration for IP resolution monitoring
    #[serde(default)]
    pub ip_plugins: Vec<MonitorPluginConfig>,

    /// Enable certificate change monitoring
    pub enable_cert_monitoring: bool,
    /// Plugin configuration for certificate monitoring
    #[serde(default)]
    pub cert_plugins: Vec<MonitorPluginConfig>,

    /// Enable content fingerprint monitoring
    pub enable_content_monitoring: bool,
    /// Plugin configuration for content monitoring
    #[serde(default)]
    pub content_plugins: Vec<MonitorPluginConfig>,

    /// Enable API endpoint monitoring
    pub enable_api_monitoring: bool,
    /// Plugin configuration for API monitoring
    #[serde(default)]
    pub api_plugins: Vec<MonitorPluginConfig>,

    /// Enable Port/Service monitoring
    pub enable_port_monitoring: bool,
    /// Plugin configuration for Port monitoring
    #[serde(default)]
    pub port_plugins: Vec<MonitorPluginConfig>,

    /// Enable Service monitoring
    #[serde(default)]
    pub enable_service_monitoring: bool,
    /// Plugin configuration for Service monitoring
    #[serde(default)]
    pub service_plugins: Vec<MonitorPluginConfig>,

    /// Enable Web monitoring (Uptime, Status, Screenshot)
    pub enable_web_monitoring: bool,
    /// Plugin configuration for Web monitoring
    #[serde(default)]
    pub web_plugins: Vec<MonitorPluginConfig>,

    /// Enable Risk monitoring
    #[serde(alias = "enable_vuln_monitoring")]
    pub enable_risk_monitoring: bool,
    /// Plugin configuration for Risk monitoring
    #[serde(default)]
    #[serde(alias = "vuln_plugins")]
    pub risk_plugins: Vec<MonitorPluginConfig>,

    /// Auto-trigger workflows on high severity events
    pub auto_trigger_enabled: bool,
    /// Minimum severity to auto-trigger
    pub auto_trigger_min_severity: ChangeSeverity,
    /// Check interval in seconds
    pub check_interval_secs: u64,
}

impl Default for ChangeMonitorConfig {
    fn default() -> Self {
        Self {
            enable_dns_monitoring: true,
            dns_plugins: vec![
                MonitorPluginConfig::new("subdomain_enumerator".to_string()),
                MonitorPluginConfig::new("subdomain_brute".to_string()),
            ],

            enable_ip_monitoring: true,
            ip_plugins: vec![MonitorPluginConfig::new("dns_resolver".to_string())],

            enable_cert_monitoring: true,
            cert_plugins: vec![MonitorPluginConfig::new("cert_monitor".to_string())],

            enable_content_monitoring: true,
            content_plugins: vec![MonitorPluginConfig::new("content_monitor".to_string())],

            enable_api_monitoring: true,
            api_plugins: vec![MonitorPluginConfig::new("api_monitor".to_string())],

            enable_port_monitoring: true,
            port_plugins: vec![MonitorPluginConfig::new("port_monitor".to_string())],

            enable_service_monitoring: true,
            service_plugins: vec![MonitorPluginConfig::with_fallbacks(
                "service_monitor".to_string(),
                vec![MonitorPluginConfig::new("service_probe".to_string())],
            )],

            enable_web_monitoring: true,
            web_plugins: vec![MonitorPluginConfig::new("http_prober".to_string())],

            enable_risk_monitoring: false, // Default off as it might be heavy
            risk_plugins: vec![
                MonitorPluginConfig::new("sensitive_file_scanner".to_string()),
                MonitorPluginConfig::new("risk_scanner".to_string()),
            ],

            auto_trigger_enabled: true,
            auto_trigger_min_severity: ChangeSeverity::Medium,
            check_interval_secs: 3600, // 1 hour
        }
    }
}

impl ChangeMonitorConfig {
    /// Get all DNS plugin IDs
    pub fn dns_plugin_ids(&self) -> Vec<String> {
        self.dns_plugins
            .iter()
            .flat_map(|p| p.all_plugins())
            .collect()
    }

    /// Get all certificate plugin IDs
    pub fn cert_plugin_ids(&self) -> Vec<String> {
        self.cert_plugins
            .iter()
            .flat_map(|p| p.all_plugins())
            .collect()
    }

    /// Get all IP plugin IDs
    pub fn ip_plugin_ids(&self) -> Vec<String> {
        self.ip_plugins
            .iter()
            .flat_map(|p| p.all_plugins())
            .collect()
    }

    /// Get all content plugin IDs
    pub fn content_plugin_ids(&self) -> Vec<String> {
        self.content_plugins
            .iter()
            .flat_map(|p| p.all_plugins())
            .collect()
    }

    /// Get all API plugin IDs
    pub fn api_plugin_ids(&self) -> Vec<String> {
        self.api_plugins
            .iter()
            .flat_map(|p| p.all_plugins())
            .collect()
    }

    /// Get all Port plugin IDs
    pub fn port_plugin_ids(&self) -> Vec<String> {
        self.port_plugins
            .iter()
            .flat_map(|p| p.all_plugins())
            .collect()
    }

    /// Get all Service plugin IDs
    pub fn service_plugin_ids(&self) -> Vec<String> {
        self.service_plugins
            .iter()
            .flat_map(|p| p.all_plugins())
            .collect()
    }

    /// Get all Web plugin IDs
    pub fn web_plugin_ids(&self) -> Vec<String> {
        self.web_plugins
            .iter()
            .flat_map(|p| p.all_plugins())
            .collect()
    }

    /// Get all risk plugin IDs
    pub fn risk_plugin_ids(&self) -> Vec<String> {
        self.risk_plugins
            .iter()
            .flat_map(|p| p.all_plugins())
            .collect()
    }

    pub fn migrate_legacy_port_service_plugins(&mut self) {
        let mut migrated_ip_plugins = Vec::new();
        let mut normalized_dns_plugins = Vec::new();

        for mut plugin in self.dns_plugins.drain(..) {
            plugin.plugin_id = normalize_monitor_plugin_id(&plugin.plugin_id);
            if plugin.plugin_id == "dns_resolver" {
                migrated_ip_plugins.push(plugin);
                continue;
            }

            let (ip_fallbacks, retained_fallbacks): (Vec<_>, Vec<_>) = plugin
                .fallback_plugins
                .into_iter()
                .partition(|fallback| fallback.plugin_id == "dns_resolver");
            plugin.fallback_plugins = retained_fallbacks;

            if !ip_fallbacks.is_empty() {
                let mut ip_plugin = MonitorPluginConfig::new("dns_resolver".to_string());
                ip_plugin.fallback_plugins = ip_fallbacks;
                migrated_ip_plugins.push(ip_plugin);
            }

            normalized_dns_plugins.push(plugin);
        }

        self.dns_plugins = normalized_dns_plugins;

        if self.ip_plugins.is_empty() && !migrated_ip_plugins.is_empty() {
            self.ip_plugins = migrated_ip_plugins;
        }

        for plugin in &mut self.dns_plugins {
            let normalized_plugin_id = plugin
                .plugin_id
                .strip_prefix("plugin__")
                .unwrap_or(&plugin.plugin_id);
            if !matches!(
                normalized_plugin_id,
                "subdomain_enumerator" | "subdomain_brute"
            ) {
                continue;
            }

            let normalized_target_types =
                normalize_dns_monitor_target_asset_types(&plugin.target_asset_types);

            if normalized_target_types.is_empty() {
                plugin.target_asset_types = vec!["domain".to_string()];
            } else {
                plugin.target_asset_types = normalized_target_types;
            }
        }

        if !self.ip_plugins.is_empty() {
            self.enable_ip_monitoring = true;
        }

        for plugin in &mut self.ip_plugins {
            let normalized_plugin_id = plugin
                .plugin_id
                .strip_prefix("plugin__")
                .unwrap_or(&plugin.plugin_id);
            if normalized_plugin_id != "dns_resolver" {
                continue;
            }

            plugin.target_asset_types = vec!["domain".to_string()];
        }

        let mut migrated_service_plugins = Vec::new();
        let mut normalized_port_plugins = Vec::new();

        for mut plugin in self.port_plugins.drain(..) {
            plugin.plugin_id = normalize_monitor_plugin_id(&plugin.plugin_id);
            if matches!(
                plugin.plugin_id.as_str(),
                "service_monitor" | "service_probe"
            ) {
                migrated_service_plugins.push(plugin);
                continue;
            }

            let (service_fallbacks, retained_fallbacks): (Vec<_>, Vec<_>) =
                plugin.fallback_plugins.into_iter().partition(|fallback| {
                    matches!(
                        normalize_monitor_plugin_id(&fallback.plugin_id).as_str(),
                        "service_monitor" | "service_probe" | "service_fingerprinter"
                    )
                });
            plugin.fallback_plugins = retained_fallbacks;

            if !service_fallbacks.is_empty() {
                const PRIMARY_SERVICE_MONITOR: &str = "service_monitor";
                const FALLBACK_SERVICE_PROBE: &str = "service_probe";

                let preferred_primary = if service_fallbacks.iter().any(|fallback| {
                    normalize_monitor_plugin_id(&fallback.plugin_id) == PRIMARY_SERVICE_MONITOR
                }) {
                    PRIMARY_SERVICE_MONITOR
                } else {
                    FALLBACK_SERVICE_PROBE
                };
                let fallback_plugins = service_fallbacks
                    .into_iter()
                    .map(|mut fallback| {
                        fallback.plugin_id = normalize_monitor_plugin_id(&fallback.plugin_id);
                        fallback
                    })
                    .filter(|fallback| fallback.plugin_id != preferred_primary)
                    .collect();

                let mut service_plugin = MonitorPluginConfig::new(preferred_primary.to_string());
                service_plugin.fallback_plugins = fallback_plugins;
                migrated_service_plugins.push(service_plugin);
            }

            normalized_port_plugins.push(plugin);
        }

        self.port_plugins = normalized_port_plugins;

        if self.service_plugins.is_empty() && !migrated_service_plugins.is_empty() {
            self.service_plugins = migrated_service_plugins;
        }

        if !self.service_plugins.is_empty() {
            self.enable_service_monitoring = true;
        }

        for plugin in &mut self.service_plugins {
            ensure_service_probe_engine_default(plugin);
        }

        for plugin in &mut self.port_plugins {
            if plugin
                .plugin_id
                .strip_prefix("plugin__")
                .unwrap_or(&plugin.plugin_id)
                != "port_monitor"
            {
                continue;
            }

            let normalized_target_types: Vec<String> = plugin
                .target_asset_types
                .iter()
                .map(|value| value.trim().to_lowercase())
                .filter(|value| !value.is_empty())
                .collect();

            if normalized_target_types.is_empty()
                || normalized_target_types
                    .iter()
                    .all(|value| matches!(value.as_str(), "service" | "port" | "endpoint"))
            {
                plugin.target_asset_types = vec!["ip".to_string()];
            }
        }

        for plugin in &mut self.service_plugins {
            let normalized_plugin_id = plugin
                .plugin_id
                .strip_prefix("plugin__")
                .unwrap_or(&plugin.plugin_id);
            if !matches!(normalized_plugin_id, "service_monitor" | "service_probe") {
                continue;
            }

            let normalized_target_types: Vec<String> = plugin
                .target_asset_types
                .iter()
                .map(|value| value.trim().to_lowercase())
                .filter(|value| !value.is_empty())
                .collect();

            if normalized_target_types.is_empty()
                || normalized_target_types.iter().all(|value| {
                    matches!(
                        value.as_str(),
                        "host"
                            | "hostname"
                            | "ip"
                            | "ip_address"
                            | "domain"
                            | "wildcard"
                            | "service"
                            | "port"
                            | "endpoint"
                    )
                })
            {
                plugin.target_asset_types = vec!["service".to_string()];
            }
        }

        for plugin in &mut self.web_plugins {
            let normalized_plugin_id = plugin
                .plugin_id
                .strip_prefix("plugin__")
                .unwrap_or(&plugin.plugin_id);
            if normalized_plugin_id != "http_prober" {
                continue;
            }

            let normalized_target_types: Vec<String> = plugin
                .target_asset_types
                .iter()
                .map(|value| value.trim().to_lowercase())
                .filter(|value| !value.is_empty())
                .collect();

            if normalized_target_types.is_empty() {
                plugin.target_asset_types = vec![
                    "web".to_string(),
                    "domain".to_string(),
                    "service".to_string(),
                ];
            }
        }

        for plugin in &mut self.cert_plugins {
            let normalized_plugin_id = plugin
                .plugin_id
                .strip_prefix("plugin__")
                .unwrap_or(&plugin.plugin_id);
            if !matches!(normalized_plugin_id, "cert_monitor" | "ssl_scanner") {
                continue;
            }

            let normalized_target_types: Vec<String> = plugin
                .target_asset_types
                .iter()
                .map(|value| value.trim().to_lowercase())
                .filter(|value| !value.is_empty())
                .collect();

            if normalized_target_types.is_empty()
                || normalized_target_types
                    .iter()
                    .all(|value| matches!(value.as_str(), "domain" | "host" | "hostname"))
            {
                plugin.target_asset_types = vec!["domain".to_string(), "service".to_string()];
            }
        }
    }
}

/// Asset snapshot for change detection
#[derive(Debug, Clone)]
pub struct AssetSnapshot {
    pub asset_id: String,
    pub dns_records: Option<Vec<String>>,
    pub cert_fingerprint: Option<String>,
    pub cert_expiry: Option<String>,
    pub content_hash: Option<String>,
    pub tech_stack: Option<Vec<String>>,
    pub api_endpoints: Option<Vec<String>>,
    pub last_checked: chrono::DateTime<Utc>,
}

/// Change monitor service
pub struct ChangeMonitor {
    config: ChangeMonitorConfig,
    /// Asset snapshots for comparison
    snapshots: Arc<RwLock<HashMap<String, AssetSnapshot>>>,
    /// Pending events to process
    pending_events: Arc<RwLock<Vec<ChangeEvent>>>,
    /// Running state
    is_running: Arc<RwLock<bool>>,
}

impl ChangeMonitor {
    pub fn new() -> Self {
        Self::with_config(ChangeMonitorConfig::default())
    }

    pub fn with_config(config: ChangeMonitorConfig) -> Self {
        Self {
            config,
            snapshots: Arc::new(RwLock::new(HashMap::new())),
            pending_events: Arc::new(RwLock::new(Vec::new())),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Get current configuration
    pub fn config(&self) -> &ChangeMonitorConfig {
        &self.config
    }

    /// Update configuration
    pub fn set_config(&mut self, config: ChangeMonitorConfig) {
        self.config = config;
    }

    /// Check if monitor is running
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    /// Store asset snapshot for later comparison
    pub async fn store_snapshot(&self, snapshot: AssetSnapshot) {
        let mut snapshots = self.snapshots.write().await;
        snapshots.insert(snapshot.asset_id.clone(), snapshot);
    }

    /// Get stored snapshot for an asset
    pub async fn get_snapshot(&self, asset_id: &str) -> Option<AssetSnapshot> {
        let snapshots = self.snapshots.read().await;
        snapshots.get(asset_id).cloned()
    }

    /// Compare new snapshot with stored one and detect changes
    pub async fn detect_changes(
        &self,
        new_snapshot: &AssetSnapshot,
        program_id: Option<String>,
    ) -> Vec<ChangeEvent> {
        let mut events = Vec::new();
        let old_snapshot = self.get_snapshot(&new_snapshot.asset_id).await;

        if let Some(old) = old_snapshot {
            // DNS change detection
            if self.config.enable_dns_monitoring {
                if let (Some(old_dns), Some(new_dns)) =
                    (&old.dns_records, &new_snapshot.dns_records)
                {
                    if old_dns != new_dns {
                        let mut event = ChangeEvent::new(
                            new_snapshot.asset_id.clone(),
                            ChangeEventType::DnsChange,
                            format!("DNS records changed for {}", new_snapshot.asset_id),
                            "dns_monitor".to_string(),
                        );
                        event.program_id = program_id.clone();
                        event.old_value = Some(old_dns.join(", "));
                        event.new_value = Some(new_dns.join(", "));
                        event.severity = self.calculate_dns_change_severity(old_dns, new_dns);
                        event.auto_trigger_enabled = self.should_auto_trigger(&event.severity);
                        event.calculate_risk_score();
                        events.push(event);
                    }
                }
            }

            // Certificate change detection
            if self.config.enable_cert_monitoring {
                if let (Some(old_cert), Some(new_cert)) =
                    (&old.cert_fingerprint, &new_snapshot.cert_fingerprint)
                {
                    if old_cert != new_cert {
                        let mut event = ChangeEvent::new(
                            new_snapshot.asset_id.clone(),
                            ChangeEventType::CertificateChange,
                            format!("SSL certificate changed for {}", new_snapshot.asset_id),
                            "cert_monitor".to_string(),
                        );
                        event.program_id = program_id.clone();
                        event.old_value = Some(old_cert.clone());
                        event.new_value = Some(new_cert.clone());
                        event.severity = ChangeSeverity::Medium;
                        event.auto_trigger_enabled = self.should_auto_trigger(&event.severity);
                        event.calculate_risk_score();
                        events.push(event);
                    }
                }
            }

            // Content fingerprint change detection
            if self.config.enable_content_monitoring {
                if let (Some(old_hash), Some(new_hash)) =
                    (&old.content_hash, &new_snapshot.content_hash)
                {
                    if old_hash != new_hash {
                        let mut event = ChangeEvent::new(
                            new_snapshot.asset_id.clone(),
                            ChangeEventType::ContentChange,
                            format!("Content changed for {}", new_snapshot.asset_id),
                            "content_monitor".to_string(),
                        );
                        event.program_id = program_id.clone();
                        event.old_value = Some(old_hash.clone());
                        event.new_value = Some(new_hash.clone());
                        event.severity = ChangeSeverity::Low;
                        event.auto_trigger_enabled = self.should_auto_trigger(&event.severity);
                        event.calculate_risk_score();
                        events.push(event);
                    }
                }
            }

            // Technology stack change detection
            if let (Some(old_tech), Some(new_tech)) = (&old.tech_stack, &new_snapshot.tech_stack) {
                if old_tech != new_tech {
                    let mut event = ChangeEvent::new(
                        new_snapshot.asset_id.clone(),
                        ChangeEventType::TechnologyChange,
                        format!("Technology stack changed for {}", new_snapshot.asset_id),
                        "tech_monitor".to_string(),
                    );
                    event.program_id = program_id.clone();
                    event.old_value = Some(old_tech.join(", "));
                    event.new_value = Some(new_tech.join(", "));
                    event.severity = ChangeSeverity::Medium;
                    event.auto_trigger_enabled = self.should_auto_trigger(&event.severity);
                    event.calculate_risk_score();
                    events.push(event);
                }
            }

            // API endpoint change detection
            if self.config.enable_api_monitoring {
                if let (Some(old_api), Some(new_api)) =
                    (&old.api_endpoints, &new_snapshot.api_endpoints)
                {
                    let added: Vec<_> = new_api.iter().filter(|e| !old_api.contains(e)).collect();
                    let removed: Vec<_> = old_api.iter().filter(|e| !new_api.contains(e)).collect();

                    if !added.is_empty() || !removed.is_empty() {
                        let mut event = ChangeEvent::new(
                            new_snapshot.asset_id.clone(),
                            ChangeEventType::ApiChange,
                            format!("API endpoints changed for {}", new_snapshot.asset_id),
                            "api_monitor".to_string(),
                        );
                        event.program_id = program_id.clone();
                        event.description = format!("Added: {:?}, Removed: {:?}", added, removed);
                        event.severity = if !added.is_empty() {
                            ChangeSeverity::High // New endpoints are high priority
                        } else {
                            ChangeSeverity::Low
                        };
                        event.auto_trigger_enabled = self.should_auto_trigger(&event.severity);
                        event.calculate_risk_score();
                        events.push(event);
                    }
                }
            }
        } else {
            // First time seeing this asset - create discovery event
            let mut event = ChangeEvent::new(
                new_snapshot.asset_id.clone(),
                ChangeEventType::AssetDiscovered,
                format!("New asset discovered: {}", new_snapshot.asset_id),
                "discovery".to_string(),
            );
            event.program_id = program_id;
            event.severity = ChangeSeverity::High;
            event.auto_trigger_enabled = self.should_auto_trigger(&event.severity);
            event.calculate_risk_score();
            events.push(event);
        }

        // Store the new snapshot
        self.store_snapshot(new_snapshot.clone()).await;

        // Add to pending events
        if !events.is_empty() {
            let mut pending = self.pending_events.write().await;
            pending.extend(events.clone());
        }

        events
    }

    /// Calculate DNS change severity based on change type
    fn calculate_dns_change_severity(&self, old: &[String], new: &[String]) -> ChangeSeverity {
        let added: Vec<_> = new.iter().filter(|r| !old.contains(r)).collect();
        let removed: Vec<_> = old.iter().filter(|r| !new.contains(r)).collect();

        if !added.is_empty() {
            // New DNS records - could be new subdomains
            ChangeSeverity::High
        } else if !removed.is_empty() {
            ChangeSeverity::Medium
        } else {
            ChangeSeverity::Low
        }
    }

    /// Check if event should auto-trigger workflows
    fn should_auto_trigger(&self, severity: &ChangeSeverity) -> bool {
        if !self.config.auto_trigger_enabled {
            return false;
        }

        let severity_rank = |s: &ChangeSeverity| match s {
            ChangeSeverity::Critical => 4,
            ChangeSeverity::High => 3,
            ChangeSeverity::Medium => 2,
            ChangeSeverity::Low => 1,
        };

        severity_rank(severity) >= severity_rank(&self.config.auto_trigger_min_severity)
    }

    /// Get pending events and clear the queue
    pub async fn take_pending_events(&self) -> Vec<ChangeEvent> {
        let mut pending = self.pending_events.write().await;
        std::mem::take(&mut *pending)
    }

    /// Get pending events count
    pub async fn pending_count(&self) -> usize {
        self.pending_events.read().await.len()
    }

    /// Create change event from request
    pub fn create_event_from_request(&self, request: CreateChangeEventRequest) -> ChangeEvent {
        let mut event = ChangeEvent::new(
            request.asset_id,
            request.event_type,
            request.title,
            request.detection_method,
        );
        event.program_id = request.program_id;
        event.severity = request.severity.unwrap_or_default();
        event.description = request.description;
        event.old_value = request.old_value;
        event.new_value = request.new_value;
        event.diff = request.diff;
        event.affected_scope = request.affected_scope;
        event.tags = request.tags.unwrap_or_default();
        event.auto_trigger_enabled = request
            .auto_trigger_enabled
            .unwrap_or(self.should_auto_trigger(&event.severity));
        event.calculate_risk_score();
        event
    }

    /// Clear all snapshots
    pub async fn clear_snapshots(&self) {
        let mut snapshots = self.snapshots.write().await;
        snapshots.clear();
    }

    /// Get snapshot count
    pub async fn snapshot_count(&self) -> usize {
        self.snapshots.read().await.len()
    }
}

impl Default for ChangeMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_detect_new_asset() {
        let monitor = ChangeMonitor::new();
        let snapshot = AssetSnapshot {
            asset_id: "test-asset".to_string(),
            dns_records: Some(vec!["1.2.3.4".to_string()]),
            cert_fingerprint: None,
            cert_expiry: None,
            content_hash: None,
            tech_stack: None,
            api_endpoints: None,
            last_checked: Utc::now(),
        };

        let events = monitor.detect_changes(&snapshot, None).await;
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0].event_type,
            ChangeEventType::AssetDiscovered
        ));
    }

    #[tokio::test]
    async fn test_detect_dns_change() {
        let monitor = ChangeMonitor::new();

        // First snapshot
        let snapshot1 = AssetSnapshot {
            asset_id: "test-asset".to_string(),
            dns_records: Some(vec!["1.2.3.4".to_string()]),
            cert_fingerprint: None,
            cert_expiry: None,
            content_hash: None,
            tech_stack: None,
            api_endpoints: None,
            last_checked: Utc::now(),
        };
        monitor.detect_changes(&snapshot1, None).await;

        // Second snapshot with DNS change
        let snapshot2 = AssetSnapshot {
            asset_id: "test-asset".to_string(),
            dns_records: Some(vec!["1.2.3.4".to_string(), "5.6.7.8".to_string()]),
            cert_fingerprint: None,
            cert_expiry: None,
            content_hash: None,
            tech_stack: None,
            api_endpoints: None,
            last_checked: Utc::now(),
        };

        let events = monitor.detect_changes(&snapshot2, None).await;
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0].event_type, ChangeEventType::DnsChange));
    }
}
