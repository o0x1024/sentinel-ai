use crate::database_service::service::DatabaseService;
use crate::database_service::surface::{SurfaceAssetFilter, SurfaceRelationFilter};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

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
    pub visible_node_count: i32,
    pub visible_edge_count: i32,
    pub node_offset: i64,
    pub edge_offset: i64,
    pub node_limit: i64,
    pub edge_limit: i64,
    pub has_more_nodes: bool,
    pub has_more_edges: bool,
}

impl DatabaseService {
    pub async fn get_surface_topology(
        &self,
        program_id: Option<&str>,
        node_limit: Option<i64>,
        node_offset: Option<i64>,
        edge_limit: Option<i64>,
        edge_offset: Option<i64>,
    ) -> Result<SurfaceTopologyResponse> {
        let all_assets = self
            .list_surface_assets(&SurfaceAssetFilter {
                program_id: program_id.map(str::to_string),
                asset_type: None,
                status: None,
                search: None,
                service_name: None,
                transport_protocol: None,
                limit: None,
                offset: None,
            })
            .await?;
        let all_relations = self
            .list_surface_relations(&SurfaceRelationFilter {
                program_id: program_id.map(str::to_string),
                asset_id: None,
                relation_type: None,
                limit: None,
            })
            .await?;

        let mut by_type = HashMap::new();
        for asset in &all_assets {
            *by_type.entry(asset.asset_type.clone()).or_insert(0) += 1;
        }

        let total_node_count = all_assets.len() as i32;
        let total_edge_count = all_relations.len() as i32;
        let node_limit = node_limit.unwrap_or(48).max(1);
        let node_offset = node_offset.unwrap_or(0).max(0);
        let edge_limit = edge_limit.unwrap_or(25).max(1);
        let edge_offset = edge_offset.unwrap_or(0).max(0);

        let paged_assets = all_assets
            .into_iter()
            .skip(node_offset as usize)
            .take(node_limit as usize)
            .collect::<Vec<_>>();

        let visible_asset_ids = paged_assets
            .iter()
            .map(|asset| asset.id.clone())
            .collect::<HashSet<_>>();
        let mut asset_map = HashMap::new();
        let nodes = paged_assets
            .into_iter()
            .map(|asset| {
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

        let visible_relations = all_relations
            .into_iter()
            .filter(|relation| {
                visible_asset_ids.contains(&relation.from_asset_id)
                    && visible_asset_ids.contains(&relation.to_asset_id)
            })
            .collect::<Vec<_>>();
        let visible_edge_count = visible_relations.len() as i32;
        let has_more_edges = (edge_offset as usize + edge_limit as usize) < visible_relations.len();
        let edges = visible_relations
            .into_iter()
            .skip(edge_offset as usize)
            .take(edge_limit as usize)
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
            node_count: total_node_count,
            edge_count: total_edge_count,
            visible_node_count: nodes.len() as i32,
            visible_edge_count,
            node_offset,
            edge_offset,
            node_limit,
            edge_limit,
            has_more_nodes: (node_offset as usize + node_limit as usize)
                < total_node_count as usize,
            has_more_edges,
            by_type,
            nodes,
            edges,
        })
    }
}
