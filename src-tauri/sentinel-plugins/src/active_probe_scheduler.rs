use crate::request_scheduler::{
    cancel_plugin_request, complete_plugin_request, configured_policy_for_kind,
    enqueue_plugin_request, fail_plugin_request, get_plugin_request_queue_snapshot,
    mark_plugin_request_running, PluginFetchPolicy, PluginFetchPolicyKind,
    PluginRequestDispatchGrant, PluginRequestPhase, PluginRequestQueueEntry,
    PluginRequestScheduleRequest,
};
use crate::runtime_events::emit_active_probe_queue_event;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use tokio::sync::oneshot;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActiveProbeQueuePhase {
    Queued,
    Scheduled,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveProbeQueueEntry {
    pub plugin_id: String,
    pub traffic_request_id: String,
    pub request_id: String,
    pub phase: ActiveProbeQueuePhase,
    pub method: String,
    pub url: String,
    pub probe_label: Option<String>,
    pub target_name: Option<String>,
    pub target_path: Option<String>,
    pub target_location: Option<String>,
    pub probe_value: Option<String>,
    pub technique: Option<String>,
    pub probe_class: String,
    pub probe_priority: i32,
    pub cooldown_key: String,
    pub cooldown_wait_ms: Option<u64>,
    pub jitter_wait_ms: Option<u64>,
    pub total_wait_ms: Option<u64>,
    pub adaptive_penalty_ms: u64,
    pub status: Option<u16>,
    pub error: Option<String>,
    pub reason: Option<String>,
    pub active_slots: u32,
    pub max_concurrent_per_host: u32,
    pub queue_depth: u32,
    pub response_elapsed_ms: Option<u64>,
    pub queued_at: String,
    pub scheduled_at: Option<String>,
    pub dispatch_started_at: Option<String>,
    pub finished_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveProbeQueueSnapshot {
    pub pending: Vec<ActiveProbeQueueEntry>,
    pub running: Vec<ActiveProbeQueueEntry>,
    pub recent: Vec<ActiveProbeQueueEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveProbeQueueEventPayload {
    pub entries: Vec<ActiveProbeQueueEntry>,
}

#[derive(Debug, Clone)]
pub struct ActiveProbeRequest {
    pub plugin_id: String,
    pub traffic_request_id: String,
    pub request_id: String,
    pub method: String,
    pub url: String,
    pub probe_label: Option<String>,
    pub target_name: Option<String>,
    pub target_path: Option<String>,
    pub target_location: Option<String>,
    pub probe_value: Option<String>,
    pub technique: Option<String>,
    pub probe_class: String,
    pub probe_priority: i32,
    pub cooldown_key: String,
    pub jitter_range: [u64; 2],
    pub min_host_cooldown_ms: u64,
    pub max_concurrent_per_host: u32,
}

#[derive(Debug, Clone)]
pub struct ActiveProbeDispatchGrant {
    pub active_slots: u32,
    pub max_concurrent_per_host: u32,
    pub queue_depth: u32,
    pub cooldown_wait_ms: u64,
    pub jitter_wait_ms: u64,
    pub total_wait_ms: u64,
    pub adaptive_penalty_ms: u64,
    pub timeout_ms: u64,
}

#[derive(Debug, Default)]
struct ActiveProbeMetadataState {
    requests: HashMap<String, ActiveProbeRequest>,
}

static ACTIVE_PROBE_METADATA: OnceLock<Mutex<ActiveProbeMetadataState>> = OnceLock::new();

fn metadata_state() -> &'static Mutex<ActiveProbeMetadataState> {
    ACTIVE_PROBE_METADATA.get_or_init(|| Mutex::new(ActiveProbeMetadataState::default()))
}

fn active_probe_policy() -> PluginFetchPolicy {
    configured_policy_for_kind(PluginFetchPolicyKind::TrafficActiveProbe)
}

fn normalize_fetch_host(url: &str) -> Result<String, String> {
    let parsed = reqwest::Url::parse(url)
        .map_err(|error| format!("Invalid active probe URL '{}': {}", url, error))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| format!("Active probe URL '{}' has no host", url))?;
    let port = parsed
        .port()
        .map(|port| format!(":{port}"))
        .unwrap_or_default();
    Ok(format!("{host}{port}"))
}

fn build_schedule(request: &ActiveProbeRequest) -> Result<PluginRequestScheduleRequest, String> {
    Ok(PluginRequestScheduleRequest {
        kind: PluginFetchPolicyKind::TrafficActiveProbe,
        request_id: request.request_id.clone(),
        run_id: request.traffic_request_id.clone(),
        plugin_id: request.plugin_id.clone(),
        execution_context: "traffic_active_probe".to_string(),
        method: request.method.clone(),
        url: request.url.clone(),
        host: normalize_fetch_host(&request.url)?,
    })
}

fn phase_from_plugin(entry: &PluginRequestQueueEntry) -> ActiveProbeQueuePhase {
    match entry.phase {
        PluginRequestPhase::Queued => ActiveProbeQueuePhase::Queued,
        PluginRequestPhase::Scheduled => ActiveProbeQueuePhase::Scheduled,
        PluginRequestPhase::Running => ActiveProbeQueuePhase::Running,
        PluginRequestPhase::Completed => ActiveProbeQueuePhase::Completed,
        PluginRequestPhase::Failed => ActiveProbeQueuePhase::Failed,
        PluginRequestPhase::Cancelled => ActiveProbeQueuePhase::Cancelled,
    }
}

fn map_entry(
    entry: &PluginRequestQueueEntry,
    metadata: Option<&ActiveProbeRequest>,
) -> ActiveProbeQueueEntry {
    let policy = active_probe_policy();
    ActiveProbeQueueEntry {
        plugin_id: entry.plugin_id.clone(),
        traffic_request_id: entry.run_id.clone(),
        request_id: entry.request_id.clone(),
        phase: phase_from_plugin(entry),
        method: entry.method.clone(),
        url: entry.url.clone(),
        probe_label: metadata.and_then(|request| request.probe_label.clone()),
        target_name: metadata.and_then(|request| request.target_name.clone()),
        target_path: metadata.and_then(|request| request.target_path.clone()),
        target_location: metadata.and_then(|request| request.target_location.clone()),
        probe_value: metadata.and_then(|request| request.probe_value.clone()),
        technique: metadata.and_then(|request| request.technique.clone()),
        probe_class: metadata
            .map(|request| request.probe_class.clone())
            .unwrap_or_else(|| "fast".to_string()),
        probe_priority: metadata.map(|request| request.probe_priority).unwrap_or(0),
        cooldown_key: metadata
            .map(|request| request.cooldown_key.clone())
            .unwrap_or_else(|| entry.host.clone()),
        cooldown_wait_ms: entry.cooldown_wait_ms,
        jitter_wait_ms: entry.jitter_wait_ms,
        total_wait_ms: entry.total_wait_ms,
        adaptive_penalty_ms: entry.adaptive_penalty_ms,
        status: entry.status,
        error: entry.error.clone(),
        reason: entry.reason.clone(),
        active_slots: 0,
        max_concurrent_per_host: policy.max_concurrent_per_host,
        queue_depth: entry.queue_depth,
        response_elapsed_ms: entry.response_elapsed_ms,
        queued_at: entry.queued_at.clone(),
        scheduled_at: entry.scheduled_at.clone(),
        dispatch_started_at: entry.dispatch_started_at.clone(),
        finished_at: entry.finished_at.clone(),
        updated_at: entry.updated_at.clone(),
    }
}

fn map_grant(grant: PluginRequestDispatchGrant) -> ActiveProbeDispatchGrant {
    let policy = active_probe_policy();
    ActiveProbeDispatchGrant {
        active_slots: grant.active_for_host,
        max_concurrent_per_host: policy.max_concurrent_per_host,
        queue_depth: 0,
        cooldown_wait_ms: grant.cooldown_wait_ms,
        jitter_wait_ms: grant.jitter_wait_ms,
        total_wait_ms: grant.total_wait_ms,
        adaptive_penalty_ms: 0,
        timeout_ms: grant.timeout_ms,
    }
}

fn emit_active_probe_snapshot() {
    let snapshot = get_active_probe_queue_snapshot();
    let entries = snapshot
        .pending
        .into_iter()
        .chain(snapshot.running)
        .chain(snapshot.recent)
        .collect::<Vec<_>>();
    if !entries.is_empty() {
        emit_active_probe_queue_event(&ActiveProbeQueueEventPayload { entries });
    }
}

fn prune_metadata(snapshot: &ActiveProbeQueueSnapshot) {
    let retained = snapshot
        .pending
        .iter()
        .chain(snapshot.running.iter())
        .chain(snapshot.recent.iter())
        .map(|entry| entry.request_id.as_str())
        .collect::<std::collections::HashSet<_>>();
    let mut state = metadata_state()
        .lock()
        .expect("active probe metadata poisoned");
    state
        .requests
        .retain(|request_id, _| retained.contains(request_id.as_str()));
}

pub fn enqueue_active_probe(
    request: ActiveProbeRequest,
) -> Result<oneshot::Receiver<Result<ActiveProbeDispatchGrant, String>>, String> {
    let schedule = build_schedule(&request)?;
    let plugin_rx = enqueue_plugin_request(schedule, active_probe_policy())?;

    metadata_state()
        .lock()
        .expect("active probe metadata poisoned")
        .requests
        .insert(request.request_id.clone(), request.clone());
    emit_active_probe_snapshot();

    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        let mapped = match plugin_rx.await {
            Ok(Ok(grant)) => Ok(map_grant(grant)),
            Ok(Err(error)) => Err(error),
            Err(_) => Err("Active probe scheduler dropped dispatch grant".to_string()),
        };
        let _ = tx.send(mapped);
    });

    Ok(rx)
}

pub fn mark_active_probe_running(request_id: &str) -> Option<ActiveProbeQueueEntry> {
    let entry = mark_plugin_request_running(PluginFetchPolicyKind::TrafficActiveProbe, request_id);
    let snapshot = get_active_probe_queue_snapshot();
    let mapped = snapshot
        .running
        .into_iter()
        .find(|entry| entry.request_id == request_id)
        .or_else(|| {
            entry.map(|entry| {
                let metadata = metadata_state()
                    .lock()
                    .expect("active probe metadata poisoned")
                    .requests
                    .get(request_id)
                    .cloned();
                map_entry(&entry, metadata.as_ref())
            })
        });
    emit_active_probe_snapshot();
    mapped
}

pub fn complete_active_probe(
    request_id: &str,
    status: Option<u16>,
    response_elapsed_ms: Option<u64>,
) -> Option<ActiveProbeQueueEntry> {
    let entry = complete_plugin_request(
        PluginFetchPolicyKind::TrafficActiveProbe,
        request_id,
        status,
        response_elapsed_ms,
    );
    let mapped = entry.map(|entry| {
        let metadata = metadata_state()
            .lock()
            .expect("active probe metadata poisoned")
            .requests
            .get(request_id)
            .cloned();
        map_entry(&entry, metadata.as_ref())
    });
    emit_active_probe_snapshot();
    mapped
}

pub fn fail_active_probe(
    request_id: &str,
    status: Option<u16>,
    error: Option<String>,
    response_elapsed_ms: Option<u64>,
) -> Option<ActiveProbeQueueEntry> {
    let entry = fail_plugin_request(
        PluginFetchPolicyKind::TrafficActiveProbe,
        request_id,
        status,
        error,
        response_elapsed_ms,
    );
    let mapped = entry.map(|entry| {
        let metadata = metadata_state()
            .lock()
            .expect("active probe metadata poisoned")
            .requests
            .get(request_id)
            .cloned();
        map_entry(&entry, metadata.as_ref())
    });
    emit_active_probe_snapshot();
    mapped
}

pub fn cancel_active_probe(
    request_id: &str,
    reason: Option<String>,
) -> Option<ActiveProbeQueueEntry> {
    let entry = cancel_plugin_request(
        PluginFetchPolicyKind::TrafficActiveProbe,
        request_id,
        reason,
    );
    let mapped = entry.map(|entry| {
        let metadata = metadata_state()
            .lock()
            .expect("active probe metadata poisoned")
            .requests
            .get(request_id)
            .cloned();
        map_entry(&entry, metadata.as_ref())
    });
    emit_active_probe_snapshot();
    mapped
}

pub fn get_active_probe_queue_snapshot() -> ActiveProbeQueueSnapshot {
    let snapshot = get_plugin_request_queue_snapshot(PluginFetchPolicyKind::TrafficActiveProbe);
    let metadata = metadata_state()
        .lock()
        .expect("active probe metadata poisoned")
        .requests
        .clone();

    let active_snapshot = ActiveProbeQueueSnapshot {
        pending: snapshot
            .pending
            .iter()
            .map(|entry| map_entry(entry, metadata.get(&entry.request_id)))
            .collect(),
        running: snapshot
            .running
            .iter()
            .map(|entry| map_entry(entry, metadata.get(&entry.request_id)))
            .collect(),
        recent: snapshot
            .recent
            .iter()
            .map(|entry| map_entry(entry, metadata.get(&entry.request_id)))
            .collect(),
    };

    prune_metadata(&active_snapshot);
    active_snapshot
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(id: &str, traffic_request_id: &str) -> ActiveProbeRequest {
        ActiveProbeRequest {
            plugin_id: "traffic-plugin".to_string(),
            traffic_request_id: traffic_request_id.to_string(),
            request_id: id.to_string(),
            method: "GET".to_string(),
            url: "https://example.com/search?q=test".to_string(),
            probe_label: Some("probe".to_string()),
            target_name: Some("q".to_string()),
            target_path: Some("q".to_string()),
            target_location: Some("query".to_string()),
            probe_value: Some("'".to_string()),
            technique: Some("error-based".to_string()),
            probe_class: "fast".to_string(),
            probe_priority: 0,
            cooldown_key: "example.com/search".to_string(),
            jitter_range: [0, 0],
            min_host_cooldown_ms: 0,
            max_concurrent_per_host: 2,
        }
    }

    #[tokio::test]
    async fn active_probe_uses_unified_scheduler_snapshot() {
        let id = format!("active-probe-test-{}", uuid::Uuid::new_v4());
        let run_id = format!("traffic-{}", uuid::Uuid::new_v4());
        let rx = enqueue_active_probe(request(&id, &run_id)).expect("request should enqueue");
        let grant = rx
            .await
            .expect("grant channel should resolve")
            .expect("grant should succeed");
        assert!(grant.timeout_ms > 0);

        mark_active_probe_running(&id);
        let snapshot = get_active_probe_queue_snapshot();
        assert!(snapshot
            .running
            .iter()
            .any(|entry| entry.request_id == id && entry.probe_label.as_deref() == Some("probe")));

        complete_active_probe(&id, Some(200), Some(10));
    }
}
