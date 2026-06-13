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
  return {
    activeProbe: {
      kind: 'traffic_active_probe',
      pendingCount: 1,
      queuedCount: 0,
      scheduledCount: 0,
      runningCount: 2,
      recentCount: 0,
      maxQueueDepth: 100,
      activeGlobal: 0,
      activeRuns: 0,
      activePlugins: 0,
      activeHosts: 0,
      hottestHost: null,
      hottestHostActive: 0,
      cancelledRunCount: 0,
      configuredMaxQueueDepth: 11,
      configuredMaxGlobalConcurrent: 21,
      configuredMaxConcurrentPerHost: 2,
      configuredMaxConcurrentPerRun: 4,
      configuredMaxConcurrentPerPlugin: 4,
      rejectedTotalCount: 3,
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
    },
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

  it('aggregates active probe into launcher summary', () => {
    const summary = buildTrafficPluginRuntimeLauncherSummary(createSnapshot(), createHistory(), [
      'activeProbe',
    ])

    expect(summary).not.toBeNull()
    expect(summary?.pendingCount).toBe(1)
    expect(summary?.pendingLimit).toBe(11)
    expect(summary?.runningCount).toBe(2)
    expect(summary?.runningLimit).toBe(21)
    expect(summary?.rejectedCount).toBe(3)
    expect(summary?.rejectedSeries).toEqual([1, 2])
  })

  it('builds human-readable tooltip text from launcher summary', () => {
    const summary = buildTrafficPluginRuntimeLauncherSummary(createSnapshot(), createHistory(), [
      'activeProbe',
    ])

    expect(summary).not.toBeNull()
    expect(getTrafficPluginRuntimePendingTooltip(summary!)).toBe(
      'pending 1/11, ratio 9%; action: no immediate adjustment needed'
    )
    expect(getTrafficPluginRuntimeRunningTooltip(summary!)).toBe(
      'running 2/21, ratio 10%; action: no immediate adjustment needed'
    )
    expect(getTrafficPluginRuntimeRejectedTooltip(summary!)).toBe(
      'rejected 3, +1 in last 2 samples; action: review queue and pending limits before rejection growth becomes sustained'
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
