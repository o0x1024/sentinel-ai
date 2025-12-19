//! Tauri Commands for Vision Explorer V2
//!
//! Provides the command interface between the frontend and V2 Engine.

use crate::engines::vision_explorer_v2::emitter::V2MessageEmitter;
use crate::engines::vision_explorer_v2::{V2Engine, VisionExplorerV2Config};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use tokio::sync::{mpsc, RwLock};

/// State container for V2 Engine sessions
pub struct VisionExplorerV2State {
    /// Active V2 engine sessions (execution_id -> engine control)
    pub sessions: Arc<RwLock<HashMap<String, V2Session>>>,
}

/// A single V2 session
pub struct V2Session {
    /// Session ID (same as execution_id for V2)
    pub session_id: String,
    /// Event sender to control the engine
    pub event_tx: mpsc::Sender<crate::engines::vision_explorer_v2::Event>,
    /// Target URL
    pub target_url: String,
    /// Credentials (if set)
    pub credentials: Option<(String, String)>,
    /// Is running
    pub is_running: bool,
}

fn split_model_with_provider(value: &str) -> (Option<String>, String) {
    if let Some((provider, model)) = value.split_once('/') {
        if !provider.trim().is_empty() && !model.trim().is_empty() {
            return (Some(provider.to_string()), model.to_string());
        }
    }
    (None, value.to_string())
}

impl Default for VisionExplorerV2State {
    fn default() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

/// Start Vision Explorer V2
#[tauri::command]
pub async fn start_vision_explorer_v2(
    mut config: VisionExplorerV2Config,
    app_handle: AppHandle,
    state: State<'_, VisionExplorerV2State>,
    db: State<'_, Arc<crate::services::database::DatabaseService>>,
) -> Result<String, String> {
    log::info!("Starting Vision Explorer V2 with config: {:?}", config);

    // Read LLM/VLM settings from database
    let default_llm_provider = db
        .get_config("ai", "default_llm_provider")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "anthropic".to_string());

    let mut default_llm_model = db
        .get_config("ai", "default_llm_model")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "claude-3-haiku".to_string());

    if default_llm_model.trim().is_empty() {
        default_llm_model = "claude-3-haiku".to_string();
    }

    let mut default_vlm_provider = match db
        .get_config("ai", "default_vlm_provider")
        .await
        .ok()
        .flatten()
    {
        Some(value) => value,
        None => {
            let legacy_provider = db
                .get_config("ai", "default_vlm_provider")
                .await
                .ok()
                .flatten();
            if let Some(ref value) = legacy_provider {
                let _ = db
                    .set_config(
                        "ai",
                        "default_vlm_provider",
                        value,
                        Some("Default VLM provider (migrated)"),
                    )
                    .await;
            }
            legacy_provider.unwrap_or_else(|| default_llm_provider.clone())
        }
    };

    if default_vlm_provider.trim().is_empty() {
        default_vlm_provider = default_llm_provider.clone();
    }

    let mut default_vlm_model = db
        .get_config("ai", "default_vlm_model")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "claude-3-sonnet".to_string());

    if default_vlm_model.trim().is_empty() {
        default_vlm_model = "claude-3-sonnet".to_string();
    }

    let (fast_provider_override, fast_model_id) = split_model_with_provider(&default_llm_model);
    let (vision_provider_override, vision_model_id) = split_model_with_provider(&default_vlm_model);

    let fast_provider = fast_provider_override.unwrap_or_else(|| default_llm_provider.clone());
    let vision_provider = vision_provider_override.unwrap_or_else(|| default_vlm_provider.clone());

    // Get API keys from AiServiceManager
    let (fast_api_key, fast_base_url) = if let Some(ai_manager) =
        app_handle.try_state::<Arc<crate::services::ai::AiServiceManager>>()
    {
        if let Some(service) = ai_manager.get_service(&fast_provider) {
            let cfg = service.get_config();
            (cfg.api_key.clone(), cfg.api_base.clone())
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    let (vision_api_key, vision_base_url) = if vision_provider != fast_provider {
        if let Some(ai_manager) =
            app_handle.try_state::<Arc<crate::services::ai::AiServiceManager>>()
        {
            if let Some(service) = ai_manager.get_service(&vision_provider) {
                let cfg = service.get_config();
                (cfg.api_key.clone(), cfg.api_base.clone())
            } else {
                (None, None)
            }
        } else {
            (None, None)
        }
    } else {
        (fast_api_key.clone(), fast_base_url.clone())
    };

    // Update config with database settings
    config.ai_config = crate::engines::vision_explorer_v2::types::AIConfig {
        fast_model_id,
        vision_model_id,
        fast_provider,
        vision_provider,
        fast_api_key,
        vision_api_key,
        fast_base_url,
        vision_base_url,
    };

    log::info!(
        "Vision Explorer V2 AI Config: fast_provider={}, fast_model_id={}, vision_provider={}, vision_model_id={}",
        config.ai_config.fast_provider,
        config.ai_config.fast_model_id,
        config.ai_config.vision_provider,
        config.ai_config.vision_model_id
    );

    let execution_id = uuid::Uuid::new_v4().to_string();
    let target_url = config.target_url.clone();

    // Create engine
    let mut engine = V2Engine::new(config);
    let session_id = engine.session_id().to_string();
    let event_tx = engine.event_sender();

    // Create emitter for frontend communication
    let emitter = V2MessageEmitter::new(
        Arc::new(app_handle.clone()),
        execution_id.clone(),
        session_id.clone(),
    );

    engine = engine.with_emitter(emitter.clone());

    // Emit start event
    emitter.emit_start(&target_url);

    // Store session
    {
        let mut sessions = state.sessions.write().await;
        sessions.insert(
            execution_id.clone(),
            V2Session {
                session_id: session_id.clone(),
                event_tx: event_tx.clone(),
                target_url: target_url.clone(),
                credentials: None,
                is_running: true,
            },
        );
    }

    let exec_id_clone = execution_id.clone();
    let state_sessions = state.sessions.clone();

    // Spawn the engine loop
    tokio::spawn(async move {
        let start_time = std::time::Instant::now();

        match engine.start().await {
            Ok(_) => {
                log::info!("Vision Explorer V2 completed successfully");
                let stats = engine.get_stats().await;
                emitter.emit_complete(&stats, "completed", start_time.elapsed().as_millis() as u64);
            }
            Err(e) => {
                log::error!("Vision Explorer V2 Engine Error: {:?}", e);
                emitter.emit_error(0, &format!("Engine error: {}", e));
            }
        }

        // Mark session as not running
        let mut sessions = state_sessions.write().await;
        if let Some(session) = sessions.get_mut(&exec_id_clone) {
            session.is_running = false;
            session.credentials = None;
        }
    });

    Ok(execution_id)
}

/// Stop Vision Explorer V2
#[tauri::command]
pub async fn stop_vision_explorer_v2(
    execution_id: String,
    state: State<'_, VisionExplorerV2State>,
) -> Result<(), String> {
    log::info!("Stop Vision Explorer V2 requested for: {}", execution_id);

    let event_tx = {
        let sessions = state.sessions.read().await;
        sessions
            .get(&execution_id)
            .map(|session| session.event_tx.clone())
    };

    if let Some(event_tx) = event_tx {
        if let Err(e) = event_tx
            .send(crate::engines::vision_explorer_v2::Event::Stop)
            .await
        {
            log::warn!("Failed to send stop event: {}", e);
        }
        Ok(())
    } else {
        Err(format!("Session not found: {}", execution_id))
    }
}

/// Submit credentials for V2 login
#[tauri::command]
pub async fn vision_explorer_v2_receive_credentials(
    execution_id: String,
    username: String,
    password: String,
    verification_code: Option<String>,
    _extra_fields: Option<HashMap<String, String>>,
    app_handle: AppHandle,
    state: State<'_, VisionExplorerV2State>,
) -> Result<(), String> {
    log::info!("V2 Credentials received for: {}", execution_id);

    let (event_tx, session_id) = {
        let mut sessions = state.sessions.write().await;
        if let Some(session) = sessions.get_mut(&execution_id) {
            session.credentials = Some((username.clone(), password.clone()));
            (
                Some(session.event_tx.clone()),
                Some(session.session_id.clone()),
            )
        } else {
            (None, None)
        }
    };

    if let (Some(event_tx), Some(session_id)) = (event_tx, session_id) {
        if let Err(e) = event_tx
            .send(
                crate::engines::vision_explorer_v2::Event::CredentialsReceived {
                    username: username.clone(),
                    password: password.clone(),
                    verification_code,
                },
            )
            .await
        {
            log::warn!("Failed to send credentials event: {}", e);
        }

        let emitter = V2MessageEmitter::new(Arc::new(app_handle), execution_id.clone(), session_id);
        emitter.emit_credentials_received(&username);

        Ok(())
    } else {
        Err(format!("Session not found: {}", execution_id))
    }
}

/// Skip login for V2
#[tauri::command]
pub async fn vision_explorer_v2_skip_login(
    execution_id: String,
    state: State<'_, VisionExplorerV2State>,
) -> Result<(), String> {
    log::info!("V2 Skip login requested for: {}", execution_id);

    let event_tx = {
        let sessions = state.sessions.read().await;
        sessions
            .get(&execution_id)
            .map(|session| session.event_tx.clone())
    };

    if let Some(event_tx) = event_tx {
        if let Err(e) = event_tx
            .send(crate::engines::vision_explorer_v2::Event::SkipLogin)
            .await
        {
            log::warn!("Failed to send skip login event: {}", e);
        }
        Ok(())
    } else {
        Err(format!("Session not found: {}", execution_id))
    }
}

/// Get V2 session status
#[tauri::command]
pub async fn get_vision_explorer_v2_status(
    execution_id: String,
    state: State<'_, VisionExplorerV2State>,
) -> Result<serde_json::Value, String> {
    let sessions = state.sessions.read().await;
    if let Some(session) = sessions.get(&execution_id) {
        Ok(serde_json::json!({
            "execution_id": execution_id,
            "session_id": session.session_id,
            "target_url": session.target_url,
            "is_running": session.is_running,
            "has_credentials": session.credentials.is_some()
        }))
    } else {
        Err(format!("Session not found: {}", execution_id))
    }
}

/// List all V2 sessions
#[tauri::command]
pub async fn list_vision_explorer_v2_sessions(
    state: State<'_, VisionExplorerV2State>,
) -> Result<Vec<serde_json::Value>, String> {
    let sessions = state.sessions.read().await;
    let result: Vec<serde_json::Value> = sessions
        .iter()
        .map(|(id, session)| {
            serde_json::json!({
                "execution_id": id,
                "session_id": session.session_id,
                "target_url": session.target_url,
                "is_running": session.is_running
            })
        })
        .collect();
    Ok(result)
}
