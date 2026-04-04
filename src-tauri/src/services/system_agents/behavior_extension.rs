use std::collections::VecDeque;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use axum::extract::State;
use axum::http::header::{
    ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN,
    CONTENT_TYPE,
};
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};
use tokio::net::TcpListener;
use url::Url;

use crate::commands::traffic_analysis_commands::TrafficAnalysisState;
use crate::services::database::DatabaseService;
use crate::services::system_agents::{
    TRAFFIC_BEHAVIOR_EXTENSION_BRIDGE_PORT, TRAFFIC_BEHAVIOR_SIGNAL_SETTINGS_KEY,
};
use sentinel_traffic::HttpRequestRecord;

const MAX_BROWSER_BEHAVIOR_EVENTS: usize = 256;
const EVENT_WINDOW_SECS: i64 = 120;
const MAX_MATCHED_EVENTS: usize = 8;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserBehaviorEvent {
    pub event_type: String,
    pub url: String,
    pub title: Option<String>,
    pub host: Option<String>,
    pub tab_id: Option<i64>,
    pub frame_id: Option<i64>,
    pub text: Option<String>,
    pub role: Option<String>,
    pub input_name: Option<String>,
    pub route: Option<String>,
    pub selector: Option<String>,
    pub metadata: Option<Value>,
    pub received_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserBehaviorEventInput {
    pub event_type: String,
    pub url: String,
    pub title: Option<String>,
    pub host: Option<String>,
    pub tab_id: Option<i64>,
    pub frame_id: Option<i64>,
    pub text: Option<String>,
    pub role: Option<String>,
    pub input_name: Option<String>,
    pub route: Option<String>,
    pub selector: Option<String>,
    pub metadata: Option<Value>,
    pub occurred_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserBehaviorIngestPayload {
    pub browser: Option<String>,
    pub extension_version: Option<String>,
    pub events: Vec<BrowserBehaviorEventInput>,
}

#[derive(Debug, Clone, Default)]
pub struct BehaviorExtensionEventStore {
    events: VecDeque<BrowserBehaviorEvent>,
}

impl BehaviorExtensionEventStore {
    pub fn record_event(&mut self, event: BrowserBehaviorEvent) {
        self.events.push_back(event);
        if self.events.len() > MAX_BROWSER_BEHAVIOR_EVENTS {
            let overflow = self.events.len() - MAX_BROWSER_BEHAVIOR_EVENTS;
            self.events.drain(0..overflow);
        }
    }

    pub fn build_context_for_request(&self, record: &HttpRequestRecord) -> Option<Value> {
        let request_host = normalize_host(Some(&record.host))?;
        let cutoff = record.timestamp - Duration::seconds(EVENT_WINDOW_SECS);
        let mut matched = self
            .events
            .iter()
            .rev()
            .filter(|event| {
                event.received_at >= cutoff
                    && normalize_host(event.host.as_deref()).as_deref() == Some(request_host.as_str())
            })
            .take(MAX_MATCHED_EVENTS)
            .cloned()
            .collect::<Vec<_>>();
        matched.reverse();

        if matched.is_empty() {
            return None;
        }

        let behavior_steps = matched
            .iter()
            .map(|event| describe_event(event))
            .filter(|step| !step.is_empty())
            .collect::<Vec<_>>();
        let intent_hints = matched
            .iter()
            .flat_map(event_intent_hints)
            .fold(Vec::<String>::new(), |mut items, value| {
                if !value.is_empty() && !items.iter().any(|item| item == &value) {
                    items.push(value);
                }
                items
            });

        let last_title = matched
            .iter()
            .rev()
            .find_map(|event| event.title.clone())
            .unwrap_or_default();
        let last_url = matched
            .iter()
            .rev()
            .map(|event| event.url.clone())
            .next()
            .unwrap_or_default();

        Some(json!({
            "kind": "browser_extension",
            "host": request_host,
            "eventCount": matched.len(),
            "behaviorSteps": behavior_steps,
            "intentHints": intent_hints,
            "lastPageTitle": last_title,
            "lastPageUrl": last_url,
            "events": matched,
        }))
    }
}

#[derive(Clone)]
struct BehaviorExtensionBridgeState {
    db: Arc<DatabaseService>,
    traffic_state: Arc<TrafficAnalysisState>,
    app_handle: AppHandle,
}

pub async fn start_behavior_extension_bridge(
    db: Arc<DatabaseService>,
    traffic_state: Arc<TrafficAnalysisState>,
    app_handle: AppHandle,
) -> Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", TRAFFIC_BEHAVIOR_EXTENSION_BRIDGE_PORT))
        .await
        .map_err(|error| anyhow!("Failed to bind behavior extension bridge: {error}"))?;
    let state = BehaviorExtensionBridgeState {
        db,
        traffic_state,
        app_handle,
    };
    let router = Router::new()
        .route("/health", get(bridge_health).options(bridge_options))
        .route("/v1/events", post(ingest_behavior_events).options(bridge_options))
        .with_state(state);

    tauri::async_runtime::spawn(async move {
        if let Err(error) = axum::serve(listener, router).await {
            tracing::warn!("Behavior extension bridge stopped: {}", error);
        }
    });

    Ok(())
}

async fn bridge_health() -> Response {
    cors_json(StatusCode::OK, json!({
        "ok": true,
        "bridgeUrl": format!("http://127.0.0.1:{TRAFFIC_BEHAVIOR_EXTENSION_BRIDGE_PORT}"),
    }))
}

async fn bridge_options() -> Response {
    cors_json(StatusCode::NO_CONTENT, Value::Null)
}

async fn ingest_behavior_events(
    State(state): State<BehaviorExtensionBridgeState>,
    Json(payload): Json<BrowserBehaviorIngestPayload>,
) -> Response {
    let received_at = Utc::now();
    let mut recorded = 0usize;
    for input in payload.events {
        let Some(url) = normalize_event_url(&input.url) else {
            continue;
        };
        let event = BrowserBehaviorEvent {
            event_type: input.event_type.trim().to_string(),
            host: input.host.or_else(|| parse_host(&url)),
            url,
            title: input.title.map(trim_small_text),
            tab_id: input.tab_id,
            frame_id: input.frame_id,
            text: input.text.map(trim_small_text),
            role: input.role.map(trim_small_text),
            input_name: input.input_name.map(trim_small_text),
            route: input.route.map(trim_small_text),
            selector: input.selector.map(trim_small_text),
            metadata: input.metadata,
            received_at: parse_occurred_at(input.occurred_at.as_deref()).unwrap_or(received_at),
        };
        state
            .traffic_state
            .record_behavior_extension_event(event)
            .await;
        recorded += 1;
    }

    if recorded > 0 {
        if let Err(error) = refresh_behavior_extension_status(&state, received_at).await {
            tracing::warn!("Failed to refresh browser extension status: {}", error);
        }
    }

    cors_json(
        StatusCode::OK,
        json!({
            "ok": true,
            "recorded": recorded,
            "connected": true,
            "receivedAt": received_at.to_rfc3339(),
        }),
    )
}

async fn refresh_behavior_extension_status(
    state: &BehaviorExtensionBridgeState,
    seen_at: DateTime<Utc>,
) -> Result<()> {
    let shared_settings = state.traffic_state.get_behavior_signal_settings();
    let mut shared = shared_settings.write().await;
    shared.mark_browser_extension_seen(seen_at);
    let current = shared.clone().sanitized();
    drop(shared);

    state
        .db
        .save_proxy_config(
            TRAFFIC_BEHAVIOR_SIGNAL_SETTINGS_KEY,
            &serde_json::to_string(&current)?,
        )
        .await?;

    let _ = state.app_handle.emit(
        "traffic-behavior:extension-status",
        json!({
            "connected": current.browser_extension_connected,
            "lastSeenAt": current.browser_extension_last_seen_at,
        }),
    );
    Ok(())
}

fn cors_json(status: StatusCode, body: Value) -> Response {
    let mut response = (status, Json(body)).into_response();
    let headers = response.headers_mut();
    headers.insert(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
    headers.insert(
        ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET,POST,OPTIONS"),
    );
    headers.insert(
        ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("content-type"),
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    response
}

fn normalize_event_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    Url::parse(trimmed).ok().map(|value| value.to_string())
}

fn parse_host(raw: &str) -> Option<String> {
    Url::parse(raw)
        .ok()
        .and_then(|value| value.host_str().map(|item| item.to_ascii_lowercase()))
}

fn normalize_host(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            let normalized = value.to_ascii_lowercase();
            if normalized.starts_with('[') {
                normalized
            } else {
                normalized.split(':').next().unwrap_or(&normalized).to_string()
            }
        })
}

fn parse_occurred_at(raw: Option<&str>) -> Option<DateTime<Utc>> {
    raw.and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&Utc))
}

fn trim_small_text(raw: String) -> String {
    let trimmed = raw.trim();
    if trimmed.chars().count() > 160 {
        trimmed.chars().take(160).collect()
    } else {
        trimmed.to_string()
    }
}

fn describe_event(event: &BrowserBehaviorEvent) -> String {
    match event.event_type.as_str() {
        "page_load" => format!("打开页面 {}", event.title.clone().unwrap_or_else(|| event.url.clone())),
        "route_change" => format!("切换路由 {}", event.route.clone().unwrap_or_else(|| event.url.clone())),
        "click" => {
            let label = event.text.clone().or(event.role.clone()).unwrap_or_else(|| "页面元素".to_string());
            format!("点击 {label}")
        }
        "submit" => {
            let label = event.text.clone().or(event.role.clone()).unwrap_or_else(|| "表单".to_string());
            format!("提交 {label}")
        }
        "input_change" => {
            let label = event.input_name.clone().unwrap_or_else(|| "输入项".to_string());
            format!("填写 {label}")
        }
        "heartbeat" => "浏览器扩展保持连接".to_string(),
        _ => event.event_type.clone(),
    }
}

fn event_intent_hints(event: &BrowserBehaviorEvent) -> Vec<String> {
    let mut hints = Vec::new();
    if let Some(route) = event.route.as_deref() {
        hints.push(route.to_ascii_lowercase());
    }
    if let Some(text) = event.text.as_deref() {
        let normalized = text.trim().to_ascii_lowercase();
        if !normalized.is_empty() {
            hints.push(normalized);
        }
    }
    if let Some(name) = event.input_name.as_deref() {
        hints.push(name.trim().to_ascii_lowercase());
    }
    hints
}
