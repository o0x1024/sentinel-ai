<template>
  <div class="flex h-full min-h-0 flex-col bg-base-100">
    <IntruderWorkspaceTabsBar
      :workspaces="workspaces"
      :active-workspace-id="activeWorkspaceId"
      @update:active-workspace-id="activeWorkspaceId = $event"
      @close-workspace="closeWorkspace"
      @close-other-workspaces="closeOtherWorkspaces"
      @add-workspace="addWorkspace()"
      @clear-all-workspaces="clearAllWorkspaces"
    />

    <div v-if="currentWorkspace" class="flex min-h-0 flex-1 flex-col">
      <div
        class="flex flex-wrap items-start gap-2 border-b border-base-300"
        :class="immersiveDrillModeEnabled ? 'bg-base-200/75 px-2 py-1.5 backdrop-blur-sm' : 'px-3 py-2'"
      >
        <div class="min-w-[24rem]">
          <IntruderAttackTypeSelect
            :model-value="currentWorkspace.attackType"
            variant="toolbar"
            @update:model-value="updateAttackType(currentWorkspace.id, $event)"
          />
        </div>

        <button class="btn btn-primary btn-sm min-h-8 px-2.5" type="button" :disabled="currentWorkspace.isRunning" @click="startAttack">
          <i :class="['fas', currentWorkspace.isRunning ? 'fa-spinner fa-spin' : 'fa-play']"></i>
          {{ $t('trafficAnalysis.intruder.actions.startAttack') }}
        </button>
        <button class="btn btn-sm btn-ghost min-h-8 px-2.5" type="button" :disabled="!currentWorkspace.isRunning" @click="stopAttack">
          <i class="fas fa-stop"></i>
          {{ $t('trafficAnalysis.intruder.actions.stopAttack') }}
        </button>
        <button class="btn btn-sm btn-ghost min-h-8 px-2.5" type="button" :disabled="!currentWorkspace.results.length && !currentWorkspace.isRunning" @click="openResultsView">
          <i class="fas fa-table"></i>
          {{ $t('trafficAnalysis.intruder.actions.showResults') }}
        </button>
        <button v-if="!immersiveDrillModeEnabled" class="btn btn-sm btn-ghost min-h-8 px-2.5" type="button" @click="duplicateWorkspace">
          <i class="fas fa-clone"></i>
          {{ $t('trafficAnalysis.intruder.actions.cloneTab') }}
        </button>
        <button v-if="!immersiveDrillModeEnabled" class="btn btn-sm btn-ghost min-h-8 px-2.5" type="button" :disabled="!currentWorkspace.results.length" @click="clearResults">
          <i class="fas fa-trash-alt"></i>
          {{ $t('trafficAnalysis.intruder.actions.clearResults') }}
        </button>
        <IntruderAttackTemplateManager
          v-if="!immersiveDrillModeEnabled"
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
        <button v-if="!immersiveDrillModeEnabled" class="btn btn-sm btn-ghost min-h-8 px-2.5" type="button" :disabled="!currentWorkspace.results.length" @click="exportResults">
          <i class="fas fa-file-export"></i>
          {{ $t('trafficAnalysis.intruder.actions.exportResults') }}
        </button>

        <div class="ml-auto flex flex-wrap items-center gap-1.5 text-sm">
          <div v-if="!immersiveDrillModeEnabled" :class="IMMERSIVE_TRAFFIC_COMPACT_BADGE_CLASS">{{ $t('trafficAnalysis.intruder.labels.requestCount') }}: {{ estimatedRequests }}</div>
          <div v-if="currentWorkspace.progress.completed > 0 || currentWorkspace.isRunning" :class="IMMERSIVE_TRAFFIC_COMPACT_BADGE_CLASS">
            {{ currentWorkspace.progress.completed }}/{{ currentWorkspace.progress.total }}
          </div>
          <div v-if="planWillTruncate" :class="[IMMERSIVE_TRAFFIC_COMPACT_BADGE_CLASS, 'badge-warning']">
            {{ $t('trafficAnalysis.intruder.messages.attackPlanTrimmed') }}
          </div>
        </div>
      </div>

      <div ref="workspaceLayoutRef" class="flex min-h-0 flex-1">
        <div class="min-w-0 flex-1">
          <IntruderRequestEditor
            :request-text="currentWorkspace.requestText"
            :request-view-tab="currentWorkspace.requestViewTab"
            :target-url="buildTargetUrl(currentWorkspace.target)"
            :source-request-id="currentWorkspace.sourceRequestId"
            :update-host-header="currentWorkspace.attackOptions.updateHostHeader"
            :positions="currentWorkspace.positions"
            @update:request-text="updateRequestText(currentWorkspace.id, $event)"
            @update:request-view-tab="updateRequestViewTab(currentWorkspace.id, $event)"
            @update:target-url="updateTargetUrl(currentWorkspace.id, $event)"
            @update:update-host-header="updateAttackOption(currentWorkspace.id, 'updateHostHeader', $event)"
            @auto-mark="autoMarkPositions(currentWorkspace.id)"
            @clear-markers="clearMarkers(currentWorkspace.id)"
            @create-draft="sendWorkspaceRequestToRepeater(currentWorkspace.id)"
            @open-draft-compare="sendWorkspaceRequestToComparer(currentWorkspace.id)"
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
            :attack-type="currentWorkspace.attackType"
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
            @apply-payload-template="(payloadSetId, value) => applyPayloadTemplate(currentWorkspace.id, payloadSetId, value)"
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

      <div v-if="!immersiveDrillModeEnabled" class="flex items-center gap-2.5 border-t border-base-300 bg-base-200 px-3 py-1.5 text-xs text-base-content/70">
        <span>{{ currentWorkspace.target.useTls ? 'https' : 'http' }}://{{ currentWorkspace.target.host || 'example.com' }}:{{ currentWorkspace.target.port }}</span>
        <span>{{ currentWorkspace.positions.length }} {{ $t('trafficAnalysis.intruder.labels.detectedPositions') }}</span><span v-if="currentWorkspace.isRunning">{{ $t('trafficAnalysis.intruder.labels.running') }}</span>
      </div>
    </div>

    <div v-else class="flex flex-1 items-center justify-center text-sm text-base-content/60">{{ $t('trafficAnalysis.intruder.empty.noWorkspace') }}</div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import { save } from '@tauri-apps/plugin-dialog'
import { writeTextFile } from '@tauri-apps/plugin-fs'
import { useI18n } from 'vue-i18n'
import { immersiveDrillModeEnabled } from '@/services/immersiveDrillMode'
import { dialog } from '@/composables/useDialog'
import { buildIntruderResultsWindowUrl } from '@/router/standalone'
import { IMMERSIVE_TRAFFIC_COMPACT_BADGE_CLASS } from './immersiveTrafficUi'
import IntruderAttackTypeSelect from './intruder/IntruderAttackTypeSelect.vue'
import IntruderAttackTemplateManager from './intruder/IntruderAttackTemplateManager.vue'
import IntruderRequestEditor from './intruder/IntruderRequestEditor.vue'
import IntruderSidePanel from './intruder/IntruderSidePanel.vue'
import IntruderWorkspaceTabsBar from './intruder/IntruderWorkspaceTabsBar.vue'
import { autoMarkIntruderPositions, buildIntruderAttackPlan, clearIntruderMarkers, estimateAttackCount, extractIntruderPositions, getRequiredPayloadSetCount } from './intruder/attack'
import { expandPayloadSet } from './intruder/payloads'
import { applyIntruderRequestSettings, buildSourceRequestFromRawRequest, countLines, countWords, createIntruderId, createRawRequestFromSource, ensureRawRequestTerminator, extractTargetFromRequest } from './intruder/http'
import { buildIntruderTargetUrl as buildTargetUrl, INTRUDER_SIDEBAR_WIDTH_KEY, IntruderWorkspace, clampIntruderSidebarWidth, createDefaultAttackOptions, createDefaultPayloadSet, createDefaultPluginProcessorBinding, createDefaultProgress, createIntruderWorkspace, loadIntruderSidebarWidth, normalizeAttackOptions, normalizePayloadSet, normalizePersistedIntruderWorkspaceList, parseIntruderTargetUrl, serializeIntruderWorkspaceSessionStore } from './intruder/workspaceSupport'
import { buildHttpReplayResponseFromCommandResult, type RawReplayCommandResult } from './http/response'
import type { TrafficComparePayload } from './transfers'
import { evaluateIntruderGrepExtracts, evaluateIntruderGrepMatches, evaluateIntruderPayloadReflections, normalizeGrepMatchRule, normalizeGrepPayloadSettings } from './intruder/analysis'
import { getIntruderResultsStorageKey, loadIntruderResultsWindowState, matchesIntruderResultFilter, normalizeIntruderResultFilter, saveIntruderResultsWindowState, sortIntruderResults } from './intruder/results'
import { buildIntruderResourcePoolAutoName, createBuiltInResourcePools, createIntruderAttackTemplate, exportIntruderResultsCsv, loadIntruderAttackTemplates, loadIntruderResourcePools, normalizeVisibleColumns, persistIntruderAttackTemplates, persistIntruderResourcePools, type IntruderAttackTemplate, upsertIntruderResourcePoolEntry } from './intruder/storage'
import { generateIntruderPluginPayloads, processIntruderPayloadWithPlugin, transformIntruderRequestWithPlugin, type IntruderRequestProcessorTrace } from './intruder/plugins'
import { getIntruderAutoThrottleStepMs, getIntruderRuntimeDelayMs, shouldIntruderThrottleForStatus, waitForIntruderDelay } from './intruder/runtimeSupport'
import { createDefaultIntruderDictionaryPayloadConfig, resolveIntruderDictionaryPayloads } from './intruder/intruderAppDictionaryPayloads'
import { useTrafficWorkbenchStore } from './workbench/stores/useTrafficWorkbenchStore'
import type { AttackWorkspace as WorkbenchAttackWorkspace } from './workbench/model/attackWorkspace'
import { loadIntruderWorkspaceSessionStore, saveIntruderWorkspaceSessionStore, type PersistedIntruderWorkspaceSessionStore } from '@/api/trafficIntruderSessions'
import type {
  IntruderAttackOptions,
  IntruderAttackResult,
  IntruderAttackType,
  IntruderGrepExtractRule,
  IntruderGrepMatchRule,
  IntruderGrepPayloadSettings,
  IntruderPluginProcessorBinding,
  IntruderPayloadProcessingRule,
  IntruderPayloadSet,
  IntruderRequestInput,
  IntruderRequestViewTab,
  IntruderResourcePool,
  IntruderResultFilter,
  IntruderResultSort,
} from './intruder/types'

interface ReplayCommandResponse<T> {
  success: boolean
  data?: T
  error?: string
}

const props = defineProps<{
  initialRequest?: IntruderRequestInput
  initialWorkspaceId?: string
}>()

const emit = defineEmits<{
  (e: 'createDraft', request: IntruderRequestInput): void
  (e: 'openCompare', payload: TrafficComparePayload): void
  (e: 'openDraftCompare', payload: { request: IntruderRequestInput; label?: string }): void
  (e: 'workspaceStatsChanged', stats: { openWorkspaceCount: number }): void
}>()

const { t } = useI18n()
const workbenchState = useTrafficWorkbenchStore()
const workspaces = ref<IntruderWorkspace[]>([])
const activeWorkspaceId = ref<string | null>(null)
const activeSidebarTab = ref<'payloads' | 'resourcePool' | 'settings'>('payloads')
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
const sidebarWidth = ref(loadIntruderSidebarWidth())
const isResizingSidebar = ref(false)
const sidebarResizeStartX = ref(0)
const sidebarResizeStartWidth = ref(sidebarWidth.value)
let applyingWorkbenchWorkspace = false
const intruderSessionPersistenceReady = ref(false)
const persistedIntruderSessionFingerprint = ref('')
const persistedIntruderSessionStore = ref<PersistedIntruderWorkspaceSessionStore | null>(null)
let persistIntruderSessionTimer: number | null = null
const openResultsWindowLabels = new Set<string>()
let unlistenResultsWindowClosed: UnlistenFn | null = null

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

function buildPersistedIntruderSessionStore(): PersistedIntruderWorkspaceSessionStore {
  return serializeIntruderWorkspaceSessionStore(activeWorkspaceId.value, workspaces.value)
}

async function persistIntruderSessionNow() {
  if (persistIntruderSessionTimer !== null) {
    window.clearTimeout(persistIntruderSessionTimer)
    persistIntruderSessionTimer = null
  }

  const store = buildPersistedIntruderSessionStore()
  const fingerprint = JSON.stringify(store)
  if (fingerprint === persistedIntruderSessionFingerprint.value) {
    return
  }

  await saveIntruderWorkspaceSessionStore(store)
  persistedIntruderSessionStore.value = store
  persistedIntruderSessionFingerprint.value = fingerprint
}

function scheduleIntruderSessionPersistence() {
  if (!intruderSessionPersistenceReady.value) {
    return
  }
  if (persistIntruderSessionTimer !== null) {
    window.clearTimeout(persistIntruderSessionTimer)
  }
  persistIntruderSessionTimer = window.setTimeout(() => {
    persistIntruderSessionTimer = null
    void persistIntruderSessionNow()
      .catch(error => {
        console.error('Failed to persist intruder workspace sessions', error)
      })
  }, 180)
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

function markResultsWindowOpen(label: string) {
  openResultsWindowLabels.add(label)
}

function syncResultsWindowState(workspace: IntruderWorkspace, options: { force?: boolean } = {}) {
  if (!options.force && !openResultsWindowLabels.has(buildResultsWindowLabel(workspace.id))) {
    return
  }
  saveIntruderResultsWindowState(buildResultsWindowState(workspace))
}

async function openResultsView() {
  const workspace = currentWorkspace.value
  if (!workspace) return

  syncResultsWindowState(workspace, { force: true })

  const label = buildResultsWindowLabel(workspace.id)
  const title = `${workspace.name} - ${t('trafficAnalysis.intruder.sections.results')}`

  try {
    const existingWindow = await WebviewWindow.getByLabel(label)
    if (existingWindow) {
      markResultsWindowOpen(label)
      syncResultsWindowState(workspace, { force: true })
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
        markResultsWindowOpen(label)
        await resultsWindow.setFocus()
        resolve()
      })
      void resultsWindow.once('tauri://error', (event) => {
        reject(new Error(String(event.payload ?? t('trafficAnalysis.intruder.messages.resultsWindowOpenFailed'))))
      })
    })
  } catch (error) {
    console.error('Failed to open intruder results window', error)
    dialog.toast.error(t('trafficAnalysis.intruder.messages.resultsWindowOpenFailed'))
  }
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

function findPersistedWorkspaceSession(workspaceId: string | null | undefined) {
  if (!workspaceId) {
    return null
  }
  const workspaces = persistedIntruderSessionStore.value?.workspaces
  if (!Array.isArray(workspaces)) {
    return null
  }
  return workspaces.find(workspace => workspace?.id === workspaceId) ?? null
}

function syncWorkbenchWorkspaceRuntime(
  workspace: IntruderWorkspace,
  runState: 'idle' | 'running' | 'done' | 'cancelled' = workspace.isRunning ? 'running' : 'done',
) {
  workbenchState.attack.updateWorkspaceRuntime(workspace.id, {
    runState,
    resultCount: workspace.results.length,
    progress: {
      total: workspace.progress.total,
      completed: workspace.progress.completed,
      failed: workspace.progress.failed,
      active: workspace.progress.active,
      truncated: workspace.progress.truncated,
    },
  })
}

function syncWorkbenchWorkspaceDefinition(workspace: IntruderWorkspace) {
  workbenchState.attack.updateWorkspaceRequestText(workspace.id, workspace.requestText)
  workbenchState.attack.updateWorkspaceTarget(workspace.id, workspace.target)
  workbenchState.attack.updateWorkspacePositions(workspace.id, workspace.positions)
  workbenchState.attack.updateWorkspaceTitle(workspace.id, workspace.name)
}

async function hydrateIntruderWorkspaceSessions() {
  try {
    const store = await loadIntruderWorkspaceSessionStore()
    persistedIntruderSessionStore.value = {
      activeWorkspaceId: store.activeWorkspaceId,
      workspaces: Array.isArray(store.workspaces) ? store.workspaces : [],
    }
    persistedIntruderSessionFingerprint.value = JSON.stringify(persistedIntruderSessionStore.value)
  } catch (error) {
    console.error('Failed to hydrate intruder workspace sessions', error)
    persistedIntruderSessionStore.value = {
      activeWorkspaceId: null,
      workspaces: [],
    }
    persistedIntruderSessionFingerprint.value = JSON.stringify(persistedIntruderSessionStore.value)
  } finally {
    intruderSessionPersistenceReady.value = true
  }
}

function clampSidebarWidth(width: number): number {
  const containerWidth = workspaceLayoutRef.value?.clientWidth ?? window.innerWidth
  return clampIntruderSidebarWidth(width, containerWidth)
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

function createWorkspace(
  source?: IntruderRequestInput,
  overrides?: { id?: string; name?: string },
): IntruderWorkspace {
  return createIntruderWorkspace({
    source,
    id: overrides?.id,
    name: overrides?.name,
    existingCount: workspaces.value.length,
    attackLabel: t('trafficAnalysis.intruder.labels.attack'),
    payloadLabel: t('trafficAnalysis.intruder.labels.payloadSet'),
  })
}

function findWorkbenchWorkspace(workspaceId: string | null | undefined) {
  if (!workspaceId) {
    return null
  }
  return workbenchState.attack.workspaces.value.find(workspace => workspace.id === workspaceId) ?? null
}

function syncLocalWorkspaceFromWorkbench(workspace: IntruderWorkspace, source: WorkbenchAttackWorkspace) {
  workspace.name = source.title
  workspace.sourceRequestId = source.source?.requestId ?? workspace.sourceRequestId
  workspace.requestText = source.requestText
  workspace.requestViewTab = 'raw'
  workspace.target = { ...source.target }
  workspace.positions = [...source.positions]
  workspace.progress = { ...source.progress }
  workspace.isRunning = source.runState === 'running'
  if (!workspace.isRunning && source.resultCount === 0) {
    workspace.results = []
    workspace.selectedResultId = null
  }
  syncPayloadSets(workspace.id, false)
}

function createWorkspaceFromWorkbench(source: WorkbenchAttackWorkspace): IntruderWorkspace {
  const persisted = normalizePersistedIntruderWorkspaceList(
    {
      activeWorkspaceId: source.id,
      workspaces: findPersistedWorkspaceSession(source.id) ? [findPersistedWorkspaceSession(source.id)!] : [],
    },
    {
      attackLabel: t('trafficAnalysis.intruder.labels.attack'),
      payloadLabel: t('trafficAnalysis.intruder.labels.payloadSet'),
    },
  )
  const request = buildSourceRequestFromRawRequest(source.requestText, source.target)
  const workspace = persisted?.workspaces[0] || createWorkspace(request || undefined, {
    id: source.id,
    name: source.title,
  })
  syncLocalWorkspaceFromWorkbench(workspace, source)
  return workspace
}

function openWorkspaceFromWorkbench(workspaceId: string | null | undefined) {
  const source = findWorkbenchWorkspace(workspaceId)
  if (!source) {
    return
  }

  const existing = findWorkspace(source.id)
  applyingWorkbenchWorkspace = true
  try {
    if (existing) {
      syncLocalWorkspaceFromWorkbench(existing, source)
      activeWorkspaceId.value = existing.id
      return
    }

    const workspace = createWorkspaceFromWorkbench(source)
    workspaces.value.push(workspace)
    activeWorkspaceId.value = workspace.id
  } finally {
    applyingWorkbenchWorkspace = false
  }
}

function openAllWorkspacesFromWorkbench() {
  const activeId = workbenchState.attack.activeWorkspaceId.value
  const ids = workbenchState.attack.workspaces.value.map(workspace => workspace.id)
  if (!ids.length) { attackControllers.forEach(controller => { controller.cancelled = true }); attackControllers.clear(); workspaces.value = []; activeWorkspaceId.value = null; return }
  workspaces.value = workspaces.value.filter(workspace =>
    ids.includes(workspace.id) || !workspace.id.startsWith('intruder-workspace-'),
  )
  ids.forEach(id => {
    if (id !== activeId) openWorkspaceFromWorkbench(id)
  })
  if (activeId) openWorkspaceFromWorkbench(activeId)
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
    workspace.payloadSets.push(
      createDefaultPayloadSet(workspace.payloadSets.length, t('trafficAnalysis.intruder.labels.payloadSet')),
    )
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

function addWorkspace(source?: IntruderRequestInput) {
  const request = source || buildSourceRequestFromRawRequest(
    createRawRequestFromSource(),
    extractTargetFromRequest(createRawRequestFromSource()),
  )
  if (!request) {
    return
  }

  const attackWorkspace = workbenchState.attack.createWorkspaceFromExchangeRequest({
    request,
    source: { kind: 'intruder', label: t('trafficAnalysis.tabs.intruder', '爆破器') },
    title: request.endpoint.host,
  })
  workbenchState.attack.selectWorkspace(attackWorkspace.id)
  openWorkspaceFromWorkbench(attackWorkspace.id)
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
  workspaces.value = workspaces.value.filter((workspace) => workspace.id !== workspaceId)
  workbenchState.attack.removeWorkspace(workspaceId)
  ensureWorkspaceSelection()
}

async function clearAllWorkspaces() {
  if (workspaces.value.length === 0) return

  const confirmed = await dialog.confirm({
    title: t('trafficAnalysis.intruder.messages.confirmClearAllWorkspaces'),
    message: t('trafficAnalysis.intruder.messages.confirmClearAllWorkspacesMessage', {
      count: workspaces.value.length,
    }),
    variant: 'warning',
  })
  if (!confirmed) return

  attackControllers.forEach(controller => { controller.cancelled = true })
  attackControllers.clear()
  workspaces.value = []
  activeWorkspaceId.value = null
  workbenchState.attack.resetAttackWorkspaceStore()
  resetRequestProcessingPreviewState()
}

function closeOtherWorkspaces(workspaceId: string) {
  const remainingWorkspace = workspaces.value.find(workspace => workspace.id === workspaceId)
  if (!remainingWorkspace) return

  attackControllers.forEach((controller, id) => {
    if (id !== workspaceId) controller.cancelled = true
  })
  attackControllers.forEach((_, id) => {
    if (id !== workspaceId) attackControllers.delete(id)
  })
  workspaces.value = [remainingWorkspace]
  activeWorkspaceId.value = workspaceId
  workbenchState.attack.replaceState(
    workbenchState.attack.workspaces.value.filter(workspace => workspace.id === workspaceId),
    workspaceId,
  )
  resetRequestProcessingPreviewState()
}

function duplicateWorkspace() {
  const workspace = currentWorkspace.value
  if (!workspace) return

  const copy: IntruderWorkspace = {
    ...workspace,
    id: createIntruderId('intruder-workspace'),
    name: `${workspace.name} Copy`,
    payloadSets: workspace.payloadSets.map((payloadSet, index) => ({
      ...normalizePayloadSet(index, payloadSet, t('trafficAnalysis.intruder.labels.payloadSet')),
      id: createIntruderId('payload-set'),
    })),
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
  if (!applyingWorkbenchWorkspace) {
    syncWorkbenchWorkspaceDefinition(workspace)
  }
}

function updateRequestViewTab(workspaceId: string, value: IntruderRequestViewTab) {
  const workspace = findWorkspace(workspaceId)
  if (!workspace) return

  workspace.requestViewTab = value
}

function sendWorkspaceRequestToRepeater(workspaceId: string) {
  const workspace = findWorkspace(workspaceId)
  if (!workspace) return

  const request = buildSourceRequestFromRawRequest(workspace.requestText, workspace.target)
  if (!request) {
    dialog.toast.warning(t('trafficAnalysis.repeater.messages.invalidRequestForIntruder'))
    return
  }
  request.sourceRequestId = workspace.sourceRequestId

  emit('createDraft', request)
  dialog.toast.success(t('trafficAnalysis.history.messages.draftCreated'))
}

function sendWorkspaceRequestToComparer(workspaceId: string) {
  const workspace = findWorkspace(workspaceId)
  if (!workspace) return

  const request = buildSourceRequestFromRawRequest(workspace.requestText, workspace.target)
  if (!request) {
    dialog.toast.warning(t('trafficAnalysis.repeater.messages.invalidRequestForComparer'))
    return
  }
  request.sourceRequestId = workspace.sourceRequestId

  emit('openDraftCompare', {
    request,
    label: t('trafficAnalysis.intruder.labels.request'),
  })
  dialog.toast.success(t('trafficAnalysis.history.messages.compareOpened'))
}

function updateTargetUrl(workspaceId: string, value: string) {
  const workspace = findWorkspace(workspaceId)
  if (!workspace) return

  const target = parseIntruderTargetUrl(value)
  if (!target) return

  workspace.target = target
  workspace.name = target.host
  if (!applyingWorkbenchWorkspace) {
    syncWorkbenchWorkspaceDefinition(workspace)
  }
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

  workspace.payloadSets = workspace.payloadSets.map((payloadSet, index) =>
    payloadSet.id === payloadSetId
      ? normalizePayloadSet(index, { ...payloadSet, ...patch }, t('trafficAnalysis.intruder.labels.payloadSet'))
      : payloadSet,
  )
}

function applyPayloadTemplate(
  workspaceId: string,
  payloadSetId: string,
  value: {
    sourceRef: string
    attackType: IntruderAttackType
    sets: Array<{ name: string; payloadsText: string }>
  },
) {
  const workspace = findWorkspace(workspaceId)
  if (!workspace) return

  const startIndex = Math.max(0, workspace.payloadSets.findIndex((payloadSet) => payloadSet.id === payloadSetId))
  workspace.attackType = value.attackType
  syncPayloadSets(workspaceId, false)

  const requiredLength = startIndex + value.sets.length
  while (workspace.payloadSets.length < requiredLength) {
    workspace.payloadSets.push(
      createDefaultPayloadSet(workspace.payloadSets.length, t('trafficAnalysis.intruder.labels.payloadSet')),
    )
  }

  workspace.payloadSets = workspace.payloadSets.map((payloadSet, index) => {
    const templateSet = value.sets[index - startIndex]
    if (!templateSet) {
      return normalizePayloadSet(index, payloadSet, t('trafficAnalysis.intruder.labels.payloadSet'))
    }

    return normalizePayloadSet(index, {
      ...payloadSet,
      name: templateSet.name,
      payloadType: 'simpleList',
      payloadsText: templateSet.payloadsText,
      dictionaryConfig: createDefaultIntruderDictionaryPayloadConfig(),
      pluginId: '',
      pluginPresetName: '',
      pluginConfig: '{}',
      filePath: '',
    }, t('trafficAnalysis.intruder.labels.payloadSet'))
  })
}

async function resolvePayloadSetValues(workspace: IntruderWorkspace, payloadSet: IntruderPayloadSet): Promise<string[]> {
  if (payloadSet.payloadType === 'appDictionary') {
    const payloads = await resolveIntruderDictionaryPayloads(payloadSet.dictionaryConfig)
    updatePayloadSet(workspace.id, payloadSet.id, {
      payloadsText: payloads.join('\n'),
    })
    return payloads
  }

  if (payloadSet.payloadType !== 'extensionGenerated') {
    return expandPayloadSet(payloadSet)
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

  const result = upsertIntruderResourcePoolEntry(resourcePools.value, value)
  resourcePools.value = result.pools
  selectResourcePoolPreset(workspaceId, result.pool.id)
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
  syncWorkbenchWorkspaceDefinition(workspace)
}

function clearMarkers(workspaceId: string) {
  const workspace = findWorkspace(workspaceId)
  if (!workspace) return

  workspace.requestText = clearIntruderMarkers(workspace.requestText)
  applyDerivedWorkspaceState(workspace)
  syncWorkbenchWorkspaceDefinition(workspace)
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
  request.sourceRequestId = workspace.sourceRequestId

  emit('createDraft', request)
  dialog.toast.success(t('trafficAnalysis.intruder.messages.draftCreated'))
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

  emit('openCompare', payload)
  dialog.toast.success(t('trafficAnalysis.intruder.messages.compareOpened'))
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
  workspace.payloadSets = template.payloadSets.map((item, index) =>
    normalizePayloadSet(index, item, t('trafficAnalysis.intruder.labels.payloadSet')),
  )
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
  syncWorkbenchWorkspaceDefinition(workspace)
  syncWorkbenchWorkspaceRuntime(workspace, 'idle')
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
    defaultPath: `${workspace.name || 'attack-results'}.csv`,
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
  workspace.isRunning = false
  syncWorkbenchWorkspaceRuntime(workspace, 'idle')
}

function resetRequestProcessingPreviewState() {
  requestProcessingPreviewLoading.value = false
  requestProcessingPreviewOriginal.value = ''
  requestProcessingPreviewFinal.value = ''
  requestProcessingPreviewPayloadSummary.value = ''
  requestProcessingPreviewTraces.value = []
  requestProcessingPreviewError.value = ''
}

function cancelAttack(workspaceId: string, notifyStopping: boolean) {
  const controller = attackControllers.get(workspaceId)
  if (!controller || controller.cancelled) return false

  controller.cancelled = true
  if (notifyStopping) {
    dialog.toast.info(t('trafficAnalysis.intruder.messages.attackStopping'))
  }
  return true
}

function stopAttack() {
  const workspace = currentWorkspace.value
  if (!workspace) return

  cancelAttack(workspace.id, true)
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

async function executeAttackRequest(
  workspace: IntruderWorkspace,
  requestText: string,
  payloadSummary: string,
  payloadValues: string[],
  index: number,
  controller: { cancelled: boolean },
): Promise<IntruderAttackResult> {
  if (controller.cancelled) {
    throw new Error('Attack cancelled')
  }

  const transformedRequest = await applyRequestProcessorPlugins(
    workspace,
    requestText,
    payloadValues,
    payloadSummary,
    index,
  )
  if (controller.cancelled) {
    throw new Error('Attack cancelled')
  }

  const preparedRequest = applyIntruderRequestSettings(
    transformedRequest.requestText,
    workspace.target,
    workspace.attackOptions,
  )

  let attempt = 0
  while (true) {
    try {
      if (controller.cancelled) {
        throw new Error('Attack cancelled')
      }

      const exchangeRequest = buildSourceRequestFromRawRequest(preparedRequest, workspace.target)
      if (!exchangeRequest) {
        throw new Error('Invalid request')
      }
      exchangeRequest.sourceRequestId = workspace.sourceRequestId

      if (controller.cancelled) {
        throw new Error('Attack cancelled')
      }

      const response = await invoke<ReplayCommandResponse<RawReplayCommandResult>>('replay_raw_request', {
        endpoint: exchangeRequest.endpoint,
        request: exchangeRequest.request,
        timeoutSecs: workspace.attackOptions.timeoutSecs,
        followRedirects: workspace.attackOptions.followRedirects,
        maxRedirects: workspace.attackOptions.maxRedirects,
        processCookiesInRedirects: workspace.attackOptions.processCookiesInRedirects,
        originKind: 'attack-workspace',
        originRefId: workspace.id,
        parentRequestId: workspace.sourceRequestId,
        sourceDraftRevisionId: findWorkbenchWorkspace(workspace.id)?.sourceDraftRevisionId ?? null,
      })

      if (!response.success || !response.data) {
        throw new Error(parseReplayError(response.error))
      }

      const replayResponse = buildHttpReplayResponseFromCommandResult(response.data)
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
        responseVersionObserved: replayResponse.versionObserved,
        responseStatusText: replayResponse.statusText,
        responseHeaders: replayResponse.headers,
        responseBodyText: replayResponse.bodyText,
        responseBodyBytesBase64: replayResponse.bodyBytesBase64,
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
      if (controller.cancelled) {
        throw error
      }

      if (attempt >= workspace.attackOptions.retryCount) {
        throw error
      }

      attempt += 1
      await waitForIntruderDelay(workspace.attackOptions.retryPauseMs)
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

  const resourcePoolResult = upsertIntruderResourcePoolEntry(resourcePools.value, {
    name: buildIntruderResourcePoolAutoName(resourcePools.value, workspace.selectedResourcePoolId),
    concurrencyEnabled: true,
    concurrency: workspace.attackOptions.concurrency,
    delayEnabled: workspace.attackOptions.delayMs > 0 || workspace.attackOptions.randomDelayMs > 0 || workspace.attackOptions.delayIncrementMs > 0,
    delayMs: workspace.attackOptions.delayMs,
    randomDelayEnabled: workspace.attackOptions.randomDelayMs > 0,
    randomDelayMs: workspace.attackOptions.randomDelayMs,
    delayIncrementEnabled: workspace.attackOptions.delayIncrementMs > 0,
    delayIncrementMs: workspace.attackOptions.delayIncrementMs,
    autoThrottleEnabled: workspace.attackOptions.autoThrottleEnabled,
    autoThrottleStatusCodes: workspace.attackOptions.autoThrottleStatusCodes,
  })
  resourcePools.value = resourcePoolResult.pools
  workspace.selectedResourcePoolId = resourcePoolResult.pool.id

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
  if (workspace.attackOptions.makeUnmodifiedBaseline && workspace.positions.length > 0) {
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
  syncWorkbenchWorkspaceRuntime(workspace, 'running')
  void openResultsView()

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
          controller,
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
        syncWorkbenchWorkspaceRuntime(updatedWorkspace, controller.cancelled ? 'cancelled' : 'running')

        if (shouldAutoPause(updatedWorkspace, result)) {
          controller.cancelled = true
          dialog.toast.warning(t('trafficAnalysis.intruder.messages.attackAutoPaused'))
        }

        const throttleStepMs = getIntruderAutoThrottleStepMs(updatedWorkspace.attackOptions)
        if (shouldIntruderThrottleForStatus(updatedWorkspace.attackOptions, result.statusCode)) {
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
            responseHeaders: [],
            responseBodyText: '',
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
        syncWorkbenchWorkspaceRuntime(updatedWorkspace, controller.cancelled ? 'cancelled' : 'running')
      } finally {
        const updatedWorkspace = findWorkspace(workspace.id)
        if (!updatedWorkspace) return

        updatedWorkspace.progress.active = Math.max(0, updatedWorkspace.progress.active - 1)
        updatedWorkspace.progress.completed += 1
        syncWorkbenchWorkspaceRuntime(updatedWorkspace, controller.cancelled ? 'cancelled' : 'running')

        if (!controller.cancelled) {
          await waitForIntruderDelay(getIntruderRuntimeDelayMs(
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
      syncWorkbenchWorkspaceRuntime(
        updatedWorkspace,
        controller.cancelled ? 'cancelled' : 'done',
      )
    }
    attackControllers.delete(workspace.id)
  }

  if (controller.cancelled) {
    dialog.toast.info(t('trafficAnalysis.intruder.messages.attackStopped'))
    return
  }

  dialog.toast.success(t('trafficAnalysis.intruder.messages.attackFinished', { count: workspace.results.length }))
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
  () => props.initialWorkspaceId,
  (workspaceId) => {
    if (workspaceId) {
      openWorkspaceFromWorkbench(workspaceId)
    }
  },
)

watch(
  () => workbenchState.attack.activeWorkspaceId.value,
  (workspaceId) => {
    if (workspaceId) {
      openWorkspaceFromWorkbench(workspaceId)
    }
  },
)

watch(
  () => workspaces.value.length,
  (openWorkspaceCount) => {
    emit('workspaceStatsChanged', { openWorkspaceCount })
  },
  { immediate: true },
)

watch(
  () => workbenchState.attack.workspaces.value.map(workspace => workspace.id).join('|'),
  openAllWorkspacesFromWorkbench,
  { immediate: true },
)

watch(
  workspaces,
  (items) => {
    items.forEach((workspace) => syncResultsWindowState(workspace))
    scheduleIntruderSessionPersistence()
  },
  { deep: true },
)

watch(activeWorkspaceId, () => {
  if (activeWorkspaceId.value && workbenchState.attack.activeWorkspaceId.value !== activeWorkspaceId.value) {
    workbenchState.attack.selectWorkspace(activeWorkspaceId.value)
  }
  scheduleIntruderSessionPersistence()
  resetRequestProcessingPreviewState()
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
  window.addEventListener('resize', handleWindowResize)
  window.addEventListener('storage', handleResultsWindowStorage)
  void listen<{ workspaceId?: string }>('intruder-results-window:closed', (event) => {
    if (!event.payload?.workspaceId) return
    openResultsWindowLabels.delete(buildResultsWindowLabel(event.payload.workspaceId))
  }).then((unlisten) => {
    unlistenResultsWindowClosed = unlisten
  })
  sidebarWidth.value = clampSidebarWidth(sidebarWidth.value)
  void hydrateIntruderWorkspaceSessions().finally(() => {
    if (workbenchState.attack.workspaces.value.length) {
      openAllWorkspacesFromWorkbench()
    } else if (props.initialWorkspaceId) {
      openWorkspaceFromWorkbench(props.initialWorkspaceId)
    } else if (props.initialRequest) {
      addRequestFromHistory(props.initialRequest)
    } else {
      ensureWorkspaceSelection()
    }
  })
})

onUnmounted(() => {
  if (persistIntruderSessionTimer !== null) {
    window.clearTimeout(persistIntruderSessionTimer)
    persistIntruderSessionTimer = null
  }
  stopSidebarResize()
  window.removeEventListener('resize', handleWindowResize)
  window.removeEventListener('storage', handleResultsWindowStorage)
  unlistenResultsWindowClosed?.()
  unlistenResultsWindowClosed = null
})

defineExpose({
  addRequestFromHistory,
  openWorkspaceFromWorkbench,
})
</script>
