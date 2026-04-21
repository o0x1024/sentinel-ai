<template>
  <div ref="trafficRouteRoot" class="relative h-full min-h-0">
    <TrafficWorkbench
      v-if="immersiveDrillModeEnabled"
      ref="trafficViewRef"
    />
    <TrafficLegacyTabs
      v-else
      ref="trafficViewRef"
    />
    <TrafficAssistantOverlay />
  </div>
</template>

<script setup lang="ts">
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import {
  onDeactivated,
  nextTick,
  onActivated,
  onMounted,
  onUnmounted,
  provide,
  ref,
} from 'vue'
import TrafficLegacyTabs from '@/components/traffic/TrafficLegacyTabs.vue'
import TrafficAssistantOverlay from '@/components/traffic/TrafficAssistantOverlay.vue'
import TrafficWorkbench from '@/components/traffic/TrafficWorkbench.vue'
import type { HttpExchangeRequest } from '@/components/traffic/http/model'
import type { TrafficAnalysisViewHandle } from '@/components/traffic/trafficAnalysisViewTypes'
import type { TrafficContextCandidateEvidenceSelection } from '@/components/traffic/trafficContextCandidateTypes'
import {
  COMPARER_TRANSFER_STORAGE_KEY,
  INTRUDER_TRANSFER_STORAGE_KEY,
  parseTransferEnvelope,
  REPEATER_TRANSFER_STORAGE_KEY,
  type TrafficComparePayload,
} from '@/components/traffic/transfers'
import { immersiveDrillModeEnabled } from '@/services/immersiveDrillMode'

defineOptions({
  name: 'TrafficAnalysis',
})

const OPEN_TRAFFIC_HISTORY_REQUEST_EVENT = 'traffic-history:open-request'
const OPEN_TRAFFIC_HISTORY_REQUEST_STORAGE_KEY = 'traffic-history:pending-open-request'

const refreshTrigger = ref(0)
const trafficViewRef = ref<TrafficAnalysisViewHandle | null>(null)
const trafficRouteRoot = ref<HTMLElement | null>(null)

let unlistenOpenHistoryRequest: UnlistenFn | null = null
let lastOpenedHistoryRequestKey: string | null = null
let lastOpenedHistoryRequestAt = 0
const scrollStateByPath = new Map<string, { top: number; left: number }>()

provide('refreshTrigger', refreshTrigger)

function buildHistoryRequestOpenKey(payload: TrafficContextCandidateEvidenceSelection) {
  return `${payload.requestId}:${payload.pane || 'request'}:${payload.matchedLocations.join('|')}:${(payload.searchTerms || []).join('|')}`
}

function parseStoredHistoryRequestPayload(
  raw: string | null,
): TrafficContextCandidateEvidenceSelection | null {
  if (!raw) {
    return null
  }

  try {
    const parsed = JSON.parse(raw)
    if (!Number.isFinite(parsed?.requestId)) {
      return null
    }

    return {
      requestId: parsed.requestId,
      pane: parsed?.pane === 'response' ? 'response' : 'request',
      matchedLocations: Array.isArray(parsed?.matchedLocations)
        ? parsed.matchedLocations.filter((item: unknown): item is string => typeof item === 'string')
        : [],
      searchTerms: Array.isArray(parsed?.searchTerms)
        ? parsed.searchTerms.filter((item: unknown): item is string => typeof item === 'string')
        : [],
    }
  } catch {
    const requestId = Number(raw)
    if (!Number.isFinite(requestId)) {
      return null
    }

    return {
      requestId,
      pane: 'request',
      matchedLocations: [],
      searchTerms: [],
    }
  }
}

async function getTrafficViewHandle() {
  if (trafficViewRef.value) {
    return trafficViewRef.value
  }

  await nextTick()
  return trafficViewRef.value
}

function isScrollableElement(element: HTMLElement) {
  const style = window.getComputedStyle(element)
  const overflowYScrollable = /(auto|scroll|overlay)/.test(style.overflowY)
  const overflowXScrollable = /(auto|scroll|overlay)/.test(style.overflowX)
  return (
    (overflowYScrollable && element.scrollHeight > element.clientHeight)
    || (overflowXScrollable && element.scrollWidth > element.clientWidth)
  )
}

function buildElementPath(element: HTMLElement, root: HTMLElement) {
  if (element === root) {
    return 'root'
  }

  const segments: string[] = []
  let current: HTMLElement | null = element
  while (current && current !== root) {
    const parent = current.parentElement
    if (!parent) {
      return null
    }

    const childIndex = Array.from(parent.children).indexOf(current)
    segments.push(String(childIndex))
    current = parent as HTMLElement
  }

  if (current !== root) {
    return null
  }

  return segments.reverse().join('.')
}

function collectScrollableElements(root: HTMLElement) {
  return [root, ...Array.from(root.querySelectorAll<HTMLElement>('*'))]
    .filter(isScrollableElement)
}

function saveTrafficScrollState() {
  const root = trafficRouteRoot.value
  if (!root) {
    return
  }

  scrollStateByPath.clear()
  for (const element of collectScrollableElements(root)) {
    const path = buildElementPath(element, root)
    if (!path) {
      continue
    }

    scrollStateByPath.set(path, {
      top: element.scrollTop,
      left: element.scrollLeft,
    })
  }
}

function restoreTrafficScrollState() {
  const root = trafficRouteRoot.value
  if (!root || scrollStateByPath.size === 0) {
    return
  }

  for (const element of collectScrollableElements(root)) {
    const path = buildElementPath(element, root)
    if (!path) {
      continue
    }

    const state = scrollStateByPath.get(path)
    if (!state) {
      continue
    }

    element.scrollTop = state.top
    element.scrollLeft = state.left
  }
}

function restoreTrafficScrollStateAfterLayout() {
  void nextTick(() => {
    requestAnimationFrame(() => {
      restoreTrafficScrollState()
      requestAnimationFrame(() => {
        restoreTrafficScrollState()
      })
    })
  })
}

async function forwardToRepeater(request: HttpExchangeRequest) {
  const view = await getTrafficViewHandle()
  view?.sendToRepeater(request)
}

async function forwardToIntruder(request: HttpExchangeRequest) {
  const view = await getTrafficViewHandle()
  view?.sendToIntruder(request)
}

async function forwardToComparer(payload: TrafficComparePayload) {
  const view = await getTrafficViewHandle()
  view?.sendToComparer(payload)
}

function handleTransferStorage(event: StorageEvent) {
  if (event.key === REPEATER_TRANSFER_STORAGE_KEY) {
    const envelope = parseTransferEnvelope<HttpExchangeRequest>(event.newValue)
    if (!envelope) {
      return
    }

    void forwardToRepeater(envelope.payload)
    window.localStorage.removeItem(REPEATER_TRANSFER_STORAGE_KEY)
    return
  }

  if (event.key === INTRUDER_TRANSFER_STORAGE_KEY) {
    const envelope = parseTransferEnvelope<HttpExchangeRequest>(event.newValue)
    if (!envelope) {
      return
    }

    void forwardToIntruder(envelope.payload)
    window.localStorage.removeItem(INTRUDER_TRANSFER_STORAGE_KEY)
    return
  }

  if (event.key === COMPARER_TRANSFER_STORAGE_KEY) {
    const envelope = parseTransferEnvelope<TrafficComparePayload>(event.newValue)
    if (!envelope) {
      return
    }

    void forwardToComparer(envelope.payload)
    window.localStorage.removeItem(COMPARER_TRANSFER_STORAGE_KEY)
  }
}

function consumePendingTransfer<T>(storageKey: string) {
  const envelope = parseTransferEnvelope<T>(window.localStorage.getItem(storageKey))
  if (!envelope) {
    return null
  }

  window.localStorage.removeItem(storageKey)
  return envelope
}

async function processPendingTransfers() {
  const pendingRepeater = consumePendingTransfer<HttpExchangeRequest>(REPEATER_TRANSFER_STORAGE_KEY)
  if (pendingRepeater) {
    await forwardToRepeater(pendingRepeater.payload)
  }

  const pendingIntruder = consumePendingTransfer<HttpExchangeRequest>(INTRUDER_TRANSFER_STORAGE_KEY)
  if (pendingIntruder) {
    await forwardToIntruder(pendingIntruder.payload)
  }

  const pendingComparer = consumePendingTransfer<TrafficComparePayload>(COMPARER_TRANSFER_STORAGE_KEY)
  if (pendingComparer) {
    await forwardToComparer(pendingComparer.payload)
  }
}

async function openHistoryRequest(payload: TrafficContextCandidateEvidenceSelection) {
  if (!Number.isFinite(payload.requestId)) {
    return
  }

  const now = Date.now()
  const openKey = buildHistoryRequestOpenKey(payload)
  if (lastOpenedHistoryRequestKey === openKey && now - lastOpenedHistoryRequestAt < 600) {
    return
  }

  lastOpenedHistoryRequestKey = openKey
  lastOpenedHistoryRequestAt = now

  const view = await getTrafficViewHandle()
  await view?.openHistoryRequest(payload)
}

onMounted(async () => {
  window.addEventListener('storage', handleTransferStorage)
  await processPendingTransfers()
  restoreTrafficScrollStateAfterLayout()

  const pendingPayload = parseStoredHistoryRequestPayload(
    window.sessionStorage.getItem(OPEN_TRAFFIC_HISTORY_REQUEST_STORAGE_KEY),
  )
  if (pendingPayload) {
    window.sessionStorage.removeItem(OPEN_TRAFFIC_HISTORY_REQUEST_STORAGE_KEY)
    await openHistoryRequest(pendingPayload)
  }

  void listen<TrafficContextCandidateEvidenceSelection>(
    OPEN_TRAFFIC_HISTORY_REQUEST_EVENT,
    async event => {
      if (!event.payload) {
        return
      }

      await openHistoryRequest({
        requestId: event.payload.requestId,
        pane: event.payload.pane || 'request',
        matchedLocations: event.payload.matchedLocations || [],
        searchTerms: event.payload.searchTerms || [],
      })
    },
  ).then(unlisten => {
    unlistenOpenHistoryRequest = unlisten
  })
})

onActivated(() => {
  void processPendingTransfers()
  restoreTrafficScrollStateAfterLayout()
  refreshTrigger.value += 1
})

onDeactivated(() => {
  saveTrafficScrollState()
})

onUnmounted(() => {
  saveTrafficScrollState()
  window.removeEventListener('storage', handleTransferStorage)
  unlistenOpenHistoryRequest?.()
  unlistenOpenHistoryRequest = null
})
</script>
