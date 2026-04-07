<template>
  <div class="card bg-transparent shadow-none">
    <div class="card-body gap-4 px-0 py-0">
      <div class="grid grid-cols-1 xl:grid-cols-[320px_minmax(0,1fr)] gap-4">
        <SystemAgentListPanel
          :loading="loading"
          :items="profileListItems"
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
            <SystemAgentProfileHeaderPanel
              :name="selectedProfile.name"
              :description="selectedProfileDescription"
              :mode-badge="selectedProfileModeBadge"
              :passive-event-name="selectedProfilePassiveEventName"
              :enabled="selectedProfile.enabled"
              @update:enabled="selectedProfile.enabled = $event"
            />

            <SystemAgentWorkspaceTabs v-model="activeWorkspaceTab" :items="workspaceTabs" />

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
                  :disabled="!selectedProfile.id"
                  @refresh="loadRuns"
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
                :name="selectedProfile.name"
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
                @update:settings="contextExtractionSettings = $event"
                @save="saveContextExtractionSettings"
              />

              <div class="grid grid-cols-1 gap-4">
                <SystemAgentLlmOverridePanel
                  :profile-id="selectedProfile.id"
                  :model-value="{
                    provider: selectedProfile.llmProviderOverride,
                    model: selectedProfile.llmModelOverride,
                  }"
                  :provider-options="llmProviderOptions"
                  :model-suggestions="llmModelSuggestions"
                  :global-default-label="globalDefaultLlmLabel"
                  @update:model-value="
                    value => {
                      selectedProfile.llmProviderOverride = value.provider
                      selectedProfile.llmModelOverride = value.model
                    }
                  "
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
                :manual-input-text="manualInputText"
                :dispatch-payload-text="dispatchPayloadText"
                :running="running"
                :dispatching="dispatching"
                @update:manual-input-text="manualInputText = $event"
                @update:dispatch-payload-text="dispatchPayloadText = $event"
                @run="runSelectedProfile"
                @dispatch="dispatchSelectedProfileEvent"
              />

              <SystemAgentRunsPanel
                :runs="runs"
                :disabled="!selectedProfile.id"
                @refresh="loadRuns"
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
              />
            </div>
          </template>
        </SystemAgentDetailLayout>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'

import SystemAgentAdvancedSettingsPanel from './system-agent/SystemAgentAdvancedSettingsPanel.vue'
import SystemAgentAutoSaveStatusBar from './system-agent/SystemAgentAutoSaveStatusBar.vue'
import SystemAgentBehaviorSourcePanel from './system-agent/SystemAgentBehaviorSourcePanel.vue'
import SystemAgentContextExtractionPanel from './system-agent/SystemAgentContextExtractionPanel.vue'
import SystemAgentDebugPanel from './system-agent/SystemAgentDebugPanel.vue'
import SystemAgentDetailLayout from './system-agent/SystemAgentDetailLayout.vue'
import SystemAgentEmptyStatePanel from './system-agent/SystemAgentEmptyStatePanel.vue'
import SystemAgentListPanel from './system-agent/SystemAgentListPanel.vue'
import SystemAgentProfileHeaderPanel from './system-agent/SystemAgentProfileHeaderPanel.vue'
import SystemAgentPromptPatchPanel from './system-agent/SystemAgentPromptPatchPanel.vue'
import SystemAgentRecentFindingsPanel from './system-agent/SystemAgentRecentFindingsPanel.vue'
import SystemAgentSopInsightsPanel from './system-agent/SystemAgentSopInsightsPanel.vue'
import SystemAgentToolbarPanel from './system-agent/SystemAgentToolbarPanel.vue'
import SystemAgentRunsPanel from './system-agent/SystemAgentRunsPanel.vue'
import SystemAgentLlmOverridePanel from './system-agent/SystemAgentLlmOverridePanel.vue'
import SystemAgentSafetyPolicyPanel from './system-agent/SystemAgentSafetyPolicyPanel.vue'
import SystemAgentStatsCards from './system-agent/SystemAgentStatsCards.vue'
import SystemAgentVersionsPanel from './system-agent/SystemAgentVersionsPanel.vue'
import SystemAgentWorkspaceTabs from './system-agent/SystemAgentWorkspaceTabs.vue'
import SystemAgentToolBindingPanel from './SystemAgentToolBindingPanel.vue'
import { useSystemAgentSettingsController } from './system-agent/useSystemAgentSettingsController'

type WorkspaceTabKey = 'overview' | 'config' | 'runs' | 'insights'

const {
  loading,
  running,
  dispatching,
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
  manualInputText,
  dispatchPayloadText,
  toolBindingValue,
  promptPatchPlaceholder,
  promptPatchGuidance,
  selectedProfileDescription,
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
  runSelectedProfile,
  dispatchSelectedProfileEvent,
  seedDefaults,
  refreshAll,
  saveContextExtractionSettings,
  loadRuns,
  loadVersions,
  loadRecentFindings,
} = useSystemAgentSettingsController()

const activeWorkspaceTab = ref<WorkspaceTabKey>('overview')

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
      description: '手动调试智能体，并查看完整运行记录与版本快照。',
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

watch(showRecentFindingsPanel, visible => {
  if (!visible && activeWorkspaceTab.value === 'insights') {
    activeWorkspaceTab.value = 'overview'
  }
})
</script>
