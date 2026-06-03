<template>
  <div ref="trafficRouteRoot" class="relative h-full min-h-0">
    <TrafficWorkbench ref="trafficViewRef" />
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
import TrafficAssistantOverlay from '@/components/traffic/TrafficAssistantOverlay.vue'
import TrafficWorkbench from '@/components/traffic/TrafficWorkbench.vue'
import type { HttpExchangeRequest } from '@/components/traffic/http/model'
import type { TrafficAnalysisViewHandle } from '@/components/traffic/trafficAnalysisViewTypes'
import type { TrafficContextCandidateEvidenceSelection } from '@/components/traffic/trafficContextCandidateTypes'
import type { TrafficComparePayload } from '@/components/traffic/transfers'
import {
  TRAFFIC_LAUNCH_QUEUE_EVENT,
  useTrafficLaunchQueueStore,
} from '@/components/traffic/workbench/stores/useTrafficLaunchQueueStore'

defineOptions({
  name: 'TrafficAnalysis',
})

const OPEN_TRAFFIC_HISTORY_REQUEST_EVENT = 'traffic-history:open-request'
const OPEN_TRAFFIC_HISTORY_REQUEST_STORAGE_KEY = 'traffic-history:pending-open-request'

const refreshTrigger = ref(0)
const trafficViewRef = ref<TrafficAnalysisViewHandle | null>(null)
const trafficRouteRoot = ref<HTMLElement | null>(null)
const trafficLaunchQueue = useTrafficLaunchQueueStore()

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
  if (!trafficViewRef.value) {
    throw new Error('Traffic workbench is not ready')
  }
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

async function createDraftFromRequest(request: HttpExchangeRequest) {
  const view = await getTrafficViewHandle()
  await view.createDraftFromRequest(request)
}

async function createAttackWorkspaceFromRequest(request: HttpExchangeRequest) {
  const view = await getTrafficViewHandle()
  await view.createAttackWorkspaceFromRequest(request)
}

async function openCompare(payload: TrafficComparePayload) {
  const view = await getTrafficViewHandle()
  await view.openCompare(payload)
}

async function processPendingTransfers() {
  const view = await getTrafficViewHandle()
  const pending = trafficLaunchQueue.consumeLaunchQueue()

  for (const request of pending.repeaterRequests) {
    await view.createDraftFromRequest(request)
  }

  for (const request of pending.intruderRequests) {
    await view.createAttackWorkspaceFromRequest(request)
  }

  for (const payload of pending.comparePayloads) {
    await view.openCompare(payload)
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
  await view.openHistoryRequest(payload)
}

function handleSecurityEvidenceTransferQueued() {
  void processPendingTransfers().catch(error => {
    console.error('Failed to process queued security evidence transfer', error)
  })
}

onMounted(async () => {
  window.addEventListener(TRAFFIC_LAUNCH_QUEUE_EVENT, handleSecurityEvidenceTransferQueued)
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
  window.removeEventListener(TRAFFIC_LAUNCH_QUEUE_EVENT, handleSecurityEvidenceTransferQueued)
  unlistenOpenHistoryRequest?.()
  unlistenOpenHistoryRequest = null
})
</script>
