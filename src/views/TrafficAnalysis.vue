<template>
  <div class="page-content-padded flex flex-col h-[calc(100vh-4rem)]">
    <!-- Tab 切换 -->
    <div
      class="tabs tabs-boxed bg-base-200 flex-shrink-0 mb-4"
      role="tablist"
      :aria-label="$t('trafficAnalysis.ariaLabels.trafficAnalysisTabs')"
    >
      <button
        type="button"
        class="tab"
        role="tab"
        :aria-selected="activeTab === 'control'"
        :class="{
          'tab-active': activeTab === 'control',
          'text-error border border-error/40 bg-error/10':
            activeTab !== 'control' && controlInterceptCount > 0,
          'control-tab-pulse': controlTabPulse,
        }"
        @click="activeTab = 'control'"
      >
        <i class="fas fa-sliders-h mr-2"></i>
        {{ $t('trafficAnalysis.tabs.control') }}
        <span v-if="controlInterceptCount > 0" class="badge badge-xs badge-error ml-1">{{
          controlInterceptCount
        }}</span>
      </button>
      <button
        type="button"
        class="tab"
        role="tab"
        :aria-selected="activeTab === 'proxyhistory'"
        :class="{ 'tab-active': activeTab === 'proxyhistory' }"
        @click="activeTab = 'proxyhistory'"
      >
        <i class="fas fa-history mr-2"></i>
        {{ $t('trafficAnalysis.tabs.history') }}
      </button>
      <button
        type="button"
        class="tab"
        role="tab"
        :aria-selected="activeTab === 'repeater'"
        :class="{ 'tab-active': activeTab === 'repeater' }"
        @click="activeTab = 'repeater'"
      >
        <i class="fas fa-redo mr-2"></i>
        {{ $t('trafficAnalysis.tabs.repeater') }}
        <span v-if="repeaterCount > 0" class="badge badge-xs badge-primary ml-1">{{
          repeaterCount
        }}</span>
      </button>
      <button
        type="button"
        class="tab"
        role="tab"
        :aria-selected="activeTab === 'comparer'"
        :class="{ 'tab-active': activeTab === 'comparer' }"
        @click="activeTab = 'comparer'"
      >
        <i class="fas fa-not-equal mr-2"></i>
        {{ $t('trafficAnalysis.tabs.comparer') }}
        <span v-if="comparerCount > 0" class="badge badge-xs badge-accent ml-1">{{
          comparerCount
        }}</span>
      </button>
      <button
        type="button"
        class="tab"
        role="tab"
        :aria-selected="activeTab === 'intruder'"
        :class="{ 'tab-active': activeTab === 'intruder' }"
        @click="activeTab = 'intruder'"
      >
        <i class="fas fa-crosshairs mr-2"></i>
        {{ $t('trafficAnalysis.tabs.intruder') }}
        <span v-if="intruderCount > 0" class="badge badge-xs badge-secondary ml-1">{{
          intruderCount
        }}</span>
      </button>
      <button
        type="button"
        class="tab"
        role="tab"
        :aria-selected="activeTab === 'proxifier'"
        :class="{ 'tab-active': activeTab === 'proxifier' }"
        @click="activeTab = 'proxifier'"
      >
        <i class="fas fa-network-wired mr-2"></i>
        {{ $t('trafficAnalysis.tabs.proxifier') }}
      </button>
      <button
        type="button"
        class="tab"
        role="tab"
        :aria-selected="activeTab === 'capture'"
        :class="{ 'tab-active': activeTab === 'capture' }"
        @click="activeTab = 'capture'"
      >
        <i class="fas fa-broadcast-tower mr-2"></i>
        {{ $t('trafficAnalysis.tabs.capture') }}
      </button>
      <button
        type="button"
        class="tab"
        role="tab"
        :aria-selected="activeTab === 'proxyconfig'"
        :class="{ 'tab-active': activeTab === 'proxyconfig' }"
        @click="activeTab = 'proxyconfig'"
      >
        <i class="fas fa-cog mr-2"></i>
        {{ $t('trafficAnalysis.tabs.proxyConfig') }}
      </button>
    </div>

    <!-- 内容区域：使用 v-show 避免组件销毁重建，保留临时数据 -->
    <div class="flex-1 min-h-0 relative">
      <TrafficControl
        v-show="activeTab === 'control'"
        @openResponseInterceptionSettings="handleOpenResponseInterceptionSettings"
        @interceptQueueChanged="handleInterceptQueueChanged"
        @sendToRepeater="handleSendToRepeater"
        @sendDraftRequestToComparer="handleSendDraftRequestToComparer"
        @sendToIntruder="handleSendToIntruder"
        class="h-full absolute inset-0 overflow-auto"
      />
      <ProxyHistory
        ref="proxyHistoryRef"
        v-show="activeTab === 'proxyhistory'"
        @sendToRepeater="handleSendToRepeater"
        @sendToIntruder="handleSendToIntruder"
        @sendDraftRequestToComparer="handleSendDraftRequestToComparer"
        @sendToComparer="handleSendToComparer"
        @addFilterRule="handleAddFilterRule"
        class="h-full absolute inset-0 overflow-auto"
      />
      <ProxyRepeater
        v-show="activeTab === 'repeater'"
        ref="repeaterRef"
        :initialRequest="pendingRepeaterRequest"
        @sendToComparer="handleSendToComparer"
        @sendDraftRequestToComparer="handleSendDraftRequestToComparer"
        @sendToIntruder="handleSendToIntruder"
        class="h-full absolute inset-0 overflow-auto"
      />
      <ProxyComparer
        v-show="activeTab === 'comparer'"
        ref="comparerRef"
        @sendToRepeater="handleSendToRepeater"
        class="h-full absolute inset-0 overflow-auto"
      />
      <ProxyIntruder
        v-show="activeTab === 'intruder'"
        ref="intruderRef"
        :initialRequest="pendingIntruderRequest"
        @sendToRepeater="handleSendToRepeater"
        @sendToComparer="handleSendToComparer"
        @sendDraftRequestToComparer="handleSendDraftRequestToComparer"
        class="h-full absolute inset-0 overflow-auto"
      />
      <ProxifierPanel v-show="activeTab === 'proxifier'" class="h-full absolute inset-0" />
      <PacketCapture v-show="activeTab === 'capture'" class="h-full absolute inset-0" />
      <ProxyConfiguration
        ref="proxyConfigRef"
        v-show="activeTab === 'proxyconfig'"
        @filterRuleAdded="handleFilterRuleAdded"
        class="h-full absolute inset-0 overflow-auto"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import {
  ref,
  onMounted,
  onActivated,
  onDeactivated,
  onErrorCaptured,
  onUnmounted,
  watch,
  provide,
  nextTick,
} from 'vue'
import TrafficControl from '../components/traffic/ProxyIntercept.vue'
import ProxyHistory from '../components/traffic/ProxyHistory.vue'
import ProxyRepeater from '../components/traffic/ProxyRepeater.vue'
import ProxyComparer from '../components/traffic/ProxyComparer.vue'
import ProxyIntruder from '../components/traffic/ProxyIntruder.vue'
import ProxyConfiguration from '../components/traffic/ProxyConfiguration.vue'
import ProxifierPanel from '../components/traffic/ProxifierPanel.vue'
import PacketCapture from '../components/traffic/PacketCapture.vue'
import {
  COMPARER_TRANSFER_STORAGE_KEY,
  INTRUDER_TRANSFER_STORAGE_KEY,
  parseTransferEnvelope,
  REPEATER_TRANSFER_STORAGE_KEY,
  type TrafficComparerDraftRequestInput,
  type TrafficComparePayload,
} from '../components/traffic/transfers'
import type { HttpExchangeRequest } from '../components/traffic/http/model'
import type { TrafficContextCandidateEvidenceSelection } from '../components/traffic/trafficContextCandidateTypes'

const activeTab = ref<
  | 'control'
  | 'proxyhistory'
  | 'repeater'
  | 'comparer'
  | 'intruder'
  | 'proxifier'
  | 'capture'
  | 'proxyconfig'
>('proxyhistory')
const isDevelopment = ref(import.meta.env.DEV)
const componentError = ref<string | null>(null)
const refreshTrigger = ref(0)
const repeaterRef = ref<InstanceType<typeof ProxyRepeater> | null>(null)
const comparerRef = ref<InstanceType<typeof ProxyComparer> | null>(null)
const intruderRef = ref<InstanceType<typeof ProxyIntruder> | null>(null)
const proxyConfigRef = ref<InstanceType<typeof ProxyConfiguration> | null>(null)
const proxyHistoryRef = ref<InstanceType<typeof ProxyHistory> | null>(null)
const pendingRepeaterRequest = ref<HttpExchangeRequest | undefined>(undefined)
const pendingIntruderRequest = ref<HttpExchangeRequest | undefined>(undefined)
const repeaterCount = ref(0)
const comparerCount = ref(0)
const intruderCount = ref(0)
const controlInterceptCount = ref(0)
const controlTabPulse = ref(false)
const OPEN_TRAFFIC_HISTORY_REQUEST_EVENT = 'traffic-history:open-request'
const OPEN_TRAFFIC_HISTORY_REQUEST_STORAGE_KEY = 'traffic-history:pending-open-request'
let controlTabPulseTimeout: ReturnType<typeof setTimeout> | null = null
let unlistenOpenHistoryRequest: UnlistenFn | null = null
let lastOpenedHistoryRequestKey: string | null = null
let lastOpenedHistoryRequestAt = 0

defineOptions({
  name: 'TrafficAnalysis',
})

// 提供刷新触发器给子组件
provide('refreshTrigger', refreshTrigger)

// 处理发送到 Repeater 的请求
function handleSendToRepeater(request: HttpExchangeRequest) {
  console.log('[TrafficAnalysis] Sending to repeater:', request)

  // 如果当前在 Repeater 页面，直接调用方法
  if (activeTab.value === 'repeater' && repeaterRef.value) {
    repeaterRef.value.addRequestFromHistory(request)
  } else {
    // 否则先保存请求，然后切换到 Repeater 页面
    pendingRepeaterRequest.value = request
    activeTab.value = 'repeater'
  }

  repeaterCount.value++
}

function handleSendToIntruder(request: HttpExchangeRequest) {
  console.log('[TrafficAnalysis] Sending to intruder:', request)

  if (activeTab.value === 'intruder' && intruderRef.value) {
    intruderRef.value.addRequestFromHistory(request)
  } else {
    pendingIntruderRequest.value = request
    activeTab.value = 'intruder'
  }

  intruderCount.value++
}

function handleSendToComparer(payload: TrafficComparePayload) {
  if (activeTab.value === 'comparer' && comparerRef.value) {
    comparerRef.value.addComparison(payload)
  } else {
    activeTab.value = 'comparer'
    requestAnimationFrame(() => {
      comparerRef.value?.addComparison(payload)
    })
  }

  comparerCount.value++
}

function handleSendDraftRequestToComparer(payload: TrafficComparerDraftRequestInput) {
  if (activeTab.value === 'comparer' && comparerRef.value) {
    comparerRef.value.addDraftRequest(payload)
    return
  }

  activeTab.value = 'comparer'
  requestAnimationFrame(() => {
    comparerRef.value?.addDraftRequest(payload)
  })
}

function handleTransferStorage(event: StorageEvent) {
  if (event.key === REPEATER_TRANSFER_STORAGE_KEY) {
    const envelope = parseTransferEnvelope<HttpExchangeRequest>(event.newValue)
    if (!envelope) {
      return
    }

    handleSendToRepeater(envelope.payload)
    window.localStorage.removeItem(REPEATER_TRANSFER_STORAGE_KEY)
    return
  }

  if (event.key === INTRUDER_TRANSFER_STORAGE_KEY) {
    const envelope = parseTransferEnvelope<HttpExchangeRequest>(event.newValue)
    if (!envelope) {
      return
    }

    handleSendToIntruder(envelope.payload)
    window.localStorage.removeItem(INTRUDER_TRANSFER_STORAGE_KEY)
    return
  }

  if (event.key === COMPARER_TRANSFER_STORAGE_KEY) {
    const envelope = parseTransferEnvelope<TrafficComparePayload>(event.newValue)
    if (!envelope) {
      return
    }

    handleSendToComparer(envelope.payload)
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

function processPendingTransfers() {
  const pendingRepeater = consumePendingTransfer<HttpExchangeRequest>(REPEATER_TRANSFER_STORAGE_KEY)
  if (pendingRepeater) {
    handleSendToRepeater(pendingRepeater.payload)
  }

  const pendingIntruder = consumePendingTransfer<HttpExchangeRequest>(INTRUDER_TRANSFER_STORAGE_KEY)
  if (pendingIntruder) {
    handleSendToIntruder(pendingIntruder.payload)
  }

  const pendingComparer = consumePendingTransfer<TrafficComparePayload>(
    COMPARER_TRANSFER_STORAGE_KEY
  )
  if (pendingComparer) {
    handleSendToComparer(pendingComparer.payload)
  }
}

// 处理添加过滤规则
interface FilterRule {
  matchType: string
  condition: string
  relationship?: string
}

function handleAddFilterRule(rule: FilterRule) {
  console.log('[TrafficAnalysis] Adding filter rule:', rule)

  if (proxyConfigRef.value) {
    proxyConfigRef.value.addRequestFilterRule(
      rule.matchType,
      rule.condition,
      rule.relationship || 'matches'
    )
    // Don't switch to proxy config tab, stay on current tab
  } else {
    console.error('[TrafficAnalysis] ProxyConfiguration ref not available')
  }
}

// 处理过滤规则添加完成事件
function handleFilterRuleAdded(rule: FilterRule) {
  console.log('[TrafficAnalysis] Filter rule added, removing matching records:', rule)

  if (proxyHistoryRef.value) {
    proxyHistoryRef.value.removeMatchingRecords({
      matchType: rule.matchType,
      condition: rule.condition,
      relationship: rule.relationship || 'matches',
    })
  }
}

async function handleOpenResponseInterceptionSettings() {
  activeTab.value = 'proxyconfig'
  await nextTick()
  await proxyConfigRef.value?.openResponseInterceptionRules?.()
}

function handleInterceptQueueChanged(count: number) {
  if (count > controlInterceptCount.value && activeTab.value !== 'control') {
    controlTabPulse.value = false
    if (controlTabPulseTimeout) {
      clearTimeout(controlTabPulseTimeout)
    }

    requestAnimationFrame(() => {
      controlTabPulse.value = true
      controlTabPulseTimeout = setTimeout(() => {
        controlTabPulse.value = false
        controlTabPulseTimeout = null
      }, 1200)
    })
  }

  controlInterceptCount.value = count
}

function buildHistoryRequestOpenKey(payload: TrafficContextCandidateEvidenceSelection) {
  return `${payload.requestId}:${payload.pane || 'request'}:${payload.matchedLocations.join('|')}:${(payload.searchTerms || []).join('|')}`
}

function parseStoredHistoryRequestPayload(
  raw: string | null
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
        ? parsed.matchedLocations.filter(
            (item: unknown): item is string => typeof item === 'string'
          )
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

async function openHistoryRequestById(payload: TrafficContextCandidateEvidenceSelection) {
  const requestId = payload.requestId
  if (!Number.isFinite(requestId)) {
    return
  }

  const now = Date.now()
  const openKey = buildHistoryRequestOpenKey(payload)
  if (lastOpenedHistoryRequestKey === openKey && now - lastOpenedHistoryRequestAt < 600) {
    return
  }

  lastOpenedHistoryRequestKey = openKey
  lastOpenedHistoryRequestAt = now
  activeTab.value = 'proxyhistory'
  await nextTick()
  await proxyHistoryRef.value?.openRequestById?.(
    requestId,
    payload.matchedLocations,
    payload.pane || 'request',
    payload.searchTerms || []
  )
}

// 监听 Tab 切换，清除待处理请求
watch(activeTab, newTab => {
  console.log('[TrafficAnalysis] activeTab ->', newTab)
  if (newTab !== 'repeater') {
    // 切换离开 Repeater 时清除待处理请求
    pendingRepeaterRequest.value = undefined
  }
  if (newTab !== 'intruder') {
    pendingIntruderRequest.value = undefined
  }
})

onMounted(() => {
  console.log('traffic view mounted, activeTab:', activeTab.value)
  window.addEventListener('storage', handleTransferStorage)
  processPendingTransfers()

  const pendingPayload = parseStoredHistoryRequestPayload(
    window.sessionStorage.getItem(OPEN_TRAFFIC_HISTORY_REQUEST_STORAGE_KEY)
  )
  if (pendingPayload) {
    window.sessionStorage.removeItem(OPEN_TRAFFIC_HISTORY_REQUEST_STORAGE_KEY)
    void openHistoryRequestById(pendingPayload)
  }

  void listen<TrafficContextCandidateEvidenceSelection>(
    OPEN_TRAFFIC_HISTORY_REQUEST_EVENT,
    async event => {
      if (!event.payload) {
        return
      }
      await openHistoryRequestById({
        requestId: event.payload.requestId,
        pane: event.payload.pane || 'request',
        matchedLocations: event.payload.matchedLocations || [],
        searchTerms: event.payload.searchTerms || [],
      })
    }
  ).then(unlisten => {
    unlistenOpenHistoryRequest = unlisten
  })
})

// 当组件从缓存中激活时，触发刷新
onActivated(() => {
  console.log('traffic view activated, triggering refresh')
  processPendingTransfers()
  refreshTrigger.value++
})

// 当组件被缓存时
onDeactivated(() => {
  console.log('traffic view deactivated')
})

onUnmounted(() => {
  window.removeEventListener('storage', handleTransferStorage)
  unlistenOpenHistoryRequest?.()
  unlistenOpenHistoryRequest = null
  if (controlTabPulseTimeout) {
    clearTimeout(controlTabPulseTimeout)
  }
})

// 捕获子组件错误
onErrorCaptured((err, instance, info) => {
  console.error('Component error caught:', err, info)
  const message = err instanceof Error ? err.message : String(err)
  componentError.value = `组件加载失败: ${message}`
  return false // 阻止错误继续传播
})
</script>

<style scoped>
.page-content-padded {
  padding: 1rem 1.5rem;
}

.tabs {
  border-radius: 0.5rem;
  padding: 0.25rem;
}

.tab {
  border-radius: 0.375rem;
  font-weight: 500;
}

.tab-active {
  background-color: hsl(var(--p));
  color: hsl(var(--pc));
}

.control-tab-pulse {
  animation: control-tab-pulse 1.2s ease-out 1;
}

@keyframes control-tab-pulse {
  0% {
    transform: scale(1);
    box-shadow: 0 0 0 0 hsl(var(--er) / 0.55);
  }
  35% {
    transform: scale(1.03);
    box-shadow: 0 0 0 10px hsl(var(--er) / 0.18);
  }
  100% {
    transform: scale(1);
    box-shadow: 0 0 0 0 hsl(var(--er) / 0);
  }
}
</style>
