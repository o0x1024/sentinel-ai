//! 消息类型模块

use rig::completion::{message::Image, Message};
use rig::message::{AssistantContent, DocumentSourceKind, ImageDetail, ImageMediaType, UserContent};
use rig::one_or_many::OneOrMany;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ============================================================================
// 聊天消息
// ============================================================================

/// 聊天消息（用于多轮对话历史）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// 角色：user 或 assistant
    pub role: String,
    /// 消息内容
    pub content: String,
}

impl ChatMessage {
    /// 创建用户消息
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    /// 创建助手消息
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }

    /// 创建新消息
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
        }
    }
}

/// 将 ChatMessage 列表转换为 rig Message 列表
pub fn convert_chat_history(history: &[ChatMessage]) -> Vec<Message> {
    history
        .iter()
        .filter_map(|msg| {
            let content = msg.content.trim();
            if content.is_empty() {
                return None;
            }
            match msg.role.to_lowercase().as_str() {
                "user" => Some(Message::User {
                    content: OneOrMany::one(UserContent::text(content.to_string())),
                }),
                "assistant" => Some(Message::Assistant {
                    id: None,
                    content: OneOrMany::one(AssistantContent::Text(rig::message::Text::from(
                        content.to_string(),
                    ))),
                }),
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

