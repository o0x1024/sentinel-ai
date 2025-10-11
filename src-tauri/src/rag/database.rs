use anyhow::{anyhow, Result};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use std::collections::HashMap;

use crate::rag::models::{DocumentChunk, QueryResult};

// 暂时使用简单的内存存储，等LanceDB版本兼容性问题解决后再切换
pub struct LanceDbManager {
    database_path: String,
    collections: Arc<RwLock<HashMap<String, Vec<DocumentChunk>>>>,
}

impl LanceDbManager {
    pub fn new(database_path: String) -> Self {
        Self {
            database_path,
            collections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn initialize(&self) -> Result<()> {
        // 确保数据库目录存在
        let db_path = Path::new(&self.database_path);
        if let Some(parent) = db_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        info!("LanceDB manager initialized at: {} (using temporary in-memory storage)", self.database_path);
        Ok(())
    }

    pub async fn create_collection(&self, collection_name: &str, embedding_dim: usize) -> Result<()> {
        let mut collections = self.collections.write().await;
        
        if collections.contains_key(collection_name) {
            info!("Collection '{}' already exists", collection_name);
            return Ok(());
        }

        collections.insert(collection_name.to_string(), Vec::new());
        info!("Created collection: {} with embedding dimension: {}", collection_name, embedding_dim);
        Ok(())
    }

    pub async fn insert_chunks(&self, collection_name: &str, chunks: Vec<DocumentChunk>) -> Result<usize> {
        if chunks.is_empty() {
            return Ok(0);
        }

        let mut collections = self.collections.write().await;
        let collection = collections.entry(collection_name.to_string()).or_insert_with(Vec::new);
        
        let count = chunks.len();
        collection.extend(chunks);
        
        info!("Inserted {} chunks into collection '{}'", count, collection_name);
        Ok(count)
    }

    pub async fn search_similar(&self, collection_name: &str, query_embedding: Vec<f32>, top_k: usize) -> Result<Vec<QueryResult>> {
        let collections = self.collections.read().await;
        let collection = collections.get(collection_name)
            .ok_or_else(|| anyhow!("Collection '{}' not found", collection_name))?;

        // 简单的相似度计算（余弦相似度）
        let mut results = Vec::new();
        
        for (index, chunk) in collection.iter().enumerate() {
            if let Some(ref embedding) = chunk.embedding {
                let similarity = cosine_similarity(&query_embedding, embedding);
                results.push(QueryResult {
                    chunk: chunk.clone(),
                    score: similarity,
                    rank: index,
                });
            }
        }

        // 按相似度排序并取top_k
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);
        
        // 更新排名
        for (i, result) in results.iter_mut().enumerate() {
            result.rank = i;
        }

        info!("Found {} similar chunks in collection '{}'", results.len(), collection_name);
        Ok(results)
    }

    pub async fn delete_collection(&self, collection_name: &str) -> Result<()> {
        let mut collections = self.collections.write().await;
        collections.remove(collection_name);
        info!("Deleted collection: {}", collection_name);
        Ok(())
    }

    pub async fn list_collections(&self) -> Result<Vec<String>> {
        let collections = self.collections.read().await;
        Ok(collections.keys().cloned().collect())
    }

    pub async fn get_collection_stats(&self, collection_name: &str) -> Result<(usize, usize)> {
        let collections = self.collections.read().await;
        let collection = collections.get(collection_name)
            .ok_or_else(|| anyhow!("Collection '{}' not found", collection_name))?;

        let chunk_count = collection.len();
        // 计算唯一文档数
        let mut unique_sources = std::collections::HashSet::new();
        for chunk in collection {
            unique_sources.insert(&chunk.source_id);
        }
        let document_count = unique_sources.len();

        Ok((document_count, chunk_count))
    }
}

// 计算余弦相似度
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
}