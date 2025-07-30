use crate::services::performance::{PerformanceConfig, PerformanceMetrics, PerformanceOptimizer};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

/// 性能监控状态
static mut PERFORMANCE_OPTIMIZER: Option<Arc<PerformanceOptimizer>> = None;
static INIT_LOCK: std::sync::Once = std::sync::Once::new();

/// 初始化性能优化器
fn get_or_init_optimizer() -> Arc<PerformanceOptimizer> {
    unsafe {
        INIT_LOCK.call_once(|| {
            let config = PerformanceConfig::default();
            PERFORMANCE_OPTIMIZER = Some(Arc::new(PerformanceOptimizer::new(config)));
        });
        PERFORMANCE_OPTIMIZER.as_ref().unwrap().clone()
    }
}

/// 获取性能指标
#[tauri::command]
pub async fn get_performance_metrics() -> Result<PerformanceMetrics, String> {
    let optimizer = get_or_init_optimizer();
    Ok(optimizer.monitor().get_metrics())
}

/// 获取性能报告
#[tauri::command]
pub async fn get_performance_report() -> Result<String, String> {
    let optimizer = get_or_init_optimizer();
    Ok(optimizer.generate_detailed_report())
}

/// 获取优化建议
#[tauri::command]
pub async fn get_optimization_suggestions() -> Result<Vec<String>, String> {
    let optimizer = get_or_init_optimizer();
    Ok(optimizer.get_optimization_suggestions())
}

/// 启动性能监控
#[tauri::command]
pub async fn start_performance_monitoring() -> Result<(), String> {
    let optimizer = get_or_init_optimizer();
    optimizer.start().await;
    Ok(())
}

/// 更新性能配置
#[tauri::command]
pub async fn update_performance_config(config: PerformanceConfig) -> Result<(), String> {
    // 创建新的优化器实例
    let new_optimizer = Arc::new(PerformanceOptimizer::new(config));

    unsafe {
        PERFORMANCE_OPTIMIZER = Some(new_optimizer.clone());
    }

    // 启动新的监控
    new_optimizer.start().await;

    Ok(())
}

/// 获取当前性能配置
#[tauri::command]
pub async fn get_performance_config() -> Result<PerformanceConfig, String> {
    // 返回默认配置，实际应用中可以从数据库或配置文件读取
    Ok(PerformanceConfig::default())
}

/// 重置性能统计
#[tauri::command]
pub async fn reset_performance_stats() -> Result<(), String> {
    let optimizer = get_or_init_optimizer();
    optimizer.monitor().reset_stats();
    Ok(())
}

/// 记录操作性能
#[tauri::command]
pub async fn record_operation_timing(operation: String, duration_ms: f64) -> Result<(), String> {
    let optimizer = get_or_init_optimizer();
    let duration = std::time::Duration::from_millis(duration_ms as u64);
    optimizer.monitor().record_timing(&operation, duration);
    Ok(())
}

/// 记录请求
#[tauri::command]
pub async fn record_request() -> Result<(), String> {
    let optimizer = get_or_init_optimizer();
    optimizer.monitor().record_request();
    Ok(())
}

/// 记录错误
#[tauri::command]
pub async fn record_error() -> Result<(), String> {
    let optimizer = get_or_init_optimizer();
    optimizer.monitor().record_error();
    Ok(())
}

/// 性能监控中间件
pub struct PerformanceMiddleware;

impl PerformanceMiddleware {
    /// 包装异步操作以进行性能监控
    pub async fn wrap_operation<F, T, E>(operation_name: &str, future: F) -> Result<T, E>
    where
        F: std::future::Future<Output = Result<T, E>>,
    {
        let optimizer = get_or_init_optimizer();
        let start = std::time::Instant::now();
        optimizer.monitor().record_request();

        let result = future.await;

        match &result {
            Ok(_) => {
                let duration = start.elapsed();
                optimizer.monitor().record_timing(operation_name, duration);
            }
            Err(_) => {
                optimizer.monitor().record_error();
            }
        }

        result
    }
}

/// 性能监控装饰器宏
#[macro_export]
macro_rules! monitor_command_performance {
    ($operation:expr, $code:block) => {{
        use crate::commands::performance::PerformanceMiddleware;
        PerformanceMiddleware::wrap_operation($operation, async move $code).await
    }};
}
