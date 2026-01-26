use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagConfig {
    pub database_path: Option<PathBuf>,
    pub chunk_size_chars: usize,
    pub chunk_overlap_chars: usize,
    pub top_k: usize,
    pub mmr_lambda: f64,
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
    pub similarity_threshold: f64,
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
    #[serde(default = "default_chunk_expansion_enabled")]
    pub chunk_expansion_enabled: bool,
    #[serde(default = "default_chunk_expansion_before")]
    pub chunk_expansion_before: usize,
    #[serde(default = "default_chunk_expansion_after")]
    pub chunk_expansion_after: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ChunkingStrategy { 
    FixedSize, 
    #[default]
    RecursiveCharacter, 
    Semantic, 
    StructureAware 
}

fn default_context_window() -> usize { 1 }
fn default_min_chunk_size() -> usize { 100 }
fn default_max_chunk_size() -> usize { 3000 }
fn default_chunk_expansion_enabled() -> bool { true }
fn default_chunk_expansion_before() -> usize { 1 }
fn default_chunk_expansion_after() -> usize { 1 }

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
            similarity_threshold: 0.7,
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

