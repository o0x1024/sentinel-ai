//! Error Recovery - Retry and fallback mechanisms
//!
//! This module provides structured error handling and recovery strategies
//! to make the system more resilient to transient failures.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, warn};

/// Fallback strategy when an operation fails after retries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FallbackStrategy {
    /// Skip the current task and continue
    Skip,
    /// Go back to previous state
    Backtrack,
    /// Request user intervention
    RequestHelp,
    /// Abort the entire exploration
    Abort,
}

/// Error recovery policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecoveryPolicy {
    /// Maximum number of retries for transient errors
    pub max_retries: u32,
    /// Delay between retries in milliseconds
    pub retry_delay_ms: u64,
    /// Backoff multiplier (1.0 = no backoff, 2.0 = exponential)
    pub backoff_multiplier: f64,
    /// Maximum retry delay in milliseconds
    pub max_retry_delay_ms: u64,
    /// Strategy to use when retries are exhausted
    pub fallback_strategy: FallbackStrategy,
}

impl Default for ErrorRecoveryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay_ms: 1000,
            backoff_multiplier: 2.0,
            max_retry_delay_ms: 10000,
            fallback_strategy: FallbackStrategy::Skip,
        }
    }
}

/// Error classification for recovery decisions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorType {
    /// Transient errors that may succeed on retry (network timeout, etc.)
    Transient,
    /// Permanent errors that won't be fixed by retrying (404, invalid selector, etc.)
    Permanent,
    /// Critical errors that require immediate abort
    Critical,
}

/// Error recovery context tracking
pub struct ErrorRecoveryContext {
    policy: ErrorRecoveryPolicy,
    attempt_count: u32,
}

impl ErrorRecoveryContext {
    /// Create a new recovery context with the given policy
    pub fn new(policy: ErrorRecoveryPolicy) -> Self {
        Self {
            policy,
            attempt_count: 0,
        }
    }

    /// Execute an operation with automatic retry and recovery
    pub async fn execute_with_recovery<F, T, E>(
        &mut self,
        operation_name: &str,
        operation: F,
    ) -> Result<T>
    where
        F: Fn() -> futures::future::BoxFuture<'static, Result<T, E>>,
        E: std::fmt::Display + std::fmt::Debug,
    {
        self.attempt_count = 0;

        loop {
            self.attempt_count += 1;

            match operation().await {
                Ok(result) => {
                    if self.attempt_count > 1 {
                        debug!(
                            "Operation '{}' succeeded after {} attempts",
                            operation_name, self.attempt_count
                        );
                    }
                    return Ok(result);
                }
                Err(e) => {
                    let error_type = classify_error(&e);

                    match error_type {
                        ErrorType::Critical => {
                            warn!("Critical error in '{}': {:?}", operation_name, e);
                            return Err(anyhow!("Critical error: {}", e));
                        }
                        ErrorType::Permanent => {
                            warn!("Permanent error in '{}': {:?}", operation_name, e);
                            return Err(anyhow!("Permanent error: {}", e));
                        }
                        ErrorType::Transient => {
                            if self.attempt_count >= self.policy.max_retries {
                                warn!(
                                    "Operation '{}' failed after {} attempts: {:?}",
                                    operation_name, self.attempt_count, e
                                );
                                return self.apply_fallback_strategy(operation_name);
                            }

                            let delay = self.calculate_retry_delay();
                            debug!(
                                "Transient error in '{}' (attempt {}), retrying in {}ms: {:?}",
                                operation_name, self.attempt_count, delay, e
                            );
                            tokio::time::sleep(Duration::from_millis(delay)).await;
                        }
                    }
                }
            }
        }
    }

    /// Calculate retry delay with exponential backoff
    fn calculate_retry_delay(&self) -> u64 {
        let base_delay = self.policy.retry_delay_ms as f64;
        let multiplier = self.policy.backoff_multiplier;
        let attempt = (self.attempt_count - 1) as f64;

        let delay = base_delay * multiplier.powf(attempt);
        let delay = delay.min(self.policy.max_retry_delay_ms as f64);

        delay as u64
    }

    /// Apply the configured fallback strategy
    fn apply_fallback_strategy<T>(&self, operation_name: &str) -> Result<T> {
        match self.policy.fallback_strategy {
            FallbackStrategy::Skip => {
                warn!("Skipping failed operation '{}'", operation_name);
                Err(anyhow!("Operation skipped after retries exhausted"))
            }
            FallbackStrategy::Backtrack => {
                warn!("Backtracking after failed operation '{}'", operation_name);
                Err(anyhow!("Operation failed, backtracking required"))
            }
            FallbackStrategy::RequestHelp => {
                warn!(
                    "Requesting user help for failed operation '{}'",
                    operation_name
                );
                Err(anyhow!("Operation failed, user intervention required"))
            }
            FallbackStrategy::Abort => {
                warn!("Aborting after failed operation '{}'", operation_name);
                Err(anyhow!("Operation failed, aborting exploration"))
            }
        }
    }

    /// Reset attempt counter
    pub fn reset(&mut self) {
        self.attempt_count = 0;
    }

    /// Get current attempt count
    pub fn attempt_count(&self) -> u32 {
        self.attempt_count
    }
}

/// Classify an error to determine if it should be retried
fn classify_error<E: std::fmt::Debug>(error: &E) -> ErrorType {
    let error_str = format!("{:?}", error).to_lowercase();

    // Critical errors
    if error_str.contains("out of memory")
        || error_str.contains("browser crashed")
        || error_str.contains("fatal")
    {
        return ErrorType::Critical;
    }

    // Permanent errors
    if error_str.contains("not found")
        || error_str.contains("404")
        || error_str.contains("invalid selector")
        || error_str.contains("element not found")
        || error_str.contains("permission denied")
    {
        return ErrorType::Permanent;
    }

    // Default to transient
    ErrorType::Transient
}

/// Convenience function to create a default recovery context
pub fn default_recovery_context() -> ErrorRecoveryContext {
    ErrorRecoveryContext::new(ErrorRecoveryPolicy::default())
}

/// Convenience function to create a permissive recovery context (more retries, longer delays)
pub fn permissive_recovery_context() -> ErrorRecoveryContext {
    ErrorRecoveryContext::new(ErrorRecoveryPolicy {
        max_retries: 5,
        retry_delay_ms: 2000,
        backoff_multiplier: 1.5,
        max_retry_delay_ms: 30000,
        fallback_strategy: FallbackStrategy::Skip,
    })
}

/// Convenience function to create a strict recovery context (fewer retries, fail fast)
pub fn strict_recovery_context() -> ErrorRecoveryContext {
    ErrorRecoveryContext::new(ErrorRecoveryPolicy {
        max_retries: 1,
        retry_delay_ms: 500,
        backoff_multiplier: 1.0,
        max_retry_delay_ms: 500,
        fallback_strategy: FallbackStrategy::Abort,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_successful_operation() {
        let mut ctx = default_recovery_context();
        let result = ctx
            .execute_with_recovery("test", || {
                Box::pin(async { Ok::<_, String>(42) })
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(ctx.attempt_count(), 1);
    }

    #[tokio::test]
    async fn test_retry_on_transient_error() {
        let mut ctx = ErrorRecoveryContext::new(ErrorRecoveryPolicy {
            max_retries: 3,
            retry_delay_ms: 10,
            backoff_multiplier: 1.0,
            max_retry_delay_ms: 100,
            fallback_strategy: FallbackStrategy::Skip,
        });

        let counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = ctx
            .execute_with_recovery("test", move || {
                let counter = counter_clone.clone();
                Box::pin(async move {
                    let count = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    if count < 2 {
                        Err("timeout")
                    } else {
                        Ok(42)
                    }
                })
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(ctx.attempt_count(), 3);
    }

    #[tokio::test]
    async fn test_permanent_error_no_retry() {
        let mut ctx = default_recovery_context();
        let result = ctx
            .execute_with_recovery("test", || {
                Box::pin(async { Err::<i32, _>("404 not found") })
            })
            .await;

        assert!(result.is_err());
        assert_eq!(ctx.attempt_count(), 1);
    }

    #[test]
    fn test_error_classification() {
        assert_eq!(classify_error(&"timeout"), ErrorType::Transient);
        assert_eq!(classify_error(&"404 not found"), ErrorType::Permanent);
        assert_eq!(classify_error(&"out of memory"), ErrorType::Critical);
    }

    #[test]
    fn test_exponential_backoff() {
        let mut ctx = ErrorRecoveryContext::new(ErrorRecoveryPolicy {
            max_retries: 5,
            retry_delay_ms: 100,
            backoff_multiplier: 2.0,
            max_retry_delay_ms: 1000,
            fallback_strategy: FallbackStrategy::Skip,
        });

        ctx.attempt_count = 1;
        assert_eq!(ctx.calculate_retry_delay(), 100); // 100 * 2^0

        ctx.attempt_count = 2;
        assert_eq!(ctx.calculate_retry_delay(), 200); // 100 * 2^1

        ctx.attempt_count = 3;
        assert_eq!(ctx.calculate_retry_delay(), 400); // 100 * 2^2

        ctx.attempt_count = 4;
        assert_eq!(ctx.calculate_retry_delay(), 800); // 100 * 2^3

        ctx.attempt_count = 5;
        assert_eq!(ctx.calculate_retry_delay(), 1000); // Capped at max_retry_delay_ms
    }
}
