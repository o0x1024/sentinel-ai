export type MonitorPluginSeedBindingConfig = {
  seed_type: string
  input_key: string
  use_project_seeds: boolean
  selected_project_values: string[]
  manual_values: string[]
}

export type MonitorPluginSeedConfig = {
  bindings: MonitorPluginSeedBindingConfig[]
}

export type MonitorPluginConfigLike = {
  plugin_id: string
  fallback_plugins: MonitorPluginConfigLike[]
  plugin_params: Record<string, unknown>
  target_asset_types: string[]
  seed_config: MonitorPluginSeedConfig
  [key: string]: unknown
}

export const normalizePluginInputMode = (value: unknown) => {
  const normalized = String(value || '').trim().toLowerCase()
  return ['asset', 'seed', 'hybrid'].includes(normalized) ? normalized : ''
}

export const DOMAIN_HIERARCHY_TARGET_ASSET_TYPES = [
  'domain_root',
  'domain_level_1',
  'domain_level_2',
  'domain_level_3_plus',
]

export const ALL_MONITOR_TARGET_ASSET_TYPES = [
  'web',
  'domain',
  ...DOMAIN_HIERARCHY_TARGET_ASSET_TYPES,
  'host',
  'ip',
  'service',
]

export const normalizeMonitorPluginId = (value: unknown) =>
  String(value || '')
    .trim()
    .replace(/^plugin__service_fingerprinter$/, 'plugin__service_probe')
    .replace(/^service_fingerprinter$/, 'service_probe')

export const createEmptyPluginConfig = (): MonitorPluginConfigLike => ({
  plugin_id: '',
  fallback_plugins: [],
  plugin_params: {},
  target_asset_types: [],
  seed_config: { bindings: [] },
})

const normalizeStringList = (value: unknown) =>
  Array.from(
    new Set(
      (Array.isArray(value) ? value : [])
        .map(item => String(item || '').trim())
        .filter(Boolean)
    )
  )

export const normalizeSeedBindingConfig = (value: unknown): MonitorPluginSeedBindingConfig | null => {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return null
  }

  const seedType = String((value as Record<string, unknown>).seed_type || '').trim().toLowerCase()
  const inputKey = String((value as Record<string, unknown>).input_key || '').trim()
  if (!seedType || !inputKey) {
    return null
  }

  return {
    seed_type: seedType,
    input_key: inputKey,
    use_project_seeds: Boolean((value as Record<string, unknown>).use_project_seeds),
    selected_project_values: normalizeStringList((value as Record<string, unknown>).selected_project_values),
    manual_values: normalizeStringList((value as Record<string, unknown>).manual_values),
  }
}

export const normalizeSeedConfig = (value: unknown): MonitorPluginSeedConfig => {
  const rawBindings =
    value && typeof value === 'object' && !Array.isArray(value)
      ? (value as Record<string, unknown>).bindings
      : []

  const bindings = Array.isArray(rawBindings)
    ? rawBindings
        .map(normalizeSeedBindingConfig)
        .filter((item): item is MonitorPluginSeedBindingConfig => Boolean(item))
    : []

  return { bindings }
}

export const normalizeTargetAssetTypes = (value: unknown) =>
  Array.from(
    new Set(
      (Array.isArray(value) ? value : [])
        .map(assetType =>
          String(assetType || '')
            .trim()
            .toLowerCase()
        )
        .filter(Boolean)
    )
  )

export const inferDefaultTargetAssetTypes = (pluginId: string): string[] => {
  const normalizedPluginId = normalizeMonitorPluginId(pluginId).replace(/^plugin__/, '')

  switch (normalizedPluginId) {
    case 'sensitive_file_scanner':
    case 'tech_fingerprinter':
    case 'favicon_fingerprinter':
    case 'content_monitor':
    case 'api_monitor':
    case 'js_analyzer':
    case 'js_link_finder':
    case 'risk_scanner':
      return ['web']
    case 'http_prober':
      return ['web', 'domain', 'service']
    case 'fofa_asset_monitor':
      return ['web', 'domain']
    case 'subdomain_enumerator':
    case 'subdomain_brute':
      return ['domain']
    case 'dns_resolver':
      return ['domain']
    case 'cert_monitor':
    case 'ssl_scanner':
      return ['domain', 'service']
    case 'port_monitor':
    case 'cidr_mapper':
      return ['ip']
    case 'service_monitor':
    case 'service_probe':
      return ['service']
    default:
      return []
  }
}

export const inferAllowedTargetAssetTypes = (monitorType: string, pluginId: string): string[] => {
  const inferred = inferDefaultTargetAssetTypes(pluginId)
  if (inferred.length > 0) {
    if (monitorType === 'dns' && inferred.includes('domain')) {
      return ['domain', ...DOMAIN_HIERARCHY_TARGET_ASSET_TYPES]
    }
    return inferred
  }

  switch (monitorType) {
    case 'dns':
      return ['domain', ...DOMAIN_HIERARCHY_TARGET_ASSET_TYPES]
    case 'cert':
      return ['domain', 'service']
    case 'ip':
      return ['domain']
    case 'port':
      return ['ip']
    case 'service':
      return ['service']
    case 'content':
    case 'api':
    case 'risk':
      return ['web']
    default:
      return ALL_MONITOR_TARGET_ASSET_TYPES
  }
}

export const getAllowedTargetAssetTypes = (monitorType: string, pluginId: string) =>
  inferAllowedTargetAssetTypes(monitorType, pluginId)

export const normalizeAllowedTargetAssetTypes = (
  monitorType: string,
  pluginId: string,
  value: unknown,
) => {
  const normalized = normalizeTargetAssetTypes(value)
  const allowed = inferAllowedTargetAssetTypes(monitorType, pluginId)
  const allowedSet = new Set(allowed)
  return normalized.filter(assetType => allowedSet.has(assetType))
}

export const sanitizeMonitorPluginParams = (value: unknown) => {
  const params =
    value && typeof value === 'object' && !Array.isArray(value)
      ? { ...(value as Record<string, unknown>) }
      : {}

  delete params.targets
  delete params.target_objects
  delete params.service_targets
  delete params.urls
  delete params.url
  delete params.domains
  delete params.domain
  delete params.__monitorExecution

  return Object.fromEntries(
    Object.entries(params).filter(([key]) => !key.startsWith('__monitor'))
  )
}

export const supportsServiceProbeEngine = (plugin: Partial<MonitorPluginConfigLike> | null | undefined) => {
  const pluginId = normalizeMonitorPluginId(plugin?.plugin_id || '')
  return ['service_monitor', 'service_probe', 'plugin__service_monitor', 'plugin__service_probe'].includes(pluginId)
}

export const getServiceProbeEngine = () => 'native'

export const ensurePluginParamsObject = (plugin: Partial<MonitorPluginConfigLike> | null | undefined) => {
  if (!plugin) {
    return {}
  }

  if (
    !plugin.plugin_params
    || typeof plugin.plugin_params !== 'object'
    || Array.isArray(plugin.plugin_params)
  ) {
    plugin.plugin_params = {}
  }

  return plugin.plugin_params as Record<string, unknown>
}

export const setServiceProbeEngine = (
  plugin: Partial<MonitorPluginConfigLike> | null | undefined,
  engine: string,
) => {
  if (!plugin) return
  const params = ensurePluginParamsObject(plugin)
  params.serviceProbeEngine = engine || 'native'
}

export const applyDefaultTargetAssetTypes = (plugin: Partial<MonitorPluginConfigLike> | null | undefined) => {
  if (!plugin) return
  plugin.target_asset_types = inferDefaultTargetAssetTypes(String(plugin.plugin_id || ''))
}

export const resetPluginForSelection = (plugin: MonitorPluginConfigLike) => {
  plugin.plugin_id = normalizeMonitorPluginId(plugin.plugin_id)
  plugin.plugin_params = {}
  applyDefaultTargetAssetTypes(plugin)
  plugin.seed_config = { bindings: [] }
  if (supportsServiceProbeEngine(plugin)) {
    setServiceProbeEngine(plugin, getServiceProbeEngine())
  }
}

export const normalizePluginConfig = (plugin: any, monitorType = ''): MonitorPluginConfigLike => {
  const pluginId = normalizeMonitorPluginId(plugin?.plugin_id || plugin || '')
  const fallbacksRaw = Array.isArray(plugin?.fallback_plugins) ? plugin.fallback_plugins : []
  const normalizedPlugin: MonitorPluginConfigLike = {
    plugin_id: pluginId,
    fallback_plugins: fallbacksRaw.map(fallback =>
      normalizePluginConfig(
        typeof fallback === 'string' ? { plugin_id: fallback } : fallback,
        monitorType,
      )
    ),
    plugin_params: sanitizeMonitorPluginParams(plugin?.plugin_params),
    target_asset_types: normalizeAllowedTargetAssetTypes(
      monitorType,
      pluginId,
      plugin?.target_asset_types,
    ),
    seed_config: normalizeSeedConfig(plugin?.seed_config),
  }

  if (supportsServiceProbeEngine(normalizedPlugin)) {
    setServiceProbeEngine(normalizedPlugin, getServiceProbeEngine())
  }

  return normalizedPlugin
}

export const normalizePluginConfigList = (plugins: unknown, monitorType = '') =>
  Array.isArray(plugins)
    ? plugins.map(plugin =>
        normalizePluginConfig(
          typeof plugin === 'string' ? { plugin_id: plugin } : plugin,
          monitorType,
        )
      )
    : []

export const mapPluginTree = (
  plugins: MonitorPluginConfigLike[],
  visitor: (plugin: MonitorPluginConfigLike) => void,
) => {
  for (const plugin of plugins) {
    visitor(plugin)
    if (Array.isArray(plugin.fallback_plugins) && plugin.fallback_plugins.length > 0) {
      mapPluginTree(plugin.fallback_plugins, visitor)
    }
  }
}
