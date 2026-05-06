import { describe, expect, it } from 'vitest'

import { createDefaultTrafficPluginRuntimeSettings } from './proxyConfigurationTypes'
import {
  createTrafficPluginRuntimePreset,
  mergeTrafficPluginRuntimeSettings,
  normalizeTrafficPluginRuntimeSettings,
} from './pluginRuntimeSettingsSupport'

describe('pluginRuntimeSettingsSupport', () => {
  it('fills missing policy groups from defaults', () => {
    const merged = mergeTrafficPluginRuntimeSettings({
      activeProbe: {
        ...createDefaultTrafficPluginRuntimeSettings().activeProbe,
        timeoutMs: 9000,
      },
    })

    expect(merged.activeProbe.timeoutMs).toBe(9000)
    expect(merged.bountyFetch.maxQueueDepth).toBe(1000)
    expect(merged.pluginTestFetch.timeoutMs).toBe(5000)
  })

  it('clamps invalid numeric values into allowed ranges', () => {
    const normalized = normalizeTrafficPluginRuntimeSettings({
      ...createDefaultTrafficPluginRuntimeSettings(),
      activeProbe: {
        ...createDefaultTrafficPluginRuntimeSettings().activeProbe,
        maxQueueDepth: 0,
        maxGlobalConcurrent: 999,
        minHostCooldownMs: -1,
        jitterRange: [9000, 100],
        timeoutMs: 999999,
      },
    })

    expect(normalized.activeProbe.maxQueueDepth).toBe(1)
    expect(normalized.activeProbe.maxGlobalConcurrent).toBe(128)
    expect(normalized.activeProbe.minHostCooldownMs).toBe(0)
    expect(normalized.activeProbe.jitterRange).toEqual([100, 9000])
    expect(normalized.activeProbe.timeoutMs).toBe(120000)
  })

  it('balanced preset matches the shipped defaults', () => {
    expect(createTrafficPluginRuntimePreset('balanced')).toEqual(
      createDefaultTrafficPluginRuntimeSettings()
    )
  })
})
