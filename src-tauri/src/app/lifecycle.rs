use std::sync::Arc;

use sentinel_core::global_proxy::{set_global_proxy as set_proxy_async, GlobalProxyConfig};
use sentinel_db::Database;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};
use tauri_plugin_dialog::{
    DialogExt, MessageDialogButtons, MessageDialogKind, MessageDialogResult,
};
use tauri_plugin_window_state::{AppHandleExt, StateFlags};

use crate::commands::traffic::{self, TrafficAnalysisState};
use crate::services::database::DatabaseService;

struct TrayProxyMenuItem(MenuItem<tauri::Wry>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CloseButtonAction {
    Hide,
    Minimize,
    Exit,
}

impl CloseButtonAction {
    fn from_config(value: &str) -> Option<Self> {
        let normalized = value.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "hide" | "tray" | "close_to_tray" => Some(Self::Hide),
            "minimize" | "minimise" => Some(Self::Minimize),
            "exit" | "quit" | "close" => Some(Self::Exit),
            _ => None,
        }
    }

    fn as_config_value(self) -> &'static str {
        match self {
            Self::Hide => "hide",
            Self::Minimize => "minimize",
            Self::Exit => "exit",
        }
    }
}

fn apply_hide_close_behavior(window: &tauri::Window) {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = window.set_skip_taskbar(true);
    }
    let _ = window.hide();
}

fn apply_minimize_close_behavior(window: &tauri::Window) {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = window.set_skip_taskbar(false);
    }
    let _ = window.minimize();
}

async fn execute_close_button_action(
    app: &tauri::AppHandle,
    window: &tauri::Window,
    action: CloseButtonAction,
) {
    match action {
        CloseButtonAction::Hide => apply_hide_close_behavior(window),
        CloseButtonAction::Minimize => apply_minimize_close_behavior(window),
        CloseButtonAction::Exit => cleanup_and_exit(app).await,
    }
}

fn prompt_and_apply_close_button_action(
    app_handle: tauri::AppHandle,
    db_service: Arc<DatabaseService>,
    window: tauri::Window,
) {
    const HIDE_LABEL: &str = "隐藏窗口";
    const EXIT_LABEL: &str = "退出程序";

    app_handle
        .dialog()
        .message("点击关闭按钮时请选择行为：隐藏窗口（后台运行）或退出程序。")
        .title("关闭行为设置")
        .kind(MessageDialogKind::Info)
        .buttons(MessageDialogButtons::OkCancelCustom(
            HIDE_LABEL.to_string(),
            EXIT_LABEL.to_string(),
        ))
        .show_with_result(move |result| {
            let selected = match result {
                MessageDialogResult::Custom(label) if label == HIDE_LABEL => {
                    Some(CloseButtonAction::Hide)
                }
                MessageDialogResult::Custom(label) if label == EXIT_LABEL => {
                    Some(CloseButtonAction::Exit)
                }
                MessageDialogResult::Ok | MessageDialogResult::Yes => Some(CloseButtonAction::Hide),
                MessageDialogResult::No => Some(CloseButtonAction::Exit),
                MessageDialogResult::Cancel => None,
                _ => None,
            };

            let app_for_action = app_handle.clone();
            let db_for_action = db_service.clone();
            let window_for_action = window.clone();
            tauri::async_runtime::spawn(async move {
                if let Some(action) = selected {
                    if let Err(e) = db_for_action
                        .set_config(
                            "ui",
                            "close_button_action",
                            action.as_config_value(),
                            Some("Window close button action: hide, minimize or exit"),
                        )
                        .await
                    {
                        tracing::warn!("Failed to save close button action: {}", e);
                    }
                    execute_close_button_action(&app_for_action, &window_for_action, action).await;
                } else {
                    // Dialog dismissed: keep app running and avoid accidental exit.
                    apply_hide_close_behavior(&window_for_action);
                }
            });
        });
}

pub fn handle_window_event(window: &tauri::Window, event: &WindowEvent) {
    if window.label() != "main" {
        return;
    }

    if let WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();

        let app_handle = window.app_handle();
        let _ = app_handle.save_window_state(StateFlags::all());
        let window_for_action = window.clone();

        let db_service = app_handle
            .try_state::<Arc<DatabaseService>>()
            .map(|state| state.inner().clone());

        if let Some(db_service) = db_service {
            let app_for_action = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                match db_service.get_config("ui", "close_button_action").await {
                    Ok(Some(value)) => {
                        if let Some(action) = CloseButtonAction::from_config(&value) {
                            execute_close_button_action(
                                &app_for_action,
                                &window_for_action,
                                action,
                            )
                            .await;
                        } else {
                            prompt_and_apply_close_button_action(
                                app_for_action,
                                db_service.clone(),
                                window_for_action,
                            );
                        }
                    }
                    Ok(None) => {
                        prompt_and_apply_close_button_action(
                            app_for_action,
                            db_service.clone(),
                            window_for_action,
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to load close button action, fallback to minimize: {}",
                            e
                        );
                        apply_minimize_close_behavior(&window_for_action);
                    }
                }
            });
        } else {
            // Database state unavailable, fallback to minimize to keep app running.
            apply_minimize_close_behavior(&window_for_action);
        }
    }
}

pub fn setup_tray(
    app: &mut tauri::App<tauri::Wry>,
    handle: &tauri::AppHandle,
) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, "show", "显示主界面", true, None::<&str>)?;
    let proxy_item = MenuItem::with_id(app, "proxy", "开启代理", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    handle.manage(TrayProxyMenuItem(proxy_item.clone()));
    let tray_menu = Menu::with_items(app, &[&show_item, &proxy_item, &quit_item])?;

    let mut tray_builder = TrayIconBuilder::with_id("main")
        .tooltip("Sentinel AI")
        .menu(&tray_menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    #[cfg(not(target_os = "macos"))]
                    {
                        let _ = window.set_skip_taskbar(false);
                    }
                    let _ = window.show();
                    let _ = window.set_focus();
                    let _ = window.unminimize();
                }
            }
            "proxy" => {
                let app_clone = app.clone();
                tauri::async_runtime::spawn(async move {
                    toggle_proxy(&app_clone).await;
                });
            }
            "quit" => {
                let app_clone = app.clone();
                tauri::async_runtime::spawn(async move {
                    cleanup_and_exit(&app_clone).await;
                });
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button,
                button_state,
                ..
            } = event
            {
                // Left click: show main window
                if button == tauri::tray::MouseButton::Left
                    && button_state == tauri::tray::MouseButtonState::Up
                {
                    let app = tray.app_handle();
                    if let Some(window) = app.get_webview_window("main") {
                        #[cfg(not(target_os = "macos"))]
                        {
                            let _ = window.set_skip_taskbar(false);
                        }
                        let _ = window.show();
                        let _ = window.set_focus();
                        let _ = window.unminimize();
                    }
                }
                // Right click menu is handled automatically by .menu()
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray_builder = tray_builder.icon(icon.clone());
    }
    let _tray_icon = tray_builder.build(app)?;
    Ok(())
}

async fn toggle_proxy(app: &tauri::AppHandle) {
    if let Some(state) = app.try_state::<TrafficAnalysisState>() {
        let is_running_arc = state.get_is_running();
        let is_running = *is_running_arc.read().await;

        if is_running {
            match traffic::stop_traffic_analysis_internal(app, &state).await {
                Ok(_) => {
                    tracing::info!("Proxy stopped from tray menu");
                    update_proxy_menu_text(app, false);
                }
                Err(e) => tracing::error!("Failed to stop proxy: {}", e),
            }
        } else {
            match traffic::start_traffic_analysis_internal(app, &state, None).await {
                Ok(port) => {
                    tracing::info!("Proxy started from tray menu on port {}", port);
                    update_proxy_menu_text(app, true);
                }
                Err(e) => tracing::error!("Failed to start proxy: {}", e),
            }
        }
    }
}

pub(crate) fn update_proxy_menu_text(app: &tauri::AppHandle, is_running: bool) {
    if let Some(proxy_item) = app.try_state::<TrayProxyMenuItem>() {
        let text = if is_running {
            "关闭代理"
        } else {
            "开启代理"
        };
        let _ = proxy_item.0.set_text(text);
    }
}

async fn cleanup_and_exit(app: &tauri::AppHandle) {
    let _ = app.save_window_state(StateFlags::all());

    if let Some(state) = app.try_state::<TrafficAnalysisState>() {
        let is_running_arc = state.get_is_running();
        let is_running = *is_running_arc.read().await;
        if is_running {
            let _ = traffic::stop_traffic_analysis_internal(app, &state).await;
        }
    }

    tracing::info!("Application cleanup completed, exiting");
    std::process::exit(0);
}

pub async fn initialize_global_proxy(db_service: &DatabaseService) -> anyhow::Result<()> {
    match db_service.get_config("network", "global_proxy").await {
        Ok(Some(json_str)) => match serde_json::from_str::<GlobalProxyConfig>(&json_str) {
            Ok(proxy_config) => {
                if proxy_config.enabled {
                    set_proxy_async(proxy_config.clone()).await;
                } else {
                    sentinel_core::global_proxy::clear_global_proxy().await;
                }
            }
            Err(e) => tracing::warn!("Failed to parse proxy configuration JSON: {}", e),
        },
        Ok(None) => {
            sentinel_core::global_proxy::clear_global_proxy().await;
        }
        Err(e) => {
            tracing::warn!("Failed to load proxy configuration from database: {}", e);
            sentinel_core::global_proxy::clear_global_proxy().await;
        }
    }
    Ok(())
}

/// Auto-start proxy listener if enabled in configuration
pub async fn auto_start_proxy_if_enabled(
    app: &tauri::AppHandle,
    traffic_state: &TrafficAnalysisState,
) -> anyhow::Result<()> {
    tracing::info!("Checking if proxy auto-start is enabled...");

    // Get database service
    let db_service = traffic_state.get_db_service();

    // Load proxy auto-start configuration
    let auto_start_enabled = match db_service
        .load_proxy_config("proxy_auto_start_enabled")
        .await
    {
        Ok(Some(value)) => value.parse::<bool>().unwrap_or(false),
        _ => false,
    };

    if !auto_start_enabled {
        tracing::info!("Proxy auto-start is disabled in configuration");
        return Ok(());
    }

    // Check if proxy is already running using public method
    let is_running_arc = traffic_state.get_is_running();
    let is_running = *is_running_arc.read().await;
    if is_running {
        tracing::info!("Proxy is already running, skipping auto-start");
        return Ok(());
    }

    // Load proxy configuration from database
    let config = match db_service.load_proxy_config("proxy_config").await {
        Ok(Some(config_json)) => {
            match serde_json::from_str::<sentinel_traffic::ProxyConfig>(&config_json) {
                Ok(config) => {
                    tracing::info!(
                        "Loaded proxy configuration for auto-start: port {}",
                        config.start_port
                    );
                    Some(config)
                }
                Err(e) => {
                    tracing::warn!("Failed to deserialize proxy config, using default: {}", e);
                    None
                }
            }
        }
        Ok(None) => {
            tracing::info!("No saved proxy configuration found, using default");
            None
        }
        Err(e) => {
            tracing::warn!("Failed to load proxy config from database: {}", e);
            None
        }
    };

    // Start the proxy listener
    match traffic::start_traffic_analysis_internal(app, traffic_state, config).await {
        Ok(port) => {
            tracing::info!(
                "✅ Proxy listener auto-started successfully on port {}",
                port
            );
            update_proxy_menu_text(app, true);
            Ok(())
        }
        Err(e) => {
            tracing::error!("Failed to auto-start proxy listener: {}", e);
            Err(anyhow::anyhow!("Failed to start proxy: {}", e))
        }
    }
}
