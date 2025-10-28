//! Sentinel AI Main Application
//! 
//! 主应用程序入口

// 重新导出所有依赖的crate
pub use sentinel_core as core;
pub use sentinel_models as models;

// 简化的应用程序入口，用于测试workspace结构
pub fn run() {
    println!("Sentinel AI - Modular Architecture Test");
    println!("Core module loaded: {:?}", core::ExecutionStatus::Pending);
    println!("Models module available");
    
    // 这里将来会包含完整的Tauri应用程序逻辑
    // 目前只是测试模块化结构是否正常工作
}