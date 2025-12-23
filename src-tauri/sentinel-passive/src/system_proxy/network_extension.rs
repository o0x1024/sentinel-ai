//! macOS Network Extension 绑定
//!
//! 提供 Rust 接口管理 Network Extension
//! 
//! 注意：完整的 Network Extension 功能需要：
//! 1. Apple Developer 账号
//! 2. Network Extension entitlement 审批
//! 3. 构建 Swift System Extension
//! 
//! 当前提供 stub 实现，允许项目编译通过

use serde::Serialize;
use tracing::warn;

/// Extension 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionStatus {
    Unknown,
    NotInstalled,
    Installing,
    Installed,
    NeedsApproval,
    Failed,
    /// Swift 库未链接
    NotAvailable,
}

impl From<i32> for ExtensionStatus {
    fn from(value: i32) -> Self {
        match value {
            0 => ExtensionStatus::Unknown,
            1 => ExtensionStatus::NotInstalled,
            2 => ExtensionStatus::Installing,
            3 => ExtensionStatus::Installed,
            4 => ExtensionStatus::NeedsApproval,
            5 => ExtensionStatus::Failed,
            _ => ExtensionStatus::Unknown,
        }
    }
}

/// VPN 连接状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VPNStatus {
    Invalid,
    Disconnected,
    Connecting,
    Connected,
    Reasserting,
    Disconnecting,
    /// Swift 库未链接
    NotAvailable,
}

impl From<i32> for VPNStatus {
    fn from(value: i32) -> Self {
        match value {
            0 => VPNStatus::Invalid,
            1 => VPNStatus::Disconnected,
            2 => VPNStatus::Connecting,
            3 => VPNStatus::Connected,
            4 => VPNStatus::Reasserting,
            5 => VPNStatus::Disconnecting,
            _ => VPNStatus::Invalid,
        }
    }
}

/// Network Extension 管理器
/// 
/// 当前为 stub 实现，完整功能需要链接 Swift 库
pub struct NetworkExtensionManager;

impl NetworkExtensionManager {
    /// 检查 Extension 状态
    pub fn check_status() -> ExtensionStatus {
        warn!("Network Extension: Swift library not linked, returning NotAvailable");
        ExtensionStatus::NotAvailable
    }

    /// 获取当前 Extension 状态
    pub fn get_status() -> ExtensionStatus {
        ExtensionStatus::NotAvailable
    }

    /// 安装 Extension
    pub fn install() -> Result<(), String> {
        warn!("Network Extension install: Swift library not linked");
        Err("Network Extension 功能需要先构建 Swift System Extension。\n\
             请参考 src-tauri/macos-extension/SETUP.md 进行配置。".to_string())
    }

    /// 卸载 Extension
    pub fn uninstall() -> Result<(), String> {
        warn!("Network Extension uninstall: Swift library not linked");
        Err("Network Extension 功能需要先构建 Swift System Extension。".to_string())
    }

    /// 启动代理
    pub fn start_proxy(_host: &str, _port: u16, _target_apps: &[String]) -> Result<(), String> {
        warn!("Network Extension start_proxy: Swift library not linked");
        Err("Network Extension 功能需要先构建 Swift System Extension。\n\
             当前可以使用「系统代理」方式代理支持系统代理的应用。".to_string())
    }

    /// 停止代理
    pub fn stop_proxy() -> Result<(), String> {
        warn!("Network Extension stop_proxy: Swift library not linked");
        Err("Network Extension 功能需要先构建 Swift System Extension。".to_string())
    }

    /// 获取代理状态
    pub fn get_proxy_status() -> VPNStatus {
        VPNStatus::NotAvailable
    }
    
    /// 检查 Network Extension 是否可用
    pub fn is_available() -> bool {
        // 当 Swift 库链接后，这个函数应该返回 true
        false
    }
}

// ============================================================================
// 以下是完整实现（需要链接 Swift 库时启用）
// ============================================================================

/*
// 当 Swift 库编译并链接后，取消注释以下代码

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

// FFI 函数声明
extern "C" {
    fn sentinel_extension_check_status() -> i32;
    fn sentinel_extension_get_status() -> i32;
    fn sentinel_extension_install(error_buffer: *mut c_char, buffer_size: i32) -> i32;
    fn sentinel_extension_uninstall(error_buffer: *mut c_char, buffer_size: i32) -> i32;
    fn sentinel_proxy_start(
        proxy_host: *const c_char,
        proxy_port: u16,
        target_apps_json: *const c_char,
        error_buffer: *mut c_char,
        buffer_size: i32,
    ) -> i32;
    fn sentinel_proxy_stop(error_buffer: *mut c_char, buffer_size: i32) -> i32;
    fn sentinel_proxy_get_status() -> i32;
}

const ERROR_BUFFER_SIZE: usize = 512;

/// 获取错误消息
fn get_error_message(buffer: &[u8]) -> Option<String> {
    let cstr = unsafe { CStr::from_ptr(buffer.as_ptr() as *const c_char) };
    let s = cstr.to_string_lossy();
    if s.is_empty() {
        None
    } else {
        Some(s.into_owned())
    }
}

impl NetworkExtensionManager {
    pub fn check_status() -> ExtensionStatus {
        let status = unsafe { sentinel_extension_check_status() };
        ExtensionStatus::from(status)
    }

    pub fn get_status() -> ExtensionStatus {
        let status = unsafe { sentinel_extension_get_status() };
        ExtensionStatus::from(status)
    }

    pub fn install() -> Result<(), String> {
        let mut error_buffer = vec![0u8; ERROR_BUFFER_SIZE];
        let result = unsafe {
            sentinel_extension_install(
                error_buffer.as_mut_ptr() as *mut c_char,
                ERROR_BUFFER_SIZE as i32,
            )
        };

        if result == 0 {
            info!("Network Extension installed successfully");
            Ok(())
        } else {
            let error = get_error_message(&error_buffer)
                .unwrap_or_else(|| "Unknown error".to_string());
            Err(error)
        }
    }

    pub fn uninstall() -> Result<(), String> {
        let mut error_buffer = vec![0u8; ERROR_BUFFER_SIZE];
        let result = unsafe {
            sentinel_extension_uninstall(
                error_buffer.as_mut_ptr() as *mut c_char,
                ERROR_BUFFER_SIZE as i32,
            )
        };

        if result == 0 {
            info!("Network Extension uninstalled successfully");
            Ok(())
        } else {
            let error = get_error_message(&error_buffer)
                .unwrap_or_else(|| "Unknown error".to_string());
            Err(error)
        }
    }

    pub fn start_proxy(host: &str, port: u16, target_apps: &[String]) -> Result<(), String> {
        let host_cstr = CString::new(host).map_err(|e| e.to_string())?;
        
        let apps_json = serde_json::to_string(target_apps).map_err(|e| e.to_string())?;
        let apps_cstr = CString::new(apps_json).map_err(|e| e.to_string())?;
        
        let mut error_buffer = vec![0u8; ERROR_BUFFER_SIZE];
        let result = unsafe {
            sentinel_proxy_start(
                host_cstr.as_ptr(),
                port,
                apps_cstr.as_ptr(),
                error_buffer.as_mut_ptr() as *mut c_char,
                ERROR_BUFFER_SIZE as i32,
            )
        };

        if result == 0 {
            info!("Proxy started: {}:{}", host, port);
            Ok(())
        } else {
            let error = get_error_message(&error_buffer)
                .unwrap_or_else(|| "Unknown error".to_string());
            Err(error)
        }
    }

    pub fn stop_proxy() -> Result<(), String> {
        let mut error_buffer = vec![0u8; ERROR_BUFFER_SIZE];
        let result = unsafe {
            sentinel_proxy_stop(
                error_buffer.as_mut_ptr() as *mut c_char,
                ERROR_BUFFER_SIZE as i32,
            )
        };

        if result == 0 {
            info!("Proxy stopped");
            Ok(())
        } else {
            let error = get_error_message(&error_buffer)
                .unwrap_or_else(|| "Unknown error".to_string());
            Err(error)
        }
    }

    pub fn get_proxy_status() -> VPNStatus {
        let status = unsafe { sentinel_proxy_get_status() };
        VPNStatus::from(status)
    }
    
    pub fn is_available() -> bool {
        true
    }
}
*/
