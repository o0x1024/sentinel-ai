<template>
  <div class="flex h-full flex-col overflow-hidden bg-base-200" @contextmenu.prevent>
    <AppDialog ref="certErrorDialog" class="modal">
      <div class="modal-box max-w-2xl">
        <h3 class="mb-4 flex items-center gap-2 text-lg font-bold">
          <i class="fas fa-exclamation-triangle text-warning"></i>
          {{ $t('trafficAnalysis.history.certificateError.title') }}
        </h3>

        <div class="space-y-4">
          <div class="alert alert-warning">
            <i class="fas fa-info-circle"></i>
            <span>{{ $t('trafficAnalysis.history.certificateError.message') }}</span>
          </div>

          <div v-if="certErrorInfo" class="rounded-lg bg-base-200 p-4">
            <h4 class="mb-2 text-sm font-semibold">{{ $t('trafficAnalysis.history.certificateError.details') }}</h4>
            <div class="space-y-1 font-mono text-xs">
              <div><span class="text-base-content/70">Host:</span> {{ certErrorInfo.host }}</div>
              <div><span class="text-base-content/70">URL:</span> {{ certErrorInfo.url }}</div>
              <div v-if="certErrorInfo.error"><span class="text-base-content/70">Error:</span> {{ certErrorInfo.error }}</div>
            </div>
          </div>
        </div>

        <div class="modal-action justify-between">
          <button class="btn btn-sm btn-ghost" @click="closeCertErrorDialog">
            {{ $t('trafficAnalysis.history.detailsPanel.close') }}
          </button>
          <div class="flex gap-2">
            <button class="btn btn-sm btn-info" @click="checkCAInstallation">
              <i class="fas fa-certificate mr-2"></i>
              {{ $t('trafficAnalysis.history.certificateError.tips.checkCAInstallation') }}
            </button>
            <button class="btn btn-sm btn-primary" @click="closeCertErrorDialog">
              {{ $t('trafficAnalysis.history.certificateError.actions.ignore') }}
            </button>
          </div>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button>{{ $t('trafficAnalysis.history.detailsPanel.close') }}</button>
      </form>
    </AppDialog>

    <TrafficContextCandidateDialog
      :open="showContextCandidateDialog"
      :loading="contextCandidateLoading"
      :applying="contextCandidateApplying"
      :previewing="contextCandidatePreviewLoading"
      :result="contextCandidateResult"
      :preview-result="contextCandidatePreviewResult"
      :selected-candidate-ids="selectedCandidateIds"
      :preferred-preview-focus="preferredPreviewFocus"
      @close="closeContextCandidateDialog"
      @apply="applySelectedContextCandidates"
      @preview="previewSelectedContextCandidates"
      @open-evidence-request="openContextCandidateEvidenceRequest"
      @update:selected-candidate-ids="updateSelectedCandidateIds"
      @update:preferred-preview-focus="preferredPreviewFocus = $event"
    />

    <TrafficHistoryFilterDialog
      :open="showHistoryFilterDialog"
      :config="appliedFilterConfig"
      @close="showHistoryFilterDialog = false"
      @apply="applyHistoryFilters"
    />

    <TrafficScopeRuleDialog
      ref="scopeRuleDialogRef"
      @save="saveScopeRuleFromDialog"
    />

    <div
      v-if="contextMenu.visible"
      ref="contextMenuRef"
      class="fixed z-50 min-w-48 rounded-lg border border-base-300 bg-base-100 py-1 shadow-xl"
      :style="{ left: `${contextMenu.x}px`, top: `${contextMenu.y}px` }"
      @click.stop
    >
      <TrafficContextMenuSections
        :sections="historyContextMenuSections"
        label-prefix="trafficAnalysis.history.contextMenu"
      />
      <div class="divider my-1 h-0"></div>
      <TrafficContextSubmenu
        v-if="historyFilterSubmenu"
        :submenu="historyFilterSubmenu"
        label-prefix="trafficAnalysis.history.contextMenu"
      />
      <TrafficContextSubmenu
        v-if="historyScopeSubmenu"
        :submenu="historyScopeSubmenu"
        label-prefix="trafficAnalysis.history.contextMenu"
      />
      <div class="divider my-1 h-0"></div>
      <button
        type="button"
        class="flex w-full items-center gap-2 px-4 py-2 text-left text-sm hover:bg-base-200"
        @click="addContextRequestToBasket"
      >
        <i class="fas fa-basket-shopping text-primary"></i>
        加入请求篮子
      </button>
      <div class="divider my-1 h-0"></div>
      <button
        type="button"
        class="flex w-full items-center gap-2 px-4 py-2 text-left text-sm text-error hover:bg-base-200"
        @click="clearHistoryFromMenu"
      >
        <i class="fas fa-trash"></i>
        {{ $t('trafficAnalysis.history.contextMenu.clearHistory') }}
      </button>
    </div>

    <div
      v-if="detailContextMenu.visible"
      class="fixed z-50 min-w-48 rounded-lg border border-base-300 bg-base-100 py-1 shadow-xl"
      :style="{ left: `${detailContextMenu.x}px`, top: `${detailContextMenu.y}px` }"
      @click.stop
    >
      <TrafficContextMenuSections
        :sections="historyDetailContextMenuSections"
        label-prefix="trafficAnalysis.history.contextMenu"
      />
      <div v-if="detailContextMenu.pane === 'request'" class="divider my-1 h-0"></div>
      <button
        v-if="detailContextMenu.pane === 'request'"
        type="button"
        class="flex w-full items-center gap-2 px-4 py-2 text-left text-sm hover:bg-base-200"
        @click="addDetailRequestToBasket"
      >
        <i class="fas fa-basket-shopping text-primary"></i>
        加入请求篮子
      </button>
    </div>

    <div ref="mainContainer" class="flex min-h-0 flex-1 flex-col overflow-hidden">
      <div
        ref="topPanel"
        class="flex flex-shrink-0 flex-col overflow-hidden border-b border-base-300 bg-base-100"
        :style="historyTopPanelStyle"
      >
        <TrafficHistoryWorkbenchToolbar
          :protocol-filter="protocolFilter"
          :has-active-filters="hasActiveFilters"
          :is-multi-select-mode="isMultiSelectMode"
          :selected-count="selectedRequests.size"
          :filtered-count="filteredRequests.length"
          :open-filter-dialog="openHistoryFilterDialog"
          :toggle-multi-select-mode="toggleMultiSelectMode"
          :select-all-visible="selectAllVisible"
          :show-send-to-repeater="enabledTargets.draft"
          :show-send-to-comparer="enabledTargets.compare"
          :show-send-to-intruder="enabledTargets.attackWorkspace"
          :send-selected-to-draft="sendSelectedToDraft"
          :send-selected-to-comparer="sendSelectedToComparer"
          :send-selected-to-intruder="sendSelectedToIntruder"
          :send-selected-to-assistant="sendSelectedToAssistant"
          :export-selected-to-file="exportSelectedToFile"
          :export-as-har="exportAsHar"
          :generate-candidates-from-filtered="generateCandidatesFromFiltered"
          :generate-candidates-from-selection="generateCandidatesFromSelection"
          :refresh-requests="refreshRequests"
          :clear-history="clearHistory"
          @update:protocol-filter="protocolFilter = $event"
        />

        <div ref="scrollContainer" class="flex-1 overflow-auto min-h-0" @scroll="handleScroll">
          <ProxyHistoryTopContent
            :is-loading="isLoading"
            :is-loading-ws="isLoadingWs"
            :protocol-filter="protocolFilter"
            :ws-connections="wsConnections"
            :expanded-ws-connections="expandedWsConnections"
            :toggle-ws-connection="toggleWsConnection"
            :format-ws-time="formatWsTime"
            :get-ws-active-tab="getWsActiveTab"
            :set-ws-active-tab="setWsActiveTab"
            :get-ws-messages-for-connection="getWsMessagesForConnection"
            :truncate-ws-content="truncateWsContent"
            :filtered-requests="filteredRequests"
            :total-height="totalHeight"
            :header-height="headerHeight"
            :item-height="itemHeight"
            :is-multi-select-mode="isMultiSelectMode"
            :selected-requests="selectedRequests"
            :clear-selection="clearSelection"
            :select-all-visible="selectAllVisible"
            :visible-columns="visibleColumns"
            :sort-state="sortState"
            :toggle-sort="toggleSort"
            :start-resize="startResize"
            :dragged-column-id="draggedColumnId"
            :column-drop-target-id="columnDropTargetId"
            :column-drop-position="columnDropPosition"
            :start-column-drag="startColumnDrag"
            :visible-rows="visibleRows"
            :selected-request="selectedRequest"
            :is-request-selected="isRequestSelected"
            :toggle-select-request="toggleSelectRequest"
            :show-certificate-error="showCertificateError"
            :select-request="selectRequest"
            :show-context-menu="showContextMenu"
            :get-method-class="getMethodClass"
            :get-status-class="getStatusClass"
            :get-status-title="getStatusTitle"
            :get-status-text="getStatusText"
            :has-params="hasParams"
          />
        </div>
      </div>

      <div
        v-if="showDetails"
        class="h-1 flex-shrink-0 cursor-row-resize bg-base-300 transition-colors hover:bg-primary/50"
        @mousedown="startHorizontalResize"
      ></div>

      <div v-if="showDetails" ref="bottomPanel" class="flex min-h-0 flex-1 flex-col overflow-hidden bg-base-100">
        <ProxyHistoryDetailsPanel
          :selected-request="selectedRequest"
          :is-loading-selected-request="isSelectedRequestLoading"
          :left-panel-width="leftPanelWidth"
          :request-tab="requestTab"
          :response-tab="responseTab"
          :request-view-mode="requestViewMode"
          :response-view-mode="responseViewMode"
          :context-evidence-pane="selectedRequestEvidence?.pane || 'request'"
          :context-evidence-matched-locations="selectedRequestEvidence?.matchedLocations || []"
          :context-evidence-search-terms="selectedRequestEvidence?.searchTerms || []"
          :show-detail-context-menu="showDetailContextMenu"
          :start-vertical-resize="startVerticalResize"
          @update:request-tab="requestTab = $event"
          @update:response-tab="responseTab = $event"
          @update:request-view-mode="requestViewMode = $event"
          @update:response-view-mode="responseViewMode = $event"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import {
  computed,
  inject,
  nextTick,
  onActivated,
  onDeactivated,
  onMounted,
  onUnmounted,
  ref,
  watch,
} from 'vue'
import { useI18n } from 'vue-i18n'
import { dialog } from '@/composables/useDialog'
import { setLocalStorageItem } from '@/utils/browserStorage'
import type { HttpExchangeRequest } from './http/model'
import ProxyHistoryDetailsPanel from './ProxyHistoryDetailsPanel.vue'
import TrafficHistoryFilterDialog from './TrafficHistoryFilterDialog.vue'
import ProxyHistoryTopContent from './ProxyHistoryTopContent.vue'
import TrafficContextCandidateDialog from './TrafficContextCandidateDialog.vue'
import TrafficHistoryWorkbenchToolbar from './TrafficHistoryWorkbenchToolbar.vue'
import TrafficContextMenuSections from './TrafficContextMenuSections.vue'
import TrafficContextSubmenu from './TrafficContextSubmenu.vue'
import {
  buildDefaultProxyHistoryFilterConfig,
  buildProxyHistoryFilterCache,
  hasActiveProxyHistoryFilters,
  hasParams,
  mergeStoredProxyHistoryFilterConfig,
} from './proxyHistoryFilterSupport'
import { buildTrafficContextCandidateId } from './trafficContextCandidateSupport'
import {
  buildHttpExchangeRequestFromHistory,
} from './proxyHistoryHttpSupport'
import {
  getColumnValue,
  hasEditedRequest,
  hasEditedResponse,
  getMethodClass,
  getStatusClass,
  getStatusText,
  getStatusTitle,
} from './proxyHistoryFormattingSupport'
import {
  buildProxyHistoryVisibleItems,
  getProxyHistoryDefaultSortDirection,
  isProxyHistoryDefaultSort,
  loadProxyHistoryColumnsFromStorage,
  loadProxyHistorySortFromStorage,
  moveProxyHistoryColumn,
  type ProxyHistoryColumnDropPosition,
  PROXY_HISTORY_BUFFER_SIZE,
  PROXY_HISTORY_COLUMNS_STORAGE_KEY,
  PROXY_HISTORY_HEADER_HEIGHT,
  PROXY_HISTORY_ITEM_HEIGHT,
  PROXY_HISTORY_SORT_STORAGE_KEY,
  translateProxyHistoryColumns,
} from './proxyHistoryTableSupport'
import { classifyProxyHistoryRequestIdStep } from './proxyHistoryListStepSupport'
import { buildProxyHistorySelectionChangeKey } from './proxyHistorySelectionChangeSupport'
import type {
  Column,
  ProxyHistoryFilterConfig,
  ProxyHistoryProtocolFilter,
  ProxyHistoryRequestTab,
  ProxyHistoryResponseTab,
  ProxyHistorySortState,
  ProxyHistoryViewMode,
  ProxyHistoryWsTab,
  ProxyRequest,
  VirtualItem,
  WebSocketConnection,
  WebSocketMessage,
} from './proxyHistoryTypes'
import type { ProxyScopeRule } from './proxyConfigurationTypes'
import type { ReferencedTraffic } from '@/types/agentReferences'
import { useProxyHistoryActions } from './useProxyHistoryActions'
import { useProxyHistoryData } from './useProxyHistoryData'
import { useProxyHistoryDerivedList } from './useProxyHistoryDerivedList'
import {
  canCompareRequestVersions,
  canCompareResponseVersions,
} from './trafficHistoryComparerSupport'
import { buildTrafficRequestActionMenuItems } from './trafficRequestActionMenuSupport'
import { buildTrafficRequestContextMenuSections } from './trafficRequestContextMenuSupport'
import { buildTrafficRequestSendMenuItems } from './trafficSendMenuSupport'
import { useTrafficSendTargets } from './trafficSendTargets'
import { useTrafficContextCandidatePreferences } from './useTrafficContextCandidatePreferences'
import { buildTrafficContextSubmenu } from './trafficContextSubmenuSupport'
import TrafficScopeRuleDialog from './TrafficScopeRuleDialog.vue'
import { useProxyHistoryScopeRuleActions } from './useProxyHistoryScopeRuleActions'
import type {
  RecommendTrafficContextDictionaryCandidatesResponse,
  TrafficContextCandidateEvidenceSelection,
  TrafficContextDictionaryCandidate,
  TrafficContextExtractionPreviewResponse,
} from './trafficContextCandidateTypes'
import type {
  TrafficComparerDraftRequestInput,
  TrafficComparePayload,
} from './transfers'
import {
  getTrafficContextExtractionSettings,
  mergeCandidatesIntoTrafficContextExtractionSettings,
  previewTrafficContextExtractionChanges,
  recommendTrafficContextDictionaryCandidates,
  setTrafficContextExtractionSettings,
} from '@/services/trafficContextCandidates'

type VisibleRow = VirtualItem & {
  cellValues: Record<string, string>
}

const emit = defineEmits<{
  (e: 'createDraft', request: HttpExchangeRequest): void
  (e: 'createAttackWorkspace', request: HttpExchangeRequest): void
  (e: 'openDraftCompare', payload: TrafficComparerDraftRequestInput): void
  (e: 'openCompare', payload: TrafficComparePayload): void
  (e: 'sendToAssistant', requests: ReferencedTraffic[]): void
  (e: 'addFilterRule', rule: { matchType: string; condition: string; relationship?: string }): void
  (e: 'addToBasket', payload: { request: HttpExchangeRequest; requestId?: number; title: string; host: string }): void
  (e: 'selectionChange', request: ProxyRequest | null): void
}>()

const props = withDefaults(defineProps<{
  basketCount: number
  showDetails?: boolean
}>(), {
  showDetails: true,
})

const { t } = useI18n()
const { enabledTargets } = useTrafficSendTargets()
const { preferredPreviewFocus } = useTrafficContextCandidatePreferences()

const refreshTrigger = inject<any>('refreshTrigger', ref(0))

const contextMenuRef = ref<HTMLElement | null>(null)
const mainContainer = ref<HTMLElement | null>(null)
const topPanel = ref<HTMLElement | null>(null)
const bottomPanel = ref<HTMLElement | null>(null)
const scrollContainer = ref<HTMLElement | null>(null)
const certErrorDialog = ref<HTMLDialogElement | null>(null)

const selectedRequests = ref<Set<number>>(new Set())
const isMultiSelectMode = ref(false)
const contextMenu = ref({
  visible: false,
  x: 0,
  y: 0,
  request: null as ProxyRequest | null,
})
const detailContextMenu = ref({
  visible: false,
  x: 0,
  y: 0,
  pane: 'request' as 'request' | 'response',
})

const requests = ref<ProxyRequest[]>([])
const selectedRequest = ref<ProxyRequest | null>(null)
const selectedRequestEvidence = ref<TrafficContextCandidateEvidenceSelection | null>(null)
const isSelectedRequestLoading = ref(false)
const protocolFilter = ref<ProxyHistoryProtocolFilter>('http')

const wsConnections = ref<WebSocketConnection[]>([])
const wsMessages = ref<WebSocketMessage[]>([])
const expandedWsConnections = ref<Set<string>>(new Set())
const activeWsTabs = ref<Map<string, ProxyHistoryWsTab>>(new Map())
const isLoadingWs = ref(false)
const wsMessagesCache = ref<Map<string, WebSocketMessage[]>>(new Map())

const isLoading = ref(false)
const hasMore = ref(true)
const isLoadingMore = ref(false)
const requestIdSet = ref<Set<number>>(new Set())
const stats = ref({
  total: 0,
  http: 0,
  https: 0,
  avgResponseTime: 0,
})

const STORAGE_KEY_TOP_HEIGHT = 'trafficHistoryWorkbench.topPanelHeight'
const STORAGE_KEY_LEFT_WIDTH = 'trafficHistoryWorkbench.leftPanelWidth'
const STORAGE_KEY_FILTERS = 'trafficHistoryWorkbench.filterConfig'
const topPanelHeight = ref(260)
const leftPanelWidth = ref(Number(localStorage.getItem(STORAGE_KEY_LEFT_WIDTH) || '600'))
const isResizingHorizontal = ref(false)
const isResizingVertical = ref(false)
const resizeStartY = ref(0)
const resizeStartX = ref(0)
const resizeStartTopHeight = ref(0)
const resizeStartLeftWidth = ref(0)

const initialLoadLimit = 100
const loadMoreSize = 50
const maxRequestsInMemory = 1000
const batchUpdateThreshold = 10
const itemHeight = PROXY_HISTORY_ITEM_HEIGHT
const headerHeight = PROXY_HISTORY_HEADER_HEIGHT
const bufferSize = PROXY_HISTORY_BUFFER_SIZE
const PREFETCH_VIEWPORT_MULTIPLIER = 2
const scrollTop = ref(0)
const containerHeight = ref(600)
const requestTab = ref<ProxyHistoryRequestTab>('pretty')
const responseTab = ref<ProxyHistoryResponseTab>('pretty')
const requestViewMode = ref<ProxyHistoryViewMode>('edited')
const responseViewMode = ref<ProxyHistoryViewMode>('edited')
const certErrorInfo = ref<{ host: string; url: string; error?: string } | null>(null)
const showHistoryFilterDialog = ref(false)
const showContextCandidateDialog = ref(false)
const contextCandidateLoading = ref(false)
const contextCandidateApplying = ref(false)
const contextCandidatePreviewLoading = ref(false)
const contextCandidateResult = ref<RecommendTrafficContextDictionaryCandidatesResponse | null>(null)
const contextCandidatePreviewResult = ref<TrafficContextExtractionPreviewResponse | null>(null)
const contextCandidateRequestIds = ref<number[]>([])
const selectedCandidateIds = ref<string[]>([])
const scopeIncludeRules = ref<ProxyScopeRule[]>([])
const scopeExcludeRules = ref<ProxyScopeRule[]>([])

let resizeObserver: ResizeObserver | null = null
let mainContainerResizeObserver: ResizeObserver | null = null
let scrollFrameId: number | null = null
let pendingScrollTop = 0
let prefetchTimer: number | null = null
let isPrefetching = false
let lastEmittedSelectionChangeKey = ''
const AUTO_FOLLOW_TOP_THRESHOLD = itemHeight
const savedScrollTop = ref(0)

const defaultFilterConfig = buildDefaultProxyHistoryFilterConfig
const appliedFilterConfig = ref(loadStoredHistoryFilterConfig())
const filterCache = computed(() => buildProxyHistoryFilterCache(appliedFilterConfig.value, {
  includeRules: scopeIncludeRules.value,
  excludeRules: scopeExcludeRules.value,
}))
const hasActiveFilters = computed(() => hasActiveProxyHistoryFilters(appliedFilterConfig.value))
const shouldBypassFrontendFilters = computed(() => !hasActiveFilters.value)

const columns = ref<Column[]>(loadProxyHistoryColumnsFromStorage())
const sortState = ref<ProxyHistorySortState>(loadProxyHistorySortFromStorage())
const translatedColumns = computed(() => translateProxyHistoryColumns(columns.value, t))
const visibleColumns = computed(() => translatedColumns.value.filter((col) => col.visible))
const historyTopPanelStyle = computed(() =>
  props.showDetails ? { height: `${topPanelHeight.value}px` } : { height: '100%' },
)
const {
  filteredRequests,
  sortedRequests,
} = useProxyHistoryDerivedList({
  requests,
  shouldBypassFrontendFilters,
  effectiveFilterConfig: computed(() => appliedFilterConfig.value),
  filterCache,
  sortState,
})
const totalHeight = computed(() => sortedRequests.value.length * itemHeight + headerHeight)
const navigableRequests = computed(() =>
  sortedRequests.value.filter(request => request.status_code !== 0),
)
const navigableRequestIndexById = computed(() => {
  const indexById = new Map<number, number>()
  navigableRequests.value.forEach((request, index) => {
    indexById.set(request.id, index)
  })
  return indexById
})
const selectedContextCandidates = computed<TrafficContextDictionaryCandidate[]>(() => {
  if (!contextCandidateResult.value) {
    return []
  }

  const selectedIdSet = new Set(selectedCandidateIds.value)
  return contextCandidateResult.value.candidates.filter(candidate =>
    selectedIdSet.has(buildTrafficContextCandidateId(candidate)),
  )
})
const visibleItems = computed((): VirtualItem[] =>
  buildProxyHistoryVisibleItems(sortedRequests.value, scrollTop.value, containerHeight.value),
)
const sortedRequestIds = computed(() => sortedRequests.value.map((request) => request.id))
const visibleRows = computed<VisibleRow[]>(() =>
  visibleItems.value.map((item) => {
    const cellValues: Record<string, string> = {}
    visibleColumns.value.forEach((column) => {
      if (['method', 'status', 'params', 'tls'].includes(column.id)) {
        return
      }
      cellValues[column.id] = getColumnValue(item.data, column.id)
    })
    return {
      ...item,
      cellValues,
    }
  }),
)

const {
  cleanupDataRuntime,
  fetchRequestPreview,
  fetchRequestDetails,
  formatWsTime,
  getWsActiveTab,
  getWsMessagesForConnection,
  loadMoreRequests,
  loadWsConnections,
  refreshRequests,
  setWsActiveTab,
  setupEventListeners,
  toggleWsConnection,
  truncateWsContent,
  updateStats,
} = useProxyHistoryData({
  requests,
  isLoading,
  hasMore,
  isLoadingMore,
  requestIdSet,
  stats,
  wsConnections,
  wsMessages,
  expandedWsConnections,
  activeWsTabs,
  isLoadingWs,
  wsMessagesCache,
  initialLoadLimit,
  loadMoreSize,
  maxRequestsInMemory,
  batchUpdateThreshold,
  t,
})

const {
  addFilterToDomain,
  addFilterToExtension,
  addFilterToMethod,
  addFilterToUrl,
  cleanupActionRuntime,
  clearHistory,
  clearHistoryFromMenu,
  clearSelection,
  compareRequestVersions,
  compareResponseVersions,
  copyAsCurl,
  copyRequest,
  copyUrl,
  detailCompareRequestVersions,
  detailCompareResponseVersions,
  detailCopyAsCurl,
  detailCopyRequest,
  detailCopyUrl,
  detailOpenInBrowser,
  detailSendRequestToAssistant,
  detailSendToComparer,
  detailSendToIntruder,
  detailSendToRepeater,
  exportAsHAR,
  exportSelectedToFile,
  hideContextMenu,
  hideDetailContextMenu,
  isRequestSelected,
  openInBrowser,
  selectAllVisible,
  selectRequest,
  sendSelectedRequestVersionsToComparer,
  sendSelectedResponseVersionsToComparer,
  sendSelectedToComparer,
  sendSelectedToDraft,
  sendSelectedToIntruder,
  sendSelectedToAssistant,
  sendRequestToAssistantFromMenu,
  openDraftCompare,
  createAttackWorkspace,
  createDraft,
  showContextMenu,
  showDetailContextMenu,
  toggleMultiSelectMode,
  toggleSelectRequest,
} = useProxyHistoryActions({
  contextMenu,
  detailContextMenu,
  selectedRequest,
  selectedRequests,
  isMultiSelectMode,
  filteredRequests,
  orderedRequests: sortedRequests,
  requests,
  stats,
  requestTab,
  responseTab,
  requestViewMode,
  responseViewMode,
  topPanelHeight,
  mainContainer,
  keepDetailsOpenOnRepeatSelect: true,
  hideContextMenu: () => hideContextMenu(),
  hideDetailContextMenu: () => hideDetailContextMenu(),
  emitCreateDraft: request => emit('createDraft', request),
  emitCreateAttackWorkspace: request => emit('createAttackWorkspace', request),
  emitOpenDraftCompare: payload => emit('openDraftCompare', payload),
  emitOpenCompare: payload => emit('openCompare', payload),
  emitSendToAssistant: requestsToSend => emit('sendToAssistant', requestsToSend),
  emitAddFilterRule: rule => emit('addFilterRule', rule),
  fetchRequestDetails,
  updateStats,
  t,
})

const exportAsHar = () => exportAsHAR()
const {
  historyScopeSubmenu,
  saveScopeRuleFromDialog,
  scopeRuleDialogRef,
} = useProxyHistoryScopeRuleActions({
  contextMenu,
  hideContextMenu,
  scopeIncludeRules,
  scopeExcludeRules,
  t,
})

const canCompareRequestFromContext = computed(() => canCompareRequestVersions(contextMenu.value.request))
const canCompareResponseFromContext = computed(() => canCompareResponseVersions(contextMenu.value.request))
const canCompareRequestFromDetail = computed(() => canCompareRequestVersions(selectedRequest.value))
const canCompareResponseFromDetail = computed(() => canCompareResponseVersions(selectedRequest.value))
const contextRequestSendMenuItems = computed(() =>
  buildTrafficRequestSendMenuItems({
    enabledTargets: enabledTargets.value,
    supportedTargets: ['draft', 'compare', 'attackWorkspace'],
    actions: {
      draft: createDraft,
      compare: openDraftCompare,
      attackWorkspace: createAttackWorkspace,
    },
  }),
)
const detailRequestSendMenuItems = computed(() =>
  buildTrafficRequestSendMenuItems({
    enabledTargets: enabledTargets.value,
    supportedTargets: ['draft', 'compare', 'attackWorkspace'],
    actions: {
      draft: detailSendToRepeater,
      compare: detailSendToComparer,
      attackWorkspace: detailSendToIntruder,
    },
  }),
)
const contextRequestActionMenuItems = computed(() =>
  buildTrafficRequestActionMenuItems({
    supportedActions: ['copyUrl', 'copyRequest', 'copyAsCurl', 'openInBrowser'],
    actions: {
      copyUrl,
      copyRequest,
      copyAsCurl,
      openInBrowser,
    },
  }),
)
const detailRequestActionMenuItems = computed(() =>
  buildTrafficRequestActionMenuItems({
    supportedActions: ['copyUrl', 'copyRequest', 'copyAsCurl', 'openInBrowser'],
    actions: {
      copyUrl: detailCopyUrl,
      copyRequest: detailCopyRequest,
      copyAsCurl: detailCopyAsCurl,
      openInBrowser: detailOpenInBrowser,
    },
  }),
)
const historyContextMenuSections = computed(() =>
  buildTrafficRequestContextMenuSections({
    sendItems: contextRequestSendMenuItems.value,
    compareItems: [
      canCompareRequestFromContext.value
        ? {
            key: 'compareRequestVersions',
            iconClass: 'fas fa-not-equal text-accent',
            labelKey: 'openCompare',
            onClick: compareRequestVersions,
          }
        : null,
      canCompareResponseFromContext.value
        ? {
            key: 'compareResponseVersions',
            iconClass: 'fas fa-not-equal text-accent',
            labelKey: 'openCompare',
            onClick: compareResponseVersions,
          }
        : null,
    ],
    requestItems: contextRequestActionMenuItems.value,
    assistantItems: [
      {
        key: 'sendToAssistant',
        iconClass: 'fas fa-upload text-accent',
        labelKey: 'sendToAssistant',
        onClick: sendRequestToAssistantFromMenu,
      },
    ],
  }),
)
const historyDetailContextMenuSections = computed(() =>
  buildTrafficRequestContextMenuSections({
    sendItems: detailContextMenu.value.pane === 'request'
      ? detailRequestSendMenuItems.value
      : detailRequestSendMenuItems.value.filter((item) => item.key === 'compare'),
    compareItems: [
      detailContextMenu.value.pane === 'request' && canCompareRequestFromDetail.value
        ? {
            key: 'detailCompareRequestVersions',
            iconClass: 'fas fa-not-equal text-accent',
            labelKey: 'openCompare',
            onClick: detailCompareRequestVersions,
          }
        : null,
      detailContextMenu.value.pane === 'response' && canCompareResponseFromDetail.value
        ? {
            key: 'detailCompareResponseVersions',
            iconClass: 'fas fa-not-equal text-accent',
            labelKey: 'openCompare',
            onClick: detailCompareResponseVersions,
          }
        : null,
    ],
    requestItems: detailContextMenu.value.pane === 'request' ? detailRequestActionMenuItems.value : [],
    assistantItems: detailContextMenu.value.pane === 'request'
      ? [
          {
            key: 'detailSendToAssistant',
            iconClass: 'fas fa-upload text-accent',
            labelKey: 'sendToAssistant',
            onClick: detailSendRequestToAssistant,
          },
        ]
      : [],
  }),
)
const historyFilterSubmenu = computed(() =>
  buildTrafficContextSubmenu({
    key: 'history-filter',
    triggerLabelKey: 'addToFilter',
    triggerIconClass: 'fas fa-filter text-primary',
    items: [
      {
        key: 'filterByDomain',
        iconClass: 'fas fa-globe text-xs',
        labelKey: 'filterByDomain',
        onClick: addFilterToDomain,
      },
      {
        key: 'filterByUrl',
        iconClass: 'fas fa-link text-xs',
        labelKey: 'filterByUrl',
        onClick: addFilterToUrl,
      },
      {
        key: 'filterByMethod',
        iconClass: 'fas fa-code text-xs',
        labelKey: 'filterByMethod',
        onClick: addFilterToMethod,
      },
      {
        key: 'filterByExtension',
        iconClass: 'fas fa-file text-xs',
        labelKey: 'filterByExtension',
        onClick: addFilterToExtension,
      },
    ],
  }),
)

function toggleSort(columnId: string) {
  if (sortState.value.columnId === columnId) {
    sortState.value = {
      columnId,
      direction: sortState.value.direction === 'asc' ? 'desc' : 'asc',
    }
  } else {
    sortState.value = {
      columnId,
      direction: getProxyHistoryDefaultSortDirection(columnId),
    }
  }

  setLocalStorageItem(PROXY_HISTORY_SORT_STORAGE_KEY, JSON.stringify(sortState.value))
  scrollTop.value = 0
  if (scrollContainer.value) {
    scrollContainer.value.scrollTop = 0
  }
}

const resizingColumn = ref<string | null>(null)
const columnResizeStartX = ref(0)
const resizeStartWidth = ref(0)
const draggedColumnId = ref<string | null>(null)
const columnDropTargetId = ref<string | null>(null)
const columnDropPosition = ref<ProxyHistoryColumnDropPosition | null>(null)
const columnDragPointerId = ref<number | null>(null)

function persistProxyHistoryColumns() {
  setLocalStorageItem(PROXY_HISTORY_COLUMNS_STORAGE_KEY, JSON.stringify(columns.value))
}

function startResize(columnId: string, event: MouseEvent) {
  event.preventDefault()
  resizingColumn.value = columnId
  columnResizeStartX.value = event.clientX
  const column = columns.value.find(col => col.id === columnId)
  if (column) {
    resizeStartWidth.value = column.width
  }

  document.addEventListener('mousemove', handleResize)
  document.addEventListener('mouseup', stopResize)
}

function handleResize(event: MouseEvent) {
  if (!resizingColumn.value) {
    return
  }

  const column = columns.value.find(col => col.id === resizingColumn.value)
  if (!column) {
    return
  }

  const diff = event.clientX - columnResizeStartX.value
  column.width = Math.max(column.minWidth, resizeStartWidth.value + diff)
}

function stopResize() {
  resizingColumn.value = null
  document.removeEventListener('mousemove', handleResize)
  document.removeEventListener('mouseup', stopResize)
  persistProxyHistoryColumns()
}

function getColumnElementFromPoint(clientX: number, clientY: number) {
  const elements = document.elementsFromPoint(clientX, clientY)
  for (const element of elements) {
    if (!(element instanceof HTMLElement)) {
      continue
    }

    const columnElement = element.closest<HTMLElement>('[data-proxy-history-column-id]')
    if (columnElement?.dataset.proxyHistoryColumnId) {
      return columnElement
    }
  }

  return null
}

function updateColumnDropTarget(clientX: number, clientY: number) {
  if (!draggedColumnId.value) {
    return
  }

  const columnElement = getColumnElementFromPoint(clientX, clientY)
  const targetColumnId = columnElement?.dataset.proxyHistoryColumnId || null

  if (!columnElement || !targetColumnId || targetColumnId === draggedColumnId.value) {
    columnDropTargetId.value = null
    columnDropPosition.value = null
    return
  }

  const rect = columnElement.getBoundingClientRect()
  columnDropTargetId.value = targetColumnId
  columnDropPosition.value = clientX < rect.left + rect.width / 2 ? 'before' : 'after'
}

function startColumnDrag(columnId: string, event: PointerEvent) {
  if (resizingColumn.value) {
    event.preventDefault()
    return
  }

  event.preventDefault()
  draggedColumnId.value = columnId
  columnDropTargetId.value = null
  columnDropPosition.value = null
  columnDragPointerId.value = event.pointerId
  updateColumnDropTarget(event.clientX, event.clientY)

  document.addEventListener('pointermove', handleColumnPointerMove)
  document.addEventListener('pointerup', stopColumnDrag)
  document.addEventListener('pointercancel', cancelColumnDrag)
  document.body.style.cursor = 'grabbing'
  document.body.style.userSelect = 'none'
}

function handleColumnPointerMove(event: PointerEvent) {
  if (columnDragPointerId.value !== event.pointerId) {
    return
  }

  event.preventDefault()
  updateColumnDropTarget(event.clientX, event.clientY)
}

function stopColumnDrag(event: PointerEvent) {
  if (columnDragPointerId.value !== event.pointerId) {
    return
  }

  const sourceColumnId = draggedColumnId.value
  const targetColumnId = columnDropTargetId.value
  const position = columnDropPosition.value

  if (sourceColumnId && targetColumnId && position) {
    columns.value = moveProxyHistoryColumn(columns.value, sourceColumnId, targetColumnId, position)
    persistProxyHistoryColumns()
  }

  cancelColumnDrag()
}

function cancelColumnDrag() {
  draggedColumnId.value = null
  columnDropTargetId.value = null
  columnDropPosition.value = null
  columnDragPointerId.value = null
  document.removeEventListener('pointermove', handleColumnPointerMove)
  document.removeEventListener('pointerup', stopColumnDrag)
  document.removeEventListener('pointercancel', cancelColumnDrag)
  document.body.style.cursor = ''
  document.body.style.userSelect = ''
}

function startHorizontalResize(event: MouseEvent) {
  event.preventDefault()
  isResizingHorizontal.value = true
  resizeStartY.value = event.clientY
  resizeStartTopHeight.value = topPanelHeight.value

  document.addEventListener('mousemove', handleHorizontalResize)
  document.addEventListener('mouseup', stopHorizontalResize)
  document.body.style.cursor = 'row-resize'
  document.body.style.userSelect = 'none'
}

function handleHorizontalResize(event: MouseEvent) {
  if (!isResizingHorizontal.value) {
    return
  }

  const container = mainContainer.value?.clientHeight || 600
  const diff = event.clientY - resizeStartY.value
  topPanelHeight.value = Math.max(160, Math.min(resizeStartTopHeight.value + diff, container - 220))
}

function stopHorizontalResize() {
  isResizingHorizontal.value = false
  document.removeEventListener('mousemove', handleHorizontalResize)
  document.removeEventListener('mouseup', stopHorizontalResize)
  document.body.style.cursor = ''
  document.body.style.userSelect = ''
  setLocalStorageItem(STORAGE_KEY_TOP_HEIGHT, String(topPanelHeight.value))
}

function startVerticalResize(event: MouseEvent) {
  event.preventDefault()
  isResizingVertical.value = true
  resizeStartX.value = event.clientX
  resizeStartLeftWidth.value = leftPanelWidth.value

  document.addEventListener('mousemove', handleVerticalResize)
  document.addEventListener('mouseup', stopVerticalResize)
  document.body.style.cursor = 'col-resize'
  document.body.style.userSelect = 'none'
}

function handleVerticalResize(event: MouseEvent) {
  if (!isResizingVertical.value) {
    return
  }

  const container = bottomPanel.value?.clientWidth || 1200
  const diff = event.clientX - resizeStartX.value
  leftPanelWidth.value = Math.max(320, Math.min(resizeStartLeftWidth.value + diff, container - 320))
}

function stopVerticalResize() {
  isResizingVertical.value = false
  document.removeEventListener('mousemove', handleVerticalResize)
  document.removeEventListener('mouseup', stopVerticalResize)
  document.body.style.cursor = ''
  document.body.style.userSelect = ''
  setLocalStorageItem(STORAGE_KEY_LEFT_WIDTH, String(leftPanelWidth.value))
}

function handleScroll(event: Event) {
  const target = event.target as HTMLElement
  pendingScrollTop = target.scrollTop
  if (scrollFrameId !== null) {
    return
  }

  scrollFrameId = window.requestAnimationFrame(() => {
    scrollTop.value = pendingScrollTop
    scrollFrameId = null
    schedulePrefetchCheck()
  })
}

function schedulePrefetchCheck() {
  if (prefetchTimer !== null) {
    clearTimeout(prefetchTimer)
  }

  prefetchTimer = window.setTimeout(() => {
    prefetchTimer = null
    void ensurePrefetchBuffer()
  }, 60)
}

async function ensurePrefetchBuffer() {
  if (
    isPrefetching
    || protocolFilter.value === 'websocket'
    || isLoading.value
    || isLoadingMore.value
    || !hasMore.value
  ) {
    return
  }

  const visibleCount = Math.max(1, Math.ceil(containerHeight.value / itemHeight))
  const desiredRemainingRows = Math.max(loadMoreSize, visibleCount * PREFETCH_VIEWPORT_MULTIPLIER)
  const currentLastVisibleIndex = Math.ceil((scrollTop.value + containerHeight.value) / itemHeight)
  const remainingRows = sortedRequests.value.length - currentLastVisibleIndex

  if (remainingRows > desiredRemainingRows) {
    return
  }

  isPrefetching = true
  try {
    for (let attempts = 0; attempts < 3; attempts += 1) {
      if (!hasMore.value || isLoadingMore.value) {
        break
      }

      const loadedCount = await loadMoreRequests()
      if (loadedCount <= 0) {
        break
      }

      await nextTick()

      const nextLastVisibleIndex = Math.ceil((scrollTop.value + containerHeight.value) / itemHeight)
      const nextRemainingRows = sortedRequests.value.length - nextLastVisibleIndex
      if (nextRemainingRows > desiredRemainingRows) {
        break
      }
    }
  } finally {
    isPrefetching = false
  }
}

function preserveScrollAnchorOnPrependedRows(previousIds: number[], nextIds: number[]) {
  if (!scrollContainer.value || protocolFilter.value === 'websocket') {
    return
  }

  if (!isProxyHistoryDefaultSort(sortState.value)) {
    return
  }

  const currentScrollTop = scrollContainer.value.scrollTop
  if (currentScrollTop <= AUTO_FOLLOW_TOP_THRESHOLD) {
    return
  }

  const step = classifyProxyHistoryRequestIdStep(previousIds, nextIds)
  if (step.type !== 'prepend' || step.addedFrontCount <= 0) {
    return
  }

  const nextScrollTop = currentScrollTop + step.addedFrontCount * itemHeight
  scrollContainer.value.scrollTop = nextScrollTop
  scrollTop.value = nextScrollTop
}

function updateContainerHeight() {
  if (!scrollContainer.value) {
    return
  }

  containerHeight.value = scrollContainer.value.clientHeight
  schedulePrefetchCheck()
}

function syncVirtualViewport() {
  if (!scrollContainer.value) {
    return
  }

  containerHeight.value = scrollContainer.value.clientHeight
  scrollTop.value = scrollContainer.value.scrollTop
  schedulePrefetchCheck()
}

async function refreshVirtualLayout() {
  await nextTick()
  requestAnimationFrame(() => {
    syncVirtualViewport()
  })
}

function captureScrollPosition() {
  if (scrollContainer.value) {
    savedScrollTop.value = scrollContainer.value.scrollTop
  }
}

async function ensureRowsForScrollPosition(targetScrollTop: number) {
  if (protocolFilter.value === 'websocket' || targetScrollTop <= 0) {
    return
  }

  const viewportHeight = Math.max(containerHeight.value, scrollContainer.value?.clientHeight || 0, 1)
  const requiredRowCount = Math.ceil((targetScrollTop + viewportHeight) / itemHeight) + bufferSize
  let attempts = 0

  while (sortedRequests.value.length < requiredRowCount && hasMore.value && attempts < 20) {
    const loadedCount = await loadMoreRequests()
    if (loadedCount <= 0) {
      break
    }
    attempts += 1
    await nextTick()
  }
}

async function restoreSavedScrollPosition() {
  if (!scrollContainer.value) {
    return
  }

  await nextTick()
  syncVirtualViewport()
  await ensureRowsForScrollPosition(savedScrollTop.value)

  if (!scrollContainer.value) {
    return
  }

  const maxScrollTop = Math.max(0, scrollContainer.value.scrollHeight - scrollContainer.value.clientHeight)
  const nextScrollTop = Math.min(savedScrollTop.value, maxScrollTop)
  scrollContainer.value.scrollTop = nextScrollTop
  syncVirtualViewport()
}

function setupResizeObserver() {
  if (!scrollContainer.value) {
    return
  }

  resizeObserver = new ResizeObserver(() => {
    updateContainerHeight()
  })
  resizeObserver.observe(scrollContainer.value)
}

function initPanelHeights() {
  const containerHeight = mainContainer.value?.clientHeight || 600
  const savedTopHeight = localStorage.getItem(STORAGE_KEY_TOP_HEIGHT)
  topPanelHeight.value = savedTopHeight
    ? Math.min(Number(savedTopHeight), containerHeight - 220)
    : Math.floor(containerHeight * 0.36)

  const containerWidth = bottomPanel.value?.clientWidth || 1200
  if (!localStorage.getItem(STORAGE_KEY_LEFT_WIDTH)) {
    leftPanelWidth.value = Math.floor(containerWidth * 0.48)
  }
}

function setupMainContainerResizeObserver() {
  if (!mainContainer.value) {
    return
  }

  mainContainerResizeObserver = new ResizeObserver(() => {
    if (selectedRequest.value) {
      const container = mainContainer.value?.clientHeight || 600
      if (topPanelHeight.value > container - 220) {
        topPanelHeight.value = Math.floor(container * 0.36)
      }
    }
    updateContainerHeight()
  })
  mainContainerResizeObserver.observe(mainContainer.value)
}

function showCertificateError(request: ProxyRequest) {
  certErrorInfo.value = {
    host: request.host,
    url: request.url,
    error: request.status_code === 0 ? 'TLS Handshake Failed' : 'Certificate validation error',
  }
  certErrorDialog.value?.showModal()
}

function closeCertErrorDialog() {
  certErrorDialog.value?.close()
  certErrorInfo.value = null
}

function closeContextCandidateDialog() {
  showContextCandidateDialog.value = false
  contextCandidatePreviewResult.value = null
}

async function openHistoryFilterDialog() {
  await loadScopeRules()
  showHistoryFilterDialog.value = true
}

function applyHistoryFilters(config: ProxyHistoryFilterConfig) {
  appliedFilterConfig.value = cloneHistoryFilterConfig(config)
  persistHistoryFilterConfig()
  showHistoryFilterDialog.value = false
}

async function loadScopeRules() {
  try {
    const response = await invoke<{ success?: boolean; data?: { scope_include_rules?: ProxyScopeRule[]; scope_exclude_rules?: ProxyScopeRule[] } }>('get_proxy_config')
    if (!response?.success || !response.data) {
      return
    }

    scopeIncludeRules.value = Array.isArray(response.data.scope_include_rules)
      ? response.data.scope_include_rules
      : []
    scopeExcludeRules.value = Array.isArray(response.data.scope_exclude_rules)
      ? response.data.scope_exclude_rules
      : []
  } catch (error) {
    console.error('Failed to load scope rules for history filters:', error)
  }
}

async function generateCandidatesFromFiltered() {
  const requestIds = filteredRequests.value
    .map(request => request.id)
    .filter(id => Number.isFinite(id))

  await openContextCandidateRecommendationDialog(requestIds)
}

async function generateCandidatesFromSelection() {
  const requestIds = Array.from(selectedRequests.value)
    .filter(id => Number.isFinite(id))

  await openContextCandidateRecommendationDialog(requestIds)
}

async function openContextCandidateRecommendationDialog(requestIds: number[]) {
  const normalizedRequestIds = [...new Set(requestIds)].slice(0, 500)
  if (normalizedRequestIds.length === 0) {
    dialog.toast.warning('当前没有可用于生成候选的历史记录')
    return
  }

  showContextCandidateDialog.value = true
  contextCandidateLoading.value = true
  contextCandidateApplying.value = false
  contextCandidatePreviewLoading.value = false
  contextCandidateResult.value = null
  contextCandidatePreviewResult.value = null
  contextCandidateRequestIds.value = normalizedRequestIds
  selectedCandidateIds.value = []

  try {
    const result = await recommendTrafficContextDictionaryCandidates({
      requestIds: normalizedRequestIds,
      maxCandidatesPerCategory: 12,
    })
    contextCandidateResult.value = result
    selectedCandidateIds.value = result.candidates
      .filter(candidate => candidate.confidence === 'high' && !candidate.alreadyCoveredBy)
      .map(buildTrafficContextCandidateId)
  } catch (error) {
    console.error('Failed to recommend traffic context candidates from history workbench:', error)
    dialog.toast.error(`生成词典候选失败: ${String(error)}`)
    showContextCandidateDialog.value = false
  } finally {
    contextCandidateLoading.value = false
  }
}

function updateSelectedCandidateIds(nextValue: string[]) {
  selectedCandidateIds.value = nextValue
  contextCandidatePreviewResult.value = null
}

async function applySelectedContextCandidates() {
  if (selectedContextCandidates.value.length === 0) {
    return
  }

  contextCandidateApplying.value = true
  try {
    const latestSettings = await getTrafficContextExtractionSettings()
    const nextSettings = mergeCandidatesIntoTrafficContextExtractionSettings(
      latestSettings,
      selectedContextCandidates.value,
    )
    await setTrafficContextExtractionSettings(nextSettings)
    dialog.toast.success(`已把 ${selectedContextCandidates.value.length} 项候选合并到上下文词典`)
    closeContextCandidateDialog()
    selectedCandidateIds.value = []
  } catch (error) {
    console.error('Failed to apply traffic context candidates from history workbench:', error)
    dialog.toast.error(`应用候选失败: ${String(error)}`)
  } finally {
    contextCandidateApplying.value = false
  }
}

async function previewSelectedContextCandidates() {
  if (selectedContextCandidates.value.length === 0 || contextCandidateRequestIds.value.length === 0) {
    return
  }

  contextCandidatePreviewLoading.value = true
  try {
    const currentSettings = await getTrafficContextExtractionSettings()
    const previewSettings = mergeCandidatesIntoTrafficContextExtractionSettings(
      currentSettings,
      selectedContextCandidates.value,
    )
    contextCandidatePreviewResult.value = await previewTrafficContextExtractionChanges({
      requestIds: contextCandidateRequestIds.value,
      currentSettings,
      previewSettings,
      sampleLimit: 6,
    })
  } catch (error) {
    console.error('Failed to preview traffic context candidates from history workbench:', error)
    dialog.toast.error(`预览命中变化失败: ${String(error)}`)
  } finally {
    contextCandidatePreviewLoading.value = false
  }
}

async function openContextCandidateEvidenceRequest(payload: TrafficContextCandidateEvidenceSelection) {
  await openRequestById(
    payload.requestId,
    payload.matchedLocations,
    payload.pane || 'request',
    payload.searchTerms || [],
  )
}

async function checkCAInstallation() {
  try {
    const response = await invoke<any>('export_root_ca')
    if (response.success && response.data) {
      dialog.toast.info(t('trafficAnalysis.history.certificateError.tips.installCA'))
      await invoke('show_in_folder', { path: response.data })
    }
  } catch (error: any) {
    dialog.toast.error(`Failed to check certificate: ${error}`)
  }
}

function persistHistoryFilterConfig() {
  setLocalStorageItem(STORAGE_KEY_FILTERS, JSON.stringify(appliedFilterConfig.value))
}

function loadStoredHistoryFilterConfig() {
  const raw = window.localStorage.getItem(STORAGE_KEY_FILTERS)
  if (!raw) {
    return defaultFilterConfig()
  }

  try {
    return mergeStoredProxyHistoryFilterConfig(JSON.parse(raw))
  } catch {
    return defaultFilterConfig()
  }
}

function cloneHistoryFilterConfig(config: ProxyHistoryFilterConfig): ProxyHistoryFilterConfig {
  return JSON.parse(JSON.stringify(config)) as ProxyHistoryFilterConfig
}

function isEditableKeyboardTarget(target: EventTarget | null) {
  if (!(target instanceof HTMLElement)) {
    return false
  }

  if (target.closest('input, textarea, select, [contenteditable="true"]')) {
    return true
  }

  return Boolean(target.closest('.editor-search-bar, .cm-textfield'))
}

function shouldHandleHistoryArrowNavigation(event: KeyboardEvent) {
  if (!mainContainer.value || mainContainer.value.offsetParent === null) {
    return false
  }

  if (protocolFilter.value === 'websocket' || sortedRequests.value.length === 0) {
    return false
  }

  if (contextMenu.value.visible || detailContextMenu.value.visible) {
    return false
  }

  if (event.metaKey || event.ctrlKey || event.altKey || event.shiftKey) {
    return false
  }

  if (event.key !== 'ArrowUp' && event.key !== 'ArrowDown') {
    return false
  }

  if (isEditableKeyboardTarget(event.target)) {
    return false
  }

  const activeElement = document.activeElement
  if (activeElement instanceof HTMLElement) {
    if (isEditableKeyboardTarget(activeElement)) {
      return false
    }

    if (activeElement !== document.body && !mainContainer.value.contains(activeElement)) {
      return false
    }
  }

  return true
}

function shouldHandleHistoryWorkbenchShortcut(event: KeyboardEvent) {
  if (!mainContainer.value || mainContainer.value.offsetParent === null) {
    return false
  }

  if (protocolFilter.value === 'websocket' || !selectedRequest.value) {
    return false
  }

  if (contextMenu.value.visible || detailContextMenu.value.visible) {
    return false
  }

  if (!(event.metaKey || event.ctrlKey) || event.altKey) {
    return false
  }

  if (isEditableKeyboardTarget(event.target)) {
    return false
  }

  const activeElement = document.activeElement
  if (activeElement instanceof HTMLElement) {
    if (isEditableKeyboardTarget(activeElement)) {
      return false
    }

    if (activeElement !== document.body && !mainContainer.value.contains(activeElement)) {
      return false
    }
  }

  return true
}

function shouldExitHistoryMultiSelect(event: KeyboardEvent) {
  if (
    event.key !== 'Escape'
    || !isMultiSelectMode.value
    || !mainContainer.value
    || mainContainer.value.offsetParent === null
  ) {
    return false
  }

  if (isEditableKeyboardTarget(event.target)) {
    return false
  }

  const activeElement = document.activeElement
  if (activeElement instanceof HTMLElement) {
    if (isEditableKeyboardTarget(activeElement)) {
      return false
    }

    if (activeElement !== document.body && !mainContainer.value.contains(activeElement)) {
      return false
    }
  }

  return true
}

function scrollRequestRowIntoView(requestId: number) {
  if (!scrollContainer.value) {
    return
  }

  const rowIndex = sortedRequests.value.findIndex(request => request.id === requestId)
  if (rowIndex < 0) {
    return
  }

  const rowTop = rowIndex * itemHeight + headerHeight
  const rowBottom = rowTop + itemHeight
  const currentScrollTop = scrollContainer.value.scrollTop
  const visibleTop = currentScrollTop + headerHeight
  const visibleBottom = currentScrollTop + scrollContainer.value.clientHeight
  let nextScrollTop: number | null = null

  if (rowTop < visibleTop) {
    nextScrollTop = Math.max(0, rowTop - headerHeight)
  } else if (rowBottom > visibleBottom) {
    nextScrollTop = Math.max(0, rowBottom - scrollContainer.value.clientHeight)
  }

  if (nextScrollTop === null) {
    return
  }

  scrollContainer.value.scrollTop = nextScrollTop
  scrollTop.value = nextScrollTop
  schedulePrefetchCheck()
}

function navigateHistorySelection(direction: -1 | 1) {
  const requests = navigableRequests.value
  if (requests.length === 0) {
    return false
  }

  const currentIndex = selectedRequest.value
    ? navigableRequestIndexById.value.get(selectedRequest.value.id) ?? -1
    : -1

  const nextIndex = currentIndex < 0
    ? (direction > 0 ? 0 : requests.length - 1)
    : Math.min(requests.length - 1, Math.max(0, currentIndex + direction))

  const nextRequest = requests[nextIndex]
  if (!nextRequest) {
    return false
  }

  if (selectedRequest.value?.id !== nextRequest.id) {
    selectRequest(nextRequest)
  }

  scrollRequestRowIntoView(nextRequest.id)
  return true
}

function handleKeydown(event: KeyboardEvent) {
  if (event.defaultPrevented || event.repeat) {
    return
  }

  if (shouldExitHistoryMultiSelect(event)) {
    event.preventDefault()
    event.stopPropagation()
    clearSelection()
    isMultiSelectMode.value = false
    return
  }

  if (shouldHandleHistoryArrowNavigation(event)) {
    event.preventDefault()
    event.stopPropagation()
    navigateHistorySelection(event.key === 'ArrowDown' ? 1 : -1)
    return
  }

  if (!shouldHandleHistoryWorkbenchShortcut(event)) {
    return
  }

  const normalizedKey = event.key.toLowerCase()
  if (normalizedKey === 'r') {
    event.preventDefault()
    event.stopPropagation()
    void detailSendToRepeater()
    return
  }

  if (normalizedKey === 'i') {
    event.preventDefault()
    event.stopPropagation()
    void detailSendToIntruder()
  }
}

async function openRequestById(
  requestId: number,
  matchedLocations: string[] = [],
  pane: 'request' | 'response' = 'request',
  searchTerms: string[] = [],
) {
  if (!Number.isFinite(requestId)) {
    return
  }

  protocolFilter.value = 'http'
  selectedRequestEvidence.value = {
    requestId,
    pane,
    matchedLocations: [...matchedLocations],
    searchTerms: [...searchTerms],
  }

  let nextRequest = requests.value.find(request => request.id === requestId) || null
  if (!nextRequest || nextRequest.has_full_details === false) {
    nextRequest = await fetchRequestPreview(requestId) || nextRequest
  }

  if (!nextRequest) {
    dialog.toast.warning(`未找到历史请求 #${requestId}`)
    return
  }

  selectRequest(nextRequest)
  await nextTick()
  scrollRequestRowIntoView(requestId)
}

async function addContextRequestToBasket() {
  hideContextMenu()
  const request = contextMenu.value.request
  if (!request) {
    return
  }

  const detailed = await fetchRequestDetails(request.id) || request
  emit('addToBasket', {
    request: buildHttpExchangeRequestFromHistory(detailed),
    requestId: detailed.id,
    title: detailed.url,
    host: detailed.host || '',
  })
}

async function addDetailRequestToBasket() {
  hideDetailContextMenu()
  if (!selectedRequest.value) {
    return
  }

  const detailed = await fetchRequestDetails(selectedRequest.value.id) || selectedRequest.value
  emit('addToBasket', {
    request: buildHttpExchangeRequestFromHistory(detailed),
    requestId: detailed.id,
    title: detailed.url,
    host: detailed.host || '',
  })
}

async function addSelectedToBasket() {
  const selectedIds = [...selectedRequests.value]
  if (selectedIds.length === 0) {
    return
  }

  const matchingRequests = requests.value.filter(request => selectedRequests.value.has(request.id))
  const detailedRequests = await Promise.all(
    matchingRequests.map(async request =>
      request.has_full_details === false
        ? (await fetchRequestDetails(request.id)) || request
        : request,
    ),
  )

  detailedRequests.forEach(request => {
    emit('addToBasket', {
      request: buildHttpExchangeRequestFromHistory(request),
      requestId: request.id,
      title: request.url,
      host: request.host || '',
    })
  })
  dialog.toast.success(`已加入 ${detailedRequests.length} 条请求到篮子`)
}

function removeMatchingRecords(rule: { matchType: string; condition: string; relationship: string }) {
  const beforeCount = requests.value.length
  requests.value = requests.value.filter((req) => {
    try {
      switch (rule.matchType) {
        case 'domain_name': {
          const domain = new URL(req.url).hostname
          return rule.relationship === 'matches'
            ? domain !== rule.condition
            : domain === rule.condition
        }
        case 'url':
          return rule.relationship === 'matches'
            ? req.url !== rule.condition
            : req.url === rule.condition
        case 'http_method': {
          const method = req.method.toLowerCase()
          const conditionMethod = rule.condition.toLowerCase()
          return rule.relationship === 'matches'
            ? method !== conditionMethod
            : method === conditionMethod
        }
        case 'file_extension': {
          const pathname = new URL(req.url).pathname
          const lastDot = pathname.lastIndexOf('.')
          if (lastDot <= 0) {
            return true
          }
          const extension = pathname.substring(lastDot + 1).split('?')[0]
          const regex = new RegExp(rule.condition)
          return rule.relationship === 'matches'
            ? !regex.test(extension)
            : regex.test(extension)
        }
        default:
          return true
      }
    } catch {
      return true
    }
  })

  const removedCount = beforeCount - requests.value.length
  if (removedCount > 0) {
    dialog.toast.info(`Removed ${removedCount} matching record(s) from history`)
    if (selectedRequest.value && !requests.value.find(r => r.id === selectedRequest.value?.id)) {
      selectedRequest.value = null
    }
  }
}

function shouldLoadPreviewDetails(request: ProxyRequest | null) {
  if (!request || request.has_full_details) {
    return false
  }

  if (request.request_body_loaded === false) {
    return true
  }

  if (hasEditedRequest(request) && request.edited_request_body_loaded === false) {
    return true
  }

  return false
}

onMounted(async () => {
  await loadScopeRules()
  await setupEventListeners()
  await refreshRequests()
  await nextTick()
  updateContainerHeight()
  setupResizeObserver()
  setupMainContainerResizeObserver()
  initPanelHeights()
  schedulePrefetchCheck()
  document.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  cleanupDataRuntime()
  cleanupActionRuntime()
  if (scrollFrameId !== null) {
    cancelAnimationFrame(scrollFrameId)
  }
  if (prefetchTimer !== null) {
    clearTimeout(prefetchTimer)
  }
  resizeObserver?.disconnect()
  mainContainerResizeObserver?.disconnect()
  document.removeEventListener('mousemove', handleResize)
  document.removeEventListener('mouseup', stopResize)
  document.removeEventListener('mousemove', handleHorizontalResize)
  document.removeEventListener('mouseup', stopHorizontalResize)
  document.removeEventListener('mousemove', handleVerticalResize)
  document.removeEventListener('mouseup', stopVerticalResize)
  document.removeEventListener('keydown', handleKeydown)
})

watch(
  () => buildProxyHistorySelectionChangeKey(selectedRequest.value),
  async (nextKey) => {
    if (nextKey === lastEmittedSelectionChangeKey) {
      return
    }
    lastEmittedSelectionChangeKey = nextKey
    emit('selectionChange', selectedRequest.value)
  },
)

watch(() => selectedRequest.value?.id ?? null, async () => {
  if (selectedRequestEvidence.value && selectedRequest.value?.id !== selectedRequestEvidence.value.requestId) {
    selectedRequestEvidence.value = null
  }
  await nextTick()
  updateContainerHeight()
})

watch(
  () => [sortedRequests.value.length, containerHeight.value, protocolFilter.value] as const,
  () => {
    schedulePrefetchCheck()
  },
)

watch(
  sortedRequestIds,
  (nextIds, previousIds = []) => {
    preserveScrollAnchorOnPrependedRows(previousIds, nextIds)
  },
  { flush: 'post' },
)

watch(
  () => [
    selectedRequest.value?.id,
    selectedRequest.value?.has_full_details,
    selectedRequest.value?.request_body_loaded,
    selectedRequest.value?.edited_request_body_loaded,
    hasEditedRequest(selectedRequest.value),
  ] as const,
  async ([requestId]) => {
    const request = selectedRequest.value
    if (!requestId || !shouldLoadPreviewDetails(request)) {
      isSelectedRequestLoading.value = false
      return
    }

    isSelectedRequestLoading.value = true
    try {
      const previewRequest = await fetchRequestPreview(requestId)
      if (previewRequest && selectedRequest.value?.id === requestId) {
        selectedRequest.value = previewRequest
      }
    } finally {
      if (selectedRequest.value?.id === requestId) {
        isSelectedRequestLoading.value = false
      }
    }
  },
)

watch(refreshTrigger, async () => {
  await refreshRequests()
  await restoreSavedScrollPosition()
})

watch(protocolFilter, async (newFilter) => {
  selectedRequest.value = null
  selectedRequestEvidence.value = null

  if (newFilter === 'websocket') {
    await loadWsConnections()
    return
  }

  await refreshRequests()
  await nextTick()
  schedulePrefetchCheck()
})

onActivated(() => {
  void refreshVirtualLayout()
})

onDeactivated(() => {
  captureScrollPosition()
})

defineExpose({
  captureScrollPosition,
  removeMatchingRecords,
  openRequestById,
  refreshVirtualLayout,
  restoreSavedScrollPosition,
})
</script>
