<template>
  <section class="active-probe-floating-shell" :style="shellStyle">
    <button
      v-if="collapsed"
      type="button"
      class="active-probe-edge-tab"
      @click="$emit('toggleCollapsed')"
    >
      <span class="active-probe-edge-label">主动探测</span>
      <span
        v-if="selectedCooldownKeyLabel"
        class="active-probe-edge-filter"
        :title="selectedCooldownKey || ''"
      >
        {{ collapsedCooldownKeyLabel }}
      </span>
      <span class="active-probe-edge-count">{{ activeQueuedCount }}</span>
    </button>

    <section v-else class="active-probe-panel" :style="panelStyle">
      <div
        class="active-probe-resize-handle"
        title="拖拽调整宽度"
        @mousedown="startPanelResize"
      ></div>
      <div class="flex items-start justify-between gap-3">
        <div>
          <p class="text-[11px] font-semibold uppercase tracking-[0.18em] text-primary/75">
            Active Probes
          </p>
          <h3 class="mt-1 text-sm font-semibold text-base-content">主动探测</h3>
          <p
            v-if="selectedCooldownKeyLabel"
            class="mt-1 max-w-[20rem] truncate text-[11px] text-base-content/52"
            :title="selectedCooldownKey || ''"
          >
            当前筛选: {{ selectedCooldownKeyLabel }}
          </p>
        </div>
        <div class="flex items-center gap-2">
          <span class="rounded-full bg-primary/10 px-2.5 py-1 text-[11px] font-semibold text-primary">
            {{ activeQueuedCount }} 队列中待请求
          </span>
          <button
            type="button"
            class="btn btn-ghost btn-xs rounded-full"
            title="靠边收起"
            @click="$emit('toggleCollapsed')"
          >
            <i class="fas fa-angles-right"></i>
          </button>
        </div>
      </div>

      <div v-if="pluginOptions.length > 1" class="mt-3 flex flex-wrap items-center gap-2">
        <button
          type="button"
          class="btn btn-xs rounded-full"
          :class="selectedPluginId ? 'btn-ghost' : 'btn-primary'"
          @click="clearPluginFilter"
        >
          全部插件
        </button>
        <button
          v-for="pluginId in pluginOptions"
          :key="pluginId"
          type="button"
          class="btn btn-xs rounded-full"
          :class="selectedPluginId === pluginId ? 'btn-primary' : 'btn-ghost'"
          @click="togglePluginFilter(pluginId)"
        >
          {{ pluginId }}
        </button>
      </div>

      <div class="mt-3 grid gap-2 sm:grid-cols-2 xl:grid-cols-4">
        <div class="active-probe-stat-card">
          <p class="active-probe-stat-card__label">Scans</p>
          <p class="active-probe-stat-card__value">{{ funnelSummary.scanCount }}</p>
        </div>
        <div class="active-probe-stat-card">
          <p class="active-probe-stat-card__label">Targets</p>
          <p class="active-probe-stat-card__value">{{ funnelSummary.targetCount }}</p>
        </div>
        <div class="active-probe-stat-card">
          <p class="active-probe-stat-card__label">Replays</p>
          <p class="active-probe-stat-card__value">{{ funnelSummary.replayCount }}</p>
        </div>
        <div class="active-probe-stat-card">
          <p class="active-probe-stat-card__label">Findings</p>
          <p class="active-probe-stat-card__value">{{ funnelSummary.findingCount }}</p>
        </div>
      </div>

      <div v-if="topCooldownGroups.length" class="mt-3 space-y-2">
        <div class="flex items-center justify-between gap-3">
          <p class="text-[11px] font-semibold uppercase tracking-[0.14em] text-base-content/48">
            Hot Paths
          </p>
          <div class="flex items-center gap-2">
            <button
              v-if="selectedCooldownKey && latestFilteredTrafficRequestId"
              type="button"
              class="btn btn-outline btn-xs rounded-full"
              @click="$emit('openHistoryByTrafficRequestId', latestFilteredTrafficRequestId)"
            >
              打开最近历史
            </button>
            <button
              v-if="selectedCooldownKey"
              type="button"
              class="btn btn-ghost btn-xs rounded-full"
              @click="clearCooldownFilter"
            >
              清除筛选
            </button>
          </div>
        </div>
        <div class="flex flex-wrap gap-2">
          <button
            v-for="group in topCooldownGroups"
            :key="group.key"
            type="button"
            class="active-probe-hot-key"
            :class="{ 'active-probe-hot-key--selected': selectedCooldownKey === group.key }"
            :title="group.key"
            @click="toggleCooldownFilter(group.key)"
          >
            <p class="active-probe-hot-key__path">
              {{ group.label }}
            </p>
            <p class="active-probe-hot-key__meta">
              {{ formatCooldownGroupMeta(group) }}
            </p>
          </button>
        </div>
      </div>

      <div v-if="selectedTimeline.length" class="mt-3 space-y-2">
        <p class="text-[11px] font-semibold uppercase tracking-[0.14em] text-base-content/48">
          Timeline
        </p>
        <div class="space-y-2">
          <div
            v-for="entry in selectedTimeline"
            :key="`timeline:${entry.request_id}`"
            class="active-probe-timeline-item"
          >
            <span class="active-probe-phase">{{ formatPhase(entry.phase) }}</span>
            <span class="truncate text-[11px] text-base-content/66">
              {{ formatTimelineMeta(entry) }}
            </span>
          </div>
        </div>
      </div>

      <p v-if="entries.length === 0" class="mt-3 text-[11px] leading-5 text-base-content/55">
        当前没有正在进行的主动探测请求。
      </p>

      <p
        v-else-if="selectedCooldownKey && filteredEntries.length === 0"
        class="mt-3 text-[11px] leading-5 text-base-content/55"
      >
        当前筛选的 path `{{ selectedCooldownKeyLabel }}` 没有活跃事件，可能已经完成或被清理。
      </p>

      <div v-else class="mt-3 space-y-2">
        <article
          v-for="entry in filteredEntries"
          :key="entry.request_id"
          class="active-probe-item"
        >
          <div class="flex items-start justify-between gap-2">
            <div class="min-w-0 flex-1">
              <div class="flex min-w-0 items-center gap-2">
                <span class="active-probe-phase">{{ formatPhase(entry.phase) }}</span>
                <span class="truncate text-[11px] font-medium text-base-content/72">
                  {{ entry.method }}
                  {{ formatPath(entry.url) }}
                </span>
              </div>
              <p class="mt-1 truncate text-[11px] text-base-content/55">
                {{ formatMeta(entry) }}
              </p>
            </div>
            <div class="shrink-0 text-right">
              <p class="text-[11px] font-medium text-base-content/68">
                {{ entry.plugin_id || 'unknown-plugin' }}
              </p>
              <p class="mt-1 max-w-[12rem] truncate text-[10px] text-base-content/45" :title="entry.url">
                URL {{ formatUrlSummary(entry.url, 56) }}
              </p>
            </div>
          </div>

          <div class="mt-3 flex flex-wrap items-center gap-2">
            <button
              v-if="entry.traffic_request_id"
              type="button"
              class="btn btn-xs btn-outline rounded-full"
              @click="openPreview(entry)"
            >
              <i class="fas fa-eye"></i>
              <span>预览请求/响应</span>
            </button>
            <button
              v-if="entry.traffic_request_id && previewRequestMap[entry.traffic_request_id]"
              type="button"
              class="btn btn-xs btn-ghost rounded-full"
              @click="$emit('openHistory', previewRequestMap[entry.traffic_request_id]!.id)"
            >
              <i class="fas fa-arrow-up-right-from-square"></i>
              <span>打开详情</span>
            </button>
          </div>
        </article>
      </div>
    </section>
  </section>

  <AppModal
    :open="Boolean(previewEntry)"
    box-class="active-probe-preview-modal-box"
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
              {{ previewEntry.method }} {{ formatPath(previewEntry.url) }}
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

      <div class="min-h-0 flex-1 overflow-hidden bg-base-200/35 p-4">
        <div v-if="previewEntry" class="flex h-full min-h-0 flex-col gap-4">
          <div class="rounded-[22px] border border-base-300/65 bg-base-100/92 px-4 py-3">
            <div class="flex flex-wrap items-center gap-2">
              <span class="active-probe-phase">{{ formatPhase(previewEntry.phase) }}</span>
              <span class="text-xs font-medium text-base-content/72">
                {{ previewEntry.plugin_id || 'unknown-plugin' }}
              </span>
              <span class="max-w-full truncate text-[11px] text-base-content/55" :title="previewEntry.url">
                URL {{ formatUrlSummary(previewEntry.url, 88) }}
              </span>
            </div>
            <p class="mt-2 text-xs text-base-content/58">
              {{ formatMeta(previewEntry) }}
            </p>
          </div>

          <div class="preview-tab-group">
            <button
              type="button"
              class="preview-tab"
              :class="{ 'preview-tab-active': previewPane === 'request' }"
              @click="previewPane = 'request'"
            >
              请求
            </button>
            <button
              type="button"
              class="preview-tab"
              :class="{ 'preview-tab-active': previewPane === 'response' }"
              @click="previewPane = 'response'"
            >
              响应
            </button>
          </div>

          <div v-if="isPreviewLoading" class="rounded-[20px] border border-base-300/55 bg-base-100/85 px-4 py-4 text-sm text-base-content/58">
            正在加载请求详情...
          </div>
          <div v-else-if="previewRequest" class="flex min-h-0 flex-1 flex-col gap-3">
            <div v-if="previewTargetHighlight" class="probe-focus-card">
              <div class="probe-focus-header">
                <span class="probe-focus-kicker">Active Probe Focus</span>
                <span class="probe-focus-location">{{ previewTargetHighlight.locationLabel }}</span>
              </div>
              <div class="probe-focus-body">
                <div class="probe-focus-param-row">
                  <span class="probe-focus-label">参数</span>
                  <code class="probe-focus-param">{{ previewTargetHighlight.parameterPath }}</code>
                </div>
                <div class="probe-focus-poc-row">
                  <span class="probe-focus-label">POC</span>
                  <code class="probe-focus-poc">{{ previewTargetHighlight.probeValue }}</code>
                </div>
              </div>
            </div>
            <div v-else class="probe-focus-empty-card">
              <div class="probe-focus-header">
                <span class="probe-focus-kicker">Active Probe Focus</span>
                <span class="probe-focus-location">未提供</span>
              </div>
              <p class="mt-3 text-sm leading-6 text-base-content/62">
                当前事件没有携带探测参数和 POC 信息。通常这是旧主动探测事件，或者插件运行时还没有加载最新探测元数据。
              </p>
              <p class="mt-2 text-xs leading-5 text-base-content/52">
                重新触发一次新的主动探测后，这里会显示真实的参数路径和当前使用的 payload。
              </p>
            </div>

            <template v-if="previewPane === 'request'">
              <div class="preview-surface-shell">
                <HttpMessageSurface
                  ref="previewRequestSurface"
                  :model-value="previewRequestRaw"
                  readonly
                  message-type="request"
                  height="100%"
                  display-mode="raw"
                  show-search-bar
                  :state-key="previewRequestStateKey"
                  :search-placeholder="t('trafficAnalysis.messageSearch.placeholder')"
                  :search-next-title="t('trafficAnalysis.messageSearch.next')"
                  :search-previous-title="t('trafficAnalysis.messageSearch.previous')"
                  :search-case-sensitive-title="t('trafficAnalysis.messageSearch.caseSensitive')"
                  :search-regexp-title="t('trafficAnalysis.messageSearch.regexp')"
                  :search-clear-title="t('trafficAnalysis.messageSearch.clear')"
                  :search-no-matches-text="t('trafficAnalysis.messageSearch.noMatches')"
                  :search-invalid-regexp-text="t('trafficAnalysis.messageSearch.invalidRegexp')"
                  :show-display-toolbar="false"
                />
              </div>
            </template>

            <template v-else>
              <div class="preview-surface-shell">
                <HttpMessageSurface
                  :model-value="previewResponseRaw"
                  readonly
                  message-type="response"
                  height="100%"
                  display-mode="raw"
                  show-search-bar
                  :state-key="previewResponseStateKey"
                  :search-placeholder="t('trafficAnalysis.messageSearch.placeholder')"
                  :search-next-title="t('trafficAnalysis.messageSearch.next')"
                  :search-previous-title="t('trafficAnalysis.messageSearch.previous')"
                  :search-case-sensitive-title="t('trafficAnalysis.messageSearch.caseSensitive')"
                  :search-regexp-title="t('trafficAnalysis.messageSearch.regexp')"
                  :search-clear-title="t('trafficAnalysis.messageSearch.clear')"
                  :search-no-matches-text="t('trafficAnalysis.messageSearch.noMatches')"
                  :search-invalid-regexp-text="t('trafficAnalysis.messageSearch.invalidRegexp')"
                  :show-display-toolbar="false"
                />
              </div>
            </template>
          </div>
          <div v-else class="rounded-[20px] border border-base-300/55 bg-base-100/85 px-4 py-4 text-sm text-base-content/58">
            暂时找不到请求详情。
          </div>
        </div>
      </div>
    </div>
  </AppModal>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import type { CSSProperties, PropType } from 'vue'
import { useI18n } from 'vue-i18n'
import AppModal from '@/components/AppModal.vue'
import HttpMessageSurface from '@/components/http-editor/HttpMessageSurface.vue'
import { formatRequestRaw, formatResponseRaw } from './proxyHistoryFormattingSupport'
import type { ProxyRequest } from './proxyHistoryTypes'
import {
  buildActiveProbeCooldownGroups,
  buildActiveProbeFunnel,
  buildActiveProbePluginOptions,
  buildActiveProbeTimeline,
  filterActiveProbeEntries,
  formatCooldownKeyLabel as formatCooldownKeyLabelFromInsights,
} from './trafficActiveProbeInsights'
import { resolveActiveProbeTargetHighlight } from './trafficActiveProbePreviewSupport'
import type { ActiveProbeEntry } from './trafficActiveProbeTypes'

const props = defineProps({
  shellStyle: {
    type: Object as PropType<CSSProperties>,
    required: true,
  },
  entries: {
    type: Array as PropType<ActiveProbeEntry[]>,
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

const ACTIVE_PROBE_PANEL_WIDTH_STORAGE_KEY = 'trafficAnalysis.activeProbe.panelWidth.v1'
const ACTIVE_PROBE_FILTER_STORAGE_KEY = 'trafficAnalysis.activeProbe.filters.v1'
const ACTIVE_PROBE_PANEL_DEFAULT_WIDTH = 448
const ACTIVE_PROBE_PANEL_MIN_WIDTH = 336
const ACTIVE_PROBE_PANEL_MAX_WIDTH = 720

const emit = defineEmits<{
  toggleCollapsed: []
  ensurePreview: [trafficRequestId: string]
  openHistory: [requestId: number]
  openHistoryByTrafficRequestId: [trafficRequestId: string]
  previewPinnedChange: [requestId: string | null]
}>()

const { t } = useI18n()
const viewportWidth = ref(typeof window === 'undefined' ? 1440 : window.innerWidth)
const panelWidth = ref(loadStoredPanelWidth())
const panelResizeState = ref<{ startX: number; startWidth: number } | null>(null)
const previewEntry = ref<ActiveProbeEntry | null>(null)
const previewPane = ref<'request' | 'response'>('request')
const previewRequestSurface = ref<InstanceType<typeof HttpMessageSurface> | null>(null)
const selectedCooldownKey = ref<string | null>(null)
const selectedPluginId = ref<string | null>(null)

const panelStyle = computed(() => ({
  width: `${clampPanelWidth(panelWidth.value, viewportWidth.value)}px`,
}))
const selectedCooldownKeyLabel = computed(() =>
  selectedCooldownKey.value ? formatCooldownKeyLabel(selectedCooldownKey.value) : '',
)
const collapsedCooldownKeyLabel = computed(() => {
  if (!selectedCooldownKey.value) {
    return ''
  }

  const label = formatCooldownKeyLabel(selectedCooldownKey.value)
  return label.length > 16 ? `${label.slice(0, 13)}...` : label
})
const pluginOptions = computed(() => buildActiveProbePluginOptions(props.entries))
const pluginFilteredEntries = computed(() => filterActiveProbeEntries(props.entries, {
  pluginId: selectedPluginId.value,
}))
const filteredEntries = computed(() => filterActiveProbeEntries(pluginFilteredEntries.value, {
  cooldownKey: selectedCooldownKey.value,
}))
const activeQueuedCount = computed(() =>
  filteredEntries.value.filter(entry => entry.phase === 'queued' || entry.phase === 'scheduled').length,
)
const topCooldownGroups = computed(() => buildActiveProbeCooldownGroups(pluginFilteredEntries.value).slice(0, 4))
const funnelSummary = computed(() => buildActiveProbeFunnel(filteredEntries.value))
const selectedTimeline = computed(() => buildActiveProbeTimeline(filteredEntries.value))
const latestFilteredTrafficRequestId = computed(() =>
  filteredEntries.value.find(entry => Boolean(entry.traffic_request_id))?.traffic_request_id || '',
)

const previewRequest = computed(() => {
  const trafficRequestId = previewEntry.value?.traffic_request_id
  if (!trafficRequestId) {
    return null
  }
  return props.previewRequestMap[trafficRequestId] || null
})

const isPreviewLoading = computed(() => {
  const trafficRequestId = previewEntry.value?.traffic_request_id
  return Boolean(trafficRequestId && props.loadingTrafficRequestId === trafficRequestId)
})

const previewRequestRaw = computed(() => {
  if (!previewRequest.value) {
    return ''
  }
  return formatRequestRaw(previewRequest.value, 'edited')
})

const previewTargetHighlight = computed(() => {
  if (!previewEntry.value || !previewRequestRaw.value) {
    return null
  }
  return resolveActiveProbeTargetHighlight(previewRequestRaw.value, previewEntry.value)
})

const previewResponseRaw = computed(() => {
  if (!previewRequest.value) {
    return ''
  }
  return formatResponseRaw(previewRequest.value, 'edited', {
    bodyText: getEffectiveResponseBody(previewRequest.value),
  })
})

const previewRequestStateKey = computed(() => {
  if (!previewEntry.value?.request_id) {
    return ''
  }
  return `active-probe:${previewEntry.value.request_id}:request:raw`
})

const previewResponseStateKey = computed(() => {
  if (!previewEntry.value?.request_id) {
    return ''
  }
  return `active-probe:${previewEntry.value.request_id}:response:raw`
})

watch(
  () => props.entries,
  (entries) => {
    if (selectedPluginId.value && !entries.some(entry => entry.plugin_id === selectedPluginId.value)) {
      selectedPluginId.value = null
    }
    if (selectedCooldownKey.value && !entries.some(entry => entry.cooldown_key === selectedCooldownKey.value)) {
      selectedCooldownKey.value = null
    }

    if (!previewEntry.value) {
      return
    }

    const latest = entries.find(entry => entry.request_id === previewEntry.value?.request_id)
    if (latest) {
      previewEntry.value = { ...latest }
    }
  },
  { deep: true },
)

watch(
  () => [selectedPluginId.value, selectedCooldownKey.value],
  () => {
    localStorage.setItem(
      ACTIVE_PROBE_FILTER_STORAGE_KEY,
      JSON.stringify({
        pluginId: selectedPluginId.value,
        cooldownKey: selectedCooldownKey.value,
      }),
    )
  },
  { deep: true },
)

watch(
  () => [
    previewPane.value,
    previewRequestRaw.value,
    previewTargetHighlight.value?.selection?.from ?? -1,
    previewTargetHighlight.value?.selection?.to ?? -1,
  ] as const,
  async () => {
    if (previewPane.value !== 'request') {
      return
    }

    const selection = previewTargetHighlight.value?.selection
    if (!selection) {
      return
    }

    await nextTick()
    previewRequestSurface.value?.setSelection?.(selection.from, selection.to)
  },
  { immediate: true },
)

function formatPhase(phase: string) {
  switch (phase) {
    case 'queued':
      return '排队中'
    case 'scheduled':
      return '等待发送'
    case 'dispatching':
      return '已发出'
    case 'completed':
      return '已完成'
    case 'failed':
      return '失败'
    case 'skipped':
      return '已跳过'
    case 'scan_started':
      return '开始探测'
    case 'plugin_invoked':
      return '插件已触发'
    case 'plugin_completed':
      return '插件已完成'
    case 'plugin_failed':
      return '插件失败'
    default:
      return phase
  }
}

function formatReason(reason: string) {
  switch (reason) {
    case 'no_response':
      return '无响应，未进入主动探测'
    case 'unsafe_request':
      return '请求不满足安全重放条件'
    case 'baseline_replay_failed':
      return 'baseline 重放失败'
    case 'dynamic_baseline':
      return 'baseline 不稳定，已跳过'
    case 'no_targets':
      return '未找到可变异参数'
    default:
      return reason
  }
}

function formatPath(url: string) {
  return formatUrlSummary(url, 80)
}

function formatCooldownKeyLabel(key: string) {
  return formatCooldownKeyLabelFromInsights(key, 48)
}

function formatUrlSummary(url: string, maxLength: number) {
  try {
    const parsed = new URL(url)
    const summary = `${parsed.host}${parsed.pathname}${parsed.search}` || url
    return summary.length > maxLength ? `${summary.slice(0, maxLength - 3)}...` : summary
  } catch {
    return url.length > maxLength ? `${url.slice(0, maxLength - 3)}...` : url
  }
}

function formatMeta(entry: ActiveProbeEntry) {
  if (entry.phase === 'scheduled' && typeof entry.total_wait_ms === 'number') {
    const occupancy = formatConcurrencyMeta(entry)
    const responseTime = typeof entry.response_elapsed_ms === 'number' ? `响应 ${entry.response_elapsed_ms}ms` : ''
    return [ `等待 ${entry.total_wait_ms}ms`, occupancy, responseTime ].filter(Boolean).join(' · ')
  }
  if (entry.phase === 'queued') {
    const occupancy = formatConcurrencyMeta(entry)
    if (occupancy) {
      return occupancy
    }
  }
  if (entry.phase === 'completed' && typeof entry.status === 'number') {
    const waitTime = typeof entry.total_wait_ms === 'number' ? `等待 ${entry.total_wait_ms}ms` : ''
    const responseTime = typeof entry.response_elapsed_ms === 'number' ? `响应 ${entry.response_elapsed_ms}ms` : ''
    return [`状态 ${entry.status}`, waitTime, responseTime].filter(Boolean).join(' · ')
  }
  if (entry.phase === 'dispatching') {
    const occupancy = formatConcurrencyMeta(entry)
    return occupancy ? `已发出 · ${occupancy}` : '已发出'
  }
  if (entry.phase === 'failed' && entry.error) {
    return entry.error.length > 72 ? `${entry.error.slice(0, 69)}...` : entry.error
  }
  if (entry.phase === 'skipped' && entry.reason) {
    return formatReason(entry.reason)
  }
  if (entry.phase === 'scan_started') {
    const count = typeof entry.target_count === 'number' ? entry.target_count : 0
    return `开始主动探测 ${count} 个参数`
  }
  if (entry.phase === 'plugin_completed') {
    const count = typeof entry.target_count === 'number' ? entry.target_count : 0
    return `插件执行完成，发现 ${count} 条结果`
  }
  if (entry.phase === 'plugin_invoked') {
    return 'scan_transaction 已进入插件运行时'
  }
  if (entry.probe_label) {
    return entry.probe_label
  }
  return entry.plugin_id || '主动探测'
}

function formatConcurrencyMeta(entry: ActiveProbeEntry) {
  const occupancy =
    typeof entry.active_slots === 'number' && typeof entry.max_concurrent_per_host === 'number'
      ? `${entry.active_slots}/${entry.max_concurrent_per_host} 并发`
      : ''
  const queue =
    typeof entry.queue_depth === 'number' && entry.queue_depth > 0
      ? `${entry.queue_depth} 排队`
      : ''

  if (occupancy && queue) {
    return `${occupancy} · ${queue}`
  }
  return occupancy || queue || ''
}

function toggleCooldownFilter(key: string) {
  selectedCooldownKey.value = selectedCooldownKey.value === key ? null : key
}

function togglePluginFilter(pluginId: string) {
  selectedPluginId.value = selectedPluginId.value === pluginId ? null : pluginId
}

function clearPluginFilter() {
  selectedPluginId.value = null
}

function clearCooldownFilter() {
  selectedCooldownKey.value = null
}

function formatCooldownGroupMeta(group: {
  maxQueueDepth: number
  maxActiveSlots: number
  maxConcurrentPerHost: number
  sampleCount: number
  averageWaitMs: number
  averageResponseMs: number
  failureRate: number
}) {
  const queue = group.maxQueueDepth > 0 ? `${group.maxQueueDepth} 排队` : ''
  const occupancy =
    group.maxActiveSlots > 0 && group.maxConcurrentPerHost > 0
      ? `${group.maxActiveSlots}/${group.maxConcurrentPerHost} 并发`
      : ''
  const sampleCount = group.sampleCount > 1 ? `${group.sampleCount} 条事件` : ''
  const averageWait = group.averageWaitMs > 0 ? `均等 ${group.averageWaitMs}ms` : ''
  const averageResponse = group.averageResponseMs > 0 ? `均响 ${group.averageResponseMs}ms` : ''
  const failureRate = group.failureRate > 0 ? `失败 ${(group.failureRate * 100).toFixed(0)}%` : ''

  return [queue, occupancy, averageWait, averageResponse, failureRate, sampleCount].filter(Boolean).join(' · ')
}

function formatTimelineMeta(entry: ActiveProbeEntry) {
  const timeLabel = new Date(entry.lastUpdatedAt).toLocaleTimeString([], {
    hour12: false,
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  })
  return `${timeLabel} · ${formatPath(entry.url)} · ${formatMeta(entry)}`
}

function openPreview(entry: ActiveProbeEntry) {
  if (!entry.traffic_request_id) {
    return
  }

  previewEntry.value = { ...entry }
  previewPane.value = 'request'
  emit('ensurePreview', entry.traffic_request_id)
  emit('previewPinnedChange', entry.request_id)
}

function closePreview() {
  previewEntry.value = null
  emit('previewPinnedChange', null)
}

function getEffectiveResponseBody(request: ProxyRequest) {
  return request.was_edited && request.edited_response_body
    ? request.edited_response_body
    : request.response_body || ''
}

function loadStoredPanelWidth() {
  const raw = localStorage.getItem(ACTIVE_PROBE_PANEL_WIDTH_STORAGE_KEY)
  const parsed = raw ? Number.parseInt(raw, 10) : ACTIVE_PROBE_PANEL_DEFAULT_WIDTH
  return Number.isFinite(parsed) ? parsed : ACTIVE_PROBE_PANEL_DEFAULT_WIDTH
}

function clampPanelWidth(width: number, viewport: number) {
  const maxViewportWidth = Math.max(280, viewport - 48)
  const minWidth = Math.min(ACTIVE_PROBE_PANEL_MIN_WIDTH, maxViewportWidth)
  const maxWidth = Math.min(ACTIVE_PROBE_PANEL_MAX_WIDTH, maxViewportWidth)
  return Math.min(Math.max(width, minWidth), maxWidth)
}

function persistPanelWidth() {
  localStorage.setItem(
    ACTIVE_PROBE_PANEL_WIDTH_STORAGE_KEY,
    String(clampPanelWidth(panelWidth.value, viewportWidth.value)),
  )
}

function handleWindowResize() {
  viewportWidth.value = window.innerWidth
  panelWidth.value = clampPanelWidth(panelWidth.value, viewportWidth.value)
}

function handlePanelResize(event: MouseEvent) {
  if (!panelResizeState.value) {
    return
  }

  const delta = panelResizeState.value.startX - event.clientX
  panelWidth.value = clampPanelWidth(
    panelResizeState.value.startWidth + delta,
    viewportWidth.value,
  )
}

function stopPanelResize() {
  if (!panelResizeState.value) {
    return
  }

  panelResizeState.value = null
  document.body.style.userSelect = ''
  document.body.style.cursor = ''
  window.removeEventListener('mousemove', handlePanelResize)
  window.removeEventListener('mouseup', stopPanelResize)
  persistPanelWidth()
}

function startPanelResize(event: MouseEvent) {
  event.preventDefault()
  event.stopPropagation()
  panelResizeState.value = {
    startX: event.clientX,
    startWidth: panelWidth.value,
  }
  document.body.style.userSelect = 'none'
  document.body.style.cursor = 'ew-resize'
  window.addEventListener('mousemove', handlePanelResize)
  window.addEventListener('mouseup', stopPanelResize)
}

onMounted(() => {
  panelWidth.value = clampPanelWidth(panelWidth.value, viewportWidth.value)
  try {
    const raw = localStorage.getItem(ACTIVE_PROBE_FILTER_STORAGE_KEY)
    if (raw) {
      const parsed = JSON.parse(raw) as { pluginId?: string | null; cooldownKey?: string | null }
      selectedPluginId.value = parsed.pluginId || null
      selectedCooldownKey.value = parsed.cooldownKey || null
    }
  } catch {
    selectedPluginId.value = null
    selectedCooldownKey.value = null
  }
  window.addEventListener('resize', handleWindowResize)
})

onUnmounted(() => {
  stopPanelResize()
  window.removeEventListener('resize', handleWindowResize)
})
</script>

<style scoped>
.active-probe-floating-shell {
  position: fixed;
  right: 1.5rem;
  bottom: 1.5rem;
  z-index: 39;
  pointer-events: none;
}

.active-probe-panel,
.active-probe-edge-tab {
  pointer-events: auto;
}

.active-probe-panel {
  position: relative;
  width: min(28rem, calc(100vw - 3rem));
  max-height: min(70vh, 52rem);
  overflow: auto;
  border: 1px solid hsl(var(--b3) / 0.72);
  border-radius: 1.4rem;
  background:
    linear-gradient(180deg, rgb(255 255 255 / 0.98), rgb(248 250 252 / 0.96)),
    hsl(var(--b1));
  box-shadow:
    0 18px 42px rgb(15 23 42 / 0.12),
    inset 0 1px 0 rgb(255 255 255 / 0.8);
  padding: 1rem;
  backdrop-filter: blur(14px);
}

.active-probe-resize-handle {
  position: absolute;
  top: 0.9rem;
  bottom: 0.9rem;
  left: -0.45rem;
  width: 0.9rem;
  cursor: ew-resize;
  border-radius: 999px;
}

.active-probe-resize-handle::before {
  content: '';
  position: absolute;
  top: 50%;
  left: 50%;
  width: 0.2rem;
  height: 4.5rem;
  border-radius: 999px;
  background: hsl(var(--b3) / 0.8);
  transform: translate(-50%, -50%);
  box-shadow:
    0 0 0 1px rgb(255 255 255 / 0.9),
    0 10px 24px rgb(15 23 42 / 0.12);
}

.active-probe-edge-tab {
  display: inline-flex;
  flex-direction: column;
  align-items: center;
  gap: 0.55rem;
  min-height: 8.5rem;
  width: 3.2rem;
  border: 1px solid hsl(var(--b3) / 0.72);
  border-right-width: 0;
  border-radius: 1.2rem 0 0 1.2rem;
  background:
    linear-gradient(180deg, rgb(255 255 255 / 0.98), rgb(248 250 252 / 0.96)),
    hsl(var(--b1));
  box-shadow: 0 18px 42px rgb(15 23 42 / 0.12);
  padding: 0.9rem 0.45rem;
}

.active-probe-edge-label {
  writing-mode: vertical-rl;
  text-orientation: mixed;
  font-size: 0.72rem;
  font-weight: 700;
  letter-spacing: 0.18em;
  color: hsl(var(--bc) / 0.72);
}

.active-probe-edge-filter {
  max-height: 7rem;
  overflow: hidden;
  writing-mode: vertical-rl;
  text-orientation: mixed;
  font-size: 0.58rem;
  font-weight: 600;
  letter-spacing: 0.08em;
  color: hsl(var(--bc) / 0.5);
}

.active-probe-edge-count {
  border-radius: 999px;
  background: hsl(var(--p) / 0.12);
  color: hsl(var(--p));
  font-size: 0.72rem;
  font-weight: 700;
  line-height: 1;
  padding: 0.35rem 0.45rem;
}

.active-probe-item {
  border: 1px solid hsl(var(--b3) / 0.55);
  border-radius: 1rem;
  background: rgb(255 255 255 / 0.88);
  padding: 0.7rem 0.8rem;
}

.active-probe-hot-key {
  appearance: none;
  text-align: left;
  min-width: 0;
  flex: 1 1 10rem;
  border: 1px solid hsl(var(--b3) / 0.5);
  border-radius: 0.95rem;
  background:
    linear-gradient(180deg, rgb(255 255 255 / 0.92), rgb(248 250 252 / 0.88)),
    hsl(var(--b1));
  padding: 0.6rem 0.75rem;
  transition:
    border-color 140ms ease,
    background-color 140ms ease,
    box-shadow 140ms ease,
    transform 140ms ease;
}

.active-probe-hot-key:hover {
  border-color: hsl(var(--p) / 0.38);
  box-shadow: 0 10px 22px rgb(15 23 42 / 0.08);
}

.active-probe-hot-key--selected {
  border-color: hsl(var(--p) / 0.42);
  background:
    linear-gradient(180deg, rgb(255 255 255 / 0.96), rgb(239 246 255 / 0.92)),
    hsl(var(--b1));
  box-shadow:
    inset 0 0 0 1px hsl(var(--p) / 0.14),
    0 10px 24px rgb(59 130 246 / 0.08);
}

.active-probe-hot-key__path {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 0.71rem;
  font-weight: 700;
  color: hsl(var(--bc) / 0.76);
}

.active-probe-hot-key__meta {
  margin-top: 0.2rem;
  font-size: 0.66rem;
  color: hsl(var(--bc) / 0.54);
}

.active-probe-stat-card {
  border: 1px solid hsl(var(--b3) / 0.5);
  border-radius: 0.9rem;
  background: rgb(255 255 255 / 0.88);
  padding: 0.65rem 0.75rem;
}

.active-probe-stat-card__label {
  font-size: 0.66rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: hsl(var(--bc) / 0.48);
}

.active-probe-stat-card__value {
  margin-top: 0.2rem;
  font-size: 1rem;
  font-weight: 700;
  color: hsl(var(--bc) / 0.82);
}

.active-probe-phase {
  display: inline-flex;
  align-items: center;
  border-radius: 999px;
  background: hsl(var(--p) / 0.12);
  color: hsl(var(--p));
  font-size: 0.68rem;
  font-weight: 700;
  line-height: 1;
  padding: 0.28rem 0.5rem;
  white-space: nowrap;
}

.active-probe-timeline-item {
  display: flex;
  align-items: center;
  gap: 0.55rem;
  border: 1px solid hsl(var(--b3) / 0.45);
  border-radius: 0.9rem;
  background: rgb(255 255 255 / 0.78);
  padding: 0.45rem 0.6rem;
}

.preview-tab {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 5rem;
  border-radius: 999px;
  border: 1px solid hsl(var(--b3) / 0.82);
  background: rgb(255 255 255 / 0.88);
  padding: 0.48rem 1rem;
  font-size: 0.76rem;
  font-weight: 700;
  color: hsl(var(--bc) / 0.72);
  box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.92);
  transition:
    transform 140ms ease,
    background-color 140ms ease,
    border-color 140ms ease,
    box-shadow 140ms ease,
    color 140ms ease;
}

.preview-tab-group {
  display: inline-flex;
  align-items: center;
  gap: 0.6rem;
  padding: 0.35rem;
  border: 1px solid hsl(var(--b3) / 0.6);
  border-radius: 999px;
  background: hsl(var(--b1) / 0.8);
  width: fit-content;
}

.preview-tab:hover {
  transform: translateY(-1px);
  border-color: hsl(var(--b3) / 0.95);
}

.preview-tab-active {
  border-color: hsl(var(--p));
  background: linear-gradient(180deg, hsl(var(--p)), hsl(var(--pf)));
  color: hsl(var(--pc));
  box-shadow:
    0 10px 22px hsl(var(--p) / 0.22),
    inset 0 1px 0 rgb(255 255 255 / 0.2);
}

.preview-surface-shell {
  min-height: 0;
  flex: 1;
  display: flex;
  overflow: hidden;
  border: 1px solid hsl(var(--b3) / 0.55);
  border-radius: 1rem;
  background: rgb(255 255 255 / 0.78);
}

.probe-focus-card {
  border: 1px solid hsl(var(--wa) / 0.26);
  border-radius: 1rem;
  background:
    linear-gradient(180deg, hsl(var(--wa) / 0.12), rgb(255 255 255 / 0.94));
  padding: 0.9rem 1rem;
  box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.8);
}

.probe-focus-empty-card {
  border: 1px solid hsl(var(--b3) / 0.55);
  border-radius: 1rem;
  background:
    linear-gradient(180deg, rgb(248 250 252 / 0.96), rgb(255 255 255 / 0.92));
  padding: 0.9rem 1rem;
  box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.8);
}

.probe-focus-header {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.6rem;
}

.probe-focus-kicker {
  font-size: 0.68rem;
  font-weight: 800;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  color: hsl(var(--wa));
}

.probe-focus-location {
  display: inline-flex;
  align-items: center;
  border-radius: 999px;
  background: hsl(var(--wa) / 0.14);
  color: hsl(var(--wa));
  padding: 0.22rem 0.55rem;
  font-size: 0.68rem;
  font-weight: 700;
}

.probe-focus-body {
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
  margin-top: 0.75rem;
}

.probe-focus-param-row,
.probe-focus-poc-row {
  display: flex;
  align-items: flex-start;
  gap: 0.65rem;
}

.probe-focus-label {
  flex: 0 0 auto;
  padding-top: 0.18rem;
  font-size: 0.72rem;
  font-weight: 700;
  color: hsl(var(--bc) / 0.55);
}

.probe-focus-param,
.probe-focus-poc {
  flex: 1;
  min-width: 0;
  border-radius: 0.7rem;
  background: rgb(15 23 42 / 0.05);
  padding: 0.45rem 0.6rem;
  font-size: 0.75rem;
  line-height: 1.5;
  color: hsl(var(--bc) / 0.84);
  word-break: break-word;
}

:global(.active-probe-preview-modal-box) {
  width: min(72rem, 92vw);
  max-width: 72rem;
  height: min(46rem, 84vh);
  max-height: 84vh;
  overflow: hidden;
}
</style>
