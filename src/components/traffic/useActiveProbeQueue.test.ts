import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Event } from '@tauri-apps/api/event'
import { useActiveProbeQueue } from './useActiveProbeQueue'
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
    method: 'GET',
    url: overrides.url ?? `https://example.com/${requestId}`,
    probe_label: null,
    target_name: null,
    target_path: null,
    target_location: null,
    probe_value: null,
    technique: null,
    probe_class: 'fast',
    probe_priority: 1,
    cooldown_key: 'example.com',
    cooldown_wait_ms: null,
    jitter_wait_ms: null,
    total_wait_ms: null,
    adaptive_penalty_ms: 0,
    status: null,
    error: null,
    reason: null,
    active_slots: 1,
    max_concurrent_per_host: 2,
    queue_depth: overrides.queue_depth ?? 0,
    response_elapsed_ms: null,
    queued_at: overrides.queued_at ?? '2026-04-27T10:00:00.000Z',
    scheduled_at: null,
    dispatch_started_at: overrides.dispatch_started_at ?? null,
    finished_at: null,
    updated_at: overrides.updated_at ?? '2026-04-27T10:00:00.000Z',
  }
}

describe('useActiveProbeQueue', () => {
  const mockInvoke = global.testUtils.mockInvoke
  const mockListen = global.testUtils.mockListen
  let queuedFrame: FrameRequestCallback | null = null
  let frameId = 0

  beforeEach(() => {
    queuedFrame = null
    frameId = 0
    mockInvoke.mockReset()
    mockListen.mockReset()
    mockInvoke.mockResolvedValue({
      success: true,
      data: {
        pending: [],
        running: [],
        recent: [],
      },
    })
    vi.stubGlobal('requestAnimationFrame', vi.fn((callback: FrameRequestCallback) => {
      queuedFrame = callback
      frameId += 1
      return frameId
    }))
    vi.stubGlobal('cancelAnimationFrame', vi.fn())
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('batches multiple queue events into a single flush', async () => {
    let queueListener: ((event: Event<unknown>) => void) | null = null
    mockListen.mockImplementation(async (_eventName: string, callback: (event: Event<unknown>) => void) => {
      queueListener = callback
      return vi.fn()
    })

    const queue = useActiveProbeQueue()
    await queue.start()

    expect(queue.pendingEntries.value).toEqual([])

    queueListener?.({
      payload: {
        entries: [
          buildEntry('pending-2', 'queued', { queue_depth: 2 }),
          buildEntry('pending-1', 'queued', { queue_depth: 1 }),
        ],
      },
    } as Event<unknown>)

    expect(requestAnimationFrame).toHaveBeenCalledTimes(1)
    expect(queue.pendingEntries.value).toEqual([])

    queuedFrame?.(performance.now())

    expect(queue.pendingEntries.value.map(entry => entry.request_id)).toEqual(['pending-1', 'pending-2'])
    expect(queue.queuedCount.value).toBe(2)
  })
})
