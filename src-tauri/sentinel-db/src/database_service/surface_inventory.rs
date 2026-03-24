use crate::database_service::service::DatabaseService;
use crate::database_service::surface::{SurfaceAssetFilter, SurfaceAssetRow};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::warn;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceInventoryItem {
    pub asset: SurfaceAssetRow,
    pub typed_details: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceInventoryResponse {
    pub items: Vec<SurfaceInventoryItem>,
    pub total: i64,
}

impl DatabaseService {
    pub async fn list_surface_inventory(
        &self,
        filter: &SurfaceAssetFilter,
    ) -> Result<SurfaceInventoryResponse> {
        let total_filter = SurfaceAssetFilter {
            program_id: filter.program_id.clone(),
            asset_type: filter.asset_type.clone(),
            status: filter.status.clone(),
            search: filter.search.clone(),
            limit: None,
            offset: None,
        };

        let total = self.list_surface_assets(&total_filter).await?.len() as i64;
        let assets = self.list_surface_assets(filter).await?;
        let mut rows = Vec::with_capacity(assets.len());

        for asset in assets {
            let typed_details = match self.get_surface_typed_details(&asset).await {
                Ok(details) => details,
                Err(error) => {
                    warn!(
                        "failed to load typed details for surface asset {} ({}): {}",
                        asset.id, asset.asset_type, error
                    );
                    None
                }
            };
            rows.push(SurfaceInventoryItem {
                asset,
                typed_details,
            });
        }

        Ok(SurfaceInventoryResponse { items: rows, total })
    }
}
