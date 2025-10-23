// Test file for Rig RAG implementation based on official docs
#[cfg(test)]
mod tests {
    
    use crate::rag::{
        database::LanceDbManager,
        models::{DocumentChunk, ChunkMetadata},
    };
    use std::collections::HashMap;
    use tempfile::tempdir;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_rig_rag_basic_flow() -> anyhow::Result<()> {
        // Create temporary directory for LanceDB
        let temp_dir = tempdir()?;
        let db_path = temp_dir.path().join("test_lancedb");
        
        // Initialize vector store manager with embedding config
        let embedding_config = crate::rag::config::EmbeddingConfig::default();
        let manager = LanceDbManager::new(db_path.to_string_lossy().to_string(), embedding_config);
        manager.initialize().await?;
        
        // Create test collection
        manager.create_collection("test_collection", 768).await?;
        
        // Create test documents following Rig pattern
        let test_chunks = vec![
            DocumentChunk {
                id: Uuid::new_v4().to_string(),
                source_id: "doc1".to_string(),
                content: "Rust is a systems programming language focused on safety and performance.".to_string(),
                content_hash: "hash1".to_string(),
                chunk_index: 0,
                metadata: ChunkMetadata {
                    file_path: "/test/doc1.txt".to_string(),
                    file_name: "doc1.txt".to_string(),
                    file_type: "txt".to_string(),
                    file_size: 100,
                    chunk_start_char: 0,
                    chunk_end_char: 70,
                    page_number: Some(1),
                    section_title: None,
                    custom_fields: HashMap::new(),
                },
                embedding: None,
                created_at: chrono::Utc::now(),
            },
            DocumentChunk {
                id: Uuid::new_v4().to_string(),
                source_id: "doc2".to_string(),
                content: "LanceDB is a serverless vector database built on Apache Arrow.".to_string(),
                content_hash: "hash2".to_string(),
                chunk_index: 0,
                metadata: ChunkMetadata {
                    file_path: "/test/doc2.txt".to_string(),
                    file_name: "doc2.txt".to_string(),
                    file_type: "txt".to_string(),
                    file_size: 80,
                    chunk_start_char: 0,
                    chunk_end_char: 62,
                    page_number: Some(1),
                    section_title: None,
                    custom_fields: HashMap::new(),
                },
                embedding: None,
                created_at: chrono::Utc::now(),
            },
        ];
        
        // Insert documents using Rig EmbeddingsBuilder pattern
        let inserted_count = manager.insert_chunks("test_collection", test_chunks).await?;
        assert_eq!(inserted_count, 2);
        
        // Test semantic search using official top_n pattern
        let search_results = manager.search_similar("test_collection", "vector database", 5).await?;
        
        // Verify results
        assert!(!search_results.is_empty());
        println!("Search results: {:?}", search_results);
        
        // Test collection stats
        let (doc_count, chunk_count) = manager.get_collection_stats("test_collection").await?;
        assert_eq!(doc_count, 2);
        assert_eq!(chunk_count, 2);
        
        // Test collection listing
        let collections = manager.list_collections().await?;
        assert!(collections.contains(&"test_collection".to_string()));
        
        // Cleanup
        manager.delete_collection("test_collection").await?;
        
        Ok(())
    }

    // Removed embedding macro-based test to avoid derive macro dependency in test scope.
}
