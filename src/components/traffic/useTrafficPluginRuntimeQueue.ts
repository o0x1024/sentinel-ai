import { onBeforeUnmount, onMounted, shallowRef } from 'vue'
import { getTrafficPluginRuntimeQueueStats } from '@/api/trafficPluginRuntimeQueue'
import type {
  TrafficPluginRuntimeQueueHistorySnapshot,
  TrafficPluginRuntimeQueueStatsSnapshot,
} from './trafficPluginRuntimeQueueTypes'
import {
  appendTrafficPluginRuntimeQueueHistory,
  createEmptyTrafficPluginRuntimeQueueHistory,
} from './trafficPluginRuntimeQueueHistorySupport'

const POLL_INTERVAL_MS = 3000

const sharedSnapshot = shallowRef<TrafficPluginRuntimeQueueStatsSnapshot | null>(null)
const sharedHistory = shallowRef<TrafficPluginRuntimeQueueHistorySnapshot>(
  createEmptyTrafficPluginRuntimeQueueHistory()
)
const sharedLoading = shallowRef(false)
const sharedError = shallowRef<string | null>(null)
const sharedLastUpdatedAt = shallowRef<string | null>(null)

let refreshTimer: number | null = null
let inFlight = false
let subscriberCount = 0

async function refreshSharedQueueState() {
  if (inFlight) {
    return
  }

  inFlight = true
  sharedLoading.value = true
  sharedError.value = null
  try {
    const nextSnapshot = await getTrafficPluginRuntimeQueueStats()
    const recordedAt = new Date().toISOString()
    sharedSnapshot.value = nextSnapshot
    sharedHistory.value = appendTrafficPluginRuntimeQueueHistory(
      sharedHistory.value,
      nextSnapshot,
      recordedAt
    )
    sharedLastUpdatedAt.value = recordedAt
  } catch (refreshError) {
    sharedError.value = refreshError instanceof Error ? refreshError.message : String(refreshError)
  } finally {
    sharedLoading.value = false
    inFlight = false
  }
}

function startSharedPolling() {
  subscriberCount += 1
  void refreshSharedQueueState()
  if (refreshTimer != null || typeof window === 'undefined') {
    return
  }
  refreshTimer = window.setInterval(() => {
    void refreshSharedQueueState()
  }, POLL_INTERVAL_MS)
}

function stopSharedPolling() {
  subscriberCount = Math.max(0, subscriberCount - 1)
  if (subscriberCount > 0) {
    return
  }
  if (refreshTimer != null && typeof window !== 'undefined') {
    window.clearInterval(refreshTimer)
  }
  refreshTimer = null
}

export function useTrafficPluginRuntimeQueue() {
  onMounted(startSharedPolling)
  onBeforeUnmount(stopSharedPolling)

  return {
    snapshot: sharedSnapshot,
    history: sharedHistory,
    loading: sharedLoading,
    error: sharedError,
    lastUpdatedAt: sharedLastUpdatedAt,
    refresh: refreshSharedQueueState,
  }
}
