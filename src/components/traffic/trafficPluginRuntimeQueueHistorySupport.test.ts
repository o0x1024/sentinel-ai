import { describe, expect, it } from 'vitest'
import {
  appendTrafficPluginRuntimeQueueHistory,
  buildTrafficPluginRuntimeSparklinePath,
  createEmptyTrafficPluginRuntimeQueueHistory,
  getTrafficPluginRuntimeHistorySeries,
  TRAFFIC_PLUGIN_RUNTIME_HISTORY_LIMIT,
} from './trafficPluginRuntimeQueueHistorySupport'
import type { TrafficPluginRuntimeQueueStatsSnapshot } from './trafficPluginRuntimeQueueTypes'

function createSnapshot(seed: number): TrafficPluginRuntimeQueueStatsSnapshot {
  return {
    activeProbe: {
      kind: 'traffic_active_probe',
      pendingCount: seed,
      queuedCount: 0,
      scheduledCount: 0,
      runningCount: seed + 1,
      recentCount: 0,
      maxQueueDepth: seed,
      activeGlobal: 0,
      activeRuns: 0,
      activePlugins: 0,
      activeHosts: 0,
      hottestHost: null,
      hottestHostActive: 0,
      cancelledRunCount: 0,
      configuredMaxQueueDepth: 10,
      configuredMaxGlobalConcurrent: 10,
      configuredMaxConcurrentPerHost: 2,
      configuredMaxConcurrentPerRun: 4,
      configuredMaxConcurrentPerPlugin: 4,
      rejectedTotalCount: seed + 2,
      rejectedCancelledRunCount: 0,
      rejectedQueueLimitCount: 0,
      rejectedRunPendingLimitCount: 0,
      rejectedPluginPendingLimitCount: 0,
      recentWindowTotalCount: seed + 3,
      recentWindowCompletedCount: 0,
      recentWindowFailedCount: 0,
      recentWindowCancelledCount: 0,
      recentWindowAvgQueueWaitMs: 0,
      recentWindowAvgResponseElapsedMs: 0,
    },
  }
}

describe('trafficPluginRuntimeQueueHistorySupport', () => {
  it('appends and trims history by policy', () => {
    let history = createEmptyTrafficPluginRuntimeQueueHistory()

    for (let index = 0; index < TRAFFIC_PLUGIN_RUNTIME_HISTORY_LIMIT + 3; index += 1) {
      history = appendTrafficPluginRuntimeQueueHistory(
        history,
        createSnapshot(index),
        `2026-05-05T00:00:${String(index).padStart(2, '0')}Z`
      )
    }

    expect(history.activeProbe).toHaveLength(TRAFFIC_PLUGIN_RUNTIME_HISTORY_LIMIT)
    expect(history.activeProbe[0]?.pendingCount).toBe(3)
    expect(history.activeProbe.at(-1)?.pendingCount).toBe(TRAFFIC_PLUGIN_RUNTIME_HISTORY_LIMIT + 2)
    expect(
      getTrafficPluginRuntimeHistorySeries(history, 'traffic_active_probe', 'runningCount').at(-1)
    ).toBe(TRAFFIC_PLUGIN_RUNTIME_HISTORY_LIMIT + 3)
  })

  it('builds a sparkline path for non-empty values', () => {
    const path = buildTrafficPluginRuntimeSparklinePath([1, 3, 2, 5], 120, 40)
    expect(path.startsWith('M ')).toBe(true)
    expect(path.includes('L ')).toBe(true)
  })
})
