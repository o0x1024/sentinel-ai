//! 威胁情报集成
//!
//! 实现RAG+工具混合模式的威胁情报查询

use super::types::*;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 威胁情报管理器
pub struct ThreatIntelManager {
    config: ThreatIntelConfig,
    cache: Arc<RwLock<ThreatIntelCache>>,
}

/// 威胁情报缓存
struct ThreatIntelCache {
    entries: HashMap<String, CachedThreatIntel>,
}

/// 缓存的威胁情报
struct CachedThreatIntel {
    data: Vec<ThreatInfo>,
    cached_at: std::time::SystemTime,
}

impl ThreatIntelManager {
    pub fn new(config: ThreatIntelConfig) -> Self {
        Self {
            config,
            cache: Arc::new(RwLock::new(ThreatIntelCache {
                entries: HashMap::new(),
            })),
        }
    }

    /// 查询威胁情报(混合模式)
    pub async fn query_threat_intel(
        &self,
        query: &str,
        context: &HashMap<String, serde_json::Value>,
    ) -> Result<Vec<ThreatInfo>> {
        // 1. 检查缓存
        if let Some(cached) = self.get_from_cache(query).await {
            log::info!("Threat intel cache hit for query: {}", query);
            return Ok(cached);
        }

        let mut threats = Vec::new();

        // 2. RAG查询(常用漏洞模式)
        if self.config.enable_rag {
            log::info!("Querying threat intel from RAG knowledge base");
            if let Ok(rag_threats) = self.query_rag(query, context).await {
                threats.extend(rag_threats);
            }
        }

        // 3. CVE工具查询(实时数据)
        if self.config.enable_cve_tool {
            log::info!("Querying CVE database via tool");
            if let Ok(cve_threats) = self.query_cve_tool(query, context).await {
                threats.extend(cve_threats);
            }
        }

        // 4. 去重和排序
        threats = self.deduplicate_and_sort(threats);

        // 5. 缓存结果
        self.cache_result(query, threats.clone()).await;

        Ok(threats)
    }

    /// 从RAG知识库查询
    async fn query_rag(
        &self,
        query: &str,
        _context: &HashMap<String, serde_json::Value>,
    ) -> Result<Vec<ThreatInfo>> {
        log::info!(
            "RAG query: '{}' (top_k={}, threshold={})",
            query,
            self.config.rag_top_k,
            self.config.rag_threshold
        );

        // 获取全局RAG服务
        use crate::commands::rag_commands::get_global_rag_service;
        let rag_service = match get_global_rag_service().await {
            Ok(service) => service,
            Err(e) => {
                log::warn!("Failed to get RAG service: {}, using fallback", e);
                return Ok(Vec::new());
            }
        };

        // 构建RAG查询请求
        use sentinel_rag::models::AssistantRagRequest;
        let request = AssistantRagRequest {
            query: query.to_string(),
            collection_id: None, // 查询所有集合
            conversation_history: None,
            top_k: Some(self.config.rag_top_k),
            use_mmr: Some(false),
            mmr_lambda: None,
            similarity_threshold: Some(self.config.rag_threshold),
            reranking_enabled: Some(true),
            model_provider: None,
            model_name: None,
            max_tokens: None,
            temperature: None,
            system_prompt: None,
        };

        // 执行查询
        match rag_service.query_for_assistant(&request).await {
            Ok((context, citations)) => {
                log::info!("RAG query returned {} citations", citations.len());
                
                // 解析RAG结果为威胁情报
                let mut threats = Vec::new();
                
                // 从上下文中提取威胁信息
                // 这里简化处理，实际应该解析结构化的威胁情报数据
                if !context.is_empty() {
                    // 尝试从上下文中提取CVE信息
                    let cve_regex = regex::Regex::new(r"CVE-\d{4}-\d{4,7}").unwrap();
                    let cves: Vec<String> = cve_regex
                        .find_iter(&context)
                        .map(|m| m.as_str().to_string())
                        .collect();

                    // 为每个引用创建一个威胁信息条目
                    for (idx, citation) in citations.iter().enumerate() {
                        let threat_level = if context.to_lowercase().contains("critical") || context.to_lowercase().contains("high") {
                            ThreatLevel::High
                        } else if context.to_lowercase().contains("medium") {
                            ThreatLevel::Medium
                        } else {
                            ThreatLevel::Low
                        };

                        threats.push(ThreatInfo {
                            id: format!("rag-{:03}", idx + 1),
                            name: citation.source_id.clone(),
                            description: citation.content_preview.clone(),
                            level: threat_level,
                            cves: cves.clone(),
                            source: ThreatSource::RAG,
                        });
                    }
                }

                Ok(threats)
            }
            Err(e) => {
                log::warn!("RAG query failed: {}", e);
                Ok(Vec::new())
            }
        }
    }

    /// 通过工具查询CVE数据库
    async fn query_cve_tool(
        &self,
        query: &str,
        context: &HashMap<String, serde_json::Value>,
    ) -> Result<Vec<ThreatInfo>> {
        log::info!("CVE tool query: '{}'", query);

        // 从context中提取技术栈信息
        let technology = context
            .get("technology")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // 构建CVE搜索参数
        let search_query = if !technology.is_empty() {
            format!("{} {}", technology, query)
        } else {
            query.to_string()
        };

        // CVE工具查询 - 占位实现
        // TODO: 实际项目中应该集成CVE数据库API或使用专门的CVE查询工具
        // 这里返回模拟数据以允许系统正常运行
        
        log::info!("CVE tool query for: '{}' (placeholder implementation)", search_query);
        
        // 返回模拟的CVE数据
        let threats = if !technology.is_empty() {
            vec![
                ThreatInfo {
                    id: "cve-001".to_string(),
                    name: format!("CVE-2024-XXXXX"),
                    description: format!("Known vulnerability in {} - {}", technology, search_query),
                    level: ThreatLevel::High,
                    cves: vec!["CVE-2024-XXXXX".to_string()],
                    source: ThreatSource::CVE,
                },
            ]
        } else {
            vec![]
        };

        log::info!("Returning {} placeholder CVE threats", threats.len());
        Ok(threats)
    }

    /// 去重和排序
    fn deduplicate_and_sort(&self, mut threats: Vec<ThreatInfo>) -> Vec<ThreatInfo> {
        // 按ID去重
        let mut seen = std::collections::HashSet::new();
        threats.retain(|t| seen.insert(t.id.clone()));

        // 按威胁等级排序(从高到低)
        threats.sort_by(|a, b| b.level.cmp(&a.level));

        threats
    }

    /// 从缓存获取
    async fn get_from_cache(&self, query: &str) -> Option<Vec<ThreatInfo>> {
        let cache = self.cache.read().await;
        if let Some(entry) = cache.entries.get(query) {
            // 检查是否过期
            let elapsed = std::time::SystemTime::now()
                .duration_since(entry.cached_at)
                .ok()?;
            if elapsed.as_secs() < self.config.cache_duration {
                return Some(entry.data.clone());
            }
        }
        None
    }

    /// 缓存结果
    async fn cache_result(&self, query: &str, data: Vec<ThreatInfo>) {
        let mut cache = self.cache.write().await;
        cache.entries.insert(
            query.to_string(),
            CachedThreatIntel {
                data,
                cached_at: std::time::SystemTime::now(),
            },
        );
    }

    /// 分析威胁并生成ThreatAnalysis
    pub async fn analyze_threats(
        &self,
        threats: Vec<ThreatInfo>,
        vulnerabilities: Vec<VulnerabilityInfo>,
    ) -> ThreatAnalysis {
        // 计算整体威胁等级
        let threat_level = if threats.iter().any(|t| t.level == ThreatLevel::Critical) {
            ThreatLevel::Critical
        } else if threats.iter().any(|t| t.level == ThreatLevel::High) {
            ThreatLevel::High
        } else if threats.iter().any(|t| t.level == ThreatLevel::Medium) {
            ThreatLevel::Medium
        } else if threats.iter().any(|t| t.level == ThreatLevel::Low) {
            ThreatLevel::Low
        } else {
            ThreatLevel::Info
        };

        // 生成推荐行动
        let recommendations = self.generate_recommendations(&threats, &vulnerabilities);

        ThreatAnalysis {
            threats,
            vulnerabilities,
            threat_level,
            recommendations,
        }
    }

    /// 生成推荐行动
    fn generate_recommendations(
        &self,
        threats: &[ThreatInfo],
        vulnerabilities: &[VulnerabilityInfo],
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        // 基于威胁生成建议
        for threat in threats {
            match threat.level {
                ThreatLevel::Critical | ThreatLevel::High => {
                    recommendations.push(format!(
                        "Prioritize testing for {} (Level: {:?})",
                        threat.name, threat.level
                    ));
                }
                _ => {}
            }
        }

        // 基于漏洞生成建议
        for vuln in vulnerabilities {
            if let Some(cvss) = vuln.cvss_score {
                if cvss >= 7.0 {
                    recommendations.push(format!(
                        "High severity vulnerability detected: {} (CVSS: {})",
                        vuln.name, cvss
                    ));
                }
            }
        }

        // 通用建议
        if recommendations.is_empty() {
            recommendations.push("Perform standard security assessment".to_string());
        }

        recommendations
    }
}

/// 漏洞信息构建器
pub struct VulnerabilityInfoBuilder {
    id: String,
    name: String,
    description: String,
    cvss_score: Option<f32>,
    cve_id: Option<String>,
    affected_component: String,
    remediation: Option<String>,
}

impl VulnerabilityInfoBuilder {
    pub fn new(name: String, affected_component: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            description: String::new(),
            cvss_score: None,
            cve_id: None,
            affected_component,
            remediation: None,
        }
    }

    pub fn description(mut self, desc: String) -> Self {
        self.description = desc;
        self
    }

    pub fn cvss_score(mut self, score: f32) -> Self {
        self.cvss_score = Some(score);
        self
    }

    pub fn cve_id(mut self, cve: String) -> Self {
        self.cve_id = Some(cve);
        self
    }

    pub fn remediation(mut self, rem: String) -> Self {
        self.remediation = Some(rem);
        self
    }

    pub fn build(self) -> VulnerabilityInfo {
        VulnerabilityInfo {
            id: self.id,
            name: self.name,
            description: self.description,
            cvss_score: self.cvss_score,
            cve_id: self.cve_id,
            affected_component: self.affected_component,
            remediation: self.remediation,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_query_threat_intel() {
        let config = ThreatIntelConfig::default();
        let manager = ThreatIntelManager::new(config);

        let mut context = HashMap::new();
        context.insert(
            "technology".to_string(),
            serde_json::Value::String("WordPress".to_string()),
        );

        let threats = manager
            .query_threat_intel("SQL injection", &context)
            .await
            .unwrap();

        assert!(!threats.is_empty());
    }

    #[tokio::test]
    async fn test_cache() {
        let config = ThreatIntelConfig::default();
        let manager = ThreatIntelManager::new(config);

        let context = HashMap::new();

        // 第一次查询
        let threats1 = manager
            .query_threat_intel("XSS", &context)
            .await
            .unwrap();

        // 第二次查询应该命中缓存
        let threats2 = manager
            .query_threat_intel("XSS", &context)
            .await
            .unwrap();

        assert_eq!(threats1.len(), threats2.len());
    }

    #[test]
    fn test_vulnerability_builder() {
        let vuln = VulnerabilityInfoBuilder::new(
            "SQL Injection".to_string(),
            "login.php".to_string(),
        )
        .description("SQL injection in login form".to_string())
        .cvss_score(9.8)
        .cve_id("CVE-2024-12345".to_string())
        .remediation("Use prepared statements".to_string())
        .build();

        assert_eq!(vuln.name, "SQL Injection");
        assert_eq!(vuln.cvss_score, Some(9.8));
    }
}

