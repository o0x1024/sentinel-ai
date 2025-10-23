//! HTTP请求工具
//! 
//! 支持自定义HTTP方法、请求头、参数等的通用HTTP客户端工具

use super::super::*;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;
use tracing::{error, info};
use uuid::Uuid;

// ============================================================================
// HTTP请求相关结构体
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequestConfig {
    pub url: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
    pub body: Option<String>,
    pub timeout_seconds: u64,
    pub follow_redirects: bool,
    pub verify_ssl: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status_code: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub response_time_ms: u64,
    pub final_url: String,
    pub content_length: Option<u64>,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequestResult {
    pub request: HttpRequestConfig,
    pub response: Option<HttpResponse>,
    pub error: Option<String>,
    pub success: bool,
    pub total_time_ms: u64,
}

// ============================================================================
// HTTP请求工具
// ============================================================================

#[derive(Debug)]
pub struct HttpRequestTool {
    metadata: ToolMetadata,
    parameters: ToolParameters,
    client: Client,
}

impl HttpRequestTool {
    pub fn new() -> Self {
        let metadata = ToolMetadata {
            author: "Built-in".to_string(),
            version: "1.0.0".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            repository: None,
            tags: vec![
                "http".to_string(), 
                "request".to_string(), 
                "web".to_string(), 
                "api".to_string(),
                "client".to_string()
            ],
            install_command: None,
            requirements: vec![],
        };

        let parameters = ToolParameters {
            parameters: vec![
                ParameterDefinition {
                    name: "url".to_string(),
                    param_type: ParameterType::String,
                    description: "Target URL for the HTTP request".to_string(),
                    required: true,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "method".to_string(),
                    param_type: ParameterType::String,
                    description: "HTTP method (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS)".to_string(),
                    required: false,
                    default_value: Some(json!("GET")),
                },
                ParameterDefinition {
                    name: "headers".to_string(),
                    param_type: ParameterType::Object,
                    description: "HTTP headers as key-value pairs".to_string(),
                    required: false,
                    default_value: Some(json!({})),
                },
                ParameterDefinition {
                    name: "query_params".to_string(),
                    param_type: ParameterType::Object,
                    description: "URL query parameters as key-value pairs".to_string(),
                    required: false,
                    default_value: Some(json!({})),
                },
                ParameterDefinition {
                    name: "body".to_string(),
                    param_type: ParameterType::String,
                    description: "Request body content (for POST, PUT, PATCH methods)".to_string(),
                    required: false,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "timeout_seconds".to_string(),
                    param_type: ParameterType::Number,
                    description: "Request timeout in seconds (1-300)".to_string(),
                    required: false,
                    default_value: Some(json!(30)),
                },
                ParameterDefinition {
                    name: "follow_redirects".to_string(),
                    param_type: ParameterType::Boolean,
                    description: "Whether to follow HTTP redirects".to_string(),
                    required: false,
                    default_value: Some(json!(true)),
                },
                ParameterDefinition {
                    name: "verify_ssl".to_string(),
                    param_type: ParameterType::Boolean,
                    description: "Whether to verify SSL certificates".to_string(),
                    required: false,
                    default_value: Some(json!(true)),
                },
            ],
            schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {"type": "string"},
                    "method": {"type": "string", "enum": ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"]},
                    "headers": {"type": "object"},
                    "query_params": {"type": "object"},
                    "body": {"type": "string"},
                    "timeout_seconds": {"type": "number", "minimum": 1, "maximum": 300},
                    "follow_redirects": {"type": "boolean"},
                    "verify_ssl": {"type": "boolean"}
                },
                "required": ["url"]
            }),
            required: vec!["url".to_string()],
            optional: vec![
                "method".to_string(),
                "headers".to_string(),
                "query_params".to_string(),
                "body".to_string(),
                "timeout_seconds".to_string(),
                "follow_redirects".to_string(),
                "verify_ssl".to_string(),
            ],
        };

        // 创建HTTP客户端
        let client = Client::builder()
            .user_agent("Sentinel-AI/1.0")
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { 
            metadata, 
            parameters,
            client,
        }
    }

    /// 解析HTTP方法
    fn parse_method(&self, method_str: &str) -> Result<Method> {
        Method::from_str(&method_str.to_uppercase())
            .map_err(|_| anyhow!("Unsupported HTTP method: {}", method_str))
    }

    /// 解析请求头
    fn parse_headers(&self, headers_value: &serde_json::Value) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        
        if let Some(obj) = headers_value.as_object() {
            for (key, value) in obj {
                if let Some(value_str) = value.as_str() {
                    headers.insert(key.clone(), value_str.to_string());
                }
            }
        }
        
        headers
    }

    /// 解析查询参数
    fn parse_query_params(&self, params_value: &serde_json::Value) -> HashMap<String, String> {
        let mut params = HashMap::new();
        
        if let Some(obj) = params_value.as_object() {
            for (key, value) in obj {
                let value_str = match value {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    _ => value.to_string(),
                };
                params.insert(key.clone(), value_str);
            }
        }
        
        params
    }

    /// 执行HTTP请求
    async fn execute_request(&self, config: HttpRequestConfig) -> Result<HttpRequestResult> {
        let start_time = std::time::Instant::now();
        
        info!("Executing HTTP {} request to: {}", config.method, config.url);
        
        // 解析HTTP方法
        let method = self.parse_method(&config.method)?;
        
        // 创建客户端配置
        let client = Client::builder()
            .user_agent("Sentinel-AI/1.0")
            .timeout(Duration::from_secs(config.timeout_seconds))
            .redirect(if config.follow_redirects {
                reqwest::redirect::Policy::limited(10)
            } else {
                reqwest::redirect::Policy::none()
            })
            .danger_accept_invalid_certs(!config.verify_ssl)
            .build()?;
        
        // 构建请求
        let mut request_builder = client.request(method, &config.url);
        
        // 添加查询参数
        if !config.query_params.is_empty() {
            request_builder = request_builder.query(&config.query_params);
        }
        
        // 添加请求头
        for (key, value) in &config.headers {
            request_builder = request_builder.header(key, value);
        }
        
        // 添加请求体
        if let Some(body) = &config.body {
            request_builder = request_builder.body(body.clone());
        }
        
        // 发送请求
        let request_start = std::time::Instant::now();
        match request_builder.send().await {
            Ok(response) => {
                let response_time_ms = request_start.elapsed().as_millis() as u64;
                let total_time_ms = start_time.elapsed().as_millis() as u64;
                
                // 获取响应信息
                let status_code = response.status().as_u16();
                let status_text = response.status().canonical_reason()
                    .unwrap_or("Unknown")
                    .to_string();
                let final_url = response.url().to_string();
                
                // 获取响应头
                let mut response_headers = HashMap::new();
                for (name, value) in response.headers() {
                    if let Ok(value_str) = value.to_str() {
                        response_headers.insert(name.to_string(), value_str.to_string());
                    }
                }
                
                // 获取内容类型和长度
                let content_type = response_headers.get("content-type").cloned();
                let content_length = response_headers.get("content-length")
                    .and_then(|s| s.parse().ok());
                
                // 获取响应体
                let body = match response.text().await {
                    Ok(text) => text,
                    Err(e) => {
                        error!("Failed to read response body: {}", e);
                        format!("Error reading response body: {}", e)
                    }
                };
                
                let http_response = HttpResponse {
                    status_code,
                    status_text,
                    headers: response_headers,
                    body,
                    response_time_ms,
                    final_url,
                    content_length,
                    content_type,
                };
                
                info!("HTTP request completed: {} {} in {}ms", 
                      status_code, config.url, total_time_ms);
                
                Ok(HttpRequestResult {
                    request: config,
                    response: Some(http_response),
                    error: None,
                    success: status_code < 400,
                    total_time_ms,
                })
            }
            Err(e) => {
                let total_time_ms = start_time.elapsed().as_millis() as u64;
                error!("HTTP request failed: {}", e);
                
                Ok(HttpRequestResult {
                    request: config,
                    response: None,
                    error: Some(e.to_string()),
                    success: false,
                    total_time_ms,
                })
            }
        }
    }
}

#[async_trait]
impl UnifiedTool for HttpRequestTool {
    fn name(&self) -> &str {
        "http_request"
    }

    fn description(&self) -> &str {
        "通用HTTP客户端工具，支持自定义请求方法、请求头、参数等，用于API测试和Web服务交互"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Utility
    }

    fn parameters(&self) -> &ToolParameters {
        &self.parameters
    }

    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn is_available(&self) -> bool {
        // HTTP请求工具总是可用
        true
    }

    async fn execute(&self, params: ToolExecutionParams) -> Result<ToolExecutionResult> {
        let execution_id = params.execution_id.unwrap_or_else(Uuid::new_v4);
        let start_time = std::time::Instant::now();
        
        info!("Executing HTTP request tool with execution_id: {}", execution_id);
        
        // 验证和解析参数
        let url = params.inputs.get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("URL parameter is required"))?;
        
        if url.is_empty() {
            return Err(anyhow!("URL不能为空"));
        }
        
        // 验证URL格式
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(anyhow!("URL必须以http://或https://开头"));
        }
        
        let method = params.inputs.get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("GET");
        
        let headers = self.parse_headers(
            params.inputs.get("headers").unwrap_or(&json!({}))
        );
        
        let query_params = self.parse_query_params(
            params.inputs.get("query_params").unwrap_or(&json!({}))
        );
        
        let body = params.inputs.get("body")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        let timeout_seconds = params.inputs.get("timeout_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(30);
        
        if timeout_seconds == 0 || timeout_seconds > 300 {
            return Err(anyhow!("超时时间必须在1-300秒之间"));
        }
        
        let follow_redirects = params.inputs.get("follow_redirects")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        
        let verify_ssl = params.inputs.get("verify_ssl")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        
        // 创建请求配置
        let config = HttpRequestConfig {
            url: url.to_string(),
            method: method.to_string(),
            headers,
            query_params,
            body,
            timeout_seconds,
            follow_redirects,
            verify_ssl,
        };
        
        // 执行HTTP请求
        match self.execute_request(config).await {
            Ok(result) => {
                let execution_time_ms = start_time.elapsed().as_millis() as u64;
                
                let result_json = json!({
                    "request": {
                        "url": result.request.url,
                        "method": result.request.method,
                        "headers": result.request.headers,
                        "query_params": result.request.query_params,
                        "body": result.request.body,
                        "timeout_seconds": result.request.timeout_seconds,
                        "follow_redirects": result.request.follow_redirects,
                        "verify_ssl": result.request.verify_ssl
                    },
                    "response": result.response,
                    "success": result.success,
                    "error": result.error,
                    "total_time_ms": result.total_time_ms,
                    "summary": {
                        "status": if result.success { "success" } else { "failed" },
                        "status_code": result.response.as_ref().map(|r| r.status_code),
                        "response_time_ms": result.response.as_ref().map(|r| r.response_time_ms),
                        "content_length": result.response.as_ref().and_then(|r| r.content_length),
                        "content_type": result.response.as_ref().and_then(|r| r.content_type.clone())
                    }
                });
                
                let status_info = if let Some(ref response) = result.response {
                    format!("HTTP {} {} - {}", result.request.method, response.status_code, result.request.url)
                } else {
                    format!("HTTP {} failed - {}", result.request.method, result.request.url)
                };
                
                info!("{} completed in {}ms", status_info, result.total_time_ms);
                
                Ok(ToolExecutionResult {
                    execution_id,
                    tool_id: self.name().to_string(),
                    tool_name: self.name().to_string(),
                    success: result.success,
                    output: result_json,
                    error: result.error,
                    execution_time_ms,
                    metadata: HashMap::new(),
                    status: if result.success { 
                        crate::tools::ExecutionStatus::Completed 
                    } else { 
                        crate::tools::ExecutionStatus::Failed 
                    },
                    started_at: chrono::Utc::now(),
                    completed_at: Some(chrono::Utc::now()),
                })
            }
            Err(e) => {
                let execution_time_ms = start_time.elapsed().as_millis() as u64;
                error!("HTTP request tool execution failed: {}", e);
                
                Ok(ToolExecutionResult {
                    execution_id,
                    tool_id: self.name().to_string(),
                    tool_name: self.name().to_string(),
                    success: false,
                    output: json!({}),
                    error: Some(e.to_string()),
                    execution_time_ms,
                    metadata: HashMap::new(),
                    started_at: chrono::Utc::now(),
                    status: crate::tools::ExecutionStatus::Failed,
                    completed_at: Some(chrono::Utc::now()),
                })
            }
        }
    }
}
