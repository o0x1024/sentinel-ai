//! 窗口管理命令
//! 
//! 提供窗口创建、管理和控制的Tauri命令接口

use tauri::{command, AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, PhysicalPosition, PhysicalSize};
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

/// 创建AI助手窗口
#[command]
pub async fn create_ai_chat_window(app: AppHandle) -> Result<String, String> {
    // 检查窗口是否已存在
    if let Some(existing_window) = app.get_webview_window("ai-chat") {
        // 如果窗口已存在，显示并聚焦
        existing_window.show().map_err(|e| e.to_string())?;
        existing_window.set_focus().map_err(|e| e.to_string())?;
        return Ok("ai-chat".to_string());
    }

    // AI助手窗口独立配置
    let ai_window_width = 732.0;
    let ai_window_height = 820.0;
    
    // 创建AI助手窗口（独立窗口，不依赖主窗口位置）
    let ai_window = WebviewWindowBuilder::new(
        &app,
        "ai-chat",
        WebviewUrl::App("index.html".into())
    )
    .title("AI 助手")
    .inner_size(ai_window_width, ai_window_height)
    .center() // 居中显示
    .resizable(true)
    .decorations(true)
    .always_on_top(false)
    .visible(true)
    .build()
    .map_err(|e| e.to_string())?;

    // 发送事件到新窗口，告诉它只显示AI助手组件
    ai_window.emit("show-ai-chat-only", true)
        .map_err(|e| e.to_string())?;

    Ok("ai-chat".to_string())
}

/// 关闭AI助手窗口
#[command]
pub async fn close_ai_chat_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("ai-chat") {
        window.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 切换AI助手窗口显示状态
#[command]
pub async fn toggle_ai_chat_window(app: AppHandle) -> Result<String, String> {
    if let Some(window) = app.get_webview_window("ai-chat") {
        if window.is_visible().unwrap_or(false) {
            window.hide().map_err(|e| e.to_string())?;
            Ok("hidden".to_string())
        } else {
            window.show().map_err(|e| e.to_string())?;
            window.set_focus().map_err(|e| e.to_string())?;
            Ok("shown".to_string())
        }
    } else {
        // 窗口不存在，创建新窗口
        create_ai_chat_window(app).await?;
        Ok("created".to_string())
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