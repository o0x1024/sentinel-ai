<template>
  <div class="flex h-full min-h-0 flex-col bg-base-100">
    <div class="flex items-center gap-2 border-b border-base-300 bg-base-200 px-2 py-1">
      <div class="flex min-w-0 flex-1 items-center gap-1 overflow-x-auto">
        <div
          v-for="(workspace, index) in workspaces"
          :key="workspace.id"
          class="flex items-center gap-2 rounded border border-base-300 px-3 py-1.5 text-sm"
          :class="activeWorkspaceId === workspace.id ? 'bg-base-100 border-primary' : 'bg-base-200 hover:bg-base-300'"
        >
          <button class="truncate" type="button" :title="workspace.name" @click="activeWorkspaceId = workspace.id">
            {{ index + 1 }}
          </button>
          <button class="btn btn-ghost btn-xs btn-circle" type="button" @click="closeWorkspace(workspace.id)">
            <i class="fas fa-times text-[10px]"></i>
          </button>
        </div>

        <button class="btn btn-xs btn-ghost" type="button" @click="addWorkspace()">
          <i class="fas fa-plus"></i>
        </button>
      </div>
    </div>

    <div v-if="currentWorkspace" class="flex min-h-0 flex-1 flex-col">
      <div class="flex flex-wrap items-start gap-3 border-b border-base-300 px-4 py-3">
        <div class="min-w-[28rem]">
          <IntruderAttackTypeSelect
            :model-value="currentWorkspace.attackType"
            variant="toolbar"
            @update:model-value="updateAttackType(currentWorkspace.id, $event)"
          />
        </div>

        <button class="btn btn-primary btn-sm" type="button" :disabled="currentWorkspace.isRunning" @click="startAttack">
          <i :class="['fas', currentWorkspace.isRunning ? 'fa-spinner fa-spin' : 'fa-play']"></i>
          {{ $t('trafficAnalysis.intruder.actions.startAttack') }}
        </button>
        <button class="btn btn-sm btn-ghost" type="button" :disabled="!currentWorkspace.isRunning" @click="stopAttack">
          <i class="fas fa-stop"></i>
          {{ $t('trafficAnalysis.intruder.actions.stopAttack') }}
        </button>
        <button class="btn btn-sm btn-ghost" type="button" :disabled="!currentWorkspace.results.length && !currentWorkspace.isRunning" @click="openResultsView">
          <i class="fas fa-table"></i>
          {{ $t('trafficAnalysis.intruder.actions.showResults') }}
        </button>
        <button class="btn btn-sm btn-ghost" type="button" @click="duplicateWorkspace">
          <i class="fas fa-clone"></i>
          {{ $t('trafficAnalysis.intruder.actions.cloneTab') }}
        </button>
        <button class="btn btn-sm btn-ghost" type="button" :disabled="!currentWorkspace.results.length" @click="clearResults">
          <i class="fas fa-trash-alt"></i>
          {{ $t('trafficAnalysis.intruder.actions.clearResults') }}
        </button>
        <IntruderAttackTemplateManager
          :templates="attackTemplates"
          :selected-template-id="selectedAttackTemplateId"
          :default-name="currentWorkspace.name"
          @update:selected-template-id="selectedAttackTemplateId = $event"
          @save="saveAttackTemplate"
          @overwrite="overwriteAttackTemplate"
          @rename="renameAttackTemplate"
          @load="loadAttackTemplate"
          @delete="deleteAttackTemplate"
        />
        <button class="btn btn-sm btn-ghost" type="button" :disabled="!currentWorkspace.results.length" @click="exportResults">
          <i class="fas fa-file-export"></i>
          {{ $t('trafficAnalysis.intruder.actions.exportResults') }}
        </button>

        <div class="ml-auto flex flex-wrap items-center gap-2 text-sm">
          <div class="badge badge-outline">{{ $t('trafficAnalysis.intruder.labels.requestCount') }}: {{ estimatedRequests }}</div>
          <div v-if="currentWorkspace.progress.completed > 0 || currentWorkspace.isRunning" class="badge badge-outline">
            {{ currentWorkspace.progress.completed }}/{{ currentWorkspace.progress.total }}
          </div>
          <div v-if="planWillTruncate" class="badge badge-warning badge-outline">
            {{ $t('trafficAnalysis.intruder.messages.attackPlanTrimmed') }}
          </div>
        </div>
      </div>

      <div ref="workspaceLayoutRef" class="flex min-h-0 flex-1">
        <div class="min-w-0 flex-1">
          <IntruderRequestEditor
            :request-text="currentWorkspace.requestText"
            :target-url="buildTargetUrl(currentWorkspace.target)"
            :update-host-header="currentWorkspace.attackOptions.updateHostHeader"
            :positions="currentWorkspace.positions"
            @update:request-text="updateRequestText(currentWorkspace.id, $event)"
            @update:target-url="updateTargetUrl(currentWorkspace.id, $event)"
            @update:update-host-header="updateAttackOption(currentWorkspace.id, 'updateHostHeader', $event)"
            @auto-mark="autoMarkPositions(currentWorkspace.id)"
            @clear-markers="clearMarkers(currentWorkspace.id)"
            @send-to-repeater="sendWorkspaceRequestToRepeater(currentWorkspace.id)"
            @send-draft-request-to-comparer="sendWorkspaceRequestToComparer(currentWorkspace.id)"
          />
        </div>

        <div
          class="w-1 cursor-col-resize border-l border-r border-base-300 bg-base-200 transition-colors hover:bg-primary/30"
          @mousedown="startSidebarResize"
        ></div>

        <div class="min-h-0 shrink-0" :style="{ width: `${sidebarWidth}px` }">
          <IntruderSidePanel
            :active-tab="activeSidebarTab"
            :payload-sets="currentWorkspace.payloadSets"
            :payload-processing-rules="currentWorkspace.payloadProcessingRules"
            :payload-processor-plugins="currentWorkspace.payloadProcessorPlugins"
            :request-processor-plugins="currentWorkspace.requestProcessorPlugins"
            :grep-match-rules="currentWorkspace.grepMatchRules"
            :grep-extract-rules="currentWorkspace.grepExtractRules"
            :grep-payload-settings="currentWorkspace.grepPayloadSettings"
            :resource-pool-presets="resourcePools"
            :selected-resource-pool-id="currentWorkspace.selectedResourcePoolId"
            :attack-options="currentWorkspace.attackOptions"
            :estimated-requests="estimatedRequests"
            :request-text="currentWorkspace.requestText"
            :target="currentWorkspace.target"
            :positions="currentWorkspace.positions"
            :max-requests="currentWorkspace.attackOptions.maxRequests"
            :request-processing-preview-loading="requestProcessingPreviewLoading"
            :request-processing-preview-original="requestProcessingPreviewOriginal"
            :request-processing-preview-final="requestProcessingPreviewFinal"
            :request-processing-preview-payload-summary="requestProcessingPreviewPayloadSummary"
            :request-processing-preview-traces="requestProcessingPreviewTraces"
            :request-processing-preview-error="requestProcessingPreviewError"
            @update:active-tab="activeSidebarTab = $event"
            @update:attack-options="updateAttackOptions(currentWorkspace.id, $event)"
            @update-payload-set="(payloadSetId, patch) => updatePayloadSet(currentWorkspace.id, payloadSetId, patch)"
            @update:payload-processing-rules="updatePayloadProcessingRules(currentWorkspace.id, $event)"
            @update:payload-processor-plugins="updatePayloadProcessorPlugins(currentWorkspace.id, $event)"
            @update:request-processor-plugins="updateRequestProcessorPlugins(currentWorkspace.id, $event)"
            @update:grep-match-rules="updateGrepMatchRules(currentWorkspace.id, $event)"
            @update:grep-extract-rules="updateGrepExtractRules(currentWorkspace.id, $event)"
            @update:grep-payload-settings="updateGrepPayloadSettings(currentWorkspace.id, $event)"
            @select-resource-pool-preset="selectResourcePoolPreset(currentWorkspace.id, $event)"
            @upsert-resource-pool="upsertResourcePool(currentWorkspace.id, $event)"
            @delete-resource-pool="deleteResourcePool(currentWorkspace.id, $event)"
            @preview-request-processing="previewRequestProcessing"
          />
        </div>
      </div>

      <div class="flex items-center gap-3 border-t border-base-300 bg-base-200 px-4 py-2 text-xs text-base-content/70">
        <span>{{ currentWorkspace.target.useTls ? 'https' : 'http' }}://{{ currentWorkspace.target.host || 'example.com' }}:{{ currentWorkspace.target.port }}</span>
        <span>{{ currentWorkspace.positions.length }} {{ $t('trafficAnalysis.intruder.labels.detectedPositions') }}</span>
        <span v-if="currentWorkspace.isRunning">{{ $t('trafficAnalysis.intruder.labels.running') }}</span>
      </div>
    </div>

    <div v-else class="flex flex-1 items-center justify-center text-sm text-base-content/60">
      {{ $t('trafficAnalysis.intruder.empty.noWorkspace') }}
    </div>

    <IntruderAttackResults
      v-if="currentWorkspace"
      :open="showResultsDialog"
      :workspace-name="`${currentWorkspace.name} (${buildTargetUrl(currentWorkspace.target)})`"
      :request-text="currentWorkspace.requestText"
      :positions="currentWorkspace.positions"
      :results="currentWorkspace.results"
      :selected-result-id="currentWorkspace.selectedResultId"
      :selected-result="selectedResult"
      :progress="currentWorkspace.progress"
      :is-running="currentWorkspace.isRunning"
      :capture-filter="currentWorkspace.captureFilter"
      :view-filter="currentWorkspace.viewFilter"
      :sort="currentWorkspace.sort"
      :grep-match-rules="currentWorkspace.grepMatchRules"
      :grep-extract-rules="currentWorkspace.grepExtractRules"
      :grep-payload-settings="currentWorkspace.grepPayloadSettings"
      :visible-columns="currentWorkspace.visibleColumns"
      @close="showResultsDialog = false"
      @select-result="selectResult(currentWorkspace.id, $event)"
      @update:capture-filter="updateCaptureFilter(currentWorkspace.id, $event)"
      @update:view-filter="updateViewFilter(currentWorkspace.id, $event)"
      @update:sort="updateResultSort(currentWorkspace.id, $event)"
      @update:visible-columns="updateVisibleColumns(currentWorkspace.id, $event)"
      @send-to-repeater="sendSelectedResultToRepeater(currentWorkspace.id, $event)"
      @send-to-comparer="sendSelectedResultToComparer(currentWorkspace.id, $event)"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import { save } from '@tauri-apps/plugin-dialog'
import { writeTextFile } from '@tauri-apps/plugin-fs'
import { useI18n } from 'vue-i18n'
import { dialog } from '@/composables/useDialog'
import IntruderAttackResults from './intruder/IntruderAttackResults.vue'
import IntruderAttackTypeSelect from './intruder/IntruderAttackTypeSelect.vue'
import IntruderAttackTemplateManager from './intruder/IntruderAttackTemplateManager.vue'
import IntruderRequestEditor from './intruder/IntruderRequestEditor.vue'
import IntruderSidePanel from './intruder/IntruderSidePanel.vue'
import {
  autoMarkIntruderPositions,
  buildIntruderAttackPlan,
  clearIntruderMarkers,
  extractIntruderPositions,
  getRequiredPayloadSetCount,
  estimateAttackCount,
} from './intruder/attack'
import {
  applyIntruderRequestSettings,
  buildSourceRequestFromRawRequest,
  countLines,
  countWords,
  createIntruderId,
  createRawRequestFromSource,
  ensureRawRequestTerminator,
  extractTargetFromRequest,
} from './intruder/http'
import type { RawReplayCommandResult } from './http/response'
import { queueComparerTransfer, queueRepeaterTransfer } from './transfers'
import type { TrafficComparePayload } from './transfers'
import {
  createDefaultVisibleColumns,
  evaluateIntruderPayloadReflections,
  evaluateIntruderGrepExtracts,
  evaluateIntruderGrepMatches,
  normalizeGrepMatchRule,
  normalizeGrepPayloadSettings,
} from './intruder/analysis'
import {
  createDefaultResultFilter,
  createDefaultResultSort,
  getIntruderResultsStorageKey,
  loadIntruderResultsWindowState,
  matchesIntruderResultFilter,
  normalizeIntruderResultFilter,
  saveIntruderResultsWindowState,
  sortIntruderResults,
} from './intruder/results'
import {
  createBuiltInResourcePools,
  createIntruderAttackTemplate,
  createIntruderResourcePool,
  exportIntruderResultsCsv,
  loadIntruderAttackTemplates,
  loadIntruderResourcePools,
  normalizeVisibleColumns,
  persistIntruderAttackTemplates,
  persistIntruderResourcePools,
  type IntruderAttackTemplate,
} from './intruder/storage'
import {
  generateIntruderPluginPayloads,
  processIntruderPayloadWithPlugin,
  transformIntruderRequestWithPlugin,
  type IntruderRequestProcessorTrace,
} from './intruder/plugins'
import { buildIntruderResultsWindowUrl } from '@/router/standalone'
import type {
  IntruderAttackOptions,
  IntruderAttackProgress,
  IntruderAttackResult,
  IntruderAttackType,
  IntruderGrepExtractRule,
  IntruderGrepMatchRule,
  IntruderGrepPayloadSettings,
  IntruderPluginProcessorBinding,
  IntruderPayloadProcessingRule,
  IntruderPayloadSet,
  IntruderPosition,
  IntruderRequestInput,
  IntruderResourcePool,
  IntruderResultFilter,
  IntruderResultSort,
  IntruderTarget,
} from './intruder/types'

interface ReplayCommandResponse<T> {
  success: boolean
  data?: T
  error?: string
}

interface IntruderWorkspace {
  id: string
  name: string
  requestText: string
  target: IntruderTarget
  positions: IntruderPosition[]
  attackType: IntruderAttackType
  payloadSets: IntruderPayloadSet[]
  payloadProcessingRules: IntruderPayloadProcessingRule[]
  payloadProcessorPlugins: IntruderPluginProcessorBinding[]
  requestProcessorPlugins: IntruderPluginProcessorBinding[]
  grepMatchRules: IntruderGrepMatchRule[]
  grepExtractRules: IntruderGrepExtractRule[]
  grepPayloadSettings: IntruderGrepPayloadSettings
  selectedResourcePoolId: string
  attackOptions: IntruderAttackOptions
  results: IntruderAttackResult[]
  selectedResultId: string | null
  progress: IntruderAttackProgress
  isRunning: boolean
  captureFilter: IntruderResultFilter
  viewFilter: IntruderResultFilter
  sort: IntruderResultSort
  visibleColumns: string[]
}

const INTRUDER_WORKSPACE_STORAGE_KEY = 'trafficAnalysis.intruder.workspaces.v1'
const INTRUDER_SIDEBAR_WIDTH_KEY = 'trafficAnalysis.intruder.sidebarWidth.v1'
const INTRUDER_SIDEBAR_MIN_WIDTH = 320
const INTRUDER_SIDEBAR_MAX_WIDTH = 720

const props = defineProps<{
  initialRequest?: IntruderRequestInput
}>()

const emit = defineEmits<{
  (e: 'sendToRepeater', request: IntruderRequestInput): void
  (e: 'sendToComparer', payload: TrafficComparePayload): void
  (e: 'sendDraftRequestToComparer', payload: { request: IntruderRequestInput; label?: string }): void
}>()

const { t } = useI18n()
const workspaces = ref<IntruderWorkspace[]>([])
const activeWorkspaceId = ref<string | null>(null)
const activeSidebarTab = ref<'payloads' | 'resourcePool' | 'settings'>('payloads')
const showResultsDialog = ref(false)
const attackControllers = new Map<string, { cancelled: boolean }>()
const resourcePools = ref<IntruderResourcePool[]>(loadIntruderResourcePools(createBuiltInResourcePools()))
const attackTemplates = ref<IntruderAttackTemplate[]>(loadIntruderAttackTemplates())
const selectedAttackTemplateId = ref('')
const requestProcessingPreviewLoading = ref(false)
const requestProcessingPreviewOriginal = ref('')
const requestProcessingPreviewFinal = ref('')
const requestProcessingPreviewPayloadSummary = ref('')
const requestProcessingPreviewTraces = ref<IntruderRequestProcessorTrace[]>([])
const requestProcessingPreviewError = ref('')
const workspaceLayoutRef = ref<HTMLElement | null>(null)
const sidebarWidth = ref(loadSidebarWidth())
const isResizingSidebar = ref(false)
const sidebarResizeStartX = ref(0)
const sidebarResizeStartWidth = ref(sidebarWidth.value)

const currentWorkspace = computed(() => workspaces.value.find((workspace) => workspace.id === activeWorkspaceId.value) ?? null)
const selectedResult = computed(() => {
  const workspace = currentWorkspace.value
  if (!workspace?.selectedResultId) return null
  return workspace.results.find((result) => result.id === workspace.selectedResultId) ?? null
})
const estimatedRequests = computed(() => {
  const workspace = currentWorkspace.value
  if (!workspace) return 0

  const estimate = estimateAttackCount(workspace.attackType, workspace.positions.length, workspace.payloadSets)
  return workspace.attackOptions.makeUnmodifiedBaseline && workspace.positions.length > 0 ? estimate + 1 : estimate
})
const planWillTruncate = computed(() => {
  const workspace = currentWorkspace.value
  if (!workspace) return false
  return estimatedRequests.value > workspace.attackOptions.maxRequests
})

function loadSidebarWidth(): number {
  const raw = localStorage.getItem(INTRUDER_SIDEBAR_WIDTH_KEY)
  const width = raw ? Number.parseInt(raw, 10) : 448
  return Number.isFinite(width) ? width : 448
}

function clampSidebarWidth(width: number): number {
  const containerWidth = workspaceLayoutRef.value?.clientWidth ?? window.innerWidth
  const maxWidth = Math.max(INTRUDER_SIDEBAR_MIN_WIDTH, Math.min(INTRUDER_SIDEBAR_MAX_WIDTH, containerWidth - 360))
  return Math.min(maxWidth, Math.max(INTRUDER_SIDEBAR_MIN_WIDTH, width))
}

function persistSidebarWidth() {
  localStorage.setItem(INTRUDER_SIDEBAR_WIDTH_KEY, String(sidebarWidth.value))
}

function handleSidebarResize(event: MouseEvent) {
  if (!isResizingSidebar.value) return
  const delta = sidebarResizeStartX.value - event.clientX
  sidebarWidth.value = clampSidebarWidth(sidebarResizeStartWidth.value + delta)
}

function stopSidebarResize() {
  if (!isResizingSidebar.value) return
  isResizingSidebar.value = false
  document.body.style.cursor = ''
  document.body.style.userSelect = ''
  window.removeEventListener('mousemove', handleSidebarResize)
  window.removeEventListener('mouseup', stopSidebarResize)
  persistSidebarWidth()
}

function startSidebarResize(event: MouseEvent) {
  event.preventDefault()
  event.stopPropagation()
  isResizingSidebar.value = true
  sidebarResizeStartX.value = event.clientX
  sidebarResizeStartWidth.value = sidebarWidth.value
  document.body.style.cursor = 'col-resize'
  document.body.style.userSelect = 'none'
  window.addEventListener('mousemove', handleSidebarResize)
  window.addEventListener('mouseup', stopSidebarResize)
}

function handleWindowResize() {
  sidebarWidth.value = clampSidebarWidth(sidebarWidth.value)
  persistSidebarWidth()
}

function buildResultsWindowLabel(workspaceId: string): string {
  return `intruder-results-${workspaceId}`
}

function buildResultsWindowState(workspace: IntruderWorkspace) {
  return {
    workspaceId: workspace.id,
    workspaceName: `${workspace.name} (${buildTargetUrl(workspace.target)})`,
    target: workspace.target,
    requestText: workspace.requestText,
    positions: workspace.positions,
    results: workspace.results,
    selectedResultId: workspace.selectedResultId,
    progress: workspace.progress,
    isRunning: workspace.isRunning,
    captureFilter: workspace.captureFilter,
    viewFilter: workspace.viewFilter,
    sort: workspace.sort,
    grepMatchRules: workspace.grepMatchRules,
    grepExtractRules: workspace.grepExtractRules,
    grepPayloadSettings: workspace.grepPayloadSettings,
    visibleColumns: workspace.visibleColumns,
  }
}

function syncResultsWindowState(workspace: IntruderWorkspace) {
  saveIntruderResultsWindowState(buildResultsWindowState(workspace))
}

function createDefaultProgress(): IntruderAttackProgress {
  return {
    total: 0,
    completed: 0,
    failed: 0,
    active: 0,
    truncated: false,
  }
}

function createDefaultAttackOptions(): IntruderAttackOptions {
  return {
    concurrency: 10,
    delayMs: 0,
    randomDelayMs: 0,
    delayIncrementMs: 0,
    autoThrottleEnabled: false,
    autoThrottleStatusCodes: [429, 503],
    timeoutSecs: 30,
    maxRequests: 500,
    updateHostHeader: true,
    updateContentLength: true,
    setConnectionClose: true,
    followRedirects: false,
    maxRedirects: 5,
    processCookiesInRedirects: true,
    retryCount: 3,
    retryPauseMs: 2000,
    storeRequests: true,
    storeResponses: true,
    storeFullPayloads: false,
    denialOfServiceMode: false,
    makeUnmodifiedBaseline: true,
    autoPauseEnabled: false,
    autoPauseMode: 'contains',
    autoPauseExpression: '',
    autoPauseExpressions: [],
  }
}

function normalizeAttackOptions(value?: Partial<IntruderAttackOptions>): IntruderAttackOptions {
  const defaults = createDefaultAttackOptions()
  const normalized = { ...defaults, ...(value || {}) }
  const autoThrottleStatusCodes = Array.isArray(value?.autoThrottleStatusCodes)
    ? value.autoThrottleStatusCodes
      .map((item) => Number(item))
      .filter((item) => Number.isInteger(item) && item >= 100 && item <= 999)
    : defaults.autoThrottleStatusCodes
  const expressions = Array.isArray(value?.autoPauseExpressions)
    ? value?.autoPauseExpressions.filter((item) => typeof item === 'string' && item.trim().length > 0)
    : []

  return {
    ...normalized,
    delayIncrementMs: Math.max(0, Number(normalized.delayIncrementMs) || 0),
    autoThrottleEnabled: Boolean(normalized.autoThrottleEnabled),
    autoThrottleStatusCodes,
    autoPauseExpressions: expressions.length
      ? expressions
      : normalized.autoPauseExpression.trim()
        ? [normalized.autoPauseExpression.trim()]
        : [],
  }
}

function createDefaultPayloadSet(index: number): IntruderPayloadSet {
  return {
    id: createIntruderId('payload-set'),
    name: `${t('trafficAnalysis.intruder.labels.payloadSet')} ${index + 1}`,
    payloadType: 'simpleList',
    payloadsText: '',
    urlEncode: false,
    urlEncodeCharacters: String.raw`./\=<>?+&*;:"' {}|^#`,
    pluginId: '',
    pluginPresetName: '',
    pluginConfig: '{}',
    filePath: '',
    characterList: '',
    substitutionSource: '',
    substitutionRules: '',
    numberFrom: 0,
    numberTo: 100,
    numberStep: 1,
    numberPadWidth: 0,
    dateFrom: '2026-01-01',
    dateTo: '2026-01-07',
    dateStepDays: 1,
    dateFormat: 'yyyy-MM-dd',
    nullCount: 10,
    nullValue: '',
    usernameFirstNames: '',
    usernameLastNames: '',
    usernameFormats: '{first}.{last}\n{f}{last}\n{first}{l}',
  }
}

function createDefaultPluginProcessorBinding(): IntruderPluginProcessorBinding {
  return {
    id: createIntruderId('plugin-processor'),
    pluginId: '',
    presetName: '',
    enabled: true,
    config: '{}',
  }
}

function createWorkspace(source?: IntruderRequestInput): IntruderWorkspace {
  const requestText = createRawRequestFromSource(source)
  const target = extractTargetFromRequest(requestText, source?.absoluteUrl)
  const positions = extractIntruderPositions(requestText)

  return {
    id: createIntruderId('intruder-workspace'),
    name: target.host || `${t('trafficAnalysis.intruder.labels.attack')} ${workspaces.value.length + 1}`,
    requestText,
    target,
    positions,
    attackType: 'sniper',
    payloadSets: [createDefaultPayloadSet(0)],
    payloadProcessingRules: [],
    payloadProcessorPlugins: [],
    requestProcessorPlugins: [],
    grepMatchRules: [],
    grepExtractRules: [],
    grepPayloadSettings: normalizeGrepPayloadSettings(),
    selectedResourcePoolId: 'default',
    attackOptions: createDefaultAttackOptions(),
    results: [],
    selectedResultId: null,
    progress: createDefaultProgress(),
    isRunning: false,
    captureFilter: createDefaultResultFilter(),
    viewFilter: createDefaultResultFilter(),
    sort: createDefaultResultSort(),
    visibleColumns: createDefaultVisibleColumns([], [], normalizeGrepPayloadSettings()),
  }
}

function serializeWorkspaces() {
  return {
    activeWorkspaceId: activeWorkspaceId.value,
    workspaces: workspaces.value.map((workspace) => ({
      id: workspace.id,
      name: workspace.name,
      requestText: workspace.requestText,
      target: workspace.target,
      positions: workspace.positions,
      attackType: workspace.attackType,
      payloadSets: workspace.payloadSets,
      payloadProcessingRules: workspace.payloadProcessingRules,
      payloadProcessorPlugins: workspace.payloadProcessorPlugins,
      requestProcessorPlugins: workspace.requestProcessorPlugins,
      grepMatchRules: workspace.grepMatchRules,
      grepExtractRules: workspace.grepExtractRules,
      grepPayloadSettings: workspace.grepPayloadSettings,
      selectedResourcePoolId: workspace.selectedResourcePoolId,
      attackOptions: workspace.attackOptions,
      captureFilter: workspace.captureFilter,
      viewFilter: workspace.viewFilter,
      sort: workspace.sort,
      visibleColumns: workspace.visibleColumns,
    })),
  }
}

function restorePersistedWorkspaces() {
  const raw = localStorage.getItem(INTRUDER_WORKSPACE_STORAGE_KEY)
  if (!raw) return false

  try {
    const persisted = JSON.parse(raw) as {
      activeWorkspaceId?: string | null
      workspaces?: Array<Partial<IntruderWorkspace>>
    }
    if (!Array.isArray(persisted.workspaces) || !persisted.workspaces.length) {
      return false
    }

    workspaces.value = persisted.workspaces.map((workspace, index) => ({
      id: workspace.id || createIntruderId('intruder-workspace'),
      name: workspace.name || `${t('trafficAnalysis.intruder.labels.attack')} ${index + 1}`,
      requestText: workspace.requestText || createRawRequestFromSource(),
      target: workspace.target || extractTargetFromRequest(workspace.requestText || ''),
      positions: workspace.positions || extractIntruderPositions(workspace.requestText || ''),
      attackType: workspace.attackType || 'sniper',
      payloadSets: (workspace.payloadSets?.length ? workspace.payloadSets : [createDefaultPayloadSet(0)]).map((item, payloadIndex) => ({
        ...createDefaultPayloadSet(payloadIndex),
        ...item,
      })),
      payloadProcessingRules: workspace.payloadProcessingRules || [],
      payloadProcessorPlugins: (workspace.payloadProcessorPlugins || []).map((item) => ({
        ...createDefaultPluginProcessorBinding(),
        ...item,
      })),
      requestProcessorPlugins: (workspace.requestProcessorPlugins || []).map((item) => ({
        ...createDefaultPluginProcessorBinding(),
        ...item,
      })),
      grepMatchRules: (workspace.grepMatchRules || []).map((item) => normalizeGrepMatchRule(item)),
      grepExtractRules: workspace.grepExtractRules || [],
      grepPayloadSettings: normalizeGrepPayloadSettings(workspace.grepPayloadSettings),
      selectedResourcePoolId: workspace.selectedResourcePoolId || 'default',
      attackOptions: normalizeAttackOptions(workspace.attackOptions),
      results: [],
      selectedResultId: null,
      progress: createDefaultProgress(),
      isRunning: false,
      captureFilter: normalizeIntruderResultFilter(workspace.captureFilter),
      viewFilter: normalizeIntruderResultFilter(workspace.viewFilter),
      sort: { ...createDefaultResultSort(), ...(workspace.sort || {}) },
      visibleColumns: normalizeVisibleColumns(
        workspace.visibleColumns,
        workspace.grepMatchRules || [],
        workspace.grepExtractRules || [],
        normalizeGrepPayloadSettings(workspace.grepPayloadSettings),
      ),
    }))

    activeWorkspaceId.value = workspaces.value.some((workspace) => workspace.id === persisted.activeWorkspaceId)
      ? persisted.activeWorkspaceId || null
      : workspaces.value[0]?.id ?? null
    return true
  } catch {
    return false
  }
}

function persistWorkspaces() {
  localStorage.setItem(INTRUDER_WORKSPACE_STORAGE_KEY, JSON.stringify(serializeWorkspaces()))
}

function createAttackTemplateFromWorkspace(workspace: IntruderWorkspace, name: string): IntruderAttackTemplate {
  return createIntruderAttackTemplate(name, {
    requestText: workspace.requestText,
    target: workspace.target,
    attackType: workspace.attackType,
    payloadSets: workspace.payloadSets,
    payloadProcessingRules: workspace.payloadProcessingRules,
    payloadProcessorPlugins: workspace.payloadProcessorPlugins,
    requestProcessorPlugins: workspace.requestProcessorPlugins,
    grepMatchRules: workspace.grepMatchRules,
    grepExtractRules: workspace.grepExtractRules,
    grepPayloadSettings: workspace.grepPayloadSettings,
    selectedResourcePoolId: workspace.selectedResourcePoolId,
    attackOptions: workspace.attackOptions,
    captureFilter: workspace.captureFilter,
    viewFilter: workspace.viewFilter,
    sort: workspace.sort,
    visibleColumns: workspace.visibleColumns,
  })
}

function findWorkspace(id: string): IntruderWorkspace | undefined {
  return workspaces.value.find((workspace) => workspace.id === id)
}

function ensureWorkspaceSelection() {
  if (!workspaces.value.length) {
    const workspace = createWorkspace()
    workspaces.value = [workspace]
    activeWorkspaceId.value = workspace.id
    return
  }

  if (!activeWorkspaceId.value || !findWorkspace(activeWorkspaceId.value)) {
    activeWorkspaceId.value = workspaces.value[0].id
  }
}

function syncPayloadSets(workspaceId: string, trimExcess: boolean) {
  const workspace = findWorkspace(workspaceId)
  if (!workspace) return

  const requiredCount = getRequiredPayloadSetCount(workspace.attackType, workspace.positions.length)
  while (workspace.payloadSets.length < requiredCount) {
    workspace.payloadSets.push(createDefaultPayloadSet(workspace.payloadSets.length))
  }

  if (trimExcess && workspace.payloadSets.length > requiredCount) {
    workspace.payloadSets = workspace.payloadSets.slice(0, requiredCount)
  }
}

function applyDerivedWorkspaceState(workspace: IntruderWorkspace) {
  workspace.positions = extractIntruderPositions(workspace.requestText)

  const detectedTarget = extractTargetFromRequest(workspace.requestText)
  if (!workspace.target.host || workspace.target.host === 'example.com') {
    workspace.target = {
      ...workspace.target,
      ...detectedTarget,
    }
  }

  if (workspace.target.host) {
    workspace.name = workspace.target.host
  }

  syncPayloadSets(workspace.id, false)
}

function buildTargetUrl(target: IntruderTarget): string {
  const protocol = target.useTls ? 'https' : 'http'
  const defaultPort = target.useTls ? 443 : 80
  const portSuffix = target.port === defaultPort ? '' : `:${target.port}`
  return `${protocol}://${target.host || 'example.com'}${portSuffix}`
}

function parseTargetUrl(value: string): IntruderTarget | null {
  try {
    const url = new URL(value.includes('://') ? value : `https://${value}`)
    return {
      host: url.hostname,
      port: url.port ? Number.parseInt(url.port, 10) : url.protocol === 'https:' ? 443 : 80,
      useTls: url.protocol === 'https:',
    }
  } catch {
    return null
  }
}

function addWorkspace(source?: IntruderRequestInput) {
  const workspace = createWorkspace(source)
  workspaces.value.push(workspace)
  activeWorkspaceId.value = workspace.id
}

function addRequestFromHistory(request: IntruderRequestInput) {
  addWorkspace(request)
}

function closeWorkspace(workspaceId: string) {
  const controller = attackControllers.get(workspaceId)
  if (controller) {
    controller.cancelled = true
  }
  attackControllers.delete(workspaceId)
  void closeResultsWindow(workspaceId)
  localStorage.removeItem(getIntruderResultsStorageKey(workspaceId))
  workspaces.value = workspaces.value.filter((workspace) => workspace.id !== workspaceId)
  ensureWorkspaceSelection()
}

function duplicateWorkspace() {
  const workspace = currentWorkspace.value
  if (!workspace) return

  const copy: IntruderWorkspace = {
    ...workspace,
    id: createIntruderId('intruder-workspace'),
    name: `${workspace.name} Copy`,
    payloadSets: workspace.payloadSets.map((payloadSet) => ({ ...payloadSet, id: createIntruderId('payload-set') })),
    payloadProcessingRules: workspace.payloadProcessingRules.map((rule) => ({ ...rule, id: createIntruderId('payload-rule') })),
    payloadProcessorPlugins: workspace.payloadProcessorPlugins.map((binding) => ({ ...binding, id: createIntruderId('plugin-processor') })),
    requestProcessorPlugins: workspace.requestProcessorPlugins.map((binding) => ({ ...binding, id: createIntruderId('plugin-processor') })),
    grepMatchRules: workspace.grepMatchRules.map((rule) => ({ ...rule, id: createIntruderId('grep-match') })),
    grepExtractRules: workspace.grepExtractRules.map((rule) => ({ ...rule, id: createIntruderId('grep-extract') })),
    results: [],
    selectedResultId: null,
    progress: createDefaultProgress(),
    isRunning: false,
    visibleColumns: [...workspace.visibleColumns],
  }

  workspaces.value.push(copy)
  activeWorkspaceId.value = copy.id
}

function updateRequestText(workspaceId: string, value: string) {
  const workspace = findWorkspace(workspaceId)
  if (!workspace) return

  workspace.requestText = value
  applyDerivedWorkspaceState(workspace)
}

function sendWorkspaceRequestToRepeater(workspaceId: string) {
  const workspace = findWorkspace(workspaceId)
  if (!workspace) return

  const request = buildSourceRequestFromRawRequest(workspace.requestText, workspace.target)
  if (!request) {
    dialog.toast.warning(t('trafficAnalysis.repeater.messages.invalidRequestForIntruder'))
    return
  }

  emit('sendToRepeater', request)
  dialog.toast.success(t('trafficAnalysis.history.messages.sentToRepeater'))
}

function sendWorkspaceRequestToComparer(workspaceId: string) {
  const workspace = findWorkspace(workspaceId)
  if (!workspace) return

  const request = buildSourceRequestFromRawRequest(workspace.requestText, workspace.target)
  if (!request) {
    dialog.toast.warning(t('trafficAnalysis.repeater.messages.invalidRequestForComparer'))
    return
  }

  emit('sendDraftRequestToComparer', {
    request,
    label: t('trafficAnalysis.intruder.labels.request'),
  })
  dialog.toast.success(t('trafficAnalysis.history.messages.sentToComparer'))
}

function updateTargetUrl(workspaceId: string, value: string) {
  const workspace = findWorkspace(workspaceId)
  if (!workspace) return

  const target = parseTargetUrl(value)
  if (!target) return

  workspace.target = target
  workspace.name = target.host
}

function updateAttackType(workspaceId: string, value: IntruderAttackType) {
  const workspace = findWorkspace(workspaceId)
  if (!workspace) return

  workspace.attackType = value
  syncPayloadSets(workspaceId, true)
}

function updateAttackOptions(workspaceId: string, value: IntruderAttackOptions) {
  const workspace = findWorkspace(workspaceId)
  if (!workspace) return
  workspace.attackOptions = value
}

function updateAttackOption<K extends keyof IntruderAttackOptions>(
  workspaceId: string,
  key: K,
  value: IntruderAttackOptions[K],
) {
  const workspace = findWorkspace(workspaceId)
  if (!workspace) return

  workspace.attackOptions = {
    ...workspace.attackOptions,
    [key]: value,
  }
}

function updatePayloadSet(workspaceId: string, payloadSetId: string, patch: Partial<IntruderPayloadSet>) {
  const workspace = findWorkspace(workspaceId)
  if (!workspace) return

  workspace.payloadSets = workspace.payloadSets.map((payloadSet) =>
    payloadSet.id === payloadSetId ? { ...payloadSet, ...patch } : payloadSet,
  )
}

async function resolvePayloadSetValues(workspace: IntruderWorkspace, payloadSet: IntruderPayloadSet): Promise<string[]> {
  if (payloadSet.payloadType !== 'extensionGenerated') {
    return []
  }

  const payloads = await generateIntruderPluginPayloads({
    requestText: workspace.requestText,
    target: workspace.target,
    positions: workspace.positions,
    payloadSet,
    maxRequests: workspace.attackOptions.maxRequests,
  })

  updatePayloadSet(workspace.id, payloadSet.id, {
    payloadsText: payloads.join('\n'),
  })

  return payloads
}

async function applyPayloadProcessorPlugins(
  workspace: IntruderWorkspace,
  payload: string,
  options: {
    originalPayload: string
    baseValue: string
    positionIndex: number
  },
): Promise<string | null> {
  let currentPayload: string | null = payload

  for (const binding of workspace.payloadProcessorPlugins) {
    if (!binding.enabled) continue
    if (!binding.pluginId.trim()) continue
    if (currentPayload == null) return null

    currentPayload = await processIntruderPayloadWithPlugin({
      requestText: workspace.requestText,
      target: workspace.target,
      positions: workspace.positions,
      binding,
      payload: currentPayload,
      originalPayload: options.originalPayload,
      baseValue: options.baseValue,
      positionIndex: options.positionIndex,
    })
  }

  return currentPayload
}

async function applyRequestProcessorPlugins(
  workspace: IntruderWorkspace,
  requestText: string,
  payloadValues: string[],
  payloadSummary: string,
  requestIndex: number,
): Promise<{
  requestText: string
  traces: IntruderRequestProcessorTrace[]
}> {
  let currentRequest = requestText
  const traces: IntruderRequestProcessorTrace[] = []

  for (const binding of workspace.requestProcessorPlugins) {
    if (!binding.enabled) continue
    if (!binding.pluginId.trim()) continue

    const result = await transformIntruderRequestWithPlugin({
      requestText: currentRequest,
      target: workspace.target,
      positions: workspace.positions,
      binding,
      payloadValues,
      payloadSummary,
      requestIndex,
    })

    currentRequest = result.requestText
    traces.push(result)
  }

  return {
    requestText: currentRequest,
    traces,
  }
}

async function buildRequestProcessingPreview(workspace: IntruderWorkspace): Promise<{
  originalRequestText: string
  finalRequestText: string
  payloadSummary: string
  traces: IntruderRequestProcessorTrace[]
}> {
  if (workspace.positions.length === 0) {
    const originalRequestText = clearIntruderMarkers(workspace.requestText)
    const transformed = await applyRequestProcessorPlugins(
      workspace,
      originalRequestText,
      [],
      t('trafficAnalysis.intruder.labels.baseline'),
      0,
    )

    return {
      originalRequestText,
      finalRequestText: applyIntruderRequestSettings(transformed.requestText, workspace.target, workspace.attackOptions),
      payloadSummary: t('trafficAnalysis.intruder.labels.baseline'),
      traces: transformed.traces,
    }
  }

  const plan = await buildIntruderAttackPlan({
    template: workspace.requestText,
    attackType: workspace.attackType,
    payloadSets: workspace.payloadSets,
    payloadProcessingRules: workspace.payloadProcessingRules,
    maxRequests: 1,
    payloadResolver: async (payloadSet) => resolvePayloadSetValues(workspace, payloadSet),
    payloadPluginProcessor: async (payload, context) =>
      applyPayloadProcessorPlugins(workspace, payload, {
        originalPayload: context.originalPayload,
        baseValue: context.baseValue,
        positionIndex: context.positionIndex,
      }),
  })

  const candidate = plan.requests[0]
  if (!candidate) {
    throw new Error(t('trafficAnalysis.intruder.messages.noPayloads'))
  }

  const transformed = await applyRequestProcessorPlugins(
    workspace,
    candidate.requestText,
    candidate.payloadValues,
    candidate.payloadSummary,
    0,
  )

  return {
    originalRequestText: candidate.requestText,
    finalRequestText: applyIntruderRequestSettings(transformed.requestText, workspace.target, workspace.attackOptions),
    payloadSummary: candidate.payloadSummary,
    traces: transformed.traces,
  }
}

function updatePayloadProcessingRules(workspaceId: string, rules: IntruderPayloadProcessingRule[]) {
  const workspace = findWorkspace(workspaceId)
  if (!workspace) return
  workspace.payloadProcessingRules = rules
}

function updatePayloadProcessorPlugins(workspaceId: string, bindings: IntruderPluginProcessorBinding[]) {
  const workspace = findWorkspace(workspaceId)
  if (!workspace) return
  workspace.payloadProcessorPlugins = bindings
}

function updateRequestProcessorPlugins(workspaceId: string, bindings: IntruderPluginProcessorBinding[]) {
  const workspace = findWorkspace(workspaceId)
  if (!workspace) return
  workspace.requestProcessorPlugins = bindings
}

async function previewRequestProcessing() {
  const workspace = currentWorkspace.value
  if (!workspace) return

  requestProcessingPreviewLoading.value = true
  requestProcessingPreviewError.value = ''
  requestProcessingPreviewOriginal.value = ''
  requestProcessingPreviewFinal.value = ''
  requestProcessingPreviewPayloadSummary.value = ''
  requestProcessingPreviewTraces.value = []

  try {
    const preview = await buildRequestProcessingPreview(workspace)
    requestProcessingPreviewOriginal.value = preview.originalRequestText
    requestProcessingPreviewFinal.value = preview.finalRequestText
    requestProcessingPreviewPayloadSummary.value = preview.payloadSummary
    requestProcessingPreviewTraces.value = preview.traces
  } catch (error) {
    requestProcessingPreviewError.value = error instanceof Error
      ? error.message
      : t('trafficAnalysis.intruder.messages.requestPreviewFailed')
  } finally {
    requestProcessingPreviewLoading.value = false
  }
}

function updateGrepMatchRules(workspaceId: string, rules: IntruderGrepMatchRule[]) {
  const workspace = findWorkspace(workspaceId)
  if (!workspace) return
  workspace.grepMatchRules = rules
  workspace.visibleColumns = normalizeVisibleColumns(
    workspace.visibleColumns,
    workspace.grepMatchRules,
    workspace.grepExtractRules,
    workspace.grepPayloadSettings,
  )
}

function updateGrepExtractRules(workspaceId: string, rules: IntruderGrepExtractRule[]) {
  const workspace = findWorkspace(workspaceId)
  if (!workspace) return
  workspace.grepExtractRules = rules
  workspace.visibleColumns = normalizeVisibleColumns(
    workspace.visibleColumns,
    workspace.grepMatchRules,
    workspace.grepExtractRules,
    workspace.grepPayloadSettings,
  )
}

function updateGrepPayloadSettings(workspaceId: string, value: IntruderGrepPayloadSettings) {
  const workspace = findWorkspace(workspaceId)
  if (!workspace) return
  workspace.grepPayloadSettings = normalizeGrepPayloadSettings(value)
  workspace.visibleColumns = normalizeVisibleColumns(
    workspace.visibleColumns,
    workspace.grepMatchRules,
    workspace.grepExtractRules,
    workspace.grepPayloadSettings,
  )
}

function selectResourcePoolPreset(workspaceId: string, presetId: string) {
  const workspace = findWorkspace(workspaceId)
  if (!workspace) return

  const preset = resourcePools.value.find((item) => item.id === presetId)
  if (!preset) return

  workspace.selectedResourcePoolId = presetId
  workspace.attackOptions = {
    ...workspace.attackOptions,
    concurrency: preset.concurrencyEnabled ? preset.concurrency : workspace.attackOptions.concurrency,
    delayMs: preset.delayEnabled ? preset.delayMs : 0,
    randomDelayMs: preset.delayEnabled && preset.randomDelayEnabled ? preset.randomDelayMs : 0,
    delayIncrementMs: preset.delayEnabled && preset.delayIncrementEnabled ? preset.delayIncrementMs : 0,
    autoThrottleEnabled: preset.autoThrottleEnabled,
    autoThrottleStatusCodes: [...preset.autoThrottleStatusCodes],
  }
}

function upsertResourcePool(
  workspaceId: string,
  value: {
    id?: string
    name: string
    concurrencyEnabled: boolean
    concurrency: number
    delayEnabled: boolean
    delayMs: number
    randomDelayEnabled: boolean
    randomDelayMs: number
    delayIncrementEnabled: boolean
    delayIncrementMs: number
    autoThrottleEnabled: boolean
    autoThrottleStatusCodes: number[]
  },
) {
  const workspace = findWorkspace(workspaceId)
  if (!workspace) return

  const name = value.name.trim()
  if (!name) return

  const nextPool = createIntruderResourcePool(name, {
    concurrencyEnabled: value.concurrencyEnabled,
    concurrency: value.concurrency,
    delayEnabled: value.delayEnabled,
    delayMs: value.delayMs,
    randomDelayEnabled: value.randomDelayEnabled,
    randomDelayMs: value.randomDelayMs,
    delayIncrementEnabled: value.delayIncrementEnabled,
    delayIncrementMs: value.delayIncrementMs,
    autoThrottleEnabled: value.autoThrottleEnabled,
    autoThrottleStatusCodes: value.autoThrottleStatusCodes,
  })
  if (value.id) {
    const currentPool = resourcePools.value.find((pool) => pool.id === value.id)
    if (currentPool?.builtIn) {
      return
    }
    nextPool.id = value.id
  }

  resourcePools.value = [
    ...resourcePools.value.filter((pool) => pool.id !== nextPool.id),
    nextPool,
  ]
  selectResourcePoolPreset(workspaceId, nextPool.id)
}

function deleteResourcePool(workspaceId: string, resourcePoolId: string) {
  const workspace = findWorkspace(workspaceId)
  if (!workspace) return

  const currentPool = resourcePools.value.find((pool) => pool.id === resourcePoolId)
  if (!currentPool || currentPool.builtIn) return

  resourcePools.value = resourcePools.value.filter((pool) => pool.id !== currentPool.id)
  workspaces.value.forEach((item) => {
    if (item.selectedResourcePoolId === resourcePoolId) {
      item.selectedResourcePoolId = 'default'
      const defaultPool = resourcePools.value.find((pool) => pool.id === 'default')
      if (defaultPool) {
        item.attackOptions = {
          ...item.attackOptions,
          concurrency: defaultPool.concurrencyEnabled ? defaultPool.concurrency : item.attackOptions.concurrency,
          delayMs: defaultPool.delayEnabled ? defaultPool.delayMs : 0,
          randomDelayMs: defaultPool.delayEnabled && defaultPool.randomDelayEnabled ? defaultPool.randomDelayMs : 0,
          delayIncrementMs: defaultPool.delayEnabled && defaultPool.delayIncrementEnabled ? defaultPool.delayIncrementMs : 0,
          autoThrottleEnabled: defaultPool.autoThrottleEnabled,
          autoThrottleStatusCodes: [...defaultPool.autoThrottleStatusCodes],
        }
      }
    }
  })
  selectResourcePoolPreset(workspaceId, 'default')
}

function updateCaptureFilter(workspaceId: string, value: IntruderResultFilter) {
  const workspace = findWorkspace(workspaceId)
  if (!workspace) return
  workspace.captureFilter = normalizeIntruderResultFilter(value)
}

function updateViewFilter(workspaceId: string, value: IntruderResultFilter) {
  const workspace = findWorkspace(workspaceId)
  if (!workspace) return
  workspace.viewFilter = normalizeIntruderResultFilter(value)
}

function updateResultSort(workspaceId: string, value: IntruderResultSort) {
  const workspace = findWorkspace(workspaceId)
  if (!workspace) return
  workspace.sort = value
}

function updateVisibleColumns(workspaceId: string, value: string[]) {
  const workspace = findWorkspace(workspaceId)
  if (!workspace) return
  workspace.visibleColumns = normalizeVisibleColumns(
    value,
    workspace.grepMatchRules,
    workspace.grepExtractRules,
    workspace.grepPayloadSettings,
  )
}

function autoMarkPositions(workspaceId: string) {
  const workspace = findWorkspace(workspaceId)
  if (!workspace) return

  workspace.requestText = autoMarkIntruderPositions(workspace.requestText)
  applyDerivedWorkspaceState(workspace)
}

function clearMarkers(workspaceId: string) {
  const workspace = findWorkspace(workspaceId)
  if (!workspace) return

  workspace.requestText = clearIntruderMarkers(workspace.requestText)
  applyDerivedWorkspaceState(workspace)
}

function selectResult(workspaceId: string, resultId: string) {
  const workspace = findWorkspace(workspaceId)
  if (!workspace) return
  workspace.selectedResultId = resultId
}

function sendSelectedResultToRepeater(workspaceId: string, resultId?: string) {
  const workspace = findWorkspace(workspaceId)
  const targetResultId = resultId || workspace?.selectedResultId
  if (!workspace || !targetResultId) return

  const result = workspace.results.find((item) => item.id === targetResultId)
  if (!result?.rawRequest) {
    dialog.toast.warning(t('trafficAnalysis.intruder.messages.noStoredRequest'))
    return
  }

  const request = buildSourceRequestFromRawRequest(result.rawRequest, workspace.target)
  if (!request) {
    dialog.toast.warning(t('trafficAnalysis.intruder.messages.invalidStoredRequest'))
    return
  }

  emit('sendToRepeater', request)
  queueRepeaterTransfer(request)
  dialog.toast.success(t('trafficAnalysis.intruder.messages.sentToRepeater'))
}

function sendSelectedResultToComparer(workspaceId: string, resultId?: string) {
  const workspace = findWorkspace(workspaceId)
  const targetResultId = resultId || workspace?.selectedResultId
  if (!workspace || !targetResultId) return

  const result = workspace.results.find((item) => item.id === targetResultId)
  const baseline = workspace.results.find((item) => item.isBaseline)
  if (!result || !baseline || result.id === baseline.id) {
    dialog.toast.warning(t('trafficAnalysis.intruder.messages.noBaselineComparison'))
    return
  }

  const protocol: 'http' | 'https' = workspace.target.useTls ? 'https' : 'http'
  const payload = {
    name: `${workspace.name} #${result.index}`,
    leftLabel: t('trafficAnalysis.intruder.labels.baseline'),
    rightLabel: `#${result.index}`,
    leftText: baseline.rawResponse || baseline.error || '',
    rightText: result.rawResponse || result.error || '',
    compareMeta: {
      source: 'intruder' as const,
      kind: 'responseDiff' as const,
    },
    leftMeta: {
      messageType: 'response' as const,
      protocol,
    },
    rightMeta: {
      messageType: 'response' as const,
      protocol,
    },
  }

  emit('sendToComparer', payload)
  queueComparerTransfer(payload)
  dialog.toast.success(t('trafficAnalysis.intruder.messages.sentToComparer'))
}

function saveAttackTemplate(name: string) {
  const workspace = currentWorkspace.value
  if (!workspace) return

  const template = createAttackTemplateFromWorkspace(workspace, name.trim())
  attackTemplates.value = [...attackTemplates.value, template]
  selectedAttackTemplateId.value = template.id
  dialog.toast.success(t('trafficAnalysis.intruder.messages.attackTemplateSaved'))
}

function overwriteAttackTemplate(payload: { id: string; name: string }) {
  const workspace = currentWorkspace.value
  if (!workspace) return

  attackTemplates.value = attackTemplates.value.map((template) =>
    template.id === payload.id
      ? {
        ...createAttackTemplateFromWorkspace(workspace, payload.name.trim()),
        id: template.id,
      }
      : template,
  )
  selectedAttackTemplateId.value = payload.id
  dialog.toast.success(t('trafficAnalysis.intruder.messages.attackTemplateSaved'))
}

function renameAttackTemplate(payload: { id: string; name: string }) {
  attackTemplates.value = attackTemplates.value.map((template) =>
    template.id === payload.id
      ? { ...template, name: payload.name.trim() }
      : template,
  )
  selectedAttackTemplateId.value = payload.id
  dialog.toast.success(t('trafficAnalysis.intruder.messages.attackTemplateSaved'))
}

function loadAttackTemplate(templateId = selectedAttackTemplateId.value) {
  const workspace = currentWorkspace.value
  if (!workspace || !templateId) return

  const template = attackTemplates.value.find((item) => item.id === templateId)
  if (!template) return

  workspace.name = template.name
  workspace.requestText = template.requestText
  workspace.target = template.target
  workspace.attackType = template.attackType
  workspace.payloadSets = template.payloadSets.map((item, index) => ({ ...createDefaultPayloadSet(index), ...item }))
  workspace.payloadProcessingRules = template.payloadProcessingRules.map((item) => ({ ...item }))
  workspace.payloadProcessorPlugins = (template.payloadProcessorPlugins || []).map((item) => ({ ...createDefaultPluginProcessorBinding(), ...item }))
  workspace.requestProcessorPlugins = (template.requestProcessorPlugins || []).map((item) => ({ ...createDefaultPluginProcessorBinding(), ...item }))
  workspace.grepMatchRules = template.grepMatchRules.map((item) => normalizeGrepMatchRule(item))
  workspace.grepExtractRules = template.grepExtractRules.map((item) => ({ ...item }))
  workspace.grepPayloadSettings = normalizeGrepPayloadSettings(template.grepPayloadSettings)
  workspace.selectedResourcePoolId = template.selectedResourcePoolId
  workspace.attackOptions = normalizeAttackOptions(template.attackOptions)
  workspace.captureFilter = normalizeIntruderResultFilter(template.captureFilter)
  workspace.viewFilter = normalizeIntruderResultFilter(template.viewFilter)
  workspace.sort = { ...template.sort }
  workspace.visibleColumns = normalizeVisibleColumns(
    template.visibleColumns,
    workspace.grepMatchRules,
    workspace.grepExtractRules,
    workspace.grepPayloadSettings,
  )
  workspace.results = []
  workspace.selectedResultId = null
  workspace.progress = createDefaultProgress()
  workspace.isRunning = false
  applyDerivedWorkspaceState(workspace)
  dialog.toast.success(t('trafficAnalysis.intruder.messages.attackTemplateLoaded'))
}

function deleteAttackTemplate(templateId = selectedAttackTemplateId.value) {
  if (!templateId) return
  attackTemplates.value = attackTemplates.value.filter((item) => item.id !== templateId)
  if (selectedAttackTemplateId.value === templateId) {
    selectedAttackTemplateId.value = ''
  }
}

async function exportResults() {
  const workspace = currentWorkspace.value
  if (!workspace?.results.length) return

  const filteredResults = workspace.results.filter((result) => matchesIntruderResultFilter(result, workspace.viewFilter))
  const sortedResults = sortIntruderResults(filteredResults, workspace.sort)
  const csv = exportIntruderResultsCsv(
    sortedResults,
    workspace.grepMatchRules,
    workspace.grepExtractRules,
    workspace.grepPayloadSettings,
    workspace.visibleColumns,
  )
  const path = await save({
    title: t('trafficAnalysis.intruder.actions.exportResults'),
    defaultPath: `${workspace.name || 'intruder-results'}.csv`,
    filters: [{ name: 'CSV', extensions: ['csv'] }],
  })

  if (!path) return
  await writeTextFile(path, csv)
  dialog.toast.success(t('trafficAnalysis.intruder.messages.resultsExported'))
}

function clearResults() {
  const workspace = currentWorkspace.value
  if (!workspace) return

  workspace.results = []
  workspace.selectedResultId = null
  workspace.progress = createDefaultProgress()
}

function stopAttack() {
  const workspace = currentWorkspace.value
  if (!workspace) return

  const controller = attackControllers.get(workspace.id)
  if (controller) {
    controller.cancelled = true
    dialog.toast.info(t('trafficAnalysis.intruder.messages.attackStopping'))
  }
}

async function closeResultsWindow(workspaceId: string) {
  const existingWindow = await WebviewWindow.getByLabel(buildResultsWindowLabel(workspaceId))
  if (!existingWindow) return

  try {
    await existingWindow.close()
  } catch (error) {
    console.warn('Failed to close intruder results window', error)
  }
}

async function openResultsView() {
  const workspace = currentWorkspace.value
  if (!workspace) return

  syncResultsWindowState(workspace)

  const label = buildResultsWindowLabel(workspace.id)
  const title = `${workspace.name} - ${t('trafficAnalysis.intruder.sections.results')}`

  try {
    const existingWindow = await WebviewWindow.getByLabel(label)
    if (existingWindow) {
      await existingWindow.show()
      await existingWindow.setFocus()
      return
    }

    const resultsWindow = new WebviewWindow(label, {
      url: buildIntruderResultsWindowUrl(workspace.id),
      title,
      width: 1440,
      height: 920,
      center: true,
      resizable: true,
    })

    await new Promise<void>((resolve, reject) => {
      void resultsWindow.once('tauri://created', async () => {
        await resultsWindow.setFocus()
        resolve()
      })
      void resultsWindow.once('tauri://error', (event) => {
        reject(new Error(String(event.payload ?? t('trafficAnalysis.intruder.messages.resultsWindowOpenFailed'))))
      })
    })
  } catch (error) {
    console.error('Failed to open intruder results window', error)
    showResultsDialog.value = true
    dialog.toast.warning(t('trafficAnalysis.intruder.messages.resultsWindowFallback'))
  }
}

function parseReplayError(error: unknown): string {
  if (!error) return t('trafficAnalysis.intruder.messages.unknownError')

  const normalized = String(error).toLowerCase()
  if (normalized.includes('timeout')) return t('trafficAnalysis.intruder.messages.timeout')
  if (normalized.includes('connection refused') || normalized.includes('econnrefused')) {
    return t('trafficAnalysis.intruder.messages.connectionRefused')
  }
  if (normalized.includes('network')) return t('trafficAnalysis.intruder.messages.networkError')
  return String(error)
}

function shouldAutoPause(workspace: IntruderWorkspace, result: IntruderAttackResult): boolean {
  if (!workspace.attackOptions.autoPauseEnabled) {
    return false
  }

  const expressions = workspace.attackOptions.autoPauseExpressions.length
    ? workspace.attackOptions.autoPauseExpressions
    : workspace.attackOptions.autoPauseExpression.trim()
      ? [workspace.attackOptions.autoPauseExpression.trim()]
      : []
  if (!expressions.length) {
    return false
  }

  const haystack = result.rawResponse || result.error || ''
  return expressions.some((expression) => {
    const hasExpression = haystack.includes(expression)
    return workspace.attackOptions.autoPauseMode === 'contains' ? hasExpression : !hasExpression
  })
}

async function wait(ms: number) {
  if (ms <= 0) return
  await new Promise((resolve) => window.setTimeout(resolve, ms))
}

function getDelayMs(options: IntruderAttackOptions): number {
  return options.delayMs
}

function getRuntimeDelayMs(
  options: IntruderAttackOptions,
  completedCount: number,
  throttlePenaltyMs: number,
): number {
  let delay = getDelayMs(options)

  if (options.delayIncrementMs > 0) {
    delay += completedCount * options.delayIncrementMs
  } else if (options.randomDelayMs > 0) {
    delay += Math.floor(Math.random() * (options.randomDelayMs + 1))
  }

  return delay + throttlePenaltyMs
}

function getAutoThrottleStepMs(options: IntruderAttackOptions): number {
  return Math.max(options.delayIncrementMs, options.delayMs > 0 ? Math.ceil(options.delayMs / 2) : 0, 200)
}

function shouldThrottleForStatus(options: IntruderAttackOptions, statusCode: number | null): boolean {
  if (!options.autoThrottleEnabled || statusCode == null) return false
  return options.autoThrottleStatusCodes.includes(statusCode)
}

async function executeAttackRequest(
  workspace: IntruderWorkspace,
  requestText: string,
  payloadSummary: string,
  payloadValues: string[],
  index: number,
): Promise<IntruderAttackResult> {
  const transformedRequest = await applyRequestProcessorPlugins(
    workspace,
    requestText,
    payloadValues,
    payloadSummary,
    index,
  )
  const preparedRequest = applyIntruderRequestSettings(
    transformedRequest.requestText,
    workspace.target,
    workspace.attackOptions,
  )

  let attempt = 0
  while (true) {
    try {
      const exchangeRequest = buildSourceRequestFromRawRequest(preparedRequest, workspace.target)
      if (!exchangeRequest) {
        throw new Error('Invalid request')
      }

      const response = await invoke<ReplayCommandResponse<RawReplayCommandResult>>('replay_raw_request', {
        endpoint: exchangeRequest.endpoint,
        request: exchangeRequest.request,
        timeoutSecs: workspace.attackOptions.timeoutSecs,
        followRedirects: workspace.attackOptions.followRedirects,
        maxRedirects: workspace.attackOptions.maxRedirects,
        processCookiesInRedirects: workspace.attackOptions.processCookiesInRedirects,
      })

      if (!response.success || !response.data) {
        throw new Error(parseReplayError(response.error))
      }

      const responseText = response.data.body_text || ''
      const payloadReflectionCount = evaluateIntruderPayloadReflections(
        response.data.raw_response,
        payloadValues,
        workspace.grepPayloadSettings,
      )
      const grepMatches = evaluateIntruderGrepMatches(response.data.raw_response, workspace.grepMatchRules)
      const grepExtracts = evaluateIntruderGrepExtracts(response.data.raw_response, workspace.grepExtractRules)

      return {
        id: createIntruderId('attack-result'),
        index,
        payloadSummary: workspace.attackOptions.storeFullPayloads ? payloadSummary : payloadValues.join(', '),
        payloadValues,
        statusCode: response.data.status_code ?? null,
        responseLength: response.data.raw_response.length,
        wordCount: countWords(responseText),
        lineCount: countLines(responseText),
        responseTimeMs: response.data.response_time_ms,
        rawRequest: workspace.attackOptions.storeRequests ? preparedRequest : '',
        rawResponse: workspace.attackOptions.storeResponses ? response.data.raw_response : '',
        redirectCount: response.data.redirect_chain?.length ?? 0,
        finalUrl: response.data.final_url || buildTargetUrl(workspace.target),
        redirectChain: (response.data.redirect_chain || []).map((hop) => ({
          url: hop.url,
          statusCode: hop.status_code,
          location: hop.location ?? null,
          setCookieCount: hop.set_cookie_count,
        })),
        payloadReflectionCount,
        grepMatches,
        grepExtracts,
      }
    } catch (error) {
      if (attempt >= workspace.attackOptions.retryCount) {
        throw error
      }

      attempt += 1
      await wait(workspace.attackOptions.retryPauseMs)
    }
  }
}

async function startAttack() {
  const workspace = currentWorkspace.value
  if (!workspace || workspace.isRunning) return

  if (!workspace.target.host.trim()) {
    dialog.toast.warning(t('trafficAnalysis.intruder.messages.fillTarget'))
    return
  }

  if (!workspace.positions.length) {
    dialog.toast.warning(t('trafficAnalysis.intruder.messages.noPositions'))
    return
  }

  syncPayloadSets(workspace.id, false)

  let plan: Awaited<ReturnType<typeof buildIntruderAttackPlan>>
  try {
    plan = await buildIntruderAttackPlan({
      template: workspace.requestText,
      attackType: workspace.attackType,
      payloadSets: workspace.payloadSets,
      payloadProcessingRules: workspace.payloadProcessingRules,
      maxRequests: workspace.attackOptions.maxRequests,
      payloadResolver: async (payloadSet) => resolvePayloadSetValues(workspace, payloadSet),
      payloadPluginProcessor: async (payload, context) =>
        applyPayloadProcessorPlugins(workspace, payload, {
          originalPayload: context.originalPayload,
          baseValue: context.baseValue,
          positionIndex: context.positionIndex,
        }),
    })
  } catch (error) {
    const message = error instanceof Error ? error.message : t('trafficAnalysis.intruder.messages.pluginPayloadGenerationFailed')
    dialog.toast.error(message)
    return
  }

  let requests = [...plan.requests]
  if (workspace.attackOptions.makeUnmodifiedBaseline) {
    requests = [
      {
        requestText: clearIntruderMarkers(workspace.requestText),
        payloadValues: [],
        payloadSummary: t('trafficAnalysis.intruder.labels.baseline'),
        isBaseline: true,
      },
      ...requests,
    ]
  }

  if (!requests.length) {
    dialog.toast.warning(t('trafficAnalysis.intruder.messages.noPayloads'))
    return
  }

  if (requests.length > workspace.attackOptions.maxRequests) {
    requests = requests.slice(0, workspace.attackOptions.maxRequests)
  }

  workspace.results = []
  workspace.selectedResultId = null
  workspace.progress = {
    total: requests.length,
    completed: 0,
    failed: 0,
    active: 0,
    truncated: plan.truncated || requests.length < estimatedRequests.value,
  }
  workspace.isRunning = true
  await openResultsView()

  const controller = { cancelled: false }
  attackControllers.set(workspace.id, controller)
  const pacingState = { throttlePenaltyMs: 0 }

  let nextIndex = 0
  const workerCount = Math.min(workspace.attackOptions.concurrency, requests.length)

  const workers = Array.from({ length: workerCount }, async () => {
    while (!controller.cancelled) {
      const currentIndex = nextIndex
      nextIndex += 1

      if (currentIndex >= requests.length) return

      const liveWorkspace = findWorkspace(workspace.id)
      if (!liveWorkspace) return

      liveWorkspace.progress.active += 1
      const candidate = requests[currentIndex]

      try {
        const result = await executeAttackRequest(
          liveWorkspace,
          candidate.requestText,
          candidate.payloadSummary,
          candidate.payloadValues,
          currentIndex + 1,
        )
        result.isBaseline = Boolean(candidate.isBaseline)

        if (controller.cancelled) return

        const updatedWorkspace = findWorkspace(workspace.id)
        if (!updatedWorkspace) return

        if (!updatedWorkspace.attackOptions.denialOfServiceMode) {
          if (matchesIntruderResultFilter(result, updatedWorkspace.captureFilter)) {
            updatedWorkspace.results.push(result)
            updatedWorkspace.results.sort((left, right) => left.index - right.index)
            if (!updatedWorkspace.selectedResultId) {
              updatedWorkspace.selectedResultId = result.id
            }
          }
        }

        if (shouldAutoPause(updatedWorkspace, result)) {
          controller.cancelled = true
          dialog.toast.warning(t('trafficAnalysis.intruder.messages.attackAutoPaused'))
        }

        const throttleStepMs = getAutoThrottleStepMs(updatedWorkspace.attackOptions)
        if (shouldThrottleForStatus(updatedWorkspace.attackOptions, result.statusCode)) {
          pacingState.throttlePenaltyMs = Math.min(10000, pacingState.throttlePenaltyMs + throttleStepMs)
        } else {
          pacingState.throttlePenaltyMs = Math.max(0, pacingState.throttlePenaltyMs - throttleStepMs)
        }
      } catch (error) {
        if (controller.cancelled) return

        const updatedWorkspace = findWorkspace(workspace.id)
        if (!updatedWorkspace) return

        updatedWorkspace.progress.failed += 1
        if (!updatedWorkspace.attackOptions.denialOfServiceMode) {
          const errorResult: IntruderAttackResult = {
            id: createIntruderId('attack-result'),
            index: currentIndex + 1,
            payloadSummary: candidate.payloadSummary,
            payloadValues: candidate.payloadValues,
            statusCode: null,
            responseLength: 0,
            wordCount: 0,
            lineCount: 0,
            responseTimeMs: null,
            rawRequest: updatedWorkspace.attackOptions.storeRequests ? ensureRawRequestTerminator(candidate.requestText) : '',
            rawResponse: '',
            redirectCount: 0,
            finalUrl: buildTargetUrl(updatedWorkspace.target),
            redirectChain: [],
            payloadReflectionCount: 0,
            isBaseline: Boolean(candidate.isBaseline),
            grepMatches: {},
            grepExtracts: {},
            error: parseReplayError(error),
          }
          if (matchesIntruderResultFilter(errorResult, updatedWorkspace.captureFilter)) {
            updatedWorkspace.results.push(errorResult)
            updatedWorkspace.results.sort((left, right) => left.index - right.index)
          }
        }
      } finally {
        const updatedWorkspace = findWorkspace(workspace.id)
        if (!updatedWorkspace) return

        updatedWorkspace.progress.active = Math.max(0, updatedWorkspace.progress.active - 1)
        updatedWorkspace.progress.completed += 1

        if (!controller.cancelled) {
          await wait(getRuntimeDelayMs(
            updatedWorkspace.attackOptions,
            updatedWorkspace.progress.completed - 1,
            pacingState.throttlePenaltyMs,
          ))
        }
      }
    }
  })

  try {
    await Promise.all(workers)
  } finally {
    const updatedWorkspace = findWorkspace(workspace.id)
    if (updatedWorkspace) {
      updatedWorkspace.isRunning = false
      updatedWorkspace.progress.active = 0
    }
    attackControllers.delete(workspace.id)
  }

  if (controller.cancelled) {
    dialog.toast.info(t('trafficAnalysis.intruder.messages.attackStopped'))
    return
  }

  dialog.toast.success(t('trafficAnalysis.intruder.messages.attackFinished', { count: workspace.results.length }))
}

function handleResultsWindowStorage(event: StorageEvent) {
  if (!event.key) return

  const workspace = workspaces.value.find((item) => getIntruderResultsStorageKey(item.id) === event.key)
  if (!workspace) return

  const persistedState = loadIntruderResultsWindowState(workspace.id)
  if (!persistedState) return

  workspace.selectedResultId = persistedState.selectedResultId
  workspace.captureFilter = normalizeIntruderResultFilter(persistedState.captureFilter)
  workspace.viewFilter = normalizeIntruderResultFilter(persistedState.viewFilter)
  workspace.sort = persistedState.sort
  workspace.visibleColumns = normalizeVisibleColumns(
    persistedState.visibleColumns,
    persistedState.grepMatchRules || [],
    persistedState.grepExtractRules || [],
    normalizeGrepPayloadSettings(persistedState.grepPayloadSettings),
  )
}

watch(
  () => props.initialRequest,
  (request) => {
    if (request) {
      addRequestFromHistory(request)
    }
  },
)

watch(
  workspaces,
  (items) => {
    items.forEach(syncResultsWindowState)
    persistWorkspaces()
  },
  { deep: true },
)

watch(activeWorkspaceId, () => {
  persistWorkspaces()
  requestProcessingPreviewLoading.value = false
  requestProcessingPreviewOriginal.value = ''
  requestProcessingPreviewFinal.value = ''
  requestProcessingPreviewPayloadSummary.value = ''
  requestProcessingPreviewTraces.value = []
  requestProcessingPreviewError.value = ''
})

watch(
  resourcePools,
  (items) => {
    persistIntruderResourcePools(items)
  },
  { deep: true },
)

watch(
  attackTemplates,
  (items) => {
    persistIntruderAttackTemplates(items)
  },
  { deep: true },
)

onMounted(() => {
  window.addEventListener('storage', handleResultsWindowStorage)
  window.addEventListener('resize', handleWindowResize)
  sidebarWidth.value = clampSidebarWidth(sidebarWidth.value)

  if (props.initialRequest) {
    addRequestFromHistory(props.initialRequest)
  } else if (!restorePersistedWorkspaces()) {
    ensureWorkspaceSelection()
  } else {
    ensureWorkspaceSelection()
  }
})

onUnmounted(() => {
  stopSidebarResize()
  window.removeEventListener('storage', handleResultsWindowStorage)
  window.removeEventListener('resize', handleWindowResize)
})

defineExpose({
  addRequestFromHistory,
})
</script>
