//! Packet capture commands - Tauri commands for network packet capture

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::RwLock;
use tracing::{error, info};

use sentinel_passive::{CapturedPacket, InterfaceInfo, PacketCaptureService};

/// Packet capture state
pub struct PacketCaptureState {
    service: Arc<RwLock<PacketCaptureService>>,
}

impl Default for PacketCaptureState {
    fn default() -> Self {
        Self {
            service: Arc::new(RwLock::new(PacketCaptureService::new())),
        }
    }
}

/// Response wrapper
#[derive(Serialize, Deserialize)]
pub struct CaptureResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> CaptureResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}

/// Get available network interfaces
#[tauri::command]
pub async fn get_network_interfaces() -> Result<Vec<InterfaceInfo>, String> {
    info!("Getting network interfaces");
    Ok(PacketCaptureService::get_interfaces())
}

/// Start packet capture on specified interface
#[tauri::command]
pub async fn start_packet_capture(
    app: AppHandle,
    state: State<'_, PacketCaptureState>,
    interface_name: String,
) -> Result<CaptureResponse<()>, String> {
    info!("Starting packet capture on interface: {}", interface_name);

    let mut service = state.service.write().await;
    
    if service.is_running() {
        return Ok(CaptureResponse::err("Capture already running"));
    }

    match service.start_capture(&interface_name) {
        Ok(mut rx) => {
            let app_handle = app.clone();
            
            // Spawn task to forward packets to frontend
            tokio::spawn(async move {
                while let Some(packet) = rx.recv().await {
                    if let Err(e) = app_handle.emit("packet-captured", &packet) {
                        error!("Failed to emit packet event: {}", e);
                    }
                }
                info!("Packet forwarding task ended");
            });

            Ok(CaptureResponse::ok(()))
        }
        Err(e) => {
            error!("Failed to start capture: {}", e);
            Ok(CaptureResponse::err(e))
        }
    }
}

/// Stop packet capture
#[tauri::command]
pub async fn stop_packet_capture(
    state: State<'_, PacketCaptureState>,
) -> Result<CaptureResponse<()>, String> {
    info!("Stopping packet capture");
    
    let mut service = state.service.write().await;
    service.stop_capture();
    
    Ok(CaptureResponse::ok(()))
}

/// Check if capture is running
#[tauri::command]
pub async fn is_capture_running(
    state: State<'_, PacketCaptureState>,
) -> Result<bool, String> {
    let service = state.service.read().await;
    Ok(service.is_running())
}

