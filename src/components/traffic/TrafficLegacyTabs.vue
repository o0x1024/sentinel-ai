<template>
  <div
    ref="legacyTabsRoot"
    class="flex flex-col h-[calc(100vh-var(--app-navbar-height,4rem))]"
    :class="immersiveDrillModeEnabled ? 'px-3 py-3' : 'page-content-padded'"
  >
    <!-- Tab 切换 -->
    <div
      class="tabs tabs-boxed bg-base-200 flex-shrink-0"
      :class="[
        immersiveDrillModeEnabled ? 'tabs-sm rounded-2xl px-1 py-1' : '',
        activeTab === 'proxyhistory' ? 'mb-2' : 'mb-4',
      ]"
      role="tablist"
      :aria-label="$t('trafficAnalysis.ariaLabels.trafficAnalysisTabs')"
    >
      <button
        v-if="isTabVisible('control')"
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
        v-if="isTabVisible('proxyhistory')"
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
        v-if="isTabVisible('repeater')"
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
        v-if="isTabVisible('comparer')"
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
        v-if="isTabVisible('intruder')"
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
        v-if="isTabVisible('proxifier')"
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
        v-if="isTabVisible('capture')"
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
        v-if="isTabVisible('proxyconfig')"
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
        v-if="isTabVisible('control')"
        v-show="activeTab === 'control'"
        @openResponseInterceptionSettings="handleOpenResponseInterceptionSettings"
        @interceptQueueChanged="handleInterceptQueueChanged"
        @sendToRepeater="handleSendToRepeater"
        @sendDraftRequestToComparer="handleSendDraftRequestToComparer"
        @sendToIntruder="handleSendToIntruder"
        class="h-full absolute inset-0 overflow-auto"
      />
      <ProxyHistory
        v-if="isTabVisible('proxyhistory')"
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
        v-if="isTabVisible('repeater')"
        v-show="activeTab === 'repeater'"
        ref="repeaterRef"
        :initialRequest="pendingRepeaterRequest"
        @sendToComparer="handleSendToComparer"
        @sendDraftRequestToComparer="handleSendDraftRequestToComparer"
        @sendToIntruder="handleSendToIntruder"
        class="h-full absolute inset-0 overflow-auto"
      />
      <ProxyComparer
        v-if="isTabVisible('comparer')"
        v-show="activeTab === 'comparer'"
        ref="comparerRef"
        @sendToRepeater="handleSendToRepeater"
        class="h-full absolute inset-0 overflow-auto"
      />
      <ProxyIntruder
        v-if="isTabVisible('intruder')"
        v-show="activeTab === 'intruder'"
        ref="intruderRef"
        :initialRequest="pendingIntruderRequest"
        @sendToRepeater="handleSendToRepeater"
        @sendToComparer="handleSendToComparer"
        @sendDraftRequestToComparer="handleSendDraftRequestToComparer"
        class="h-full absolute inset-0 overflow-auto"
      />
      <ProxifierPanel
        v-if="isTabVisible('proxifier')"
        v-show="activeTab === 'proxifier'"
        class="h-full absolute inset-0"
      />
      <PacketCapture
        v-if="isTabVisible('capture')"
        v-show="activeTab === 'capture'"
        class="h-full absolute inset-0"
      />
      <ProxyConfiguration
        v-if="isTabVisible('proxyconfig')"
        ref="proxyConfigRef"
        v-show="activeTab === 'proxyconfig'"
        @filterRuleAdded="handleFilterRuleAdded"
        class="h-full absolute inset-0 overflow-auto"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import {
  onActivated,
  onDeactivated,
  onMounted,
  ref,
  watch,
  nextTick,
  computed,
  onUnmounted,
} from 'vue'
import TrafficControl from './ProxyIntercept.vue'
import ProxyHistory from './ProxyHistory.vue'
import ProxyRepeater from './ProxyRepeater.vue'
import ProxyComparer from './ProxyComparer.vue'
import ProxyIntruder from './ProxyIntruder.vue'
import ProxyConfiguration from './ProxyConfiguration.vue'
import ProxifierPanel from './ProxifierPanel.vue'
import PacketCapture from './PacketCapture.vue'
import {
  type TrafficComparerDraftRequestInput,
  type TrafficComparePayload,
} from './transfers'
import type { HttpExchangeRequest } from './http/model'
import type { TrafficContextCandidateEvidenceSelection } from './trafficContextCandidateTypes'
import { immersiveDrillModeEnabled } from '@/services/immersiveDrillMode'
import { immersiveDrillTrafficTabs } from '@/services/immersiveDrillPreset'
import type { TrafficAnalysisViewHandle } from './trafficAnalysisViewTypes'

type TrafficTab =
  | 'control'
  | 'proxyhistory'
  | 'repeater'
  | 'comparer'
  | 'intruder'
  | 'proxifier'
  | 'capture'
  | 'proxyconfig'

const allTrafficTabs: TrafficTab[] = [
  'control',
  'proxyhistory',
  'repeater',
  'comparer',
  'intruder',
  'proxifier',
  'capture',
  'proxyconfig',
]

const activeTab = ref<TrafficTab>('proxyhistory')
const legacyTabsRoot = ref<HTMLElement | null>(null)
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
let controlTabPulseTimeout: ReturnType<typeof setTimeout> | null = null
const TRAFFIC_LEGACY_TABS_SCROLL_STORAGE_KEY = 'trafficLegacyTabs.scrollTop'

const visibleTrafficTabs = computed<TrafficTab[]>(() =>
  immersiveDrillModeEnabled.value ? [...immersiveDrillTrafficTabs] : allTrafficTabs,
)

const isTabVisible = (tab: TrafficTab) => visibleTrafficTabs.value.includes(tab)

const ensureVisibleTrafficTab = (tab: TrafficTab) => {
  if (isTabVisible(tab)) {
    activeTab.value = tab
    return
  }

  activeTab.value = 'proxyhistory'
}

const saveLegacyTabsScrollState = () => {
  if (!legacyTabsRoot.value) {
    return
  }

  window.sessionStorage.setItem(
    TRAFFIC_LEGACY_TABS_SCROLL_STORAGE_KEY,
    String(legacyTabsRoot.value.scrollTop),
  )
}

const restoreLegacyTabsScrollState = () => {
  if (!legacyTabsRoot.value) {
    return
  }

  const saved = Number(window.sessionStorage.getItem(TRAFFIC_LEGACY_TABS_SCROLL_STORAGE_KEY) || '0')
  legacyTabsRoot.value.scrollTop = Number.isFinite(saved) ? saved : 0
}

defineOptions({
  name: 'TrafficLegacyTabs',
})

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
  ensureVisibleTrafficTab('proxyconfig')
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
async function openHistoryRequest(payload: TrafficContextCandidateEvidenceSelection) {
  const requestId = payload.requestId
  if (!Number.isFinite(requestId)) {
    return
  }

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

  if (newTab === 'proxyhistory') {
    void nextTick(() => proxyHistoryRef.value?.refreshVirtualLayout?.())
  }
})

watch(visibleTrafficTabs, tabs => {
  if (!tabs.includes(activeTab.value)) {
    activeTab.value = tabs[0] ?? 'proxyhistory'
  }
})

watch(
  () => immersiveDrillModeEnabled.value,
  enabled => {
    if (enabled) {
      activeTab.value = 'proxyhistory'
    }
  },
)

defineExpose<TrafficAnalysisViewHandle>({
  sendToRepeater: handleSendToRepeater,
  sendToIntruder: handleSendToIntruder,
  sendToComparer: handleSendToComparer,
  sendDraftRequestToComparer: handleSendDraftRequestToComparer,
  openHistoryRequest,
})

watch(
  () => activeTab.value,
  () => {
    if (controlTabPulseTimeout && activeTab.value === 'control') {
      clearTimeout(controlTabPulseTimeout)
      controlTabPulseTimeout = null
      controlTabPulse.value = false
    }
  },
)

onUnmounted(() => {
  saveLegacyTabsScrollState()
  legacyTabsRoot.value?.removeEventListener('scroll', saveLegacyTabsScrollState)
  if (controlTabPulseTimeout) {
    clearTimeout(controlTabPulseTimeout)
  }
})

onMounted(() => {
  legacyTabsRoot.value?.addEventListener('scroll', saveLegacyTabsScrollState, { passive: true })
  requestAnimationFrame(() => {
    restoreLegacyTabsScrollState()
  })
})

onDeactivated(() => {
  saveLegacyTabsScrollState()
})

onActivated(() => {
  void nextTick(() => {
    requestAnimationFrame(() => {
      restoreLegacyTabsScrollState()
    })
  })
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
