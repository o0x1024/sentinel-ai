//! Sentinel AI Prompt 高级定制系统集成示例
//! 
//! 这个示例展示了如何在实际项目中集成和使用 Prompt 定制系统

use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
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

/// 示例：完整的安全测试场景
/// 
/// 这个示例展示了如何使用 Prompt 定制系统来执行一个完整的安全测试流程
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::init();
    
    println!("🚀 启动 Sentinel AI Prompt 定制系统示例");
    
    // 1. 初始化 Prompt 服务
    let prompt_service = initialize_prompt_service().await?;
    println!("✅ Prompt 服务初始化完成");
    
    // 2. 设置配置和模板
    setup_configurations(&prompt_service).await?;
    println!("✅ 配置和模板设置完成");
    
    // 3. 创建 A/B 测试
    let ab_test_id = create_ab_test(&prompt_service).await?;
    println!("✅ A/B 测试创建完成: {}", ab_test_id);
    
    // 4. 执行安全测试场景
    execute_security_test_scenario(&prompt_service).await?;
    println!("✅ 安全测试场景执行完成");
    
    // 5. 演示自动优化
    demonstrate_auto_optimization(&prompt_service).await?;
    println!("✅ 自动优化演示完成");
    
    // 6. 生成性能报告
    generate_performance_report(&prompt_service).await?;
    println!("✅ 性能报告生成完成");
    
    println!("🎉 示例执行完成！");
    Ok(())
}

/// 初始化 Prompt 服务
async fn initialize_prompt_service() -> Result<Arc<PromptService>, Box<dyn std::error::Error>> {
    let config = PromptServiceConfig {
        config_dir: "./config/prompts".to_string(),
        template_dir: "./templates/prompts".to_string(),
        cache_size: 1000,
        enable_hot_reload: true,
        enable_ab_testing: true,
        enable_auto_optimization: true,
        validation: ValidationSettings {
            max_length: 8192,
            min_length: 10,
            required_variables: vec!["user_query".to_string()],
            forbidden_patterns: vec!["<script>".to_string(), "javascript:".to_string()],
            custom_rules: HashMap::new(),
        },
    };
    
    let service = PromptService::new(config).await?;
    Ok(Arc::new(service))
}

/// 设置配置和模板
async fn setup_configurations(service: &PromptService) -> Result<(), Box<dyn std::error::Error>> {
    // 创建安全分析师代理配置
    let security_analyst_config = create_security_analyst_config();
    service.save_agent_profile("security_analyst".to_string(), security_analyst_config).await?;
    
    // 创建渗透测试专家代理配置
    let pentest_expert_config = create_pentest_expert_config();
    service.save_agent_profile("pentest_expert".to_string(), pentest_expert_config).await?;
    
    // 创建 Web 安全领域模板
    let web_security_template = create_web_security_template();
    service.save_domain_template("web_security".to_string(), web_security_template).await?;
    
    // 创建网络安全领域模板
    let network_security_template = create_network_security_template();
    service.save_domain_template("network_security".to_string(), network_security_template).await?;
    
    // 创建自定义模板
    let custom_templates = create_custom_templates();
    for (name, template) in custom_templates {
        service.save_custom_template(name, template).await?;
    }
    
    Ok(())
}

/// 创建安全分析师代理配置
fn create_security_analyst_config() -> AgentProfile {
    AgentProfile {
        name: "Security Analyst".to_string(),
        description: "专业的安全分析师，擅长漏洞分析和风险评估".to_string(),
        capabilities: vec![
            "vulnerability_analysis".to_string(),
            "risk_assessment".to_string(),
            "compliance_check".to_string(),
            "threat_modeling".to_string(),
        ],
        llm_config: LlmConfig {
            model: "gpt-4".to_string(),
            temperature: 0.3, // 低温度确保一致性
            max_tokens: 3072,
            top_p: 0.9,
            frequency_penalty: 0.1,
            presence_penalty: 0.1,
            stop_sequences: vec![],
        },
        prompt_templates: HashMap::new(),
    }
}

/// 创建渗透测试专家代理配置
fn create_pentest_expert_config() -> AgentProfile {
    AgentProfile {
        name: "Penetration Testing Expert".to_string(),
        description: "经验丰富的渗透测试专家，专注于实际攻击模拟".to_string(),
        capabilities: vec![
            "exploit_development".to_string(),
            "payload_crafting".to_string(),
            "privilege_escalation".to_string(),
            "lateral_movement".to_string(),
        ],
        llm_config: LlmConfig {
            model: "gpt-4".to_string(),
            temperature: 0.4, // 稍高温度增加创造性
            max_tokens: 4096,
            top_p: 0.95,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            stop_sequences: vec![],
        },
        prompt_templates: HashMap::new(),
    }
}

/// 创建 Web 安全领域模板
fn create_web_security_template() -> DomainTemplate {
    DomainTemplate {
        name: "Web Security Testing".to_string(),
        description: "Web 应用安全测试专用模板集合".to_string(),
        planner_template: Some(TemplateContent {
            system_prompt: r#"
你是一个专业的 Web 安全测试规划师。你的任务是为 Web 应用安全测试制定详细的执行计划。

核心职责：
1. 分析目标 Web 应用的架构和技术栈
2. 识别潜在的攻击面和安全风险
3. 制定系统化的测试计划
4. 选择合适的测试工具和技术
5. 评估测试风险和制定安全措施

测试方法论：
- OWASP Top 10 漏洞检测
- 业务逻辑漏洞分析
- 认证和授权机制测试
- 输入验证和输出编码检查
- 会话管理安全性评估

当前任务上下文：
- 用户需求：{{user_query}}
- 目标信息：{{target_info}}
- 可用工具：{{available_tools}}
- 测试约束：{{constraints}}

请制定详细的 Web 安全测试计划。
"#.to_string(),
            user_prompt: r#"
请为以下 Web 应用制定安全测试计划：

目标 URL: {{target_url}}
应用类型: {{app_type}}
技术栈: {{tech_stack}}
测试范围: {{test_scope}}
时间限制: {{time_limit}}

请提供：
1. 测试计划概述
2. 详细的测试步骤
3. 工具和技术选择
4. 风险评估和安全措施
5. 预期的测试结果
"#.to_string(),
        }),
        executor_template: Some(TemplateContent {
            system_prompt: r#"
你是一个专业的 Web 安全测试执行者。你需要根据测试计划执行具体的安全测试步骤。

执行原则：
1. 严格按照计划执行测试
2. 详细记录每个步骤的结果
3. 发现漏洞时进行深入分析
4. 确保测试的安全性和合规性
5. 及时报告异常情况

技术能力：
- 熟练使用各种 Web 安全测试工具
- 深入理解 Web 应用安全机制
- 具备手工测试和自动化测试能力
- 能够分析和验证安全漏洞

当前执行环境：
- 测试计划：{{execution_plan}}
- 当前步骤：{{current_step}}
- 目标信息：{{target_info}}
- 可用工具：{{available_tools}}
- 执行上下文：{{execution_context}}

请执行指定的测试步骤并报告结果。
"#.to_string(),
            user_prompt: r#"
请执行以下 Web 安全测试步骤：

步骤描述: {{step_description}}
测试目标: {{test_target}}
使用工具: {{required_tools}}
预期结果: {{expected_result}}
安全约束: {{safety_constraints}}

请开始执行并提供详细的测试报告。
"#.to_string(),
        }),
        replanner_template: None,
        report_generator_template: None,
        custom_variables: HashMap::new(),
    }
}

/// 创建网络安全领域模板
fn create_network_security_template() -> DomainTemplate {
    DomainTemplate {
        name: "Network Security Testing".to_string(),
        description: "网络安全测试专用模板集合".to_string(),
        planner_template: Some(TemplateContent {
            system_prompt: r#"
你是一个专业的网络安全测试规划师。你的任务是为网络基础设施安全测试制定详细的执行计划。

核心职责：
1. 分析目标网络架构和拓扑
2. 识别网络设备和服务
3. 评估网络安全配置
4. 制定渗透测试策略
5. 规划后渗透活动

测试阶段：
1. 网络发现和枚举
2. 端口扫描和服务识别
3. 漏洞扫描和分析
4. 渗透测试和利用
5. 权限提升和横向移动
6. 数据收集和影响评估

当前任务上下文：
- 用户需求：{{user_query}}
- 目标网络：{{target_network}}
- 可用工具：{{available_tools}}
- 测试授权：{{authorization_scope}}

请制定详细的网络安全测试计划。
"#.to_string(),
            user_prompt: r#"
请为以下网络环境制定安全测试计划：

目标网络: {{target_network}}
网络规模: {{network_size}}
关键资产: {{critical_assets}}
测试类型: {{test_type}}
授权范围: {{authorization_scope}}

请提供：
1. 网络测试策略
2. 详细的测试阶段
3. 工具和技术选择
4. 风险控制措施
5. 预期的发现和影响
"#.to_string(),
        }),
        executor_template: Some(TemplateContent {
            system_prompt: r#"
你是一个专业的网络安全测试执行者。你需要根据测试计划执行具体的网络安全测试任务。

执行能力：
1. 网络扫描和枚举
2. 漏洞识别和验证
3. 渗透测试和利用
4. 后渗透活动
5. 证据收集和分析

安全原则：
1. 最小化对生产环境的影响
2. 严格遵守测试授权范围
3. 详细记录所有测试活动
4. 及时报告重大发现
5. 确保测试数据的安全性

当前执行环境：
- 测试计划：{{execution_plan}}
- 当前阶段：{{current_phase}}
- 目标网络：{{target_network}}
- 可用工具：{{available_tools}}
- 执行状态：{{execution_status}}

请执行指定的网络测试任务。
"#.to_string(),
            user_prompt: r#"
请执行以下网络安全测试任务：

任务描述: {{task_description}}
目标范围: {{target_scope}}
使用工具: {{required_tools}}
预期发现: {{expected_findings}}
安全限制: {{safety_limits}}

请开始执行并提供详细的测试结果。
"#.to_string(),
        }),
        replanner_template: None,
        report_generator_template: None,
        custom_variables: HashMap::new(),
    }
}

/// 创建自定义模板
fn create_custom_templates() -> HashMap<String, CustomTemplate> {
    let mut templates = HashMap::new();
    
    // 快速扫描模板
    templates.insert(
        "quick_scan".to_string(),
        CustomTemplate {
            name: "Quick Security Scan".to_string(),
            description: "快速安全扫描模板，适用于初步安全评估".to_string(),
            template_type: TemplateType::Planner,
            content: r#"
你需要为目标 {{target}} 制定一个快速安全扫描计划。

扫描重点：
1. 端口扫描 - 识别开放的服务
2. 服务识别 - 确定服务版本和配置
3. 常见漏洞检测 - 检查已知的安全问题
4. 基础配置检查 - 验证安全配置

时间限制：{{time_limit | default: "30分钟"}}
扫描深度：{{scan_depth | default: "基础"}}

请提供简洁高效的扫描计划。
"#.to_string(),
            variables: vec![
                "target".to_string(),
                "time_limit".to_string(),
                "scan_depth".to_string(),
            ],
            tags: vec!["quick".to_string(), "scan".to_string(), "basic".to_string()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    );
    
    // 深度分析模板
    templates.insert(
        "deep_analysis".to_string(),
        CustomTemplate {
            name: "Deep Security Analysis".to_string(),
            description: "深度安全分析模板，适用于全面的安全评估".to_string(),
            template_type: TemplateType::Executor,
            content: r#"
对目标 {{target}} 进行深度安全分析：

分析维度：
1. 架构安全性 - 评估系统架构的安全设计
2. 配置安全性 - 检查安全配置的完整性
3. 代码安全性 - 分析代码中的安全漏洞
4. 数据安全性 - 评估数据保护措施
5. 运行时安全性 - 监控运行时的安全状态

当前发现：{{current_findings}}
分析深度：{{analysis_depth | default: "全面"}}
重点关注：{{focus_areas}}

请进行详细分析并提供专业的安全建议。
"#.to_string(),
            variables: vec![
                "target".to_string(),
                "current_findings".to_string(),
                "analysis_depth".to_string(),
                "focus_areas".to_string(),
            ],
            tags: vec!["deep".to_string(), "analysis".to_string(), "comprehensive".to_string()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    );
    
    templates
}

/// 创建 A/B 测试
async fn create_ab_test(service: &PromptService) -> Result<String, Box<dyn std::error::Error>> {
    let test_id = format!("planner_optimization_test_{}", Uuid::new_v4());
    
    // 创建控制组变体（原始模板）
    let control_variant = TestVariant {
        id: "control_group".to_string(),
        name: "原始规划器模板".to_string(),
        description: "当前使用的标准规划器模板".to_string(),
        template_config: PromptConfig::default(),
        is_control: true,
    };
    
    // 创建实验组变体（优化模板）
    let mut optimized_config = PromptConfig::default();
    optimized_config.llm_config.temperature = 0.2; // 降低温度提高一致性
    optimized_config.llm_config.max_tokens = 3072; // 增加最大令牌数
    
    let experimental_variant = TestVariant {
        id: "experimental_group".to_string(),
        name: "优化规划器模板".to_string(),
        description: "经过优化的规划器模板，旨在提高成功率和一致性".to_string(),
        template_config: optimized_config,
        is_control: false,
    };
    
    // 创建 A/B 测试
    let ab_test = ABTest {
        id: test_id.clone(),
        name: "规划器模板优化测试".to_string(),
        description: "测试优化后的规划器模板是否能提高安全测试的成功率和质量".to_string(),
        variants: vec![control_variant, experimental_variant],
        traffic_allocation: TrafficAllocation::Even,
        evaluation_metrics: vec![
            EvaluationMetric::SuccessRate,
            EvaluationMetric::ExecutionTime,
            EvaluationMetric::UserSatisfaction,
        ],
        conditions: TestConditions {
            min_sample_size: 1000,
            max_duration: 604800, // 7 天
            confidence_level: 0.95,
            early_stopping: true,
        },
        status: TestStatus::Draft,
        created_at: Utc::now(),
        started_at: None,
        ended_at: None,
    };
    
    // 创建并启动测试
    service.create_ab_test(ab_test).await?;
    service.start_ab_test(&test_id).await?;
    
    Ok(test_id)
}

/// 执行安全测试场景
async fn execute_security_test_scenario(service: &PromptService) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔍 开始执行安全测试场景...");
    
    // 场景 1: Web 应用安全测试
    println!("\n📱 场景 1: Web 应用安全测试");
    let web_session_id = "web_security_test_session".to_string();
    service.create_session(
        web_session_id.clone(),
        Some("security_analyst".to_string()),
        Some("web_security".to_string()),
    ).await?;
    
    let web_test_result = execute_web_security_test(service, &web_session_id).await?;
    println!("✅ Web 安全测试完成，成功率: {:.1}%", web_test_result.success_rate * 100.0);
    
    // 场景 2: 网络安全测试
    println!("\n🌐 场景 2: 网络安全测试");
    let network_session_id = "network_security_test_session".to_string();
    service.create_session(
        network_session_id.clone(),
        Some("pentest_expert".to_string()),
        Some("network_security".to_string()),
    ).await?;
    
    let network_test_result = execute_network_security_test(service, &network_session_id).await?;
    println!("✅ 网络安全测试完成，成功率: {:.1}%", network_test_result.success_rate * 100.0);
    
    // 场景 3: 快速扫描
    println!("\n⚡ 场景 3: 快速安全扫描");
    let quick_scan_session_id = "quick_scan_session".to_string();
    service.create_session(
        quick_scan_session_id.clone(),
        Some("security_analyst".to_string()),
        None,
    ).await?;
    
    let quick_scan_result = execute_quick_scan(service, &quick_scan_session_id).await?;
    println!("✅ 快速扫描完成，发现 {} 个潜在问题", quick_scan_result.findings_count);
    
    Ok(())
}

/// 执行 Web 安全测试
async fn execute_web_security_test(
    service: &PromptService,
    session_id: &str,
) -> Result<TestResult, Box<dyn std::error::Error>> {
    let target_info = TargetInfo {
        url: Some("https://demo.testfire.net".to_string()),
        domain: Some("demo.testfire.net".to_string()),
        port: Some(443),
        service: Some("https".to_string()),
        custom_fields: {
            let mut fields = HashMap::new();
            fields.insert("app_type".to_string(), "banking_application".to_string());
            fields.insert("tech_stack".to_string(), "Java, Spring, MySQL".to_string());
            fields
        },
        ..Default::default()
    };
    
    let tools = vec![
        ToolInfo {
            name: "burp_suite".to_string(),
            description: "Web application security testing platform".to_string(),
            version: Some("2023.10".to_string()),
            capabilities: vec!["proxy".to_string(), "scanner".to_string(), "intruder".to_string()],
            requirements: None,
        },
        ToolInfo {
            name: "sqlmap".to_string(),
            description: "Automatic SQL injection tool".to_string(),
            version: Some("1.7.2".to_string()),
            capabilities: vec!["sql_injection".to_string(), "database_enumeration".to_string()],
            requirements: None,
        },
    ];
    
    // 1. 规划阶段
    let planning_request = PromptBuildRequest {
        build_type: PromptBuildType::Planner,
        context: PromptBuildContext {
            user_query: "对银行应用进行全面的 Web 安全测试，重点关注 SQL 注入、XSS 和认证绕过漏洞".to_string(),
            target_info: Some(target_info.clone()),
            available_tools: Some(tools.clone()),
            execution_context: None,
            history: None,
            custom_variables: {
                let mut vars = HashMap::new();
                vars.insert("test_scope".to_string(), "full_application".to_string());
                vars.insert("time_limit".to_string(), "4_hours".to_string());
                vars
            },
        },
        template_override: None,
        validation_settings: None,
    };
    
    let planning_response = service.build_prompt(session_id.to_string(), planning_request).await?;
    println!("📋 规划阶段完成，生成了 {} 字符的测试计划", planning_response.prompt.len());
    
    // 模拟执行多个测试步骤
    let mut success_count = 0;
    let test_steps = vec![
        "信息收集和侦察",
        "认证机制测试",
        "SQL 注入测试",
        "XSS 漏洞测试",
        "CSRF 保护测试",
        "会话管理测试",
    ];
    
    for (i, step) in test_steps.iter().enumerate() {
        let execution_request = PromptBuildRequest {
            build_type: PromptBuildType::Executor,
            context: PromptBuildContext {
                user_query: format!("执行测试步骤: {}", step),
                target_info: Some(target_info.clone()),
                available_tools: Some(tools.clone()),
                execution_context: Some(ExecutionContext {
                    current_step: i + 1,
                    total_steps: test_steps.len(),
                    previous_results: vec![],
                    error_info: None,
                }),
                history: None,
                custom_variables: HashMap::new(),
            },
            template_override: None,
            validation_settings: None,
        };
        
        let execution_response = service.build_prompt(session_id.to_string(), execution_request).await?;
        
        // 模拟执行结果
        let execution_success = (i + 1) % 5 != 0; // 大部分成功，偶尔失败
        if execution_success {
            success_count += 1;
        }
        
        // 记录性能数据
        let performance_data = PerformanceData {
            session_id: session_id.to_string(),
            prompt_type: PromptType::Executor,
            execution_time: Duration::from_millis(1500 + i as u64 * 200),
            success: execution_success,
            user_satisfaction: Some(4.0 + (i as f64 * 0.1)),
            error_message: if !execution_success {
                Some(format!("步骤 {} 执行失败", step))
            } else {
                None
            },
            metadata: HashMap::new(),
        };
        
        service.record_performance(performance_data).await?;
        
        println!("  ✓ 步骤 {}: {} - {}", i + 1, step, if execution_success { "成功" } else { "失败" });
        
        // 模拟执行时间
        sleep(Duration::from_millis(100)).await;
    }
    
    Ok(TestResult {
        success_rate: success_count as f64 / test_steps.len() as f64,
        total_steps: test_steps.len(),
        successful_steps: success_count,
        findings_count: success_count * 2, // 假设每个成功步骤发现2个问题
    })
}

/// 执行网络安全测试
async fn execute_network_security_test(
    service: &PromptService,
    session_id: &str,
) -> Result<TestResult, Box<dyn std::error::Error>> {
    let target_info = TargetInfo {
        ip: Some("192.168.1.0/24".to_string()),
        custom_fields: {
            let mut fields = HashMap::new();
            fields.insert("network_size".to_string(), "small_office".to_string());
            fields.insert("critical_assets".to_string(), "file_server,domain_controller".to_string());
            fields
        },
        ..Default::default()
    };
    
    let tools = vec![
        ToolInfo {
            name: "nmap".to_string(),
            description: "Network discovery and security auditing".to_string(),
            version: Some("7.94".to_string()),
            capabilities: vec!["port_scan".to_string(), "service_detection".to_string(), "os_detection".to_string()],
            requirements: None,
        },
        ToolInfo {
            name: "metasploit".to_string(),
            description: "Penetration testing framework".to_string(),
            version: Some("6.3.31".to_string()),
            capabilities: vec!["exploitation".to_string(), "payload_generation".to_string(), "post_exploitation".to_string()],
            requirements: None,
        },
    ];
    
    // 网络测试阶段
    let test_phases = vec![
        "网络发现和主机枚举",
        "端口扫描和服务识别",
        "漏洞扫描和分析",
        "渗透测试和利用",
        "权限提升测试",
        "横向移动模拟",
    ];
    
    let mut success_count = 0;
    
    for (i, phase) in test_phases.iter().enumerate() {
        let request = PromptBuildRequest {
            build_type: if i == 0 { PromptBuildType::Planner } else { PromptBuildType::Executor },
            context: PromptBuildContext {
                user_query: format!("执行网络安全测试阶段: {}", phase),
                target_info: Some(target_info.clone()),
                available_tools: Some(tools.clone()),
                execution_context: Some(ExecutionContext {
                    current_step: i + 1,
                    total_steps: test_phases.len(),
                    previous_results: vec![],
                    error_info: None,
                }),
                history: None,
                custom_variables: HashMap::new(),
            },
            template_override: None,
            validation_settings: None,
        };
        
        let response = service.build_prompt(session_id.to_string(), request).await?;
        
        // 模拟执行结果
        let phase_success = i < 4; // 前4个阶段成功，后面的可能失败
        if phase_success {
            success_count += 1;
        }
        
        // 记录性能数据
        let performance_data = PerformanceData {
            session_id: session_id.to_string(),
            prompt_type: if i == 0 { PromptType::Planner } else { PromptType::Executor },
            execution_time: Duration::from_millis(2000 + i as u64 * 500),
            success: phase_success,
            user_satisfaction: Some(4.2 + (i as f64 * 0.05)),
            error_message: if !phase_success {
                Some(format!("阶段 {} 执行受限", phase))
            } else {
                None
            },
            metadata: HashMap::new(),
        };
        
        service.record_performance(performance_data).await?;
        
        println!("  ✓ 阶段 {}: {} - {}", i + 1, phase, if phase_success { "成功" } else { "受限" });
        
        sleep(Duration::from_millis(150)).await;
    }
    
    Ok(TestResult {
        success_rate: success_count as f64 / test_phases.len() as f64,
        total_steps: test_phases.len(),
        successful_steps: success_count,
        findings_count: success_count * 3, // 网络测试通常发现更多问题
    })
}

/// 执行快速扫描
async fn execute_quick_scan(
    service: &PromptService,
    session_id: &str,
) -> Result<TestResult, Box<dyn std::error::Error>> {
    let target_info = TargetInfo {
        url: Some("https://example.com".to_string()),
        ip: Some("93.184.216.34".to_string()),
        domain: Some("example.com".to_string()),
        port: Some(443),
        service: Some("https".to_string()),
        ..Default::default()
    };
    
    let request = PromptBuildRequest {
        build_type: PromptBuildType::Planner,
        context: PromptBuildContext {
            user_query: "对目标进行快速安全扫描，识别明显的安全问题".to_string(),
            target_info: Some(target_info),
            available_tools: Some(vec![
                ToolInfo {
                    name: "nmap".to_string(),
                    description: "Network scanner".to_string(),
                    version: Some("7.94".to_string()),
                    capabilities: vec!["port_scan".to_string()],
                    requirements: None,
                },
            ]),
            execution_context: None,
            history: None,
            custom_variables: {
                let mut vars = HashMap::new();
                vars.insert("time_limit".to_string(), "15分钟".to_string());
                vars.insert("scan_depth".to_string(), "基础".to_string());
                vars
            },
        },
        template_override: Some("quick_scan".to_string()),
        validation_settings: None,
    };
    
    let response = service.build_prompt(session_id.to_string(), request).await?;
    
    // 记录性能数据
    let performance_data = PerformanceData {
        session_id: session_id.to_string(),
        prompt_type: PromptType::Planner,
        execution_time: Duration::from_millis(800),
        success: true,
        user_satisfaction: Some(4.3),
        error_message: None,
        metadata: HashMap::new(),
    };
    
    service.record_performance(performance_data).await?;
    
    println!("  ✓ 快速扫描计划生成完成");
    
    Ok(TestResult {
        success_rate: 1.0,
        total_steps: 1,
        successful_steps: 1,
        findings_count: 5, // 快速扫描发现的问题较少
    })
}

/// 演示自动优化
async fn demonstrate_auto_optimization(service: &PromptService) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🤖 开始自动优化演示...");
    
    // 创建优化会话
    let optimization_session_id = "optimization_demo_session".to_string();
    service.create_session(
        optimization_session_id.clone(),
        Some("security_analyst".to_string()),
        Some("web_security".to_string()),
    ).await?;
    
    // 生成一些性能数据用于优化
    println!("📊 生成性能数据...");
    for i in 0..20 {
        let performance_data = PerformanceData {
            session_id: optimization_session_id.clone(),
            prompt_type: PromptType::Planner,
            execution_time: Duration::from_millis(1000 + (i * 100) + (rand::random::<u64>() % 500)),
            success: i % 7 != 0, // 大部分成功
            user_satisfaction: Some(3.5 + (i as f64 * 0.02) + (rand::random::<f64>() * 0.5)),
            error_message: if i % 7 == 0 {
                Some("模拟执行错误".to_string())
            } else {
                None
            },
            metadata: HashMap::new(),
        };
        
        service.record_performance(performance_data).await?;
    }
    
    // 请求优化建议
    println!("🔍 分析性能数据并生成优化建议...");
    let optimization_request = OptimizationRequest {
        target: OptimizationTarget::SuccessRate,
        current_config: PromptConfig::default(),
        performance_history: vec![], // 实际实现中会从服务获取
        constraints: HashMap::new(),
    };
    
    let optimization_result = service.optimize_config(
        optimization_session_id.clone(),
        optimization_request,
    ).await?;
    
    if let Some(optimized_config) = optimization_result.optimized_config {
        println!("✅ 找到优化配置:");
        println!("   - 预期成功率提升: {:.1}%", optimization_result.expected_improvement * 100.0);
        println!("   - 优化的参数: 温度 = {:.2}, 最大令牌 = {}", 
                optimized_config.llm_config.temperature,
                optimized_config.llm_config.max_tokens);
        
        // 应用优化配置
        service.apply_config_optimization(optimization_session_id, optimized_config).await?;
        println!("✅ 优化配置已应用");
    } else {
        println!("ℹ️  当前配置已经是最优的，无需进一步优化");
    }
    
    // 显示优化建议
    if !optimization_result.suggestions.is_empty() {
        println!("\n💡 优化建议:");
        for (i, suggestion) in optimization_result.suggestions.iter().enumerate() {
            println!("   {}. {} (预期提升: {:.1}%)", 
                    i + 1, 
                    suggestion.description,
                    suggestion.expected_improvement * 100.0);
        }
    }
    
    Ok(())
}

/// 生成性能报告
async fn generate_performance_report(service: &PromptService) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📊 生成性能报告...");
    
    // 获取服务状态
    let status = service.get_status().await;
    println!("\n🔧 服务状态:");
    println!("   - 服务已初始化: {}", status.is_initialized);
    println!("   - 活跃会话数: {}", status.active_sessions);
    println!("   - 总Prompt构建数: {}", status.total_prompts_built);
    println!("   - 缓存命中率: {:.1}%", status.cache_hit_rate * 100.0);
    println!("   - 平均构建时间: {}ms", status.avg_build_time.as_millis());
    
    // 获取所有会话的性能统计
    let session_ids = vec![
        "web_security_test_session",
        "network_security_test_session",
        "quick_scan_session",
        "optimization_demo_session",
    ];
    
    println!("\n📈 会话性能统计:");
    for session_id in session_ids {
        if let Ok(stats) = service.get_performance_stats(session_id).await {
            println!("\n   会话: {}", session_id);
            println!("     - 总请求数: {}", stats.total_requests);
            println!("     - 成功率: {:.1}%", stats.success_rate * 100.0);
            println!("     - 平均执行时间: {}ms", stats.avg_execution_time.as_millis());
            println!("     - 平均用户满意度: {:.1}/5.0", stats.avg_user_satisfaction);
            println!("     - 错误率: {:.1}%", stats.error_rate * 100.0);
        }
    }
    
    // 生成总结报告
    println!("\n📋 总结报告:");
    println!("   ✅ 成功演示了 Prompt 定制系统的核心功能");
    println!("   ✅ 验证了多种安全测试场景的适用性");
    println!("   ✅ 展示了 A/B 测试和自动优化能力");
    println!("   ✅ 确认了性能监控和报告功能");
    
    Ok(())
}

/// 测试结果结构
#[derive(Debug)]
struct TestResult {
    success_rate: f64,
    total_steps: usize,
    successful_steps: usize,
    findings_count: usize,
}

/// 模拟随机数生成（简化版）
mod rand {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};
    
    pub fn random<T>() -> T
    where
        T: From<u64>,
    {
        let mut hasher = DefaultHasher::new();
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .hash(&mut hasher);
        T::from(hasher.finish())
    }
}