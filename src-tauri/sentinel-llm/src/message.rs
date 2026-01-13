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
pub fn convert_chat_history(history: &[ChatMessage]) -> Vec<Message> {
    use std::collections::HashSet;
    
    // Track seen tool_call_ids to prevent duplicate tool_results
    // Anthropic API requires each tool_use to have exactly one tool_result
    let mut seen_tool_call_ids: HashSet<String> = HashSet::new();
    
    history
        .iter()
        .filter_map(|msg| {
            let content = msg.content.trim();
            let role = msg.role.to_lowercase();
            
            match role.as_str() {
                "user" => {
                    if content.is_empty() {
                        return None;
                    }
                    Some(Message::User {
                        content: OneOrMany::one(UserContent::text(content.to_string())),
                    })
                },
                "assistant" => {
                    // assistant 消息可能只有 tool_calls 而没有 content
                    let has_content = !content.is_empty();
                    let has_tool_calls = msg.tool_calls.as_ref().map(|tc| !tc.trim().is_empty()).unwrap_or(false);
                    
                    if !has_content && !has_tool_calls {
                        return None;
                    }
                    
                    // 构建 AssistantContent
                    let mut contents = Vec::new();
                    
                    // 根据 DeepSeek API 要求，如果有 tool_calls，必须先添加 reasoning_content
                    // 参考：https://api-docs.deepseek.com/guides/thinking_mode#tool-calls
                    if has_tool_calls {
                        // 如果有推理内容，使用它；否则使用空字符串
                        let reasoning = msg.reasoning_content.as_ref()
                            .map(|r| r.trim())
                            .filter(|r| !r.is_empty())
                            .unwrap_or("");
                        contents.push(AssistantContent::reasoning(reasoning));
                    } else if let Some(ref reasoning) = msg.reasoning_content {
                        // 没有 tool_calls 但有 reasoning_content 的情况
                        if !reasoning.trim().is_empty() {
                            contents.push(AssistantContent::reasoning(reasoning.clone()));
                        }
                    }
                    
                    // 添加工具调用（如果有）
                    if has_tool_calls {
                        if let Some(ref tc_str) = msg.tool_calls {
                            if let Ok(tool_calls) = serde_json::from_str::<Vec<ToolCall>>(tc_str) {
                                for tc in tool_calls {
                                    contents.push(AssistantContent::tool_call(
                                        tc.id.clone(),
                                        tc.function.name.clone(),
                                        tc.function.arguments.clone(),
                                    ));
                                }
                            }
                        }
                    }
                    
                    // 添加文本内容（如果有）
                    if has_content {
                        contents.push(AssistantContent::Text(rig::message::Text::from(content.to_string())));
                    }
                    
                    if contents.is_empty() {
                        return None;
                    }
                    
                    Some(Message::Assistant {
                        id: None,
                        content: if contents.len() == 1 {
                            OneOrMany::one(contents.into_iter().next().unwrap())
                        } else {
                            OneOrMany::many(contents).ok()?
                        },
                    })
                },
                "tool" => {
                    // 处理 tool 角色的消息
                    if content.is_empty() {
                        return None;
                    }
                    // tool 消息必须有 tool_call_id
                    let tool_call_id = msg.tool_call_id.as_ref()?.clone();
                    
                    // Deduplicate: skip if already seen this tool_call_id
                    if seen_tool_call_ids.contains(&tool_call_id) {
                        return None;
                    }
                    seen_tool_call_ids.insert(tool_call_id.clone());
                    
                    Some(Message::tool_result(tool_call_id, content.to_string()))
                },
                _ => None,
            }
        })
        .collect()
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

