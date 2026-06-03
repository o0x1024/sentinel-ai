use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

pub const TRAFFIC_CONTEXT_EXTRACTION_SETTINGS_KEY: &str = "traffic_context_extraction_settings";

const DEFAULT_PRINCIPAL_KEYS: &[&str] = &[
    "userId",
    "user_id",
    "uid",
    "username",
    "email",
    "sub",
    "subject",
    "tenantId",
    "tenant_id",
    "orgId",
    "org_id",
    "role",
    "roleId",
    "role_id",
];
const DEFAULT_RESOURCE_KEY_HINTS: &[&str] = &[
    "id",
    "uid",
    "userId",
    "user_id",
    "accountId",
    "account_id",
    "memberId",
    "member_id",
    "orderId",
    "order_id",
    "projectId",
    "project_id",
    "tenantId",
    "tenant_id",
    "orgId",
    "org_id",
    "roleId",
    "role_id",
];
const DEFAULT_AUTH_HEADER_KEYS: &[&str] = &[
    "authorization",
    "x-api-key",
    "x-auth-token",
    "x-access-token",
    "x-session-id",
];
const DEFAULT_AUTH_TOKEN_KEYS: &[&str] = &["access_token", "token", "session", "session_id"];
const DEFAULT_COOKIE_HINT_KEYS: &[&str] = &[
    "session", "sess", "sid", "token", "jwt", "auth", "bearer", "phpessid",
];
const DEFAULT_ACTION_ALIASES: &[(&str, &[&str])] = &[
    ("authenticate", &["login", "signin", "auth"]),
    ("synchronize", &["sync", "chrome-sync"]),
    ("approve", &["approve", "approval"]),
    ("review", &["review"]),
    ("confirm", &["confirm"]),
    ("submit", &["submit"]),
    ("redeem", &["redeem"]),
    ("claim", &["claim"]),
    ("grant", &["issue", "grant"]),
    ("create", &["create", "draft"]),
    ("pay", &["pay", "payment", "checkout"]),
    ("refund", &["refund"]),
    ("export", &["export", "download"]),
    ("search", &["search"]),
    ("read", &["list", "query"]),
    ("cancel", &["cancel", "revoke"]),
    ("complete", &["complete", "finish"]),
    ("execute", &["execute", "trigger", "run"]),
    ("callback", &["callback", "webhook"]),
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TrafficContextExtractionSettings {
    #[serde(default)]
    pub principal_keys: Vec<String>,
    #[serde(default)]
    pub resource_key_hints: Vec<String>,
    #[serde(default)]
    pub auth_header_keys: Vec<String>,
    #[serde(default)]
    pub auth_token_keys: Vec<String>,
    #[serde(default)]
    pub cookie_hint_keys: Vec<String>,
    #[serde(default)]
    pub action_aliases: BTreeMap<String, Vec<String>>,
}

impl Default for TrafficContextExtractionSettings {
    fn default() -> Self {
        Self {
            principal_keys: Vec::new(),
            resource_key_hints: Vec::new(),
            auth_header_keys: Vec::new(),
            auth_token_keys: Vec::new(),
            cookie_hint_keys: Vec::new(),
            action_aliases: BTreeMap::new(),
        }
    }
}

impl TrafficContextExtractionSettings {
    pub fn sanitized(mut self) -> Self {
        self.principal_keys = sanitize_values(self.principal_keys, false);
        self.resource_key_hints = sanitize_values(self.resource_key_hints, false);
        self.auth_header_keys = sanitize_values(self.auth_header_keys, true);
        self.auth_token_keys = sanitize_values(self.auth_token_keys, false);
        self.cookie_hint_keys = sanitize_values(self.cookie_hint_keys, true);
        self.action_aliases = sanitize_action_aliases(self.action_aliases);
        self
    }

    pub fn principal_keys(&self) -> Vec<String> {
        merge_values(DEFAULT_PRINCIPAL_KEYS, Some(&self.principal_keys), false)
    }

    pub fn resource_key_hints(&self) -> Vec<String> {
        merge_values(
            DEFAULT_RESOURCE_KEY_HINTS,
            Some(&self.resource_key_hints),
            false,
        )
    }

    pub fn auth_header_keys(&self) -> Vec<String> {
        merge_values(DEFAULT_AUTH_HEADER_KEYS, Some(&self.auth_header_keys), true)
    }

    pub fn auth_token_keys(&self) -> Vec<String> {
        merge_values(DEFAULT_AUTH_TOKEN_KEYS, Some(&self.auth_token_keys), false)
    }

    pub fn cookie_hint_keys(&self) -> Vec<String> {
        merge_values(DEFAULT_COOKIE_HINT_KEYS, Some(&self.cookie_hint_keys), true)
    }

    pub fn ordered_action_aliases(&self) -> Vec<(String, Vec<String>)> {
        let mut aliases = Vec::new();
        let mut seen = HashSet::new();

        for (action, defaults) in DEFAULT_ACTION_ALIASES {
            let action_name = action.to_string();
            seen.insert(action_name.clone());
            aliases.push((
                action_name.clone(),
                merge_values(defaults, self.action_aliases.get(&action_name), true),
            ));
        }

        for (action, custom_aliases) in &self.action_aliases {
            if seen.insert(action.clone()) {
                aliases.push((
                    action.clone(),
                    sanitize_values(custom_aliases.clone(), true),
                ));
            }
        }

        aliases
    }
}

fn sanitize_action_aliases(
    aliases: BTreeMap<String, Vec<String>>,
) -> BTreeMap<String, Vec<String>> {
    let mut sanitized = BTreeMap::new();

    for (action, values) in aliases {
        let action = normalize_action_name(&action);
        if action.is_empty() {
            continue;
        }
        let values = sanitize_values(values, true);
        if values.is_empty() {
            continue;
        }
        sanitized.insert(action, values);
    }

    sanitized
}

fn sanitize_values(values: Vec<String>, lowercase: bool) -> Vec<String> {
    let mut sanitized = Vec::new();
    let mut seen = HashSet::new();

    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        let normalized = if lowercase {
            trimmed.to_ascii_lowercase()
        } else {
            trimmed.to_string()
        };
        let dedupe_key = normalize_lookup_key(&normalized);
        if dedupe_key.is_empty() || !seen.insert(dedupe_key) {
            continue;
        }
        sanitized.push(normalized);
    }

    sanitized
}

fn merge_values(defaults: &[&str], custom: Option<&Vec<String>>, lowercase: bool) -> Vec<String> {
    let mut merged = Vec::new();
    let mut seen = HashSet::new();

    for value in defaults {
        let normalized = if lowercase {
            value.to_ascii_lowercase()
        } else {
            (*value).to_string()
        };
        let dedupe_key = normalize_lookup_key(&normalized);
        if seen.insert(dedupe_key) {
            merged.push(normalized);
        }
    }

    if let Some(custom_values) = custom {
        for value in custom_values {
            let normalized = if lowercase {
                value.to_ascii_lowercase()
            } else {
                value.clone()
            };
            let dedupe_key = normalize_lookup_key(&normalized);
            if seen.insert(dedupe_key) {
                merged.push(normalized);
            }
        }
    }

    merged
}

fn normalize_action_name(raw: &str) -> String {
    raw.trim().to_ascii_lowercase()
}

fn normalize_lookup_key(raw: &str) -> String {
    raw.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::TrafficContextExtractionSettings;
    use std::collections::BTreeMap;

    #[test]
    fn ordered_action_aliases_include_defaults_and_custom_entries() {
        let mut action_aliases = BTreeMap::new();
        action_aliases.insert("complete".to_string(), vec!["finalize".to_string()]);
        action_aliases.insert("writeoff".to_string(), vec!["write-off".to_string()]);
        let settings = TrafficContextExtractionSettings {
            action_aliases,
            ..TrafficContextExtractionSettings::default()
        }
        .sanitized();

        let ordered = settings.ordered_action_aliases();

        let complete = ordered
            .iter()
            .find(|(action, _)| action == "complete")
            .expect("complete aliases should exist");
        assert!(complete.1.iter().any(|alias| alias == "finish"));
        assert!(complete.1.iter().any(|alias| alias == "finalize"));

        let writeoff = ordered
            .iter()
            .find(|(action, _)| action == "writeoff")
            .expect("custom action should exist");
        assert_eq!(writeoff.1, vec!["write-off".to_string()]);
    }
}
