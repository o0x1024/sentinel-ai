//! Workflow Step Retry & Rate Limiting (P0-3)
//!
//! Provides unified retry logic with exponential backoff and concurrency control.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Semaphore, RwLock};
use tokio::time::sleep;

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum retry attempts (default: 3)
    pub max_attempts: u32,
    /// Initial delay in milliseconds (default: 1000)
    pub initial_delay_ms: u64,
    /// Maximum delay in milliseconds (default: 30000)
    pub max_delay_ms: u64,
    /// Backoff strategy
    pub backoff: BackoffStrategy,
    /// Jitter factor (0.0 - 1.0, default: 0.1)
    pub jitter: f64,
    /// Retryable error patterns
    pub retryable_errors: Vec<String>,
    /// Non-retryable error patterns
    pub non_retryable_errors: Vec<String>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff: BackoffStrategy::Exponential { multiplier: 2.0 },
            jitter: 0.1,
            retryable_errors: vec![
                "timeout".to_string(),
                "connection refused".to_string(),
                "connection reset".to_string(),
                "temporary failure".to_string(),
                "rate limit".to_string(),
                "429".to_string(),
                "503".to_string(),
                "504".to_string(),
            ],
            non_retryable_errors: vec![
                "invalid argument".to_string(),
                "not found".to_string(),
                "unauthorized".to_string(),
                "forbidden".to_string(),
                "400".to_string(),
                "401".to_string(),
                "403".to_string(),
                "404".to_string(),
            ],
        }
    }
}

/// Backoff strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BackoffStrategy {
    /// Fixed delay
    Fixed,
    /// Linear increase: delay = initial + (attempt * increment)
    Linear { increment_ms: u64 },
    /// Exponential increase: delay = initial * (multiplier ^ attempt)
    Exponential { multiplier: f64 },
}

impl RetryConfig {
    /// Create a config for network operations (more retries, longer delays)
    pub fn for_network() -> Self {
        Self {
            max_attempts: 5,
            initial_delay_ms: 2000,
            max_delay_ms: 60000,
            backoff: BackoffStrategy::Exponential { multiplier: 2.0 },
            jitter: 0.2,
            ..Default::default()
        }
    }
    
    /// Create a config for fast operations (fewer retries, shorter delays)
    pub fn for_fast() -> Self {
        Self {
            max_attempts: 2,
            initial_delay_ms: 500,
            max_delay_ms: 5000,
            backoff: BackoffStrategy::Fixed,
            jitter: 0.1,
            ..Default::default()
        }
    }
    
    /// Calculate delay for given attempt number
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let base_delay = match &self.backoff {
            BackoffStrategy::Fixed => self.initial_delay_ms,
            BackoffStrategy::Linear { increment_ms } => {
                self.initial_delay_ms + (attempt as u64 * increment_ms)
            }
            BackoffStrategy::Exponential { multiplier } => {
                (self.initial_delay_ms as f64 * multiplier.powi(attempt as i32)) as u64
            }
        };
        
        // Apply max delay cap
        let capped_delay = base_delay.min(self.max_delay_ms);
        
        // Apply jitter
        let jitter_range = (capped_delay as f64 * self.jitter) as u64;
        let jitter_offset = if jitter_range > 0 {
            (rand_simple() * jitter_range as f64) as u64
        } else {
            0
        };
        
        Duration::from_millis(capped_delay + jitter_offset)
    }
    
    /// Check if an error is retryable
    pub fn is_retryable(&self, error: &str) -> bool {
        let error_lower = error.to_lowercase();
        
        // Check non-retryable first
        for pattern in &self.non_retryable_errors {
            if error_lower.contains(&pattern.to_lowercase()) {
                return false;
            }
        }
        
        // Check retryable patterns
        for pattern in &self.retryable_errors {
            if error_lower.contains(&pattern.to_lowercase()) {
                return true;
            }
        }
        
        // Default: retry on unknown errors
        true
    }
}

/// Generate random number between 0.0 and 1.0
fn rand_simple() -> f64 {
    use rand::Rng;
    rand::thread_rng().gen::<f64>()
}

/// Rate limiter for controlling request concurrency
pub struct RateLimiter {
    /// Global concurrency semaphore
    global_semaphore: Arc<Semaphore>,
    /// Per-host semaphores
    host_semaphores: RwLock<HashMap<String, Arc<Semaphore>>>,
    /// Global concurrency limit
    global_limit: usize,
    /// Per-host concurrency limit
    per_host_limit: usize,
    /// Minimum delay between requests to same host (ms)
    per_host_delay_ms: u64,
    /// Last request times per host
    last_request_times: RwLock<HashMap<String, std::time::Instant>>,
}

impl RateLimiter {
    pub fn new(global_limit: usize, per_host_limit: usize, per_host_delay_ms: u64) -> Self {
        Self {
            global_semaphore: Arc::new(Semaphore::new(global_limit)),
            host_semaphores: RwLock::new(HashMap::new()),
            global_limit,
            per_host_limit,
            per_host_delay_ms,
            last_request_times: RwLock::new(HashMap::new()),
        }
    }
    
    /// Create with default limits (20 global, 5 per host, 100ms delay)
    pub fn default_limits() -> Self {
        Self::new(20, 5, 100)
    }
    
    /// Acquire permits for a request to given host
    pub async fn acquire(&self, host: &str) -> RateLimitGuard {
        // Acquire global permit
        let global_permit = self.global_semaphore.clone().acquire_owned().await.unwrap();
        
        // Get or create host semaphore
        let host_semaphore = {
            let mut semaphores = self.host_semaphores.write().await;
            semaphores.entry(host.to_string())
                .or_insert_with(|| Arc::new(Semaphore::new(self.per_host_limit)))
                .clone()
        };
        
        // Acquire host permit
        let host_permit = host_semaphore.acquire_owned().await.unwrap();
        
        // Apply per-host delay
        if self.per_host_delay_ms > 0 {
            let last_times = self.last_request_times.write().await;
            if let Some(last_time) = last_times.get(host) {
                let elapsed = last_time.elapsed().as_millis() as u64;
                if elapsed < self.per_host_delay_ms {
                    let wait = Duration::from_millis(self.per_host_delay_ms - elapsed);
                    drop(last_times);
                    sleep(wait).await;
                }
            }
        }
        
        // Update last request time
        {
            let mut last_times = self.last_request_times.write().await;
            last_times.insert(host.to_string(), std::time::Instant::now());
        }
        
        RateLimitGuard {
            _global_permit: global_permit,
            _host_permit: host_permit,
        }
    }
    
    /// Get current stats
    pub fn stats(&self) -> RateLimitStats {
        RateLimitStats {
            global_available: self.global_semaphore.available_permits(),
            global_limit: self.global_limit,
            per_host_limit: self.per_host_limit,
            per_host_delay_ms: self.per_host_delay_ms,
        }
    }
}

/// Guard that holds rate limit permits
pub struct RateLimitGuard {
    _global_permit: tokio::sync::OwnedSemaphorePermit,
    _host_permit: tokio::sync::OwnedSemaphorePermit,
}

/// Rate limit statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitStats {
    pub global_available: usize,
    pub global_limit: usize,
    pub per_host_limit: usize,
    pub per_host_delay_ms: u64,
}

/// Retry executor - executes operations with retry logic
pub struct RetryExecutor {
    config: RetryConfig,
    rate_limiter: Option<Arc<RateLimiter>>,
}

impl RetryExecutor {
    pub fn new(config: RetryConfig) -> Self {
        Self {
            config,
            rate_limiter: None,
        }
    }
    
    pub fn with_rate_limiter(mut self, limiter: Arc<RateLimiter>) -> Self {
        self.rate_limiter = Some(limiter);
        self
    }
    
    /// Execute an async operation with retry
    pub async fn execute<F, Fut, T, E>(&self, host: Option<&str>, operation: F) -> Result<T, RetryError<E>>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        let mut last_error = None;
        
        for attempt in 0..self.config.max_attempts {
            // Acquire rate limit if configured
            let _guard = if let (Some(limiter), Some(h)) = (&self.rate_limiter, host) {
                Some(limiter.acquire(h).await)
            } else {
                None
            };
            
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    let error_str = e.to_string();
                    tracing::warn!(
                        "Attempt {}/{} failed: {}",
                        attempt + 1,
                        self.config.max_attempts,
                        error_str
                    );
                    
                    // Check if retryable
                    if !self.config.is_retryable(&error_str) {
                        return Err(RetryError::NonRetryable(e));
                    }
                    
                    last_error = Some(e);
                    
                    // Don't sleep on last attempt
                    if attempt + 1 < self.config.max_attempts {
                        let delay = self.config.calculate_delay(attempt);
                        tracing::debug!("Waiting {:?} before retry", delay);
                        sleep(delay).await;
                    }
                }
            }
        }
        
        Err(RetryError::MaxRetriesExceeded {
            attempts: self.config.max_attempts,
            last_error: last_error.unwrap(),
        })
    }
    
    /// Execute with a specific timeout per attempt
    pub async fn execute_with_timeout<F, Fut, T, E>(
        &self,
        host: Option<&str>,
        timeout: Duration,
        operation: F,
    ) -> Result<T, RetryError<E>>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Display + From<TimeoutError>,
    {
        self.execute(host, || async {
            tokio::time::timeout(timeout, operation())
                .await
                .map_err(|_| E::from(TimeoutError))?
        }).await
    }
}

/// Retry error types
#[derive(Debug)]
pub enum RetryError<E> {
    /// Operation failed after max retries
    MaxRetriesExceeded { attempts: u32, last_error: E },
    /// Error is not retryable
    NonRetryable(E),
}

impl<E: std::fmt::Display> std::fmt::Display for RetryError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RetryError::MaxRetriesExceeded { attempts, last_error } => {
                write!(f, "Failed after {} attempts: {}", attempts, last_error)
            }
            RetryError::NonRetryable(e) => {
                write!(f, "Non-retryable error: {}", e)
            }
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for RetryError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RetryError::MaxRetriesExceeded { last_error, .. } => Some(last_error),
            RetryError::NonRetryable(e) => Some(e),
        }
    }
}

/// Timeout error
#[derive(Debug)]
pub struct TimeoutError;

impl std::fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Operation timed out")
    }
}

impl std::error::Error for TimeoutError {}

/// Step execution context with retry and rate limiting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepExecutionConfig {
    /// Step ID
    pub step_id: String,
    /// Retry configuration override (uses default if None)
    pub retry: Option<RetryConfig>,
    /// Timeout per attempt (seconds)
    pub timeout_secs: Option<u64>,
    /// Target host for rate limiting
    pub target_host: Option<String>,
    /// Whether to skip this step on upstream failure
    pub skip_on_upstream_failure: bool,
    /// Whether this step is critical (fail workflow on error)
    pub is_critical: bool,
}

impl Default for StepExecutionConfig {
    fn default() -> Self {
        Self {
            step_id: String::new(),
            retry: None,
            timeout_secs: Some(60),
            target_host: None,
            skip_on_upstream_failure: false,
            is_critical: false,
        }
    }
}

/// Step execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepExecutionResult {
    pub step_id: String,
    pub success: bool,
    pub attempts: u32,
    pub duration_ms: u64,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub was_rate_limited: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_backoff_calculation() {
        let config = RetryConfig {
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff: BackoffStrategy::Exponential { multiplier: 2.0 },
            jitter: 0.0, // No jitter for deterministic test
            ..Default::default()
        };
        
        // First attempt: 1000ms
        let delay0 = config.calculate_delay(0);
        assert_eq!(delay0.as_millis(), 1000);
        
        // Second attempt: 2000ms
        let delay1 = config.calculate_delay(1);
        assert_eq!(delay1.as_millis(), 2000);
        
        // Third attempt: 4000ms
        let delay2 = config.calculate_delay(2);
        assert_eq!(delay2.as_millis(), 4000);
    }
    
    #[test]
    fn test_is_retryable() {
        let config = RetryConfig::default();
        
        assert!(config.is_retryable("Connection timeout"));
        assert!(config.is_retryable("Rate limit exceeded (429)"));
        assert!(!config.is_retryable("Not found (404)"));
        assert!(!config.is_retryable("Unauthorized"));
    }
    
    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(2, 1, 0);
        
        let stats_before = limiter.stats();
        assert_eq!(stats_before.global_available, 2);
        
        let _guard1 = limiter.acquire("host1").await;
        let stats_during = limiter.stats();
        assert_eq!(stats_during.global_available, 1);
    }
}
