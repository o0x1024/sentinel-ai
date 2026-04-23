use crate::runtime_events::emit_active_probe_queue_event;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};
use tokio::sync::oneshot;

const SLOW_PROBE_EXTRA_COOLDOWN_MS: u64 = 1_500;
const ADAPTIVE_PENALTY_MAX_MS: u64 = 15_000;
const ACTIVE_PROBE_RECENT_LIMIT: usize = 50;

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
}

#[derive(Debug)]
struct QueuedWaiter {
    request: ActiveProbeRequest,
    sequence: u64,
    sender: Option<oneshot::Sender<ActiveProbeDispatchGrant>>,
}

#[derive(Debug, Default)]
struct ActiveProbeGroupState {
    active_slots: u32,
    next_dispatch_at: Option<DateTime<Utc>>,
    adaptive_penalty_ms: u64,
    waiters: Vec<QueuedWaiter>,
}

#[derive(Debug, Default)]
struct ActiveProbeSchedulerState {
    entries: HashMap<String, ActiveProbeQueueEntry>,
    groups: HashMap<String, ActiveProbeGroupState>,
    recent_ids: VecDeque<String>,
    sequence: u64,
}

static ACTIVE_PROBE_SCHEDULER: OnceLock<Mutex<ActiveProbeSchedulerState>> = OnceLock::new();

fn scheduler_state() -> &'static Mutex<ActiveProbeSchedulerState> {
    ACTIVE_PROBE_SCHEDULER.get_or_init(|| Mutex::new(ActiveProbeSchedulerState::default()))
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

fn parse_time_or_min(raw: Option<&str>) -> DateTime<Utc> {
    raw.and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&Utc))
        .unwrap_or(DateTime::<Utc>::MIN_UTC)
}

fn jitter_ms(range: [u64; 2]) -> u64 {
    let lower = range[0].min(range[1]);
    let upper = range[0].max(range[1]);
    if upper <= lower {
        return lower;
    }

    let span = upper - lower + 1;
    lower + (rand::random::<u64>() % span)
}

fn sort_waiters(group: &mut ActiveProbeGroupState) {
    group.waiters.sort_by(|left, right| {
        if right.request.probe_priority != left.request.probe_priority {
            return right.request.probe_priority.cmp(&left.request.probe_priority);
        }
        left.sequence.cmp(&right.sequence)
    });
}

fn effective_concurrency(request: &ActiveProbeRequest) -> u32 {
    if request.probe_class == "slow" {
        return 1;
    }

    request.max_concurrent_per_host.max(1)
}

fn effective_cooldown_ms(request: &ActiveProbeRequest, adaptive_penalty_ms: u64) -> u64 {
    request.min_host_cooldown_ms
        + adaptive_penalty_ms
        + if request.probe_class == "slow" {
            SLOW_PROBE_EXTRA_COOLDOWN_MS
        } else {
            0
        }
}

fn build_entry(request: &ActiveProbeRequest, adaptive_penalty_ms: u64, queue_depth: u32) -> ActiveProbeQueueEntry {
    let now = now_rfc3339();
    ActiveProbeQueueEntry {
        plugin_id: request.plugin_id.clone(),
        traffic_request_id: request.traffic_request_id.clone(),
        request_id: request.request_id.clone(),
        phase: ActiveProbeQueuePhase::Queued,
        method: request.method.clone(),
        url: request.url.clone(),
        probe_label: request.probe_label.clone(),
        target_name: request.target_name.clone(),
        target_path: request.target_path.clone(),
        target_location: request.target_location.clone(),
        probe_value: request.probe_value.clone(),
        technique: request.technique.clone(),
        probe_class: request.probe_class.clone(),
        probe_priority: request.probe_priority,
        cooldown_key: request.cooldown_key.clone(),
        cooldown_wait_ms: None,
        jitter_wait_ms: None,
        total_wait_ms: None,
        adaptive_penalty_ms,
        status: None,
        error: None,
        reason: None,
        active_slots: 0,
        max_concurrent_per_host: effective_concurrency(request),
        queue_depth,
        response_elapsed_ms: None,
        queued_at: now.clone(),
        scheduled_at: None,
        dispatch_started_at: None,
        finished_at: None,
        updated_at: now,
    }
}

fn emit_entry(entry: &ActiveProbeQueueEntry) {
    emit_active_probe_queue_event(entry);
}

fn emit_entries(entries: Vec<ActiveProbeQueueEntry>) {
    for entry in entries {
        emit_entry(&entry);
    }
}

fn refresh_waiter_entries_locked(
    state: &mut ActiveProbeSchedulerState,
    cooldown_key: &str,
) -> Vec<ActiveProbeQueueEntry> {
    let mut updates = Vec::new();
    let Some(group) = state.groups.get_mut(cooldown_key) else {
        return updates;
    };

    sort_waiters(group);
    for (index, waiter) in group.waiters.iter().enumerate() {
        let Some(entry) = state.entries.get_mut(&waiter.request.request_id) else {
            continue;
        };

        if entry.phase != ActiveProbeQueuePhase::Queued {
            continue;
        }

        entry.queue_depth = index as u32;
        entry.active_slots = group.active_slots;
        entry.max_concurrent_per_host = effective_concurrency(&waiter.request);
        entry.adaptive_penalty_ms = group.adaptive_penalty_ms;
        entry.updated_at = now_rfc3339();
        updates.push(entry.clone());
    }

    updates
}

fn prune_recent_locked(state: &mut ActiveProbeSchedulerState) {
    while state.recent_ids.len() > ACTIVE_PROBE_RECENT_LIMIT {
        let Some(oldest_id) = state.recent_ids.pop_back() else {
            break;
        };
        let can_remove = state
            .entries
            .get(&oldest_id)
            .map(|entry| {
                matches!(
                    entry.phase,
                    ActiveProbeQueuePhase::Completed
                        | ActiveProbeQueuePhase::Failed
                        | ActiveProbeQueuePhase::Cancelled
                )
            })
            .unwrap_or(false);
        if can_remove {
            state.entries.remove(&oldest_id);
        }
    }
}

fn enqueue_recent_locked(state: &mut ActiveProbeSchedulerState, request_id: &str) {
    if let Some(position) = state.recent_ids.iter().position(|existing| existing == request_id) {
        state.recent_ids.remove(position);
    }
    state.recent_ids.push_front(request_id.to_string());
    prune_recent_locked(state);
}

fn promote_waiters_locked(
    state: &mut ActiveProbeSchedulerState,
    cooldown_key: &str,
) -> Vec<ActiveProbeQueueEntry> {
    let mut updates = Vec::new();
    let mut grants = Vec::new();

    {
        let Some(group) = state.groups.get_mut(cooldown_key) else {
            return updates;
        };

        sort_waiters(group);
        loop {
            let Some(next_waiter) = group.waiters.first() else {
                break;
            };

            let max_concurrent_per_host = effective_concurrency(&next_waiter.request);
            if group.active_slots >= max_concurrent_per_host {
                break;
            }

            let mut waiter = group.waiters.remove(0);
            let now = Utc::now();
            let adaptive_penalty_ms = group.adaptive_penalty_ms;
            let jitter_wait_ms = jitter_ms(waiter.request.jitter_range);
            let next_dispatch_at = group.next_dispatch_at.unwrap_or(now);
            let cooldown_wait_ms = next_dispatch_at
                .signed_duration_since(now)
                .num_milliseconds()
                .max(0) as u64;
            let total_wait_ms = cooldown_wait_ms + jitter_wait_ms;
            let scheduled_at = now_rfc3339();
            let dispatch_at = now
                + chrono::Duration::milliseconds(total_wait_ms as i64);

            group.active_slots += 1;
            group.next_dispatch_at = Some(
                dispatch_at
                    + chrono::Duration::milliseconds(
                        effective_cooldown_ms(&waiter.request, adaptive_penalty_ms) as i64,
                    ),
            );

            let grant = ActiveProbeDispatchGrant {
                active_slots: group.active_slots,
                max_concurrent_per_host,
                queue_depth: 0,
                cooldown_wait_ms,
                jitter_wait_ms,
                total_wait_ms,
                adaptive_penalty_ms,
            };

            if let Some(entry) = state.entries.get_mut(&waiter.request.request_id) {
                entry.phase = ActiveProbeQueuePhase::Scheduled;
                entry.active_slots = grant.active_slots;
                entry.max_concurrent_per_host = grant.max_concurrent_per_host;
                entry.queue_depth = grant.queue_depth;
                entry.cooldown_wait_ms = Some(grant.cooldown_wait_ms);
                entry.jitter_wait_ms = Some(grant.jitter_wait_ms);
                entry.total_wait_ms = Some(grant.total_wait_ms);
                entry.adaptive_penalty_ms = adaptive_penalty_ms;
                entry.scheduled_at = Some(scheduled_at.clone());
                entry.updated_at = scheduled_at;
                updates.push(entry.clone());
            }

            grants.push((waiter.sender.take(), grant));
            sort_waiters(group);
        }
    }

    updates.extend(refresh_waiter_entries_locked(state, cooldown_key));

    for (sender, grant) in grants {
        if let Some(sender) = sender {
            let _ = sender.send(grant);
        }
    }

    updates
}

pub fn enqueue_active_probe(
    request: ActiveProbeRequest,
) -> oneshot::Receiver<ActiveProbeDispatchGrant> {
    let (tx, rx) = oneshot::channel();
    let mut state = scheduler_state()
        .lock()
        .expect("active probe scheduler poisoned");

    let adaptive_penalty_ms = state
        .groups
        .get(&request.cooldown_key)
        .map(|group| group.adaptive_penalty_ms)
        .unwrap_or(0);
    let entry = build_entry(
        &request,
        adaptive_penalty_ms,
        state
            .groups
            .get(&request.cooldown_key)
            .map(|group| group.waiters.len() as u32)
            .unwrap_or(0),
    );
    state.entries.insert(request.request_id.clone(), entry.clone());

    let sequence = state.sequence;
    state.sequence += 1;

    let group = state
        .groups
        .entry(request.cooldown_key.clone())
        .or_default();
    group.waiters.push(QueuedWaiter {
        request: request.clone(),
        sequence,
        sender: Some(tx),
    });

    let mut updates = vec![entry];
    updates.extend(refresh_waiter_entries_locked(&mut state, &request.cooldown_key));
    updates.extend(promote_waiters_locked(&mut state, &request.cooldown_key));
    drop(state);

    emit_entries(updates);
    rx
}

pub fn mark_active_probe_running(request_id: &str) -> Option<ActiveProbeQueueEntry> {
    let mut state = scheduler_state()
        .lock()
        .expect("active probe scheduler poisoned");
    let entry = state.entries.get_mut(request_id)?;
    entry.phase = ActiveProbeQueuePhase::Running;
    entry.dispatch_started_at = Some(now_rfc3339());
    entry.updated_at = entry.dispatch_started_at.clone().unwrap_or_else(now_rfc3339);
    let entry = entry.clone();
    drop(state);
    emit_entry(&entry);
    Some(entry)
}

fn adjust_penalty(group: &mut ActiveProbeGroupState, status: Option<u16>, error: Option<&str>, response_elapsed_ms: Option<u64>) {
    let status = status.unwrap_or_default();
    let error_text = error.unwrap_or("").to_ascii_lowercase();
    let elapsed = response_elapsed_ms.unwrap_or_default();
    let is_hard_failure = status == 429
        || status >= 500
        || error_text.contains("timeout")
        || error_text.contains("aborted")
        || !error_text.is_empty()
        || elapsed >= 7_500;

    if is_hard_failure {
        group.adaptive_penalty_ms = std::cmp::min(
            ADAPTIVE_PENALTY_MAX_MS,
            std::cmp::max(
                400,
                if group.adaptive_penalty_ms > 0 {
                    group.adaptive_penalty_ms * 2
                } else {
                    400
                },
            ),
        );
        return;
    }

    group.adaptive_penalty_ms = group.adaptive_penalty_ms / 2;
}

fn finalize_active_probe(
    request_id: &str,
    phase: ActiveProbeQueuePhase,
    status: Option<u16>,
    error: Option<String>,
    response_elapsed_ms: Option<u64>,
) -> Option<ActiveProbeQueueEntry> {
    let mut state = scheduler_state()
        .lock()
        .expect("active probe scheduler poisoned");
    let cooldown_key = state.entries.get(request_id)?.cooldown_key.clone();

    let mut updates = Vec::new();
    {
      let group = state.groups.get_mut(&cooldown_key)?;
      group.active_slots = group.active_slots.saturating_sub(1);
      adjust_penalty(group, status, error.as_deref(), response_elapsed_ms);
    }

    let final_entry = {
        let entry = state.entries.get_mut(request_id)?;
        let now = now_rfc3339();
        entry.phase = phase;
        entry.status = status;
        entry.error = error;
        entry.response_elapsed_ms = response_elapsed_ms;
        entry.finished_at = Some(now.clone());
        entry.updated_at = now;
        entry.active_slots = state
            .groups
            .get(&cooldown_key)
            .map(|group| group.active_slots)
            .unwrap_or(0);
        entry.adaptive_penalty_ms = state
            .groups
            .get(&cooldown_key)
            .map(|group| group.adaptive_penalty_ms)
            .unwrap_or(0);
        entry.clone()
    };

    enqueue_recent_locked(&mut state, request_id);
    updates.push(final_entry.clone());
    updates.extend(refresh_waiter_entries_locked(&mut state, &cooldown_key));
    updates.extend(promote_waiters_locked(&mut state, &cooldown_key));
    if let Some(group) = state.groups.get(&cooldown_key) {
        if group.active_slots == 0 && group.waiters.is_empty() {
            state.groups.remove(&cooldown_key);
        }
    }
    drop(state);

    emit_entries(updates);
    Some(final_entry)
}

pub fn complete_active_probe(
    request_id: &str,
    status: Option<u16>,
    response_elapsed_ms: Option<u64>,
) -> Option<ActiveProbeQueueEntry> {
    finalize_active_probe(
        request_id,
        ActiveProbeQueuePhase::Completed,
        status,
        None,
        response_elapsed_ms,
    )
}

pub fn fail_active_probe(
    request_id: &str,
    status: Option<u16>,
    error: Option<String>,
    response_elapsed_ms: Option<u64>,
) -> Option<ActiveProbeQueueEntry> {
    finalize_active_probe(
        request_id,
        ActiveProbeQueuePhase::Failed,
        status,
        error,
        response_elapsed_ms,
    )
}

pub fn cancel_active_probe(request_id: &str, reason: Option<String>) -> Option<ActiveProbeQueueEntry> {
    let mut state = scheduler_state()
        .lock()
        .expect("active probe scheduler poisoned");
    let entry = state.entries.get(request_id)?.clone();
    let cooldown_key = entry.cooldown_key.clone();
    let mut updates = Vec::new();
    let mut removed_waiter = false;

    if let Some(group) = state.groups.get_mut(&cooldown_key) {
        if let Some(index) = group
            .waiters
            .iter()
            .position(|waiter| waiter.request.request_id == request_id)
        {
            group.waiters.remove(index);
            removed_waiter = true;
        } else if matches!(
            entry.phase,
            ActiveProbeQueuePhase::Scheduled | ActiveProbeQueuePhase::Running
        ) {
            group.active_slots = group.active_slots.saturating_sub(1);
        }
    }

    {
        let entry = state.entries.get_mut(request_id)?;
        let now = now_rfc3339();
        entry.phase = ActiveProbeQueuePhase::Cancelled;
        entry.reason = reason;
        entry.finished_at = Some(now.clone());
        entry.updated_at = now;
        entry.active_slots = state
            .groups
            .get(&cooldown_key)
            .map(|group| group.active_slots)
            .unwrap_or(0);
        updates.push(entry.clone());
    }

    enqueue_recent_locked(&mut state, request_id);
    if removed_waiter {
        updates.extend(refresh_waiter_entries_locked(&mut state, &cooldown_key));
    }
    updates.extend(promote_waiters_locked(&mut state, &cooldown_key));
    if let Some(group) = state.groups.get(&cooldown_key) {
        if group.active_slots == 0 && group.waiters.is_empty() {
            state.groups.remove(&cooldown_key);
        }
    }
    drop(state);

    emit_entries(updates.clone());
    updates.into_iter().find(|item| item.request_id == request_id)
}

pub fn get_active_probe_queue_snapshot() -> ActiveProbeQueueSnapshot {
    let state = scheduler_state()
        .lock()
        .expect("active probe scheduler poisoned");

    let mut pending = state
        .entries
        .values()
        .filter(|entry| {
            matches!(
                entry.phase,
                ActiveProbeQueuePhase::Queued | ActiveProbeQueuePhase::Scheduled
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    pending.sort_by(|left, right| {
        left.queue_depth
            .cmp(&right.queue_depth)
            .then_with(|| parse_time_or_min(Some(left.updated_at.as_str())).cmp(&parse_time_or_min(Some(right.updated_at.as_str()))))
    });

    let mut running = state
        .entries
        .values()
        .filter(|entry| entry.phase == ActiveProbeQueuePhase::Running)
        .cloned()
        .collect::<Vec<_>>();
    running.sort_by(|left, right| {
        parse_time_or_min(Some(right.updated_at.as_str()))
            .cmp(&parse_time_or_min(Some(left.updated_at.as_str())))
    });

    let recent = state
        .recent_ids
        .iter()
        .filter_map(|request_id| state.entries.get(request_id).cloned())
        .collect::<Vec<_>>();

    ActiveProbeQueueSnapshot {
        pending,
        running,
        recent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_request(id: &str, priority: i32) -> ActiveProbeRequest {
        ActiveProbeRequest {
            plugin_id: "traffic-plugin".to_string(),
            traffic_request_id: "traffic-1".to_string(),
            request_id: id.to_string(),
            method: "GET".to_string(),
            url: "https://example.com/items?id=1".to_string(),
            probe_label: Some("sql-injection".to_string()),
            target_name: Some("id".to_string()),
            target_path: Some("query:id".to_string()),
            target_location: Some("query".to_string()),
            probe_value: Some("1'".to_string()),
            technique: Some("error-based".to_string()),
            probe_class: "fast".to_string(),
            probe_priority: priority,
            cooldown_key: "example.com/items".to_string(),
            jitter_range: [0, 0],
            min_host_cooldown_ms: 0,
            max_concurrent_per_host: 1,
        }
    }

    #[tokio::test]
    async fn scheduler_respects_priority_and_snapshot() {
        let first_rx = enqueue_active_probe(build_request("req-1", 10));
        let second_rx = enqueue_active_probe(build_request("req-2", 100));

        let first_grant = first_rx.await.expect("first grant");
        assert_eq!(first_grant.active_slots, 1);
        mark_active_probe_running("req-1");

        let snapshot = get_active_probe_queue_snapshot();
        assert_eq!(snapshot.pending.len(), 1);
        assert_eq!(snapshot.running.len(), 1);
        assert_eq!(snapshot.pending[0].request_id, "req-2");

        complete_active_probe("req-1", Some(200), Some(50));
        let second_grant = second_rx.await.expect("second grant");
        assert_eq!(second_grant.active_slots, 1);
        mark_active_probe_running("req-2");

        let snapshot = get_active_probe_queue_snapshot();
        assert!(snapshot.pending.is_empty());
        assert_eq!(snapshot.running[0].request_id, "req-2");
        assert_eq!(snapshot.recent[0].request_id, "req-1");
    }
}
