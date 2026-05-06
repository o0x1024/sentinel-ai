import {
  createDefaultTrafficPluginRuntimeSettings,
  type TrafficPluginActiveProbeSettings,
  type TrafficPluginFetchPolicySettings,
  type TrafficPluginRuntimeSettings,
} from './proxyConfigurationTypes'

export type TrafficPluginRuntimePreset = 'local_fast' | 'balanced' | 'conservative'
export type TrafficPluginRuntimePolicyId =
  | 'activeProbe'
  | 'bountyFetch'
  | 'monitorFetch'
  | 'agentFetch'
  | 'pluginTestFetch'

function clampInteger(value: number, min: number, max: number) {
  if (!Number.isFinite(value)) {
    return min
  }
  return Math.min(max, Math.max(min, Math.round(value)))
}

function normalizeJitterRange(range: [number, number], max: number): [number, number] {
  const start = clampInteger(range[0] ?? 0, 0, max)
  const end = clampInteger(range[1] ?? 0, 0, max)
  return start <= end ? [start, end] : [end, start]
}

function normalizeFetchPolicySettings(
  settings: TrafficPluginFetchPolicySettings
): TrafficPluginFetchPolicySettings {
  return {
    maxQueueDepth: clampInteger(settings.maxQueueDepth || 1, 1, 5000),
    maxPendingPerRun: clampInteger(settings.maxPendingPerRun || 1, 1, 2000),
    maxPendingPerPlugin: clampInteger(settings.maxPendingPerPlugin || 1, 1, 5000),
    maxGlobalConcurrent: clampInteger(settings.maxGlobalConcurrent || 1, 1, 128),
    maxConcurrentPerHost: clampInteger(settings.maxConcurrentPerHost || 1, 1, 32),
    maxConcurrentPerRun: clampInteger(settings.maxConcurrentPerRun || 1, 1, 128),
    maxConcurrentPerPlugin: clampInteger(settings.maxConcurrentPerPlugin || 1, 1, 128),
    minHostDelayMs: clampInteger(settings.minHostDelayMs || 0, 0, 60000),
    jitterRange: normalizeJitterRange(settings.jitterRange, 30000),
    timeoutMs: clampInteger(settings.timeoutMs || 1000, 1000, 120000),
  }
}

function normalizeActiveProbeSettings(
  settings: TrafficPluginActiveProbeSettings
): TrafficPluginActiveProbeSettings {
  return {
    maxQueueDepth: clampInteger(settings.maxQueueDepth || 1, 1, 5000),
    maxPendingPerRun: clampInteger(settings.maxPendingPerRun || 1, 1, 2000),
    maxPendingPerPlugin: clampInteger(settings.maxPendingPerPlugin || 1, 1, 5000),
    maxGlobalConcurrent: clampInteger(settings.maxGlobalConcurrent || 1, 1, 128),
    jitterRange: normalizeJitterRange(settings.jitterRange, 30000),
    minHostCooldownMs: clampInteger(settings.minHostCooldownMs || 0, 0, 60000),
    maxConcurrentPerHost: clampInteger(settings.maxConcurrentPerHost || 1, 1, 32),
    maxConcurrentPerRun: clampInteger(settings.maxConcurrentPerRun || 1, 1, 128),
    maxConcurrentPerPlugin: clampInteger(settings.maxConcurrentPerPlugin || 1, 1, 128),
    timeoutMs: clampInteger(settings.timeoutMs || 1000, 1000, 120000),
  }
}

export function normalizeTrafficPluginRuntimeSettings(
  settings: TrafficPluginRuntimeSettings
): TrafficPluginRuntimeSettings {
  return {
    activeProbe: normalizeActiveProbeSettings(settings.activeProbe),
    bountyFetch: normalizeFetchPolicySettings(settings.bountyFetch),
    monitorFetch: normalizeFetchPolicySettings(settings.monitorFetch),
    agentFetch: normalizeFetchPolicySettings(settings.agentFetch),
    pluginTestFetch: normalizeFetchPolicySettings(settings.pluginTestFetch),
  }
}

export function mergeTrafficPluginRuntimeSettings(
  settings: Partial<TrafficPluginRuntimeSettings>
): TrafficPluginRuntimeSettings {
  const defaults = createDefaultTrafficPluginRuntimeSettings()
  return normalizeTrafficPluginRuntimeSettings({
    ...defaults,
    ...settings,
    activeProbe: {
      ...defaults.activeProbe,
      ...(settings.activeProbe || {}),
    },
    bountyFetch: {
      ...defaults.bountyFetch,
      ...(settings.bountyFetch || {}),
    },
    monitorFetch: {
      ...defaults.monitorFetch,
      ...(settings.monitorFetch || {}),
    },
    agentFetch: {
      ...defaults.agentFetch,
      ...(settings.agentFetch || {}),
    },
    pluginTestFetch: {
      ...defaults.pluginTestFetch,
      ...(settings.pluginTestFetch || {}),
    },
  })
}

export function createTrafficPluginRuntimePreset(
  preset: TrafficPluginRuntimePreset
): TrafficPluginRuntimeSettings {
  const defaults = createDefaultTrafficPluginRuntimeSettings()
  if (preset === 'balanced') {
    return defaults
  }

  if (preset === 'local_fast') {
    return {
      activeProbe: {
        ...defaults.activeProbe,
        maxGlobalConcurrent: 24,
        minHostCooldownMs: 200,
        maxConcurrentPerHost: 3,
        maxConcurrentPerRun: 8,
        maxConcurrentPerPlugin: 12,
        jitterRange: [0, 50],
        timeoutMs: 6000,
      },
      bountyFetch: {
        ...defaults.bountyFetch,
        maxGlobalConcurrent: 24,
        maxConcurrentPerHost: 3,
        maxConcurrentPerRun: 8,
        maxConcurrentPerPlugin: 12,
        minHostDelayMs: 200,
        jitterRange: [0, 100],
      },
      monitorFetch: {
        ...defaults.monitorFetch,
        maxGlobalConcurrent: 10,
        maxConcurrentPerHost: 2,
        maxConcurrentPerRun: 4,
        maxConcurrentPerPlugin: 6,
        minHostDelayMs: 500,
        jitterRange: [50, 150],
        timeoutMs: 10000,
      },
      agentFetch: {
        ...defaults.agentFetch,
        maxGlobalConcurrent: 12,
        maxConcurrentPerHost: 3,
        maxConcurrentPerRun: 5,
        maxConcurrentPerPlugin: 8,
        minHostDelayMs: 100,
        jitterRange: [0, 50],
        timeoutMs: 12000,
      },
      pluginTestFetch: {
        ...defaults.pluginTestFetch,
        maxGlobalConcurrent: 4,
        maxConcurrentPerHost: 2,
        maxConcurrentPerRun: 3,
        maxConcurrentPerPlugin: 3,
        minHostDelayMs: 0,
        jitterRange: [0, 20],
        timeoutMs: 4000,
      },
    }
  }

  return {
    activeProbe: {
      ...defaults.activeProbe,
      maxGlobalConcurrent: 8,
      minHostCooldownMs: 1500,
      maxConcurrentPerHost: 1,
      maxConcurrentPerRun: 3,
      maxConcurrentPerPlugin: 4,
      jitterRange: [400, 1000],
      timeoutMs: 12000,
    },
    bountyFetch: {
      ...defaults.bountyFetch,
      maxGlobalConcurrent: 10,
      maxConcurrentPerHost: 1,
      maxConcurrentPerRun: 3,
      maxConcurrentPerPlugin: 4,
      minHostDelayMs: 1500,
      jitterRange: [400, 1000],
      timeoutMs: 12000,
    },
    monitorFetch: {
      ...defaults.monitorFetch,
      maxGlobalConcurrent: 4,
      maxConcurrentPerRun: 2,
      maxConcurrentPerPlugin: 3,
      minHostDelayMs: 3000,
      jitterRange: [800, 2500],
      timeoutMs: 20000,
    },
    agentFetch: {
      ...defaults.agentFetch,
      maxGlobalConcurrent: 6,
      maxConcurrentPerHost: 1,
      maxConcurrentPerRun: 2,
      maxConcurrentPerPlugin: 3,
      minHostDelayMs: 1000,
      jitterRange: [300, 800],
      timeoutMs: 20000,
    },
    pluginTestFetch: {
      ...defaults.pluginTestFetch,
      maxGlobalConcurrent: 1,
      maxConcurrentPerHost: 1,
      maxConcurrentPerRun: 1,
      maxConcurrentPerPlugin: 1,
      minHostDelayMs: 300,
      jitterRange: [50, 200],
      timeoutMs: 8000,
    },
  }
}

export function applyTrafficPluginRuntimePresetToPolicies(
  current: TrafficPluginRuntimeSettings,
  preset: TrafficPluginRuntimePreset,
  policyIds: TrafficPluginRuntimePolicyId[]
): TrafficPluginRuntimeSettings {
  const presetSettings = createTrafficPluginRuntimePreset(preset)
  const nextSettings = mergeTrafficPluginRuntimeSettings(current)
  for (const policyId of policyIds) {
    if (policyId === 'activeProbe') {
      nextSettings.activeProbe = presetSettings.activeProbe
      continue
    }
    nextSettings[policyId] = presetSettings[policyId]
  }
  return normalizeTrafficPluginRuntimeSettings(nextSettings)
}

export function resetTrafficPluginRuntimePoliciesToDefaults(
  current: TrafficPluginRuntimeSettings,
  policyIds: TrafficPluginRuntimePolicyId[]
): TrafficPluginRuntimeSettings {
  const defaults = createDefaultTrafficPluginRuntimeSettings()
  const nextSettings = mergeTrafficPluginRuntimeSettings(current)
  for (const policyId of policyIds) {
    if (policyId === 'activeProbe') {
      nextSettings.activeProbe = defaults.activeProbe
      continue
    }
    nextSettings[policyId] = defaults[policyId]
  }
  return normalizeTrafficPluginRuntimeSettings(nextSettings)
}
