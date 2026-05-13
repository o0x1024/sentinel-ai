use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use crate::database_service::sqlx_compat::{MySql, Postgres};
use crate::database_service::surface_asset_query::{
    push_surface_asset_filters, push_surface_asset_pagination,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::QueryBuilder;
use std::collections::HashMap;

const SURFACE_MARK_VIEWED_BATCH_SIZE: usize = 200;

/// Shared surface asset row used by all typed network mapping objects.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SurfaceAssetRow {
    pub id: String,
    pub program_id: String,
    pub asset_type: String,
    pub asset_name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub org_id: Option<String>,
    pub business_unit: Option<String>,
    pub project: Option<String>,
    pub owner: Option<String>,
    pub maintainer: Option<String>,
    pub contact: Option<String>,
    pub env: Option<String>,
    pub internet_exposure: Option<String>,
    pub criticality: Option<String>,
    pub data_level: Option<String>,
    pub source: Option<String>,
    pub first_seen_at: String,
    pub last_seen_at: String,
    pub last_verified_at: Option<String>,
    pub discovery_task_id: Option<String>,
    pub status: String,
    pub alive_status: Option<String>,
    pub confidence_score: Option<f64>,
    pub fingerprint_confidence: Option<f64>,
    pub risk_score: Option<f64>,
    pub risk_level: Option<String>,
    pub vulnerabilities_count: Option<i32>,
    pub weak_password_flag: Option<bool>,
    pub expired_cert_flag: Option<bool>,
    pub exposed_to_internet_flag: Option<bool>,
    pub viewed_at: Option<String>,
    pub viewed_by: Option<String>,
    pub is_favorite: bool,
    pub metadata_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SurfaceOrgAssetRow {
    pub asset_id: String,
    pub org_name: String,
    pub org_short_name: Option<String>,
    pub parent_org_id: Option<String>,
    pub business_line: Option<String>,
    pub importance_level: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SurfaceDomainAssetRow {
    pub asset_id: String,
    pub fqdn: String,
    pub main_domain: Option<String>,
    pub root_domain: Option<String>,
    pub subdomain_level: Option<i32>,
    pub record_type: Option<String>,
    pub record_value: Option<String>,
    pub ttl: Option<i32>,
    pub registrar: Option<String>,
    pub registered_at: Option<String>,
    pub expires_at: Option<String>,
    pub whois_json: Option<String>,
    pub wildcard_enabled: Option<bool>,
    pub dnssec_enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SurfaceIpAssetRow {
    pub asset_id: String,
    pub ip_address: String,
    pub ip_version: Option<String>,
    pub cidr: Option<String>,
    pub asn: Option<i32>,
    pub bgp_prefix: Option<String>,
    pub country: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
    pub isp: Option<String>,
    pub reverse_dns: Option<String>,
    pub cloud_provider: Option<String>,
    pub network_boundary_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SurfaceHostAssetRow {
    pub asset_id: String,
    pub hostname: String,
    pub fqdn: Option<String>,
    pub ip_addresses_json: Option<String>,
    pub operating_system: Option<String>,
    pub device_type: Option<String>,
    pub cloud_instance_id: Option<String>,
    pub region_or_datacenter: Option<String>,
    pub vpc_or_subnet: Option<String>,
    pub mac_address: Option<String>,
    pub agent_status: Option<String>,
    pub labels_json: Option<String>,
    pub lifecycle_status: Option<String>,
    pub last_online_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SurfacePortAssetRow {
    pub asset_id: String,
    pub host_asset_id: Option<String>,
    pub ip_address: Option<String>,
    pub port_number: i32,
    pub transport_protocol: String,
    pub port_state: Option<String>,
    pub scan_source: Option<String>,
    pub related_service_asset_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SurfaceServiceAssetRow {
    pub asset_id: String,
    pub host_asset_id: Option<String>,
    pub ip_address: Option<String>,
    pub port_number: Option<i32>,
    pub transport_protocol: Option<String>,
    pub protocol_name: Option<String>,
    pub application_service_name: Option<String>,
    pub banner: Option<String>,
    pub product_name: Option<String>,
    pub vendor: Option<String>,
    pub version: Option<String>,
    pub middleware_type: Option<String>,
    pub component_fingerprint: Option<String>,
    pub auth_type: Option<String>,
    pub login_required: Option<bool>,
    pub weak_password_risk: Option<bool>,
    pub encrypted_transport: Option<bool>,
    pub related_cves_json: Option<String>,
    pub related_domain_asset_id: Option<String>,
    pub system_asset_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SurfaceWebAssetRow {
    pub asset_id: String,
    pub service_asset_id: Option<String>,
    pub canonical_url: String,
    pub scheme: Option<String>,
    pub domain_asset_id: Option<String>,
    pub ip_address: Option<String>,
    pub port_number: Option<i32>,
    pub site_title: Option<String>,
    pub http_status_code: Option<i32>,
    pub server_header: Option<String>,
    pub response_headers_json: Option<String>,
    pub page_fingerprint: Option<String>,
    pub favicon_hash: Option<String>,
    pub screenshot_path: Option<String>,
    pub framework: Option<String>,
    pub cms: Option<String>,
    pub waf_flag: Option<bool>,
    pub cdn_flag: Option<bool>,
    pub login_flag: Option<bool>,
    pub api_flag: Option<bool>,
    pub openapi_url: Option<String>,
    pub business_type: Option<String>,
    pub language: Option<String>,
    pub filing_info: Option<String>,
    pub content_summary: Option<String>,
    pub sensitive_path_results_json: Option<String>,
    pub last_accessed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SurfaceCertAssetRow {
    pub asset_id: String,
    pub sha256: String,
    pub sha1: Option<String>,
    pub serial_number: Option<String>,
    pub public_key_algorithm: Option<String>,
    pub signature_algorithm: Option<String>,
    pub issuer: Option<String>,
    pub subject: Option<String>,
    pub san_list_json: Option<String>,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub self_signed_flag: Option<bool>,
    pub related_domains_json: Option<String>,
    pub related_ips_json: Option<String>,
    pub certificate_chain_json: Option<String>,
    pub risk_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SurfaceRelationRow {
    pub id: String,
    pub program_id: String,
    pub from_asset_id: String,
    pub to_asset_id: String,
    pub relation_type: String,
    pub source: Option<String>,
    pub confidence_score: Option<f64>,
    pub evidence_id: Option<String>,
    pub first_seen_at: String,
    pub last_seen_at: String,
    pub active: bool,
    pub metadata_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SurfaceFingerprintRow {
    pub id: String,
    pub program_id: String,
    pub asset_id: String,
    pub fingerprint_type: String,
    pub fingerprint_key: Option<String>,
    pub fingerprint_value: String,
    pub rule_id: Option<String>,
    pub rule_word: Option<String>,
    pub rule_name: Option<String>,
    pub normalized_product: Option<String>,
    pub normalized_vendor: Option<String>,
    pub normalized_category: Option<String>,
    pub normalized_family: Option<String>,
    pub version: Option<String>,
    pub is_primary: Option<bool>,
    pub match_source_part: Option<String>,
    pub confidence_score: Option<f64>,
    pub source: Option<String>,
    pub observed_at: String,
    pub metadata_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SurfaceEvidenceRow {
    pub id: String,
    pub program_id: String,
    pub asset_id: Option<String>,
    pub evidence_type: String,
    pub title: Option<String>,
    pub content_text: Option<String>,
    pub content_path: Option<String>,
    pub content_json: Option<String>,
    pub collected_at: String,
    pub collected_by: Option<String>,
    pub probe_node: Option<String>,
    pub metadata_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SurfaceChangeLogRow {
    pub id: String,
    pub program_id: String,
    pub asset_id: Option<String>,
    pub relation_id: Option<String>,
    pub change_type: String,
    pub old_value_json: Option<String>,
    pub new_value_json: Option<String>,
    pub summary: String,
    pub detected_at: String,
    pub source_run_id: Option<String>,
    pub risk_delta: Option<f64>,
    pub metadata_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SurfaceDiscoveryRunRow {
    pub id: String,
    pub program_id: String,
    pub trigger_source: String,
    pub schedule_id: Option<String>,
    pub workflow_id: Option<String>,
    pub workflow_template_id: Option<String>,
    pub plugin_id: Option<String>,
    pub status: String,
    pub seed_count: Option<i32>,
    pub observation_count: Option<i32>,
    pub imported_asset_count: Option<i32>,
    pub changed_asset_count: Option<i32>,
    pub error_message: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub metadata_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SurfaceObservationRow {
    pub id: String,
    pub run_id: String,
    pub program_id: String,
    pub artifact_type: String,
    pub object_key: Option<String>,
    pub payload_json: String,
    pub source_plugin: Option<String>,
    pub confidence_score: Option<f64>,
    pub observed_at: String,
    pub normalized: bool,
    pub metadata_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SurfaceSeedRow {
    pub id: String,
    pub program_id: String,
    pub seed_type: String,
    pub seed_value: String,
    pub status: String,
    pub source: Option<String>,
    pub confidence_score: Option<f64>,
    pub last_run_at: Option<String>,
    pub metadata_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SurfaceSeedCandidateRow {
    pub id: String,
    pub program_id: String,
    pub seed_type: String,
    pub seed_value: String,
    pub status: String,
    pub source_asset_id: String,
    pub source_asset_type: String,
    pub source_detail_key: String,
    pub source_display_value: String,
    pub source_canonical_url: Option<String>,
    pub confidence_score: Option<f64>,
    pub observed_at: String,
    pub reviewed_at: Option<String>,
    pub metadata_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SurfaceAssetFilter {
    pub program_id: Option<String>,
    pub asset_type: Option<String>,
    pub status: Option<String>,
    pub search: Option<String>,
    pub favicon_hash: Option<String>,
    pub has_favicon_hash: Option<bool>,
    pub http_status_code: Option<i32>,
    pub service_name: Option<String>,
    pub transport_protocol: Option<String>,
    pub view_state: Option<String>,
    pub is_favorite: Option<bool>,
    pub column_filters: Option<Vec<SurfaceAssetColumnFilter>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SurfaceAssetColumnFilter {
    pub key: String,
    pub operator: Option<String>,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SurfaceRelationFilter {
    pub program_id: Option<String>,
    pub asset_id: Option<String>,
    pub relation_type: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SurfaceOverview {
    pub total_assets: i32,
    pub active_assets: i32,
    pub high_risk_assets: i32,
    pub by_type: HashMap<String, i32>,
    pub total_relations: i32,
    pub recent_changes: i32,
    pub recent_runs: i32,
}

impl DatabaseService {
    pub async fn get_surface_assets_by_ids(
        &self,
        asset_ids: &[String],
    ) -> Result<Vec<SurfaceAssetRow>> {
        if asset_ids.is_empty() {
            return Ok(Vec::new());
        }

        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let rows: Vec<SurfaceAssetRow> = match runtime {
            DatabasePool::SQLite(pool) => {
                let mut query_builder =
                    QueryBuilder::<sqlx::Sqlite>::new("SELECT * FROM surface_assets WHERE id IN (");
                {
                    let mut separated = query_builder.separated(", ");
                    for asset_id in asset_ids {
                        separated.push_bind(asset_id.clone());
                    }
                }
                query_builder.push(") ORDER BY last_seen_at DESC, id DESC");
                query_builder
                    .build_query_as::<SurfaceAssetRow>()
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                let mut query_builder =
                    QueryBuilder::<MySql>::new("SELECT * FROM surface_assets WHERE id IN (");
                {
                    let mut separated = query_builder.separated(", ");
                    for asset_id in asset_ids {
                        separated.push_bind(asset_id.clone());
                    }
                }
                query_builder.push(") ORDER BY last_seen_at DESC, id DESC");
                query_builder
                    .build_query_as::<SurfaceAssetRow>()
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut query_builder =
                    QueryBuilder::<Postgres>::new("SELECT * FROM surface_assets WHERE id IN (");
                {
                    let mut separated = query_builder.separated(", ");
                    for asset_id in asset_ids {
                        separated.push_bind(asset_id.clone());
                    }
                }
                query_builder.push(") ORDER BY last_seen_at DESC, id DESC");
                query_builder
                    .build_query_as::<SurfaceAssetRow>()
                    .fetch_all(pool)
                    .await?
            }
        };

        Ok(rows)
    }

    pub async fn count_surface_assets(&self, filter: &SurfaceAssetFilter) -> Result<i64> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::SQLite(pool) => {
                let mut query_builder = QueryBuilder::<sqlx::Sqlite>::new(
                    "SELECT COUNT(*) FROM surface_assets WHERE 1=1",
                );
                push_surface_asset_filters(&mut query_builder, filter);
                Ok(query_builder
                    .build_query_scalar::<i64>()
                    .fetch_one(pool)
                    .await?)
            }
            DatabasePool::MySQL(pool) => {
                let mut query_builder =
                    QueryBuilder::<MySql>::new("SELECT COUNT(*) FROM surface_assets WHERE 1=1");
                push_surface_asset_filters(&mut query_builder, filter);
                Ok(query_builder
                    .build_query_scalar::<i64>()
                    .fetch_one(pool)
                    .await?)
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut query_builder =
                    QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM surface_assets WHERE 1=1");
                push_surface_asset_filters(&mut query_builder, filter);
                Ok(query_builder
                    .build_query_scalar::<i64>()
                    .fetch_one(pool)
                    .await?)
            }
        }
    }

    pub async fn get_surface_asset_by_identity(
        &self,
        program_id: &str,
        asset_type: &str,
        asset_name: &str,
    ) -> Result<Option<SurfaceAssetRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::SQLite(pool) => Ok(sqlx::query_as(
                "SELECT * FROM surface_assets WHERE program_id = ? AND asset_type = ? AND asset_name = ? LIMIT 1",
            )
            .bind(program_id)
            .bind(asset_type)
            .bind(asset_name)
            .fetch_optional(pool)
            .await?),
            DatabasePool::MySQL(pool) => Ok(sqlx::query_as(
                "SELECT * FROM surface_assets WHERE program_id = ? AND asset_type = ? AND asset_name = ? LIMIT 1",
            )
            .bind(program_id)
            .bind(asset_type)
            .bind(asset_name)
            .fetch_optional(pool)
            .await?),
            DatabasePool::PostgreSQL(pool) => Ok(sqlx::query_as(
                "SELECT * FROM surface_assets WHERE program_id = $1 AND asset_type = $2 AND asset_name = $3 LIMIT 1",
            )
            .bind(program_id)
            .bind(asset_type)
            .bind(asset_name)
            .fetch_optional(pool)
            .await?),
        }
    }

    pub async fn create_surface_asset(&self, asset: &SurfaceAssetRow) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO surface_assets (
                        id, program_id, asset_type, asset_name, display_name, description, org_id,
                        business_unit, project, owner, maintainer, contact, env, internet_exposure,
                        criticality, data_level, source, first_seen_at, last_seen_at, last_verified_at,
                        discovery_task_id, status, alive_status, confidence_score, fingerprint_confidence,
                        risk_score, risk_level, vulnerabilities_count, weak_password_flag,
                        expired_cert_flag, exposed_to_internet_flag, viewed_at, viewed_by,
                        metadata_json, created_at, updated_at, created_by, updated_by
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                    .bind(&asset.id)
                    .bind(&asset.program_id)
                    .bind(&asset.asset_type)
                    .bind(&asset.asset_name)
                    .bind(&asset.display_name)
                    .bind(&asset.description)
                    .bind(&asset.org_id)
                    .bind(&asset.business_unit)
                    .bind(&asset.project)
                    .bind(&asset.owner)
                    .bind(&asset.maintainer)
                    .bind(&asset.contact)
                    .bind(&asset.env)
                    .bind(&asset.internet_exposure)
                    .bind(&asset.criticality)
                    .bind(&asset.data_level)
                    .bind(&asset.source)
                    .bind(&asset.first_seen_at)
                    .bind(&asset.last_seen_at)
                    .bind(&asset.last_verified_at)
                    .bind(&asset.discovery_task_id)
                    .bind(&asset.status)
                    .bind(&asset.alive_status)
                    .bind(asset.confidence_score)
                    .bind(asset.fingerprint_confidence)
                    .bind(asset.risk_score)
                    .bind(&asset.risk_level)
                    .bind(asset.vulnerabilities_count)
                    .bind(asset.weak_password_flag)
                    .bind(asset.expired_cert_flag)
                    .bind(asset.exposed_to_internet_flag)
                    .bind(&asset.viewed_at)
                    .bind(&asset.viewed_by)
                    .bind(&asset.metadata_json)
                    .bind(&asset.created_at)
                    .bind(&asset.updated_at)
                    .bind(&asset.created_by)
                    .bind(&asset.updated_by)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO surface_assets (
                        id, program_id, asset_type, asset_name, display_name, description, org_id,
                        business_unit, project, owner, maintainer, contact, env, internet_exposure,
                        criticality, data_level, source, first_seen_at, last_seen_at, last_verified_at,
                        discovery_task_id, status, alive_status, confidence_score, fingerprint_confidence,
                        risk_score, risk_level, vulnerabilities_count, weak_password_flag,
                        expired_cert_flag, exposed_to_internet_flag, viewed_at, viewed_by,
                        metadata_json, created_at, updated_at, created_by, updated_by
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                    .bind(&asset.id)
                    .bind(&asset.program_id)
                    .bind(&asset.asset_type)
                    .bind(&asset.asset_name)
                    .bind(&asset.display_name)
                    .bind(&asset.description)
                    .bind(&asset.org_id)
                    .bind(&asset.business_unit)
                    .bind(&asset.project)
                    .bind(&asset.owner)
                    .bind(&asset.maintainer)
                    .bind(&asset.contact)
                    .bind(&asset.env)
                    .bind(&asset.internet_exposure)
                    .bind(&asset.criticality)
                    .bind(&asset.data_level)
                    .bind(&asset.source)
                    .bind(&asset.first_seen_at)
                    .bind(&asset.last_seen_at)
                    .bind(&asset.last_verified_at)
                    .bind(&asset.discovery_task_id)
                    .bind(&asset.status)
                    .bind(&asset.alive_status)
                    .bind(asset.confidence_score)
                    .bind(asset.fingerprint_confidence)
                    .bind(asset.risk_score)
                    .bind(&asset.risk_level)
                    .bind(asset.vulnerabilities_count)
                    .bind(asset.weak_password_flag)
                    .bind(asset.expired_cert_flag)
                    .bind(asset.exposed_to_internet_flag)
                    .bind(&asset.viewed_at)
                    .bind(&asset.viewed_by)
                    .bind(&asset.metadata_json)
                    .bind(&asset.created_at)
                    .bind(&asset.updated_at)
                    .bind(&asset.created_by)
                    .bind(&asset.updated_by)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO surface_assets (
                        id, program_id, asset_type, asset_name, display_name, description, org_id,
                        business_unit, project, owner, maintainer, contact, env, internet_exposure,
                        criticality, data_level, source, first_seen_at, last_seen_at, last_verified_at,
                        discovery_task_id, status, alive_status, confidence_score, fingerprint_confidence,
                        risk_score, risk_level, vulnerabilities_count, weak_password_flag,
                        expired_cert_flag, exposed_to_internet_flag, viewed_at, viewed_by,
                        metadata_json, created_at, updated_at, created_by, updated_by
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32, $33, $34, $35, $36, $37, $38)
                    "#,
                )
                .bind(&asset.id)
                .bind(&asset.program_id)
                .bind(&asset.asset_type)
                .bind(&asset.asset_name)
                .bind(&asset.display_name)
                .bind(&asset.description)
                .bind(&asset.org_id)
                .bind(&asset.business_unit)
                .bind(&asset.project)
                .bind(&asset.owner)
                .bind(&asset.maintainer)
                .bind(&asset.contact)
                .bind(&asset.env)
                .bind(&asset.internet_exposure)
                .bind(&asset.criticality)
                .bind(&asset.data_level)
                .bind(&asset.source)
                .bind(&asset.first_seen_at)
                .bind(&asset.last_seen_at)
                .bind(&asset.last_verified_at)
                .bind(&asset.discovery_task_id)
                .bind(&asset.status)
                .bind(&asset.alive_status)
                .bind(asset.confidence_score)
                .bind(asset.fingerprint_confidence)
                .bind(asset.risk_score)
                .bind(&asset.risk_level)
                .bind(asset.vulnerabilities_count)
                .bind(asset.weak_password_flag)
                .bind(asset.expired_cert_flag)
                .bind(asset.exposed_to_internet_flag)
                .bind(&asset.viewed_at)
                .bind(&asset.viewed_by)
                .bind(&asset.metadata_json)
                .bind(&asset.created_at)
                .bind(&asset.updated_at)
                .bind(&asset.created_by)
                .bind(&asset.updated_by)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn update_surface_asset(&self, asset: &SurfaceAssetRow) -> Result<bool> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let rows = match runtime {
            DatabasePool::SQLite(pool) => sqlx::query(
                r#"
                UPDATE surface_assets
                SET asset_name = ?, display_name = ?, description = ?, org_id = ?, business_unit = ?, project = ?,
                    owner = ?, maintainer = ?, contact = ?, env = ?, internet_exposure = ?,
                    criticality = ?, data_level = ?, source = ?, first_seen_at = ?, last_seen_at = ?,
                    last_verified_at = ?, discovery_task_id = ?, status = ?, alive_status = ?,
                    confidence_score = ?, fingerprint_confidence = ?, risk_score = ?, risk_level = ?,
                    vulnerabilities_count = ?, weak_password_flag = ?, expired_cert_flag = ?,
                    exposed_to_internet_flag = ?, viewed_at = ?, viewed_by = ?, metadata_json = ?,
                    updated_at = ?, created_by = ?, updated_by = ?
                WHERE id = ?
                "#,
            )
                .bind(&asset.asset_name)
                .bind(&asset.display_name)
                .bind(&asset.description)
                .bind(&asset.org_id)
                .bind(&asset.business_unit)
                .bind(&asset.project)
                .bind(&asset.owner)
                .bind(&asset.maintainer)
                .bind(&asset.contact)
                .bind(&asset.env)
                .bind(&asset.internet_exposure)
                .bind(&asset.criticality)
                .bind(&asset.data_level)
                .bind(&asset.source)
                .bind(&asset.first_seen_at)
                .bind(&asset.last_seen_at)
                .bind(&asset.last_verified_at)
                .bind(&asset.discovery_task_id)
                .bind(&asset.status)
                .bind(&asset.alive_status)
                .bind(asset.confidence_score)
                .bind(asset.fingerprint_confidence)
                .bind(asset.risk_score)
                .bind(&asset.risk_level)
                .bind(asset.vulnerabilities_count)
                .bind(asset.weak_password_flag)
                .bind(asset.expired_cert_flag)
                .bind(asset.exposed_to_internet_flag)
                .bind(&asset.viewed_at)
                .bind(&asset.viewed_by)
                .bind(&asset.metadata_json)
                .bind(&asset.updated_at)
                .bind(&asset.created_by)
                .bind(&asset.updated_by)
                .bind(&asset.id)
                .execute(pool)
                .await?
                .rows_affected(),
            DatabasePool::MySQL(pool) => sqlx::query(
                r#"
                UPDATE surface_assets
                SET asset_name = ?, display_name = ?, description = ?, org_id = ?, business_unit = ?, project = ?,
                    owner = ?, maintainer = ?, contact = ?, env = ?, internet_exposure = ?,
                    criticality = ?, data_level = ?, source = ?, first_seen_at = ?, last_seen_at = ?,
                    last_verified_at = ?, discovery_task_id = ?, status = ?, alive_status = ?,
                    confidence_score = ?, fingerprint_confidence = ?, risk_score = ?, risk_level = ?,
                    vulnerabilities_count = ?, weak_password_flag = ?, expired_cert_flag = ?,
                    exposed_to_internet_flag = ?, viewed_at = ?, viewed_by = ?, metadata_json = ?,
                    updated_at = ?, created_by = ?, updated_by = ?
                WHERE id = ?
                "#,
            )
                .bind(&asset.asset_name)
                .bind(&asset.display_name)
                .bind(&asset.description)
                .bind(&asset.org_id)
                .bind(&asset.business_unit)
                .bind(&asset.project)
                .bind(&asset.owner)
                .bind(&asset.maintainer)
                .bind(&asset.contact)
                .bind(&asset.env)
                .bind(&asset.internet_exposure)
                .bind(&asset.criticality)
                .bind(&asset.data_level)
                .bind(&asset.source)
                .bind(&asset.first_seen_at)
                .bind(&asset.last_seen_at)
                .bind(&asset.last_verified_at)
                .bind(&asset.discovery_task_id)
                .bind(&asset.status)
                .bind(&asset.alive_status)
                .bind(asset.confidence_score)
                .bind(asset.fingerprint_confidence)
                .bind(asset.risk_score)
                .bind(&asset.risk_level)
                .bind(asset.vulnerabilities_count)
                .bind(asset.weak_password_flag)
                .bind(asset.expired_cert_flag)
                .bind(asset.exposed_to_internet_flag)
                .bind(&asset.viewed_at)
                .bind(&asset.viewed_by)
                .bind(&asset.metadata_json)
                .bind(&asset.updated_at)
                .bind(&asset.created_by)
                .bind(&asset.updated_by)
                .bind(&asset.id)
                .execute(pool)
                .await?
                .rows_affected(),
            DatabasePool::PostgreSQL(pool) => sqlx::query(
                r#"
                UPDATE surface_assets
                SET asset_name = $1, display_name = $2, description = $3, org_id = $4, business_unit = $5, project = $6,
                    owner = $7, maintainer = $8, contact = $9, env = $10, internet_exposure = $11,
                    criticality = $12, data_level = $13, source = $14, first_seen_at = $15, last_seen_at = $16,
                    last_verified_at = $17, discovery_task_id = $18, status = $19, alive_status = $20,
                    confidence_score = $21, fingerprint_confidence = $22, risk_score = $23, risk_level = $24,
                    vulnerabilities_count = $25, weak_password_flag = $26, expired_cert_flag = $27,
                    exposed_to_internet_flag = $28, viewed_at = $29, viewed_by = $30,
                    metadata_json = $31, updated_at = $32, created_by = $33, updated_by = $34
                WHERE id = $35
                "#,
            )
                .bind(&asset.asset_name)
                .bind(&asset.display_name)
                .bind(&asset.description)
                .bind(&asset.org_id)
                .bind(&asset.business_unit)
                .bind(&asset.project)
                .bind(&asset.owner)
                .bind(&asset.maintainer)
                .bind(&asset.contact)
                .bind(&asset.env)
                .bind(&asset.internet_exposure)
                .bind(&asset.criticality)
                .bind(&asset.data_level)
                .bind(&asset.source)
                .bind(&asset.first_seen_at)
                .bind(&asset.last_seen_at)
                .bind(&asset.last_verified_at)
                .bind(&asset.discovery_task_id)
                .bind(&asset.status)
                .bind(&asset.alive_status)
                .bind(asset.confidence_score)
                .bind(asset.fingerprint_confidence)
                .bind(asset.risk_score)
                .bind(&asset.risk_level)
                .bind(asset.vulnerabilities_count)
                .bind(asset.weak_password_flag)
                .bind(asset.expired_cert_flag)
                .bind(asset.exposed_to_internet_flag)
                .bind(&asset.viewed_at)
                .bind(&asset.viewed_by)
                .bind(&asset.metadata_json)
                .bind(&asset.updated_at)
                .bind(&asset.created_by)
                .bind(&asset.updated_by)
                .bind(&asset.id)
                .execute(pool)
                .await?
                .rows_affected(),
        };

        Ok(rows > 0)
    }

    pub async fn mark_surface_assets_viewed(
        &self,
        asset_ids: &[String],
        viewed_at: &str,
        viewed_by: &str,
    ) -> Result<usize> {
        if asset_ids.is_empty() {
            return Ok(0);
        }

        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let rows = match runtime {
            DatabasePool::SQLite(pool) => {
                let mut tx = pool.begin().await?;
                let mut updated = 0u64;
                for batch in asset_ids.chunks(SURFACE_MARK_VIEWED_BATCH_SIZE) {
                    let mut query_builder =
                        QueryBuilder::<sqlx::Sqlite>::new("UPDATE surface_assets SET viewed_at = ");
                    query_builder
                        .push_bind(viewed_at.to_string())
                        .push(", viewed_by = ")
                        .push_bind(viewed_by.to_string())
                        .push(" WHERE viewed_at IS NULL AND id IN (");
                    {
                        let mut separated = query_builder.separated(", ");
                        for asset_id in batch {
                            separated.push_bind(asset_id.clone());
                        }
                    }
                    query_builder.push(")");
                    updated += query_builder
                        .build()
                        .execute(&mut *tx)
                        .await?
                        .rows_affected();
                }
                tx.commit().await?;
                updated
            }
            DatabasePool::MySQL(pool) => {
                let mut tx = pool.begin().await?;
                let mut updated = 0u64;
                for batch in asset_ids.chunks(SURFACE_MARK_VIEWED_BATCH_SIZE) {
                    let mut query_builder =
                        QueryBuilder::<MySql>::new("UPDATE surface_assets SET viewed_at = ");
                    query_builder
                        .push_bind(viewed_at.to_string())
                        .push(", viewed_by = ")
                        .push_bind(viewed_by.to_string())
                        .push(" WHERE viewed_at IS NULL AND id IN (");
                    {
                        let mut separated = query_builder.separated(", ");
                        for asset_id in batch {
                            separated.push_bind(asset_id.clone());
                        }
                    }
                    query_builder.push(")");
                    updated += query_builder
                        .build()
                        .execute(&mut *tx)
                        .await?
                        .rows_affected();
                }
                tx.commit().await?;
                updated
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut tx = pool.begin().await?;
                let mut updated = 0u64;
                for batch in asset_ids.chunks(SURFACE_MARK_VIEWED_BATCH_SIZE) {
                    let mut query_builder =
                        QueryBuilder::<Postgres>::new("UPDATE surface_assets SET viewed_at = ");
                    query_builder
                        .push_bind(viewed_at.to_string())
                        .push(", viewed_by = ")
                        .push_bind(viewed_by.to_string())
                        .push(" WHERE viewed_at IS NULL AND id IN (");
                    {
                        let mut separated = query_builder.separated(", ");
                        for asset_id in batch {
                            separated.push_bind(asset_id.clone());
                        }
                    }
                    query_builder.push(")");
                    updated += query_builder
                        .build()
                        .execute(&mut *tx)
                        .await?
                        .rows_affected();
                }
                tx.commit().await?;
                updated
            }
        };

        Ok(rows as usize)
    }

    pub async fn mark_surface_assets_new(&self, asset_ids: &[String]) -> Result<usize> {
        if asset_ids.is_empty() {
            return Ok(0);
        }

        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let rows = match runtime {
            DatabasePool::SQLite(pool) => {
                let mut tx = pool.begin().await?;
                let mut updated = 0u64;
                for batch in asset_ids.chunks(SURFACE_MARK_VIEWED_BATCH_SIZE) {
                    let mut query_builder = QueryBuilder::<sqlx::Sqlite>::new(
                        "UPDATE surface_assets SET viewed_at = NULL, viewed_by = NULL WHERE id IN (",
                    );
                    {
                        let mut separated = query_builder.separated(", ");
                        for asset_id in batch {
                            separated.push_bind(asset_id.clone());
                        }
                    }
                    query_builder.push(")");
                    updated += query_builder
                        .build()
                        .execute(&mut *tx)
                        .await?
                        .rows_affected();
                }
                tx.commit().await?;
                updated
            }
            DatabasePool::MySQL(pool) => {
                let mut tx = pool.begin().await?;
                let mut updated = 0u64;
                for batch in asset_ids.chunks(SURFACE_MARK_VIEWED_BATCH_SIZE) {
                    let mut query_builder = QueryBuilder::<MySql>::new(
                        "UPDATE surface_assets SET viewed_at = NULL, viewed_by = NULL WHERE id IN (",
                    );
                    {
                        let mut separated = query_builder.separated(", ");
                        for asset_id in batch {
                            separated.push_bind(asset_id.clone());
                        }
                    }
                    query_builder.push(")");
                    updated += query_builder
                        .build()
                        .execute(&mut *tx)
                        .await?
                        .rows_affected();
                }
                tx.commit().await?;
                updated
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut tx = pool.begin().await?;
                let mut updated = 0u64;
                for batch in asset_ids.chunks(SURFACE_MARK_VIEWED_BATCH_SIZE) {
                    let mut query_builder = QueryBuilder::<Postgres>::new(
                        "UPDATE surface_assets SET viewed_at = NULL, viewed_by = NULL WHERE id IN (",
                    );
                    {
                        let mut separated = query_builder.separated(", ");
                        for asset_id in batch {
                            separated.push_bind(asset_id.clone());
                        }
                    }
                    query_builder.push(")");
                    updated += query_builder
                        .build()
                        .execute(&mut *tx)
                        .await?
                        .rows_affected();
                }
                tx.commit().await?;
                updated
            }
        };

        Ok(rows as usize)
    }

    pub async fn mark_surface_inventory_viewed(
        &self,
        filter: &SurfaceAssetFilter,
        viewed_at: &str,
        viewed_by: &str,
    ) -> Result<usize> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let view_filter = SurfaceAssetFilter {
            program_id: filter.program_id.clone(),
            asset_type: filter.asset_type.clone(),
            status: filter.status.clone(),
            search: filter.search.clone(),
            favicon_hash: filter.favicon_hash.clone(),
            has_favicon_hash: filter.has_favicon_hash,
            http_status_code: filter.http_status_code,
            service_name: filter.service_name.clone(),
            transport_protocol: filter.transport_protocol.clone(),
            view_state: filter.view_state.clone(),
            is_favorite: filter.is_favorite,
            column_filters: filter.column_filters.clone(),
            limit: None,
            offset: None,
        };

        let rows = match runtime {
            DatabasePool::SQLite(pool) => {
                let mut query_builder =
                    QueryBuilder::<sqlx::Sqlite>::new("UPDATE surface_assets SET viewed_at = ");
                query_builder
                    .push_bind(viewed_at.to_string())
                    .push(", viewed_by = ")
                    .push_bind(viewed_by.to_string())
                    .push(" WHERE viewed_at IS NULL");
                push_surface_asset_filters(&mut query_builder, &view_filter);
                query_builder.build().execute(pool).await?.rows_affected()
            }
            DatabasePool::MySQL(pool) => {
                let mut query_builder =
                    QueryBuilder::<MySql>::new("UPDATE surface_assets SET viewed_at = ");
                query_builder
                    .push_bind(viewed_at.to_string())
                    .push(", viewed_by = ")
                    .push_bind(viewed_by.to_string())
                    .push(" WHERE viewed_at IS NULL");
                push_surface_asset_filters(&mut query_builder, &view_filter);
                query_builder.build().execute(pool).await?.rows_affected()
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut query_builder =
                    QueryBuilder::<Postgres>::new("UPDATE surface_assets SET viewed_at = ");
                query_builder
                    .push_bind(viewed_at.to_string())
                    .push(", viewed_by = ")
                    .push_bind(viewed_by.to_string())
                    .push(" WHERE viewed_at IS NULL");
                push_surface_asset_filters(&mut query_builder, &view_filter);
                query_builder.build().execute(pool).await?.rows_affected()
            }
        };

        Ok(rows as usize)
    }

    pub async fn set_surface_asset_favorite(
        &self,
        asset_id: &str,
        is_favorite: bool,
        updated_at: &str,
        updated_by: &str,
    ) -> Result<bool> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let rows = match runtime {
            DatabasePool::SQLite(pool) => sqlx::query(
                "UPDATE surface_assets SET is_favorite = ?, updated_at = ?, updated_by = ? WHERE id = ?",
            )
            .bind(is_favorite)
            .bind(updated_at)
            .bind(updated_by)
            .bind(asset_id)
            .execute(pool)
            .await?
            .rows_affected(),
            DatabasePool::MySQL(pool) => sqlx::query(
                "UPDATE surface_assets SET is_favorite = ?, updated_at = ?, updated_by = ? WHERE id = ?",
            )
            .bind(is_favorite)
            .bind(updated_at)
            .bind(updated_by)
            .bind(asset_id)
            .execute(pool)
            .await?
            .rows_affected(),
            DatabasePool::PostgreSQL(pool) => sqlx::query(
                "UPDATE surface_assets SET is_favorite = $1, updated_at = $2, updated_by = $3 WHERE id = $4",
            )
            .bind(is_favorite)
            .bind(updated_at)
            .bind(updated_by)
            .bind(asset_id)
            .execute(pool)
            .await?
            .rows_affected(),
        };

        Ok(rows > 0)
    }

    pub async fn upsert_surface_asset(&self, asset: &SurfaceAssetRow) -> Result<SurfaceAssetRow> {
        if let Some(mut existing) = self
            .get_surface_asset_by_identity(&asset.program_id, &asset.asset_type, &asset.asset_name)
            .await?
        {
            existing.display_name = asset.display_name.clone().or(existing.display_name);
            existing.description = asset.description.clone().or(existing.description);
            existing.org_id = asset.org_id.clone().or(existing.org_id);
            existing.business_unit = asset.business_unit.clone().or(existing.business_unit);
            existing.project = asset.project.clone().or(existing.project);
            existing.owner = asset.owner.clone().or(existing.owner);
            existing.maintainer = asset.maintainer.clone().or(existing.maintainer);
            existing.contact = asset.contact.clone().or(existing.contact);
            existing.env = asset.env.clone().or(existing.env);
            existing.internet_exposure = asset
                .internet_exposure
                .clone()
                .or(existing.internet_exposure);
            existing.criticality = asset.criticality.clone().or(existing.criticality);
            existing.data_level = asset.data_level.clone().or(existing.data_level);
            existing.source = asset.source.clone().or(existing.source);
            existing.last_seen_at = asset.last_seen_at.clone();
            existing.last_verified_at =
                asset.last_verified_at.clone().or(existing.last_verified_at);
            existing.discovery_task_id = asset
                .discovery_task_id
                .clone()
                .or(existing.discovery_task_id);
            existing.status = asset.status.clone();
            existing.alive_status = asset.alive_status.clone().or(existing.alive_status);
            existing.confidence_score = asset.confidence_score.or(existing.confidence_score);
            existing.fingerprint_confidence = asset
                .fingerprint_confidence
                .or(existing.fingerprint_confidence);
            existing.risk_score = asset.risk_score.or(existing.risk_score);
            existing.risk_level = asset.risk_level.clone().or(existing.risk_level);
            existing.vulnerabilities_count = asset
                .vulnerabilities_count
                .or(existing.vulnerabilities_count);
            existing.weak_password_flag = asset.weak_password_flag.or(existing.weak_password_flag);
            existing.expired_cert_flag = asset.expired_cert_flag.or(existing.expired_cert_flag);
            existing.exposed_to_internet_flag = asset
                .exposed_to_internet_flag
                .or(existing.exposed_to_internet_flag);
            existing.metadata_json = asset.metadata_json.clone().or(existing.metadata_json);
            existing.updated_at = asset.updated_at.clone();
            existing.updated_by = asset.updated_by.clone().or(existing.updated_by);
            self.update_surface_asset(&existing).await?;
            return Ok(existing);
        }

        self.create_surface_asset(asset).await?;
        Ok(asset.clone())
    }

    pub async fn create_surface_relation_if_missing(
        &self,
        relation: &SurfaceRelationRow,
    ) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let sql = r#"
            INSERT INTO surface_relations (
                id, program_id, from_asset_id, to_asset_id, relation_type, source, confidence_score,
                evidence_id, first_seen_at, last_seen_at, active, metadata_json, created_at, updated_at
            )
            SELECT ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?
            WHERE NOT EXISTS (
                SELECT 1 FROM surface_relations
                WHERE program_id = ? AND from_asset_id = ? AND to_asset_id = ? AND relation_type = ?
            )
        "#;

        match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query(sql)
                    .bind(&relation.id)
                    .bind(&relation.program_id)
                    .bind(&relation.from_asset_id)
                    .bind(&relation.to_asset_id)
                    .bind(&relation.relation_type)
                    .bind(&relation.source)
                    .bind(relation.confidence_score)
                    .bind(&relation.evidence_id)
                    .bind(&relation.first_seen_at)
                    .bind(&relation.last_seen_at)
                    .bind(relation.active)
                    .bind(&relation.metadata_json)
                    .bind(&relation.created_at)
                    .bind(&relation.updated_at)
                    .bind(&relation.program_id)
                    .bind(&relation.from_asset_id)
                    .bind(&relation.to_asset_id)
                    .bind(&relation.relation_type)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(sql)
                    .bind(&relation.id)
                    .bind(&relation.program_id)
                    .bind(&relation.from_asset_id)
                    .bind(&relation.to_asset_id)
                    .bind(&relation.relation_type)
                    .bind(&relation.source)
                    .bind(relation.confidence_score)
                    .bind(&relation.evidence_id)
                    .bind(&relation.first_seen_at)
                    .bind(&relation.last_seen_at)
                    .bind(relation.active)
                    .bind(&relation.metadata_json)
                    .bind(&relation.created_at)
                    .bind(&relation.updated_at)
                    .bind(&relation.program_id)
                    .bind(&relation.from_asset_id)
                    .bind(&relation.to_asset_id)
                    .bind(&relation.relation_type)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO surface_relations (
                        id, program_id, from_asset_id, to_asset_id, relation_type, source, confidence_score,
                        evidence_id, first_seen_at, last_seen_at, active, metadata_json, created_at, updated_at
                    )
                    SELECT $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14
                    WHERE NOT EXISTS (
                        SELECT 1 FROM surface_relations
                        WHERE program_id = $15 AND from_asset_id = $16 AND to_asset_id = $17 AND relation_type = $18
                    )
                    "#,
                )
                .bind(&relation.id)
                .bind(&relation.program_id)
                .bind(&relation.from_asset_id)
                .bind(&relation.to_asset_id)
                .bind(&relation.relation_type)
                .bind(&relation.source)
                .bind(relation.confidence_score)
                .bind(&relation.evidence_id)
                .bind(&relation.first_seen_at)
                .bind(&relation.last_seen_at)
                .bind(relation.active)
                .bind(&relation.metadata_json)
                .bind(&relation.created_at)
                .bind(&relation.updated_at)
                .bind(&relation.program_id)
                .bind(&relation.from_asset_id)
                .bind(&relation.to_asset_id)
                .bind(&relation.relation_type)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn create_surface_discovery_run(&self, run: &SurfaceDiscoveryRunRow) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let sql = r#"
            INSERT INTO surface_discovery_runs (
                id, program_id, trigger_source, schedule_id, workflow_id, workflow_template_id,
                plugin_id, status, seed_count, observation_count, imported_asset_count,
                changed_asset_count, error_message, started_at, completed_at, metadata_json
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query(sql)
                    .bind(&run.id)
                    .bind(&run.program_id)
                    .bind(&run.trigger_source)
                    .bind(&run.schedule_id)
                    .bind(&run.workflow_id)
                    .bind(&run.workflow_template_id)
                    .bind(&run.plugin_id)
                    .bind(&run.status)
                    .bind(run.seed_count)
                    .bind(run.observation_count)
                    .bind(run.imported_asset_count)
                    .bind(run.changed_asset_count)
                    .bind(&run.error_message)
                    .bind(&run.started_at)
                    .bind(&run.completed_at)
                    .bind(&run.metadata_json)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(sql)
                    .bind(&run.id)
                    .bind(&run.program_id)
                    .bind(&run.trigger_source)
                    .bind(&run.schedule_id)
                    .bind(&run.workflow_id)
                    .bind(&run.workflow_template_id)
                    .bind(&run.plugin_id)
                    .bind(&run.status)
                    .bind(run.seed_count)
                    .bind(run.observation_count)
                    .bind(run.imported_asset_count)
                    .bind(run.changed_asset_count)
                    .bind(&run.error_message)
                    .bind(&run.started_at)
                    .bind(&run.completed_at)
                    .bind(&run.metadata_json)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO surface_discovery_runs (
                        id, program_id, trigger_source, schedule_id, workflow_id, workflow_template_id,
                        plugin_id, status, seed_count, observation_count, imported_asset_count,
                        changed_asset_count, error_message, started_at, completed_at, metadata_json
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
                    "#,
                )
                .bind(&run.id)
                .bind(&run.program_id)
                .bind(&run.trigger_source)
                .bind(&run.schedule_id)
                .bind(&run.workflow_id)
                .bind(&run.workflow_template_id)
                .bind(&run.plugin_id)
                .bind(&run.status)
                .bind(run.seed_count)
                .bind(run.observation_count)
                .bind(run.imported_asset_count)
                .bind(run.changed_asset_count)
                .bind(&run.error_message)
                .bind(&run.started_at)
                .bind(&run.completed_at)
                .bind(&run.metadata_json)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn update_surface_discovery_run(
        &self,
        run_id: &str,
        status: &str,
        observation_count: Option<i32>,
        imported_asset_count: Option<i32>,
        changed_asset_count: Option<i32>,
        error_message: Option<&str>,
        completed_at: Option<&str>,
    ) -> Result<bool> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let sql = r#"
            UPDATE surface_discovery_runs
            SET status = ?, observation_count = COALESCE(?, observation_count),
                imported_asset_count = COALESCE(?, imported_asset_count),
                changed_asset_count = COALESCE(?, changed_asset_count),
                error_message = ?, completed_at = ?
            WHERE id = ?
        "#;

        let rows = match runtime {
            DatabasePool::SQLite(pool) => sqlx::query(sql)
                .bind(status)
                .bind(observation_count)
                .bind(imported_asset_count)
                .bind(changed_asset_count)
                .bind(error_message)
                .bind(completed_at)
                .bind(run_id)
                .execute(pool)
                .await?
                .rows_affected(),
            DatabasePool::MySQL(pool) => sqlx::query(sql)
                .bind(status)
                .bind(observation_count)
                .bind(imported_asset_count)
                .bind(changed_asset_count)
                .bind(error_message)
                .bind(completed_at)
                .bind(run_id)
                .execute(pool)
                .await?
                .rows_affected(),
            DatabasePool::PostgreSQL(pool) => sqlx::query(
                r#"
                    UPDATE surface_discovery_runs
                    SET status = $1, observation_count = COALESCE($2, observation_count),
                        imported_asset_count = COALESCE($3, imported_asset_count),
                        changed_asset_count = COALESCE($4, changed_asset_count),
                        error_message = $5, completed_at = $6
                    WHERE id = $7
                    "#,
            )
            .bind(status)
            .bind(observation_count)
            .bind(imported_asset_count)
            .bind(changed_asset_count)
            .bind(error_message)
            .bind(completed_at)
            .bind(run_id)
            .execute(pool)
            .await?
            .rows_affected(),
        };

        Ok(rows > 0)
    }

    pub async fn create_surface_observation(
        &self,
        observation: &SurfaceObservationRow,
    ) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let sql = r#"
            INSERT INTO surface_observations (
                id, run_id, program_id, artifact_type, object_key, payload_json,
                source_plugin, confidence_score, observed_at, normalized, metadata_json
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query(sql)
                    .bind(&observation.id)
                    .bind(&observation.run_id)
                    .bind(&observation.program_id)
                    .bind(&observation.artifact_type)
                    .bind(&observation.object_key)
                    .bind(&observation.payload_json)
                    .bind(&observation.source_plugin)
                    .bind(observation.confidence_score)
                    .bind(&observation.observed_at)
                    .bind(observation.normalized)
                    .bind(&observation.metadata_json)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(sql)
                    .bind(&observation.id)
                    .bind(&observation.run_id)
                    .bind(&observation.program_id)
                    .bind(&observation.artifact_type)
                    .bind(&observation.object_key)
                    .bind(&observation.payload_json)
                    .bind(&observation.source_plugin)
                    .bind(observation.confidence_score)
                    .bind(&observation.observed_at)
                    .bind(observation.normalized)
                    .bind(&observation.metadata_json)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO surface_observations (
                        id, run_id, program_id, artifact_type, object_key, payload_json,
                        source_plugin, confidence_score, observed_at, normalized, metadata_json
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                    "#,
                )
                .bind(&observation.id)
                .bind(&observation.run_id)
                .bind(&observation.program_id)
                .bind(&observation.artifact_type)
                .bind(&observation.object_key)
                .bind(&observation.payload_json)
                .bind(&observation.source_plugin)
                .bind(observation.confidence_score)
                .bind(&observation.observed_at)
                .bind(observation.normalized)
                .bind(&observation.metadata_json)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn create_surface_observations(
        &self,
        observations: &[SurfaceObservationRow],
    ) -> Result<()> {
        if observations.is_empty() {
            return Ok(());
        }

        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        const OBSERVATION_BATCH_SIZE: usize = 80;

        match runtime {
            DatabasePool::SQLite(pool) => {
                let mut tx = pool.begin().await?;
                for batch in observations.chunks(OBSERVATION_BATCH_SIZE) {
                    let mut query_builder = QueryBuilder::<sqlx::Sqlite>::new(
                        "INSERT INTO surface_observations (id, run_id, program_id, artifact_type, object_key, payload_json, source_plugin, confidence_score, observed_at, normalized, metadata_json) ",
                    );
                    query_builder.push_values(batch, |mut row, observation| {
                        row.push_bind(&observation.id)
                            .push_bind(&observation.run_id)
                            .push_bind(&observation.program_id)
                            .push_bind(&observation.artifact_type)
                            .push_bind(&observation.object_key)
                            .push_bind(&observation.payload_json)
                            .push_bind(&observation.source_plugin)
                            .push_bind(observation.confidence_score)
                            .push_bind(&observation.observed_at)
                            .push_bind(observation.normalized)
                            .push_bind(&observation.metadata_json);
                    });
                    query_builder.build().execute(&mut *tx).await?;
                }
                tx.commit().await?;
            }
            DatabasePool::MySQL(pool) => {
                let mut tx = pool.begin().await?;
                for batch in observations.chunks(OBSERVATION_BATCH_SIZE) {
                    let mut query_builder = QueryBuilder::<MySql>::new(
                        "INSERT INTO surface_observations (id, run_id, program_id, artifact_type, object_key, payload_json, source_plugin, confidence_score, observed_at, normalized, metadata_json) ",
                    );
                    query_builder.push_values(batch, |mut row, observation| {
                        row.push_bind(&observation.id)
                            .push_bind(&observation.run_id)
                            .push_bind(&observation.program_id)
                            .push_bind(&observation.artifact_type)
                            .push_bind(&observation.object_key)
                            .push_bind(&observation.payload_json)
                            .push_bind(&observation.source_plugin)
                            .push_bind(observation.confidence_score)
                            .push_bind(&observation.observed_at)
                            .push_bind(observation.normalized)
                            .push_bind(&observation.metadata_json);
                    });
                    query_builder.build().execute(&mut *tx).await?;
                }
                tx.commit().await?;
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut tx = pool.begin().await?;
                for batch in observations.chunks(OBSERVATION_BATCH_SIZE) {
                    let mut query_builder = QueryBuilder::<Postgres>::new(
                        "INSERT INTO surface_observations (id, run_id, program_id, artifact_type, object_key, payload_json, source_plugin, confidence_score, observed_at, normalized, metadata_json) ",
                    );
                    query_builder.push_values(batch, |mut row, observation| {
                        row.push_bind(&observation.id)
                            .push_bind(&observation.run_id)
                            .push_bind(&observation.program_id)
                            .push_bind(&observation.artifact_type)
                            .push_bind(&observation.object_key)
                            .push_bind(&observation.payload_json)
                            .push_bind(&observation.source_plugin)
                            .push_bind(observation.confidence_score)
                            .push_bind(&observation.observed_at)
                            .push_bind(observation.normalized)
                            .push_bind(&observation.metadata_json);
                    });
                    query_builder.build().execute(&mut *tx).await?;
                }
                tx.commit().await?;
            }
        }

        Ok(())
    }

    pub async fn list_surface_assets(
        &self,
        filter: &SurfaceAssetFilter,
    ) -> Result<Vec<SurfaceAssetRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let rows: Vec<SurfaceAssetRow> = match runtime {
            DatabasePool::SQLite(pool) => {
                let mut query_builder =
                    QueryBuilder::<sqlx::Sqlite>::new("SELECT * FROM surface_assets WHERE 1=1");
                push_surface_asset_filters(&mut query_builder, filter);
                query_builder.push(" ORDER BY last_seen_at DESC, id DESC");
                push_surface_asset_pagination(
                    &mut query_builder,
                    filter,
                    Some(" LIMIT -1 OFFSET "),
                );
                query_builder
                    .build_query_as::<SurfaceAssetRow>()
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                let mut query_builder =
                    QueryBuilder::<MySql>::new("SELECT * FROM surface_assets WHERE 1=1");
                push_surface_asset_filters(&mut query_builder, filter);
                query_builder.push(" ORDER BY last_seen_at DESC, id DESC");
                push_surface_asset_pagination(
                    &mut query_builder,
                    filter,
                    Some(" LIMIT 18446744073709551615 OFFSET "),
                );
                query_builder
                    .build_query_as::<SurfaceAssetRow>()
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut query_builder =
                    QueryBuilder::<Postgres>::new("SELECT * FROM surface_assets WHERE 1=1");
                push_surface_asset_filters(&mut query_builder, filter);
                query_builder.push(" ORDER BY last_seen_at DESC, id DESC");
                push_surface_asset_pagination(&mut query_builder, filter, None);
                query_builder
                    .build_query_as::<SurfaceAssetRow>()
                    .fetch_all(pool)
                    .await?
            }
        };

        Ok(rows)
    }

    pub async fn list_surface_relations(
        &self,
        filter: &SurfaceRelationFilter,
    ) -> Result<Vec<SurfaceRelationRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let rows: Vec<SurfaceRelationRow> = match runtime {
            DatabasePool::SQLite(pool) => {
                let mut query_builder =
                    QueryBuilder::<sqlx::Sqlite>::new("SELECT * FROM surface_relations WHERE 1=1");
                if let Some(program_id) = filter.program_id.as_deref() {
                    query_builder.push(" AND program_id = ");
                    query_builder.push_bind(program_id);
                }
                if let Some(asset_id) = filter.asset_id.as_deref() {
                    query_builder.push(" AND (from_asset_id = ");
                    query_builder.push_bind(asset_id);
                    query_builder.push(" OR to_asset_id = ");
                    query_builder.push_bind(asset_id);
                    query_builder.push(")");
                }
                if let Some(relation_type) = filter.relation_type.as_deref() {
                    query_builder.push(" AND relation_type = ");
                    query_builder.push_bind(relation_type);
                }
                query_builder.push(" ORDER BY last_seen_at DESC, id DESC");
                if let Some(limit) = filter.limit {
                    query_builder.push(" LIMIT ");
                    query_builder.push_bind(limit.max(0));
                }
                query_builder
                    .build_query_as::<SurfaceRelationRow>()
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                let mut query_builder =
                    QueryBuilder::<MySql>::new("SELECT * FROM surface_relations WHERE 1=1");
                if let Some(program_id) = filter.program_id.as_deref() {
                    query_builder.push(" AND program_id = ");
                    query_builder.push_bind(program_id);
                }
                if let Some(asset_id) = filter.asset_id.as_deref() {
                    query_builder.push(" AND (from_asset_id = ");
                    query_builder.push_bind(asset_id);
                    query_builder.push(" OR to_asset_id = ");
                    query_builder.push_bind(asset_id);
                    query_builder.push(")");
                }
                if let Some(relation_type) = filter.relation_type.as_deref() {
                    query_builder.push(" AND relation_type = ");
                    query_builder.push_bind(relation_type);
                }
                query_builder.push(" ORDER BY last_seen_at DESC, id DESC");
                if let Some(limit) = filter.limit {
                    query_builder.push(" LIMIT ");
                    query_builder.push_bind(limit.max(0));
                }
                query_builder
                    .build_query_as::<SurfaceRelationRow>()
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut query_builder =
                    QueryBuilder::<Postgres>::new("SELECT * FROM surface_relations WHERE 1=1");
                if let Some(program_id) = filter.program_id.as_deref() {
                    query_builder.push(" AND program_id = ");
                    query_builder.push_bind(program_id);
                }
                if let Some(asset_id) = filter.asset_id.as_deref() {
                    query_builder.push(" AND (from_asset_id = ");
                    query_builder.push_bind(asset_id);
                    query_builder.push(" OR to_asset_id = ");
                    query_builder.push_bind(asset_id);
                    query_builder.push(")");
                }
                if let Some(relation_type) = filter.relation_type.as_deref() {
                    query_builder.push(" AND relation_type = ");
                    query_builder.push_bind(relation_type);
                }
                query_builder.push(" ORDER BY last_seen_at DESC, id DESC");
                if let Some(limit) = filter.limit {
                    query_builder.push(" LIMIT ");
                    query_builder.push_bind(limit.max(0));
                }
                query_builder
                    .build_query_as::<SurfaceRelationRow>()
                    .fetch_all(pool)
                    .await?
            }
        };

        Ok(rows)
    }
}
