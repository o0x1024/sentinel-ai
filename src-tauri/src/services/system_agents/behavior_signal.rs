use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use chrono::{DateTime, Utc};

pub const TRAFFIC_BEHAVIOR_SIGNAL_SETTINGS_KEY: &str = "traffic_behavior_signal_settings";
pub const BEHAVIOR_SOURCE_PROXY_INFERRED: &str = "proxy_inferred";
pub const BEHAVIOR_SOURCE_BROWSER_EXTENSION: &str = "browser_extension";
pub const TRAFFIC_BEHAVIOR_EXTENSION_BRIDGE_PORT: u16 = 18931;
pub const TRAFFIC_BEHAVIOR_EXTENSION_TTL_SECS: i64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficBehaviorSignalSettings {
    pub mode: String,
    pub browser_extension_connected: bool,
    pub browser_extension_last_seen_at: Option<String>,
}

impl Default for TrafficBehaviorSignalSettings {
    fn default() -> Self {
        Self {
            mode: BEHAVIOR_SOURCE_PROXY_INFERRED.to_string(),
            browser_extension_connected: false,
            browser_extension_last_seen_at: None,
        }
    }
}

impl TrafficBehaviorSignalSettings {
    pub fn normalize_mode(raw: &str) -> String {
        match raw {
            BEHAVIOR_SOURCE_BROWSER_EXTENSION => BEHAVIOR_SOURCE_BROWSER_EXTENSION.to_string(),
            _ => BEHAVIOR_SOURCE_PROXY_INFERRED.to_string(),
        }
    }

    pub fn sanitized(mut self) -> Self {
        self.mode = Self::normalize_mode(&self.mode);
        self = self.refresh_connection_state();
        self
    }

    pub fn refresh_connection_state(mut self) -> Self {
        let is_fresh = self
            .browser_extension_last_seen_at
            .as_deref()
            .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
            .map(|dt| {
                let last_seen = dt.with_timezone(&Utc);
                Utc::now().signed_duration_since(last_seen).num_seconds()
                    <= TRAFFIC_BEHAVIOR_EXTENSION_TTL_SECS
            })
            .unwrap_or(false);
        self.browser_extension_connected = self.browser_extension_connected && is_fresh;
        self
    }

    pub fn mark_browser_extension_seen(&mut self, seen_at: DateTime<Utc>) {
        self.browser_extension_connected = true;
        self.browser_extension_last_seen_at = Some(seen_at.to_rfc3339());
    }

    pub fn effective_mode(&self) -> String {
        if self.mode == BEHAVIOR_SOURCE_BROWSER_EXTENSION && self.browser_extension_connected {
            BEHAVIOR_SOURCE_BROWSER_EXTENSION.to_string()
        } else {
            BEHAVIOR_SOURCE_PROXY_INFERRED.to_string()
        }
    }

    pub fn to_payload(&self) -> Value {
        json!({
            "selectedMode": self.mode,
            "effectiveMode": self.effective_mode(),
            "browserExtensionConnected": self.browser_extension_connected,
            "browserExtensionLastSeenAt": self.browser_extension_last_seen_at,
            "browserExtensionBridgeUrl": format!("http://127.0.0.1:{TRAFFIC_BEHAVIOR_EXTENSION_BRIDGE_PORT}"),
        })
    }
}
