//! HTTP request tool (migrated)

use crate::unified_types::*;
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
    pub use_passive_proxy: bool,
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
                "client".to_string(),
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
                ParameterDefinition {
                    name: "use_passive_proxy".to_string(),
                    param_type: ParameterType::Boolean,
                    description: "Route traffic through passive scanning proxy (port 4201) for vulnerability detection".to_string(),
                    required: false,
                    default_value: Some(json!(false)),
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
                    "verify_ssl": {"type": "boolean"},
                    "use_passive_proxy": {"type": "boolean"}
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
                "use_passive_proxy".to_string(),
            ],
        };

        let client = Client::builder().user_agent("Sentinel-AI/1.0").build().unwrap_or_else(|_| Client::new());

        Self { metadata, parameters, client }
    }

    fn parse_method(&self, method_str: &str) -> Result<Method> {
        Method::from_str(&method_str.to_uppercase()).map_err(|_| anyhow!("Unsupported HTTP method: {}", method_str))
    }

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

    fn categorize_error(&self, error: &reqwest::Error) -> String {
        if error.is_timeout() { return "请求超时：服务器响应时间过长".to_string(); }
        if error.is_connect() { return format!("连接失败：无法连接到目标服务器 ({})", error.url().map(|u| u.to_string()).unwrap_or_else(|| "未知地址".to_string())); }
        if error.is_request() { return "请求格式错误：请检查URL、请求头或请求体格式".to_string(); }
        if error.is_redirect() { return "重定向错误：重定向次数过多或重定向循环".to_string(); }
        if error.is_body() { return "响应体读取错误：服务器响应数据格式异常".to_string(); }
        if error.is_decode() { return "响应解码错误：无法解析服务器响应内容".to_string(); }
        let error_str = error.to_string();
        if error_str.contains("dns") || error_str.contains("resolve") { return "DNS解析失败：无法解析域名，请检查网络连接或域名是否正确".to_string(); }
        if error_str.contains("ssl") || error_str.contains("tls") || error_str.contains("certificate") { return "SSL/TLS错误：证书验证失败，可尝试设置verify_ssl为false".to_string(); }
        if error_str.contains("unreachable") || error_str.contains("network") { return "网络不可达：目标主机无法访问，请检查网络连接和防火墙设置".to_string(); }
        if error_str.contains("refused") || error_str.contains("connection refused") { return "连接被拒绝：目标端口未开放或服务未启动".to_string(); }
        format!("HTTP请求失败：{}", error)
    }

    async fn execute_request(&self, config: HttpRequestConfig) -> Result<HttpRequestResult> {
        let start_time = std::time::Instant::now();
        
        if config.use_passive_proxy {
            info!("Executing HTTP {} request to: {} (via passive proxy)", config.method, config.url);
        } else {
            info!("Executing HTTP {} request to: {}", config.method, config.url);
        }
        
        let method = self.parse_method(&config.method)?;
        
        let mut client_builder = Client::builder()
            .user_agent("Sentinel-AI/1.0")
            .timeout(Duration::from_secs(config.timeout_seconds))
            .redirect(if config.follow_redirects { reqwest::redirect::Policy::limited(10) } else { reqwest::redirect::Policy::none() })
            .danger_accept_invalid_certs(!config.verify_ssl);
        
        // Configure passive scanning proxy if requested (优先级最高)
        if config.use_passive_proxy {
            client_builder = client_builder
                .proxy(reqwest::Proxy::http("http://127.0.0.1:4201")?)
                .proxy(reqwest::Proxy::https("http://127.0.0.1:4201")?);
        } else {
            // 如果未使用被动代理，则应用全局代理配置
            client_builder = crate::global_proxy::apply_proxy_to_client(client_builder).await;
        }
        
        let client = client_builder.build()?;

        let mut request_builder = client.request(method, &config.url);
        if !config.query_params.is_empty() { request_builder = request_builder.query(&config.query_params); }
        for (key, value) in &config.headers { request_builder = request_builder.header(key, value); }
        if let Some(body) = &config.body { request_builder = request_builder.body(body.clone()); }

        let request_start = std::time::Instant::now();
        match request_builder.send().await {
            Ok(response) => {
                let response_time_ms = request_start.elapsed().as_millis() as u64;
                let total_time_ms = start_time.elapsed().as_millis() as u64;
                let status_code = response.status().as_u16();
                let status_text = response.status().canonical_reason().unwrap_or("Unknown").to_string();
                let final_url = response.url().to_string();
                let mut response_headers = HashMap::new();
                for (name, value) in response.headers() { if let Ok(value_str) = value.to_str() { response_headers.insert(name.to_string(), value_str.to_string()); } }
                let content_type = response_headers.get("content-type").cloned();
                let content_length = response_headers.get("content-length").and_then(|s| s.parse().ok());
                let body = match response.text().await { Ok(text) => text, Err(e) => { error!("Failed to read response body: {}", e); format!("Error reading response body: {}", e) } };

                let http_response = HttpResponse { status_code, status_text, headers: response_headers, body, response_time_ms, final_url, content_length, content_type };
                info!("HTTP request completed: {} {} in {}ms", status_code, config.url, total_time_ms);
                Ok(HttpRequestResult { request: config, response: Some(http_response), error: None, success: status_code < 400, total_time_ms })
            }
            Err(e) => {
                let total_time_ms = start_time.elapsed().as_millis() as u64;
                let error_msg = self.categorize_error(&e);
                error!("HTTP request failed: {}", error_msg);
                Ok(HttpRequestResult { request: config, response: None, error: Some(error_msg), success: false, total_time_ms })
            }
        }
    }
}

#[async_trait]
impl UnifiedTool for HttpRequestTool {
    fn name(&self) -> &str { "http_request" }
    fn description(&self) -> &str { "通用HTTP客户端工具，支持自定义请求方法、请求头、参数等，用于API测试和Web服务交互" }
    fn category(&self) -> ToolCategory { ToolCategory::Utility }
    fn parameters(&self) -> &ToolParameters { &self.parameters }
    fn metadata(&self) -> &ToolMetadata { &self.metadata }
    async fn is_available(&self) -> bool { true }

    async fn execute(&self, params: ToolExecutionParams) -> Result<ToolExecutionResult> {
        let execution_id = params.execution_id.unwrap_or_else(Uuid::new_v4);
        let start_time = std::time::Instant::now();
        info!("Executing HTTP request tool with execution_id: {}", execution_id);

        let url = params.inputs.get("url").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("参数错误：缺少必需的URL参数"))?;
        if url.is_empty() { return Err(anyhow!("参数错误：URL不能为空")); }
        if !url.starts_with("http://") && !url.starts_with("https://") { return Err(anyhow!("参数错误：URL必须以http://或https://开头，当前URL: {}", url)); }

        let method = params.inputs.get("method").and_then(|v| v.as_str()).unwrap_or("GET");
        let headers = self.parse_headers(params.inputs.get("headers").unwrap_or(&json!({})));
        let query_params = self.parse_query_params(params.inputs.get("query_params").unwrap_or(&json!({})));
        let body = params.inputs.get("body").and_then(|v| v.as_str()).map(|s| s.to_string());
        let timeout_seconds = params.inputs.get("timeout_seconds").and_then(|v| v.as_u64()).unwrap_or(30);
        if timeout_seconds == 0 || timeout_seconds > 300 { return Err(anyhow!("参数错误：超时时间必须在1-300秒之间，当前值: {}秒", timeout_seconds)); }
        let follow_redirects = params.inputs.get("follow_redirects").and_then(|v| v.as_bool()).unwrap_or(true);
        let verify_ssl = params.inputs.get("verify_ssl").and_then(|v| v.as_bool()).unwrap_or(true);
        let use_passive_proxy = params.inputs.get("use_passive_proxy").and_then(|v| v.as_bool()).unwrap_or(false);

        let config = HttpRequestConfig { 
            url: url.to_string(), 
            method: method.to_string(), 
            headers, 
            query_params, 
            body, 
            timeout_seconds, 
            follow_redirects, 
            verify_ssl,
            use_passive_proxy,
        };

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
                        "content_type": result.response.as_ref().and_then(|r| r.content_type.clone()),
                        "error_details": result.error.clone()
                    }
                });

                if result.success {
                    info!("HTTP {} {} completed in {}ms", result.request.method, result.request.url, result.total_time_ms);
                } else {
                    error!("HTTP {} {} failed in {}ms", result.request.method, result.request.url, result.total_time_ms);
                }

                Ok(ToolExecutionResult { execution_id, tool_id: self.name().to_string(), tool_name: self.name().to_string(), success: result.success, output: result_json, error: result.error.clone(), execution_time_ms, metadata: HashMap::new(), status: if result.success { ExecutionStatus::Completed } else { ExecutionStatus::Failed }, started_at: chrono::Utc::now(), completed_at: Some(chrono::Utc::now()) })
            }
            Err(e) => {
                let execution_time_ms = start_time.elapsed().as_millis() as u64;
                let detailed_error = format!("HTTP请求工具执行失败：{}", e);
                error!("{}", detailed_error);
                let error_output = json!({
                    "success": false,
                    "error": detailed_error,
                    "error_type": "tool_execution_error",
                    "execution_time_ms": execution_time_ms,
                    "summary": { "status": "failed", "error_details": detailed_error }
                });
                Ok(ToolExecutionResult { execution_id, tool_id: self.name().to_string(), tool_name: self.name().to_string(), success: false, output: error_output, error: Some(detailed_error), execution_time_ms, metadata: HashMap::new(), started_at: chrono::Utc::now(), status: ExecutionStatus::Failed, completed_at: Some(chrono::Utc::now()) })
            }
        }
    }
}


