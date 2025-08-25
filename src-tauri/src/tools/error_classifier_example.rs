//! 错误分类器使用示例
//! 
//! 展示如何使用智能错误分类系统来处理MCP工具错误

use super::error_classifier::{ErrorClassifier, ErrorContext, ErrorCategory, RecoveryStrategy, RecoveryExecutor};
use super::error_config_loader::{ErrorConfigLoader, ErrorConfig};
use std::collections::HashMap;
use tracing::info;

/// 演示基本的错误分类
pub fn demo_basic_error_classification() {
    println!("=== 基本错误分类演示 ===");
    
    let mut classifier = ErrorClassifier::new();
    
    // 模拟浏览器关闭错误
    let browser_error = ErrorContext {
        error_message: "page.goto: Target page, context or browser has been closed".to_string(),
        error_code: Some(-32603),
        error_type: None,
        tool_name: "browser_navigate".to_string(),
        connection_name: "playwright".to_string(),
        retry_count: 0,
        metadata: HashMap::new(),
    };
    
    let (category, strategy) = classifier.classify_error(&browser_error);
    println!("浏览器错误分类: {:?}", category);
    println!("恢复策略: {:?}", strategy);
    
    // 模拟传输错误
    let transport_error = ErrorContext {
        error_message: "Transport closed unexpectedly".to_string(),
        error_code: None,
        error_type: None,
        tool_name: "network_scan".to_string(),
        connection_name: "network_tools".to_string(),
        retry_count: 1,
        metadata: HashMap::new(),
    };
    
    let (category, strategy) = classifier.classify_error(&transport_error);
    println!("传输错误分类: {:?}", category);
    println!("恢复策略: {:?}", strategy);
    
    // 计算重连延迟
    let delay = RecoveryExecutor::calculate_delay(&strategy, transport_error.retry_count);
    println!("建议延迟: {:?}ms", delay);
}

/// 演示从配置文件加载自定义规则
pub fn demo_config_based_classification() {
    println!("\n=== 配置文件错误分类演示 ===");
    
    // 创建默认配置
    let config = ErrorConfigLoader::create_default_config();
    
    // 从配置创建分类器
    let mut classifier = ErrorConfigLoader::create_classifier_from_config(&config)
        .expect("Failed to create classifier from config");
    
    // 测试浏览器错误
    let browser_error = ErrorContext {
        error_message: "Browser has been closed".to_string(),
        error_code: Some(-32603),
        error_type: None,
        tool_name: "browser_click".to_string(),
        connection_name: "playwright".to_string(),
        retry_count: 0,
        metadata: HashMap::new(),
    };
    
    let (category, strategy) = classifier.classify_error(&browser_error);
    println!("配置驱动的分类结果: {:?} -> {:?}", category, strategy);
}

/// 演示智能重试逻辑
pub fn demo_smart_retry_logic() {
    println!("\n=== 智能重试逻辑演示 ===");
    
    let strategy = RecoveryStrategy::ExponentialBackoff { 
        initial_delay_ms: 1000, 
        max_delay_ms: 10000, 
        multiplier: 2.0 
    };
    
    println!("指数退避策略演示:");
    for retry_count in 0..6 {
        let delay = RecoveryExecutor::calculate_delay(&strategy, retry_count);
        let should_retry = RecoveryExecutor::should_retry(&strategy, retry_count, 5);
        
        println!("重试 {}: 延迟 {:?}ms, 是否继续重试: {}", 
                retry_count, delay, should_retry);
        
        if !should_retry {
            break;
        }
    }
}

/// 演示自定义错误规则
pub fn demo_custom_error_rules() {
    println!("\n=== 自定义错误规则演示 ===");
    
    let mut custom_config = ErrorConfig {
        global: super::error_config_loader::GlobalConfig {
            max_retries: 5,
            default_timeout_ms: 45000,
        },
        rules: vec![
            super::error_config_loader::ConfigRule {
                name: "Custom Database Error".to_string(),
                priority: 120,
                error_code: None,
                message_pattern: Some(r"(?i)database.*connection.*failed".to_string()),
                error_type: None,
                category: "Connection".to_string(),
                recovery_strategy: super::error_config_loader::ConfigRecoveryStrategy::DelayedReconnect { delay_ms: 5000 },
            },
        ],
        tool_specific: HashMap::new(),
    };
    
    // 添加工具特定配置
    let mut playwright_config = super::error_config_loader::ToolSpecificConfig {
        max_retries: Some(8),
        rules: vec![
            super::error_config_loader::ConfigRule {
                name: "Playwright Specific Error".to_string(),
                priority: 150,
                error_code: None,
                message_pattern: Some(r"(?i)playwright.*timeout".to_string()),
                error_type: None,
                category: "Timeout".to_string(),
                recovery_strategy: super::error_config_loader::ConfigRecoveryStrategy::ExponentialBackoff {
                    initial_delay_ms: 2000,
                    max_delay_ms: 20000,
                    multiplier: 1.5,
                },
            },
        ],
    };
    
    custom_config.tool_specific.insert("playwright".to_string(), playwright_config);
    
    let mut classifier = ErrorConfigLoader::create_classifier_from_config(&custom_config)
        .expect("Failed to create custom classifier");
    
    // 测试自定义规则
    let custom_error = ErrorContext {
        error_message: "Database connection failed due to network issues".to_string(),
        error_code: None,
        error_type: None,
        tool_name: "db_scan".to_string(),
        connection_name: "database".to_string(),
        retry_count: 0,
        metadata: HashMap::new(),
    };
    
    let (category, strategy) = classifier.classify_error(&custom_error);
    println!("自定义规则分类结果: {:?} -> {:?}", category, strategy);
    
    let max_retries = ErrorConfigLoader::get_tool_max_retries(&custom_config, "playwright");
    println!("Playwright工具最大重试次数: {}", max_retries);
}

/// 主演示函数
pub fn run_all_demos() {
    println!("🔧 智能错误分类器演示\n");
    
    demo_basic_error_classification();
    demo_config_based_classification();
    demo_smart_retry_logic();
    demo_custom_error_rules();
    
    println!("\n✅ 所有演示完成！");
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_classification() {
        demo_basic_error_classification();
    }
    
    #[test]
    fn test_config_classification() {
        demo_config_based_classification();
    }
    
    #[test]
    fn test_retry_logic() {
        demo_smart_retry_logic();
    }
    
    #[test]
    fn test_custom_rules() {
        demo_custom_error_rules();
    }
}
