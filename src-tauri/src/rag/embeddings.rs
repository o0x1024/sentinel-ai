use anyhow::{anyhow, Result};
use async_trait::async_trait;
use log::{debug, info, warn, error};
use serde_json::Value;
use std::collections::HashMap;

use crate::ai_adapter::types::{AiProvider, ChatRequest, Message, MessageRole};
use crate::rag::config::EmbeddingConfig;

/// 嵌入服务特征
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// 生成文本嵌入向量
    async fn embed_texts(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
    
    /// 获取嵌入向量维度
    async fn get_embedding_dimension(&self) -> Result<usize>;
    
    /// 获取提供商名称
    fn provider_name(&self) -> &str;
    
    /// 获取模型名称
    fn model_name(&self) -> &str;
}

/// 重排序服务特征
#[async_trait]
pub trait RerankingProvider: Send + Sync {
    /// 对查询和文档列表进行重排序
    async fn rerank(&self, query: &str, documents: &[String], top_k: Option<usize>) -> Result<Vec<RerankResult>>;
    
    /// 获取提供商名称
    fn provider_name(&self) -> &str;
    
    /// 获取模型名称
    fn model_name(&self) -> &str;
}

/// 重排序结果
#[derive(Debug, Clone)]
pub struct RerankResult {
    pub index: usize,      // 原始文档在输入列表中的索引
    pub score: f32,        // 重排序分数
    pub document: String,  // 文档内容
}

/// 嵌入管理器
pub struct EmbeddingManager {
    providers: HashMap<String, Box<dyn EmbeddingProvider>>,
    current_provider: Option<String>,
}

impl EmbeddingManager {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            current_provider: None,
        }
    }

    /// 注册嵌入提供商
    pub fn register_provider(&mut self, provider: Box<dyn EmbeddingProvider>) {
        let provider_key = format!("{}:{}", provider.provider_name(), provider.model_name());
        info!("注册嵌入提供商: {}", provider_key);
        self.current_provider = Some(provider_key.clone());
        self.providers.insert(provider_key, provider);
    }

    /// 设置当前提供商
    pub fn set_current_provider(&mut self, provider_name: &str, model_name: &str) -> Result<()> {
        let provider_key = format!("{}:{}", provider_name, model_name);
        if self.providers.contains_key(&provider_key) {
            self.current_provider = Some(provider_key);
            Ok(())
        } else {
            Err(anyhow!("提供商 {} 未找到", provider_key))
        }
    }

    /// 获取当前提供商
    fn get_current_provider(&self) -> Result<&dyn EmbeddingProvider> {
        match &self.current_provider {
            Some(provider_key) => {
                self.providers.get(provider_key)
                    .map(|p| p.as_ref())
                    .ok_or_else(|| anyhow!("当前提供商 {} 未找到", provider_key))
            }
            None => Err(anyhow!("未设置当前提供商"))
        }
    }

    /// 生成文本嵌入向量
    pub async fn embed_texts(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let provider = self.get_current_provider()?;
        debug!("使用提供商 {} 生成 {} 个文本的嵌入向量", 
               provider.provider_name(), texts.len());
        
        let embeddings = provider.embed_texts(texts).await?;
        
        if embeddings.len() != texts.len() {
            return Err(anyhow!("嵌入向量数量与文本数量不匹配"));
        }
        
        Ok(embeddings)
    }

    /// 获取嵌入向量维度
    pub async fn get_embedding_dimension(&self) -> Result<usize> {
        let provider = self.get_current_provider()?;
        provider.get_embedding_dimension().await
    }

    /// 列出所有提供商
    pub fn list_providers(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }
}

/// 基础嵌入提供商实现
pub struct BasicEmbeddingProvider {
    provider_name: String,
    model_name: String,
    dimensions: usize,
}

impl BasicEmbeddingProvider {
    pub fn new(config: &EmbeddingConfig) -> Result<Self> {
        Ok(Self {
            provider_name: config.provider.clone(),
            model_name: config.model.clone(),
            dimensions: config.dimensions.unwrap_or(768),
        })
    }
}

#[async_trait]
impl EmbeddingProvider for BasicEmbeddingProvider {
    async fn embed_texts(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        // 基础实现：返回随机向量（仅用于测试）
        warn!("使用基础嵌入提供商，返回随机向量");
        let mut embeddings = Vec::new();
        for _ in texts {
            let embedding: Vec<f32> = (0..self.dimensions)
                .map(|_| rand::random::<f32>() - 0.5)
                .collect();
            embeddings.push(embedding);
        }
        Ok(embeddings)
    }

    async fn get_embedding_dimension(&self) -> Result<usize> {
        Ok(self.dimensions)
    }

    fn provider_name(&self) -> &str {
        &self.provider_name
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

/// LM Studio嵌入提供商
pub struct LmStudioEmbeddingProvider {
    model_name: String,
    base_url: String,
    api_key: Option<String>,
    dimensions: usize,
}

impl LmStudioEmbeddingProvider {
    pub fn new(config: &EmbeddingConfig) -> Result<Self> {
        Ok(Self {
            model_name: config.model.clone(),
            base_url: config.base_url.clone().unwrap_or_else(|| "http://localhost:1234".to_string()),
            api_key: config.api_key.clone(),
            dimensions: config.dimensions.unwrap_or(768),
        })
    }
}

#[async_trait]
impl EmbeddingProvider for LmStudioEmbeddingProvider {
    async fn embed_texts(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        info!("使用LM Studio生成嵌入向量，模型: {}，文本数量: {}", self.model_name, texts.len());
        
        let client = reqwest::Client::new();
        
        // 准备请求头
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse().unwrap());
        
        if let Some(api_key) = &self.api_key {
            headers.insert("Authorization", format!("Bearer {}", api_key).parse().unwrap());
        }
        
        // 根据LM Studio API格式构建请求
        let input = if texts.len() == 1 {
            // 单个文本直接传字符串
            serde_json::Value::String(texts[0].clone())
        } else {
            // 多个文本传数组
            serde_json::Value::Array(texts.iter().map(|t| serde_json::Value::String(t.clone())).collect())
        };
        
        let payload = serde_json::json!({
            "model": self.model_name,
            "input": input
        });
        
        info!("发送LM Studio嵌入请求: {}", serde_json::to_string(&payload).unwrap_or_default());
        
        let response = client
            .post(&format!("{}/v1/embeddings", self.base_url))
            .headers(headers)
            .json(&payload)
            .send()
            .await?;
        
        let status = response.status();
        let response_text = response.text().await?;
        
        if status.is_success() {
            info!("LM Studio响应成功: {}", response_text.chars().take(200).collect::<String>());
            
            let result: Value = serde_json::from_str(&response_text)?;
            
            if let Some(data) = result.get("data") {
                if let Some(data_array) = data.as_array() {
                    let mut embeddings = Vec::new();
                    
                    for item in data_array {
                        if let Some(embedding_array) = item.get("embedding") {
                            if let Some(embedding_vec) = embedding_array.as_array() {
                                let embedding: Vec<f32> = embedding_vec
                                    .iter()
                                    .filter_map(|v| v.as_f64().map(|f| f as f32))
                                    .collect();
                                embeddings.push(embedding);
                            } else {
                                return Err(anyhow!("Invalid embedding format in LM Studio response"));
                            }
                        } else {
                            return Err(anyhow!("No embedding field in LM Studio response item"));
                        }
                    }
                    
                    if embeddings.len() != texts.len() {
                        return Err(anyhow!("Embedding count mismatch: expected {}, got {}", texts.len(), embeddings.len()));
                    }
                    
                    info!("成功生成 {} 个嵌入向量", embeddings.len());
                    return Ok(embeddings);
                } else {
                    return Err(anyhow!("LM Studio response data is not an array"));
                }
            } else {
                return Err(anyhow!("No data field in LM Studio response"));
            }
        } else {
            error!("LM Studio请求失败 (状态: {}): {}", status, response_text);
            return Err(anyhow!("LM Studio embedding request failed ({}): {}", status, response_text));
        }
    }

    async fn get_embedding_dimension(&self) -> Result<usize> {
        Ok(self.dimensions)
    }

    fn provider_name(&self) -> &str {
        "LM Studio"
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

/// Ollama嵌入提供商
pub struct OllamaEmbeddingProvider {
    model_name: String,
    base_url: String,
    dimensions: usize,
}

impl OllamaEmbeddingProvider {
    pub fn new(config: &EmbeddingConfig) -> Result<Self> {
        Ok(Self {
            model_name: config.model.clone(),
            base_url: config.base_url.clone().unwrap_or_else(|| "http://localhost:11434".to_string()),
            dimensions: config.dimensions.unwrap_or(768),
        })
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaEmbeddingProvider {
    async fn embed_texts(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        info!("使用Ollama生成嵌入向量，模型: {}", self.model_name);
        
        let client = reqwest::Client::new();
        let mut embeddings = Vec::new();
        
        for text in texts {
            let payload = serde_json::json!({
                "model": self.model_name,
                "prompt": text
            });
            
            let response = client
                .post(&format!("{}/api/embeddings", self.base_url))
                .json(&payload)
                .send()
                .await?;
            
            if response.status().is_success() {
                let result: Value = response.json().await?;
                if let Some(embedding_array) = result.get("embedding") {
                    if let Some(embedding_vec) = embedding_array.as_array() {
                        let embedding: Vec<f32> = embedding_vec
                            .iter()
                            .filter_map(|v| v.as_f64().map(|f| f as f32))
                            .collect();
                        embeddings.push(embedding);
                    } else {
                        return Err(anyhow!("Invalid embedding format from Ollama"));
                    }
                } else {
                    return Err(anyhow!("No embedding in Ollama response"));
                }
            } else {
                let error_text = response.text().await.unwrap_or_default();
                return Err(anyhow!("Ollama embedding request failed: {}", error_text));
            }
        }
        
        info!("成功生成 {} 个嵌入向量", embeddings.len());
        Ok(embeddings)
    }

    async fn get_embedding_dimension(&self) -> Result<usize> {
        Ok(self.dimensions)
    }

    fn provider_name(&self) -> &str {
        "ollama"
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

/// LM Studio重排序提供商
pub struct LmStudioRerankingProvider {
    model_name: String,
    base_url: String,
    api_key: Option<String>,
}

impl LmStudioRerankingProvider {
    pub fn new(model: &str, base_url: Option<String>, api_key: Option<String>) -> Self {
        Self {
            model_name: model.to_string(),
            base_url: base_url.unwrap_or_else(|| "http://localhost:1234".to_string()),
            api_key,
        }
    }
}

#[async_trait]
impl RerankingProvider for LmStudioRerankingProvider {
    async fn rerank(&self, query: &str, documents: &[String], top_k: Option<usize>) -> Result<Vec<RerankResult>> {
        info!("使用LM Studio重排序，模型: {}，文档数量: {}", self.model_name, documents.len());
        
        let client = reqwest::Client::new();
        
        // 准备请求头
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse().unwrap());
        
        if let Some(api_key) = &self.api_key {
            headers.insert("Authorization", format!("Bearer {}", api_key).parse().unwrap());
        }
        
        // 构建重排序请求 - 使用chat completions格式来执行重排序任务
        let documents_text = documents.iter().enumerate()
            .map(|(i, doc)| format!("Document {}: {}", i, doc))
            .collect::<Vec<String>>()
            .join("\n\n");
        
        let system_prompt = format!(
            "You are a document reranking system. Your task is to rerank the provided documents based on their relevance to the given query. \
            Return ONLY a JSON array of objects with 'index' (0-based document index) and 'score' (relevance score from 0.0 to 1.0). \
            Sort by score in descending order. Return top {} documents.",
            top_k.unwrap_or(documents.len())
        );
        
        let user_prompt = format!(
            "Query: {}\n\nDocuments to rerank:\n{}\n\nReturn the reranked results as JSON array:",
            query, documents_text
        );
        
        let payload = serde_json::json!({
            "model": self.model_name,
            "messages": [
                { "role": "system", "content": system_prompt },
                { "role": "user", "content": user_prompt }
            ],
            "temperature": 0.1,
            "max_tokens": 2000,
            "stream": false
        });
        
        info!("发送LM Studio重排序请求，查询: {}", query);
        
        let response = client
            .post(&format!("{}/v1/chat/completions", self.base_url))
            .headers(headers)
            .json(&payload)
            .send()
            .await?;
        
        let status = response.status();
        let response_text = response.text().await?;
        
        if status.is_success() {
            info!("LM Studio重排序响应成功");
            
            let result: Value = serde_json::from_str(&response_text)?;
            
            // 解析chat completions响应
            if let Some(choices) = result.get("choices") {
                if let Some(choices_array) = choices.as_array() {
                    if let Some(first_choice) = choices_array.first() {
                        if let Some(message) = first_choice.get("message") {
                            if let Some(content) = message.get("content").and_then(|v| v.as_str()) {
                                info!("重排序模型响应: {}", content);
                                
                                // 尝试解析模型返回的JSON
                                if let Ok(rerank_data) = serde_json::from_str::<Value>(content) {
                                    if let Some(results_array) = rerank_data.as_array() {
                                        let mut rerank_results = Vec::new();
                                        
                                        for item in results_array {
                                            if let (Some(index), Some(score)) = (
                                                item.get("index").and_then(|v| v.as_u64()),
                                                item.get("score").and_then(|v| v.as_f64())
                                            ) {
                                                if (index as usize) < documents.len() {
                                                    rerank_results.push(RerankResult {
                                                        index: index as usize,
                                                        score: score as f32,
                                                        document: documents[index as usize].clone(),
                                                    });
                                                }
                                            }
                                        }
                                        
                                        if !rerank_results.is_empty() {
                                            info!("重排序完成，返回 {} 个结果", rerank_results.len());
                                            return Ok(rerank_results);
                                        }
                                    }
                                }
                                
                                // 如果JSON解析失败，尝试简单的文本解析
                                warn!("无法解析重排序JSON响应，使用简单解析");
                            }
                        }
                    }
                }
            }
            
            // 如果API格式不符合预期，回退到简单的分数分配
            warn!("LM Studio重排序响应格式不符合预期，使用简单排序");
            let mut simple_results = Vec::new();
            for (i, doc) in documents.iter().enumerate() {
                simple_results.push(RerankResult {
                    index: i,
                    score: 1.0 - (i as f32 / documents.len() as f32), // 简单的递减分数
                    document: doc.clone(),
                });
            }
            Ok(simple_results)
        } else {
            error!("LM Studio重排序请求失败 (状态: {}): {}", status, response_text);
            
            // 重排序失败时，回退到原始顺序
            warn!("重排序失败，保持原始顺序");
            let mut fallback_results = Vec::new();
            for (i, doc) in documents.iter().enumerate() {
                fallback_results.push(RerankResult {
                    index: i,
                    score: 0.5, // 中等分数
                    document: doc.clone(),
                });
            }
            Ok(fallback_results)
        }
    }
    
    fn provider_name(&self) -> &str {
        "LM Studio"
    }
    
    fn model_name(&self) -> &str {
        &self.model_name
    }
}

/// 重排序管理器
pub struct RerankingManager {
    provider: Option<Box<dyn RerankingProvider>>,
}

impl RerankingManager {
    pub fn new() -> Self {
        Self {
            provider: None,
        }
    }
    
    pub fn register_provider(&mut self, provider: Box<dyn RerankingProvider>) {
        info!("注册重排序提供商: {}", provider.provider_name());
        self.provider = Some(provider);
    }
    
    pub async fn rerank(&self, query: &str, documents: &[String], top_k: Option<usize>) -> Result<Vec<RerankResult>> {
        if let Some(provider) = &self.provider {
            provider.rerank(query, documents, top_k).await
        } else {
            // 没有重排序提供商时，返回原始顺序
            info!("没有重排序提供商，保持原始顺序");
            let mut results = Vec::new();
            for (i, doc) in documents.iter().enumerate() {
                results.push(RerankResult {
                    index: i,
                    score: 1.0 - (i as f32 / documents.len() as f32),
                    document: doc.clone(),
                });
            }
            Ok(results)
        }
    }
    
    pub fn is_enabled(&self) -> bool {
        self.provider.is_some()
    }
}

/// 创建重排序提供商
pub fn create_reranking_provider(
    provider: &str,
    model: &str,
    base_url: Option<String>,
    api_key: Option<String>,
) -> Result<Box<dyn RerankingProvider>> {
    match provider.to_lowercase().as_str() {
        "lm studio" | "lmstudio" | "lm_studio" => {
            info!("创建LM Studio重排序提供商: {}", model);
            Ok(Box::new(LmStudioRerankingProvider::new(model, base_url, api_key)))
        }
        _ => {
            Err(anyhow!("不支持的重排序提供商: {}", provider))
        }
    }
}

/// 创建嵌入提供商
pub fn create_embedding_provider(config: &EmbeddingConfig) -> Result<Box<dyn EmbeddingProvider>> {
    match config.provider.to_lowercase().as_str() {
        "lm studio" | "lmstudio" | "lm_studio" => {
            info!("创建LM Studio嵌入提供商: {}", config.model);
            Ok(Box::new(LmStudioEmbeddingProvider::new(config)?))
        }
        "ollama" => {
            info!("创建Ollama嵌入提供商: {}", config.model);
            Ok(Box::new(OllamaEmbeddingProvider::new(config)?))
        }
        _ => {
            warn!("未知的嵌入提供商: {}，使用基础提供商", config.provider);
            Ok(Box::new(BasicEmbeddingProvider::new(config)?))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_embedding_manager() {
        let mut manager = EmbeddingManager::new();
        
        let config = EmbeddingConfig::default();
        let provider = create_embedding_provider(&config).unwrap();
        manager.register_provider(provider);
        
        let texts = vec!["Hello world".to_string(), "Test text".to_string()];
        let embeddings = manager.embed_texts(&texts).await.unwrap();
        
        assert_eq!(embeddings.len(), 2);
        assert_eq!(embeddings[0].len(), 768); // 默认维度
    }
}