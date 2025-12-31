use anyhow::{anyhow, Result};
use async_trait::async_trait;
use log::{debug, error, info, warn};
use serde_json::Value;
use std::collections::HashMap;

use crate::config::EmbeddingConfig;

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed_texts(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
    async fn get_embedding_dimension(&self) -> Result<usize>;
    fn provider_name(&self) -> &str;
    fn model_name(&self) -> &str;
}

#[async_trait]
pub trait RerankingProvider: Send + Sync {
    async fn rerank(
        &self,
        query: &str,
        documents: &[String],
        top_k: Option<usize>,
    ) -> Result<Vec<RerankResult>>;
    fn provider_name(&self) -> &str;
    fn model_name(&self) -> &str;
}

#[derive(Debug, Clone)]
pub struct RerankResult {
    pub index: usize,
    pub score: f32,
    pub document: String,
}

pub struct EmbeddingManager {
    providers: HashMap<String, Box<dyn EmbeddingProvider>>,
    current_provider: Option<String>,
}
impl Default for EmbeddingManager {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddingManager {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            current_provider: None,
        }
    }
    pub fn register_provider(&mut self, provider: Box<dyn EmbeddingProvider>) {
        let key = format!("{}:{}", provider.provider_name(), provider.model_name());
        info!("注册嵌入提供商: {}", key);
        self.current_provider = Some(key.clone());
        self.providers.insert(key, provider);
    }
    pub fn set_current_provider(&mut self, provider_name: &str, model_name: &str) -> Result<()> {
        let key = format!("{}:{}", provider_name, model_name);
        if self.providers.contains_key(&key) {
            self.current_provider = Some(key);
            Ok(())
        } else {
            Err(anyhow!("提供商 {} 未找到", key))
        }
    }
    fn get_current_provider(&self) -> Result<&dyn EmbeddingProvider> {
        match &self.current_provider {
            Some(k) => self
                .providers
                .get(k)
                .map(|p| p.as_ref())
                .ok_or_else(|| anyhow!("当前提供商 {} 未找到", k)),
            None => Err(anyhow!("未设置当前提供商")),
        }
    }
    pub async fn embed_texts(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let p = self.get_current_provider()?;
        debug!(
            "使用提供商 {} 生成 {} 个文本的嵌入向量",
            p.provider_name(),
            texts.len()
        );
        let embeddings = p.embed_texts(texts).await?;
        if embeddings.len() != texts.len() {
            return Err(anyhow!("嵌入向量数量与文本数量不匹配"));
        }
        Ok(embeddings)
    }
    pub async fn get_embedding_dimension(&self) -> Result<usize> {
        self.get_current_provider()?.get_embedding_dimension().await
    }
    pub fn list_providers(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }
}

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
        warn!("使用基础嵌入提供商，返回随机向量");
        Ok(texts
            .iter()
            .map(|_| {
                (0..self.dimensions)
                    .map(|_| rand::random::<f32>() - 0.5)
                    .collect()
            })
            .collect())
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
            base_url: config
                .base_url
                .clone()
                .unwrap_or_else(|| "http://localhost:1234".to_string()),
            api_key: config.api_key.clone(),
            dimensions: config.dimensions.unwrap_or(768),
        })
    }
}
#[async_trait]
impl EmbeddingProvider for LmStudioEmbeddingProvider {
    async fn embed_texts(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        info!(
            "使用LM Studio生成嵌入向量，模型: {}，文本数量: {}",
            self.model_name,
            texts.len()
        );
        // Apply global proxy configuration
        let builder = reqwest::Client::builder();
        let builder = sentinel_core::global_proxy::apply_proxy_to_client(builder).await;
        let client = builder.build().unwrap_or_else(|_| reqwest::Client::new());
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse().unwrap());
        if let Some(api_key) = &self.api_key {
            headers.insert(
                "Authorization",
                format!("Bearer {}", api_key).parse().unwrap(),
            );
        }
        let input = if texts.len() == 1 {
            Value::String(texts[0].clone())
        } else {
            Value::Array(texts.iter().map(|t| Value::String(t.clone())).collect())
        };
        let payload = serde_json::json!({ "model": self.model_name, "input": input });
        let response = client
            .post(format!("{}/v1/embeddings", self.base_url))
            .headers(headers)
            .json(&payload)
            .send()
            .await?;
        let status = response.status();
        let response_text = response.text().await?;
        if status.is_success() {
            let result: Value = serde_json::from_str(&response_text)?;
            if let Some(data) = result.get("data").and_then(|d| d.as_array()) {
                let mut out = Vec::new();
                for item in data {
                    if let Some(ev) = item.get("embedding").and_then(|v| v.as_array()) {
                        let embedding: Vec<f32> = ev
                            .iter()
                            .filter_map(|v| v.as_f64().map(|f| f as f32))
                            .collect();
                        out.push(embedding);
                    } else {
                        return Err(anyhow!("Invalid embedding format in LM Studio response"));
                    }
                }
                if out.len() != texts.len() {
                    return Err(anyhow!(
                        "Embedding count mismatch: expected {}, got {}",
                        texts.len(),
                        out.len()
                    ));
                }
                Ok(out)
            } else {
                Err(anyhow!("LM Studio response data invalid"))?
            }
        } else {
            error!("LM Studio请求失败 (状态: {}): {}", status, response_text);
            Err(anyhow!(
                "LM Studio embedding request failed ({}): {}",
                status,
                response_text
            ))
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

pub struct OllamaEmbeddingProvider {
    model_name: String,
    base_url: String,
    dimensions: usize,
}
impl OllamaEmbeddingProvider {
    pub fn new(config: &EmbeddingConfig) -> Result<Self> {
        Ok(Self {
            model_name: config.model.clone(),
            base_url: config
                .base_url
                .clone()
                .unwrap_or_else(|| "http://localhost:11434".to_string()),
            dimensions: config.dimensions.unwrap_or(768),
        })
    }
}
#[async_trait]
impl EmbeddingProvider for OllamaEmbeddingProvider {
    async fn embed_texts(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        info!("使用Ollama生成嵌入向量，模型: {}", self.model_name);
        // Apply global proxy configuration
        let builder = reqwest::Client::builder();
        let builder = sentinel_core::global_proxy::apply_proxy_to_client(builder).await;
        let client = builder.build().unwrap_or_else(|_| reqwest::Client::new());
        let mut embeddings = Vec::new();
        for text in texts {
            let payload = serde_json::json!({ "model": self.model_name, "prompt": text });
            let response = client
                .post(format!("{}/api/embeddings", self.base_url))
                .json(&payload)
                .send()
                .await?;
            if response.status().is_success() {
                let result: Value = response.json().await?;
                if let Some(embedding_array) = result.get("embedding") {
                    if let Some(vec_array) = embedding_array.as_array() {
                        let embedding: Vec<f32> = vec_array
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
    async fn rerank(
        &self,
        query: &str,
        documents: &[String],
        top_k: Option<usize>,
    ) -> Result<Vec<RerankResult>> {
        info!(
            "使用LM Studio重排序，模型: {}，文档数量: {}",
            self.model_name,
            documents.len()
        );
        // Apply global proxy configuration
        let builder = reqwest::Client::builder();
        let builder = sentinel_core::global_proxy::apply_proxy_to_client(builder).await;
        let client = builder.build().unwrap_or_else(|_| reqwest::Client::new());
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse().unwrap());
        if let Some(api_key) = &self.api_key {
            headers.insert(
                "Authorization",
                format!("Bearer {}", api_key).parse().unwrap(),
            );
        }
        let documents_text = documents
            .iter()
            .enumerate()
            .map(|(i, d)| format!("Document {}: {}", i, d))
            .collect::<Vec<String>>()
            .join("\n\n");
        let system_prompt = format!(
            "You are a document reranking system... Return top {} documents.",
            top_k.unwrap_or(documents.len())
        );
        let user_prompt = format!(
            "Query: {}\n\nDocuments to rerank:\n{}\n\nReturn the reranked results as JSON array:",
            query, documents_text
        );
        let payload = serde_json::json!({ "model": self.model_name, "messages": [ {"role":"system","content":system_prompt}, {"role":"user","content":user_prompt} ], "temperature": 0.1, "max_tokens": 2000, "stream": false });
        let response = client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .headers(headers)
            .json(&payload)
            .send()
            .await?;
        let status = response.status();
        let response_text = response.text().await?;
        if status.is_success() {
            let result: Value = serde_json::from_str(&response_text)?;
            if let Some(choices) = result.get("choices").and_then(|c| c.as_array()) {
                if let Some(first) = choices.first() {
                    if let Some(content) = first
                        .get("message")
                        .and_then(|m| m.get("content"))
                        .and_then(|v| v.as_str())
                    {
                        if let Ok(arr) = serde_json::from_str::<Value>(content) {
                            if let Some(items) = arr.as_array() {
                                let mut out = Vec::new();
                                for item in items {
                                    if let (Some(index), Some(score)) = (
                                        item.get("index").and_then(|v| v.as_u64()),
                                        item.get("score").and_then(|v| v.as_f64()),
                                    ) {
                                        if (index as usize) < documents.len() {
                                            out.push(RerankResult {
                                                index: index as usize,
                                                score: score as f32,
                                                document: documents[index as usize].clone(),
                                            });
                                        }
                                    }
                                }
                                if !out.is_empty() {
                                    return Ok(out);
                                }
                            }
                        }
                    }
                }
            }
            // if success response not parseable, fall back below
        }
        // Fallback for non-success or unparsable payloads
        warn!("LM Studio重排序失败或响应格式异常，使用简单回退");
        Ok(documents
            .iter()
            .enumerate()
            .map(|(i, d)| RerankResult { index: i, score: 0.5, document: d.clone() })
            .collect())
    }

    fn provider_name(&self) -> &str { "LM Studio" }
    fn model_name(&self) -> &str { &self.model_name }
}

pub struct RerankingManager {
    provider: Option<Box<dyn RerankingProvider>>,
}
impl Default for RerankingManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RerankingManager {
    pub fn new() -> Self {
        Self { provider: None }
    }
    pub fn register_provider(&mut self, provider: Box<dyn RerankingProvider>) {
        info!("注册重排序提供商: {}", provider.provider_name());
        self.provider = Some(provider);
    }
    pub async fn rerank(
        &self,
        query: &str,
        documents: &[String],
        top_k: Option<usize>,
    ) -> Result<Vec<RerankResult>> {
        if let Some(p) = &self.provider {
            p.rerank(query, documents, top_k).await
        } else {
            info!("没有重排序提供商，保持原始顺序");
            Ok(documents
                .iter()
                .enumerate()
                .map(|(i, d)| RerankResult {
                    index: i,
                    score: 1.0 - (i as f32 / documents.len() as f32),
                    document: d.clone(),
                })
                .collect())
        }
    }
    pub fn is_enabled(&self) -> bool {
        self.provider.is_some()
    }

}

    pub fn create_reranking_provider(
        provider: &str,
        model: &str,
        base_url: Option<String>,
        api_key: Option<String>,
    ) -> Result<Box<dyn RerankingProvider>> {
        match provider.to_lowercase().as_str() {
            "lm studio" | "lmstudio" | "lm_studio" => Ok(Box::new(LmStudioRerankingProvider::new(
                model, base_url, api_key,
            ))),
            _ => Err(anyhow!("不支持的重排序提供商: {}", provider)),
        }
    }
    pub fn create_embedding_provider(
        config: &EmbeddingConfig,
    ) -> Result<Box<dyn EmbeddingProvider>> {
        match config.provider.to_lowercase().as_str() {
            "lm studio" | "lmstudio" | "lm_studio" => {
                Ok(Box::new(LmStudioEmbeddingProvider::new(config)?))
            }
            "ollama" => Ok(Box::new(OllamaEmbeddingProvider::new(config)?)),
            "rig" | "rig-ollama" | "rig_ollama" | "rig/ollama" => {
                Ok(Box::new(OllamaEmbeddingProvider::new(config)?))
            }
            _ => {
                warn!("未知的嵌入提供商: {}，使用基础提供商", config.provider);
                Ok(Box::new(BasicEmbeddingProvider::new(config)?))
            }
        }
    }

