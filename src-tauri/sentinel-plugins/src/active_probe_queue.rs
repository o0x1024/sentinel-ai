use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Mutex, OnceLock};
use tokio::sync::oneshot;

use crate::runtime_config::get_plugin_runtime_settings;

const RECENT_LIMIT: usize = 100;
const ADAPTIVE_PENALTY_MAX_MS: u64 = 15_000;
const RECENT_WINDOW_SECONDS: i64 = 60;
const TERMINAL_SAMPLE_LIMIT: usize = 2_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActiveProbeQueuePhase {
    Queued,
    Scheduled,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalSamplePhase {
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct ActiveProbePolicy {
    pub max_queue_depth: usize,
    pub max_pending_per_run: usize,
    pub max_pending_per_plugin: usize,
    pub max_global_concurrent: u32,
    pub max_concurrent_per_host: u32,
    pub max_concurrent_per_run: u32,
    pub max_concurrent_per_plugin: u32,
    pub min_host_delay_ms: u64,
    pub jitter_range: [u64; 2],
    pub timeout_ms: u64,
}

pub fn configured_active_probe_policy() -> ActiveProbePolicy {
    let settings = get_plugin_runtime_settings().active_probe;
    ActiveProbePolicy {
        max_queue_depth: settings.max_queue_depth as usize,
        max_pending_per_run: settings.max_pending_per_run as usize,
        max_pending_per_plugin: settings.max_pending_per_plugin as usize,
        max_global_concurrent: settings.max_global_concurrent as u32,
        max_concurrent_per_host: settings.max_concurrent_per_host as u32,
        max_concurrent_per_run: settings.max_concurrent_per_run as u32,
        max_concurrent_per_plugin: settings.max_concurrent_per_plugin as u32,
        min_host_delay_ms: settings.min_host_cooldown_ms,
        jitter_range: settings.jitter_range,
        timeout_ms: settings.timeout_ms,
    }
}

#[derive(Debug, Clone)]
pub struct ActiveProbeScheduleRequest {
    pub request_id: String,
    pub run_id: String,
    pub plugin_id: String,
    pub execution_context: String,
    pub method: String,
    pub url: String,
    pub host: String,
}

#[derive(Debug, Clone)]
pub struct ActiveProbeDispatchGrant {
    pub cooldown_wait_ms: u64,
    pub jitter_wait_ms: u64,
    pub total_wait_ms: u64,
    pub timeout_ms: u64,
    pub active_global: u32,
    pub active_for_host: u32,
    pub active_for_run: u32,
    pub active_for_plugin: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveProbeQueueEntry {
    pub request_id: String,
    pub run_id: String,
    pub plugin_id: String,
    pub execution_context: String,
    pub phase: ActiveProbeQueuePhase,
    pub method: String,
    pub url: String,
    pub host: String,
    pub queue_depth: u32,
    pub status: Option<u16>,
    pub error: Option<String>,
    pub reason: Option<String>,
    pub cooldown_wait_ms: Option<u64>,
    pub jitter_wait_ms: Option<u64>,
    pub total_wait_ms: Option<u64>,
    pub adaptive_penalty_ms: u64,
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
#[serde(rename_all = "camelCase")]
pub struct ActiveProbeQueueStats {
    pub pending_count: u32,
    pub queued_count: u32,
    pub scheduled_count: u32,
    pub running_count: u32,
    pub recent_count: u32,
    pub max_queue_depth: u32,
    pub active_global: u32,
    pub active_runs: u32,
    pub active_plugins: u32,
    pub active_hosts: u32,
    pub hottest_host: Option<String>,
    pub hottest_host_active: u32,
    pub cancelled_run_count: u32,
    pub configured_max_queue_depth: u32,
    pub configured_max_global_concurrent: u32,
    pub configured_max_concurrent_per_host: u32,
    pub configured_max_concurrent_per_run: u32,
    pub configured_max_concurrent_per_plugin: u32,
    pub rejected_total_count: u32,
    pub rejected_cancelled_run_count: u32,
    pub rejected_queue_limit_count: u32,
    pub rejected_run_pending_limit_count: u32,
    pub rejected_plugin_pending_limit_count: u32,
    pub recent_window_total_count: u32,
    pub recent_window_completed_count: u32,
    pub recent_window_failed_count: u32,
    pub recent_window_cancelled_count: u32,
    pub recent_window_avg_queue_wait_ms: u64,
    pub recent_window_avg_response_elapsed_ms: u64,
}

#[derive(Debug, Default)]
struct RejectionStats {
    cancelled_run: u32,
    queue_limit: u32,
    run_pending_limit: u32,
    plugin_pending_limit: u32,
}

#[derive(Debug, Clone)]
struct TerminalSample {
    finished_at: DateTime<Utc>,
    phase: TerminalSamplePhase,
    queue_wait_ms: Option<u64>,
    response_elapsed_ms: Option<u64>,
}

#[derive(Debug)]
struct QueuedWaiter {
    request: ActiveProbeScheduleRequest,
    sequence: u64,
    sender: Option<oneshot::Sender<Result<ActiveProbeDispatchGrant, String>>>,
}

#[derive(Debug, Default)]
struct HostState {
    active_slots: u32,
    next_dispatch_at: Option<DateTime<Utc>>,
    adaptive_penalty_ms: u64,
}

#[derive(Debug, Default)]
struct QueueState {
    entries: HashMap<String, ActiveProbeQueueEntry>,
    waiters: Vec<QueuedWaiter>,
    run_order: VecDeque<String>,
    recent_ids: VecDeque<String>,
    cancelled_runs: HashSet<String>,
    hosts: HashMap<String, HostState>,
    active_global: u32,
    active_by_run: HashMap<String, u32>,
    active_by_plugin: HashMap<String, u32>,
    rejection_stats: RejectionStats,
    terminal_samples: VecDeque<TerminalSample>,
    sequence: u64,
}

#[derive(Debug)]
struct ScheduledWaiter {
    grant: ActiveProbeDispatchGrant,
    sender: Option<oneshot::Sender<Result<ActiveProbeDispatchGrant, String>>>,
}

static ACTIVE_PROBE_QUEUE: OnceLock<Mutex<QueueState>> = OnceLock::new();

fn queue_state() -> &'static Mutex<QueueState> {
    ACTIVE_PROBE_QUEUE.get_or_init(|| Mutex::new(QueueState::default()))
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

fn parse_rfc3339(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|parsed| parsed.with_timezone(&Utc))
}

fn jitter_ms(range: [u64; 2]) -> u64 {
    let lower = range[0].min(range[1]);
    let upper = range[0].max(range[1]);
    if upper <= lower {
        return lower;
    }

    let span = upper - lower + 1;
    let seed = Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_default()
        .unsigned_abs();
    lower + (seed % span)
}

fn is_pending_phase(phase: ActiveProbeQueuePhase) -> bool {
    matches!(
        phase,
        ActiveProbeQueuePhase::Queued
            | ActiveProbeQueuePhase::Scheduled
            | ActiveProbeQueuePhase::Running
    )
}

fn pending_counts(
    state: &QueueState,
    run_id: &str,
    plugin_id: &str,
) -> (usize, usize, usize) {
    let mut total = 0;
    let mut run = 0;
    let mut plugin = 0;
    for entry in state.entries.values() {
        if !is_pending_phase(entry.phase) {
            continue;
        }
        total += 1;
        if entry.run_id == run_id {
            run += 1;
        }
        if entry.plugin_id == plugin_id {
            plugin += 1;
        }
    }
    (total, run, plugin)
}

fn active_count(map: &HashMap<String, u32>, key: &str) -> u32 {
    map.get(key).copied().unwrap_or(0)
}

fn can_schedule(
    state: &QueueState,
    policy: &ActiveProbePolicy,
    request: &ActiveProbeScheduleRequest,
) -> bool {
    if state.active_global >= policy.max_global_concurrent.max(1) {
        return false;
    }
    if active_count(&state.active_by_run, &request.run_id) >= policy.max_concurrent_per_run.max(1)
    {
        return false;
    }
    if active_count(&state.active_by_plugin, &request.plugin_id)
        >= policy.max_concurrent_per_plugin.max(1)
    {
        return false;
    }
    let host_active = state
        .hosts
        .get(&request.host)
        .map(|host| host.active_slots)
        .unwrap_or(0);
    host_active < policy.max_concurrent_per_host.max(1)
}

fn build_entry(
    request: &ActiveProbeScheduleRequest,
    adaptive_penalty_ms: u64,
    queue_depth: u32,
) -> ActiveProbeQueueEntry {
    let now = now_rfc3339();
    ActiveProbeQueueEntry {
        request_id: request.request_id.clone(),
        run_id: request.run_id.clone(),
        plugin_id: request.plugin_id.clone(),
        execution_context: request.execution_context.clone(),
        phase: ActiveProbeQueuePhase::Queued,
        method: request.method.clone(),
        url: request.url.clone(),
        host: request.host.clone(),
        queue_depth,
        status: None,
        error: None,
        reason: None,
        cooldown_wait_ms: None,
        jitter_wait_ms: None,
        total_wait_ms: None,
        adaptive_penalty_ms,
        response_elapsed_ms: None,
        queued_at: now.clone(),
        scheduled_at: None,
        dispatch_started_at: None,
        finished_at: None,
        updated_at: now,
    }
}

fn refresh_queue_depths(state: &mut QueueState) {
    let mut depths_by_run: HashMap<String, u32> = HashMap::new();
    let mut queued: Vec<_> = state
        .waiters
        .iter()
        .map(|waiter| {
            (
                waiter.sequence,
                waiter.request.request_id.clone(),
                waiter.request.run_id.clone(),
            )
        })
        .collect();
    queued.sort_by_key(|(sequence, _, _)| *sequence);

    for (_, request_id, run_id) in queued {
        let depth = depths_by_run.entry(run_id).or_insert(0);
        if let Some(entry) = state.entries.get_mut(&request_id) {
            entry.queue_depth = *depth;
            entry.updated_at = now_rfc3339();
        }
        *depth += 1;
    }
}

fn apply_active_increment(state: &mut QueueState, request: &ActiveProbeScheduleRequest) {
    state.active_global += 1;
    *state
        .active_by_run
        .entry(request.run_id.clone())
        .or_insert(0) += 1;
    *state
        .active_by_plugin
        .entry(request.plugin_id.clone())
        .or_insert(0) += 1;
    state
        .hosts
        .entry(request.host.clone())
        .or_default()
        .active_slots += 1;
}

fn apply_active_decrement(state: &mut QueueState, entry: &ActiveProbeQueueEntry) {
    state.active_global = state.active_global.saturating_sub(1);
    decrement_map(&mut state.active_by_run, &entry.run_id);
    decrement_map(&mut state.active_by_plugin, &entry.plugin_id);
    if let Some(host) = state.hosts.get_mut(&entry.host) {
        host.active_slots = host.active_slots.saturating_sub(1);
    }
}

fn decrement_map(map: &mut HashMap<String, u32>, key: &str) {
    if let Some(value) = map.get_mut(key) {
        *value = value.saturating_sub(1);
        if *value == 0 {
            map.remove(key);
        }
    }
}

fn prune_recent(state: &mut QueueState) {
    while state.recent_ids.len() > RECENT_LIMIT {
        let Some(oldest_id) = state.recent_ids.pop_back() else {
            break;
        };
        let can_remove = state
            .entries
            .get(&oldest_id)
            .map(|entry| !is_pending_phase(entry.phase))
            .unwrap_or(false);
        if can_remove {
            state.entries.remove(&oldest_id);
        }
    }
}

fn enqueue_recent(state: &mut QueueState, request_id: &str) {
    if let Some(position) = state
        .recent_ids
        .iter()
        .position(|existing| existing == request_id)
    {
        state.recent_ids.remove(position);
    }
    state.recent_ids.push_front(request_id.to_string());
    prune_recent(state);
}

fn enqueue_terminal_sample(state: &mut QueueState, entry: &ActiveProbeQueueEntry) {
    let Some(finished_at) = entry.finished_at.as_deref().and_then(parse_rfc3339) else {
        return;
    };

    let phase = match entry.phase {
        ActiveProbeQueuePhase::Completed => TerminalSamplePhase::Completed,
        ActiveProbeQueuePhase::Failed => TerminalSamplePhase::Failed,
        ActiveProbeQueuePhase::Cancelled => TerminalSamplePhase::Cancelled,
        _ => return,
    };

    let queue_wait_ms = match (
        parse_rfc3339(&entry.queued_at),
        entry.dispatch_started_at.as_deref().and_then(parse_rfc3339),
    ) {
        (Some(queued_at), Some(dispatch_started_at)) if dispatch_started_at >= queued_at => {
            Some((dispatch_started_at - queued_at).num_milliseconds().max(0) as u64)
        }
        _ => None,
    };

    state.terminal_samples.push_back(TerminalSample {
        finished_at,
        phase,
        queue_wait_ms,
        response_elapsed_ms: entry.response_elapsed_ms,
    });
    prune_terminal_samples(state, Utc::now());
}

fn prune_terminal_samples(state: &mut QueueState, now: DateTime<Utc>) {
    let cutoff = now - chrono::Duration::seconds(RECENT_WINDOW_SECONDS);
    while let Some(sample) = state.terminal_samples.front() {
        if sample.finished_at >= cutoff && state.terminal_samples.len() <= TERMINAL_SAMPLE_LIMIT {
            break;
        }
        state.terminal_samples.pop_front();
    }
}

fn adjust_penalty(
    host: &mut HostState,
    status: Option<u16>,
    error: Option<&str>,
    response_elapsed_ms: Option<u64>,
) {
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
        host.adaptive_penalty_ms = std::cmp::min(
            ADAPTIVE_PENALTY_MAX_MS,
            std::cmp::max(
                400,
                if host.adaptive_penalty_ms > 0 {
                    host.adaptive_penalty_ms * 2
                } else {
                    400
                },
            ),
        );
        return;
    }

    host.adaptive_penalty_ms /= 2;
}

fn promote_waiters_locked(
    state: &mut QueueState,
    policy: &ActiveProbePolicy,
) -> Vec<ScheduledWaiter> {
    let mut scheduled = Vec::new();
    if state.waiters.is_empty() {
        return scheduled;
    }

    loop {
        let run_count = state.run_order.len();
        if run_count == 0 {
            break;
        }

        let mut selected: Option<(usize, String)> = None;
        for _ in 0..run_count {
            let Some(run_id) = state.run_order.pop_front() else {
                break;
            };
            let candidate_index = state
                .waiters
                .iter()
                .position(|waiter| waiter.request.run_id == run_id);
            let should_keep_run = candidate_index.is_some();
            if let Some(index) = candidate_index {
                if can_schedule(state, policy, &state.waiters[index].request) {
                    selected = Some((index, run_id.clone()));
                    break;
                }
            }
            if should_keep_run {
                state.run_order.push_back(run_id);
            }
        }

        let Some((index, run_id)) = selected else {
            break;
        };

        let mut waiter = state.waiters.remove(index);
        if state
            .waiters
            .iter()
            .any(|queued| queued.request.run_id == run_id)
        {
            state.run_order.push_back(run_id);
        }

        let now = Utc::now();
        let (cooldown_wait_ms, jitter_wait_ms, total_wait_ms, adaptive_penalty_ms) = {
            let host = state.hosts.entry(waiter.request.host.clone()).or_default();
            let next_dispatch_at = host.next_dispatch_at.unwrap_or(now);
            let cooldown_wait_ms = next_dispatch_at
                .signed_duration_since(now)
                .num_milliseconds()
                .max(0) as u64;
            let jitter_wait_ms = jitter_ms(policy.jitter_range);
            let adaptive_penalty_ms = host.adaptive_penalty_ms;
            let total_wait_ms = cooldown_wait_ms + jitter_wait_ms;
            host.next_dispatch_at = Some(
                now + chrono::Duration::milliseconds(
                    (total_wait_ms + policy.min_host_delay_ms + adaptive_penalty_ms) as i64,
                ),
            );
            (
                cooldown_wait_ms,
                jitter_wait_ms,
                total_wait_ms,
                adaptive_penalty_ms,
            )
        };

        apply_active_increment(state, &waiter.request);

        let grant = ActiveProbeDispatchGrant {
            cooldown_wait_ms,
            jitter_wait_ms,
            total_wait_ms,
            timeout_ms: policy.timeout_ms,
            active_global: state.active_global,
            active_for_host: state
                .hosts
                .get(&waiter.request.host)
                .map(|host| host.active_slots)
                .unwrap_or(0),
            active_for_run: active_count(&state.active_by_run, &waiter.request.run_id),
            active_for_plugin: active_count(&state.active_by_plugin, &waiter.request.plugin_id),
        };

        let scheduled_at = now_rfc3339();
        if let Some(entry) = state.entries.get_mut(&waiter.request.request_id) {
            entry.phase = ActiveProbeQueuePhase::Scheduled;
            entry.cooldown_wait_ms = Some(cooldown_wait_ms);
            entry.jitter_wait_ms = Some(jitter_wait_ms);
            entry.total_wait_ms = Some(total_wait_ms);
            entry.adaptive_penalty_ms = adaptive_penalty_ms;
            entry.scheduled_at = Some(scheduled_at.clone());
            entry.updated_at = scheduled_at;
        }

        scheduled.push(ScheduledWaiter {
            grant,
            sender: waiter.sender.take(),
        });
    }

    refresh_queue_depths(state);
    scheduled
}

fn dispatch_scheduled(waiters: Vec<ScheduledWaiter>) {
    for waiter in waiters {
        if let Some(sender) = waiter.sender {
            let _ = sender.send(Ok(waiter.grant));
        }
    }
}

pub fn enqueue_request(
    request: ActiveProbeScheduleRequest,
    policy: ActiveProbePolicy,
) -> Result<oneshot::Receiver<Result<ActiveProbeDispatchGrant, String>>, String> {
    let (tx, rx) = oneshot::channel();
    let mut state = queue_state()
        .lock()
        .expect("active probe queue poisoned");
    if state.cancelled_runs.contains(&request.run_id) {
        state.rejection_stats.cancelled_run =
            state.rejection_stats.cancelled_run.saturating_add(1);
        return Err("Active probe run has been cancelled".to_string());
    }
    let (total_pending, run_pending, plugin_pending) =
        pending_counts(&state, &request.run_id, &request.plugin_id);

    if total_pending >= policy.max_queue_depth {
        state.rejection_stats.queue_limit = state.rejection_stats.queue_limit.saturating_add(1);
        return Err("Active probe queue limit exceeded".to_string());
    }
    if run_pending >= policy.max_pending_per_run {
        state.rejection_stats.run_pending_limit = state
            .rejection_stats
            .run_pending_limit
            .saturating_add(1);
        return Err("Active probe run pending limit exceeded".to_string());
    }
    if plugin_pending >= policy.max_pending_per_plugin {
        state.rejection_stats.plugin_pending_limit = state
            .rejection_stats
            .plugin_pending_limit
            .saturating_add(1);
        return Err("Active probe plugin pending limit exceeded".to_string());
    }

    let queue_depth = state.waiters.len() as u32;
    let adaptive_penalty_ms = state
        .hosts
        .get(&request.host)
        .map(|host| host.adaptive_penalty_ms)
        .unwrap_or(0);
    let entry = build_entry(&request, adaptive_penalty_ms, queue_depth);
    state.entries.insert(request.request_id.clone(), entry);

    let sequence = state.sequence;
    state.sequence += 1;
    if !state
        .run_order
        .iter()
        .any(|existing| existing == &request.run_id)
    {
        state.run_order.push_back(request.run_id.clone());
    }
    state.waiters.push(QueuedWaiter {
        request,
        sequence,
        sender: Some(tx),
    });

    let scheduled = promote_waiters_locked(&mut state, &policy);
    drop(state);
    dispatch_scheduled(scheduled);

    Ok(rx)
}

pub fn mark_request_running(request_id: &str) -> Option<ActiveProbeQueueEntry> {
    let mut state = queue_state()
        .lock()
        .expect("active probe queue poisoned");
    let entry = state.entries.get_mut(request_id)?;
    if entry.phase == ActiveProbeQueuePhase::Cancelled {
        return None;
    }
    entry.phase = ActiveProbeQueuePhase::Running;
    entry.dispatch_started_at = Some(now_rfc3339());
    entry.updated_at = entry
        .dispatch_started_at
        .clone()
        .unwrap_or_else(now_rfc3339);
    Some(entry.clone())
}

fn finalize_request(
    request_id: &str,
    phase: ActiveProbeQueuePhase,
    status: Option<u16>,
    error: Option<String>,
    reason: Option<String>,
    response_elapsed_ms: Option<u64>,
) -> Option<ActiveProbeQueueEntry> {
    let policy = configured_active_probe_policy();
    let mut state = queue_state()
        .lock()
        .expect("active probe queue poisoned");
    let mut entry = state.entries.get(request_id)?.clone();

    if matches!(
        entry.phase,
        ActiveProbeQueuePhase::Scheduled | ActiveProbeQueuePhase::Running
    ) {
        apply_active_decrement(&mut state, &entry);
    }

    if let Some(host) = state.hosts.get_mut(&entry.host) {
        adjust_penalty(host, status, error.as_deref(), response_elapsed_ms);
        entry.adaptive_penalty_ms = host.adaptive_penalty_ms;
    }

    let now = now_rfc3339();
    entry.phase = phase;
    entry.status = status;
    entry.error = error;
    entry.reason = reason;
    entry.response_elapsed_ms = response_elapsed_ms;
    entry.finished_at = Some(now.clone());
    entry.updated_at = now;
    state.entries.insert(request_id.to_string(), entry.clone());
    enqueue_recent(&mut state, request_id);
    enqueue_terminal_sample(&mut state, &entry);
    let scheduled = promote_waiters_locked(&mut state, &policy);
    drop(state);
    dispatch_scheduled(scheduled);

    Some(entry)
}

pub fn complete_request(
    request_id: &str,
    status: Option<u16>,
    response_elapsed_ms: Option<u64>,
) -> Option<ActiveProbeQueueEntry> {
    finalize_request(
        request_id,
        ActiveProbeQueuePhase::Completed,
        status,
        None,
        None,
        response_elapsed_ms,
    )
}

pub fn fail_request(
    request_id: &str,
    status: Option<u16>,
    error: Option<String>,
    response_elapsed_ms: Option<u64>,
) -> Option<ActiveProbeQueueEntry> {
    finalize_request(
        request_id,
        ActiveProbeQueuePhase::Failed,
        status,
        error,
        None,
        response_elapsed_ms,
    )
}

pub fn cancel_request(
    request_id: &str,
    reason: Option<String>,
) -> Option<ActiveProbeQueueEntry> {
    let policy = configured_active_probe_policy();
    let mut state = queue_state()
        .lock()
        .expect("active probe queue poisoned");

    if let Some(position) = state
        .waiters
        .iter()
        .position(|waiter| waiter.request.request_id == request_id)
    {
        let mut waiter = state.waiters.remove(position);
        if let Some(sender) = waiter.sender.take() {
            let _ = sender.send(Err(reason
                .clone()
                .unwrap_or_else(|| "HTTP request cancelled".to_string())));
        }
    }

    let mut entry = state.entries.get(request_id)?.clone();
    if matches!(
        entry.phase,
        ActiveProbeQueuePhase::Scheduled | ActiveProbeQueuePhase::Running
    ) {
        apply_active_decrement(&mut state, &entry);
    }

    let now = now_rfc3339();
    entry.phase = ActiveProbeQueuePhase::Cancelled;
    entry.reason = reason;
    entry.finished_at = Some(now.clone());
    entry.updated_at = now;
    state.entries.insert(request_id.to_string(), entry.clone());
    enqueue_recent(&mut state, request_id);
    enqueue_terminal_sample(&mut state, &entry);
    refresh_queue_depths(&mut state);
    let scheduled = promote_waiters_locked(&mut state, &policy);
    drop(state);
    dispatch_scheduled(scheduled);

    Some(entry)
}

pub fn cancel_requests_by_run(run_id: &str, reason: Option<String>) -> Vec<String> {
    let policy = configured_active_probe_policy();
    let mut cancelled_senders = Vec::new();
    let mut state = queue_state()
        .lock()
        .expect("active probe queue poisoned");
    state.cancelled_runs.insert(run_id.to_string());

    let request_ids = state
        .entries
        .values()
        .filter(|entry| entry.run_id == run_id && is_pending_phase(entry.phase))
        .map(|entry| entry.request_id.clone())
        .collect::<Vec<_>>();

    let mut index = 0;
    while index < state.waiters.len() {
        if state.waiters[index].request.run_id == run_id {
            let mut waiter = state.waiters.remove(index);
            if let Some(sender) = waiter.sender.take() {
                cancelled_senders.push(sender);
            }
        } else {
            index += 1;
        }
    }

    for request_id in &request_ids {
        let Some(mut entry) = state.entries.get(request_id).cloned() else {
            continue;
        };
        if matches!(
            entry.phase,
            ActiveProbeQueuePhase::Scheduled | ActiveProbeQueuePhase::Running
        ) {
            apply_active_decrement(&mut state, &entry);
        }

        let now = now_rfc3339();
        entry.phase = ActiveProbeQueuePhase::Cancelled;
        entry.reason = reason.clone();
        entry.finished_at = Some(now.clone());
        entry.updated_at = now;
        state.entries.insert(request_id.clone(), entry.clone());
        enqueue_recent(&mut state, request_id);
        enqueue_terminal_sample(&mut state, &entry);
    }

    refresh_queue_depths(&mut state);
    let scheduled = promote_waiters_locked(&mut state, &policy);
    drop(state);

    let message = reason.unwrap_or_else(|| "HTTP request cancelled".to_string());
    for sender in cancelled_senders {
        let _ = sender.send(Err(message.clone()));
    }
    dispatch_scheduled(scheduled);

    request_ids
}

pub fn get_queue_snapshot() -> ActiveProbeQueueSnapshot {
    let state = queue_state()
        .lock()
        .expect("active probe queue poisoned");

    let mut pending = Vec::new();
    let mut running = Vec::new();
    for entry in state.entries.values() {
        match entry.phase {
            ActiveProbeQueuePhase::Queued | ActiveProbeQueuePhase::Scheduled => {
                pending.push(entry.clone())
            }
            ActiveProbeQueuePhase::Running => running.push(entry.clone()),
            _ => {}
        }
    }

    pending.sort_by(|left, right| left.queued_at.cmp(&right.queued_at));
    running.sort_by(|left, right| left.updated_at.cmp(&right.updated_at));
    let recent = state
        .recent_ids
        .iter()
        .filter_map(|request_id| state.entries.get(request_id).cloned())
        .collect();

    ActiveProbeQueueSnapshot {
        pending,
        running,
        recent,
    }
}

#[cfg(test)]
fn empty_stats(policy: &ActiveProbePolicy) -> ActiveProbeQueueStats {
    ActiveProbeQueueStats {
        pending_count: 0,
        queued_count: 0,
        scheduled_count: 0,
        running_count: 0,
        recent_count: 0,
        max_queue_depth: 0,
        active_global: 0,
        active_runs: 0,
        active_plugins: 0,
        active_hosts: 0,
        hottest_host: None,
        hottest_host_active: 0,
        cancelled_run_count: 0,
        configured_max_queue_depth: policy.max_queue_depth as u32,
        configured_max_global_concurrent: policy.max_global_concurrent,
        configured_max_concurrent_per_host: policy.max_concurrent_per_host,
        configured_max_concurrent_per_run: policy.max_concurrent_per_run,
        configured_max_concurrent_per_plugin: policy.max_concurrent_per_plugin,
        rejected_total_count: 0,
        rejected_cancelled_run_count: 0,
        rejected_queue_limit_count: 0,
        rejected_run_pending_limit_count: 0,
        rejected_plugin_pending_limit_count: 0,
        recent_window_total_count: 0,
        recent_window_completed_count: 0,
        recent_window_failed_count: 0,
        recent_window_cancelled_count: 0,
        recent_window_avg_queue_wait_ms: 0,
        recent_window_avg_response_elapsed_ms: 0,
    }
}

pub fn get_queue_stats() -> ActiveProbeQueueStats {
    let policy = configured_active_probe_policy();
    let mut state = queue_state()
        .lock()
        .expect("active probe queue poisoned");
    let now = Utc::now();
    prune_terminal_samples(&mut state, now);

    let mut queued_count = 0u32;
    let mut scheduled_count = 0u32;
    let mut running_count = 0u32;
    let mut recent_count = 0u32;
    let mut max_queue_depth = 0u32;

    for entry in state.entries.values() {
        match entry.phase {
            ActiveProbeQueuePhase::Queued => {
                queued_count += 1;
                max_queue_depth = max_queue_depth.max(entry.queue_depth.saturating_add(1));
            }
            ActiveProbeQueuePhase::Scheduled => {
                scheduled_count += 1;
                max_queue_depth = max_queue_depth.max(entry.queue_depth.saturating_add(1));
            }
            ActiveProbeQueuePhase::Running => {
                running_count += 1;
            }
            ActiveProbeQueuePhase::Completed
            | ActiveProbeQueuePhase::Failed
            | ActiveProbeQueuePhase::Cancelled => {
                recent_count += 1;
            }
        }
    }

    let mut hottest_host = None;
    let mut hottest_host_active = 0u32;
    let active_hosts = state
        .hosts
        .iter()
        .filter_map(|(host, host_state)| {
            if host_state.active_slots == 0 {
                return None;
            }
            if host_state.active_slots > hottest_host_active {
                hottest_host_active = host_state.active_slots;
                hottest_host = Some(host.clone());
            }
            Some(host)
        })
        .count() as u32;

    let cutoff = now - chrono::Duration::seconds(RECENT_WINDOW_SECONDS);
    let mut recent_window_total_count = 0u32;
    let mut recent_window_completed_count = 0u32;
    let mut recent_window_failed_count = 0u32;
    let mut recent_window_cancelled_count = 0u32;
    let mut queue_wait_total_ms = 0u64;
    let mut queue_wait_sample_count = 0u64;
    let mut response_elapsed_total_ms = 0u64;
    let mut response_elapsed_sample_count = 0u64;

    for sample in &state.terminal_samples {
        if sample.finished_at < cutoff {
            continue;
        }
        recent_window_total_count = recent_window_total_count.saturating_add(1);
        match sample.phase {
            TerminalSamplePhase::Completed => {
                recent_window_completed_count = recent_window_completed_count.saturating_add(1)
            }
            TerminalSamplePhase::Failed => {
                recent_window_failed_count = recent_window_failed_count.saturating_add(1)
            }
            TerminalSamplePhase::Cancelled => {
                recent_window_cancelled_count = recent_window_cancelled_count.saturating_add(1)
            }
        }
        if let Some(queue_wait_ms) = sample.queue_wait_ms {
            queue_wait_total_ms = queue_wait_total_ms.saturating_add(queue_wait_ms);
            queue_wait_sample_count = queue_wait_sample_count.saturating_add(1);
        }
        if let Some(response_elapsed_ms) = sample.response_elapsed_ms {
            response_elapsed_total_ms =
                response_elapsed_total_ms.saturating_add(response_elapsed_ms);
            response_elapsed_sample_count = response_elapsed_sample_count.saturating_add(1);
        }
    }

    ActiveProbeQueueStats {
        pending_count: queued_count + scheduled_count,
        queued_count,
        scheduled_count,
        running_count,
        recent_count,
        max_queue_depth,
        active_global: state.active_global,
        active_runs: state.active_by_run.len() as u32,
        active_plugins: state.active_by_plugin.len() as u32,
        active_hosts,
        hottest_host,
        hottest_host_active,
        cancelled_run_count: state.cancelled_runs.len() as u32,
        configured_max_queue_depth: policy.max_queue_depth as u32,
        configured_max_global_concurrent: policy.max_global_concurrent,
        configured_max_concurrent_per_host: policy.max_concurrent_per_host,
        configured_max_concurrent_per_run: policy.max_concurrent_per_run,
        configured_max_concurrent_per_plugin: policy.max_concurrent_per_plugin,
        rejected_total_count: state
            .rejection_stats
            .cancelled_run
            .saturating_add(state.rejection_stats.queue_limit)
            .saturating_add(state.rejection_stats.run_pending_limit)
            .saturating_add(state.rejection_stats.plugin_pending_limit),
        rejected_cancelled_run_count: state.rejection_stats.cancelled_run,
        rejected_queue_limit_count: state.rejection_stats.queue_limit,
        rejected_run_pending_limit_count: state.rejection_stats.run_pending_limit,
        rejected_plugin_pending_limit_count: state.rejection_stats.plugin_pending_limit,
        recent_window_total_count,
        recent_window_completed_count,
        recent_window_failed_count,
        recent_window_cancelled_count,
        recent_window_avg_queue_wait_ms: if queue_wait_sample_count == 0 {
            0
        } else {
            queue_wait_total_ms / queue_wait_sample_count
        },
        recent_window_avg_response_elapsed_ms: if response_elapsed_sample_count == 0 {
            0
        } else {
            response_elapsed_total_ms / response_elapsed_sample_count
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(id: &str, run_id: &str, plugin_id: &str, host: &str) -> ActiveProbeScheduleRequest {
        ActiveProbeScheduleRequest {
            request_id: id.to_string(),
            run_id: run_id.to_string(),
            plugin_id: plugin_id.to_string(),
            execution_context: "active_probe".to_string(),
            method: "GET".to_string(),
            url: format!("https://{host}/{id}"),
            host: host.to_string(),
        }
    }

    fn test_policy() -> ActiveProbePolicy {
        ActiveProbePolicy {
            max_queue_depth: 10,
            max_pending_per_run: 4,
            max_pending_per_plugin: 10,
            max_global_concurrent: 2,
            max_concurrent_per_host: 1,
            max_concurrent_per_run: 1,
            max_concurrent_per_plugin: 2,
            min_host_delay_ms: 0,
            jitter_range: [0, 0],
            timeout_ms: 3_000,
        }
    }

    #[tokio::test]
    async fn rejects_run_pending_limit_before_enqueue() {
        let policy = ActiveProbePolicy {
            max_pending_per_run: 1,
            max_global_concurrent: 0,
            ..test_policy()
        };
        let _ = enqueue_request(request("limit-run-1", "limit-run", "p", "a.test"), policy.clone())
            .expect("first request should enqueue");
        let error = enqueue_request(request("limit-run-2", "limit-run", "p", "b.test"), policy)
            .expect_err("second request should exceed run pending limit");
        assert!(error.contains("run pending limit"));
    }

    #[tokio::test]
    async fn dispatches_round_robin_by_run() {
        let policy = test_policy();
        let rx1 = enqueue_request(request("rr-1", "run-a", "p", "a.test"), policy.clone()).unwrap();
        let mut rx2 =
            enqueue_request(request("rr-2", "run-a", "p", "b.test"), policy.clone()).unwrap();
        let rx3 = enqueue_request(request("rr-3", "run-b", "p", "c.test"), policy.clone()).unwrap();

        assert!(rx1.await.unwrap().is_ok());
        assert!(rx2.try_recv().is_err());
        assert!(rx3.await.unwrap().is_ok());
    }

    #[tokio::test]
    async fn cancels_all_pending_requests_for_run() {
        let policy = ActiveProbePolicy {
            max_global_concurrent: 1,
            ..test_policy()
        };
        let _blocker =
            enqueue_request(request("cancel-blocker", "other-run", "p", "a.test"), policy.clone())
                .unwrap();
        let rx1 = enqueue_request(
            request("cancel-run-1", "cancel-run", "p", "a.test"),
            policy.clone(),
        )
        .unwrap();
        let rx2 =
            enqueue_request(request("cancel-run-2", "cancel-run", "p", "b.test"), policy).unwrap();

        let cancelled = cancel_requests_by_run("cancel-run", Some("run stopped".to_string()));
        assert_eq!(cancelled.len(), 2);
        assert!(rx1.await.unwrap().is_err());
        assert!(rx2.await.unwrap().is_err());
    }

    #[tokio::test]
    async fn rejects_new_requests_for_cancelled_run() {
        let policy = test_policy();
        let _ = cancel_requests_by_run("already-cancelled", Some("stopped".to_string()));
        let error = enqueue_request(
            request("cancelled-new-1", "already-cancelled", "p", "a.test"),
            policy,
        )
        .expect_err("new request for cancelled run should be rejected");
        assert!(error.contains("cancelled"));
    }

    #[tokio::test]
    async fn tracks_rejection_reason_counters() {
        let queue_limit_policy = ActiveProbePolicy {
            max_queue_depth: 1,
            max_global_concurrent: 0,
            ..test_policy()
        };
        let _ = enqueue_request(
            request(
                "reject-queue-1",
                "queue-run-a",
                "queue-plugin-a",
                "queue-a.test",
            ),
            queue_limit_policy.clone(),
        )
        .expect("first request should enqueue");
        let queue_error = enqueue_request(
            request(
                "reject-queue-2",
                "queue-run-b",
                "queue-plugin-b",
                "queue-b.test",
            ),
            queue_limit_policy,
        )
        .expect_err("second request should exceed queue limit");
        assert!(queue_error.contains("queue limit"));

        let run_limit_policy = ActiveProbePolicy {
            max_pending_per_run: 1,
            max_global_concurrent: 0,
            ..test_policy()
        };
        let _ = enqueue_request(
            request(
                "reject-run-1",
                "run-limit",
                "run-plugin-a",
                "run-a.test",
            ),
            run_limit_policy.clone(),
        )
        .expect("first run-limited request should enqueue");
        let run_error = enqueue_request(
            request(
                "reject-run-2",
                "run-limit",
                "run-plugin-b",
                "run-b.test",
            ),
            run_limit_policy,
        )
        .expect_err("second request should exceed run pending limit");
        assert!(run_error.contains("run pending limit"));

        let plugin_limit_policy = ActiveProbePolicy {
            max_pending_per_plugin: 1,
            max_global_concurrent: 0,
            ..test_policy()
        };
        let _ = enqueue_request(
            request(
                "reject-plugin-1",
                "plugin-run-a",
                "shared-plugin-limit",
                "plugin-a.test",
            ),
            plugin_limit_policy.clone(),
        )
        .expect("first plugin-limited request should enqueue");
        let plugin_error = enqueue_request(
            request(
                "reject-plugin-2",
                "plugin-run-b",
                "shared-plugin-limit",
                "plugin-b.test",
            ),
            plugin_limit_policy,
        )
        .expect_err("second request should exceed plugin pending limit");
        assert!(plugin_error.contains("plugin pending limit"));

        let _ = cancel_requests_by_run("rejected-cancelled-run", Some("stop".to_string()));
        let cancelled_error = enqueue_request(
            request(
                "reject-cancelled-1",
                "rejected-cancelled-run",
                "cancelled-plugin",
                "cancelled.test",
            ),
            test_policy(),
        )
        .expect_err("request for cancelled run should be rejected");
        assert!(cancelled_error.contains("cancelled"));

        assert!(cancel_request(
            "reject-queue-1",
            Some("cleanup queue limit sample".to_string())
        )
        .is_some());
        assert!(cancel_request(
            "reject-run-1",
            Some("cleanup run limit sample".to_string())
        )
        .is_some());
        assert!(cancel_request(
            "reject-plugin-1",
            Some("cleanup plugin limit sample".to_string())
        )
        .is_some());

        let throughput_policy = ActiveProbePolicy {
            max_global_concurrent: 1,
            ..test_policy()
        };

        let success_rx = enqueue_request(
            request(
                "throughput-complete",
                "throughput-run-complete",
                "throughput-plugin-complete",
                "throughput-complete.test",
            ),
            throughput_policy.clone(),
        )
        .expect("success request should enqueue");
        assert!(success_rx.await.unwrap().is_ok());
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        assert!(mark_request_running("throughput-complete").is_some());
        assert!(complete_request("throughput-complete", Some(200), Some(120)).is_some());

        let fail_rx = enqueue_request(
            request(
                "throughput-fail",
                "throughput-run-fail",
                "throughput-plugin-fail",
                "throughput-fail.test",
            ),
            throughput_policy.clone(),
        )
        .expect("failed request should enqueue");
        assert!(fail_rx.await.unwrap().is_ok());
        tokio::time::sleep(std::time::Duration::from_millis(7)).await;
        assert!(mark_request_running("throughput-fail").is_some());
        assert!(fail_request(
            "throughput-fail",
            Some(500),
            Some("boom".to_string()),
            Some(250)
        )
        .is_some());

        let blocker_rx = enqueue_request(
            request(
                "throughput-blocker",
                "throughput-run-blocker",
                "throughput-plugin-blocker",
                "throughput-blocker.test",
            ),
            throughput_policy.clone(),
        )
        .expect("blocker should enqueue");
        assert!(blocker_rx.await.unwrap().is_ok());
        assert!(mark_request_running("throughput-blocker").is_some());

        let cancelled_pending_rx = enqueue_request(
            request(
                "throughput-cancelled",
                "throughput-run-cancelled",
                "throughput-plugin-cancelled",
                "throughput-cancelled.test",
            ),
            throughput_policy,
        )
        .expect("cancelled pending request should enqueue");
        assert!(cancel_request(
            "throughput-cancelled",
            Some("cancel throughput".to_string())
        )
        .is_some());
        assert!(cancelled_pending_rx.await.unwrap().is_err());
        assert!(complete_request("throughput-blocker", Some(200), Some(50)).is_some());

        let stats = get_queue_stats();
        assert_eq!(stats.rejected_queue_limit_count, 1);
        assert_eq!(stats.rejected_run_pending_limit_count, 1);
        assert_eq!(stats.rejected_plugin_pending_limit_count, 1);
        assert_eq!(stats.rejected_cancelled_run_count, 1);
        assert_eq!(stats.rejected_total_count, 4);
        assert_eq!(stats.recent_window_total_count, 7);
        assert_eq!(stats.recent_window_completed_count, 2);
        assert_eq!(stats.recent_window_failed_count, 1);
        assert_eq!(stats.recent_window_cancelled_count, 4);
        assert!(stats.recent_window_avg_queue_wait_ms > 0);
        assert!(stats.recent_window_avg_response_elapsed_ms > 0);
    }

    #[test]
    fn configured_active_probe_policy_reads_runtime_settings() {
        let policy = configured_active_probe_policy();
        let settings = get_plugin_runtime_settings().active_probe;
        assert_eq!(policy.max_queue_depth, settings.max_queue_depth as usize);
        assert_eq!(policy.timeout_ms, settings.timeout_ms);
    }

    #[test]
    fn empty_snapshot_before_any_requests() {
        let snapshot = get_queue_snapshot();
        assert!(snapshot.pending.is_empty());
        assert!(snapshot.running.is_empty());
        assert!(snapshot.recent.is_empty());
    }

    #[test]
    fn empty_stats_uses_configured_policy_limits() {
        let policy = configured_active_probe_policy();
        let stats = empty_stats(&policy);
        assert_eq!(
            stats.configured_max_queue_depth,
            policy.max_queue_depth as u32
        );
        assert_eq!(
            stats.configured_max_global_concurrent,
            policy.max_global_concurrent
        );
    }
}
