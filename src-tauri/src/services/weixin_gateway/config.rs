use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const DEFAULT_ILINK_BASE_URL: &str = "https://ilinkai.weixin.qq.com";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeixinGatewayConfig {
    pub enabled: bool,
    pub account_id: String,
    pub token: String,
    pub base_url: String,
    #[serde(default)]
    pub assistant_profile_id: Option<String>,
    pub dm_policy: String,
    pub allowed_users: Vec<String>,
    pub group_policy: String,
    pub group_allowed_users: Vec<String>,
    pub default_service_name: String,
    pub max_iterations: usize,
    pub timeout_secs: u64,
}

impl Default for WeixinGatewayConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            account_id: String::new(),
            token: String::new(),
            base_url: DEFAULT_ILINK_BASE_URL.to_string(),
            assistant_profile_id: None,
            dm_policy: "open".to_string(),
            allowed_users: Vec::new(),
            group_policy: "open".to_string(),
            group_allowed_users: Vec::new(),
            default_service_name: "default".to_string(),
            max_iterations: 50,
            timeout_secs: 300,
        }
    }
}

impl WeixinGatewayConfig {
    pub fn normalize(&mut self) {
        self.account_id = self.account_id.trim().to_string();
        self.token = self.token.trim().to_string();
        self.base_url = self.base_url.trim().trim_end_matches('/').to_string();
        if self.base_url.is_empty() {
            self.base_url = DEFAULT_ILINK_BASE_URL.to_string();
        }
        self.assistant_profile_id = self
            .assistant_profile_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        self.dm_policy = normalize_policy(&self.dm_policy, "open");
        self.group_policy = normalize_policy(&self.group_policy, "open");
        self.allowed_users = normalize_list(&self.allowed_users);
        self.group_allowed_users = normalize_list(&self.group_allowed_users);
        self.default_service_name = self.default_service_name.trim().to_string();
        if self.default_service_name.is_empty() {
            self.default_service_name = "default".to_string();
        }
        self.max_iterations = self.max_iterations.max(1);
        self.timeout_secs = self.timeout_secs.max(30);
    }

    pub fn validate_for_runtime(&self) -> Result<(), String> {
        if self.account_id.is_empty() {
            return Err("Weixin account_id is required".to_string());
        }
        if self.token.is_empty() {
            return Err("Weixin token is required".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct WeixinGatewayStatus {
    pub running: bool,
    pub account_id: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub last_message_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeixinQrLoginResponse {
    pub qrcode: String,
    pub qrcode_img_content: String,
    pub scan_data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeixinQrLoginStatus {
    pub status: String,
    pub account_id: Option<String>,
    pub token_configured: bool,
    pub base_url: Option<String>,
    pub user_id: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WeixinQrLoginCredentials {
    pub account_id: String,
    pub token: String,
    pub base_url: String,
    pub user_id: Option<String>,
}

fn normalize_list(values: &[String]) -> Vec<String> {
    let mut normalized = values
        .iter()
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    normalized
}

fn normalize_policy(raw: &str, default_value: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "allowlist" | "disabled" | "open" => raw.trim().to_ascii_lowercase(),
        _ => default_value.to_string(),
    }
}
