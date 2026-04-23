use std::collections::HashSet;
use std::sync::Arc;

use crate::services::ensure_bug_bounty_access;
use chrono::Utc;
use sentinel_db::{populate_bounty_asset_domain_fields, DatabaseService};
use serde::Serialize;
use tauri::State;

#[derive(Debug, Clone, Serialize)]
pub struct BountyAssetDomainHierarchyBackfillResult {
    pub domain_assets: usize,
    pub root_domains: usize,
    pub updated_assets: usize,
}

#[tauri::command]
pub async fn bounty_backfill_domain_asset_hierarchy(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: String,
) -> Result<BountyAssetDomainHierarchyBackfillResult, String> {
    ensure_bug_bounty_access()?;

    let program_id = program_id.trim();
    if program_id.is_empty() {
        return Err("Program is required".to_string());
    }

    let assets = db_service
        .list_bounty_assets(
            Some(program_id),
            None,
            Some("domain"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .map_err(|error| error.to_string())?;

    let now = Utc::now().to_rfc3339();
    let mut root_domains = HashSet::new();
    let mut updated_assets = 0usize;

    for asset in assets.iter() {
        let mut normalized = asset.clone();
        populate_bounty_asset_domain_fields(&mut normalized);

        if let Some(root_domain) = normalized.root_domain.clone() {
            root_domains.insert(root_domain);
        }

        let hierarchy_changed = normalized.hostname != asset.hostname
            || normalized.parent_domain != asset.parent_domain
            || normalized.root_domain != asset.root_domain
            || normalized.subdomain_level != asset.subdomain_level;

        if !hierarchy_changed {
            continue;
        }

        normalized.updated_at = now.clone();
        db_service
            .update_bounty_asset(&normalized)
            .await
            .map_err(|error| error.to_string())?;
        updated_assets += 1;
    }

    Ok(BountyAssetDomainHierarchyBackfillResult {
        domain_assets: assets.len(),
        root_domains: root_domains.len(),
        updated_assets,
    })
}
