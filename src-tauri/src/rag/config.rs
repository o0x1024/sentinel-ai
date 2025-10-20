use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagConfig {
    pub database_path: Option<PathBuf>,
    pub chunk_size_chars: usize,
    pub chunk_overlap_chars: usize,
    pub top_k: usize,
    pub mmr_lambda: f32,
    pub batch_size: usize,
    pub max_concurrent: usize,
    pub embedding_provider: String,
    pub embedding_model: String,
    pub embedding_dimensions: Option<usize>,
    pub embedding_api_key: Option<String>,
    pub embedding_base_url: Option<String>,
    pub reranking_provider: Option<String>,
    pub reranking_model: Option<String>,
    pub reranking_enabled: bool,
    pub similarity_threshold: f32,
    /// 是否启用聊天与任务的知识库增强（全局开关）
    #[serde(default)]
    pub augmentation_enabled: bool,
    /// 上下文窗口大小：检索到相关块后，前后各扩展多少个块
    #[serde(default = "default_context_window")]
    pub context_window_size: usize,
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            database_path: Some(PathBuf::from("AppData/lancedb")),
            chunk_size_chars: 1000,
            chunk_overlap_chars: 200,
            top_k: 5,
            mmr_lambda: 0.7,
            batch_size: 10,
            max_concurrent: 4,
            embedding_provider: "ollama".to_string(),
            embedding_model: "nomic-embed-text".to_string(),
            embedding_dimensions: None,
            embedding_api_key: None,
            embedding_base_url: Some("http://localhost:11434".to_string()),
            reranking_provider: None,
            reranking_model: None,
            reranking_enabled: false,
            similarity_threshold: 0.7,
            augmentation_enabled: false,
            context_window_size: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub dimensions: Option<usize>,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: "ollama".to_string(),
            model: "nomic-embed-text".to_string(),
            api_key: None,
            base_url: Some("http://localhost:11434".to_string()),
            dimensions: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SupportedFileType {
    Txt,
    Md,
    Docx,
    Pdf,
}

impl SupportedFileType {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "txt" => Some(Self::Txt),
            "md" | "markdown" => Some(Self::Md),
            "docx" => Some(Self::Docx),
            "pdf" => Some(Self::Pdf),
            _ => None,
        }
    }
}

fn default_context_window() -> usize {
    1
}