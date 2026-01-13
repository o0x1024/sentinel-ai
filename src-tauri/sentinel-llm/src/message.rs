//! 消息类型模块

use rig::completion::{message::Image, Message, AssistantContent};
use rig::message::{DocumentSourceKind, ImageDetail, ImageMediaType, UserContent, ToolCall};
use rig::one_or_many::OneOrMany;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ============================================================================
// 聊天消息
// ============================================================================

/// 聊天消息（用于多轮对话历史）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// 角色：user 或 assistant 或 tool
    pub role: String,
    /// 消息内容
    pub content: String,
    /// 工具调用（仅 assistant 消息）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<String>,
    /// 推理内容（deepseek-reasoner 模型需要）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    /// 工具调用 ID（仅 tool 消息）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    /// 创建用户消息
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
            tool_calls: None,
            reasoning_content: None,
            tool_call_id: None,
        }
    }

    /// 创建助手消息
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
            tool_calls: None,
            reasoning_content: None,
            tool_call_id: None,
        }
    }

    /// 创建新消息
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
            tool_calls: None,
            reasoning_content: None,
            tool_call_id: None,
        }
    }

    /// 创建工具消息
    pub fn tool(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        Self {
            role: "tool".to_string(),
            content: content.into(),
            tool_calls: None,
            reasoning_content: None,
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

/// 将 ChatMessage 列表转换为 rig Message 列表
/// 
/// Note: Anthropic API requires each tool_use to have exactly one tool_result,
/// and consecutive tool_result messages should be merged into a single User message.
pub fn convert_chat_history(history: &[ChatMessage]) -> Vec<Message> {
    use std::collections::HashSet;
    use rig::message::{ToolResult, ToolResultContent, Text};
    
    let mut result = Vec::new();
    let mut seen_tool_call_ids: HashSet<String> = HashSet::new();
    let mut valid_tool_call_ids: HashSet<String> = HashSet::new();
    let mut i = 0;
    
    while i < history.len() {
        let msg = &history[i];
        let content = msg.content.trim();
        let role = msg.role.to_lowercase();
        
        match role.as_str() {
            "user" => {
                if !content.is_empty() {
                    result.push(Message::User {
                        content: OneOrMany::one(UserContent::text(content.to_string())),
                    });
                }
                i += 1;
            }
            "assistant" => {
                let has_content = !content.is_empty();
                let has_tool_calls = msg.tool_calls.as_ref().map(|tc| !tc.trim().is_empty()).unwrap_or(false);
                
                if has_content || has_tool_calls {
                    let mut contents = Vec::new();
                    
                    // Add reasoning content if present (required for DeepSeek)
                    if has_tool_calls {
                        let reasoning = msg.reasoning_content.as_ref()
                            .map(|r| r.trim())
                            .filter(|r| !r.is_empty())
                            .unwrap_or("");
                        contents.push(AssistantContent::reasoning(reasoning));
                    } else if let Some(ref reasoning) = msg.reasoning_content {
                        if !reasoning.trim().is_empty() {
                            contents.push(AssistantContent::reasoning(reasoning.clone()));
                        }
                    }
                    
                    // Add tool calls and track valid tool_call_ids
                    if has_tool_calls {
                        if let Some(ref tc_str) = msg.tool_calls {
                            if let Ok(tool_calls) = serde_json::from_str::<Vec<ToolCall>>(tc_str) {
                                for tc in tool_calls {
                                    // Track this tool_call_id as valid
                                    valid_tool_call_ids.insert(tc.id.clone());
                                    
                                    contents.push(AssistantContent::tool_call(
                                        tc.id.clone(),
                                        tc.function.name.clone(),
                                        tc.function.arguments.clone(),
                                    ));
                                }
                            }
                        }
                    }
                    
                    // Add text content
                    if has_content {
                        contents.push(AssistantContent::Text(rig::message::Text::from(content.to_string())));
                    }
                    
                    if !contents.is_empty() {
                        let assistant_msg = Message::Assistant {
                            id: None,
                            content: if contents.len() == 1 {
                                OneOrMany::one(contents.into_iter().next().unwrap())
                            } else {
                                match OneOrMany::many(contents) {
                                    Ok(m) => m,
                                    Err(_) => {
                                        i += 1;
                                        continue;
                                    }
                                }
                            },
                        };
                        result.push(assistant_msg);
                    }
                }
                i += 1;
            }
            "tool" => {
                // Collect all consecutive tool messages and merge into a single User message
                // This is required by Anthropic API which expects tool_results in the same message
                // IMPORTANT: Only include tool messages that have corresponding tool_calls from previous assistant message
                let mut tool_results: Vec<UserContent> = Vec::new();
                let mut orphaned_tools = 0;
                
                while i < history.len() && history[i].role.to_lowercase() == "tool" {
                    let tool_msg = &history[i];
                    let tool_content = tool_msg.content.trim();
                    
                    if !tool_content.is_empty() {
                        if let Some(ref tool_call_id) = tool_msg.tool_call_id {
                            // Only process if this tool_call_id was present in a previous assistant message
                            if valid_tool_call_ids.contains(tool_call_id) {
                                // Deduplicate: skip if already seen
                                if !seen_tool_call_ids.contains(tool_call_id) {
                                    seen_tool_call_ids.insert(tool_call_id.clone());
                                    tool_results.push(UserContent::ToolResult(ToolResult {
                                        id: tool_call_id.clone(),
                                        call_id: None,
                                        content: OneOrMany::one(ToolResultContent::Text(Text {
                                            text: tool_content.to_string(),
                                        })),
                                    }));
                                }
                            } else {
                                // Skip orphaned tool message without corresponding tool_call
                                orphaned_tools += 1;
                                tracing::warn!(
                                    "Skipping orphaned tool message with id '{}' - no corresponding tool_call found in previous assistant messages",
                                    tool_call_id
                                );
                            }
                        }
                    }
                    i += 1;
                }
                
                if orphaned_tools > 0 {
                    tracing::info!(
                        "Filtered out {} orphaned tool message(s) to prevent API errors",
                        orphaned_tools
                    );
                }
                
                // Create merged User message with all tool results
                if !tool_results.is_empty() {
                    let user_msg = if tool_results.len() == 1 {
                        Message::User {
                            content: OneOrMany::one(tool_results.into_iter().next().unwrap()),
                        }
                    } else {
                        match OneOrMany::many(tool_results) {
                            Ok(content) => Message::User { content },
                            Err(_) => continue,
                        }
                    };
                    result.push(user_msg);
                }
            }
            _ => {
                i += 1;
            }
        }
    }
    
    result
}

// ============================================================================
// 图片附件
// ============================================================================

/// 图片附件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageAttachment {
    /// base64 编码的图片数据
    pub base64_data: String,
    /// 媒体类型：png, jpeg, webp, gif
    pub media_type: String,
}

impl ImageAttachment {
    /// 创建新的图片附件
    pub fn new(base64_data: impl Into<String>, media_type: impl Into<String>) -> Self {
        Self {
            base64_data: base64_data.into(),
            media_type: media_type.into(),
        }
    }

    /// 从 base64 数据和媒体类型创建
    pub fn from_base64(base64_data: impl Into<String>, media_type: impl Into<String>) -> Self {
        Self::new(base64_data, media_type)
    }

    /// 解析媒体类型为 rig 的 ImageMediaType
    pub(crate) fn to_image_media_type(&self) -> ImageMediaType {
        match self.media_type.to_lowercase().as_str() {
            "png" => ImageMediaType::PNG,
            "webp" => ImageMediaType::WEBP,
            "gif" => ImageMediaType::GIF,
            "jpeg" | "jpg" => ImageMediaType::JPEG,
            _ => ImageMediaType::JPEG, // 默认 JPEG
        }
    }

    /// 转换为 rig 的 Image 对象
    pub(crate) fn to_rig_image(&self) -> Image {
        Image {
            data: DocumentSourceKind::base64(&self.base64_data),
            media_type: Some(self.to_image_media_type()),
            detail: Some(ImageDetail::Auto),
            additional_params: None,
        }
    }
}

// ============================================================================
// 消息构建辅助函数
// ============================================================================

/// 构建用户消息（可能包含图片）
pub fn build_user_message(user_prompt: &str, image: Option<&ImageAttachment>) -> Message {
    if let Some(img) = image {
        Message::User {
            content: OneOrMany::many(vec![
                UserContent::Image(img.to_rig_image()),
                UserContent::text(user_prompt.to_string()),
            ])
            .expect("Failed to create multi-content message"),
        }
    } else {
        Message::User {
            content: OneOrMany::one(UserContent::text(user_prompt.to_string())),
        }
    }
}

/// 从 JSON 附件解析图片
///
/// 支持的 JSON 格式：
/// ```json
/// [{ "type": "image", "media_type": "png", "data": { "type": "base64", "data": "..." } }]
/// ```
pub fn parse_image_from_json(attachments: Option<&Value>) -> Option<ImageAttachment> {
    let att_json = attachments?;
    let arr = att_json.as_array()?;
    let first = arr.first()?;

    let is_image = first
        .get("type")
        .and_then(|v| v.as_str())
        .map(|t| t.eq_ignore_ascii_case("image"))
        .unwrap_or(false);

    if !is_image {
        return None;
    }

    let media_type = first
        .get("media_type")
        .and_then(|v| v.as_str())
        .unwrap_or("jpeg")
        .to_string();

    let base64_data = first
        .get("data")
        .and_then(|v| v.as_object())
        .and_then(|obj| obj.get("data"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if base64_data.is_empty() {
        return None;
    }

    Some(ImageAttachment::new(base64_data, media_type))
}

