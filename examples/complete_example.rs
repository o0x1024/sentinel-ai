//! 完整的可扩展架构使用示例
//! 展示如何使用SDK开发自定义Agent并集成到工作流中

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use anyhow::Result;
use tokio;

// 导入我们的SDK和引擎
mod workflow_engine;
mod agent_sdk;

use workflow_engine::*;
use agent_sdk::*;

/// 数据分析Agent配置
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DataAnalysisConfig {
    pub database_url: String,
    pub api_key: String,
    pub max_records: usize,
    pub timeout_seconds: u64,
}

impl AgentConfig for DataAnalysisConfig {
    fn validate(&self) -> Result<()> {
        if self.database_url.is_empty() {
            return Err(anyhow::anyhow!("数据库URL不能为空"));
        }
        if self.api_key.is_empty() {
            return Err(anyhow::anyhow!("API密钥不能为空"));
        }
        if self.max_records == 0 {
            return Err(anyhow::anyhow!("最大记录数必须大于0"));
        }
        Ok(())
    }
    
    fn get_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "database_url": {
                    "type": "string",
                    "description": "数据库连接URL"
                },
                "api_key": {
                    "type": "string",
                    "description": "API访问密钥"
                },
                "max_records": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 10000,
                    "description": "最大处理记录数"
                },
                "timeout_seconds": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 3600,
                    "description": "超时时间（秒）"
                }
            },
            "required": ["database_url", "api_key"]
        })
    }
}

/// 数据获取处理器
action_handler!(
    FetchDataHandler,
    description: "从数据源获取数据",
    inputs: json!({
        "type": "object",
        "properties": {
            "query": {
                "type": "string",
                "description": "SQL查询语句"
            },
            "parameters": {
                "type": "object",
                "description": "查询参数"
            }
        },
        "required": ["query"]
    }),
    outputs: json!({
        "type": "object",
        "properties": {
            "data": {
                "type": "array",
                "description": "查询结果数据"
            },
            "count": {
                "type": "integer",
                "description": "记录数量"
            },
            "metadata": {
                "type": "object",
                "description": "数据元信息"
            }
        }
    }),
    handler: |inputs: &HashMap<String, Value>, context: &ExecutionContext| {
        let validated = validate_inputs!(inputs, {
            query: String
        });
        
        let query = validated.get("query").unwrap().as_str().unwrap();
        let logger = Logger::new("FetchDataHandler".to_string());
        
        logger.info(&format!("执行查询: {}", query));
        
        // 模拟数据库查询
        let mock_data = vec![
            json!({
                "id": 1,
                "name": "产品A",
                "sales": 1000,
                "category": "电子产品",
                "date": "2024-01-01"
            }),
            json!({
                "id": 2,
                "name": "产品B",
                "sales": 1500,
                "category": "家居用品",
                "date": "2024-01-02"
            }),
            json!({
                "id": 3,
                "name": "产品C",
                "sales": 800,
                "category": "电子产品",
                "date": "2024-01-03"
            })
        ];
        
        let metadata = json!({
            "query_time": chrono::Utc::now().to_rfc3339(),
            "source": "mock_database",
            "query": query
        });
        
        logger.info(&format!("获取到 {} 条记录", mock_data.len()));
        
        Ok(build_outputs!({
            data: mock_data.clone(),
            count: mock_data.len(),
            metadata: metadata
        }))
    }
);

/// 数据分析处理器
action_handler!(
    AnalyzeDataHandler,
    description: "分析数据并生成洞察",
    inputs: json!({
        "type": "object",
        "properties": {
            "data": {
                "type": "array",
                "description": "要分析的数据"
            },
            "analysis_type": {
                "type": "string",
                "enum": ["summary", "trend", "correlation", "prediction"],
                "description": "分析类型"
            },
            "group_by": {
                "type": "string",
                "description": "分组字段"
            }
        },
        "required": ["data", "analysis_type"]
    }),
    outputs: json!({
        "type": "object",
        "properties": {
            "insights": {
                "type": "object",
                "description": "分析洞察"
            },
            "charts": {
                "type": "array",
                "description": "图表数据"
            },
            "recommendations": {
                "type": "array",
                "description": "建议"
            }
        }
    }),
    handler: |inputs: &HashMap<String, Value>, context: &ExecutionContext| {
        let validated = validate_inputs!(inputs, {
            data: Vec<Value>,
            analysis_type: String
        });
        
        let data = validated.get("data").unwrap().as_array().unwrap();
        let analysis_type = validated.get("analysis_type").unwrap().as_str().unwrap();
        let logger = Logger::new("AnalyzeDataHandler".to_string());
        
        logger.info(&format!("执行 {} 分析，数据量: {}", analysis_type, data.len()));
        
        let insights = match analysis_type {
            "summary" => {
                let total_sales: i64 = data.iter()
                    .filter_map(|item| item.get("sales")?.as_i64())
                    .sum();
                let avg_sales = if data.is_empty() { 0.0 } else { total_sales as f64 / data.len() as f64 };
                
                json!({
                    "total_records": data.len(),
                    "total_sales": total_sales,
                    "average_sales": avg_sales,
                    "analysis_type": "summary"
                })
            },
            "trend" => {
                json!({
                    "trend_direction": "上升",
                    "growth_rate": 15.5,
                    "period": "月度",
                    "analysis_type": "trend"
                })
            },
            "correlation" => {
                json!({
                    "correlations": [
                        {
                            "variables": ["sales", "category"],
                            "coefficient": 0.75,
                            "strength": "强正相关"
                        }
                    ],
                    "analysis_type": "correlation"
                })
            },
            "prediction" => {
                json!({
                    "predictions": [
                        {
                            "period": "下月",
                            "predicted_sales": 1200,
                            "confidence": 0.85
                        }
                    ],
                    "model_accuracy": 0.92,
                    "analysis_type": "prediction"
                })
            },
            _ => {
                return Err(anyhow::anyhow!("不支持的分析类型: {}", analysis_type));
            }
        };
        
        let charts = vec![
            json!({
                "type": "bar",
                "title": "销售数据分布",
                "data": data.iter().take(5).collect::<Vec<_>>()
            }),
            json!({
                "type": "line",
                "title": "销售趋势",
                "data": [
                    {"date": "2024-01-01", "value": 1000},
                    {"date": "2024-01-02", "value": 1500},
                    {"date": "2024-01-03", "value": 800}
                ]
            })
        ];
        
        let recommendations = vec![
            "建议关注电子产品类别的销售表现",
            "考虑在销售高峰期增加库存",
            "分析客户购买模式以优化营销策略"
        ];
        
        logger.info(&format!("分析完成，生成 {} 个图表和 {} 条建议", charts.len(), recommendations.len()));
        
        Ok(build_outputs!({
            insights: insights,
            charts: charts,
            recommendations: recommendations
        }))
    }
);

/// 报告生成处理器
action_handler!(
    GenerateReportHandler,
    description: "生成分析报告",
    inputs: json!({
        "type": "object",
        "properties": {
            "insights": {
                "type": "object",
                "description": "分析洞察"
            },
            "charts": {
                "type": "array",
                "description": "图表数据"
            },
            "recommendations": {
                "type": "array",
                "description": "建议"
            },
            "format": {
                "type": "string",
                "enum": ["html", "pdf", "json", "markdown"],
                "description": "报告格式"
            }
        },
        "required": ["insights", "format"]
    }),
    outputs: json!({
        "type": "object",
        "properties": {
            "report_content": {
                "type": "string",
                "description": "报告内容"
            },
            "report_path": {
                "type": "string",
                "description": "报告文件路径"
            },
            "metadata": {
                "type": "object",
                "description": "报告元信息"
            }
        }
    }),
    handler: |inputs: &HashMap<String, Value>, context: &ExecutionContext| {
        let validated = validate_inputs!(inputs, {
            insights: Value,
            format: String
        });
        
        let insights = validated.get("insights").unwrap();
        let format = validated.get("format").unwrap().as_str().unwrap();
        let charts = inputs.get("charts").cloned().unwrap_or(json!([]));
        let recommendations = inputs.get("recommendations").cloned().unwrap_or(json!([]));
        
        let logger = Logger::new("GenerateReportHandler".to_string());
        logger.info(&format!("生成 {} 格式的报告", format));
        
        let report_content = match format {
            "html" => {
                format!(
                    r#"<!DOCTYPE html>
<html>
<head>
    <title>数据分析报告</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        .header {{ background-color: #f0f0f0; padding: 20px; border-radius: 5px; }}
        .section {{ margin: 20px 0; }}
        .chart {{ background-color: #f9f9f9; padding: 15px; border-radius: 5px; }}
        .recommendation {{ background-color: #e8f4fd; padding: 10px; margin: 5px 0; border-radius: 3px; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>数据分析报告</h1>
        <p>生成时间: {}</p>
    </div>
    
    <div class="section">
        <h2>分析洞察</h2>
        <pre>{}</pre>
    </div>
    
    <div class="section">
        <h2>图表</h2>
        <div class="chart">{}</div>
    </div>
    
    <div class="section">
        <h2>建议</h2>
        {}
    </div>
</body>
</html>"#,
                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
                    serde_json::to_string_pretty(insights).unwrap(),
                    serde_json::to_string_pretty(&charts).unwrap(),
                    recommendations.as_array().unwrap_or(&vec![])
                        .iter()
                        .map(|r| format!("<div class=\"recommendation\">{}</div>", r.as_str().unwrap_or("")))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            },
            "markdown" => {
                format!(
                    "# 数据分析报告\n\n生成时间: {}\n\n## 分析洞察\n\n```json\n{}\n```\n\n## 图表\n\n```json\n{}\n```\n\n## 建议\n\n{}\n",
                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
                    serde_json::to_string_pretty(insights).unwrap(),
                    serde_json::to_string_pretty(&charts).unwrap(),
                    recommendations.as_array().unwrap_or(&vec![])
                        .iter()
                        .map(|r| format!("- {}", r.as_str().unwrap_or("")))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            },
            "json" => {
                json!({
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "insights": insights,
                    "charts": charts,
                    "recommendations": recommendations
                }).to_string()
            },
            _ => {
                return Err(anyhow::anyhow!("不支持的报告格式: {}", format));
            }
        };
        
        let report_path = format!("/tmp/report_{}.{}", 
            chrono::Utc::now().timestamp(), 
            if format == "html" { "html" } else if format == "markdown" { "md" } else { "json" }
        );
        
        let metadata = json!({
            "generated_at": chrono::Utc::now().to_rfc3339(),
            "format": format,
            "size_bytes": report_content.len(),
            "sections": ["insights", "charts", "recommendations"]
        });
        
        logger.info(&format!("报告生成完成，大小: {} 字节", report_content.len()));
        
        Ok(build_outputs!({
            report_content: report_content,
            report_path: report_path,
            metadata: metadata
        }))
    }
);

/// 定义数据分析Agent
define_agent!(
    name: "data_analysis_agent",
    version: "1.0.0",
    description: "智能数据分析Agent，支持数据获取、分析和报告生成",
    author: "Sentinel AI Team",
    config: DataAnalysisConfig,
    actions: {
        fetch_data {
            description: "从数据源获取数据",
            inputs: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "parameters": { "type": "object" }
                },
                "required": ["query"]
            }),
            outputs: json!({
                "type": "object",
                "properties": {
                    "data": { "type": "array" },
                    "count": { "type": "integer" },
                    "metadata": { "type": "object" }
                }
            }),
            handler: FetchDataHandler
        },
        analyze_data {
            description: "分析数据并生成洞察",
            inputs: json!({
                "type": "object",
                "properties": {
                    "data": { "type": "array" },
                    "analysis_type": { "type": "string" },
                    "group_by": { "type": "string" }
                },
                "required": ["data", "analysis_type"]
            }),
            outputs: json!({
                "type": "object",
                "properties": {
                    "insights": { "type": "object" },
                    "charts": { "type": "array" },
                    "recommendations": { "type": "array" }
                }
            }),
            handler: AnalyzeDataHandler
        },
        generate_report {
            description: "生成分析报告",
            inputs: json!({
                "type": "object",
                "properties": {
                    "insights": { "type": "object" },
                    "charts": { "type": "array" },
                    "recommendations": { "type": "array" },
                    "format": { "type": "string" }
                },
                "required": ["insights", "format"]
            }),
            outputs: json!({
                "type": "object",
                "properties": {
                    "report_content": { "type": "string" },
                    "report_path": { "type": "string" },
                    "metadata": { "type": "object" }
                }
            }),
            handler: GenerateReportHandler
        }
    }
);

/// 通知Agent（简化版）
struct NotificationAgent;

#[async_trait::async_trait]
impl AgentExecutor for NotificationAgent {
    async fn execute(
        &self,
        action: &str,
        inputs: &HashMap<String, Value>,
        _context: &ExecutionContext,
    ) -> Result<HashMap<String, Value>> {
        match action {
            "send_notification" => {
                let message = inputs.get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("默认通知消息");
                let channel = inputs.get("channel")
                    .and_then(|v| v.as_str())
                    .unwrap_or("email");
                
                println!("📧 [{}] {}", channel.to_uppercase(), message);
                
                Ok(build_outputs!({
                    success: true,
                    sent_at: chrono::Utc::now().to_rfc3339(),
                    channel: channel
                }))
            },
            _ => Err(anyhow::anyhow!("不支持的动作: {}", action))
        }
    }
    
    fn get_capabilities(&self) -> Vec<String> {
        vec!["send_notification".to_string()]
    }
    
    fn get_agent_type(&self) -> String {
        "notification_agent".to_string()
    }
}

/// 主函数 - 演示完整的工作流
#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 启动可扩展AI工作流演示");
    
    // 1. 创建Agent实例
    let data_config = DataAnalysisConfig {
        database_url: "postgresql://localhost:5432/analytics".to_string(),
        api_key: "demo_api_key_12345".to_string(),
        max_records: 1000,
        timeout_seconds: 300,
    };
    
    let data_agent = Agent::new(data_config)?;
    let notification_agent = NotificationAgent;
    
    // 2. 创建工作流引擎并注册Agent
    let (engine, mut event_receiver) = WorkflowEngineBuilder::new()
        .with_agent("data_analysis_agent".to_string(), std::sync::Arc::new(data_agent))
        .with_agent("notification_agent".to_string(), std::sync::Arc::new(notification_agent))
        .build()
        .await;
    
    // 3. 启动事件监听器
    tokio::spawn(async move {
        while let Some(event) = event_receiver.recv().await {
            match event {
                WorkflowEvent::WorkflowStarted { execution_id } => {
                    println!("✅ 工作流开始执行: {}", execution_id);
                },
                WorkflowEvent::StepStarted { execution_id, step_id } => {
                    println!("🔄 步骤开始: {} -> {}", execution_id, step_id);
                },
                WorkflowEvent::StepCompleted { execution_id, step_id } => {
                    println!("✅ 步骤完成: {} -> {}", execution_id, step_id);
                },
                WorkflowEvent::WorkflowCompleted { execution_id } => {
                    println!("🎉 工作流完成: {}", execution_id);
                },
                WorkflowEvent::WorkflowFailed { execution_id, error } => {
                    println!("❌ 工作流失败: {} - {}", execution_id, error);
                },
                WorkflowEvent::StepFailed { execution_id, step_id, error } => {
                    println!("❌ 步骤失败: {} -> {} - {}", execution_id, step_id, error);
                },
            }
        }
    });
    
    // 4. 定义工作流
    let workflow = WorkflowDefinition {
        metadata: WorkflowMetadata {
            name: "数据分析工作流".to_string(),
            version: "1.0.0".to_string(),
            description: Some("完整的数据分析和报告生成流程".to_string()),
            author: Some("Sentinel AI".to_string()),
            tags: vec!["数据分析".to_string(), "报告".to_string(), "自动化".to_string()],
            timeout: Some(1800), // 30分钟
        },
        variables: HashMap::from([
            ("report_format".to_string(), json!("html")),
            ("analysis_type".to_string(), json!("summary")),
            ("notification_channel".to_string(), json!("email")),
        ]),
        steps: vec![
            WorkflowStep {
                id: "fetch_sales_data".to_string(),
                name: "获取销售数据".to_string(),
                agent_type: "data_analysis_agent".to_string(),
                action: "fetch_data".to_string(),
                inputs: HashMap::from([
                    ("query".to_string(), json!("SELECT * FROM sales WHERE date >= '2024-01-01'")),
                    ("parameters".to_string(), json!({})),
                ]),
                outputs: vec!["data".to_string(), "count".to_string(), "metadata".to_string()],
                depends_on: vec![],
                condition: None,
                retry: Some(RetryConfig {
                    max_attempts: 3,
                    delay_seconds: 5,
                    backoff_multiplier: Some(2.0),
                }),
                timeout: Some(120),
                parallel: Some(false),
            },
            WorkflowStep {
                id: "analyze_sales_data".to_string(),
                name: "分析销售数据".to_string(),
                agent_type: "data_analysis_agent".to_string(),
                action: "analyze_data".to_string(),
                inputs: HashMap::from([
                    ("data".to_string(), json!("${{fetch_sales_data.data}}")),
                    ("analysis_type".to_string(), json!("${{analysis_type}}")),
                    ("group_by".to_string(), json!("category")),
                ]),
                outputs: vec!["insights".to_string(), "charts".to_string(), "recommendations".to_string()],
                depends_on: vec!["fetch_sales_data".to_string()],
                condition: Some("${{fetch_sales_data.count}} > 0".to_string()),
                retry: Some(RetryConfig {
                    max_attempts: 2,
                    delay_seconds: 3,
                    backoff_multiplier: None,
                }),
                timeout: Some(180),
                parallel: Some(false),
            },
            WorkflowStep {
                id: "generate_analysis_report".to_string(),
                name: "生成分析报告".to_string(),
                agent_type: "data_analysis_agent".to_string(),
                action: "generate_report".to_string(),
                inputs: HashMap::from([
                    ("insights".to_string(), json!("${{analyze_sales_data.insights}}")),
                    ("charts".to_string(), json!("${{analyze_sales_data.charts}}")),
                    ("recommendations".to_string(), json!("${{analyze_sales_data.recommendations}}")),
                    ("format".to_string(), json!("${{report_format}}")),
                ]),
                outputs: vec!["report_content".to_string(), "report_path".to_string(), "metadata".to_string()],
                depends_on: vec!["analyze_sales_data".to_string()],
                condition: None,
                retry: Some(RetryConfig {
                    max_attempts: 2,
                    delay_seconds: 2,
                    backoff_multiplier: None,
                }),
                timeout: Some(60),
                parallel: Some(false),
            },
            WorkflowStep {
                id: "send_completion_notification".to_string(),
                name: "发送完成通知".to_string(),
                agent_type: "notification_agent".to_string(),
                action: "send_notification".to_string(),
                inputs: HashMap::from([
                    ("message".to_string(), json!("数据分析工作流已完成，报告已生成")),
                    ("channel".to_string(), json!("${{notification_channel}}")),
                ]),
                outputs: vec!["success".to_string(), "sent_at".to_string()],
                depends_on: vec!["generate_analysis_report".to_string()],
                condition: None,
                retry: Some(RetryConfig {
                    max_attempts: 3,
                    delay_seconds: 1,
                    backoff_multiplier: None,
                }),
                timeout: Some(30),
                parallel: Some(false),
            },
        ],
        error_handling: Some(ErrorHandling {
            on_failure: "stop".to_string(),
            cleanup_steps: vec![],
            notification: Some(NotificationConfig {
                channels: vec!["email".to_string(), "slack".to_string()],
                template: "工作流执行失败: {{error}}".to_string(),
            }),
        }),
    };
    
    // 5. 验证工作流
    println!("🔍 验证工作流定义...");
    let validation_issues = engine.validate_workflow(&workflow).await?;
    if !validation_issues.is_empty() {
        println!("⚠️  工作流验证发现问题:");
        for issue in &validation_issues {
            println!("   - {}", issue);
        }
        return Err(anyhow::anyhow!("工作流验证失败"));
    }
    println!("✅ 工作流验证通过");
    
    // 6. 执行工作流
    println!("🚀 开始执行工作流...");
    let execution_id = engine.execute_workflow(workflow, None).await?;
    println!("📋 执行ID: {}", execution_id);
    
    // 7. 等待工作流完成
    let mut completed = false;
    let start_time = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(60);
    
    while !completed && start_time.elapsed() < timeout {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        if let Some(execution) = engine.get_execution(&execution_id).await {
            match execution.status {
                WorkflowStatus::Completed => {
                    completed = true;
                    println!("\n🎉 工作流执行完成!");
                    println!("📊 执行结果:");
                    
                    for (step_id, result) in &execution.step_results {
                        println!("   步骤 {}: {:?}", step_id, result.status);
                        if !result.outputs.is_empty() {
                            println!("     输出: {} 个字段", result.outputs.len());
                        }
                    }
                    
                    // 显示最终报告内容（截取前500字符）
                    if let Some(report_result) = execution.step_results.get("generate_analysis_report") {
                        if let Some(report_content) = report_result.outputs.get("report_content") {
                            let content = report_content.as_str().unwrap_or("");
                            println!("\n📄 生成的报告内容（前500字符）:");
                            println!("{}", &content[..content.len().min(500)]);
                            if content.len() > 500 {
                                println!("... (内容已截断)");
                            }
                        }
                    }
                },
                WorkflowStatus::Failed => {
                    completed = true;
                    println!("\n❌ 工作流执行失败: {:?}", execution.error);
                },
                WorkflowStatus::Cancelled => {
                    completed = true;
                    println!("\n⏹️  工作流已取消");
                },
                _ => {
                    print!(".");
                    std::io::Write::flush(&mut std::io::stdout()).unwrap();
                }
            }
        }
    }
    
    if !completed {
        println!("\n⏰ 工作流执行超时");
    }
    
    println!("\n✨ 演示完成!");
    println!("\n📝 本示例展示了:");
    println!("   • 使用SDK快速开发自定义Agent");
    println!("   • 定义复杂的工作流（包含依赖、条件、重试等）");
    println!("   • 工作流引擎的执行和监控");
    println!("   • 事件驱动的架构");
    println!("   • 可扩展的Agent生态系统");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_data_analysis_agent() {
        let config = DataAnalysisConfig {
            database_url: "test://localhost".to_string(),
            api_key: "test_key".to_string(),
            max_records: 100,
            timeout_seconds: 30,
        };
        
        let agent = Agent::new(config).unwrap();
        let tester = AgentTester::new(agent);
        
        // 测试数据获取
        let fetch_inputs = HashMap::from([
            ("query".to_string(), json!("SELECT * FROM test_table"))
        ]);
        
        let fetch_result = tester.test_action("fetch_data", fetch_inputs, None).await.unwrap();
        assert!(fetch_result.contains_key("data"));
        assert!(fetch_result.contains_key("count"));
        
        // 测试数据分析
        let analyze_inputs = HashMap::from([
            ("data".to_string(), fetch_result.get("data").unwrap().clone()),
            ("analysis_type".to_string(), json!("summary"))
        ]);
        
        let analyze_result = tester.test_action("analyze_data", analyze_inputs, None).await.unwrap();
        assert!(analyze_result.contains_key("insights"));
        assert!(analyze_result.contains_key("charts"));
        assert!(analyze_result.contains_key("recommendations"));
        
        // 测试报告生成
        let report_inputs = HashMap::from([
            ("insights".to_string(), analyze_result.get("insights").unwrap().clone()),
            ("format".to_string(), json!("json"))
        ]);
        
        let report_result = tester.test_action("generate_report", report_inputs, None).await.unwrap();
        assert!(report_result.contains_key("report_content"));
        assert!(report_result.contains_key("report_path"));
    }
    
    #[test]
    fn test_agent_metadata() {
        let metadata = Agent::get_metadata();
        assert_eq!(metadata.name, "data_analysis_agent");
        assert_eq!(metadata.version, "1.0.0");
        assert_eq!(metadata.capabilities.len(), 3);
        
        let capability_names: Vec<_> = metadata.capabilities.iter()
            .map(|c| c.action.as_str())
            .collect();
        assert!(capability_names.contains(&"fetch_data"));
        assert!(capability_names.contains(&"analyze_data"));
        assert!(capability_names.contains(&"generate_report"));
    }
}