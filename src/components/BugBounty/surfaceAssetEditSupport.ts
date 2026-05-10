export interface SurfaceAssetEditPayload {
  display_name?: string | null
  description?: string | null
  owner?: string | null
  status?: string | null
  internet_exposure?: string | null
  criticality?: string | null
  risk_level?: string | null
  typed_details?: Record<string, unknown> | null
}

export interface SurfaceAssetEditTarget {
  asset: Record<string, any>
  typed_details?: Record<string, unknown> | null
}

export type SurfaceAssetEditFieldType = 'text' | 'number' | 'select'

export interface SurfaceAssetEditOption {
  value: string
  label?: string
  labelKey?: string
}

export interface SurfaceAssetEditFieldSchema {
  key: string
  labelKey: string
  type: SurfaceAssetEditFieldType
  options?: SurfaceAssetEditOption[]
}

export interface SurfaceAssetTypeEditSchema {
  fields: SurfaceAssetEditFieldSchema[]
}

export type SurfaceAssetTypedFormState = Record<string, string>

const YES_NO_OPTIONS: SurfaceAssetEditOption[] = [
  { value: 'true', labelKey: 'common.yes' },
  { value: 'false', labelKey: 'common.no' },
]

const TRANSPORT_PROTOCOL_OPTIONS: SurfaceAssetEditOption[] = [
  { value: 'tcp', label: 'tcp' },
  { value: 'udp', label: 'udp' },
]

const IP_VERSION_OPTIONS: SurfaceAssetEditOption[] = [
  { value: 'ipv4', label: 'ipv4' },
  { value: 'ipv6', label: 'ipv6' },
]

const HTTP_SCHEME_OPTIONS: SurfaceAssetEditOption[] = [
  { value: 'http', label: 'http' },
  { value: 'https', label: 'https' },
]

const surfaceAssetEditSchemas: Record<string, SurfaceAssetTypeEditSchema> = {
  org: {
    fields: [
      { key: 'org_name', labelKey: 'bugBounty.surface.inventory.fields.orgName', type: 'text' },
      { key: 'business_line', labelKey: 'bugBounty.surface.inventory.fields.businessLine', type: 'text' },
      { key: 'importance_level', labelKey: 'bugBounty.surface.inventory.fields.importanceLevel', type: 'text' },
    ],
  },
  domain: {
    fields: [
      { key: 'fqdn', labelKey: 'bugBounty.surface.inventory.fields.fqdn', type: 'text' },
      { key: 'root_domain', labelKey: 'bugBounty.surface.inventory.fields.rootDomain', type: 'text' },
      { key: 'subdomain_level', labelKey: 'bugBounty.surface.inventory.fields.subdomainLevel', type: 'number' },
      { key: 'record_type', labelKey: 'bugBounty.surface.inventory.fields.recordType', type: 'text' },
      { key: 'record_value', labelKey: 'bugBounty.surface.inventory.fields.recordValue', type: 'text' },
      { key: 'registrar', labelKey: 'bugBounty.surface.inventory.fields.registrar', type: 'text' },
    ],
  },
  ip: {
    fields: [
      { key: 'ip_address', labelKey: 'bugBounty.surface.inventory.fields.ipAddress', type: 'text' },
      { key: 'ip_version', labelKey: 'bugBounty.surface.inventory.edit.ipVersion', type: 'select', options: IP_VERSION_OPTIONS },
      { key: 'cidr', labelKey: 'bugBounty.surface.inventory.fields.cidr', type: 'text' },
      { key: 'asn', labelKey: 'bugBounty.surface.inventory.fields.asn', type: 'number' },
      { key: 'cloud_provider', labelKey: 'bugBounty.surface.inventory.fields.cloudProvider', type: 'text' },
      { key: 'network_boundary_type', labelKey: 'bugBounty.surface.inventory.fields.boundaryType', type: 'text' },
    ],
  },
  host: {
    fields: [
      { key: 'hostname', labelKey: 'bugBounty.surface.inventory.edit.hostname', type: 'text' },
      { key: 'fqdn', labelKey: 'bugBounty.surface.inventory.fields.fqdn', type: 'text' },
      { key: 'operating_system', labelKey: 'bugBounty.surface.inventory.fields.operatingSystem', type: 'text' },
      { key: 'device_type', labelKey: 'bugBounty.surface.inventory.fields.deviceType', type: 'text' },
      { key: 'region_or_datacenter', labelKey: 'bugBounty.surface.inventory.fields.regionOrDatacenter', type: 'text' },
      { key: 'last_online_at', labelKey: 'bugBounty.surface.inventory.fields.lastOnlineAt', type: 'text' },
    ],
  },
  port: {
    fields: [
      { key: 'ip_address', labelKey: 'bugBounty.surface.inventory.fields.ipAddress', type: 'text' },
      { key: 'port', labelKey: 'bugBounty.surface.inventory.fields.portNumber', type: 'number' },
      {
        key: 'transport_protocol',
        labelKey: 'bugBounty.surface.inventory.fields.transportProtocol',
        type: 'select',
        options: TRANSPORT_PROTOCOL_OPTIONS,
      },
      { key: 'state', labelKey: 'bugBounty.surface.inventory.fields.portState', type: 'text' },
    ],
  },
  service: {
    fields: [
      { key: 'ip_address', labelKey: 'bugBounty.surface.inventory.fields.ipAddress', type: 'text' },
      { key: 'port', labelKey: 'bugBounty.surface.inventory.fields.portNumber', type: 'number' },
      {
        key: 'transport_protocol',
        labelKey: 'bugBounty.surface.inventory.fields.transportProtocol',
        type: 'select',
        options: TRANSPORT_PROTOCOL_OPTIONS,
      },
      {
        key: 'application_service_name',
        labelKey: 'bugBounty.surface.inventory.fields.applicationServiceName',
        type: 'text',
      },
      { key: 'product_name', labelKey: 'bugBounty.surface.inventory.fields.productName', type: 'text' },
      { key: 'version', labelKey: 'bugBounty.surface.inventory.fields.version', type: 'text' },
      { key: 'auth_type', labelKey: 'bugBounty.surface.inventory.fields.authType', type: 'text' },
      { key: 'login_required', labelKey: 'bugBounty.surface.inventory.edit.loginRequired', type: 'select', options: YES_NO_OPTIONS },
      {
        key: 'encrypted_transport',
        labelKey: 'bugBounty.surface.inventory.edit.encryptedTransport',
        type: 'select',
        options: YES_NO_OPTIONS,
      },
    ],
  },
  web: {
    fields: [
      { key: 'canonical_url', labelKey: 'bugBounty.surface.inventory.fields.canonicalUrl', type: 'text' },
      { key: 'scheme', labelKey: 'bugBounty.surface.inventory.edit.scheme', type: 'select', options: HTTP_SCHEME_OPTIONS },
      { key: 'site_title', labelKey: 'bugBounty.surface.inventory.fields.siteTitle', type: 'text' },
      { key: 'http_status_code', labelKey: 'bugBounty.surface.inventory.fields.httpStatusCode', type: 'number' },
      { key: 'favicon_hash', labelKey: 'bugBounty.surface.inventory.fields.favicon', type: 'text' },
      { key: 'framework', labelKey: 'bugBounty.surface.inventory.fields.framework', type: 'text' },
      { key: 'business_type', labelKey: 'bugBounty.surface.inventory.fields.businessType', type: 'text' },
      { key: 'login_flag', labelKey: 'bugBounty.surface.inventory.edit.loginRequired', type: 'select', options: YES_NO_OPTIONS },
      { key: 'api_flag', labelKey: 'bugBounty.surface.inventory.edit.apiFlag', type: 'select', options: YES_NO_OPTIONS },
    ],
  },
  certificate: {
    fields: [
      { key: 'sha256', labelKey: 'bugBounty.surface.inventory.fields.sha256', type: 'text' },
      { key: 'subject', labelKey: 'bugBounty.surface.inventory.fields.subject', type: 'text' },
      { key: 'issuer', labelKey: 'bugBounty.surface.inventory.fields.issuer', type: 'text' },
      { key: 'valid_to', labelKey: 'bugBounty.surface.inventory.fields.validTo', type: 'text' },
      { key: 'risk_status', labelKey: 'bugBounty.surface.inventory.fields.riskStatus', type: 'text' },
    ],
  },
}

const deepCloneJsonRecord = (value: Record<string, unknown> | null | undefined): Record<string, unknown> => {
  if (!value || Array.isArray(value)) return {}
  return JSON.parse(JSON.stringify(value)) as Record<string, unknown>
}

const normalizeFormValue = (field: SurfaceAssetEditFieldSchema, value: string): unknown => {
  const trimmed = value.trim()
  if (!trimmed) return undefined
  if (field.type === 'number') {
    const parsed = Number.parseInt(trimmed, 10)
    return Number.isFinite(parsed) ? parsed : undefined
  }
  if (field.options === YES_NO_OPTIONS) {
    return trimmed === 'true'
  }
  return trimmed
}

export const getSurfaceAssetEditSchema = (assetType: string | null | undefined): SurfaceAssetTypeEditSchema => {
  return surfaceAssetEditSchemas[String(assetType || '').trim()] || { fields: [] }
}

export const buildSurfaceAssetTypedForm = (
  assetType: string | null | undefined,
  typedDetails: Record<string, unknown> | null | undefined,
): SurfaceAssetTypedFormState => {
  const schema = getSurfaceAssetEditSchema(assetType)
  const details = typedDetails && !Array.isArray(typedDetails) ? typedDetails : {}
  return schema.fields.reduce<SurfaceAssetTypedFormState>((acc, field) => {
    const rawValue = details[field.key]
    if (typeof rawValue === 'boolean') {
      acc[field.key] = rawValue ? 'true' : 'false'
      return acc
    }
    if (rawValue === null || rawValue === undefined) {
      acc[field.key] = ''
      return acc
    }
    acc[field.key] = String(rawValue)
    return acc
  }, {})
}

export const buildSurfaceAssetTypedDetailsPayload = (
  assetType: string | null | undefined,
  originalTypedDetails: Record<string, unknown> | null | undefined,
  typedForm: SurfaceAssetTypedFormState,
): Record<string, unknown> | null => {
  const schema = getSurfaceAssetEditSchema(assetType)
  const merged = deepCloneJsonRecord(originalTypedDetails)

  for (const field of schema.fields) {
    const nextValue = normalizeFormValue(field, typedForm[field.key] || '')
    if (nextValue === undefined) {
      delete merged[field.key]
    } else {
      merged[field.key] = nextValue
    }
  }

  return Object.keys(merged).length ? merged : null
}
