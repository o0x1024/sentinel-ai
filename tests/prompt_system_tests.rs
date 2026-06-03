//! Prompt 高级定制系统的集成测试
//! 
//! 这个测试文件展示了如何测试 Prompt 定制系统的各个组件

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::Utc;

use sentinel_ai::engines::{
    prompt_config::*,
    prompt_builder::*,
    prompt_template_manager::*,
    prompt_ab_test_manager::*,
    prompt_optimizer::*,
};
use sentinel_ai::services::prompt_service::*;

/// 创建测试用的 Prompt 服务配置
fn create_test_config() -> PromptServiceConfig {
    PromptServiceConfig {
        config_dir: "./test_data/config/prompts".to_string(),
        template_dir: "./test_data/templates/prompts".to_string(),
        cache_size: 100,
        enable_hot_reload: false, // 测试时禁用热重载
        enable_ab_testing: true,
        enable_auto_optimization: true,
        validation: ValidationSettings {
            max_length: 4096,
            min_length: 10,
            required_variables: vec!["user_query".to_string()],
            forbidden_patterns: vec!["<script>".to_string()],
            custom_rules: HashMap::new(),
        },
    }
}

/// 创建测试用的目标信息
fn create_test_target_info() -> TargetInfo {
    TargetInfo {
        url: Some("https://example.com".to_string()),
        ip: Some("192.168.1.100".to_string()),
        domain: Some("example.com".to_string()),
        port: Some(443),
        service: Some("https".to_string()),
        version: Some("nginx/1.18.0".to_string()),
        os: Some("Linux".to_string()),
        auth_info: Some(AuthInfo {
            username: Some("admin".to_string()),
            password: None,
            token: Some("test_token".to_string()),
            api_key: None,
        }),
        custom_fields: HashMap::new(),
    }
}

/// 创建测试用的工具信息
fn create_test_tools() -> Vec<ToolInfo> {
    vec![
        ToolInfo {
            name: "nmap".to_string(),
            description: "Network exploration tool and security scanner".to_string(),
            version: Some("7.80".to_string()),
            capabilities: vec!["port_scan".to_string(), "service_detection".to_string()],
            requirements: Some(ResourceRequirements {
                cpu_cores: 1,
                memory_mb: 512,
                disk_space_mb: 100,
                network_access: true,
                elevated_privileges: false,
            }),
        },
        ToolInfo {
            name: "sqlmap".to_string(),
            description: "Automatic SQL injection and database takeover tool".to_string(),
            version: Some("1.5.2".to_string()),
            capabilities: vec!["sql_injection".to_string(), "database_enumeration".to_string()],
            requirements: Some(ResourceRequirements {
                cpu_cores: 1,
                memory_mb: 256,
                disk_space_mb: 50,
                network_access: true,
                elevated_privileges: false,
            }),
        },
    ]
}

#[tokio::test]
async fn test_prompt_service_initialization() {
    let config = create_test_config();
    let service = PromptService::new(config).await;
    
    assert!(service.is_ok(), "Prompt service should initialize successfully");
    
    let service = service.unwrap();
    let status = service.get_status().await;
    
    assert!(status.is_initialized, "Service should be initialized");
    assert_eq!(status.active_sessions, 0, "Should have no active sessions initially");
}

#[tokio::test]
async fn test_session_management() {
    let config = create_test_config();
    let service = PromptService::new(config).await.unwrap();
    
    // 创建会话
    let session_id = "test_session_1".to_string();
    let result = service.create_session(
        session_id.clone(),
        Some("security_analyst".to_string()),
        Some("web_security".to_string()),
    ).await;
    
    assert!(result.is_ok(), "Session creation should succeed");
    
    // 获取会话
    let session = service.get_session(&session_id).await;
    assert!(session.is_ok(), "Should be able to get session");
    
    let session = session.unwrap();
    assert_eq!(session.id, session_id);
    assert_eq!(session.agent_profile, Some("security_analyst".to_string()));
    assert_eq!(session.domain_template, Some("web_security".to_string()));
    
    // 关闭会话
    let result = service.close_session(&session_id).await;
    assert!(result.is_ok(), "Session closure should succeed");
    
    // 验证会话已关闭
    let session = service.get_session(&session_id).await;
    assert!(session.is_err(), "Session should not exist after closure");
}

#[tokio::test]
async fn test_prompt_building() {
    let config = create_test_config();
    let service = PromptService::new(config).await.unwrap();
    
    // 创建会话
    let session_id = "test_session_2".to_string();
    service.create_session(
        session_id.clone(),
        Some("security_analyst".to_string()),
        Some("web_security".to_string()),
    ).await.unwrap();
    
    // 创建构建请求
    let request = PromptBuildRequest {
        build_type: PromptBuildType::Planner,
        context: PromptBuildContext {
            user_query: "对目标网站进行全面的安全测试".to_string(),
            target_info: Some(create_test_target_info()),
            available_tools: Some(create_test_tools()),
            execution_context: None,
            history: None,
            custom_variables: HashMap::new(),
        },
        template_override: None,
        validation_settings: None,
    };
    
    // 构建 Prompt
    let response = service.build_prompt(session_id.clone(), request).await;
    assert!(response.is_ok(), "Prompt building should succeed");
    
    let response = response.unwrap();
    assert!(!response.prompt.is_empty(), "Generated prompt should not be empty");
    assert!(response.validation_result.is_valid, "Prompt should be valid");
    assert!(response.build_stats.build_time.as_millis() > 0, "Build time should be recorded");
    
    println!("Generated prompt: {}", response.prompt);
    println!("Build stats: {:?}", response.build_stats);
}

#[tokio::test]
async fn test_template_management() {
    let config = TemplateManagerConfig {
        template_dir: "./test_data/templates".to_string(),
        cache_size: 50,
        enable_hot_reload: false,
        validation_rules: ValidationRules {
            max_template_size: 10240,
            allowed_extensions: vec![".hbs".to_string(), ".mustache".to_string()],
            required_metadata: vec!["name".to_string(), "version".to_string()],
        },
    };
    
    let manager = PromptTemplateManager::new(config).await;
    assert!(manager.is_ok(), "Template manager should initialize successfully");
    
    let manager = manager.unwrap();
    
    // 创建自定义模板
    let template = CustomTemplate {
        name: "test_template".to_string(),
        description: "Test template for unit testing".to_string(),
        template_type: TemplateType::Planner,
        content: "You are a {{agent_type}} testing {{target}}. Please {{action}}.".to_string(),
        variables: vec![
            "agent_type".to_string(),
            "target".to_string(),
            "action".to_string(),
        ],
        tags: vec!["test".to_string(), "unit_test".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    
    // 保存模板
    let result = manager.save_template("test_template", template.clone()).await;
    assert!(result.is_ok(), "Template saving should succeed");
    
    // 加载模板
    let loaded_template = manager.load_template("test_template").await;
    assert!(loaded_template.is_ok(), "Template loading should succeed");
    
    let loaded_template = loaded_template.unwrap();
    assert_eq!(loaded_template.name, template.name);
    assert_eq!(loaded_template.content, template.content);
    
    // 搜索模板
    let search_results = manager.search_templates("test", None).await;
    assert!(search_results.is_ok(), "Template search should succeed");
    
    let search_results = search_results.unwrap();
    assert!(!search_results.is_empty(), "Should find at least one template");
    assert!(search_results.iter().any(|r| r.template_id == "test_template"));
}

#[tokio::test]
async fn test_ab_testing() {
    let config = ABTestManagerConfig {
        storage_dir: "./test_data/ab_tests".to_string(),
        default_confidence_level: 0.95,
        min_sample_size: 100,
        max_test_duration: 604800, // 7 days
    };
    
    let manager = PromptABTestManager::new(config).await;
    assert!(manager.is_ok(), "AB test manager should initialize successfully");
    
    let manager = manager.unwrap();
    
    // 创建测试变体
    let variant_a = TestVariant {
        id: "variant_a".to_string(),
        name: "Control Group".to_string(),
        description: "Original template".to_string(),
        template_config: PromptConfig::default(),
        is_control: true,
    };
    
    let variant_b = TestVariant {
        id: "variant_b".to_string(),
        name: "Experimental Group".to_string(),
        description: "Optimized template".to_string(),
        template_config: PromptConfig::default(),
        is_control: false,
    };
    
    // 创建 A/B 测试
    let test = ABTest {
        id: "test_planner_optimization".to_string(),
        name: "Planner Template Optimization".to_string(),
        description: "Testing optimized planner template".to_string(),
        variants: vec![variant_a, variant_b],
        traffic_allocation: TrafficAllocation::Even,
        evaluation_metrics: vec![
            EvaluationMetric::SuccessRate,
            EvaluationMetric::ExecutionTime,
        ],
        conditions: TestConditions {
            min_sample_size: 100,
            max_duration: 86400, // 1 day for test
            confidence_level: 0.95,
            early_stopping: true,
        },
        status: TestStatus::Draft,
        created_at: Utc::now(),
        started_at: None,
        ended_at: None,
    };
    
    // 创建测试
    let result = manager.create_test(test.clone()).await;
    assert!(result.is_ok(), "AB test creation should succeed");
    
    // 启动测试
    let result = manager.start_test(&test.id).await;
    assert!(result.is_ok(), "AB test start should succeed");
    
    // 分配变体
    let variant = manager.assign_variant(&test.id, "user_123").await;
    assert!(variant.is_ok(), "Variant assignment should succeed");
    
    let variant = variant.unwrap();
    assert!(variant.id == "variant_a" || variant.id == "variant_b");
    
    // 记录执行结果
    let execution_result = TestExecutionResult {
        test_id: test.id.clone(),
        variant_id: variant.id.clone(),
        user_id: "user_123".to_string(),
        success: true,
        execution_time: std::time::Duration::from_millis(1500),
        user_satisfaction: Some(4.5),
        error_message: None,
        timestamp: Utc::now(),
        metadata: HashMap::new(),
    };
    
    let result = manager.record_execution_result(execution_result).await;
    assert!(result.is_ok(), "Recording execution result should succeed");
}

#[tokio::test]
async fn test_prompt_optimization() {
    let config = OptimizerConfig {
        strategy: OptimizationStrategy::RuleBased,
        optimization_interval: 3600,
        min_samples_for_optimization: 10, // Lower for testing
        target_metrics: vec![
            OptimizationTarget::SuccessRate,
            OptimizationTarget::ExecutionTime,
        ],
        genetic_algorithm: None,
        reinforcement_learning: None,
    };
    
    let optimizer = PromptOptimizer::new(config).await;
    assert!(optimizer.is_ok(), "Prompt optimizer should initialize successfully");
    
    let optimizer = optimizer.unwrap();
    
    // 创建性能记录
    let mut performance_records = Vec::new();
    for i in 0..15 {
        performance_records.push(PerformanceRecord {
            id: Uuid::new_v4().to_string(),
            config_id: "test_config".to_string(),
            timestamp: Utc::now(),
            success_rate: 0.8 + (i as f64 * 0.01), // 逐渐提高成功率
            execution_time: std::time::Duration::from_millis(1000 + i * 50),
            user_satisfaction: Some(4.0 + (i as f64 * 0.05)),
            error_rate: 0.1 - (i as f64 * 0.005),
            metadata: HashMap::new(),
        });
    }
    
    // 记录性能数据
    for record in &performance_records {
        let result = optimizer.record_performance(record.clone()).await;
        assert!(result.is_ok(), "Recording performance should succeed");
    }
    
    // 创建优化请求
    let request = OptimizationRequest {
        target: OptimizationTarget::SuccessRate,
        current_config: PromptConfig::default(),
        performance_history: performance_records,
        constraints: HashMap::new(),
    };
    
    // 执行优化
    let result = optimizer.optimize_config(request).await;
    assert!(result.is_ok(), "Configuration optimization should succeed");
    
    let optimization_result = result.unwrap();
    assert!(optimization_result.suggestions.len() > 0, "Should provide optimization suggestions");
    
    println!("Optimization suggestions: {:?}", optimization_result.suggestions);
}

#[tokio::test]
async fn test_performance_monitoring() {
    let config = create_test_config();
    let service = PromptService::new(config).await.unwrap();
    
    // 创建会话
    let session_id = "test_session_3".to_string();
    service.create_session(
        session_id.clone(),
        Some("security_analyst".to_string()),
        None,
    ).await.unwrap();
    
    // 记录多个性能数据点
    for i in 0..10 {
        let performance_data = PerformanceData {
            session_id: session_id.clone(),
            prompt_type: PromptType::Planner,
            execution_time: std::time::Duration::from_millis(1000 + i * 100),
            success: i % 8 != 0, // 大部分成功，偶尔失败
            user_satisfaction: Some(4.0 + (i as f64 * 0.1)),
            error_message: if i % 8 == 0 { Some("Test error".to_string()) } else { None },
            metadata: HashMap::new(),
        };
        
        let result = service.record_performance(performance_data).await;
        assert!(result.is_ok(), "Recording performance should succeed");
    }
    
    // 获取性能统计
    let stats = service.get_performance_stats(&session_id).await;
    assert!(stats.is_ok(), "Getting performance stats should succeed");
    
    let stats = stats.unwrap();
    assert!(stats.total_requests > 0, "Should have recorded requests");
    assert!(stats.success_rate > 0.0, "Should have calculated success rate");
    assert!(stats.avg_execution_time.as_millis() > 0, "Should have calculated average execution time");
    
    println!("Performance stats: {:?}", stats);
}

#[tokio::test]
async fn test_configuration_hierarchy() {
    let mut config_manager = PromptConfigManager::new();
    
    // 设置全局配置
    let global_config = PromptConfig {
        id: "global".to_string(),
        name: "Global Configuration".to_string(),
        description: "Default global configuration".to_string(),
        agent_profile: None,
        domain_template: None,
        core_templates: CoreTemplates {
            planner: "default_planner_template".to_string(),
            executor: "default_executor_template".to_string(),
            replanner: "default_replanner_template".to_string(),
            report_generator: "default_report_template".to_string(),
        },
        llm_config: LlmConfig {
            model: "gpt-4".to_string(),
            temperature: 0.7,
            max_tokens: 2048,
            top_p: 0.9,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            stop_sequences: vec![],
        },
        domain_templates: HashMap::new(),
        custom_templates: HashMap::new(),
        ab_test_config: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    
    config_manager.set_global_config(global_config);
    
    // 设置代理配置
    let agent_config = AgentProfile {
        name: "Security Analyst".to_string(),
        description: "Professional security analyst".to_string(),
        capabilities: vec!["vulnerability_analysis".to_string(), "risk_assessment".to_string()],
        llm_config: LlmConfig {
            model: "gpt-4".to_string(),
            temperature: 0.3, // 更低的温度用于安全分析
            max_tokens: 3072,
            top_p: 0.9,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            stop_sequences: vec![],
        },
        prompt_templates: HashMap::new(),
    };
    
    config_manager.add_agent_profile("security_analyst".to_string(), agent_config);
    
    // 获取合并后的配置
    let merged_config = config_manager.get_effective_config(
        Some("security_analyst".to_string()),
        None,
        None,
    );
    
    assert!(merged_config.is_ok(), "Getting effective config should succeed");
    
    let merged_config = merged_config.unwrap();
    assert_eq!(merged_config.llm_config.temperature, 0.3, "Agent config should override global config");
    assert_eq!(merged_config.llm_config.max_tokens, 3072, "Agent config should override global config");
    assert_eq!(merged_config.llm_config.model, "gpt-4", "Model should be inherited from global config");
}

#[tokio::test]
async fn test_template_validation() {
    let validation_rules = ValidationRules {
        max_template_size: 1024,
        allowed_extensions: vec![".hbs".to_string()],
        required_metadata: vec!["name".to_string(), "version".to_string()],
    };
    
    // 测试有效模板
    let valid_template = CustomTemplate {
        name: "valid_template".to_string(),
        description: "A valid template".to_string(),
        template_type: TemplateType::Planner,
        content: "Hello {{name}}, please {{action}}.".to_string(),
        variables: vec!["name".to_string(), "action".to_string()],
        tags: vec!["test".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    
    let validation_result = validate_template(&valid_template, &validation_rules);
    assert!(validation_result.is_valid, "Valid template should pass validation");
    assert!(validation_result.errors.is_empty(), "Valid template should have no errors");
    
    // 测试无效模板（内容过长）
    let invalid_template = CustomTemplate {
        name: "invalid_template".to_string(),
        description: "An invalid template".to_string(),
        template_type: TemplateType::Planner,
        content: "x".repeat(2048), // 超过最大长度
        variables: vec![],
        tags: vec![],
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    
    let validation_result = validate_template(&invalid_template, &validation_rules);
    assert!(!validation_result.is_valid, "Invalid template should fail validation");
    assert!(!validation_result.errors.is_empty(), "Invalid template should have errors");
}

/// 辅助函数：验证模板
fn validate_template(template: &CustomTemplate, rules: &ValidationRules) -> ValidationResult {
    let mut errors = Vec::new();
    
    // 检查模板大小
    if template.content.len() > rules.max_template_size {
        errors.push(format!(
            "Template content exceeds maximum size: {} > {}",
            template.content.len(),
            rules.max_template_size
        ));
    }
    
    // 检查必需的元数据
    for required_field in &rules.required_metadata {
        match required_field.as_str() {
            "name" => {
                if template.name.is_empty() {
                    errors.push("Template name is required".to_string());
                }
            }
            "version" => {
                // 这里可以添加版本检查逻辑
            }
            _ => {}
        }
    }
    
    ValidationResult {
        is_valid: errors.is_empty(),
        errors,
        warnings: vec![],
    }
}

#[tokio::test]
async fn test_error_handling() {
    let config = create_test_config();
    let service = PromptService::new(config).await.unwrap();
    
    // 测试获取不存在的会话
    let result = service.get_session("non_existent_session").await;
    assert!(result.is_err(), "Getting non-existent session should fail");
    
    // 测试在不存在的会话中构建 Prompt
    let request = PromptBuildRequest {
        build_type: PromptBuildType::Planner,
        context: PromptBuildContext {
            user_query: "test query".to_string(),
            target_info: None,
            available_tools: None,
            execution_context: None,
            history: None,
            custom_variables: HashMap::new(),
        },
        template_override: None,
        validation_settings: None,
    };
    
    let result = service.build_prompt("non_existent_session".to_string(), request).await;
    assert!(result.is_err(), "Building prompt in non-existent session should fail");
    
    // 测试重复创建会话
    let session_id = "duplicate_session".to_string();
    let result1 = service.create_session(session_id.clone(), None, None).await;
    assert!(result1.is_ok(), "First session creation should succeed");
    
    let result2 = service.create_session(session_id.clone(), None, None).await;
    assert!(result2.is_err(), "Duplicate session creation should fail");
}

#[tokio::test]
async fn test_concurrent_operations() {
    let config = create_test_config();
    let service = Arc::new(PromptService::new(config).await.unwrap());
    
    // 并发创建多个会话
    let mut handles = Vec::new();
    for i in 0..10 {
        let service_clone = service.clone();
        let handle = tokio::spawn(async move {
            let session_id = format!("concurrent_session_{}", i);
            service_clone.create_session(session_id, None, None).await
        });
        handles.push(handle);
    }
    
    // 等待所有任务完成
    let results = futures::future::join_all(handles).await;
    
    // 验证所有会话都创建成功
    for result in results {
        let session_result = result.unwrap();
        assert!(session_result.is_ok(), "Concurrent session creation should succeed");
    }
    
    // 验证服务状态
    let status = service.get_status().await;
    assert_eq!(status.active_sessions, 10, "Should have 10 active sessions");
}