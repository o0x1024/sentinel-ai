import { invoke } from '@tauri-apps/api/core'

interface CommandResponse<T> {
  success: boolean
  data?: T
  error?: string
}

export function buildPluginConfigSkeleton(schema: any): any {
  const type = String(schema?.type || '').toLowerCase()

  if (type === 'object' || (!type && schema?.properties)) {
    const properties = schema?.properties || {}
    const result: Record<string, any> = {}

    for (const key of Object.keys(properties)) {
      const propSchema = properties[key]
      if (propSchema?.default !== undefined) {
        result[key] = propSchema.default
        continue
      }
      result[key] = buildPluginConfigSkeleton(propSchema)
    }

    return result
  }

  if (type === 'array') {
    return []
  }
  if (type === 'string') {
    if (Array.isArray(schema?.enum) && schema.enum.length > 0) {
      return schema.enum[0]
    }
    return ''
  }
  if (type === 'number' || type === 'integer') {
    if (typeof schema?.minimum === 'number') {
      return schema.minimum
    }
    return 0
  }
  if (type === 'boolean') {
    return false
  }

  return {}
}

export function deepMergePluginConfig(base: any, override: any): any {
  if (!isPlainObject(base)) {
    return override === undefined ? base : override
  }
  if (!isPlainObject(override)) {
    return override === undefined ? base : override
  }

  const merged: Record<string, any> = { ...base }
  for (const [key, value] of Object.entries(override)) {
    const existing = merged[key]
    if (isPlainObject(existing) && isPlainObject(value)) {
      merged[key] = deepMergePluginConfig(existing, value)
      continue
    }
    merged[key] = value
  }

  return merged
}

export async function getPluginDefaultConfig(pluginId: string): Promise<Record<string, any>> {
  const response = await invoke<CommandResponse<Record<string, any>>>('get_plugin_default_input_config', { pluginId })

  if (!response.success) {
    throw new Error(response.error || `Failed to load default config for plugin ${pluginId}`)
  }

  return normalizePluginConfigObject(response.data)
}

export async function savePluginDefaultConfig(
  pluginId: string,
  config: Record<string, any>,
): Promise<Record<string, any>> {
  const response = await invoke<CommandResponse<Record<string, any>>>('set_plugin_default_input_config', {
    payload: {
      pluginId,
      config,
    },
  })

  if (!response.success) {
    throw new Error(response.error || `Failed to save default config for plugin ${pluginId}`)
  }

  return normalizePluginConfigObject(response.data)
}

export async function buildResolvedPluginDefaultConfig(
  pluginId: string,
  schema: any,
): Promise<Record<string, any>> {
  const stored = await getPluginDefaultConfig(pluginId)
  const skeleton = buildPluginConfigSkeleton(schema)
  return normalizePluginConfigObject(deepMergePluginConfig(skeleton, stored))
}

function isPlainObject(value: unknown): value is Record<string, any> {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value)
}

function normalizePluginConfigObject(value: unknown): Record<string, any> {
  return isPlainObject(value) ? { ...(value as Record<string, any>) } : {}
}
