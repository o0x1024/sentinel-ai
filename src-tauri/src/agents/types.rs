//! Shared agent types.

/// Document attachment info for prompt injection.
#[derive(Debug, Clone)]
pub struct DocumentAttachmentInfo {
    pub id: String,
    pub original_filename: String,
    pub file_size: u64,
    pub mime_type: String,
    pub processing_mode: String,
    pub extracted_text: Option<String>,
    pub container_path: Option<String>,
}

