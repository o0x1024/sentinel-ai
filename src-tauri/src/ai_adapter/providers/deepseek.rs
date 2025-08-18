//! DeepSeek提供商适配器

use crate::ai_adapter::types::*;
use crate::ai_adapter::error::{AiAdapterError, Result};
use async_trait::async_trait;
use std::ops::Deref;
use std::collections::HashMap;
use tokio_util::bytes::Bytes;
use crate::ai_adapter::providers::base::BaseProvider;
/// DeepSeek提供商
#[derive(Debug)]
pub struct DeepSeekProvider {
    base: BaseProvider,
}

impl Deref for DeepSeekProvider {
    type Target = BaseProvider;
    
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DeepSeekProvider {
    /// 创建新的DeepSeek提供商实例
    pub fn new(config: ProviderConfig) -> Result<Self> {
        let base = BaseProvider::new(
            "deepseek".to_string(),
            "1.0.0".to_string(),
            config,
        )?;
        
        Ok(Self { base })
    }
    
    /// 转换消息格式
    fn convert_messages(&self, messages: &[Message]) -> Result<Vec<serde_json::Value>> {
        let mut converted = Vec::new();
        
        for message in messages {
            let mut msg = serde_json::json!({
                "role": message.role,
                "content": message.content
            });
            
            // 处理工具调用
            if let Some(tool_calls) = &message.tool_calls {
                msg["tool_calls"] = serde_json::to_value(tool_calls)?;
            }
            
            // 处理工具调用ID
            if let Some(tool_call_id) = &message.tool_call_id {
                msg["tool_call_id"] = serde_json::Value::String(tool_call_id.clone());
            }
            
            converted.push(msg);
        }
        
        Ok(converted)
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
        
        // 解析第一个choice的消息
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
    
    /// 解析选择
    fn parse_choices(&self, choices: &serde_json::Value) -> Result<Vec<Choice>> {
        let choices_array = choices.as_array()
            .ok_or_else(|| AiAdapterError::DeserializationError("Invalid choices format".to_string()))?;
            
        let mut parsed_choices = Vec::new();
        
        for choice in choices_array {
            let index = choice["index"].as_u64().unwrap_or(0) as u32;
            let message = self.parse_message(&choice["message"])?;
            let finish_reason = choice["finish_reason"].as_str().map(|s| s.to_string());
            
            parsed_choices.push(Choice {
                index,
                message,
                finish_reason,
            });
        }
        
        Ok(parsed_choices)
    }
    
    /// 解析消息
    fn parse_message(&self, message: &serde_json::Value) -> Result<Message> {
        let role_str = message["role"].as_str().unwrap_or("user");
        let role = match role_str {
            "system" => MessageRole::System,
            "user" => MessageRole::User,
            "assistant" => MessageRole::Assistant,
            "tool" => MessageRole::Tool,
            _ => MessageRole::User,
        };
        let content_str = message["content"].as_str().unwrap_or("");
        let content = content_str.to_string();
        
        let tool_calls = if let Some(tool_calls_array) = message["tool_calls"].as_array() {
            let mut parsed_tool_calls = Vec::new();
            for tool_call in tool_calls_array {
                if let (Some(id), Some(function)) = (
                    tool_call["id"].as_str(),
                    tool_call["function"].as_object()
                ) {
                    if let (Some(name), Some(arguments)) = (
                        function["name"].as_str(),
                        function["arguments"].as_str()
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
        } else {
            None
        };
        
        let tool_call_id = message["tool_call_id"].as_str().map(|s| s.to_string());
        
        Ok(Message {
            role,
            content,
            tool_calls,
            tool_call_id,
            name: message["name"].as_str().map(|s| s.to_string()),
        })
    }
    
    /// 解析使用情况
    fn parse_usage(&self, usage: &serde_json::Value) -> Option<Usage> {
        if usage.is_null() {
            return None;
        }
        
        Some(Usage {
            prompt_tokens: usage["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: usage["completion_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: usage["total_tokens"].as_u64().unwrap_or(0) as u32,
        })
    }
    
    /// 解析流式响应
    fn parse_stream(
        &self,
        stream: impl futures::Stream<Item = std::result::Result<Bytes, AiAdapterError>> + Send + 'static,
    ) -> Box<dyn futures::Stream<Item = Result<StreamChunk>> + Send + Unpin> {
        use futures::StreamExt;
        use tokio_stream::wrappers::UnboundedReceiverStream;
        
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        
        tokio::spawn(async move {
            let mut stream = std::pin::pin!(stream);
            
            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        let chunk_str = String::from_utf8_lossy(&chunk);
                        
                        // 处理SSE格式的数据
                        for line in chunk_str.lines() {
                            if line.starts_with("data: ") {
                                let data = &line[6..]; // 移除"data: "前缀
                                
                                if data == "[DONE]" {
                                    break;
                                }
                                
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                    if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
                                        if let Some(choice) = choices.first() {
                                            let delta = choice.get("delta").unwrap_or(&serde_json::Value::Null);
                                            let content = delta.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
                                            let finish_reason = choice.get("finish_reason").and_then(|f| f.as_str()).map(|s| s.to_string());
                                            
                                            let chunk = StreamChunk {
                                                id: json.get("id").and_then(|i| i.as_str()).unwrap_or("unknown").to_string(),
                                                model: json.get("model").and_then(|m| m.as_str()).unwrap_or("deepseek-chat").to_string(),
                                                content,
                                                finish_reason,
                                                usage: None,
                                            };
                                            
                                            if tx.send(Ok(chunk)).is_err() {
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e));
                        break;
                    }
                }
            }
        });
        
        Box::new(UnboundedReceiverStream::new(rx))
    }
    
    /// 解析流式数据块
    #[allow(unused)]
    fn parse_stream_chunk(&self, chunk: &serde_json::Value) -> Result<StreamChunk> {
        let id = chunk["id"].as_str().unwrap_or("").to_string();
        let model = chunk["model"].as_str().unwrap_or("").to_string();
        
        // 从第一个choice的delta中获取content
        let content = if let Some(choices) = chunk["choices"].as_array() {
            if let Some(choice) = choices.first() {
                if let Some(delta) = choice.get("delta") {
                    delta.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string()
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        
        let finish_reason = chunk["choices"].as_array()
            .and_then(|choices| choices.first())
            .and_then(|choice| choice["finish_reason"].as_str())
            .map(|s| s.to_string());
        
        let usage = self.parse_usage(&chunk["usage"]);
        
        Ok(StreamChunk {
            id,
            model,
            content,
            finish_reason,
            usage,
        })
    }

}

#[async_trait]
impl AiProvider for DeepSeekProvider {
    fn name(&self) -> &str {
        self.base.name()
    }
    
    fn version(&self) -> &str {
        self.base.version()
    }
    
    fn supported_models(&self) -> Vec<String> {
        vec![
            "deepseek-chat".to_string(),
            "deepseek-coder".to_string(),
        ]
    }
    
    async fn test_connection(&self) -> Result<bool> {
        let url = format!("{}/models", self.get_api_base("https://api.deepseek.com"));
        
        let mut headers = self.build_base_headers()?;
        headers.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(&format!("Bearer {}", self.config.api_key))
                .map_err(|e| AiAdapterError::ConfigurationError(e.to_string()))?,
        );

        let mut headers_map = HashMap::new();
        headers_map.insert("Authorization".to_string(), format!("Bearer {}", self.config.api_key));
        
        let _response = self.http_client.post_json(&url, &serde_json::json!({}), Some(headers_map))
            .await?;
            
        Ok(true)
    }

    async fn send_chat_request(&self, request: &ChatRequest) -> Result<ChatResponse> {
        let url = format!("{}/chat/completions", self.get_api_base("https://api.deepseek.com"));
        
        tracing::info!("🔍 发送请求: {:?}", request);
        // 构建请求头
        let mut headers = self.build_base_headers()?;
        headers.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(&format!("Bearer {}", self.config.api_key))
                .map_err(|e| AiAdapterError::ConfigurationError(e.to_string()))?,
        );
        
        // 构建请求体
        let mut body = serde_json::json!({
            "model": request.model,
            "messages": self.convert_messages(&request.messages)?,
            "stream": false
        });
        
        // 添加可选参数
        if let Some(options) = &request.options {
            if let Some(temperature) = options.temperature {
                body["temperature"] = serde_json::Value::Number(serde_json::Number::from_f64(temperature as f64).unwrap());
            }
            if let Some(max_tokens) = options.max_tokens {
                body["max_tokens"] = serde_json::Value::Number(serde_json::Number::from(max_tokens));
            }
            if let Some(top_p) = options.top_p {
                body["top_p"] = serde_json::Value::Number(serde_json::Number::from_f64(top_p as f64).unwrap());
            }
            if let Some(frequency_penalty) = options.frequency_penalty {
                body["frequency_penalty"] = serde_json::Value::Number(serde_json::Number::from_f64(frequency_penalty as f64).unwrap());
            }
            if let Some(presence_penalty) = options.presence_penalty {
                body["presence_penalty"] = serde_json::Value::Number(serde_json::Number::from_f64(presence_penalty as f64).unwrap());
            }
            if let Some(stop) = &options.stop {
                body["stop"] = serde_json::to_value(stop)?;
            }
        }
        if let Some(tools) = &request.tools {
            body["tools"] = serde_json::to_value(tools)?;
        }
        
        let body_str = serde_json::to_string(&body)?;
        
        // 记录详细的请求信息
        let request_info = HttpRequest {
            method: "POST".to_string(),
            url: url.clone(),
            headers: headers.iter().map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string())).collect(),
            body: Some(body_str.clone()),
            timestamp: std::time::SystemTime::now(),
        };
        self.record_request_info(request_info.clone());
        
        tracing::info!("📄 完整请求体: {}", body_str);
        
        // 发送请求
        let mut headers_map = HashMap::new();
        headers_map.insert("Authorization".to_string(), format!("Bearer {}", self.config.api_key));
        
        let response_json = self.http_client.post_json(&url, &body, Some(headers_map))
            .await
            .map_err(|e| {
                tracing::error!("DeepSeek post_json failed: {}", e);
                // 打印部分请求体帮助定位422（注意避免敏感信息泄露）
                if let Ok(body_str) = serde_json::to_string(&body) {
                    let snippet = if body_str.len() > 2000 { &body_str[..2000] } else { &body_str };
                    tracing::debug!("DeepSeek request body (truncated): {}", snippet);
                }
                e
            })?;
            
        tracing::info!("📄 完整响应体: {}", serde_json::to_string(&response_json)?);

        // 记录请求和响应信息
        let response_info = HttpResponse {
            status: 200,
            headers: HashMap::new(),
            body: Some(serde_json::to_string(&response_json).unwrap_or_default()),
            timestamp: std::time::SystemTime::now(),
            duration: std::time::Duration::from_millis(0),
        };
        self.record_response_info(response_info.clone());
            
        self.parse_chat_response(&response_json, Some(request_info), Some(response_info))
    }

    async fn send_chat_stream(&self, request: &ChatRequest) -> Result<ChatStream> {
        let url = format!("{}/chat/completions", self.get_api_base("https://api.deepseek.com"));
        
        // 构建请求头
        let mut headers = self.build_base_headers()?;
        headers.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(&format!("Bearer {}", self.config.api_key))
                .map_err(|e| AiAdapterError::ConfigurationError(e.to_string()))?,
        );
        
        // 构建请求体（流式）
        let mut body = serde_json::json!({
            "model": request.model,
            "messages": self.convert_messages(&request.messages)?,
            "stream": true
        });
        
        // 添加可选参数
        if let Some(options) = &request.options {
            if let Some(temperature) = options.temperature {
                body["temperature"] = serde_json::Value::Number(serde_json::Number::from_f64(temperature as f64).unwrap());
            }
            if let Some(max_tokens) = options.max_tokens {
                body["max_tokens"] = serde_json::Value::Number(serde_json::Number::from(max_tokens));
            }
            if let Some(top_p) = options.top_p {
                body["top_p"] = serde_json::Value::Number(serde_json::Number::from_f64(top_p as f64).unwrap());
            }
            if let Some(frequency_penalty) = options.frequency_penalty {
                body["frequency_penalty"] = serde_json::Value::Number(serde_json::Number::from_f64(frequency_penalty as f64).unwrap());
            }
            if let Some(presence_penalty) = options.presence_penalty {
                body["presence_penalty"] = serde_json::Value::Number(serde_json::Number::from_f64(presence_penalty as f64).unwrap());
            }
            if let Some(stop) = &options.stop {
                body["stop"] = serde_json::to_value(stop)?;
            }
        }
        if let Some(tools) = &request.tools {
            body["tools"] = serde_json::to_value(tools)?;
        }
        
        let body_str = serde_json::to_string(&body)?;
        
        // 记录请求信息
        let request_info = HttpRequest {
            method: "POST".to_string(),
            url: url.clone(),
            headers: headers.iter().map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string())).collect(),
            body: Some(body_str.clone()),
            timestamp: std::time::SystemTime::now(),
        };
        self.record_request_info(request_info.clone());
        
        // 发送流式请求
        let mut headers_map = HashMap::new();
        headers_map.insert("Authorization".to_string(), format!("Bearer {}", self.config.api_key));
        
        let stream = self.http_client.post_stream(&url, &body, Some(headers_map))
            .await?;
        
        // 创建流
        let parsed_stream = self.parse_stream(stream);
        
        Ok(ChatStreamResponse {
            stream: parsed_stream,
            request_info: Some(request_info),
            response_info: None, // 流式响应没有完整的响应信息
        })
    }
    
    fn get_last_request_info(&self) -> Option<HttpRequest> {
        self.base.get_last_request_info()
    }
    
    fn get_last_response_info(&self) -> Option<HttpResponse> {
        self.base.get_last_response_info()
    }
    
}