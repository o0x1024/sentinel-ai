use crate::services::database::DatabaseService;
use crate::services::http_gateway::{
    hash_api_key_for_storage, normalize_gateway_config, start_gateway_server, HttpGatewayConfig,
    HttpGatewayRuntime, HttpGatewayStatus,
};
use crate::services::ai::AiServiceManager;
use once_cell::sync::Lazy;
use rand::distributions::{Alphanumeric, DistString};
use sentinel_db::Database;
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

const HTTP_GATEWAY_CATEGORY: &str = "network";
const HTTP_GATEWAY_KEY: &str = "http_gateway_config";

struct HttpGatewayRuntimeState {
    runtime: Option<HttpGatewayRuntime>,
    last_error: Option<String>,
}

impl Default for HttpGatewayRuntimeState {
    fn default() -> Self {
        Self {
            runtime: None,
            last_error: None,
        }
    }
}

static HTTP_GATEWAY_RUNTIME: Lazy<Arc<RwLock<HttpGatewayRuntimeState>>> =
    Lazy::new(|| Arc::new(RwLock::new(HttpGatewayRuntimeState::default())));

async fn load_gateway_config(db: &Arc<DatabaseService>) -> Result<HttpGatewayConfig, String> {
    match db.get_config(HTTP_GATEWAY_CATEGORY, HTTP_GATEWAY_KEY).await {
        Ok(Some(raw)) => {
            let mut cfg = serde_json::from_str::<HttpGatewayConfig>(&raw)
                .map_err(|e| format!("Failed to parse gateway config: {}", e))?;
            normalize_gateway_config(&mut cfg);
            Ok(cfg)
        }
        Ok(None) => Ok(HttpGatewayConfig::default()),
        Err(e) => Err(format!("Failed to load gateway config: {}", e)),
    }
}

pub async fn auto_start_http_gateway_if_enabled(
    db: Arc<DatabaseService>,
    ai_manager: Arc<AiServiceManager>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let config = load_gateway_config(&db).await?;
    if !config.enabled {
        return Ok(());
    }

    let mut state = HTTP_GATEWAY_RUNTIME.write().await;
    if state
        .runtime
        .as_ref()
        .map(|runtime| runtime.is_finished())
        .unwrap_or(false)
    {
        state.runtime = None;
    }
    if state.runtime.is_some() {
        return Ok(());
    }

    match start_gateway_server(&config, ai_manager, db.clone(), app_handle).await {
        Ok(runtime) => {
            state.runtime = Some(runtime);
            state.last_error = None;
            Ok(())
        }
        Err(e) => {
            state.last_error = Some(e.clone());
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn save_http_gateway_config(
    mut config: HttpGatewayConfig,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    normalize_gateway_config(&mut config);
    let serialized = serde_json::to_string(&config)
        .map_err(|e| format!("Failed to serialize gateway config: {}", e))?;

    db.set_config(
        HTTP_GATEWAY_CATEGORY,
        HTTP_GATEWAY_KEY,
        &serialized,
        Some("HTTP gateway config"),
    )
    .await
    .map_err(|e| format!("Failed to save gateway config: {}", e))
}

#[tauri::command]
pub async fn get_http_gateway_config(
    db: State<'_, Arc<DatabaseService>>,
) -> Result<HttpGatewayConfig, String> {
    load_gateway_config(db.inner()).await
}

#[tauri::command]
pub async fn start_http_gateway(
    config: Option<HttpGatewayConfig>,
    db: State<'_, Arc<DatabaseService>>,
    ai_manager: State<'_, Arc<AiServiceManager>>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let resolved_config = if let Some(cfg) = config {
        cfg
    } else {
        load_gateway_config(db.inner()).await?
    };

    let mut state = HTTP_GATEWAY_RUNTIME.write().await;
    if state
        .runtime
        .as_ref()
        .map(|runtime| runtime.is_finished())
        .unwrap_or(false)
    {
        state.runtime = None;
    }

    if state.runtime.is_some() {
        return Err("HTTP gateway is already running".to_string());
    }

    match start_gateway_server(
        &resolved_config,
        ai_manager.inner().clone(),
        db.inner().clone(),
        app_handle,
    ).await {
        Ok(runtime) => {
            let bind_addr = runtime.bind_addr();
            state.runtime = Some(runtime);
            state.last_error = None;
            Ok(format!("HTTP gateway started on {}", bind_addr))
        }
        Err(e) => {
            state.last_error = Some(e.clone());
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn stop_http_gateway() -> Result<String, String> {
    let runtime = {
        let mut state = HTTP_GATEWAY_RUNTIME.write().await;
        state.runtime.take()
    };

    if let Some(runtime) = runtime {
        runtime.stop().await;
        Ok("HTTP gateway stopped".to_string())
    } else {
        Ok("HTTP gateway is already stopped".to_string())
    }
}

#[tauri::command]
pub async fn get_http_gateway_status() -> Result<HttpGatewayStatus, String> {
    let state = HTTP_GATEWAY_RUNTIME.read().await;

    if let Some(runtime) = state.runtime.as_ref() {
        Ok(HttpGatewayStatus {
            running: !runtime.is_finished(),
            bind_addr: Some(runtime.bind_addr()),
            started_at: Some(runtime.started_at()),
            last_error: state.last_error.clone(),
        })
    } else {
        Ok(HttpGatewayStatus {
            running: false,
            bind_addr: None,
            started_at: None,
            last_error: state.last_error.clone(),
        })
    }
}

#[tauri::command]
pub async fn rotate_http_gateway_api_key(
    db: State<'_, Arc<DatabaseService>>,
) -> Result<String, String> {
    let raw_key = format!(
        "sgw_{}",
        Alphanumeric.sample_string(&mut rand::thread_rng(), 40)
    );
    let hashed = hash_api_key_for_storage(&raw_key);

    let mut config = load_gateway_config(db.inner()).await?;
    if !config.auth.api_keys.contains(&hashed) {
        config.auth.api_keys.push(hashed);
    }
    config.auth.required = true;
    normalize_gateway_config(&mut config);

    let serialized = serde_json::to_string(&config)
        .map_err(|e| format!("Failed to serialize gateway config: {}", e))?;
    db.set_config(
        HTTP_GATEWAY_CATEGORY,
        HTTP_GATEWAY_KEY,
        &serialized,
        Some("HTTP gateway config"),
    )
    .await
    .map_err(|e| format!("Failed to save gateway config: {}", e))?;

    Ok(raw_key)
}
