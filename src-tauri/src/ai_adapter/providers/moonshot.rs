
use crate::ai_adapter::types::*;
use crate::ai_adapter::error::{AiAdapterError, Result};
use crate::ai_adapter::http::SseParser;
use async_trait::async_trait;
use std::ops::Deref;
use crate::ai_adapter::providers::base::BaseProvider;
use futures_util::StreamExt;
use log::{debug, warn};

/// 规范化函数名称以满足 Moonshot/Kimi 要求：
/// 必须以字母开头，只能包含字母、数字、下划线和短横线
fn sanitize_function_name(original: &str) -> String {
    // 替换非法字符为下划线
    let mut cleaned: String = original
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();

    // 如果为空，设为默认名
    if cleaned.is_empty() {
        return "f".to_string();
    }

    // 确保首字符为字母
    if !cleaned.chars().next().unwrap_or('f').is_ascii_alphabetic() {
        cleaned.insert(0, 'f');
    }

    cleaned
}

#[derive(Debug)]
pub struct MoonshotProvider {
    base: BaseProvider,
}

impl Deref for MoonshotProvider {
    type Target = BaseProvider;
    
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl MoonshotProvider {
    /// Parse raw Moonshot stream chunk data
    /// This static method can be called by upper layers to parse provider-specific response format
    pub fn parse_stream_chunk(raw_content: &str) -> Result<Option<StreamChunk>> {
        // First, try to parse the raw content directly as JSON (for when it's already JSON)
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(raw_content) {
            if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
                if let Some(choice) = choices.first() {
                    let delta = choice.get("delta").unwrap_or(&serde_json::Value::Null);
                    let content = delta.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
                    let finish_reason = choice.get("finish_reason").and_then(|f| f.as_str()).map(|s| s.to_string());
                    
                    let chunk = StreamChunk {
                        id: json.get("id").and_then(|i| i.as_str()).unwrap_or("unknown").to_string(),
                        model: json.get("model").and_then(|m| m.as_str()).unwrap_or("").to_string(),
                        content,
                        finish_reason,
                        usage: None,
                    };
                    
                    debug!("Parsed Moonshot chunk: id='{}', content='{}', finish_reason={:?}", 
                           chunk.id, chunk.content, chunk.finish_reason);
                    
                    return Ok(Some(chunk));
                }
            }
        }
        
        // Handle SSE format data lines (compatible with OpenAI format)
        for line in raw_content.lines() {
            let line = line.trim();
            
            if line.starts_with("data: ") {
                let data = &line[6..]; // Remove "data: " prefix
                let data = data.trim();
                
                // Skip empty data lines
                if data.is_empty() {
                    continue;
                }
                
                if data == "[DONE]" {
                    return Ok(None);
                }
                
                // Try to parse JSON and handle possible parsing errors
                match serde_json::from_str::<serde_json::Value>(data) {
                    Ok(json) => {
                        if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
                            if let Some(choice) = choices.first() {
                                let delta = choice.get("delta").unwrap_or(&serde_json::Value::Null);
                                let content = delta.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
                                let finish_reason = choice.get("finish_reason").and_then(|f| f.as_str()).map(|s| s.to_string());
                                
                                let chunk = StreamChunk {
                                    id: json.get("id").and_then(|i| i.as_str()).unwrap_or("unknown").to_string(),
                                    model: json.get("model").and_then(|m| m.as_str()).unwrap_or("").to_string(),
                                    content,
                                    finish_reason,
                                    usage: None,
                                };
                                
                                debug!("Parsed Moonshot SSE chunk: id='{}', content='{}', finish_reason={:?}", 
                                       chunk.id, chunk.content, chunk.finish_reason);
                                
                                return Ok(Some(chunk));
                            }
                        }
                    }
                    Err(e) => {
                        // Log JSON parsing error but don't interrupt stream processing
                        log::debug!("Failed to parse SSE data as JSON: {} | Data: {}", e, data);
                        // Continue processing next line, don't return error
                        continue;
                    }
                }
            }
        }
        
        warn!("No valid chunk found in raw content: {}", raw_content);
        Ok(None)
    }

    /// 创建新的 Moonshot 提供商实例
    pub fn new(config: ProviderConfig) -> Result<Self> {
        let base = BaseProvider::new(
            "moonshot".to_string(),
            "1.0.0".to_string(),
            config,
        )?;
        
        Ok(Self { base })
    }
    
    /// 获取 API 基础 URL
    fn get_api_base(&self) -> String {
        self.base.get_api_base("https://api.moonshot.cn/v1")
    }
    
    /// 构建认证头
    pub(crate) fn build_auth_headers(&self) -> Result<reqwest::header::HeaderMap> {
        let mut headers = self.base.build_base_headers()?;
        
        // 添加 Bearer token 认证
        let auth_header = format!("Bearer {}", self.base.config.api_key);
        headers.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(&auth_header)
                .map_err(|e| AiAdapterError::ConfigurationError(e.to_string()))?,
        );
        
        Ok(headers)
    }
    
    /// 转换消息格式为 Moonshot API 格式
    pub(crate) fn convert_messages(&self, messages: &[Message]) -> Result<Vec<serde_json::Value>> {
        let mut converted = Vec::new();
        
        for message in messages {
            let mut msg = serde_json::json!({
                "role": message.role,
                "content": message.content
            });
            
            // Moonshot 支持工具调用（OpenAI风格）
            if let Some(tool_calls) = &message.tool_calls {
                let mapped_calls: Vec<serde_json::Value> = tool_calls
                    .iter()
                    .map(|tc| {
                        let sanitized = sanitize_function_name(&tc.name);
                        serde_json::json!({
                            "id": tc.id,
                            "type": "function",
                            "function": {
                                "name": sanitized,
                                "arguments": tc.arguments
                            }
                        })
                    })
                    .collect();
                msg["tool_calls"] = serde_json::Value::Array(mapped_calls);
            }
            
            // 处理工具调用 ID
            if let Some(tool_call_id) = &message.tool_call_id {
                msg["tool_call_id"] = serde_json::Value::String(tool_call_id.clone());
            }
            
            converted.push(msg);
        }
        
        Ok(converted)
    }
    
    /// 构建聊天请求体
    pub(crate) fn build_chat_request_body(&self, request: &ChatRequest) -> Result<serde_json::Value> {
        let messages = self.convert_messages(&request.messages)?;
        
        let mut body = serde_json::json!({
            "model": request.model,
            "messages": messages,
        });
        
        // 从选项中提取参数
        if let Some(options) = &request.options {
            if let Some(max_tokens) = options.max_tokens {
                body["max_tokens"] = serde_json::Value::Number(serde_json::Number::from(max_tokens));
            }
            
            if let Some(temperature) = options.temperature {
                body["temperature"] = serde_json::Value::Number(
                    serde_json::Number::from_f64(temperature as f64)
                        .ok_or_else(|| AiAdapterError::ConfigurationError("Invalid temperature value".to_string()))?
                );
            }
            
            if let Some(top_p) = options.top_p {
                body["top_p"] = serde_json::Value::Number(
                    serde_json::Number::from_f64(top_p as f64)
                        .ok_or_else(|| AiAdapterError::ConfigurationError("Invalid top_p value".to_string()))?
                );
            }
            
            if let Some(stream) = options.stream {
                body["stream"] = serde_json::Value::Bool(stream);
            }
            
            // 设置 stop sequences
            if let Some(stop) = &options.stop {
                if !stop.is_empty() {
                    body["stop"] = serde_json::to_value(stop)?;
                }
            }
        }
        
        // 添加其他参数
        if let Some(extra_params) = &request.extra_params {
            for (key, value) in extra_params {
                body[key] = value.clone();
            }
        }
        
        Ok(body)
    }
    
    /// 解析消息
    pub(crate) fn parse_message(&self, message_value: &serde_json::Value) -> Result<Message> {
        let role_str = message_value["role"].as_str()
            .unwrap_or("assistant");
        
        let role = match role_str {
            "user" => MessageRole::User,
            "assistant" => MessageRole::Assistant,
            "system" => MessageRole::System,
            "tool" => MessageRole::Tool,
            _ => MessageRole::Assistant,
        };
        
        let content = message_value["content"].as_str()
            .unwrap_or("")
            .to_string();
        
        // 解析工具调用（OpenAI风格）
        let tool_calls = if let Some(tool_calls_array) = message_value.get("tool_calls").and_then(|v| v.as_array()) {
            let mut parsed_tool_calls = Vec::new();
            for tool_call in tool_calls_array {
                if let (Some(id), Some(function)) = (
                    tool_call["id"].as_str(),
                    tool_call.get("function").and_then(|f| f.as_object())
                ) {
                    if let (Some(name), Some(arguments)) = (
                        function.get("name").and_then(|n| n.as_str()),
                        function.get("arguments").and_then(|a| a.as_str())
                    ) {
                        parsed_tool_calls.push(ToolCall {
                            id: id.to_string(),
                            name: name.to_string(),
                            arguments: arguments.to_string(),
                        });
                    }
                }
            }
            if parsed_tool_calls.is_empty() { None } else { Some(parsed_tool_calls) }
        } else { None };
        
        Ok(Message {
            role,
            content,
            name: message_value.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()),
            tool_calls,
            tool_call_id: message_value.get("tool_call_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
        })
    }
    
    /// 解析使用情况
    pub(crate) fn parse_usage(&self, usage_value: &serde_json::Value) -> Option<Usage> {
        if usage_value.is_null() {
            return None;
        }
        
        Some(Usage {
            prompt_tokens: usage_value["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: usage_value["completion_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: usage_value["total_tokens"].as_u64().unwrap_or(0) as u32,
        })
    }
    
    /// 解析选择
    pub(crate) fn parse_choices(&self, choices_value: &serde_json::Value) -> Result<Vec<Choice>> {
        let mut choices = Vec::new();
        
        if let Some(choices_array) = choices_value.as_array() {
            for choice_value in choices_array {
                let index = choice_value["index"].as_u64().unwrap_or(0) as u32;
                let message = self.parse_message(&choice_value["message"])?;
                let finish_reason = choice_value["finish_reason"].as_str()
                    .map(|s| s.to_string());
                
                choices.push(Choice {
                    index,
                    message,
                    finish_reason,
                });
            }
        }
        
        Ok(choices)
    }
    
    /// 解析聊天响应
    fn parse_chat_response(
        &self,
        response: &serde_json::Value,
        _request_info: Option<HttpRequest>,
        _response_info: Option<HttpResponse>,
    ) -> Result<ChatResponse> {
        let id = response["id"].as_str().unwrap_or("").to_string();
        let model = response["model"].as_str().unwrap_or("").to_string();
        
        // 解析第一个 choice 的消息
        let message = if let Some(choices) = response["choices"].as_array() {
            if let Some(choice) = choices.first() {
                self.parse_message(&choice["message"])?
            } else {
                Message::assistant("")
            }
        } else {
            Message::assistant("")
        };
        
        let usage = self.parse_usage(&response["usage"]);
        let finish_reason = response["choices"].as_array()
            .and_then(|choices| choices.first())
            .and_then(|choice| choice["finish_reason"].as_str())
            .map(|s| s.to_string());
        
        let choices = self.parse_choices(&response["choices"])?;
        
        Ok(ChatResponse {
            id,
            model,
            message,
            usage,
            finish_reason,
            created_at: std::time::SystemTime::now(),
            choices,
        })
    }

}

#[async_trait]
impl AiProvider for MoonshotProvider {
    fn name(&self) -> &str {
        "moonshot"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn supported_models(&self) -> Vec<String> {
        vec![
            "moonshot-v1-8k".to_string(),
            "moonshot-v1-32k".to_string(),
            "moonshot-v1-128k".to_string(),
        ]
    }
    
    fn build_chat_request(&self, request: &ChatRequest) -> Result<serde_json::Value> {
        let mut body = serde_json::json!({
            "model": request.model,
            "messages": self.convert_messages(&request.messages)?,
        });
        
        // 添加可选参数
        if let Some(options) = &request.options {
            if let Some(temperature) = options.temperature {
                body["temperature"] = serde_json::json!(temperature);
            }
            if let Some(max_tokens) = options.max_tokens {
                body["max_tokens"] = serde_json::json!(max_tokens);
            }
            if let Some(top_p) = options.top_p {
                body["top_p"] = serde_json::json!(top_p);
            }
            if let Some(stop) = &options.stop {
                body["stop"] = serde_json::json!(stop);
            }
        }
        
        // 添加工具（Moonshot支持工具调用）
        if let Some(tools) = &request.tools {
            if !tools.is_empty() {
                let converted_tools: Vec<serde_json::Value> = tools
                    .iter()
                    .map(|tool| {
                        serde_json::json!({
                            "type": "function",
                            "function": {
                                "name": sanitize_function_name(&tool.name),
                                "description": tool.description,
                                "parameters": tool.parameters
                            }
                        })
                    })
                    .collect();
                body["tools"] = serde_json::json!(converted_tools);
            }
        }
        
        Ok(body)
    }
    
    async fn test_connection(&self) -> Result<bool> {
        debug!("Testing Moonshot connection");
        
        // 创建一个简单的测试请求
        let test_request = ChatRequest {
            model: "moonshot-v1-8k".to_string(),
            messages: vec![Message::user("Hello")],
            tools: None,
            tool_choice: None,
            user: None,
            extra_params: None,
            options: Some(ChatOptions {
                temperature: Some(0.1),
                max_tokens: Some(5),
                top_p: None,
                frequency_penalty: None,
                presence_penalty: None,
                stop: None,
                stream: Some(false),
            }),
        };
        
        match self.send_chat_request(&test_request).await {
            Ok(_) => {
                debug!("Moonshot connection test successful");
                Ok(true)
            }
            Err(e) => {
                warn!("Moonshot connection test failed: {}", e);
                Ok(false)
            }
        }
    }
    
    async fn send_chat_request(&self, request: &ChatRequest) -> Result<ChatResponse> {
        debug!("Sending Moonshot chat request for model: {}", request.model);
        
        let url = format!("{}/chat/completions", self.get_api_base());
        let headers = self.build_auth_headers()?;
        let body = self.build_chat_request_body(request)?;
        
        let operation = || async {
            // 转换headers格式
            let mut header_map = std::collections::HashMap::new();
            for (name, value) in headers.iter() {
                header_map.insert(name.to_string(), value.to_str().unwrap_or("").to_string());
            }
            
            let response_json = self.http_client.post_json(&url, &body, Some(header_map)).await?;
            self.parse_chat_response(&response_json, None, None)
        };
        
        self.execute_with_retry(operation).await
    }
    
    async fn send_chat_stream(&self, request: &ChatRequest) -> Result<ChatStreamResponse> {
        debug!("Sending Moonshot streaming chat request for model: {}", request.model);
        
        let url = format!("{}/chat/completions", self.get_api_base());
        let headers = self.build_auth_headers()?;
        
        // 构建流式请求体
        let mut stream_request = request.clone();
        if let Some(ref mut options) = stream_request.options {
            options.stream = Some(true);
        } else {
            stream_request.options = Some(ChatOptions {
                stream: Some(true),
                ..Default::default()
            });
        }
        let body = self.build_chat_request_body(&stream_request)?;
        
        // 转换headers格式
        let mut header_map = std::collections::HashMap::new();
        for (name, value) in headers.iter() {
            header_map.insert(name.to_string(), value.to_str().unwrap_or("").to_string());
        }
        
        let stream = self.http_client.post_stream(&url, &body, Some(header_map)).await?;
        
        // 使用SSE解析器处理流，但只做基本的SSE解析，返回原始JSON数据
        let sse_stream = SseParser::new(stream);
        let model_name = request.model.clone();
        
        let processed_stream = sse_stream.filter_map(move |result| {
            let model_name = model_name.clone();
            futures::future::ready(match result {
                Ok(sse_event) => {
                    if sse_event.event_type.as_deref() == Some("done") || sse_event.data == "[DONE]" {
                        Some(Ok(StreamChunk {
                            id: "finish".to_string(),
                            model: model_name.clone(),
                            content: String::new(),
                            finish_reason: Some("stop".to_string()),
                            usage: None,
                        }))
                    } else if !sse_event.data.trim().is_empty() {
                        // 返回原始JSON数据作为content，让上层调用者解析
                        // 这样可以保持provider的简洁性，将具体的解析逻辑移到调用层
                        Some(Ok(StreamChunk {
                            id: "raw".to_string(),
                            model: model_name.clone(),
                            content: sse_event.data.clone(), // 原始JSON数据
                            finish_reason: None,
                            usage: None,
                        }))
                    } else {
                        None // 跳过空数据
                    }
                },
                Err(e) => Some(Err(e))
            })
        });
        
        Ok(ChatStream {
            stream: Box::new(processed_stream),
            request_info: None,
            response_info: None,
        })
    }
    
    fn get_last_request_info(&self) -> Option<HttpRequestInfo> {
        self.base.get_last_request_info()
    }
    
    fn get_last_response_info(&self) -> Option<HttpResponseInfo> {
        self.base.get_last_response_info()
    }
    
    fn parse_stream(&self, chunk: &str) -> Result<Option<StreamChunk>> {
        // 处理SSE格式的数据行（与OpenAI兼容格式）
        for line in chunk.lines() {
            let line = line.trim(); // 去除行首尾空白字符
            
            if line.starts_with("data: ") {
                let data = &line[6..]; // 移除"data: "前缀
                let data = data.trim(); // 去除数据部分的空白字符
                
                // 跳过空数据行
                if data.is_empty() {
                    continue;
                }
                
                if data == "[DONE]" {
                    return Ok(None);
                }
                
                // 尝试解析JSON，并处理可能的解析错误
                match serde_json::from_str::<serde_json::Value>(data) {
                    Ok(json) => {
                        if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
                            if let Some(choice) = choices.first() {
                                let delta = choice.get("delta").unwrap_or(&serde_json::Value::Null);
                                let content = delta.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
                                let finish_reason = choice.get("finish_reason").and_then(|f| f.as_str()).map(|s| s.to_string());
                                
                                let chunk = StreamChunk {
                                    id: json.get("id").and_then(|i| i.as_str()).unwrap_or("unknown").to_string(),
                                    model: json.get("model").and_then(|m| m.as_str()).unwrap_or("").to_string(),
                                    content,
                                    finish_reason,
                                    usage: None,
                                };
                                
                                return Ok(Some(chunk));
                            }
                        }
                    }
                    Err(e) => {
                        // 记录JSON解析错误，但不中断流处理
                        log::debug!("Failed to parse SSE data as JSON: {} | Data: {}", e, data);
                        // 继续处理下一行，不返回错误
                        continue;
                    }
                }
            }
        }
        
        Ok(None)
    }
}


