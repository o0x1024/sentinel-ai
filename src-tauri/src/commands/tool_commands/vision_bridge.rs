//! Vision Explorer bridge commands
//!
//! These commands keep legacy frontend invoke names working, while routing to the V2 engine.

use std::collections::HashMap;
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager};

use crate::commands::vision_explorer_v2::VisionExplorerV2State;
use crate::engines::vision_explorer_v2::emitter::V2MessageEmitter;
use crate::engines::vision_explorer_v2::Event;

/// Receive login credentials from frontend for a running VisionExplorer V2.
///
/// Legacy command name used by frontend as V2 fallback.
pub async fn vision_explorer_receive_credentials(
    app: AppHandle,
    execution_id: String,
    username: String,
    password: String,
    verification_code: Option<String>,
    _extra_fields: Option<HashMap<String, String>>,
) -> Result<(), String> {
    tracing::info!(
        "V2(bridge): Received credentials for VisionExplorer execution_id: {}",
        execution_id
    );

    let state = app.state::<VisionExplorerV2State>();

    let (event_tx, session_id) = {
        let mut sessions = state.sessions.write().await;
        let session = sessions
            .get_mut(&execution_id)
            .ok_or_else(|| format!("Session not found: {}", execution_id))?;

        session.has_credentials = true;
        session.last_username = Some(username.clone());

        (session.event_tx.clone(), session.session_id.clone())
    };

    event_tx
        .send(Event::CredentialsReceived {
            username: username.clone(),
            password,
            verification_code,
        })
        .await
        .map_err(|e| format!("Failed to send credentials event: {}", e))?;

    let emitter = V2MessageEmitter::new(Arc::new(app), execution_id, session_id);
    emitter.emit_credentials_received(&username);

    Ok(())
}

/// Send a user message to a running VisionExplorer V2 (for mid-run guidance).
pub async fn vision_explorer_send_user_message(
    app: AppHandle,
    execution_id: String,
    message: String,
) -> Result<(), String> {
    tracing::info!(
        "V2(bridge): Received user message for VisionExplorer execution_id: {} ({} chars)",
        execution_id,
        message.len()
    );

    let state = app.state::<VisionExplorerV2State>();
    let event_tx = {
        let sessions = state.sessions.read().await;
        sessions
            .get(&execution_id)
            .map(|session| session.event_tx.clone())
    }
    .ok_or_else(|| format!("Session not found: {}", execution_id))?;

    event_tx
        .send(Event::Log {
            level: "info".to_string(),
            message: format!("User message: {}", message),
        })
        .await
        .map_err(|e| format!("Failed to send user message event: {}", e))?;

    Ok(())
}

/// Skip login for a running VisionExplorer V2 (user chose not to provide credentials).
pub async fn vision_explorer_skip_login(app: AppHandle, execution_id: String) -> Result<(), String> {
    tracing::info!(
        "V2(bridge): Skip login requested for VisionExplorer execution_id: {}",
        execution_id
    );

    let state = app.state::<VisionExplorerV2State>();
    let (event_tx, session_id) = {
        let sessions = state.sessions.read().await;
        let session = sessions
            .get(&execution_id)
            .ok_or_else(|| format!("Session not found: {}", execution_id))?;
        (session.event_tx.clone(), session.session_id.clone())
    };

    event_tx
        .send(Event::SkipLogin)
        .await
        .map_err(|e| format!("Failed to send skip login event: {}", e))?;

    let exec_id = execution_id.clone();

    // Close takeover UI immediately (keep legacy behavior)
    let payload = serde_json::json!({
        "execution_id": exec_id,
        "skipped": true
    });
    let _ = app.emit("vision:credentials_received", &payload);

    let emitter = V2MessageEmitter::new(
        Arc::new(app),
        execution_id,
        session_id,
    );
    emitter.emit_v2_event("credentials_received", &payload);

    Ok(())
}

/// Mark manual login as complete for a running VisionExplorer V2.
pub async fn vision_explorer_manual_login_complete(
    app: AppHandle,
    execution_id: String,
) -> Result<(), String> {
    tracing::info!(
        "V2(bridge): Manual login complete signaled for VisionExplorer execution_id: {}",
        execution_id
    );

    let state = app.state::<VisionExplorerV2State>();
    let event_tx = {
        let sessions = state.sessions.read().await;
        sessions
            .get(&execution_id)
            .map(|session| session.event_tx.clone())
    }
    .ok_or_else(|| format!("Session not found: {}", execution_id))?;

    event_tx
        .send(Event::ManualLoginComplete)
        .await
        .map_err(|e| format!("Failed to send manual login complete event: {}", e))?;

    Ok(())
}


