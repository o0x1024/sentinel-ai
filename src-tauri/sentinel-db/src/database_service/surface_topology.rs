use crate::database_service::service::DatabaseService;
use crate::database_service::surface::{
    SurfaceAssetFilter, SurfaceRelationFilter,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceTopologyNode {
    pub id: String,
    pub asset_type: String,
    pub asset_name: String,
    pub display_name: Option<String>,
    pub status: String,
    pub risk_level: Option<String>,
    pub source: Option<String>,
    pub last_seen_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceTopologyEdge {
    pub id: String,
    pub from_asset_id: String,
    pub to_asset_id: String,
    pub relation_type: String,
    pub source: Option<String>,
    pub active: bool,
    pub from_asset_type: String,
    pub from_asset_name: String,
    pub from_display_name: Option<String>,
    pub to_asset_type: String,
    pub to_asset_name: String,
    pub to_display_name: Option<String>,
    pub last_seen_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SurfaceTopologyResponse {
    pub nodes: Vec<SurfaceTopologyNode>,
    pub edges: Vec<SurfaceTopologyEdge>,
    pub by_type: HashMap<String, i32>,
    pub node_count: i32,
    pub edge_count: i32,
}

impl DatabaseService {
    pub async fn get_surface_topology(
        &self,
        program_id: Option<&str>,
        limit: Option<i64>,
    ) -> Result<SurfaceTopologyResponse> {
        let node_limit = limit.unwrap_or(200).max(0);
        let assets = self
            .list_surface_assets(&SurfaceAssetFilter {
                program_id: program_id.map(str::to_string),
                asset_type: None,
                search: None,
                limit: Some(node_limit),
                offset: Some(0),
            })
            .await?;

        let mut by_type = HashMap::new();
        let mut asset_map = HashMap::new();
        let nodes = assets
            .into_iter()
            .map(|asset| {
                *by_type.entry(asset.asset_type.clone()).or_insert(0) += 1;
                asset_map.insert(asset.id.clone(), asset.clone());
                SurfaceTopologyNode {
                    id: asset.id,
                    asset_type: asset.asset_type,
                    asset_name: asset.asset_name,
                    display_name: asset.display_name,
                    status: asset.status,
                    risk_level: asset.risk_level,
                    source: asset.source,
                    last_seen_at: asset.last_seen_at,
                }
            })
            .collect::<Vec<_>>();

        let relation_limit = Some((node_limit * 4).max(200));
        let edges = self
            .list_surface_relations(&SurfaceRelationFilter {
                program_id: program_id.map(str::to_string),
                asset_id: None,
                relation_type: None,
                limit: relation_limit,
            })
            .await?
            .into_iter()
            .filter_map(|relation| {
                let from_asset = asset_map.get(&relation.from_asset_id)?;
                let to_asset = asset_map.get(&relation.to_asset_id)?;

                Some(SurfaceTopologyEdge {
                    id: relation.id,
                    from_asset_id: relation.from_asset_id,
                    to_asset_id: relation.to_asset_id,
                    relation_type: relation.relation_type,
                    source: relation.source,
                    active: relation.active,
                    from_asset_type: from_asset.asset_type.clone(),
                    from_asset_name: from_asset.asset_name.clone(),
                    from_display_name: from_asset.display_name.clone(),
                    to_asset_type: to_asset.asset_type.clone(),
                    to_asset_name: to_asset.asset_name.clone(),
                    to_display_name: to_asset.display_name.clone(),
                    last_seen_at: relation.last_seen_at,
                })
            })
            .collect::<Vec<_>>();

        Ok(SurfaceTopologyResponse {
            node_count: nodes.len() as i32,
            edge_count: edges.len() as i32,
            by_type,
            nodes,
            edges,
        })
    }
}
