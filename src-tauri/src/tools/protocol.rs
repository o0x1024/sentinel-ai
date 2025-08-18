use anyhow::{anyhow, Result};
use rmcp::model::{Implementation, ProtocolVersion};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

/// MCP协议版本
pub const MCP_PROTOCOL_VERSION: &str = "0.2.0";

/// JSON-RPC 2.0 消息基础结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcMessage {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// MCP初始化请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeRequest {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
    #[serde(rename = "clientInfo")]
    pub client_info: ClientInfo,
}

/// 客户端能力
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClientCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolCapabilities>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourceCapabilities>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptCapabilities>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<LoggingCapabilities>,
}

/// 工具能力
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_streaming: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_progress: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_cancellation: Option<bool>,
}

/// 资源能力
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_content_type: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_partial_read: Option<bool>,
}

/// 提示能力
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PromptCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_parameters: Option<bool>,
}

/// 日志能力
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LoggingCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_logging: Option<bool>,
}

/// 客户端信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

/// 服务器信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerInfo {
    pub name: String,
    pub version: String,
}

/// MCP初始化响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResponse {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: McpServerCapabilities,
    #[serde(rename = "serverInfo")]
    pub server_info: McpServerInfo,
}

/// 服务器能力
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolCapabilities>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourceCapabilities>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptCapabilities>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<LoggingCapabilities>,
}

/// 工具列表请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolListRequest {}

/// 工具列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolListResponse {
    pub tools: Vec<ToolDefinition>,
}

/// MCP工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
    #[serde(rename = "toolPermissions", skip_serializing_if = "Option::is_none")]
    pub tool_permissions: Option<Vec<String>>,
    #[serde(rename = "outputContentTypes", skip_serializing_if = "Option::is_none")]
    pub output_content_types: Option<Vec<String>>,
}

/// 工具调用请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
}

/// 工具调用响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResponse {
    pub contents: Vec<ToolContent>,
    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// 工具内容
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum ToolContent {
    #[serde(rename = "text")]
    Text {
        text: String,
        #[serde(rename = "contentType", skip_serializing_if = "Option::is_none")]
        content_type: Option<String>,
    },
    #[serde(rename = "image")]
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
    #[serde(rename = "resource")]
    Resource {
        uri: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
    },
}

/// 资源引用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceReference {
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// 取消操作请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
}

/// 消息构建器
pub struct MessageBuilder;

impl MessageBuilder {
    /// 创建初始化请求
    pub fn initialize_request(client_name: &str, client_version: &str) -> JsonRpcMessage {
        JsonRpcMessage {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::Number(serde_json::Number::from(1))),
            method: Some("initialize".to_string()),
            params: Some(
                serde_json::to_value(InitializeRequest {
                    protocol_version: MCP_PROTOCOL_VERSION.to_string(),
                    capabilities: ClientCapabilities {
                        tools: Some(ToolCapabilities {
                            supports_streaming: Some(true),
                            supports_progress: Some(true),
                            supports_cancellation: Some(true),
                        }),
                        resources: Some(ResourceCapabilities {
                            supports_content_type: Some(vec![
                                "text/plain".to_string(),
                                "application/json".to_string(),
                                "image/png".to_string(),
                                "image/jpeg".to_string(),
                            ]),
                            supports_partial_read: Some(true),
                        }),
                        prompts: None,
                        logging: Some(LoggingCapabilities {
                            supports_logging: Some(true),
                        }),
                    },
                    client_info: ClientInfo {
                        name: client_name.to_string(),
                        version: client_version.to_string(),
                    },
                })
                .unwrap(),
            ),
            result: None,
            error: None,
        }
    }

    /// 创建工具列表请求
    pub fn tool_list_request() -> JsonRpcMessage {
        JsonRpcMessage {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::String(Uuid::new_v4().to_string())),
            method: Some("tools/list".to_string()),
            params: Some(serde_json::to_value(ToolListRequest {}).unwrap()),
            result: None,
            error: None,
        }
    }

    /// 创建工具调用请求
    pub fn tool_call_request(name: String, arguments: Option<Value>) -> JsonRpcMessage {
        JsonRpcMessage {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::String(Uuid::new_v4().to_string())),
            method: Some("tools/call".to_string()),
            params: Some(serde_json::to_value(ToolCallRequest { name, arguments }).unwrap()),
            result: None,
            error: None,
        }
    }

    /// 创建取消操作请求
    pub fn cancel_request(id: Option<Value>) -> JsonRpcMessage {
        JsonRpcMessage {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::String(Uuid::new_v4().to_string())),
            method: Some("$/cancel".to_string()),
            params: Some(serde_json::to_value(CancelRequest { id }).unwrap()),
            result: None,
            error: None,
        }
    }

    /// 创建成功响应
    pub fn success_response(id: Value, result: Value) -> JsonRpcMessage {
        JsonRpcMessage {
            jsonrpc: "2.0".to_string(),
            id: Some(id),
            method: None,
            params: None,
            result: Some(result),
            error: None,
        }
    }

    /// 创建错误响应
    pub fn error_response(
        id: Option<Value>,
        code: i32,
        message: String,
        data: Option<Value>,
    ) -> JsonRpcMessage {
        JsonRpcMessage {
            jsonrpc: "2.0".to_string(),
            id,
            method: None,
            params: None,
            result: None,
            error: Some(JsonRpcError {
                code,
                message,
                data,
            }),
        }
    }

    /// 创建通知消息
    pub fn notification(method: String, params: Option<Value>) -> JsonRpcMessage {
        JsonRpcMessage {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: Some(method),
            params,
            result: None,
            error: None,
        }
    }

    /// 创建进度通知
    pub fn progress_notification(
        id: &Value,
        progress: f64,
        message: Option<&str>,
    ) -> JsonRpcMessage {
        let mut params = serde_json::Map::new();
        params.insert("id".to_string(), id.clone());
        params.insert("progress".to_string(), serde_json::json!(progress));

        if let Some(msg) = message {
            params.insert("message".to_string(), serde_json::json!(msg));
        }

        JsonRpcMessage {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: Some("$/progress".to_string()),
            params: Some(Value::Object(params)),
            result: None,
            error: None,
        }
    }

    /// 创建日志通知
    pub fn log_notification(level: &str, message: &str) -> JsonRpcMessage {
        let mut params = serde_json::Map::new();
        params.insert("level".to_string(), serde_json::json!(level));
        params.insert("message".to_string(), serde_json::json!(message));

        JsonRpcMessage {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: Some("$/log".to_string()),
            params: Some(Value::Object(params)),
            result: None,
            error: None,
        }
    }
}

/// 消息解析器
pub struct MessageParser;

impl MessageParser {
    /// 解析消息类型
    pub fn parse_message_type(message: &JsonRpcMessage) -> MessageType {
        if message.id.is_none() {
            return MessageType::Notification;
        }

        if message.method.is_some() {
            return MessageType::Request;
        }

        if message.result.is_some() || message.error.is_some() {
            return MessageType::Response;
        }

        MessageType::Unknown
    }

    /// 解析工具列表响应
    pub fn parse_tool_list_response(
        message: &JsonRpcMessage,
    ) -> Result<Vec<ToolDefinition>, String> {
        if let Some(result) = &message.result {
            match serde_json::from_value::<ToolListResponse>(result.clone()) {
                Ok(response) => Ok(response.tools),
                Err(e) => Err(format!("Failed to parse tool list response: {}", e)),
            }
        } else if let Some(error) = &message.error {
            Err(format!(
                "Tool list error: {} ({})",
                error.message, error.code
            ))
        } else {
            Err("Invalid tool list response: no result or error".to_string())
        }
    }

    /// 解析工具调用响应
    pub fn parse_tool_call_response(message: &JsonRpcMessage) -> Result<ToolCallResponse, String> {
        if let Some(result) = &message.result {
            match serde_json::from_value::<ToolCallResponse>(result.clone()) {
                Ok(response) => Ok(response),
                Err(e) => Err(format!("Failed to parse tool call response: {}", e)),
            }
        } else if let Some(error) = &message.error {
            Err(format!(
                "Tool call error: {} ({})",
                error.message, error.code
            ))
        } else {
            Err("Invalid tool call response: no result or error".to_string())
        }
    }

    /// 验证消息格式
    pub fn validate_message(message: &JsonRpcMessage) -> Result<(), String> {
        // 检查JSON-RPC版本
        if message.jsonrpc != "2.0" {
            return Err(format!("Unsupported JSON-RPC version: {}", message.jsonrpc));
        }

        // 根据消息类型验证字段
        match Self::parse_message_type(message) {
            MessageType::Request => {
                // 请求必须有id和method
                if message.id.is_none() {
                    return Err("Request message missing id field".to_string());
                }

                if message.method.is_none() {
                    return Err("Request message missing method field".to_string());
                }

                // 请求不应该有result或error
                if message.result.is_some() || message.error.is_some() {
                    return Err(
                        "Request message should not contain result or error fields".to_string()
                    );
                }
            }
            MessageType::Response => {
                // 响应必须有id和(result或error)
                if message.id.is_none() {
                    return Err("Response message missing id field".to_string());
                }

                if message.result.is_none() && message.error.is_none() {
                    return Err("Response message missing result or error field".to_string());
                }

                // 响应不应该有method或params
                if message.method.is_some() || message.params.is_some() {
                    return Err(
                        "Response message should not contain method or params fields".to_string(),
                    );
                }
            }
            MessageType::Notification => {
                // 通知必须有method
                if message.method.is_none() {
                    return Err("Notification message missing method field".to_string());
                }

                // 通知不应该有id、result或error
                if message.id.is_some() || message.result.is_some() || message.error.is_some() {
                    return Err(
                        "Notification message should not contain id, result or error fields"
                            .to_string(),
                    );
                }
            }
            MessageType::Unknown => {
                return Err("Unable to determine message type".to_string());
            }
        }

        Ok(())
    }

    /// 将工具内容转换为RMCP内容
    pub fn tool_content_to_rmcp_content(content: &ToolContent) -> rmcp::model::Content {
        let raw_content = match content {
            ToolContent::Text {
                text,
                content_type: _,
            } => rmcp::model::RawContent::Text(rmcp::model::RawTextContent { text: text.clone() }),
            ToolContent::Image { data, mime_type } => {
                rmcp::model::RawContent::image(data.clone(), mime_type.clone())
            }
            ToolContent::Resource { uri, text } => {
                let resource_text = text.clone().unwrap_or_else(|| uri.clone());
                rmcp::model::RawContent::Text(rmcp::model::RawTextContent {
                    text: resource_text,
                })
            }
        };

        rmcp::model::Content {
            raw: raw_content,
            annotations: Default::default(),
        }
    }
}

/// 消息类型
#[derive(Debug, Clone, PartialEq)]
pub enum MessageType {
    Request,
    Response,
    Notification,
    Unknown,
}

/// 错误码定义
pub mod error_codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
    pub const SERVER_ERROR_START: i32 = -32000;
    pub const SERVER_ERROR_END: i32 = -32099;
    pub const TOOL_NOT_FOUND: i32 = -32000;
    pub const TOOL_EXECUTION_ERROR: i32 = -32001;
    pub const CONNECTION_ERROR: i32 = -32002;
    pub const TIMEOUT_ERROR: i32 = -32003;
    pub const SERVER_NOT_INITIALIZED: i32 = -32002;
    pub const UNKNOWN_ERROR_CODE: i32 = -32001;
    pub const REQUEST_CANCELLED: i32 = -32800;
}

/// 协议版本转换
pub fn get_latest_protocol_version() -> ProtocolVersion {
    ProtocolVersion::V_2024_11_05
}

/// 创建默认服务器信息
pub fn create_default_server_info(name: &str, version: &str) -> rmcp::model::ServerInfo {
    rmcp::model::ServerInfo {
        protocol_version: get_latest_protocol_version(),
        capabilities: rmcp::model::ServerCapabilities::builder()
            .enable_tools()
            .build(),
        server_info: rmcp::model::Implementation {
            name: name.to_string(),
            version: version.to_string(),
        },
        instructions: Some(format!("{} MCP服务器 v{}", name, version)),
    }
}

/// MCP 消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpMessage {
    #[serde(rename = "initialize")]
    Initialize { client_info: Implementation },

    #[serde(rename = "initialize_result")]
    InitializeResult {
        server_info: rmcp::model::ServerInfo,
    },

    #[serde(rename = "list_tools")]
    ListTools,

    #[serde(rename = "list_tools_result")]
    ListToolsResult { tools: Vec<rmcp::model::Tool> },

    #[serde(rename = "call_tool")]
    CallTool {
        name: String,
        arguments: Option<HashMap<String, Value>>,
    },

    #[serde(rename = "call_tool_result")]
    CallToolResult { contents: Vec<rmcp::model::Content> },

    #[serde(rename = "error")]
    Error {
        code: i32,
        message: String,
        data: Option<Value>,
    },

    #[serde(rename = "cancel")]
    Cancel,
}

/// 解析MCP消息
pub fn parse_message(json_str: &str) -> Result<McpMessage> {
    serde_json::from_str(json_str).map_err(|e| anyhow!("Failed to parse MCP message: {}", e))
}

/// 序列化MCP消息
pub fn serialize_message(message: &McpMessage) -> Result<String> {
    serde_json::to_string(message).map_err(|e| anyhow!("Failed to serialize MCP message: {}", e))
}



/// 创建错误消息
pub fn create_error_message(code: i32, message: &str, data: Option<Value>) -> McpMessage {
    McpMessage::Error {
        code,
        message: message.to_string(),
        data,
    }
}

/// 工具调用上下文
#[derive(Debug, Clone)]
pub struct ToolCallContext {
    pub tool_name: String,
    pub arguments: Option<HashMap<String, Value>>,
    pub call_id: String,
    pub start_time: std::time::Instant,
}

impl ToolCallContext {
    pub fn new(tool_name: &str, arguments: Option<HashMap<String, Value>>) -> Self {
        Self {
            tool_name: tool_name.to_string(),
            arguments,
            call_id: uuid::Uuid::new_v4().to_string(),
            start_time: std::time::Instant::now(),
        }
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization() {
        let message = McpMessage::Initialize {
            client_info: Implementation {
                name: "test-client".to_string(),
                version: "1.0.0".to_string(),
            },
        };

        let json = serialize_message(&message).unwrap();
        let parsed: McpMessage = parse_message(&json).unwrap();

        match parsed {
            McpMessage::Initialize { client_info } => {
                assert_eq!(client_info.name, "test-client");
                assert_eq!(client_info.version, "1.0.0");
            }
            _ => panic!("Unexpected message type"),
        }
    }


}
