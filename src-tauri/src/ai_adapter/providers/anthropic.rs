//! Anthropic提供商适配器

use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::{json, Value};

// Provider trait 已删除，使用 AiProvider
use crate::ai_adapter::error::{AiAdapterError, Result};
use crate::ai_adapter::providers::base::BaseProvider;
use crate::ai_adapter::types::*;
use crate::models::ai::MessageRole;
use crate::ai_adapter::utils::{json as json_utils, time};

/// Anthropic提供商
#[derive(Debug)]
pub struct AnthropicProvider {
    base: BaseProvider,
}

impl AnthropicProvider {
    /// 创建新的Anthropic提供商
    pub fn new(config: ProviderConfig) -> Result<Self> {
        let base = BaseProvider::new("anthropic".to_string(), "1.0.0".to_string(), config)?;

        Ok(Self { base })
    }

    /// 获取API基础URL
    fn get_api_base(&self) -> String {
        self.base.get_api_base("https://api.anthropic.com")
    }

    /// 构建认证头
    fn build_auth_headers(&self) -> Result<HeaderMap> {
        let mut headers = self.base.build_base_headers()?;

        headers.insert(
            "x-api-key",
            HeaderValue::from_str(&self.base.config.api_key)
                .map_err(|e| AiAdapterError::ConfigurationError(e.to_string()))?,
        );

        // Anthropic需要指定API版本
        let api_version = self
            .base
            .config
            .api_version
            .as_deref()
            .unwrap_or("2023-06-01");
        headers.insert(
            "anthropic-version",
            HeaderValue::from_str(api_version)
                .map_err(|e| AiAdapterError::ConfigurationError(e.to_string()))?,
        );

        Ok(headers)
    }

    /// 构建请求体
    fn build_request_body(&self, request: &ChatRequest) -> Result<Value> {
        let (system_message, messages) = self.extract_system_message(&request.messages)?;

        let mut body = json!({
            "model": request.model,
            "messages": messages
        });

        if let Some(system) = system_message {
            body["system"] = json!(system);
        }

        // 处理选项参数
        if let Some(options) = &request.options {
            if let Some(max_tokens) = options.max_tokens {
                body["max_tokens"] = json!(max_tokens);
            } else {
                // Anthropic要求必须指定max_tokens
                body["max_tokens"] = json!(4096);
            }

            if let Some(temperature) = options.temperature {
                body["temperature"] = json!(temperature);
            }

            if let Some(top_p) = options.top_p {
                body["top_p"] = json!(top_p);
            }

            if let Some(stop) = &options.stop {
                body["stop_sequences"] = json!(stop);
            }

            if let Some(stream) = options.stream {
                body["stream"] = json!(stream);
            }
        } else {
            // Anthropic要求必须指定max_tokens
            body["max_tokens"] = json!(4096);
        }

        // 工具调用支持
        if let Some(tools) = &request.tools {
            body["tools"] = json!(self.convert_tools(tools)?);
        }

        Ok(body)
    }

    /// 提取系统消息（Anthropic将系统消息单独处理）
    fn extract_system_message(&self, messages: &[Message]) -> Result<(Option<String>, Vec<Value>)> {
        let mut system_message = None;
        let mut converted_messages = Vec::new();

        for message in messages {
            if matches!(message.role, MessageRole::System) {
                if let MessageContent::Text(text) = &message.content {
                    system_message = Some(text.clone());
                }
            } else {
                converted_messages.push(self.convert_message(message)?);
            }
        }

        Ok((system_message, converted_messages))
    }

    /// 转换消息格式
    fn convert_message(&self, message: &Message) -> Result<Value> {
        let role = match message.role {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "user", // Anthropic将工具响应作为用户消息
            MessageRole::System => {
                return Err(AiAdapterError::ValidationError(
                    "System messages should be handled separately".to_string(),
                ))
            }
        };

        let content_text = match &message.content {
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

        let msg = json!({
            "role": role,
            "content": content_text
        });

        Ok(msg)
    }

    /// 转换工具定义
    fn convert_tools(&self, tools: &[Tool]) -> Result<Vec<Value>> {
        tools
            .iter()
            .map(|tool| {
                Ok(json!({
                    "name": tool.name,
                    "description": tool.description,
                    "input_schema": tool.parameters
                }))
            })
            .collect()
    }

    /// 解析响应
    fn parse_response(&self, response_body: &str) -> Result<ChatResponse> {
        let response_json: Value = serde_json::from_str(response_body)?;

        // 检查错误
        if let Some(error) = response_json.get("error") {
            let error_message = json_utils::extract_string(error, "message")
                .unwrap_or_else(|| "Unknown error".to_string());
            let error_type =
                json_utils::extract_string(error, "type").unwrap_or_else(|| "unknown".to_string());

            return match error_type.as_str() {
                "invalid_request_error" => Err(AiAdapterError::ClientError(error_message)),
                "authentication_error" => Err(AiAdapterError::AuthenticationError(error_message)),
                "permission_error" => Err(AiAdapterError::AuthorizationError(error_message)),
                "rate_limit_error" => Err(AiAdapterError::RateLimitError(error_message)),
                "api_error" => Err(AiAdapterError::ServerError(error_message)),
                _ => Err(AiAdapterError::UnknownError(error_message)),
            };
        }

        // 解析成功响应
        let id = json_utils::extract_string(&response_json, "id").ok_or_else(|| {
            AiAdapterError::DeserializationError("Missing 'id' field".to_string())
        })?;

        let model = json_utils::extract_string(&response_json, "model").ok_or_else(|| {
            AiAdapterError::DeserializationError("Missing 'model' field".to_string())
        })?;

        let content = self.parse_content(&response_json)?;
        let usage = self.parse_usage(&response_json)?;

        // 构建标准格式的响应
        let message = Message {
            role: MessageRole::Assistant,
            content: MessageContent::Text(content),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        };

        Ok(ChatResponse {
            id,
            model,
            message: message.clone(),
            choices: vec![Choice {
                index: 0,
                message,
                finish_reason: json_utils::extract_string(&response_json, "stop_reason"),
            }],
            usage,
            finish_reason: json_utils::extract_string(&response_json, "stop_reason"),
            created_at: std::time::SystemTime::now(),
        })
    }

    /// 解析内容
    fn parse_content(&self, response_json: &Value) -> Result<String> {
        if let Some(content_array) = json_utils::extract_array(response_json, "content") {
            let mut text_content = String::new();

            for content_item in content_array {
                if let Some(content_type) = json_utils::extract_string(content_item, "type") {
                    if content_type == "text" {
                        if let Some(text) = json_utils::extract_string(content_item, "text") {
                            text_content.push_str(&text);
                        }
                    }
                }
            }

            Ok(text_content)
        } else {
            Err(AiAdapterError::DeserializationError(
                "Missing or invalid 'content' field".to_string(),
            ))
        }
    }

    /// 解析使用情况
    fn parse_usage(&self, response_json: &Value) -> Result<Option<Usage>> {
        if let Some(usage_json) = response_json.get("usage") {
            let input_tokens =
                json_utils::extract_u64(usage_json, "input_tokens").unwrap_or(0) as u32;

            let output_tokens =
                json_utils::extract_u64(usage_json, "output_tokens").unwrap_or(0) as u32;

            Ok(Some(Usage {
                prompt_tokens: input_tokens,
                completion_tokens: output_tokens,
                total_tokens: input_tokens + output_tokens,
            }))
        } else {
            Ok(None)
        }
    }
}

// Provider trait 实现已删除，使用 AiProvider trait

#[async_trait]
impl AiProvider for AnthropicProvider {
    fn name(&self) -> &str {
        self.base.name()
    }

    fn version(&self) -> &str {
        self.base.version()
    }

    fn supported_models(&self) -> Vec<String> {
        vec![
            "claude-3-5-sonnet-20241022".to_string(),
            "claude-3-5-haiku-20241022".to_string(),
            "claude-3-opus-20240229".to_string(),
            "claude-3-sonnet-20240229".to_string(),
            "claude-3-haiku-20240307".to_string(),
        ]
    }

    async fn test_connection(&self) -> Result<bool> {
        // Anthropic没有专门的测试端点，我们发送一个简单的请求

        // 简单的连接测试
        let test_request = ChatRequest {
            model: "claude-3-5-haiku-20241022".to_string(),
            messages: vec![Message {
                role: crate::models::ai::MessageRole::User,
                content: MessageContent::Text("Hello".to_string()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            }],
            tools: None,
            temperature: None,
            max_tokens: Some(10),
            tool_choice: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            stream: Some(false),
            user: None,
            extra_params: None,
            options: None,
        };
        
        match self.send_chat_request(&test_request).await {
            Ok(_) => Ok(true),
            Err(AiAdapterError::AuthenticationError(_)) => Ok(false),
            Err(AiAdapterError::AuthorizationError(_)) => Ok(false),
            Err(_) => Ok(true), // 其他错误可能是网络问题，认为连接正常
        }
    }

    fn get_last_request_info(&self) -> Option<HttpRequest> {
        self.base.get_last_request_info()
    }

    fn get_last_response_info(&self) -> Option<HttpResponse> {
        self.base.get_last_response_info()
    }

    async fn send_chat_request(&self, request: &ChatRequest) -> Result<ChatResponse> {
        let url = format!("{}/v1/messages", self.get_api_base());
        let headers = self.build_auth_headers()?;
        let body = self.build_request_body(request)?;
        
        // 将HeaderMap转换为HashMap<String, String>
        let headers_map: std::collections::HashMap<String, String> = headers
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        
        let response_json = self.base.http_client.post_json(&url, &body, Some(headers_map)).await?;
        let response_body = serde_json::to_string(&response_json)?;
        self.parse_response(&response_body)
    }
    
    async fn send_chat_stream(&self, _request: &ChatRequest) -> Result<ChatStream> {
        // TODO: 实现Anthropic流式响应
        Err(AiAdapterError::UnknownError(
            "Anthropic streaming not yet implemented".to_string()
        ))
    }
    
    // 重复的方法定义已删除

}
