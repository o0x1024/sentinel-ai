//! Change monitoring service for ASM (Attack Surface Management)
//!
//! Monitors assets for changes and triggers workflows when changes are detected.

use crate::models::{
    ChangeEvent, ChangeEventType, ChangeSeverity, ChangeEventStatus,
    CreateChangeEventRequest,
};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Change monitor configuration
#[derive(Debug, Clone)]
pub struct ChangeMonitorConfig {
    /// Enable DNS change monitoring
    pub enable_dns_monitoring: bool,
    /// Enable certificate change monitoring
    pub enable_cert_monitoring: bool,
    /// Enable content fingerprint monitoring
    pub enable_content_monitoring: bool,
    /// Enable API endpoint monitoring
    pub enable_api_monitoring: bool,
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
            enable_cert_monitoring: true,
            enable_content_monitoring: true,
            enable_api_monitoring: true,
            auto_trigger_enabled: true,
            auto_trigger_min_severity: ChangeSeverity::Medium,
            check_interval_secs: 3600, // 1 hour
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
                if let (Some(old_dns), Some(new_dns)) = (&old.dns_records, &new_snapshot.dns_records) {
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
                if let (Some(old_cert), Some(new_cert)) = (&old.cert_fingerprint, &new_snapshot.cert_fingerprint) {
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
                if let (Some(old_hash), Some(new_hash)) = (&old.content_hash, &new_snapshot.content_hash) {
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
                if let (Some(old_api), Some(new_api)) = (&old.api_endpoints, &new_snapshot.api_endpoints) {
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
                        event.description = format!(
                            "Added: {:?}, Removed: {:?}",
                            added, removed
                        );
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
        event.auto_trigger_enabled = request.auto_trigger_enabled.unwrap_or(
            self.should_auto_trigger(&event.severity)
        );
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
        assert!(matches!(events[0].event_type, ChangeEventType::AssetDiscovered));
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
