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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginFetchPolicyKind {
    BountyFetch,
    MonitorFetch,
    AgentFetch,
    TrafficActiveProbe,
    PluginTestFetch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginRequestPhase {
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
pub struct PluginFetchPolicy {
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

impl PluginFetchPolicy {
    pub fn for_kind(kind: PluginFetchPolicyKind) -> Self {
        match kind {
            PluginFetchPolicyKind::BountyFetch => Self {
                max_queue_depth: 1_000,
                max_pending_per_run: 250,
                max_pending_per_plugin: 500,
                max_global_concurrent: 16,
                max_concurrent_per_host: 2,
                max_concurrent_per_run: 16,
                max_concurrent_per_plugin: 16,
                min_host_delay_ms: 1_000,
                jitter_range: [300, 1_000],
                timeout_ms: 3_000,
            },
            PluginFetchPolicyKind::MonitorFetch => Self {
                max_queue_depth: 5_000,
                max_pending_per_run: 1_000,
                max_pending_per_plugin: 2_000,
                max_global_concurrent: 200,
                max_concurrent_per_host: 20,
                max_concurrent_per_run: 200,
                max_concurrent_per_plugin: 200,
                min_host_delay_ms: 50,
                jitter_range: [0, 50],
                timeout_ms: 8_000,
            },
            PluginFetchPolicyKind::AgentFetch => Self {
                max_queue_depth: 300,
                max_pending_per_run: 75,
                max_pending_per_plugin: 150,
                max_global_concurrent: 16,
                max_concurrent_per_host: 2,
                max_concurrent_per_run: 16,
                max_concurrent_per_plugin: 16,
                min_host_delay_ms: 500,
                jitter_range: [100, 500],
                timeout_ms: 3_000,
            },
            PluginFetchPolicyKind::TrafficActiveProbe => Self {
                max_queue_depth: 1_000,
                max_pending_per_run: 250,
                max_pending_per_plugin: 500,
                max_global_concurrent: 16,
                max_concurrent_per_host: 2,
                max_concurrent_per_run: 16,
                max_concurrent_per_plugin: 16,
                min_host_delay_ms: 1_000,
                jitter_range: [300, 1_000],
                timeout_ms: 3_000,
            },
            PluginFetchPolicyKind::PluginTestFetch => Self {
                max_queue_depth: 50,
                max_pending_per_run: 20,
                max_pending_per_plugin: 30,
                max_global_concurrent: 16,
                max_concurrent_per_host: 1,
                max_concurrent_per_run: 16,
                max_concurrent_per_plugin: 16,
                min_host_delay_ms: 100,
                jitter_range: [0, 100],
                timeout_ms: 3_000,
            },
        }
    }
}

pub fn configured_policy_for_kind(kind: PluginFetchPolicyKind) -> PluginFetchPolicy {
    let settings = get_plugin_runtime_settings();
    match kind {
        PluginFetchPolicyKind::TrafficActiveProbe => PluginFetchPolicy {
            max_queue_depth: settings.active_probe.max_queue_depth as usize,
            max_pending_per_run: settings.active_probe.max_pending_per_run as usize,
            max_pending_per_plugin: settings.active_probe.max_pending_per_plugin as usize,
            max_global_concurrent: settings.active_probe.max_global_concurrent as u32,
            max_concurrent_per_host: settings.active_probe.max_concurrent_per_host as u32,
            max_concurrent_per_run: settings.active_probe.max_concurrent_per_run as u32,
            max_concurrent_per_plugin: settings.active_probe.max_concurrent_per_plugin as u32,
            min_host_delay_ms: settings.active_probe.min_host_cooldown_ms,
            jitter_range: settings.active_probe.jitter_range,
            timeout_ms: settings.active_probe.timeout_ms,
        },
        PluginFetchPolicyKind::BountyFetch => map_runtime_policy(settings.bounty_fetch),
        PluginFetchPolicyKind::MonitorFetch => map_runtime_policy(settings.monitor_fetch),
        PluginFetchPolicyKind::AgentFetch => map_runtime_policy(settings.agent_fetch),
        PluginFetchPolicyKind::PluginTestFetch => map_runtime_policy(settings.plugin_test_fetch),
    }
}

fn map_runtime_policy(
    settings: crate::runtime_config::PluginFetchRuntimeSettings,
) -> PluginFetchPolicy {
    PluginFetchPolicy {
        max_queue_depth: settings.max_queue_depth as usize,
        max_pending_per_run: settings.max_pending_per_run as usize,
        max_pending_per_plugin: settings.max_pending_per_plugin as usize,
        max_global_concurrent: settings.max_global_concurrent as u32,
        max_concurrent_per_host: settings.max_concurrent_per_host as u32,
        max_concurrent_per_run: settings.max_concurrent_per_run as u32,
        max_concurrent_per_plugin: settings.max_concurrent_per_plugin as u32,
        min_host_delay_ms: settings.min_host_delay_ms,
        jitter_range: settings.jitter_range,
        timeout_ms: settings.timeout_ms,
    }
}

#[derive(Debug, Clone)]
pub struct PluginRequestScheduleRequest {
    pub kind: PluginFetchPolicyKind,
    pub request_id: String,
    pub run_id: String,
    pub plugin_id: String,
    pub execution_context: String,
    pub method: String,
    pub url: String,
    pub host: String,
}

#[derive(Debug, Clone)]
pub struct PluginRequestDispatchGrant {
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
pub struct PluginRequestQueueEntry {
    pub kind: PluginFetchPolicyKind,
    pub request_id: String,
    pub run_id: String,
    pub plugin_id: String,
    pub execution_context: String,
    pub phase: PluginRequestPhase,
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
pub struct PluginRequestQueueSnapshot {
    pub pending: Vec<PluginRequestQueueEntry>,
    pub running: Vec<PluginRequestQueueEntry>,
    pub recent: Vec<PluginRequestQueueEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRequestQueueStats {
    pub kind: PluginFetchPolicyKind,
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
    request: PluginRequestScheduleRequest,
    sequence: u64,
    sender: Option<oneshot::Sender<Result<PluginRequestDispatchGrant, String>>>,
}

#[derive(Debug, Default)]
struct HostState {
    active_slots: u32,
    next_dispatch_at: Option<DateTime<Utc>>,
    adaptive_penalty_ms: u64,
}

#[derive(Debug, Default)]
struct PolicyState {
    entries: HashMap<String, PluginRequestQueueEntry>,
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

#[derive(Debug, Default)]
struct SchedulerState {
    policies: HashMap<PluginFetchPolicyKind, PolicyState>,
}

#[derive(Debug)]
struct ScheduledWaiter {
    grant: PluginRequestDispatchGrant,
    sender: Option<oneshot::Sender<Result<PluginRequestDispatchGrant, String>>>,
}

static REQUEST_SCHEDULER: OnceLock<Mutex<SchedulerState>> = OnceLock::new();

fn scheduler_state() -> &'static Mutex<SchedulerState> {
    REQUEST_SCHEDULER.get_or_init(|| Mutex::new(SchedulerState::default()))
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

fn is_pending_phase(phase: PluginRequestPhase) -> bool {
    matches!(
        phase,
        PluginRequestPhase::Queued | PluginRequestPhase::Scheduled | PluginRequestPhase::Running
    )
}

fn pending_counts(
    policy_state: &PolicyState,
    run_id: &str,
    plugin_id: &str,
) -> (usize, usize, usize) {
    let mut total = 0;
    let mut run = 0;
    let mut plugin = 0;
    for entry in policy_state.entries.values() {
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
    policy_state: &PolicyState,
    policy: &PluginFetchPolicy,
    request: &PluginRequestScheduleRequest,
) -> bool {
    if policy_state.active_global >= policy.max_global_concurrent.max(1) {
        return false;
    }
    if active_count(&policy_state.active_by_run, &request.run_id)
        >= policy.max_concurrent_per_run.max(1)
    {
        return false;
    }
    if active_count(&policy_state.active_by_plugin, &request.plugin_id)
        >= policy.max_concurrent_per_plugin.max(1)
    {
        return false;
    }
    let host_active = policy_state
        .hosts
        .get(&request.host)
        .map(|host| host.active_slots)
        .unwrap_or(0);
    host_active < policy.max_concurrent_per_host.max(1)
}

fn build_entry(
    request: &PluginRequestScheduleRequest,
    adaptive_penalty_ms: u64,
    queue_depth: u32,
) -> PluginRequestQueueEntry {
    let now = now_rfc3339();
    PluginRequestQueueEntry {
        kind: request.kind,
        request_id: request.request_id.clone(),
        run_id: request.run_id.clone(),
        plugin_id: request.plugin_id.clone(),
        execution_context: request.execution_context.clone(),
        phase: PluginRequestPhase::Queued,
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

fn refresh_queue_depths(policy_state: &mut PolicyState) {
    let mut depths_by_run: HashMap<String, u32> = HashMap::new();
    let mut queued: Vec<_> = policy_state
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
        if let Some(entry) = policy_state.entries.get_mut(&request_id) {
            entry.queue_depth = *depth;
            entry.updated_at = now_rfc3339();
        }
        *depth += 1;
    }
}

fn apply_active_increment(policy_state: &mut PolicyState, request: &PluginRequestScheduleRequest) {
    policy_state.active_global += 1;
    *policy_state
        .active_by_run
        .entry(request.run_id.clone())
        .or_insert(0) += 1;
    *policy_state
        .active_by_plugin
        .entry(request.plugin_id.clone())
        .or_insert(0) += 1;
    policy_state
        .hosts
        .entry(request.host.clone())
        .or_default()
        .active_slots += 1;
}

fn apply_active_decrement(policy_state: &mut PolicyState, entry: &PluginRequestQueueEntry) {
    policy_state.active_global = policy_state.active_global.saturating_sub(1);
    decrement_map(&mut policy_state.active_by_run, &entry.run_id);
    decrement_map(&mut policy_state.active_by_plugin, &entry.plugin_id);
    if let Some(host) = policy_state.hosts.get_mut(&entry.host) {
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

fn prune_recent(policy_state: &mut PolicyState) {
    while policy_state.recent_ids.len() > RECENT_LIMIT {
        let Some(oldest_id) = policy_state.recent_ids.pop_back() else {
            break;
        };
        let can_remove = policy_state
            .entries
            .get(&oldest_id)
            .map(|entry| !is_pending_phase(entry.phase))
            .unwrap_or(false);
        if can_remove {
            policy_state.entries.remove(&oldest_id);
        }
    }
}

fn enqueue_recent(policy_state: &mut PolicyState, request_id: &str) {
    if let Some(position) = policy_state
        .recent_ids
        .iter()
        .position(|existing| existing == request_id)
    {
        policy_state.recent_ids.remove(position);
    }
    policy_state.recent_ids.push_front(request_id.to_string());
    prune_recent(policy_state);
}

fn enqueue_terminal_sample(policy_state: &mut PolicyState, entry: &PluginRequestQueueEntry) {
    let Some(finished_at) = entry.finished_at.as_deref().and_then(parse_rfc3339) else {
        return;
    };

    let phase = match entry.phase {
        PluginRequestPhase::Completed => TerminalSamplePhase::Completed,
        PluginRequestPhase::Failed => TerminalSamplePhase::Failed,
        PluginRequestPhase::Cancelled => TerminalSamplePhase::Cancelled,
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

    policy_state.terminal_samples.push_back(TerminalSample {
        finished_at,
        phase,
        queue_wait_ms,
        response_elapsed_ms: entry.response_elapsed_ms,
    });
    prune_terminal_samples(policy_state, Utc::now());
}

fn prune_terminal_samples(policy_state: &mut PolicyState, now: DateTime<Utc>) {
    let cutoff = now - chrono::Duration::seconds(RECENT_WINDOW_SECONDS);
    while let Some(sample) = policy_state.terminal_samples.front() {
        if sample.finished_at >= cutoff
            && policy_state.terminal_samples.len() <= TERMINAL_SAMPLE_LIMIT
        {
            break;
        }
        policy_state.terminal_samples.pop_front();
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
    policy_state: &mut PolicyState,
    policy: &PluginFetchPolicy,
) -> Vec<ScheduledWaiter> {
    let mut scheduled = Vec::new();
    if policy_state.waiters.is_empty() {
        return scheduled;
    }

    loop {
        let run_count = policy_state.run_order.len();
        if run_count == 0 {
            break;
        }

        let mut selected: Option<(usize, String)> = None;
        for _ in 0..run_count {
            let Some(run_id) = policy_state.run_order.pop_front() else {
                break;
            };
            let candidate_index = policy_state
                .waiters
                .iter()
                .position(|waiter| waiter.request.run_id == run_id);
            let should_keep_run = candidate_index.is_some();
            if let Some(index) = candidate_index {
                if can_schedule(policy_state, policy, &policy_state.waiters[index].request) {
                    selected = Some((index, run_id.clone()));
                    break;
                }
            }
            if should_keep_run {
                policy_state.run_order.push_back(run_id);
            }
        }

        let Some((index, run_id)) = selected else {
            break;
        };

        let mut waiter = policy_state.waiters.remove(index);
        if policy_state
            .waiters
            .iter()
            .any(|queued| queued.request.run_id == run_id)
        {
            policy_state.run_order.push_back(run_id);
        }

        let now = Utc::now();
        let (cooldown_wait_ms, jitter_wait_ms, total_wait_ms, adaptive_penalty_ms) = {
            let host = policy_state
                .hosts
                .entry(waiter.request.host.clone())
                .or_default();
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

        apply_active_increment(policy_state, &waiter.request);

        let grant = PluginRequestDispatchGrant {
            cooldown_wait_ms,
            jitter_wait_ms,
            total_wait_ms,
            timeout_ms: policy.timeout_ms,
            active_global: policy_state.active_global,
            active_for_host: policy_state
                .hosts
                .get(&waiter.request.host)
                .map(|host| host.active_slots)
                .unwrap_or(0),
            active_for_run: active_count(&policy_state.active_by_run, &waiter.request.run_id),
            active_for_plugin: active_count(
                &policy_state.active_by_plugin,
                &waiter.request.plugin_id,
            ),
        };

        let scheduled_at = now_rfc3339();
        if let Some(entry) = policy_state.entries.get_mut(&waiter.request.request_id) {
            entry.phase = PluginRequestPhase::Scheduled;
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

    refresh_queue_depths(policy_state);
    scheduled
}

pub fn enqueue_plugin_request(
    request: PluginRequestScheduleRequest,
    policy: PluginFetchPolicy,
) -> Result<oneshot::Receiver<Result<PluginRequestDispatchGrant, String>>, String> {
    let (tx, rx) = oneshot::channel();
    let mut state = scheduler_state()
        .lock()
        .expect("plugin request scheduler poisoned");
    let policy_state = state.policies.entry(request.kind).or_default();
    if policy_state.cancelled_runs.contains(&request.run_id) {
        policy_state.rejection_stats.cancelled_run =
            policy_state.rejection_stats.cancelled_run.saturating_add(1);
        return Err("Plugin request run has been cancelled".to_string());
    }
    let (total_pending, run_pending, plugin_pending) =
        pending_counts(policy_state, &request.run_id, &request.plugin_id);

    if total_pending >= policy.max_queue_depth {
        policy_state.rejection_stats.queue_limit =
            policy_state.rejection_stats.queue_limit.saturating_add(1);
        return Err("Plugin request queue limit exceeded".to_string());
    }
    if run_pending >= policy.max_pending_per_run {
        policy_state.rejection_stats.run_pending_limit = policy_state
            .rejection_stats
            .run_pending_limit
            .saturating_add(1);
        return Err("Plugin request run pending limit exceeded".to_string());
    }
    if plugin_pending >= policy.max_pending_per_plugin {
        policy_state.rejection_stats.plugin_pending_limit = policy_state
            .rejection_stats
            .plugin_pending_limit
            .saturating_add(1);
        return Err("Plugin request plugin pending limit exceeded".to_string());
    }

    let queue_depth = policy_state.waiters.len() as u32;
    let adaptive_penalty_ms = policy_state
        .hosts
        .get(&request.host)
        .map(|host| host.adaptive_penalty_ms)
        .unwrap_or(0);
    let entry = build_entry(&request, adaptive_penalty_ms, queue_depth);
    policy_state
        .entries
        .insert(request.request_id.clone(), entry);

    let sequence = policy_state.sequence;
    policy_state.sequence += 1;
    if !policy_state
        .run_order
        .iter()
        .any(|existing| existing == &request.run_id)
    {
        policy_state.run_order.push_back(request.run_id.clone());
    }
    policy_state.waiters.push(QueuedWaiter {
        request,
        sequence,
        sender: Some(tx),
    });

    let scheduled = promote_waiters_locked(policy_state, &policy);
    drop(state);

    for waiter in scheduled {
        if let Some(sender) = waiter.sender {
            let _ = sender.send(Ok(waiter.grant));
        }
    }

    Ok(rx)
}

pub fn mark_plugin_request_running(
    kind: PluginFetchPolicyKind,
    request_id: &str,
) -> Option<PluginRequestQueueEntry> {
    let mut state = scheduler_state()
        .lock()
        .expect("plugin request scheduler poisoned");
    let policy_state = state.policies.get_mut(&kind)?;
    let entry = policy_state.entries.get_mut(request_id)?;
    if entry.phase == PluginRequestPhase::Cancelled {
        return None;
    }
    entry.phase = PluginRequestPhase::Running;
    entry.dispatch_started_at = Some(now_rfc3339());
    entry.updated_at = entry
        .dispatch_started_at
        .clone()
        .unwrap_or_else(now_rfc3339);
    Some(entry.clone())
}

fn finalize_plugin_request(
    kind: PluginFetchPolicyKind,
    request_id: &str,
    phase: PluginRequestPhase,
    status: Option<u16>,
    error: Option<String>,
    reason: Option<String>,
    response_elapsed_ms: Option<u64>,
) -> Option<PluginRequestQueueEntry> {
    let policy = configured_policy_for_kind(kind);
    let mut state = scheduler_state()
        .lock()
        .expect("plugin request scheduler poisoned");
    let policy_state = state.policies.get_mut(&kind)?;
    let mut entry = policy_state.entries.get(request_id)?.clone();

    if matches!(
        entry.phase,
        PluginRequestPhase::Scheduled | PluginRequestPhase::Running
    ) {
        apply_active_decrement(policy_state, &entry);
    }

    if let Some(host) = policy_state.hosts.get_mut(&entry.host) {
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
    policy_state
        .entries
        .insert(request_id.to_string(), entry.clone());
    enqueue_recent(policy_state, request_id);
    enqueue_terminal_sample(policy_state, &entry);
    let scheduled = promote_waiters_locked(policy_state, &policy);
    drop(state);

    for waiter in scheduled {
        if let Some(sender) = waiter.sender {
            let _ = sender.send(Ok(waiter.grant));
        }
    }

    Some(entry)
}

pub fn complete_plugin_request(
    kind: PluginFetchPolicyKind,
    request_id: &str,
    status: Option<u16>,
    response_elapsed_ms: Option<u64>,
) -> Option<PluginRequestQueueEntry> {
    finalize_plugin_request(
        kind,
        request_id,
        PluginRequestPhase::Completed,
        status,
        None,
        None,
        response_elapsed_ms,
    )
}

pub fn fail_plugin_request(
    kind: PluginFetchPolicyKind,
    request_id: &str,
    status: Option<u16>,
    error: Option<String>,
    response_elapsed_ms: Option<u64>,
) -> Option<PluginRequestQueueEntry> {
    finalize_plugin_request(
        kind,
        request_id,
        PluginRequestPhase::Failed,
        status,
        error,
        None,
        response_elapsed_ms,
    )
}

pub fn cancel_plugin_request(
    kind: PluginFetchPolicyKind,
    request_id: &str,
    reason: Option<String>,
) -> Option<PluginRequestQueueEntry> {
    let policy = configured_policy_for_kind(kind);
    let mut state = scheduler_state()
        .lock()
        .expect("plugin request scheduler poisoned");
    let policy_state = state.policies.get_mut(&kind)?;

    if let Some(position) = policy_state
        .waiters
        .iter()
        .position(|waiter| waiter.request.request_id == request_id)
    {
        let mut waiter = policy_state.waiters.remove(position);
        if let Some(sender) = waiter.sender.take() {
            let _ = sender.send(Err(reason
                .clone()
                .unwrap_or_else(|| "HTTP request cancelled".to_string())));
        }
    }

    let mut entry = policy_state.entries.get(request_id)?.clone();
    if matches!(
        entry.phase,
        PluginRequestPhase::Scheduled | PluginRequestPhase::Running
    ) {
        apply_active_decrement(policy_state, &entry);
    }

    let now = now_rfc3339();
    entry.phase = PluginRequestPhase::Cancelled;
    entry.reason = reason;
    entry.finished_at = Some(now.clone());
    entry.updated_at = now;
    policy_state
        .entries
        .insert(request_id.to_string(), entry.clone());
    enqueue_recent(policy_state, request_id);
    enqueue_terminal_sample(policy_state, &entry);
    refresh_queue_depths(policy_state);
    let scheduled = promote_waiters_locked(policy_state, &policy);
    drop(state);

    for waiter in scheduled {
        if let Some(sender) = waiter.sender {
            let _ = sender.send(Ok(waiter.grant));
        }
    }

    Some(entry)
}

pub fn cancel_plugin_requests_by_run(
    kind: PluginFetchPolicyKind,
    run_id: &str,
    reason: Option<String>,
) -> Vec<String> {
    let policy = configured_policy_for_kind(kind);
    let mut cancelled_senders = Vec::new();
    let mut state = scheduler_state()
        .lock()
        .expect("plugin request scheduler poisoned");
    let policy_state = state.policies.entry(kind).or_default();
    policy_state.cancelled_runs.insert(run_id.to_string());

    let request_ids = policy_state
        .entries
        .values()
        .filter(|entry| entry.run_id == run_id && is_pending_phase(entry.phase))
        .map(|entry| entry.request_id.clone())
        .collect::<Vec<_>>();

    let mut index = 0;
    while index < policy_state.waiters.len() {
        if policy_state.waiters[index].request.run_id == run_id {
            let mut waiter = policy_state.waiters.remove(index);
            if let Some(sender) = waiter.sender.take() {
                cancelled_senders.push(sender);
            }
        } else {
            index += 1;
        }
    }

    for request_id in &request_ids {
        let Some(mut entry) = policy_state.entries.get(request_id).cloned() else {
            continue;
        };
        if matches!(
            entry.phase,
            PluginRequestPhase::Scheduled | PluginRequestPhase::Running
        ) {
            apply_active_decrement(policy_state, &entry);
        }

        let now = now_rfc3339();
        entry.phase = PluginRequestPhase::Cancelled;
        entry.reason = reason.clone();
        entry.finished_at = Some(now.clone());
        entry.updated_at = now;
        policy_state
            .entries
            .insert(request_id.clone(), entry.clone());
        enqueue_recent(policy_state, request_id);
        enqueue_terminal_sample(policy_state, &entry);
    }

    refresh_queue_depths(policy_state);
    let scheduled = promote_waiters_locked(policy_state, &policy);
    drop(state);

    let message = reason.unwrap_or_else(|| "HTTP request cancelled".to_string());
    for sender in cancelled_senders {
        let _ = sender.send(Err(message.clone()));
    }
    for waiter in scheduled {
        if let Some(sender) = waiter.sender {
            let _ = sender.send(Ok(waiter.grant));
        }
    }

    request_ids
}

pub fn get_plugin_request_queue_snapshot(
    kind: PluginFetchPolicyKind,
) -> PluginRequestQueueSnapshot {
    let state = scheduler_state()
        .lock()
        .expect("plugin request scheduler poisoned");
    let Some(policy_state) = state.policies.get(&kind) else {
        return PluginRequestQueueSnapshot {
            pending: Vec::new(),
            running: Vec::new(),
            recent: Vec::new(),
        };
    };

    let mut pending = Vec::new();
    let mut running = Vec::new();
    for entry in policy_state.entries.values() {
        match entry.phase {
            PluginRequestPhase::Queued | PluginRequestPhase::Scheduled => {
                pending.push(entry.clone())
            }
            PluginRequestPhase::Running => running.push(entry.clone()),
            _ => {}
        }
    }

    pending.sort_by(|left, right| left.queued_at.cmp(&right.queued_at));
    running.sort_by(|left, right| left.updated_at.cmp(&right.updated_at));
    let recent = policy_state
        .recent_ids
        .iter()
        .filter_map(|request_id| policy_state.entries.get(request_id).cloned())
        .collect();

    PluginRequestQueueSnapshot {
        pending,
        running,
        recent,
    }
}

pub fn get_plugin_request_queue_stats(kind: PluginFetchPolicyKind) -> PluginRequestQueueStats {
    let policy = configured_policy_for_kind(kind);
    let mut state = scheduler_state()
        .lock()
        .expect("plugin request scheduler poisoned");
    let Some(policy_state) = state.policies.get_mut(&kind) else {
        return PluginRequestQueueStats {
            kind,
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
        };
    };
    let now = Utc::now();
    prune_terminal_samples(policy_state, now);

    let mut queued_count = 0u32;
    let mut scheduled_count = 0u32;
    let mut running_count = 0u32;
    let mut recent_count = 0u32;
    let mut max_queue_depth = 0u32;

    for entry in policy_state.entries.values() {
        match entry.phase {
            PluginRequestPhase::Queued => {
                queued_count += 1;
                max_queue_depth = max_queue_depth.max(entry.queue_depth.saturating_add(1));
            }
            PluginRequestPhase::Scheduled => {
                scheduled_count += 1;
                max_queue_depth = max_queue_depth.max(entry.queue_depth.saturating_add(1));
            }
            PluginRequestPhase::Running => {
                running_count += 1;
            }
            PluginRequestPhase::Completed
            | PluginRequestPhase::Failed
            | PluginRequestPhase::Cancelled => {
                recent_count += 1;
            }
        }
    }

    let mut hottest_host = None;
    let mut hottest_host_active = 0u32;
    let active_hosts = policy_state
        .hosts
        .iter()
        .filter_map(|(host, state)| {
            if state.active_slots == 0 {
                return None;
            }
            if state.active_slots > hottest_host_active {
                hottest_host_active = state.active_slots;
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

    for sample in &policy_state.terminal_samples {
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

    PluginRequestQueueStats {
        kind,
        pending_count: queued_count + scheduled_count,
        queued_count,
        scheduled_count,
        running_count,
        recent_count,
        max_queue_depth,
        active_global: policy_state.active_global,
        active_runs: policy_state.active_by_run.len() as u32,
        active_plugins: policy_state.active_by_plugin.len() as u32,
        active_hosts,
        hottest_host,
        hottest_host_active,
        cancelled_run_count: policy_state.cancelled_runs.len() as u32,
        configured_max_queue_depth: policy.max_queue_depth as u32,
        configured_max_global_concurrent: policy.max_global_concurrent,
        configured_max_concurrent_per_host: policy.max_concurrent_per_host,
        configured_max_concurrent_per_run: policy.max_concurrent_per_run,
        configured_max_concurrent_per_plugin: policy.max_concurrent_per_plugin,
        rejected_total_count: policy_state
            .rejection_stats
            .cancelled_run
            .saturating_add(policy_state.rejection_stats.queue_limit)
            .saturating_add(policy_state.rejection_stats.run_pending_limit)
            .saturating_add(policy_state.rejection_stats.plugin_pending_limit),
        rejected_cancelled_run_count: policy_state.rejection_stats.cancelled_run,
        rejected_queue_limit_count: policy_state.rejection_stats.queue_limit,
        rejected_run_pending_limit_count: policy_state.rejection_stats.run_pending_limit,
        rejected_plugin_pending_limit_count: policy_state.rejection_stats.plugin_pending_limit,
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

    fn request(
        kind: PluginFetchPolicyKind,
        id: &str,
        run_id: &str,
        plugin_id: &str,
        host: &str,
    ) -> PluginRequestScheduleRequest {
        PluginRequestScheduleRequest {
            kind,
            request_id: id.to_string(),
            run_id: run_id.to_string(),
            plugin_id: plugin_id.to_string(),
            execution_context: "plugin_test".to_string(),
            method: "GET".to_string(),
            url: format!("https://{host}/{id}"),
            host: host.to_string(),
        }
    }

    fn test_policy() -> PluginFetchPolicy {
        PluginFetchPolicy {
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
        let kind = PluginFetchPolicyKind::PluginTestFetch;
        let policy = PluginFetchPolicy {
            max_pending_per_run: 1,
            max_global_concurrent: 0,
            ..test_policy()
        };
        let _ = enqueue_plugin_request(
            request(kind, "limit-run-1", "limit-run", "p", "a.test"),
            policy.clone(),
        )
        .expect("first request should enqueue");
        let error = enqueue_plugin_request(
            request(kind, "limit-run-2", "limit-run", "p", "b.test"),
            policy,
        )
        .expect_err("second request should exceed run pending limit");
        assert!(error.contains("run pending limit"));
    }

    #[tokio::test]
    async fn dispatches_round_robin_by_run() {
        let kind = PluginFetchPolicyKind::AgentFetch;
        let policy = test_policy();
        let rx1 = enqueue_plugin_request(
            request(kind, "rr-1", "run-a", "p", "a.test"),
            policy.clone(),
        )
        .unwrap();
        let mut rx2 = enqueue_plugin_request(
            request(kind, "rr-2", "run-a", "p", "b.test"),
            policy.clone(),
        )
        .unwrap();
        let rx3 = enqueue_plugin_request(
            request(kind, "rr-3", "run-b", "p", "c.test"),
            policy.clone(),
        )
        .unwrap();

        assert!(rx1.await.unwrap().is_ok());
        assert!(rx2.try_recv().is_err());
        assert!(rx3.await.unwrap().is_ok());
    }

    #[tokio::test]
    async fn cancels_all_pending_requests_for_run() {
        let kind = PluginFetchPolicyKind::MonitorFetch;
        let policy = PluginFetchPolicy {
            max_global_concurrent: 1,
            ..test_policy()
        };
        let _blocker = enqueue_plugin_request(
            request(kind, "cancel-blocker", "other-run", "p", "a.test"),
            policy.clone(),
        )
        .unwrap();
        let rx1 = enqueue_plugin_request(
            request(kind, "cancel-run-1", "cancel-run", "p", "a.test"),
            policy.clone(),
        )
        .unwrap();
        let rx2 = enqueue_plugin_request(
            request(kind, "cancel-run-2", "cancel-run", "p", "b.test"),
            policy,
        )
        .unwrap();

        let cancelled =
            cancel_plugin_requests_by_run(kind, "cancel-run", Some("run stopped".to_string()));
        assert_eq!(cancelled.len(), 2);
        assert!(rx1.await.unwrap().is_err());
        assert!(rx2.await.unwrap().is_err());
    }

    #[tokio::test]
    async fn rejects_new_requests_for_cancelled_run() {
        let kind = PluginFetchPolicyKind::BountyFetch;
        let policy = test_policy();
        let _ =
            cancel_plugin_requests_by_run(kind, "already-cancelled", Some("stopped".to_string()));
        let error = enqueue_plugin_request(
            request(kind, "cancelled-new-1", "already-cancelled", "p", "a.test"),
            policy,
        )
        .expect_err("new request for cancelled run should be rejected");
        assert!(error.contains("cancelled"));
    }

    #[tokio::test]
    async fn tracks_rejection_reason_counters() {
        let kind = PluginFetchPolicyKind::TrafficActiveProbe;

        let queue_limit_policy = PluginFetchPolicy {
            max_queue_depth: 1,
            max_global_concurrent: 0,
            ..test_policy()
        };
        let _ = enqueue_plugin_request(
            request(
                kind,
                "reject-queue-1",
                "queue-run-a",
                "queue-plugin-a",
                "queue-a.test",
            ),
            queue_limit_policy.clone(),
        )
        .expect("first request should enqueue");
        let queue_error = enqueue_plugin_request(
            request(
                kind,
                "reject-queue-2",
                "queue-run-b",
                "queue-plugin-b",
                "queue-b.test",
            ),
            queue_limit_policy,
        )
        .expect_err("second request should exceed queue limit");
        assert!(queue_error.contains("queue limit"));

        let run_limit_policy = PluginFetchPolicy {
            max_pending_per_run: 1,
            max_global_concurrent: 0,
            ..test_policy()
        };
        let _ = enqueue_plugin_request(
            request(
                kind,
                "reject-run-1",
                "run-limit",
                "run-plugin-a",
                "run-a.test",
            ),
            run_limit_policy.clone(),
        )
        .expect("first run-limited request should enqueue");
        let run_error = enqueue_plugin_request(
            request(
                kind,
                "reject-run-2",
                "run-limit",
                "run-plugin-b",
                "run-b.test",
            ),
            run_limit_policy,
        )
        .expect_err("second request should exceed run pending limit");
        assert!(run_error.contains("run pending limit"));

        let plugin_limit_policy = PluginFetchPolicy {
            max_pending_per_plugin: 1,
            max_global_concurrent: 0,
            ..test_policy()
        };
        let _ = enqueue_plugin_request(
            request(
                kind,
                "reject-plugin-1",
                "plugin-run-a",
                "shared-plugin-limit",
                "plugin-a.test",
            ),
            plugin_limit_policy.clone(),
        )
        .expect("first plugin-limited request should enqueue");
        let plugin_error = enqueue_plugin_request(
            request(
                kind,
                "reject-plugin-2",
                "plugin-run-b",
                "shared-plugin-limit",
                "plugin-b.test",
            ),
            plugin_limit_policy,
        )
        .expect_err("second request should exceed plugin pending limit");
        assert!(plugin_error.contains("plugin pending limit"));

        let _ =
            cancel_plugin_requests_by_run(kind, "rejected-cancelled-run", Some("stop".to_string()));
        let cancelled_error = enqueue_plugin_request(
            request(
                kind,
                "reject-cancelled-1",
                "rejected-cancelled-run",
                "cancelled-plugin",
                "cancelled.test",
            ),
            test_policy(),
        )
        .expect_err("request for cancelled run should be rejected");
        assert!(cancelled_error.contains("cancelled"));

        assert!(cancel_plugin_request(
            kind,
            "reject-queue-1",
            Some("cleanup queue limit sample".to_string())
        )
        .is_some());
        assert!(cancel_plugin_request(
            kind,
            "reject-run-1",
            Some("cleanup run limit sample".to_string())
        )
        .is_some());
        assert!(cancel_plugin_request(
            kind,
            "reject-plugin-1",
            Some("cleanup plugin limit sample".to_string())
        )
        .is_some());

        let throughput_policy = PluginFetchPolicy {
            max_global_concurrent: 1,
            ..test_policy()
        };

        let success_rx = enqueue_plugin_request(
            request(
                kind,
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
        assert!(mark_plugin_request_running(kind, "throughput-complete").is_some());
        assert!(
            complete_plugin_request(kind, "throughput-complete", Some(200), Some(120)).is_some()
        );

        let fail_rx = enqueue_plugin_request(
            request(
                kind,
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
        assert!(mark_plugin_request_running(kind, "throughput-fail").is_some());
        assert!(fail_plugin_request(
            kind,
            "throughput-fail",
            Some(500),
            Some("boom".to_string()),
            Some(250)
        )
        .is_some());

        let blocker_rx = enqueue_plugin_request(
            request(
                kind,
                "throughput-blocker",
                "throughput-run-blocker",
                "throughput-plugin-blocker",
                "throughput-blocker.test",
            ),
            throughput_policy.clone(),
        )
        .expect("blocker should enqueue");
        assert!(blocker_rx.await.unwrap().is_ok());
        assert!(mark_plugin_request_running(kind, "throughput-blocker").is_some());

        let cancelled_pending_rx = enqueue_plugin_request(
            request(
                kind,
                "throughput-cancelled",
                "throughput-run-cancelled",
                "throughput-plugin-cancelled",
                "throughput-cancelled.test",
            ),
            throughput_policy,
        )
        .expect("cancelled pending request should enqueue");
        assert!(cancel_plugin_request(
            kind,
            "throughput-cancelled",
            Some("cancel throughput".to_string())
        )
        .is_some());
        assert!(cancelled_pending_rx.await.unwrap().is_err());
        assert!(complete_plugin_request(kind, "throughput-blocker", Some(200), Some(50)).is_some());

        let stats = get_plugin_request_queue_stats(kind);
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
}
