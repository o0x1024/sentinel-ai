//! Prompt相关的Tauri命令
//! 
//! 为前端提供prompt管理和优化的API接口

use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;
use anyhow::Result;

use sentinel_prompt::{ABTest, CustomTemplate, TemplateType, TestScenario, UsageStats};
use crate::services::{PromptBuildRequest, PromptService};
use sentinel_prompt::{
    CreateTestRequest, OptimizationResult, OptimizationSuggestion,
    PerformanceRecord, 
    PromptConfig, ReportType
};
use crate::services::{
    PromptServiceConfig, PromptSession, PromptBuildResponse, ServiceStats, OptimizationRequest
};

/// Prompt服务状态
pub type PromptServiceState = Arc<RwLock<Option<PromptService>>>;

/// 初始化prompt服务
#[tauri::command]
pub async fn init_prompt_service(
    state: State<'_, PromptServiceState>,
    config: Option<PromptServiceConfig>,
) -> Result<String, String> {
    let config = config.unwrap_or_default();
    
    match PromptService::new(config).await {
        Ok(service) => {
            let mut state_guard = state.write().await;
            *state_guard = Some(service);
            Ok("Prompt service initialized successfully".to_string())
        },
        Err(e) => Err(format!("Failed to initialize prompt service: {}", e)),
    }
}

/// 获取服务状态
#[tauri::command]
pub async fn get_prompt_service_status(
    state: State<'_, PromptServiceState>,
) -> Result<ServiceStats, String> {
    let mut state_guard = state.write().await;
    match state_guard.as_mut() {
        Some(service) => {
            service.get_service_stats().await
                .map_err(|e| format!("Failed to get service stats: {}", e))
        },
        None => Err("Prompt service not initialized".to_string()),
    }
}

/// 创建会话
#[tauri::command]
pub async fn create_prompt_session(
    state: State<'_, PromptServiceState>,
    user_id: Option<String>,
    config_id: Option<String>,
) -> Result<String, String> {
    let state_guard = state.read().await;
    match state_guard.as_ref() {
        Some(service) => {
            service.create_session(user_id, config_id).await
                .map_err(|e| format!("Failed to create session: {}", e))
        },
        None => Err("Prompt service not initialized".to_string()),
    }
}

/// 获取会话信息
#[tauri::command]
pub async fn get_prompt_session(
    state: State<'_, PromptServiceState>,
    session_id: String,
) -> Result<PromptSession, String> {
    let state_guard = state.read().await;
    match state_guard.as_ref() {
        Some(service) => {
            service.get_session(&session_id).await
                .map_err(|e| format!("Failed to get session: {}", e))
        },
        None => Err("Prompt service not initialized".to_string()),
    }
}

/// 关闭会话
#[tauri::command]
pub async fn close_prompt_session(
    state: State<'_, PromptServiceState>,
    session_id: String,
) -> Result<String, String> {
    let state_guard = state.read().await;
    match state_guard.as_ref() {
        Some(service) => {
            service.close_session(&session_id).await
                .map_err(|e| format!("Failed to close session: {}", e))?;
            Ok("Session closed successfully".to_string())
        },
        None => Err("Prompt service not initialized".to_string()),
    }
}

/// 构建prompt
#[tauri::command]
pub async fn build_prompt(
    state: State<'_, PromptServiceState>,
    request: PromptBuildRequest,
) -> Result<PromptBuildResponse, String> {
    let state_guard = state.read().await;
    match state_guard.as_ref() {
        Some(service) => {
            service.build_prompt(request).await
                .map_err(|e| format!("Failed to build prompt: {}", e))
        },
        None => Err("Prompt service not initialized".to_string()),
    }
}

/// 获取prompt配置
#[tauri::command]
pub async fn get_prompt_config(
    state: State<'_, PromptServiceState>,
    config_id: String,
) -> Result<PromptConfig, String> {
    let state_guard = state.read().await;
    match state_guard.as_ref() {
        Some(service) => {
            service.config_manager().get_config(&config_id).await
                .map_err(|e| format!("Failed to get config: {}", e))
        },
        None => Err("Prompt service not initialized".to_string()),
    }
}

/// 保存prompt配置
#[tauri::command]
pub async fn save_prompt_config(
    state: State<'_, PromptServiceState>,
    config_id: String,
    config: PromptConfig,
) -> Result<String, String> {
    let state_guard = state.read().await;
    match state_guard.as_ref() {
        Some(service) => {
            service.config_manager().save_config(&config_id, &config).await
                .map_err(|e| format!("Failed to save config: {}", e))?;
            Ok("Configuration saved successfully".to_string())
        },
        None => Err("Prompt service not initialized".to_string()),
    }
}

/// 列出所有配置
#[tauri::command]
pub async fn list_prompt_configs(
    state: State<'_, PromptServiceState>,
) -> Result<Vec<String>, String> {
    let state_guard = state.read().await;
    match state_guard.as_ref() {
        Some(service) => {
            service.config_manager().list_configs().await
                .map_err(|e| format!("Failed to list configs: {}", e))
        },
        None => Err("Prompt service not initialized".to_string()),
    }
}

/// 优化配置
#[tauri::command]
pub async fn optimize_prompt_config(
    state: State<'_, PromptServiceState>,
    request: OptimizationRequest,
) -> Result<OptimizationResult, String> {
    let state_guard = state.read().await;
    match state_guard.as_ref() {
        Some(service) => {
            service.optimize_config(request).await
                .map_err(|e| format!("Failed to optimize config: {}", e))
        },
        None => Err("Prompt service not initialized".to_string()),
    }
}

/// 获取Prompt优化建议
#[tauri::command]
pub async fn get_prompt_optimization_suggestions(
    state: State<'_, PromptServiceState>,
    config_id: String,
) -> Result<Vec<OptimizationSuggestion>, String> {
    let state_guard = state.read().await;
    match state_guard.as_ref() {
        Some(service) => {
            service.get_optimization_suggestions(&config_id).await
                .map_err(|e| format!("Failed to get optimization suggestions: {}", e))
        },
        None => Err("Prompt service not initialized".to_string()),
    }
}

/// 创建A/B测试
#[tauri::command]
pub async fn create_ab_test(
    state: State<'_, PromptServiceState>,
    request: CreateTestRequest,
) -> Result<ABTest, String> {
    let state_guard = state.read().await;
    match state_guard.as_ref() {
        Some(service) => {
            service.create_ab_test(request).await
                .map_err(|e| format!("Failed to create A/B test: {}", e))
        },
        None => Err("Prompt service not initialized".to_string()),
    }
}

/// 记录性能数据
#[tauri::command]
pub async fn record_performance_data(
    state: State<'_, PromptServiceState>,
    record: PerformanceRecord,
) -> Result<String, String> {
    let state_guard = state.read().await;
    match state_guard.as_ref() {
        Some(service) => {
            service.record_performance_data(record).await
                .map_err(|e| format!("Failed to record performance data: {}", e))?;
            Ok("Performance data recorded successfully".to_string())
        },
        None => Err("Prompt service not initialized".to_string()),
    }
}

/// 获取模板列表
#[tauri::command]
pub async fn list_prompt_templates(
    state: State<'_, PromptServiceState>,
    _category: Option<String>,
    _tags: Option<Vec<String>>,
) -> Result<Vec<String>, String> {
    let state_guard = state.read().await;
    match state_guard.as_ref() {
        Some(service) => {
            service.template_manager().list_templates().await
                .map_err(|e| format!("Failed to list templates: {}", e))
        },
        None => Err("Prompt service not initialized".to_string()),
    }
}

/// 获取模板内容
#[tauri::command]
pub async fn get_prompt_template(
    state: State<'_, PromptServiceState>,
    template_id: String,
) -> Result<String, String> {
    let state_guard = state.read().await;
    match state_guard.as_ref() {
        Some(service) => {
            let template = service.template_manager().load_template(&template_id).await
                .map_err(|e| format!("Failed to load template: {}", e))?;
            Ok(template.content)
        },
        None => Err("Prompt service not initialized".to_string()),
    }
}

/// 保存模板
#[tauri::command]
pub async fn save_prompt_template(
    state: State<'_, PromptServiceState>,
    template_id: String,
    content: String,
    metadata: Option<HashMap<String, serde_json::Value>>,
) -> Result<String, String> {
    let mut state_guard = state.write().await;
    match state_guard.as_mut() {
        Some(service) => {

            let template = CustomTemplate {
                id: template_id.clone(),
                name: template_id.clone(),
                content,
                template_type: TemplateType::Custom,
                description: metadata.as_ref()
                    .and_then(|m| m.get("description"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("").to_string(),
                creator: "user".to_string(),
                created_at: chrono::Utc::now(),
                version: "1.0.0".to_string(),
                tags: Vec::new(),
                usage_stats: UsageStats::default(),
                variables: Vec::new(),
                metadata: metadata.unwrap_or_default().into_iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_string())))
                    .collect(),
                category: None,
                target_architecture: None,
                is_system: false,
                priority: 0,
            };
            service.template_manager_mut().save_template(&template_id, &template).await
                .map_err(|e| format!("Failed to save template: {}", e))?;
            Ok("Template saved successfully".to_string())
        },
        None => Err("Prompt service not initialized".to_string()),
    }
}

/// 验证prompt模板
#[tauri::command]
pub async fn validate_prompt_template(
    state: State<'_, PromptServiceState>,
    template_content: String,
) -> Result<bool, String> {
    let state_guard = state.read().await;
    match state_guard.as_ref() {
        Some(_service) => {
            // 简单的模板验证逻辑
            if template_content.trim().is_empty() {
                return Err("Template content cannot be empty".to_string());
            }
            if template_content.len() > 10000 {
                return Err("Template content too long (max 10000 characters)".to_string());
            }
            Ok(true)
        },
        None => Err("Prompt service not initialized".to_string()),
    }
}

/// 搜索模板
#[tauri::command]
pub async fn search_prompt_templates(
    state: State<'_, PromptServiceState>,
    query: String,
    filters: Option<HashMap<String, serde_json::Value>>,
) -> Result<Vec<serde_json::Value>, String> {
    let state_guard = state.read().await;
    match state_guard.as_ref() {
        Some(service) => {
            let template_type = filters.as_ref()
                .and_then(|f| f.get("template_type"))
                .and_then(|v| v.as_str())
                .and_then(|t| {
                    match t {
                        "Planner" => Some(TemplateType::Planner),
                        "Executor" => Some(TemplateType::Executor),
                        "Replanner" => Some(TemplateType::Replanner),
                        "ReportGenerator" => Some(TemplateType::ReportGenerator),
                        "Custom" => Some(TemplateType::Custom),
                        _ => None,
                    }
                });
            
            match service.template_manager().search_templates(&query, template_type).await {
                Ok(results) => {
                    let json_results: Vec<serde_json::Value> = results.into_iter()
                        .map(|r| serde_json::json!({
                            "template_id": r.template_id,
                            "name": r.name,
                            "description": r.description,
                            "match_score": r.match_score,
                            "template_type": r.template_type,
                            "tags": r.tags
                        }))
                        .collect();
                    Ok(json_results)
                },
                Err(e) => Err(format!("Failed to search templates: {}", e)),
            }
        },
        None => Err("Prompt service not initialized".to_string()),
    }
}

/// 导出配置
#[tauri::command]
pub async fn export_prompt_config(
    state: State<'_, PromptServiceState>,
    config_id: String,
    format: Option<String>, // "json", "yaml", "toml"
) -> Result<String, String> {
    let state_guard = state.read().await;
    match state_guard.as_ref() {
        Some(service) => {
            match service.config_manager().get_config(&config_id).await {
                Ok(config) => {
                    let format = format.unwrap_or_else(|| "json".to_string());
                    match format.as_str() {
                        "json" => {
                            match serde_json::to_string_pretty(&config) {
                                Ok(json_str) => Ok(json_str),
                                Err(e) => Err(format!("Failed to serialize config to JSON: {}", e)),
                            }
                        },
                        "yaml" => {
                            match serde_yaml::to_string(&config) {
                                Ok(yaml_str) => Ok(yaml_str),
                                Err(e) => Err(format!("Failed to serialize config to YAML: {}", e)),
                            }
                        },
                        _ => Err(format!("Unsupported format: {}", format)),
                    }
                },
                Err(e) => Err(format!("Failed to get config: {}", e)),
            }
        },
        None => Err("Prompt service not initialized".to_string()),
    }
}

/// 导入配置
#[tauri::command]
pub async fn import_prompt_config(
    state: State<'_, PromptServiceState>,
    config_data: String,
    format: Option<String>, // "json", "yaml", "toml"
    config_id: Option<String>,
) -> Result<String, String> {
    let state_guard = state.read().await;
    match state_guard.as_ref() {
        Some(service) => {
            let format = format.unwrap_or_else(|| "json".to_string());
            let config: PromptConfig = match format.as_str() {
                "json" => {
                    match serde_json::from_str(&config_data) {
                        Ok(config) => config,
                        Err(e) => return Err(format!("Failed to parse JSON config: {}", e)),
                    }
                },
                "yaml" => {
                    match serde_yaml::from_str(&config_data) {
                        Ok(config) => config,
                        Err(e) => return Err(format!("Failed to parse YAML config: {}", e)),
                    }
                },
                _ => return Err(format!("Unsupported format: {}", format)),
            };
            
            let config_id = config_id.unwrap_or_else(|| format!("imported_{}", chrono::Utc::now().timestamp()));
             match service.config_manager().save_config(&config_id, &config).await {
                Ok(_) => Ok(config_id),
                Err(e) => Err(format!("Failed to save imported config: {}", e)),
            }
        },
        None => Err("Prompt service not initialized".to_string()),
    }
}

/// 获取A/B测试列表
#[tauri::command]
pub async fn list_ab_tests(
    state: State<'_, PromptServiceState>,
    status_filter: Option<String>, // "active", "completed", "paused"
) -> Result<Vec<serde_json::Value>, String> {
    let state_guard = state.read().await;
    match state_guard.as_ref() {
        Some(service) => {
            match service.ab_test_manager().list_tests().await {
                Ok(tests) => {
                    let filtered_tests: Vec<serde_json::Value> = tests.into_iter()
                        .filter(|test| {
                            if let Some(ref filter) = status_filter {
                                format!("{:?}", test.status).to_lowercase() == filter.to_lowercase()
                            } else {
                                true
                            }
                        })
                        .map(|test| serde_json::json!({
                            "test_id": test.test_id,
                            "name": test.name,
                            "status": test.status,
                            "created_at": test.created_at,
                            "variants": test.variants.len(),
                            "traffic_allocation": test.traffic_allocation
                        }))
                        .collect();
                    Ok(filtered_tests)
                },
                Err(e) => Err(format!("Failed to list A/B tests: {}", e)),
            }
        },
        None => Err("Prompt service not initialized".to_string()),
    }
}

/// 获取A/B测试结果
#[tauri::command]
pub async fn get_ab_test_results(
    state: State<'_, PromptServiceState>,
    test_id: String,
) -> Result<serde_json::Value, String> {
    let state_guard = state.read().await;
    match state_guard.as_ref() {
        Some(service) => {
            match service.ab_test_manager().get_test_results(&test_id).await {
                Ok(results) => {
                    let json_results = serde_json::json!({
                        "test_id": results.test_id,
                        "test_name": results.test_name,
                        "status": results.status,
                        "started_at": results.started_at,
                        "ended_at": results.ended_at,
                        "variant_results": results.variant_results,
                        "winning_variant": results.winning_variant,
                        "statistical_significance": results.statistical_significance,
                        "total_samples": results.total_samples,
                        "confidence_level": results.confidence_level,
                        "effect_size": results.effect_size,
                        "recommendations": results.recommendations,
                        "summary": results.summary
                    });
                    Ok(json_results)
                },
                Err(e) => Err(format!("Failed to get A/B test results: {}", e)),
            }
        },
        None => Err("Prompt service not initialized".to_string()),
    }
}

/// 停止A/B测试
#[tauri::command]
pub async fn stop_ab_test(
    state: State<'_, PromptServiceState>,
    test_id: String,
) -> Result<String, String> {
    let state_guard = state.read().await;
    match state_guard.as_ref() {
        Some(service) => {
            match service.ab_test_manager().stop_test(&test_id).await {
                Ok(_) => Ok(format!("A/B test {} stopped successfully", test_id)),
                Err(e) => Err(format!("Failed to stop A/B test: {}", e)),
            }
        },
        None => Err("Prompt service not initialized".to_string()),
    }
}

/// 获取性能分析报告
#[tauri::command]
pub async fn get_performance_analysis(
    state: State<'_, PromptServiceState>,
    config_id: String,
    time_range: Option<(String, String)>, // (start_date, end_date)
) -> Result<serde_json::Value, String> {
    let state_guard = state.read().await;
    match state_guard.as_ref() {
        Some(service) => {
            // 转换时间范围
            let parsed_time_range = if let Some((start, end)) = time_range {
                match (chrono::DateTime::parse_from_rfc3339(&start), chrono::DateTime::parse_from_rfc3339(&end)) {
                    (Ok(start_dt), Ok(end_dt)) => Some((start_dt.with_timezone(&chrono::Utc), end_dt.with_timezone(&chrono::Utc))),
                    _ => return Err("Invalid time range format. Use RFC3339 format.".to_string()),
                }
            } else {
                None
            };
            
            match service.get_performance_analysis(&config_id, parsed_time_range).await {
                Ok(analysis) => {
                    let json_analysis = serde_json::json!({
                        "config_id": analysis.config_id,
                        "time_range": analysis.time_range,
                        "overall_stats": analysis.overall_stats,
                        "trend_analysis": analysis.trend_analysis,
                        "bottlenecks": analysis.bottlenecks,
                        "recommendations": analysis.recommendations,
                        "analysis_id": analysis.analysis_id
                    });
                    Ok(json_analysis)
                },
                Err(e) => Err(format!("Failed to get performance analysis: {}", e)),
            }
        },
        None => Err("Prompt service not initialized".to_string()),
    }
}

/// 批量测试配置
#[tauri::command]
pub async fn batch_test_configs(
    state: State<'_, PromptServiceState>,
    config_ids: Vec<String>,
    scenarios_json: Vec<serde_json::Value>,
) -> Result<Vec<serde_json::Value>, String> {
    let state_guard = state.read().await;
    match state_guard.as_ref() {
        Some(service) => {
            let scenarios: Vec<TestScenario> = scenarios_json.into_iter()
                .filter_map(|v| serde_json::from_value(v).ok())
                .collect();
            
            match service.batch_test_configs(config_ids, scenarios).await {
                Ok(batch_result) => {
                    let json_result = serde_json::json!({
                        "test_id": batch_result.test_id,
                        "tested_configs": batch_result.tested_configs,
                        "scenarios": batch_result.scenarios,
                        "config_results": batch_result.config_results,
                        "best_config": batch_result.best_config,
                        "started_at": batch_result.started_at,
                        "completed_at": batch_result.completed_at,
                        "summary": batch_result.summary
                    });
                    Ok(vec![json_result])
                },
                Err(e) => Err(format!("Failed to batch test configs: {}", e)),
            }
        },
        None => Err("Prompt service not initialized".to_string()),
    }
}

/// 生成配置报告
#[tauri::command]
pub async fn generate_config_report(
    state: State<'_, PromptServiceState>,
    config_id: String,
    report_type: String,
) -> Result<serde_json::Value, String> {
    let state_guard = state.read().await;
    match state_guard.as_ref() {
        Some(service) => {
            let parsed_report_type = match report_type.to_lowercase().as_str() {
                "performance" => ReportType::Performance,
                "optimization" => ReportType::Optimization,
                "comparison" => ReportType::Comparison,
                "trend" => ReportType::Trend,
                "detailed" => ReportType::Detailed,
                "summary" => ReportType::Summary,
                _ => ReportType::Summary, // 默认为摘要报告
            };
            
            match service.generate_config_report(&config_id, parsed_report_type).await {
                Ok(report) => {
                    let json_report = serde_json::json!({
                        "report_id": report.report_id,
                        "config_id": report.config_id,
                        "report_type": report.report_type,
                        "title": report.title,
                        "generated_at": report.generated_at,
                        "time_range": report.time_range,
                        "executive_summary": report.executive_summary,
                        "key_metrics": report.key_metrics,
                        "detailed_analysis": report.detailed_analysis,
                        "recommendations": report.recommendations,
                        "attachments": report.attachments
                    });
                    Ok(json_report)
                },
                Err(e) => Err(format!("Failed to generate config report: {}", e)),
            }
        },
        None => Err("Prompt service not initialized".to_string()),
    }
}
