//! 智能记忆模块实现
//! 
//! 提供执行经验存储、检索和学习功能

use std::collections::HashMap;
use anyhow::{Result, anyhow};
use serde_json;
use uuid::Uuid;
use chrono::Utc;
use crate::engines::memory::*;
use crate::engines::types::*;

/// 记忆配置
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    pub max_experiences: usize,
    pub max_templates: usize,
    pub similarity_threshold: f64,
    pub cleanup_interval_hours: f64,
    pub vector_dimensions: usize,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_experiences: 10000,
            max_templates: 1000,
            similarity_threshold: 0.7,
            cleanup_interval_hours: 24,
            vector_dimensions: 384,
        }
    }
}

/// 智能记忆模块实现
pub struct IntelligentMemory {
    config: MemoryConfig,
    experiences: HashMap<String, ExecutionExperience>,
    templates: HashMap<String, PlanTemplate>,
    entities: HashMap<String, KnowledgeEntity>,
    relationships: HashMap<String, KnowledgeRelationship>,
    feedback_history: Vec<LearningFeedback>,
    embeddings: HashMap<String, VectorEmbedding>,
    query_history: Vec<MemoryQueryHistory>,
}

impl IntelligentMemory {
    pub fn new() -> Self {
        Self::with_config(MemoryConfig::default())
    }

    pub fn with_config(config: MemoryConfig) -> Self {
        Self {
            config,
            experiences: HashMap::new(),
            templates: HashMap::new(),
            entities: HashMap::new(),
            relationships: HashMap::new(),
            feedback_history: Vec::new(),
            embeddings: HashMap::new(),
            query_history: Vec::new(),
        }
    }

    /// 计算文本相似度
    fn calculate_text_similarity(&self, text1: &str, text2: &str) -> f64 {
        let words1: std::collections::HashSet<&str> = text1.split_whitespace().collect();
        let words2: std::collections::HashSet<&str> = text2.split_whitespace().collect();
        
        let intersection = words1.intersection(&words2).count();
        let union = words1.union(&words2).count();
        
        if union == 0 {
            0.0
        } else {
            intersection as f64 / union as f64
        }
    }

    /// 生成简单的文本嵌入向量
    fn generate_text_embedding(&self, text: &str) -> Vec<f32> {
        // 简化的嵌入生成，实际应该使用预训练模型
        let mut embedding = vec![0.0; self.config.vector_dimensions];
        let words: Vec<&str> = text.split_whitespace().collect();
        
        for (i, word) in words.iter().enumerate() {
            let hash = word.chars().map(|c| c as u32).sum::<u32>() as usize;
            let index = hash % self.config.vector_dimensions;
            embedding[index] += 1.0 / (words.len() as f32);
        }
        
        embedding
    }

    /// 计算向量相似度
    fn calculate_vector_similarity(&self, vec1: &[f32], vec2: &[f32]) -> f64 {
        if vec1.len() != vec2.len() {
            return 0.0;
        }
        
        let dot_product: f32 = vec1.iter().zip(vec2.iter()).map(|(a, b)| a * b).sum();
        let norm1: f32 = vec1.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm2: f32 = vec2.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if norm1 == 0.0 || norm2 == 0.0 {
            0.0
        } else {
            (dot_product / (norm1 * norm2)) as f64
        }
    }
}

impl Memory for IntelligentMemory {
    fn store_experience(&mut self, experience: ExecutionExperience) -> Result<()> {
        // 生成嵌入向量
        let text_content = format!("{} {} {}", 
            experience.task_type, 
            experience.target_description, 
            experience.environment_context
        );
        let embedding_vector = self.generate_text_embedding(&text_content);
        
        // 存储嵌入
        let embedding = VectorEmbedding {
            id: Uuid::new_v4().to_string(),
            content_type: "experience".to_string(),
            content_id: experience.id.clone(),
            embedding: embedding_vector,
            dimensions: self.config.vector_dimensions as i32,
            model_name: "simple_text_embedding".to_string(),
            created_at: Utc::now().timestamp(),
            updated_at: Utc::now().timestamp(),
        };
        
        self.embeddings.insert(embedding.id.clone(), embedding);
        self.experiences.insert(experience.id.clone(), experience);
        
        Ok(())
    }

    fn retrieve_similar_experiences(
        &self,
        query: &MemoryQuery,
    ) -> Result<Vec<SimilaritySearchResult<ExecutionExperience>>> {
        let mut results = Vec::new();
        
        // 生成查询向量
        let query_text = format!(
            "{} {} {}",
            query.task_type.as_deref().unwrap_or(""),
            query.target_description.as_deref().unwrap_or(""),
            query.environment_context.as_deref().unwrap_or("")
        );
        let query_vector = self.generate_text_embedding(&query_text);
        
        for experience in self.experiences.values() {
            let mut similarity_score = 0.0;
            let mut relevance_factors = Vec::new();
            
            // 文本相似度
            if let Some(task_type) = &query.task_type {
                let task_sim = self.calculate_text_similarity(task_type, &experience.task_type);
                similarity_score += task_sim * 0.4;
                if task_sim > 0.5 {
                    relevance_factors.push("task_type_match".to_string());
                }
            }
            
            // 向量相似度
            if let Some(embedding) = self.embeddings.values()
                .find(|e| e.content_id == experience.id) {
                let vector_sim = self.calculate_vector_similarity(&query_vector, &embedding.embedding);
                similarity_score += vector_sim * 0.6;
                if vector_sim > 0.7 {
                    relevance_factors.push("high_vector_similarity".to_string());
                }
            }
            
            if similarity_score >= query.similarity_threshold {
                results.push(SimilaritySearchResult {
                    item: experience.clone(),
                    similarity_score,
                    relevance_factors,
                });
            }
        }
        
        // 按相似度排序
        results.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap());
        results.truncate(query.max_results);
        
        Ok(results)
    }

    fn store_template(&mut self, template: PlanTemplate) -> Result<()> {
        // 生成嵌入向量
        let text_content = format!("{} {} {}", 
            template.name, 
            template.description.as_deref().unwrap_or(""),
            template.task_type
        );
        let embedding_vector = self.generate_text_embedding(&text_content);
        
        // 存储嵌入
        let embedding = VectorEmbedding {
            id: Uuid::new_v4().to_string(),
            content_type: "template".to_string(),
            content_id: template.id.clone(),
            embedding: embedding_vector,
            dimensions: self.config.vector_dimensions as i32,
            model_name: "simple_text_embedding".to_string(),
            created_at: Utc::now().timestamp(),
            updated_at: Utc::now().timestamp(),
        };
        
        self.embeddings.insert(embedding.id.clone(), embedding);
        self.templates.insert(template.id.clone(), template);
        
        Ok(())
    }

    fn retrieve_applicable_templates(
        &self,
        task_type: &str,
        environment: &str,
        target_properties: &serde_json::Value,
    ) -> Result<Vec<SimilaritySearchResult<PlanTemplate>>> {
        let mut results = Vec::new();
        
        for template in self.templates.values() {
            let mut similarity_score = 0.0;
            let mut relevance_factors = Vec::new();
            
            // 任务类型匹配
            let task_sim = self.calculate_text_similarity(task_type, &template.task_type);
            similarity_score += task_sim * 0.6;
            
            if task_sim > 0.8 {
                relevance_factors.push("exact_task_match".to_string());
            }
            
            // 成功率权重
            similarity_score += template.success_rate * 0.3;
            
            // 使用频率权重
            let usage_weight = (template.usage_count as f64).ln().max(0.0) / 10.0;
            similarity_score += usage_weight * 0.1;
            
            if similarity_score >= self.config.similarity_threshold {
                results.push(SimilaritySearchResult {
                    item: template.clone(),
                    similarity_score,
                    relevance_factors,
                });
            }
        }
        
        results.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap());
        
        Ok(results)
    }

    fn update_knowledge_graph(
        &mut self,
        entities: Vec<KnowledgeEntity>,
        relationships: Vec<KnowledgeRelationship>,
    ) -> Result<()> {
        for entity in entities {
            self.entities.insert(entity.id.clone(), entity);
        }
        
        for relationship in relationships {
            self.relationships.insert(relationship.id.clone(), relationship);
        }
        
        Ok(())
    }

    fn query_knowledge_graph(
        &self,
        entity_name: &str,
        relationship_types: &[RelationshipType],
        max_depth: usize,
    ) -> Result<Vec<KnowledgeEntity>> {
        let mut results = Vec::new();
        let mut visited = std::collections::HashSet::new();
        
        // 找到起始实体
        if let Some(start_entity) = self.entities.values()
            .find(|e| e.name == entity_name) {
            
            self.traverse_knowledge_graph(
                &start_entity.id,
                relationship_types,
                max_depth,
                &mut visited,
                &mut results,
            );
        }
        
        Ok(results)
    }

    fn process_learning_feedback(&mut self, feedback: LearningFeedback) -> Result<()> {
        self.feedback_history.push(feedback);
        Ok(())
    }

    fn learn_from_execution(&mut self, update: LearningUpdate) -> Result<()> {
        // 简化的学习实现
        let feedback = LearningFeedback {
            id: Uuid::new_v4().to_string(),
            experience_id: None,
            template_id: None,
            entity_id: None,
            relationship_id: None,
            feedback_type: if update.execution_result.status == crate::engines::types::StepExecutionStatus::Completed {
                crate::engines::memory::FeedbackType::Success
            } else {
                crate::engines::memory::FeedbackType::Failure
            },
            feedback_content: serde_json::json!({
                "session_id": update.session_id,
                "metrics": update.performance_metrics,
                "context": update.context_info
            }),
            improvements: None,
            confidence_adjustments: None,
            user_rating: None,
            automated_score: Some(if update.execution_result.status == crate::engines::types::StepExecutionStatus::Completed { 1.0 } else { 0.0 }),
            created_at: Utc::now().timestamp(),
            processed_at: Some(Utc::now().timestamp()),
        };
        
        self.process_learning_feedback(feedback)
    }

    fn get_tool_effectiveness(
        &self,
        tool_name: &str,
        target_type: Option<&str>,
        environment: Option<&str>,
    ) -> Result<f64> {
        let mut total_uses = 0;
        let mut successful_uses = 0;
        
        for experience in self.experiences.values() {
            for step in &experience.successful_steps {
                if let Some(step_tool) = step.get("tool_name").and_then(|v| v.as_str()) {
                    if step_tool == tool_name {
                        total_uses += 1;
                        successful_uses += 1;
                    }
                }
            }
        }
        
        if total_uses == 0 {
            Ok(0.0)
        } else {
            Ok(successful_uses as f64 / total_uses as f64)
        }
    }

    fn get_environment_specific_recommendations(
        &self,
        environment: &str,
        task_type: &str,
    ) -> Result<Vec<String>> {
        let mut recommendations = Vec::new();
        
        // 基于历史经验生成建议
        for experience in self.experiences.values() {
            if self.calculate_text_similarity(&experience.environment_context, environment) > 0.7
                && self.calculate_text_similarity(&experience.task_type, task_type) > 0.7 {
                
                if experience.confidence_score > 0.8 {
                    recommendations.push(format!(
                        "Based on successful execution: Use tools from successful steps"
                    ));
                }
            }
        }
        
        if recommendations.is_empty() {
            recommendations.push("No specific recommendations available for this environment".to_string());
        }
        
        Ok(recommendations)
    }
}

impl IntelligentMemory {
    fn traverse_knowledge_graph(
        &self,
        entity_id: &str,
        relationship_types: &[RelationshipType],
        max_depth: usize,
        visited: &mut std::collections::HashSet<String>,
        results: &mut Vec<KnowledgeEntity>,
    ) {
        if max_depth == 0 || visited.contains(entity_id) {
            return;
        }
        
        visited.insert(entity_id.to_string());
        
        if let Some(entity) = self.entities.get(entity_id) {
            results.push(entity.clone());
        }
        
        // 查找相关关系
        for relationship in self.relationships.values() {
            if relationship.from_entity == entity_id 
                && relationship_types.contains(&relationship.relationship_type) {
                
                self.traverse_knowledge_graph(
                    &relationship.to_entity,
                    relationship_types,
                    max_depth - 1,
                    visited,
                    results,
                );
            }
        }
    }
}