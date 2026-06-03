use anyhow::{Context, Result};
use std::net::SocketAddr;
use std::path::PathBuf;

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:8787";
const DEFAULT_TOKEN_TTL_SECS: i64 = 24 * 60 * 60;
const MIN_TOKEN_TTL_SECS: i64 = 5 * 60;
const MAX_TOKEN_TTL_SECS: i64 = 7 * 24 * 60 * 60;
const DEFAULT_DATABASE_URL: &str = "sqlite://entitlement-server.db";
const DEFAULT_POLICY_FILE: &str = "entitlement-policy.json";

#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub bind_addr: SocketAddr,
    pub api_key: Option<String>,
    pub database_url: String,
    pub bootstrap_policy_file: Option<PathBuf>,
    pub bootstrap_admin_id: Option<String>,
    pub bootstrap_admin_api_key: Option<String>,
    pub bootstrap_admin_role: String,
    pub token_ttl_secs: i64,
    pub refresh_rate_limit_per_minute: u32,
    pub admin_read_rate_limit_per_minute: u32,
    pub admin_write_rate_limit_per_minute: u32,
}

impl ServerConfig {
    pub fn from_env() -> Result<Self> {
        let bind_addr = std::env::var("SENTINEL_ENTITLEMENT_SERVER_BIND")
            .unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_string())
            .parse()
            .context("invalid SENTINEL_ENTITLEMENT_SERVER_BIND")?;
        let api_key = std::env::var("SENTINEL_ENTITLEMENT_SERVER_API_KEY")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let database_url = std::env::var("SENTINEL_ENTITLEMENT_DATABASE_URL")
            .unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string());
        let bootstrap_policy_file = std::env::var("SENTINEL_ENTITLEMENT_POLICY_FILE")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                let default_path = PathBuf::from(DEFAULT_POLICY_FILE);
                default_path.exists().then_some(default_path)
            });
        let bootstrap_admin_id = std::env::var("SENTINEL_ENTITLEMENT_BOOTSTRAP_ADMIN_ID")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let bootstrap_admin_api_key = std::env::var("SENTINEL_ENTITLEMENT_BOOTSTRAP_ADMIN_API_KEY")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let bootstrap_admin_role = std::env::var("SENTINEL_ENTITLEMENT_BOOTSTRAP_ADMIN_ROLE")
            .ok()
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "admin".to_string());
        let token_ttl_secs = std::env::var("SENTINEL_ENTITLEMENT_TOKEN_TTL_SECS")
            .ok()
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or(DEFAULT_TOKEN_TTL_SECS);
        let refresh_rate_limit_per_minute = std::env::var("SENTINEL_ENTITLEMENT_REFRESH_RPM")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(180)
            .clamp(10, 20_000);
        let admin_read_rate_limit_per_minute = std::env::var("SENTINEL_ENTITLEMENT_ADMIN_READ_RPM")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(120)
            .clamp(10, 10_000);
        let admin_write_rate_limit_per_minute =
            std::env::var("SENTINEL_ENTITLEMENT_ADMIN_WRITE_RPM")
                .ok()
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or(60)
                .clamp(5, 5_000);

        Ok(Self {
            bind_addr,
            api_key,
            database_url,
            bootstrap_policy_file,
            bootstrap_admin_id,
            bootstrap_admin_api_key,
            bootstrap_admin_role,
            token_ttl_secs: clamp_ttl(token_ttl_secs),
            refresh_rate_limit_per_minute,
            admin_read_rate_limit_per_minute,
            admin_write_rate_limit_per_minute,
        })
    }
}

pub fn clamp_ttl(ttl_seconds: i64) -> i64 {
    ttl_seconds.clamp(MIN_TOKEN_TTL_SECS, MAX_TOKEN_TTL_SECS)
}
