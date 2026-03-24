export interface SurfaceTopologyNode {
  id: string
  asset_type: string
  asset_name: string
  display_name?: string | null
  status: string
  risk_level?: string | null
  source?: string | null
  last_seen_at: string
}

export interface SurfaceTopologyEdge {
  id: string
  from_asset_id: string
  to_asset_id: string
  relation_type: string
  source?: string | null
  active: boolean
  from_asset_type: string
  from_asset_name: string
  from_display_name?: string | null
  to_asset_type: string
  to_asset_name: string
  to_display_name?: string | null
  last_seen_at: string
}

export interface SurfaceTopologyResponse {
  nodes: SurfaceTopologyNode[]
  edges: SurfaceTopologyEdge[]
  by_type: Record<string, number>
  node_count: number
  edge_count: number
  visible_node_count: number
  visible_edge_count: number
  node_offset: number
  edge_offset: number
  node_limit: number
  edge_limit: number
  has_more_nodes: boolean
  has_more_edges: boolean
}

export const SURFACE_TOPOLOGY_NODE_PAGE_SIZES = [24, 48, 96]
export const SURFACE_TOPOLOGY_EDGE_PAGE_SIZES = [10, 25, 50]
export const SURFACE_TOPOLOGY_TYPE_ORDER = ['org', 'domain', 'ip', 'host', 'port', 'service', 'web', 'certificate']

export const createEmptySurfaceTopology = (): SurfaceTopologyResponse => ({
  nodes: [],
  edges: [],
  by_type: {},
  node_count: 0,
  edge_count: 0,
  visible_node_count: 0,
  visible_edge_count: 0,
  node_offset: 0,
  edge_offset: 0,
  node_limit: SURFACE_TOPOLOGY_NODE_PAGE_SIZES[0],
  edge_limit: SURFACE_TOPOLOGY_EDGE_PAGE_SIZES[1],
  has_more_nodes: false,
  has_more_edges: false,
})
