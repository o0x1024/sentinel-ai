import { computed, shallowRef } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getActiveProbeQueueSnapshot } from '@/api/trafficActiveProbe'
import {
  normalizeActiveProbeQueueEventPayload,
  type ActiveProbeQueueEntry,
} from './trafficActiveProbeTypes'

const ACTIVE_PROBE_QUEUE_EVENT = 'traffic:active-probe-queue-updated'

function parseTime(value?: string | null): number {
  if (!value) {
    return 0
  }
  const parsed = Date.parse(value)
  return Number.isFinite(parsed) ? parsed : 0
}

function pendingPhaseRank(phase: ActiveProbeQueueEntry['phase']): number {
  switch (phase) {
    case 'scheduled':
      return 0
    case 'queued':
      return 1
    default:
      return 9
  }
}

function sortPending(entries: ActiveProbeQueueEntry[]): ActiveProbeQueueEntry[] {
  return [...entries].sort((left, right) => (
    pendingPhaseRank(left.phase) - pendingPhaseRank(right.phase)
    || left.queue_depth - right.queue_depth
    || parseTime(left.queued_at) - parseTime(right.queued_at)
  ))
}

function sortRunning(entries: ActiveProbeQueueEntry[]): ActiveProbeQueueEntry[] {
  return [...entries].sort((left, right) => (
    parseTime(right.dispatch_started_at || right.updated_at)
    - parseTime(left.dispatch_started_at || left.updated_at)
  ))
}

function sortRecent(entries: ActiveProbeQueueEntry[]): ActiveProbeQueueEntry[] {
  return [...entries].sort((left, right) => (
    parseTime(right.finished_at || right.updated_at) - parseTime(left.finished_at || left.updated_at)
  ))
}

export function useActiveProbeQueue() {
  const pendingEntries = shallowRef<ActiveProbeQueueEntry[]>([])
  const runningEntries = shallowRef<ActiveProbeQueueEntry[]>([])
  const recentEntries = shallowRef<ActiveProbeQueueEntry[]>([])
  const loading = shallowRef(false)
  const error = shallowRef<string | null>(null)
  const pendingEntryMap = new Map<string, ActiveProbeQueueEntry>()
  const runningEntryMap = new Map<string, ActiveProbeQueueEntry>()
  const recentEntryMap = new Map<string, ActiveProbeQueueEntry>()
  let unlistenQueue: UnlistenFn | null = null
  let flushHandle: number | null = null

  const queuedCount = computed(() => pendingEntries.value.length)

  function cancelFlush() {
    if (flushHandle == null || typeof window === 'undefined') {
      flushHandle = null
      return
    }
    window.cancelAnimationFrame(flushHandle)
    flushHandle = null
  }

  function flushEntries() {
    flushHandle = null
    pendingEntries.value = sortPending([...pendingEntryMap.values()])
    runningEntries.value = sortRunning([...runningEntryMap.values()])
    recentEntries.value = sortRecent([...recentEntryMap.values()])
  }

  function scheduleFlush() {
    if (flushHandle != null) {
      return
    }
    if (typeof window === 'undefined') {
      flushEntries()
      return
    }
    flushHandle = window.requestAnimationFrame(flushEntries)
  }

  function resetEntryMaps() {
    pendingEntryMap.clear()
    runningEntryMap.clear()
    recentEntryMap.clear()
  }

  function applySnapshot(entries: ActiveProbeQueueEntry[], target: Map<string, ActiveProbeQueueEntry>) {
    target.clear()
    for (const entry of entries) {
      target.set(entry.request_id, entry)
    }
  }

  async function refresh() {
    loading.value = true
    error.value = null
    try {
      const snapshot = await getActiveProbeQueueSnapshot()
      resetEntryMaps()
      applySnapshot(snapshot.pending, pendingEntryMap)
      applySnapshot(snapshot.running, runningEntryMap)
      applySnapshot(snapshot.recent, recentEntryMap)
      flushEntries()
    } catch (refreshError) {
      error.value = refreshError instanceof Error ? refreshError.message : String(refreshError)
    } finally {
      loading.value = false
    }
  }

  function upsertEntry(entry: ActiveProbeQueueEntry) {
    pendingEntryMap.delete(entry.request_id)
    runningEntryMap.delete(entry.request_id)
    recentEntryMap.delete(entry.request_id)

    switch (entry.phase) {
      case 'queued':
      case 'scheduled':
        pendingEntryMap.set(entry.request_id, entry)
        break
      case 'running':
        runningEntryMap.set(entry.request_id, entry)
        break
      case 'completed':
      case 'failed':
      case 'cancelled':
        recentEntryMap.set(entry.request_id, entry)
        break
    }
    scheduleFlush()
  }

  async function start() {
    await refresh()
    if (unlistenQueue) {
      return
    }
    unlistenQueue = await listen(ACTIVE_PROBE_QUEUE_EVENT, event => {
      const payload = normalizeActiveProbeQueueEventPayload(event.payload)
      if (!payload || payload.entries.length === 0) {
        return
      }
      for (const entry of payload.entries) {
        upsertEntry(entry)
      }
    })
  }

  function stop() {
    cancelFlush()
    if (unlistenQueue) {
      unlistenQueue()
      unlistenQueue = null
    }
  }

  return {
    pendingEntries,
    runningEntries,
    recentEntries,
    queuedCount,
    loading,
    error,
    refresh,
    start,
    stop,
  }
}
