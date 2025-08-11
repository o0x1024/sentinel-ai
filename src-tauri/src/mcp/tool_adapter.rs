use crate::mcp::types::{CallToolResult, McpTool, ToolDefinition, ToolInput};
use crate::tools::{UnifiedTool, ToolExecutionParams, ToolExecutionResult};
use crate::models::scan::ScanConfig;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

/// 将 UnifiedTool 适配为 McpTool 的适配器
pub struct ScanToolAdapter {
    scan_tool: Arc<dyn UnifiedTool>,
}

impl ScanToolAdapter {
    pub fn new(scan_tool: Arc<dyn UnifiedTool>) -> Self {
        Self { scan_tool }
    }
}

#[async_trait]
impl McpTool for ScanToolAdapter {
    fn definition(&self) -> ToolDefinition {
        let tool_name = self.scan_tool.name();

        let (input_schema, description) = match tool_name {
            "subdomain_scanner" => (
                json!({
                    "type": "object",
                    "properties": {
                        "domain": {
                            "type": "string",
                            "description": "要扫描的目标域名，例如: example.com"
                        },

                        "timeout": {
                            "type": "integer",
                            "description": "DNS查询超时时间（秒），影响扫描精度和速度",
                            "default": 30,
                            "minimum": 5,
                            "maximum": 120
                        },
                        "dns_servers": {
                            "type": "string",
                            "description": "自定义DNS服务器，多个服务器用逗号分隔，例如: 8.8.8.8,1.1.1.1",
                            "default": ""
                        },
                        "recursive": {
                            "type": "boolean",
                            "description": "是否启用递归扫描，对发现的子域名进一步扫描",
                            "default": false
                        }
                    },
                    "required": ["domain"]
                }),
                "高性能子域名扫描工具，使用rsubdomain库进行并发DNS解析和子域名暴破发现".to_string(),
            ),
            "port_scanner" => (
                json!({
                    "type": "object",
                    "properties": {
                        "target": {
                            "type": "string",
                            "description": "要扫描的目标IP地址或域名"
                        },
                        "ports": {
                            "type": "string",
                            "description": "要扫描的端口范围，例如: 1-1000 或 80,443,8080",
                            "default": "1-1000"
                        },
                        "threads": {
                            "type": "integer",
                            "description": "并发线程数，默认为100",
                            "default": 100,
                            "minimum": 1,
                            "maximum": 1000
                        },
                        "timeout": {
                            "type": "integer",
                            "description": "连接超时时间（毫秒），默认为3000",
                            "default": 3000,
                            "minimum": 100,
                            "maximum": 10000
                        }
                    },
                    "required": ["target"]
                }),
                "高性能端口扫描工具，用于发现目标主机的开放端口".to_string(),
            ),
            _ => (
                json!({
                    "type": "object",
                    "properties": {
                        "target": {
                            "type": "string",
                            "description": "扫描目标"
                        }
                    },
                    "required": ["target"]
                }),
                self.scan_tool.description().to_string(),
            ),
        };

        ToolDefinition {
            name: tool_name.to_string(),
            description,
            category: crate::mcp::types::ToolCategory::Scanning,
            input_schema,
            metadata: crate::mcp::types::ToolMetadata {
                author: "Sentinel AI Internal".to_string(),
                license: "MIT".to_string(),
                homepage: None,
                repository: None,
                tags: vec!["security".to_string(), "scanning".to_string()],
                install_command: None,
                requirements: vec![],
            },
        }
    }

    async fn call(&self, input: ToolInput) -> CallToolResult {
        // 将 MCP 参数转换为 ToolExecutionParams
        let execution_params = self.convert_parameters_to_execution_params(input.arguments)?;

        // 执行工具
        let execution_result = self.scan_tool.execute(execution_params).await?;

        // 将执行结果转换为 MCP Content 格式
        let content = crate::mcp::types::ToolContent {
            text: serde_json::to_string_pretty(&execution_result.output)?,
        };

        Ok(vec![content])
    }
}

impl ScanToolAdapter {
    fn convert_parameters_to_execution_params(&self, parameters: Value) -> Result<ToolExecutionParams> {
        use std::collections::HashMap;
        
        let mut inputs = HashMap::new();
        
        // 将所有参数转换为inputs
        if let Value::Object(map) = parameters {
            for (key, value) in map {
                inputs.insert(key, value);
            }
        }
        
        Ok(ToolExecutionParams {
            inputs,
            context: HashMap::new(),
            timeout: None,
            execution_id: None,
        })
    }
    
}
