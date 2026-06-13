import { describe, expect, it } from 'vitest'
import { normalizeTrafficPluginRuntimeQueueStatsSnapshot } from './trafficPluginRuntimeQueueTypes'

describe('normalizeTrafficPluginRuntimeQueueStatsSnapshot', () => {
  it('normalizes active probe queue stats snapshot', () => {
    const snapshot = normalizeTrafficPluginRuntimeQueueStatsSnapshot({
      activeProbe: {
        kind: 'traffic_active_probe',
        pendingCount: 3,
        queuedCount: 2,
        scheduledCount: 1,
        runningCount: 4,
        recentCount: 10,
        maxQueueDepth: 3,
        activeGlobal: 4,
        activeRuns: 2,
        activePlugins: 1,
        activeHosts: 2,
        hottestHost: 'example.com',
        hottestHostActive: 2,
        cancelledRunCount: 0,
        configuredMaxQueueDepth: 1000,
        configuredMaxGlobalConcurrent: 20,
        configuredMaxConcurrentPerHost: 2,
        configuredMaxConcurrentPerRun: 6,
        configuredMaxConcurrentPerPlugin: 10,
        rejectedTotalCount: 4,
        rejectedCancelledRunCount: 1,
        rejectedQueueLimitCount: 1,
        rejectedRunPendingLimitCount: 1,
        rejectedPluginPendingLimitCount: 1,
        recentWindowTotalCount: 3,
        recentWindowCompletedCount: 2,
        recentWindowFailedCount: 1,
        recentWindowCancelledCount: 0,
        recentWindowAvgQueueWaitMs: 120,
        recentWindowAvgResponseElapsedMs: 340,
      },
    })

    expect(snapshot?.activeProbe.pendingCount).toBe(3)
    expect(snapshot?.activeProbe.hottestHost).toBe('example.com')
    expect(snapshot?.activeProbe.rejectedTotalCount).toBe(4)
    expect(snapshot?.activeProbe.recentWindowAvgQueueWaitMs).toBe(120)
    expect(snapshot?.activeProbe.kind).toBe('traffic_active_probe')
  })
})
