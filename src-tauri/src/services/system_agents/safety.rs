use anyhow::{anyhow, Result};
use serde_json::Value;
use url::Url;

use sentinel_db::SystemAgentProfileRecord;

#[derive(Debug, Clone, Default)]
pub struct SystemAgentSafetyPolicy {
    pub allow_active_replay: bool,
    pub auto_mode: bool,
    pub shadow_mode: bool,
    pub scope_hosts: Vec<String>,
}

impl SystemAgentSafetyPolicy {
    pub fn from_profile(profile: &SystemAgentProfileRecord) -> Self {
        Self::from_json_str(&profile.safety_policy_json)
    }

    pub fn from_json_str(raw: &str) -> Self {
        serde_json::from_str::<Value>(raw)
            .ok()
            .map(|value| Self::from_value(&value))
            .unwrap_or_default()
    }

    pub fn from_value(value: &Value) -> Self {
        let scope_hosts = value
            .get("scopeHosts")
            .or_else(|| value.get("allowedHosts"))
            .or_else(|| value.get("hostAllowlist"))
            .and_then(Value::as_array)
            .map(|items| {
                items.iter()
                    .filter_map(Value::as_str)
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Self {
            allow_active_replay: value
                .get("allowActiveReplay")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            auto_mode: value
                .get("autoMode")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            shadow_mode: value
                .get("shadowMode")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            scope_hosts,
        }
    }

    pub fn allows_auto_mode(&self) -> bool {
        self.auto_mode
    }

    pub fn is_shadow_mode_enabled(&self) -> bool {
        self.shadow_mode
    }

    pub fn ensure_active_replay_allowed(&self) -> Result<()> {
        if self.allow_active_replay {
            Ok(())
        } else {
            Err(anyhow!("Active replay is disabled by system-agent safety policy"))
        }
    }

    pub fn ensure_url_in_scope(&self, url: &str) -> Result<()> {
        if self.scope_hosts.is_empty() {
            return Ok(());
        }

        let parsed =
            Url::parse(url).map_err(|error| anyhow!("Invalid target URL for scope guard: {error}"))?;
        let host = parsed
            .host_str()
            .ok_or_else(|| anyhow!("Missing host in target URL for scope guard"))?;

        if self.scope_hosts.iter().any(|pattern| host_matches_pattern(host, pattern)) {
            Ok(())
        } else {
            Err(anyhow!(
                "Target host '{}' is outside the configured scope guard",
                host
            ))
        }
    }

    pub fn ensure_host_in_scope(&self, host: &str) -> Result<()> {
        if self.scope_hosts.is_empty() {
            return Ok(());
        }

        let normalized_host = host.trim();
        if normalized_host.is_empty() {
            return Err(anyhow!("Missing host for scope guard"));
        }

        if self
            .scope_hosts
            .iter()
            .any(|pattern| host_matches_pattern(normalized_host, pattern))
        {
            Ok(())
        } else {
            Err(anyhow!(
                "Target host '{}' is outside the configured scope guard",
                normalized_host
            ))
        }
    }

    pub fn ensure_url_or_host_in_scope(&self, url: Option<&str>, host: Option<&str>) -> Result<()> {
        if self.scope_hosts.is_empty() {
            return Ok(());
        }

        if let Some(url) = url.filter(|value| !value.trim().is_empty()) {
            return self.ensure_url_in_scope(url);
        }

        if let Some(host) = host.filter(|value| !value.trim().is_empty()) {
            return self.ensure_host_in_scope(host);
        }

        Err(anyhow!(
            "Missing target URL/host for scope guard with non-empty scope configuration"
        ))
    }
}

fn host_matches_pattern(host: &str, pattern: &str) -> bool {
    let normalized_host = host.trim().trim_end_matches('.').to_ascii_lowercase();
    let normalized_pattern = pattern.trim().trim_end_matches('.').to_ascii_lowercase();

    if normalized_pattern.is_empty() {
        return false;
    }
    if normalized_pattern == "*" {
        return true;
    }
    if normalized_host == normalized_pattern {
        return true;
    }
    if let Some(suffix) = normalized_pattern.strip_prefix("*.") {
        return normalized_host == suffix || normalized_host.ends_with(&format!(".{suffix}"));
    }

    false
}
