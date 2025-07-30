use crate::engines::{ScanEngine, ScanEngineConfig, ScanEngineEvent, ScanEngineStatus};
use crate::services::scan_session::ScanSessionService;
use crate::tools::tool_manager::ToolManager;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

/// 扫描引擎管理器
pub struct ScanEngineManager {
    engine: Arc<RwLock<Option<ScanEngine>>>,
    event_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<ScanEngineEvent>>>>,
}

impl ScanEngineManager {
    pub fn new() -> Self {
        Self {
            engine: Arc::new(RwLock::new(None)),
            event_receiver: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn initialize(
        &self,
        config: ScanEngineConfig,
        tool_manager: Arc<ToolManager>,
        scan_session_service: Arc<ScanSessionService>,
    ) -> Result<()> {
        let (engine, event_receiver) = ScanEngine::new(config, tool_manager, scan_session_service);

        *self.engine.write().await = Some(engine);
        *self.event_receiver.write().await = Some(event_receiver);

        Ok(())
    }

    pub async fn get_engine(&self) -> Option<Arc<ScanEngine>> {
        self.engine
            .read()
            .await
            .as_ref()
            .map(|engine| Arc::new(engine.clone()))
    }

    pub async fn get_event_receiver(&self) -> Option<mpsc::UnboundedReceiver<ScanEngineEvent>> {
        // UnboundedReceiver不能被克隆，这里返回None或重新设计
        None
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StartScanEngineRequest {
    pub session_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StopScanEngineRequest {
    pub session_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScanEngineResponse {
    pub success: bool,
    pub message: Option<String>,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScanEngineStatusResponse {
    pub success: bool,
    pub status: Option<ScanEngineStatus>,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScanEngineConfigRequest {
    pub max_concurrent_scans: Option<usize>,
    pub scan_timeout_seconds: Option<u64>,
    pub stage_delay_ms: Option<u64>,
    pub enable_smart_optimization: Option<bool>,
}

/// 初始化扫描引擎
#[tauri::command]
pub async fn initialize_scan_engine(
    config_request: Option<ScanEngineConfigRequest>,
    app_handle: AppHandle,
) -> Result<ScanEngineResponse, String> {
    let tool_manager = app_handle.state::<Arc<ToolManager>>().inner().clone();

    let scan_session_service = app_handle
        .state::<Arc<ScanSessionService>>()
        .inner()
        .clone();

    // 构建配置
    let mut config = ScanEngineConfig::default();
    if let Some(req) = config_request {
        if let Some(max_concurrent) = req.max_concurrent_scans {
            config.max_concurrent_scans = max_concurrent;
        }
        if let Some(timeout) = req.scan_timeout_seconds {
            config.scan_timeout_seconds = timeout;
        }
        if let Some(delay) = req.stage_delay_ms {
            config.stage_delay_ms = delay;
        }
        if let Some(optimization) = req.enable_smart_optimization {
            config.enable_smart_optimization = optimization;
        }
    }

    // 获取或创建扫描引擎管理器
    let manager = if let Some(manager) = app_handle.try_state::<Arc<ScanEngineManager>>() {
        manager.inner().clone()
    } else {
        let manager = Arc::new(ScanEngineManager::new());
        app_handle.manage(manager.clone());
        manager
    };

    match manager
        .initialize(config, tool_manager, scan_session_service)
        .await
    {
        Ok(_) => Ok(ScanEngineResponse {
            success: true,
            message: Some("扫描引擎初始化成功".to_string()),
            data: None,
        }),
        Err(e) => Ok(ScanEngineResponse {
            success: false,
            message: Some(format!("扫描引擎初始化失败: {}", e)),
            data: None,
        }),
    }
}

/// 启动扫描
#[tauri::command]
pub async fn start_scan_engine(
    request: StartScanEngineRequest,
    engine_manager: State<'_, Arc<ScanEngineManager>>,
) -> Result<ScanEngineResponse, String> {
    let session_id =
        Uuid::parse_str(&request.session_id).map_err(|e| format!("无效的会话ID: {}", e))?;

    if let Some(engine) = engine_manager.get_engine().await {
        match engine.start_scan(session_id).await {
            Ok(_) => Ok(ScanEngineResponse {
                success: true,
                message: Some("扫描启动成功".to_string()),
                data: Some(serde_json::json!({ "session_id": session_id })),
            }),
            Err(e) => Ok(ScanEngineResponse {
                success: false,
                message: Some(format!("启动扫描失败: {}", e)),
                data: None,
            }),
        }
    } else {
        Ok(ScanEngineResponse {
            success: false,
            message: Some("扫描引擎未初始化".to_string()),
            data: None,
        })
    }
}

/// 停止扫描
#[tauri::command]
pub async fn stop_scan_engine(
    request: StopScanEngineRequest,
    engine_manager: State<'_, Arc<ScanEngineManager>>,
) -> Result<ScanEngineResponse, String> {
    let session_id =
        Uuid::parse_str(&request.session_id).map_err(|e| format!("无效的会话ID: {}", e))?;

    if let Some(engine) = engine_manager.get_engine().await {
        match engine.stop_scan(session_id).await {
            Ok(_) => Ok(ScanEngineResponse {
                success: true,
                message: Some("扫描停止成功".to_string()),
                data: Some(serde_json::json!({ "session_id": session_id })),
            }),
            Err(e) => Ok(ScanEngineResponse {
                success: false,
                message: Some(format!("停止扫描失败: {}", e)),
                data: None,
            }),
        }
    } else {
        Ok(ScanEngineResponse {
            success: false,
            message: Some("扫描引擎未初始化".to_string()),
            data: None,
        })
    }
}

/// 获取扫描引擎状态
#[tauri::command]
pub async fn get_scan_engine_status(
    engine_manager: State<'_, Arc<ScanEngineManager>>,
) -> Result<ScanEngineStatusResponse, String> {
    if let Some(engine) = engine_manager.get_engine().await {
        let status = engine.get_status().await;
        Ok(ScanEngineStatusResponse {
            success: true,
            status: Some(status),
            message: None,
        })
    } else {
        Ok(ScanEngineStatusResponse {
            success: false,
            status: None,
            message: Some("扫描引擎未初始化".to_string()),
        })
    }
}

/// 获取扫描引擎配置
#[tauri::command]
pub async fn get_scan_engine_config() -> Result<ScanEngineResponse, String> {
    let config = ScanEngineConfig::default();
    Ok(ScanEngineResponse {
        success: true,
        message: None,
        data: Some(serde_json::json!({
            "max_concurrent_scans": config.max_concurrent_scans,
            "scan_timeout_seconds": config.scan_timeout_seconds,
            "stage_delay_ms": config.stage_delay_ms,
            "enable_smart_optimization": config.enable_smart_optimization
        })),
    })
}

/// 更新扫描引擎配置
#[tauri::command]
pub async fn update_scan_engine_config(
    config_request: ScanEngineConfigRequest,
    app_handle: AppHandle,
) -> Result<ScanEngineResponse, String> {
    // 重新初始化引擎以应用新配置
    initialize_scan_engine(Some(config_request), app_handle).await
}
