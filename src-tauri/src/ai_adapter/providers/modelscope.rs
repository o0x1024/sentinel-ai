//! ModelScope提供商实现
//!
//! ModelScope API推理服务的适配器实现（兼容OpenAI API格式）

use async_trait::async_trait;
use futures_util::StreamExt;
use serde_json::{json, Value};
use tracing::{debug, info, warn};
use std::collections::HashMap;

use crate::ai_adapter::core::BaseProvider;
use crate::ai_adapter::error::{AiAdapterError, Result};
use crate::ai_adapter::http::HttpClient;
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

        // ModelScope支持的热门模型（兼容OpenAI格式）
        let models = vec![
            "qwen-turbo".to_string(),
            "qwen-plus".to_string(),
            "qwen-max".to_string(),
            "Qwen/Qwen2-VL-7B-Instruct".to_string(),
            "Qwen/Qwen2-7B-Instruct".to_string(),
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
            true,  // 支持工具调用（兼容OpenAI）
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

    /// 构建聊天请求（OpenAI兼容格式）
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

        // // 添加工具支持（OpenAI兼容）
        // if let Some(tools) = &request.tools {
        //     if !tools.is_empty() {
        //         body["tools"] = json!(self.convert_tools(tools)?);
        //     }
        // }
        
        // // 添加工具选择（OpenAI兼容）
        // if let Some(tool_choice) = &request.tool_choice {
        //     body["tool_choice"] = json!(tool_choice);
        // }

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

    /// 转换消息格式为OpenAI兼容格式
    fn convert_messages(&self, messages: &[Message]) -> Result<Value> {
        let mut converted = Vec::new();

        for message in messages {
            let role_str = match message.role {
                MessageRole::System => "system",
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::Tool => "tool", // 支持工具消息（OpenAI兼容）
            };

            let mut msg = json!({
                "role": role_str,
                "content": message.content
            });

            // 添加名称字段
            if let Some(name) = &message.name {
                msg["name"] = json!(name);
            }
            
            // 添加工具调用（OpenAI兼容）
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
    
    /// 转换工具定义（OpenAI兼容）
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
    
    /// 转换工具调用（OpenAI兼容）
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
    
    /// 解析SSE流式响应块（通用方法，兼容OpenAI格式）
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
        // 使用内部OpenAI兼容的方法
        self.build_chat_request(request)
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
        let headers = self.base.build_auth_headers();
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
        
        // 使用SSE解析器处理流式响应，但只做基本的SSE解析，不做业务逻辑解析
        use crate::ai_adapter::http::SseParser;
        use futures::StreamExt;
        let sse_stream = SseParser::new(stream);
        
        // Capture model name to avoid lifetime issues
        let model_name = request.model.clone();
        
        // 将SSE事件转换为StreamChunk，但保持最小化处理
        let mapped_stream = sse_stream.filter_map(move |result| {
            let model_name = model_name.clone();
            async move {
                match result {
                    Ok(sse_event) => {
                        debug!("ModelScope SSE event: type={:?}, data_len={}", sse_event.event_type, sse_event.data.len());
                        
                        // 处理[DONE]事件
                        if sse_event.data.trim() == "[DONE]" {
                            debug!("ModelScope stream completed with [DONE]");
                            return Some(Ok(StreamChunk {
                                id: "done".to_string(),
                                model: model_name.clone(),
                                content: String::new(),
                                usage: None,
                                finish_reason: Some("stop".to_string()),
                            }));
                        }
                        
                        // 直接返回原始JSON数据作为content，让上层调用者解析
                        // 这样可以保持provider的简洁性，将具体的解析逻辑移到调用层
                        if !sse_event.data.trim().is_empty() {
                            Some(Ok(StreamChunk {
                                id: "raw".to_string(),
                                model: model_name.clone(), 
                                content: sse_event.data.clone(), // 原始JSON数据
                                usage: None,
                                finish_reason: None,
                            }))
                        } else {
                            None // 跳过空事件
                        }
                    },
                    Err(e) => {
                        warn!("ModelScope SSE parsing error: {}", e);
                        Some(Err(e))
                    }
                }
            }
        });
        
        Ok(ChatStreamResponse {
            stream: Box::new(mapped_stream.boxed()),
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
        // 使用统一的SSE解析方法（OpenAI兼容）
        Self::parse_sse_chunk(chunk)
    }
}

impl ModelScopeProvider {
    /// Parse ModelScope-specific streaming response
    /// This method should be called by the upper layer to parse the raw JSON data
    pub fn parse_stream_chunk(raw_json: &str, model: &str) -> Result<Option<StreamChunk>> {
        let json: Value = serde_json::from_str(raw_json)
            .map_err(|e| AiAdapterError::SerializationError(format!("Failed to parse ModelScope JSON: {}", e)))?;
        
        debug!("ModelScope parsing JSON: {}", json);
        
        let mut accumulated_content = String::new();
        let mut finish_reason = None;
        let mut chunk_id = "unknown".to_string();
        let mut chunk_model = model.to_string();
        let mut usage = None;
        
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
                    if !reasoning_content.is_empty() {
                        content.push_str(reasoning_content);
                        debug!("ModelScope using reasoning_content: '{}'", reasoning_content);
                    }
                }
                
                // 3. 检查function_call字段（如果有工具调用）
                if let Some(function_call) = delta["function_call"].as_object() {
                    if let Some(arguments) = function_call["arguments"].as_str() {
                        if !arguments.is_empty() {
                            content.push_str(arguments);
                            debug!("ModelScope using function_call arguments: '{}'", arguments);
                        }
                    }
                }
                
                if !content.is_empty() {
                    accumulated_content.push_str(&content);
                    debug!("ModelScope content accumulated: '{}', total_len: {}", content, accumulated_content.len());
                }
                
                // 检查完成原因
                if let Some(finish_reason_str) = choice["finish_reason"].as_str() {
                    finish_reason = Some(finish_reason_str.to_string());
                    debug!("ModelScope finish_reason: {}", finish_reason_str);
                }
            }
        } else {
            // 如果没有choices，检查是否有直接的content字段
            if let Some(direct_content) = json["content"].as_str() {
                accumulated_content.push_str(direct_content);
                debug!("ModelScope using direct content: '{}'", direct_content);
            }
        }
        
        // 获取元数据
        chunk_id = json["id"].as_str().unwrap_or("unknown").to_string();
        if let Some(model_str) = json["model"].as_str() {
            chunk_model = model_str.to_string();
        }
        
        // 解析使用情况
        if let Some(usage_obj) = json["usage"].as_object() {
            usage = Some(Usage {
                prompt_tokens: usage_obj["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: usage_obj["completion_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: usage_obj["total_tokens"].as_u64().unwrap_or(0) as u32,
            });
        }
        
        Ok(Some(StreamChunk {
            id: chunk_id,
            model: chunk_model,
            content: accumulated_content,
            usage,
            finish_reason,
        }))
    }
}
