use crate::services::ai::AiServiceManager;
use crate::services::database::DatabaseService;
use crate::services::ensure_bot_console_access;
use crate::services::weixin_gateway::{
    start_weixin_gateway_runtime, stop_weixin_gateway_runtime, weixin_gateway_status,
    WeixinGatewayConfig, WeixinGatewayStatus, WeixinIlinkClient, WeixinQrLoginResponse,
    WeixinQrLoginStatus,
};
use sentinel_db::Database;
use std::sync::Arc;
use tauri::State;

const WEIXIN_GATEWAY_CATEGORY: &str = "network";
const WEIXIN_GATEWAY_KEY: &str = "weixin_gateway_config";

async fn load_weixin_gateway_config(
    db: &Arc<DatabaseService>,
) -> Result<WeixinGatewayConfig, String> {
    match db
        .get_config(WEIXIN_GATEWAY_CATEGORY, WEIXIN_GATEWAY_KEY)
        .await
    {
        Ok(Some(raw)) => {
            let mut config = serde_json::from_str::<WeixinGatewayConfig>(&raw)
                .map_err(|e| format!("Failed to parse Weixin gateway config: {e}"))?;
            config.normalize();
            Ok(config)
        }
        Ok(None) => Ok(WeixinGatewayConfig::default()),
        Err(e) => Err(format!("Failed to load Weixin gateway config: {e}")),
    }
}

async fn save_weixin_gateway_config_internal(
    db: &Arc<DatabaseService>,
    config: &WeixinGatewayConfig,
) -> Result<(), String> {
    let serialized = serde_json::to_string(config)
        .map_err(|e| format!("Failed to serialize Weixin gateway config: {e}"))?;
    db.set_config(
        WEIXIN_GATEWAY_CATEGORY,
        WEIXIN_GATEWAY_KEY,
        &serialized,
        Some("Weixin remote control gateway config"),
    )
    .await
    .map_err(|e| format!("Failed to save Weixin gateway config: {e}"))
}

pub async fn auto_start_weixin_gateway_if_enabled(
    db: Arc<DatabaseService>,
    ai_manager: Arc<AiServiceManager>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let config = load_weixin_gateway_config(&db).await?;
    if !config.enabled {
        return Ok(());
    }
    ensure_bot_console_access()?;
    start_weixin_gateway_runtime(config, db, ai_manager, app_handle).await
}

#[tauri::command]
pub async fn get_weixin_gateway_config(
    db: State<'_, Arc<DatabaseService>>,
) -> Result<WeixinGatewayConfig, String> {
    load_weixin_gateway_config(db.inner()).await
}

#[tauri::command]
pub async fn save_weixin_gateway_config(
    mut config: WeixinGatewayConfig,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    ensure_bot_console_access()?;

    config.normalize();
    save_weixin_gateway_config_internal(db.inner(), &config).await
}

#[tauri::command]
pub async fn start_weixin_gateway(
    config: Option<WeixinGatewayConfig>,
    db: State<'_, Arc<DatabaseService>>,
    ai_manager: State<'_, Arc<AiServiceManager>>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    ensure_bot_console_access()?;

    let resolved = if let Some(mut config) = config {
        config.normalize();
        config
    } else {
        load_weixin_gateway_config(db.inner()).await?
    };
    start_weixin_gateway_runtime(
        resolved,
        db.inner().clone(),
        ai_manager.inner().clone(),
        app_handle,
    )
    .await?;
    Ok("Weixin gateway started".to_string())
}

#[tauri::command]
pub async fn stop_weixin_gateway() -> Result<String, String> {
    ensure_bot_console_access()?;

    stop_weixin_gateway_runtime().await?;
    Ok("Weixin gateway stopped".to_string())
}

#[tauri::command]
pub async fn get_weixin_gateway_status() -> Result<WeixinGatewayStatus, String> {
    Ok(weixin_gateway_status().await)
}

#[tauri::command]
pub async fn create_weixin_qr_login(
    bot_type: Option<String>,
) -> Result<WeixinQrLoginResponse, String> {
    ensure_bot_console_access()?;

    let client = WeixinIlinkClient::new()?;
    client
        .create_qr_login(bot_type.as_deref().unwrap_or("3"))
        .await
}

#[tauri::command]
pub async fn poll_weixin_qr_login(
    qrcode: String,
    base_url: Option<String>,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<WeixinQrLoginStatus, String> {
    ensure_bot_console_access()?;

    let client = WeixinIlinkClient::new()?;
    let (status, credentials) = client
        .poll_qr_login_with_credentials(&qrcode, base_url.as_deref())
        .await?;

    if let Some(credentials) = credentials {
        let mut config = load_weixin_gateway_config(db.inner()).await?;
        let user_id = credentials.user_id.clone();
        config.account_id = credentials.account_id;
        config.token = credentials.token;
        config.base_url = credentials.base_url;
        config.normalize();
        save_weixin_gateway_config_internal(db.inner(), &config).await?;
        tracing::info!(
            "Weixin QR login confirmed for account {}, user {:?}",
            config.account_id,
            user_id
        );
    }

    Ok(status)
}
