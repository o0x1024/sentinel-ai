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
    #[serde(default)]
    pub augmentation_enabled: bool,
    #[serde(default = "default_context_window")]
    pub context_window_size: usize,
    #[serde(default)]
    pub chunking_strategy: ChunkingStrategy,
    #[serde(default = "default_min_chunk_size")]
    pub min_chunk_size_chars: usize,
    #[serde(default = "default_max_chunk_size")]
    pub max_chunk_size_chars: usize,
    /// Enable automatic expansion of retrieved chunks with adjacent chunks
    #[serde(default = "default_chunk_expansion_enabled")]
    pub chunk_expansion_enabled: bool,
    /// Number of adjacent chunks to include before the matched chunk
    #[serde(default = "default_chunk_expansion_before")]
    pub chunk_expansion_before: usize,
    /// Number of adjacent chunks to include after the matched chunk
    #[serde(default = "default_chunk_expansion_after")]
    pub chunk_expansion_after: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub enum ChunkingStrategy {
    FixedSize,
    #[default]
    RecursiveCharacter,
    Semantic,
    StructureAware,
}


impl Default for RagConfig {
    fn default() -> Self {
        Self {
            database_path: Some(PathBuf::from("AppData/lancedb")),
            chunk_size_chars: 1500,
            chunk_overlap_chars: 150,
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
            similarity_threshold: 0.2,
            augmentation_enabled: false,
            context_window_size: 1,
            chunking_strategy: ChunkingStrategy::RecursiveCharacter,
            min_chunk_size_chars: 100,
            max_chunk_size_chars: 3000,
            chunk_expansion_enabled: true,
            chunk_expansion_before: 1,
            chunk_expansion_after: 1,
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
    
    /// Get all supported file extensions
    pub fn all_extensions() -> Vec<String> {
        vec![
            "txt".to_string(),
            "md".to_string(),
            "markdown".to_string(),
            "docx".to_string(),
            "pdf".to_string(),
        ]
    }
    
    /// Get primary extensions (without aliases)
    pub fn primary_extensions() -> Vec<String> {
        vec![
            "txt".to_string(),
            "md".to_string(),
            "docx".to_string(),
            "pdf".to_string(),
        ]
    }
}

fn default_context_window() -> usize { 1 }
fn default_min_chunk_size() -> usize { 100 }
fn default_max_chunk_size() -> usize { 3000 }
fn default_chunk_expansion_enabled() -> bool { true }
fn default_chunk_expansion_before() -> usize { 1 }
fn default_chunk_expansion_after() -> usize { 1 }

