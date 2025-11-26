use serde::{Deserialize, Serialize};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};

/// 图片媒体类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ImageMediaType {
    JPEG,
    PNG,
    GIF,
    WEBP,
}

impl ImageMediaType {
    /// 从文件扩展名推断媒体类型
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "jpg" | "jpeg" => Some(ImageMediaType::JPEG),
            "png" => Some(ImageMediaType::PNG),
            "gif" => Some(ImageMediaType::GIF),
            "webp" => Some(ImageMediaType::WEBP),
            _ => None,
        }
    }

    /// 获取 MIME 类型字符串
    pub fn to_mime_type(&self) -> &'static str {
        match self {
            ImageMediaType::JPEG => "image/jpeg",
            ImageMediaType::PNG => "image/png",
            ImageMediaType::GIF => "image/gif",
            ImageMediaType::WEBP => "image/webp",
        }
    }
}

/// 文档源类型（与 Rig 兼容）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DocumentSourceKind {
    Base64 { data: String },
    Url { url: String },
}

impl DocumentSourceKind {
    /// 创建 base64 类型的文档源
    pub fn base64(data: &str) -> Self {
        DocumentSourceKind::Base64 {
            data: data.to_string(),
        }
    }

    /// 创建 URL 类型的文档源
    pub fn url(url: &str) -> Self {
        DocumentSourceKind::Url {
            url: url.to_string(),
        }
    }
}

/// 图片附件（与 Rig Image 结构兼容）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageAttachment {
    /// 图片数据源（base64 或 URL）
    pub data: DocumentSourceKind,
    /// 媒体类型
    pub media_type: Option<ImageMediaType>,
    /// 文件名（可选）
    pub filename: Option<String>,
    /// 图片详细描述级别（low, high, auto）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl ImageAttachment {
    /// 从字节数据创建图片附件
    pub fn from_bytes(bytes: &[u8], media_type: ImageMediaType, filename: Option<String>) -> Self {
        let base64_data = BASE64_STANDARD.encode(bytes);
        Self {
            data: DocumentSourceKind::base64(&base64_data),
            media_type: Some(media_type),
            filename,
            detail: None,
        }
    }

    /// 从 base64 字符串创建图片附件
    pub fn from_base64(
        base64_data: String,
        media_type: ImageMediaType,
        filename: Option<String>,
    ) -> Self {
        Self {
            data: DocumentSourceKind::base64(&base64_data),
            media_type: Some(media_type),
            filename,
            detail: None,
        }
    }

    /// 从 URL 创建图片附件
    pub fn from_url(url: String, media_type: Option<ImageMediaType>) -> Self {
        Self {
            data: DocumentSourceKind::url(&url),
            media_type,
            filename: None,
            detail: None,
        }
    }

    /// 设置详细级别
    pub fn with_detail(mut self, detail: &str) -> Self {
        self.detail = Some(detail.to_string());
        self
    }
}

/// 消息附件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageAttachment {
    /// 图片附件
    Image(ImageAttachment),
    /// 文件附件（未来扩展）
    File {
        filename: String,
        data: String, // base64
        mime_type: String,
    },
}

impl MessageAttachment {
    /// 判断是否为图片附件
    pub fn is_image(&self) -> bool {
        matches!(self, MessageAttachment::Image(_))
    }

    /// 获取图片附件
    pub fn as_image(&self) -> Option<&ImageAttachment> {
        match self {
            MessageAttachment::Image(img) => Some(img),
            _ => None,
        }
    }
}

/// 从文件路径读取图片并创建附件
pub async fn load_image_from_path(file_path: &str) -> anyhow::Result<ImageAttachment> {
    use std::path::Path;

    let path = Path::new(file_path);
    
    // 获取文件扩展名
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| anyhow::anyhow!("无法获取文件扩展名"))?;

    // 推断媒体类型
    let media_type = ImageMediaType::from_extension(extension)
        .ok_or_else(|| anyhow::anyhow!("不支持的图片格式: {}", extension))?;

    // 读取文件内容
    let bytes = tokio::fs::read(file_path).await?;

    // 获取文件名
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string());

    Ok(ImageAttachment::from_bytes(&bytes, media_type, filename))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_media_type_from_extension() {
        assert_eq!(
            ImageMediaType::from_extension("jpg"),
            Some(ImageMediaType::JPEG)
        );
        assert_eq!(
            ImageMediaType::from_extension("PNG"),
            Some(ImageMediaType::PNG)
        );
        assert_eq!(ImageMediaType::from_extension("gif"), Some(ImageMediaType::GIF));
        assert_eq!(ImageMediaType::from_extension("txt"), None);
    }

    #[test]
    fn test_image_attachment_from_base64() {
        let attachment = ImageAttachment::from_base64(
            "iVBORw0KGgoAAAANS...".to_string(),
            ImageMediaType::PNG,
            Some("test.png".to_string()),
        );

        assert!(matches!(attachment.data, DocumentSourceKind::Base64 { .. }));
        assert_eq!(attachment.media_type, Some(ImageMediaType::PNG));
        assert_eq!(attachment.filename, Some("test.png".to_string()));
    }

    #[test]
    fn test_message_attachment_is_image() {
        let img_attachment = ImageAttachment::from_url(
            "https://example.com/image.jpg".to_string(),
            Some(ImageMediaType::JPEG),
        );
        let attachment = MessageAttachment::Image(img_attachment);

        assert!(attachment.is_image());
        assert!(attachment.as_image().is_some());
    }
}
