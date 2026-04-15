use chrono::{Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::sleep;

const DEFAULT_INITIAL_BACKOFF_SECS: u64 = 5;
const DEFAULT_MAX_BACKOFF_SECS: u64 = 180;
const QUOTA_BACKOFF_FALLBACK_SECS: u64 = 300;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmThrottleSnapshot {
    pub paused_until: Option<String>,
    pub consecutive_rate_limits: u32,
    pub current_backoff_secs: u64,
    pub last_error: Option<String>,
}

#[derive(Debug, Default)]
struct LlmThrottleState {
    paused_until: Option<Instant>,
    paused_until_wall: Option<String>,
    consecutive_rate_limits: u32,
    last_error: Option<String>,
}

#[derive(Clone, Default)]
pub struct SharedLlmThrottle {
    inner: Arc<Mutex<LlmThrottleState>>,
}

impl SharedLlmThrottle {
    pub async fn wait_for_clearance(&self) -> u64 {
        let mut slept_secs = 0_u64;

        loop {
            let delay = {
                let mut guard = self.inner.lock().await;
                match guard.paused_until {
                    Some(paused_until) => {
                        let now = Instant::now();
                        if paused_until > now {
                            Some(paused_until.duration_since(now))
                        } else {
                            guard.paused_until = None;
                            guard.paused_until_wall = None;
                            None
                        }
                    }
                    None => None,
                }
            };

            let Some(delay) = delay else {
                break;
            };

            slept_secs += delay.as_secs().max(1);
            sleep(delay).await;
        }

        slept_secs
    }

    pub async fn register_rate_limit(&self, error: impl Into<String>) -> LlmThrottleSnapshot {
        let error = error.into();
        let mut guard = self.inner.lock().await;
        guard.consecutive_rate_limits = guard.consecutive_rate_limits.saturating_add(1);

        let backoff_secs = compute_backoff_secs(&error, guard.consecutive_rate_limits);
        let paused_until_wall =
            (Utc::now() + ChronoDuration::seconds(backoff_secs as i64)).to_rfc3339();

        guard.paused_until = Some(Instant::now() + Duration::from_secs(backoff_secs));
        guard.paused_until_wall = Some(paused_until_wall);
        guard.last_error = Some(error);
        snapshot_from_state(&guard)
    }

    pub async fn register_success(&self) -> LlmThrottleSnapshot {
        let mut guard = self.inner.lock().await;
        guard.consecutive_rate_limits = 0;
        guard.paused_until = None;
        guard.paused_until_wall = None;
        snapshot_from_state(&guard)
    }

    pub async fn snapshot(&self) -> LlmThrottleSnapshot {
        let mut guard = self.inner.lock().await;
        if let Some(paused_until) = guard.paused_until {
            if paused_until <= Instant::now() {
                guard.paused_until = None;
                guard.paused_until_wall = None;
            }
        }
        snapshot_from_state(&guard)
    }
}

fn compute_backoff_secs(error: &str, consecutive_rate_limits: u32) -> u64 {
    if let Some(reset_secs) = parse_quota_reset_delay_secs(error) {
        return reset_secs.max(QUOTA_BACKOFF_FALLBACK_SECS);
    }

    let normalized = error.to_ascii_lowercase();
    if normalized.contains("accountquotaexceeded")
        || normalized.contains("usage quota")
        || normalized.contains("quota")
    {
        return QUOTA_BACKOFF_FALLBACK_SECS;
    }

    let exponent = consecutive_rate_limits.saturating_sub(1).min(4);
    let multiplier = 1_u64 << exponent;
    (DEFAULT_INITIAL_BACKOFF_SECS * multiplier)
        .min(DEFAULT_MAX_BACKOFF_SECS)
        .saturating_add((consecutive_rate_limits as u64).min(3))
}

fn parse_quota_reset_delay_secs(error: &str) -> Option<u64> {
    let marker = "reset at ";
    let start = error.to_ascii_lowercase().find(marker)?;
    let tail = &error[start + marker.len()..];
    let end = tail.find('.').unwrap_or(tail.len());
    let timestamp = tail[..end].trim();
    let parsed = chrono::DateTime::parse_from_str(timestamp, "%Y-%m-%d %H:%M:%S %z %Z").ok()?;
    let diff = parsed.with_timezone(&Utc) - Utc::now();
    Some(diff.num_seconds().max(0) as u64)
}

fn snapshot_from_state(state: &LlmThrottleState) -> LlmThrottleSnapshot {
    let current_backoff_secs = state
        .paused_until
        .and_then(|paused_until| paused_until.checked_duration_since(Instant::now()))
        .map(|remaining| remaining.as_secs().max(1))
        .unwrap_or(0);

    LlmThrottleSnapshot {
        paused_until: state.paused_until_wall.clone(),
        consecutive_rate_limits: state.consecutive_rate_limits,
        current_backoff_secs,
        last_error: state.last_error.clone(),
    }
}
