//! Safety Layer - Prevents destructive actions during exploration
//!
//! This module provides safety checks before executing any action,
//! filtering out potentially dangerous operations like:
//! - Delete/Remove operations
//! - Logout/Sign out
//! - Payment/Purchase
//! - Account deletion
//! - Data export/download (in bulk)

use crate::engines::vision_explorer_v2::core::SuggestedAction;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Safety policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyPolicy {
    /// Whether safety checks are enabled
    pub enabled: bool,

    /// Block actions containing these keywords (case-insensitive)
    pub blocked_keywords: Vec<String>,

    /// Block actions on elements with these CSS classes
    pub blocked_classes: Vec<String>,

    /// Block actions on elements with these IDs
    pub blocked_ids: Vec<String>,

    /// Block URLs matching these patterns
    pub blocked_url_patterns: Vec<String>,

    /// Allowed actions even if they match blocked patterns (whitelist)
    pub allowed_overrides: Vec<String>,

    /// Maximum number of form submissions per session
    pub max_form_submissions: u32,

    /// Whether to allow actions on modal dialogs
    pub allow_modal_actions: bool,
}

impl Default for SafetyPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            blocked_keywords: vec![
                // Destructive actions
                "delete".to_string(),
                "remove".to_string(),
                "destroy".to_string(),
                "erase".to_string(),
                "purge".to_string(),
                "clear all".to_string(),
                "reset".to_string(),
                // Logout
                "logout".to_string(),
                "log out".to_string(),
                "sign out".to_string(),
                "signout".to_string(),
                // Payment
                "pay".to_string(),
                "purchase".to_string(),
                "buy".to_string(),
                "checkout".to_string(),
                "subscribe".to_string(),
                // Account management
                "deactivate".to_string(),
                "close account".to_string(),
                "cancel subscription".to_string(),
                // Email/Notifications
                "send".to_string(),
                "broadcast".to_string(),
                "notify all".to_string(),
            ],
            blocked_classes: vec![
                "btn-danger".to_string(),
                "btn-destructive".to_string(),
                "delete-btn".to_string(),
                "danger".to_string(),
            ],
            blocked_ids: vec![
                "delete".to_string(),
                "logout".to_string(),
                "signout".to_string(),
            ],
            blocked_url_patterns: vec![
                "/logout".to_string(),
                "/signout".to_string(),
                "/delete".to_string(),
                "/destroy".to_string(),
                "/unsubscribe".to_string(),
            ],
            allowed_overrides: vec![
                // Some legitimate actions that might match blocked keywords
                "delete draft".to_string(), // Often safe
            ],
            max_form_submissions: 10,
            allow_modal_actions: true,
        }
    }
}

/// Result of a safety check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyCheckResult {
    /// Whether the action is allowed
    pub allowed: bool,
    /// Reason for blocking (if blocked)
    pub reason: Option<String>,
    /// Risk level (0-10)
    pub risk_level: u8,
    /// Matched patterns that triggered the check
    pub matched_patterns: Vec<String>,
}

/// The Safety Layer
#[derive(Debug, Clone)]
pub struct SafetyLayer {
    policy: SafetyPolicy,
    form_submission_count: u32,
    blocked_actions: HashSet<String>,
}

impl SafetyLayer {
    pub fn new(policy: SafetyPolicy) -> Self {
        Self {
            policy,
            form_submission_count: 0,
            blocked_actions: HashSet::new(),
        }
    }

    /// Check if an action is safe to execute
    pub fn check_action(&self, action: &SuggestedAction) -> SafetyCheckResult {
        if !self.policy.enabled {
            return SafetyCheckResult {
                allowed: true,
                reason: None,
                risk_level: 0,
                matched_patterns: vec![],
            };
        }

        let mut matched_patterns = Vec::new();
        let mut risk_level = 0u8;

        // Normalize for comparison
        let description_lower = action.description.to_lowercase();
        let selector_lower = action.selector.to_lowercase();
        let action_type_lower = action.action_type.to_lowercase();

        // Check allowed overrides first
        for override_pattern in &self.policy.allowed_overrides {
            if description_lower.contains(&override_pattern.to_lowercase()) {
                return SafetyCheckResult {
                    allowed: true,
                    reason: None,
                    risk_level: 0,
                    matched_patterns: vec![],
                };
            }
        }

        // Check blocked keywords
        for keyword in &self.policy.blocked_keywords {
            let keyword_lower = keyword.to_lowercase();
            if description_lower.contains(&keyword_lower) {
                matched_patterns.push(format!("keyword:{}", keyword));
                risk_level = risk_level.saturating_add(3);
            }
        }

        // Check blocked classes
        for class in &self.policy.blocked_classes {
            let class_lower = class.to_lowercase();
            if selector_lower.contains(&format!(".{}", class_lower))
                || selector_lower.contains(&format!("class=\"{}", class_lower))
            {
                matched_patterns.push(format!("class:{}", class));
                risk_level = risk_level.saturating_add(4);
            }
        }

        // Check blocked IDs
        for id in &self.policy.blocked_ids {
            let id_lower = id.to_lowercase();
            if selector_lower.contains(&format!("#{}", id_lower))
                || selector_lower.contains(&format!("id=\"{}", id_lower))
            {
                matched_patterns.push(format!("id:{}", id));
                risk_level = risk_level.saturating_add(4);
            }
        }

        // Check for dangerous action types
        if action_type_lower == "submit"
            && self.form_submission_count >= self.policy.max_form_submissions
        {
            matched_patterns.push("max_form_submissions_exceeded".to_string());
            risk_level = risk_level.saturating_add(5);
        }

        // Determine if action is allowed
        let allowed = matched_patterns.is_empty() || risk_level < 5;

        SafetyCheckResult {
            allowed,
            reason: if allowed {
                None
            } else {
                Some(format!(
                    "Action blocked due to safety policy: {}",
                    matched_patterns.join(", ")
                ))
            },
            risk_level: risk_level.min(10),
            matched_patterns,
        }
    }

    /// Check if a URL is safe to navigate to
    pub fn check_url(&self, url: &str) -> SafetyCheckResult {
        if !self.policy.enabled {
            return SafetyCheckResult {
                allowed: true,
                reason: None,
                risk_level: 0,
                matched_patterns: vec![],
            };
        }

        let url_lower = url.to_lowercase();
        let mut matched_patterns = Vec::new();

        for pattern in &self.policy.blocked_url_patterns {
            if url_lower.contains(&pattern.to_lowercase()) {
                matched_patterns.push(format!("url_pattern:{}", pattern));
            }
        }

        let allowed = matched_patterns.is_empty();

        SafetyCheckResult {
            allowed,
            reason: if allowed {
                None
            } else {
                Some(format!(
                    "URL blocked due to safety policy: {}",
                    matched_patterns.join(", ")
                ))
            },
            risk_level: if allowed { 0 } else { 8 },
            matched_patterns,
        }
    }

    /// Filter a list of actions, keeping only safe ones
    pub fn filter_actions(&self, actions: Vec<SuggestedAction>) -> Vec<SuggestedAction> {
        actions
            .into_iter()
            .filter(|action| {
                let check = self.check_action(action);
                if !check.allowed {
                    log::info!(
                        "SafetyLayer blocked action: {} (reason: {:?})",
                        action.description,
                        check.reason
                    );
                }
                check.allowed
            })
            .collect()
    }

    /// Record a form submission
    pub fn record_form_submission(&mut self) {
        self.form_submission_count += 1;
    }

    /// Get current statistics
    pub fn get_stats(&self) -> SafetyStats {
        SafetyStats {
            form_submissions: self.form_submission_count,
            blocked_action_count: self.blocked_actions.len() as u32,
            policy_enabled: self.policy.enabled,
        }
    }
}

impl Default for SafetyLayer {
    fn default() -> Self {
        Self::new(SafetyPolicy::default())
    }
}

/// Safety statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyStats {
    pub form_submissions: u32,
    pub blocked_action_count: u32,
    pub policy_enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blocks_delete_action() {
        let safety = SafetyLayer::default();
        let action = SuggestedAction {
            description: "Delete all users".to_string(),
            selector: "#delete-btn".to_string(),
            action_type: "click".to_string(),
            value: None,
            confidence: 0.9,
        };

        let result = safety.check_action(&action);
        assert!(!result.allowed);
        assert!(result.risk_level > 0);
    }

    #[test]
    fn test_allows_safe_action() {
        let safety = SafetyLayer::default();
        let action = SuggestedAction {
            description: "View user profile".to_string(),
            selector: "#profile-link".to_string(),
            action_type: "click".to_string(),
            value: None,
            confidence: 0.9,
        };

        let result = safety.check_action(&action);
        assert!(result.allowed);
        assert_eq!(result.risk_level, 0);
    }

    #[test]
    fn test_blocks_logout_url() {
        let safety = SafetyLayer::default();
        let result = safety.check_url("https://example.com/api/logout");
        assert!(!result.allowed);
    }
}
