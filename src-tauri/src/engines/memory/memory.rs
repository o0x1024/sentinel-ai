//! Memory（记忆模块）模块
//! 
//! 负责存储和检索执行经验、计划模板、知识图谱等信息
//! 支持基于相似度的智能检索和学习反馈机制

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::engines::types::*;
use anyhow::Result;

/// 知识实体类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EntityType {
    Tool,
    Target,
    Environment,
    Technique,
    Vulnerability,
    Asset,
}

/// 知识关系类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RelationshipType {
    /// 对...有效
    EffectiveAgainst,
    /// 前置条件
    Precedes,
    /// 需要
    Requires,
    /// 冲突
    Conflicts,
    /// 增强
    Enhances,
    /// 替代
    Substitutes,
}

/// 反馈类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FeedbackType {
    Success,
    Failure,
    Improvement,
    UserCorrection,
}

/// 查询类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QueryType {
    SimilarFailures,
    SuccessfulPatterns,
    ToolEffectiveness,
    EnvironmentSpecific,
}

/// 执行经验
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionExperience {
    pub id: String,
    pub task_type: String,
    pub target_description: String,
    pub target_hash: String,
    pub target_properties: Option<serde_json::Value>,
    pub environment_context: String,
    pub environment_hash: String,
    pub environment_properties: Option<serde_json::Value>,
    pub successful_steps: Vec<serde_json::Value>,
    pub failure_info: Option<serde_json::Value>,
    pub performance_metrics: Option<serde_json::Value>,
    pub confidence_score: f64,
    pub usage_count: i32,
    pub last_used_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 计划模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanTemplate {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub domain: String,
    pub task_type: String,
    pub template_steps: Vec<serde_json::Value>,
    pub success_rate: f64,
    pub usage_count: i32,
    pub effectiveness_score: f64,
    pub applicability_conditions: Option<serde_json::Value>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_used_at: Option<i64>,
}

/// 知识实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntity {
    pub id: String,
    pub entity_type: EntityType,
    pub name: String,
    pub properties: Option<serde_json::Value>,
    pub confidence: f64,
    pub usage_count: i32,
    pub effectiveness_score: f64,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 知识关系
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeRelationship {
    pub id: String,
    pub from_entity: String,
    pub to_entity: String,
    pub relationship_type: RelationshipType,
    pub strength: f64,
    pub context: Option<serde_json::Value>,
    pub confidence: f64,
    pub usage_count: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 学习反馈
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningFeedback {
    pub id: String,
    pub experience_id: Option<String>,
    pub template_id: Option<String>,
    pub entity_id: Option<String>,
    pub relationship_id: Option<String>,
    pub feedback_type: FeedbackType,
    pub feedback_content: serde_json::Value,
    pub improvements: Option<serde_json::Value>,
    pub confidence_adjustments: Option<serde_json::Value>,
    pub user_rating: Option<f64>,
    pub automated_score: Option<f64>,
    pub created_at: i64,
    pub processed_at: Option<i64>,
}

/// 向量嵌入
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorEmbedding {
    pub id: String,
    pub content_type: String,
    pub content_id: String,
    pub embedding: Vec<f32>,
    pub dimensions: i32,
    pub model_name: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 记忆查询历史
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQueryHistory {
    pub id: String,
    pub query_type: QueryType,
    pub query_content: serde_json::Value,
    pub results_count: i32,
    pub execution_time_ms: i32,
    pub similarity_threshold: Option<f64>,
    pub created_at: i64,
}

/// 相似度搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilaritySearchResult<T> {
    pub item: T,
    pub similarity_score: f64,
    pub relevance_factors: Vec<String>,
}

/// 记忆检索查询
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQuery {
    pub query_type: QueryType,
    pub target_description: Option<String>,
    pub environment_context: Option<String>,
    pub task_type: Option<String>,
    pub tool_names: Option<Vec<String>>,
    pub error_patterns: Option<Vec<String>>,
    pub similarity_threshold: f64,
    pub max_results: usize,
    pub include_metadata: bool,
}

/// 学习更新请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningUpdate {
    pub session_id: String,
    pub execution_result: StepExecutionResult,
    pub user_feedback: Option<serde_json::Value>,
    pub performance_metrics: ExecutionMetrics,
    pub context_info: serde_json::Value,
}

/// 记忆模块接口
pub trait Memory: Send + Sync {
    /// 存储执行经验
    fn store_experience(&mut self, experience: ExecutionExperience) -> Result<()>;

    /// 检索相似经验
    fn retrieve_similar_experiences(
        &self,
        query: &MemoryQuery,
    ) -> Result<Vec<SimilaritySearchResult<ExecutionExperience>>>;

    /// 存储计划模板
    fn store_template(&mut self, template: PlanTemplate) -> Result<()>;

    /// 检索适用模板
    fn retrieve_applicable_templates(
        &self,
        task_type: &str,
        environment: &str,
        target_properties: &serde_json::Value,
    ) -> Result<Vec<SimilaritySearchResult<PlanTemplate>>>;

    /// 更新知识图谱
    fn update_knowledge_graph(
        &mut self,
        entities: Vec<KnowledgeEntity>,
        relationships: Vec<KnowledgeRelationship>,
    ) -> Result<()>;

    /// 查询知识图谱
    fn query_knowledge_graph(
        &self,
        entity_name: &str,
        relationship_types: &[RelationshipType],
        max_depth: usize,
    ) -> Result<Vec<KnowledgeEntity>>;

    /// 学习反馈处理
    fn process_learning_feedback(&mut self, feedback: LearningFeedback) -> Result<()>;

    /// 从执行结果学习
    fn learn_from_execution(&mut self, update: LearningUpdate) -> Result<()>;

    /// 获取工具效果统计
    fn get_tool_effectiveness(
        &self,
        tool_name: &str,
        target_type: Option<&str>,
        environment: Option<&str>,
    ) -> Result<f64>;

    /// 获取环境特定建议
    fn get_environment_specific_recommendations(
        &self,
        environment: &str,
        task_type: &str,
    ) -> Result<Vec<String>>;
    
    // === 架构特定的记忆增强接口 ===
    
    /// Plan-and-Execute: 检索相似的失败轨迹（用于避免重复错误）
    fn retrieve_failure_trajectories(
        &self,
        task_description: &str,
        error_pattern: &str,
    ) -> Result<Vec<SimilaritySearchResult<ExecutionExperience>>>;
    
    /// Plan-and-Execute: 存储执行轨迹（包含完整的步骤序列）
    fn store_execution_trajectory(
        &mut self,
        task_description: String,
        steps: Vec<serde_json::Value>,
        success: bool,
        error_info: Option<serde_json::Value>,
    ) -> Result<()>;
    
    /// ReWOO: 检索Few-Shot规划示例（用于规划阶段）
    fn retrieve_few_shot_plans(
        &self,
        task_description: &str,
        max_results: usize,
    ) -> Result<Vec<SimilaritySearchResult<PlanTemplate>>>;
    
    /// ReWOO: 存储成功的规划蓝图
    fn store_rewoo_plan_blueprint(
        &mut self,
        task_description: String,
        plan_steps: Vec<serde_json::Value>,
        success_rate: f64,
    ) -> Result<()>;
    
    /// LLM Compiler: 检查工具调用缓存
    fn check_tool_call_cache(
        &self,
        tool_name: &str,
        tool_args: &serde_json::Value,
    ) -> Result<Option<serde_json::Value>>;
    
    /// LLM Compiler: 存储工具调用结果到缓存
    fn cache_tool_call_result(
        &mut self,
        tool_name: String,
        tool_args: serde_json::Value,
        result: serde_json::Value,
        execution_time_ms: u64,
    ) -> Result<()>;
    
    /// ReAct: 检索相似的推理链（用于提示工程）
    fn retrieve_reasoning_chains(
        &self,
        task_description: &str,
        max_results: usize,
    ) -> Result<Vec<SimilaritySearchResult<ExecutionExperience>>>;
}

/// 智能记忆模块实现
pub struct IntelligentMemory {
    /// 执行经验存储
    experiences: HashMap<String, ExecutionExperience>,
    /// 计划模板存储
    templates: HashMap<String, PlanTemplate>,
    /// 知识实体存储
    entities: HashMap<String, KnowledgeEntity>,
    /// 知识关系存储
    relationships: HashMap<String, KnowledgeRelationship>,
    /// 学习反馈存储
    feedback_history: Vec<LearningFeedback>,
    /// 向量嵌入存储
    #[allow(unused)]
    embeddings: HashMap<String, VectorEmbedding>,
    /// 查询历史
    #[allow(unused)]
    query_history: Vec<MemoryQueryHistory>,
}

impl IntelligentMemory {
    pub fn new() -> Self {
        Self {
            experiences: HashMap::new(),
            templates: HashMap::new(),
            entities: HashMap::new(),
            relationships: HashMap::new(),
            feedback_history: Vec::new(),
            embeddings: HashMap::new(),
            query_history: Vec::new(),
        }
    }

    /// 计算文本相似度（简化实现）
    fn calculate_text_similarity(&self, text1: &str, text2: &str) -> f64 {
        // 简化的相似度计算，实际应该使用更复杂的算法
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

    /// 计算环境相似度
    fn calculate_environment_similarity(
        &self,
        env1: &str,
        env2: &str,
        props1: &Option<serde_json::Value>,
        props2: &Option<serde_json::Value>,
    ) -> f64 {
        let text_sim = self.calculate_text_similarity(env1, env2);
        
        // 如果有属性信息，也考虑属性相似度
        let props_sim = match (props1, props2) {
            (Some(p1), Some(p2)) => {
                // 简化的属性相似度计算
                if p1 == p2 { 1.0 } else { 0.5 }
            }
            (None, None) => 1.0,
            _ => 0.5,
        };
        
        (text_sim + props_sim) / 2.0
    }

    /// 计算目标相似度
    fn calculate_target_similarity(
        &self,
        target1: &str,
        target2: &str,
        hash1: &str,
        hash2: &str,
        props1: &Option<serde_json::Value>,
        props2: &Option<serde_json::Value>,
    ) -> f64 {
        // 如果哈希相同，认为是同一目标
        if hash1 == hash2 {
            return 1.0;
        }
        
        let text_sim = self.calculate_text_similarity(target1, target2);
        
        let props_sim = match (props1, props2) {
            (Some(p1), Some(p2)) => {
                if p1 == p2 { 1.0 } else { 0.5 }
            }
            (None, None) => 1.0,
            _ => 0.5,
        };
        
        (text_sim + props_sim) / 2.0
    }

    /// 从执行会话提取经验
    #[allow(unused)]
    fn extract_experience_from_session(&self, session: &ExecutionSession) -> Option<ExecutionExperience> {
        if session.step_results.is_empty() {
            return None;
        }

        // 提取成功的步骤
        let successful_steps: Vec<serde_json::Value> = session.step_results.iter()
            .filter(|(_, result)| matches!(result.status, crate::engines::types::StepExecutionStatus::Completed))
            .map(|(step_id, result)| serde_json::json!({
                "step_id": step_id,
                "tool_name": "unknown", // StepExecutionResult没有tool_name字段
                "parameters": {}, // StepExecutionResult没有parameters字段
                "execution_time_ms": result.completed_at.and_then(|end| 
                    Some((end.duration_since(result.started_at).ok()?.as_millis() as i64))
                ).unwrap_or(0),
                "output": result.result_data
            }))
            .collect();

        // 提取失败信息
        let failure_info = if session.step_results.iter().any(|(_, r)| matches!(r.status, crate::engines::types::StepExecutionStatus::Failed)) {
            let failed_steps: Vec<serde_json::Value> = session.step_results.iter()
                .filter(|(_, result)| matches!(result.status, crate::engines::types::StepExecutionStatus::Failed))
                .map(|(step_id, result)| serde_json::json!({
                    "step_id": step_id,
                    "tool_name": "unknown", // StepExecutionResult没有tool_name字段
                    "error": result.error,
                    "parameters": {} // StepExecutionResult没有parameters字段
                }))
                .collect();
            Some(serde_json::json!({ "failed_steps": failed_steps }))
        } else {
            None
        };

        // 计算性能指标
        let total_time: u64 = session.step_results.iter()
            .map(|(_, r)| r.metrics.execution_time_ms)
            .sum();
        
        let success_rate = successful_steps.len() as f64 / session.step_results.len() as f64;
        
        let performance_metrics = serde_json::json!({
            "total_execution_time_ms": total_time,
            "success_rate": success_rate,
            "step_count": session.step_results.len(),
            "successful_steps": successful_steps.len()
        });

        Some(ExecutionExperience {
            id: Uuid::new_v4().to_string(),
            task_type: "security_scan".to_string(), // 应该从会话中提取
            target_description: "Unknown target".to_string(), // 应该从会话中提取
            target_hash: "unknown_hash".to_string(), // 应该计算目标哈希
            target_properties: None,
            environment_context: "Unknown environment".to_string(), // 应该从会话中提取
            environment_hash: "unknown_env_hash".to_string(), // 应该计算环境哈希
            environment_properties: None,
            successful_steps,
            failure_info,
            performance_metrics: Some(performance_metrics),
            confidence_score: success_rate,
            usage_count: 0,
            last_used_at: None,
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
        })
    }

    /// 更新实体使用统计
    #[allow(unused)]
    fn update_entity_usage(&mut self, entity_id: &str) {
        if let Some(entity) = self.entities.get_mut(entity_id) {
            entity.usage_count += 1;
            entity.updated_at = chrono::Utc::now().timestamp();
        }
    }

    /// 更新关系使用统计
    #[allow(unused)]
    fn update_relationship_usage(&mut self, relationship_id: &str) {
        if let Some(relationship) = self.relationships.get_mut(relationship_id) {
            relationship.usage_count += 1;
            relationship.updated_at = chrono::Utc::now().timestamp();
        }
    }

    /// 计算工具在特定环境下的效果
    fn calculate_tool_effectiveness_in_context(
        &self,
        tool_name: &str,
        target_type: Option<&str>,
        environment: Option<&str>,
    ) -> f64 {
        let relevant_experiences: Vec<&ExecutionExperience> = self.experiences.values()
            .filter(|exp| {
                // 检查是否包含该工具
                exp.successful_steps.iter().any(|step| {
                    step.get("tool_name")
                        .and_then(|v| v.as_str())
                        .map(|name| name == tool_name)
                        .unwrap_or(false)
                })
            })
            .filter(|exp| {
                // 过滤目标类型
                target_type.map(|tt| exp.task_type.contains(tt)).unwrap_or(true)
            })
            .filter(|exp| {
                // 过滤环境
                environment.map(|env| exp.environment_context.contains(env)).unwrap_or(true)
            })
            .collect();

        if relevant_experiences.is_empty() {
            return 0.5; // 默认效果
        }

        let total_effectiveness: f64 = relevant_experiences.iter()
            .map(|exp| exp.confidence_score)
            .sum();

        total_effectiveness / relevant_experiences.len() as f64
    }
}

impl Memory for IntelligentMemory {
    fn store_experience(&mut self, experience: ExecutionExperience) -> Result<()> {
        self.experiences.insert(experience.id.clone(), experience);
        Ok(())
    }

    fn retrieve_similar_experiences(
        &self,
        query: &MemoryQuery,
    ) -> Result<Vec<SimilaritySearchResult<ExecutionExperience>>> {
        let start_time = std::time::Instant::now();
        let mut results = Vec::new();

        for experience in self.experiences.values() {
            let mut similarity_score = 0.0;
            let mut relevance_factors = Vec::new();
            let mut factor_count = 0;

            // 任务类型相似度
            if let Some(task_type) = &query.task_type {
                let task_sim = self.calculate_text_similarity(&experience.task_type, task_type);
                similarity_score += task_sim;
                factor_count += 1;
                if task_sim > 0.5 {
                    relevance_factors.push("task_type_match".to_string());
                }
            }

            // 目标相似度
            if let Some(target_desc) = &query.target_description {
                let target_sim = self.calculate_target_similarity(
                    &experience.target_description,
                    target_desc,
                    &experience.target_hash,
                    "query_hash", // 应该计算查询的哈希
                    &experience.target_properties,
                    &None,
                );
                similarity_score += target_sim;
                factor_count += 1;
                if target_sim > 0.5 {
                    relevance_factors.push("target_similarity".to_string());
                }
            }

            // 环境相似度
            if let Some(env_context) = &query.environment_context {
                let env_sim = self.calculate_environment_similarity(
                    &experience.environment_context,
                    env_context,
                    &experience.environment_properties,
                    &None,
                );
                similarity_score += env_sim;
                factor_count += 1;
                if env_sim > 0.5 {
                    relevance_factors.push("environment_match".to_string());
                }
            }

            // 工具匹配
            if let Some(tool_names) = &query.tool_names {
                let tool_match = experience.successful_steps.iter().any(|step| {
                    step.get("tool_name")
                        .and_then(|v| v.as_str())
                        .map(|name| tool_names.contains(&name.to_string()))
                        .unwrap_or(false)
                });
                if tool_match {
                    similarity_score += 1.0;
                    relevance_factors.push("tool_match".to_string());
                }
                factor_count += 1;
            }

            // 计算平均相似度
            if factor_count > 0 {
                similarity_score /= factor_count as f64;
            }

            // 应用置信度权重
            similarity_score *= experience.confidence_score;

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
        
        // 限制结果数量
        results.truncate(query.max_results);

        // 记录查询历史
        let _query_history = MemoryQueryHistory {
            id: Uuid::new_v4().to_string(),
            query_type: query.query_type.clone(),
            query_content: serde_json::to_value(query)?,
            results_count: results.len() as i32,
            execution_time_ms: start_time.elapsed().as_millis() as i32,
            similarity_threshold: Some(query.similarity_threshold),
            created_at: chrono::Utc::now().timestamp(),
        };
        
        // 注意：这里应该存储到数据库，但为了简化，我们暂时跳过

        Ok(results)
    }

    fn store_template(&mut self, template: PlanTemplate) -> Result<()> {
        self.templates.insert(template.id.clone(), template);
        Ok(())
    }

    fn retrieve_applicable_templates(
        &self,
        task_type: &str,
        _environment: &str,
        _target_properties: &serde_json::Value,
    ) -> Result<Vec<SimilaritySearchResult<PlanTemplate>>> {
        let mut results = Vec::new();

        for template in self.templates.values() {
            let mut similarity_score = 0.0;
            let mut relevance_factors = Vec::new();

            // 任务类型匹配
            let task_sim = self.calculate_text_similarity(&template.task_type, task_type);
            similarity_score += task_sim * 0.4; // 权重40%
            if task_sim > 0.7 {
                relevance_factors.push("task_type_match".to_string());
            }

            // 领域匹配
            let domain_sim = self.calculate_text_similarity(&template.domain, task_type);
            similarity_score += domain_sim * 0.3; // 权重30%
            if domain_sim > 0.5 {
                relevance_factors.push("domain_match".to_string());
            }

            // 成功率权重
            similarity_score += template.success_rate * 0.2; // 权重20%
            if template.success_rate > 0.8 {
                relevance_factors.push("high_success_rate".to_string());
            }

            // 效果评分权重
            similarity_score += template.effectiveness_score * 0.1; // 权重10%
            if template.effectiveness_score > 0.8 {
                relevance_factors.push("high_effectiveness".to_string());
            }

            if similarity_score >= 0.5 { // 阈值
                results.push(SimilaritySearchResult {
                    item: template.clone(),
                    similarity_score,
                    relevance_factors,
                });
            }
        }

        // 按相似度排序
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
        let mut queue = std::collections::VecDeque::new();
        
        // 找到起始实体
        let start_entities: Vec<&KnowledgeEntity> = self.entities.values()
            .filter(|entity| entity.name.contains(entity_name))
            .collect();
        
        for entity in start_entities {
            queue.push_back((entity.id.clone(), 0));
        }
        
        while let Some((entity_id, depth)) = queue.pop_front() {
            if depth >= max_depth || visited.contains(&entity_id) {
                continue;
            }
            
            visited.insert(entity_id.clone());
            
            if let Some(entity) = self.entities.get(&entity_id) {
                results.push(entity.clone());
                
                // 查找相关关系
                for relationship in self.relationships.values() {
                    if relationship_types.contains(&relationship.relationship_type) {
                        if relationship.from_entity == entity_id {
                            queue.push_back((relationship.to_entity.clone(), depth + 1));
                        } else if relationship.to_entity == entity_id {
                            queue.push_back((relationship.from_entity.clone(), depth + 1));
                        }
                    }
                }
            }
        }
        
        Ok(results)
    }

    fn process_learning_feedback(&mut self, feedback: LearningFeedback) -> Result<()> {
        self.feedback_history.push(feedback);
        
        // 这里应该处理反馈，更新相关实体的置信度等
        // 简化实现，实际应该更复杂
        
        Ok(())
    }

    fn learn_from_execution(&mut self, update: LearningUpdate) -> Result<()> {
        // 从执行结果中提取经验并存储
        // 这里应该分析执行结果，提取有用的模式和知识
        
        // 简化实现：创建学习反馈
        let feedback = LearningFeedback {
            id: Uuid::new_v4().to_string(),
            experience_id: None,
            template_id: None,
            entity_id: None,
            relationship_id: None,
            feedback_type: if update.execution_result.status == StepExecutionStatus::Completed {
                FeedbackType::Success
            } else {
                FeedbackType::Failure
            },
            feedback_content: serde_json::to_value(&update.execution_result)?,
            improvements: None,
            confidence_adjustments: None,
            user_rating: None,
            automated_score: Some(if update.execution_result.status == StepExecutionStatus::Completed { 1.0 } else { 0.0 }),
            created_at: chrono::Utc::now().timestamp(),
            processed_at: None,
        };
        
        self.process_learning_feedback(feedback)?;
        
        Ok(())
    }

    fn get_tool_effectiveness(
        &self,
        tool_name: &str,
        target_type: Option<&str>,
        environment: Option<&str>,
    ) -> Result<f64> {
        Ok(self.calculate_tool_effectiveness_in_context(tool_name, target_type, environment))
    }

    fn get_environment_specific_recommendations(
        &self,
        environment: &str,
        task_type: &str,
    ) -> Result<Vec<String>> {
        let mut recommendations = Vec::new();
        
        // 基于历史经验生成建议
        let relevant_experiences: Vec<&ExecutionExperience> = self.experiences.values()
            .filter(|exp| {
                exp.environment_context.contains(environment) && 
                exp.task_type.contains(task_type) &&
                exp.confidence_score > 0.7
            })
            .collect();
        
        // 提取成功的工具和技术
        let mut tool_usage = HashMap::new();
        for exp in relevant_experiences {
            for step in &exp.successful_steps {
                if let Some(tool_name) = step.get("tool_name").and_then(|v| v.as_str()) {
                    *tool_usage.entry(tool_name.to_string()).or_insert(0) += 1;
                }
            }
        }
        
        // 生成工具推荐
        let mut tool_recommendations: Vec<(String, usize)> = tool_usage.into_iter().collect();
        tool_recommendations.sort_by(|a, b| b.1.cmp(&a.1));
        
        for (tool, count) in tool_recommendations.into_iter().take(5) {
            recommendations.push(format!("推荐使用 {} (成功使用 {} 次)", tool, count));
        }
        
        // 如果没有足够的历史数据，提供通用建议
        if recommendations.is_empty() {
            recommendations.push("建议先进行端口扫描以了解目标服务".to_string());
            recommendations.push("根据发现的服务选择相应的扫描工具".to_string());
            recommendations.push("注意在扫描过程中监控资源使用情况".to_string());
        }
        
        Ok(recommendations)
    }
    
    // 架构特定的记忆增强接口实现
    fn retrieve_failure_trajectories(
        &self,
        task_description: &str,
        error_pattern: &str,
    ) -> Result<Vec<SimilaritySearchResult<ExecutionExperience>>> {
        self.retrieve_failure_trajectories(task_description, error_pattern)
    }
    
    fn store_execution_trajectory(
        &mut self,
        task_description: String,
        steps: Vec<serde_json::Value>,
        success: bool,
        error_info: Option<serde_json::Value>,
    ) -> Result<()> {
        self.store_execution_trajectory(task_description, steps, success, error_info)
    }
    
    fn retrieve_few_shot_plans(
        &self,
        task_description: &str,
        max_results: usize,
    ) -> Result<Vec<SimilaritySearchResult<PlanTemplate>>> {
        self.retrieve_few_shot_plans(task_description, max_results)
    }
    
    fn store_rewoo_plan_blueprint(
        &mut self,
        task_description: String,
        plan_steps: Vec<serde_json::Value>,
        success_rate: f64,
    ) -> Result<()> {
        self.store_rewoo_plan_blueprint(task_description, plan_steps, success_rate)
    }
    
    fn check_tool_call_cache(
        &self,
        tool_name: &str,
        tool_args: &serde_json::Value,
    ) -> Result<Option<serde_json::Value>> {
        self.check_tool_call_cache(tool_name, tool_args)
    }
    
    fn cache_tool_call_result(
        &mut self,
        tool_name: String,
        tool_args: serde_json::Value,
        result: serde_json::Value,
        execution_time_ms: u64,
    ) -> Result<()> {
        self.cache_tool_call_result(tool_name, tool_args, result, execution_time_ms)
    }
    
    fn retrieve_reasoning_chains(
        &self,
        task_description: &str,
        max_results: usize,
    ) -> Result<Vec<SimilaritySearchResult<ExecutionExperience>>> {
        self.retrieve_reasoning_chains(task_description, max_results)
    }
}

impl Default for IntelligentMemory {
    fn default() -> Self {
        Self::new()
    }
}

// 为IntelligentMemory添加架构特定的记忆增强方法
impl IntelligentMemory {
    /// Plan-and-Execute: 检索相似的失败轨迹（用于避免重复错误）
    pub fn retrieve_failure_trajectories(
        &self,
        task_description: &str,
        error_pattern: &str,
    ) -> Result<Vec<SimilaritySearchResult<ExecutionExperience>>> {
        let mut results = Vec::new();
        
        for experience in self.experiences.values() {
            // 只检索失败的经验
            if experience.failure_info.is_none() {
                continue;
            }
            
            // 计算任务描述相似度
            let task_sim = self.calculate_text_similarity(&experience.task_type, task_description);
            
            // 检查错误模式匹配
            let error_match = if let Some(failure) = &experience.failure_info {
                let failure_str = serde_json::to_string(failure).unwrap_or_default();
                self.calculate_text_similarity(&failure_str, error_pattern) > 0.3
            } else {
                false
            };
            
            if task_sim > 0.5 || error_match {
                let similarity_score = if error_match { task_sim + 0.3 } else { task_sim };
                results.push(SimilaritySearchResult {
                    item: experience.clone(),
                    similarity_score,
                    relevance_factors: vec!["failure_trajectory".to_string()],
                });
            }
        }
        
        results.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap());
        results.truncate(5); // 最多返回5个失败案例
        
        Ok(results)
    }
    
    /// Plan-and-Execute: 存储执行轨迹（包含完整的步骤序列）
    pub fn store_execution_trajectory(
        &mut self,
        task_description: String,
        steps: Vec<serde_json::Value>,
        success: bool,
        error_info: Option<serde_json::Value>,
    ) -> Result<()> {
        let experience = ExecutionExperience {
            id: Uuid::new_v4().to_string(),
            task_type: task_description.clone(),
            target_description: "trajectory".to_string(),
            target_hash: format!("{:x}", md5::compute(&task_description)),
            target_properties: None,
            environment_context: "plan_and_execute".to_string(),
            environment_hash: "pae".to_string(),
            environment_properties: None,
            successful_steps: if success { steps } else { vec![] },
            failure_info: error_info,
            performance_metrics: None,
            confidence_score: if success { 0.9 } else { 0.3 },
            usage_count: 0,
            last_used_at: None,
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
        };
        
        self.store_experience(experience)
    }
    
    /// ReWOO: 检索Few-Shot规划示例（用于规划阶段）
    pub fn retrieve_few_shot_plans(
        &self,
        task_description: &str,
        max_results: usize,
    ) -> Result<Vec<SimilaritySearchResult<PlanTemplate>>> {
        let mut results = Vec::new();
        
        for template in self.templates.values() {
            // 只检索高成功率的模板
            if template.success_rate < 0.7 {
                continue;
            }
            
            let task_sim = self.calculate_text_similarity(&template.task_type, task_description);
            let domain_sim = self.calculate_text_similarity(&template.domain, task_description);
            
            let similarity_score = (task_sim * 0.6 + domain_sim * 0.4) * template.success_rate;
            
            if similarity_score > 0.5 {
                results.push(SimilaritySearchResult {
                    item: template.clone(),
                    similarity_score,
                    relevance_factors: vec!["few_shot_plan".to_string(), "high_success_rate".to_string()],
                });
            }
        }
        
        results.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap());
        results.truncate(max_results);
        
        Ok(results)
    }
    
    /// ReWOO: 存储成功的规划蓝图
    pub fn store_rewoo_plan_blueprint(
        &mut self,
        task_description: String,
        plan_steps: Vec<serde_json::Value>,
        success_rate: f64,
    ) -> Result<()> {
        let template = PlanTemplate {
            id: Uuid::new_v4().to_string(),
            name: format!("ReWOO Plan: {}", task_description),
            description: Some(format!("Successful ReWOO plan blueprint")),
            domain: "rewoo".to_string(),
            task_type: task_description,
            template_steps: plan_steps,
            success_rate,
            usage_count: 0,
            effectiveness_score: success_rate,
            applicability_conditions: None,
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
            last_used_at: None,
        };
        
        self.store_template(template)
    }
    
    /// LLM Compiler: 检查工具调用缓存
    pub fn check_tool_call_cache(
        &self,
        tool_name: &str,
        tool_args: &serde_json::Value,
    ) -> Result<Option<serde_json::Value>> {
        // 计算缓存键（工具名+参数哈希）
        let args_str = serde_json::to_string(tool_args)?;
        let cache_key = format!("{}:{:x}", tool_name, md5::compute(&args_str));
        
        // 在经验中查找匹配的工具调用
        for experience in self.experiences.values() {
            for step in &experience.successful_steps {
                if let Some(step_tool) = step.get("tool_name").and_then(|v| v.as_str()) {
                    if step_tool == tool_name {
                        if let Some(step_args) = step.get("parameters") {
                            let step_args_str = serde_json::to_string(step_args).unwrap_or_default();
                            let step_cache_key = format!("{}:{:x}", tool_name, md5::compute(&step_args_str));
                            
                            if cache_key == step_cache_key {
                                // 找到缓存命中
                                if let Some(output) = step.get("output") {
                                    return Ok(Some(output.clone()));
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(None)
    }
    
    /// LLM Compiler: 存储工具调用结果到缓存
    pub fn cache_tool_call_result(
        &mut self,
        tool_name: String,
        tool_args: serde_json::Value,
        result: serde_json::Value,
        execution_time_ms: u64,
    ) -> Result<()> {
        // 创建一个特殊的经验记录用于缓存
        let cache_step = serde_json::json!({
            "tool_name": tool_name,
            "parameters": tool_args,
            "output": result,
            "execution_time_ms": execution_time_ms,
            "cached_at": chrono::Utc::now().timestamp()
        });
        
        let experience = ExecutionExperience {
            id: Uuid::new_v4().to_string(),
            task_type: "tool_cache".to_string(),
            target_description: tool_name.clone(),
            target_hash: format!("{:x}", md5::compute(&tool_name)),
            target_properties: None,
            environment_context: "llm_compiler".to_string(),
            environment_hash: "llmc".to_string(),
            environment_properties: None,
            successful_steps: vec![cache_step],
            failure_info: None,
            performance_metrics: Some(serde_json::json!({
                "execution_time_ms": execution_time_ms
            })),
            confidence_score: 1.0,
            usage_count: 0,
            last_used_at: None,
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
        };
        
        self.store_experience(experience)
    }
    
    /// ReAct: 检索相似的推理链（用于提示工程）
    pub fn retrieve_reasoning_chains(
        &self,
        task_description: &str,
        max_results: usize,
    ) -> Result<Vec<SimilaritySearchResult<ExecutionExperience>>> {
        let mut results = Vec::new();
        
        for experience in self.experiences.values() {
            // 只检索成功的经验
            if experience.confidence_score < 0.7 {
                continue;
            }
            
            let task_sim = self.calculate_text_similarity(&experience.task_type, task_description);
            
            if task_sim > 0.5 {
                results.push(SimilaritySearchResult {
                    item: experience.clone(),
                    similarity_score: task_sim * experience.confidence_score,
                    relevance_factors: vec!["reasoning_chain".to_string()],
                });
            }
        }
        
        results.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap());
        results.truncate(max_results);
        
        Ok(results)
    }
}