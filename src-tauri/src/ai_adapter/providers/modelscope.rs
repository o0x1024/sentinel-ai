//! ModelScope提供商实现
//!
//! ModelScope API推理服务的适配器实现

use async_trait::async_trait;
use futures_util::StreamExt;
use serde_json::{json, Value};
use tracing::{info, warn};
use std::collections::HashMap;


use crate::ai_adapter::core::BaseProvider;
use crate::ai_adapter::error::{AiAdapterError, Result};
use crate::ai_adapter::http::{HttpClient, SseParser};
use crate::ai_adapter::types::*;
use crate::models::ai::MessageRole;

/// ModelScope提供商
#[derive(Debug)]
pub struct ModelScopeProvider {
    base: BaseProvider,
}

impl ModelScopeProvider {
    /// 创建新的ModelScope提供商实例
    pub fn new(config: ProviderConfig) -> Result<Self> {
        // 验证配置
        if config.api_key.is_empty() {
            return Err(AiAdapterError::ConfigurationError(
                "ModelScope API key is required".to_string(),
            ));
        }

        // ModelScope支持的热门模型
        let models = vec![
        ];

        let mut final_config = config;

        // 设置默认API基础URL
        if final_config.api_base.is_none() {
            final_config.api_base = Some("https://api-inference.modelscope.cn/v1/".to_string());
        }

        let base = BaseProvider::new(
            "modelscope".to_string(),
            "1.0.0".to_string(),
            final_config,
            models,
            true,  // 支持流式
            false, // 暂不支持工具调用
        )?;

        Ok(Self { base })
    }


        /// 获取API基础URL
        fn get_api_base(&self) -> String {
            self.base.get_api_base("https://api-inference.modelscope.cn/v1")
        }
        
         /// 构建认证头
    pub(crate) fn build_auth_headers(&self) -> Result<std::collections::HashMap<String, String>> {
        let mut headers = std::collections::HashMap::new();
        
        // 添加基础头
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("Accept".to_string(), "application/json".to_string());
        
        // 添加Bearer token认证
        let auth_header = format!("Bearer {}", self.base.config.api_key);
        headers.insert("Authorization".to_string(), auth_header);
        
        // 添加ModelScope特定的额外头信息（可选）
        if let Some(extra_headers) = &self.base.config.extra_headers {
            if let Some(referer) = extra_headers.get("HTTP-Referer") {
                headers.insert("HTTP-Referer".to_string(), referer.clone());
            }
            if let Some(title) = extra_headers.get("X-Title") {
                headers.insert("X-Title".to_string(), title.clone());
            }
        }
        
        Ok(headers)
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
            if let Some(frequency_penalty) = options.frequency_penalty {
                body["frequency_penalty"] = json!(frequency_penalty);
            }
            if let Some(presence_penalty) = options.presence_penalty {
                body["presence_penalty"] = json!(presence_penalty);
            }
            if let Some(stop) = &options.stop {
                body["stop"] = json!(stop);
            }
        }


        // 添加用户标识符（可选）
        if let Some(user) = &request.user {
            body["user"] = json!(user);
        }

        // 添加额外的提供商特定参数
        if let Some(extra_params) = &request.extra_params {
            for (key, value) in extra_params {
                body[key] = value.clone();
            }
        }

        Ok(body)
    }

    /// 转换消息格式为ModelScope格式
    fn convert_messages(&self, messages: &[Message]) -> Result<Value> {
        let mut converted = Vec::new();

        for message in messages {
            let role_str = match message.role {
                MessageRole::System => "system",
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::Tool => {
                    // ModelScope暂不支持工具消息，跳过或转换为用户消息
                    continue;
                }
            };

            let mut msg = json!({
                "role": role_str,
                "content": message.content
            });

            // 如果有名称，添加name字段
            if let Some(name) = &message.name {
                msg["name"] = json!(name);
            }

            converted.push(msg);
        }

        Ok(json!(converted))
    }

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
        let role_str = message["role"].as_str().unwrap_or("assistant");
        
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
        
        let message_role = match role_str {
            "system" => MessageRole::System,
            "user" => MessageRole::User,
            "assistant" => MessageRole::Assistant,
            "tool" => MessageRole::Tool,
            _ => MessageRole::Assistant,
        };
        
        let message_obj = Message {
            role: message_role,
            content: content.clone(),
            name: message["name"].as_str().map(|s| s.to_string()),
            tool_calls: if tool_calls.is_empty() { None } else { Some(tool_calls) },
            tool_call_id: message["tool_call_id"].as_str().map(|s| s.to_string()),
        };
        
        Ok(ChatResponse {
            id: response["id"].as_str().unwrap_or("").to_string(),
            model: response["model"].as_str().unwrap_or("").to_string(),
            message: message_obj.clone(),
            choices: vec![Choice {
                index: choice["index"].as_u64().unwrap_or(0) as u32,
                message: message_obj,
                finish_reason: choice["finish_reason"].as_str().map(|s| s.to_string()),
            }],
            usage,
            finish_reason: choice["finish_reason"].as_str().map(|s| s.to_string()),
            created_at: std::time::SystemTime::now(),
        })
    }
    
         /// 解析流式响应块（静态方法）
    fn parse_chunk_data(chunk_str: &str) -> Result<StreamChunk> {
        // 解析SSE数据格式，支持多行数据
        for line in chunk_str.lines() {
            let line = line.trim(); // 去除行首尾空白字符
            
            if line.starts_with("data: ") {
                let data = &line[6..]; // 移除"data: "前缀
                let data = data.trim(); // 去除数据部分的空白字符
                
                // 跳过空数据行
                if data.is_empty() {
                    continue;
                }
                
                if data == "[DONE]" {
                    return Ok(StreamChunk {
                        id: "".to_string(),
                        model: "".to_string(),
                        content: "".to_string(),
                        usage: None,
                        finish_reason: Some("stop".to_string()),
                    });
                }
                
                // 尝试解析JSON，并处理可能的解析错误
                match serde_json::from_str::<Value>(data) {
                    Ok(json) => {
                        let empty_vec = vec![];
                        let choices = json["choices"].as_array().unwrap_or(&empty_vec);
                        if let Some(choice) = choices.first() {
                            let delta = &choice["delta"];
                            let content = delta["content"].as_str().unwrap_or("").to_string();
                            
                            // 解析使用情况（如果存在）
                            let usage = if let Some(usage_obj) = json["usage"].as_object() {
                                Some(Usage {
                                    prompt_tokens: usage_obj["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                                    completion_tokens: usage_obj["completion_tokens"].as_u64().unwrap_or(0) as u32,
                                    total_tokens: usage_obj["total_tokens"].as_u64().unwrap_or(0) as u32,
                                })
                            } else {
                                None
                            };
                            
                            return Ok(StreamChunk {
                                id: json["id"].as_str().unwrap_or("").to_string(),
                                model: json["model"].as_str().unwrap_or("").to_string(),
                                content,
                                usage,
                                finish_reason: choice["finish_reason"].as_str().map(|s| s.to_string()),
                            });
                        }
                    }
                    Err(e) => {
                        // 记录JSON解析错误，但不中断流处理
                        log::debug!("Failed to parse ModelScope SSE data as JSON: {} | Data: {}", e, data);
                        // 继续处理下一行，不返回错误
                        continue;
                    }
                }
            }
        }
        
        // 如果没有找到有效的数据，返回空内容块
        Ok(StreamChunk {
            id: "".to_string(),
            model: "".to_string(),
            content: "".to_string(),
            usage: None,
            finish_reason: None,
        })
    }
}

#[async_trait]
impl AiProvider for ModelScopeProvider {
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
        
        // 添加工具支持
        if let Some(tools) = &request.tools {
            if !tools.is_empty() {
                body["tools"] = serde_json::json!(tools);
            }
        }
        
        Ok(body)
    }

    async fn test_connection(&self) -> Result<bool> {
        let test_request = ChatRequest {
            model: "qwen-turbo".to_string(),
            messages: vec![Message::user("Hello")],
            tools: None,
            tool_choice: None,
            user: None,
            extra_params: None,
            options: Some(ChatOptions {
                max_tokens: Some(1),
                ..Default::default()
            }),
        };

        match self.send_chat_request(&test_request).await {
            Ok(_) => Ok(true),
            Err(e) => {
                tracing::warn!("ModelScope connection test failed: {}", e);
                Ok(false)
            }
        }
    }

    async fn send_chat_request(&self, request: &ChatRequest) -> Result<ChatResponse> {
        let url = format!(
            "{}/chat/completions",
            self.base.config.api_base.as_ref().unwrap()
        );

        let request_body = self.build_chat_request(request)?;

        let mut headers = HashMap::new();
        headers.insert(
            "Authorization".to_string(),
            format!("Bearer {}", self.base.config.api_key),
        );
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        // 添加额外头部
        if let Some(extra_headers) = &self.base.config.extra_headers {
            for (key, value) in extra_headers {
                headers.insert(key.clone(), value.clone());
            }
        }

        let http_client = HttpClient::new(
            self.base
                .config
                .timeout
                .unwrap_or(std::time::Duration::from_secs(30)),
        )?;

        let response = http_client
            .post_json(&url, &request_body, Some(headers))
            .await?;

        self.parse_chat_response(response)
    }

    async fn send_chat_stream(&self, request: &ChatRequest) -> Result<ChatStreamResponse> {
        info!("Sending ModelScope stream chat request for model: {}", request.model);
        
        let url = format!("{}/chat/completions", self.get_api_base());
        let headers = self.build_auth_headers()?;
        let mut body = self.build_chat_request(request)?;
        body["stream"] = json!(true);
        
        info!("ModelScope stream request URL: {}", url);
        info!("ModelScope stream request body: {}", body);
        
        let stream = self.base.execute_with_retry(|| {
            let url = url.clone();
            let headers = headers.clone();
            let body = body.clone();
            async move {
                self.base.http_client.post_stream(&url, &body, Some(headers)).await
            }
        }).await?;
        
        // 使用SSE解析器处理流
        let sse_stream = SseParser::new(stream);
        
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
                        match Self::parse_chunk_data(&sse_event.data) {
                            Ok(chunk) => Some(Ok(chunk)),
                            Err(e) => {
                                warn!("Failed to parse ModelScope chunk: {}", e);
                                None // 跳过解析失败的数据
                            }
                        }
                    } else {
                        None // 跳过空数据
                    }
                },
                Err(e) => {
                    warn!("Stream error: {}", e);
                    Some(Err(e))
                }
            })
        });
        
        Ok(ChatStreamResponse {
            stream: Box::new(mapped_stream),
            request_info: self.base.get_last_request_info(),
            response_info: self.base.get_last_response_info(),
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
                        log::debug!("Failed to parse ModelScope SSE data as JSON: {} | Data: {}", e, data);
                        // 继续处理下一行，不返回错误
                        continue;
                    }
                }
            }
        }
        
        Ok(None)
    }
}
