//! OpenAI提供商实现

use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use futures_util::StreamExt;

use crate::ai_adapter::types::*;
use crate::ai_adapter::error::{AiAdapterError, Result};
use crate::ai_adapter::core::BaseProvider;
use crate::models::ai::MessageRole;
use crate::ai_adapter::http::HttpClient;

/// OpenAI提供商
#[derive(Debug)]
pub struct OpenAiProvider {
    base: BaseProvider,
}

impl OpenAiProvider {
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
    
    /// 构建聊天请求
    fn build_chat_request(&self, request: &ChatRequest) -> Result<Value> {
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
            
            let content_str = match &message.content {
                MessageContent::Text(text) => text.clone(),
                MessageContent::Image(_) => {
                    return Err(AiAdapterError::ValidationError(
                        "Image content not yet supported".to_string(),
                    ))
                }
                MessageContent::Mixed(_) => {
                    return Err(AiAdapterError::ValidationError(
                        "Mixed content not yet supported".to_string(),
                    ))
                }
            };
            
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
            content: MessageContent::Text(content),
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
    
    async fn test_connection(&self) -> Result<bool> {
        let url = format!("{}/models", self.base.get_api_base("https://api.openai.com/v1"));
        let headers = self.base.build_auth_headers();
        
        match self.base.http_client.get(&url, Some(headers)).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    async fn send_chat_request(&self, request: &ChatRequest) -> Result<ChatResponse> {
        let url = format!("{}/chat/completions", self.base.get_api_base("https://api.openai.com/v1"));
        let headers = self.base.build_auth_headers();
        let body = self.build_chat_request(&request)?;
        
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
        let mut body = self.build_chat_request(&request)?;
        body["stream"] = json!(true);
        
        let stream = self.base.execute_with_retry(|| {
            let url = url.clone();
            let headers = headers.clone();
            let body = body.clone();
            async move {
                self.base.http_client.post_stream(&url, &body, Some(headers)).await
            }
        }).await?;
        
        // 转换流
        let mapped_stream = stream.map(|result| {
            match result {
                Ok(chunk) => {
                    // 解析SSE数据
                    if chunk.starts_with(b"data: ") {
                        let data = &chunk[6..];
                        if data == b"[DONE]" {
                            return Ok(StreamChunk {
                                 id: "".to_string(),
                                 model: "".to_string(),
                                 content: "".to_string(),
                                 usage: None,
                                 finish_reason: Some("stop".to_string()),
                             });
                        }
                        
                        match serde_json::from_str::<Value>(std::str::from_utf8(data).unwrap_or("")) {
                            Ok(json) => {
                                let empty_choices = vec![];
                                let choices = json["choices"].as_array().unwrap_or(&empty_choices);
                                if let Some(choice) = choices.first() {
                                    let delta = &choice["delta"];
                                    let content = delta["content"].as_str().unwrap_or("").to_string();
                                    
                                    Ok(StreamChunk {
                                         id: json["id"].as_str().unwrap_or("").to_string(),
                                         model: json["model"].as_str().unwrap_or("").to_string(),
                                         content,
                                         usage: None,
                                         finish_reason: choice["finish_reason"].as_str().map(|s| s.to_string()),
                                     })
                                } else {
                                    Err(AiAdapterError::StreamError("Empty choices in stream chunk".to_string()))
                                }
                            },
                            Err(e) => Err(AiAdapterError::SerializationError(e.to_string()))
                        }
                    } else {
                        Err(AiAdapterError::StreamError("Invalid SSE format".to_string()))
                    }
                },
                Err(e) => Err(e)
            }
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