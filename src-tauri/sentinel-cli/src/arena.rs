use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::fs;
use tokio::sync::Mutex;
use tokio::time::sleep;

const DEFAULT_RATE_LIMIT_INTERVAL_MS: u64 = 400;
const CONFIG_FILE_NAME: &str = "arena_config.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArenaConfig {
    pub base_url: String,
    pub agent_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeListResponse {
    pub current_level: i32,
    pub total_challenges: i32,
    pub solved_challenges: i32,
    pub challenges: Vec<ChallengeInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeInfo {
    pub title: String,
    pub code: String,
    pub difficulty: String,
    pub description: String,
    pub level: i32,
    pub total_score: i32,
    pub total_got_score: i32,
    pub flag_count: i32,
    pub flag_got_count: i32,
    pub hint_viewed: bool,
    pub instance_status: String,
    pub entrypoint: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartChallengeResponse {
    pub entrypoints: Vec<String>,
    pub already_completed: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitFlagResponse {
    pub correct: bool,
    pub message: String,
    pub flag_count: i32,
    pub flag_got_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HintResponse {
    pub code: String,
    pub hint_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEnvelope<T> {
    code: i32,
    message: String,
    data: T,
}

#[derive(Clone)]
pub struct ArenaClient {
    config: ArenaConfig,
    client: Client,
    last_call_at: Arc<Mutex<Option<Instant>>>,
}

impl ArenaClient {
    pub fn new(config: ArenaConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .danger_accept_invalid_certs(true)
            .no_proxy()
            .build()
            .context("failed to build arena HTTP client")?;

        Ok(Self {
            config: ArenaConfig {
                base_url: normalize_base_url(&config.base_url),
                agent_token: config.agent_token,
            },
            client,
            last_call_at: Arc::new(Mutex::new(None)),
        })
    }

    pub async fn list_challenges(&self) -> Result<ChallengeListResponse> {
        let envelope = self.request("GET", "challenges", None).await?;
        Ok(envelope.data)
    }

    pub async fn start_challenge(&self, code: &str) -> Result<StartChallengeResponse> {
        let payload = serde_json::json!({ "code": code });
        let envelope: ApiEnvelope<Value> = self
            .request("POST", "start_challenge", Some(payload))
            .await?;

        if let Some(already_completed) = envelope
            .data
            .get("already_completed")
            .and_then(|value| value.as_bool())
        {
            return Ok(StartChallengeResponse {
                entrypoints: Vec::new(),
                already_completed,
                message: envelope.message,
            });
        }

        let entrypoints = envelope
            .data
            .as_array()
            .map(|values| {
                values
                    .iter()
                    .filter_map(|value| value.as_str().map(ToOwned::to_owned))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(StartChallengeResponse {
            entrypoints,
            already_completed: false,
            message: envelope.message,
        })
    }

    pub async fn stop_challenge(&self, code: &str) -> Result<ApiEnvelope<Value>> {
        self.request(
            "POST",
            "stop_challenge",
            Some(serde_json::json!({ "code": code })),
        )
        .await
    }

    pub async fn submit_flag(&self, code: &str, flag: &str) -> Result<SubmitFlagResponse> {
        let envelope = self
            .request(
                "POST",
                "submit",
                Some(serde_json::json!({ "code": code, "flag": flag })),
            )
            .await?;
        Ok(envelope.data)
    }

    pub async fn view_hint(&self, code: &str) -> Result<HintResponse> {
        let envelope = self
            .request("POST", "hint", Some(serde_json::json!({ "code": code })))
            .await?;
        Ok(envelope.data)
    }

    async fn request<T>(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
    ) -> Result<ApiEnvelope<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.wait_for_rate_limit().await;

        let url = format!("{}/{}", self.config.base_url, path.trim_start_matches('/'));
        let method = reqwest::Method::from_bytes(method.as_bytes())
            .map_err(|error| anyhow!("invalid HTTP method: {error}"))?;

        let mut request = self
            .client
            .request(method, &url)
            .header("Agent-Token", &self.config.agent_token);

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request
            .send()
            .await
            .with_context(|| format!("arena request failed: {}", url))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .with_context(|| format!("failed to read arena response body: {}", url))?;

        if !status.is_success() {
            return Err(anyhow!("arena request failed ({}): {}", status, text));
        }

        let parsed = serde_json::from_str::<ApiEnvelope<T>>(&text)
            .with_context(|| format!("failed to parse arena response JSON: {}", text))?;
        if parsed.code != 0 {
            return Err(anyhow!("arena business error: {}", parsed.message));
        }
        Ok(parsed)
    }

    async fn wait_for_rate_limit(&self) {
        let mut guard = self.last_call_at.lock().await;
        if let Some(last_call_at) = *guard {
            let min_interval = Duration::from_millis(DEFAULT_RATE_LIMIT_INTERVAL_MS);
            let elapsed = last_call_at.elapsed();
            if elapsed < min_interval {
                sleep(min_interval - elapsed).await;
            }
        }
        *guard = Some(Instant::now());
    }
}

pub async fn save_arena_config(state_dir: PathBuf, config: &ArenaConfig) -> Result<PathBuf> {
    fs::create_dir_all(&state_dir)
        .await
        .with_context(|| format!("failed to create state dir: {}", state_dir.display()))?;
    let path = state_dir.join(CONFIG_FILE_NAME);
    let content =
        serde_json::to_string_pretty(config).context("failed to serialize arena config")?;
    fs::write(&path, content)
        .await
        .with_context(|| format!("failed to write arena config: {}", path.display()))?;
    Ok(path)
}

pub async fn load_arena_config(state_dir: PathBuf) -> Result<ArenaConfig> {
    if let (Ok(base_url), Ok(agent_token)) = (
        std::env::var("SENTINEL_ARENA_BASE_URL"),
        std::env::var("SENTINEL_ARENA_AGENT_TOKEN"),
    ) {
        if !base_url.trim().is_empty() && !agent_token.trim().is_empty() {
            return Ok(ArenaConfig {
                base_url,
                agent_token,
            });
        }
    }

    let path = state_dir.join(CONFIG_FILE_NAME);
    let content = fs::read_to_string(&path)
        .await
        .with_context(|| format!("failed to read arena config: {}", path.display()))?;
    let config = serde_json::from_str::<ArenaConfig>(&content)
        .with_context(|| format!("invalid arena config JSON: {}", path.display()))?;
    Ok(ArenaConfig {
        base_url: normalize_base_url(&config.base_url),
        agent_token: config.agent_token,
    })
}

pub fn normalize_base_url(base_url: &str) -> String {
    let mut trimmed = base_url.trim().trim_end_matches('/').to_string();
    if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        trimmed = format!("http://{}", trimmed);
    }
    if trimmed.ends_with("/api") {
        return trimmed;
    }
    if trimmed.ends_with("api") {
        return trimmed;
    }
    format!("{trimmed}/api")
}
