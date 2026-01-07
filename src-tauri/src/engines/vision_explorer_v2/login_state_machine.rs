//! Login State Machine - Simplified login flow management
//!
//! This module implements a state machine for managing complex login workflows,
//! eliminating scattered login-related state in the Blackboard.

use crate::engines::vision_explorer_v2::core::{Event, LoginField};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

/// Login state representing the current phase of authentication
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoginState {
    /// Login is not required for this target
    NotRequired,

    /// Login page detected, waiting for user to decide
    Detected {
        url: String,
        fields: Vec<LoginField>,
    },

    /// Waiting for user to manually login in browser
    WaitingForUser {
        url: String,
        timeout: u64, // seconds
        started_at: u64, // Unix timestamp millis
    },

    /// User provided credentials, ready for auto-login
    CredentialsProvided {
        username: String,
        password: String,
        verification_code: Option<String>,
    },

    /// Attempting automatic login
    AutoLoginInProgress {
        url: String,
        attempt: u32,
    },

    /// Login completed successfully
    Completed {
        authenticated_at: u64, // Unix timestamp millis
    },

    /// Login failed
    Failed {
        reason: String,
        retry_count: u32,
    },

    /// User chose to skip login
    Skipped,
}

impl std::fmt::Display for LoginState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoginState::NotRequired => write!(f, "NotRequired"),
            LoginState::Detected { url, .. } => write!(f, "Detected({})", url),
            LoginState::WaitingForUser { .. } => write!(f, "WaitingForUser"),
            LoginState::CredentialsProvided { .. } => write!(f, "CredentialsProvided"),
            LoginState::AutoLoginInProgress { .. } => write!(f, "AutoLoginInProgress"),
            LoginState::Completed { .. } => write!(f, "Completed"),
            LoginState::Failed { reason, .. } => write!(f, "Failed({})", reason),
            LoginState::Skipped => write!(f, "Skipped"),
        }
    }
}

/// Login state machine
pub struct LoginStateMachine {
    current_state: LoginState,
}

impl LoginStateMachine {
    /// Create a new login state machine in NotRequired state
    pub fn new() -> Self {
        Self {
            current_state: LoginState::NotRequired,
        }
    }

    /// Get current state
    pub fn current_state(&self) -> &LoginState {
        &self.current_state
    }

    /// Detect login page
    pub fn detect_login(&mut self, url: String, fields: Vec<LoginField>) -> LoginState {
        debug!("Login detected at {}", url);
        self.current_state = LoginState::Detected {
            url: url.clone(),
            fields,
        };
        self.current_state.clone()
    }

    /// Start waiting for user login
    pub fn start_user_wait(&mut self, url: String, timeout: u64) -> LoginState {
        let started_at = current_unix_millis();
        debug!("Starting user login wait for {} with {}s timeout", url, timeout);
        self.current_state = LoginState::WaitingForUser {
            url,
            timeout,
            started_at,
        };
        self.current_state.clone()
    }

    /// Check if user login timeout has expired
    pub fn is_user_wait_expired(&self) -> bool {
        match &self.current_state {
            LoginState::WaitingForUser {
                timeout,
                started_at,
                ..
            } => {
                let now = current_unix_millis();
                let elapsed_ms = now.saturating_sub(*started_at);
                elapsed_ms > (*timeout as u64 * 1000)
            }
            _ => false,
        }
    }

    /// Get remaining time for user wait (in seconds)
    pub fn get_user_wait_remaining_seconds(&self) -> Option<u64> {
        match &self.current_state {
            LoginState::WaitingForUser {
                timeout,
                started_at,
                ..
            } => {
                let now = current_unix_millis();
                let elapsed_ms = now.saturating_sub(*started_at);
                let elapsed_s = elapsed_ms / 1000;
                let remaining = timeout.saturating_sub(elapsed_s);
                Some(remaining)
            }
            _ => None,
        }
    }

    /// Receive user credentials
    pub fn receive_credentials(
        &mut self,
        username: String,
        password: String,
        verification_code: Option<String>,
    ) -> LoginState {
        info!("Credentials received for user {}", username);
        self.current_state = LoginState::CredentialsProvided {
            username,
            password,
            verification_code,
        };
        self.current_state.clone()
    }

    /// Start auto-login attempt
    pub fn start_auto_login(&mut self, url: String) -> LoginState {
        let attempt = match &self.current_state {
            LoginState::AutoLoginInProgress { attempt, .. } => *attempt + 1,
            _ => 1,
        };
        debug!("Starting auto-login attempt {} at {}", attempt, url);
        self.current_state = LoginState::AutoLoginInProgress { url, attempt };
        self.current_state.clone()
    }

    /// Mark login as completed
    pub fn complete_login(&mut self) -> LoginState {
        let authenticated_at = current_unix_millis();
        info!("Login completed successfully");
        self.current_state = LoginState::Completed { authenticated_at };
        self.current_state.clone()
    }

    /// Mark login as failed
    pub fn fail_login(&mut self, reason: String) -> LoginState {
        let retry_count = match &self.current_state {
            LoginState::Failed { retry_count, .. } => *retry_count + 1,
            _ => 1,
        };
        warn!("Login failed: {} (retry count: {})", reason, retry_count);
        self.current_state = LoginState::Failed { reason, retry_count };
        self.current_state.clone()
    }

    /// User chose to skip login
    pub fn skip_login(&mut self) -> LoginState {
        info!("User chose to skip login");
        self.current_state = LoginState::Skipped;
        self.current_state.clone()
    }

    /// Check if login is completed or skipped
    pub fn is_login_resolved(&self) -> bool {
        matches!(
            self.current_state,
            LoginState::Completed { .. } | LoginState::Skipped | LoginState::Failed { .. }
        )
    }

    /// Check if login was successful
    pub fn is_authenticated(&self) -> bool {
        matches!(self.current_state, LoginState::Completed { .. })
    }

    /// Convert state to event if action is needed
    pub fn to_event_if_action_needed(&self) -> Option<Event> {
        match &self.current_state {
            LoginState::Detected { url, fields } => Some(Event::LoginTakeoverRequest {
                url: url.clone(),
                fields: fields.clone(),
            }),
            LoginState::WaitingForUser { .. } if self.is_user_wait_expired() => {
                if let LoginState::WaitingForUser { url, .. } = &self.current_state {
                    Some(Event::LoginTimeout {
                        url: url.clone(),
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl Default for LoginStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

/// Get current Unix timestamp in milliseconds
fn current_unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_state_transitions() {
        let mut sm = LoginStateMachine::new();
        assert_eq!(sm.current_state(), &LoginState::NotRequired);

        let url = "https://example.com/login".to_string();
        let fields = vec![];

        sm.detect_login(url.clone(), fields);
        assert!(matches!(sm.current_state(), LoginState::Detected { .. }));

        sm.receive_credentials(
            "user@example.com".to_string(),
            "password123".to_string(),
            None,
        );
        assert!(matches!(sm.current_state(), LoginState::CredentialsProvided { .. }));

        sm.start_auto_login(url);
        assert!(matches!(sm.current_state(), LoginState::AutoLoginInProgress { .. }));

        sm.complete_login();
        assert!(sm.is_authenticated());
    }

    #[test]
    fn test_user_wait_timeout() {
        let mut sm = LoginStateMachine::new();
        
        // Start with 1 second timeout
        sm.start_user_wait("https://example.com/login".to_string(), 1);
        assert!(!sm.is_user_wait_expired());

        // Check remaining time
        let remaining = sm.get_user_wait_remaining_seconds();
        assert!(remaining.is_some());
        assert!(remaining.unwrap() <= 1);
    }

    #[test]
    fn test_skip_login() {
        let mut sm = LoginStateMachine::new();
        sm.detect_login("https://example.com/login".to_string(), vec![]);
        sm.skip_login();

        assert!(sm.is_login_resolved());
        assert!(!sm.is_authenticated());
    }

    #[test]
    fn test_failed_login_retry_count() {
        let mut sm = LoginStateMachine::new();
        
        sm.fail_login("Invalid credentials".to_string());
        match sm.current_state() {
            LoginState::Failed { retry_count, .. } => assert_eq!(*retry_count, 1),
            _ => panic!("Expected Failed state"),
        }

        sm.fail_login("Network error".to_string());
        match sm.current_state() {
            LoginState::Failed { retry_count, .. } => assert_eq!(*retry_count, 2),
            _ => panic!("Expected Failed state"),
        }
    }
}
