//! 窗口管理命令
//! 
//! 提供窗口创建、管理和控制的Tauri命令接口

use tauri::{command, AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct WindowConfig {
    pub label: String,
    pub title: String,
    pub width: f64,
    pub height: f64,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub resizable: Option<bool>,
    pub maximized: Option<bool>,
    pub visible: Option<bool>,
    pub decorations: Option<bool>,
    pub always_on_top: Option<bool>,
}

/// 创建通用窗口
#[command]
pub async fn create_window(app: AppHandle, config: WindowConfig) -> Result<String, String> {
    // 检查窗口是否已存在
    if let Some(existing_window) = app.get_webview_window(&config.label) {
        // 如果窗口已存在，显示并聚焦
        existing_window.show().map_err(|e| e.to_string())?;
        existing_window.set_focus().map_err(|e| e.to_string())?;
        return Ok(config.label);
    }

    // 创建窗口构建器
    let mut builder = WebviewWindowBuilder::new(
        &app,
        &config.label,
        WebviewUrl::App("index.html".into())
    )
    .title(&config.title)
    .inner_size(config.width, config.height);

    // 设置位置
    if let (Some(x), Some(y)) = (config.x, config.y) {
        builder = builder.position(x, y);
    } else {
        builder = builder.center();
    }

    // 设置其他属性
    if let Some(resizable) = config.resizable {
        builder = builder.resizable(resizable);
    }
    if let Some(maximized) = config.maximized {
        builder = builder.maximized(maximized);
    }
    if let Some(visible) = config.visible {
        builder = builder.visible(visible);
    }
    if let Some(decorations) = config.decorations {
        builder = builder.decorations(decorations);
    }
    if let Some(always_on_top) = config.always_on_top {
        builder = builder.always_on_top(always_on_top);
    }

    let _window = builder.build().map_err(|e| e.to_string())?;

    Ok(config.label)
}

/// 关闭窗口
#[command]
pub async fn close_window(app: AppHandle, label: String) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(&label) {
        window.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 切换窗口显示状态
#[command]
pub async fn toggle_window(app: AppHandle, label: String) -> Result<String, String> {
    if let Some(window) = app.get_webview_window(&label) {
        if window.is_visible().unwrap_or(false) {
            window.hide().map_err(|e| e.to_string())?;
            Ok("hidden".to_string())
        } else {
            window.show().map_err(|e| e.to_string())?;
            window.set_focus().map_err(|e| e.to_string())?;
            Ok("shown".to_string())
        }
    } else {
        Err(format!("Window '{}' not found", label))
    }
}

/// 获取窗口信息
#[command]
pub async fn get_window_info(app: AppHandle, label: String) -> Result<serde_json::Value, String> {
    if let Some(window) = app.get_webview_window(&label) {
        let position = window.outer_position().map_err(|e| e.to_string())?;
        let size = window.outer_size().map_err(|e| e.to_string())?;
        let is_visible = window.is_visible().map_err(|e| e.to_string())?;
        let is_maximized = window.is_maximized().map_err(|e| e.to_string())?;
        
        Ok(serde_json::json!({
            "label": label,
            "position": {
                "x": position.x,
                "y": position.y
            },
            "size": {
                "width": size.width,
                "height": size.height
            },
            "visible": is_visible,
            "maximized": is_maximized
        }))
    } else {
        Err(format!("Window '{}' not found", label))
    }
}

/// 设置窗口位置
#[command]
pub async fn set_window_position(app: AppHandle, label: String, x: f64, y: f64) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(&label) {
        window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x: x as i32, y: y as i32 }))
            .map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err(format!("Window '{}' not found", label))
    }
}

/// 设置窗口大小
#[command]
pub async fn set_window_size(app: AppHandle, label: String, width: f64, height: f64) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(&label) {
        window.set_size(tauri::Size::Physical(tauri::PhysicalSize { width: width as u32, height: height as u32 }))
            .map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err(format!("Window '{}' not found", label))
    }
}