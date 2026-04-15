<template>
  <div class="card bg-transparent shadow-none">
    <TrafficContextCandidateDialog
      :open="showContextCandidateDialog"
      :loading="contextCandidateLoading"
      :applying="contextCandidateApplying"
      :previewing="contextCandidatePreviewLoading"
      :result="contextCandidateResult"
      :preview-result="contextCandidatePreviewResult"
      :selected-candidate-ids="selectedCandidateIds"
      :preferred-preview-focus="preferredPreviewFocus"
      @close="showContextCandidateDialog = false"
      @apply="applySelectedContextCandidates"
      @open-evidence-request="openContextCandidateEvidenceRequest"
      @preview="previewSelectedContextCandidates"
      @update:preferred-preview-focus="preferredPreviewFocus = $event"
      @update:selected-candidate-ids="updateSelectedCandidateIds"
    />

    <div class="card-body gap-4 px-0 py-0">
      <div class="grid grid-cols-1 xl:grid-cols-[320px_minmax(0,1fr)] gap-4">
        <AgentListPanel
          title="后台 Agent 列表"
          empty-text="当前还没有可用后台 Agent。"
          :loading="loading"
          :items="profileListItems"
          :filter-options="systemAgentListFilterOptions"
          :selected-id="selectedProfileId"
          @select="selectProfile"
        />

        <SystemAgentDetailLayout :has-selection="!!selectedProfile">
          <template #toolbar>
            <SystemAgentToolbarPanel
              :loading="loading"
              @seed="seedDefaults"
              @refresh="refreshAll"
            />
          </template>

          <template #empty>
            <SystemAgentEmptyStatePanel />
          </template>

          <template v-if="selectedProfile">
            <AgentIdentityPanel
              eyebrow="后台型 Agent"
              :title="selectedProfileDisplayName"
              :description="selectedProfileDescription"
              :meta-items="selectedProfileMetaItems"
            >
              <template #badges>
                <span class="badge badge-sm" :class="selectedProfileModeBadge.className">
                  {{ selectedProfileModeBadge.label }}
                </span>
              </template>

              <template #actions>
                <label class="label cursor-pointer justify-start gap-3">
                  <input
                    :checked="selectedProfile.enabled"
                    type="checkbox"
                    class="toggle toggle-primary"
                    @change="handleSelectedProfileEnabledToggle"
                  />
                  <span class="label-text">启用</span>
                </label>
              </template>
            </AgentIdentityPanel>

            <AgentWorkspaceTabs v-model="activeWorkspaceTab" :items="workspaceTabs" />

            <div
              class="rounded-lg border border-base-300 bg-base-100 px-4 py-3 text-sm text-base-content/70"
            >
              {{ activeWorkspaceTabDescription }}
            </div>

            <div v-if="activeWorkspaceTab === 'overview'" class="space-y-4">
              <SystemAgentStatsCards
                v-model="statsWindow"
                :runs="runs"
                :recent-findings="recentFindings"
                :versions="versions"
                :total-findings-count="totalFindingsCount"
                :false-positive-findings-count="falsePositiveFindingsCount"
                :stats-mode="selectedProfileStatsMode"
              />

              <div class="grid grid-cols-1 2xl:grid-cols-2 gap-4">
                <SystemAgentRunsPanel
                  :runs="overviewRuns"
                  :disabled="!selectedProfile.id || mutatingRuns"
                  :deleting-run-id="deletingRunId"
                  @refresh="loadRuns"
                  @delete-run="deleteRun"
                  @clear-runs="clearRuns"
                />

                <SystemAgentRecentFindingsPanel
                  v-if="showRecentFindingsPanel"
                  :findings="overviewFindings"
                  :disabled="!selectedProfile.id"
                  :window-start="statsWindowStart"
                  @refresh="loadRecentFindings"
                />

                <SystemAgentVersionsPanel
                  v-else
                  :versions="overviewVersions"
                  :disabled="!selectedProfile.id"
                  @refresh="loadVersions"
                />
              </div>
            </div>

            <div v-else-if="activeWorkspaceTab === 'config'" class="space-y-4">
              <SystemAgentPromptPatchPanel
                :name="selectedProfileDisplayName"
                :prompt-patch="selectedProfile.promptPatch || ''"
                :placeholder="promptPatchPlaceholder"
                :guidance="promptPatchGuidance"
                @update:prompt-patch="selectedProfile.promptPatch = $event"
              />

              <SystemAgentBehaviorSourcePanel
                v-if="showBehaviorSourcePanel"
                :settings="behaviorSignalSettings"
                :stats="behaviorEffectStats"
                :window="statsWindow"
              />

              <SystemAgentContextExtractionPanel
                v-if="showBehaviorSourcePanel"
                :settings="contextExtractionSettings"
                :saving="contextExtractionSaving"
                :recommending="contextCandidateLoading"
                :recent-history-limit="recentHistoryLimit"
                :recent-history-limit-options="recentHistoryLimitOptions"
                @update:settings="contextExtractionSettings = $event"
                @update:recent-history-limit="recentHistoryLimit = $event"
                @recommend-from-history="recommendContextCandidatesFromRecentHistory"
                @save="saveContextExtractionSettings"
              />

              <div class="grid grid-cols-1 gap-4">
                <AgentModelPanel
                  title="LLM 覆盖"
                  description="不设置时自动继承 AI 设置中的默认提供商和模型。"
                  provider-label="LLM 提供商"
                  model-label="模型"
                  :provider-value="selectedProfile.llmProviderOverride || ''"
                  :model-value="selectedProfile.llmModelOverride || ''"
                  :provider-options="llmProviderOptions"
                  :model-options="selectedProfileModelOptions"
                  :global-default-label="globalDefaultLlmLabel"
                  :datalist-id="selectedProfileModelDatalistId"
                  follow-default-option-label="跟随全局默认"
                  follow-badge-label="跟随全局"
                  custom-badge-label="自定义中"
                  suggested-hint="已加载该提供商的模型建议，也可手动输入模型 ID。"
                  @update:provider-value="updateSelectedProfileProviderOverride"
                  @update:model-value="updateSelectedProfileModelOverride"
                />
                <SystemAgentToolBindingPanel v-model="toolBindingValue" />
                <SystemAgentSafetyPolicyPanel
                  v-model="safetyPolicyValue"
                  :safety-mode="selectedProfileSafetyMode"
                />
              </div>

              <SystemAgentAdvancedSettingsPanel
                :profile-id="selectedProfile.id"
                :base-prompt-id="selectedProfile.basePromptId"
                :cooldown-secs="selectedProfile.cooldownSecs"
                :max-concurrency="selectedProfile.maxConcurrency"
                @update:cooldown-secs="selectedProfile.cooldownSecs = $event"
                @update:max-concurrency="selectedProfile.maxConcurrency = $event"
              />

              <SystemAgentAutoSaveStatusBar
                :status-text="autoSaveStatusText"
                :status-class="autoSaveStatusClass"
              />
            </div>

            <div v-else-if="activeWorkspaceTab === 'runs'" class="space-y-4">
              <SystemAgentDebugPanel
                :profile="selectedProfile"
                :dispatch-payload-text="dispatchPayloadText"
                :dispatching="dispatching"
                @update:dispatch-payload-text="dispatchPayloadText = $event"
                @dispatch="dispatchSelectedProfileEvent"
              />

              <SystemAgentRunsPanel
                :runs="runs"
                :disabled="!selectedProfile.id || mutatingRuns"
                :deleting-run-id="deletingRunId"
                @refresh="loadRuns"
                @delete-run="deleteRun"
                @clear-runs="clearRuns"
              />

              <SystemAgentVersionsPanel
                :versions="versions"
                :disabled="!selectedProfile.id"
                @refresh="loadVersions"
              />
            </div>

            <div v-else class="space-y-4">
              <SystemAgentRecentFindingsPanel
                :findings="recentFindings"
                :disabled="!selectedProfile.id"
                :window-start="statsWindowStart"
                @refresh="loadRecentFindings"
              />

              <SystemAgentSopInsightsPanel
                :profile-id="selectedProfile.id"
                :findings="recentFindings"
                :window-start="statsWindowStart"
                :definitions="selectedProfile.sopDefinitions || []"
                @update:definitions="selectedProfile.sopDefinitions = $event"
              />
            </div>
          </template>
        </SystemAgentDetailLayout>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { emit as tauriEmit } from '@tauri-apps/api/event'
import { computed, ref, watch } from 'vue'
import { useRouter } from 'vue-router'

import { dialog } from '@/composables/useDialog'
import AgentIdentityPanel from './AgentIdentityPanel.vue'
import AgentListPanel from './AgentListPanel.vue'
import AgentModelPanel from './AgentModelPanel.vue'
import AgentWorkspaceTabs from './AgentWorkspaceTabs.vue'
import SystemAgentAdvancedSettingsPanel from './system-agent/SystemAgentAdvancedSettingsPanel.vue'
import SystemAgentAutoSaveStatusBar from './system-agent/SystemAgentAutoSaveStatusBar.vue'
import SystemAgentBehaviorSourcePanel from './system-agent/SystemAgentBehaviorSourcePanel.vue'
import SystemAgentContextExtractionPanel from './system-agent/SystemAgentContextExtractionPanel.vue'
import SystemAgentDebugPanel from './system-agent/SystemAgentDebugPanel.vue'
import SystemAgentDetailLayout from './system-agent/SystemAgentDetailLayout.vue'
import SystemAgentEmptyStatePanel from './system-agent/SystemAgentEmptyStatePanel.vue'
import SystemAgentPromptPatchPanel from './system-agent/SystemAgentPromptPatchPanel.vue'
import SystemAgentRecentFindingsPanel from './system-agent/SystemAgentRecentFindingsPanel.vue'
import SystemAgentSopInsightsPanel from './system-agent/SystemAgentSopInsightsPanel.vue'
import SystemAgentToolbarPanel from './system-agent/SystemAgentToolbarPanel.vue'
import SystemAgentRunsPanel from './system-agent/SystemAgentRunsPanel.vue'
import SystemAgentSafetyPolicyPanel from './system-agent/SystemAgentSafetyPolicyPanel.vue'
import SystemAgentStatsCards from './system-agent/SystemAgentStatsCards.vue'
import SystemAgentVersionsPanel from './system-agent/SystemAgentVersionsPanel.vue'
import SystemAgentToolBindingPanel from './SystemAgentToolBindingPanel.vue'
import TrafficContextCandidateDialog from '@/components/traffic/TrafficContextCandidateDialog.vue'
import { buildTrafficContextCandidateId } from '@/components/traffic/trafficContextCandidateSupport'
import { useTrafficContextCandidatePreferences } from '@/components/traffic/useTrafficContextCandidatePreferences'
import type {
  RecommendTrafficContextDictionaryCandidatesResponse,
  TrafficContextCandidateEvidenceSelection,
  TrafficContextExtractionPreviewResponse,
} from '@/components/traffic/trafficContextCandidateTypes'
import { listHttpRequests } from '@/services/proxy_history'
import {
  getTrafficContextExtractionSettings,
  mergeCandidatesIntoTrafficContextExtractionSettings,
  previewTrafficContextExtractionChanges,
  recommendTrafficContextDictionaryCandidates,
  setTrafficContextExtractionSettings,
} from '@/services/trafficContextCandidates'
import { useSystemAgentSettingsController } from './system-agent/useSystemAgentSettingsController'

type WorkspaceTabKey = 'overview' | 'config' | 'runs' | 'insights'
const OPEN_TRAFFIC_HISTORY_REQUEST_EVENT = 'traffic-history:open-request'
const OPEN_TRAFFIC_HISTORY_REQUEST_STORAGE_KEY = 'traffic-history:pending-open-request'
const systemAgentListFilterOptions = [
  { key: 'enabled', label: '已启用' },
  { key: 'disabled', label: '已停用' },
  { key: 'triage', label: 'Triage' },
  { key: 'verifier', label: 'Verifier' },
]

const {
  loading,
  dispatching,
  mutatingRuns,
  deletingRunId,
  runs,
  versions,
  recentFindings,
  totalFindingsCount,
  falsePositiveFindingsCount,
  statsWindow,
  selectedProfileId,
  selectedProfile,
  llmProviderOptions,
  llmModelSuggestions,
  globalDefaultLlmLabel,
  safetyPolicyValue,
  dispatchPayloadText,
  toolBindingValue,
  promptPatchPlaceholder,
  promptPatchGuidance,
  selectedProfileDescription,
  selectedProfileDisplayName,
  autoSaveStatusText,
  autoSaveStatusClass,
  behaviorSignalSettings,
  contextExtractionSettings,
  contextExtractionSaving,
  behaviorEffectStats,
  showBehaviorSourcePanel,
  showRecentFindingsPanel,
  selectedProfileModeBadge,
  selectedProfilePassiveEventName,
  selectedProfileStatsMode,
  selectedProfileSafetyMode,
  statsWindowStart,
  profileListItems,
  selectProfile,
  dispatchSelectedProfileEvent,
  seedDefaults,
  refreshAll,
  saveContextExtractionSettings,
  loadRuns,
  deleteRun,
  clearRuns,
  loadVersions,
  loadRecentFindings,
} = useSystemAgentSettingsController()

const activeWorkspaceTab = ref<WorkspaceTabKey>('overview')
const router = useRouter()
const { preferredPreviewFocus } = useTrafficContextCandidatePreferences()
const showContextCandidateDialog = ref(false)
const contextCandidateLoading = ref(false)
const contextCandidateApplying = ref(false)
const contextCandidatePreviewLoading = ref(false)
const contextCandidateResult = ref<RecommendTrafficContextDictionaryCandidatesResponse | null>(null)
const contextCandidatePreviewResult = ref<TrafficContextExtractionPreviewResponse | null>(null)
const selectedCandidateIds = ref<string[]>([])
const contextCandidateRequestIds = ref<number[]>([])
const recentHistoryLimitOptions = [50, 100, 300]
const recentHistoryLimit = ref(100)
const selectedProfileModelDatalistId = computed(() =>
  `system-agent-llm-models-${selectedProfile.value?.id || 'profile'}`
)
const selectedProfileMetaItems = computed(() => {
  if (!selectedProfile.value) return []

  return [
    {
      label: 'Agent ID',
      value: selectedProfile.value.id,
    },
    {
      label: '能力',
      value: selectedProfile.value.capability,
    },
    {
      label: '自动事件',
      value: selectedProfilePassiveEventName.value || '未配置',
    },
  ]
})
const selectedProfileModelOptions = computed(() =>
  llmModelSuggestions.value.map(model => ({
    value: model,
    label: model,
  }))
)

const workspaceTabs = computed(() => {
  const tabs: Array<{
    key: WorkspaceTabKey
    label: string
    description: string
    count?: number
  }> = [
    {
      key: 'overview',
      label: '概览',
      description: '先看状态和近期结果，再决定进入配置、运行或洞察。',
    },
    {
      key: 'config',
      label: '配置',
      description: '集中调整提示词、工具绑定、安全策略和高级参数。',
    },
    {
      key: 'runs',
      label: '运行',
      description: '通过测试事件回放调试后台 Agent，并查看完整运行记录与版本快照。',
      count: runs.value.length,
    },
  ]

  if (showRecentFindingsPanel.value) {
    tabs.push({
      key: 'insights',
      label: '洞察',
      description: '查看最近发现与 SOP 洞察，避免把信息都堆在首页。',
      count: totalFindingsCount.value,
    })
  }

  return tabs
})

const activeWorkspaceTabDescription = computed(() => {
  return workspaceTabs.value.find(tab => tab.key === activeWorkspaceTab.value)?.description || ''
})

const overviewRuns = computed(() => runs.value.slice(0, 3))
const overviewVersions = computed(() => versions.value.slice(0, 3))
const overviewFindings = computed(() => recentFindings.value.slice(0, 3))
const selectedContextCandidates = computed(() => {
  const selectedIds = new Set(selectedCandidateIds.value)
  return (contextCandidateResult.value?.candidates || []).filter(candidate =>
    selectedIds.has(buildTrafficContextCandidateId(candidate)),
  )
})

function handleSelectedProfileEnabledToggle(event: Event) {
  const target = event.target as HTMLInputElement | null
  if (!selectedProfile.value) return
  selectedProfile.value.enabled = target?.checked === true
}

function updateSelectedProfileProviderOverride(value: string) {
  if (!selectedProfile.value) return
  const provider = value.trim() || null
  selectedProfile.value.llmProviderOverride = provider
  if (!provider) {
    selectedProfile.value.llmModelOverride = null
  }
}

function updateSelectedProfileModelOverride(value: string) {
  if (!selectedProfile.value) return
  selectedProfile.value.llmModelOverride = value.trim() || null
}

async function recommendContextCandidatesFromRecentHistory() {
  showContextCandidateDialog.value = true
  contextCandidateLoading.value = true
  contextCandidateResult.value = null
  contextCandidatePreviewResult.value = null
  selectedCandidateIds.value = []

  try {
    const requests = await listHttpRequests({
      limit: recentHistoryLimit.value,
      offset: 0,
    })
    const requestIds = [...new Set(requests.map(request => request.id).filter(id => Number.isFinite(id)))]
    if (requestIds.length === 0) {
      dialog.toast.warning('当前没有可用于生成候选的历史记录')
      showContextCandidateDialog.value = false
      return
    }
    contextCandidateRequestIds.value = requestIds

    const result = await recommendTrafficContextDictionaryCandidates({
      requestIds,
      maxCandidatesPerCategory: 12,
    })
    contextCandidateResult.value = result
    selectedCandidateIds.value = result.candidates
      .filter(candidate => candidate.confidence === 'high' && !candidate.alreadyCoveredBy)
      .map(buildTrafficContextCandidateId)
  } catch (error) {
    console.error('Failed to recommend traffic context candidates from settings:', error)
    dialog.toast.error(`生成词典候选失败: ${String(error)}`)
    showContextCandidateDialog.value = false
  } finally {
    contextCandidateLoading.value = false
  }
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
    const savedSettings = await setTrafficContextExtractionSettings(nextSettings)
    contextExtractionSettings.value = savedSettings
    dialog.toast.success(`已把 ${selectedContextCandidates.value.length} 项候选合并到上下文词典`)
    showContextCandidateDialog.value = false
    selectedCandidateIds.value = []
  } catch (error) {
    console.error('Failed to apply traffic context candidates from settings:', error)
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
    const currentSettings = {
      principalKeys: [...contextExtractionSettings.value.principalKeys],
      resourceKeyHints: [...contextExtractionSettings.value.resourceKeyHints],
      authHeaderKeys: [...contextExtractionSettings.value.authHeaderKeys],
      authTokenKeys: [...contextExtractionSettings.value.authTokenKeys],
      cookieHintKeys: [...contextExtractionSettings.value.cookieHintKeys],
      actionAliases: Object.fromEntries(
        Object.entries(contextExtractionSettings.value.actionAliases).map(([action, aliases]) => [action, [...aliases]]),
      ),
    }
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
    console.error('Failed to preview traffic context candidates from settings:', error)
    dialog.toast.error(`预览命中变化失败: ${String(error)}`)
  } finally {
    contextCandidatePreviewLoading.value = false
  }
}

function updateSelectedCandidateIds(nextValue: string[]) {
  selectedCandidateIds.value = nextValue
  contextCandidatePreviewResult.value = null
}

async function openContextCandidateEvidenceRequest(payload: TrafficContextCandidateEvidenceSelection) {
  try {
    window.sessionStorage.setItem(OPEN_TRAFFIC_HISTORY_REQUEST_STORAGE_KEY, JSON.stringify(payload))
    await router.push({ name: 'TrafficAnalysis' })
    await tauriEmit(OPEN_TRAFFIC_HISTORY_REQUEST_EVENT, payload)
  } catch (error) {
    console.error('Failed to open context candidate evidence request from settings:', error)
    dialog.toast.error(`打开证据请求失败: ${String(error)}`)
  }
}

watch(showRecentFindingsPanel, visible => {
  if (!visible && activeWorkspaceTab.value === 'insights') {
    activeWorkspaceTab.value = 'overview'
  }
})
</script>
