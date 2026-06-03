<template>
  <section class="active-probe-floating-shell" :style="shellStyle">
    <button
      v-if="collapsed"
      ref="collapsedButtonRef"
      type="button"
      class="active-probe-edge-tab"
      @click="handleCollapsedButtonClick"
      @mousedown="startCollapsedButtonDrag"
    >
      <span class="active-probe-edge-label">主动探测</span>
      <span v-if="selectedCooldownKeyLabel" class="active-probe-edge-filter" :title="selectedCooldownKey || ''">
        {{ selectedCooldownKeyLabel }}
      </span>
      <span class="active-probe-edge-count">{{ queuedCount }}</span>
    </button>

    <section v-else class="active-probe-panel" :style="panelStyle">
      <div
        class="active-probe-height-resize-handle"
        title="拖拽调整高度"
        @mousedown="startPanelHeightResize"
      ></div>
      <div
        class="active-probe-resize-handle"
        title="拖拽调整宽度"
        @mousedown="startPanelResize"
      ></div>

      <div class="flex items-start justify-between gap-3">
        <div>
          <p class="text-[11px] font-semibold uppercase tracking-[0.18em] text-primary/75">
            Active Probe Queue
          </p>
          <h3 class="mt-1 text-sm font-semibold text-base-content">主动探测队列</h3>
          <p class="mt-1 text-[11px] text-base-content/55">
            Queue 显示真实 pending/running，Recent 显示最近完成或失败。
          </p>
        </div>
        <div class="flex items-center gap-2">
          <span class="rounded-full bg-primary/10 px-2.5 py-1 text-[11px] font-semibold text-primary">
            {{ queuedCount }} 队列中待请求
          </span>
          <button
            type="button"
            class="btn btn-ghost btn-xs rounded-full"
            title="靠边收起"
            @click="emit('toggleCollapsed')"
          >
            <i class="fas fa-angles-right"></i>
          </button>
        </div>
      </div>

      <div class="active-probe-panel__body">
        <div class="mt-3 grid gap-2 sm:grid-cols-2 xl:grid-cols-4">
          <div class="active-probe-stat-card">
            <p class="active-probe-stat-card__label">Pending</p>
            <p class="active-probe-stat-card__value">{{ pendingEntries.length }}</p>
          </div>
          <div class="active-probe-stat-card">
            <p class="active-probe-stat-card__label">Running</p>
            <p class="active-probe-stat-card__value">{{ runningEntries.length }}</p>
          </div>
          <div class="active-probe-stat-card">
            <p class="active-probe-stat-card__label">Completed</p>
            <p class="active-probe-stat-card__value">{{ derivedState.completedCount }}</p>
          </div>
          <div class="active-probe-stat-card">
            <p class="active-probe-stat-card__label">Failed</p>
            <p class="active-probe-stat-card__value">{{ derivedState.failedCount }}</p>
          </div>
        </div>

        <div v-if="derivedState.pluginOptions.length > 1" class="mt-3 flex flex-wrap items-center gap-2">
          <button
            type="button"
            class="btn btn-xs rounded-full"
            :class="selectedPluginId ? 'btn-ghost' : 'btn-primary'"
            @click="selectedPluginId = null"
          >
            全部插件
          </button>
          <button
            v-for="pluginId in derivedState.pluginOptions"
            :key="pluginId"
            type="button"
            class="btn btn-xs rounded-full"
            :class="selectedPluginId === pluginId ? 'btn-primary' : 'btn-ghost'"
            @click="selectedPluginId = selectedPluginId === pluginId ? null : pluginId"
          >
            {{ pluginId }}
          </button>
        </div>

        <div v-if="derivedState.hotPaths.length" class="mt-3 space-y-2">
          <div class="flex items-center justify-between gap-3">
            <p class="text-[11px] font-semibold uppercase tracking-[0.14em] text-base-content/48">
              Hot Paths
            </p>
            <div class="flex items-center gap-2">
              <button
                v-if="selectedCooldownKey"
                type="button"
                class="btn btn-ghost btn-xs rounded-full"
                @click="selectedCooldownKey = null"
              >
                清除筛选
              </button>
              <button
                v-if="selectedCooldownKey && derivedState.latestFilteredTrafficRequestId"
                type="button"
                class="btn btn-outline btn-xs rounded-full"
                @click="emit('openHistoryByTrafficRequestId', derivedState.latestFilteredTrafficRequestId)"
              >
                打开最近历史
              </button>
            </div>
          </div>
          <div class="flex flex-wrap gap-2">
            <button
              v-for="group in derivedState.hotPaths"
              :key="group.key"
              type="button"
              class="active-probe-hot-key"
              :class="{ 'active-probe-hot-key--selected': selectedCooldownKey === group.key }"
              :title="group.key"
              @click="selectedCooldownKey = selectedCooldownKey === group.key ? null : group.key"
            >
              <p class="active-probe-hot-key__path">{{ group.label }}</p>
              <p class="active-probe-hot-key__meta">{{ group.pending }} pending / {{ group.running }} running</p>
            </button>
          </div>
        </div>

        <div class="mt-4 space-y-3">
          <div class="flex items-center justify-between gap-2">
            <p class="text-[11px] font-semibold uppercase tracking-[0.14em] text-base-content/48">Queue</p>
            <p class="text-[11px] text-base-content/45">
              {{ derivedState.filteredPendingCount }} pending / {{ derivedState.filteredRunningCount }} running
            </p>
          </div>

          <p
            v-if="derivedState.filteredPendingCount === 0 && derivedState.filteredRunningCount === 0"
            class="rounded-[20px] border border-base-300/60 bg-base-100/90 px-4 py-4 text-[11px] text-base-content/55"
          >
            当前筛选下没有待执行或正在执行的主动探测请求。
          </p>

          <div v-else class="space-y-2">
            <article
              v-for="item in derivedState.visibleQueueEntries"
              :key="`queue:${item.entry.request_id}`"
              class="active-probe-item"
            >
              <div class="flex items-start justify-between gap-3">
                <div class="min-w-0 flex-1">
                  <div class="flex min-w-0 items-center gap-2">
                    <span class="active-probe-phase">{{ item.phaseLabel }}</span>
                    <span class="truncate text-[11px] font-medium text-base-content/72">
                      {{ item.entry.method }} {{ item.pathLabel }}
                    </span>
                  </div>
                  <p class="mt-1 truncate text-[11px] text-base-content/55">{{ item.queueMeta }}</p>
                </div>
                <div class="shrink-0 text-right">
                  <p class="text-[11px] font-medium text-base-content/68">{{ item.entry.plugin_id }}</p>
                  <p class="mt-1 text-[10px] text-base-content/45">
                    {{ item.urlSummary }}
                  </p>
                </div>
              </div>

              <div class="mt-3 flex flex-wrap items-center gap-2">
                <button
                  v-if="item.entry.traffic_request_id"
                  type="button"
                  class="btn btn-xs btn-outline rounded-full"
                  @click="openPreview(item.entry)"
                >
                  <i class="fas fa-eye"></i>
                  <span>预览请求/响应</span>
                </button>
                <button
                  v-if="item.entry.traffic_request_id && previewRequestMap[item.entry.traffic_request_id]"
                  type="button"
                  class="btn btn-xs btn-ghost rounded-full"
                  @click="emit('openHistory', previewRequestMap[item.entry.traffic_request_id]!.id)"
                >
                  <i class="fas fa-arrow-up-right-from-square"></i>
                  <span>打开详情</span>
                </button>
              </div>
            </article>
            <p
              v-if="derivedState.queueHiddenCount > 0"
              class="rounded-[18px] border border-dashed border-base-300/65 bg-base-100/75 px-3 py-2 text-[11px] text-base-content/52"
            >
              仅渲染前 {{ derivedState.visibleQueueEntries.length }} 条队列记录，剩余 {{ derivedState.queueHiddenCount }} 条未展开显示。
            </p>
            <button
              v-if="derivedState.queueHiddenCount > 0"
              type="button"
              class="btn btn-sm btn-outline rounded-full"
              @click="showMoreQueueEntries"
            >
              继续展开 {{ Math.min(queuePageSize, derivedState.queueHiddenCount) }} 条队列记录
            </button>
          </div>
        </div>

        <div class="mt-4 space-y-3">
          <div class="flex items-center justify-between gap-2">
            <p class="text-[11px] font-semibold uppercase tracking-[0.14em] text-base-content/48">Recent</p>
            <p class="text-[11px] text-base-content/45">{{ derivedState.filteredRecentCount }} recent</p>
          </div>

          <p
            v-if="derivedState.filteredRecentCount === 0"
            class="rounded-[20px] border border-base-300/60 bg-base-100/90 px-4 py-4 text-[11px] text-base-content/55"
          >
            还没有最近完成、失败或取消的主动探测记录。
          </p>

          <div v-else class="space-y-2">
            <article
              v-for="item in derivedState.visibleRecentEntries"
              :key="`recent:${item.entry.request_id}`"
              class="active-probe-item"
            >
              <div class="flex items-start justify-between gap-3">
                <div class="min-w-0 flex-1">
                  <div class="flex min-w-0 items-center gap-2">
                    <span class="active-probe-phase">{{ item.phaseLabel }}</span>
                    <span class="truncate text-[11px] font-medium text-base-content/72">
                      {{ item.entry.method }} {{ item.pathLabel }}
                    </span>
                  </div>
                  <p class="mt-1 truncate text-[11px] text-base-content/55">{{ item.recentMeta }}</p>
                </div>
                <div class="shrink-0 text-right">
                  <p class="text-[11px] font-medium text-base-content/68">{{ item.entry.plugin_id }}</p>
                  <p class="mt-1 text-[10px] text-base-content/45">
                    {{ item.finishedTimeLabel }}
                  </p>
                </div>
              </div>

              <div class="mt-3 flex flex-wrap items-center gap-2">
                <button
                  v-if="item.entry.traffic_request_id"
                  type="button"
                  class="btn btn-xs btn-outline rounded-full"
                  @click="openPreview(item.entry)"
                >
                  <i class="fas fa-eye"></i>
                  <span>预览请求/响应</span>
                </button>
                <button
                  v-if="item.entry.traffic_request_id && previewRequestMap[item.entry.traffic_request_id]"
                  type="button"
                  class="btn btn-xs btn-ghost rounded-full"
                  @click="emit('openHistory', previewRequestMap[item.entry.traffic_request_id]!.id)"
                >
                  <i class="fas fa-arrow-up-right-from-square"></i>
                  <span>打开详情</span>
                </button>
              </div>
            </article>
            <p
              v-if="derivedState.recentHiddenCount > 0"
              class="rounded-[18px] border border-dashed border-base-300/65 bg-base-100/75 px-3 py-2 text-[11px] text-base-content/52"
            >
              仅渲染最近 {{ derivedState.visibleRecentEntries.length }} 条记录，剩余 {{ derivedState.recentHiddenCount }} 条未展开显示。
            </p>
            <button
              v-if="derivedState.recentHiddenCount > 0"
              type="button"
              class="btn btn-sm btn-outline rounded-full"
              @click="showMoreRecentEntries"
            >
              继续展开 {{ Math.min(recentPageSize, derivedState.recentHiddenCount) }} 条 Recent 记录
            </button>
          </div>
        </div>
      </div>
    </section>
  </section>

  <AppModal
    :open="Boolean(previewEntry)"
    box-class="active-probe-preview-modal-box"
    resizable
    resize-storage-key="active-probe-preview-modal-size-v3"
    :min-width="952"
    :min-height="760"
    :default-width="1160"
    :default-height="920"
    @close="closePreview"
  >
    <div class="flex h-full min-h-0 flex-col overflow-hidden">
      <div class="border-b border-base-300/70 px-6 py-4">
        <div class="flex items-start justify-between gap-4">
          <div class="min-w-0">
            <p class="text-[11px] font-semibold uppercase tracking-[0.22em] text-primary/75">
              Probe Preview
            </p>
            <h3 class="mt-1 text-lg font-semibold text-base-content">主动探测预览</h3>
            <p v-if="previewEntry" class="mt-1 truncate text-xs text-base-content/55">
              {{ previewEntry.method }} {{ previewEntry.url }}
            </p>
          </div>
          <div class="flex items-center gap-2">
            <button
              v-if="previewRequest"
              type="button"
              class="btn btn-xs btn-outline rounded-full"
              @click="emit('openHistory', previewRequest.id)"
            >
              <i class="fas fa-arrow-up-right-from-square"></i>
              <span>打开详情</span>
            </button>
            <button type="button" class="btn btn-sm btn-ghost rounded-2xl" @click="closePreview">
              <i class="fas fa-times"></i>
            </button>
          </div>
        </div>
      </div>

      <div class="min-h-0 flex-1 overflow-auto bg-base-200/35 p-4">
        <div v-if="isPreviewLoading" class="rounded-[20px] border border-base-300/55 bg-base-100/85 px-4 py-4 text-sm text-base-content/58">
          正在加载请求详情...
        </div>
        <div
          v-else-if="previewRequest"
          class="grid min-h-0 gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]"
        >
          <section class="min-w-0 rounded-[22px] border border-base-300/65 bg-base-100/92 px-4 py-3">
            <div class="flex items-center justify-between gap-2">
              <h4 class="text-sm font-semibold text-base-content">Request</h4>
              <span class="text-[11px] text-base-content/45">{{ previewRequest.method }} {{ previewRequest.status_code }}</span>
            </div>
            <pre class="active-probe-preview-block">{{ formatRequest(previewRequest) }}</pre>
          </section>
          <section class="min-w-0 rounded-[22px] border border-base-300/65 bg-base-100/92 px-4 py-3">
            <div class="flex items-center justify-between gap-2">
              <h4 class="text-sm font-semibold text-base-content">Response</h4>
              <span class="text-[11px] text-base-content/45">{{ previewRequest.response_time }} ms</span>
            </div>
            <pre class="active-probe-preview-block">{{ formatResponse(previewRequest) }}</pre>
          </section>
        </div>
        <div v-else class="rounded-[20px] border border-base-300/55 bg-base-100/85 px-4 py-4 text-sm text-base-content/58">
          暂时找不到请求详情。
        </div>
      </div>
    </div>
  </AppModal>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import type { CSSProperties, PropType } from 'vue'
import AppModal from '@/components/AppModal.vue'
import { setLocalStorageItem } from '@/utils/browserStorage'
import { formatRequestRaw, formatResponseRaw } from './proxyHistoryFormattingSupport'
import {
  ACTIVE_PROBE_COLLAPSED_POSITION_STORAGE_KEY,
  clampActiveProbeCollapsedPosition,
  loadStoredActiveProbeCollapsedPosition,
} from './trafficActiveProbeCollapsedPosition'
import {
  ACTIVE_PROBE_INITIAL_QUEUE_LIMIT,
  ACTIVE_PROBE_INITIAL_RECENT_LIMIT,
  ACTIVE_PROBE_QUEUE_PAGE_SIZE,
  ACTIVE_PROBE_RECENT_PAGE_SIZE,
  buildActiveProbePanelDerivedState,
  formatCooldownKeyLabel,
} from './trafficActiveProbePanelSupport'
import type { ProxyRequest } from './proxyHistoryTypes'
import type { ActiveProbeQueueEntry } from './trafficActiveProbeTypes'

const ACTIVE_PROBE_PANEL_WIDTH_STORAGE_KEY = 'trafficAnalysis.activeProbe.panelWidth.v2'
const ACTIVE_PROBE_PANEL_HEIGHT_STORAGE_KEY = 'trafficAnalysis.activeProbe.panelHeight.v1'
const ACTIVE_PROBE_PANEL_DEFAULT_WIDTH = 448
const ACTIVE_PROBE_PANEL_MIN_WIDTH = 336
const ACTIVE_PROBE_PANEL_MAX_WIDTH = 760
const ACTIVE_PROBE_PANEL_DEFAULT_HEIGHT = 760
const ACTIVE_PROBE_PANEL_MIN_HEIGHT = 420

const props = defineProps({
  shellStyle: {
    type: Object as PropType<CSSProperties>,
    required: true,
  },
  pendingEntries: {
    type: Array as PropType<ActiveProbeQueueEntry[]>,
    required: true,
  },
  runningEntries: {
    type: Array as PropType<ActiveProbeQueueEntry[]>,
    required: true,
  },
  recentEntries: {
    type: Array as PropType<ActiveProbeQueueEntry[]>,
    required: true,
  },
  queuedCount: {
    type: Number,
    required: true,
  },
  collapsed: {
    type: Boolean,
    required: true,
  },
  previewRequestMap: {
    type: Object as PropType<Record<string, ProxyRequest | undefined>>,
    required: true,
  },
  loadingTrafficRequestId: {
    type: String as PropType<string | null>,
    default: null,
  },
})

const emit = defineEmits<{
  toggleCollapsed: []
  ensurePreview: [trafficRequestId: string]
  openHistory: [requestId: number]
  openHistoryByTrafficRequestId: [trafficRequestId: string]
}>()

const viewportWidth = ref(typeof window === 'undefined' ? 1440 : window.innerWidth)
const viewportHeight = ref(typeof window === 'undefined' ? 900 : window.innerHeight)
const panelWidth = ref(loadStoredPanelWidth())
const panelHeight = ref(loadStoredPanelHeight())
const panelResizeState = ref<{ startX: number; startWidth: number } | null>(null)
const panelHeightResizeState = ref<{ startY: number; startHeight: number } | null>(null)
const collapsedButtonRef = ref<HTMLButtonElement | null>(null)
const collapsedPosition = ref(loadStoredActiveProbeCollapsedPosition())
const collapsedButtonDragState = ref<{
  startX: number
  startY: number
  offsetX: number
  offsetY: number
  width: number
  height: number
  moved: boolean
} | null>(null)
const suppressCollapsedButtonClick = ref(false)
const selectedPluginId = ref<string | null>(null)
const selectedCooldownKey = ref<string | null>(null)
const previewEntry = ref<ActiveProbeQueueEntry | null>(null)
const visibleQueueLimit = ref(ACTIVE_PROBE_INITIAL_QUEUE_LIMIT)
const visibleRecentLimit = ref(ACTIVE_PROBE_INITIAL_RECENT_LIMIT)

const shellStyle = computed(() => {
  if (!collapsed.value || !collapsedPosition.value) {
    return props.shellStyle
  }

  return {
    ...props.shellStyle,
    left: `${collapsedPosition.value.left}px`,
    top: `${collapsedPosition.value.top}px`,
    right: 'auto',
    bottom: 'auto',
  } satisfies CSSProperties
})
const pendingEntries = computed(() => props.pendingEntries)
const runningEntries = computed(() => props.runningEntries)
const recentEntries = computed(() => props.recentEntries)
const queuedCount = computed(() => props.queuedCount)
const collapsed = computed(() => props.collapsed)
const previewRequestMap = computed(() => props.previewRequestMap)
const loadingTrafficRequestId = computed(() => props.loadingTrafficRequestId)

const selectedCooldownKeyLabel = computed(() => (
  selectedCooldownKey.value ? formatCooldownKeyLabel(selectedCooldownKey.value) : ''
))
const queuePageSize = ACTIVE_PROBE_QUEUE_PAGE_SIZE
const recentPageSize = ACTIVE_PROBE_RECENT_PAGE_SIZE

const panelStyle = computed(() => ({
  width: `${clampPanelWidth(panelWidth.value, viewportWidth.value)}px`,
  height: `${clampPanelHeight(panelHeight.value, viewportHeight.value)}px`,
}))

const derivedState = computed(() => buildActiveProbePanelDerivedState({
  collapsed: collapsed.value,
  pendingEntries: pendingEntries.value,
  runningEntries: runningEntries.value,
  recentEntries: recentEntries.value,
  selectedPluginId: selectedPluginId.value,
  selectedCooldownKey: selectedCooldownKey.value,
  visibleQueueLimit: visibleQueueLimit.value,
  visibleRecentLimit: visibleRecentLimit.value,
}))

const previewRequest = computed(() => {
  const trafficRequestId = previewEntry.value?.traffic_request_id
  if (!trafficRequestId) {
    return null
  }
  return previewRequestMap.value[trafficRequestId] || null
})

const isPreviewLoading = computed(() => {
  const trafficRequestId = previewEntry.value?.traffic_request_id
  return Boolean(trafficRequestId && loadingTrafficRequestId.value === trafficRequestId)
})

watch(collapsed, isCollapsed => {
  if (isCollapsed) {
    stopPanelResize()
    stopPanelHeightResize()
    resetVisibleEntryLimits()
    void nextTick(() => {
      syncCollapsedPositionWithinViewport()
    })
  }
})

watch(previewEntry, entry => {
  if (!entry?.traffic_request_id) {
    return
  }
  emit('ensurePreview', entry.traffic_request_id)
})

watch([selectedPluginId, selectedCooldownKey], () => {
  resetVisibleEntryLimits()
})

function resetVisibleEntryLimits() {
  visibleQueueLimit.value = ACTIVE_PROBE_INITIAL_QUEUE_LIMIT
  visibleRecentLimit.value = ACTIVE_PROBE_INITIAL_RECENT_LIMIT
}

function showMoreQueueEntries() {
  visibleQueueLimit.value += ACTIVE_PROBE_QUEUE_PAGE_SIZE
}

function showMoreRecentEntries() {
  visibleRecentLimit.value += ACTIVE_PROBE_RECENT_PAGE_SIZE
}

function openPreview(entry: ActiveProbeQueueEntry) {
  previewEntry.value = entry
}

function closePreview() {
  previewEntry.value = null
}

function formatRequest(request: ProxyRequest) {
  return formatRequestRaw(request, 'edited')
}

function formatResponse(request: ProxyRequest) {
  return formatResponseRaw(request, 'edited', {
    bodyText: request.edited_response_body ?? request.response_body ?? '',
  })
}

function loadStoredPanelWidth() {
  if (typeof window === 'undefined') {
    return ACTIVE_PROBE_PANEL_DEFAULT_WIDTH
  }
  const stored = Number(
    window.localStorage.getItem(ACTIVE_PROBE_PANEL_WIDTH_STORAGE_KEY) || String(ACTIVE_PROBE_PANEL_DEFAULT_WIDTH),
  )
  return Number.isFinite(stored) ? stored : ACTIVE_PROBE_PANEL_DEFAULT_WIDTH
}

function loadStoredPanelHeight() {
  if (typeof window === 'undefined') {
    return ACTIVE_PROBE_PANEL_DEFAULT_HEIGHT
  }
  const stored = Number(
    window.localStorage.getItem(ACTIVE_PROBE_PANEL_HEIGHT_STORAGE_KEY) || String(ACTIVE_PROBE_PANEL_DEFAULT_HEIGHT),
  )
  return Number.isFinite(stored) ? stored : ACTIVE_PROBE_PANEL_DEFAULT_HEIGHT
}

function clampPanelWidth(width: number, stageWidth: number) {
  return Math.min(
    Math.max(ACTIVE_PROBE_PANEL_MIN_WIDTH, width),
    Math.max(ACTIVE_PROBE_PANEL_MIN_WIDTH, Math.min(ACTIVE_PROBE_PANEL_MAX_WIDTH, stageWidth - 48)),
  )
}

function clampPanelHeight(height: number, stageHeight: number) {
  return Math.min(
    Math.max(ACTIVE_PROBE_PANEL_MIN_HEIGHT, height),
    Math.max(ACTIVE_PROBE_PANEL_MIN_HEIGHT, stageHeight - 48),
  )
}

function persistPanelWidth() {
  window.localStorage.setItem(ACTIVE_PROBE_PANEL_WIDTH_STORAGE_KEY, String(panelWidth.value))
}

function persistPanelHeight() {
  window.localStorage.setItem(ACTIVE_PROBE_PANEL_HEIGHT_STORAGE_KEY, String(panelHeight.value))
}

function startPanelResize(event: MouseEvent) {
  panelResizeState.value = {
    startX: event.clientX,
    startWidth: panelWidth.value,
  }
  window.addEventListener('mousemove', handlePanelResize)
  window.addEventListener('mouseup', stopPanelResize)
}

function handlePanelResize(event: MouseEvent) {
  if (!panelResizeState.value) {
    return
  }

  panelWidth.value = clampPanelWidth(
    panelResizeState.value.startWidth - (event.clientX - panelResizeState.value.startX),
    viewportWidth.value,
  )
}

function startPanelHeightResize(event: MouseEvent) {
  panelHeightResizeState.value = {
    startY: event.clientY,
    startHeight: panelHeight.value,
  }
  window.addEventListener('mousemove', handlePanelHeightResize)
  window.addEventListener('mouseup', stopPanelHeightResize)
}

function handlePanelHeightResize(event: MouseEvent) {
  if (!panelHeightResizeState.value) {
    return
  }

  panelHeight.value = clampPanelHeight(
    panelHeightResizeState.value.startHeight - (event.clientY - panelHeightResizeState.value.startY),
    viewportHeight.value,
  )
}

function stopPanelResize() {
  if (!panelResizeState.value) {
    return
  }
  panelResizeState.value = null
  persistPanelWidth()
  window.removeEventListener('mousemove', handlePanelResize)
  window.removeEventListener('mouseup', stopPanelResize)
}

function stopPanelHeightResize() {
  if (!panelHeightResizeState.value) {
    return
  }
  panelHeightResizeState.value = null
  persistPanelHeight()
  window.removeEventListener('mousemove', handlePanelHeightResize)
  window.removeEventListener('mouseup', stopPanelHeightResize)
}

function persistCollapsedPosition() {
  if (!collapsedPosition.value) {
    return
  }

  setLocalStorageItem(
    ACTIVE_PROBE_COLLAPSED_POSITION_STORAGE_KEY,
    JSON.stringify(collapsedPosition.value),
  )
}

function syncCollapsedPositionWithinViewport() {
  if (!collapsed.value || !collapsedButtonRef.value) {
    return
  }

  const rect = collapsedButtonRef.value.getBoundingClientRect()
  const nextPosition = clampActiveProbeCollapsedPosition({
    position: collapsedPosition.value ?? {
      left: rect.left,
      top: rect.top,
    },
    viewportWidth: viewportWidth.value,
    viewportHeight: viewportHeight.value,
    width: rect.width,
    height: rect.height,
  })

  collapsedPosition.value = nextPosition
  persistCollapsedPosition()
}

function startCollapsedButtonDrag(event: MouseEvent) {
  if (!collapsedButtonRef.value) {
    return
  }

  const rect = collapsedButtonRef.value.getBoundingClientRect()
  collapsedPosition.value = clampActiveProbeCollapsedPosition({
    position: {
      left: rect.left,
      top: rect.top,
    },
    viewportWidth: viewportWidth.value,
    viewportHeight: viewportHeight.value,
    width: rect.width,
    height: rect.height,
  })
  collapsedButtonDragState.value = {
    startX: event.clientX,
    startY: event.clientY,
    offsetX: event.clientX - rect.left,
    offsetY: event.clientY - rect.top,
    width: rect.width,
    height: rect.height,
    moved: false,
  }
  suppressCollapsedButtonClick.value = false
  document.body.style.cursor = 'grabbing'
  document.body.style.userSelect = 'none'
  window.addEventListener('mousemove', handleCollapsedButtonDrag)
  window.addEventListener('mouseup', stopCollapsedButtonDrag)
  event.preventDefault()
}

function handleCollapsedButtonDrag(event: MouseEvent) {
  const state = collapsedButtonDragState.value
  if (!state) {
    return
  }

  if (!state.moved && (
    Math.abs(event.clientX - state.startX) >= 3
    || Math.abs(event.clientY - state.startY) >= 3
  )) {
    state.moved = true
  }

  collapsedPosition.value = clampActiveProbeCollapsedPosition({
    position: {
      left: event.clientX - state.offsetX,
      top: event.clientY - state.offsetY,
    },
    viewportWidth: viewportWidth.value,
    viewportHeight: viewportHeight.value,
    width: state.width,
    height: state.height,
  })
}

function stopCollapsedButtonDrag() {
  if (!collapsedButtonDragState.value) {
    return
  }

  suppressCollapsedButtonClick.value = collapsedButtonDragState.value.moved
  collapsedButtonDragState.value = null
  persistCollapsedPosition()
  document.body.style.cursor = ''
  document.body.style.userSelect = ''
  window.removeEventListener('mousemove', handleCollapsedButtonDrag)
  window.removeEventListener('mouseup', stopCollapsedButtonDrag)
}

function handleCollapsedButtonClick() {
  if (suppressCollapsedButtonClick.value) {
    suppressCollapsedButtonClick.value = false
    return
  }

  emit('toggleCollapsed')
}

function handleWindowResize() {
  viewportWidth.value = window.innerWidth
  viewportHeight.value = window.innerHeight
  panelHeight.value = clampPanelHeight(panelHeight.value, viewportHeight.value)
  syncCollapsedPositionWithinViewport()
}

onMounted(() => {
  window.addEventListener('resize', handleWindowResize)
  void nextTick(() => {
    syncCollapsedPositionWithinViewport()
  })
})

onUnmounted(() => {
  stopPanelResize()
  stopPanelHeightResize()
  stopCollapsedButtonDrag()
  window.removeEventListener('resize', handleWindowResize)
})
</script>

<style scoped>
.active-probe-floating-shell {
  position: fixed;
  z-index: 45;
}

.active-probe-edge-tab {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  border-radius: 999px;
  border: 1px solid hsl(var(--b3) / 0.7);
  background: rgb(255 255 255 / 0.94);
  padding: 0.8rem 1rem;
  box-shadow: 0 18px 40px rgb(15 23 42 / 0.14);
  cursor: grab;
  user-select: none;
}

.active-probe-edge-tab:active {
  cursor: grabbing;
}

.active-probe-edge-label {
  font-size: 0.75rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.active-probe-edge-filter {
  max-width: 10rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 0.7rem;
  color: rgb(15 23 42 / 0.55);
}

.active-probe-edge-count {
  border-radius: 999px;
  background: hsl(var(--p) / 0.12);
  padding: 0.2rem 0.55rem;
  font-size: 0.72rem;
  font-weight: 700;
  color: hsl(var(--p));
}

.active-probe-panel {
  position: relative;
  display: flex;
  flex-direction: column;
  border-radius: 28px;
  border: 1px solid hsl(var(--b3) / 0.75);
  background: rgb(255 255 255 / 0.96);
  padding: 1rem;
  overflow: hidden;
  box-shadow: 0 26px 64px rgb(15 23 42 / 0.18);
  backdrop-filter: blur(16px);
}

.active-probe-panel__body {
  min-height: 0;
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  padding-right: 0.2rem;
}

.active-probe-height-resize-handle {
  position: absolute;
  left: 1.2rem;
  right: 1.2rem;
  top: -6px;
  height: 12px;
  cursor: ns-resize;
}

.active-probe-resize-handle {
  position: absolute;
  left: -6px;
  top: 1.25rem;
  bottom: 1.25rem;
  width: 12px;
  cursor: ew-resize;
}

.active-probe-stat-card {
  border-radius: 20px;
  border: 1px solid hsl(var(--b3) / 0.65);
  background: hsl(var(--b1) / 0.84);
  padding: 0.8rem 0.9rem;
}

.active-probe-stat-card__label {
  font-size: 0.68rem;
  font-weight: 700;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: rgb(15 23 42 / 0.46);
}

.active-probe-stat-card__value {
  margin-top: 0.4rem;
  font-size: 1.2rem;
  font-weight: 700;
  color: hsl(var(--bc));
}

.active-probe-hot-key {
  min-width: 9rem;
  border-radius: 18px;
  border: 1px solid hsl(var(--b3) / 0.7);
  background: hsl(var(--b1) / 0.82);
  padding: 0.7rem 0.8rem;
  text-align: left;
}

.active-probe-hot-key--selected {
  border-color: hsl(var(--p) / 0.6);
  background: hsl(var(--p) / 0.08);
}

.active-probe-hot-key__path {
  font-size: 0.72rem;
  font-weight: 600;
  color: hsl(var(--bc));
}

.active-probe-hot-key__meta {
  margin-top: 0.2rem;
  font-size: 0.66rem;
  color: rgb(15 23 42 / 0.52);
}

.active-probe-item {
  border-radius: 22px;
  border: 1px solid hsl(var(--b3) / 0.7);
  background: hsl(var(--b1) / 0.88);
  padding: 0.9rem;
}

.active-probe-phase {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 5.25rem;
  border-radius: 999px;
  background: hsl(var(--p) / 0.12);
  padding: 0.2rem 0.55rem;
  font-size: 0.68rem;
  font-weight: 700;
  text-transform: uppercase;
  color: hsl(var(--p));
}

.active-probe-preview-block {
  margin-top: 0.85rem;
  max-height: 28rem;
  overflow: auto;
  border-radius: 16px;
  background: rgb(15 23 42 / 0.04);
  padding: 0.9rem;
  font-size: 0.72rem;
  line-height: 1.55;
  white-space: pre-wrap;
  word-break: break-word;
}

</style>
