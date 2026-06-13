import {
  createDefaultTrafficPluginRuntimeSettings,
  type TrafficPluginActiveProbeSettings,
  type TrafficPluginRuntimeSettings,
} from './proxyConfigurationTypes'

export type TrafficPluginRuntimePreset = 'local_fast' | 'balanced' | 'conservative'
export type TrafficPluginRuntimePolicyId = 'activeProbe'
export type TrafficPluginRuntimePolicySettings =
  TrafficPluginRuntimeSettings[TrafficPluginRuntimePolicyId]

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
    timeoutMs: clampInteger(settings.timeoutMs || 3000, 1000, 120000),
  }
}

export function normalizeTrafficPluginRuntimeSettings(
  settings: TrafficPluginRuntimeSettings
): TrafficPluginRuntimeSettings {
  return {
    activeProbe: normalizeActiveProbeSettings(settings.activeProbe),
    directFetchMaxConcurrent: clampInteger(
      settings.directFetchMaxConcurrent ?? 200,
      1,
      1000
    ),
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
        maxGlobalConcurrent: 16,
        minHostCooldownMs: 200,
        maxConcurrentPerHost: 3,
        maxConcurrentPerRun: 16,
        maxConcurrentPerPlugin: 16,
        jitterRange: [0, 50],
        timeoutMs: 3000,
      },
      directFetchMaxConcurrent: 300,
    }
  }

  return {
    activeProbe: {
      ...defaults.activeProbe,
      maxGlobalConcurrent: 16,
      minHostCooldownMs: 1500,
      maxConcurrentPerHost: 1,
      maxConcurrentPerRun: 16,
      maxConcurrentPerPlugin: 16,
      jitterRange: [400, 1000],
      timeoutMs: 3000,
    },
    directFetchMaxConcurrent: 100,
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
    }
  }
  nextSettings.directFetchMaxConcurrent = presetSettings.directFetchMaxConcurrent
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
    }
  }
  nextSettings.directFetchMaxConcurrent = defaults.directFetchMaxConcurrent
  return normalizeTrafficPluginRuntimeSettings(nextSettings)
}

export function updateTrafficPluginRuntimePolicySettings(
  current: TrafficPluginRuntimeSettings,
  policyId: TrafficPluginRuntimePolicyId,
  policy: TrafficPluginRuntimePolicySettings
): TrafficPluginRuntimeSettings {
  return normalizeTrafficPluginRuntimeSettings({
    ...current,
    [policyId]: {
      ...current[policyId],
      ...policy,
      jitterRange: [...policy.jitterRange] as [number, number],
    },
  })
}
