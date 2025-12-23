//! Configuration management commands
//!
//! Provides commands for managing auto-approval configuration

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::generators::PluginAutoApprovalConfig;

/// 获取当前自动批准配置
#[tauri::command]
pub async fn get_auto_approval_config(
    // TODO: 从数据库或配置文件加载
) -> Result<PluginAutoApprovalConfig, String> {
    log::info!("Getting auto-approval config");
    
    // 暂时返回默认配置
    Ok(PluginAutoApprovalConfig::default())
}

/// 更新自动批准配置
#[tauri::command]
pub async fn update_auto_approval_config(
    config: PluginAutoApprovalConfig,
    // TODO: 保存到数据库或配置文件
) -> Result<(), String> {
    log::info!("Updating auto-approval config: enabled={}, thresholds={}/{}/{}", 
        config.enabled,
        config.auto_approve_threshold,
        config.require_review_threshold,
        config.auto_reject_threshold
    );
    
    // 验证配置
    if config.auto_approve_threshold < config.require_review_threshold {
        return Err("Auto-approve threshold must be >= require-review threshold".to_string());
    }
    
    if config.require_review_threshold < config.auto_reject_threshold {
        return Err("Require-review threshold must be >= auto-reject threshold".to_string());
    }
    
    if config.auto_approve_threshold > 100.0 || config.auto_reject_threshold < 0.0 {
        return Err("Thresholds must be between 0 and 100".to_string());
    }
    
    // TODO: 保存配置到数据库或文件
    log::info!("Auto-approval config updated successfully");
    
    Ok(())
}

/// 获取配置预设
#[tauri::command]
pub async fn get_config_presets() -> Result<Vec<ConfigPreset>, String> {
    log::info!("Getting config presets");
    
    Ok(vec![
        ConfigPreset {
            name: "Conservative (手动为主)".to_string(),
            description: "Most plugins require human review. Only extremely high-quality plugins (90+) are auto-approved.".to_string(),
            config: PluginAutoApprovalConfig {
                enabled: true,
                auto_approve_threshold: 90.0,
                require_review_threshold: 0.0,
                auto_reject_threshold: 30.0,
                auto_regenerate_on_low_quality: true,
                max_regeneration_attempts: 2,
                check_dangerous_patterns: true,
                dangerous_patterns: vec![
                    "eval(".to_string(),
                    "Function(".to_string(),
                    "fetch(".to_string(),
                    "XMLHttpRequest".to_string(),
                    "require(".to_string(),
                    "import(".to_string(),
                    "Deno.readFile".to_string(),
                    "Deno.writeFile".to_string(),
                ],
            },
        },
        ConfigPreset {
            name: "Balanced (半自动化，推荐)".to_string(),
            description: "High-quality plugins (80+) are auto-approved. Medium-quality (60-80) requires review.".to_string(),
            config: PluginAutoApprovalConfig::default(),
        },
        ConfigPreset {
            name: "Aggressive (自动为主)".to_string(),
            description: "Most plugins (70+) are auto-approved. Only low-quality plugins require review.".to_string(),
            config: PluginAutoApprovalConfig {
                enabled: true,
                auto_approve_threshold: 70.0,
                require_review_threshold: 50.0,
                auto_reject_threshold: 50.0,
                auto_regenerate_on_low_quality: true,
                max_regeneration_attempts: 3,
                check_dangerous_patterns: true,
                dangerous_patterns: vec![
                    "eval(".to_string(),
                    "Function(".to_string(),
                ],
            },
        },
        ConfigPreset {
            name: "Manual Only (全手动)".to_string(),
            description: "All plugins require human review. No automatic approval or rejection.".to_string(),
            config: PluginAutoApprovalConfig {
                enabled: false,
                auto_approve_threshold: 100.0,
                require_review_threshold: 0.0,
                auto_reject_threshold: 0.0,
                auto_regenerate_on_low_quality: false,
                max_regeneration_attempts: 0,
                check_dangerous_patterns: true,
                dangerous_patterns: vec![],
            },
        },
    ])
}

/// 测试配置效果
#[tauri::command]
pub async fn test_config_impact(
    config: PluginAutoApprovalConfig,
    test_scores: Vec<f32>,
) -> Result<TestResult, String> {
    log::info!("Testing config impact with {} sample scores", test_scores.len());
    
    use crate::generators::PluginAutoApprovalEngine;
    
    let engine = PluginAutoApprovalEngine::new(config);
    
    let mut results = vec![];
    for score in test_scores {
        let decision = engine.evaluate_plugin(
            score,
            "Passed",
            "// Test code without dangerous patterns",
            0,
        );
        results.push(decision);
    }
    
    let stats = engine.get_stats(&results);
    
    Ok(TestResult {
        total_plugins: stats.total,
        auto_approved: stats.auto_approved,
        require_review: stats.require_review,
        auto_rejected: stats.auto_rejected,
        automation_rate: stats.automation_rate(),
        approval_rate: stats.approval_rate(),
    })
}

/// 配置预设
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigPreset {
    pub name: String,
    pub description: String,
    pub config: PluginAutoApprovalConfig,
}

/// 配置测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub total_plugins: usize,
    pub auto_approved: usize,
    pub require_review: usize,
    pub auto_rejected: usize,
    pub automation_rate: f64,
    pub approval_rate: f64,
}

