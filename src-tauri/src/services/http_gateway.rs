use crate::agents::tool_router::{clear_tool_usage_records, get_tool_usage_statistics};
use crate::agents::AgentExecuteParams;
use crate::agents::ToolConfig;
use crate::commands::tool_commands::PendingPermissionRequest;
use crate::models::database::{AiConversation, AiMessage};
use crate::services::ai::AiServiceManager;
use crate::services::database::DatabaseService;
use crate::skills::{parse_skill_markdown, scan_and_upsert_skills, skills_root};
use crate::utils::ai_generation_settings::apply_generation_settings_from_db;
use axum::body::Body;
use axum::extract::{ConnectInfo, DefaultBodyLimit, Path, Query, Request, State};
use axum::http::header::{HeaderValue, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderName, StatusCode};
use axum::middleware::{from_fn_with_state, Next};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use chrono::Utc;
use sentinel_db::Database;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::{Component, Path as StdPath, PathBuf};
use std::str::FromStr;
use std::sync::Mutex as StdMutex;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;
use tokio::fs;
use tokio::net::TcpListener;
use tokio::sync::{oneshot, Semaphore};
use tokio::task::JoinHandle;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_stream::StreamExt;
use tracing::{error, info, warn};
use uuid::Uuid;

mod http_gateway_bridge;
use http_gateway_bridge::bridge_invoke;
mod http_gateway_chat;
use http_gateway_chat::{
    chat, chat_stream, get_pending_permissions, respond_permission, session_chat,
};

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
    max_iterations: Option<usize>,
    #[serde(default)]
    timeout_secs: Option<u64>,
    #[serde(default)]
    enable_tenth_man_rule: Option<bool>,
    #[serde(default)]
    current_browser_shell_direct_write_enabled: Option<bool>,
    #[serde(default)]
    current_browser_shell_session_id: Option<String>,
    #[serde(default)]
    current_terminal_session_fingerprint: Option<String>,
    #[serde(default)]
    current_terminal_session_id: Option<String>,
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
    max_iterations: Option<usize>,
    #[serde(default)]
    timeout_secs: Option<u64>,
    #[serde(default)]
    enable_tenth_man_rule: Option<bool>,
    #[serde(default)]
    current_browser_shell_direct_write_enabled: Option<bool>,
    #[serde(default)]
    current_browser_shell_session_id: Option<String>,
    #[serde(default)]
    current_terminal_session_fingerprint: Option<String>,
    #[serde(default)]
    current_terminal_session_id: Option<String>,
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
            if (evt.type === "ask_user_question_required" && evt.question) __gwDispatch("ask-user-question-request", evt.question);
            if (evt.type === "shell_background_task" && evt.task) __gwDispatch("shell-background-task-update", evt.task);
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
    let candidate = if normalized.is_empty() {
        "index.html"
    } else {
        normalized
    };
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
        max_iterations: p?.config?.max_iterations,
        timeout_secs: p?.config?.timeout_secs,
        enable_tenth_man_rule: p?.config?.enable_tenth_man_rule,
        current_terminal_session_fingerprint: p?.config?.current_terminal_session_fingerprint,
        current_terminal_session_id: p?.config?.current_terminal_session_id,
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
      __gwDispatch("agent:execution_finished", {
        execution_id: execId,
        outcome: "failed",
        success: false,
        error: err,
        response: null,
        message: null,
      });
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
          __gwDispatch("agent:execution_finished", {
            execution_id: execId,
            outcome: "failed",
            success: false,
            error: err,
            response: null,
            message: null,
          });
          __gwDispatch("agent:error", { execution_id: execId, error: err });
          throw new Error(err);
        } else if (evt.type === "done") {
          if (typeof evt.message === "string" && evt.message) finalText = evt.message;
          __gwDispatch("agent:execution_finished", {
            execution_id: execId,
            outcome: evt.status === "failed" ? "failed" : "succeeded",
            success: evt.status !== "failed",
            error: evt.status === "failed" ? (evt.error || "agent execution failed") : null,
            response: finalText,
            message: null,
          });
          __gwDispatch("agent:complete", {
            execution_id: execId,
            success: evt.status !== "failed",
            response: finalText,
          });
        } else if (evt.type === "permission_required" && evt.permission) {
          __gwDispatch("shell-permission-request", evt.permission);
        } else if (evt.type === "ask_user_question_required" && evt.question) {
          __gwDispatch("ask-user-question-request", evt.question);
        } else if (evt.type === "shell_background_task" && evt.task) {
          __gwDispatch("shell-background-task-update", evt.task);
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
    let first = chars
        .next()
        .ok_or_else(|| "Skill name is required".to_string())?;
    if !(first.is_ascii_lowercase() || first.is_ascii_digit()) {
        return Err("Skill name must start with a lowercase letter or digit".to_string());
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
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
    let parent = target
        .parent()
        .ok_or_else(|| "Invalid file path".to_string())?;
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
        return Err(
            "When LAN access is disabled, host must be loopback (127.0.0.1/::1/localhost)"
                .to_string(),
        );
    }

    if config.allow_lan && (!config.auth.required || config.auth.api_keys.is_empty()) {
        return Err("LAN mode requires auth.required=true and at least one API key".to_string());
    }

    if config.remote.enabled
        && (!config.auth.required
            || config.auth.api_keys.is_empty()
            || config.remote.public_base_url.trim().is_empty())
    {
        return Err(
            "Remote mode requires auth, at least one API key, and public_base_url".to_string(),
        );
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

fn resolve_stream_cursor(
    payload_since_message_id: Option<String>,
    headers: &HeaderMap,
) -> Option<String> {
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
            );
        }
    };

    let response = next.run(req).await;
    drop(permit);
    response
}

fn resolve_service(
    state: &GatewayAppState,
    service_name: Option<&str>,
) -> Result<crate::services::ai::AiServiceWrapper, String> {
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
        Ok(Some(config_str)) => ToolConfig::from_json_str(&config_str).ok(),
        _ => None,
    }
}

async fn run_agent_execution(
    state: &GatewayAppState,
    session_id: &str,
    task: &str,
    system_prompt: Option<&str>,
    tool_config_override: Option<ToolConfig>,
    max_iterations: Option<usize>,
    timeout_secs: Option<u64>,
    enable_tenth_man_rule: Option<bool>,
    active_browser_shell_direct_write_enabled: Option<bool>,
    active_browser_shell_session_id: Option<&str>,
    active_terminal_session_id: Option<&str>,
    active_terminal_session_fingerprint: Option<&str>,
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

    let tool_config = if tool_config_override.is_some() {
        tool_config_override
    } else {
        load_tool_config_from_db(state).await
    };
    let params = AgentExecuteParams {
        execution_id: session_id.to_string(),
        conversation_id: None,
        cancellation_generation: None,
        model: model_name,
        system_prompt: system_prompt.unwrap_or_default().to_string(),
        task: task.to_string(),
        active_browser_shell_direct_write_enabled: active_browser_shell_direct_write_enabled
            .unwrap_or(false),
        active_browser_shell_session_id: active_browser_shell_session_id.map(|v| v.to_string()),
        active_terminal_session_fingerprint: active_terminal_session_fingerprint
            .map(|v| v.to_string()),
        active_terminal_session_id: active_terminal_session_id.map(|v| v.to_string()),
        working_directory: None,
        provider_config_key: provider.to_string(),
        rig_provider,
        api_key: provider_config.api_key.clone(),
        api_base: provider_config.api_base.clone(),
        max_iterations: max_iterations
            .unwrap_or(provider_config.max_turns.unwrap_or(50))
            .max(1),
        timeout_secs: timeout_secs.unwrap_or(300),
        tool_config,
        enable_tenth_man_rule: enable_tenth_man_rule.unwrap_or(false),
        tenth_man_config: None,
        document_attachments: None,
        image_attachments: None,
        referenced_traffic: None,
        persist_messages: true,
        subagent_run_id: None,
        harness_run_id: None,
        context_policy: None,
        context_engine_mode: Some(crate::agents::ContextEngineMode::ClaudeLike),
        recursion_depth: 0,
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

    let llm_config =
        apply_generation_settings_from_db(state.db.as_ref(), service.service.to_llm_config()).await;
    let llm_client = sentinel_llm::LlmClient::new(llm_config);

    llm_client
        .chat(system_prompt, message, history, None)
        .await
        .map_err(|e| format!("LLM completion error: {}", e))
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

async fn delete_session(
    Path(session_id): Path<String>,
    State(state): State<GatewayAppState>,
) -> Response {
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
        .route_layer(from_fn_with_state(
            state.clone(),
            auth_and_rate_limit_middleware,
        ));

    let app = Router::new()
        .route("/", get(web_entry))
        .route("/{*path}", get(web_assets))
        .route("/health", get(health))
        .nest("/api", protected_api)
        .layer(DefaultBodyLimit::max(
            normalized_config.limits.max_body_bytes.max(1024),
        ))
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
