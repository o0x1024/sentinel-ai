import { describe, expect, it } from 'vitest'
import {
  ACTIVE_PROBE_INITIAL_QUEUE_LIMIT,
  ACTIVE_PROBE_INITIAL_RECENT_LIMIT,
  buildActiveProbePanelDerivedState,
} from './trafficActiveProbePanelSupport'
import type { ActiveProbeQueueEntry } from './trafficActiveProbeTypes'

function buildEntry(
  requestId: string,
  phase: ActiveProbeQueueEntry['phase'],
  overrides: Partial<ActiveProbeQueueEntry> = {},
): ActiveProbeQueueEntry {
  return {
    plugin_id: overrides.plugin_id ?? 'plugin-a',
    traffic_request_id: overrides.traffic_request_id ?? `traffic-${requestId}`,
    request_id: requestId,
    phase,
    method: overrides.method ?? 'GET',
    url: overrides.url ?? `https://example.com/items/${requestId}?q=1`,
    probe_label: null,
    target_name: overrides.target_name ?? null,
    target_path: null,
    target_location: null,
    probe_value: null,
    technique: null,
    probe_class: 'fast',
    probe_priority: 1,
    cooldown_key: overrides.cooldown_key ?? 'example.com/items',
    cooldown_wait_ms: null,
    jitter_wait_ms: null,
    total_wait_ms: overrides.total_wait_ms ?? null,
    adaptive_penalty_ms: 0,
    status: overrides.status ?? null,
    error: overrides.error ?? null,
    reason: overrides.reason ?? null,
    active_slots: overrides.active_slots ?? 1,
    max_concurrent_per_host: overrides.max_concurrent_per_host ?? 2,
    queue_depth: overrides.queue_depth ?? 0,
    response_elapsed_ms: overrides.response_elapsed_ms ?? null,
    queued_at: overrides.queued_at ?? '2026-04-27T10:00:00.000Z',
    scheduled_at: null,
    dispatch_started_at: null,
    finished_at: overrides.finished_at ?? null,
    updated_at: overrides.updated_at ?? '2026-04-27T10:00:00.000Z',
  }
}

describe('buildActiveProbePanelDerivedState', () => {
  it('short-circuits when collapsed', () => {
    const state = buildActiveProbePanelDerivedState({
      collapsed: true,
      pendingEntries: [buildEntry('pending-1', 'queued')],
      runningEntries: [buildEntry('running-1', 'running')],
      recentEntries: [buildEntry('recent-1', 'completed')],
      selectedPluginId: null,
      selectedCooldownKey: null,
      visibleQueueLimit: ACTIVE_PROBE_INITIAL_QUEUE_LIMIT,
      visibleRecentLimit: ACTIVE_PROBE_INITIAL_RECENT_LIMIT,
    })

    expect(state.visibleQueueEntries).toEqual([])
    expect(state.visibleRecentEntries).toEqual([])
    expect(state.hotPaths).toEqual([])
  })

  it('limits rendered queue and recent rows while preserving counts', () => {
    const pendingEntries = Array.from({ length: ACTIVE_PROBE_INITIAL_QUEUE_LIMIT + 12 }, (_, index) =>
      buildEntry(`pending-${index}`, 'queued', {
        queue_depth: index,
        plugin_id: index % 2 === 0 ? 'plugin-a' : 'plugin-b',
        cooldown_key: index % 3 === 0 ? 'example.com/a' : 'example.com/b',
        total_wait_ms: index,
      }),
    )
    const runningEntries = [
      buildEntry('running-1', 'running', {
        plugin_id: 'plugin-c',
        updated_at: '2026-04-27T10:01:00.000Z',
      }),
    ]
    const recentEntries = Array.from({ length: ACTIVE_PROBE_INITIAL_RECENT_LIMIT + 7 }, (_, index) =>
      buildEntry(`recent-${index}`, index % 2 === 0 ? 'completed' : 'failed', {
        status: 200,
        response_elapsed_ms: 100 + index,
        finished_at: '2026-04-27T10:02:00.000Z',
        updated_at: '2026-04-27T10:02:00.000Z',
      }),
    )

    const state = buildActiveProbePanelDerivedState({
      collapsed: false,
      pendingEntries,
      runningEntries,
      recentEntries,
      selectedPluginId: null,
      selectedCooldownKey: null,
      visibleQueueLimit: ACTIVE_PROBE_INITIAL_QUEUE_LIMIT,
      visibleRecentLimit: ACTIVE_PROBE_INITIAL_RECENT_LIMIT,
    })

    expect(state.filteredPendingCount).toBe(pendingEntries.length)
    expect(state.filteredRunningCount).toBe(runningEntries.length)
    expect(state.filteredRecentCount).toBe(recentEntries.length)
    expect(state.visibleQueueEntries).toHaveLength(ACTIVE_PROBE_INITIAL_QUEUE_LIMIT)
    expect(state.queueHiddenCount).toBe(13)
    expect(state.visibleRecentEntries).toHaveLength(ACTIVE_PROBE_INITIAL_RECENT_LIMIT)
    expect(state.recentHiddenCount).toBe(7)
    expect(state.pluginOptions).toEqual(['plugin-a', 'plugin-b', 'plugin-c'])
    expect(state.hotPaths.map(item => item.key)).toContain('example.com/b')
    expect(state.visibleQueueEntries[0]?.pathLabel).toContain('/items/')
  })
})
