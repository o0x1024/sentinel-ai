use crate::error::ApiError;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Default)]
pub struct RateLimiter {
    inner: Arc<Mutex<HashMap<String, VecDeque<Instant>>>>,
}

impl RateLimiter {
    pub fn check(&self, key: impl Into<String>, limit_per_minute: u32) -> Result<(), ApiError> {
        let key = key.into();
        let now = Instant::now();
        let window = Duration::from_secs(60);
        let mut inner = self.inner.lock().map_err(|_| {
            ApiError::internal("rate_limit_lock_failed", "failed to acquire rate limiter")
        })?;
        let entries = inner.entry(key).or_default();

        while let Some(oldest) = entries.front() {
            if now.duration_since(*oldest) >= window {
                entries.pop_front();
            } else {
                break;
            }
        }

        if entries.len() >= limit_per_minute as usize {
            let retry_after_secs = entries
                .front()
                .map(|oldest| 60_u64.saturating_sub(now.duration_since(*oldest).as_secs()))
                .unwrap_or(60)
                .max(1);
            return Err(ApiError::too_many_requests(
                "admin_rate_limited",
                "admin request rate limit exceeded",
                retry_after_secs,
            ));
        }

        entries.push_back(now);
        Ok(())
    }
}
