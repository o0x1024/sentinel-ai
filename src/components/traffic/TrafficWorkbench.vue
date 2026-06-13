<template>
  <div class="traffic-workbench flex h-[calc(100vh-var(--app-navbar-height,4rem))] flex-col px-1.5 py-1.5">
    <div ref="workspaceStageRef" class="relative min-h-0 flex-1">
      <div class="grid h-full min-h-0 gap-1.5" :style="workbenchGridStyle">
        <div ref="leftColumnRef" class="grid min-h-0 min-w-0 gap-1.5" :style="leftColumnStyle">
          <section class="workbench-solid-surface min-h-0 min-w-0 overflow-hidden rounded-[16px] border border-base-300/70 shadow-[0_14px_32px_rgba(15,23,42,0.08)]">
            <TrafficHistoryWorkbench
              ref="proxyHistoryRef"
              class="h-full overflow-auto"
              :basket-count="basketItems.length"
              :show-details="false"
              @selectionChange="handleHistorySelectionChange"
              @createDraft="handleCreateDraftFromHistory"
              @createAttackWorkspace="handleCreateAttackWorkspaceFromHistory"
              @openDraftCompare="handleOpenDraftCompareFromHistory"
              @openCompare="handleOpenCompareFromHistory"
              @addFilterRule="handleAddFilterRule"
              @addToBasket="handleAddToBasketFromHistory"
            />
          </section>

          <div
            v-if="showSidebarHeightResizeHandle"
            :class="[sidebarResizeHandleClass, 'hidden lg:block']"
            @mousedown="startSidebarHeightResize($event)"
          ></div>

          <TrafficWorkbenchSidebar
            class="min-h-0 h-full"
            :basket-count="basketItems.length"
            :control-intercept-count="controlInterceptCount"
            :repeater-history-count="workbenchState.counts.value.drafts"
            :intruder-history-count="workbenchState.counts.value.attackWorkspaces"
            :has-replay-history="hasReplayHistory"
            :running-attack-count="runningAttackCount"
            :layout-toggle-label="layoutToggleLabel"
            :layout-toggle-icon="layoutToggleIcon"
            :effective-layout-label="effectiveLayoutLabel"
            @open-capture="openCaptureWorkbench"
            @open-repeater="openRepeaterWorkbench"
            @open-intruder="openIntruderWorkbench"
            @clear-tool-history="clearToolHistory"
            @toggle-workbench-layout="toggleWorkbenchLayoutPreference"
            @toggle-intercept="toggleInterceptDrawer"
            @toggle-basket="toggleBasketDrawer"
            @open-settings="openProxySettingsDrawer"
            @open-plugins="openTrafficPluginsPanel"
          />
        </div>

        <div
          v-if="showHistoryPanelResizeHandle"
          :class="[historyResizeHandleClass, 'hidden lg:block']"
          @mousedown="startWorkbenchPanelResize('history', $event)"
        ></div>

        <section class="workbench-solid-surface min-h-0 min-w-0 overflow-hidden rounded-[16px] border border-base-300/70 shadow-[0_14px_32px_rgba(15,23,42,0.08)]">
          <div class="h-full min-h-0 p-1.5">
            <TrafficWorkbenchMainStage
              ref="mainStageRef"
              :workbench-open="workbenchOpen"
              :active-workbench-tool="activeWorkbenchTool"
              :mounted-tools="mountedWorkbenchTools"
              :active-workbench-meta="activeWorkbenchMeta"
              :tool-chips="toolChips"
              :pending-repeater-request="pendingRepeaterRequest"
              :pending-repeater-draft-id="pendingRepeaterDraftId"
              :pending-intruder-request="pendingIntruderRequest"
              :pending-intruder-workspace-id="pendingIntruderWorkspaceId"
              :active-request-context="activeRequestContext"
              @open-tool="handleOpenWorkbenchTool"
              @open-compare-from-repeater="handleOpenCompareFromRepeater"
              @open-draft-compare-from-repeater="handleOpenDraftCompareFromRepeater"
              @create-attack-workspace-from-repeater="handleCreateAttackWorkspaceFromRepeater"
              @repeater-tab-mode-changed="handleRepeaterTabModeChanged"
              @repeater-tab-stats-changed="handleRepeaterTabStatsChanged"
              @intruder-workspace-stats-changed="handleIntruderWorkspaceStatsChanged"
              @switch-request-variant="handleSwitchActiveRequestVariant"
              @create-draft-from-intruder="handleCreateDraftFromIntruder"
              @open-compare-from-intruder="handleOpenCompareFromIntruder"
              @open-draft-compare-from-intruder="handleOpenDraftCompareFromIntruder"
              @create-draft-from-comparer="handleCreateDraftFromComparer"
              @open-proxy-settings="openProxySettingsDrawer"
              @open-history-request-from-oast="openHistoryRequestFromOast"
            />
          </div>
        </section>

      </div>

      <TrafficWorkbenchOverlays
        ref="overlaysRef"
        :intercept-drawer-open="interceptDrawerOpen"
        :intercept-drawer-style="interceptDrawerStyle"
        :proxy-settings-open="proxySettingsOpen"
        :traffic-plugins-open="trafficPluginsOpen"
        :basket-open="basketOpen"
        :basket-items="basketItems"
        :control-title="$t('trafficAnalysis.tabs.control', '代理控制')"
        :plugins-title="$t('trafficAnalysis.immersivePlugins.title', '流量分析插件')"
        :active-probe-shell-style="activeProbeShellStyle"
        :pending-active-probe-entries="pendingActiveProbeEntries"
        :running-active-probe-entries="runningActiveProbeEntries"
        :recent-active-probe-entries="recentActiveProbeEntries"
        :active-probe-queued-count="activeProbeQueuedCount"
        :active-probe-collapsed="activeProbeCollapsed"
        :active-probe-preview-requests="activeProbePreviewRequests"
        :active-probe-preview-loading-id="activeProbePreviewLoadingId"
        :start-drawer-width-resize="startDrawerWidthResize"
        @open-proxy-settings="openProxySettingsDrawer"
        @close-intercept-drawer="closeInterceptDrawer"
        @open-response-interception-settings="handleOpenResponseInterceptionSettings"
        @intercept-queue-changed="handleInterceptQueueChanged"
        @create-draft-from-intercept="handleCreateDraftFromIntercept"
        @open-draft-compare-from-intercept="handleOpenDraftCompareFromIntercept"
        @create-attack-workspace-from-intercept="handleCreateAttackWorkspaceFromIntercept"
        @add-to-basket-from-intercept="handleAddToBasketFromIntercept"
        @close-proxy-settings="closeProxySettingsDrawer"
        @filter-rule-added="handleFilterRuleAdded"
        @close-traffic-plugins="closeTrafficPluginsPanel"
        @close-basket="closeBasketDrawer"
        @remove-basket-item="removeBasketItem"
        @clear-basket="clearBasket"
        @create-draft-from-basket-item="createDraftFromBasketItem"
        @create-attack-workspace-from-basket-item="createAttackWorkspaceFromBasketItem"
        @create-drafts-for-all-basket-items="createDraftsForAllBasketItems"
        @create-attack-workspaces-for-all-basket-items="createAttackWorkspacesForAllBasketItems"
        @open-history-request-from-basket="openHistoryRequestFromBasket"
        @toggle-active-probe-collapsed="toggleActiveProbeCollapsed"
        @ensure-active-probe-preview="ensureActiveProbePreview"
        @open-history-request-by-id="openHistoryRequestById"
        @open-history-request-by-traffic-request-id="openHistoryRequestByTrafficRequestId"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch, watchEffect } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useI18n } from 'vue-i18n'
import TrafficHistoryWorkbench from './TrafficHistoryWorkbench.vue'
import TrafficWorkbenchMainStage from './workbench/components/TrafficWorkbenchMainStage.vue'
import TrafficWorkbenchOverlays from './workbench/components/TrafficWorkbenchOverlays.vue'
import TrafficWorkbenchSidebar from './workbench/components/TrafficWorkbenchSidebar.vue'
import { useToast } from '@/composables/useToast'
import { dialog } from '@/composables/useDialog'
import type { HttpExchangeRequest } from './http/model'
import type { ProxyRequest } from './proxyHistoryTypes'
import type { TrafficAnalysisViewHandle } from './trafficAnalysisViewTypes'
import type {
  TrafficWorkbenchBasketItem,
  TrafficWorkbenchRequestContext,
  TrafficWorkbenchRequestVariant,
  TrafficWorkbenchToolSession,
} from './trafficWorkbenchTypes'
import type { RepeaterActiveTabState, RepeaterTabStats } from './proxyRepeaterTypes'
import type { TrafficComparerDraftRequestInput, TrafficComparePayload } from './transfers'
import { useActiveProbeQueue } from './useActiveProbeQueue'
import { useTrafficWorkbenchBasket } from './useTrafficWorkbenchBasket'
import { useTrafficWorkbenchSessions } from './useTrafficWorkbenchSessions'
import { hasEditedRequest, hasEditedResponse } from './proxyHistoryFormattingSupport'
import { useTrafficWorkbenchActions } from './workbench/composables/useTrafficWorkbenchActions'
import { useActiveProbePreview } from './workbench/composables/useActiveProbePreview'
import { useTrafficWorkbenchHistoryNavigation } from './workbench/composables/useTrafficWorkbenchHistoryNavigation'
import { useTrafficWorkbenchLayout } from './workbench/composables/useTrafficWorkbenchLayout'
import { useTrafficWorkbenchPersistence } from './workbench/composables/useTrafficWorkbenchPersistence'
import type { HistorySnapshotVariant } from './workbench/model/historySnapshot'
import { usePacketCaptureStatusStore } from './workbench/stores/usePacketCaptureStatusStore'
import {
  getHistorySource,
} from './workbench/services/sourceSupport'
import { useTrafficWorkbenchStore } from './workbench/stores/useTrafficWorkbenchStore'
import {
  refreshTrafficOastRecordCount,
  resetTrafficOastRecordCount,
  useTrafficOastRecordCount,
} from './trafficOastRecordCount'
import { immersiveDrillModeEnabled } from '@/services/immersiveDrillMode'
import { closeTopmostImmersiveTool } from '@/services/immersiveToolCoordinator'
import {
  closeImmersiveTrafficWorkbench,
  openImmersiveTrafficPluginsPanel,
  openImmersiveTrafficWorkbenchTool,
  openImmersiveTrafficBasket,
  openImmersiveTrafficProxySettings,
  resetImmersiveTrafficDockState,
  syncImmersiveTrafficDockState,
  toggleImmersiveTrafficBasket,
  toggleImmersiveTrafficInterceptDrawer,
  useImmersiveTrafficDockState,
} from './immersiveTrafficDockState'
interface FilterRule {
  matchType: string
  condition: string
  relationship?: string
}

interface ScanFindingEventPayload {
  vuln_id: string
  vuln_type: string
  severity: string
  url: string
  summary: string
  timestamp: string
}

type WorkbenchTool = TrafficWorkbenchToolSession['tool']

const { t } = useI18n()
const toast = useToast()

const proxyHistoryRef = ref<InstanceType<typeof TrafficHistoryWorkbench> | null>(null)
const mainStageRef = ref<InstanceType<typeof TrafficWorkbenchMainStage> | null>(null)
const overlaysRef = ref<InstanceType<typeof TrafficWorkbenchOverlays> | null>(null)
const workspaceStageRef = ref<HTMLElement | null>(null)
const leftColumnRef = ref<HTMLElement | null>(null)

const INTERCEPT_DRAWER_WIDTH_STORAGE_KEY = 'sentinel:traffic-workbench-intercept-width:v1'
const ACTIVE_PROBE_COLLAPSED_STORAGE_KEY = 'sentinel:traffic-active-probe-collapsed:v1'
const HISTORY_PANEL_WIDTH_STORAGE_KEY = 'sentinel:traffic-workbench-history-width:v1'
const HISTORY_PANEL_HEIGHT_STORAGE_KEY = 'sentinel:traffic-workbench-history-height:v1'
const SIDEBAR_HEIGHT_STORAGE_KEY = 'sentinel:traffic-workbench-sidebar-height:v1'
const SIDEBAR_WIDTH_STORAGE_KEY = 'sentinel:traffic-workbench-sidebar-width:v1'
const WORKBENCH_LAYOUT_STORAGE_KEY = 'sentinel:traffic-workbench-layout:v1'

const pendingRepeaterRequest = ref<HttpExchangeRequest | undefined>(undefined)
const pendingRepeaterDraftId = ref<string | undefined>(undefined)
const pendingIntruderRequest = ref<HttpExchangeRequest | undefined>(undefined)
const pendingIntruderWorkspaceId = ref<string | undefined>(undefined)
const pendingComparerPayloads = ref<TrafficComparePayload[]>([])
const pendingComparerDraftRequests = ref<TrafficComparerDraftRequestInput[]>([])
const selectedHistoryRequest = ref<ProxyRequest | null>(null)
const activeRequestContext = ref<TrafficWorkbenchRequestContext | null>(null)
const repeaterEditedTabCount = ref(0)
const intruderOpenWorkspaceCount = ref(0)
const mountedWorkbenchToolSet = ref<Set<WorkbenchTool>>(new Set())
const {
  workbenchOpen,
  activeWorkbenchTool,
  interceptDrawerOpen,
  proxySettingsOpen,
  trafficPluginsOpen,
  basketOpen,
  controlInterceptCount,
} = useImmersiveTrafficDockState()

const { basketItems, addRequest, removeItem, clear } = useTrafficWorkbenchBasket()
const { sessions, markSession, clearSessionCount } = useTrafficWorkbenchSessions()
const workbenchState = useTrafficWorkbenchStore()
const { oastRecordCount } = useTrafficOastRecordCount()
const packetCaptureStatus = usePacketCaptureStatusStore()
const persistence = useTrafficWorkbenchPersistence(workbenchState)
const {
  pendingEntries: pendingActiveProbeEntries,
  runningEntries: runningActiveProbeEntries,
  recentEntries: recentActiveProbeEntries,
  queuedCount: activeProbeQueuedCount,
  start: startActiveProbeQueue,
  stop: stopActiveProbeQueue,
} = useActiveProbeQueue()
const {
  previewRequests: activeProbePreviewRequests,
  previewLoadingId: activeProbePreviewLoadingId,
  rememberPreviewRequest: rememberActiveProbePreviewRequest,
  ensurePreview: ensureActiveProbePreview,
} = useActiveProbePreview()
const {
  openHistoryRequest,
  openHistoryRequestById,
  openHistoryRequestByTrafficRequestId,
  openHistoryRequestFromBasket,
  openHistoryRequestFromOast,
} = useTrafficWorkbenchHistoryNavigation({
  proxyHistoryRef,
  closeBasketDrawer,
  closeWorkbench,
  ensurePreview: ensureActiveProbePreview,
})
const {
  activeProbeCollapsed,
  workbenchGridStyle,
  leftColumnStyle,
  historyResizeHandleClass,
  sidebarResizeHandleClass,
  layoutToggleLabel,
  layoutToggleIcon,
  effectiveLayoutLabel,
  showHistoryPanelResizeHandle,
  showSidebarHeightResizeHandle,
  interceptDrawerStyle,
  activeProbeShellStyle,
  handleWindowResize,
  startWorkbenchPanelResize,
  stopWorkbenchPanelResize,
  startSidebarHeightResize,
  stopSidebarHeightResize,
  startDrawerWidthResize,
  stopDrawerWidthResize,
  normalizeWorkbenchLayout,
  toggleWorkbenchLayoutPreference,
  toggleActiveProbeCollapsed,
} = useTrafficWorkbenchLayout({
  workspaceStageRef,
  leftColumnRef,
  historyPanelWidthStorageKey: HISTORY_PANEL_WIDTH_STORAGE_KEY,
  historyPanelHeightStorageKey: HISTORY_PANEL_HEIGHT_STORAGE_KEY,
  sidebarHeightStorageKey: SIDEBAR_HEIGHT_STORAGE_KEY,
  sidebarWidthStorageKey: SIDEBAR_WIDTH_STORAGE_KEY,
  interceptDrawerWidthStorageKey: INTERCEPT_DRAWER_WIDTH_STORAGE_KEY,
  activeProbeCollapsedStorageKey: ACTIVE_PROBE_COLLAPSED_STORAGE_KEY,
  layoutPreferenceStorageKey: WORKBENCH_LAYOUT_STORAGE_KEY,
})
const findingToastIds = new Set<string>()
let hydrationPromise: Promise<void> | null = null

const workbenchMetaMap: Record<
  WorkbenchTool,
  { title: string; shortTitle: string; description: string }
> = {
  capture: {
    title: t('trafficAnalysis.tabs.capture', '抓包'),
    shortTitle: t('trafficAnalysis.tabs.capture', '抓包'),
    description: t('trafficAnalysis.workbench.toolDescriptions.capture'),
  },
  repeater: {
    title: t('trafficAnalysis.tabs.repeater', '重放器'),
    shortTitle: t('trafficAnalysis.tabs.repeater', '重放器'),
    description: t('trafficAnalysis.workbench.toolDescriptions.repeater'),
  },
  intruder: {
    title: t('trafficAnalysis.tabs.intruder', '爆破器'),
    shortTitle: t('trafficAnalysis.tabs.intruder', '爆破器'),
    description: t('trafficAnalysis.workbench.toolDescriptions.intruder'),
  },
  comparer: {
    title: t('trafficAnalysis.tabs.comparer', '对比器'),
    shortTitle: t('trafficAnalysis.tabs.comparer', '对比器'),
    description: t('trafficAnalysis.workbench.toolDescriptions.comparer'),
  },
  oast: {
    title: t('trafficAnalysis.tabs.oast', 'OAST'),
    shortTitle: t('trafficAnalysis.tabs.oast', 'OAST'),
    description: t('trafficAnalysis.workbench.toolDescriptions.oast'),
  },
}

const activeWorkbenchMeta = computed(() => workbenchMetaMap[activeWorkbenchTool.value])
const runningAttackCount = computed(() =>
  workbenchState.attack.workspaces.value.filter(workspace => workspace.runState === 'running').length,
)
const hasReplayHistory = computed(() => workbenchState.replay.replayRuns.value.length > 0)

const toolChips = computed(() => [
  {
    tool: 'repeater' as const,
    label: t('trafficAnalysis.tabs.repeater', '重放器'),
    shortLabel: t('trafficAnalysis.tabs.repeater', '重放器'),
    icon: 'fas fa-redo',
    count: workbenchState.counts.value.drafts,
  },
  {
    tool: 'intruder' as const,
    label: t('trafficAnalysis.tabs.intruder', '爆破器'),
    shortLabel: t('trafficAnalysis.tabs.intruder', '爆破器'),
    icon: 'fas fa-crosshairs',
    count: intruderOpenWorkspaceCount.value,
  },
  {
    tool: 'comparer' as const,
    label: t('trafficAnalysis.tabs.comparer', '对比器'),
    shortLabel: t('trafficAnalysis.tabs.comparer', '对比器'),
    icon: 'fas fa-not-equal',
    count: sessions.value.comparer.count,
  },
  {
    tool: 'oast' as const,
    label: t('trafficAnalysis.tabs.oast', 'OAST'),
    shortLabel: t('trafficAnalysis.tabs.oast', 'OAST'),
    icon: 'fas fa-satellite-dish',
    count: oastRecordCount.value,
  },
])
const mountedWorkbenchTools = computed(() => [...mountedWorkbenchToolSet.value])
const workbenchMetaTitleMap = {
  capture: workbenchMetaMap.capture.title,
  repeater: workbenchMetaMap.repeater.title,
  intruder: workbenchMetaMap.intruder.title,
  comparer: workbenchMetaMap.comparer.title,
  oast: workbenchMetaMap.oast.title,
} as const
const {
  handleCreateDraftFromHistory,
  handleCreateAttackWorkspaceFromHistory,
  handleOpenCompareFromHistory,
  handleOpenDraftCompareFromHistory,
  handleCreateDraftFromIntercept,
  handleCreateAttackWorkspaceFromIntercept,
  handleOpenDraftCompareFromIntercept,
  handleCreateAttackWorkspaceFromRepeater,
  handleOpenCompareFromRepeater,
  handleOpenDraftCompareFromRepeater,
  handleCreateDraftFromIntruder,
  handleOpenCompareFromIntruder,
  handleOpenDraftCompareFromIntruder,
  handleCreateDraftFromComparer,
  handleAddToBasketFromHistory,
  handleAddToBasketFromIntercept,
  previewRequestInRepeater,
  createDraftFromBasketItem,
  createAttackWorkspaceFromBasketItem,
  createDraftsForAllBasketItems,
  createAttackWorkspacesForAllBasketItems,
} = useTrafficWorkbenchActions({
  workbenchState,
  mainStageRef,
  basketItems,
  addBasketRequest: addRequest,
  markSession,
  openWorkbenchTool,
  closeInterceptDrawer,
  openBasket: openImmersiveTrafficBasket,
  pendingRepeaterDraftId,
  pendingRepeaterRequest,
  pendingIntruderWorkspaceId,
  pendingIntruderRequest,
  queueComparerComparison: payload => {
    pendingComparerPayloads.value.push(payload)
  },
  queueComparerDraftRequest: payload => {
    pendingComparerDraftRequests.value.push(payload)
  },
  workbenchMetaTitleMap,
})

function normalizeScanFindingPayload(payload: unknown): ScanFindingEventPayload | null {
  if (!payload || typeof payload !== 'object') {
    return null
  }

  const candidate = payload as Record<string, unknown>
  const vulnId = typeof candidate.vuln_id === 'string' ? candidate.vuln_id.trim() : ''
  const summary = typeof candidate.summary === 'string' ? candidate.summary.trim() : ''
  const severity = typeof candidate.severity === 'string' ? candidate.severity.trim().toLowerCase() : ''
  const url = typeof candidate.url === 'string' ? candidate.url.trim() : ''
  const vulnType = typeof candidate.vuln_type === 'string' ? candidate.vuln_type.trim() : ''
  const timestamp = typeof candidate.timestamp === 'string' ? candidate.timestamp : ''

  if (!vulnId || !summary || !severity || !url) {
    return null
  }

  return {
    vuln_id: vulnId,
    vuln_type: vulnType,
    severity,
    url,
    summary,
    timestamp,
  }
}

function findingToastType(severity: string): 'error' | 'warning' | 'info' {
  if (severity === 'critical' || severity === 'high') {
    return 'error'
  }
  if (severity === 'medium') {
    return 'warning'
  }
  return 'info'
}

function emitImmersiveFindingToast(finding: ScanFindingEventPayload) {
  if (!immersiveDrillModeEnabled.value) {
    return
  }

  if (findingToastIds.has(finding.vuln_id)) {
    return
  }
  findingToastIds.add(finding.vuln_id)

  const host = (() => {
    try {
      return new URL(finding.url).host
    } catch {
      return finding.url
    }
  })()

  toast.show({
    type: findingToastType(finding.severity),
    duration: 6000,
    message: t('trafficAnalysis.workbench.findingToast', {
      severity: finding.severity.toUpperCase(),
      summary: finding.summary,
      host,
    }),
  })
}

function openWorkbenchTool(tool: WorkbenchTool) {
  if (!mountedWorkbenchToolSet.value.has(tool)) {
    mountedWorkbenchToolSet.value = new Set([...mountedWorkbenchToolSet.value, tool])
  }
  openImmersiveTrafficWorkbenchTool(tool)
}

function flushPendingComparerTransfers() {
  const mainStage = mainStageRef.value
  if (!mainStage) {
    return
  }

  for (const payload of pendingComparerPayloads.value) {
    mainStage.addComparison(payload)
  }
  pendingComparerPayloads.value = []

  for (const payload of pendingComparerDraftRequests.value) {
    mainStage.addDraftRequest(payload)
  }
  pendingComparerDraftRequests.value = []
}

function hydratePersistence() {
  if (!hydrationPromise) {
    hydrationPromise = persistence.hydrate()
  }
  return hydrationPromise
}

async function ensurePersistenceReady() {
  if (persistence.ready.value) {
    return
  }
  await hydratePersistence()
}

async function handleOpenWorkbenchTool(tool: WorkbenchTool) {
  if (tool === 'repeater' || tool === 'intruder') {
    await ensurePersistenceReady()
  }
  clearSessionCount(tool)
  openWorkbenchTool(tool)
  if (tool === 'comparer') {
    await nextTick()
    flushPendingComparerTransfers()
  }
}

async function openCaptureWorkbench() {
  await handleOpenWorkbenchTool('capture')
}

async function openRepeaterWorkbench() {
  await handleOpenWorkbenchTool('repeater')
}

async function openIntruderWorkbench() {
  await handleOpenWorkbenchTool('intruder')
}

function closeWorkbench() {
  closeImmersiveTrafficWorkbench()
}

function syncHistorySnapshot(
  request: ProxyRequest,
  variant: HistorySnapshotVariant,
) {
  const snapshot = workbenchState.historySnapshots.createSnapshotFromProxyRequest(
    request,
    variant,
    getHistorySource(request),
  )
  workbenchState.selection.selectHistorySnapshot(snapshot)
  return snapshot
}

function resolveHistoryVariantUrl(request: ProxyRequest, variant: TrafficWorkbenchRequestVariant) {
  return variant === 'edited' && hasEditedRequest(request) && request.edited_url
    ? request.edited_url
    : request.url
}

function resolveHistoryVariantMethod(request: ProxyRequest, variant: TrafficWorkbenchRequestVariant) {
  return variant === 'edited' && hasEditedRequest(request) && request.edited_method
    ? request.edited_method
    : request.method
}

function resolveHistoryVariantStatusCode(request: ProxyRequest, variant: TrafficWorkbenchRequestVariant) {
  const statusCode = variant === 'edited' && hasEditedResponse(request) && request.edited_status_code
    ? request.edited_status_code
    : request.status_code
  return Number.isFinite(statusCode) ? statusCode : null
}

function buildActiveRequestContext(
  request: ProxyRequest,
  variant: TrafficWorkbenchRequestVariant,
  mode: TrafficWorkbenchRequestContext['mode'],
): TrafficWorkbenchRequestContext {
  const parsedUrl = new URL(resolveHistoryVariantUrl(request, variant))
  const source = getHistorySource(request)
  return {
    sourceKind: 'history',
    sourceLabel: source.label,
    requestId: request.id,
    sourceRequestId: request.db_request_id ?? null,
    method: resolveHistoryVariantMethod(request, variant),
    host: parsedUrl.host,
    path: `${parsedUrl.pathname}${parsedUrl.search}`,
    statusCode: resolveHistoryVariantStatusCode(request, variant),
    variant,
    hasEditedVariant: hasEditedRequest(request),
    hasEditedResponseVariant: hasEditedResponse(request),
    mode,
    modeLabel: resolveRequestContextModeLabel(mode),
  }
}

function previewHistoryRequest(request: ProxyRequest, variant: TrafficWorkbenchRequestVariant) {
  const snapshot = syncHistorySnapshot(request, variant)
  selectedHistoryRequest.value = request
  activeRequestContext.value = buildActiveRequestContext(request, variant, 'preview')
  previewRequestInRepeater(snapshot.request)
}

function handleHistorySelectionChange(request: ProxyRequest | null) {
  if (!request) {
    selectedHistoryRequest.value = null
    activeRequestContext.value = null
    workbenchState.historySnapshots.selectSnapshot(null)
    workbenchState.selection.clearSelectionKind('history-snapshot')
    return
  }

  previewHistoryRequest(request, hasEditedRequest(request) || hasEditedResponse(request) ? 'edited' : 'original')
}

function handleSwitchActiveRequestVariant(variant: TrafficWorkbenchRequestVariant) {
  const request = selectedHistoryRequest.value
  if (!request || (!hasEditedRequest(request) && !hasEditedResponse(request))) {
    return
  }
  previewHistoryRequest(request, variant)
}

function handleRepeaterTabModeChanged(state: RepeaterActiveTabState) {
  if (
    !activeRequestContext.value
    || !state.mode
    || activeRequestContext.value.sourceRequestId === null
    || state.sourceRequestId !== activeRequestContext.value.sourceRequestId
  ) {
    return
  }

  const mode = state.mode === 'draft' ? 'draft' : 'preview'
  activeRequestContext.value = {
    ...activeRequestContext.value,
    mode,
    modeLabel: resolveRequestContextModeLabel(mode),
  }
}

function resolveRequestContextModeLabel(mode: TrafficWorkbenchRequestContext['mode']) {
  switch (mode) {
    case 'draft':
      return t('trafficAnalysis.workbench.mainStage.modeDraft')
    case 'workspace':
      return t('trafficAnalysis.workbench.mainStage.modeWorkspace')
    case 'preview':
    default:
      return t('trafficAnalysis.workbench.mainStage.modePreview')
  }
}

function handleRepeaterTabStatsChanged(stats: RepeaterTabStats) {
  repeaterEditedTabCount.value = stats.editedTabCount
}

function handleIntruderWorkspaceStatsChanged(stats: { openWorkspaceCount: number }) {
  intruderOpenWorkspaceCount.value = stats.openWorkspaceCount
}

function openTrafficPluginsPanel() {
  openImmersiveTrafficPluginsPanel()
}

function removeBasketItem(id: string) {
  removeItem(id)
}

function clearBasket() {
  clear()
}

async function clearToolHistory() {
  await ensurePersistenceReady()
  const total = workbenchState.counts.value.drafts
    + workbenchState.counts.value.attackWorkspaces
    + workbenchState.replay.replayRuns.value.length
  if (total === 0) {
    return
  }

  const confirmed = await dialog.confirm({
    title: t('trafficAnalysis.workbench.sidebar.clearToolHistoryConfirmTitle'),
    message: t('trafficAnalysis.workbench.sidebar.clearToolHistoryConfirmMessage', { count: total }),
    confirmText: t('trafficAnalysis.workbench.sidebar.clearToolHistoryConfirm'),
    variant: 'warning',
  })
  if (!confirmed) {
    return
  }

  workbenchState.drafts.resetDraftStore()
  workbenchState.attack.resetAttackWorkspaceStore()
  workbenchState.replay.resetReplayStore()
  workbenchState.selection.clearSelection()
  activeRequestContext.value = null
  pendingRepeaterDraftId.value = undefined
  pendingRepeaterRequest.value = undefined
  pendingIntruderWorkspaceId.value = undefined
  pendingIntruderRequest.value = undefined
  repeaterEditedTabCount.value = 0
  intruderOpenWorkspaceCount.value = 0
  await persistence.persist()
  dialog.toast.success(t('trafficAnalysis.workbench.sidebar.clearToolHistorySuccess'))
}

async function handleAddFilterRule(rule: FilterRule) {
  openImmersiveTrafficProxySettings()
  await nextTick()
  overlaysRef.value?.addRequestFilterRule(
    rule.matchType,
    rule.condition,
    rule.relationship || 'matches',
  )
}

function handleFilterRuleAdded(rule: FilterRule) {
  proxyHistoryRef.value?.removeMatchingRecords({
    matchType: rule.matchType,
    condition: rule.condition,
    relationship: rule.relationship || 'matches',
  })
}

async function handleOpenResponseInterceptionSettings() {
  openImmersiveTrafficProxySettings()
  await nextTick()
  await overlaysRef.value?.openResponseInterceptionRules()
}

function handleInterceptQueueChanged(count: number) {
  controlInterceptCount.value = count
}

function closeInterceptDrawer() {
  interceptDrawerOpen.value = false
}

function toggleInterceptDrawer() {
  toggleImmersiveTrafficInterceptDrawer()
}

function closeBasketDrawer() {
  basketOpen.value = false
}

function toggleBasketDrawer() {
  toggleImmersiveTrafficBasket()
}

function openProxySettingsDrawer() {
  openImmersiveTrafficProxySettings()
}

function closeProxySettingsDrawer() {
  proxySettingsOpen.value = false
}

function closeTrafficPluginsPanel() {
  trafficPluginsOpen.value = false
}

function handleWindowKeydown(event: KeyboardEvent) {
  if (!immersiveDrillModeEnabled.value) {
    return
  }

  if (event.defaultPrevented || event.isComposing || event.repeat) {
    return
  }

  if (event.key !== 'Escape') {
    return
  }

  if (!closeTopmostImmersiveTool('transient')) {
    return
  }

  event.preventDefault()
  event.stopPropagation()
}

let unlistenProxyRequest: UnlistenFn | null = null
let unlistenScanFinding: UnlistenFn | null = null

async function createDraftFromRequest(request: HttpExchangeRequest) {
  await ensurePersistenceReady()
  handleCreateDraftFromHistory(request)
}

async function createAttackWorkspaceFromRequest(request: HttpExchangeRequest) {
  await ensurePersistenceReady()
  handleCreateAttackWorkspaceFromHistory(request)
}

async function openCompare(payload: Parameters<typeof handleOpenCompareFromHistory>[0]) {
  await ensurePersistenceReady()
  handleOpenCompareFromHistory(payload)
}

async function openDraftCompare(payload: Parameters<typeof handleOpenDraftCompareFromHistory>[0]) {
  await ensurePersistenceReady()
  handleOpenDraftCompareFromHistory(payload)
}

defineExpose<TrafficAnalysisViewHandle>({
  createDraftFromRequest,
  createAttackWorkspaceFromRequest,
  openCompare,
  openDraftCompare,
  openHistoryRequest,
})

onMounted(() => {
  void hydratePersistence()
  void refreshTrafficOastRecordCount().catch(error => {
    console.error('[TrafficWorkbench] Failed to load OAST record count:', error)
  })
  void nextTick(() => {
    normalizeWorkbenchLayout()
  })
  window.addEventListener('resize', handleWindowResize)
  window.addEventListener('keydown', handleWindowKeydown)
  void startActiveProbeQueue()
  void listen<ProxyRequest>('proxy:request', event => {
    const request = event.payload
    if (!request || typeof request !== 'object') {
      return
    }
    rememberActiveProbePreviewRequest(request)
  }).then(unlisten => {
    unlistenProxyRequest = unlisten
  })
  void listen('scan:finding', event => {
    const payload = normalizeScanFindingPayload(event.payload)
    if (!payload) {
      return
    }
    emitImmersiveFindingToast(payload)
  }).then(unlisten => {
    unlistenScanFinding = unlisten
  })
})

watch(
  [
    () => workbenchState.drafts.mutationVersion.value,
    () => workbenchState.attack.mutationVersion.value,
    () => workbenchState.replay.mutationVersion.value,
  ],
  () => {
    persistence.schedulePersist()
  },
)

watchEffect(() => {
  if (workbenchOpen.value) {
    if (!mountedWorkbenchToolSet.value.has(activeWorkbenchTool.value)) {
      mountedWorkbenchToolSet.value = new Set([
        ...mountedWorkbenchToolSet.value,
        activeWorkbenchTool.value,
      ])
    }
  }

  syncImmersiveTrafficDockState({
    workbenchOpen: workbenchOpen.value,
    activeWorkbenchTool: activeWorkbenchTool.value,
    interceptDrawerOpen: interceptDrawerOpen.value,
    proxySettingsOpen: proxySettingsOpen.value,
    trafficPluginsOpen: trafficPluginsOpen.value,
    basketOpen: basketOpen.value,
    captureCount: sessions.value.capture.count,
    repeaterCount: workbenchState.counts.value.drafts,
    intruderCount: intruderOpenWorkspaceCount.value,
    comparerCount: sessions.value.comparer.count,
    oastCount: oastRecordCount.value,
    controlInterceptCount: controlInterceptCount.value,
    basketCount: basketItems.value.length,
  })
})

onUnmounted(() => {
  persistence.stopPersistTimer()
  stopDrawerWidthResize()
  stopWorkbenchPanelResize()
  stopSidebarHeightResize()
  window.removeEventListener('resize', handleWindowResize)
  window.removeEventListener('keydown', handleWindowKeydown)
  stopActiveProbeQueue()
  if (unlistenProxyRequest) {
    unlistenProxyRequest()
    unlistenProxyRequest = null
  }
  if (unlistenScanFinding) {
    unlistenScanFinding()
    unlistenScanFinding = null
  }
  resetTrafficOastRecordCount()
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

.workbench-sidebar-card {
  background:
    radial-gradient(circle at top right, rgb(14 165 233 / 0.08), transparent 34%),
    linear-gradient(180deg, rgb(255 255 255 / 0.98), rgb(248 250 252 / 0.96));
}

.workbench-metric-card {
  display: flex;
  min-height: 5.5rem;
  flex-direction: column;
  justify-content: center;
  border-radius: 1.35rem;
  border: 1px solid hsl(var(--b3) / 0.7);
  background: hsl(var(--b1) / 0.84);
  padding: 1rem 1.1rem;
  box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.55);
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
.workspace-record:hover {
  transform: translateY(-1px);
}

.tool-switch-active {
  border-color: hsl(var(--p) / 0.28);
  background: hsl(var(--p) / 0.12);
  color: hsl(var(--p));
  box-shadow: 0 10px 26px hsl(var(--p) / 0.1);
}

.workspace-record {
  display: flex;
  width: 100%;
  align-items: center;
  gap: 0.75rem;
  border-radius: 1rem;
  border: 1px solid hsl(var(--b3) / 0.6);
  background: hsl(var(--b1) / 0.84);
  padding: 0.85rem 0.9rem;
  text-align: left;
  transition:
    transform 160ms ease,
    border-color 160ms ease,
    background-color 160ms ease,
    box-shadow 160ms ease;
}

.workspace-record-active {
  border-color: hsl(var(--p) / 0.28);
  background: hsl(var(--p) / 0.08);
  box-shadow: 0 10px 22px hsl(var(--p) / 0.08);
}

.workspace-empty-state {
  border-radius: 1rem;
  border: 1px dashed hsl(var(--b3) / 0.7);
  background: hsl(var(--b1) / 0.65);
  padding: 1rem;
  font-size: 0.82rem;
  color: hsl(var(--bc) / 0.58);
}

.workbench-column-resizer {
  position: relative;
  width: 4px;
  min-height: 0;
  cursor: col-resize;
  border-radius: 999px;
  background: linear-gradient(180deg, transparent 0%, rgb(148 163 184 / 0.14) 50%, transparent 100%);
  opacity: 0.42;
  transition: opacity 160ms ease, background-color 160ms ease;
}

.workbench-column-resizer::before {
  content: '';
  position: absolute;
  inset: 20px 1px;
  border-radius: 999px;
  background: rgb(148 163 184 / 0.38);
}

.workbench-column-resizer:hover {
  opacity: 1;
  background: linear-gradient(180deg, transparent 0%, rgb(59 130 246 / 0.24) 50%, transparent 100%);
}

.workbench-column-resizer:hover::before {
  background: rgb(59 130 246 / 0.58);
}

.workbench-row-resizer {
  position: relative;
  height: 4px;
  min-width: 0;
  cursor: row-resize;
  border-radius: 999px;
  background: linear-gradient(90deg, transparent 0%, rgb(148 163 184 / 0.14) 50%, transparent 100%);
  opacity: 0.42;
  transition: opacity 160ms ease, background-color 160ms ease;
}

.workbench-row-resizer::before {
  content: '';
  position: absolute;
  inset: 1px 20px;
  border-radius: 999px;
  background: rgb(148 163 184 / 0.38);
}

.workbench-row-resizer:hover {
  opacity: 1;
  background: linear-gradient(90deg, transparent 0%, rgb(59 130 246 / 0.24) 50%, transparent 100%);
}

.workbench-row-resizer:hover::before {
  background: rgb(59 130 246 / 0.58);
}

</style>
