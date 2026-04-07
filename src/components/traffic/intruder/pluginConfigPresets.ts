const PLUGIN_CONFIG_PRESETS_STORAGE_KEY = 'trafficAnalysis.intruder.pluginConfigPresets.v1'

export interface IntruderPluginConfigPreset {
  name: string
  configText: string
  updatedAt: string
}

type PluginPresetStore = Record<string, IntruderPluginConfigPreset[]>

function normalizePluginId(pluginId: string): string {
  return pluginId.trim()
}

function loadPresetStore(): PluginPresetStore {
  const raw = localStorage.getItem(PLUGIN_CONFIG_PRESETS_STORAGE_KEY)
  if (!raw) {
    return {}
  }

  try {
    const parsed = JSON.parse(raw) as PluginPresetStore
    return parsed && typeof parsed === 'object' ? parsed : {}
  } catch {
    return {}
  }
}

function persistPresetStore(store: PluginPresetStore) {
  localStorage.setItem(PLUGIN_CONFIG_PRESETS_STORAGE_KEY, JSON.stringify(store))
}

export function loadIntruderPluginConfigPresets(pluginId: string): IntruderPluginConfigPreset[] {
  const normalizedPluginId = normalizePluginId(pluginId)
  if (!normalizedPluginId) {
    return []
  }

  const store = loadPresetStore()
  const presets = Array.isArray(store[normalizedPluginId]) ? store[normalizedPluginId] : []
  return [...presets].sort((left, right) => right.updatedAt.localeCompare(left.updatedAt))
}

export function saveIntruderPluginConfigPreset(
  pluginId: string,
  presetName: string,
  configText: string,
) {
  const normalizedPluginId = normalizePluginId(pluginId)
  const normalizedPresetName = presetName.trim()
  if (!normalizedPluginId || !normalizedPresetName) {
    return
  }

  const store = loadPresetStore()
  const presets = Array.isArray(store[normalizedPluginId]) ? store[normalizedPluginId] : []
  const nextPreset: IntruderPluginConfigPreset = {
    name: normalizedPresetName,
    configText,
    updatedAt: new Date().toISOString(),
  }

  store[normalizedPluginId] = [
    nextPreset,
    ...presets.filter((preset) => preset.name !== normalizedPresetName),
  ]
  persistPresetStore(store)
}

export function deleteIntruderPluginConfigPreset(pluginId: string, presetName: string) {
  const normalizedPluginId = normalizePluginId(pluginId)
  const normalizedPresetName = presetName.trim()
  if (!normalizedPluginId || !normalizedPresetName) {
    return
  }

  const store = loadPresetStore()
  const presets = Array.isArray(store[normalizedPluginId]) ? store[normalizedPluginId] : []
  const nextPresets = presets.filter((preset) => preset.name !== normalizedPresetName)

  if (nextPresets.length === 0) {
    delete store[normalizedPluginId]
  } else {
    store[normalizedPluginId] = nextPresets
  }

  persistPresetStore(store)
}
