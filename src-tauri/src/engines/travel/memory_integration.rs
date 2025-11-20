//! Memory系统集成
//!
//! 将智能记忆系统集成到Travel OODA循环的各个阶段

use super::types::*;
use crate::engines::memory::IntelligentMemory;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};

/// Travel Memory集成器
pub struct TravelMemoryIntegration {
    memory: Arc<RwLock<IntelligentMemory>>,
}

impl TravelMemoryIntegration {
    /// 创建新的Memory集成器
    pub fn new(memory: Arc<RwLock<IntelligentMemory>>) -> Self {
        Self { memory }
    }

    /// Observe阶段:查询相似任务经验
    ///
    /// 在侦察阶段,查询历史上类似任务的执行经验,帮助决策
    pub async fn query_similar_experiences(
        &self,
        task_description: &str,
        target_info: &str,
    ) -> Result<Vec<ExecutionExperience>> {
        log::info!("Querying similar experiences for task: {} target: {}", task_description, target_info);

        // 构建查询
        use crate::engines::memory::memory::{MemoryQuery, QueryType};
        let query = MemoryQuery {
            query_type: QueryType::SuccessfulPatterns,
            target_description: Some(format!("{} {}", task_description, target_info)),
            environment_context: None,
            task_type: Some("security_testing".to_string()),
            tool_names: None,
            error_patterns: None,
            similarity_threshold: 0.6,
            max_results: 5,
            include_metadata: true,
        };

        // 从Memory中检索相似经验
        let memory_guard = self.memory.read().await;
        use crate::engines::memory::memory::Memory;
        let results = memory_guard.retrieve_similar_experiences(&query)?;

        // 转换为简化的ExecutionExperience
        let experiences: Vec<ExecutionExperience> = results
            .into_iter()
            .map(|result| ExecutionExperience {
                task_type: result.item.task_type.clone(),
                success: !result.item.failure_info.is_some(),
                tools_used: vec![], // 简化处理
                duration_ms: 0,
                notes: format!("Similarity: {:.2}", result.similarity_score),
            })
            .collect();

        log::info!("Found {} similar experiences", experiences.len());
        Ok(experiences)
    }

    /// Orient阶段:查询知识图谱
    ///
    /// 在分析阶段,查询知识图谱中的相关实体和关系
    pub async fn query_knowledge_graph(
        &self,
        entities: &[String],
    ) -> Result<Vec<KnowledgeEntity>> {
        log::info!("Querying knowledge graph for entities: {:?}", entities);

        let memory_guard = self.memory.read().await;
        use crate::engines::memory::memory::{Memory, RelationshipType};
        
        let mut all_entities = Vec::new();
        
        // 对每个实体查询知识图谱
        for entity_name in entities {
            match memory_guard.query_knowledge_graph(
                entity_name,
                &[
                    RelationshipType::EffectiveAgainst,
                    RelationshipType::Requires,
                    RelationshipType::Enhances,
                ],
                2, // max_depth
            ) {
                Ok(entities_result) => {
                    // 转换为我们的KnowledgeEntity格式
                    for entity in entities_result {
                        all_entities.push(KnowledgeEntity {
                            name: entity.name.clone(),
                            entity_type: format!("{:?}", entity.entity_type),
                            properties: entity.properties.unwrap_or_default().as_object().cloned().unwrap_or_default(),
                            relationships: vec![], // 简化处理
                        });
                    }
                }
                Err(e) => {
                    log::warn!("Failed to query knowledge graph for entity '{}': {}", entity_name, e);
                }
            }
        }

        log::info!("Found {} knowledge entities", all_entities.len());
        Ok(all_entities)
    }

    /// Decide阶段:获取计划模板
    ///
    /// 在决策阶段,获取历史成功的计划模板
    pub async fn get_plan_templates(
        &self,
        task_type: &str,
    ) -> Result<Vec<PlanTemplate>> {
        log::info!("Getting plan templates for task type: {}", task_type);

        let memory_guard = self.memory.read().await;
        use crate::engines::memory::memory::Memory;
        
        // 检索适用的模板
        let results = memory_guard.retrieve_applicable_templates(
            task_type,
            "default", // environment
            &serde_json::json!({}), // target_properties
        )?;

        // 转换为PlanTemplate
        let templates: Vec<PlanTemplate> = results
            .into_iter()
            .map(|result| PlanTemplate {
                name: result.item.name.clone(),
                task_type: result.item.task_type.clone(),
                steps: vec![], // 简化处理
                success_rate: result.item.success_rate as f32,
                usage_count: result.item.usage_count as u32,
            })
            .collect();

        log::info!("Found {} plan templates", templates.len());
        Ok(templates)
    }

    /// Act后:存储执行经验
    ///
    /// 在执行完成后,将OODA循环的经验存储到记忆系统
    pub async fn store_execution(&self, cycle: &OodaCycle) -> Result<()> {
        log::info!("Storing execution experience for cycle {}", cycle.cycle_number);

        // 构建ExecutionExperience
        use crate::engines::memory::memory::ExecutionExperience as MemExperience;
        use chrono::Utc;
        
        // 提取工具调用信息
        let mut successful_steps = Vec::new();
        for phase in &cycle.phase_history {
            for tool_call in &phase.tool_calls {
                successful_steps.push(serde_json::json!({
                    "tool": tool_call.tool_name,
                    "phase": format!("{:?}", phase.phase),
                }));
            }
        }

        // 构建经验记录
        let experience = MemExperience {
            id: uuid::Uuid::new_v4().to_string(),
            task_type: "security_testing".to_string(),
            target_description: format!("OODA Cycle #{}", cycle.cycle_number),
            target_hash: format!("{:x}", md5::compute(format!("cycle-{}", cycle.cycle_number))),
            target_properties: None,
            environment_context: format!("Phase: {:?}", cycle.current_phase),
            environment_hash: format!("{:x}", md5::compute(format!("{:?}", cycle.current_phase))),
            environment_properties: None,
            successful_steps,
            failure_info: if cycle.status != OodaCycleStatus::Completed {
                Some(serde_json::json!({"status": format!("{:?}", cycle.status)}))
            } else {
                None
            },
            performance_metrics: Some(serde_json::json!({
                "phases": cycle.phase_history.len(),
                "status": format!("{:?}", cycle.status),
            })),
            confidence_score: if cycle.status == OodaCycleStatus::Completed { 0.8 } else { 0.3 },
            usage_count: 1,
            last_used_at: Some(Utc::now().timestamp()),
            created_at: Utc::now().timestamp(),
            updated_at: Utc::now().timestamp(),
        };

        // 存储到Memory
        let mut memory_guard = self.memory.write().await;
        use crate::engines::memory::memory::Memory;
        memory_guard.store_experience(experience)?;
        
        log::info!("Successfully stored execution experience for cycle {}", cycle.cycle_number);
        Ok(())
    }

    /// 查询威胁情报相关记忆
    pub async fn query_threat_intelligence(
        &self,
        threat_type: &str,
    ) -> Result<Vec<String>> {
        log::info!("Querying threat intelligence for: {}", threat_type);

        // 构建查询
        use crate::engines::memory::memory::{MemoryQuery, QueryType};
        let query = MemoryQuery {
            query_type: QueryType::SuccessfulPatterns,
            target_description: Some(format!("threat intelligence {}", threat_type)),
            environment_context: None,
            task_type: Some("threat_analysis".to_string()),
            tool_names: None,
            error_patterns: None,
            similarity_threshold: 0.5,
            max_results: 10,
            include_metadata: true,
        };

        let memory_guard = self.memory.read().await;
        use crate::engines::memory::memory::Memory;
        
        // 查询相似经验
        let results = memory_guard.retrieve_similar_experiences(&query)?;
        
        // 提取威胁情报信息
        let intel: Vec<String> = results
            .into_iter()
            .map(|result| format!("{}: confidence {:.2}", result.item.task_type, result.similarity_score))
            .collect();

        log::info!("Found {} threat intelligence records", intel.len());
        Ok(intel)
    }

    /// 查询漏洞相关记忆
    pub async fn query_vulnerability_knowledge(
        &self,
        cve_id: &str,
    ) -> Result<Option<VulnerabilityKnowledge>> {
        log::info!("Querying vulnerability knowledge for: {}", cve_id);

        let memory_guard = self.memory.read().await;
        use crate::engines::memory::memory::{Memory, RelationshipType};
        
        // 查询知识图谱中的CVE实体
        match memory_guard.query_knowledge_graph(
            cve_id,
            &[RelationshipType::EffectiveAgainst, RelationshipType::Requires],
            1,
        ) {
            Ok(entities) => {
                if let Some(entity) = entities.first() {
                    // 从实体属性中提取漏洞信息
                    let props = entity.properties.as_ref().and_then(|v| v.as_object());
                    let vuln = VulnerabilityKnowledge {
                        cve_id: cve_id.to_string(),
                        description: props
                            .and_then(|p| p.get("description"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("No description")
                            .to_string(),
                        severity: props
                            .and_then(|p| p.get("severity"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown")
                            .to_string(),
                        affected_systems: props
                            .and_then(|p| p.get("affected_systems"))
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                            .unwrap_or_default(),
                        exploit_available: props
                            .and_then(|p| p.get("exploit_available"))
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                    };
                    Ok(Some(vuln))
                } else {
                    Ok(None)
                }
            }
            Err(e) => {
                log::warn!("Failed to query vulnerability knowledge for {}: {}", cve_id, e);
                Ok(None)
            }
        }
    }
}

/// 执行经验
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionExperience {
    pub task_type: String,
    pub success: bool,
    pub tools_used: Vec<String>,
    pub duration_ms: u64,
    pub notes: String,
}

/// 知识实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntity {
    pub name: String,
    pub entity_type: String,
    pub properties: serde_json::Map<String, serde_json::Value>,
    pub relationships: Vec<String>,
}

/// 计划模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanTemplate {
    pub name: String,
    pub task_type: String,
    pub steps: Vec<String>,
    pub success_rate: f32,
    pub usage_count: u32,
}

/// 漏洞知识
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VulnerabilityKnowledge {
    pub cve_id: String,
    pub description: String,
    pub severity: String,
    pub affected_systems: Vec<String>,
    pub exploit_available: bool,
}
