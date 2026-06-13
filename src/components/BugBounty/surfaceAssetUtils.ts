export interface SurfaceReferencedAsset {
  id: string
  name: string
  value: string
  asset_type: string
  risk_level?: string
  status?: string
  description?: string
  tags?: string[]
  metadata?: Record<string, any>
}

const primaryDetailKeysByType: Record<string, string[]> = {
  org: ['org_name', 'business_line'],
  domain: ['fqdn', 'root_domain', 'record_value'],
  ip: ['ip_address', 'cidr'],
  host: ['hostname', 'fqdn'],
  port: ['ip_address', 'port_number'],
  service: ['application_service_name', 'product_name', 'ip_address'],
  web: ['canonical_url', 'site_title', 'openapi_url'],
  certificate: ['sha256', 'subject'],
}

export function getSurfaceAssetPrimaryValue(asset: any, typedDetails?: Record<string, any> | null): string {
  const assetType = String(asset?.asset_type || '')
  const detailKeys = primaryDetailKeysByType[assetType] || []
  for (const key of detailKeys) {
    const value = typedDetails?.[key]
    if (value !== null && value !== undefined && String(value).trim() !== '') {
      if (assetType === 'port' && key === 'port_number') {
        const host = typedDetails?.ip_address || asset?.asset_name || ''
        return `${host}:${value}`
      }
      return String(value)
    }
  }

  return String(asset?.display_name || asset?.asset_name || asset?.id || '')
}

let pendingSurfaceAssets: SurfaceReferencedAsset[] = []

export function queueSurfaceAssetForAssistant(asset: SurfaceReferencedAsset) {
  pendingSurfaceAssets.push(asset)
}

export function consumePendingSurfaceAssistantAssets(): SurfaceReferencedAsset[] {
  const assets = pendingSurfaceAssets
  pendingSurfaceAssets = []
  return assets
}

export function buildReferencedSurfaceAsset(detail: any): SurfaceReferencedAsset {
  const asset = detail?.asset || {}
  const typedDetails = detail?.typed_details || {}
  const fingerprints = Array.isArray(detail?.fingerprints) ? detail.fingerprints : []
  const evidence = Array.isArray(detail?.evidence) ? detail.evidence : []
  const changes = Array.isArray(detail?.changes) ? detail.changes : []

  return {
    id: String(asset.id || ''),
    name: String(asset.display_name || asset.asset_name || asset.id || ''),
    value: getSurfaceAssetPrimaryValue(asset, typedDetails),
    asset_type: String(asset.asset_type || ''),
    risk_level: asset.risk_level || undefined,
    status: asset.status || undefined,
    description: asset.description || undefined,
    tags: [asset.internet_exposure, asset.criticality].filter(Boolean),
    metadata: {
      program_id: asset.program_id,
      source: asset.source,
      owner: asset.owner,
      last_seen_at: asset.last_seen_at,
      typed_details: typedDetails,
      fingerprints: fingerprints.slice(0, 10).map((item: any) => ({
        type: item?.fingerprint_type,
        key: item?.fingerprint_key,
        value: item?.fingerprint_value,
        confidence: item?.confidence_score,
      })),
      evidence: evidence.slice(0, 5).map((item: any) => ({
        type: item?.evidence_type,
        title: item?.title,
        collected_at: item?.collected_at,
      })),
      changes: changes.slice(0, 5).map((item: any) => ({
        type: item?.change_type,
        summary: item?.summary,
        detected_at: item?.detected_at,
      })),
    },
  }
}
