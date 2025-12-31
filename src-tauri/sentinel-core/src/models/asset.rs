use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use uuid::Uuid;

/// 资产类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AssetType {
    #[serde(rename = "domain")]
    Domain,
    #[serde(rename = "subdomain")]
    Subdomain,
    #[serde(rename = "ip")]
    Ip,
    #[serde(rename = "port")]
    Port,
    #[serde(rename = "service")]
    Service,
    #[serde(rename = "website")]
    Website,
    #[serde(rename = "api")]
    Api,
    #[serde(rename = "certificate")]
    Certificate,
    #[serde(rename = "fingerprint")]
    Fingerprint,
    #[serde(rename = "vulnerability")]
    Vulnerability,
    #[serde(rename = "technology")]
    Technology,
    #[serde(rename = "email")]
    Email,
    #[serde(rename = "phone")]
    Phone,
    #[serde(rename = "file")]
    File,
    #[serde(rename = "directory")]
    Directory,
}

impl AssetType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AssetType::Domain => "domain",
            AssetType::Subdomain => "subdomain",
            AssetType::Ip => "ip",
            AssetType::Port => "port",
            AssetType::Service => "service",
            AssetType::Website => "website",
            AssetType::Api => "api",
            AssetType::Certificate => "certificate",
            AssetType::Fingerprint => "fingerprint",
            AssetType::Vulnerability => "vulnerability",
            AssetType::Technology => "technology",
            AssetType::Email => "email",
            AssetType::Phone => "phone",
            AssetType::File => "file",
            AssetType::Directory => "directory",
        }
    }
}

impl FromStr for AssetType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "domain" => Ok(AssetType::Domain),
            "subdomain" => Ok(AssetType::Subdomain),
            "ip" => Ok(AssetType::Ip),
            "port" => Ok(AssetType::Port),
            "service" => Ok(AssetType::Service),
            "website" => Ok(AssetType::Website),
            "api" => Ok(AssetType::Api),
            "certificate" => Ok(AssetType::Certificate),
            "fingerprint" => Ok(AssetType::Fingerprint),
            "vulnerability" => Ok(AssetType::Vulnerability),
            "technology" => Ok(AssetType::Technology),
            "email" => Ok(AssetType::Email),
            "phone" => Ok(AssetType::Phone),
            "file" => Ok(AssetType::File),
            "directory" => Ok(AssetType::Directory),
            _ => Err(()),
        }
    }
}

/// 资产状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AssetStatus {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "inactive")]
    Inactive,
    #[serde(rename = "verified")]
    Verified,
    #[serde(rename = "unverified")]
    Unverified,
}

impl AssetStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AssetStatus::Active => "active",
            AssetStatus::Inactive => "inactive",
            AssetStatus::Verified => "verified",
            AssetStatus::Unverified => "unverified",
        }
    }
}

/// 风险等级
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskLevel {
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "high")]
    High,
    #[serde(rename = "critical")]
    Critical,
    #[serde(rename = "unknown")]
    Unknown,
}

impl RiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskLevel::Low => "low",
            RiskLevel::Medium => "medium",
            RiskLevel::High => "high",
            RiskLevel::Critical => "critical",
            RiskLevel::Unknown => "unknown",
        }
    }
}

impl FromStr for RiskLevel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "low" => Ok(RiskLevel::Low),
            "medium" => Ok(RiskLevel::Medium),
            "high" => Ok(RiskLevel::High),
            "critical" => Ok(RiskLevel::Critical),
            _ => Ok(RiskLevel::Unknown),
        }
    }
}

/// 资产实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub project_id: Option<String>,
    pub asset_type: AssetType,
    pub name: String,
    pub value: String,
    pub description: Option<String>,
    pub confidence: f64,
    pub status: AssetStatus,
    pub source: Option<String>,
    pub source_scan_id: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub tags: Vec<String>,
    pub risk_level: RiskLevel,
    pub last_seen: DateTime<Utc>,
    pub first_seen: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: String,
}

impl Asset {
    pub fn new(
        asset_type: AssetType,
        name: String,
        value: String,
        created_by: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            project_id: None,
            asset_type,
            name,
            value,
            description: None,
            confidence: 1.0,
            status: AssetStatus::Active,
            source: None,
            source_scan_id: None,
            metadata: HashMap::new(),
            tags: Vec::new(),
            risk_level: RiskLevel::Unknown,
            last_seen: now,
            first_seen: now,
            created_at: now,
            updated_at: now,
            created_by,
        }
    }

    pub fn with_source(mut self, source: String, scan_id: Option<String>) -> Self {
        self.source = Some(source);
        self.source_scan_id = scan_id;
        self
    }

    pub fn with_metadata(mut self, metadata: HashMap<String, serde_json::Value>) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_risk_level(mut self, risk_level: RiskLevel) -> Self {
        self.risk_level = risk_level;
        self
    }

    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    pub fn remove_tag(&mut self, tag: &str) {
        self.tags.retain(|t| t != tag);
    }

    pub fn update_last_seen(&mut self) {
        self.last_seen = Utc::now();
        self.updated_at = Utc::now();
    }
}

/// 资产关系类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RelationshipType {
    #[serde(rename = "belongs_to")]
    BelongsTo,
    #[serde(rename = "contains")]
    Contains,
    #[serde(rename = "connects_to")]
    ConnectsTo,
    #[serde(rename = "depends_on")]
    DependsOn,
    #[serde(rename = "resolves_to")]
    ResolvesTo,
    #[serde(rename = "hosts")]
    Hosts,
    #[serde(rename = "uses")]
    Uses,
    #[serde(rename = "exposes")]
    Exposes,
}

impl RelationshipType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RelationshipType::BelongsTo => "belongs_to",
            RelationshipType::Contains => "contains",
            RelationshipType::ConnectsTo => "connects_to",
            RelationshipType::DependsOn => "depends_on",
            RelationshipType::ResolvesTo => "resolves_to",
            RelationshipType::Hosts => "hosts",
            RelationshipType::Uses => "uses",
            RelationshipType::Exposes => "exposes",
        }
    }
}

/// 资产关系
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetRelationship {
    pub id: String,
    pub source_asset_id: String,
    pub target_asset_id: String,
    pub relationship_type: RelationshipType,
    pub description: Option<String>,
    pub confidence: f64,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
}

impl AssetRelationship {
    pub fn new(
        source_asset_id: String,
        target_asset_id: String,
        relationship_type: RelationshipType,
        created_by: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            source_asset_id,
            target_asset_id,
            relationship_type,
            description: None,
            confidence: 1.0,
            metadata: HashMap::new(),
            created_at: Utc::now(),
            created_by,
        }
    }
}

/// 资产历史记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetHistory {
    pub id: String,
    pub asset_id: String,
    pub action: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub changed_fields: HashMap<String, serde_json::Value>,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
}

/// 资产标签
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetTag {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
}

/// 创建资产请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAssetRequest {
    pub project_id: Option<String>,
    pub asset_type: AssetType,
    pub name: String,
    pub value: String,
    pub description: Option<String>,
    pub confidence: Option<f64>,
    pub source: Option<String>,
    pub source_scan_id: Option<String>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    pub tags: Option<Vec<String>>,
    pub risk_level: Option<RiskLevel>,
}

/// 更新资产请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAssetRequest {
    pub project_id: Option<String>,
    pub name: Option<String>,
    pub value: Option<String>,
    pub description: Option<String>,
    pub confidence: Option<f64>,
    pub status: Option<AssetStatus>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    pub tags: Option<Vec<String>>,
    pub risk_level: Option<RiskLevel>,
}

/// 资产查询过滤器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetFilter {
    pub asset_types: Option<Vec<AssetType>>,
    pub statuses: Option<Vec<AssetStatus>>,
    pub risk_levels: Option<Vec<RiskLevel>>,
    pub sources: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub search: Option<String>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub last_seen_after: Option<DateTime<Utc>>,
    pub last_seen_before: Option<DateTime<Utc>>,
}

/// 资产统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetStats {
    pub total_assets: f64,
    pub by_type: HashMap<String, f64>,
    pub by_status: HashMap<String, f64>,
    pub by_risk_level: HashMap<String, f64>,
    pub by_source: HashMap<String, f64>,
    pub recent_additions: f64,
    pub stale_assets: f64,
}

/// 资产详情（包含关系信息）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetDetail {
    pub asset: Asset,
    pub incoming_relationships: Vec<AssetRelationship>,
    pub outgoing_relationships: Vec<AssetRelationship>,
    pub history: Vec<AssetHistory>,
}

/// 资产导入请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportAssetsRequest {
    pub assets: Vec<CreateAssetRequest>,
    pub source: String,
    pub source_scan_id: Option<String>,
    pub merge_strategy: MergeStrategy,
}

/// 合并策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MergeStrategy {
    #[serde(rename = "skip")]
    Skip,
    #[serde(rename = "update")]
    Update,
    #[serde(rename = "replace")]
    Replace,
}

/// 资产导入结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub total: usize,
    pub created: usize,
    pub updated: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

