//! Shell tool permission commands

use crate::agents::executor::{
    append_execution_tool_trace, next_execution_tool_trace_sequence, ToolCallRecord,
};
use std::collections::HashMap;
use std::sync::Arc;

use once_cell::sync::Lazy;
use serde::Serialize;
use serde_json::json;
use tauri::Manager;
use tokio::sync::RwLock;

use sentinel_tools::buildin_tools::shell::{
    describe_shell_command_for_review, get_shell_config, set_permission_handler, set_shell_config,
    ShellConfig, ShellPermissionHandler,
};
use sentinel_tools::buildin_tools::shell_policy::{
    suggest_allow_rule_details, suggest_allow_rules,
};

// Global storage for permission response channels
static SHELL_PERMISSION_SENDERS: Lazy<RwLock<HashMap<String, tokio::sync::oneshot::Sender<bool>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

// Global storage for pending permission requests (for frontend polling)
#[derive(Debug, Clone, Serialize)]
pub struct PendingPermissionRequest {
    pub id: String,
    pub command: String,
    pub execution_id: Option<String>,
    pub timestamp: u64,
    pub semantic_kind: String,
    pub semantic_code: String,
    pub semantic_summary_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_reason_key: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suggested_allow_rules: Vec<SuggestedAllowRulePreview>,
}

static PENDING_PERMISSION_REQUESTS: Lazy<RwLock<Vec<PendingPermissionRequest>>> =
    Lazy::new(|| RwLock::new(Vec::new()));

#[derive(Debug, Clone, Serialize)]
pub struct PersistedShellAllowRules {
    pub added_rules: Vec<String>,
    pub all_rules: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SuggestedAllowRulePreview {
    pub rule: String,
    pub reason_key: String,
}

struct ShellPermissionImpl {
    app: tauri::AppHandle,
}

#[derive(Debug, Clone, Copy)]
enum ShellPermissionDecisionKind {
    Allow,
    Deny,
    AllowForever,
}

impl ShellPermissionDecisionKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
            Self::AllowForever => "allow_forever",
        }
    }
}

#[async_trait::async_trait]
impl ShellPermissionHandler for ShellPermissionImpl {
    async fn check_permission(&self, command: &str, execution_id: Option<&str>) -> bool {
        let id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = tokio::sync::oneshot::channel();
        let review_info = describe_shell_command_for_review(command);
        let suggested_allow_rules = suggest_allow_rule_details(command)
            .into_iter()
            .map(|item| SuggestedAllowRulePreview {
                rule: item.rule,
                reason_key: item.reason_key,
            })
            .collect::<Vec<_>>();

        {
            let mut senders = SHELL_PERMISSION_SENDERS.write().await;
            senders.insert(id.clone(), tx);
        }

        // Store pending request
        {
            let mut pending = PENDING_PERMISSION_REQUESTS.write().await;
            pending.push(PendingPermissionRequest {
                id: id.clone(),
                command: command.to_string(),
                execution_id: execution_id.map(|v| v.to_string()),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                semantic_kind: review_info.semantic_kind.clone(),
                semantic_code: review_info.semantic_code.clone(),
                semantic_summary_key: review_info.semantic_summary_key.clone(),
                semantic_reason_key: review_info.semantic_reason_key.clone(),
                suggested_allow_rules: suggested_allow_rules.clone(),
            });
        }

        // Emit event to frontend
        use tauri::Emitter;
        tracing::info!(
            "Requesting permission for command: {} (id: {}, execution_id: {:?})",
            command,
            id,
            execution_id
        );
        if let Err(e) = self.app.emit(
            "shell-permission-request",
            serde_json::json!({
                "id": id,
                "command": command,
                "execution_id": execution_id,
                "semantic_kind": review_info.semantic_kind,
                "semantic_code": review_info.semantic_code,
                "semantic_summary_key": review_info.semantic_summary_key,
                "semantic_reason_key": review_info.semantic_reason_key,
                "suggested_allow_rules": suggested_allow_rules,
            }),
        ) {
            tracing::error!("Failed to emit permission request: {}", e);
            // Clean up pending request
            let mut pending = PENDING_PERMISSION_REQUESTS.write().await;
            pending.retain(|r| r.id != id);
            return false;
        }

        // Wait for response with timeout (e.g. 5 minutes)
        let result = match tokio::time::timeout(std::time::Duration::from_secs(300), rx).await {
            Ok(Ok(allowed)) => {
                tracing::info!("Permission response for {}: {}", id, allowed);
                allowed
            }
            Ok(Err(_)) => {
                tracing::warn!("Permission channel dropped for {}", id);
                false
            }
            Err(_) => {
                tracing::warn!("Permission request timed out for {}", id);
                false
            }
        };

        // Clean up pending request
        {
            let mut pending = PENDING_PERMISSION_REQUESTS.write().await;
            pending.retain(|r| r.id != id);
        }

        result
    }
}

/// Initialize the shell permission handler and load config from database
pub async fn init_shell_permission_handler(app: tauri::AppHandle) -> Result<(), String> {
    set_permission_handler(Arc::new(ShellPermissionImpl { app: app.clone() })).await;

    // Load shell config from database
    if let Some(db) = app.try_state::<Arc<sentinel_db::DatabaseService>>() {
        let db_ref: &sentinel_db::DatabaseService = db.inner();
        let config =
            crate::commands::tool_commands::agent_config::load_shell_config_from_db(db_ref).await;
        set_shell_config(config).await;
        tracing::info!("Shell config loaded from database on startup");
    } else {
        tracing::warn!("Database service not available, using default shell config");
    }

    Ok(())
}

/// Get shell tool configuration
pub async fn get_shell_tool_config() -> Result<ShellConfig, String> {
    Ok(get_shell_config().await)
}

/// Set shell tool configuration (deprecated, use save_agent_config instead)
pub async fn set_shell_tool_config(config: ShellConfig) -> Result<(), String> {
    set_shell_config(config).await;
    Ok(())
}

/// Respond to a shell permission request
pub async fn respond_shell_permission(id: String, allowed: bool) -> Result<(), String> {
    finalize_shell_permission_response(
        id,
        allowed,
        if allowed {
            ShellPermissionDecisionKind::Allow
        } else {
            ShellPermissionDecisionKind::Deny
        },
        &[],
    )
    .await
}

pub async fn allow_shell_permission_forever(
    id: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<PersistedShellAllowRules, String> {
    allow_shell_permission_forever_with_db(id, db_service.inner()).await
}

pub(crate) async fn allow_shell_permission_forever_with_db(
    id: String,
    db: &sentinel_db::DatabaseService,
) -> Result<PersistedShellAllowRules, String> {
    let pending_request = {
        let pending = PENDING_PERMISSION_REQUESTS.read().await;
        pending
            .iter()
            .find(|request| request.id == id)
            .cloned()
            .ok_or_else(|| format!("Request ID {} not found or already handled", id))?
    };

    let all_rules = suggest_allow_rules(&pending_request.command);
    if all_rules.is_empty() {
        return Err("High-risk shell commands cannot be permanently approved".to_string());
    }

    let mut shell_config =
        crate::commands::tool_commands::agent_config::load_shell_config_from_db(db).await;
    let mut added_rules = Vec::new();

    for rule in &all_rules {
        if !shell_config.allowed_commands.contains(rule) {
            shell_config.allowed_commands.push(rule.clone());
            added_rules.push(rule.clone());
        }
    }

    crate::commands::tool_commands::agent_config::save_shell_config_to_db(&shell_config, db)
        .await?;
    set_shell_config(shell_config).await;

    finalize_shell_permission_response(
        id,
        true,
        ShellPermissionDecisionKind::AllowForever,
        &all_rules,
    )
    .await?;

    Ok(PersistedShellAllowRules {
        added_rules,
        all_rules,
    })
}

async fn finalize_shell_permission_response(
    id: String,
    allowed: bool,
    decision_kind: ShellPermissionDecisionKind,
    persisted_allow_rules: &[String],
) -> Result<(), String> {
    tracing::info!(
        "Responding to shell permission: id={}, allowed={}, decision={}",
        id,
        allowed,
        decision_kind.as_str()
    );

    let pending_request = {
        let mut pending = PENDING_PERMISSION_REQUESTS.write().await;
        let before_len = pending.len();
        let removed = pending
            .iter()
            .position(|request| request.id == id)
            .map(|index| pending.remove(index));
        tracing::info!(
            "Removed from pending: before={}, after={}",
            before_len,
            pending.len()
        );
        removed
    };

    let mut senders = SHELL_PERMISSION_SENDERS.write().await;
    tracing::info!(
        "Available senders: {:?}",
        senders.keys().collect::<Vec<_>>()
    );

    if let Some(tx) = senders.remove(&id) {
        let send_result = tx.send(allowed);
        tracing::info!("Sent permission response: {:?}", send_result);
        if let Some(request) = pending_request.as_ref() {
            record_shell_permission_history(request, allowed, decision_kind, persisted_allow_rules);
        }
        Ok(())
    } else {
        tracing::warn!("Request ID {} not found in senders map", id);
        Err(format!("Request ID {} not found or already handled", id))
    }
}

fn record_shell_permission_history(
    request: &PendingPermissionRequest,
    allowed: bool,
    decision_kind: ShellPermissionDecisionKind,
    persisted_allow_rules: &[String],
) {
    let audit_payload = json!({
        "permission_request_id": request.id,
        "command": request.command,
        "execution_id": request.execution_id,
        "decision": decision_kind.as_str(),
        "allowed": allowed,
        "semantic_kind": request.semantic_kind,
        "semantic_code": request.semantic_code,
        "semantic_summary_key": request.semantic_summary_key,
        "semantic_reason_key": request.semantic_reason_key,
        "suggested_allow_rules": request.suggested_allow_rules,
        "persisted_allow_rules": persisted_allow_rules,
        "timestamp": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    });

    let session_id = request
        .execution_id
        .clone()
        .unwrap_or_else(|| format!("shell-permission::{}", request.id));

    sentinel_llm::log::write_tool_log(
        &session_id,
        request.execution_id.as_deref(),
        "sentinel-shell",
        "permission-review",
        "SHELL PERMISSION",
        &serde_json::to_string_pretty(&audit_payload).unwrap_or_else(|_| audit_payload.to_string()),
    );
    sentinel_llm::log::log_structured_tool_event(
        &session_id,
        request.execution_id.as_deref(),
        "sentinel-shell",
        "permission-review",
        "shell_permission",
        &audit_payload,
    );

    if let Some(execution_id) = request.execution_id.as_deref() {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let sequence = next_execution_tool_trace_sequence(execution_id);
        let arguments = json!({
            "command": request.command,
            "semantic_kind": request.semantic_kind,
            "semantic_code": request.semantic_code,
            "semantic_summary_key": request.semantic_summary_key,
            "semantic_reason_key": request.semantic_reason_key,
            "suggested_allow_rules": request.suggested_allow_rules,
        });
        let result = json!({
            "decision": decision_kind.as_str(),
            "allowed": allowed,
            "persisted_allow_rules": persisted_allow_rules,
        });

        append_execution_tool_trace(
            execution_id,
            ToolCallRecord {
                id: format!("shell-permission-{}", request.id),
                name: "shell_permission_decision".to_string(),
                arguments: arguments.to_string(),
                result: Some(result.to_string()),
                success: allowed,
                sequence,
                started_at_ms: now_ms,
                completed_at_ms: now_ms,
                duration_ms: 0,
            },
        );
    }
}

/// Get all pending shell permission requests (for frontend polling)
pub async fn get_pending_shell_permissions() -> Result<Vec<PendingPermissionRequest>, String> {
    let pending = PENDING_PERMISSION_REQUESTS.read().await;
    Ok(pending.clone())
}
