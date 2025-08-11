// Sentinel AI 自定义Agent开发示例
// 本示例展示如何使用SDK创建一个自定义的数据分析Agent

use sentinel_ai_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use async_trait::async_trait;

// 1. 定义Agent配置结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataAnalysisConfig {
    pub analysis_type: String,
    pub output_format: String,
    pub max_records: usize,
    pub enable_visualization: bool,
}

// 2. 定义输入输出数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisInput {
    pub data_source: String,
    pub query: Option<String>,
    pub filters: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub summary: String,
    pub metrics: HashMap<String, f64>,
    pub charts: Vec<ChartData>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartData {
    pub chart_type: String,
    pub title: String,
    pub data: Vec<DataPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    pub x: String,
    pub y: f64,
}

// 3. 使用宏自动实现Agent trait
#[derive(UniversalAgent)]
#[agent(
    name = "DataAnalysisAgent",
    version = "1.0.0",
    description = "智能数据分析Agent，支持多种数据源和分析类型",
    author = "Your Name",
    category = "data_analytics",
    tags = ["data", "analytics", "visualization", "ai"]
)]
pub struct DataAnalysisAgent {
    config: DataAnalysisConfig,
    ai_client: Option<AIClient>,
    database_pool: Option<DatabasePool>,
}

// 4. 实现Agent的核心逻辑
#[async_trait]
impl UniversalAgent for DataAnalysisAgent {
    async fn initialize(&mut self, config: AgentConfig) -> Result<(), AgentError> {
        // 解析配置
        self.config = serde_json::from_value(config.parameters)?;
        
        // 初始化AI客户端
        self.ai_client = Some(AIClient::new(&config.ai_config).await?);
        
        // 初始化数据库连接
        if let Some(db_config) = config.database_config {
            self.database_pool = Some(DatabasePool::new(&db_config).await?);
        }
        
        Ok(())
    }

    async fn execute_task(&self, task: AgentTask, context: &ExecutionContext) -> Result<TaskResult, AgentError> {
        match task.action.as_str() {
            "analyze_data" => self.analyze_data(task, context).await,
            "generate_report" => self.generate_report(task, context).await,
            "create_visualization" => self.create_visualization(task, context).await,
            _ => Err(AgentError::UnsupportedAction(task.action)),
        }
    }

    async fn validate_task(&self, task: &AgentTask) -> Result<ValidationResult, AgentError> {
        let mut result = ValidationResult::new();
        
        // 验证必需参数
        if !task.inputs.contains_key("data_source") {
            result.add_error("Missing required parameter: data_source");
        }
        
        // 验证数据源可访问性
        if let Some(data_source) = task.inputs.get("data_source") {
            if !self.validate_data_source(data_source).await? {
                result.add_error("Data source is not accessible");
            }
        }
        
        Ok(result)
    }

    fn capabilities(&self) -> Vec<AgentCapability> {
        vec![
            AgentCapability {
                name: "analyze_data".to_string(),
                description: "分析数据并生成洞察".to_string(),
                input_schema: json_schema_for!(AnalysisInput),
                output_schema: json_schema_for!(AnalysisResult),
                examples: vec![
                    CapabilityExample {
                        name: "销售数据分析".to_string(),
                        description: "分析销售数据趋势".to_string(),
                        input: json!({
                            "data_source": "sales_database",
                            "query": "SELECT * FROM sales WHERE date >= '2024-01-01'",
                            "filters": {
                                "region": "北美",
                                "product_category": "电子产品"
                            }
                        }),
                        output: json!({
                            "summary": "2024年北美电子产品销售增长15%",
                            "metrics": {
                                "total_revenue": 1250000.0,
                                "growth_rate": 0.15,
                                "avg_order_value": 350.0
                            },
                            "recommendations": [
                                "增加移动设备产品线投入",
                                "优化Q4季度营销策略"
                            ]
                        }),
                    }
                ],
            },
            AgentCapability {
                name: "generate_report".to_string(),
                description: "生成分析报告".to_string(),
                input_schema: json_schema_for!(AnalysisResult),
                output_schema: json!({
                    "type": "object",
                    "properties": {
                        "report_url": {"type": "string"},
                        "format": {"type": "string"},
                        "size": {"type": "integer"}
                    }
                }),
                examples: vec![],
            },
            AgentCapability {
                name: "create_visualization".to_string(),
                description: "创建数据可视化图表".to_string(),
                input_schema: json_schema_for!(AnalysisResult),
                output_schema: json!({
                    "type": "object",
                    "properties": {
                        "charts": {
                            "type": "array",
                            "items": {"$ref": "#/definitions/ChartData"}
                        }
                    }
                }),
                examples: vec![],
            },
        ]
    }
}

// 5. 实现具体的业务逻辑
impl DataAnalysisAgent {
    pub fn new() -> Self {
        Self {
            config: DataAnalysisConfig {
                analysis_type: "descriptive".to_string(),
                output_format: "json".to_string(),
                max_records: 10000,
                enable_visualization: true,
            },
            ai_client: None,
            database_pool: None,
        }
    }

    async fn analyze_data(&self, task: AgentTask, context: &ExecutionContext) -> Result<TaskResult, AgentError> {
        // 解析输入
        let input: AnalysisInput = serde_json::from_value(task.inputs.get("data").unwrap().clone())?;
        
        // 获取数据
        let raw_data = self.fetch_data(&input).await?;
        
        // 使用AI进行分析
        let ai_analysis = self.perform_ai_analysis(&raw_data).await?;
        
        // 生成统计指标
        let metrics = self.calculate_metrics(&raw_data).await?;
        
        // 创建可视化
        let charts = if self.config.enable_visualization {
            self.create_charts(&raw_data, &metrics).await?
        } else {
            vec![]
        };
        
        // 生成建议
        let recommendations = self.generate_recommendations(&ai_analysis, &metrics).await?;
        
        let result = AnalysisResult {
            summary: ai_analysis.summary,
            metrics,
            charts,
            recommendations,
        };
        
        Ok(TaskResult {
            status: TaskStatus::Completed,
            outputs: vec![("analysis_result".to_string(), serde_json::to_value(result)?)],
            metadata: HashMap::new(),
        })
    }

    async fn generate_report(&self, task: AgentTask, context: &ExecutionContext) -> Result<TaskResult, AgentError> {
        let analysis_result: AnalysisResult = serde_json::from_value(
            task.inputs.get("analysis_result").unwrap().clone()
        )?;
        
        // 生成报告模板
        let template = self.select_report_template(&analysis_result).await?;
        
        // 填充数据
        let report_content = self.populate_template(&template, &analysis_result).await?;
        
        // 保存报告
        let report_path = self.save_report(&report_content).await?;
        
        Ok(TaskResult {
            status: TaskStatus::Completed,
            outputs: vec![
                ("report_url".to_string(), json!(report_path)),
                ("format".to_string(), json!(self.config.output_format)),
            ],
            metadata: HashMap::new(),
        })
    }

    async fn create_visualization(&self, task: AgentTask, context: &ExecutionContext) -> Result<TaskResult, AgentError> {
        let analysis_result: AnalysisResult = serde_json::from_value(
            task.inputs.get("analysis_result").unwrap().clone()
        )?;
        
        // 创建图表
        let charts = self.generate_interactive_charts(&analysis_result).await?;
        
        Ok(TaskResult {
            status: TaskStatus::Completed,
            outputs: vec![
                ("charts".to_string(), serde_json::to_value(charts)?),
            ],
            metadata: HashMap::new(),
        })
    }

    // 辅助方法
    async fn validate_data_source(&self, data_source: &str) -> Result<bool, AgentError> {
        // 实现数据源验证逻辑
        Ok(true)
    }

    async fn fetch_data(&self, input: &AnalysisInput) -> Result<Vec<HashMap<String, serde_json::Value>>, AgentError> {
        // 实现数据获取逻辑
        Ok(vec![])
    }

    async fn perform_ai_analysis(&self, data: &[HashMap<String, serde_json::Value>]) -> Result<AIAnalysisResult, AgentError> {
        if let Some(ai_client) = &self.ai_client {
            let prompt = format!(
                "请分析以下数据并提供洞察：\n{}",
                serde_json::to_string_pretty(data)?
            );
            
            let response = ai_client.chat(&prompt).await?;
            
            Ok(AIAnalysisResult {
                summary: response.content,
                confidence: response.confidence,
                insights: response.insights,
            })
        } else {
            Err(AgentError::MissingDependency("AI client not initialized".to_string()))
        }
    }

    async fn calculate_metrics(&self, data: &[HashMap<String, serde_json::Value>]) -> Result<HashMap<String, f64>, AgentError> {
        let mut metrics = HashMap::new();
        
        // 计算基础统计指标
        metrics.insert("record_count".to_string(), data.len() as f64);
        
        // 添加更多指标计算逻辑...
        
        Ok(metrics)
    }

    async fn create_charts(&self, data: &[HashMap<String, serde_json::Value>], metrics: &HashMap<String, f64>) -> Result<Vec<ChartData>, AgentError> {
        let mut charts = vec![];
        
        // 创建趋势图
        charts.push(ChartData {
            chart_type: "line".to_string(),
            title: "数据趋势".to_string(),
            data: vec![
                DataPoint { x: "1月".to_string(), y: 100.0 },
                DataPoint { x: "2月".to_string(), y: 120.0 },
                DataPoint { x: "3月".to_string(), y: 150.0 },
            ],
        });
        
        Ok(charts)
    }

    async fn generate_recommendations(&self, ai_analysis: &AIAnalysisResult, metrics: &HashMap<String, f64>) -> Result<Vec<String>, AgentError> {
        let mut recommendations = vec![];
        
        // 基于AI分析生成建议
        recommendations.extend(ai_analysis.insights.clone());
        
        // 基于指标生成建议
        if let Some(count) = metrics.get("record_count") {
            if *count < 100.0 {
                recommendations.push("数据量较少，建议收集更多数据以提高分析准确性".to_string());
            }
        }
        
        Ok(recommendations)
    }

    async fn select_report_template(&self, result: &AnalysisResult) -> Result<String, AgentError> {
        // 根据分析结果选择合适的报告模板
        Ok("default_template".to_string())
    }

    async fn populate_template(&self, template: &str, result: &AnalysisResult) -> Result<String, AgentError> {
        // 填充模板数据
        Ok(format!("分析报告\n\n摘要：{}\n\n建议：{:?}", result.summary, result.recommendations))
    }

    async fn save_report(&self, content: &str) -> Result<String, AgentError> {
        // 保存报告文件
        Ok("/tmp/analysis_report.html".to_string())
    }

    async fn generate_interactive_charts(&self, result: &AnalysisResult) -> Result<Vec<ChartData>, AgentError> {
        // 生成交互式图表
        Ok(result.charts.clone())
    }
}

// 6. 辅助数据结构
#[derive(Debug, Clone)]
struct AIAnalysisResult {
    summary: String,
    confidence: f64,
    insights: Vec<String>,
}

// 7. Agent注册函数
#[no_mangle]
pub extern "C" fn create_agent() -> Box<dyn UniversalAgent> {
    Box::new(DataAnalysisAgent::new())
}

// 8. Agent元数据导出
#[no_mangle]
pub extern "C" fn get_agent_metadata() -> AgentMetadata {
    AgentMetadata {
        id: "data_analysis_agent".to_string(),
        name: "数据分析Agent".to_string(),
        version: "1.0.0".to_string(),
        description: "智能数据分析Agent，支持多种数据源和分析类型".to_string(),
        author: "Sentinel AI Team".to_string(),
        category: "data_analytics".to_string(),
        tags: vec!["data".to_string(), "analytics".to_string(), "visualization".to_string()],
        license: "MIT".to_string(),
        homepage: Some("https://github.com/sentinel-ai/data-analysis-agent".to_string()),
        repository: Some("https://github.com/sentinel-ai/data-analysis-agent".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_initialization() {
        let mut agent = DataAnalysisAgent::new();
        let config = AgentConfig {
            parameters: json!({
                "analysis_type": "predictive",
                "output_format": "html",
                "max_records": 5000,
                "enable_visualization": true
            }),
            ai_config: AIConfig::default(),
            database_config: None,
        };
        
        let result = agent.initialize(config).await;
        assert!(result.is_ok());
        assert_eq!(agent.config.analysis_type, "predictive");
        assert_eq!(agent.config.max_records, 5000);
    }

    #[tokio::test]
    async fn test_task_validation() {
        let agent = DataAnalysisAgent::new();
        let task = AgentTask {
            id: "test_task".to_string(),
            action: "analyze_data".to_string(),
            inputs: HashMap::from([
                ("data_source".to_string(), json!("test_db")),
            ]),
            metadata: HashMap::new(),
        };
        
        let result = agent.validate_task(&task).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_valid());
    }

    #[tokio::test]
    async fn test_capabilities() {
        let agent = DataAnalysisAgent::new();
        let capabilities = agent.capabilities();
        
        assert_eq!(capabilities.len(), 3);
        assert!(capabilities.iter().any(|c| c.name == "analyze_data"));
        assert!(capabilities.iter().any(|c| c.name == "generate_report"));
        assert!(capabilities.iter().any(|c| c.name == "create_visualization"));
    }
}