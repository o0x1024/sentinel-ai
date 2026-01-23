//! Asset Enrichment Commands

use tauri::State;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use sentinel_bounty::services::AssetEnrichmentService;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichAssetRequest {
    pub asset_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichAssetResponse {
    pub success: bool,
    pub message: Option<String>,
    pub error: Option<String>,
}

/// Enrich a single asset
#[tauri::command]
pub async fn enrich_asset(
    enrichment_service: State<'_, Arc<AssetEnrichmentService>>,
    request: EnrichAssetRequest,
) -> Result<EnrichAssetResponse, String> {
    match enrichment_service.enrich_asset(&request.asset_id).await {
        Ok(_) => Ok(EnrichAssetResponse {
            success: true,
            message: Some("Asset enriched successfully".to_string()),
            error: None,
        }),
        Err(e) => Ok(EnrichAssetResponse {
            success: false,
            message: None,
            error: Some(e.to_string()),
        }),
    }
}

/// Start enrichment service
#[tauri::command]
pub async fn start_asset_enrichment(
    enrichment_service: State<'_, Arc<AssetEnrichmentService>>,
) -> Result<bool, String> {
    enrichment_service.start().await.map_err(|e| e.to_string())?;
    Ok(true)
}

/// Stop enrichment service
#[tauri::command]
pub async fn stop_asset_enrichment(
    enrichment_service: State<'_, Arc<AssetEnrichmentService>>,
) -> Result<bool, String> {
    enrichment_service.stop().await.map_err(|e| e.to_string())?;
    Ok(true)
}
