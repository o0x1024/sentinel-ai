//! OpenAI提供商实现
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use futures_util::StreamExt;
use log::{debug, warn};

use crate::ai_adapter::types::*;
use crate::ai_adapter::error::{AiAdapterError, Result};
use crate::ai_adapter::core::BaseProvider;
use crate::models::ai::MessageRole;
use crate::ai_adapter::http::{HttpClient, SseParser};

/// OpenAI提供商
#[derive(Debug)]
pub struct OpenAiProvider {
    base: BaseProvider,
}

impl OpenAiProvider {
    /// Parse raw OpenAI stream chunk data
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
                    
                    debug!("Parsed OpenAI chunk: id='{}', content='{}', finish_reason={:?}", 
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
                                
                                debug!("Parsed OpenAI SSE chunk: id='{}', content='{}', finish_reason={:?}", 
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

    /// 创建新的OpenAI提供商实例
    pub fn new(config: ProviderConfig) -> Result<Self> {
        // 验证配置
        if config.api_key.is_empty() {
            return Err(AiAdapterError::ConfigurationError(
                "OpenAI API key is required".to_string()
            ));
        }
        
        let models = vec![
            "gpt-4".to_string(),
            "gpt-4-turbo".to_string(),
            "gpt-3.5-turbo".to_string(),
            "gpt-4o".to_string(),
            "gpt-4o-mini".to_string(),
        ];
        
        let base = BaseProvider::new(
            "openai".to_string(),
            "1.0.0".to_string(),
            config,
            vec!["gpt-4".to_string(), "gpt-3.5-turbo".to_string()],
            true,
            true
        )?;
        
        Ok(Self { base })
    }
    
    /// 构建API请求体
    fn build_api_request(&self, request: &ChatRequest) -> Result<Value> {
        let mut body = json!({
            "model": request.model,
            "messages": self.convert_messages(&request.messages)?,
        });
        
        // 添加可选参数
        if let Some(options) = &request.options {
            if let Some(temperature) = options.temperature {
                body["temperature"] = json!(temperature);
            }
            if let Some(max_tokens) = options.max_tokens {
                body["max_tokens"] = json!(max_tokens);
            }
            if let Some(top_p) = options.top_p {
                body["top_p"] = json!(top_p);
            }
            if let Some(stop) = &options.stop {
                body["stop"] = json!(stop);
            }
        }
        
        // 添加工具
        if let Some(tools) = &request.tools {
            if !tools.is_empty() {
                body["tools"] = json!(self.convert_tools(tools)?);
            }
        }
        
        Ok(body)
    }
    
    /// 转换消息格式
    fn convert_messages(&self, messages: &[Message]) -> Result<Value> {
        let mut converted = Vec::new();
        
        for message in messages {
            let role_str = match message.role {
                MessageRole::System => "system",
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::Tool => "tool",
            };
            
                let content_str = &message.content;
            
            let mut msg = json!({
                "role": role_str,
                "content": content_str
            });
            
            // 添加工具调用
            if let Some(tool_calls) = &message.tool_calls {
                if !tool_calls.is_empty() {
                    msg["tool_calls"] = json!(self.convert_tool_calls(tool_calls)?);
                }
            }
            
            // 添加工具调用ID（用于工具响应）
            if let Some(tool_call_id) = &message.tool_call_id {
                msg["tool_call_id"] = json!(tool_call_id);
            }
            
            converted.push(msg);
        }
        
        Ok(json!(converted))
    }
    
    /// 转换工具定义
    fn convert_tools(&self, tools: &[Tool]) -> Result<Value> {
        let mut converted = Vec::new();
        
        for tool in tools {
            converted.push(json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.parameters
                }
            }));
        }
        
        Ok(json!(converted))
    }
    
    /// 转换工具调用
    fn convert_tool_calls(&self, tool_calls: &[ToolCall]) -> Result<Value> {
        let mut converted = Vec::new();
        
        for tool_call in tool_calls {
            converted.push(json!({
                "id": tool_call.id,
                "type": "function",
                "function": {
                    "name": tool_call.name,
                    "arguments": tool_call.arguments
                }
            }));
        }
        
        Ok(json!(converted))
    }
    
    /// 解析聊天响应
    fn parse_chat_response(&self, response: Value) -> Result<ChatResponse> {
        let choices = response["choices"].as_array()
            .ok_or_else(|| AiAdapterError::SerializationError(
                "Missing choices in response".to_string()
            ))?;
        
        if choices.is_empty() {
            return Err(AiAdapterError::SerializationError(
                "Empty choices in response".to_string()
            ));
        }
        
        let choice = &choices[0];
        let message = &choice["message"];
        
        let content = message["content"].as_str().unwrap_or("").to_string();
        let role = message["role"].as_str().unwrap_or("assistant").to_string();
        
        // 解析工具调用
        let mut tool_calls = Vec::new();
        if let Some(calls) = message["tool_calls"].as_array() {
            for call in calls {
                if let (Some(id), Some(function)) = (
                    call["id"].as_str(),
                    call["function"].as_object()
                ) {
                    tool_calls.push(ToolCall {
                        id: id.to_string(),
                        name: function["name"].as_str().unwrap_or("").to_string(),
                        arguments: function["arguments"].as_str().unwrap_or("{}").to_string(),
                    });
                }
            }
        }
        
        // 解析使用情况
        let usage = if let Some(usage_obj) = response["usage"].as_object() {
            Some(Usage {
                prompt_tokens: usage_obj["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: usage_obj["completion_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: usage_obj["total_tokens"].as_u64().unwrap_or(0) as u32,
            })
        } else {
            None
        };
        
        let message_role = match role.as_str() {
            "system" => MessageRole::System,
            "user" => MessageRole::User,
            "assistant" => MessageRole::Assistant,
            "tool" => MessageRole::Tool,
            _ => MessageRole::Assistant,
        };
        
        let message = Message {
            role: message_role,
            content,
            name: None,
            tool_calls: if tool_calls.is_empty() { None } else { Some(tool_calls) },
            tool_call_id: None,
        };
        
        Ok(ChatResponse {
            id: response["id"].as_str().unwrap_or("").to_string(),
            model: response["model"].as_str().unwrap_or("").to_string(),
            message: message.clone(),
            choices: vec![Choice {
                index: 0,
                message,
                finish_reason: choice["finish_reason"].as_str().map(|s| s.to_string()),
            }],
            usage,
            finish_reason: choice["finish_reason"].as_str().map(|s| s.to_string()),
            created_at: std::time::SystemTime::now(),
        })
    }


    fn parse_sse_chunk(chunk_str: &str) -> Result<Option<StreamChunk>> {
        debug!("Parsing ModelScope SSE chunk: {}", chunk_str);
        
        let mut accumulated_content = String::new();
        let mut last_chunk_info = None;
        let mut has_finish_reason = false;
        
        // 处理SSE格式的数据行（与OpenAI兼容格式）
        for line in chunk_str.lines() {
            let line = line.trim(); // 去除行首尾空白字符
            
            if line.starts_with("data: ") {
                let data = &line[6..]; // 移除"data: "前缀
                let data = data.trim(); // 去除数据部分的空白字符
                
                // 跳过空数据行
                if data.is_empty() {
                    continue;
                }
                
                // 检查结束标记
                if data == "[DONE]" {
                    debug!("ModelScope stream finished with [DONE]");
                    return Ok(None); // 返回None表示流结束
                }
                
                // 尝试解析JSON
                match serde_json::from_str::<Value>(data) {
                    Ok(json) => {
                        debug!("ModelScope JSON parsed: {}", json);
                        
                        // 解析choices数组
                        if let Some(choices) = json["choices"].as_array() {
                            for choice in choices {
                                let delta = &choice["delta"];
                                
                                // ModelScope可能在多个字段中返回内容
                                let mut content = String::new();
                                
                                // 1. 检查标准content字段
                                if let Some(content_str) = delta["content"].as_str() {
                                    content.push_str(content_str);
                                }
                                
                                // 2. 检查reasoning_content字段（ModelScope特有）
                                if let Some(reasoning_content) = delta["reasoning_content"].as_str() {
                                    if !content.is_empty() {
                                        content.push('\n'); // 如果已有内容，添加换行分隔
                                    }
                                    content.push_str(reasoning_content);
                                    debug!("ModelScope using reasoning_content: '{}'", reasoning_content);
                                }
                                
                                // 3. 检查function_call字段（如果有工具调用）
                                if let Some(function_call) = delta["function_call"].as_object() {
                                    if let Some(arguments) = function_call["arguments"].as_str() {
                                        if !content.is_empty() {
                                            content.push('\n');
                                        }
                                        content.push_str(arguments);
                                        debug!("ModelScope using function_call arguments: '{}'", arguments);
                                    }
                                }
                                
                                if !content.is_empty() {
                                    accumulated_content.push_str(&content);
                                    debug!("ModelScope delta content accumulated: '{}', total_len: {}", content, accumulated_content.len());
                                }
                                
                                // 检查完成原因
                                if let Some(finish_reason_str) = choice["finish_reason"].as_str() {
                                    has_finish_reason = true;
                                    debug!("ModelScope finish_reason: {}", finish_reason_str);
                                }
                                
                                // 保存块信息用于最终构建
                                last_chunk_info = Some((
                                    json["id"].as_str().unwrap_or("unknown").to_string(),
                                    json["model"].as_str().unwrap_or("").to_string(),
                                    choice["finish_reason"].as_str().map(|s| s.to_string()),
                                    if let Some(usage_obj) = json["usage"].as_object() {
                                        Some(Usage {
                                            prompt_tokens: usage_obj["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                                            completion_tokens: usage_obj["completion_tokens"].as_u64().unwrap_or(0) as u32,
                                            total_tokens: usage_obj["total_tokens"].as_u64().unwrap_or(0) as u32,
                                        })
                                    } else {
                                        None
                                    }
                                ));
                            }
                        } else {
                            // 如果没有choices，检查是否有直接的content字段（某些接口可能这样返回）
                            if let Some(direct_content) = json["content"].as_str() {
                                accumulated_content.push_str(direct_content);
                                debug!("ModelScope using direct content: '{}'", direct_content);
                                
                                last_chunk_info = Some((
                                    json["id"].as_str().unwrap_or("unknown").to_string(),
                                    json["model"].as_str().unwrap_or("").to_string(),
                                    json["finish_reason"].as_str().map(|s| s.to_string()),
                                    None
                                ));
                            } else {
                                debug!("ModelScope chunk without choices or content (metadata): {}", json);
                            }
                        }
                    }
                    Err(e) => {
                        // 记录JSON解析错误，但不中断流处理
                        debug!("Failed to parse ModelScope SSE data as JSON: {} | Data: {}", e, data);
                        continue; // 继续处理下一行
                    }
                }
            }
        }
        
        // 构建最终的chunk，只有在有内容或有完成原因时才返回
        if !accumulated_content.is_empty() || has_finish_reason {
            if let Some((id, model, finish_reason, usage)) = last_chunk_info {
                let chunk = StreamChunk {
                    id,
                    model,
                    content: accumulated_content,
                    usage,
                    finish_reason,
                };
                
                return Ok(Some(chunk));
            }
        }
        
        // 如果没有找到有效的数据，返回空值
        debug!("ModelScope chunk processed, no valid content found");
        Ok(None)
    }
    
}

#[async_trait]
impl AiProvider for OpenAiProvider {
    fn name(&self) -> &str {
        &self.base.name
    }
    
    fn version(&self) -> &str {
        &self.base.version
    }
    
    fn supported_models(&self) -> Vec<String> {
        self.base.models.clone()
    }
    
    fn build_chat_request(&self, request: &ChatRequest) -> Result<serde_json::Value> {
        self.build_api_request(request)
    }
    
    async fn test_connection(&self) -> Result<bool> {
        let url = format!("{}/models", self.base.get_api_base("https://api.openai.com/v1"));
        let headers = self.base.build_auth_headers();
        
        match self.base.http_client.get(&url, Some(headers)).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    fn parse_stream(&self, chunk: &str) -> Result<Option<StreamChunk>> {
        // 使用统一的SSE解析方法（OpenAI兼容）
        Self::parse_sse_chunk(chunk)
    }

    async fn send_chat_request(&self, request: &ChatRequest) -> Result<ChatResponse> {
        let url = format!("{}/chat/completions", self.base.get_api_base("https://api.openai.com/v1"));
        let headers = self.base.build_auth_headers();
        let body = self.build_api_request(&request)?;
        
        let response = self.base.execute_with_retry(|| {
            let url = url.clone();
            let headers = headers.clone();
            let body = body.clone();
            async move {
                self.base.http_client.post_json(&url, &body, Some(headers)).await
            }
        }).await?;
        
        self.parse_chat_response(response)
    }
    
    async fn send_chat_stream(&self, request: &ChatRequest) -> Result<ChatStream> {
        let url = format!("{}/chat/completions", self.base.get_api_base("https://api.openai.com/v1"));
        let headers = self.base.build_auth_headers();
        let mut body = self.build_api_request(&request)?;
        body["stream"] = json!(true);
        
        let stream = self.base.execute_with_retry(|| {
            let url = url.clone();
            let headers = headers.clone();
            let body = body.clone();
            async move {
                self.base.http_client.post_stream(&url, &body, Some(headers)).await
            }
        }).await?;
        
        // 创建SSE解析器流
        let sse_stream = SseParser::new(stream);
        
        // 转换为ChatStream
        let mapped_stream = sse_stream.filter_map(|result| {
            futures::future::ready(match result {
                Ok(sse_event) => {
                    if sse_event.event_type.as_deref() == Some("done") || sse_event.data == "[DONE]" {
                        Some(Ok(StreamChunk {
                            id: "".to_string(),
                            model: "".to_string(),
                            content: "".to_string(),
                            usage: None,
                            finish_reason: Some("stop".to_string()),
                        }))
                    } else if !sse_event.data.is_empty() {
                        match serde_json::from_str::<Value>(&sse_event.data) {
                            Ok(json) => {
                                let empty_choices = vec![];
                                let choices = json["choices"].as_array().unwrap_or(&empty_choices);
                                if let Some(choice) = choices.first() {
                                    let delta = &choice["delta"];
                                    let content = delta["content"].as_str().unwrap_or("").to_string();
                                    
                                    Some(Ok(StreamChunk {
                                        id: json["id"].as_str().unwrap_or("").to_string(),
                                        model: json["model"].as_str().unwrap_or("").to_string(),
                                        content,
                                        usage: None,
                                        finish_reason: choice["finish_reason"].as_str().map(|s| s.to_string()),
                                    }))
                                } else {
                                    None // 跳过空选择
                                }
                            },
                            Err(e) => Some(Err(AiAdapterError::SerializationError(e.to_string())))
                        }
                    } else {
                        None // 跳过空数据
                    }
                },
                Err(e) => Some(Err(e))
            })
        });
        
        Ok(ChatStream {
            stream: Box::new(mapped_stream),
            request_info: self.base.get_last_request_info(),
            response_info: self.base.get_last_response_info(),
        })
    }
    
    fn get_last_request_info(&self) -> Option<HttpRequest> {
        self.base.get_last_request_info()
    }
    
    fn get_last_response_info(&self) -> Option<HttpResponse> {
        self.base.get_last_response_info()
    }

}