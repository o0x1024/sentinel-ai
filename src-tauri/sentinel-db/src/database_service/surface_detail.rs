use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use crate::database_service::surface::{
    SurfaceAssetRow, SurfaceCertAssetRow, SurfaceDomainAssetRow, SurfaceHostAssetRow,
    SurfaceIpAssetRow, SurfacePortAssetRow, SurfaceRelationFilter, SurfaceRelationRow,
    SurfaceServiceAssetRow, SurfaceWebAssetRow,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceAssetRelationDetail {
    pub relation: SurfaceRelationRow,
    pub direction: String,
    pub peer_asset: SurfaceAssetRow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceAssetDetailResponse {
    pub asset: SurfaceAssetRow,
    pub typed_details: Option<Value>,
    pub relations: Vec<SurfaceAssetRelationDetail>,
    pub fingerprints: Vec<crate::database_service::surface::SurfaceFingerprintRow>,
    pub evidence: Vec<crate::database_service::surface::SurfaceEvidenceRow>,
    pub changes: Vec<crate::database_service::surface::SurfaceChangeLogRow>,
}

impl DatabaseService {
    pub async fn get_surface_asset_by_id(&self, asset_id: &str) -> Result<Option<SurfaceAssetRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::SQLite(pool) => Ok(sqlx::query_as(
                "SELECT * FROM surface_assets WHERE id = ? LIMIT 1",
            )
            .bind(asset_id)
            .fetch_optional(pool)
            .await?),
            DatabasePool::MySQL(pool) => Ok(sqlx::query_as(
                "SELECT * FROM surface_assets WHERE id = ? LIMIT 1",
            )
            .bind(asset_id)
            .fetch_optional(pool)
            .await?),
            DatabasePool::PostgreSQL(pool) => Ok(sqlx::query_as(
                "SELECT * FROM surface_assets WHERE id = $1 LIMIT 1",
            )
            .bind(asset_id)
            .fetch_optional(pool)
            .await?),
        }
    }

    async fn get_surface_typed_details(&self, asset: &SurfaceAssetRow) -> Result<Option<Value>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match asset.asset_type.as_str() {
            "domain" => match runtime {
                DatabasePool::SQLite(pool) => Ok(sqlx::query_as::<_, SurfaceDomainAssetRow>(
                    "SELECT * FROM surface_domain_assets WHERE asset_id = ? LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
                DatabasePool::MySQL(pool) => Ok(sqlx::query_as::<_, SurfaceDomainAssetRow>(
                    "SELECT * FROM surface_domain_assets WHERE asset_id = ? LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
                DatabasePool::PostgreSQL(pool) => {
                    Ok(sqlx::query_as::<_, SurfaceDomainAssetRow>(
                        "SELECT * FROM surface_domain_assets WHERE asset_id = $1 LIMIT 1",
                    )
                    .bind(&asset.id)
                    .fetch_optional(pool)
                    .await?
                    .map(serde_json::to_value)
                    .transpose()?)
                }
            },
            "ip" => match runtime {
                DatabasePool::SQLite(pool) => Ok(sqlx::query_as::<_, SurfaceIpAssetRow>(
                    "SELECT * FROM surface_ip_assets WHERE asset_id = ? LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
                DatabasePool::MySQL(pool) => Ok(sqlx::query_as::<_, SurfaceIpAssetRow>(
                    "SELECT * FROM surface_ip_assets WHERE asset_id = ? LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
                DatabasePool::PostgreSQL(pool) => Ok(sqlx::query_as::<_, SurfaceIpAssetRow>(
                    "SELECT * FROM surface_ip_assets WHERE asset_id = $1 LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
            },
            "host" => match runtime {
                DatabasePool::SQLite(pool) => Ok(sqlx::query_as::<_, SurfaceHostAssetRow>(
                    "SELECT * FROM surface_host_assets WHERE asset_id = ? LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
                DatabasePool::MySQL(pool) => Ok(sqlx::query_as::<_, SurfaceHostAssetRow>(
                    "SELECT * FROM surface_host_assets WHERE asset_id = ? LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
                DatabasePool::PostgreSQL(pool) => Ok(sqlx::query_as::<_, SurfaceHostAssetRow>(
                    "SELECT * FROM surface_host_assets WHERE asset_id = $1 LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
            },
            "port" => match runtime {
                DatabasePool::SQLite(pool) => Ok(sqlx::query_as::<_, SurfacePortAssetRow>(
                    "SELECT * FROM surface_port_assets WHERE asset_id = ? LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
                DatabasePool::MySQL(pool) => Ok(sqlx::query_as::<_, SurfacePortAssetRow>(
                    "SELECT * FROM surface_port_assets WHERE asset_id = ? LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
                DatabasePool::PostgreSQL(pool) => Ok(sqlx::query_as::<_, SurfacePortAssetRow>(
                    "SELECT * FROM surface_port_assets WHERE asset_id = $1 LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
            },
            "service" => match runtime {
                DatabasePool::SQLite(pool) => Ok(sqlx::query_as::<_, SurfaceServiceAssetRow>(
                    "SELECT * FROM surface_service_assets WHERE asset_id = ? LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
                DatabasePool::MySQL(pool) => Ok(sqlx::query_as::<_, SurfaceServiceAssetRow>(
                    "SELECT * FROM surface_service_assets WHERE asset_id = ? LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
                DatabasePool::PostgreSQL(pool) => {
                    Ok(sqlx::query_as::<_, SurfaceServiceAssetRow>(
                        "SELECT * FROM surface_service_assets WHERE asset_id = $1 LIMIT 1",
                    )
                    .bind(&asset.id)
                    .fetch_optional(pool)
                    .await?
                    .map(serde_json::to_value)
                    .transpose()?)
                }
            },
            "web" => match runtime {
                DatabasePool::SQLite(pool) => Ok(sqlx::query_as::<_, SurfaceWebAssetRow>(
                    "SELECT * FROM surface_web_assets WHERE asset_id = ? LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
                DatabasePool::MySQL(pool) => Ok(sqlx::query_as::<_, SurfaceWebAssetRow>(
                    "SELECT * FROM surface_web_assets WHERE asset_id = ? LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
                DatabasePool::PostgreSQL(pool) => Ok(sqlx::query_as::<_, SurfaceWebAssetRow>(
                    "SELECT * FROM surface_web_assets WHERE asset_id = $1 LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
            },
            "certificate" => match runtime {
                DatabasePool::SQLite(pool) => Ok(sqlx::query_as::<_, SurfaceCertAssetRow>(
                    "SELECT * FROM surface_cert_assets WHERE asset_id = ? LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
                DatabasePool::MySQL(pool) => Ok(sqlx::query_as::<_, SurfaceCertAssetRow>(
                    "SELECT * FROM surface_cert_assets WHERE asset_id = ? LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
                DatabasePool::PostgreSQL(pool) => Ok(sqlx::query_as::<_, SurfaceCertAssetRow>(
                    "SELECT * FROM surface_cert_assets WHERE asset_id = $1 LIMIT 1",
                )
                .bind(&asset.id)
                .fetch_optional(pool)
                .await?
                .map(serde_json::to_value)
                .transpose()?),
            },
            _ => Ok(None),
        }
    }

    pub async fn get_surface_asset_detail(
        &self,
        asset_id: &str,
    ) -> Result<Option<SurfaceAssetDetailResponse>> {
        let Some(asset) = self.get_surface_asset_by_id(asset_id).await? else {
            return Ok(None);
        };

        let typed_details = self.get_surface_typed_details(&asset).await?;
        let fingerprints = self
            .list_surface_fingerprints(Some(&asset.program_id), Some(asset_id), Some(50))
            .await?;
        let evidence = self
            .list_surface_evidence(Some(&asset.program_id), Some(asset_id), Some(50))
            .await?;
        let changes = self
            .list_surface_change_logs(Some(&asset.program_id), Some(asset_id), Some(50))
            .await?;

        let mut relations = Vec::new();
        for relation in self
            .list_surface_relations(&SurfaceRelationFilter {
                program_id: Some(asset.program_id.clone()),
                asset_id: Some(asset.id.clone()),
                relation_type: None,
                limit: Some(50),
            })
            .await?
        {
            let (peer_id, direction) = if relation.from_asset_id == asset.id {
                (relation.to_asset_id.clone(), "outgoing")
            } else {
                (relation.from_asset_id.clone(), "incoming")
            };

            if let Some(peer_asset) = self.get_surface_asset_by_id(&peer_id).await? {
                relations.push(SurfaceAssetRelationDetail {
                    relation,
                    direction: direction.to_string(),
                    peer_asset,
                });
            }
        }

        Ok(Some(SurfaceAssetDetailResponse {
            asset,
            typed_details,
            relations,
            fingerprints,
            evidence,
            changes,
        }))
    }
}
