import { describe, expect, it } from 'vitest'
import { createEmptyTrafficPluginRuntimeQueueHistory } from './trafficPluginRuntimeQueueHistorySupport'
import {
  buildTrafficPluginRuntimeLauncherSummary,
  getTrafficPluginRuntimePendingTooltip,
  getTrafficPluginRuntimeLauncherPresetRecommendation,
  getTrafficPluginRuntimeRejectedTooltip,
  getTrafficPluginRuntimeRunningTooltip,
  isTrafficPluginRuntimePresetApplied,
} from './trafficPluginRuntimeDialogLauncherSupport'
import { createTrafficPluginRuntimePreset } from './pluginRuntimeSettingsSupport'
import type {
  TrafficPluginRuntimeQueueHistorySnapshot,
  TrafficPluginRuntimeQueueStatsSnapshot,
} from './trafficPluginRuntimeQueueTypes'

function createSnapshot(): TrafficPluginRuntimeQueueStatsSnapshot {
  const stats = (
    kind: TrafficPluginRuntimeQueueStatsSnapshot['activeProbe']['kind'],
    seed: number
  ) => ({
    kind,
    pendingCount: seed,
    queuedCount: 0,
    scheduledCount: 0,
    runningCount: seed + 1,
    recentCount: 0,
    maxQueueDepth: 100,
    activeGlobal: 0,
    activeRuns: 0,
    activePlugins: 0,
    activeHosts: 0,
    hottestHost: null,
    hottestHostActive: 0,
    cancelledRunCount: 0,
    configuredMaxQueueDepth: 10 + seed,
    configuredMaxGlobalConcurrent: 20 + seed,
    configuredMaxConcurrentPerHost: 2,
    configuredMaxConcurrentPerRun: 4,
    configuredMaxConcurrentPerPlugin: 4,
    rejectedTotalCount: seed + 2,
    rejectedCancelledRunCount: 0,
    rejectedQueueLimitCount: 0,
    rejectedRunPendingLimitCount: 0,
    rejectedPluginPendingLimitCount: 0,
    recentWindowTotalCount: 0,
    recentWindowCompletedCount: 0,
    recentWindowFailedCount: 0,
    recentWindowCancelledCount: 0,
    recentWindowAvgQueueWaitMs: 0,
    recentWindowAvgResponseElapsedMs: 0,
  })

  return {
    activeProbe: stats('traffic_active_probe', 1),
    bountyFetch: stats('bounty_fetch', 2),
    monitorFetch: stats('monitor_fetch', 3),
    agentFetch: stats('agent_fetch', 4),
    pluginTestFetch: stats('plugin_test_fetch', 5),
  }
}

function createHistory(): TrafficPluginRuntimeQueueHistorySnapshot {
  const history = createEmptyTrafficPluginRuntimeQueueHistory()
  history.activeProbe = [
    {
      recordedAt: '2026-05-05T00:00:00Z',
      pendingCount: 1,
      runningCount: 2,
      rejectedTotalCount: 1,
      recentWindowTotalCount: 0,
    },
    {
      recordedAt: '2026-05-05T00:00:03Z',
      pendingCount: 2,
      runningCount: 2,
      rejectedTotalCount: 2,
      recentWindowTotalCount: 0,
    },
  ]
  history.bountyFetch = [
    {
      recordedAt: '2026-05-05T00:00:00Z',
      pendingCount: 3,
      runningCount: 4,
      rejectedTotalCount: 5,
      recentWindowTotalCount: 0,
    },
    {
      recordedAt: '2026-05-05T00:00:03Z',
      pendingCount: 4,
      runningCount: 4,
      rejectedTotalCount: 7,
      recentWindowTotalCount: 0,
    },
  ]
  return history
}

describe('trafficPluginRuntimeDialogLauncherSupport', () => {
  it('returns null when snapshot is unavailable', () => {
    expect(
      buildTrafficPluginRuntimeLauncherSummary(
        null,
        createEmptyTrafficPluginRuntimeQueueHistory(),
        ['activeProbe']
      )
    ).toBeNull()
  })

  it('aggregates selected policies into one launcher summary', () => {
    const summary = buildTrafficPluginRuntimeLauncherSummary(createSnapshot(), createHistory(), [
      'activeProbe',
      'bountyFetch',
    ])

    expect(summary).not.toBeNull()
    expect(summary?.pendingCount).toBe(3)
    expect(summary?.pendingLimit).toBe(23)
    expect(summary?.runningCount).toBe(5)
    expect(summary?.runningLimit).toBe(43)
    expect(summary?.rejectedCount).toBe(7)
    expect(summary?.rejectedSeries).toEqual([6, 9])
  })

  it('builds human-readable tooltip text from launcher summary', () => {
    const summary = buildTrafficPluginRuntimeLauncherSummary(createSnapshot(), createHistory(), [
      'activeProbe',
      'bountyFetch',
    ])

    expect(summary).not.toBeNull()
    expect(getTrafficPluginRuntimePendingTooltip(summary!)).toBe(
      'pending 3/23, ratio 13%; action: no immediate adjustment needed'
    )
    expect(getTrafficPluginRuntimeRunningTooltip(summary!)).toBe(
      'running 5/43, ratio 12%; action: no immediate adjustment needed'
    )
    expect(getTrafficPluginRuntimeRejectedTooltip(summary!)).toBe(
      'rejected 7, +3 in last 2 samples; action: inspect queue depth and per-run or per-plugin pending limits; requests are being dropped'
    )
  })

  it('returns stronger actions when pending or running is saturated', () => {
    const saturatedSummary = {
      pendingCount: 19,
      pendingLimit: 20,
      runningCount: 18,
      runningLimit: 20,
      rejectedCount: 1,
      rejectedSeries: [1, 1, 1, 1],
    }

    expect(getTrafficPluginRuntimePendingTooltip(saturatedSummary)).toContain(
      'action: raise maxQueueDepth or reduce producer rate now'
    )
    expect(getTrafficPluginRuntimeRunningTooltip(saturatedSummary)).toContain(
      'action: raise maxGlobalConcurrent if targets can tolerate it, or slow request producers'
    )
  })

  it('recommends a fast preset when queue pressure is high', () => {
    const recommendation = getTrafficPluginRuntimeLauncherPresetRecommendation({
      pendingCount: 18,
      pendingLimit: 20,
      runningCount: 18,
      runningLimit: 20,
      rejectedCount: 5,
      rejectedSeries: [1, 2, 4, 5],
    })

    expect(recommendation).toEqual({
      preset: 'local_fast',
      severity: 'error',
      reason: '当前队列已经接近或达到上限，建议切到本地快速预设，立即提高吞吐并减少排队或拒绝。',
      alternativeExplanation:
        '这次不推荐 balanced，因为当前问题已经不是常规负载；也不推荐 conservative，因为它会进一步降低吞吐，更容易放大排队和拒绝。',
    })
  })

  it('detects whether a preset is already applied for selected policies', () => {
    const localFast = createTrafficPluginRuntimePreset('local_fast')
    expect(isTrafficPluginRuntimePresetApplied(localFast, 'local_fast', ['activeProbe'])).toBe(true)
    expect(isTrafficPluginRuntimePresetApplied(localFast, 'balanced', ['activeProbe'])).toBe(false)
  })
})
