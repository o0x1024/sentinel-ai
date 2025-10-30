use sentinel_db::database::AssetDao;
use crate::models::asset::*;
use sqlx::SqlitePool;
use std::collections::HashMap;

pub struct AssetService {
    dao: AssetDao,
}

impl AssetService {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            dao: AssetDao::new(pool),
        }
    }

    /// 创建资产
    pub async fn create_asset(
        &self,
        request: CreateAssetRequest,
        created_by: String,
    ) -> Result<Asset, String> {
        self.dao.create_asset(request, created_by).await
            .map_err(|e| format!("Database error: {}", e))
    }

    /// 获取资产详情（包含关系信息）
    pub async fn get_asset_detail(&self, id: &str) -> Result<Option<AssetDetail>, String> {
        if let Some(asset) = self.dao.get_asset_by_id(id).await.map_err(|e| format!("Database error: {}", e))? {
            let (incoming, outgoing) = self.dao.get_asset_relationships(id).await
                .map_err(|e| format!("Database error: {}", e))?;
            
            // TODO: 获取历史记录
            let history = Vec::new();
            
            Ok(Some(AssetDetail {
                asset,
                incoming_relationships: incoming,
                outgoing_relationships: outgoing,
                history,
            }))
        } else {
            Ok(None)
        }
    }

    /// 更新资产
    pub async fn update_asset(&self, id: &str, request: UpdateAssetRequest) -> Result<bool, String> {
        self.dao.update_asset(id, request).await
            .map_err(|e| format!("Database error: {}", e))
    }

    /// 删除资产
    pub async fn delete_asset(&self, id: &str) -> Result<bool, String> {
        self.dao.delete_asset(id).await
            .map_err(|e| format!("Database error: {}", e))
    }

    /// 查询资产列表
    pub async fn list_assets(
        &self,
        filter: Option<AssetFilter>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<Asset>, String> {
        self.dao.list_assets(filter, limit, offset).await
            .map_err(|e| format!("Database error: {}", e))
    }

    /// 获取资产统计信息
    pub async fn get_asset_stats(&self) -> Result<AssetStats, String> {
        self.dao.get_asset_stats().await
            .map_err(|e| format!("Database error: {}", e))
    }

    /// 创建资产关系
    pub async fn create_relationship(
        &self,
        source_asset_id: String,
        target_asset_id: String,
        relationship_type: RelationshipType,
        created_by: String,
    ) -> Result<AssetRelationship, String> {
        self.dao.create_relationship(source_asset_id, target_asset_id, relationship_type, created_by).await
            .map_err(|e| format!("Database error: {}", e))
    }

    /// 批量导入资产
    pub async fn import_assets(
        &self,
        request: ImportAssetsRequest,
        created_by: String,
    ) -> Result<ImportResult, String> {
        self.dao.import_assets(request, created_by).await
            .map_err(|e| format!("Database error: {}", e))
    }

    /// 从扫描结果中提取并创建资产
    pub async fn extract_assets_from_scan(
        &self,
        scan_id: &str,
        scan_results: &HashMap<String, serde_json::Value>,
        created_by: String,
    ) -> Result<Vec<Asset>, String> {
        let mut assets = Vec::new();

        // 从端口扫描结果中提取资产
        if let Some(port_scan_results) = scan_results.get("port_scan") {
            if let Some(open_ports) = port_scan_results.get("open_ports").and_then(|v| v.as_array()) {
                for port_info in open_ports {
                    if let (Some(ip), Some(port)) = (
                        port_info.get("ip").and_then(|v| v.as_str()),
                        port_info.get("port").and_then(|v| v.as_u64()),
                    ) {
                        // 创建IP资产
                        let ip_request = CreateAssetRequest {
                            project_id: None,
                            asset_type: AssetType::Ip,
                            name: format!("IP Address {}", ip),
                            value: ip.to_string(),
                            description: Some("IP address discovered during port scan".to_string()),
                            confidence: Some(1.0),
                            source: Some("port_scan".to_string()),
                            source_scan_id: Some(scan_id.to_string()),
                            metadata: Some({
                                let mut meta = HashMap::new();
                                meta.insert("scan_type".to_string(), serde_json::Value::String("port_scan".to_string()));
                                meta
                            }),
                            tags: Some(vec!["discovered".to_string(), "port_scan".to_string()]),
                            risk_level: Some(RiskLevel::Unknown),
                        };

                        // 检查IP是否已存在
                        if self.dao.find_asset_by_type_and_value(&AssetType::Ip, ip).await.map_err(|e| format!("Database error: {}", e))?.is_none() {
                            let ip_asset = self.dao.create_asset(ip_request, created_by.clone()).await
                                .map_err(|e| format!("Database error: {}", e))?;
                            assets.push(ip_asset.clone());

                            // 创建端口资产
                            let port_value = format!("{}:{}", ip, port);
                            let port_request = CreateAssetRequest {
                                project_id: None,
                                asset_type: AssetType::Port,
                                name: format!("Port {} on {}", port, ip),
                                value: port_value.clone(),
                                description: port_info.get("service").and_then(|v| v.as_str()).map(|s| format!("Service: {}", s)),
                                confidence: Some(1.0),
                                source: Some("port_scan".to_string()),
                                source_scan_id: Some(scan_id.to_string()),
                                metadata: Some({
                                    let mut meta = HashMap::new();
                                    meta.insert("port".to_string(), serde_json::Value::Number(serde_json::Number::from(port)));
                                    meta.insert("ip".to_string(), serde_json::Value::String(ip.to_string()));
                                    if let Some(service) = port_info.get("service").and_then(|v| v.as_str()) {
                                        meta.insert("service".to_string(), serde_json::Value::String(service.to_string()));
                                    }
                                    meta
                                }),
                                tags: Some(vec!["open_port".to_string(), "discovered".to_string()]),
                                risk_level: Some(self.assess_port_risk(port)),
                            };

                            if self.dao.find_asset_by_type_and_value(&AssetType::Port, &port_value).await.map_err(|e| format!("Database error: {}", e))?.is_none() {
                                let port_asset = self.dao.create_asset(port_request, created_by.clone()).await
                                    .map_err(|e| format!("Database error: {}", e))?;
                                assets.push(port_asset.clone());

                                // 创建IP和端口之间的关系
                                self.dao.create_relationship(
                                    ip_asset.id.clone(),
                                    port_asset.id,
                                    RelationshipType::Exposes,
                                    created_by.clone(),
                                ).await.map_err(|e| format!("Database error: {}", e))?;
                            }
                        }
                    }
                }
            }
        }

        // 从域名扫描结果中提取资产
        if let Some(domain_scan_results) = scan_results.get("domain_scan") {
            if let Some(domains) = domain_scan_results.get("domains").and_then(|v| v.as_array()) {
                for domain_info in domains {
                    if let Some(domain) = domain_info.get("domain").and_then(|v| v.as_str()) {
                        let domain_request = CreateAssetRequest {
                            project_id: None,
                            asset_type: if domain.starts_with("www.") || domain.contains('.') {
                                AssetType::Subdomain
                            } else {
                                AssetType::Domain
                            },
                            name: format!("Domain {}", domain),
                            value: domain.to_string(),
                            description: Some("Domain discovered during scan".to_string()),
                            confidence: Some(1.0),
                            source: Some("domain_scan".to_string()),
                            source_scan_id: Some(scan_id.to_string()),
                            metadata: Some({
                                let mut meta = HashMap::new();
                                meta.insert("scan_type".to_string(), serde_json::Value::String("domain_scan".to_string()));
                                if let Some(ip) = domain_info.get("ip").and_then(|v| v.as_str()) {
                                    meta.insert("resolved_ip".to_string(), serde_json::Value::String(ip.to_string()));
                                }
                                meta
                            }),
                            tags: Some(vec!["discovered".to_string(), "domain_scan".to_string()]),
                            risk_level: Some(RiskLevel::Unknown),
                        };

                        if self.dao.find_asset_by_type_and_value(&domain_request.asset_type, domain).await.map_err(|e| format!("Database error: {}", e))?.is_none() {
                            let domain_asset = self.dao.create_asset(domain_request, created_by.clone()).await
                                .map_err(|e| format!("Database error: {}", e))?;
                            assets.push(domain_asset);
                        }
                    }
                }
            }
        }

        // 从Web扫描结果中提取资产
        if let Some(web_scan_results) = scan_results.get("web_scan") {
            if let Some(websites) = web_scan_results.get("websites").and_then(|v| v.as_array()) {
                for website_info in websites {
                    if let Some(url) = website_info.get("url").and_then(|v| v.as_str()) {
                        let website_request = CreateAssetRequest {
                            project_id: None,
                            asset_type: AssetType::Website,
                            name: format!("Website {}", url),
                            value: url.to_string(),
                            description: website_info.get("title").and_then(|v| v.as_str()).map(|s| format!("Title: {}", s)),
                            confidence: Some(1.0),
                            source: Some("web_scan".to_string()),
                            source_scan_id: Some(scan_id.to_string()),
                            metadata: Some({
                                let mut meta = HashMap::new();
                                meta.insert("scan_type".to_string(), serde_json::Value::String("web_scan".to_string()));
                                if let Some(status) = website_info.get("status").and_then(|v| v.as_u64()) {
                                    meta.insert("status_code".to_string(), serde_json::Value::Number(serde_json::Number::from(status)));
                                }
                                if let Some(title) = website_info.get("title").and_then(|v| v.as_str()) {
                                    meta.insert("title".to_string(), serde_json::Value::String(title.to_string()));
                                }
                                meta
                            }),
                            tags: Some(vec!["website".to_string(), "discovered".to_string()]),
                            risk_level: Some(RiskLevel::Unknown),
                        };

                        if self.dao.find_asset_by_type_and_value(&AssetType::Website, url).await.map_err(|e| format!("Database error: {}", e))?.is_none() {
                            let website_asset = self.dao.create_asset(website_request, created_by.clone()).await
                                .map_err(|e| format!("Database error: {}", e))?;
                            assets.push(website_asset);
                        }
                    }
                }
            }
        }

        Ok(assets)
    }

    /// 评估端口风险等级
    fn assess_port_risk(&self, port: u64) -> RiskLevel {
        match port {
            // 高风险端口
            21 | 23 | 135 | 139 | 445 | 1433 | 1521 | 3306 | 3389 | 5432 | 5900 | 6379 => RiskLevel::High,
            // 中等风险端口
            22 | 25 | 53 | 110 | 143 | 993 | 995 | 1080 | 8080 | 8443 => RiskLevel::Medium,
            // 常见Web端口
            80 | 443 | 8000 | 8008 | 8888 => RiskLevel::Low,
            // 其他端口
            _ => RiskLevel::Unknown,
        }
    }

    /// 搜索资产
    pub async fn search_assets(
        &self,
        query: &str,
        asset_types: Option<Vec<AssetType>>,
        limit: Option<u32>,
    ) -> Result<Vec<Asset>, String> {
        let filter = AssetFilter {
            asset_types,
            statuses: None,
            risk_levels: None,
            sources: None,
            tags: None,
            search: Some(query.to_string()),
            created_after: None,
            created_before: None,
            last_seen_after: None,
            last_seen_before: None,
        };

        self.list_assets(Some(filter), limit, None).await
    }

    /// 获取资产的相关资产（通过关系）
    pub async fn get_related_assets(&self, asset_id: &str) -> Result<Vec<Asset>, String> {
        let (incoming, outgoing) = self.dao.get_asset_relationships(asset_id).await
            .map_err(|e| format!("Database error: {}", e))?;

        let mut related_asset_ids = Vec::new();
        for rel in incoming {
            related_asset_ids.push(rel.source_asset_id);
        }
        for rel in outgoing {
            related_asset_ids.push(rel.target_asset_id);
        }

        let mut related_assets = Vec::new();
        for asset_id in related_asset_ids {
            if let Some(asset) = self.dao.get_asset_by_id(&asset_id).await
                .map_err(|e| format!("Database error: {}", e))? {
                related_assets.push(asset);
            }
        }

        Ok(related_assets)
    }

    /// 标记资产为已验证
    pub async fn verify_asset(&self, asset_id: &str) -> Result<bool, String> {
        let update_request = UpdateAssetRequest {
            name: None,
            value: None,
            description: None,
            confidence: None,
            status: Some(AssetStatus::Verified),
            metadata: None,
            tags: None,
            risk_level: None,
            project_id: None,
        };

        self.update_asset(asset_id, update_request).await
    }

    /// 更新资产的最后发现时间
    pub async fn update_last_seen(&self, asset_id: &str) -> Result<bool, String> {
        if let Some(mut asset) = self.dao.get_asset_by_id(asset_id).await
            .map_err(|e| format!("Database error: {}", e))? {
            asset.update_last_seen();
            
            let update_request = UpdateAssetRequest {
                name: None,
                value: None,
                description: None,
                confidence: None,
                status: None,
                metadata: None,
                tags: None,
                risk_level: None,
                project_id: None,
            };
            
            self.dao.update_asset(asset_id, update_request).await
                .map_err(|e| format!("Database error: {}", e))
        } else {
            Ok(false)
        }
    }
}