use crate::services::ai::AiServiceManager;
use crate::commands::tool_commands::PendingPermissionRequest;
use crate::skills::{parse_skill_markdown, scan_and_upsert_skills, skills_root};
use crate::services::database::DatabaseService;
use crate::utils::ai_generation_settings::apply_generation_settings_from_db;
use crate::models::database::{AiConversation, AiMessage};
use crate::agents::AgentExecuteParams;
use crate::agents::{ToolConfig, ToolSelectionStrategy};
use crate::agents::tool_router::{clear_tool_usage_records, get_tool_usage_statistics};
use axum::body::Body;
use axum::extract::{
    ConnectInfo, DefaultBodyLimit, Path, Query, Request, State,
};
use axum::http::{HeaderMap, HeaderName, StatusCode};
use axum::http::header::{CONTENT_TYPE, HeaderValue};
use axum::middleware::{from_fn_with_state, Next};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::{Component, Path as StdPath, PathBuf};
use std::str::FromStr;
use std::sync::Mutex as StdMutex;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::fs;
use tokio::sync::{oneshot, Semaphore};
use tokio::task::JoinHandle;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_stream::StreamExt;
use tracing::{error, info, warn};
use uuid::Uuid;
use sentinel_db::Database;

const HASH_PREFIX: &str = "sha256:";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpGatewayCorsConfig {
    pub enabled: bool,
    pub origins: Vec<String>,
}

impl Default for HttpGatewayCorsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            origins: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpGatewayAuthConfig {
    pub required: bool,
    pub api_keys: Vec<String>,
    pub header_name: String,
}

impl Default for HttpGatewayAuthConfig {
    fn default() -> Self {
        Self {
            required: false,
            api_keys: Vec::new(),
            header_name: "X-API-Key".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpGatewayRemoteConfig {
    pub enabled: bool,
    pub mode: String,
    pub public_base_url: String,
}

impl Default for HttpGatewayRemoteConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: "reverse_proxy".to_string(),
            public_base_url: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpGatewayLimitsConfig {
    pub max_body_bytes: usize,
    pub requests_per_minute: u32,
    pub max_concurrent_requests: usize,
}

impl Default for HttpGatewayLimitsConfig {
    fn default() -> Self {
        Self {
            max_body_bytes: 1024 * 1024,
            requests_per_minute: 600,
            max_concurrent_requests: 32,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpGatewayAuditConfig {
    pub enabled: bool,
    pub log_auth_failures: bool,
}

impl Default for HttpGatewayAuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_auth_failures: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpGatewayConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub allow_lan: bool,
    pub cors: HttpGatewayCorsConfig,
    pub auth: HttpGatewayAuthConfig,
    pub remote: HttpGatewayRemoteConfig,
    pub limits: HttpGatewayLimitsConfig,
    pub audit: HttpGatewayAuditConfig,
}

impl Default for HttpGatewayConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: "127.0.0.1".to_string(),
            port: 18765,
            allow_lan: false,
            cors: HttpGatewayCorsConfig::default(),
            auth: HttpGatewayAuthConfig::default(),
            remote: HttpGatewayRemoteConfig::default(),
            limits: HttpGatewayLimitsConfig::default(),
            audit: HttpGatewayAuditConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HttpGatewayStatus {
    pub running: bool,
    pub bind_addr: Option<String>,
    pub started_at: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    timestamp: String,
}

#[derive(Debug, Serialize)]
struct ApiStatusResponse {
    status: &'static str,
    service: &'static str,
    request_id: String,
}

#[derive(Debug, Deserialize)]
struct ChatRequest {
    message: String,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    request_id: Option<String>,
    #[serde(default)]
    since_message_id: Option<String>,
    #[serde(default)]
    service_name: Option<String>,
    #[serde(default)]
    system_prompt: Option<String>,
    #[serde(default)]
    mode: Option<String>, // "agent" | "llm"
    #[serde(default)]
    tool_config: Option<ToolConfig>,
    #[serde(default)]
    audit_config: Option<GatewayAuditExecuteConfig>,
    #[serde(default)]
    max_iterations: Option<usize>,
    #[serde(default)]
    timeout_secs: Option<u64>,
    #[serde(default)]
    enable_tenth_man_rule: Option<bool>,
}

#[derive(Debug, Serialize)]
struct ChatResponse {
    id: String,
    session_id: String,
    message: String,
    request_id: String,
    mode: String,
}

#[derive(Debug, Serialize)]
struct CreateSessionResponse {
    session_id: String,
    created_at: String,
}

#[derive(Debug, Serialize)]
struct SessionMessageItem {
    id: String,
    role: String,
    content: String,
    metadata: Option<serde_json::Value>,
    timestamp: String,
}

#[derive(Debug, Serialize)]
struct SessionMessagesResponse {
    session_id: String,
    request_id: String,
    messages: Vec<SessionMessageItem>,
}

#[derive(Debug, Deserialize)]
struct SessionMessagesQuery {
    #[serde(default)]
    after_id: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct BridgeInvokeRequest {
    command: String,
    #[serde(default)]
    payload: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct BridgeCreateConversationRequest {
    title: String,
    service_name: String,
}

#[derive(Debug, Deserialize)]
struct BridgeSaveMessageRequest {
    #[serde(default)]
    id: Option<String>,
    conversation_id: String,
    role: String,
    content: String,
    #[serde(default)]
    metadata: Option<serde_json::Value>,
    #[serde(default)]
    architecture_type: Option<String>,
    #[serde(default)]
    architecture_meta: Option<serde_json::Value>,
    #[serde(default)]
    structured_data: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct BridgeAgentExecuteRequest {
    task: String,
    #[serde(default)]
    config: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct BridgeInvokeResponse {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct PendingPermissionsResponse {
    items: Vec<PendingPermissionRequest>,
    request_id: String,
}

#[derive(Debug, Deserialize)]
struct PendingPermissionsQuery {
    #[serde(default)]
    session_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PermissionRespondRequest {
    id: String,
    allowed: bool,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    request_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SessionChatRequest {
    message: String,
    #[serde(default)]
    request_id: Option<String>,
    #[serde(default)]
    since_message_id: Option<String>,
    #[serde(default)]
    service_name: Option<String>,
    #[serde(default)]
    system_prompt: Option<String>,
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    tool_config: Option<ToolConfig>,
    #[serde(default)]
    audit_config: Option<GatewayAuditExecuteConfig>,
    #[serde(default)]
    max_iterations: Option<usize>,
    #[serde(default)]
    timeout_secs: Option<u64>,
    #[serde(default)]
    enable_tenth_man_rule: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
struct GatewayAuditExecuteConfig {
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    required_tools: Option<Vec<String>>,
}

#[derive(Clone)]
struct GatewayAppState {
    auth_required: bool,
    header_name: HeaderName,
    api_key_hashes: Arc<HashSet<String>>,
    concurrent_limiter: Arc<Semaphore>,
    audit_enabled: bool,
    log_auth_failures: bool,
    ai_manager: Arc<AiServiceManager>,
    db: Arc<DatabaseService>,
    app_handle: tauri::AppHandle,
    active_executions: Arc<StdMutex<HashMap<String, String>>>, // session_id -> request_id
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "http_gateway",
        timestamp: Utc::now().to_rfc3339(),
    })
}

async fn api_status() -> Json<ApiStatusResponse> {
    Json(ApiStatusResponse {
        status: "ok",
        service: "http_gateway",
        request_id: Uuid::new_v4().to_string(),
    })
}

async fn web_ui() -> Html<&'static str> {
    Html(
        r#"<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Sentinel AI Gateway</title>
  <style>
    :root { --bg:#0b1020; --panel:#131a2d; --line:#26304a; --text:#e9edf8; --muted:#9aa8c7; --accent:#4da3ff; --ok:#29c089; }
    body { margin:0; font-family: ui-sans-serif,system-ui,-apple-system,Segoe UI,Roboto; background:linear-gradient(160deg,#0b1020,#0f1b34); color:var(--text); }
    .wrap { max-width: 980px; margin: 0 auto; padding: 16px; }
    .card { background: rgba(19,26,45,.95); border:1px solid var(--line); border-radius: 12px; }
    .head { display:flex; gap:8px; flex-wrap:wrap; padding: 12px; align-items:end; }
    .field { display:flex; flex-direction:column; gap:4px; min-width:180px; flex:1; }
    .field label { color: var(--muted); font-size: 12px; }
    input, textarea { background:#0f1528; color:var(--text); border:1px solid var(--line); border-radius:8px; padding:10px; }
    textarea { width:100%; min-height:74px; resize:vertical; }
    button { background:var(--accent); border:none; color:white; padding:10px 14px; border-radius:8px; cursor:pointer; font-weight:600; }
    button.secondary { background:#2a3555; }
    button:disabled { opacity:.55; cursor:not-allowed; }
    .chat { margin-top: 12px; padding: 12px; min-height: 54vh; max-height: 58vh; overflow:auto; }
    .msg { border:1px solid var(--line); border-radius:10px; padding:10px; margin-bottom:10px; white-space: pre-wrap; word-break: break-word; }
    .msg.user { border-left:4px solid var(--accent); }
    .msg.assistant { border-left:4px solid var(--ok); }
    .meta { color:var(--muted); font-size:12px; margin-bottom:6px; }
    .tool { color:#ffd87a; font-size:12px; margin-top:8px; }
    .composer { margin-top: 12px; padding: 12px; }
    .perm-panel { margin: 12px; padding: 10px; border:1px solid var(--line); border-radius:10px; background:#0f1528; }
    .perm-head { display:flex; align-items:center; justify-content:space-between; margin-bottom:8px; font-size:12px; color:var(--muted); }
    .perm-count { background:#24365a; color:#dbe9ff; border-radius:999px; padding:2px 8px; font-weight:700; }
    .perm-list { display:flex; flex-direction:column; gap:8px; max-height:180px; overflow:auto; }
    .perm-item { border:1px solid var(--line); border-radius:8px; padding:8px; }
    .perm-cmd { font-size:12px; white-space: pre-wrap; word-break: break-word; }
    .perm-time { margin-top:4px; color:var(--muted); font-size:11px; }
    .perm-actions { display:flex; gap:8px; margin-top:8px; }
    .perm-empty { color:var(--muted); font-size:12px; padding:6px 2px; }
    .row { display:flex; gap:8px; margin-top:10px; }
    .status { color:var(--muted); font-size: 12px; padding:0 12px 12px; }
  </style>
</head>
<body>
  <div class="wrap">
    <div class="card">
      <div class="head">
        <div class="field">
          <label>API Key (X-API-Key)</label>
          <input id="apiKey" type="password" placeholder="sgw_xxx" />
        </div>
        <div class="field">
          <label>Session ID</label>
          <input id="sessionId" type="text" placeholder="自动创建或手动指定" />
        </div>
        <div class="field">
          <label>Service Name</label>
          <input id="serviceName" type="text" placeholder="default" />
        </div>
        <button id="newSession" class="secondary">新会话</button>
      </div>
      <div id="chat" class="chat"></div>
      <div class="perm-panel">
        <div class="perm-head">
          <span>待审批命令</span>
          <span id="permissionCount" class="perm-count">0</span>
        </div>
        <div id="permissionList" class="perm-list"></div>
      </div>
      <div class="composer">
        <textarea id="message" placeholder="输入你的问题，支持流式输出与工具调用事件显示"></textarea>
        <div class="row">
          <button id="sendStream">流式发送</button>
          <button id="sendOnce" class="secondary">非流式发送</button>
        </div>
      </div>
      <div id="status" class="status">Ready</div>
    </div>
  </div>
  <script>
    const $ = (id) => document.getElementById(id);
    const chatEl = $("chat");
    const statusEl = $("status");
    const apiKeyEl = $("apiKey");
    const sessionEl = $("sessionId");
    const serviceEl = $("serviceName");
    const messageEl = $("message");
    const sendStreamBtn = $("sendStream");
    const sendOnceBtn = $("sendOnce");
    const newSessionBtn = $("newSession");
    const permissionListEl = $("permissionList");
    const permissionCountEl = $("permissionCount");
    let lastMessageId = "";
    let permissionPollTimer = null;
    const permissionMap = new Map();

    const LS_KEY = "sentinel-gateway-api-key";
    const LS_SESSION = "sentinel-gateway-session";
    apiKeyEl.value = localStorage.getItem(LS_KEY) || "";
    sessionEl.value = localStorage.getItem(LS_SESSION) || "";
    apiKeyEl.addEventListener("change", () => localStorage.setItem(LS_KEY, apiKeyEl.value.trim()));
    sessionEl.addEventListener("change", () => localStorage.setItem(LS_SESSION, sessionEl.value.trim()));

    function setStatus(t) { statusEl.textContent = t; }
    function append(role, text, extra = "") {
      const div = document.createElement("div");
      div.className = "msg " + role;
      div.innerHTML = `<div class="meta">${role}</div><div class="body"></div>${extra ? `<div class="tool">${extra}</div>` : ""}`;
      div.querySelector(".body").textContent = text;
      chatEl.appendChild(div);
      chatEl.scrollTop = chatEl.scrollHeight;
      return div.querySelector(".body");
    }

    async function respondPermission(id, allowed, session_id) {
      const key = apiKeyEl.value.trim();
      const r = await fetch("/api/permissions/respond", {
        method: "POST",
        headers: { "Content-Type": "application/json", ...(key ? { "X-API-Key": key } : {}) },
        body: JSON.stringify({ id, allowed, session_id })
      });
      if (!r.ok) {
        let err = "HTTP " + r.status;
        try { const j = await r.json(); if (j?.error?.message) err = j.error.message; } catch {}
        throw new Error(err);
      }
    }

    function formatTimestamp(ts) {
      if (!ts) return "";
      const d = new Date(ts * 1000);
      return isNaN(d.getTime()) ? "" : d.toLocaleString();
    }

    function renderPermissionQueue() {
      permissionCountEl.textContent = String(permissionMap.size);
      permissionListEl.innerHTML = "";
      const items = Array.from(permissionMap.values()).sort((a, b) => (a.timestamp || 0) - (b.timestamp || 0));
      if (!items.length) {
        const empty = document.createElement("div");
        empty.className = "perm-empty";
        empty.textContent = "当前会话无待审批请求";
        permissionListEl.appendChild(empty);
        return;
      }
      for (const permission of items) {
        const item = document.createElement("div");
        item.className = "perm-item";
        const cmd = document.createElement("div");
        cmd.className = "perm-cmd";
        cmd.textContent = permission.command || "";
        const time = document.createElement("div");
        time.className = "perm-time";
        time.textContent = formatTimestamp(permission.timestamp);
        const actions = document.createElement("div");
        actions.className = "perm-actions";
        const ok = document.createElement("button");
        ok.textContent = "允许";
        const no = document.createElement("button");
        no.className = "secondary";
        no.textContent = "拒绝";
        const done = (msg) => {
          ok.disabled = true;
          no.disabled = true;
          setStatus(msg);
        };
        ok.onclick = async () => {
          try {
            await respondPermission(permission.id, true, sessionEl.value.trim() || undefined);
            permissionMap.delete(permission.id);
            renderPermissionQueue();
            done("permission approved");
          } catch (e) {
            done("approve failed: " + String(e.message || e));
          }
        };
        no.onclick = async () => {
          try {
            await respondPermission(permission.id, false, sessionEl.value.trim() || undefined);
            permissionMap.delete(permission.id);
            renderPermissionQueue();
            done("permission denied");
          } catch (e) {
            done("deny failed: " + String(e.message || e));
          }
        };
        actions.appendChild(ok);
        actions.appendChild(no);
        item.appendChild(cmd);
        item.appendChild(time);
        item.appendChild(actions);
        permissionListEl.appendChild(item);
      }
    }

    function upsertPermission(permission) {
      if (!permission || !permission.id) return;
      permissionMap.set(permission.id, permission);
      renderPermissionQueue();
    }

    async function refreshPermissionQueue() {
      const session_id = sessionEl.value.trim();
      if (!session_id) {
        permissionMap.clear();
        renderPermissionQueue();
        return;
      }
      const key = apiKeyEl.value.trim();
      const r = await fetch(`/api/permissions/pending?session_id=${encodeURIComponent(session_id)}`, {
        headers: key ? { "X-API-Key": key } : {}
      });
      if (!r.ok) return;
      const j = await r.json();
      const items = Array.isArray(j.items) ? j.items : [];
      const ids = new Set(items.map((v) => v.id));
      for (const id of Array.from(permissionMap.keys())) {
        if (!ids.has(id)) permissionMap.delete(id);
      }
      for (const item of items) permissionMap.set(item.id, item);
      renderPermissionQueue();
    }

    async function ensureSession() {
      if (sessionEl.value.trim()) return sessionEl.value.trim();
      const key = apiKeyEl.value.trim();
      const r = await fetch("/api/session", { method: "POST", headers: key ? { "X-API-Key": key } : {} });
      if (!r.ok) throw new Error("创建会话失败: HTTP " + r.status);
      const j = await r.json();
      sessionEl.value = j.session_id || "";
      localStorage.setItem(LS_SESSION, sessionEl.value);
      return sessionEl.value;
    }

    async function loadSessionMessages() {
      const session_id = sessionEl.value.trim();
      if (!session_id) return;
      const key = apiKeyEl.value.trim();
      const r = await fetch(`/api/session/${encodeURIComponent(session_id)}/messages?limit=200`, {
        headers: key ? { "X-API-Key": key } : {}
      });
      if (!r.ok) return;
      const j = await r.json();
      chatEl.innerHTML = "";
      for (const m of (j.messages || [])) {
        lastMessageId = m.id || lastMessageId;
        append(m.role === "user" ? "user" : "assistant", m.content || "");
      }
      await refreshPermissionQueue();
      setStatus("history loaded");
    }

    async function sendOnce() {
      const msg = messageEl.value.trim();
      if (!msg) return;
      append("user", msg);
      const out = append("assistant", "...");
      setStatus("sending (non-stream)...");
      sendOnceBtn.disabled = sendStreamBtn.disabled = true;
      try {
        const session_id = await ensureSession();
        const key = apiKeyEl.value.trim();
        const service_name = serviceEl.value.trim() || undefined;
        const r = await fetch("/api/chat", {
          method: "POST",
          headers: { "Content-Type": "application/json", ...(key ? { "X-API-Key": key } : {}) },
          body: JSON.stringify({ message: msg, session_id, service_name })
        });
        const j = await r.json();
        if (!r.ok) throw new Error(j?.error?.message || ("HTTP " + r.status));
        out.textContent = j.message || "";
        messageEl.value = "";
        setStatus("done");
      } catch (e) {
        out.textContent = "Error: " + String(e.message || e);
        setStatus("error");
      } finally {
        sendOnceBtn.disabled = sendStreamBtn.disabled = false;
      }
    }

    async function sendStream() {
      const msg = messageEl.value.trim();
      if (!msg) return;
      append("user", msg);
      const out = append("assistant", "");
      let toolMeta = "";
      setStatus("sending (stream)...");
      sendOnceBtn.disabled = sendStreamBtn.disabled = true;
      try {
        const session_id = await ensureSession();
        const key = apiKeyEl.value.trim();
        const service_name = serviceEl.value.trim() || undefined;
        const r = await fetch("/api/chat/stream", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            ...(key ? { "X-API-Key": key } : {}),
            ...(lastMessageId ? { "Last-Event-ID": lastMessageId } : {}),
          },
          body: JSON.stringify({ message: msg, session_id, service_name, since_message_id: lastMessageId || undefined })
        });
        if (!r.ok || !r.body) {
          let err = "HTTP " + r.status;
          try { const j = await r.json(); if (j?.error?.message) err = j.error.message; } catch {}
          throw new Error(err);
        }

        const reader = r.body.getReader();
        const decoder = new TextDecoder();
        let buf = "";
        while (true) {
          const { value, done } = await reader.read();
          if (done) break;
          buf += decoder.decode(value, { stream: true });
          const parts = buf.split("\\n\\n");
          buf = parts.pop() || "";
          for (const block of parts) {
            const line = block.split("\\n").find(l => l.startsWith("data: "));
            if (!line) continue;
            const raw = line.slice(6);
            let evt;
            try { evt = JSON.parse(raw); } catch { continue; }
            if (evt.message_id) lastMessageId = evt.message_id;
            if (evt.type === "assistant_delta" || evt.type === "delta" || evt.type === "reasoning") out.textContent += (evt.content || "");
            if (evt.type && evt.type.startsWith("tool_")) toolMeta = (toolMeta + "\\n[" + evt.type + "] " + JSON.stringify(evt)).trim();
            if (evt.type === "permission_required" && evt.permission) upsertPermission(evt.permission);
            if (evt.type === "error") throw new Error(evt.message || "stream error");
            if (evt.type === "task_status" && evt.status) setStatus(String(evt.status));
            if (evt.type === "completed" || evt.type === "done") setStatus("done");
          }
        }
        if (toolMeta) {
          const t = document.createElement("div");
          t.className = "tool";
          t.textContent = toolMeta;
          out.parentElement.appendChild(t);
        }
        messageEl.value = "";
      } catch (e) {
        out.textContent += "\\nError: " + String(e.message || e);
        setStatus("error");
      } finally {
        sendOnceBtn.disabled = sendStreamBtn.disabled = false;
      }
    }

    newSessionBtn.addEventListener("click", async () => {
      try {
        sessionEl.value = "";
        lastMessageId = "";
        permissionMap.clear();
        renderPermissionQueue();
        await ensureSession();
        chatEl.innerHTML = "";
        await refreshPermissionQueue();
        setStatus("new session created");
      } catch (e) {
        setStatus("new session failed: " + String(e.message || e));
      }
    });
    sessionEl.addEventListener("change", async () => {
      await loadSessionMessages();
      await refreshPermissionQueue();
    });
    sendOnceBtn.addEventListener("click", sendOnce);
    sendStreamBtn.addEventListener("click", sendStream);
    permissionPollTimer = setInterval(() => { refreshPermissionQueue().catch(() => {}); }, 2000);
    renderPermissionQueue();
    if (sessionEl.value.trim()) {
      loadSessionMessages();
      refreshPermissionQueue();
    }
  </script>
</body>
</html>"#,
    )
}

fn gateway_dist_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../dist")
}

fn sanitize_asset_path(path: &str) -> Option<PathBuf> {
    let normalized = path.trim_start_matches('/');
    let candidate = if normalized.is_empty() { "index.html" } else { normalized };
    let rel = PathBuf::from(candidate);
    for comp in rel.components() {
        if !matches!(comp, Component::Normal(_)) {
            return None;
        }
    }
    Some(rel)
}

fn content_type_for_path(path: &str) -> &'static str {
    if path.ends_with(".html") {
        "text/html; charset=utf-8"
    } else if path.ends_with(".js") {
        "application/javascript; charset=utf-8"
    } else if path.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if path.ends_with(".json") {
        "application/json; charset=utf-8"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        "image/jpeg"
    } else if path.ends_with(".woff2") {
        "font/woff2"
    } else if path.ends_with(".woff") {
        "font/woff"
    } else {
        "application/octet-stream"
    }
}

fn maybe_inject_gateway_bootstrap(html: String) -> String {
    let bootstrap = r##"<script>
window.__SENTINEL_GATEWAY__ = { enabled: true, transport: "http-bridge-v1" };
window.__TAURI_EVENT_PLUGIN_INTERNALS__ = window.__TAURI_EVENT_PLUGIN_INTERNALS__ || {
  unregisterListener: () => {},
};
const __gatewayEventState = window.__gatewayEventState || {
  nextId: 1,
  byEvent: new Map(),
  byId: new Map(),
};
window.__gatewayEventState = __gatewayEventState;
const __gwAuth = window.__gwAuth || {
  key: null,
  headerName: "X-API-Key",
  storageKey: "sentinel:http-gateway:api-key",
};
window.__gwAuth = __gwAuth;
function __gwResolveApiKey() {
  try {
    if (__gwAuth.key && String(__gwAuth.key).trim()) return String(__gwAuth.key).trim();
    const fromLs = localStorage.getItem(__gwAuth.storageKey);
    if (fromLs && fromLs.trim()) {
      __gwAuth.key = fromLs.trim();
      return __gwAuth.key;
    }
    const u = new URL(window.location.href);
    const fromQuery = u.searchParams.get("api_key") || u.searchParams.get("apikey");
    if (fromQuery && fromQuery.trim()) {
      __gwAuth.key = fromQuery.trim();
      localStorage.setItem(__gwAuth.storageKey, __gwAuth.key);
      return __gwAuth.key;
    }
  } catch {}
  return null;
}
function __gwHeaders(extra) {
  const h = Object.assign({}, extra || {});
  const key = __gwResolveApiKey();
  if (key) h[__gwAuth.headerName] = key;
  return h;
}
function __gwHideAuthPrompt() {
  const el = document.getElementById("__gw-auth-overlay");
  if (el) el.remove();
}
function __gwShowAuthPrompt(message) {
  let overlay = document.getElementById("__gw-auth-overlay");
  if (!overlay) {
    overlay = document.createElement("div");
    overlay.id = "__gw-auth-overlay";
    overlay.style.position = "fixed";
    overlay.style.inset = "0";
    overlay.style.zIndex = "2147483647";
    overlay.style.background = "rgba(3,7,18,0.55)";
    overlay.style.display = "flex";
    overlay.style.alignItems = "center";
    overlay.style.justifyContent = "center";
    overlay.innerHTML = `
      <div style="width:min(560px,92vw);background:#0b1220;color:#e5e7eb;border:1px solid #243047;border-radius:14px;box-shadow:0 20px 45px rgba(0,0,0,.45);padding:20px 20px 16px;">
        <div style="font-size:18px;font-weight:700;line-height:1.3;">HTTP 网关访问认证</div>
        <div id="__gw-auth-msg" style="font-size:13px;color:#a6b0c3;margin-top:8px;"></div>
        <div style="margin-top:14px;">
          <input id="__gw-auth-input" type="password" placeholder="请输入 API Key" style="width:100%;height:40px;border-radius:10px;border:1px solid #32405f;background:#0f1728;color:#e5e7eb;padding:0 12px;outline:none;" />
        </div>
        <div style="display:flex;gap:8px;justify-content:flex-end;margin-top:14px;">
          <button id="__gw-auth-clear" style="height:34px;border-radius:8px;border:1px solid #334155;background:#0f172a;color:#cbd5e1;padding:0 10px;cursor:pointer;">清除已保存</button>
          <button id="__gw-auth-save" style="height:34px;border-radius:8px;border:1px solid #2563eb;background:#2563eb;color:white;padding:0 12px;cursor:pointer;">保存并重试</button>
        </div>
      </div>
    `;
    document.body.appendChild(overlay);
    const input = overlay.querySelector("#__gw-auth-input");
    const clearBtn = overlay.querySelector("#__gw-auth-clear");
    const saveBtn = overlay.querySelector("#__gw-auth-save");
    if (input) {
      input.addEventListener("keydown", (e) => {
        if (e.key === "Enter" && saveBtn) saveBtn.click();
      });
    }
    if (clearBtn) {
      clearBtn.addEventListener("click", () => {
        try { localStorage.removeItem(__gwAuth.storageKey); } catch {}
        __gwAuth.key = null;
        if (input) input.value = "";
      });
    }
    if (saveBtn) {
      saveBtn.addEventListener("click", () => {
        const key = String(input?.value || "").trim();
        if (!key) return;
        __gwAuth.key = key;
        try { localStorage.setItem(__gwAuth.storageKey, key); } catch {}
        __gwHideAuthPrompt();
        window.location.reload();
      });
    }
  }
  const msg = overlay.querySelector("#__gw-auth-msg");
  if (msg) msg.textContent = message || "当前网关已启用鉴权，请输入 API Key 后继续。";
  const input = overlay.querySelector("#__gw-auth-input");
  if (input && !input.value) {
    const existing = __gwResolveApiKey();
    if (existing) input.value = existing;
  }
  setTimeout(() => {
    try { input?.focus(); } catch {}
  }, 0);
}
async function __gwMaybePromptForAuth() {
  try {
    if (__gwResolveApiKey()) return;
    const resp = await fetch("/api/status", { method: "GET" });
    if (resp.status === 401) {
      __gwShowAuthPrompt("网关需要 API Key。请输入后点击“保存并重试”。");
    }
  } catch {}
}
function __gwRandom() {
  return `${Date.now()}_${Math.random().toString(36).slice(2)}`;
}
function __gwDispatch(eventName, payload) {
  const m = __gatewayEventState.byEvent.get(eventName);
  if (!m) return;
  for (const [eventId, handlerId] of m.entries()) {
    try {
      const cb = window[handlerId];
      if (typeof cb === "function") {
        cb({ event: eventName, id: eventId, payload });
      }
    } catch {}
  }
}
window.__TAURI_INTERNALS__ = window.__TAURI_INTERNALS__ || {};
window.__TAURI_INTERNALS__.transformCallback = window.__TAURI_INTERNALS__.transformCallback || ((fn) => {
  const id = `cb_${__gwRandom()}`;
  window[id] = fn;
  return id;
});
window.__TAURI_INTERNALS__.invoke = async (cmd, payload) => {
  const p = payload ?? {};
  if (cmd === "plugin:event|listen") {
    const eventName = p.event;
    const handlerId = p.handler;
    const eventId = __gatewayEventState.nextId++;
    if (!__gatewayEventState.byEvent.has(eventName)) {
      __gatewayEventState.byEvent.set(eventName, new Map());
    }
    __gatewayEventState.byEvent.get(eventName).set(eventId, handlerId);
    __gatewayEventState.byId.set(eventId, { eventName, handlerId });
    return eventId;
  }
  if (cmd === "plugin:event|unlisten") {
    const eventName = p.event;
    const eventId = p.eventId;
    const rec = __gatewayEventState.byId.get(eventId);
    if (rec) {
      const map = __gatewayEventState.byEvent.get(rec.eventName);
      if (map) map.delete(eventId);
      __gatewayEventState.byId.delete(eventId);
    } else if (__gatewayEventState.byEvent.has(eventName)) {
      __gatewayEventState.byEvent.get(eventName).delete(eventId);
    }
    return null;
  }
  if (cmd === "plugin:event|emit" || cmd === "plugin:event|emit_to") {
    __gwDispatch(p.event, p.payload);
    return null;
  }

  if (cmd === "agent_execute") {
    const execId = p?.config?.conversation_id || `conv_${__gwRandom()}`;
    const msgId = p?.config?.message_id || `msg_${__gwRandom()}`;
    const display = p?.config?.display_content || p?.task || "";
    __gwDispatch("agent:user_message", {
      execution_id: execId,
      message_id: msgId,
      content: display,
      timestamp: Date.now(),
      document_attachments: p?.config?.document_attachments || undefined,
      image_attachments: p?.config?.attachments || undefined,
    });
    __gwDispatch("agent:start", {
      execution_id: execId,
      task: p?.task || "",
    });

    let finalText = "";
    const streamResp = await fetch("/api/chat/stream", {
      method: "POST",
      headers: __gwHeaders({ "Content-Type": "application/json" }),
      body: JSON.stringify({
        message: p?.task || "",
        session_id: execId,
        request_id: msgId,
        mode: "agent",
        tool_config: p?.config?.tool_config,
        audit_config: p?.config?.audit_config,
        max_iterations: p?.config?.max_iterations,
        timeout_secs: p?.config?.timeout_secs,
        enable_tenth_man_rule: p?.config?.enable_tenth_man_rule,
      }),
    });
    if (streamResp.status === 401) {
      __gwShowAuthPrompt("API Key 无效或未提供，无法发起 AI 会话。");
      throw new Error("Unauthorized: missing or invalid API key");
    }
    if (!streamResp.ok || !streamResp.body) {
      let err = `agent_execute stream failed: HTTP ${streamResp.status}`;
      try {
        const j = await streamResp.json();
        if (j?.error?.message) err = j.error.message;
      } catch {}
      __gwDispatch("agent:error", { execution_id: execId, error: err });
      throw new Error(err);
    }

    const reader = streamResp.body.getReader();
    const decoder = new TextDecoder();
    let buf = "";
    while (true) {
      const { value, done } = await reader.read();
      if (done) break;
      buf += decoder.decode(value, { stream: true });
      const parts = buf.split("\n\n");
      buf = parts.pop() || "";
      for (const block of parts) {
        const line = block.split("\n").find((l) => l.startsWith("data: "));
        if (!line) continue;
        const raw = line.slice(6);
        let evt = null;
        try { evt = JSON.parse(raw); } catch { continue; }
        if (!evt || typeof evt !== "object") continue;

        if (evt.type === "assistant_delta" || evt.type === "delta") {
          const content = evt.content || "";
          finalText += content;
          __gwDispatch("agent:chunk", {
            execution_id: execId,
            chunk_type: "text",
            content,
          });
        } else if (evt.type === "reasoning") {
          __gwDispatch("agent:chunk", {
            execution_id: execId,
            chunk_type: "reasoning",
            content: evt.content || "",
          });
        } else if (evt.type === "tool_call_start") {
          __gwDispatch("agent:tool_call_complete", {
            execution_id: execId,
            tool_call_id: evt.id || `tc_${__gwRandom()}`,
            tool_name: evt.name || "unknown",
            arguments: "{}",
          });
        } else if (evt.type === "tool_call_complete") {
          __gwDispatch("agent:tool_call_complete", {
            execution_id: execId,
            tool_call_id: evt.id || `tc_${__gwRandom()}`,
            tool_name: evt.name || "unknown",
            arguments: typeof evt.arguments === "string" ? evt.arguments : JSON.stringify(evt.arguments || {}),
          });
        } else if (evt.type === "tool_result") {
          __gwDispatch("agent:tool_result", {
            execution_id: execId,
            tool_call_id: evt.id || `tc_${__gwRandom()}`,
            result: typeof evt.result === "string" ? evt.result : JSON.stringify(evt.result || {}),
            success: true,
          });
        } else if (evt.type === "error") {
          const err = evt.message || "agent_execute stream error";
          __gwDispatch("agent:error", { execution_id: execId, error: err });
          throw new Error(err);
        } else if (evt.type === "done") {
          if (typeof evt.message === "string" && evt.message) finalText = evt.message;
          __gwDispatch("agent:complete", {
            execution_id: execId,
            success: evt.status !== "failed",
            response: finalText,
          });
        } else if (evt.type === "permission_required" && evt.permission) {
          __gwDispatch("shell-permission-request", evt.permission);
        }
      }
    }
    return msgId;
  }

  if (cmd === "generate_plugin_stream") {
    const streamId = p?.request?.stream_id || `plugin_${__gwRandom()}`;
    const msg = p?.request?.message || "";
    const systemPrompt = p?.request?.system_prompt;
    const serviceName = p?.request?.service_name || "default";
    __gwDispatch("plugin_gen_start", { stream_id: streamId });
    let finalText = "";
    try {
      const resp = await fetch("/api/chat/stream", {
        method: "POST",
        headers: __gwHeaders({ "Content-Type": "application/json" }),
        body: JSON.stringify({
          message: msg,
          session_id: `plugin_stream_${streamId}`,
          request_id: streamId,
          mode: "llm",
          system_prompt: systemPrompt,
          service_name: serviceName,
        }),
      });
      if (resp.status === 401) {
        __gwShowAuthPrompt("API Key 无效或未提供，无法执行流式生成。");
        throw new Error("Unauthorized: missing or invalid API key");
      }
      if (!resp.ok || !resp.body) {
        throw new Error(`generate_plugin_stream failed: HTTP ${resp.status}`);
      }
      const reader = resp.body.getReader();
      const decoder = new TextDecoder();
      let buf = "";
      while (true) {
        const { value, done } = await reader.read();
        if (done) break;
        buf += decoder.decode(value, { stream: true });
        const blocks = buf.split("\n\n");
        buf = blocks.pop() || "";
        for (const block of blocks) {
          const line = block.split("\n").find((l) => l.startsWith("data: "));
          if (!line) continue;
          let evt = null;
          try { evt = JSON.parse(line.slice(6)); } catch { continue; }
          if (!evt || typeof evt !== "object") continue;
          if (evt.type === "assistant_delta" && evt.content) {
            finalText += evt.content;
            __gwDispatch("plugin_gen_delta", { stream_id: streamId, delta: evt.content });
          } else if (evt.type === "done") {
            if (typeof evt.message === "string" && evt.message) finalText = evt.message;
          } else if (evt.type === "error") {
            throw new Error(evt.message || "plugin stream error");
          }
        }
      }
      __gwDispatch("plugin_gen_complete", { stream_id: streamId, result: finalText });
      return streamId;
    } catch (e) {
      const err = e?.message || String(e);
      __gwDispatch("plugin_gen_error", { stream_id: streamId, error: err });
      throw e;
    }
  }

  if (cmd === "plugin:shell|open") {
    const target = p?.path || p?.url || p?.value || "";
    if (target) window.open(target, "_blank", "noopener,noreferrer");
    return null;
  }
  if (cmd.startsWith("plugin:window|")) {
    return null;
  }
  if (cmd === "plugin:dialog|open" || cmd === "plugin:dialog|save") {
    return null;
  }
  if (cmd.startsWith("plugin:fs|")) {
    throw new Error("File-system plugin is unavailable in HTTP gateway mode");
  }

  const r = await fetch("/api/bridge/invoke", {
    method: "POST",
    headers: __gwHeaders({ "Content-Type": "application/json" }),
    body: JSON.stringify({ command: cmd, payload: p }),
  });
  if (r.status === 401) {
    __gwShowAuthPrompt("API Key 无效或未提供，前端调用已被网关拒绝。");
    throw new Error("Unauthorized: missing or invalid API key");
  }
  const j = await r.json().catch(() => ({}));
  if (!r.ok || !j?.ok) {
    try { console.error("[Gateway invoke failed]", { cmd, status: r.status, body: j }); } catch {}
    throw new Error(j?.error || `invoke failed: ${cmd}`);
  }
  return j.data;
};
if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", () => { __gwMaybePromptForAuth(); }, { once: true });
} else {
  __gwMaybePromptForAuth();
}
</script>"##;
    if html.contains("</head>") {
        html.replacen("</head>", &format!("{}\n</head>", bootstrap), 1)
    } else {
        format!("{}\n{}", bootstrap, html)
    }
}

fn normalize_skill_id(raw: &str) -> Result<String, String> {
    let id = raw.trim();
    if id.is_empty() {
        return Err("Skill name is required".to_string());
    }
    if id.len() > 64 {
        return Err("Skill name must be 64 characters or less".to_string());
    }
    let mut chars = id.chars();
    let first = chars.next().ok_or_else(|| "Skill name is required".to_string())?;
    if !(first.is_ascii_lowercase() || first.is_ascii_digit()) {
        return Err("Skill name must start with a lowercase letter or digit".to_string());
    }
    if !id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        return Err("Skill name must use lowercase letters, numbers, and hyphens only".to_string());
    }
    if id.ends_with('-') {
        return Err("Skill name must not end with a hyphen".to_string());
    }
    Ok(id.to_string())
}

fn build_skill_markdown(name: &str, description: &str, content: &str) -> String {
    format!(
        "---\nname: {}\ndescription: {}\n---\n\n{}\n",
        name,
        description.trim(),
        content.trim()
    )
}

fn sanitize_skill_relative_path(path: &str) -> Result<PathBuf, String> {
    if path.trim().is_empty() {
        return Err("Path is required".to_string());
    }
    let rel = StdPath::new(path);
    if rel.is_absolute() {
        return Err("Absolute paths are not allowed".to_string());
    }
    for comp in rel.components() {
        if matches!(comp, Component::ParentDir | Component::CurDir) {
            return Err("Invalid relative path".to_string());
        }
    }
    Ok(rel.to_path_buf())
}

fn resolve_skill_file_for_read(skill_dir: &StdPath, rel_path: &str) -> Result<PathBuf, String> {
    let rel = sanitize_skill_relative_path(rel_path)?;
    let target = skill_dir.join(rel);
    let canonical_root = std::fs::canonicalize(skill_dir).map_err(|e| e.to_string())?;
    let canonical_target = std::fs::canonicalize(&target).map_err(|e| e.to_string())?;
    if !canonical_target.starts_with(&canonical_root) {
        return Err("Path escapes skill directory".to_string());
    }
    Ok(canonical_target)
}

fn resolve_skill_file_for_write(skill_dir: &StdPath, rel_path: &str) -> Result<PathBuf, String> {
    let rel = sanitize_skill_relative_path(rel_path)?;
    let target = skill_dir.join(rel);
    let parent = target.parent().ok_or_else(|| "Invalid file path".to_string())?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let canonical_root = std::fs::canonicalize(skill_dir).map_err(|e| e.to_string())?;
    let canonical_parent = std::fs::canonicalize(parent).map_err(|e| e.to_string())?;
    if !canonical_parent.starts_with(&canonical_root) {
        return Err("Path escapes skill directory".to_string());
    }
    Ok(target)
}

async fn serve_dist_asset(path: &str) -> Option<Response> {
    let root = gateway_dist_root();
    let rel = sanitize_asset_path(path)?;
    let full = root.join(&rel);
    let (serve_path, is_html_fallback) = if full.is_file() {
        (full, false)
    } else if rel.extension().is_none() {
        let index = root.join("index.html");
        if index.is_file() {
            (index, true)
        } else {
            return None;
        }
    } else {
        return None;
    };

    let mut body = fs::read(&serve_path).await.ok()?;
    let filename = serve_path
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or("application/octet-stream");
    let mut content_type = content_type_for_path(filename).to_string();

    if is_html_fallback || serve_path.ends_with("index.html") {
        if let Ok(html) = String::from_utf8(body.clone()) {
            body = maybe_inject_gateway_bootstrap(html).into_bytes();
            content_type = "text/html; charset=utf-8".to_string();
        }
    }

    let mut resp = Response::new(Body::from(body));
    let _ = resp.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_str(&content_type)
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    Some(resp)
}

async fn web_entry() -> Response {
    if let Some(resp) = serve_dist_asset("index.html").await {
        return resp;
    }
    web_ui().await.into_response()
}

async fn web_assets(Path(path): Path<String>) -> Response {
    if let Some(resp) = serve_dist_asset(&path).await {
        return resp;
    }
    web_ui().await.into_response()
}

fn hash_api_key(raw_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw_key.as_bytes());
    format!("{}{:x}", HASH_PREFIX, hasher.finalize())
}

pub fn normalize_gateway_config(config: &mut HttpGatewayConfig) {
    let normalized: Vec<String> = config
        .auth
        .api_keys
        .iter()
        .filter(|s| !s.trim().is_empty())
        .map(|k| {
            if k.starts_with(HASH_PREFIX) {
                k.clone()
            } else {
                hash_api_key(k)
            }
        })
        .collect();

    config.auth.api_keys = normalized;
}

pub fn hash_api_key_for_storage(raw_key: &str) -> String {
    hash_api_key(raw_key)
}

pub struct HttpGatewayRuntime {
    bind_addr: SocketAddr,
    started_at: chrono::DateTime<chrono::Utc>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    task: JoinHandle<()>,
}

impl HttpGatewayRuntime {
    pub fn bind_addr(&self) -> String {
        self.bind_addr.to_string()
    }

    pub fn started_at(&self) -> String {
        self.started_at.to_rfc3339()
    }

    pub fn is_finished(&self) -> bool {
        self.task.is_finished()
    }

    pub async fn stop(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        let _ = tokio::time::timeout(Duration::from_secs(3), self.task).await;
    }
}

pub fn validate_gateway_config(config: &HttpGatewayConfig) -> Result<(), String> {
    if config.port < 1024 {
        return Err("Port must be between 1024 and 65535".to_string());
    }

    if !config.allow_lan && !is_loopback_host(&config.host) {
        return Err("When LAN access is disabled, host must be loopback (127.0.0.1/::1/localhost)".to_string());
    }

    if config.allow_lan && (!config.auth.required || config.auth.api_keys.is_empty()) {
        return Err("LAN mode requires auth.required=true and at least one API key".to_string());
    }

    if config.remote.enabled
        && (!config.auth.required
            || config.auth.api_keys.is_empty()
            || config.remote.public_base_url.trim().is_empty())
    {
        return Err("Remote mode requires auth, at least one API key, and public_base_url".to_string());
    }

    Ok(())
}

fn is_loopback_host(host: &str) -> bool {
    matches!(host, "127.0.0.1" | "::1") || host.eq_ignore_ascii_case("localhost")
}

fn error_response(status: StatusCode, code: &str, message: &str) -> Response {
    error_response_with_request_id(status, code, message, &Uuid::new_v4().to_string())
}

fn error_response_with_request_id(
    status: StatusCode,
    code: &str,
    message: &str,
    request_id: &str,
) -> Response {
    (
        status,
        Json(json!({
            "error": {
                "code": code,
                "message": message,
            },
            "request_id": request_id,
        })),
    )
        .into_response()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GatewayMode {
    Agent,
    Llm,
}

fn parse_mode(raw: Option<&str>) -> Result<GatewayMode, String> {
    let value = raw.unwrap_or("agent").trim().to_ascii_lowercase();
    match value.as_str() {
        "agent" => Ok(GatewayMode::Agent),
        "llm" => Ok(GatewayMode::Llm),
        _ => Err("mode must be one of: agent, llm".to_string()),
    }
}

fn build_audit_allowed_tools(audit_config: &GatewayAuditExecuteConfig) -> Vec<String> {
    let mut allowed = vec![
        "skills".to_string(),
        "git_clone_repo".to_string(),
        "code_search".to_string(),
        "git_diff_scope".to_string(),
        "tenth_man_review".to_string(),
        "todos".to_string(),
        "audit_finding_upsert".to_string(),
    ];

    if let Some(required) = &audit_config.required_tools {
        allowed.extend(
            required
                .iter()
                .map(|v| v.trim())
                .filter(|v| !v.is_empty())
                .map(|v| v.to_string()),
        );
    }

    let mut seen = HashSet::new();
    allowed.retain(|tool| seen.insert(tool.clone()));
    allowed
}

fn enforce_audit_tool_whitelist(
    config: Option<ToolConfig>,
    audit_config: &GatewayAuditExecuteConfig,
) -> ToolConfig {
    let mut tool_config = config.unwrap_or_default();
    let allowed_tools = build_audit_allowed_tools(audit_config);
    let allowed_set = allowed_tools.iter().cloned().collect::<HashSet<_>>();

    tool_config.enabled = true;
    tool_config.allowed_tools = allowed_tools.clone();
    tool_config.selection_strategy = ToolSelectionStrategy::Manual(allowed_tools.clone());
    tool_config.fixed_tools.retain(|tool| allowed_set.contains(tool));
    for tool in &allowed_tools {
        if !tool_config.fixed_tools.contains(tool) {
            tool_config.fixed_tools.push(tool.clone());
        }
    }
    tool_config
        .disabled_tools
        .retain(|tool| allowed_set.contains(tool));
    tool_config
}

fn resolve_stream_cursor(payload_since_message_id: Option<String>, headers: &HeaderMap) -> Option<String> {
    if payload_since_message_id
        .as_ref()
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
    {
        return payload_since_message_id;
    }
    headers
        .get("Last-Event-ID")
        .or_else(|| headers.get("last-event-id"))
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string())
}

fn try_register_active_execution(
    state: &GatewayAppState,
    session_id: &str,
    request_id: &str,
) -> Result<(), String> {
    let mut map = state
        .active_executions
        .lock()
        .map_err(|_| "execution registry lock poisoned".to_string())?;
    if let Some(active) = map.get(session_id) {
        if active != request_id {
            return Err(format!(
                "session '{}' already has active request '{}'",
                session_id, active
            ));
        }
        return Ok(());
    }
    map.insert(session_id.to_string(), request_id.to_string());
    Ok(())
}

fn release_active_execution(state: &GatewayAppState, session_id: &str, request_id: &str) {
    if let Ok(mut map) = state.active_executions.lock() {
        if map.get(session_id).map(|v| v.as_str()) == Some(request_id) {
            map.remove(session_id);
        }
    }
}

async fn auth_and_rate_limit_middleware(
    State(state): State<GatewayAppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let mut key_fingerprint = "anonymous".to_string();

    if state.auth_required {
        let provided_key = match req
            .headers()
            .get(&state.header_name)
            .and_then(|v| v.to_str().ok())
            .map(str::trim)
        {
            Some(v) if !v.is_empty() => v,
            _ => {
                if state.audit_enabled && state.log_auth_failures {
                    warn!("gateway.auth.fail reason=missing_key ip={}", addr.ip());
                }
                return error_response(StatusCode::UNAUTHORIZED, "UNAUTHORIZED", "Missing API key");
            }
        };

        let provided_hash = hash_api_key(provided_key);
        if !state.api_key_hashes.contains(&provided_hash) {
            if state.audit_enabled && state.log_auth_failures {
                warn!("gateway.auth.fail reason=invalid_key ip={}", addr.ip());
            }
            return error_response(StatusCode::UNAUTHORIZED, "UNAUTHORIZED", "Invalid API key");
        }

        key_fingerprint = provided_hash;
    }

    let _ = (addr.ip(), key_fingerprint.as_str());

    let permit = match state.concurrent_limiter.clone().try_acquire_owned() {
        Ok(p) => p,
        Err(_) => {
            return error_response(
                StatusCode::TOO_MANY_REQUESTS,
                "RATE_LIMITED",
                "Too many concurrent requests",
            )
        }
    };

    let response = next.run(req).await;
    drop(permit);
    response
}

fn resolve_service(state: &GatewayAppState, service_name: Option<&str>) -> Result<crate::services::ai::AiServiceWrapper, String> {
    if let Some(name) = service_name {
        if let Some(service) = state.ai_manager.get_service(name) {
            return Ok(service);
        }
    }

    state
        .ai_manager
        .get_service("default")
        .ok_or_else(|| "No available AI service (default)".to_string())
}

async fn ensure_conversation_exists(
    state: &GatewayAppState,
    session_id: &str,
    service_name_hint: Option<&str>,
) -> Result<(), String> {
    let exists = state
        .db
        .get_ai_conversation(session_id)
        .await
        .map_err(|e| format!("Failed to query conversation: {}", e))?
        .is_some();

    if exists {
        return Ok(());
    }

    let service = resolve_service(state, service_name_hint)?;
    let mut conv = AiConversation::new(
        service.config.model.clone(),
        service_name_hint.unwrap_or("default").to_string(),
    );
    conv.id = session_id.to_string();
    conv.model_provider = Some(service.config.provider.clone());
    conv.created_at = Utc::now();
    conv.updated_at = Utc::now();

    state
        .db
        .create_ai_conversation(&conv)
        .await
        .map_err(|e| format!("Failed to create conversation: {}", e))
}

async fn load_history_from_db(
    state: &GatewayAppState,
    session_id: &str,
) -> Result<Vec<sentinel_llm::ChatMessage>, String> {
    let db_messages = state
        .db
        .get_ai_messages_by_conversation(session_id)
        .await
        .map_err(|e| format!("Failed to load conversation history: {}", e))?;
    Ok(crate::commands::ai::reconstruct_chat_history(&db_messages))
}

async fn persist_message(
    state: &GatewayAppState,
    session_id: &str,
    role: &str,
    content: String,
) -> Result<(), String> {
    let msg = AiMessage {
        id: Uuid::new_v4().to_string(),
        conversation_id: session_id.to_string(),
        role: role.to_string(),
        content,
        metadata: None,
        token_count: None,
        cost: None,
        tool_calls: None,
        attachments: None,
        reasoning_content: None,
        timestamp: Utc::now(),
        architecture_type: None,
        architecture_meta: None,
        structured_data: None,
    };

    state
        .db
        .upsert_ai_message_append(&msg)
        .await
        .map_err(|e| format!("Failed to persist message: {}", e))
}

async fn load_tool_config_from_db(state: &GatewayAppState) -> Option<ToolConfig> {
    match state.db.get_config("agent", "tool_config").await {
        Ok(Some(config_str)) => serde_json::from_str::<ToolConfig>(&config_str).ok(),
        _ => None,
    }
}

async fn run_agent_execution(
    state: &GatewayAppState,
    session_id: &str,
    task: &str,
    system_prompt: Option<&str>,
    tool_config_override: Option<ToolConfig>,
    audit_config: Option<GatewayAuditExecuteConfig>,
    max_iterations: Option<usize>,
    timeout_secs: Option<u64>,
    enable_tenth_man_rule: Option<bool>,
) -> Result<String, String> {
    let (provider, model_name) = state
        .ai_manager
        .get_default_llm_model()
        .await
        .map_err(|e| format!("Failed to get default model: {}", e))?
        .ok_or_else(|| "Default chat model is not configured".to_string())?;

    let provider_config = state
        .ai_manager
        .get_provider_config(&provider)
        .await
        .map_err(|e| format!("Failed to load provider config '{}': {}", provider, e))?
        .ok_or_else(|| format!("Provider '{}' configuration not found", provider))?;

    let rig_provider = provider_config
        .rig_provider
        .clone()
        .unwrap_or(provider_config.provider.clone());

    let mut tool_config = if tool_config_override.is_some() {
        tool_config_override
    } else {
        load_tool_config_from_db(state).await
    };
    let audit_mode = audit_config.as_ref().map(|v| v.enabled).unwrap_or(false);
    if let Some(ref audit_cfg) = audit_config {
        if audit_cfg.enabled {
            tool_config = Some(enforce_audit_tool_whitelist(tool_config.clone(), audit_cfg));
        }
    }
    let params = AgentExecuteParams {
        execution_id: session_id.to_string(),
        model: model_name,
        system_prompt: system_prompt.unwrap_or_default().to_string(),
        task: task.to_string(),
        rig_provider,
        api_key: provider_config.api_key.clone(),
        api_base: provider_config.api_base.clone(),
        max_iterations: max_iterations.unwrap_or(provider_config.max_turns.unwrap_or(50)).max(10),
        timeout_secs: timeout_secs.unwrap_or(300),
        tool_config,
        enable_tenth_man_rule: enable_tenth_man_rule.unwrap_or(false),
        tenth_man_config: None,
        document_attachments: None,
        image_attachments: None,
        persist_messages: true,
        subagent_run_id: None,
        context_policy: None,
        recursion_depth: 0,
        audit_mode,
    };

    crate::agents::execute_agent(&state.app_handle, params)
        .await
        .map_err(|e| format!("Agent execution failed: {}", e))
}

async fn run_chat_completion(
    state: &GatewayAppState,
    service_name: Option<&str>,
    system_prompt: Option<&str>,
    message: &str,
    history: &[sentinel_llm::ChatMessage],
) -> Result<String, String> {
    let service = resolve_service(state, service_name)?;

    let llm_config = apply_generation_settings_from_db(state.db.as_ref(), service.service.to_llm_config()).await;
    let streaming_client = sentinel_llm::StreamingLlmClient::new(llm_config);

    streaming_client
        .stream_chat(system_prompt, message, history, None, |_| true)
        .await
        .map_err(|e| format!("LLM stream error: {}", e))
}

async fn create_session(State(state): State<GatewayAppState>) -> Response {
    let session_id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    if let Err(e) = ensure_conversation_exists(&state, &session_id, None).await {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", &e);
    }

    Json(CreateSessionResponse {
        session_id,
        created_at,
    })
    .into_response()
}

async fn delete_session(Path(session_id): Path<String>, State(state): State<GatewayAppState>) -> Response {
    match state.db.delete_ai_conversation(&session_id).await {
        Ok(_) => (StatusCode::NO_CONTENT, "").into_response(),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("not found") {
                error_response(StatusCode::NOT_FOUND, "NOT_FOUND", "Session not found")
            } else {
                error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    &format!("Failed to delete session: {}", msg),
                )
            }
        }
    }
}

async fn get_session_messages(
    Path(session_id): Path<String>,
    Query(query): Query<SessionMessagesQuery>,
    State(state): State<GatewayAppState>,
) -> Response {
    let request_id = Uuid::new_v4().to_string();
    let limit = query.limit.unwrap_or(200).clamp(1, 2000);

    let messages = match state.db.get_ai_messages_by_conversation(&session_id).await {
        Ok(rows) => rows,
        Err(e) => {
            return error_response_with_request_id(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                &format!("Failed to load session messages: {}", e),
                &request_id,
            );
        }
    };

    let start_index = if let Some(after_id) = query.after_id.as_deref() {
        messages
            .iter()
            .position(|m| m.id == after_id)
            .map(|idx| idx + 1)
            .unwrap_or(0)
    } else {
        0
    };

    let slice = &messages[start_index..];
    let sliced = if slice.len() > limit {
        &slice[slice.len() - limit..]
    } else {
        slice
    };

    let items = sliced
        .iter()
        .map(|m| SessionMessageItem {
            id: m.id.clone(),
            role: m.role.clone(),
            content: m.content.clone(),
            metadata: m
                .metadata
                .as_ref()
                .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok()),
            timestamp: m.timestamp.to_rfc3339(),
        })
        .collect::<Vec<_>>();

    Json(SessionMessagesResponse {
        session_id,
        request_id,
        messages: items,
    })
    .into_response()
}

async fn bridge_invoke(
    State(state): State<GatewayAppState>,
    Json(req): Json<BridgeInvokeRequest>,
) -> Response {
    let result = match req.command.as_str() {
        "init_shell_permission_handler" => Ok(json!(true)),
        "get_license_info" => Ok(json!({
            "is_licensed": true,
            "needs_activation": false
        })),
        "get_pending_shell_permissions" | "get_pending_shell_permissionss" => {
            match crate::commands::tool_commands::get_pending_shell_permissions().await {
                Ok(items) => Ok(json!(items)),
                Err(e) => Err(format!("get_pending_shell_permissions failed: {}", e)),
            }
        }
        "respond_shell_permission" => {
            let id = req
                .payload
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let allowed = req
                .payload
                .get("allowed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if id.is_empty() {
                Err("respond_shell_permission missing id".to_string())
            } else {
                match crate::commands::tool_commands::respond_shell_permission(id, allowed).await {
                    Ok(_) => Ok(json!(null)),
                    Err(e) => Err(format!("respond_shell_permission failed: {}", e)),
                }
            }
        }
        "get_ai_conversations" => {
            let services = state.ai_manager.list_services();
            if let Some(service_name) = services.first() {
                if let Some(service) = state.ai_manager.get_service(service_name) {
                    match service.list_conversations().await {
                        Ok(items) => Ok(json!(items)),
                        Err(e) => Err(format!("get_ai_conversations failed: {}", e)),
                    }
                } else {
                    Ok(json!([]))
                }
            } else {
                Ok(json!([]))
            }
        }
        "get_ai_conversations_count" => {
            let services = state.ai_manager.list_services();
            if let Some(service_name) = services.first() {
                if let Some(service) = state.ai_manager.get_service(service_name) {
                    match service.get_conversations_count().await {
                        Ok(v) => Ok(json!(v)),
                        Err(e) => Err(format!("get_ai_conversations_count failed: {}", e)),
                    }
                } else {
                    Ok(json!(0))
                }
            } else {
                Ok(json!(0))
            }
        }
        "get_ai_conversations_paginated" => {
            let limit = req
                .payload
                .get("limit")
                .and_then(|v| v.as_i64())
                .unwrap_or(20);
            let offset = req
                .payload
                .get("offset")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let services = state.ai_manager.list_services();
            if let Some(service_name) = services.first() {
                if let Some(service) = state.ai_manager.get_service(service_name) {
                    match service.list_conversations_paginated(limit, offset).await {
                        Ok(items) => Ok(json!(items)),
                        Err(e) => Err(format!("get_ai_conversations_paginated failed: {}", e)),
                    }
                } else {
                    Ok(json!([]))
                }
            } else {
                Ok(json!([]))
            }
        }
        "create_ai_conversation" => {
            let req_obj = req
                .payload
                .get("request")
                .cloned()
                .ok_or_else(|| "create_ai_conversation missing request".to_string())
                .and_then(|v| serde_json::from_value::<BridgeCreateConversationRequest>(v).map_err(|e| e.to_string()));
            match req_obj {
                Ok(v) => {
                    let service_name = if v.service_name.trim().is_empty() {
                        "default"
                    } else {
                        v.service_name.as_str()
                    };
                    if let Some(service) = state.ai_manager.get_service(service_name).or_else(|| state.ai_manager.get_service("default")) {
                        match service.create_conversation(Some(v.title)).await {
                            Ok(id) => Ok(json!(id)),
                            Err(e) => Err(format!("create_ai_conversation failed: {}", e)),
                        }
                    } else {
                        Err("No available AI service".to_string())
                    }
                }
                Err(e) => Err(e),
            }
        }
        "get_ai_messages_by_conversation" => {
            let conversation_id = req
                .payload
                .get("conversation_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if conversation_id.is_empty() {
                Err("get_ai_messages_by_conversation missing conversation_id".to_string())
            } else {
                match state.db.get_ai_messages_by_conversation(&conversation_id).await {
                    Ok(items) => Ok(json!(items)),
                    Err(e) => Err(format!("get_ai_messages_by_conversation failed: {}", e)),
                }
            }
        }
        "save_ai_message" => {
            let req_obj = req
                .payload
                .get("request")
                .cloned()
                .ok_or_else(|| "save_ai_message missing request".to_string())
                .and_then(|v| serde_json::from_value::<BridgeSaveMessageRequest>(v).map_err(|e| e.to_string()));
            match req_obj {
                Ok(v) => {
                    let msg = AiMessage {
                        id: v.id.unwrap_or_else(|| Uuid::new_v4().to_string()),
                        conversation_id: v.conversation_id,
                        role: v.role,
                        content: v.content,
                        metadata: v.metadata.as_ref().and_then(|x| serde_json::to_string(x).ok()),
                        token_count: None,
                        cost: None,
                        tool_calls: None,
                        attachments: None,
                        reasoning_content: None,
                        timestamp: Utc::now(),
                        architecture_type: v.architecture_type,
                        architecture_meta: v
                            .architecture_meta
                            .as_ref()
                            .and_then(|x| serde_json::to_string(x).ok()),
                        structured_data: v
                            .structured_data
                            .as_ref()
                            .and_then(|x| serde_json::to_string(x).ok()),
                    };
                    match state.db.create_ai_message(&msg).await {
                        Ok(_) => Ok(json!(null)),
                        Err(e) => Err(format!("save_ai_message failed: {}", e)),
                    }
                }
                Err(e) => Err(e),
            }
        }
        "update_ai_conversation_title" => {
            let conversation_id = req.payload.get("conversationId").or_else(|| req.payload.get("conversation_id"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let title = req.payload.get("title").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let service_name = req.payload.get("serviceName").or_else(|| req.payload.get("service_name"))
                .and_then(|v| v.as_str())
                .unwrap_or("default")
                .to_string();
            if conversation_id.is_empty() {
                Err("update_ai_conversation_title missing conversation_id".to_string())
            } else if let Some(service) = state.ai_manager.get_service(&service_name).or_else(|| state.ai_manager.get_service("default")) {
                match service.update_conversation_title(&conversation_id, &title).await {
                    Ok(_) => Ok(json!(null)),
                    Err(e) => Err(format!("update_ai_conversation_title failed: {}", e)),
                }
            } else {
                Err("No available AI service".to_string())
            }
        }
        "delete_ai_conversation" => {
            let conversation_id = req.payload.get("conversationId").or_else(|| req.payload.get("conversation_id"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let service_name = req.payload.get("serviceName").or_else(|| req.payload.get("service_name"))
                .and_then(|v| v.as_str())
                .unwrap_or("default")
                .to_string();
            if conversation_id.is_empty() {
                Err("delete_ai_conversation missing conversation_id".to_string())
            } else if let Some(service) = state.ai_manager.get_service(&service_name).or_else(|| state.ai_manager.get_service("default")) {
                match service.delete_conversation(&conversation_id).await {
                    Ok(_) => Ok(json!(null)),
                    Err(e) => Err(format!("delete_ai_conversation failed: {}", e)),
                }
            } else {
                Err("No available AI service".to_string())
            }
        }
        "delete_ai_message" => {
            let message_id = req.payload.get("message_id").or_else(|| req.payload.get("messageId"))
                .and_then(|v| v.as_str()).unwrap_or_default().to_string();
            if message_id.is_empty() {
                Err("delete_ai_message missing message_id".to_string())
            } else {
                match state.db.delete_ai_message(&message_id).await {
                    Ok(_) => Ok(json!(null)),
                    Err(e) => Err(format!("delete_ai_message failed: {}", e)),
                }
            }
        }
        "delete_ai_messages_after" => {
            let conversation_id = req.payload.get("conversation_id").or_else(|| req.payload.get("conversationId"))
                .and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let message_id = req.payload.get("message_id").or_else(|| req.payload.get("messageId"))
                .and_then(|v| v.as_str()).unwrap_or_default().to_string();
            if conversation_id.is_empty() || message_id.is_empty() {
                Err("delete_ai_messages_after missing conversation_id or message_id".to_string())
            } else {
                match state.db.delete_ai_messages_after(&conversation_id, &message_id).await {
                    Ok(n) => Ok(json!(n)),
                    Err(e) => Err(format!("delete_ai_messages_after failed: {}", e)),
                }
            }
        }
        "clear_conversation_messages" => {
            let conversation_id = req.payload.get("conversation_id").or_else(|| req.payload.get("conversationId"))
                .and_then(|v| v.as_str()).unwrap_or_default().to_string();
            if conversation_id.is_empty() {
                Err("clear_conversation_messages missing conversation_id".to_string())
            } else {
                match state.db.delete_ai_messages_by_conversation(&conversation_id).await {
                    Ok(_) => Ok(json!(null)),
                    Err(e) => Err(format!("clear_conversation_messages failed: {}", e)),
                }
            }
        }
        "get_tool_config" => {
            match load_tool_config_from_db(&state).await {
                Some(cfg) => Ok(json!(cfg)),
                None => Ok(json!(null)),
            }
        }
        "save_tool_config" => {
            let tool_config = req
                .payload
                .get("toolConfig")
                .or_else(|| req.payload.get("tool_config"))
                .cloned()
                .ok_or_else(|| "save_tool_config missing tool_config".to_string())
                .and_then(|v| serde_json::from_value::<ToolConfig>(v).map_err(|e| e.to_string()));
            match tool_config {
                Ok(cfg) => {
                    let raw = serde_json::to_string(&cfg).map_err(|e| e.to_string());
                    match raw {
                        Ok(value) => match state.db.set_config("agent", "tool_config", &value, None).await {
                            Ok(_) => Ok(json!(null)),
                            Err(e) => Err(format!("save_tool_config failed: {}", e)),
                        },
                        Err(e) => Err(e),
                    }
                }
                Err(e) => Err(e),
            }
        }
        "get_ai_roles" => {
            match state.db.get_ai_roles().await {
                Ok(items) => Ok(json!(items)),
                Err(e) => Err(format!("get_ai_roles failed: {}", e)),
            }
        }
        "get_current_ai_role" => {
            match state.db.get_current_ai_role().await {
                Ok(v) => Ok(json!(v)),
                Err(e) => Err(format!("get_current_ai_role failed: {}", e)),
            }
        }
        "set_current_ai_role" => {
            let role_id = req.payload.get("roleId").or_else(|| req.payload.get("role_id"))
                .and_then(|v| v.as_str())
                .map(|v| v.to_string());
            match state.db.set_current_ai_role(role_id.as_deref()).await {
                Ok(_) => Ok(json!(null)),
                Err(e) => Err(format!("set_current_ai_role failed: {}", e)),
            }
        }
        "create_ai_role" => {
            let payload = req.payload.get("payload").cloned().unwrap_or_default();
            let title = payload.get("title").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let description = payload.get("description").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let prompt = payload.get("prompt").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            if title.is_empty() {
                Err("create_ai_role missing title".to_string())
            } else {
                let now = Utc::now();
                let role = sentinel_core::models::ai::AiRole {
                    id: Uuid::new_v4().to_string(),
                    title,
                    description,
                    prompt,
                    is_system: false,
                    created_at: now,
                    updated_at: now,
                };
                match state.db.create_ai_role(&role).await {
                    Ok(_) => Ok(json!(role)),
                    Err(e) => Err(format!("create_ai_role failed: {}", e)),
                }
            }
        }
        "update_ai_role" => {
            let payload = req.payload.get("payload").cloned().unwrap_or_default();
            let id = payload.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let title = payload.get("title").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let description = payload.get("description").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let prompt = payload.get("prompt").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            if id.is_empty() {
                Err("update_ai_role missing id".to_string())
            } else {
                let roles = state.db.get_ai_roles().await.map_err(|e| e.to_string());
                match roles {
                    Ok(items) => {
                        let existing = items.iter().find(|r| r.id == id);
                        let role = sentinel_core::models::ai::AiRole {
                            id,
                            title,
                            description,
                            prompt,
                            is_system: existing.map(|r| r.is_system).unwrap_or(false),
                            created_at: existing.map(|r| r.created_at).unwrap_or_else(Utc::now),
                            updated_at: Utc::now(),
                        };
                        match state.db.update_ai_role(&role).await {
                            Ok(_) => Ok(json!(null)),
                            Err(e) => Err(format!("update_ai_role failed: {}", e)),
                        }
                    }
                    Err(e) => Err(format!("update_ai_role failed: {}", e)),
                }
            }
        }
        "delete_ai_role" => {
            let id = req.payload.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            if id.is_empty() {
                Err("delete_ai_role missing id".to_string())
            } else {
                match state.db.delete_ai_role(&id).await {
                    Ok(_) => Ok(json!(null)),
                    Err(e) => Err(format!("delete_ai_role failed: {}", e)),
                }
            }
        }
        "cancel_ai_stream" => {
            let conversation_id = req.payload.get("conversation_id").or_else(|| req.payload.get("conversationId"))
                .and_then(|v| v.as_str()).unwrap_or_default().to_string();
            if conversation_id.is_empty() {
                Err("cancel_ai_stream missing conversation_id".to_string())
            } else {
                let _ = crate::managers::cancellation_manager::cancel_execution(&conversation_id).await;
                Ok(json!(null))
            }
        }
        "cancel_shell_execution" => {
            let execution_id = req
                .payload
                .get("execution_id")
                .or_else(|| req.payload.get("executionId"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if execution_id.is_empty() {
                Err("cancel_shell_execution missing execution_id".to_string())
            } else {
                let _ = sentinel_tools::buildin_tools::shell::cancel_shell_execution(&execution_id).await;
                Ok(json!(null))
            }
        }
        "get_tool_usage_stats" => {
            let stats = get_tool_usage_statistics().await;
            Ok(json!(stats))
        }
        "get_all_tool_metadata" => {
            let router = crate::agents::tool_router::ToolRouter::new_with_all_tools(Some(&state.db)).await;
            let tools = router
                .list_all_tools()
                .into_iter()
                .filter(|t| t.id != sentinel_tools::buildin_tools::SkillsTool::NAME)
                .collect::<Vec<_>>();
            Ok(json!(tools))
        }
        "get_tool_statistics" => {
            let router = crate::agents::tool_router::ToolRouter::new_with_all_tools(Some(&state.db)).await;
            Ok(json!(router.get_statistics()))
        }
        "clear_tool_usage_stats" => {
            clear_tool_usage_records().await;
            Ok(json!(null))
        }
        "get_config" => {
            let request = req.payload.get("request").cloned().unwrap_or_else(|| req.payload.clone());
            let category = request.get("category").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let key = request.get("key").and_then(|v| v.as_str()).map(|v| v.to_string());
            if category.is_empty() {
                Err("get_config missing category".to_string())
            } else if let Some(key) = key {
                match state.db.get_config(&category, &key).await {
                    Ok(Some(value)) => Ok(json!([{
                        "id": format!("cfg_{}_{}", category, key),
                        "category": category,
                        "key": key,
                        "value": value,
                        "description": serde_json::Value::Null,
                        "is_encrypted": false
                    }])),
                    Ok(None) => Ok(json!([])),
                    Err(e) => Err(format!("get_config failed: {}", e)),
                }
            } else {
                match state.db.get_configs_by_category(&category).await {
                    Ok(items) => Ok(json!(items.into_iter().map(|c| json!({
                        "id": c.id,
                        "category": c.category,
                        "key": c.key,
                        "value": c.value.unwrap_or_default(),
                        "description": c.description,
                        "is_encrypted": c.is_encrypted
                    })).collect::<Vec<_>>())),
                    Err(e) => Err(format!("get_config failed: {}", e)),
                }
            }
        }
        "set_config" => {
            let category = req.payload.get("category").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let key = req.payload.get("key").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let value = req.payload.get("value").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            if category.is_empty() || key.is_empty() {
                Err("set_config missing category or key".to_string())
            } else {
                match state.db.set_config(&category, &key, &value, None).await {
                    Ok(_) => Ok(json!(null)),
                    Err(e) => Err(format!("set_config failed: {}", e)),
                }
            }
        }
        "get_agent_config" => {
            let shell = crate::commands::tool_commands::agent_config::load_shell_config_from_db(state.db.as_ref()).await;
            let terminal = crate::commands::tool_commands::agent_config::load_terminal_config_from_db(state.db.as_ref()).await;
            let image_attachments = crate::commands::tool_commands::agent_config::load_image_attachment_config_from_db(state.db.as_ref()).await;
            let subagent = crate::commands::tool_commands::agent_config::load_subagent_config_from_db(state.db.as_ref()).await;
            Ok(json!({
                "shell": shell,
                "terminal": terminal,
                "image_attachments": image_attachments,
                "subagent": subagent
            }))
        }
        "save_agent_config" => {
            let save_result: Result<(), String> = async {
                if let Some(config) = req.payload.get("config").or_else(|| req.payload.get("payload")) {
                    if let Some(terminal) = config.get("terminal") {
                        if let Some(v) = terminal.get("docker_image").and_then(|v| v.as_str()) {
                            state.db.set_config("agent", "terminal_docker_image", v, None).await
                                .map_err(|e| format!("save_agent_config failed: {}", e))?;
                        }
                        if let Some(v) = terminal.get("default_execution_mode").and_then(|v| v.as_str()) {
                            state.db.set_config("agent", "default_execution_mode", v, None).await
                                .map_err(|e| format!("save_agent_config failed: {}", e))?;
                        }
                        if let Some(v) = terminal.get("docker_memory_limit").and_then(|v| v.as_str()) {
                            state.db.set_config("agent", "docker_memory_limit", v, None).await
                                .map_err(|e| format!("save_agent_config failed: {}", e))?;
                        }
                        if let Some(v) = terminal.get("docker_cpu_limit").and_then(|v| v.as_str()) {
                            state.db.set_config("agent", "docker_cpu_limit", v, None).await
                                .map_err(|e| format!("save_agent_config failed: {}", e))?;
                        }
                        if let Some(v) = terminal.get("docker_use_host_network").and_then(|v| v.as_bool()) {
                            let val = if v { "true" } else { "false" };
                            state.db.set_config("agent", "docker_use_host_network", val, None).await
                                .map_err(|e| format!("save_agent_config failed: {}", e))?;
                        }
                    }
                    if let Some(subagent) = config.get("subagent") {
                        if let Some(v) = subagent.get("timeout_secs").and_then(|v| v.as_u64()) {
                            state.db.set_config("agent", "subagent_timeout_secs", &v.to_string(), None).await
                                .map_err(|e| format!("save_agent_config failed: {}", e))?;
                        }
                    }
                    if let Some(image_cfg) = config.get("image_attachments") {
                        if let Some(v) = image_cfg.get("mode").and_then(|v| v.as_str()) {
                            state.db.set_config("agent", "image_attachment_mode", v, None).await
                                .map_err(|e| format!("save_agent_config failed: {}", e))?;
                        }
                        if let Some(v) = image_cfg.get("allow_upload_to_model").and_then(|v| v.as_bool()) {
                            let val = if v { "true" } else { "false" };
                            state.db.set_config("agent", "allow_image_upload_to_model", val, None).await
                                .map_err(|e| format!("save_agent_config failed: {}", e))?;
                        }
                    }
                }
                Ok(())
            }.await;
            match save_result {
                Ok(_) => Ok(json!(null)),
                Err(e) => Err(e),
            }
        }
        "save_audit_policy_gate" => Ok(json!(null)),
        "get_subagent_runs" => {
            let parent_id = req
                .payload
                .get("parentExecutionId")
                .or_else(|| req.payload.get("parent_execution_id"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if parent_id.is_empty() {
                Err("get_subagent_runs missing parent_execution_id".to_string())
            } else {
                match state.db.get_subagent_runs_by_parent_internal(&parent_id).await {
                    Ok(v) => Ok(json!(v)),
                    Err(e) => Err(format!("get_subagent_runs failed: {}", e)),
                }
            }
        }
        "start_terminal_server" => {
            let cfg = req
                .payload
                .get("config")
                .cloned()
                .and_then(|v| serde_json::from_value::<crate::commands::terminal_commands::TerminalServerConfig>(v).ok());
            match crate::commands::terminal_commands::start_terminal_server(cfg).await {
                Ok(v) => Ok(json!(v)),
                Err(e) => Err(format!("start_terminal_server failed: {}", e)),
            }
        }
        "stop_terminal_server" => {
            match crate::commands::terminal_commands::stop_terminal_server().await {
                Ok(v) => Ok(json!(v)),
                Err(e) => Err(format!("stop_terminal_server failed: {}", e)),
            }
        }
        "get_terminal_server_status" => {
            match crate::commands::terminal_commands::get_terminal_server_status().await {
                Ok(v) => Ok(json!(v)),
                Err(e) => Err(format!("get_terminal_server_status failed: {}", e)),
            }
        }
        "list_terminal_sessions" => {
            match crate::commands::terminal_commands::list_terminal_sessions().await {
                Ok(v) => Ok(json!(v)),
                Err(e) => Err(format!("list_terminal_sessions failed: {}", e)),
            }
        }
        "stop_terminal_session" => {
            let session_id = req.payload.get("sessionId").or_else(|| req.payload.get("session_id"))
                .and_then(|v| v.as_str()).unwrap_or_default().to_string();
            if session_id.is_empty() {
                Err("stop_terminal_session missing session_id".to_string())
            } else {
                match crate::commands::terminal_commands::stop_terminal_session(session_id).await {
                    Ok(v) => Ok(json!(v)),
                    Err(e) => Err(format!("stop_terminal_session failed: {}", e)),
                }
            }
        }
        "get_terminal_websocket_url" => {
            match crate::commands::terminal_commands::get_terminal_websocket_url().await {
                Ok(v) => Ok(json!(v)),
                Err(e) => Err(format!("get_terminal_websocket_url failed: {}", e)),
            }
        }
        "list_skills" => {
            match state.db.list_skills_summary().await {
                Ok(v) => Ok(json!(v)),
                Err(e) => Err(format!("list_skills failed: {}", e)),
            }
        }
        "list_skills_full" => {
            match state.db.list_all_skills().await {
                Ok(v) => Ok(json!(v)),
                Err(e) => Err(format!("list_skills_full failed: {}", e)),
            }
        }
        "refresh_skills_index" => {
            match scan_and_upsert_skills(state.db.as_ref()).await {
                Ok(v) => Ok(json!(v)),
                Err(e) => Err(format!("refresh_skills_index failed: {}", e)),
            }
        }
        "get_skill_markdown" => {
            let id = req.payload.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            if id.is_empty() {
                Err("get_skill_markdown missing id".to_string())
            } else {
                match state.db.get_skill(&id).await {
                    Ok(Some(skill)) => {
                        let root = skills_root(state.db.as_ref());
                        let skill_path = if !skill.source_path.is_empty() {
                            root.join(&skill.source_path)
                        } else {
                            root.join(&skill.id).join("SKILL.md")
                        };
                        match std::fs::read_to_string(&skill_path) {
                            Ok(content) => match parse_skill_markdown(&content) {
                                Ok(doc) => Ok(json!(doc.body)),
                                Err(e) => Err(format!("get_skill_markdown failed: {}", e)),
                            },
                            Err(e) => Err(format!("get_skill_markdown failed: {}", e)),
                        }
                    }
                    Ok(None) => Err("Skill not found".to_string()),
                    Err(e) => Err(format!("get_skill_markdown failed: {}", e)),
                }
            }
        }
        "create_skill" => {
            let payload = req.payload.get("payload").cloned().unwrap_or_else(|| req.payload.clone());
            let name = payload.get("name").and_then(|v| v.as_str()).unwrap_or_default();
            match normalize_skill_id(name) {
                Ok(id) => {
                    let description = payload.get("description").and_then(|v| v.as_str()).unwrap_or_default().trim().to_string();
                    if description.is_empty() {
                        Err("Skill description is required".to_string())
                    } else {
                        match state.db.get_skill(&id).await {
                            Ok(Some(_)) => Err("Skill name already exists".to_string()),
                            Ok(None) => {
                                let root = skills_root(state.db.as_ref());
                                let skill_dir = root.join(&id);
                                let skill_md = skill_dir.join("SKILL.md");
                                if let Err(e) = std::fs::create_dir_all(&skill_dir) {
                                    Err(format!("create_skill failed: {}", e))
                                } else {
                                    let content = payload.get("content").and_then(|v| v.as_str()).unwrap_or_default();
                                    let markdown = build_skill_markdown(&id, &description, content);
                                    if let Err(e) = std::fs::write(&skill_md, markdown) {
                                        Err(format!("create_skill failed: {}", e))
                                    } else {
                                        let create_payload = sentinel_db::CreateSkill {
                                            id: id.clone(),
                                            name: id.clone(),
                                            description,
                                            source_path: skill_md
                                                .strip_prefix(&root)
                                                .unwrap_or(&skill_md)
                                                .to_string_lossy()
                                                .to_string(),
                                            argument_hint: payload.get("argument_hint").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                                            disable_model_invocation: payload.get("disable_model_invocation").and_then(|v| v.as_bool()).unwrap_or(false),
                                            user_invocable: payload.get("user_invocable").and_then(|v| v.as_bool()).unwrap_or(true),
                                            allowed_tools: payload.get("allowed_tools")
                                                .and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok())
                                                .unwrap_or_default(),
                                            model: payload.get("model").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                                            context: payload.get("context").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                                            agent: payload.get("agent").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                                            hooks: payload.get("hooks").cloned(),
                                        };
                                        match state.db.create_skill(&create_payload).await {
                                            Ok(skill) => Ok(json!(skill)),
                                            Err(e) => Err(format!("create_skill failed: {}", e)),
                                        }
                                    }
                                }
                            }
                            Err(e) => Err(format!("create_skill failed: {}", e)),
                        }
                    }
                }
                Err(e) => Err(e),
            }
        }
        "update_skill" => {
            let id = req.payload.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let payload = req.payload.get("payload").cloned().unwrap_or_else(|| req.payload.clone());
            if id.is_empty() {
                Err("update_skill missing id".to_string())
            } else {
                let root = skills_root(state.db.as_ref());
                let skill_dir = root.join(&id);
                let mut write_error: Option<String> = None;
                if let Some(content) = payload.get("content").and_then(|v| v.as_str()) {
                    let desc = payload.get("description").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                    let desc = if desc.is_empty() {
                        match state.db.get_skill(&id).await {
                            Ok(Some(existing)) => existing.description,
                            _ => String::new(),
                        }
                    } else {
                        desc
                    };
                    if desc.is_empty() {
                        write_error = Some("update_skill missing description".to_string());
                    } else {
                        let _ = std::fs::create_dir_all(&skill_dir);
                        let _ = std::fs::write(skill_dir.join("SKILL.md"), build_skill_markdown(&id, &desc, content));
                    }
                }

                if let Some(err) = write_error {
                    Err(err)
                } else {
                    let update_payload = sentinel_db::UpdateSkill {
                        name: payload.get("name").and_then(|v| v.as_str()).map(|v| v.to_string()),
                        description: payload.get("description").and_then(|v| v.as_str()).map(|v| v.to_string()),
                        source_path: Some(skill_dir.join("SKILL.md")
                            .strip_prefix(&root)
                            .unwrap_or(&skill_dir.join("SKILL.md"))
                            .to_string_lossy()
                            .to_string()),
                        argument_hint: payload.get("argument_hint").and_then(|v| v.as_str()).map(|v| v.to_string()),
                        disable_model_invocation: payload.get("disable_model_invocation").and_then(|v| v.as_bool()),
                        user_invocable: payload.get("user_invocable").and_then(|v| v.as_bool()),
                        allowed_tools: payload.get("allowed_tools").and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok()),
                        model: payload.get("model").and_then(|v| v.as_str()).map(|v| v.to_string()),
                        context: payload.get("context").and_then(|v| v.as_str()).map(|v| v.to_string()),
                        agent: payload.get("agent").and_then(|v| v.as_str()).map(|v| v.to_string()),
                        hooks: payload.get("hooks").cloned(),
                    };
                    match state.db.update_skill(&id, &update_payload).await {
                        Ok(v) => Ok(json!(v)),
                        Err(e) => Err(format!("update_skill failed: {}", e)),
                    }
                }
            }
        }
        "delete_skill" => {
            let id = req.payload.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            if id.is_empty() {
                Err("delete_skill missing id".to_string())
            } else {
                let root = skills_root(state.db.as_ref());
                let dir = root.join(&id);
                if dir.exists() {
                    let _ = std::fs::remove_dir_all(&dir);
                }
                match state.db.delete_skill(&id).await {
                    Ok(v) => Ok(json!(v)),
                    Err(e) => Err(format!("delete_skill failed: {}", e)),
                }
            }
        }
        "list_skill_files" => {
            let id = req.payload.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            if id.is_empty() {
                Err("list_skill_files missing id".to_string())
            } else {
                let root = skills_root(state.db.as_ref());
                let skill_dir = root.join(&id);
                if !skill_dir.exists() {
                    Err("Skill directory not found".to_string())
                } else {
                    let mut out = Vec::new();
                    for entry in walkdir::WalkDir::new(&skill_dir).max_depth(5).into_iter().filter_map(|e| e.ok()) {
                        if entry.file_type().is_file() {
                            let p = entry.path();
                            let rel = p.strip_prefix(&skill_dir).unwrap_or(p).to_string_lossy().to_string();
                            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                            out.push(json!({ "path": rel, "size": size }));
                        }
                    }
                    out.sort_by(|a, b| a.get("path").and_then(|v| v.as_str()).cmp(&b.get("path").and_then(|v| v.as_str())));
                    Ok(json!(out))
                }
            }
        }
        "read_skill_file" => {
            let id = req.payload.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let path = req.payload.get("path").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            if id.is_empty() || path.is_empty() {
                Err("read_skill_file missing id or path".to_string())
            } else {
                let root = skills_root(state.db.as_ref());
                let skill_dir = root.join(&id);
                match resolve_skill_file_for_read(&skill_dir, &path) {
                    Ok(p) => match std::fs::read_to_string(&p) {
                        Ok(v) => Ok(json!(v)),
                        Err(e) => Err(format!("read_skill_file failed: {}", e)),
                    },
                    Err(e) => Err(format!("read_skill_file failed: {}", e)),
                }
            }
        }
        "save_skill_file" => {
            let id = req.payload.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let path = req.payload.get("path").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let content = req.payload.get("content").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            if id.is_empty() || path.is_empty() {
                Err("save_skill_file missing id or path".to_string())
            } else {
                let root = skills_root(state.db.as_ref());
                let skill_dir = root.join(&id);
                if !skill_dir.exists() {
                    Err("Skill directory not found".to_string())
                } else {
                    match resolve_skill_file_for_write(&skill_dir, &path) {
                        Ok(p) => match std::fs::write(&p, content) {
                            Ok(_) => Ok(json!(true)),
                            Err(e) => Err(format!("save_skill_file failed: {}", e)),
                        },
                        Err(e) => Err(format!("save_skill_file failed: {}", e)),
                    }
                }
            }
        }
        "delete_skill_file" => {
            let id = req.payload.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let path = req.payload.get("path").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            if id.is_empty() || path.is_empty() {
                Err("delete_skill_file missing id or path".to_string())
            } else {
                let root = skills_root(state.db.as_ref());
                let skill_dir = root.join(&id);
                match resolve_skill_file_for_read(&skill_dir, &path) {
                    Ok(p) => match std::fs::remove_file(&p) {
                        Ok(_) => Ok(json!(true)),
                        Err(e) => Err(format!("delete_skill_file failed: {}", e)),
                    },
                    Err(e) => Err(format!("delete_skill_file failed: {}", e)),
                }
            }
        }
        "import_skill_file" => {
            let id = req.payload.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let source_path = req.payload.get("sourcePath").or_else(|| req.payload.get("source_path"))
                .and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let target_path = req.payload.get("targetPath").or_else(|| req.payload.get("target_path"))
                .and_then(|v| v.as_str()).map(|v| v.to_string());
            if id.is_empty() || source_path.is_empty() {
                Err("import_skill_file missing id or source_path".to_string())
            } else {
                let source = PathBuf::from(&source_path);
                if !source.is_file() {
                    Err("Source file not found".to_string())
                } else {
                    let filename = source.file_name().and_then(|v| v.to_str()).unwrap_or("imported.txt").to_string();
                    let rel_target = target_path.unwrap_or(filename);
                    let root = skills_root(state.db.as_ref());
                    let skill_dir = root.join(&id);
                    match resolve_skill_file_for_write(&skill_dir, &rel_target) {
                        Ok(p) => match std::fs::copy(&source, &p) {
                            Ok(_) => Ok(json!(rel_target)),
                            Err(e) => Err(format!("import_skill_file failed: {}", e)),
                        },
                        Err(e) => Err(format!("import_skill_file failed: {}", e)),
                    }
                }
            }
        }
        "web_explorer_send_user_message" => Ok(json!(null)),
        "web_explorer_skip_login" => Ok(json!(null)),
        "web_explorer_manual_login_complete" => Ok(json!(null)),
        "web_explorer_receive_credentials" => Ok(json!(null)),
        "generate_plugin_stream" => Ok(json!(null)),
        "agent_execute" => {
            let req_obj = serde_json::from_value::<BridgeAgentExecuteRequest>(req.payload.clone())
                .map_err(|e| format!("invalid agent_execute payload: {}", e));
            match req_obj {
                Ok(v) => {
                    let conversation_id = v
                        .config
                        .as_ref()
                        .and_then(|c| c.get("conversation_id").and_then(|x| x.as_str()))
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| Uuid::new_v4().to_string());
                    let system_prompt = state
                        .db
                        .get_current_ai_role()
                        .await
                        .ok()
                        .flatten()
                        .map(|r| r.prompt);
                    let tool_config = v
                        .config
                        .as_ref()
                        .and_then(|c| c.get("tool_config"))
                        .and_then(|x| serde_json::from_value::<ToolConfig>(x.clone()).ok());
                    let audit_config = v
                        .config
                        .as_ref()
                        .and_then(|c| c.get("audit_config"))
                        .and_then(|x| serde_json::from_value::<GatewayAuditExecuteConfig>(x.clone()).ok());
                    let max_iterations = v
                        .config
                        .as_ref()
                        .and_then(|c| c.get("max_iterations").and_then(|x| x.as_u64()))
                        .map(|v| v as usize);
                    let timeout_secs = v
                        .config
                        .as_ref()
                        .and_then(|c| c.get("timeout_secs").and_then(|x| x.as_u64()));
                    let enable_tenth_man_rule = v
                        .config
                        .as_ref()
                        .and_then(|c| c.get("enable_tenth_man_rule").and_then(|x| x.as_bool()));
                    if let Err(e) = ensure_conversation_exists(&state, &conversation_id, Some("default")).await {
                        Err(e)
                    } else {
                        match run_agent_execution(
                            &state,
                            &conversation_id,
                            &v.task,
                            system_prompt.as_deref(),
                            tool_config,
                            audit_config,
                            max_iterations,
                            timeout_secs,
                            enable_tenth_man_rule,
                        )
                        .await
                        {
                            Ok(_) => {
                                let message_id = v
                                    .config
                                    .as_ref()
                                    .and_then(|c| c.get("message_id").and_then(|x| x.as_str()))
                                    .map(|v| v.to_string())
                                    .unwrap_or_else(|| Uuid::new_v4().to_string());
                                Ok(json!(message_id))
                            }
                            Err(e) => Err(format!("agent_execute failed: {}", e)),
                        }
                    }
                }
                Err(e) => Err(e),
            }
        }
        "http_gateway_get_status" => Ok(json!({
            "running": true,
            "bind_addr": serde_json::Value::Null,
            "started_at": serde_json::Value::Null
        })),
        _ => {
            warn!("http_gateway.bridge.unimplemented command={}", req.command);
            Ok(json!(null))
        }
    };

    match result {
        Ok(data) => Json(BridgeInvokeResponse {
            ok: true,
            data: Some(data),
            error: None,
        })
        .into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(BridgeInvokeResponse {
                ok: false,
                data: None,
                error: Some(err),
            }),
        )
            .into_response(),
    }
}

async fn get_pending_permissions(Query(query): Query<PendingPermissionsQuery>) -> Response {
    let request_id = Uuid::new_v4().to_string();
    match crate::commands::tool_commands::get_pending_shell_permissions().await {
        Ok(items) => {
            let filtered = if let Some(session_id) = query.session_id.as_deref() {
                items
                    .into_iter()
                    .filter(|v| v.execution_id.as_deref() == Some(session_id))
                    .collect()
            } else {
                items
            };
            Json(PendingPermissionsResponse { items: filtered, request_id }).into_response()
        }
        Err(e) => error_response_with_request_id(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            &format!("Failed to query pending permissions: {}", e),
            &request_id,
        ),
    }
}

async fn respond_permission(Json(payload): Json<PermissionRespondRequest>) -> Response {
    let request_id = payload
        .request_id
        .clone()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    if payload.id.trim().is_empty() {
        return error_response_with_request_id(
            StatusCode::BAD_REQUEST,
            "BAD_REQUEST",
            "id is required",
            &request_id,
        );
    }

    if let Some(session_id) = payload.session_id.as_deref() {
        match crate::commands::tool_commands::get_pending_shell_permissions().await {
            Ok(items) => {
                let matched = items
                    .iter()
                    .find(|v| v.id == payload.id)
                    .map(|v| v.execution_id.as_deref() == Some(session_id))
                    .unwrap_or(false);
                if !matched {
                    return error_response_with_request_id(
                        StatusCode::FORBIDDEN,
                        "FORBIDDEN",
                        "permission request does not belong to this session",
                        &request_id,
                    );
                }
            }
            Err(e) => {
                return error_response_with_request_id(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    &format!("Failed to validate permission request: {}", e),
                    &request_id,
                );
            }
        }
    }

    match crate::commands::tool_commands::respond_shell_permission(payload.id, payload.allowed).await {
        Ok(_) => Json(json!({
            "ok": true,
            "request_id": request_id,
        }))
        .into_response(),
        Err(e) => error_response_with_request_id(
            StatusCode::BAD_REQUEST,
            "BAD_REQUEST",
            &format!("Failed to respond permission: {}", e),
            &request_id,
        ),
    }
}

async fn chat(State(state): State<GatewayAppState>, Json(payload): Json<ChatRequest>) -> Response {
    let request_id = payload
        .request_id
        .clone()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    if payload.message.trim().is_empty() {
        return error_response_with_request_id(
            StatusCode::BAD_REQUEST,
            "BAD_REQUEST",
            "message is required",
            &request_id,
        );
    }

    let session_id = payload
        .session_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let mode = match parse_mode(payload.mode.as_deref()) {
        Ok(v) => v,
        Err(e) => return error_response_with_request_id(StatusCode::BAD_REQUEST, "BAD_REQUEST", &e, &request_id),
    };

    if let Err(e) = ensure_conversation_exists(&state, &session_id, payload.service_name.as_deref()).await {
        return error_response_with_request_id(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", &e, &request_id);
    }

    if let Err(e) = try_register_active_execution(&state, &session_id, &request_id) {
        return error_response_with_request_id(StatusCode::CONFLICT, "CONFLICT", &e, &request_id);
    }

    let history = match load_history_from_db(&state, &session_id).await {
        Ok(h) => h,
        Err(e) => {
            release_active_execution(&state, &session_id, &request_id);
            return error_response_with_request_id(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", &e, &request_id);
        }
    };

    let completion = if mode == GatewayMode::Agent {
        match run_agent_execution(
            &state,
            &session_id,
            &payload.message,
            payload.system_prompt.as_deref(),
            payload.tool_config.clone(),
            payload.audit_config.clone(),
            payload.max_iterations,
            payload.timeout_secs,
            payload.enable_tenth_man_rule,
        )
        .await
        {
            Ok(text) => text,
            Err(e) => {
                release_active_execution(&state, &session_id, &request_id);
                return error_response_with_request_id(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", &e, &request_id);
            }
        }
    } else {
        let completion = match run_chat_completion(
            &state,
            payload.service_name.as_deref(),
            payload.system_prompt.as_deref(),
            &payload.message,
            &history,
        )
        .await
        {
            Ok(text) => text,
            Err(e) => {
                release_active_execution(&state, &session_id, &request_id);
                return error_response_with_request_id(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", &e, &request_id);
            }
        };

        if let Err(e) = persist_message(&state, &session_id, "user", payload.message.clone()).await {
            release_active_execution(&state, &session_id, &request_id);
            return error_response_with_request_id(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", &e, &request_id);
        }
        if let Err(e) = persist_message(&state, &session_id, "assistant", completion.clone()).await {
            release_active_execution(&state, &session_id, &request_id);
            return error_response_with_request_id(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", &e, &request_id);
        }
        completion
    };

    release_active_execution(&state, &session_id, &request_id);

    Json(ChatResponse {
        id: Uuid::new_v4().to_string(),
        session_id,
        message: completion,
        request_id,
        mode: if mode == GatewayMode::Agent { "agent".to_string() } else { "llm".to_string() },
    })
    .into_response()
}

async fn session_chat(
    Path(session_id): Path<String>,
    State(state): State<GatewayAppState>,
    Json(payload): Json<SessionChatRequest>,
) -> Response {
    if payload.message.trim().is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "BAD_REQUEST", "message is required");
    }

    match state.db.get_ai_conversation(&session_id).await {
        Ok(Some(_)) => {}
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "NOT_FOUND", "Session not found"),
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                &format!("Failed to query session: {}", e),
            );
        }
    }

    let req = ChatRequest {
        message: payload.message,
        session_id: Some(session_id),
        request_id: payload.request_id,
        since_message_id: payload.since_message_id,
        service_name: payload.service_name,
        system_prompt: payload.system_prompt,
        mode: payload.mode,
        tool_config: payload.tool_config,
        audit_config: payload.audit_config,
        max_iterations: payload.max_iterations,
        timeout_secs: payload.timeout_secs,
        enable_tenth_man_rule: payload.enable_tenth_man_rule,
    };
    chat(State(state), Json(req)).await
}

async fn chat_stream(
    State(state): State<GatewayAppState>,
    headers: HeaderMap,
    Json(payload): Json<ChatRequest>,
) -> Response {
    let request_id = payload
        .request_id
        .clone()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    if payload.message.trim().is_empty() {
        return error_response_with_request_id(
            StatusCode::BAD_REQUEST,
            "BAD_REQUEST",
            "message is required",
            &request_id,
        );
    }

    let session_id = payload
        .session_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let mode = match parse_mode(payload.mode.as_deref()) {
        Ok(v) => v,
        Err(e) => return error_response_with_request_id(StatusCode::BAD_REQUEST, "BAD_REQUEST", &e, &request_id),
    };

    if let Err(e) = ensure_conversation_exists(&state, &session_id, payload.service_name.as_deref()).await {
        return error_response_with_request_id(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", &e, &request_id);
    }

    if let Err(e) = try_register_active_execution(&state, &session_id, &request_id) {
        return error_response_with_request_id(StatusCode::CONFLICT, "CONFLICT", &e, &request_id);
    }

    let history = match load_history_from_db(&state, &session_id).await {
        Ok(h) => h,
        Err(e) => {
            release_active_execution(&state, &session_id, &request_id);
            return error_response_with_request_id(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", &e, &request_id);
        }
    };

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let state_for_task = state.clone();
    let user_message = payload.message.clone();
    let service_name = payload.service_name.clone();
    let system_prompt = payload.system_prompt.clone();
    let tool_config = payload.tool_config.clone();
    let audit_config = payload.audit_config.clone();
    let max_iterations = payload.max_iterations;
    let timeout_secs = payload.timeout_secs;
    let enable_tenth_man_rule = payload.enable_tenth_man_rule;
    let since_message_id = resolve_stream_cursor(payload.since_message_id.clone(), &headers);
    let session_id_for_task = session_id.clone();
    let mode_for_task = mode;
    let request_id_for_task = request_id.clone();

    let initial_seen_ids: HashSet<String> = match state.db.get_ai_messages_by_conversation(&session_id).await {
        Ok(messages) => {
            if let Some(since_id) = since_message_id.as_deref() {
                match messages.iter().position(|m| m.id == since_id) {
                    Some(idx) => messages[..=idx].iter().map(|m| m.id.clone()).collect(),
                    None => HashSet::new(),
                }
            } else {
                messages.iter().map(|m| m.id.clone()).collect()
            }
        }
        Err(_) => HashSet::new(),
    };

    if mode_for_task != GatewayMode::Agent {
        if let Err(e) = persist_message(&state, &session_id, "user", payload.message.clone()).await {
            release_active_execution(&state, &session_id, &request_id);
            return error_response_with_request_id(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", &e, &request_id);
        }
    }

    tokio::spawn(async move {
        let _ = tx.send(
            json!({
                "type":"task_status",
                "status":"running",
                "session_id": session_id_for_task,
                "request_id": request_id_for_task,
                "mode": if mode_for_task == GatewayMode::Agent { "agent" } else { "llm" },
            })
            .to_string(),
        );
        if mode_for_task == GatewayMode::Agent {
            let mut seen_ids: HashSet<String> = initial_seen_ids;
            let mut seen_permission_ids: HashSet<String> = HashSet::new();
            let finished = Arc::new(AtomicBool::new(false));
            let poll_state = state_for_task.clone();
            let poll_sid = session_id_for_task.clone();
            let tx_poll = tx.clone();
            let finished_for_poll = finished.clone();
            let request_id_for_poll = request_id_for_task.clone();

            let poller = tokio::spawn(async move {
                while !finished_for_poll.load(Ordering::Relaxed) {
                    if let Ok(messages) = poll_state.db.get_ai_messages_by_conversation(&poll_sid).await {
                        for m in messages {
                            if !seen_ids.insert(m.id.clone()) {
                                continue;
                            }
                            if m.role == "assistant" {
                                if tx_poll.send(json!({"type":"assistant_delta","role":"assistant","content": m.content, "message_id": m.id, "request_id": request_id_for_poll}).to_string()).is_err() {
                                    let _ = crate::managers::cancellation_manager::cancel_execution(&poll_sid).await;
                                    return;
                                }
                            } else if m.role == "tool" {
                                let payload = m.metadata.as_ref()
                                    .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                                    .unwrap_or(json!({"raw": m.content}));
                                if tx_poll.send(json!({"type":"tool_event","data": payload, "message_id": m.id, "request_id": request_id_for_poll}).to_string()).is_err() {
                                    let _ = crate::managers::cancellation_manager::cancel_execution(&poll_sid).await;
                                    return;
                                }
                            }
                        }
                    }
                    if let Ok(pending_perms) = crate::commands::tool_commands::get_pending_shell_permissions().await {
                        for req in pending_perms {
                            if req.execution_id.as_deref() != Some(&poll_sid) {
                                continue;
                            }
                            if !seen_permission_ids.insert(req.id.clone()) {
                                continue;
                            }
                            if tx_poll.send(json!({
                                "type":"permission_required",
                                "permission": req,
                                "request_id": request_id_for_poll,
                            }).to_string()).is_err() {
                                let _ = crate::managers::cancellation_manager::cancel_execution(&poll_sid).await;
                                return;
                            }
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(250)).await;
                }
            });

            let completion = run_agent_execution(
                &state_for_task,
                &session_id_for_task,
                &user_message,
                system_prompt.as_deref(),
                tool_config.clone(),
                audit_config.clone(),
                max_iterations,
                timeout_secs,
                enable_tenth_man_rule,
            )
            .await;
            finished.store(true, Ordering::Relaxed);
            poller.abort();

            match completion {
                Ok(final_text) => {
                    let _ = tx.send(
                        json!({
                            "type":"done",
                            "status":"completed",
                            "session_id": session_id_for_task,
                            "message": final_text,
                            "service_name": service_name,
                            "user_message": user_message,
                            "request_id": request_id_for_task,
                        })
                        .to_string(),
                    );
                }
                Err(e) => {
                    let _ = tx.send(
                        json!({
                            "type":"error",
                            "code":"INTERNAL_ERROR",
                            "message": e,
                            "request_id": request_id_for_task,
                        })
                        .to_string(),
                    );
                    let _ = tx.send(
                        json!({
                            "type":"done",
                            "status":"failed",
                            "session_id": session_id_for_task,
                            "request_id": request_id_for_task,
                        })
                        .to_string(),
                    );
                }
            }
        } else {
            let service = match resolve_service(&state_for_task, service_name.as_deref()) {
                Ok(s) => s,
                Err(e) => {
                    let _ = tx.send(json!({"type":"error","code":"INTERNAL_ERROR","message": e}).to_string());
                    release_active_execution(&state_for_task, &session_id_for_task, &request_id_for_task);
                    return;
                }
            };
            let llm_config = apply_generation_settings_from_db(state_for_task.db.as_ref(), service.service.to_llm_config()).await;
            let streaming_client = sentinel_llm::StreamingLlmClient::new(llm_config);
            let completion = streaming_client
                .stream_chat(
                    system_prompt.as_deref(),
                    &user_message,
                    &history,
                    None,
                    |chunk| {
                        let evt = match chunk {
                            sentinel_llm::StreamContent::Text(text) => json!({"type":"assistant_delta","role":"assistant","content": text, "request_id": request_id_for_task}),
                            sentinel_llm::StreamContent::Reasoning(text) => json!({"type":"reasoning","content": text}),
                            sentinel_llm::StreamContent::ToolCallStart { id, name } => json!({"type":"tool_call_start","id":id,"name":name, "request_id": request_id_for_task}),
                            sentinel_llm::StreamContent::ToolCallDelta { id, delta } => json!({"type":"tool_call_delta","id":id,"delta":delta, "request_id": request_id_for_task}),
                            sentinel_llm::StreamContent::ToolCallComplete { id, name, arguments } => json!({"type":"tool_call_complete","id":id,"name":name,"arguments":arguments, "request_id": request_id_for_task}),
                            sentinel_llm::StreamContent::ToolResult { id, result } => json!({"type":"tool_result","id":id,"result":result, "request_id": request_id_for_task}),
                            sentinel_llm::StreamContent::Usage { input_tokens, output_tokens } => json!({"type":"usage","input_tokens":input_tokens,"output_tokens":output_tokens, "request_id": request_id_for_task}),
                            sentinel_llm::StreamContent::Done => json!({"type":"task_status","status":"stream_done","request_id": request_id_for_task}),
                        };
                        tx.send(evt.to_string()).is_ok()
                    },
                )
                .await;

            match completion {
                Ok(final_text) => {
                    let _ = persist_message(&state_for_task, &session_id_for_task, "assistant", final_text.clone()).await;
                    let _ = tx.send(
                        json!({
                            "type":"done",
                            "status":"completed",
                            "session_id": session_id_for_task,
                            "message": final_text,
                            "service_name": service_name,
                            "request_id": request_id_for_task,
                        })
                        .to_string(),
                    );
                }
                Err(e) => {
                    let _ = tx.send(
                        json!({
                            "type":"error",
                            "code":"INTERNAL_ERROR",
                            "message": format!("LLM stream error: {}", e),
                            "request_id": request_id_for_task,
                        })
                        .to_string(),
                    );
                    let _ = tx.send(
                        json!({
                            "type":"done",
                            "status":"failed",
                            "session_id": session_id_for_task,
                            "request_id": request_id_for_task,
                        })
                        .to_string(),
                    );
                }
            }
        }
        release_active_execution(&state_for_task, &session_id_for_task, &request_id_for_task);
    });

    let stream = UnboundedReceiverStream::new(rx).map(|msg| {
        let mut event = Event::default().event("message");
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&msg) {
            if let Some(id) = v.get("message_id").and_then(|x| x.as_str()).filter(|s| !s.is_empty()) {
                event = event.id(id);
            }
        }
        Ok::<Event, Infallible>(event.data(msg))
    });

    Sse::new(stream)
        .keep_alive(KeepAlive::default().interval(Duration::from_secs(10)).text("keepalive"))
        .into_response()
}

pub async fn start_gateway_server(
    config: &HttpGatewayConfig,
    ai_manager: Arc<AiServiceManager>,
    db: Arc<DatabaseService>,
    app_handle: tauri::AppHandle,
) -> Result<HttpGatewayRuntime, String> {
    let mut normalized_config = config.clone();
    normalize_gateway_config(&mut normalized_config);
    validate_gateway_config(&normalized_config)?;

    let normalized_host = if normalized_config.host.eq_ignore_ascii_case("localhost") {
        "127.0.0.1".to_string()
    } else {
        normalized_config.host.clone()
    };

    let addr: SocketAddr = format!("{}:{}", normalized_host, normalized_config.port)
        .parse()
        .map_err(|e| format!("Invalid gateway address: {}", e))?;

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| format!("Failed to bind HTTP gateway: {}", e))?;

    let local_addr = listener
        .local_addr()
        .map_err(|e| format!("Failed to read local address: {}", e))?;

    let header_name = HeaderName::from_str(&normalized_config.auth.header_name)
        .map_err(|e| format!("Invalid auth header name: {}", e))?;

    let state = GatewayAppState {
        auth_required: normalized_config.auth.required,
        header_name,
        api_key_hashes: Arc::new(normalized_config.auth.api_keys.iter().cloned().collect()),
        concurrent_limiter: Arc::new(Semaphore::new(
            normalized_config.limits.max_concurrent_requests.max(1),
        )),
        audit_enabled: normalized_config.audit.enabled,
        log_auth_failures: normalized_config.audit.log_auth_failures,
        ai_manager,
        db,
        app_handle,
        active_executions: Arc::new(StdMutex::new(HashMap::new())),
    };

    let protected_api = Router::new()
        .route("/status", get(api_status))
        .route("/chat", post(chat))
        .route("/chat/stream", post(chat_stream))
        .route("/bridge/invoke", post(bridge_invoke))
        .route("/permissions/pending", get(get_pending_permissions))
        .route("/permissions/respond", post(respond_permission))
        .route("/session", post(create_session))
        .route("/session/{session_id}/messages", get(get_session_messages))
        .route("/session/{session_id}/chat", post(session_chat))
        .route("/session/{session_id}", delete(delete_session))
        .route_layer(from_fn_with_state(state.clone(), auth_and_rate_limit_middleware));

    let app = Router::new()
        .route("/", get(web_entry))
        .route("/{*path}", get(web_assets))
        .route("/health", get(health))
        .nest("/api", protected_api)
        .layer(DefaultBodyLimit::max(normalized_config.limits.max_body_bytes.max(1024)))
        .with_state(state);

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let task = tokio::spawn(async move {
        info!("HTTP gateway listening on {}", local_addr);
        let result = axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        })
        .await;

        if let Err(e) = result {
            error!("HTTP gateway server error: {}", e);
        }

        info!("HTTP gateway stopped");
    });

    Ok(HttpGatewayRuntime {
        bind_addr: local_addr,
        started_at: Utc::now(),
        shutdown_tx: Some(shutdown_tx),
        task,
    })
}
