use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use super::analysis_commands::start_traffic_analysis_internal;
use super::TrafficAnalysisState;
use crate::commands::command_response_support::CommandResponse;
use crate::events::{emit_proxy_status, ProxyStatusEvent};
use sentinel_traffic::ProxyStats;

/// 下载 Root CA 证书（返回证书路径）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaCertPath {
    pub path: String,
}

/// 下载 CA 证书（前端友好接口）
#[tauri::command]
pub async fn download_ca_cert(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<CaCertPath>, String> {
    if let Err(e) = state.certificate_service.ensure_root_ca().await {
        return Ok(CommandResponse::err(format!("Failed to ensure CA: {}", e)));
    }

    match state.certificate_service.export_root_ca() {
        Ok(path) => {
            let path_str = path.to_string_lossy().to_string();
            tracing::info!("CA certificate available at: {}", path_str);
            Ok(CommandResponse::ok(CaCertPath { path: path_str }))
        }
        Err(e) => Ok(CommandResponse::err(format!(
            "Failed to get CA path: {}",
            e
        ))),
    }
}

#[tauri::command]
pub async fn get_ca_cert_path(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<String>, String> {
    if let Err(e) = state.certificate_service.ensure_root_ca().await {
        return Ok(CommandResponse::err(format!("Failed to ensure CA: {}", e)));
    }

    match state.certificate_service.export_root_ca() {
        Ok(path) => {
            let path_str = path.to_string_lossy().to_string();
            tracing::info!("CA certificate path: {}", path_str);
            Ok(CommandResponse::ok(path_str))
        }
        Err(e) => Ok(CommandResponse::err(format!(
            "Failed to get CA path: {}",
            e
        ))),
    }
}

#[tauri::command]
pub async fn trust_ca_cert(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<String>, String> {
    #[cfg(target_os = "macos")]
    {
        if let Err(e) = state.certificate_service.ensure_root_ca().await {
            return Ok(CommandResponse::err(format!("Failed to ensure CA: {}", e)));
        }

        match state.certificate_service.trust_root_ca_macos().await {
            Ok(_) => {
                tracing::info!("CA certificate trusted in macOS Keychain");
                Ok(CommandResponse::ok(
                    "CA certificate successfully trusted in system Keychain".to_string(),
                ))
            }
            Err(e) => Ok(CommandResponse::err(format!(
                "Failed to trust CA: {}. You may need to run with administrator privileges.",
                e
            ))),
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(CommandResponse::err(
            "Certificate trust is only supported on macOS".to_string(),
        ))
    }
}

#[tauri::command]
pub async fn regenerate_ca_cert(
    app: AppHandle,
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<String>, String> {
    let was_running = *state.is_running.read().await;

    if was_running {
        tracing::info!("Proxy is running, stopping before regenerating CA...");

        if let Some(proxy) = state.proxy_service.read().await.as_ref() {
            let _port = proxy.get_port().await;
        }

        let mut proxy = state.proxy_service.write().await;
        if let Some(p) = proxy.take() {
            if let Err(e) = p.stop().await {
                tracing::error!("Failed to stop proxy before regenerating CA: {}", e);
            }
        }

        *state.is_running.write().await = false;

        emit_proxy_status(
            &app,
            ProxyStatusEvent {
                running: false,
                port: 0,
                mitm: false,
                stats: ProxyStats::default(),
            },
        );
    }

    match state.certificate_service.regenerate_root_ca().await {
        Ok(_) => {
            let path = state
                .certificate_service
                .export_root_ca()
                .map_err(|e| format!("Failed to get CA path: {}", e))?;
            let path_str = path.to_string_lossy().to_string();

            tracing::info!("CA certificate regenerated at: {}", path_str);

            if was_running {
                tracing::info!("Restarting proxy with new CA certificate...");

                match start_traffic_analysis_internal(&app, &state, None).await {
                    Ok(new_port) => {
                        tracing::info!(
                            "Proxy restarted on port {} with new CA certificate",
                            new_port
                        );
                        Ok(CommandResponse::ok(format!(
                            "CA certificate regenerated and proxy restarted on port {}. Please re-import: {}",
                            new_port, path_str
                        )))
                    }
                    Err(e) => {
                        tracing::error!("Failed to restart proxy after CA regeneration: {}", e);
                        Ok(CommandResponse::ok(format!(
                            "CA certificate regenerated but failed to restart proxy: {}. Please re-import: {}",
                            e, path_str
                        )))
                    }
                }
            } else {
                Ok(CommandResponse::ok(format!(
                    "CA certificate regenerated successfully. Please re-import: {}",
                    path_str
                )))
            }
        }
        Err(e) => Ok(CommandResponse::err(format!(
            "Failed to regenerate CA: {}",
            e
        ))),
    }
}

#[tauri::command]
pub async fn get_ca_fingerprint(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<String>, String> {
    match state.certificate_service.get_certificate_fingerprint() {
        Ok(fingerprint) => {
            tracing::info!("CA certificate fingerprint: {}", fingerprint);
            Ok(CommandResponse::ok(fingerprint))
        }
        Err(e) => Ok(CommandResponse::err(format!(
            "Failed to get fingerprint: {}",
            e
        ))),
    }
}

#[tauri::command]
pub async fn export_ca_cert(
    state: State<'_, TrafficAnalysisState>,
    format: String,
) -> Result<CommandResponse<CaCertPath>, String> {
    if let Err(e) = state.certificate_service.ensure_root_ca().await {
        return Ok(CommandResponse::err(format!("Failed to ensure CA: {}", e)));
    }

    match format.as_str() {
        "der" => match state.certificate_service.export_cert_der() {
            Ok(path) => {
                let path_str = path.to_string_lossy().to_string();
                tracing::info!("Exported CA certificate in DER format: {}", path_str);
                Ok(CommandResponse::ok(CaCertPath { path: path_str }))
            }
            Err(e) => Ok(CommandResponse::err(format!("Failed to export: {}", e))),
        },
        _ => match state.certificate_service.export_root_ca() {
            Ok(path) => {
                let path_str = path.to_string_lossy().to_string();
                Ok(CommandResponse::ok(CaCertPath { path: path_str }))
            }
            Err(e) => Ok(CommandResponse::err(format!("Failed to export: {}", e))),
        },
    }
}

#[tauri::command]
pub async fn export_ca_key(
    state: State<'_, TrafficAnalysisState>,
    format: String,
) -> Result<CommandResponse<CaCertPath>, String> {
    if let Err(e) = state.certificate_service.ensure_root_ca().await {
        return Ok(CommandResponse::err(format!("Failed to ensure CA: {}", e)));
    }

    match format.as_str() {
        "der" => match state.certificate_service.export_key_der() {
            Ok(path) => {
                let path_str = path.to_string_lossy().to_string();
                tracing::info!("Exported CA private key in DER format: {}", path_str);
                Ok(CommandResponse::ok(CaCertPath { path: path_str }))
            }
            Err(e) => Ok(CommandResponse::err(format!("Failed to export: {}", e))),
        },
        _ => Ok(CommandResponse::err("Unsupported format".to_string())),
    }
}

#[tauri::command]
pub async fn export_ca_pkcs12(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<CaCertPath>, String> {
    if let Err(e) = state.certificate_service.ensure_root_ca().await {
        return Ok(CommandResponse::err(format!("Failed to ensure CA: {}", e)));
    }

    match state.certificate_service.export_root_ca() {
        Ok(path) => {
            let path_str = path.to_string_lossy().to_string();
            tracing::info!("Exported CA in PEM format: {}", path_str);
            Ok(CommandResponse::ok(CaCertPath { path: path_str }))
        }
        Err(e) => Ok(CommandResponse::err(format!("Failed to export: {}", e))),
    }
}

#[tauri::command]
pub async fn open_ca_cert_dir(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<String>, String> {
    match state.certificate_service.export_root_ca() {
        Ok(cert_path) => {
            let dir_path = cert_path
                .parent()
                .ok_or_else(|| "Failed to get parent directory".to_string())?;
            let dir_str = dir_path.to_string_lossy().to_string();

            #[cfg(target_os = "macos")]
            {
                match std::process::Command::new("open").arg(&dir_str).spawn() {
                    Ok(_) => {
                        tracing::info!("Opened CA cert directory: {}", dir_str);
                        Ok(CommandResponse::ok(dir_str))
                    }
                    Err(e) => Ok(CommandResponse::err(format!(
                        "Failed to open directory: {}",
                        e
                    ))),
                }
            }

            #[cfg(target_os = "windows")]
            {
                match std::process::Command::new("explorer").arg(&dir_str).spawn() {
                    Ok(_) => {
                        tracing::info!("Opened CA cert directory: {}", dir_str);
                        Ok(CommandResponse::ok(dir_str))
                    }
                    Err(e) => Ok(CommandResponse::err(format!(
                        "Failed to open directory: {}",
                        e
                    ))),
                }
            }

            #[cfg(target_os = "linux")]
            {
                match std::process::Command::new("xdg-open").arg(&dir_str).spawn() {
                    Ok(_) => {
                        tracing::info!("Opened CA cert directory: {}", dir_str);
                        Ok(CommandResponse::ok(dir_str))
                    }
                    Err(e) => Ok(CommandResponse::err(format!(
                        "Failed to open directory: {}",
                        e
                    ))),
                }
            }
        }
        Err(e) => Ok(CommandResponse::err(format!(
            "Failed to get CA path: {}",
            e
        ))),
    }
}
