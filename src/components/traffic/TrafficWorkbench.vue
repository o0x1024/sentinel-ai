<template>
  <div class="traffic-workbench flex h-[calc(100vh-var(--app-navbar-height,4rem))] flex-col px-3 py-3">
    <div ref="workspaceStageRef" class="relative min-h-0 flex-1">
      <div class="workbench-solid-surface h-full overflow-hidden rounded-[30px] border border-base-300/70 shadow-[0_28px_72px_rgba(15,23,42,0.08)]">
        <TrafficHistoryWorkbench
          ref="proxyHistoryRef"
          class="h-full overflow-auto"
          :basket-count="basketItems.length"
          @sendToRepeater="handleSendToRepeaterFromHistory"
          @sendToIntruder="handleSendToIntruderFromHistory"
          @sendDraftRequestToComparer="handleSendDraftRequestToComparerFromHistory"
          @sendToComparer="handleSendToComparerFromHistory"
          @addFilterRule="handleAddFilterRule"
          @addToBasket="handleAddToBasketFromHistory"
        />
      </div>

      <section
        v-if="workbenchToolsMounted"
        v-show="workbenchOpen && activeWorkbenchTool"
        class="workbench-drawer workbench-solid-surface absolute inset-x-4 bottom-4 top-auto z-20 overflow-hidden rounded-[30px] border border-base-300/80 shadow-[0_28px_72px_rgba(15,23,42,0.14)]"
        :style="workbenchDrawerStyle"
      >
        <div
          class="workbench-header-surface relative z-[1] flex h-3.5 cursor-row-resize items-center justify-center border-b border-base-300/60"
          @mousedown="startWorkbenchResize"
        >
          <span class="h-1 w-12 rounded-full bg-base-300/90"></span>
        </div>

        <div class="workbench-header-surface relative z-[1] border-b border-base-300/70 px-4 py-2">
          <div class="flex flex-wrap items-center justify-between gap-2">
            <div class="min-w-0 flex-1">
              <div class="flex min-w-0 flex-wrap items-center gap-2">
                <h3 class="shrink-0 text-base font-semibold text-base-content">
                  {{ activeWorkbenchMeta.title }}
                </h3>
                <span class="rounded-full bg-base-200 px-2.5 py-1 text-[11px] font-medium text-base-content/65">
                  {{ activeWorkbenchMeta.shortTitle }}
                </span>
                <p class="min-w-0 text-xs text-base-content/60">
                  {{ activeWorkbenchMeta.description }}
                </p>
              </div>
              <p v-if="activeWorkbenchSourceLabel" class="mt-1 text-[11px] text-base-content/55">
                来源：{{ activeWorkbenchSourceLabel }}
              </p>
            </div>

            <div class="flex flex-wrap items-center gap-2">
              <button
                v-for="chip in toolChips"
                :key="`drawer-${chip.tool}`"
                type="button"
                class="tool-switch"
                :class="{ 'tool-switch-active': activeWorkbenchTool === chip.tool }"
                @click="openWorkbenchTool(chip.tool)"
              >
                <i :class="`${chip.icon} text-xs`"></i>
                <span>{{ chip.shortLabel }}</span>
                <span v-if="chip.count > 0" class="badge badge-xs badge-primary">{{ chip.count }}</span>
              </button>
              <button type="button" class="btn btn-xs btn-ghost rounded-2xl" @click="closeWorkbench">
                <i class="fas fa-times"></i>
              </button>
            </div>
          </div>
        </div>

        <div class="workbench-content-surface relative min-h-0 flex-1 overflow-hidden">
          <ProxyRepeater
            v-show="activeWorkbenchTool === 'repeater'"
            ref="repeaterRef"
            :initialRequest="pendingRepeaterRequest"
            class="absolute inset-0 h-full overflow-auto"
            @sendToComparer="handleSendToComparerFromRepeater"
            @sendDraftRequestToComparer="handleSendDraftRequestToComparerFromRepeater"
            @sendToIntruder="handleSendToIntruderFromRepeater"
          />
          <ProxyIntruder
            v-show="activeWorkbenchTool === 'intruder'"
            ref="intruderRef"
            :initialRequest="pendingIntruderRequest"
            class="absolute inset-0 h-full overflow-auto"
            @sendToRepeater="handleSendToRepeaterFromIntruder"
            @sendToComparer="handleSendToComparerFromIntruder"
            @sendDraftRequestToComparer="handleSendDraftRequestToComparerFromIntruder"
          />
          <ProxyComparer
            v-show="activeWorkbenchTool === 'comparer'"
            ref="comparerRef"
            class="absolute inset-0 h-full overflow-auto"
            @sendToRepeater="handleSendToRepeaterFromComparer"
          />
        </div>
      </section>

      <section
        v-show="interceptDrawerOpen"
        class="intercept-drawer workbench-solid-surface absolute right-4 top-4 z-30 overflow-hidden rounded-[28px] border border-warning/20 shadow-[0_28px_72px_rgba(15,23,42,0.16)]"
        :style="interceptDrawerStyle"
      >
        <div
          class="drawer-width-resizer drawer-width-resizer-warning absolute bottom-0 left-0 top-0 z-[2]"
          @mousedown="startDrawerWidthResize('intercept', $event)"
        ></div>
        <div class="border-b border-warning/10 px-4 py-3">
          <div class="flex items-start justify-between gap-3">
            <div>
              <p class="text-[11px] font-semibold uppercase tracking-[0.22em] text-warning">
                Intercept Queue
              </p>
              <h3 class="mt-1 text-lg font-semibold text-base-content">
                {{ $t('trafficAnalysis.tabs.control', '代理控制') }}
              </h3>
              <p class="mt-1 text-sm text-base-content/65">
                高频拦截处理留在这里，低频监听器和规则设置收进单独设置抽屉。
              </p>
            </div>
            <div class="flex items-center gap-2">
              <button type="button" class="btn btn-xs btn-outline rounded-2xl" @click="proxySettingsOpen = true">
                <i class="fas fa-cog mr-1"></i>
                代理设置
              </button>
              <button type="button" class="btn btn-sm btn-ghost rounded-2xl" @click="interceptDrawerOpen = false">
                <i class="fas fa-times"></i>
              </button>
            </div>
          </div>
        </div>

        <TrafficControl
          class="h-full max-h-[min(72vh,760px)] overflow-auto"
          @openResponseInterceptionSettings="handleOpenResponseInterceptionSettings"
          @interceptQueueChanged="handleInterceptQueueChanged"
          @sendToRepeater="handleSendToRepeaterFromIntercept"
          @sendDraftRequestToComparer="handleSendDraftRequestToComparerFromIntercept"
          @sendToIntruder="handleSendToIntruderFromIntercept"
          @addToBasket="handleAddToBasketFromIntercept"
        />
      </section>

      <section
        v-show="proxySettingsOpen"
        class="settings-drawer workbench-solid-surface absolute bottom-4 right-4 top-4 z-[32] overflow-hidden rounded-[28px] border border-base-300/80 shadow-[0_28px_72px_rgba(15,23,42,0.16)]"
        :style="settingsDrawerStyle"
      >
        <div
          class="drawer-width-resizer absolute bottom-0 left-0 top-0 z-[2]"
          @mousedown="startDrawerWidthResize('settings', $event)"
        ></div>
        <div class="border-b border-base-300/70 px-4 py-3">
          <div class="flex items-start justify-between gap-3">
            <div>
              <p class="text-[11px] font-semibold uppercase tracking-[0.22em] text-primary/80">
                Proxy Settings
              </p>
              <h3 class="mt-1 text-lg font-semibold text-base-content">
                代理设置
              </h3>
              <p class="mt-1 text-sm text-base-content/65">
                监听器、作用域、响应拦截和规则编辑收进这里，不再打断主工作流。
              </p>
            </div>
            <button type="button" class="btn btn-sm btn-ghost rounded-2xl" @click="proxySettingsOpen = false">
              <i class="fas fa-times"></i>
            </button>
          </div>
        </div>

        <ProxyConfiguration
          ref="proxyConfigRef"
          class="h-full overflow-auto px-4 py-4"
          @filterRuleAdded="handleFilterRuleAdded"
        />
      </section>

      <TrafficWorkbenchBasketDrawer
        :open="basketOpen"
        :items="basketItems"
        @close="basketOpen = false"
        @remove="removeBasketItem"
        @clear="clearBasket"
        @sendToRepeater="sendBasketItemToRepeater"
        @sendToIntruder="sendBasketItemToIntruder"
        @sendAllToRepeater="sendAllBasketItemsToRepeater"
        @sendAllToIntruder="sendAllBasketItemsToIntruder"
        @openHistoryRequest="openHistoryRequestFromBasket"
      />
    </div>

    <div v-if="!immersiveDrillModeEnabled" class="fixed bottom-6 right-6 z-40 flex flex-col gap-3">
      <button
        type="button"
        class="floating-trigger"
        :class="{ 'floating-trigger-active': !workbenchOpen }"
        @click="closeWorkbench"
      >
        <i class="fas fa-history text-sm"></i>
        <span class="floating-trigger-label">
          {{ $t('trafficAnalysis.tabs.history', '历史记录') }}
        </span>
      </button>
      <button
        v-for="chip in toolChips"
        :key="`floating-${chip.tool}`"
        type="button"
        class="floating-trigger"
        :class="{ 'floating-trigger-active': workbenchOpen && activeWorkbenchTool === chip.tool }"
        @click="openWorkbenchTool(chip.tool)"
      >
        <i :class="`${chip.icon} text-sm`"></i>
        <span class="floating-trigger-label">
          {{ chip.label }}
        </span>
        <span v-if="chip.count > 0" class="floating-badge">
          {{ chip.count }}
        </span>
      </button>
      <button
        type="button"
        class="floating-trigger"
        :class="{ 'floating-trigger-warning': controlInterceptCount > 0 }"
        @click="interceptDrawerOpen = !interceptDrawerOpen"
      >
        <i class="fas fa-sliders-h text-sm"></i>
        <span class="floating-trigger-label">
          {{ $t('trafficAnalysis.tabs.control', '代理控制') }}
        </span>
        <span v-if="controlInterceptCount > 0" class="floating-badge">
          {{ controlInterceptCount }}
        </span>
      </button>
      <button
        type="button"
        class="floating-trigger"
        :class="{ 'floating-trigger-active': basketOpen }"
        @click="basketOpen = !basketOpen"
      >
        <i class="fas fa-basket-shopping text-sm"></i>
        <span class="floating-trigger-label">请求篮子</span>
        <span v-if="basketItems.length > 0" class="floating-badge">
          {{ basketItems.length }}
        </span>
      </button>
      <button
        type="button"
        class="floating-trigger"
        :class="{ 'floating-trigger-active': proxySettingsOpen }"
        @click="proxySettingsOpen = !proxySettingsOpen"
      >
        <i class="fas fa-cog text-sm"></i>
        <span class="floating-trigger-label">代理设置</span>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { nextTick, computed, onMounted, onUnmounted, ref, watchEffect } from 'vue'
import { useI18n } from 'vue-i18n'
import ProxyRepeater from './ProxyRepeater.vue'
import ProxyIntruder from './ProxyIntruder.vue'
import ProxyComparer from './ProxyComparer.vue'
import TrafficControl from './ProxyIntercept.vue'
import ProxyConfiguration from './ProxyConfiguration.vue'
import TrafficHistoryWorkbench from './TrafficHistoryWorkbench.vue'
import TrafficWorkbenchBasketDrawer from './TrafficWorkbenchBasketDrawer.vue'
import type { HttpExchangeRequest } from './http/model'
import type { TrafficContextCandidateEvidenceSelection } from './trafficContextCandidateTypes'
import type {
  TrafficComparerDraftRequestInput,
  TrafficComparePayload,
} from './transfers'
import type { TrafficAnalysisViewHandle } from './trafficAnalysisViewTypes'
import type {
  TrafficWorkbenchBasketCandidateInput,
  TrafficWorkbenchBasketItem,
  TrafficWorkbenchSource,
  TrafficWorkbenchToolSession,
} from './trafficWorkbenchTypes'
import { useTrafficWorkbenchBasket } from './useTrafficWorkbenchBasket'
import { useTrafficWorkbenchSessions } from './useTrafficWorkbenchSessions'
import { immersiveDrillModeEnabled } from '@/services/immersiveDrillMode'
import {
  closeImmersiveTrafficWorkbench,
  openImmersiveTrafficWorkbenchTool,
  resetImmersiveTrafficDockState,
  syncImmersiveTrafficDockState,
  useImmersiveTrafficDockState,
} from './immersiveTrafficDockState'
interface FilterRule {
  matchType: string
  condition: string
  relationship?: string
}

type WorkbenchTool = TrafficWorkbenchToolSession['tool']

const { t } = useI18n()

const proxyHistoryRef = ref<InstanceType<typeof TrafficHistoryWorkbench> | null>(null)
const proxyConfigRef = ref<InstanceType<typeof ProxyConfiguration> | null>(null)
const repeaterRef = ref<InstanceType<typeof ProxyRepeater> | null>(null)
const intruderRef = ref<InstanceType<typeof ProxyIntruder> | null>(null)
const comparerRef = ref<InstanceType<typeof ProxyComparer> | null>(null)
const workspaceStageRef = ref<HTMLElement | null>(null)

const WORKBENCH_HEIGHT_STORAGE_KEY = 'sentinel:traffic-workbench-height:v1'
const INTERCEPT_DRAWER_WIDTH_STORAGE_KEY = 'sentinel:traffic-workbench-intercept-width:v1'
const SETTINGS_DRAWER_WIDTH_STORAGE_KEY = 'sentinel:traffic-workbench-settings-width:v1'
const WORKBENCH_MIN_HEIGHT = 300
const INTERCEPT_DRAWER_DEFAULT_WIDTH = 416
const INTERCEPT_DRAWER_MIN_WIDTH = 340
const SETTINGS_DRAWER_DEFAULT_WIDTH = 544
const SETTINGS_DRAWER_MIN_WIDTH = 420

const pendingRepeaterRequest = ref<HttpExchangeRequest | undefined>(undefined)
const pendingIntruderRequest = ref<HttpExchangeRequest | undefined>(undefined)
const workbenchToolsMounted = ref(false)
const {
  workbenchOpen,
  activeWorkbenchTool,
  interceptDrawerOpen,
  proxySettingsOpen,
  basketOpen,
  controlInterceptCount,
} = useImmersiveTrafficDockState()
const workbenchHeight = ref(loadWorkbenchHeight())
const interceptDrawerWidth = ref(loadStoredDrawerWidth(INTERCEPT_DRAWER_WIDTH_STORAGE_KEY, INTERCEPT_DRAWER_DEFAULT_WIDTH))
const settingsDrawerWidth = ref(loadStoredDrawerWidth(SETTINGS_DRAWER_WIDTH_STORAGE_KEY, SETTINGS_DRAWER_DEFAULT_WIDTH))
const viewportWidth = ref(typeof window === 'undefined' ? 1440 : window.innerWidth)
const isResizingWorkbench = ref(false)
const resizeStartY = ref(0)
const resizeStartHeight = ref(workbenchHeight.value)
const drawerWidthResizeState = ref<{
  drawer: 'intercept' | 'settings' | null
  startX: number
  startWidth: number
} | null>(null)

const { basketItems, addRequest, removeItem, clear } = useTrafficWorkbenchBasket()
const { sessions, markSession } = useTrafficWorkbenchSessions()

const workbenchMetaMap: Record<
  WorkbenchTool,
  { title: string; shortTitle: string; description: string }
> = {
  repeater: {
    title: t('trafficAnalysis.tabs.repeater', '重放器'),
    shortTitle: t('trafficAnalysis.tabs.repeater', '重放器'),
    description: '单条请求精修和重放只在需要时展开，不常驻占用主视图。',
  },
  intruder: {
    title: t('trafficAnalysis.tabs.intruder', '爆破器'),
    shortTitle: t('trafficAnalysis.tabs.intruder', '爆破器'),
    description: '批量化尝试收进底部工作台，避免把同屏密度拉高。',
  },
  comparer: {
    title: t('trafficAnalysis.tabs.comparer', '对比器'),
    shortTitle: t('trafficAnalysis.tabs.comparer', '对比器'),
    description: '只在确认差异时临时展开，看完后立即回到主工作流。',
  },
}

const activeWorkbenchMeta = computed(() => workbenchMetaMap[activeWorkbenchTool.value])
const activeWorkbenchSourceLabel = computed(
  () => sessions.value[activeWorkbenchTool.value].source?.label || '',
)

const toolChips = computed(() => [
  {
    tool: 'repeater' as const,
    label: t('trafficAnalysis.tabs.repeater', '重放器'),
    shortLabel: t('trafficAnalysis.tabs.repeater', '重放器'),
    icon: 'fas fa-redo',
    count: sessions.value.repeater.count,
  },
  {
    tool: 'intruder' as const,
    label: t('trafficAnalysis.tabs.intruder', '爆破器'),
    shortLabel: t('trafficAnalysis.tabs.intruder', '爆破器'),
    icon: 'fas fa-crosshairs',
    count: sessions.value.intruder.count,
  },
  {
    tool: 'comparer' as const,
    label: t('trafficAnalysis.tabs.comparer', '对比器'),
    shortLabel: t('trafficAnalysis.tabs.comparer', '对比器'),
    icon: 'fas fa-not-equal',
    count: sessions.value.comparer.count,
  },
])

const workbenchDrawerStyle = computed(() => ({
  height: `${workbenchHeight.value}px`,
}))
const interceptDrawerStyle = computed(() =>
  viewportWidth.value <= 1024 ? {} : { width: `${interceptDrawerWidth.value}px` },
)
const settingsDrawerStyle = computed(() =>
  viewportWidth.value <= 1024 ? {} : { width: `${settingsDrawerWidth.value}px` },
)

function buildSource(
  kind: TrafficWorkbenchSource['kind'],
  label: string,
  requestId?: number,
): TrafficWorkbenchSource {
  return { kind, label, requestId }
}

function getHistorySource(requestId?: number) {
  return buildSource('history', requestId ? `历史记录 #${requestId}` : '历史记录', requestId)
}

function getInterceptSource() {
  return buildSource('intercept', '拦截队列')
}

function getBasketSource(item?: TrafficWorkbenchBasketItem) {
  return buildSource('basket', item?.requestId ? `请求篮子 -> #${item.requestId}` : '请求篮子', item?.requestId)
}

function getToolSource(tool: WorkbenchTool) {
  return buildSource(tool, workbenchMetaMap[tool].title)
}

function loadWorkbenchHeight() {
  if (typeof window === 'undefined') {
    return 520
  }

  const stored = Number(window.localStorage.getItem(WORKBENCH_HEIGHT_STORAGE_KEY) || '520')
  return Number.isFinite(stored) ? stored : 520
}

function loadStoredDrawerWidth(storageKey: string, fallback: number) {
  if (typeof window === 'undefined') {
    return fallback
  }

  const stored = Number(window.localStorage.getItem(storageKey) || String(fallback))
  return Number.isFinite(stored) ? stored : fallback
}

function clampWorkbenchHeight(height: number) {
  const stageHeight = workspaceStageRef.value?.offsetHeight ?? window.innerHeight
  const maxHeight = Math.max(WORKBENCH_MIN_HEIGHT, stageHeight - 24)
  return Math.min(Math.max(WORKBENCH_MIN_HEIGHT, height), maxHeight)
}

function clampDrawerWidth(drawer: 'intercept' | 'settings', width: number) {
  const minWidth = drawer === 'intercept' ? INTERCEPT_DRAWER_MIN_WIDTH : SETTINGS_DRAWER_MIN_WIDTH
  const stageWidth = workspaceStageRef.value?.offsetWidth ?? viewportWidth.value
  const maxWidth = Math.max(minWidth, stageWidth - 32)
  return Math.min(Math.max(minWidth, width), maxWidth)
}

function persistWorkbenchHeight() {
  window.localStorage.setItem(WORKBENCH_HEIGHT_STORAGE_KEY, String(workbenchHeight.value))
}

function persistDrawerWidth(drawer: 'intercept' | 'settings') {
  window.localStorage.setItem(
    drawer === 'intercept' ? INTERCEPT_DRAWER_WIDTH_STORAGE_KEY : SETTINGS_DRAWER_WIDTH_STORAGE_KEY,
    String(drawer === 'intercept' ? interceptDrawerWidth.value : settingsDrawerWidth.value),
  )
}

function applyWorkbenchHeight(height: number) {
  workbenchHeight.value = clampWorkbenchHeight(height)
}

function applyDrawerWidth(drawer: 'intercept' | 'settings', width: number) {
  if (drawer === 'intercept') {
    interceptDrawerWidth.value = clampDrawerWidth(drawer, width)
    return
  }

  settingsDrawerWidth.value = clampDrawerWidth(drawer, width)
}

function handleWorkbenchResize(event: MouseEvent) {
  if (!isResizingWorkbench.value) {
    return
  }

  const diff = resizeStartY.value - event.clientY
  applyWorkbenchHeight(resizeStartHeight.value + diff)
}

function stopWorkbenchResize() {
  if (!isResizingWorkbench.value) {
    return
  }

  isResizingWorkbench.value = false
  document.body.style.cursor = ''
  document.body.style.userSelect = ''
  window.removeEventListener('mousemove', handleWorkbenchResize)
  window.removeEventListener('mouseup', stopWorkbenchResize)
  persistWorkbenchHeight()
}

function handleDrawerWidthResize(event: MouseEvent) {
  const state = drawerWidthResizeState.value
  if (!state) {
    return
  }

  const diffX = event.clientX - state.startX
  applyDrawerWidth(state.drawer, state.startWidth - diffX)
}

function stopDrawerWidthResize() {
  const state = drawerWidthResizeState.value
  if (!state) {
    return
  }

  persistDrawerWidth(state.drawer)
  drawerWidthResizeState.value = null
  document.body.style.cursor = ''
  document.body.style.userSelect = ''
  window.removeEventListener('mousemove', handleDrawerWidthResize)
  window.removeEventListener('mouseup', stopDrawerWidthResize)
}

function startDrawerWidthResize(drawer: 'intercept' | 'settings', event: MouseEvent) {
  if (viewportWidth.value <= 1024) {
    return
  }

  drawerWidthResizeState.value = {
    drawer,
    startX: event.clientX,
    startWidth: drawer === 'intercept' ? interceptDrawerWidth.value : settingsDrawerWidth.value,
  }
  document.body.style.cursor = 'col-resize'
  document.body.style.userSelect = 'none'
  window.addEventListener('mousemove', handleDrawerWidthResize)
  window.addEventListener('mouseup', stopDrawerWidthResize)
  event.preventDefault()
}

function startWorkbenchResize(event: MouseEvent) {
  isResizingWorkbench.value = true
  resizeStartY.value = event.clientY
  resizeStartHeight.value = workbenchHeight.value
  document.body.style.cursor = 'ns-resize'
  document.body.style.userSelect = 'none'
  window.addEventListener('mousemove', handleWorkbenchResize)
  window.addEventListener('mouseup', stopWorkbenchResize)
  event.preventDefault()
}

function handleWindowResize() {
  viewportWidth.value = window.innerWidth
  applyWorkbenchHeight(workbenchHeight.value)
  applyDrawerWidth('intercept', interceptDrawerWidth.value)
  applyDrawerWidth('settings', settingsDrawerWidth.value)
}

function openWorkbenchTool(tool: WorkbenchTool) {
  openImmersiveTrafficWorkbenchTool(tool)
  workbenchToolsMounted.value = true
  applyWorkbenchHeight(workbenchHeight.value)
}

function closeWorkbench() {
  closeImmersiveTrafficWorkbench()
}

function pushRequestToRepeater(request: HttpExchangeRequest, source: TrafficWorkbenchSource) {
  markSession('repeater', source)
  if (repeaterRef.value) {
    repeaterRef.value.addRequestFromHistory(request)
    openWorkbenchTool('repeater')
    return
  }

  pendingRepeaterRequest.value = request
  openWorkbenchTool('repeater')
  void nextTick(() => {
    pendingRepeaterRequest.value = undefined
  })
}

function pushRequestToIntruder(request: HttpExchangeRequest, source: TrafficWorkbenchSource) {
  markSession('intruder', source)
  if (intruderRef.value) {
    intruderRef.value.addRequestFromHistory(request)
    openWorkbenchTool('intruder')
    return
  }

  pendingIntruderRequest.value = request
  openWorkbenchTool('intruder')
  void nextTick(() => {
    pendingIntruderRequest.value = undefined
  })
}

function pushPayloadToComparer(payload: TrafficComparePayload, source: TrafficWorkbenchSource) {
  markSession('comparer', source)
  if (comparerRef.value) {
    comparerRef.value.addComparison(payload)
    openWorkbenchTool('comparer')
    return
  }

  openWorkbenchTool('comparer')
  requestAnimationFrame(() => {
    comparerRef.value?.addComparison(payload)
  })
}

function pushDraftToComparer(payload: TrafficComparerDraftRequestInput, source: TrafficWorkbenchSource) {
  markSession('comparer', source)
  if (comparerRef.value) {
    comparerRef.value.addDraftRequest(payload)
    openWorkbenchTool('comparer')
    return
  }

  openWorkbenchTool('comparer')
  requestAnimationFrame(() => {
    comparerRef.value?.addDraftRequest(payload)
  })
}

function handleSendToRepeaterFromHistory(request: HttpExchangeRequest) {
  pushRequestToRepeater(request, getHistorySource())
}

function handleSendToIntruderFromHistory(request: HttpExchangeRequest) {
  pushRequestToIntruder(request, getHistorySource())
}

function handleSendToComparerFromHistory(payload: TrafficComparePayload) {
  pushPayloadToComparer(payload, getHistorySource())
}

function handleSendDraftRequestToComparerFromHistory(payload: TrafficComparerDraftRequestInput) {
  pushDraftToComparer(payload, getHistorySource())
}

function handleSendToRepeaterFromIntercept(request: HttpExchangeRequest) {
  interceptDrawerOpen.value = false
  pushRequestToRepeater(request, getInterceptSource())
}

function handleSendToIntruderFromIntercept(request: HttpExchangeRequest) {
  interceptDrawerOpen.value = false
  pushRequestToIntruder(request, getInterceptSource())
}

function handleSendDraftRequestToComparerFromIntercept(payload: TrafficComparerDraftRequestInput) {
  interceptDrawerOpen.value = false
  pushDraftToComparer(payload, getInterceptSource())
}

function handleSendToIntruderFromRepeater(request: HttpExchangeRequest) {
  pushRequestToIntruder(request, getToolSource('repeater'))
}

function handleSendToComparerFromRepeater(payload: TrafficComparePayload) {
  pushPayloadToComparer(payload, getToolSource('repeater'))
}

function handleSendDraftRequestToComparerFromRepeater(payload: TrafficComparerDraftRequestInput) {
  pushDraftToComparer(payload, getToolSource('repeater'))
}

function handleSendToRepeaterFromIntruder(request: HttpExchangeRequest) {
  pushRequestToRepeater(request, getToolSource('intruder'))
}

function handleSendToComparerFromIntruder(payload: TrafficComparePayload) {
  pushPayloadToComparer(payload, getToolSource('intruder'))
}

function handleSendDraftRequestToComparerFromIntruder(payload: TrafficComparerDraftRequestInput) {
  pushDraftToComparer(payload, getToolSource('intruder'))
}

function handleSendToRepeaterFromComparer(request: HttpExchangeRequest) {
  pushRequestToRepeater(request, getToolSource('comparer'))
}

function addBasketCandidate(source: TrafficWorkbenchSource, payload: TrafficWorkbenchBasketCandidateInput) {
  addRequest(payload.request, source, {
    requestId: payload.requestId,
    title: payload.title,
  })
}

function handleAddToBasketFromHistory(payload: TrafficWorkbenchBasketCandidateInput) {
  addBasketCandidate(getHistorySource(payload.requestId), payload)
  basketOpen.value = true
}

function handleAddToBasketFromIntercept(payload: TrafficWorkbenchBasketCandidateInput) {
  addBasketCandidate(getInterceptSource(), payload)
  basketOpen.value = true
}

function removeBasketItem(id: string) {
  removeItem(id)
}

function clearBasket() {
  clear()
}

function findBasketItem(id: string) {
  return basketItems.value.find(item => item.id === id) || null
}

function sendBasketItemToRepeater(id: string) {
  const item = findBasketItem(id)
  if (!item) {
    return
  }
  pushRequestToRepeater(item.request, getBasketSource(item))
}

function sendBasketItemToIntruder(id: string) {
  const item = findBasketItem(id)
  if (!item) {
    return
  }
  pushRequestToIntruder(item.request, getBasketSource(item))
}

function sendAllBasketItemsToRepeater() {
  basketItems.value.forEach(item => {
    pushRequestToRepeater(item.request, getBasketSource(item))
  })
}

function sendAllBasketItemsToIntruder() {
  basketItems.value.forEach(item => {
    pushRequestToIntruder(item.request, getBasketSource(item))
  })
}

async function openHistoryRequestFromBasket(requestId: number) {
  basketOpen.value = false
  await openHistoryRequest({
    requestId,
    pane: 'request',
    matchedLocations: [],
    searchTerms: [],
  })
}

function handleAddFilterRule(rule: FilterRule) {
  proxyConfigRef.value?.addRequestFilterRule(
    rule.matchType,
    rule.condition,
    rule.relationship || 'matches',
  )
  proxySettingsOpen.value = true
}

function handleFilterRuleAdded(rule: FilterRule) {
  proxyHistoryRef.value?.removeMatchingRecords({
    matchType: rule.matchType,
    condition: rule.condition,
    relationship: rule.relationship || 'matches',
  })
}

async function handleOpenResponseInterceptionSettings() {
  proxySettingsOpen.value = true
  await nextTick()
  await proxyConfigRef.value?.openResponseInterceptionRules?.()
}

function handleInterceptQueueChanged(count: number) {
  controlInterceptCount.value = count
}

async function openHistoryRequest(payload: TrafficContextCandidateEvidenceSelection) {
  if (!Number.isFinite(payload.requestId)) {
    return
  }

  basketOpen.value = false
  await nextTick()
  await proxyHistoryRef.value?.openRequestById?.(
    payload.requestId,
    payload.matchedLocations,
    payload.pane || 'request',
    payload.searchTerms || [],
  )
}

defineExpose<TrafficAnalysisViewHandle>({
  sendToRepeater: handleSendToRepeaterFromHistory,
  sendToIntruder: handleSendToIntruderFromHistory,
  sendToComparer: handleSendToComparerFromHistory,
  sendDraftRequestToComparer: handleSendDraftRequestToComparerFromHistory,
  openHistoryRequest,
})

onMounted(() => {
  applyWorkbenchHeight(workbenchHeight.value)
  window.addEventListener('resize', handleWindowResize)
})

watchEffect(() => {
  if (workbenchOpen.value) {
    workbenchToolsMounted.value = true
    applyWorkbenchHeight(workbenchHeight.value)
  }

  syncImmersiveTrafficDockState({
    workbenchOpen: workbenchOpen.value,
    activeWorkbenchTool: activeWorkbenchTool.value,
    interceptDrawerOpen: interceptDrawerOpen.value,
    proxySettingsOpen: proxySettingsOpen.value,
    basketOpen: basketOpen.value,
    repeaterCount: sessions.value.repeater.count,
    intruderCount: sessions.value.intruder.count,
    comparerCount: sessions.value.comparer.count,
    controlInterceptCount: controlInterceptCount.value,
    basketCount: basketItems.value.length,
  })
})

onUnmounted(() => {
  stopWorkbenchResize()
  stopDrawerWidthResize()
  window.removeEventListener('resize', handleWindowResize)
  resetImmersiveTrafficDockState()
})
</script>

<style scoped>
.workbench-solid-surface {
  background: rgba(255, 255, 255, 0.985) !important;
}

.workbench-header-surface {
  position: relative;
  isolation: isolate;
  background: rgba(255, 255, 255, 0.995) !important;
  box-shadow:
    inset 0 -1px 0 hsl(var(--b3) / 0.5),
    0 10px 24px rgb(15 23 42 / 0.05);
}

.workbench-header-surface::before {
  content: '';
  position: absolute;
  inset: 0;
  z-index: -1;
  background: rgba(255, 255, 255, 0.995);
}

.workbench-content-surface {
  background: #fff !important;
  position: relative;
  z-index: 0;
  box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.75);
}

.workbench-content-surface::before {
  content: '';
  position: absolute;
  inset: 0;
  z-index: 0;
  background: #fff;
  pointer-events: none;
}

.tool-switch {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  border-radius: 999px;
  border: 1px solid hsl(var(--b3) / 0.7);
  background: hsl(var(--b1) / 0.72);
  padding: 0.6rem 0.9rem;
  font-size: 0.8rem;
  font-weight: 600;
  color: hsl(var(--bc) / 0.72);
  transition:
    transform 160ms ease,
    border-color 160ms ease,
    background-color 160ms ease,
    color 160ms ease,
    box-shadow 160ms ease;
}

.tool-switch:hover,
.floating-trigger:hover {
  transform: translateY(-1px);
}

.tool-switch-active {
  border-color: hsl(var(--p) / 0.28);
  background: hsl(var(--p) / 0.12);
  color: hsl(var(--p));
  box-shadow: 0 10px 26px hsl(var(--p) / 0.1);
}

.workbench-drawer {
  display: flex;
  flex-direction: column;
  isolation: isolate;
}

.intercept-drawer {
  height: min(72vh, 48rem);
  display: flex;
  flex-direction: column;
}

.settings-drawer {
  display: flex;
  flex-direction: column;
}

.drawer-width-resizer {
  width: 10px;
  cursor: col-resize;
  background: linear-gradient(180deg, transparent 0%, rgb(148 163 184 / 0.12) 50%, transparent 100%);
  transition: background-color 160ms ease, opacity 160ms ease;
  opacity: 0.4;
}

.drawer-width-resizer:hover {
  opacity: 1;
  background: linear-gradient(180deg, transparent 0%, rgb(59 130 246 / 0.3) 50%, transparent 100%);
}

.drawer-width-resizer-warning:hover {
  background: linear-gradient(180deg, transparent 0%, rgb(245 158 11 / 0.28) 50%, transparent 100%);
}

.floating-trigger {
  position: relative;
  display: inline-flex;
  height: 3.75rem;
  width: 3.75rem;
  align-items: center;
  justify-content: center;
  border-radius: 1.5rem;
  border: 1px solid hsl(var(--b3) / 0.72);
  background: hsl(var(--b1) / 0.92);
  color: hsl(var(--bc) / 0.78);
  box-shadow: 0 22px 48px rgb(15 23 42 / 0.16);
  transition:
    transform 160ms ease,
    border-color 160ms ease,
    background-color 160ms ease,
    color 160ms ease;
}

.floating-trigger-active {
  border-color: hsl(var(--p) / 0.3);
  background: hsl(var(--p) / 0.12);
  color: hsl(var(--p));
}

.floating-trigger-warning {
  border-color: hsl(var(--wa) / 0.24);
  color: hsl(var(--wa));
}

.floating-trigger-label {
  position: absolute;
  right: calc(100% + 0.75rem);
  white-space: nowrap;
  border-radius: 999px;
  background: rgb(15 23 42 / 0.88);
  color: #fff;
  padding: 0.45rem 0.7rem;
  font-size: 0.72rem;
  opacity: 0;
  pointer-events: none;
  transition: opacity 140ms ease;
}

.floating-trigger:hover .floating-trigger-label {
  opacity: 1;
}

.floating-badge {
  position: absolute;
  right: -0.2rem;
  top: -0.2rem;
  min-width: 1.2rem;
  height: 1.2rem;
  border-radius: 999px;
  background: hsl(var(--er));
  color: hsl(var(--erc));
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0 0.25rem;
  font-size: 0.68rem;
  font-weight: 700;
}

@media (max-width: 1024px) {
  .intercept-drawer,
  .settings-drawer {
    left: 1rem;
    right: 1rem;
    width: auto;
  }
}
</style>
