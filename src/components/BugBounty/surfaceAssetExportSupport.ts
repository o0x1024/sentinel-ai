import { getSurfaceAssetPrimaryValue } from './surfaceAssetUtils'

export type SurfaceAssetExportType =
  | 'all'
  | 'current'
  | 'api'
  | 'org'
  | 'domain'
  | 'ip'
  | 'host'
  | 'port'
  | 'service'
  | 'web'
  | 'certificate'

export interface SurfaceInventoryItemLike {
  asset: Record<string, any>
  typed_details?: Record<string, any> | null
}

const BASE_EXPORT_COLUMNS = [
  'id',
  'program_id',
  'asset_type',
  'asset_name',
  'display_name',
  'primary_value',
  'status',
  'internet_exposure',
  'criticality',
  'risk_level',
  'source',
  'owner',
  'maintainer',
  'last_seen_at',
  'created_at',
  'updated_at',
] as const

const API_HINT_RE = /(^api[.\-])|(\/api(?:\/|$))|(\bgraphql\b)|(\bopenapi\b)|(\bswagger\b)/

const normalizeText = (value: unknown) => String(value || '').trim()

const stringifyScalar = (value: unknown) => {
  if (value === null || value === undefined) return ''
  if (typeof value === 'string') return value
  if (typeof value === 'number' || typeof value === 'boolean') return String(value)
  return JSON.stringify(value)
}

export const isApiLikeSurfaceAsset = (item: SurfaceInventoryItemLike) => {
  const details = item.typed_details || {}
  if (details.api_flag === true) return true
  if (normalizeText(details.openapi_url)) return true

  const joined = [
    details.canonical_url,
    details.site_title,
    details.content_summary,
    details.application_service_name,
    details.protocol_name,
    details.product_name,
    item.asset?.asset_name,
    item.asset?.display_name,
  ]
    .map(value => normalizeText(value).toLowerCase())
    .filter(Boolean)
    .join(' ')

  return API_HINT_RE.test(joined)
}

export const filterSurfaceInventoryItemsForExport = (
  items: SurfaceInventoryItemLike[],
  exportType: SurfaceAssetExportType,
  currentAssetType: string | null,
) => {
  if (exportType === 'all') return items
  if (exportType === 'current') {
    if (!currentAssetType) return items
    return items.filter(item => item.asset?.asset_type === currentAssetType)
  }
  if (exportType === 'api') {
    return items.filter(isApiLikeSurfaceAsset)
  }
  return items.filter(item => item.asset?.asset_type === exportType)
}

export const buildSurfaceAssetExportRows = (
  items: SurfaceInventoryItemLike[],
  exportType: SurfaceAssetExportType,
) => {
  const detailKeys = Array.from(
    new Set(
      items.flatMap(item =>
        Object.keys(item.typed_details || {}).filter(key => key !== 'asset_id'),
      ),
    ),
  ).sort()

  return items.map(item => {
    const asset = item.asset || {}
    const details = item.typed_details || {}
    const row: Record<string, string> = {}

    BASE_EXPORT_COLUMNS.forEach(key => {
      if (key === 'primary_value') {
        row[key] = getSurfaceAssetPrimaryValue(asset, details)
        return
      }
      row[key] = stringifyScalar(asset[key])
    })

    row.export_category = exportType === 'current' ? asset.asset_type || '' : exportType
    detailKeys.forEach(key => {
      row[key] = stringifyScalar(details[key])
    })

    return row
  })
}

export const convertSurfaceAssetExportRowsToCsv = (rows: Record<string, string>[]) => {
  if (!rows.length) return ''

  const headers = Array.from(
    rows.reduce((set, row) => {
      Object.keys(row).forEach(key => set.add(key))
      return set
    }, new Set<string>()),
  )

  const escapeCsvValue = (value: string) => `"${value.replace(/"/g, '""')}"`
  const body = rows.map(row => headers.map(header => escapeCsvValue(row[header] || '')).join(','))
  return [headers.join(','), ...body].join('\n')
}

export const sanitizeExportFilenamePart = (value: string) =>
  value
    .trim()
    .replace(/[^\w.-]+/g, '_')
    .replace(/^_+|_+$/g, '') || 'assets'
