//! AuthAgent - Handles authentication detection and login flows
//!
//! Responsibilities:
//! - Detect if current page requires authentication
//! - Manage login takeover flow (wait for user credentials)
//! - Detect login success/failure
//! - Manage session state

use crate::engines::vision_explorer_v2::agent_framework::{Agent, AgentMetadata, AgentMetrics, AgentStatus};
use crate::engines::vision_explorer_v2::blackboard::Blackboard;
use crate::engines::vision_explorer_v2::core::{Event, PageContext, TaskResult};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Patterns that indicate a login page
const LOGIN_PATTERNS: &[&str] = &[
    "login",
    "signin",
    "sign-in",
    "sign_in",
    "authenticate",
    "auth",
    "password",
    "username",
    "credentials",
];

/// Patterns that indicate login success
const LOGIN_SUCCESS_PATTERNS: &[&str] = &[
    "dashboard",
    "home",
    "welcome",
    "profile",
    "account",
    "logout", // If we see logout, we're logged in
];

/// Patterns that indicate login failure
const LOGIN_FAILURE_PATTERNS: &[&str] = &[
    "invalid",
    "incorrect",
    "failed",
    "error",
    "wrong password",
    "try again",
];

#[derive(Debug)]
pub struct AuthAgent {
    id: String,
    blackboard: Arc<Blackboard>,
    event_tx: mpsc::Sender<Event>,
}

impl AuthAgent {
    pub fn new(id: String, blackboard: Arc<Blackboard>, event_tx: mpsc::Sender<Event>) -> Self {
        Self {
            id,
            blackboard,
            event_tx,
        }
    }

    /// Detect if the current page is a login page
    pub fn is_login_page(context: &PageContext) -> bool {
        let url_lower = context.url.to_lowercase();
        let dom_lower = context.dom_snapshot.to_lowercase();
        let title_lower = context.title.to_lowercase();

        // Check URL
        for pattern in LOGIN_PATTERNS {
            if url_lower.contains(pattern) {
                return true;
            }
        }

        // Check title
        for pattern in LOGIN_PATTERNS {
            if title_lower.contains(pattern) {
                return true;
            }
        }

        // Check DOM for login form indicators
        let has_password_field = dom_lower.contains("type=\"password\"")
            || dom_lower.contains("type='password'")
            || dom_lower.contains("password");

        let has_username_field = dom_lower.contains("type=\"email\"")
            || dom_lower.contains("type='email'")
            || dom_lower.contains("username")
            || dom_lower.contains("email");

        let has_login_button = dom_lower.contains("login")
            || dom_lower.contains("sign in")
            || dom_lower.contains("signin")
            || dom_lower.contains("submit");

        has_password_field && (has_username_field || has_login_button)
    }

    /// Detect if login was successful based on page transition
    pub fn detect_login_success(before: &PageContext, after: &PageContext) -> bool {
        let was_login = Self::is_login_page(before);
        let is_still_login = Self::is_login_page(after);

        if was_login && !is_still_login {
            // Moved away from login page
            let after_lower = format!("{}{}", after.url.to_lowercase(), after.title.to_lowercase());

            for pattern in LOGIN_SUCCESS_PATTERNS {
                if after_lower.contains(pattern) {
                    return true;
                }
            }

            // URL changed significantly
            if before.url != after.url {
                return true;
            }
        }

        false
    }

    /// Detect if login failed
    pub fn detect_login_failure(context: &PageContext) -> bool {
        let dom_lower = context.dom_snapshot.to_lowercase();

        for pattern in LOGIN_FAILURE_PATTERNS {
            if dom_lower.contains(pattern) {
                return true;
            }
        }

        false
    }

    /// Extract login form selectors from the page
    pub fn extract_login_selectors(context: &PageContext) -> Option<LoginFormSelectors> {
        // This is a simplified version - in production, use more sophisticated parsing
        let dom_lower = context.dom_snapshot.to_lowercase();

        // Try to find common patterns
        let username_selector = if dom_lower.contains("id=\"username\"") {
            Some("#username".to_string())
        } else if dom_lower.contains("id=\"email\"") {
            Some("#email".to_string())
        } else if dom_lower.contains("name=\"username\"") {
            Some("[name=\"username\"]".to_string())
        } else if dom_lower.contains("name=\"email\"") {
            Some("[name=\"email\"]".to_string())
        } else if dom_lower.contains("type=\"email\"") {
            Some("[type=\"email\"]".to_string())
        } else {
            None
        };

        let password_selector = if dom_lower.contains("id=\"password\"") {
            Some("#password".to_string())
        } else if dom_lower.contains("name=\"password\"") {
            Some("[name=\"password\"]".to_string())
        } else if dom_lower.contains("type=\"password\"") {
            Some("[type=\"password\"]".to_string())
        } else {
            None
        };

        let submit_selector = if dom_lower.contains("type=\"submit\"") {
            Some("[type=\"submit\"]".to_string())
        } else if dom_lower.contains("id=\"login\"") {
            Some("#login".to_string())
        } else if dom_lower.contains("id=\"submit\"") {
            Some("#submit".to_string())
        } else {
            Some(
                "button[type=\"submit\"], input[type=\"submit\"], button:contains('Login')"
                    .to_string(),
            )
        };

        if username_selector.is_some() && password_selector.is_some() {
            Some(LoginFormSelectors {
                username: username_selector.unwrap(),
                password: password_selector.unwrap(),
                submit: submit_selector.unwrap_or_default(),
            })
        } else {
            None
        }
    }
}

#[async_trait]
impl Agent for AuthAgent {
    fn metadata(&self) -> AgentMetadata {
        AgentMetadata {
            id: self.id.clone(),
            name: "Authentication Agent".to_string(),
            description: "Handles authentication detection and login flows".to_string(),
            version: "1.0.0".to_string(),
            tags: vec!["auth".to_string(), "login".to_string(), "security".to_string()],
        }
    }

    fn status(&self) -> AgentStatus {
        AgentStatus::Idle
    }

    fn metrics(&self) -> AgentMetrics {
        AgentMetrics::default()
    }

    async fn handle_event(&self, event: &Event) -> Result<Vec<Event>> {
        match event {
            Event::TaskAssigned {
                agent_id,
                task_id,
                target_node_id,
                payload,
            } if agent_id == &self.id => {
                let mut success = true;
                let mut message = "No context provided".to_string();
                let mut is_login = false;
                
                // Parse context from payload
                if let Some(val) = payload {
                    if let Ok(context) = serde_json::from_value::<PageContext>(val.clone()) {
                        is_login = Self::is_login_page(&context);

                        if is_login {
                            log::info!("AuthAgent detected login page at {}", context.url);

                            // Store login URL
                            self.blackboard.set_login_url(context.url.clone()).await;

                            // Check if we have credentials
                            if let Some(creds) = self.blackboard.get_credentials().await {
                                // Try to extract login form selectors
                                if let Some(selectors) = Self::extract_login_selectors(&context) {
                                    log::info!(
                                        "AuthAgent will attempt login with stored credentials"
                                    );

                                    // Emit task for Operator to fill the form
                                    let fill_payload = serde_json::json!({
                                        "operation": "fill_form",
                                        "data": {
                                            selectors.username: creds.username,
                                            selectors.password: creds.password,
                                        }
                                    });

                                    if let Err(e) = self.event_tx
                                        .send(Event::TaskAssigned {
                                            agent_id: "operator_1".to_string(),
                                            task_id: uuid::Uuid::new_v4().to_string(),
                                            target_node_id: target_node_id.clone(),
                                            payload: Some(fill_payload),
                                        })
                                        .await
                                    {
                                        log::error!("AuthAgent: Failed to send fill_form task: {}", e);
                                        success = false;
                                        message = format!("Failed to send fill_form task: {}", e);
                                    }

                                    // Then click submit
                                    let click_payload = serde_json::json!({
                                        "action_type": "click",
                                        "selector": selectors.submit,
                                        "description": "Submit login form"
                                    });

                                    if let Err(e) = self.event_tx
                                        .send(Event::TaskAssigned {
                                            agent_id: "navigator_1".to_string(),
                                            task_id: uuid::Uuid::new_v4().to_string(),
                                            target_node_id: target_node_id.clone(),
                                            payload: Some(click_payload),
                                        })
                                        .await
                                    {
                                        log::error!("AuthAgent: Failed to send click task: {}", e);
                                        success = false;
                                        message = format!("Failed to send click task: {}", e);
                                    }
                                } else {
                                    log::warn!("AuthAgent: Could not extract login form selectors");
                                    message = "Login page detected but could not extract form selectors".to_string();
                                }
                            } else {
                                log::info!(
                                    "AuthAgent: No credentials available, waiting for takeover"
                                );
                                // Emit event to notify UI that login is required
                                let _ = self.event_tx
                                    .send(Event::Log {
                                        level: "info".to_string(),
                                        message: "Login required. Please provide credentials."
                                            .to_string(),
                                    })
                                    .await;
                                message = "Login page detected, waiting for credentials".to_string();
                            }
                        } else {
                            message = "Not a login page".to_string();
                        }
                    } else {
                        log::error!("AuthAgent: Invalid PageContext payload");
                        success = false;
                        message = "Invalid PageContext payload".to_string();
                    }
                } else {
                    log::error!("AuthAgent: No payload provided");
                    success = false;
                    message = "No payload provided".to_string();
                }

                // Create and return TaskCompleted event
                let task_completed = Event::TaskCompleted {
                    agent_id: self.id.clone(),
                    task_id: task_id.clone(),
                    result: TaskResult {
                        success,
                        message,
                        new_nodes: vec![],
                        data: Some(serde_json::json!({
                            "is_login_page": is_login,
                            "is_authenticated": self.blackboard.is_authenticated().await,
                        })),
                    },
                };

                // Send other events and return task completed
                if let Err(e) = self.event_tx.send(task_completed.clone()).await {
                    log::error!("AuthAgent: Failed to send TaskCompleted: {}", e);
                }

                Ok(vec![task_completed])
            }
            _ => Ok(vec![]),
        }
    }
}

/// Selectors for a login form
#[derive(Debug, Clone)]
pub struct LoginFormSelectors {
    pub username: String,
    pub password: String,
    pub submit: String,
}
