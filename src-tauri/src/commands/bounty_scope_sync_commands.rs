use std::sync::Arc;

use crate::services::ensure_bug_bounty_access;
use sentinel_db::{DatabaseService, SurfaceAssetFilter};
use serde::Serialize;
use tauri::State;

use super::surface_scope_sync_support::create_missing_in_scope_domains;

#[derive(Debug, Clone, Serialize)]
pub struct BountyDomainScopeBackfillResult {
    pub domain_assets: usize,
    pub root_domains: usize,
    pub scopes_created: usize,
}

#[tauri::command]
pub async fn bounty_backfill_domain_scopes_from_assets(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: String,
) -> Result<BountyDomainScopeBackfillResult, String> {
    ensure_bug_bounty_access()?;

    let program_id = program_id.trim();
    if program_id.is_empty() {
        return Err("Program is required".to_string());
    }

    let assets = db_service
        .list_surface_assets(&SurfaceAssetFilter {
            program_id: Some(program_id.to_string()),
            asset_type: Some("domain".to_string()),
            status: None,
            search: None,
            favicon_hash: None,
            has_favicon_hash: None,
            http_status_code: None,
            service_name: None,
            transport_protocol: None,
            view_state: None,
            is_favorite: None,
            column_filters: None,
            limit: None,
            offset: None,
        })
        .await
        .map_err(|e| e.to_string())?;

    let domains: Vec<String> = assets
        .iter()
        .map(|asset| asset.asset_name.trim().to_ascii_lowercase())
        .filter(|asset_name| !asset_name.is_empty())
        .collect();

    let root_domains = domains
        .iter()
        .filter_map(|domain| super::surface_scope_sync_support::infer_root_domain(domain))
        .collect::<std::collections::HashSet<_>>()
        .len();

    let scopes_created = create_missing_in_scope_domains(
        db_service.inner().as_ref(),
        program_id,
        &domains,
        "Auto-created from domain asset backfill",
        "auto_root_domain_from_existing_assets",
    )
    .await?;

    Ok(BountyDomainScopeBackfillResult {
        domain_assets: domains.len(),
        root_domains,
        scopes_created,
    })
}
