-- RAG系统数据库迁移
-- 创建时间: 2025-01-16
-- 描述: 为RAG（检索增强生成）系统创建必要的数据库表结构

BEGIN TRANSACTION;

-- ============================================================================
-- RAG集合管理表
-- ============================================================================

-- RAG集合表
CREATE TABLE IF NOT EXISTS rag_collections (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    embedding_model TEXT NOT NULL DEFAULT 'ollama:nomic-embed-text',
    embedding_dimension INTEGER NOT NULL DEFAULT 768,
    chunk_size INTEGER NOT NULL DEFAULT 1000,
    chunk_overlap INTEGER NOT NULL DEFAULT 200,
    status TEXT NOT NULL DEFAULT 'active', -- active, inactive, indexing, error
    document_count INTEGER NOT NULL DEFAULT 0,
    chunk_count INTEGER NOT NULL DEFAULT 0,
    total_size_bytes INTEGER NOT NULL DEFAULT 0,
    metadata TEXT NOT NULL DEFAULT '{}', -- JSON格式的元数据
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    created_by TEXT NOT NULL DEFAULT 'system'
);

-- ============================================================================
-- 文档管理表
-- ============================================================================

-- 文档表
CREATE TABLE IF NOT EXISTS rag_documents (
    id TEXT PRIMARY KEY,
    collection_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    file_name TEXT NOT NULL,
    file_type TEXT NOT NULL, -- pdf, docx, txt, md, html, etc.
    file_size_bytes INTEGER NOT NULL,
    file_hash TEXT NOT NULL, -- 文件内容哈希，用于去重
    title TEXT,
    author TEXT,
    language TEXT DEFAULT 'zh',
    encoding TEXT DEFAULT 'utf-8',
    status TEXT NOT NULL DEFAULT 'pending', -- pending, processing, completed, failed
    error_message TEXT,
    chunk_count INTEGER NOT NULL DEFAULT 0,
    processing_time_ms INTEGER,
    metadata TEXT NOT NULL DEFAULT '{}', -- JSON格式的元数据
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    processed_at INTEGER,
    FOREIGN KEY (collection_id) REFERENCES rag_collections(id) ON DELETE CASCADE
);

-- ============================================================================
-- 文档块表
-- ============================================================================

-- 文档块表
CREATE TABLE IF NOT EXISTS rag_chunks (
    id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL,
    collection_id TEXT NOT NULL,
    chunk_index INTEGER NOT NULL, -- 在文档中的块索引
    content TEXT NOT NULL,
    content_hash TEXT NOT NULL, -- 内容哈希，用于去重
    token_count INTEGER,
    char_count INTEGER NOT NULL,
    start_position INTEGER, -- 在原文档中的起始位置
    end_position INTEGER, -- 在原文档中的结束位置
    page_number INTEGER, -- 页码（如果适用）
    section_title TEXT, -- 章节标题
    embedding_vector BLOB, -- 嵌入向量（二进制存储）
    embedding_model TEXT NOT NULL,
    embedding_dimension INTEGER NOT NULL,
    similarity_threshold REAL DEFAULT 0.7,
    metadata TEXT NOT NULL DEFAULT '{}', -- JSON格式的元数据
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (document_id) REFERENCES rag_documents(id) ON DELETE CASCADE,
    FOREIGN KEY (collection_id) REFERENCES rag_collections(id) ON DELETE CASCADE
);

-- ============================================================================
-- 查询历史表
-- ============================================================================

-- 查询历史表
CREATE TABLE IF NOT EXISTS rag_query_history (
    id TEXT PRIMARY KEY,
    collection_id TEXT NOT NULL,
    query_text TEXT NOT NULL,
    query_hash TEXT NOT NULL, -- 查询哈希，用于缓存
    query_type TEXT NOT NULL DEFAULT 'similarity', -- similarity, hybrid, keyword
    top_k INTEGER NOT NULL DEFAULT 5,
    similarity_threshold REAL DEFAULT 0.7,
    result_count INTEGER NOT NULL DEFAULT 0,
    processing_time_ms INTEGER NOT NULL,
    embedding_time_ms INTEGER,
    search_time_ms INTEGER,
    rerank_time_ms INTEGER,
    metadata TEXT NOT NULL DEFAULT '{}', -- JSON格式的元数据
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (collection_id) REFERENCES rag_collections(id) ON DELETE CASCADE
);

-- ============================================================================
-- 查询结果表
-- ============================================================================

-- 查询结果表
CREATE TABLE IF NOT EXISTS rag_query_results (
    id TEXT PRIMARY KEY,
    query_id TEXT NOT NULL,
    chunk_id TEXT NOT NULL,
    similarity_score REAL NOT NULL,
    rank_position INTEGER NOT NULL,
    rerank_score REAL,
    metadata TEXT NOT NULL DEFAULT '{}', -- JSON格式的元数据
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (query_id) REFERENCES rag_query_history(id) ON DELETE CASCADE,
    FOREIGN KEY (chunk_id) REFERENCES rag_chunks(id) ON DELETE CASCADE
);

-- ============================================================================
-- 嵌入模型配置表
-- ============================================================================

-- 嵌入模型配置表
CREATE TABLE IF NOT EXISTS rag_embedding_models (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    provider TEXT NOT NULL, -- ollama, openai, huggingface, local
    model_name TEXT NOT NULL,
    api_endpoint TEXT,
    api_key_env TEXT, -- 环境变量名
    dimension INTEGER NOT NULL,
    max_tokens INTEGER DEFAULT 512,
    batch_size INTEGER DEFAULT 32,
    status TEXT NOT NULL DEFAULT 'active', -- active, inactive, error
    performance_score REAL, -- 性能评分
    cost_per_1k_tokens REAL, -- 每1k token成本
    metadata TEXT NOT NULL DEFAULT '{}', -- JSON格式的元数据
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- ============================================================================
-- 系统配置表
-- ============================================================================

-- RAG系统配置表
CREATE TABLE IF NOT EXISTS rag_system_config (
    id TEXT PRIMARY KEY,
    config_key TEXT NOT NULL UNIQUE,
    config_value TEXT NOT NULL,
    config_type TEXT NOT NULL DEFAULT 'string', -- string, integer, float, boolean, json
    description TEXT,
    is_sensitive BOOLEAN NOT NULL DEFAULT FALSE, -- 是否为敏感配置
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- ============================================================================
-- 索引创建
-- ============================================================================

-- 集合表索引
CREATE INDEX IF NOT EXISTS idx_rag_collections_name ON rag_collections(name);
CREATE INDEX IF NOT EXISTS idx_rag_collections_status ON rag_collections(status);
CREATE INDEX IF NOT EXISTS idx_rag_collections_created_at ON rag_collections(created_at);

-- 文档表索引
CREATE INDEX IF NOT EXISTS idx_rag_documents_collection_id ON rag_documents(collection_id);
CREATE INDEX IF NOT EXISTS idx_rag_documents_file_hash ON rag_documents(file_hash);
CREATE INDEX IF NOT EXISTS idx_rag_documents_status ON rag_documents(status);
CREATE INDEX IF NOT EXISTS idx_rag_documents_file_type ON rag_documents(file_type);
CREATE INDEX IF NOT EXISTS idx_rag_documents_created_at ON rag_documents(created_at);

-- 文档块表索引
CREATE INDEX IF NOT EXISTS idx_rag_chunks_document_id ON rag_chunks(document_id);
CREATE INDEX IF NOT EXISTS idx_rag_chunks_collection_id ON rag_chunks(collection_id);
CREATE INDEX IF NOT EXISTS idx_rag_chunks_content_hash ON rag_chunks(content_hash);
CREATE INDEX IF NOT EXISTS idx_rag_chunks_chunk_index ON rag_chunks(chunk_index);
CREATE INDEX IF NOT EXISTS idx_rag_chunks_embedding_model ON rag_chunks(embedding_model);

-- 查询历史表索引
CREATE INDEX IF NOT EXISTS idx_rag_query_history_collection_id ON rag_query_history(collection_id);
CREATE INDEX IF NOT EXISTS idx_rag_query_history_query_hash ON rag_query_history(query_hash);
CREATE INDEX IF NOT EXISTS idx_rag_query_history_created_at ON rag_query_history(created_at);

-- 查询结果表索引
CREATE INDEX IF NOT EXISTS idx_rag_query_results_query_id ON rag_query_results(query_id);
CREATE INDEX IF NOT EXISTS idx_rag_query_results_chunk_id ON rag_query_results(chunk_id);
CREATE INDEX IF NOT EXISTS idx_rag_query_results_similarity_score ON rag_query_results(similarity_score);

-- 嵌入模型配置表索引
CREATE INDEX IF NOT EXISTS idx_rag_embedding_models_name ON rag_embedding_models(name);
CREATE INDEX IF NOT EXISTS idx_rag_embedding_models_provider ON rag_embedding_models(provider);
CREATE INDEX IF NOT EXISTS idx_rag_embedding_models_status ON rag_embedding_models(status);

-- 系统配置表索引
CREATE INDEX IF NOT EXISTS idx_rag_system_config_key ON rag_system_config(config_key);

-- ============================================================================
-- 初始数据插入
-- ============================================================================

-- 插入默认嵌入模型配置
INSERT OR IGNORE INTO rag_embedding_models (id, name, provider, model_name, dimension, max_tokens, batch_size) VALUES
('ollama_nomic', 'Ollama Nomic Embed Text', 'ollama', 'nomic-embed-text', 768, 512, 32),
('ollama_mxbai', 'Ollama MxBai Embed Large', 'ollama', 'mxbai-embed-large', 1024, 512, 16);

-- 插入默认系统配置
INSERT OR IGNORE INTO rag_system_config (id, config_key, config_value, config_type, description) VALUES
('default_collection', 'default_collection_name', 'default', 'string', '默认RAG集合名称'),
('chunk_size', 'default_chunk_size', '1000', 'integer', '默认文档块大小'),
('chunk_overlap', 'default_chunk_overlap', '200', 'integer', '默认文档块重叠大小'),
('similarity_threshold', 'default_similarity_threshold', '0.7', 'float', '默认相似度阈值'),
('top_k', 'default_top_k', '5', 'integer', '默认返回结果数量'),
('embedding_model', 'default_embedding_model', 'ollama_nomic', 'string', '默认嵌入模型'),
('max_file_size', 'max_file_size_mb', '100', 'integer', '最大文件大小(MB)'),
('supported_formats', 'supported_file_formats', '["pdf","docx","txt","md","html","rtf"]', 'json', '支持的文件格式');

COMMIT;