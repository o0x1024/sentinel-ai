import { computed, ref } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getActiveProbeQueueSnapshot } from '@/api/trafficActiveProbe'
import {
  normalizeActiveProbeQueueEntry,
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

function removeEntry(
  entries: ActiveProbeQueueEntry[],
  requestId: string,
): ActiveProbeQueueEntry[] {
  return entries.filter(entry => entry.request_id !== requestId)
}

export function useActiveProbeQueue() {
  const pendingEntries = ref<ActiveProbeQueueEntry[]>([])
  const runningEntries = ref<ActiveProbeQueueEntry[]>([])
  const recentEntries = ref<ActiveProbeQueueEntry[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  let unlistenQueue: UnlistenFn | null = null

  const queuedCount = computed(() => pendingEntries.value.length)

  async function refresh() {
    loading.value = true
    error.value = null
    try {
      const snapshot = await getActiveProbeQueueSnapshot()
      pendingEntries.value = sortPending(snapshot.pending)
      runningEntries.value = sortRunning(snapshot.running)
      recentEntries.value = sortRecent(snapshot.recent)
    } catch (refreshError) {
      error.value = refreshError instanceof Error ? refreshError.message : String(refreshError)
    } finally {
      loading.value = false
    }
  }

  function upsertEntry(entry: ActiveProbeQueueEntry) {
    pendingEntries.value = removeEntry(pendingEntries.value, entry.request_id)
    runningEntries.value = removeEntry(runningEntries.value, entry.request_id)
    recentEntries.value = removeEntry(recentEntries.value, entry.request_id)

    switch (entry.phase) {
      case 'queued':
      case 'scheduled':
        pendingEntries.value = sortPending([...pendingEntries.value, entry])
        break
      case 'running':
        runningEntries.value = sortRunning([...runningEntries.value, entry])
        break
      case 'completed':
      case 'failed':
      case 'cancelled':
        recentEntries.value = sortRecent([...recentEntries.value, entry])
        break
    }
  }

  async function start() {
    await refresh()
    if (unlistenQueue) {
      return
    }
    unlistenQueue = await listen(ACTIVE_PROBE_QUEUE_EVENT, event => {
      const entry = normalizeActiveProbeQueueEntry(event.payload)
      if (!entry) {
        return
      }
      upsertEntry(entry)
    })
  }

  function stop() {
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
